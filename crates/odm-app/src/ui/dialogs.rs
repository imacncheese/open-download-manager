//! All dialogs: Add URL, Download progress, Options, Scheduler, Batch,
//! Find, Properties, Categories, About, completion and tips.

use crate::ui::app::{Filter, IdmApp};
use crate::ui::theme;
use crate::ui::widgets;
use eframe::egui::{self, Align, Color32, Layout, RichText, Sense, Stroke};
use odm_core::config::{Category, CompletionAction, Config, ProxyMode, SiteLogin};
use odm_core::model::Download;
use odm_core::util;
use odm_core::{AddRequest, Status};
use std::path::PathBuf;

#[derive(Clone, Copy)]
enum ProgAction {
    Pause,
    Stop,
    Start,
}

#[derive(Default)]
pub struct Dialogs {
    pub add_url: Option<AddUrlState>,
    pub progress_windows: Vec<u64>,
    pub options: Option<OptionsState>,
    pub scheduler: bool,
    pub about: bool,
    pub batch: Option<BatchState>,
    pub find: Option<FindState>,
    pub properties: Option<PropertiesState>,
    pub complete: Option<CompleteState>,
    pub category: Option<CategoryState>,
    pub delete_category: Option<String>,
    pub confirm_delete: Option<Vec<u64>>,
    pub confirm_delete_files: bool,
    pub columns: bool,
    pub speed_limiter: bool,
    pub tip: bool,
    pub browser_setup: bool,
}

#[derive(Default, Clone)]
pub struct AddUrlState {
    pub url: String,
    pub save_dir: String,
    pub description: String,
    pub category: String,
    pub queue: u64,
    pub use_auth: bool,
    pub username: String,
    pub password: String,
    pub start_later: bool,
    pub referer: String,
    pub cookies: String,
}

pub struct CompleteState {
    pub file_name: String,
    pub path: PathBuf,
    pub size: u64,
}

#[derive(Clone)]
pub struct PropertiesState {
    pub id: u64,
    pub file_name: String,
    pub url: String,
    pub save_dir: String,
    pub size: Option<u64>,
    pub downloaded: u64,
    pub status: String,
    pub description: String,
    pub referer: String,
    pub content_type: String,
    pub created: i64,
    pub completed: Option<i64>,
    pub last_try: Option<i64>,
    pub connections: u32,
    pub error: String,
}

impl From<&Download> for PropertiesState {
    fn from(d: &Download) -> Self {
        Self {
            id: d.id,
            file_name: d.file_name.clone(),
            url: d.url.clone(),
            save_dir: d.save_dir.to_string_lossy().to_string(),
            size: d.size,
            downloaded: d.downloaded,
            status: d.status.label().into(),
            description: d.description.clone(),
            referer: d.referer.clone(),
            content_type: d.content_type.clone().unwrap_or_default(),
            created: d.created,
            completed: d.completed,
            last_try: d.last_try,
            connections: d.connections,
            error: d.error.clone().unwrap_or_default(),
        }
    }
}

pub struct OptionsState {
    pub tab: usize,
    pub cfg: Config,
    pub types_text: String,
    pub sites_text: String,
    pub login_site: String,
    pub login_user: String,
    pub login_pass: String,
    pub selected_login: Option<usize>,
    pub category_sel: usize,
    pub new_type: String,
    pub conn_server: String,
    pub conn_max: u32,
    pub status: String,
}

impl OptionsState {
    pub fn new(cfg: &Config) -> Self {
        Self {
            tab: 0,
            cfg: cfg.clone(),
            types_text: cfg.file_types.join(" "),
            sites_text: cfg.proxy.no_proxy.clone(),
            login_site: String::new(),
            login_user: String::new(),
            login_pass: String::new(),
            selected_login: None,
            category_sel: 0,
            new_type: String::new(),
            conn_server: String::new(),
            conn_max: 4,
            status: String::new(),
        }
    }
}

#[derive(Default)]
pub struct BatchState {
    pub url: String,
    pub from: u64,
    pub to: u64,
    pub width: usize,
    pub letters: bool,
    pub save_all_to: String,
    pub preview: Vec<String>,
    pub error: String,
}

#[derive(Default)]
pub struct FindState {
    pub text: String,
    pub match_case: bool,
    pub whole: bool,
    pub name: bool,
    pub description: bool,
    pub site: bool,
}

pub struct CategoryState {
    pub id: String,
    pub name: String,
    pub folder: String,
    pub masks: String,
    pub sites: String,
    pub edit: bool,
}

impl Default for CategoryState {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            folder: String::new(),
            masks: String::new(),
            sites: String::new(),
            edit: false,
        }
    }
}

impl CategoryState {
    pub fn edit(c: &Category) -> Self {
        Self {
            id: c.id.clone(),
            name: c.name.clone(),
            folder: c.folder.to_string_lossy().to_string(),
            masks: c.masks.join(" "),
            sites: c.sites.join(" "),
            edit: true,
        }
    }
}

impl IdmApp {
    pub(crate) fn show_dialogs(&mut self, ctx: &egui::Context) {
        self.dialog_add_url(ctx);
        self.dialog_progress(ctx);
        self.dialog_options(ctx);
        self.dialog_scheduler(ctx);
        self.dialog_batch(ctx);
        self.dialog_find(ctx);
        self.dialog_properties(ctx);
        self.dialog_complete(ctx);
        self.dialog_category(ctx);
        self.dialog_columns(ctx);
        self.dialog_speed_limiter(ctx);
        self.dialog_confirm_delete(ctx);
        self.dialog_about(ctx);
        self.dialog_browser_setup(ctx);
        self.dialog_tip(ctx);
        self.dialog_clipboard(ctx);
        self.dialog_info(ctx);
        crate::ui::grabber_ui::show(self, ctx);
    }

    fn dialog_add_url(&mut self, ctx: &egui::Context) {
        let Some(mut st) = self.dialogs.add_url.take() else { return };
        if st.category.is_empty() {
            st.category = self.cfg.last_category.clone();
        }
        let cfg = self.cfg.clone();
        let mut open = true;
        let mut submit = false;
        egui::Window::new("Enter new address to download")
            .id(egui::Id::new("add_url"))
            .collapsible(false)
            .resizable(false)
            .default_width(430.0)
                        .show(ctx, |ui| {
                egui::Grid::new("add_grid")
                    .num_columns(2)
                    .spacing([8.0, 6.0])
                    .show(ui, |ui| {
                        ui.label("Address:");
                        ui.add(egui::TextEdit::singleline(&mut st.url).desired_width(320.0));
                        ui.end_row();
                        ui.label("Save to:");
                        ui.horizontal(|ui| {
                            ui.add(egui::TextEdit::singleline(&mut st.save_dir).desired_width(250.0));
                            if ui.button("Browse...").clicked() {
                                if let Some(p) = rfd::FileDialog::new()
                                    .set_directory(&st.save_dir)
                                    .pick_folder()
                                {
                                    st.save_dir = p.to_string_lossy().to_string();
                                }
                            }
                        });
                        ui.end_row();
                        ui.label("Description:");
                        ui.add(egui::TextEdit::singleline(&mut st.description).desired_width(320.0));
                        ui.end_row();
                        ui.label("Category:");
                        egui::ComboBox::from_id_salt("add_cat")
                            .selected_text(
                                cfg.category(&st.category)
                                    .map(|c| c.name.clone())
                                    .unwrap_or_else(|| "General".into()),
                            )
                            .show_ui(ui, |ui| {
                                for c in &cfg.categories {
                                    ui.selectable_value(&mut st.category, c.id.clone(), &c.name);
                                }
                            });
                        ui.end_row();
                        ui.label("Queue:");
                        egui::ComboBox::from_id_salt("add_queue")
                            .selected_text(
                                cfg.queues
                                    .iter()
                                    .find(|q| q.id == st.queue)
                                    .map(|q| q.name.clone())
                                    .unwrap_or_else(|| "Main download queue".into()),
                            )
                            .show_ui(ui, |ui| {
                                for q in &cfg.queues {
                                    ui.selectable_value(&mut st.queue, q.id, &q.name);
                                }
                            });
                        ui.end_row();
                    });
                ui.add_space(4.0);
                ui.checkbox(&mut st.use_auth, "Use authorization");
                if st.use_auth {
                    ui.horizontal(|ui| {
                        ui.label("Login:");
                        ui.add(egui::TextEdit::singleline(&mut st.username).desired_width(120.0));
                        ui.label("Password:");
                        ui.add(egui::TextEdit::singleline(&mut st.password).password(true).desired_width(120.0));
                    });
                }
                ui.checkbox(&mut st.start_later, "Download later (do not start now)");
                ui.add_space(6.0);
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Batch download...").clicked() {
                        self.dialogs.batch = Some(BatchState {
                            url: st.url.clone(),
                            ..Default::default()
                        });
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new("Cancel")
                                    .min_size(egui::vec2(86.0, 26.0)),
                            )
                            .clicked()
                        {
                            open = false;
                        }
                        if ui
                            .add(
                                egui::Button::new(RichText::new("OK").strong())
                                    .min_size(egui::vec2(86.0, 26.0)),
                            )
                            .clicked()
                        {
                            submit = true;
                        }
                    });
                });
            });
        if submit {
            let url = st.url.trim().to_string();
            if !(url.starts_with("http://") || url.starts_with("https://") || url.starts_with("ftp://")) {
                self.info = Some(("Error".into(), "Invalid URL entered. Please correct.".into()));
                self.dialogs.add_url = Some(st);
                return;
            }
            let req = AddRequest {
                url,
                save_dir: Some(PathBuf::from(&st.save_dir)),
                description: st.description.clone(),
                category: Some(st.category.clone()),
                queue_id: Some(st.queue),
                referer: st.referer.clone(),
                cookies: if st.cookies.is_empty() { None } else { Some(st.cookies.clone()) },
                username: if st.use_auth { Some(st.username.clone()) } else { None },
                password: if st.use_auth { Some(st.password.clone()) } else { None },
                start: !st.start_later,
                ..Default::default()
            };
            self.cfg.last_category = st.category.clone();
            self.cfg.last_save_dir = PathBuf::from(&st.save_dir);
            self.engine.set_config(self.cfg.clone());
            self.add_download(req);
        } else if open {
            self.dialogs.add_url = Some(st);
        }
    }

    fn dialog_progress(&mut self, ctx: &egui::Context) {
        if self.dialogs.progress_windows.is_empty() {
            return;
        }
        let ids = self.dialogs.progress_windows.clone();
        let mut actions: Vec<(u64, ProgAction)> = Vec::new();
        let mut close: Vec<u64> = Vec::new();

        for (win_idx, id) in ids.iter().copied().enumerate() {
            let Some(d) = self.downloads.iter().find(|d| d.id == id).cloned() else {
                // The engine can emit Started before the UI snapshot includes
                // the new download. Keep the window during that brief gap.
                if !self.progress_auto.contains(&id) {
                    close.push(id);
                }
                continue;
            };
            let mut open = true;
            let mut action: Option<ProgAction> = None;
            let mut close_clicked = false;
            egui::Window::new(format!("Download progress - {}", d.file_name))
                .id(egui::Id::new(("progress", id)))
                .collapsible(false)
                .resizable(true)
                .default_width(560.0)
                .default_pos(egui::pos2(
                    70.0 + (win_idx % 8) as f32 * 28.0,
                    60.0 + (win_idx % 8) as f32 * 28.0,
                ))
                .open(&mut open)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&d.file_name).strong());
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(
                                RichText::new(d.status.label())
                                    .color(if d.status == Status::Error {
                                        theme::ODM_RED
                                    } else {
                                        Color32::from_gray(90)
                                    }),
                            );
                        });
                    });
                    ui.add_space(4.0);
                    widgets::progress_bar_animated(
                        ui,
                        ("progress", id),
                        d.percent() / 100.0,
                        20.0,
                        true,
                        ui.visuals().dark_mode,
                    );
                    ui.add_space(4.0);
                    egui::Grid::new(("prog_grid", id))
                        .num_columns(4)
                        .spacing([16.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("File size:");
                            ui.label(d.size.map(util::format_bytes).unwrap_or_else(|| "(unknown)".into()));
                            ui.label("Downloaded:");
                            ui.label(util::format_bytes(d.downloaded));
                            ui.end_row();
                            ui.label("Transfer rate:");
                            ui.label(if d.speed > 0 { util::format_speed(d.speed) } else { String::new() });
                            ui.label("Time left:");
                            ui.label(d.time_left().map(util::format_time_left).unwrap_or_default());
                            ui.end_row();
                            ui.label("Resume capability:");
                            ui.label(if d.resumable { "Yes" } else { "No" });
                            ui.label("Connections:");
                            ui.label(format!("{}", d.parts.len()));
                            ui.end_row();
                        });
                    if let Some(err) = &d.error {
                        ui.colored_label(theme::ODM_RED, err);
                    }
                    ui.add_space(4.0);
                    ui.group(|ui| {
                        ui.label(
                            RichText::new("Start positions and download progress by connections")
                                .strong()
                                .size(11.0),
                        );
                        egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
                            if d.parts.is_empty() {
                                ui.label("(no connection data)");
                            }
                            for (i, p) in d.parts.iter().enumerate() {
                                ui.horizontal(|ui| {
                                    ui.add_sized([24.0, 16.0], egui::Label::new(format!("{}.", i + 1)));
                                    ui.add_sized(
                                        [140.0, 16.0],
                                        egui::Label::new(format!(
                                            "{} - {}",
                                            util::format_bytes(p.start),
                                            util::format_bytes(p.end.saturating_add(1))
                                        )),
                                    );
                                    let w = (ui.available_width() - 90.0).max(60.0);
                                    let pct = p.percent() / 100.0;
                                    widgets::mini_progress_bar(
                                        ui,
                                        ("part", id, i),
                                        pct,
                                        w,
                                        ui.visuals().dark_mode,
                                    );
                                    ui.label(format!("{:.0}%", pct * 100.0));
                                });
                            }
                        });
                    });
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        match d.status {
                            Status::Downloading | Status::Connecting | Status::Assembling => {
                                if ui.button("Pause").clicked() {
                                    action = Some(ProgAction::Pause);
                                }
                                if ui.button("Stop").clicked() {
                                    action = Some(ProgAction::Stop);
                                }
                            }
                            Status::Complete => {
                                if ui.button("Open").clicked() {
                                    let _ = open::that(d.dest_path());
                                }
                                if ui.button("Open folder").clicked() {
                                    let _ = open::that(&d.save_dir);
                                }
                            }
                            _ => {
                                if ui.button("Start").clicked() {
                                    action = Some(ProgAction::Start);
                                }
                                if ui.button("Remove").clicked() {
                                    action = Some(ProgAction::Stop);
                                }
                            }
                        }
                        if ui.button("Close").clicked() {
                            close_clicked = true;
                        }
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            ui.label(format!("{}%", d.percent()));
                        });
                    });
                });
            if let Some(a) = action {
                actions.push((id, a));
            }
            if close_clicked || !open {
                close.push(id);
            }
        }

        for (id, action) in actions {
            match action {
                ProgAction::Pause => self.engine.stop(id, true),
                ProgAction::Stop => self.engine.stop(id, false),
                ProgAction::Start => self.engine.start(id),
            }
        }
        if !close.is_empty() {
            self.dialogs
                .progress_windows
                .retain(|id| !close.contains(id));
        }
    }

    fn dialog_options(&mut self, ctx: &egui::Context) {
        let Some(mut st) = self.dialogs.options.take() else { return };
        let mut open = true;
        let mut apply = false;
        egui::Window::new("Open Download Manager Configuration")
            .id(egui::Id::new("options"))
            .collapsible(false)
            .resizable(true)
            .default_width(640.0)
            .default_height(470.0)
                        .show(ctx, |ui| {
                let tabs = [
                    "General",
                    "File Types",
                    "Connection",
                    "Proxy/Socks",
                    "Sites Logins",
                    "Dial Up/VPN",
                    "Sounds",
                    "Downloads",
                    "Browser",
                ];
                ui.horizontal_wrapped(|ui| {
                    for (i, t) in tabs.iter().enumerate() {
                        ui.selectable_value(&mut st.tab, i, *t);
                    }
                });
                ui.separator();
                egui::ScrollArea::vertical().max_height(390.0).show(ui, |ui| match st.tab {
                    0 => options_general(ui, &mut st),
                    1 => options_file_types(ui, &mut st),
                    2 => options_connection(ui, &mut st),
                    3 => options_proxy(ui, &mut st),
                    4 => options_logins(ui, &mut st),
                    5 => options_dialup(ui, &mut st),
                    6 => options_sounds(ui, &mut st),
                    7 => options_downloads(ui, &mut st),
                    _ => options_browser(ui, &mut st),
                });
                if !st.status.is_empty() {
                    ui.colored_label(Color32::from_rgb(0, 120, 0), &st.status);
                }
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        apply = true;
                    }
                    if ui.button("Cancel").clicked() {
                        open = false;
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Restore defaults").clicked() {
                            st.cfg = Config::default();
                            st.types_text = st.cfg.file_types.join(" ");
                        }
                    });
                });
            });
        if apply {
            st.cfg.file_types = st.types_text.split_whitespace().map(|s| s.to_string()).collect();
            if st.cfg.speed_limit_kbps > 1_000_000 {
                st.cfg.speed_limit_kbps = 0;
            }
            self.engine.set_config(st.cfg.clone());
            self.cfg = st.cfg.clone();
            self.dark = matches!(self.cfg.theme, odm_core::config::ThemeMode::Dark)
                || (matches!(self.cfg.theme, odm_core::config::ThemeMode::System)
                    && theme::windows_dark_mode());
            theme::apply(ctx, self.dark);
            self.status = "Settings saved.".into();
            open = false;
        }
        if open {
            self.dialogs.options = Some(st);
        }
    }

    fn dialog_scheduler(&mut self, ctx: &egui::Context) {
        if !self.dialogs.scheduler {
            return;
        }
        let mut cfg = self.cfg.clone();
        let mut open = true;
        let mut changed = false;
        egui::Window::new("Scheduler")
            .id(egui::Id::new("scheduler"))
            .collapsible(false)
            .resizable(true)
            .default_width(680.0)
            .default_height(460.0)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(220.0);
                        ui.label(RichText::new("Queues").strong());
                        let mut sel = 0usize;
                        for (i, q) in cfg.queues.iter().enumerate() {
                            let text = format!(
                                "{} {}",
                                if q.running { "[>]" } else { "[ ]" },
                                q.name
                            );
                            if ui.selectable_label(i == 0, text).clicked() {
                                sel = i;
                            }
                        }
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            if ui.button("New queue").clicked() {
                                let id = cfg.queues.iter().map(|q| q.id).max().unwrap_or(0) + 1;
                                cfg.queues.push(odm_core::config::Queue {
                                    id,
                                    name: format!("Queue # {}", id),
                                    ..odm_core::config::Queue::main()
                                });
                                changed = true;
                            }
                            if ui.button("Delete").clicked() && cfg.queues.len() > 1 {
                                cfg.queues.remove(sel);
                                changed = true;
                            }
                        });
                    });
                    ui.separator();
                    ui.vertical(|ui| {
                        let idx = 0usize.min(cfg.queues.len().saturating_sub(1));
                        if let Some(q) = cfg.queues.get_mut(idx) {
                            ui.horizontal(|ui| {
                                ui.label("Queue name:");
                                if ui.text_edit_singleline(&mut q.name).changed() {
                                    changed = true;
                                }
                            });
                            ui.add_space(4.0);
                            ui.checkbox(&mut q.schedule.enabled, "Use scheduler");
                            ui.horizontal(|ui| {
                                ui.label("Start download at");
                                ui.add(egui::TextEdit::singleline(&mut q.schedule.start_time).desired_width(50.0));
                                ui.label("Stop download at");
                                ui.add(egui::TextEdit::singleline(&mut q.schedule.stop_time).desired_width(50.0));
                            });
                            ui.horizontal_wrapped(|ui| {
                                for (i, d) in ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"].iter().enumerate() {
                                    ui.checkbox(&mut q.schedule.days[i], *d);
                                }
                            });
                            ui.checkbox(&mut q.schedule.once, "One-time downloading");
                            ui.horizontal(|ui| {
                                ui.label("Turn off computer when done:");
                                egui::ComboBox::from_id_salt("action")
                                    .selected_text(match q.schedule.action {
                                        CompletionAction::None => "Do nothing",
                                        CompletionAction::Exit => "Exit ODM",
                                        CompletionAction::Shutdown => "Shut down",
                                        CompletionAction::Sleep => "Sleep",
                                        CompletionAction::Hibernate => "Hibernate",
                                        CompletionAction::Hangup => "Hang up modem",
                                    })
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut q.schedule.action, CompletionAction::None, "Do nothing");
                                        ui.selectable_value(&mut q.schedule.action, CompletionAction::Exit, "Exit ODM");
                                        ui.selectable_value(&mut q.schedule.action, CompletionAction::Shutdown, "Shut down");
                                        ui.selectable_value(&mut q.schedule.action, CompletionAction::Sleep, "Sleep");
                                        ui.selectable_value(&mut q.schedule.action, CompletionAction::Hibernate, "Hibernate");
                                        ui.selectable_value(&mut q.schedule.action, CompletionAction::Hangup, "Hang up modem");
                                    });
                            });
                            ui.add_space(4.0);
                            ui.group(|ui| {
                                ui.label(RichText::new("Download limits").strong());
                                ui.checkbox(&mut q.schedule.limit_enabled, "Enable download limits");
                                ui.horizontal(|ui| {
                                    ui.label("Download no more than");
                                    ui.add(egui::DragValue::new(&mut q.schedule.limit_mb).range(0..=100000));
                                    ui.label("MBytes every");
                                    ui.add(egui::DragValue::new(&mut q.schedule.limit_hours).range(0..=23));
                                    ui.label("hours");
                                });
                            });
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.label("Number of retries for each file:");
                                ui.add(egui::DragValue::new(&mut q.retries).range(0..=100));
                            });
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                if ui
                                    .button(if q.running { "Stop queue" } else { "Start queue" })
                                    .clicked()
                                {
                                    if q.running {
                                        self.engine.stop_queue(q.id);
                                    } else {
                                        self.engine.start_queue(q.id);
                                    }
                                }
                                if ui.button("Start now").clicked() {
                                    changed = true;
                                }
                            });
                            if changed {
                                // reflected by the config applied below
                            }
                        }
                    });
                });
            });
        if changed || open {
            self.engine.set_config(cfg.clone());
            self.cfg = cfg;
        }
        self.dialogs.scheduler = open;
    }

    fn dialog_batch(&mut self, ctx: &egui::Context) {
        let Some(mut st) = self.dialogs.batch.take() else { return };
        let mut open = true;
        let mut submit = false;
        egui::Window::new("Batch download")
            .id(egui::Id::new("batch"))
            .collapsible(false)
            .resizable(true)
            .default_width(560.0)
                        .show(ctx, |ui| {
                ui.label("Use the asterisk wildcard for the file name pattern, e.g.");
                ui.label(RichText::new("http://www.example.com/pictures/img*.jpg").monospace());
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label("URL:");
                    ui.add(egui::TextEdit::singleline(&mut st.url).desired_width(430.0));
                });
                ui.horizontal(|ui| {
                    ui.label("Numbers from:");
                    ui.add(egui::DragValue::new(&mut st.from).range(0..=1000000));
                    ui.label("to:");
                    ui.add(egui::DragValue::new(&mut st.to).range(0..=1000000));
                    ui.label("Wildcard size:");
                    ui.add(egui::DragValue::new(&mut st.width).range(0..=10));
                });
                ui.checkbox(&mut st.letters, "Letters");
                ui.horizontal(|ui| {
                    ui.label("Save to directory:");
                    ui.add(egui::TextEdit::singleline(&mut st.save_all_to).desired_width(360.0));
                    if ui.button("Browse...").clicked() {
                        if let Some(p) = rfd::FileDialog::new().pick_folder() {
                            st.save_all_to = p.to_string_lossy().to_string();
                        }
                    }
                });
                ui.add_space(4.0);
                if ui.button("Preview").clicked() {
                    st.preview = odm_core::batch::expand_range(&st.url, st.from, st.to, st.width);
                    st.preview.truncate(50);
                }
                if !st.error.is_empty() {
                    ui.colored_label(theme::ODM_RED, &st.error);
                }
                egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
                    for u in &st.preview {
                        ui.label(RichText::new(u).monospace().size(11.0));
                    }
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        submit = true;
                    }
                    if ui.button("Cancel").clicked() {
                        open = false;
                    }
                });
            });
        if submit {
            if !odm_core::batch::has_wildcard(&st.url) {
                st.error = "There is no asterisk wildcard in URL to create a download file list!".into();
            } else if st.to < st.from || st.to - st.from > 1000 {
                st.error = "Too many files. The number of files should not exceed 1000.".into();
            } else {
                let urls = odm_core::batch::expand_range(&st.url, st.from, st.to, st.width);
                for u in &urls {
                    self.engine.add(AddRequest {
                        url: u.clone(),
                        save_dir: if st.save_all_to.is_empty() {
                            None
                        } else {
                            Some(PathBuf::from(&st.save_all_to))
                        },
                        start: true,
                        ..Default::default()
                    });
                }
                self.status = format!("{} files added to the download list.", urls.len());
                open = false;
            }
        }
        if open {
            self.dialogs.batch = Some(st);
        }
    }

    fn dialog_find(&mut self, ctx: &egui::Context) {
        let Some(mut st) = self.dialogs.find.take() else { return };
        let mut open = true;
        let mut find = false;
        egui::Window::new("Find file")
            .id(egui::Id::new("find"))
            .collapsible(false)
            .resizable(false)
            .default_width(360.0)
                        .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Find:");
                    ui.add(egui::TextEdit::singleline(&mut st.text).desired_width(230.0));
                });
                ui.checkbox(&mut st.name, "File name or part of the name");
                ui.checkbox(&mut st.description, "Description or part of the description");
                ui.checkbox(&mut st.site, "Site name/Download link/Parent web page/Referer");
                ui.checkbox(&mut st.match_case, "Match case");
                ui.checkbox(&mut st.whole, "Match whole string only");
                ui.horizontal(|ui| {
                    if ui.button("Find").clicked() {
                        find = true;
                    }
                    if ui.button("Close").clicked() {
                        open = false;
                    }
                });
            });
        if find {
            self.search = st.text.clone();
            self.filter = Filter::All;
            open = false;
        }
        if open {
            self.dialogs.find = Some(st);
        } else if !st.text.is_empty() {
            self.search.clear();
        }
    }

    fn dialog_properties(&mut self, ctx: &egui::Context) {
        let Some(st) = self.dialogs.properties.take() else { return };
        let mut open = true;
        let mut remove = false;
        let mut close_clicked = false;
        egui::Window::new("File Properties")
            .id(egui::Id::new("properties"))
            .collapsible(false)
            .resizable(true)
            .default_width(520.0)
            .open(&mut open)
            .show(ctx, |ui| {
                egui::Grid::new("props").num_columns(2).spacing([10.0, 5.0]).show(ui, |ui| {
                    ui.label("File name:");
                    ui.label(&st.file_name);
                    ui.end_row();
                    ui.label("Address:");
                    ui.add(egui::Label::new(&st.url).wrap());
                    ui.end_row();
                    ui.label("Save To:");
                    ui.label(&st.save_dir);
                    ui.end_row();
                    ui.label("Type:");
                    ui.label(if st.content_type.is_empty() { "(unknown)" } else { &st.content_type });
                    ui.end_row();
                    ui.label("Size:");
                    ui.label(st.size.map(util::format_bytes).unwrap_or_else(|| "(unknown)".into()));
                    ui.end_row();
                    ui.label("Downloaded:");
                    ui.label(util::format_bytes(st.downloaded));
                    ui.end_row();
                    ui.label("Status:");
                    ui.label(&st.status);
                    ui.end_row();
                    ui.label("Connections:");
                    ui.label(st.connections.to_string());
                    ui.end_row();
                    ui.label("Description:");
                    ui.label(if st.description.is_empty() { "(none)" } else { &st.description });
                    ui.end_row();
                    ui.label("Referer:");
                    ui.label(if st.referer.is_empty() { "(none)" } else { &st.referer });
                    ui.end_row();
                    ui.label("Date added:");
                    ui.label(util::format_date(st.created));
                    ui.end_row();
                    ui.label("Last try:");
                    ui.label(st.last_try.map(util::format_date).unwrap_or_default());
                    ui.end_row();
                    ui.label("Completed:");
                    ui.label(st.completed.map(util::format_date).unwrap_or_default());
                    ui.end_row();
                });
                if !st.error.is_empty() {
                    ui.colored_label(theme::ODM_RED, &st.error);
                }
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Open folder").clicked() {
                        let _ = open::that(&st.save_dir);
                    }
                    if ui.button("Copy URL").clicked() {
                        ui.ctx().copy_text(st.url.clone());
                    }
                    if st.status != "Complete" && ui.button("Remove").clicked() {
                        remove = true;
                    }
                    if ui.button("Close").clicked() {
                        close_clicked = true;
                    }
                });
            });
        let _ = remove;
        if close_clicked {
            open = false;
        }
        if open {
            self.dialogs.properties = Some(st);
        }
    }

    fn dialog_complete(&mut self, ctx: &egui::Context) {
        let Some(st) = self.dialogs.complete.take() else { return };
        let mut open = true;
        let mut close_clicked = false;
        egui::Window::new("Download complete")
            .id(egui::Id::new("complete"))
            .collapsible(false)
            .resizable(false)
            .default_width(450.0)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    let (icon_rect, _) =
                        ui.allocate_exact_size(egui::vec2(44.0, 44.0), Sense::hover());
                    let c = icon_rect.center();
                    ui.painter()
                        .circle_filled(c, 20.0, theme::ODM_GREEN.gamma_multiply(0.16));
                    ui.painter().circle_stroke(
                        c,
                        20.0,
                        Stroke::new(1.5, theme::ODM_GREEN),
                    );
                    ui.painter().line_segment(
                        [c + egui::vec2(-8.0, 0.0), c + egui::vec2(-2.0, 7.0)],
                        Stroke::new(2.6, theme::ODM_GREEN),
                    );
                    ui.painter().line_segment(
                        [c + egui::vec2(-2.0, 7.0), c + egui::vec2(9.0, -7.0)],
                        Stroke::new(2.6, theme::ODM_GREEN),
                    );
                    ui.add_space(6.0);
                    ui.label(RichText::new("Download complete").size(16.0).strong());
                    ui.label(RichText::new(&st.file_name).strong());
                });
                ui.add_space(6.0);
                egui::Grid::new("complete_grid")
                    .num_columns(2)
                    .spacing([12.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new("Size").color(theme::muted(ui.visuals().dark_mode)),
                        );
                        ui.label(format!(
                            "{}  ({} bytes)",
                            util::format_bytes(st.size),
                            st.size
                        ));
                        ui.end_row();
                        ui.label(
                            RichText::new("Saved to").color(theme::muted(ui.visuals().dark_mode)),
                        );
                        ui.add(egui::Label::new(st.path.display().to_string()).wrap());
                        ui.end_row();
                    });
                ui.add_space(8.0);
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Open").clicked() {
                        let _ = open::that(&st.path);
                    }
                    if ui.button("Open folder").clicked() {
                        if let Some(dir) = st.path.parent() {
                            let _ = open::that(dir);
                        }
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .add(egui::Button::new("Close").min_size(egui::vec2(86.0, 26.0)))
                            .clicked()
                        {
                            close_clicked = true;
                        }
                    });
                });
            });
        if close_clicked {
            open = false;
        }
        if open {
            self.dialogs.complete = Some(st);
        }
    }

    fn dialog_category(&mut self, ctx: &egui::Context) {
        let Some(mut st) = self.dialogs.category.take() else { return };
        let mut open = true;
        let mut submit = false;
        egui::Window::new("Open Download Manager categories")
            .id(egui::Id::new("category"))
            .collapsible(false)
            .resizable(false)
            .default_width(420.0)
                        .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Category name:");
                    ui.add(egui::TextEdit::singleline(&mut st.name).desired_width(250.0));
                });
                ui.label("Automatically put in this category the following file types:");
                ui.add(egui::TextEdit::singleline(&mut st.masks).desired_width(360.0));
                ui.label(RichText::new("Note: type file extensions separated by space (e.g. avi mpg mpeg)").size(10.0).color(Color32::from_gray(110)));
                ui.horizontal(|ui| {
                    ui.label("Save to folder:");
                    ui.add(egui::TextEdit::singleline(&mut st.folder).desired_width(240.0));
                    if ui.button("Browse...").clicked() {
                        if let Some(p) = rfd::FileDialog::new().pick_folder() {
                            st.folder = p.to_string_lossy().to_string();
                        }
                    }
                });
                ui.label("Automatically put in this category the files from the following sites only:");
                ui.add(egui::TextEdit::singleline(&mut st.sites).desired_width(360.0));
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        submit = true;
                    }
                    if ui.button("Cancel").clicked() {
                        open = false;
                    }
                });
            });
        if submit {
            if st.name.trim().is_empty() {
                self.info = Some(("Categories".into(), "Please fill in the \"Category name\" field.".into()));
                self.dialogs.category = Some(st);
                return;
            }
            let id = if st.edit && !st.id.is_empty() {
                st.id.clone()
            } else {
                st.name.to_ascii_lowercase().replace(' ', "_")
            };
            let cat = Category {
                id: id.clone(),
                name: st.name.clone(),
                folder: PathBuf::from(&st.folder),
                masks: st.masks.split_whitespace().map(|s| s.to_string()).collect(),
                sites: st.sites.split_whitespace().map(|s| s.to_string()).collect(),
                builtin: false,
                remember_last: false,
            };
            if let Some(existing) = self.cfg.categories.iter_mut().find(|c| c.id == id) {
                *existing = cat;
            } else {
                self.cfg.categories.push(cat);
            }
            self.engine.set_config(self.cfg.clone());
        } else if open {
            self.dialogs.category = Some(st);
        }
    }

    fn dialog_columns(&mut self, ctx: &egui::Context) {
        if !self.dialogs.columns {
            return;
        }
        let mut open = true;
        let mut close_clicked = false;
        egui::Window::new("Columns")
            .id(egui::Id::new("columns"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label("Check the columns that you would like visible in this list.");
                ui.checkbox(&mut self.columns.queue, "Queue");
                ui.checkbox(&mut self.columns.name, "File Name");
                ui.checkbox(&mut self.columns.status, "Status");
                ui.checkbox(&mut self.columns.size, "Size");
                ui.checkbox(&mut self.columns.time_left, "Time left");
                ui.checkbox(&mut self.columns.rate, "Transfer rate");
                ui.checkbox(&mut self.columns.description, "Description");
                ui.checkbox(&mut self.columns.date_added, "Date Added");
                ui.checkbox(&mut self.columns.last_try, "Last Try Date");
                ui.checkbox(&mut self.columns.save_to, "Save To");
                ui.checkbox(&mut self.columns.referer, "Referer");
                ui.separator();
                if ui.button("OK").clicked() {
                    close_clicked = true;
                }
            });
        if close_clicked {
            open = false;
        }
        self.dialogs.columns = open;
    }

    fn dialog_speed_limiter(&mut self, ctx: &egui::Context) {
        if !self.dialogs.speed_limiter {
            return;
        }
        let mut open = true;
        let mut cfg = self.cfg.clone();
        egui::Window::new("Speed Limiter")
            .id(egui::Id::new("speed"))
            .collapsible(false)
            .resizable(false)
                        .show(ctx, |ui| {
                ui.checkbox(
                    &mut cfg.speed_limiter_on_startup,
                    "Always turn on Speed Limiter on ODM startup",
                );
                ui.horizontal(|ui| {
                    ui.label("Maximum download speed:");
                    let mut on = cfg.speed_limit_kbps > 0;
                    ui.checkbox(&mut on, "Use Speed Limiter");
                    if on && cfg.speed_limit_kbps == 0 {
                        cfg.speed_limit_kbps = 500;
                    }
                    if !on {
                        cfg.speed_limit_kbps = 0;
                    }
                });
                ui.horizontal(|ui| {
                    ui.add(
                        egui::DragValue::new(&mut cfg.speed_limit_kbps)
                            .range(0..=1_000_000)
                            .speed(10.0),
                    );
                    ui.label("KBytes/sec");
                });
                ui.label(
                    RichText::new(
                        "Note: Internet servers may break connection when the speed is too limited!",
                    )
                    .size(10.0)
                    .color(Color32::from_gray(110)),
                );
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("OK").clicked() {
                        self.engine.set_speed_limit(cfg.speed_limit_kbps);
                        self.cfg = cfg.clone();
                        self.engine.set_config(cfg.clone());
                        open = false;
                    }
                    if ui.button("Cancel").clicked() {
                        open = false;
                    }
                });
            });
        self.dialogs.speed_limiter = open;
    }

    fn dialog_confirm_delete(&mut self, ctx: &egui::Context) {
        let Some(ids) = self.dialogs.confirm_delete.take() else { return };
        let mut open = true;
        let mut yes = false;
        let mut cancel = false;
        egui::Window::new("Confirm deletion of downloads")
            .id(egui::Id::new("confirm_del"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label(if ids.len() == 1 {
                    "Are you sure you want to delete the selected download from the download list?"
                } else {
                    "Are you sure you want to delete the selected downloads from the download list?"
                });
                ui.checkbox(
                    &mut self.dialogs.confirm_delete_files,
                    "Delete completely downloaded files from your hard disk as well. Be careful!",
                );
                ui.separator();
                ui.horizontal(|ui| {
                    if ui
                        .add(egui::Button::new("Yes").min_size(egui::vec2(86.0, 26.0)))
                        .clicked()
                    {
                        yes = true;
                    }
                    if ui
                        .add(egui::Button::new("No").min_size(egui::vec2(86.0, 26.0)))
                        .clicked()
                    {
                        cancel = true;
                    }
                });
            });
        if cancel {
            open = false;
        }
        if yes {
            self.cfg.delete_file_with_item = self.dialogs.confirm_delete_files;
            self.engine
                .remove(ids.clone(), self.dialogs.confirm_delete_files);
            self.engine.set_config(self.cfg.clone());
            self.selected.retain(|id| !ids.contains(id));
        } else if open {
            self.dialogs.confirm_delete = Some(ids);
        }
    }

    fn dialog_about(&mut self, ctx: &egui::Context) {
        if !self.dialogs.about {
            return;
        }
        let mut open = true;
        let mut close_clicked = false;
        egui::Window::new("About Open Download Manager")
            .id(egui::Id::new("about"))
            .collapsible(false)
            .resizable(false)
            .default_width(420.0)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(4.0);
                    ui.label(RichText::new("Open Download Manager").size(18.0).strong());
                    ui.label("Free Rust download manager");
                    ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                    ui.add_space(6.0);
                    ui.label("All features are unlocked and free to use.");
                    ui.add_space(6.0);
                    ui.label(RichText::new("Clean-room Rust implementation for interoperability and study.").size(10.5).color(Color32::from_gray(110)));
                });
                ui.separator();
                if ui.button("OK").clicked() {
                    close_clicked = true;
                }
            });
        if close_clicked {
            open = false;
        }
        self.dialogs.about = open;
    }

    fn dialog_browser_setup(&mut self, ctx: &egui::Context) {
        if !self.dialogs.browser_setup {
            return;
        }
        let mut open = true;
        let mut close = false;
        egui::Window::new("Browser integration")
            .id(egui::Id::new("browser_setup"))
            .collapsible(false)
            .resizable(false)
            .default_width(500.0)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label(
                    RichText::new("Download with ODM in your browser")
                        .size(15.0)
                        .strong(),
                );
                ui.add_space(4.0);
                ui.label(
                    "The extension is installed for Chrome and Edge. Chromium disables \
                     locally installed extensions until you enable them once.",
                );
                ui.add_space(6.0);
                ui.label("1. Open the browser's extensions page.");
                ui.label("2. Turn on \"Open Download Manager Integration\".");
                ui.label("3. Right-click any link and choose \"Download with ODM\".");
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Open Chrome extensions").clicked() {
                        launch_browser_extensions(
                            "chrome://extensions",
                            &[
                                r"C:\Program Files\Google\Chrome\Application\chrome.exe",
                                r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
                            ],
                        );
                    }
                    if ui.button("Open Edge extensions").clicked() {
                        launch_browser_extensions(
                            "edge://extensions",
                            &[
                                r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
                                r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
                            ],
                        );
                    }
                });
                ui.add_space(6.0);
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(
                            "The extension and its registration are removed when ODM is uninstalled.",
                        )
                        .size(10.5)
                        .color(theme::muted(ui.visuals().dark_mode)),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .add(egui::Button::new("Close").min_size(egui::vec2(86.0, 26.0)))
                            .clicked()
                        {
                            close = true;
                        }
                    });
                });
            });
        if close || !open {
            self.dialogs.browser_setup = false;
        }
    }

    fn dialog_tip(&mut self, ctx: &egui::Context) {
        if !self.dialogs.tip {
            return;
        }
        let mut open = true;
        let mut close_clicked = false;
        let tip = self
            .tips
            .get(self.tip_index % self.tips.len().max(1))
            .cloned()
            .unwrap_or_default();
        egui::Window::new("Tip of the Day")
            .id(egui::Id::new("tip"))
            .collapsible(false)
            .resizable(false)
            .default_width(420.0)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label(RichText::new("Did you know...").strong());
                ui.add_space(4.0);
                ui.label(tip);
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Next Tip").clicked() {
                        self.tip_index = self.tip_index.wrapping_add(1);
                    }
                    if ui.button("Close").clicked() {
                        close_clicked = true;
                    }
                });
            });
        if close_clicked {
            open = false;
        }
        self.dialogs.tip = open;
    }

    fn dialog_clipboard(&mut self, ctx: &egui::Context) {
        let Some(url) = self.clip_prompt.clone() else { return };
        let mut open = true;
        let mut add = false;
        egui::Window::new("Open Download Manager")
            .id(egui::Id::new("clip"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label("Download link was found in clipboard. Do you want to download it?");
                ui.label(RichText::new(&url).monospace().size(11.0));
                ui.horizontal(|ui| {
                    if ui.button("Yes").clicked() {
                        add = true;
                    }
                    if ui.button("No").clicked() {
                        // dismiss
                    }
                });
            });
        if add {
            self.add_download(AddRequest {
                url,
                start: true,
                ..Default::default()
            });
        }
        if !open || !add {
            self.clip_prompt = None;
        }
    }

    fn dialog_info(&mut self, ctx: &egui::Context) {
        let Some((title, text)) = self.info.clone() else { return };
        let mut open = true;
        let mut close_clicked = false;
        egui::Window::new(title)
            .id(egui::Id::new("info"))
            .collapsible(false)
            .resizable(false)
            .default_width(400.0)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label(text);
                ui.separator();
                if ui.button("OK").clicked() {
                    close_clicked = true;
                }
            });
        if close_clicked {
            open = false;
        }
        if !open {
            self.info = None;
        }
    }
}

fn launch_browser_extensions(url: &str, candidates: &[&str]) {
    for candidate in candidates {
        if std::path::Path::new(candidate).exists() {
            let _ = std::process::Command::new(candidate).arg(url).spawn();
            return;
        }
    }
}

fn options_general(ui: &mut egui::Ui, st: &mut OptionsState) {
    ui.checkbox(&mut st.cfg.launch_on_startup, "Launch Open Download Manager on startup");
    ui.checkbox(
        &mut st.cfg.monitor_clipboard,
        "Automatically start downloading of URLs placed to clipboard",
    );
    ui.checkbox(&mut st.cfg.close_to_tray, "Minimize on close (tray icon stays available)");
    ui.checkbox(&mut st.cfg.start_minimized, "Start minimized");
    ui.add_space(6.0);
    widgets::group_title(ui, "Browser/System Integration");
    ui.label("Integrate Open Download Manager into browsers:");
    ui.horizontal(|ui| {
        ui.checkbox(&mut st.cfg.first_run, "Detect new applications that try to download files");
        ui.checkbox(&mut st.cfg.check_updates, "Check for updates");
    });
    ui.horizontal(|ui| {
        if ui.button("Install browser integration").clicked() {
            st.status = install_native_host();
        }
        if ui.button("Open extension folder").clicked() {
            let p = std::env::current_exe()
                .ok()
                .and_then(|e| e.parent().map(|p| p.join("extension")))
                .unwrap_or_else(|| PathBuf::from("extension"));
            let _ = open::that(p);
        }
    });
    ui.add_space(6.0);
    widgets::group_title(ui, "Theme");
    ui.horizontal(|ui| {
        ui.radio_value(&mut st.cfg.theme, odm_core::config::ThemeMode::System, "System");
        ui.radio_value(&mut st.cfg.theme, odm_core::config::ThemeMode::Light, "Light");
        ui.radio_value(&mut st.cfg.theme, odm_core::config::ThemeMode::Dark, "Dark");
    });
    ui.add_space(6.0);
    widgets::group_title(ui, "Dialogs");
    ui.checkbox(&mut st.cfg.show_complete_dialog, "Show download complete dialog");
    ui.checkbox(&mut st.cfg.show_start_dialog, "Show start download dialog");
    ui.checkbox(
        &mut st.cfg.start_immediately_with_info,
        "Start downloading immediately while displaying \"Download File Info\" dialog",
    );
}

fn options_file_types(ui: &mut egui::Ui, st: &mut OptionsState) {
    ui.label("Automatically start downloading the following file types:");
    ui.add(
        egui::TextEdit::multiline(&mut st.types_text)
            .desired_rows(8)
            .desired_width(f32::INFINITY),
    );
    ui.label(
        RichText::new("Type file extensions separated by spaces (e.g. avi mpg mpeg).")
            .size(10.0)
            .color(Color32::from_gray(110)),
    );
    ui.add_space(6.0);
    ui.label("Don't start downloading automatically from the following sites:");
    ui.add(egui::TextEdit::multiline(&mut st.sites_text).desired_rows(3).desired_width(f32::INFINITY));
    ui.label(RichText::new("Separate names by spaces. You may use asterisk as a wildcard pattern.").size(10.0).color(Color32::from_gray(110)));
}

fn options_connection(ui: &mut egui::Ui, st: &mut OptionsState) {
    ui.label("Connection Type/Speed");
    ui.radio_value(&mut st.cfg.max_connections, 4, "Low speed: Dial Up modem / ISDN / Bluetooth / Mobile Edge");
    ui.radio_value(&mut st.cfg.max_connections, 8, "Medium speed: ADSL / DSL / Mobile 3G / Wi-Fi");
    ui.radio_value(&mut st.cfg.max_connections, 16, "High speed: Direct connection (Ethernet/Cable) / Mobile 4G");
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label("Max. connections number:");
        ui.add(egui::DragValue::new(&mut st.cfg.max_connections).range(1..=32));
    });
    ui.horizontal(|ui| {
        ui.label("Max. simultaneous downloads:");
        ui.add(egui::DragValue::new(&mut st.cfg.max_concurrent_downloads).range(1..=32));
    });
    ui.add_space(6.0);
    ui.group(|ui| {
        ui.label(RichText::new("Max. connections number for a server").strong());
        ui.label(
            RichText::new("Exceptions override the connection number for matching servers (asterisk wildcard allowed).")
                .size(10.0)
                .color(Color32::from_gray(110)),
        );
        let mut remove = None;
        egui::ScrollArea::vertical().max_height(90.0).show(ui, |ui| {
            for (i, e) in st.cfg.connection_exceptions.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(format!("{}  ->  {}", e.server, e.max));
                    if ui.small_button("x").clicked() {
                        remove = Some(i);
                    }
                });
            }
        });
        if let Some(i) = remove {
            st.cfg.connection_exceptions.remove(i);
        }
        ui.horizontal(|ui| {
            ui.label("Server:");
            ui.add(egui::TextEdit::singleline(&mut st.conn_server).desired_width(180.0));
            ui.label("Number:");
            ui.add(egui::DragValue::new(&mut st.conn_max).range(1..=32));
            if ui.button("Add / Edit").clicked() && !st.conn_server.trim().is_empty() {
                let ex = odm_core::config::ConnectionException {
                    server: st.conn_server.trim().to_string(),
                    max: st.conn_max,
                };
                if let Some(existing) = st
                    .cfg
                    .connection_exceptions
                    .iter_mut()
                    .find(|x| x.server.eq_ignore_ascii_case(&ex.server))
                {
                    *existing = ex;
                } else {
                    st.cfg.connection_exceptions.push(ex);
                }
            }
        });
    });
    ui.add_space(6.0);
    ui.label(
        RichText::new(
            "Please use a high number of connections with caution. Some servers may have specific limitations for a number of connections from one client.",
        )
        .size(10.0)
        .color(Color32::from_gray(110)),
    );
}

fn options_proxy(ui: &mut egui::Ui, st: &mut OptionsState) {
    ui.radio_value(&mut st.cfg.proxy.mode, ProxyMode::None, "No proxy/socks");
    ui.radio_value(&mut st.cfg.proxy.mode, ProxyMode::System, "Use system settings");
    ui.radio_value(&mut st.cfg.proxy.mode, ProxyMode::Manual, "Manual proxy/socks configuration");
    ui.radio_value(&mut st.cfg.proxy.mode, ProxyMode::Auto, "Automatically detect proxy settings");
    ui.add_space(6.0);
    ui.group(|ui| {
        ui.checkbox(&mut st.cfg.proxy.use_socks, "Use socks");
        ui.horizontal(|ui| {
            ui.label("Socks server:");
            ui.add(egui::TextEdit::singleline(&mut st.cfg.proxy.socks_host).desired_width(150.0));
            ui.label("Port:");
            ui.add(egui::DragValue::new(&mut st.cfg.proxy.socks_port).range(1..=65535));
        });
        ui.horizontal(|ui| {
            ui.label("User:");
            ui.add(egui::TextEdit::singleline(&mut st.cfg.proxy.socks_user).desired_width(100.0));
            ui.label("Password:");
            ui.add(egui::TextEdit::singleline(&mut st.cfg.proxy.socks_pass).password(true).desired_width(100.0));
        });
    });
    ui.add_space(4.0);
    ui.group(|ui| {
        ui.label(RichText::new("HTTP proxy").strong());
        ui.horizontal(|ui| {
            ui.label("Proxy:");
            ui.add(egui::TextEdit::singleline(&mut st.cfg.proxy.http_host).desired_width(150.0));
            ui.label("Port:");
            ui.add(egui::DragValue::new(&mut st.cfg.proxy.http_port).range(1..=65535));
        });
        ui.horizontal(|ui| {
            ui.label("UserName:");
            ui.add(egui::TextEdit::singleline(&mut st.cfg.proxy.http_user).desired_width(100.0));
            ui.label("Password:");
            ui.add(egui::TextEdit::singleline(&mut st.cfg.proxy.http_pass).password(true).desired_width(100.0));
        });
    });
    ui.add_space(4.0);
    ui.label("Do not use proxy server for addresses beginning with:");
    ui.add(egui::TextEdit::singleline(&mut st.sites_text).desired_width(f32::INFINITY));
}

fn options_logins(ui: &mut egui::Ui, st: &mut OptionsState) {
    ui.label("User names and passwords for servers/sites");
    let mut remove = None;
    egui::ScrollArea::vertical().max_height(150.0).show(ui, |ui| {
        for (i, l) in st.cfg.sites.iter().enumerate() {
            ui.horizontal(|ui| {
                if ui.selectable_label(st.selected_login == Some(i), format!("{} — {}", l.site, l.user)).clicked() {
                    st.selected_login = Some(i);
                    st.login_site = l.site.clone();
                    st.login_user = l.user.clone();
                    st.login_pass = l.password.clone();
                }
                if ui.small_button("x").clicked() {
                    remove = Some(i);
                }
            });
        }
        if st.cfg.sites.is_empty() {
            ui.label("(no saved logins)");
        }
    });
    if let Some(i) = remove {
        st.cfg.sites.remove(i);
        st.selected_login = None;
    }
    ui.separator();
    ui.horizontal(|ui| {
        ui.label("Site/path:");
        ui.add(egui::TextEdit::singleline(&mut st.login_site).desired_width(180.0));
    });
    ui.horizontal(|ui| {
        ui.label("User:");
        ui.add(egui::TextEdit::singleline(&mut st.login_user).desired_width(120.0));
        ui.label("Password:");
        ui.add(egui::TextEdit::singleline(&mut st.login_pass).password(true).desired_width(120.0));
    });
    ui.horizontal(|ui| {
        if ui.button("New").clicked() {
            st.login_site.clear();
            st.login_user.clear();
            st.login_pass.clear();
            st.selected_login = None;
        }
        if ui.button("Add / Save").clicked() && !st.login_site.is_empty() {
            let login = SiteLogin {
                site: st.login_site.clone(),
                user: st.login_user.clone(),
                password: st.login_pass.clone(),
            };
            if let Some(i) = st.selected_login {
                st.cfg.sites[i] = login;
            } else {
                st.cfg.sites.push(login);
            }
        }
        if ui.button("Remove").clicked() {
            if let Some(i) = st.selected_login {
                st.cfg.sites.remove(i);
                st.selected_login = None;
            }
        }
    });
}

fn options_dialup(ui: &mut egui::Ui, st: &mut OptionsState) {
    ui.checkbox(&mut st.cfg.dial_up_enabled, "Use Windows Dial Up / VPN Networking");
    ui.horizontal(|ui| {
        ui.label("Connection:");
        ui.add(egui::TextEdit::singleline(&mut st.cfg.dial_up_connection).desired_width(200.0));
    });
    ui.horizontal(|ui| {
        ui.label("User name:");
        ui.add(egui::TextEdit::singleline(&mut st.cfg.dial_up_user).desired_width(140.0));
        ui.label("Password:");
        ui.add(egui::TextEdit::singleline(&mut st.cfg.dial_up_pass).password(true).desired_width(140.0));
    });
    ui.horizontal(|ui| {
        ui.label("Redial attempts:");
        ui.add(egui::DragValue::new(&mut st.cfg.dial_up_retries).range(0..=100));
        ui.label("Time between attempts (sec):");
        ui.add(egui::DragValue::new(&mut st.cfg.dial_up_interval).range(1..=600));
    });
    ui.checkbox(&mut st.cfg.dial_up_hangup, "Hang up modem when done");
}

fn options_sounds(ui: &mut egui::Ui, st: &mut OptionsState) {
    ui.checkbox(&mut st.cfg.sounds.enabled, "Select sounds for Open Download Manager events");
    let rows = [
        ("Download complete", 0),
        ("Download failed", 1),
        ("Queue processing started", 2),
        ("Queue processing stopped/finished", 3),
    ];
    for (label, idx) in rows {
        ui.horizontal(|ui| {
            ui.label(label);
            let p = match idx {
                0 => st.cfg.sounds.complete.clone(),
                1 => st.cfg.sounds.failed.clone(),
                2 => st.cfg.sounds.queue_started.clone(),
                _ => st.cfg.sounds.queue_stopped.clone(),
            };
            if ui.button("Browse...").clicked() {
                if let Some(file) = rfd::FileDialog::new()
                    .add_filter("Sound Files", &["wav", "mp3"])
                    .pick_file()
                {
                    match idx {
                        0 => st.cfg.sounds.complete = Some(file),
                        1 => st.cfg.sounds.failed = Some(file),
                        2 => st.cfg.sounds.queue_started = Some(file),
                        _ => st.cfg.sounds.queue_stopped = Some(file),
                    }
                }
            }
            if ui.button("Play").clicked() {
                if let Some(p) = p {
                    let _ = open::that(p);
                }
            }
        });
    }
}

fn options_downloads(ui: &mut egui::Ui, st: &mut OptionsState) {
    ui.horizontal(|ui| {
        ui.label("Default download directory:");
        let mut dir = st.cfg.download_dir.to_string_lossy().to_string();
        if ui.add(egui::TextEdit::singleline(&mut dir).desired_width(300.0)).changed() {
            st.cfg.download_dir = PathBuf::from(dir);
        }
        if ui.button("Browse...").clicked() {
            if let Some(p) = rfd::FileDialog::new().pick_folder() {
                st.cfg.download_dir = p;
            }
        }
    });
    ui.horizontal(|ui| {
        ui.label("Temporary directory:");
        let mut dir = st.cfg.temp_dir.to_string_lossy().to_string();
        if ui.add(egui::TextEdit::singleline(&mut dir).desired_width(300.0)).changed() {
            st.cfg.temp_dir = PathBuf::from(dir);
        }
        if ui.button("Browse...").clicked() {
            if let Some(p) = rfd::FileDialog::new().pick_folder() {
                st.cfg.temp_dir = p;
            }
        }
    });
    ui.add_space(4.0);
    ui.label("Categories, file types, folders:");
    let cats = st.cfg.categories.clone();
    for (i, c) in cats.iter().enumerate() {
        ui.horizontal(|ui| {
            if ui.selectable_label(st.category_sel == i, &c.name).clicked() {
                st.category_sel = i;
            }
            ui.label(RichText::new(c.folder.to_string_lossy()).size(10.5).color(Color32::from_gray(110)));
        });
    }
    let cat_idx = st.category_sel.min(st.cfg.categories.len().saturating_sub(1)); if let Some(c) = st.cfg.categories.get_mut(cat_idx) {
        ui.horizontal(|ui| {
            ui.label("Folder:");
            let mut f = c.folder.to_string_lossy().to_string();
            if ui.add(egui::TextEdit::singleline(&mut f).desired_width(280.0)).changed() {
                c.folder = PathBuf::from(f);
            }
        });
        ui.horizontal(|ui| {
            ui.label("File types:");
            let mut m = c.masks.join(" ");
            if ui.add(egui::TextEdit::singleline(&mut m).desired_width(280.0)).changed() {
                c.masks = m.split_whitespace().map(|s| s.to_string()).collect();
            }
        });
    }
    ui.add_space(6.0);
    ui.group(|ui| {
        ui.label(RichText::new("Virus checking").strong());
        ui.checkbox(&mut st.cfg.virus_enabled, "Run virus scanner on download completion");
        ui.horizontal(|ui| {
            ui.label("Virus scanner program:");
            ui.add(egui::TextEdit::singleline(&mut st.cfg.virus_scanner).desired_width(260.0));
            if ui.button("Browse...").clicked() {
                if let Some(p) = rfd::FileDialog::new().add_filter("Programs", &["exe"]).pick_file() {
                    st.cfg.virus_scanner = p.to_string_lossy().to_string();
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label("Command line parameters:");
            ui.add(egui::TextEdit::singleline(&mut st.cfg.virus_args).desired_width(260.0));
        });
    });
    ui.add_space(6.0);
    ui.group(|ui| {
        ui.label(RichText::new("Duplicate download link").strong());
        ui.radio_value(&mut st.cfg.duplicate_mode, 0, "Show a dialog and ask what to do");
        ui.radio_value(&mut st.cfg.duplicate_mode, 1, "Add the duplicate with a numbered file name");
        ui.radio_value(&mut st.cfg.duplicate_mode, 2, "Add the duplicate and overwrite the existing file");
        ui.radio_value(&mut st.cfg.duplicate_mode, 3, "If existing file is complete, show download complete dialog");
    });
}

fn options_browser(ui: &mut egui::Ui, st: &mut OptionsState) {
    ui.label(RichText::new("Open Download Manager integration into browsers").strong());
    ui.label(
        "The Open Download Manager extension takes over downloads from Chrome, Edge, Firefox and other \
         Chromium-based browsers. Click the button below to install the native messaging host, \
         then load the unpacked extension from the extension folder.",
    );
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        if ui.button("Install native messaging host").clicked() {
            st.status = install_native_host();
        }
        if ui.button("Open extension folder").clicked() {
            let p = std::env::current_exe()
                .ok()
                .and_then(|e| e.parent().map(|p| p.join("extension")))
                .unwrap_or_else(|| PathBuf::from("extension"));
            let _ = open::that(p);
        }
    });
    ui.add_space(6.0);
    ui.label("Capture downloads from the following browsers:");
    for b in [
        "Google Chrome",
        "Microsoft Edge",
        "Mozilla Firefox",
        "Opera",
        "Brave",
        "Vivaldi",
    ] {
        let mut on = true;
        ui.checkbox(&mut on, b);
    }
    ui.add_space(6.0);
    ui.checkbox(&mut st.cfg.monitor_clipboard, "Monitor system Clipboard for links to download with ODM");
}

fn install_native_host() -> String {
    let exe = std::env::current_exe().ok();
    let dir = exe.as_ref().and_then(|e| e.parent()).map(|p| p.to_path_buf());
    let host = dir
        .as_ref()
        .map(|d| d.join("odm-native-host.exe"))
        .filter(|p| p.exists());
    match host {
        Some(h) => match std::process::Command::new(h).arg("--install").output() {
            Ok(o) if o.status.success() => {
                String::from_utf8_lossy(&o.stdout).trim().to_string()
            }
            Ok(o) => format!("Install failed: {}", String::from_utf8_lossy(&o.stderr)),
            Err(e) => format!("Cannot run native host: {e}"),
        },
        None => "odm-native-host.exe was not found next to the application.".into(),
    }
}
