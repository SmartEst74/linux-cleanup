mod cleanup;
mod distro;
mod safety;
mod scanner;
mod tui;

use anyhow::Result;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "linux-cleanup",
    about = "Friendly Linux disk cleanup utility",
    long_about = "Scans your system for reclaimable disk space and presents findings\n\
                  in an interactive TUI with categorized cleanup options.\n\n\
                  Safe items are auto-identified. Review items need your attention.\n\
                  Press Space to toggle, Enter to execute selected cleanups."
)]
pub struct Args {
    /// Scan path (default: /)
    #[arg(short, long, default_value = "/")]
    pub path: String,

    /// Skip interactive TUI, run safe cleanups automatically
    #[arg(long)]
    pub auto_clean: bool,

    /// Dry run — show what would be cleaned without doing it
    #[arg(short, long)]
    pub dry_run: bool,

    /// Minimum file size in MB to include in scan results
    #[arg(short, long, default_value = "1")]
    pub min_size_mb: u64,

    /// Maximum scan depth (directories deep)
    #[arg(long, default_value = "5")]
    pub max_depth: usize,

    /// CLI-only mode: scan and print results, no TUI
    #[arg(long)]
    pub scan_only: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    if args.auto_clean {
        return cleanup::run_auto_clean(&args);
    }

    if args.scan_only {
        return run_scan_only(&args);
    }

    tui::run(&args)
}

fn run_scan_only(args: &Args) -> Result<()> {
    let spinner = indicatif::ProgressBar::new_spinner();
    spinner.set_style(
        indicatif::ProgressStyle::default_spinner()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("{spinner} {msg}")?,
    );
    spinner.set_message(format!("Scanning {} ...", args.path));
    spinner.enable_steady_tick(std::time::Duration::from_millis(80));

    let mut files_scanned: u64 = 0;
    let scan = scanner::scan(
        std::path::Path::new(&args.path),
        args.min_size_mb * 1024 * 1024,
        args.max_depth,
        Some(&mut |count| {
            files_scanned = count;
            spinner.set_message(format!("Scanning {} ... ({} files)", args.path, count));
        }),
    )?;
    spinner.finish_and_clear();

    let distro = distro::detect_distro();
    let disks = scanner::get_disk_info();

    println!("System: {} {}", distro.name, distro.version);
    println!("Package manager: {}", distro.pkg_manager.label());
    println!("Files scanned: {}", files_scanned);
    println!();

    for disk in &disks {
        let pct = if disk.total > 0 {
            (disk.used as f64 / disk.total as f64 * 100.0) as u64
        } else {
            0
        };
        println!(
            "  {} {}% ({}/{})",
            disk.mount_point.display(),
            pct,
            scanner::format_bytes(disk.used),
            scanner::format_bytes(disk.total)
        );
    }
    println!();

    // Category breakdown
    println!("Cleanup Categories:");
    println!(
        "  {:<6} {:<25} {:>12}  {}",
        "Safety", "Category", "Size", "Hint"
    );
    println!("  {}", "─".repeat(75));

    let mut sorted_cats: Vec<_> = scan.category_totals.iter().collect();
    sorted_cats.sort_by(|a, b| b.1.cmp(a.1));

    for (cat, &size) in &sorted_cats {
        if size == 0 {
            continue;
        }
        let safety = safety::classify_safety(cat, cat.label());
        let hint = safety::get_cleanup_hint(cat);
        println!(
            "  {:<6} {:<25} {:>12}  {}",
            safety.label(),
            cat.label(),
            scanner::format_bytes(size),
            hint
        );
    }
    println!();

    // Top items
    let top_n = 20;
    println!("Largest Items (top {}):", top_n);
    println!("  {:>12}  {:<6}  {}", "Size", "Safety", "Path");
    println!("  {}", "─".repeat(80));

    for entry in scan.entries.iter().take(top_n) {
        let safety = safety::classify_safety(&entry.category, &entry.path.to_string_lossy());
        let path_str = entry.path.to_string_lossy();
        let short = if path_str.len() > 60 {
            format!("...{}", &path_str[path_str.len() - 57..])
        } else {
            path_str.to_string()
        };
        println!(
            "  {:>12}  {:<6}  {}",
            scanner::format_bytes(entry.size),
            safety.label(),
            short
        );
    }

    // Show cleanup actions
    let old_snaps = if distro.has_snap {
        scanner::detect_old_snap_revisions()
    } else {
        Vec::new()
    };
    if !old_snaps.is_empty() {
        println!();
        println!("Old Snap Revisions:");
        let snap_total: u64 = old_snaps.iter().map(|r| r.size).sum();
        println!(
            "  {} revisions ({})",
            old_snaps.len(),
            scanner::format_bytes(snap_total)
        );
        for snap in old_snaps.iter().take(5) {
            println!(
                "    {} (rev {}, {})",
                snap.name,
                snap.revision,
                scanner::format_bytes(snap.size)
            );
        }
        if old_snaps.len() > 5 {
            println!("    ... and {} more", old_snaps.len() - 5);
        }
    }

    let actions = cleanup::plan_cleanups(&scan, &distro, &old_snaps);
    if !actions.is_empty() {
        println!();
        println!("Planned Cleanup Actions:");
        println!("  {:<6} {:<40} {:>12}", "Safety", "Action", "Estimated");
        println!("  {}", "─".repeat(60));
        for action in &actions {
            println!(
                "  {:<6} {:<40} {:>12}",
                action.safety.label(),
                action.description,
                scanner::format_bytes(action.estimated_bytes)
            );
        }
    }

    Ok(())
}
