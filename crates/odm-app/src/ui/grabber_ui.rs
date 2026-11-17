//! Site Grabber window.

use crate::ui::app::IdmApp;
use crate::ui::theme;
use eframe::egui::{self, Color32, RichText};
use odm_core::grabber::{self, GrabberConfig, GrabberResult};
use odm_core::AddRequest;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};

pub struct GrabberState {
    pub start_url: String,
    pub save_dir: String,
    pub depth: usize,
    pub same_site: bool,
    pub include: String,
    pub exclude: String,
    pub running: bool,
    pub cancel: Arc<AtomicBool>,
    pub result: Option<GrabberResult>,
    pub rx: Option<mpsc::Receiver<Result<GrabberResult, String>>>,
    pub error: String,
    pub checked: Vec<bool>,
}

impl Default for GrabberState {
    fn default() -> Self {
        Self {
            start_url: String::new(),
            save_dir: odm_core::paths::default_download_dir()
                .to_string_lossy()
                .to_string(),
            depth: 3,
            same_site: true,
            include: String::new(),
            exclude: String::new(),
            running: false,
            cancel: Arc::new(AtomicBool::new(false)),
            result: None,
            rx: None,
            error: String::new(),
            checked: Vec::new(),
        }
    }
}

pub fn show(app: &mut IdmApp, ctx: &egui::Context) {
    if app.grabber.is_none() {
        return;
    }
    let mut g = app.grabber.take().unwrap();
    // Poll the worker.
    if let Some(rx) = &g.rx {
        if let Ok(res) = rx.try_recv() {
            g.running = false;
            match res {
                Ok(r) => {
                    g.checked = vec![true; r.files.len()];
                    g.result = Some(r);
                }
                Err(e) => g.error = e,
            }
            g.rx = None;
        }
    }
    let mut open = true;
    egui::Window::new("Open Download Manager Site Grabber")
        .id(egui::Id::new("grabber"))
        .collapsible(false)
        .resizable(true)
        .default_width(760.0)
        .default_height(520.0)
        .open(&mut open)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Start page/address:");
                ui.add(egui::TextEdit::singleline(&mut g.start_url).desired_width(380.0));
                ui.label("Levels:");
                ui.add(egui::DragValue::new(&mut g.depth).range(0..=20));
                ui.checkbox(&mut g.same_site, "This site only");
            });
            ui.horizontal(|ui| {
                ui.label("Save files to:");
                ui.add(egui::TextEdit::singleline(&mut g.save_dir).desired_width(300.0));
                if ui.button("Browse...").clicked() {
                    if let Some(p) = rfd::FileDialog::new().pick_folder() {
                        g.save_dir = p.to_string_lossy().to_string();
                    }
                }
            });
            ui.horizontal(|ui| {
                ui.label("Include file types:");
                ui.add(egui::TextEdit::singleline(&mut g.include).desired_width(150.0));
                ui.label("Exclude:");
                ui.add(egui::TextEdit::singleline(&mut g.exclude).desired_width(150.0));
                ui.label(RichText::new("space separated, empty = all").size(10.0).color(Color32::from_gray(110)));
            });
            ui.horizontal(|ui| {
                if g.running {
                    if ui.button("Stop Explore Site").clicked() {
                        g.cancel.store(true, Ordering::SeqCst);
                        g.running = false;
                    }
                } else if ui.button("Start Explore Site").clicked() {
                    if g.start_url.trim().is_empty() {
                        g.error = "Please enter start page/address".into();
                    } else {
                        let cfg = GrabberConfig {
                            start_url: g.start_url.clone(),
                            save_dir: std::path::PathBuf::from(&g.save_dir),
                            depth: g.depth,
                            same_site_only: g.same_site,
                            include_exts: g.include.split_whitespace().map(|s| s.to_ascii_lowercase()).collect(),
                            exclude_exts: g.exclude.split_whitespace().map(|s| s.to_ascii_lowercase()).collect(),
                            ..Default::default()
                        };
                        let cancel = Arc::new(AtomicBool::new(false));
                        g.cancel = cancel.clone();
                        let (tx, rx) = mpsc::channel();
                        g.rx = Some(rx);
                        g.running = true;
                        g.error.clear();
                        g.result = None;
                        std::thread::spawn(move || {
                            let res = grabber::explore(&cfg, cancel).map_err(|e| format!("{e:#}"));
                            let _ = tx.send(res);
                        });
                    }
                }
                if let Some(r) = &g.result {
                    ui.label(format!(
                        "Files found: {}   Web pages processed: {}",
                        r.files.len(),
                        r.stats.pages_processed
                    ));
                }
                if g.running {
                    ui.spinner();
                }
            });
            if !g.error.is_empty() {
                ui.colored_label(theme::ODM_RED, &g.error);
            }
            ui.separator();
            let mut add_checked = false;
            egui::ScrollArea::both().max_height(330.0).show(ui, |ui| {
                if let Some(r) = &mut g.result {
                    egui::Grid::new("grabber_grid")
                        .striped(true)
                        .num_columns(4)
                        .show(ui, |ui| {
                            ui.label(RichText::new("Download").strong());
                            ui.label(RichText::new("File Name").strong());
                            ui.label(RichText::new("URL").strong());
                            ui.label(RichText::new("Page").strong());
                            ui.end_row();
                            for (i, f) in r.files.iter().enumerate() {
                                let mut checked = g.checked.get(i).copied().unwrap_or(true);
                                if ui.checkbox(&mut checked, "").changed() {
                                    if i < g.checked.len() {
                                        g.checked[i] = checked;
                                    }
                                }
                                ui.label(&f.file_name);
                                ui.add(egui::Label::new(RichText::new(&f.url).monospace().size(10.5)).wrap());
                                ui.add(egui::Label::new(RichText::new(&f.page).size(10.5)).wrap());
                                ui.end_row();
                            }
                        });
                    if r.files.is_empty() {
                        ui.label("No files found.");
                    }
                } else {
                    ui.label("Enter a start page and press \"Start Explore Site\".");
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Download checked files").clicked() {
                    add_checked = true;
                }
                if ui.button("Close").clicked() {
                    // close
                }
            });
            if add_checked {
                if let Some(r) = &g.result {
                    let mut n = 0;
                    for (i, f) in r.files.iter().enumerate() {
                        if g.checked.get(i).copied().unwrap_or(true) {
                            app.engine.add(AddRequest {
                                url: f.url.clone(),
                                referer: f.referer.clone(),
                                save_dir: Some(std::path::PathBuf::from(&g.save_dir)),
                                start: true,
                                ..Default::default()
                            });
                            n += 1;
                        }
                    }
                    app.status = format!("{n} files were added to the main download list.");
                }
            }
        });
    if open {
        app.grabber = Some(g);
    }
}
