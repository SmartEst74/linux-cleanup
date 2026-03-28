# Linux Cleanup

A Rust-based Linux disk cleanup utility with an interactive TUI interface, packaged as a snap for distribution. The tool scans the filesystem, categorizes files by cleanup type, and provides safe/review/dangerous safety classifications for cleanup actions.

## Build & Test Commands

```bash
cargo build --release    # Release build
cargo test              # Run all 41 tests
cargo build             # Debug build (faster)
snapcraft               # Build snap package
```

## Run Commands

```bash
sudo ./target/release/linux-cleanup           # Interactive TUI
./target/release/linux-cleanup --scan-only     # CLI report
./target/release/linux-cleanup --auto-clean --dry-run  # Preview cleanups
```

## Architecture

- `src/main.rs` - CLI entry point with clap argument parsing
- `src/scanner.rs` - Filesystem scanning, path classification into 16 categories
- `src/safety.rs` - Safety level classification (Safe/Review/Dangerous)
- `src/cleanup.rs` - Cleanup action planning and execution
- `src/distro.rs` - Distro/package manager auto-detection
- `src/tui/` - Ratatui-based TUI with 4 tabs (Overview, Clean Up, Browse, Details)

## Code Conventions

- Use `anyhow::Result` for error handling
- Scanner categories are exhaustive enum variants in `Category`
- Safety levels: `Safe` (green, auto-clean), `Review` (yellow, confirm), `Dangerous` (red, manual only)
- All public types derive `Debug, Clone`
- Tests use `#[cfg(test)] mod tests` pattern within each module

## Cleanup Categories (16 types)

PackageCache, SystemdJournals, TempFiles, Docker, Logs, UserCache, BrowserCache, Trash, CrashDumps, OldKernels, SnapRevisions, Flatpak, BuildArtifacts, LargeFiles, ThumbnailCache, DevShm

## Adding New Categories

1. Add variant to `Category` enum in `scanner.rs`
2. Add `label()` match arm
3. Add classification logic in `classify_path()`
4. Add safety classification in `safety.rs` `classify_safety()`
5. Add cleanup action in `cleanup.rs` `plan_cleanups()`
6. Add tests for path matching and safety level

## Key Patterns

- Path classification is case-insensitive (convert to lowercase first)
- Pseudo-filesystems (/proc, /sys, /dev, /run) are excluded from scanning
- Snap revisions detected via `snap list --all` command output parsing
- Distro detection via `/etc/os-release` and `which` for package managers
