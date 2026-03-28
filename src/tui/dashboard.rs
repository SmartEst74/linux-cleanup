use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs, Wrap},
    Frame,
};

use super::piechart::PieChart;
use super::{App, AppPhase, TABS};
use crate::safety::{self, SafetyLevel};
use crate::scanner;

pub fn draw(frame: &mut Frame, app: &App) {
    match app.phase {
        AppPhase::Scanning => draw_scanning(frame),
        AppPhase::Ready | AppPhase::ConfirmClean => draw_ready(frame, app),
    }
}

fn draw_scanning(frame: &mut Frame) {
    let area = frame.area();
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Linux Cleanup ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(inner);

    let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let tick = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as usize
        / 80;
    let spinner = spinner_chars[tick % spinner_chars.len()];

    let text = Paragraph::new(Line::from(vec![
        Span::styled(format!("{} ", spinner), Style::default().fg(Color::Cyan)),
        Span::raw("Scanning filesystem..."),
    ]))
    .alignment(ratatui::layout::Alignment::Center);

    frame.render_widget(text, chunks[1]);
}

fn draw_ready(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(frame.area());

    draw_header(frame, chunks[0], app);
    draw_body(frame, chunks[1], app);
    draw_footer(frame, chunks[2], app);

    // Confirmation overlay
    if let AppPhase::ConfirmClean = app.phase {
        draw_confirm_dialog(frame, app);
    }
}

fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    let titles: Vec<Line> = TABS.iter().map(|t| Line::from(*t)).collect();
    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Linux Cleanup "),
        )
        .select(app.current_tab)
        .style(Style::default().fg(Color::Gray))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(tabs, area);
}

fn draw_body(frame: &mut Frame, area: Rect, app: &App) {
    match app.current_tab {
        0 => draw_overview(frame, area, app),
        1 => draw_cleanup(frame, area, app),
        2 => draw_browse(frame, area, app),
        3 => draw_details(frame, area, app),
        _ => {}
    }
}

// ─── Tab 1: Overview ─────────────────────────────────────────────

fn draw_overview(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(area);

    draw_system_info(frame, chunks[0], app);
    draw_pie_chart(frame, chunks[1], app);
}

fn draw_system_info(frame: &mut Frame, area: Rect, app: &App) {
    let scan = match &app.scan {
        Some(s) => s,
        None => return,
    };
    let distro = match &app.distro {
        Some(d) => d,
        None => return,
    };

    let mut lines = vec![
        Line::from(vec![
            Span::styled("System:  ", Style::default().fg(Color::Yellow)),
            Span::raw(format!("{} {}", distro.name, distro.version)),
        ]),
        Line::from(vec![
            Span::styled("Pkg Mgr: ", Style::default().fg(Color::Yellow)),
            Span::raw(distro.pkg_manager.label().to_string()),
        ]),
    ];

    // Capabilities
    let mut caps = Vec::new();
    if distro.has_docker {
        caps.push("Docker");
    }
    if distro.has_snap {
        caps.push("Snap");
    }
    if distro.has_flatpak {
        caps.push("Flatpak");
    }
    if !caps.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Tools:   ", Style::default().fg(Color::Yellow)),
            Span::raw(caps.join(", ")),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Disk Usage",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));

    for disk in scan.disk_info.iter().take(3) {
        let pct = if disk.total > 0 {
            (disk.used as f64 / disk.total as f64 * 100.0) as u64
        } else {
            0
        };
        let bar_color = if pct > 90 {
            Color::Red
        } else if pct > 70 {
            Color::Yellow
        } else {
            Color::Green
        };

        lines.push(Line::from(vec![
            Span::raw(format!("  {} ", disk.mount_point.display())),
            Span::styled(
                format!("{}%", pct),
                Style::default().fg(bar_color).add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                " ({}/{})",
                scanner::format_bytes(disk.used),
                scanner::format_bytes(disk.total)
            )),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Scan Summary",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(vec![
        Span::styled("Scanned: ", Style::default().fg(Color::Yellow)),
        Span::raw(format!(
            "{} files in {}",
            scan.files_scanned,
            scan.scan_path.display()
        )),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Found:   ", Style::default().fg(Color::Yellow)),
        Span::raw(format!("{} cleanup categories", scan.category_totals.len())),
    ]));

    let safe_bytes: u64 = app
        .actions
        .iter()
        .filter(|a| a.safety == SafetyLevel::Safe)
        .map(|a| a.estimated_bytes)
        .sum();
    if safe_bytes > 0 {
        lines.push(Line::from(vec![
            Span::styled("Safe:    ", Style::default().fg(Color::Green)),
            Span::styled(
                scanner::format_bytes(safe_bytes),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" reclaimable"),
        ]));
    }

    let review_bytes: u64 = app
        .actions
        .iter()
        .filter(|a| a.safety == SafetyLevel::Review)
        .map(|a| a.estimated_bytes)
        .sum();
    if review_bytes > 0 {
        lines.push(Line::from(vec![
            Span::styled("Review:  ", Style::default().fg(Color::Yellow)),
            Span::styled(
                scanner::format_bytes(review_bytes),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" needs attention"),
        ]));
    }

    // Show old snap revisions if any
    if !app.old_snaps.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Old Snap Revisions",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )));
        let snap_total: u64 = app.old_snaps.iter().map(|r| r.size).sum();
        lines.push(Line::from(vec![
            Span::styled("  Found: ", Style::default().fg(Color::Magenta)),
            Span::raw(format!(
                "{} revisions ({})",
                app.old_snaps.len(),
                scanner::format_bytes(snap_total)
            )),
        ]));
        // Show top 3 old snaps
        for snap in app.old_snaps.iter().take(3) {
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(&snap.name, Style::default().fg(Color::Magenta)),
                Span::raw(format!(
                    " (rev {}, {})",
                    snap.revision,
                    scanner::format_bytes(snap.size)
                )),
            ]));
        }
        if app.old_snaps.len() > 3 {
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(
                    format!("... and {} more", app.old_snaps.len() - 3),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }
    }

    let info = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(" System "))
        .wrap(Wrap { trim: false });
    frame.render_widget(info, area);
}

fn draw_pie_chart(frame: &mut Frame, area: Rect, app: &App) {
    let scan = match &app.scan {
        Some(s) => s,
        None => return,
    };

    // Show category breakdown as pie chart
    let mut cat_data: Vec<(String, u64)> = scan
        .category_totals
        .iter()
        .filter(|(_, &v)| v > 0)
        .map(|(cat, &size)| (cat.label().to_string(), size))
        .collect();
    cat_data.sort_by(|a, b| b.1.cmp(&a.1));
    cat_data.truncate(10);

    if cat_data.is_empty() {
        // Fall back to toplevel dir sizes
        let pie = PieChart::new("Directory Distribution").data(scan.toplevel_sizes.clone());
        frame.render_widget(pie, area);
    } else {
        let pie = PieChart::new("Cleanup Categories").data(cat_data);
        frame.render_widget(pie, area);
    }
}

// ─── Tab 2: Clean Up ─────────────────────────────────────────────

fn draw_cleanup(frame: &mut Frame, area: Rect, app: &App) {
    if app.actions.is_empty() {
        let msg = Paragraph::new("No cleanup actions found.\nYour system looks clean!")
            .style(Style::default().fg(Color::Green))
            .alignment(ratatui::layout::Alignment::Center)
            .block(Block::default().borders(Borders::ALL).title(" Clean Up "));
        frame.render_widget(msg, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(area);

    // Actions list
    let list_items: Vec<ListItem> = app
        .actions
        .iter()
        .enumerate()
        .map(|(i, action)| {
            let selected = app.selected_actions.get(i).copied().unwrap_or(false);
            let safety_color = match action.safety {
                SafetyLevel::Safe => Color::Green,
                SafetyLevel::Review => Color::Yellow,
                SafetyLevel::Dangerous => Color::Red,
            };

            let checkbox = if selected { "[x]" } else { "[ ]" };
            let cursor = if i == app.scroll_offset { "▸" } else { " " };

            ListItem::new(Line::from(vec![
                Span::raw(format!("{}{} ", cursor, checkbox)),
                Span::styled(
                    format!("[{}] ", action.safety.short_label()),
                    Style::default()
                        .fg(safety_color)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!("{:<45}", action.description)),
                Span::styled(
                    scanner::format_bytes(action.estimated_bytes),
                    Style::default().fg(Color::Cyan),
                ),
            ]))
        })
        .collect();

    let list = List::new(list_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Cleanup Actions (Space=toggle, a=all-safe, Enter=execute) "),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(list, chunks[0]);

    // Summary bar
    let count = app.selected_count();
    let bytes = app.selected_bytes();
    let summary = if count > 0 {
        format!(
            "{} actions selected — ~{} reclaimable",
            count,
            scanner::format_bytes(bytes)
        )
    } else {
        "Press Space to toggle items, 'a' to select all safe items".to_string()
    };

    let bar = Paragraph::new(summary)
        .style(Style::default().fg(if count > 0 { Color::Cyan } else { Color::Gray }))
        .alignment(ratatui::layout::Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(bar, chunks[1]);
}

// ─── Tab 3: Browse ───────────────────────────────────────────────

fn draw_browse(frame: &mut Frame, area: Rect, app: &App) {
    let scan = match &app.scan {
        Some(s) => s,
        None => return,
    };

    // Show top entries sorted by size
    let top_n = 30;
    let items: Vec<ListItem> = scan
        .entries
        .iter()
        .take(top_n)
        .enumerate()
        .map(|(i, entry)| {
            let safety = safety::classify_safety(&entry.category, &entry.path.to_string_lossy());
            let safety_color = match safety {
                SafetyLevel::Safe => Color::Green,
                SafetyLevel::Review => Color::Yellow,
                SafetyLevel::Dangerous => Color::Red,
            };

            let path_str = entry.path.to_string_lossy();
            let short_path = if path_str.len() > 55 {
                format!("...{}", &path_str[path_str.len() - 52..])
            } else {
                path_str.to_string()
            };

            let cursor = if i == app.scroll_offset { "▸" } else { " " };

            ListItem::new(Line::from(vec![
                Span::raw(format!("{} ", cursor)),
                Span::styled(
                    format!("{:>12} ", scanner::format_bytes(entry.size)),
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled(
                    format!("[{}] ", safety.short_label()),
                    Style::default().fg(safety_color),
                ),
                Span::styled(
                    format!("[{}] ", entry.category.label()),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::raw(short_path),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(format!(
            " Largest Items — {} ({} total) ",
            scan.scan_path.display(),
            scan.entries.len()
        )))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );
    frame.render_widget(list, area);
}

// ─── Tab 4: Details ──────────────────────────────────────────────

fn draw_details(frame: &mut Frame, area: Rect, app: &App) {
    let scan = match &app.scan {
        Some(s) => s,
        None => return,
    };

    let mut lines = vec![
        Line::from(Span::styled(
            "Category Breakdown",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!("  {:<6}", "Safe"),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {:<6}", "Rev"),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {:<6}", "Risk"),
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(" {:<25}", "Category")),
            Span::styled(format!(" {:>12}", "Size"), Style::default().fg(Color::Cyan)),
            Span::styled(
                format!(" {:>8}", "Files"),
                Style::default().fg(Color::DarkGray),
            ),
            Span::raw("  Hint"),
        ]),
        Line::from(format!("  {}", "─".repeat(90))),
    ];

    let mut sorted_cats: Vec<_> = scan.category_totals.iter().collect();
    sorted_cats.sort_by(|a, b| b.1.cmp(a.1));

    for (cat, &size) in &sorted_cats {
        if size == 0 {
            continue;
        }
        let safety = safety::classify_safety(cat, cat.label());
        let hint = safety::get_cleanup_hint(cat);
        let count = scan.category_counts.get(cat).copied().unwrap_or(0);

        let safety_marker = match safety {
            SafetyLevel::Safe => "●",
            SafetyLevel::Review => "◐",
            SafetyLevel::Dangerous => "○",
        };
        let safety_color = match safety {
            SafetyLevel::Safe => Color::Green,
            SafetyLevel::Review => Color::Yellow,
            SafetyLevel::Dangerous => Color::Red,
        };

        lines.push(Line::from(vec![
            Span::styled(
                format!("  {:<6} ", safety_marker),
                Style::default().fg(safety_color),
            ),
            Span::raw(format!("{:<25}", cat.label())),
            Span::styled(
                format!(" {:>12}", scanner::format_bytes(size)),
                Style::default().fg(Color::Cyan),
            ),
            Span::styled(
                format!(" {:>8}", count),
                Style::default().fg(Color::DarkGray),
            ),
            Span::styled(format!("  {}", hint), Style::default().fg(Color::DarkGray)),
        ]));
    }

    // Deduplication info
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Legend",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));
    lines.push(Line::from(vec![
        Span::styled("  ●", Style::default().fg(Color::Green)),
        Span::raw(" = Safe to clean    "),
        Span::styled("◐", Style::default().fg(Color::Yellow)),
        Span::raw(" = Review first    "),
        Span::styled("○", Style::default().fg(Color::Red)),
        Span::raw(" = Risky"),
    ]));
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  [S]", Style::default().fg(Color::Green)),
        Span::raw(" = Safe    "),
        Span::styled("[R]", Style::default().fg(Color::Yellow)),
        Span::raw(" = Review    "),
        Span::styled("[X]", Style::default().fg(Color::Red)),
        Span::raw(" = Risky"),
    ]));

    let details = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Scan Details "),
        )
        .wrap(Wrap { trim: false });

    // Allow scrolling
    let scroll_offset = app.scroll_offset as u16;
    frame.render_widget(details.scroll((scroll_offset, 0)), area);
}

// ─── Footer ──────────────────────────────────────────────────────

fn draw_footer(frame: &mut Frame, area: Rect, app: &App) {
    let mut spans = vec![
        Span::styled(
            " q",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Quit  "),
        Span::styled(
            "Tab/1-4",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Tabs  "),
        Span::styled(
            "j/k",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Scroll  "),
    ];

    if app.current_tab == 1 {
        spans.push(Span::styled(
            "Space",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(" Toggle  "));
        spans.push(Span::styled(
            "a",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(" All Safe  "));
        spans.push(Span::styled(
            "Enter",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(" Execute"));
    }

    let help = Paragraph::new(Line::from(spans)).block(Block::default().borders(Borders::ALL));
    frame.render_widget(help, area);
}

// ─── Confirmation Dialog ─────────────────────────────────────────

fn draw_confirm_dialog(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Create a centered popup
    let popup_width = 60.min(area.width - 4);
    let popup_height = 7;
    let popup_x = (area.width - popup_width) / 2;
    let popup_y = (area.height - popup_height) / 2;

    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    // Clear the area
    let clear = Block::default().style(Style::default().bg(Color::DarkGray));
    frame.render_widget(clear, popup_area);

    let dialog = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(" Confirm Cleanup ");

    let inner = dialog.inner(popup_area);
    frame.render_widget(dialog, popup_area);

    let lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            &app.confirm_message,
            Style::default().fg(Color::White),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  y",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" = Execute    "),
            Span::styled(
                "n/Esc",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::raw(" = Cancel"),
        ]),
    ];

    let text = Paragraph::new(lines).alignment(ratatui::layout::Alignment::Center);
    frame.render_widget(text, inner);
}
