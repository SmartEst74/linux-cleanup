use crate::scanner::Category;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SafetyLevel {
    Safe,
    Review,
    Dangerous,
}

impl SafetyLevel {
    pub fn label(&self) -> &'static str {
        match self {
            SafetyLevel::Safe => "SAFE",
            SafetyLevel::Review => "REVIEW",
            SafetyLevel::Dangerous => "RISKY",
        }
    }

    pub fn short_label(&self) -> &'static str {
        match self {
            SafetyLevel::Safe => "S",
            SafetyLevel::Review => "R",
            SafetyLevel::Dangerous => "X",
        }
    }

    pub fn color(&self) -> &'static str {
        match self {
            SafetyLevel::Safe => "green",
            SafetyLevel::Review => "yellow",
            SafetyLevel::Dangerous => "red",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            SafetyLevel::Safe => "Safe to clean - will not affect system operation",
            SafetyLevel::Review => "Review recommended - check details before cleaning",
            SafetyLevel::Dangerous => {
                "Potentially risky - clean only if you know what you are doing"
            }
        }
    }
}

pub fn classify_safety(category: &Category, path: &str) -> SafetyLevel {
    let path_lower = path.to_lowercase();

    match category {
        Category::PackageCache => SafetyLevel::Safe,
        Category::SystemdJournals => SafetyLevel::Safe,
        Category::TempFiles => {
            if path_lower.contains("/var/tmp") {
                SafetyLevel::Review
            } else {
                SafetyLevel::Safe
            }
        }
        Category::Docker => SafetyLevel::Safe,
        Category::Logs => {
            if path_lower.contains(".gz")
                || path_lower.contains(".1")
                || path_lower.contains(".old")
                || path_lower.contains("/journal/")
            {
                SafetyLevel::Safe
            } else {
                SafetyLevel::Review
            }
        }
        Category::UserCache => SafetyLevel::Safe,
        Category::BrowserCache => SafetyLevel::Safe,
        Category::Trash => SafetyLevel::Safe,
        Category::CrashDumps => SafetyLevel::Safe,
        Category::OldKernels => SafetyLevel::Review,
        Category::SnapRevisions => SafetyLevel::Review,
        Category::Flatpak => SafetyLevel::Review,
        Category::BuildArtifacts => SafetyLevel::Review,
        Category::LargeFiles => SafetyLevel::Review,
        Category::ThumbnailCache => SafetyLevel::Safe,
        Category::DevShm => SafetyLevel::Safe,
        Category::Other(_) => SafetyLevel::Dangerous,
    }
}

pub fn get_cleanup_hint(category: &Category) -> &'static str {
    match category {
        Category::PackageCache => "apt clean / dnf clean all / pacman -Sc",
        Category::SystemdJournals => "journalctl --vacuum-size=100M",
        Category::TempFiles => "Remove files older than 3 days from /tmp, /var/tmp",
        Category::Docker => "docker system prune -f",
        Category::Logs => "Rotate or truncate oversized log files",
        Category::UserCache => "Clear ~/.cache contents older than 7 days",
        Category::BrowserCache => "Clear browser cache directories",
        Category::Trash => "Empty the trash",
        Category::CrashDumps => "Remove crash reports and core dumps",
        Category::OldKernels => "Remove old kernel versions (keep current + 1)",
        Category::SnapRevisions => "Remove old snap revisions: snap list --all",
        Category::Flatpak => "Remove unused flatpak runtimes: flatpak uninstall --unused",
        Category::BuildArtifacts => "Clean build dirs (target/, node_modules/)",
        Category::LargeFiles => "Review and manually delete large unused files",
        Category::ThumbnailCache => "Clear thumbnail cache",
        Category::DevShm => "Clear shared memory segments",
        Category::Other(_) => "Review manually before cleaning",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::Category;

    #[test]
    fn test_safety_level_ordering() {
        assert!(SafetyLevel::Safe < SafetyLevel::Review);
        assert!(SafetyLevel::Review < SafetyLevel::Dangerous);
    }

    #[test]
    fn test_classify_safety_safe_items() {
        assert_eq!(
            classify_safety(&Category::PackageCache, "/var/cache/apt"),
            SafetyLevel::Safe
        );
        assert_eq!(
            classify_safety(&Category::TempFiles, "/tmp/file"),
            SafetyLevel::Safe
        );
        assert_eq!(
            classify_safety(&Category::Trash, "/home/user/.local/share/Trash"),
            SafetyLevel::Safe
        );
        assert_eq!(
            classify_safety(&Category::CrashDumps, "/var/crash/dump"),
            SafetyLevel::Safe
        );
        assert_eq!(
            classify_safety(&Category::UserCache, "/home/.cache/app"),
            SafetyLevel::Safe
        );
        assert_eq!(
            classify_safety(&Category::BrowserCache, "/home/.cache/chrome"),
            SafetyLevel::Safe
        );
        assert_eq!(
            classify_safety(&Category::ThumbnailCache, "/home/.cache/thumbnails"),
            SafetyLevel::Safe
        );
        assert_eq!(
            classify_safety(&Category::DevShm, "/dev/shm/shared"),
            SafetyLevel::Safe
        );
    }

    #[test]
    fn test_classify_safety_review_items() {
        assert_eq!(
            classify_safety(&Category::OldKernels, "/boot/vmlinuz"),
            SafetyLevel::Review
        );
        assert_eq!(
            classify_safety(&Category::SnapRevisions, "/var/lib/snapd/snaps"),
            SafetyLevel::Review
        );
        assert_eq!(
            classify_safety(&Category::Flatpak, "/var/lib/flatpak"),
            SafetyLevel::Review
        );
        assert_eq!(
            classify_safety(&Category::BuildArtifacts, "/project/target"),
            SafetyLevel::Review
        );
        assert_eq!(
            classify_safety(&Category::LargeFiles, "/home/user/bigfile"),
            SafetyLevel::Review
        );
    }

    #[test]
    fn test_classify_safety_var_tmp_review() {
        assert_eq!(
            classify_safety(&Category::TempFiles, "/var/tmp/file"),
            SafetyLevel::Review
        );
    }

    #[test]
    fn test_classify_safety_dangerous() {
        assert_eq!(
            classify_safety(&Category::Other("random".to_string()), "/some/path"),
            SafetyLevel::Dangerous
        );
    }

    #[test]
    fn test_classify_safety_logs() {
        assert_eq!(
            classify_safety(&Category::Logs, "/var/log/syslog.gz"),
            SafetyLevel::Safe
        );
        assert_eq!(
            classify_safety(&Category::Logs, "/var/log/syslog.1"),
            SafetyLevel::Safe
        );
        assert_eq!(
            classify_safety(&Category::Logs, "/var/log/syslog"),
            SafetyLevel::Review
        );
    }

    #[test]
    fn test_safety_labels() {
        assert_eq!(SafetyLevel::Safe.label(), "SAFE");
        assert_eq!(SafetyLevel::Review.label(), "REVIEW");
        assert_eq!(SafetyLevel::Dangerous.label(), "RISKY");
    }

    #[test]
    fn test_safety_short_labels() {
        assert_eq!(SafetyLevel::Safe.short_label(), "S");
        assert_eq!(SafetyLevel::Review.short_label(), "R");
        assert_eq!(SafetyLevel::Dangerous.short_label(), "X");
    }

    #[test]
    fn test_safety_color() {
        assert_eq!(SafetyLevel::Safe.color(), "green");
        assert_eq!(SafetyLevel::Review.color(), "yellow");
        assert_eq!(SafetyLevel::Dangerous.color(), "red");
    }

    #[test]
    fn test_safety_description() {
        assert!(!SafetyLevel::Safe.description().is_empty());
        assert!(!SafetyLevel::Review.description().is_empty());
        assert!(!SafetyLevel::Dangerous.description().is_empty());
        assert!(SafetyLevel::Safe.description().contains("Safe"));
    }

    #[test]
    fn test_cleanup_hints_not_empty() {
        for cat in [
            Category::PackageCache,
            Category::SystemdJournals,
            Category::TempFiles,
            Category::Docker,
            Category::Logs,
            Category::UserCache,
            Category::BrowserCache,
            Category::Trash,
            Category::CrashDumps,
            Category::OldKernels,
            Category::SnapRevisions,
            Category::Flatpak,
            Category::BuildArtifacts,
            Category::LargeFiles,
            Category::ThumbnailCache,
            Category::DevShm,
        ] {
            let hint = get_cleanup_hint(&cat);
            assert!(!hint.is_empty(), "Hint for {:?} should not be empty", cat);
        }
    }
}
