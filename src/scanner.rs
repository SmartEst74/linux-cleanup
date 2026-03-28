use anyhow::Result;
use humansize::{format_size, BINARY};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use sysinfo::Disks;
use walkdir::WalkDir;

#[derive(Debug, Clone)]
pub struct DiskInfo {
    pub name: String,
    pub mount_point: PathBuf,
    pub total: u64,
    pub used: u64,
    pub available: u64,
    pub fs_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Category {
    PackageCache,
    SystemdJournals,
    TempFiles,
    Docker,
    Logs,
    UserCache,
    BrowserCache,
    Trash,
    CrashDumps,
    OldKernels,
    SnapRevisions,
    Flatpak,
    BuildArtifacts,
    LargeFiles,
    ThumbnailCache,
    DevShm,
    Other(String),
}

impl Category {
    pub fn label(&self) -> &str {
        match self {
            Category::PackageCache => "Package Caches",
            Category::SystemdJournals => "Systemd Journals",
            Category::TempFiles => "Temporary Files",
            Category::Docker => "Docker Artifacts",
            Category::Logs => "Log Files",
            Category::UserCache => "User Caches",
            Category::BrowserCache => "Browser Caches",
            Category::Trash => "Trash",
            Category::CrashDumps => "Crash Dumps",
            Category::OldKernels => "Old Kernels",
            Category::SnapRevisions => "Old Snap Revisions",
            Category::Flatpak => "Flatpak Leftovers",
            Category::BuildArtifacts => "Build Artifacts",
            Category::LargeFiles => "Large Files",
            Category::ThumbnailCache => "Thumbnail Cache",
            Category::DevShm => "Shared Memory",
            Category::Other(name) => name.as_str(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScanEntry {
    pub path: PathBuf,
    pub size: u64,
    pub category: Category,
    pub is_dir: bool,
    pub age_days: Option<u64>,
}

pub struct ScanResult {
    pub entries: Vec<ScanEntry>,
    pub category_totals: HashMap<Category, u64>,
    pub category_counts: HashMap<Category, u64>,
    pub total_size: u64,
    pub disk_info: Vec<DiskInfo>,
    pub scan_path: PathBuf,
    pub files_scanned: u64,
    pub toplevel_sizes: Vec<(String, u64)>,
}

pub fn get_disk_info() -> Vec<DiskInfo> {
    let disks = Disks::new_with_refreshed_list();
    disks
        .list()
        .iter()
        .map(|d| {
            let total = d.total_space();
            let available = d.available_space();
            DiskInfo {
                name: d.name().to_string_lossy().into_owned(),
                mount_point: d.mount_point().to_path_buf(),
                total,
                used: total - available,
                available,
                fs_type: d.file_system().to_string_lossy().into_owned(),
            }
        })
        .collect()
}

fn file_age_days(path: &Path) -> Option<u64> {
    let metadata = std::fs::metadata(path).ok()?;
    let modified = metadata.modified().ok()?;
    let elapsed = std::time::SystemTime::now().duration_since(modified).ok()?;
    Some(elapsed.as_secs() / 86400)
}

pub fn classify_path(path: &Path) -> Category {
    let path_str = path.to_string_lossy();
    let lower = path_str.to_lowercase();

    // Skip pseudo-filesystems (should already be excluded by walker, but double-check)
    if lower.starts_with("/proc/")
        || lower.starts_with("/sys/")
        || lower.starts_with("/dev/")
        || lower.starts_with("/run/")
    {
        return Category::Other("Pseudo FS".to_string());
    }

    // Snap old revisions
    if lower.contains("/var/lib/snapd/snaps/") {
        return Category::SnapRevisions;
    }
    if lower.contains("/snap/") && (lower.contains("/.cache") || lower.contains("/partial")) {
        return Category::SnapRevisions;
    }

    // Package manager caches
    if lower.contains("/cache/apt/")
        || lower.contains("/var/cache/apt")
        || lower.contains("/cache/dnf/")
        || lower.contains("/var/cache/dnf")
        || lower.contains("/cache/pacman/")
        || lower.contains("/var/cache/pacman")
        || lower.contains("/cache/yum/")
        || lower.contains("/var/cache/yum")
        || lower.contains("/cache/zypper/")
        || lower.contains("/var/cache/zypper")
    {
        return Category::PackageCache;
    }

    // Flatpak
    if lower.contains("/.local/share/flatpak/")
        || lower.contains("/var/lib/flatpak/")
        || lower.contains("/flatpak/")
    {
        return Category::Flatpak;
    }

    // Docker
    if lower.contains("/docker/")
        || lower.contains("/containers/storage/")
        || lower.contains("/overlay2/")
    {
        return Category::Docker;
    }

    // Systemd journals
    if lower.contains("/systemd/journal") || lower.contains("/log/journal/") {
        return Category::SystemdJournals;
    }

    // Temp files
    if lower.starts_with("/tmp/")
        || lower.starts_with("/var/tmp/")
        || lower == "/tmp"
        || lower == "/var/tmp"
    {
        return Category::TempFiles;
    }

    // /dev/shm
    if lower.starts_with("/dev/shm/") {
        return Category::DevShm;
    }

    // Crash dumps
    if lower.contains("/var/crash/")
        || lower.contains("/var/lib/apport/")
        || lower.contains("/var/lib/systemd/coredump/")
    {
        return Category::CrashDumps;
    }

    // Log files (but not journal)
    if lower.contains("/var/log/") && !lower.contains("/journal") {
        return Category::Logs;
    }

    // Browser caches (Chrome, Chromium, Firefox, Brave, Opera, Vivaldi)
    if lower.contains("/.cache/google-chrome")
        || lower.contains("/.cache/chromium")
        || lower.contains("/.cache/mozilla")
        || lower.contains("/.cache/firefox")
        || lower.contains("/.cache/brave")
        || lower.contains("/.cache/opera")
        || lower.contains("/.cache/vivaldi")
        || lower.contains(".mozilla/firefox/") && lower.contains("/cache")
        || lower.contains("google-chrome/") && lower.contains("/cache")
        || lower.contains(".config/chromium/")
    {
        return Category::BrowserCache;
    }

    // Thumbnail cache
    if lower.contains("/.cache/thumbnails") || lower.contains("/.thumbnails") {
        return Category::ThumbnailCache;
    }

    // User caches (general - must come after specific caches)
    if (lower.contains("/.cache/") || lower.contains("/.local/share/Trash/"))
        && !lower.contains("/apt")
        && !lower.contains("/dnf")
        && !lower.contains("/pacman")
    {
        if lower.contains("/trash") || lower.contains("/Trash") {
            return Category::Trash;
        }
        return Category::UserCache;
    }

    // Trash
    if lower.contains("/.local/share/Trash") || lower.contains("/trash") {
        return Category::Trash;
    }

    // Build artifacts
    if lower.contains("/target/debug/")
        || lower.contains("/target/release/")
        || lower.contains("/node_modules/")
        || lower.contains("/__pycache__/")
        || lower.contains("/.tox/")
        || lower.contains("/build/")
            && (lower.contains("/cmake") || lower.contains("/meson") || lower.contains("/make"))
    {
        return Category::BuildArtifacts;
    }

    // Old kernels (in /boot)
    if lower.starts_with("/boot/") && (lower.contains("vmlinuz-") || lower.contains("initrd.img-"))
    {
        return Category::OldKernels;
    }

    Category::Other("Uncategorized".to_string())
}

pub fn scan(
    root: &Path,
    min_size_threshold: u64,
    max_depth: usize,
    mut progress: Option<&mut dyn FnMut(u64)>,
) -> Result<ScanResult> {
    let mut entries = Vec::new();
    let mut category_totals: HashMap<Category, u64> = HashMap::new();
    let mut category_counts: HashMap<Category, u64> = HashMap::new();
    let mut total_size: u64 = 0;
    let mut files_scanned: u64 = 0;

    let disk_info = get_disk_info();
    let toplevel_sizes = get_toplevel_sizes(root);

    let walker = WalkDir::new(root)
        .max_depth(max_depth)
        .follow_links(false)
        .into_iter();

    const SKIP_DIRS: &[&str] = &["/proc", "/sys", "/dev", "/run", "/snap"];

    for entry in walker.filter_map(|e| e.ok()) {
        files_scanned += 1;

        if files_scanned % 1000 == 0 {
            if let Some(ref mut cb) = progress {
                cb(files_scanned);
            }
        }

        let path_str = entry.path().to_string_lossy();
        if SKIP_DIRS.iter().any(|d| path_str.starts_with(d)) {
            continue;
        }

        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };

        let size = if metadata.is_dir() { 0 } else { metadata.len() };

        if size > 1_000_000_000_000 {
            continue;
        }

        if size < min_size_threshold && !metadata.is_dir() {
            continue;
        }

        let category = classify_path(entry.path());
        let age = if !metadata.is_dir() {
            file_age_days(entry.path())
        } else {
            None
        };

        let scan_entry = ScanEntry {
            path: entry.path().to_path_buf(),
            size,
            category: category.clone(),
            is_dir: metadata.is_dir(),
            age_days: age,
        };

        *category_totals.entry(category.clone()).or_insert(0) += size;
        *category_counts.entry(category).or_insert(0) += 1;
        total_size += size;
        entries.push(scan_entry);
    }

    if let Some(ref mut cb) = progress {
        cb(files_scanned);
    }

    entries.sort_by(|a, b| b.size.cmp(&a.size));

    Ok(ScanResult {
        entries,
        category_totals,
        category_counts,
        total_size,
        disk_info,
        scan_path: root.to_path_buf(),
        files_scanned,
        toplevel_sizes,
    })
}

pub fn format_bytes(bytes: u64) -> String {
    format_size(bytes, BINARY)
}

pub fn get_toplevel_sizes(path: &Path) -> Vec<(String, u64)> {
    let mut sizes = Vec::new();

    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.filter_map(|e| e.ok()) {
            let name = entry.file_name().to_string_lossy().to_string();
            if matches!(
                name.as_str(),
                "proc" | "sys" | "dev" | "run" | "snap" | "cdrom" | "lost+found"
            ) {
                continue;
            }
            let size = dir_size_quick(&entry.path());
            if size > 0 {
                sizes.push((name, size));
            }
        }
    }

    sizes.sort_by(|a, b| b.1.cmp(&a.1));
    sizes.truncate(10);
    sizes
}

pub fn dir_size_quick(path: &Path) -> u64 {
    let mut total = 0;
    for entry in WalkDir::new(path)
        .max_depth(2)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if let Ok(meta) = entry.metadata() {
            if !meta.is_dir() {
                total += meta.len();
            }
        }
    }
    total
}

pub fn entries_by_category<'a>(scan: &'a ScanResult, category: &Category) -> Vec<&'a ScanEntry> {
    let mut filtered: Vec<_> = scan
        .entries
        .iter()
        .filter(|e| &e.category == category)
        .collect();
    filtered.sort_by(|a, b| b.size.cmp(&a.size));
    filtered.truncate(50);
    filtered
}

#[derive(Debug, Clone)]
pub struct SnapRevisionInfo {
    pub name: String,
    pub revision: String,
    pub size: u64,
}

pub fn detect_old_snap_revisions() -> Vec<SnapRevisionInfo> {
    let mut revisions = Vec::new();

    let output = match Command::new("sh")
        .arg("-c")
        .arg("snap list --all 2>/dev/null | grep -E '\\(disabled\\)|\\sdisabled\\s' || true")
        .output()
    {
        Ok(o) => o,
        Err(_) => return revisions,
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 3 {
            let name = parts[0].to_string();
            let revision = parts[1].to_string();
            let snap_path = format!("/var/lib/snapd/snaps/{}_{}.snap", name, revision);
            let size = std::fs::metadata(&snap_path).map(|m| m.len()).unwrap_or(0);

            if size > 0 {
                revisions.push(SnapRevisionInfo {
                    name,
                    revision,
                    size,
                });
            }
        }
    }

    revisions
}

pub fn old_snap_revisions_total_size(revisions: &[SnapRevisionInfo]) -> u64 {
    revisions.iter().map(|r| r.size).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(1024), "1 KiB");
        assert_eq!(format_bytes(1024 * 1024), "1 MiB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1 GiB");
    }

    #[test]
    fn test_classify_path_package_cache() {
        assert_eq!(
            classify_path(Path::new("/var/cache/apt/archives/pkg.deb")),
            Category::PackageCache
        );
        assert_eq!(
            classify_path(Path::new("/var/cache/dnf/packages/pkg.rpm")),
            Category::PackageCache
        );
        assert_eq!(
            classify_path(Path::new("/var/cache/pacman/pkg/")),
            Category::PackageCache
        );
    }

    #[test]
    fn test_classify_path_snap() {
        assert_eq!(
            classify_path(Path::new("/var/lib/snapd/snaps/gnome-42_100.snap")),
            Category::SnapRevisions
        );
    }

    #[test]
    fn test_classify_path_logs() {
        assert_eq!(
            classify_path(Path::new("/var/log/syslog.1")),
            Category::Logs
        );
        assert_eq!(
            classify_path(Path::new("/var/log/nginx/access.log")),
            Category::Logs
        );
    }

    #[test]
    fn test_classify_path_temp_files() {
        assert_eq!(
            classify_path(Path::new("/tmp/random_file")),
            Category::TempFiles
        );
        assert_eq!(
            classify_path(Path::new("/var/tmp/stuff")),
            Category::TempFiles
        );
    }

    #[test]
    fn test_classify_path_user_cache() {
        assert_eq!(
            classify_path(Path::new("/home/user/.cache/app/data")),
            Category::UserCache
        );
    }

    #[test]
    fn test_classify_path_browser_cache() {
        assert_eq!(
            classify_path(Path::new(
                "/home/user/.cache/google-chrome/Default/Cache/data"
            )),
            Category::BrowserCache
        );
        assert_eq!(
            classify_path(Path::new(
                "/home/user/.cache/firefox/abc123/cache2/entries/xyz"
            )),
            Category::BrowserCache
        );
    }

    #[test]
    fn test_classify_path_crash_dumps() {
        assert_eq!(
            classify_path(Path::new("/var/crash/_usr_bin_app.123.crash")),
            Category::CrashDumps
        );
        assert_eq!(
            classify_path(Path::new("/var/lib/systemd/coredump/app.core")),
            Category::CrashDumps
        );
    }

    #[test]
    fn test_classify_path_trash() {
        assert_eq!(
            classify_path(Path::new("/home/user/.local/share/Trash/files/doc.txt")),
            Category::Trash
        );
    }

    #[test]
    fn test_classify_path_flatpak() {
        assert_eq!(
            classify_path(Path::new("/var/lib/flatpak/app/com.example.App")),
            Category::Flatpak
        );
        assert_eq!(
            classify_path(Path::new("/home/user/.local/share/flatpak/runtime/xyz")),
            Category::Flatpak
        );
    }

    #[test]
    fn test_classify_path_docker() {
        assert_eq!(
            classify_path(Path::new("/var/lib/docker/overlay2/abc123/merged")),
            Category::Docker
        );
    }

    #[test]
    fn test_classify_path_old_kernels() {
        assert_eq!(
            classify_path(Path::new("/boot/vmlinuz-5.15.0-50-generic")),
            Category::OldKernels
        );
        assert_eq!(
            classify_path(Path::new("/boot/initrd.img-5.15.0-50-generic")),
            Category::OldKernels
        );
    }

    #[test]
    fn test_classify_path_journals() {
        assert_eq!(
            classify_path(Path::new("/var/log/journal/abc123/system.journal")),
            Category::SystemdJournals
        );
    }

    #[test]
    fn test_classify_path_dev_shm() {
        // /dev/shm is part of /dev which is excluded as pseudo-filesystem
        // but if it were scanned, it would be classified as DevShm
        // This test verifies the pseudo-filesystem exclusion takes precedence
        let result = classify_path(Path::new("/dev/shm/shared_memory"));
        assert!(matches!(result, Category::Other(_)));
    }

    #[test]
    fn test_classify_path_large_files() {
        // LargeFiles is not auto-assigned by classify_path,
        // it's set by the scanner for user-defined large file detection
        // This test documents that behavior
        let result = classify_path(Path::new("/some/random/large/file.bin"));
        assert_eq!(result, Category::Other("Uncategorized".to_string()));
    }

    #[test]
    fn test_classify_path_excludes_pseudo_filesystems() {
        let result = classify_path(Path::new("/proc/self/mem"));
        assert!(matches!(result, Category::Other(_)));
    }

    #[test]
    fn test_scan_entry_has_is_dir() {
        // Verify the ScanEntry struct has is_dir field
        let entry = ScanEntry {
            path: PathBuf::from("/tmp/test"),
            size: 1024,
            category: Category::TempFiles,
            is_dir: false,
            age_days: Some(5),
        };
        assert!(!entry.is_dir);
        assert_eq!(entry.age_days, Some(5));
    }

    #[test]
    fn test_scan_result_has_category_counts() {
        // Verify ScanResult has category_counts and total_size
        let scan = ScanResult {
            entries: vec![],
            category_totals: HashMap::new(),
            category_counts: HashMap::new(),
            total_size: 0,
            disk_info: vec![],
            scan_path: PathBuf::from("/"),
            files_scanned: 0,
            toplevel_sizes: vec![],
        };
        assert_eq!(scan.category_counts.len(), 0);
        assert_eq!(scan.total_size, 0);
    }

    #[test]
    fn test_disk_info_fields() {
        let info = DiskInfo {
            name: String::from("sda1"),
            mount_point: PathBuf::from("/"),
            total: 1000,
            used: 500,
            available: 500,
            fs_type: String::from("ext4"),
        };
        assert_eq!(info.name, "sda1");
        assert_eq!(info.fs_type, "ext4");
        assert_eq!(info.available, 500);
    }
}
