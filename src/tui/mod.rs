mod dashboard;
mod piechart;
pub mod tree;

use crate::cleanup::CleanupAction;
use crate::distro::DistroInfo;
use crate::safety;
use crate::scanner::{self, ScanResult, SnapRevisionInfo};
use crate::Args;
use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;

pub const TABS: &[&str] = &["Overview", "Clean Up", "Browse", "Details"];

pub enum AppPhase {
    Scanning,
    Ready,
    ConfirmClean,
}

pub struct App {
    pub should_quit: bool,
    pub current_tab: usize,
    pub scroll_offset: usize,
    pub scan_path: String,
    pub dry_run: bool,
    pub phase: AppPhase,

    // Scan results
    pub scan: Option<ScanResult>,
    pub distro: Option<DistroInfo>,
    pub actions: Vec<CleanupAction>,
    pub selected_actions: Vec<bool>,
    pub old_snaps: Vec<SnapRevisionInfo>,

    // Confirmation dialog
    pub confirm_message: String,
}

impl App {
    pub fn new(args: &Args) -> Self {
        Self {
            should_quit: false,
            current_tab: 0,
            scroll_offset: 0,
            scan_path: args.path.clone(),
            dry_run: args.dry_run,
            phase: AppPhase::Scanning,
            scan: None,
            distro: None,
            actions: Vec::new(),
            selected_actions: Vec::new(),
            old_snaps: Vec::new(),
            confirm_message: String::new(),
        }
    }

    pub fn selected_count(&self) -> usize {
        self.selected_actions.iter().filter(|&&v| v).count()
    }

    pub fn selected_bytes(&self) -> u64 {
        self.actions
            .iter()
            .enumerate()
            .filter(|(i, _)| self.selected_actions.get(*i).copied().unwrap_or(false))
            .map(|(_, a)| a.estimated_bytes)
            .sum()
    }

    pub fn toggle_selected(&mut self) {
        if self.current_tab == 1 {
            let idx = self.scroll_offset;
            if idx < self.selected_actions.len() {
                self.selected_actions[idx] = !self.selected_actions[idx];
            }
        }
    }

    pub fn toggle_all_safe(&mut self) {
        for (i, action) in self.actions.iter().enumerate() {
            if action.safety == safety::SafetyLevel::Safe {
                self.selected_actions[i] = !self.selected_actions[i];
            }
        }
    }
}

pub fn run(args: &Args) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(args);
    let result = run_app(&mut terminal, &mut app);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    result
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    // Run scan first, then enter interactive loop
    perform_scan(app)?;

    loop {
        terminal.draw(|frame| dashboard::draw(frame, app))?;

        if event::poll(std::time::Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                match app.phase {
                    AppPhase::Ready => handle_ready_key(app, key),
                    AppPhase::ConfirmClean => handle_confirm_key(app, key),
                    AppPhase::Scanning => {} // shouldn't reach here
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

fn perform_scan(app: &mut App) -> Result<()> {
    let distro = crate::distro::detect_distro();
    let scan = scanner::scan(
        std::path::Path::new(&app.scan_path),
        1024 * 1024, // 1MB minimum
        5,
        None,
    )?;

    // Detect old snap revisions
    let old_snaps = if distro.has_snap {
        scanner::detect_old_snap_revisions()
    } else {
        Vec::new()
    };

    let actions = crate::cleanup::plan_cleanups(&scan, &distro, &old_snaps);
    let selected_actions = vec![false; actions.len()];

    app.scan = Some(scan);
    app.distro = Some(distro);
    app.old_snaps = old_snaps;
    app.actions = actions;
    app.selected_actions = selected_actions;
    app.phase = AppPhase::Ready;

    Ok(())
}

fn handle_ready_key(app: &mut App, key: event::KeyEvent) {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.should_quit = true;
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
        }
        KeyCode::Tab => {
            app.current_tab = (app.current_tab + 1) % TABS.len();
            app.scroll_offset = 0;
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.scroll_offset = app.scroll_offset.saturating_add(1);
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.scroll_offset = app.scroll_offset.saturating_sub(1);
        }
        KeyCode::Char('1') => {
            app.current_tab = 0;
            app.scroll_offset = 0;
        }
        KeyCode::Char('2') => {
            app.current_tab = 1;
            app.scroll_offset = 0;
        }
        KeyCode::Char('3') => {
            app.current_tab = 2;
            app.scroll_offset = 0;
        }
        KeyCode::Char('4') => {
            app.current_tab = 3;
            app.scroll_offset = 0;
        }
        KeyCode::Char(' ') => {
            app.toggle_selected();
        }
        KeyCode::Char('a') => {
            app.toggle_all_safe();
        }
        KeyCode::Enter => {
            if app.selected_count() > 0 {
                app.confirm_message = format!(
                    "Execute {} cleanup actions (~{} reclaimable)? [y/N]",
                    app.selected_count(),
                    scanner::format_bytes(app.selected_bytes())
                );
                app.phase = AppPhase::ConfirmClean;
            }
        }
        _ => {}
    }
}

fn handle_confirm_key(app: &mut App, key: event::KeyEvent) {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            // Execute selected cleanups
            let dry_run = app.dry_run;
            for (i, action) in app.actions.iter().enumerate() {
                if app.selected_actions.get(i).copied().unwrap_or(false) {
                    let _ = crate::cleanup::execute_cleanup(action, dry_run);
                }
            }
            app.selected_actions.fill(false);
            app.phase = AppPhase::Ready;
        }
        KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
            app.phase = AppPhase::Ready;
        }
        _ => {}
    }
}
