use crate::distro::{self, DistroInfo};
use crate::safety::SafetyLevel;
use crate::scanner::{self, Category, ScanResult, SnapRevisionInfo};
use crate::Args;
use anyhow::Result;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct CleanupAction {
    pub description: String,
    pub command: String,
    pub safety: SafetyLevel,
    pub estimated_bytes: u64,
}

pub fn plan_cleanups(
    scan: &ScanResult,
    distro: &DistroInfo,
    old_snaps: &[SnapRevisionInfo],
) -> Vec<CleanupAction> {
    let mut actions = Vec::new();

    // Package cache
    if let Some(&size) = scan.category_totals.get(&Category::PackageCache) {
        if size > 0 {
            if let Some(cmd) = distro.pkg_manager.cache_clean_cmd() {
                actions.push(CleanupAction {
                    description: format!("Clean {} package cache", distro.pkg_manager.label()),
                    command: cmd.to_string(),
                    safety: SafetyLevel::Safe,
                    estimated_bytes: size,
                });
            }
        }
    }

    // Systemd journals
    if let Some(&size) = scan.category_totals.get(&Category::SystemdJournals) {
        if size > 0 && distro.has_journal {
            actions.push(CleanupAction {
                description: "Vacuum systemd journals (keep 100M)".to_string(),
                command: "sudo journalctl --vacuum-size=100M".to_string(),
                safety: SafetyLevel::Safe,
                estimated_bytes: size.min(500_000_000),
            });
        }
    }

    // Temp files
    if let Some(&size) = scan.category_totals.get(&Category::TempFiles) {
        if size > 0 {
            actions.push(CleanupAction {
                description: "Remove temp files older than 3 days".to_string(),
                command: "sudo find /tmp /var/tmp -type f -atime +3 -delete 2>/dev/null; \
                          sudo find /tmp /var/tmp -type d -empty -delete 2>/dev/null"
                    .to_string(),
                safety: SafetyLevel::Safe,
                estimated_bytes: size,
            });
        }
    }

    // Docker
    if let Some(&size) = scan.category_totals.get(&Category::Docker) {
        if size > 0 && distro.has_docker {
            actions.push(CleanupAction {
                description: "Prune dangling Docker images and build cache".to_string(),
                command: "docker system prune -f".to_string(),
                safety: SafetyLevel::Safe,
                estimated_bytes: size,
            });
        }
    }

    // User caches
    if let Some(&size) = scan.category_totals.get(&Category::UserCache) {
        if size > 0 {
            actions.push(CleanupAction {
                description: "Clear user cache (>7 days old)".to_string(),
                command: "find ~/.cache -type f -atime +7 -delete 2>/dev/null; \
                          find ~/.cache -type d -empty -delete 2>/dev/null"
                    .to_string(),
                safety: SafetyLevel::Safe,
                estimated_bytes: size / 2,
            });
        }
    }

    // Browser caches
    if let Some(&size) = scan.category_totals.get(&Category::BrowserCache) {
        if size > 0 {
            actions.push(CleanupAction {
                description: "Clear browser caches".to_string(),
                command: "rm -rf ~/.cache/google-chrome/Default/Cache/* \
                          ~/.cache/chromium/Default/Cache/* \
                          ~/.cache/mozilla/firefox/*/cache2/* 2>/dev/null"
                    .to_string(),
                safety: SafetyLevel::Safe,
                estimated_bytes: size,
            });
        }
    }

    // Thumbnail cache
    if let Some(&size) = scan.category_totals.get(&Category::ThumbnailCache) {
        if size > 0 {
            actions.push(CleanupAction {
                description: "Clear thumbnail cache".to_string(),
                command: "rm -rf ~/.cache/thumbnails/* 2>/dev/null".to_string(),
                safety: SafetyLevel::Safe,
                estimated_bytes: size,
            });
        }
    }

    // Trash
    if let Some(&size) = scan.category_totals.get(&Category::Trash) {
        if size > 0 {
            actions.push(CleanupAction {
                description: "Empty trash".to_string(),
                command: "rm -rf ~/.local/share/Trash/files/* \
                          ~/.local/share/Trash/info/* 2>/dev/null"
                    .to_string(),
                safety: SafetyLevel::Safe,
                estimated_bytes: size,
            });
        }
    }

    // Crash dumps
    if let Some(&size) = scan.category_totals.get(&Category::CrashDumps) {
        if size > 0 {
            actions.push(CleanupAction {
                description: "Remove crash dumps and core dumps".to_string(),
                command: "sudo rm -rf /var/crash/* /var/lib/systemd/coredump/* 2>/dev/null"
                    .to_string(),
                safety: SafetyLevel::Safe,
                estimated_bytes: size,
            });
        }
    }

    // Old snap revisions - use the more accurate snap list-based detection
    if !old_snaps.is_empty() && distro.has_snap {
        let total_snap_size: u64 = old_snaps.iter().map(|r| r.size).sum();
        actions.push(CleanupAction {
            description: format!("Remove {} old snap revisions", old_snaps.len()),
            command: "snap list --all | awk '/disabled/{print $1, $3}' | \
                      while read snap rev; do sudo snap remove \"$snap\" --revision=\"$rev\"; done"
                .to_string(),
            safety: SafetyLevel::Review,
            estimated_bytes: total_snap_size,
        });
    } else if let Some(&size) = scan.category_totals.get(&Category::SnapRevisions) {
        // Fallback to file-based detection
        if size > 0 && distro.has_snap {
            actions.push(CleanupAction {
                description: "Remove disabled snap revisions".to_string(),
                command: "snap list --all | awk '/disabled/{print $1, $3}' | \
                          while read snap rev; do sudo snap remove \"$snap\" --revision=\"$rev\"; done"
                    .to_string(),
                safety: SafetyLevel::Review,
                estimated_bytes: size,
            });
        }
    }

    // Flatpak
    if let Some(&size) = scan.category_totals.get(&Category::Flatpak) {
        if size > 0 && distro.has_flatpak {
            actions.push(CleanupAction {
                description: "Remove unused Flatpak runtimes".to_string(),
                command: "flatpak uninstall --unused -y".to_string(),
                safety: SafetyLevel::Review,
                estimated_bytes: size,
            });
        }
    }

    // Logs
    if let Some(&size) = scan.category_totals.get(&Category::Logs) {
        if size > 100 * 1024 * 1024 {
            // Only if > 100MB
            actions.push(CleanupAction {
                description: "Rotate and compress old logs".to_string(),
                command: "sudo find /var/log -name '*.gz' -delete 2>/dev/null; \
                          sudo find /var/log -name '*.[0-9]' -delete 2>/dev/null; \
                          sudo find /var/log -name '*.old' -delete 2>/dev/null"
                    .to_string(),
                safety: SafetyLevel::Safe,
                estimated_bytes: size / 3,
            });
        }
    }

    // Sort by estimated bytes descending
    actions.sort_by(|a, b| b.estimated_bytes.cmp(&a.estimated_bytes));
    actions
}

pub fn execute_cleanup(action: &CleanupAction, dry_run: bool) -> Result<(bool, String)> {
    if dry_run {
        return Ok((true, format!("[DRY RUN] {}", action.command)));
    }

    let output = Command::new("sh").arg("-c").arg(&action.command).output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = if stderr.is_empty() {
        stdout.to_string()
    } else {
        format!("{}\n{}", stdout, stderr)
    };

    Ok((output.status.success(), combined))
}

pub fn run_auto_clean(args: &Args) -> Result<()> {
    use std::io::Write;

    eprint!("Scanning {} ...", args.path);
    std::io::stderr().flush()?;

    let mut file_count = 0u64;
    let scan = scanner::scan(
        Path::new(&args.path),
        args.min_size_mb * 1024 * 1024,
        args.max_depth,
        Some(&mut |count| {
            file_count = count;
            eprint!("\rScanning {} ... {} files", args.path, count);
            let _ = std::io::stderr().flush();
        }),
    )?;
    eprintln!("\rScanning {} ... {} files done.", args.path, file_count);

    let distro = distro::detect_distro();

    eprintln!("Detected: {} {}", distro.name, distro.version);
    eprintln!("Package manager: {}", distro.pkg_manager.label());

    // Detect old snap revisions
    let old_snaps = if distro.has_snap {
        let snaps = scanner::detect_old_snap_revisions();
        if !snaps.is_empty() {
            eprintln!("Old snap revisions: {}", snaps.len());
        }
        snaps
    } else {
        Vec::new()
    };
    eprintln!();

    let actions = plan_cleanups(&scan, &distro, &old_snaps);
    let safe_actions: Vec<_> = actions
        .iter()
        .filter(|a| a.safety == SafetyLevel::Safe)
        .collect();

    if safe_actions.is_empty() {
        eprintln!("No safe cleanup actions found.");
        return Ok(());
    }

    let total: u64 = safe_actions.iter().map(|a| a.estimated_bytes).sum();
    eprintln!(
        "Found {} safe cleanup actions (~{} reclaimable)",
        safe_actions.len(),
        scanner::format_bytes(total)
    );
    eprintln!();

    for action in &safe_actions {
        eprintln!("[SAFE] {}", action.description);
        eprintln!("  Command: {}", action.command);
        eprintln!(
            "  Estimated: {}",
            scanner::format_bytes(action.estimated_bytes)
        );

        let (success, output) = execute_cleanup(action, args.dry_run)?;
        if args.dry_run {
            eprintln!("  {}", output);
        } else if success {
            eprintln!("  Result: OK");
        } else {
            eprintln!("  Result: FAILED - {}", output.trim());
        }
        eprintln!();
    }

    // Show review actions too
    let review_actions: Vec<_> = actions
        .iter()
        .filter(|a| a.safety == SafetyLevel::Review)
        .collect();
    if !review_actions.is_empty() {
        eprintln!("Additional items requiring review:");
        for action in &review_actions {
            eprintln!(
                "  [REVIEW] {} (~{})",
                action.description,
                scanner::format_bytes(action.estimated_bytes)
            );
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::safety::SafetyLevel;
    use crate::scanner::{self, Category, ScanResult};
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn make_test_scan() -> ScanResult {
        let mut category_totals = HashMap::new();
        category_totals.insert(Category::PackageCache, 100 * 1024 * 1024); // 100MB
        category_totals.insert(Category::TempFiles, 50 * 1024 * 1024); // 50MB
        category_totals.insert(Category::SystemdJournals, 30 * 1024 * 1024); // 30MB
        category_totals.insert(Category::Docker, 200 * 1024 * 1024); // 200MB
        category_totals.insert(Category::BrowserCache, 80 * 1024 * 1024); // 80MB
        category_totals.insert(Category::SnapRevisions, 500 * 1024 * 1024); // 500MB

        let mut category_counts = HashMap::new();
        category_counts.insert(Category::PackageCache, 100);
        category_counts.insert(Category::TempFiles, 50);

        ScanResult {
            entries: vec![],
            category_totals,
            category_counts,
            total_size: 1_010_000_000,
            disk_info: vec![],
            scan_path: PathBuf::from("/"),
            files_scanned: 1000,
            toplevel_sizes: vec![],
        }
    }

    fn make_test_distro(has_snap: bool, has_docker: bool, has_flatpak: bool) -> DistroInfo {
        DistroInfo {
            name: "Ubuntu".to_string(),
            version: "22.04".to_string(),
            pkg_manager: crate::distro::PkgManager::Apt,
            has_snap,
            has_docker,
            has_flatpak,
            has_journal: true,
        }
    }

    #[test]
    fn test_plan_cleanups_generates_package_cache_action() {
        let scan = make_test_scan();
        let distro = make_test_distro(false, false, false);
        let actions = plan_cleanups(&scan, &distro, &[]);

        assert!(actions
            .iter()
            .any(|a| a.description.contains("package cache")));
    }

    #[test]
    fn test_plan_cleanups_generates_temp_files_action() {
        let scan = make_test_scan();
        let distro = make_test_distro(false, false, false);
        let actions = plan_cleanups(&scan, &distro, &[]);

        assert!(actions.iter().any(|a| a.description.contains("temp files")));
    }

    #[test]
    fn test_plan_cleanups_generates_journal_action() {
        let scan = make_test_scan();
        let distro = make_test_distro(false, false, false);
        let actions = plan_cleanups(&scan, &distro, &[]);

        assert!(actions.iter().any(|a| a.description.contains("journal")));
    }

    #[test]
    fn test_plan_cleanups_generates_docker_action_when_docker_present() {
        let scan = make_test_scan();
        let distro = make_test_distro(false, true, false);
        let actions = plan_cleanups(&scan, &distro, &[]);

        assert!(actions.iter().any(|a| a.description.contains("Docker")));
    }

    #[test]
    fn test_plan_cleanups_no_docker_action_when_docker_absent() {
        let scan = make_test_scan();
        let distro = make_test_distro(false, false, false);
        let actions = plan_cleanups(&scan, &distro, &[]);

        assert!(!actions.iter().any(|a| a.description.contains("Docker")));
    }

    #[test]
    fn test_plan_cleanups_generates_browser_cache_action() {
        let scan = make_test_scan();
        let distro = make_test_distro(false, false, false);
        let actions = plan_cleanups(&scan, &distro, &[]);

        assert!(actions.iter().any(|a| a.description.contains("browser")));
    }

    #[test]
    fn test_plan_cleanups_with_old_snaps() {
        let scan = make_test_scan();
        let distro = make_test_distro(true, false, false);
        let old_snaps = vec![
            scanner::SnapRevisionInfo {
                name: "firefox".to_string(),
                revision: "123".to_string(),
                size: 200 * 1024 * 1024,
            },
            scanner::SnapRevisionInfo {
                name: "code".to_string(),
                revision: "456".to_string(),
                size: 300 * 1024 * 1024,
            },
        ];

        let actions = plan_cleanups(&scan, &distro, &old_snaps);

        let snap_action = actions.iter().find(|a| a.description.contains("snap"));
        assert!(snap_action.is_some());
        assert_eq!(snap_action.unwrap().estimated_bytes, 500 * 1024 * 1024);
        assert_eq!(snap_action.unwrap().safety, SafetyLevel::Review);
    }

    #[test]
    fn test_plan_cleanups_actions_sorted_by_size() {
        let scan = make_test_scan();
        let distro = make_test_distro(false, false, false);
        let actions = plan_cleanups(&scan, &distro, &[]);

        // Actions should be sorted by estimated_bytes descending
        for i in 1..actions.len() {
            assert!(actions[i - 1].estimated_bytes >= actions[i].estimated_bytes);
        }
    }

    #[test]
    fn test_execute_cleanup_dry_run() {
        let action = CleanupAction {
            description: "Test action".to_string(),
            command: "echo test".to_string(),
            safety: SafetyLevel::Safe,
            estimated_bytes: 1024,
        };

        let (success, output) = execute_cleanup(&action, true).unwrap();
        assert!(success);
        assert!(output.contains("[DRY RUN]"));
    }

    #[test]
    fn test_execute_cleanup_actual_run() {
        let action = CleanupAction {
            description: "Test action".to_string(),
            command: "echo hello".to_string(),
            safety: SafetyLevel::Safe,
            estimated_bytes: 1024,
        };

        let (success, output) = execute_cleanup(&action, false).unwrap();
        assert!(success);
        assert!(output.contains("hello"));
    }

    #[test]
    fn test_cleanup_action_safety_levels() {
        let scan = make_test_scan();
        let distro = make_test_distro(true, true, false);
        let actions = plan_cleanups(&scan, &distro, &[]);

        // Package cache, temp files, journals should be Safe
        let safe_actions: Vec<_> = actions
            .iter()
            .filter(|a| a.safety == SafetyLevel::Safe)
            .collect();
        assert!(
            !safe_actions.is_empty(),
            "Should have at least one safe action"
        );

        // Verify package cache action is safe
        let pkg_action = actions
            .iter()
            .find(|a| a.description.contains("package cache"));
        assert!(pkg_action.is_some());
        assert_eq!(pkg_action.unwrap().safety, SafetyLevel::Safe);
    }
}
