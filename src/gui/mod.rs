use eframe::egui;
use std::sync::mpsc::{channel, Receiver, Sender};

use crate::cleanup::{self, CleanupAction};
use crate::distro;
use crate::safety::{self, SafetyLevel};
use crate::scanner::{self, ScanResult};

pub fn run() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 650.0])
            .with_title("Linux Cleanup"),
        ..Default::default()
    };

    let _ = eframe::run_native(
        "Linux Cleanup",
        options,
        Box::new(|_cc| Ok(Box::new(CleanupApp::default()))),
    );
}

#[derive(PartialEq)]
enum Tab {
    Overview,
    Categories,
    Actions,
}

struct ScanState {
    scan: ScanResult,
    distro: distro::DistroInfo,
    actions: Vec<CleanupAction>,
    selected: Vec<bool>,
}

enum ScanMsg {
    InProgress(String),
    Done(ScanState),
}

struct CleanupApp {
    tab: Tab,
    scan_state: Option<ScanState>,
    scan_status: String,
    is_scanning: bool,
    scan_rx: Option<Receiver<ScanMsg>>,
    safe_total: u64,
    review_total: u64,
    risky_total: u64,
}

impl Default for CleanupApp {
    fn default() -> Self {
        Self {
            tab: Tab::Overview,
            scan_state: None,
            scan_status: "Click 'Scan System' to begin".to_string(),
            is_scanning: false,
            scan_rx: None,
            safe_total: 0,
            review_total: 0,
            risky_total: 0,
        }
    }
}

impl eframe::App for CleanupApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check for scan messages
        if let Some(rx) = &self.scan_rx {
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    ScanMsg::InProgress(msg) => self.scan_status = msg,
                    ScanMsg::Done(state) => {
                        self.scan_status =
                            format!("Scan complete: {} files scanned", state.scan.files_scanned);
                        self.safe_total = state
                            .actions
                            .iter()
                            .filter(|a| a.safety == SafetyLevel::Safe)
                            .map(|a| a.estimated_bytes)
                            .sum();
                        self.review_total = state
                            .actions
                            .iter()
                            .filter(|a| a.safety == SafetyLevel::Review)
                            .map(|a| a.estimated_bytes)
                            .sum();
                        self.risky_total = state
                            .actions
                            .iter()
                            .filter(|a| a.safety == SafetyLevel::Dangerous)
                            .map(|a| a.estimated_bytes)
                            .sum();
                        self.scan_state = Some(state);
                        self.is_scanning = false;
                    }
                }
            }
        }

        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Linux Cleanup");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Clean Selected").clicked() {
                        if let Some(state) = &self.scan_state {
                            let selected = state.selected.clone();
                            let actions = state.actions.clone();
                            for (i, action) in actions.iter().enumerate() {
                                if selected.get(i).copied().unwrap_or(false) {
                                    let _ = cleanup::execute_cleanup(action, false);
                                }
                            }
                        }
                    }

                    if ui.button("Scan System").clicked() && !self.is_scanning {
                        self.is_scanning = true;
                        self.scan_status = "Scanning...".to_string();
                        let (tx, rx): (Sender<ScanMsg>, Receiver<ScanMsg>) = channel();
                        self.scan_rx = Some(rx);

                        std::thread::spawn(move || {
                            tx.send(ScanMsg::InProgress("Scanning / ...".to_string()))
                                .ok();

                            let scan =
                                scanner::scan(std::path::Path::new("/"), 1024 * 1024, 5, None);
                            let distro = distro::detect_distro();

                            if let Ok(scan) = scan {
                                let old_snaps = if distro.has_snap {
                                    scanner::detect_old_snap_revisions()
                                } else {
                                    Vec::new()
                                };
                                let actions = cleanup::plan_cleanups(&scan, &distro, &old_snaps);
                                let selected = vec![false; actions.len()];
                                tx.send(ScanMsg::Done(ScanState {
                                    scan,
                                    distro,
                                    actions,
                                    selected,
                                }))
                                .ok();
                            }
                        });
                    }
                });
            });
        });

        egui::TopBottomPanel::bottom("tabs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.tab, Tab::Overview, "Overview");
                ui.selectable_value(&mut self.tab, Tab::Categories, "Categories");
                ui.selectable_value(&mut self.tab, Tab::Actions, "Cleanup Actions");
            });
        });

        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.label(&self.scan_status);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.is_scanning {
                ui.centered_and_justified(|ui| {
                    ui.spinner();
                    ui.label(&self.scan_status);
                });
                ctx.request_repaint();
                return;
            }

            match self.tab {
                Tab::Overview => self.draw_overview(ui),
                Tab::Categories => self.draw_categories(ui),
                Tab::Actions => self.draw_actions(ui),
            }
        });
    }
}

impl CleanupApp {
    fn draw_overview(&mut self, ui: &mut egui::Ui) {
        if let Some(state) = &self.scan_state {
            ui.heading("Disk Usage");
            for disk in &state.scan.disk_info {
                let pct = if disk.total > 0 {
                    disk.used as f64 / disk.total as f64
                } else {
                    0.0
                };

                ui.horizontal(|ui| {
                    ui.label(format!("{}", disk.mount_point.display()));
                    ui.label(format!("({})", scanner::format_bytes(disk.used)));
                });
                ui.add(egui::ProgressBar::new(pct as f32).text(format!("{:.1}%", pct * 100.0)));
            }

            ui.separator();
            ui.heading("Cleanup Summary");

            ui.horizontal(|ui| {
                ui.colored_label(
                    egui::Color32::from_rgb(80, 200, 120),
                    format!("Safe: {}", scanner::format_bytes(self.safe_total)),
                );
                ui.colored_label(
                    egui::Color32::YELLOW,
                    format!("Review: {}", scanner::format_bytes(self.review_total)),
                );
                ui.colored_label(
                    egui::Color32::RED,
                    format!("Risky: {}", scanner::format_bytes(self.risky_total)),
                );
            });

            let total_reclaimable = self.safe_total + self.review_total;
            ui.separator();
            ui.label(format!(
                "Total reclaimable: {} ({} categories)",
                scanner::format_bytes(total_reclaimable),
                state.scan.category_totals.len()
            ));

            if state.distro.has_snap {
                ui.label("Snap detected: old revision cleanup available");
            }
            if state.distro.has_docker {
                ui.label("Docker detected: cleanup available");
            }
            if state.distro.has_flatpak {
                ui.label("Flatpak detected: cleanup available");
            }
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("Click 'Scan System' to begin");
            });
        }
    }

    fn draw_categories(&mut self, ui: &mut egui::Ui) {
        if let Some(state) = &self.scan_state {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Cleanup Categories");

                let mut sorted: Vec<_> = state.scan.category_totals.iter().collect();
                sorted.sort_by(|a, b| b.1.cmp(a.1));

                egui::Grid::new("categories_grid")
                    .num_columns(4)
                    .striped(true)
                    .min_col_width(150.0)
                    .show(ui, |ui| {
                        ui.strong("Safety");
                        ui.strong("Category");
                        ui.strong("Size");
                        ui.strong("Hint");
                        ui.end_row();

                        for (cat, &size) in &sorted {
                            if size == 0 {
                                continue;
                            }
                            let safety = safety::classify_safety(cat, cat.label());
                            let hint = safety::get_cleanup_hint(cat);

                            let color = match safety {
                                SafetyLevel::Safe => egui::Color32::from_rgb(80, 200, 120),
                                SafetyLevel::Review => egui::Color32::YELLOW,
                                SafetyLevel::Dangerous => egui::Color32::RED,
                            };

                            ui.colored_label(color, safety.label());
                            ui.label(cat.label());
                            ui.label(scanner::format_bytes(size));
                            ui.label(hint);
                            ui.end_row();
                        }
                    });
            });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("No scan data. Click 'Scan System' first.");
            });
        }
    }

    fn draw_actions(&mut self, ui: &mut egui::Ui) {
        if let Some(state) = &mut self.scan_state {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Cleanup Actions");
                ui.label("Select actions to execute, then click 'Clean Selected'.");

                for (i, action) in state.actions.iter().enumerate() {
                    let color = match action.safety {
                        SafetyLevel::Safe => egui::Color32::from_rgb(80, 200, 120),
                        SafetyLevel::Review => egui::Color32::YELLOW,
                        SafetyLevel::Dangerous => egui::Color32::RED,
                    };

                    ui.horizontal(|ui| {
                        ui.checkbox(state.selected.get_mut(i).unwrap_or(&mut false), "");
                        ui.colored_label(color, format!("[{}] ", action.safety.label()));
                        ui.label(&action.description);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(scanner::format_bytes(action.estimated_bytes));
                        });
                    });
                }
            });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("No scan data. Click 'Scan System' first.");
            });
        }
    }
}
