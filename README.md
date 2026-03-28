# Linux Cleanup

A Rust-based Linux disk cleanup utility with an interactive TUI interface, packaged as a snap for distribution.

## Features

- Visual disk space analysis with pie charts
- Categorized cleanup items: SAFE, REVIEW, RISKY
- Auto-detection of distro and package manager
- Support for apt, dnf, pacman, zypper, yum
- Snap, Flatpak, Docker cleanup detection
- Old kernel and log rotation detection
- Interactive TUI with toggle selection
- CLI mode for scripting and automation
- Dry-run mode for safe preview

## Build from Source

### Requirements

- Rust 1.70+ (with cargo)
- For full system scan: `sudo` privileges
- For snap build: `snapcraft` installed

### Build Commands

```bash
# Clone the repo
git clone https://github.com/SmartEst74/linux-cleanup.git
cd linux-cleanup

# Build release binary
cargo build --release

# Run all tests (41 tests)
cargo test

# Run the TUI
sudo ./target/release/linux-cleanup

# CLI scan report (no TUI)
./target/release/linux-cleanup --scan-only --path /

# Preview cleanups (dry run)
./target/release/linux-cleanup --auto-clean --dry-run
```

## Building the Snap Package

```bash
# Install snapcraft (if not already installed)
sudo snap install snapcraft --classic

# Build the snap
snapcraft

# Install locally for testing
sudo snap install linux-cleanup_0.1.0_amd64.snap --dangerous

# Run the snap
sudo linux-cleanup
```

### Snap Confinement

The snap uses `classic` confinement because it needs full filesystem access to:
- Scan all directories for cleanup candidates
- Remove files from system locations
- Access package manager caches

## TUI Controls

| Key | Action |
|-----|--------|
| Tab / 1/2/3/4 | Switch tabs |
| j / k or ↑/↓ | Scroll |
| Space | Toggle selection |
| a | Select all safe items |
| Enter | Execute selected cleanups |
| q | Quit |

## Cleanup Categories

| Category | Safety | Description |
|----------|--------|-------------|
| Package Caches | SAFE | apt/dnf/pacman/zypper caches |
| Systemd Journals | SAFE | Old journal logs |
| Temporary Files | SAFE | /tmp, /var/tmp |
| Docker Artifacts | SAFE | Dangling images, build cache |
| Browser Caches | SAFE | Chrome, Firefox, Brave, etc. |
| User Caches | SAFE | ~/.cache contents |
| Trash | SAFE | Deleted files |
| Crash Dumps | SAFE | Core dumps, apport reports |
| Old Kernels | REVIEW | Keep current + 1 previous |
| Snap Revisions | REVIEW | Disabled snap revisions |
| Flatpak Leftovers | REVIEW | Unused runtimes |
| Build Artifacts | REVIEW | target/, node_modules/ |
| Large Files | REVIEW | User-defined large files |
| Thumbnail Cache | SAFE | ~/.cache/thumbnails |
| Shared Memory | SAFE | /dev/shm contents |

## Architecture

```
src/
├── main.rs           # CLI entry point (clap)
├── scanner.rs        # Filesystem scanning, path classification
├── safety.rs         # Safety level classification
├── cleanup.rs        # Cleanup action planning and execution
├── distro.rs         # Distro/package manager detection
└── tui/
    ├── mod.rs        # TUI entry, App state, event loop
    ├── dashboard.rs  # 4-tab dashboard
    ├── piechart.rs   # Pie chart widget
    └── tree.rs       # File tree widget
```

## License

MIT
