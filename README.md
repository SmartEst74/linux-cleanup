# Linux Cleanup

A Rust-based Linux disk cleanup utility with an interactive TUI and native GUI window, packaged as a snap for distribution.

## Features

- Visual disk space analysis with pie charts and progress bars
- Categorized cleanup items: SAFE, REVIEW, RISKY
- Auto-detection of distro and package manager
- Support for apt, dnf, pacman, zypper, yum
- Snap, Flatpak, Docker cleanup detection
- Old kernel and log rotation detection
- Interactive TUI with toggle selection
- Native GUI window (egui) for desktop users
- CLI mode for scripting and automation
- Dry-run mode for safe preview

## Build from Source

### Requirements

- Rust 1.70+ (with cargo)
- For full system scan: `sudo` privileges

### Build Commands

```bash
# Clone the repo
git clone https://github.com/SmartEst74/linux-cleanup.git
cd linux-cleanup

# Build release binary
cargo build --release

# Run all tests (41 tests)
cargo test

# Run the native GUI window
sudo ./target/release/linux-cleanup --gui

# Run the TUI (terminal interface)
sudo ./target/release/linux-cleanup

# CLI scan report (no GUI)
./target/release/linux-cleanup --scan-only --path /

# Preview cleanups (dry run)
./target/release/linux-cleanup --auto-clean --dry-run
```

## Building the Snap Package

Snap configuration lives in the `snap/` directory:

```
snap/
├── snapcraft.yaml       # Snap build configuration
└── gui/
    ├── icon.svg         # App icon
    └── linux-cleanup.desktop  # Desktop entry
```

### Build the Snap

```bash
# Install snapcraft (if not already installed)
sudo snap install snapcraft --classic

# Build the snap (from project root)
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

## Project Structure

```
linux-cleanup/
├── AGENTS.md            # AI agent instructions
├── README.md            # This file
├── Cargo.toml           # Rust dependencies
├── snap/
│   ├── snapcraft.yaml   # Snap packaging config
│   └── gui/
│       ├── icon.svg
│       └── linux-cleanup.desktop
└── src/
    ├── main.rs          # CLI entry point (clap)
    ├── scanner.rs       # Filesystem scanning, path classification
    ├── safety.rs        # Safety level classification
    ├── cleanup.rs       # Cleanup action planning and execution
    ├── distro.rs        # Distro/package manager detection
    └── tui/
        ├── mod.rs       # TUI entry, App state, event loop
        ├── dashboard.rs # 4-tab dashboard
        ├── piechart.rs  # Pie chart widget
        └── tree.rs      # File tree widget
```

## TUI Controls

| Key | Action |
|-----|--------|
| Tab / 1/2/3/4 | Switch tabs |
| j / k or ↑/↓ | Scroll |
| Space | Toggle selection |
| a | Select all safe items |
| Enter | Execute selected cleanups |
| q | Quit |

## Contact & Support

- **Email**: linux_cleanup@it1st.com
- **Issues**: https://github.com/SmartEst74/linux-cleanup/issues
- **Source**: https://github.com/SmartEst74/linux-cleanup

## License

MIT
