use std::process::Command;

#[derive(Debug, Clone, PartialEq)]
pub enum Distro {
    Debian,
    Ubuntu,
    Fedora,
    RHEL,
    Arch,
    OpenSUSE,
    Unknown(String),
}

#[derive(Debug, Clone)]
pub struct DistroInfo {
    pub name: String,
    pub version: String,
    pub pkg_manager: PkgManager,
    pub has_snap: bool,
    pub has_flatpak: bool,
    pub has_docker: bool,
    pub has_journal: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PkgManager {
    Apt,
    Dnf,
    Yum,
    Pacman,
    Zypper,
    Unknown,
}

impl PkgManager {
    pub fn cache_clean_cmd(&self) -> Option<&'static str> {
        match self {
            PkgManager::Apt => Some("sudo apt clean && sudo apt autoremove -y"),
            PkgManager::Dnf => Some("sudo dnf clean all"),
            PkgManager::Yum => Some("sudo yum clean all"),
            PkgManager::Pacman => Some("sudo pacman -Sc --noconfirm"),
            PkgManager::Zypper => Some("sudo zypper clean --all"),
            PkgManager::Unknown => None,
        }
    }

    pub fn label(&self) -> &str {
        match self {
            PkgManager::Apt => "apt",
            PkgManager::Dnf => "dnf",
            PkgManager::Yum => "yum",
            PkgManager::Pacman => "pacman",
            PkgManager::Zypper => "zypper",
            PkgManager::Unknown => "unknown",
        }
    }
}

pub fn detect_distro() -> DistroInfo {
    let (name, version) = parse_os_release();
    let distro = identify_distro(&name);
    let pkg_manager = detect_pkg_manager(&distro);

    DistroInfo {
        name,
        version,
        pkg_manager,
        has_snap: cmd_exists("snap"),
        has_flatpak: cmd_exists("flatpak"),
        has_docker: cmd_exists("docker"),
        has_journal: cmd_exists("journalctl"),
    }
}

fn parse_os_release() -> (String, String) {
    let content = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
    let mut name = String::from("Unknown");
    let mut version = String::from("Unknown");

    for line in content.lines() {
        if let Some(val) = line.strip_prefix("NAME=") {
            name = val.trim_matches('"').to_string();
        }
        if let Some(val) = line.strip_prefix("VERSION_ID=") {
            version = val.trim_matches('"').to_string();
        }
    }

    (name, version)
}

fn identify_distro(name: &str) -> Distro {
    let lower = name.to_lowercase();
    if lower.contains("ubuntu") {
        Distro::Ubuntu
    } else if lower.contains("debian") {
        Distro::Debian
    } else if lower.contains("fedora") {
        Distro::Fedora
    } else if lower.contains("rhel") || lower.contains("red hat") {
        Distro::RHEL
    } else if lower.contains("arch") || lower.contains("manjaro") {
        Distro::Arch
    } else if lower.contains("suse") || lower.contains("opensuse") {
        Distro::OpenSUSE
    } else {
        Distro::Unknown(name.to_string())
    }
}

fn detect_pkg_manager(distro: &Distro) -> PkgManager {
    match distro {
        Distro::Ubuntu | Distro::Debian => PkgManager::Apt,
        Distro::Fedora => PkgManager::Dnf,
        Distro::RHEL => {
            if cmd_exists("dnf") {
                PkgManager::Dnf
            } else {
                PkgManager::Yum
            }
        }
        Distro::Arch => PkgManager::Pacman,
        Distro::OpenSUSE => PkgManager::Zypper,
        Distro::Unknown(_) => {
            if cmd_exists("apt") {
                PkgManager::Apt
            } else if cmd_exists("dnf") {
                PkgManager::Dnf
            } else if cmd_exists("pacman") {
                PkgManager::Pacman
            } else if cmd_exists("zypper") {
                PkgManager::Zypper
            } else if cmd_exists("yum") {
                PkgManager::Yum
            } else {
                PkgManager::Unknown
            }
        }
    }
}

fn cmd_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
