//! Main window: menus, toolbar, category tree, download list and status bar.

use crate::ui::dialogs::{AddUrlState, Dialogs};
use crate::ui::grabber_ui::GrabberState;
use crate::ui::icons::{self, Icon};
use crate::ui::theme;
use crate::ui::tray;
use crate::ui::widgets;
use eframe::egui::{
    self, Align, Align2, Color32, FontId, Key, Layout, Pos2, Rect, RichText, Sense, Stroke,
    Vec2, ViewportCommand,
};
use odm_core::config::ToolbarStyle;
use odm_core::{AddRequest, Config, Download, Engine, EngineEvent, Status};
use std::collections::BTreeSet;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SortCol {
    Order,
    Queue,
    FileName,
    Status,
    Size,
    TimeLeft,
    Rate,
    Description,
    DateAdded,
    LastTry,
    SaveTo,
    Referer,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Filter {
    All,
    Unfinished,
    Finished,
    Category(String),
    Queue(u64),
}

pub struct Columns {
    pub queue: bool,
    pub name: bool,
    pub status: bool,
    pub size: bool,
    pub time_left: bool,
    pub rate: bool,
    pub description: bool,
    pub date_added: bool,
    pub last_try: bool,
    pub save_to: bool,
    pub referer: bool,
    pub name_w: f32,
    pub desc_w: f32,
}

impl Default for Columns {
    fn default() -> Self {
        Self {
            queue: true,
            name: true,
            status: true,
            size: true,
            time_left: true,
            rate: true,
            description: true,
            date_added: false,
            last_try: false,
            save_to: false,
            referer: false,
            name_w: 330.0,
            desc_w: 220.0,
        }
    }
}

pub struct IdmApp {
    pub engine: Engine,
    pub cfg: Config,
    pub downloads: Vec<Download>,
    pub events: std::sync::mpsc::Receiver<EngineEvent>,
    pub filter: Filter,
    pub selected: BTreeSet<u64>,
    pub anchor: Option<u64>,
    pub sort: SortCol,
    pub sort_desc: bool,
    pub status: String,
    pub dark: bool,
    pub columns: Columns,
    pub dialogs: Dialogs,
    pub tray: Option<tray::Tray>,
    pub hidden: bool,
    pub last_refresh: Instant,
    pub last_clip: String,
    pub clip_prompt: Option<String>,
    pub search: String,
    pub grabber: Option<GrabberState>,
    pub tips: Vec<String>,
    pub tip_index: usize,
    pub info: Option<(String, String)>,
    pub clip: Option<arboard::Clipboard>,
    pub last_clip_check: Instant,
    pub hwnd: Option<isize>,
    pub hwnd_shared: std::sync::Arc<std::sync::atomic::AtomicIsize>,
    pub watcher_started: bool,
    pub progress_auto: std::collections::HashSet<u64>,
}

impl IdmApp {
    pub fn new(cc: &eframe::CreationContext<'_>, engine: Engine, events: std::sync::mpsc::Receiver<EngineEvent>) -> Self {
        theme::install(&cc.egui_ctx);
        let cfg = engine.config();
        let dark = match cfg.theme {
            odm_core::config::ThemeMode::Dark => true,
            odm_core::config::ThemeMode::Light => false,
            odm_core::config::ThemeMode::System => theme::windows_dark_mode(),
        };
        theme::apply(&cc.egui_ctx, dark);
        let downloads = engine.snapshot();
        let tips = load_tips();
        Self {
            engine,
            cfg,
            downloads,
            events,
            filter: Filter::All,
            selected: BTreeSet::new(),
            anchor: None,
            sort: SortCol::Order,
            sort_desc: false,
            status: String::new(),
            dark,
            columns: Columns::default(),
            dialogs: Dialogs::default(),
            tray: tray::create(),
            hidden: false,
            last_refresh: Instant::now() - Duration::from_secs(1),
            last_clip: String::new(),
            clip_prompt: None,
            search: String::new(),
            grabber: None,
            tips,
            tip_index: 0,
            info: None,
            clip: arboard::Clipboard::new().ok(),
            last_clip_check: Instant::now(),
            hwnd: None,
            hwnd_shared: std::sync::Arc::new(std::sync::atomic::AtomicIsize::new(0)),
            watcher_started: false,
            progress_auto: std::collections::HashSet::new(),
        }
    }

    // ---------------------------------------------------------------- events

    fn poll(&mut self, ctx: &egui::Context) {
        while let Ok(ev) = self.events.try_recv() {
            match ev {
                EngineEvent::ShowWindow => {
                    self.restore(ctx);
                }
                EngineEvent::Started(id) => {
                    if self.cfg.show_progress_dialog {
                        self.progress_auto.insert(id);
                        self.open_progress(id);
                    }
                }
                EngineEvent::Completed(id) => {
                    if self.cfg.show_complete_dialog {
                        self.dialogs.progress_windows.retain(|x| *x != id);
                    }
                    if let Some(d) = self.downloads.iter().find(|d| d.id == id).cloned() {
                        self.status = format!("{} download complete.", d.file_name);
                    }
                    if self.cfg.show_complete_dialog {
                        if let Some(d) = self.engine.snapshot().into_iter().find(|d| d.id == id) {
                            self.dialogs.complete = Some(crate::ui::dialogs::CompleteState {
                                file_name: d.file_name.clone(),
                                path: d.dest_path(),
                                size: d.size.unwrap_or(d.downloaded),
                            });
                        }
                    }
                }
                EngineEvent::Failed(id, msg) => {
                    if let Some(d) = self.downloads.iter().find(|d| d.id == id) {
                        self.status = format!("{}: {}", d.file_name, msg);
                    }
                }
                EngineEvent::Added(id) => {
                    if let Some(d) = self.engine.snapshot().into_iter().find(|d| d.id == id) {
                        self.status = format!("Added {}", d.file_name);
                    }
                }
                _ => {}
            }
        }
        if self.last_refresh.elapsed() > Duration::from_millis(250) {
            self.last_refresh = Instant::now();
            self.downloads = self.engine.snapshot();
            self.cfg = self.engine.config();
            if self.cfg.show_progress_dialog {
                let mut active = std::collections::HashSet::new();
                let mut to_open = Vec::new();
                for d in &self.downloads {
                    if d.status.is_active() {
                        active.insert(d.id);
                        if !self.progress_auto.contains(&d.id) {
                            to_open.push(d.id);
                        }
                    }
                }
                self.progress_auto.retain(|id| active.contains(id));
                for id in to_open {
                    self.progress_auto.insert(id);
                    self.open_progress(id);
                }
            }
        }
    }

    pub fn open_progress(&mut self, id: u64) {
        if !self.dialogs.progress_windows.contains(&id) {
            self.dialogs.progress_windows.push(id);
        }
    }

    fn total_speed(&self) -> u64 {
        self.downloads.iter().map(|d| d.speed).sum()
    }

    fn selected_downloads(&self) -> Vec<Download> {
        self.downloads
            .iter()
            .filter(|d| self.selected.contains(&d.id))
            .cloned()
            .collect()
    }

    fn visible_ids(&self) -> Vec<u64> {
        let mut v: Vec<&Download> = self
            .downloads
            .iter()
            .filter(|d| match &self.filter {
                Filter::All => true,
                Filter::Unfinished => d.status != Status::Complete,
                Filter::Finished => d.status == Status::Complete,
                Filter::Category(c) => &d.category == c,
                Filter::Queue(q) => d.queue_id == *q,
            })
            .filter(|d| {
                if self.search.is_empty() {
                    return true;
                }
                let s = self.search.to_ascii_lowercase();
                d.file_name.to_ascii_lowercase().contains(&s)
                    || d.url.to_ascii_lowercase().contains(&s)
                    || d.description.to_ascii_lowercase().contains(&s)
            })
            .collect();
        let desc = self.sort_desc;
        let col = self.sort;
        v.sort_by(|a, b| {
            let ord = match col {
                SortCol::Order => a.id.cmp(&b.id),
                SortCol::Queue => a.queue_id.cmp(&b.queue_id),
                SortCol::FileName => a
                    .file_name
                    .to_ascii_lowercase()
                    .cmp(&b.file_name.to_ascii_lowercase()),
                SortCol::Status => a
                    .status
                    .order()
                    .cmp(&b.status.order())
                    .then(a.file_name.cmp(&b.file_name)),
                SortCol::Size => a.size.unwrap_or(0).cmp(&b.size.unwrap_or(0)),
                SortCol::TimeLeft => a
                    .time_left()
                    .unwrap_or(u64::MAX)
                    .cmp(&b.time_left().unwrap_or(u64::MAX)),
                SortCol::Rate => a.speed.cmp(&b.speed),
                SortCol::Description => a
                    .description
                    .to_ascii_lowercase()
                    .cmp(&b.description.to_ascii_lowercase()),
                SortCol::DateAdded => a.created.cmp(&b.created),
                SortCol::LastTry => a.last_try.unwrap_or(0).cmp(&b.last_try.unwrap_or(0)),
                SortCol::SaveTo => a.save_dir.cmp(&b.save_dir),
                SortCol::Referer => a.referer.cmp(&b.referer),
            };
            if desc {
                ord.reverse()
            } else {
                ord
            }
        });
        v.into_iter().map(|d| d.id).collect()
    }

    fn set_sort(&mut self, col: SortCol) {
        if self.sort == col {
            self.sort_desc = !self.sort_desc;
        } else {
            self.sort = col;
            self.sort_desc = false;
        }
    }

    // --------------------------------------------------------------- actions

    pub fn open_add(&mut self) {
        let dir = self
            .cfg
            .category(&self.cfg.last_category)
            .map(|c| c.folder.clone())
            .unwrap_or_else(|| self.cfg.download_dir.clone());
        let clip = self.clipboard_url().unwrap_or_default();
        self.dialogs.add_url = Some(AddUrlState {
            url: clip,
            save_dir: dir.to_string_lossy().to_string(),
            category: self.cfg.last_category.clone(),
            queue: 1,
            start_later: false,
            ..Default::default()
        });
    }

    fn clipboard_url(&mut self) -> Option<String> {
        self.clip
            .as_mut()
            .and_then(|c| c.get_text().ok())
            .filter(|t| t.starts_with("http://") || t.starts_with("https://"))
            .map(|t| t.trim().to_string())
    }

    pub fn add_download(&mut self, req: AddRequest) {
        let id = self.engine.add(req);
        if id > 0 {
            self.selected.clear();
            self.selected.insert(id);
            self.status = "Download added.".into();
        } else {
            self.info = Some(("Error".into(), "Cannot download this file. Invalid URL entered.".into()));
        }
    }

    fn start_selected(&mut self) {
        for d in self.selected_downloads() {
            if matches!(
                d.status,
                Status::Stopped | Status::Paused | Status::Error | Status::Queued
            ) {
                self.engine.start(d.id);
            }
        }
    }

    fn stop_selected(&mut self) {
        for d in self.selected_downloads() {
            if d.status.is_active() || d.status == Status::Queued {
                self.engine.stop(d.id, false);
            }
        }
    }

    fn pause_selected(&mut self) {
        for d in self.selected_downloads() {
            if d.status.is_active() {
                self.engine.stop(d.id, true);
            }
        }
    }

    fn remove_selected(&mut self) {
        let ids: Vec<u64> = self.selected.iter().copied().collect();
        if ids.is_empty() {
            return;
        }
        if self.cfg.confirm_delete {
            self.dialogs.confirm_delete_files = self.cfg.delete_file_with_item;
            self.dialogs.confirm_delete = Some(ids);
        } else {
            self.engine.remove(ids.clone(), self.cfg.delete_file_with_item);
            self.selected.retain(|id| !ids.contains(id));
        }
    }

    fn delete_completed(&mut self) {
        let ids: Vec<u64> = self
            .downloads
            .iter()
            .filter(|d| d.status == Status::Complete)
            .map(|d| d.id)
            .collect();
        if !ids.is_empty() {
            self.engine.remove(ids.clone(), false);
            self.selected.retain(|id| !ids.contains(id));
        }
    }

    fn select_all(&mut self) {
        self.selected = self.visible_ids().into_iter().collect();
    }

    fn open_selected(&mut self) {
        if let Some(d) = self.selected_downloads().first() {
            if d.status == Status::Complete {
                let _ = open::that(d.dest_path());
            } else {
                self.open_progress(d.id);
            }
        }
    }

    fn show_properties(&mut self) {
        if let Some(d) = self.selected_downloads().first() {
            self.dialogs.properties = Some(crate::ui::dialogs::PropertiesState::from(d));
        }
    }

    // ------------------------------------------------------------------- UI

    fn menu_bar(&mut self, ctx: &egui::Context) {
        let dark = self.dark;
        egui::TopBottomPanel::top("menubar")
            .exact_height(27.0)
            .frame(
                egui::Frame::none()
                    .fill(if dark { Color32::from_rgb(40, 40, 40) } else { Color32::from_rgb(240, 240, 240) })
                    .inner_margin(egui::Margin::symmetric(8, 3)),
            )
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.menu_button("Tasks", |ui| {
                        if ui.button("Add new download...\tCtrl+N").clicked() {
                            self.open_add();
                            ui.close_menu();
                        }
                        ui.separator();
                        ui.menu_button("Import", |ui| {
                            if ui.button("From ODM export file...").clicked() {
                                self.import_links(true);
                                ui.close_menu();
                            }
                            if ui.button("From text file...").clicked() {
                                self.import_links(false);
                                ui.close_menu();
                            }
                        });
                        ui.menu_button("Export", |ui| {
                            if ui.button("To ODM export file...").clicked() {
                                self.export_links(true);
                                ui.close_menu();
                            }
                            if ui.button("To text file...").clicked() {
                                self.export_links(false);
                                ui.close_menu();
                            }
                        });
                        ui.separator();
                        if ui.button("Run site grabber").clicked() {
                            self.open_grabber();
                            ui.close_menu();
                        }
                        if ui.button("Batch download...").clicked() {
                            self.dialogs.batch = Some(Default::default());
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Exit").clicked() {
                            ctx.send_viewport_cmd(ViewportCommand::Close);
                        }
                    });
                    ui.menu_button("File", |ui| {
                        if ui.button("Stop Download").clicked() {
                            self.stop_selected();
                            ui.close_menu();
                        }
                        if ui.button("Remove").clicked() {
                            self.remove_selected();
                            ui.close_menu();
                        }
                        if ui.button("Download again").clicked() {
                            self.restart_selected();
                            ui.close_menu();
                        }
                        if ui.button("Download Now").clicked() {
                            self.start_selected();
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Properties").clicked() {
                            self.show_properties();
                            ui.close_menu();
                        }
                    });
                    ui.menu_button("Downloads", |ui| {
                        if ui.button("Pause All").clicked() {
                            for d in self.downloads.clone() {
                                if d.status.is_active() {
                                    self.engine.stop(d.id, true);
                                }
                            }
                            ui.close_menu();
                        }
                        if ui.button("Stop All").clicked() {
                            self.engine.stop_all();
                            ui.close_menu();
                        }
                        if ui.button("Start Queue").clicked() {
                            self.engine.start_queue(1);
                            ui.close_menu();
                        }
                        if ui.button("Stop Queue").clicked() {
                            self.engine.stop_queue(1);
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Delete selected...").clicked() {
                            self.remove_selected();
                            ui.close_menu();
                        }
                        if ui.button("Download again").clicked() {
                            self.restart_selected();
                            ui.close_menu();
                        }
                        if ui.button("Delete All Completed").clicked() {
                            self.delete_completed();
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Scheduler and queues...").clicked() {
                            self.dialogs.scheduler = true;
                            ui.close_menu();
                        }
                        if ui.button("Options...").clicked() {
                            self.open_options();
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Find (Ctrl-F)").clicked() {
                            self.dialogs.find = Some(Default::default());
                            ui.close_menu();
                        }
                        if ui.button("Find Next (F3)").clicked() {
                            self.find_next();
                            ui.close_menu();
                        }
                    });
                    ui.menu_button("View", |ui| {
                        let mut show = self.cfg.show_categories;
                        if ui.checkbox(&mut show, "Categories").changed() {
                            self.cfg.show_categories = show;
                            self.engine.set_config(self.cfg.clone());
                        }
                        ui.separator();
                        ui.menu_button("Arrange files", |ui| {
                            for (label, col) in [
                                ("By Order Of Addition", SortCol::Order),
                                ("By File Name", SortCol::FileName),
                                ("By Size", SortCol::Size),
                                ("By Status", SortCol::Status),
                                ("By Time Left", SortCol::TimeLeft),
                                ("By Transfer Rate", SortCol::Rate),
                                ("By Description", SortCol::Description),
                                ("By Save Path", SortCol::SaveTo),
                                ("By Date Added", SortCol::DateAdded),
                                ("By Last Try Date", SortCol::LastTry),
                            ] {
                                if ui.button(label).clicked() {
                                    self.sort = col;
                                    self.sort_desc = false;
                                    ui.close_menu();
                                }
                            }
                        });
                        ui.menu_button("Toolbar", |ui| {
                            for (label, style) in [
                                ("Classic Buttons", ToolbarStyle::Classic),
                                ("Small Buttons", ToolbarStyle::Small),
                                ("Large Buttons", ToolbarStyle::Large),
                            ] {
                                let mut sel = self.cfg.toolbar_style == style;
                                if ui.radio_value(&mut sel, true, label).clicked() {
                                    self.cfg.toolbar_style = style;
                                    self.engine.set_config(self.cfg.clone());
                                    ui.close_menu();
                                }
                            }
                        });
                        if ui.button("Columns...").clicked() {
                            self.dialogs.columns = true;
                            ui.close_menu();
                        }
                        ui.separator();
                        let mut dark = self.dark;
                        if ui.checkbox(&mut dark, "Dark Mode").changed() {
                            self.dark = dark;
                            theme::apply(ctx, dark);
                            self.cfg.theme = if dark {
                                odm_core::config::ThemeMode::Dark
                            } else {
                                odm_core::config::ThemeMode::Light
                            };
                            self.engine.set_config(self.cfg.clone());
                        }
                        ui.separator();
                        ui.menu_button("Language", |ui| {
                            ui.label("English");
                        });
                    });
                    ui.menu_button("Help", |ui| {
                        if ui.button("Tutorials").clicked() {
                            self.info = Some((
                                "Tutorials".into(),
                                "Open Download Manager documentation is in the docs folder next to the application.\n\n\
                                 Use Tasks -> Add new download (Ctrl+N) to add a URL, or let the browser \
                                 extension capture downloads."
                                    .into(),
                            ));
                            ui.close_menu();
                        }
                        if ui.button("Tip of the Day...").clicked() {
                            self.dialogs.tip = true;
                            ui.close_menu();
                        }
                        if ui.button("Project folder").clicked() {
                            if let Ok(exe) = std::env::current_exe() {
                                if let Some(dir) = exe.parent() {
                                    let _ = open::that(dir);
                                }
                            }
                            ui.close_menu();
                        }
                        if ui.button("Check for updates...").clicked() {
                            self.info = Some((
                                "Check for updates".into(),
                                "You are using the latest version of Open Download Manager (1.0.0).\n\
                                 This is a free build."
                                    .into(),
                            ));
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("About Open Download Manager...").clicked() {
                            self.dialogs.about = true;
                            ui.close_menu();
                        }
                    });
                });
            });
    }

    fn toolbar(&mut self, ctx: &egui::Context) {
        let dark = self.dark;
        let large = self.cfg.toolbar_style != ToolbarStyle::Small;
        let classic = self.cfg.toolbar_style == ToolbarStyle::Classic;
        let h = match self.cfg.toolbar_style {
            ToolbarStyle::Large => 60.0,
            ToolbarStyle::Small => 34.0,
            ToolbarStyle::Classic => 44.0,
        };
        egui::TopBottomPanel::top("toolbar")
            .exact_height(h)
            .frame(
                egui::Frame::none()
                    .fill(if dark { theme::TOOLBAR_DARK } else { theme::TOOLBAR_LIGHT })
                    .inner_margin(egui::Margin::symmetric(6, 3))
                    .stroke(Stroke::new(1.0, if dark { Color32::from_gray(70) } else { Color32::from_gray(180) })),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    let has = !self.selected.is_empty();
                    let can_start = self.selected_downloads().iter().any(|d| {
                        matches!(
                            d.status,
                            Status::Stopped | Status::Paused | Status::Error | Status::Queued
                        )
                    });
                    let can_stop = self.selected_downloads().iter().any(|d| d.status.is_active());
                    if widgets::toolbar_button(ui, Icon::AddUrl, "Add URL", true, large, dark).clicked() {
                        self.open_add();
                    }
                    if widgets::toolbar_button(ui, Icon::Resume, "Resume", can_start, large, dark).clicked() {
                        self.start_selected();
                    }
                    if widgets::toolbar_button(ui, Icon::Stop, "Stop", can_stop, large, dark).clicked() {
                        self.stop_selected();
                    }
                    if widgets::toolbar_button(ui, Icon::StopAll, "Stop All", true, large, dark).clicked() {
                        self.engine.stop_all();
                    }
                    if widgets::toolbar_button(ui, Icon::Delete, "Delete", has, large, dark).clicked() {
                        self.remove_selected();
                    }
                    ui.separator();
                    if widgets::toolbar_button(ui, Icon::Options, "Options", true, large, dark).clicked() {
                        self.open_options();
                    }
                    if widgets::toolbar_button(ui, Icon::Scheduler, "Scheduler", true, large, dark).clicked() {
                        self.dialogs.scheduler = true;
                    }
                    if widgets::toolbar_button(ui, Icon::QueueStart, "Start Queue", true, large, dark).clicked() {
                        self.engine.start_queue(1);
                    }
                    if widgets::toolbar_button(ui, Icon::QueueStop, "Stop Queue", true, large, dark).clicked() {
                        self.engine.stop_queue(1);
                    }
                    if widgets::toolbar_button(ui, Icon::Grabber, "Grabber", true, large, dark).clicked() {
                        self.open_grabber();
                    }
                    if !classic {
                        ui.separator();
                        if widgets::toolbar_button(ui, Icon::Speed, "Speed Limiter", true, large, dark).clicked() {
                            self.dialogs.speed_limiter = true;
                        }
                        if widgets::toolbar_button(ui, Icon::Find, "Find", true, large, dark).clicked() {
                            self.dialogs.find = Some(Default::default());
                        }
                    }
                });
            });
    }

    fn side_panel(&mut self, ctx: &egui::Context) {
        if !self.cfg.show_categories {
            return;
        }
        let dark = self.dark;
        egui::SidePanel::left("categories")
            .default_width(196.0)
            .width_range(140.0..=320.0)
            .resizable(true)
            .frame(
                egui::Frame::none()
                    .fill(if dark { theme::PANEL_DARK } else { theme::PANEL_LIGHT })
                    .inner_margin(egui::Margin::symmetric(6, 5)),
            )
            .show(ctx, |ui| {
                ui.add_space(2.0);
                let all = self.downloads.len();
                let unfinished = self.downloads.iter().filter(|d| d.status != Status::Complete).count();
                let finished = self.downloads.iter().filter(|d| d.status == Status::Complete).count();
                if tree_row(ui, Icon::Folder, "All Downloads", all, self.filter == Filter::All, 0.0, dark).clicked() {
                    self.filter = Filter::All;
                }
                if tree_row(ui, Icon::File, "Unfinished", unfinished, self.filter == Filter::Unfinished, 1.0, dark).clicked() {
                    self.filter = Filter::Unfinished;
                }
                if tree_row(ui, Icon::File, "Finished", finished, self.filter == Filter::Finished, 1.0, dark).clicked() {
                    self.filter = Filter::Finished;
                }
                ui.add_space(4.0);
                ui.separator();
                ui.add_space(2.0);
                ui.label(
                    RichText::new("CATEGORIES")
                        .size(9.5)
                        .strong()
                        .color(theme::muted(dark)),
                );
                let cats = self.cfg.categories.clone();
                for c in &cats {
                    let count = self.downloads.iter().filter(|d| d.category == c.id).count();
                    let sel = self.filter == Filter::Category(c.id.clone());
                    let icon = match c.id.as_str() {
                        "music" => Icon::Music,
                        "video" => Icon::Video,
                        "programs" | "games" => Icon::Program,
                        "documents" => Icon::Doc,
                        "compressed" => Icon::Archive,
                        _ => Icon::Folder,
                    };
                    let resp = tree_row(ui, icon, &c.name, count, sel, 0.0, dark);
                    if resp.clicked() {
                        self.filter = Filter::Category(c.id.clone());
                    }
                    resp.context_menu(|ui| {
                        if ui.button("Add category...").clicked() {
                            self.dialogs.category = Some(Default::default());
                            ui.close_menu();
                        }
                        if ui.button("Properties...").clicked() {
                            self.dialogs.category = Some(crate::ui::dialogs::CategoryState::edit(&c));
                            ui.close_menu();
                        }
                        if c.id != "general" && ui.button("Delete").clicked() {
                            self.dialogs.delete_category = Some(c.id.clone());
                            ui.close_menu();
                        }
                    });
                }
                ui.add_space(6.0);
                ui.separator();
                ui.add_space(2.0);
                ui.label(
                    RichText::new("QUEUES")
                        .size(9.5)
                        .strong()
                        .color(theme::muted(dark)),
                );
                let queues = self.cfg.queues.clone();
                for q in &queues {
                    let count = self.downloads.iter().filter(|d| d.queue_id == q.id).count();
                    let label = if q.running {
                        format!("{} (running)", q.name)
                    } else {
                        q.name.clone()
                    };
                    if tree_row(ui, Icon::QueueStart, &label, count, self.filter == Filter::Queue(q.id), 0.0, dark).clicked() {
                        self.filter = Filter::Queue(q.id);
                    }
                }
                ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("{} downloads", self.downloads.len()))
                                .size(10.0)
                                .color(theme::muted(dark)),
                        );
                    });
                });
            });
    }

    fn status_bar(&mut self, ctx: &egui::Context) {
        let dark = self.dark;
        egui::TopBottomPanel::bottom("statusbar")
            .exact_height(27.0)
            .frame(
                egui::Frame::none()
                    .fill(if dark { theme::TOOLBAR_DARK } else { theme::TOOLBAR_LIGHT })
                    .inner_margin(egui::Margin::symmetric(8, 3))
                    .stroke(Stroke::new(1.0, if dark { Color32::from_gray(70) } else { Color32::from_gray(180) })),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let files = self.downloads.len();
                    let selected = self.selected.len();
                    let mut summary = if files == 1 {
                        "1 file".to_string()
                    } else {
                        format!("{} files", files)
                    };
                    if selected > 0 {
                        summary.push_str(&format!("  ·  {} selected", selected));
                    }
                    ui.label(RichText::new(summary).color(theme::muted(dark)));
                    ui.separator();
                    let speed = self.total_speed();
                    if speed > 0 {
                        ui.label(
                            RichText::new(format!(
                                "▼ {}/sec",
                                odm_core::util::format_bytes(speed)
                            ))
                            .color(theme::ODM_GREEN),
                        );
                    } else {
                        ui.label(RichText::new("Idle").color(theme::muted(dark)));
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let limited = self.cfg.speed_limit_kbps > 0;
                        let text = if limited {
                            format!("Speed Limiter: {} KB/sec", self.cfg.speed_limit_kbps)
                        } else {
                            "Speed Limiter: off".to_string()
                        };
                        if ui
                            .add(egui::Label::new(RichText::new(text).color(if limited { theme::ODM_RED } else { Color32::from_gray(110) })).sense(Sense::click()))
                            .clicked()
                        {
                            self.dialogs.speed_limiter = true;
                        }
                    });
                });
            });
    }

    fn main_list(&mut self, ctx: &egui::Context) {
        let dark = self.dark;
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(if dark { theme::PANEL_DARK } else { theme::PANEL_LIGHT })
                    .inner_margin(egui::Margin::same(4)),
            )
            .show(ctx, |ui| {
                if self.downloads.is_empty() {
                    ui.add_space(56.0);
                    ui.with_layout(Layout::top_down(Align::Center), |ui| {
                        egui::Frame::none()
                            .fill(theme::surface(dark))
                            .stroke(Stroke::new(1.0, theme::border(dark)))
                            .rounding(egui::Rounding::same(10))
                            .inner_margin(egui::Margin::same(26))
                            .show(ui, |ui| {
                                ui.set_width(360.0);
                                ui.vertical_centered(|ui| {
                                    let (icon_rect, _) = ui.allocate_exact_size(
                                        Vec2::splat(46.0),
                                        Sense::hover(),
                                    );
                                    icons::draw(
                                        ui.painter(),
                                        icon_rect,
                                        Icon::AddUrl,
                                        theme::ACCENT,
                                    );
                                    ui.add_space(8.0);
                                    ui.label(
                                        RichText::new("No downloads yet").size(16.0).strong(),
                                    );
                                    ui.label(
                                        RichText::new(
                                            "Add a URL or drop a link to start downloading.",
                                        )
                                        .color(theme::muted(dark)),
                                    );
                                    ui.add_space(10.0);
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new("Add URL").strong(),
                                            )
                                            .min_size(egui::vec2(120.0, 28.0)),
                                        )
                                        .clicked()
                                    {
                                        self.open_add();
                                    }
                                });
                            });
                    });
                    return;
                }
                let ids = self.visible_ids();
                if ids.is_empty() {
                    ui.label(
                        RichText::new("No downloads match the current filter.")
                            .color(theme::muted(dark)),
                    );
                    ui.add_space(4.0);
                }
                let ctrl_pressed = ui.input(|i| i.modifiers.ctrl);
                let shift_pressed = ui.input(|i| i.modifiers.shift);
                let mut actions: Vec<RowAction> = Vec::new();
                let mut table = egui_extras::TableBuilder::new(ui)
                    .striped(true)
                    .resizable(true)
                    .sense(egui::Sense::click())
                    .cell_layout(Layout::left_to_right(Align::Center))
                    .min_scrolled_height(0.0)
                    .auto_shrink([false, false]);
                if self.columns.queue {
                    table = table.column(egui_extras::Column::initial(36.0).at_least(28.0));
                }
                if self.columns.name {
                    table = table.column(
                        egui_extras::Column::initial(self.columns.name_w)
                            .at_least(120.0)
                            .clip(true),
                    );
                }
                if self.columns.status {
                    table = table.column(egui_extras::Column::initial(120.0).at_least(70.0).clip(true));
                }
                if self.columns.size {
                    table = table.column(egui_extras::Column::initial(80.0).at_least(60.0));
                }
                if self.columns.time_left {
                    table = table.column(egui_extras::Column::initial(90.0).at_least(60.0));
                }
                if self.columns.rate {
                    table = table.column(egui_extras::Column::initial(95.0).at_least(70.0));
                }
                if self.columns.description {
                    table = table.column(
                        egui_extras::Column::initial(self.columns.desc_w).at_least(80.0).clip(true),
                    );
                }
                if self.columns.date_added {
                    table = table.column(egui_extras::Column::initial(120.0).at_least(90.0));
                }
                if self.columns.last_try {
                    table = table.column(egui_extras::Column::initial(120.0).at_least(90.0));
                }
                if self.columns.save_to {
                    table = table.column(egui_extras::Column::initial(200.0).at_least(100.0).clip(true));
                }
                if self.columns.referer {
                    table = table.column(egui_extras::Column::initial(180.0).at_least(100.0).clip(true));
                }
                let mut sort_click: Option<SortCol> = None;
                let sort_now = self.sort;
                let sort_desc_now = self.sort_desc;
                table
                    .header(26.0, |mut header| {
                        let mut h = |header: &mut egui_extras::TableRow, label: &str, col: SortCol| {
                            header.col(|ui| {
                                let arrow = if sort_now == col {
                                    if sort_desc_now { " ▼" } else { " ▲" }
                                } else {
                                    ""
                                };
                                if ui
                                    .add(
                                        egui::Button::new(
                                            RichText::new(format!("{}{}", label, arrow)).size(11.5),
                                        )
                                        .frame(false),
                                    )
                                    .clicked()
                                {
                                    sort_click = Some(col);
                                }
                            });
                        };
                        if self.columns.queue {
                            h(&mut header, "Q", SortCol::Queue);
                        }
                        if self.columns.name {
                            h(&mut header, "File Name", SortCol::FileName);
                        }
                        if self.columns.status {
                            h(&mut header, "Status", SortCol::Status);
                        }
                        if self.columns.size {
                            h(&mut header, "Size", SortCol::Size);
                        }
                        if self.columns.time_left {
                            h(&mut header, "Time left", SortCol::TimeLeft);
                        }
                        if self.columns.rate {
                            h(&mut header, "Transfer rate", SortCol::Rate);
                        }
                        if self.columns.description {
                            h(&mut header, "Description", SortCol::Description);
                        }
                        if self.columns.date_added {
                            h(&mut header, "Date Added", SortCol::DateAdded);
                        }
                        if self.columns.last_try {
                            h(&mut header, "Last Try Date", SortCol::LastTry);
                        }
                        if self.columns.save_to {
                            h(&mut header, "Save To", SortCol::SaveTo);
                        }
                        if self.columns.referer {
                            h(&mut header, "Referer", SortCol::Referer);
                        }
                    })
                    .body(|body| {
                        body.rows(24.0, ids.len(), |mut row| {
                            let i = row.index();
                            let id = ids[i];
                            let dl = self.downloads.iter().find(|d| d.id == id).cloned();
                            let Some(dl) = dl else { return };
                            let selected = self.selected.contains(&id);
                            row.set_selected(selected);
                            if self.columns.queue {
                                row.col(|ui| {
                                    cell_text(ui, &format!("Q{}", dl.queue_id), 11.5);
                                });
                            }
                            if self.columns.name {
                                row.col(|ui| {
                                    let rect = ui.max_rect();
                                    let icon_rect = Rect::from_center_size(
                                        Pos2::new(rect.left() + 10.0, rect.center().y),
                                        Vec2::splat(16.0),
                                    );
                                    icons::draw(
                                        ui.painter(),
                                        icon_rect,
                                        icons::icon_for_file(&dl.file_name),
                                        theme::muted(ui.visuals().dark_mode),
                                    );
                                    let painter = ui.painter().with_clip_rect(rect);
                                    painter.text(
                                        Pos2::new(rect.left() + 24.0, rect.center().y),
                                        Align2::LEFT_CENTER,
                                        &dl.file_name,
                                        FontId::proportional(12.5),
                                        theme::text(ui.visuals().dark_mode),
                                    );
                                });
                            }
                            if self.columns.status {
                                row.col(|ui| {
                                    let (text, color) = status_text(&dl);
                                    cell_text_colored(ui, &text, color, 11.5);
                                });
                            }
                            if self.columns.size {
                                row.col(|ui| {
                                    cell_text(
                                        ui,
                                        &dl.size
                                            .map(odm_core::util::format_bytes)
                                            .unwrap_or_else(|| "?".into()),
                                        12.5,
                                    );
                                });
                            }
                            if self.columns.time_left {
                                row.col(|ui| {
                                    cell_text(
                                        ui,
                                        &dl.time_left()
                                            .map(odm_core::util::format_time_left)
                                            .unwrap_or_default(),
                                        12.5,
                                    );
                                });
                            }
                            if self.columns.rate {
                                row.col(|ui| {
                                    if dl.status.is_active() {
                                        cell_text(
                                            ui,
                                            &odm_core::util::format_speed(dl.speed),
                                            12.5,
                                        );
                                    }
                                });
                            }
                            if self.columns.description {
                                row.col(|ui| {
                                    cell_text(ui, &dl.description, 12.5);
                                });
                            }
                            if self.columns.date_added {
                                row.col(|ui| {
                                    cell_text(ui, &odm_core::util::format_date(dl.created), 12.5);
                                });
                            }
                            if self.columns.last_try {
                                row.col(|ui| {
                                    cell_text(
                                        ui,
                                        &dl.last_try
                                            .map(odm_core::util::format_date)
                                            .unwrap_or_default(),
                                        12.5,
                                    );
                                });
                            }
                            if self.columns.save_to {
                                row.col(|ui| {
                                    cell_text(ui, &dl.save_dir.to_string_lossy(), 12.5);
                                });
                            }
                            if self.columns.referer {
                                row.col(|ui| {
                                    cell_text(ui, &dl.referer, 12.5);
                                });
                            }
                            let resp = row.response();
                            if resp.clicked() {
                                actions.push(RowAction::Select(id, ctrl_pressed, shift_pressed));
                            }
                            if resp.secondary_clicked() && !selected {
                                actions.push(RowAction::Select(id, false, false));
                            }
                            if resp.double_clicked() {
                                actions.push(RowAction::Open(id));
                            }
                            let complete = dl.status == Status::Complete;
                            resp.context_menu(|ui| {
                                if complete {
                                    if ui.button("Download again").clicked() {
                                        actions.push(RowAction::Restart(id));
                                        ui.close_menu();
                                    }
                                } else {
                                    if ui.button("Resume").clicked() {
                                        actions.push(RowAction::Start(id));
                                        ui.close_menu();
                                    }
                                    if ui.button("Stop").clicked() {
                                        actions.push(RowAction::Stop(id));
                                        ui.close_menu();
                                    }
                                    if ui.button("Pause").clicked() {
                                        actions.push(RowAction::Pause(id));
                                        ui.close_menu();
                                    }
                                }
                                if ui.button("Remove").clicked() {
                                    actions.push(RowAction::Remove(id));
                                    ui.close_menu();
                                }
                                ui.separator();
                                if ui.button("Open").clicked() {
                                    actions.push(RowAction::Open(id));
                                    ui.close_menu();
                                }
                                if ui.button("Open folder").clicked() {
                                    if let Some(d) = self.downloads.iter().find(|d| d.id == id) {
                                        let _ = open::that(&d.save_dir);
                                    }
                                    ui.close_menu();
                                }
                                if ui.button("Properties").clicked() {
                                    actions.push(RowAction::Properties(id));
                                    ui.close_menu();
                                }
                                ui.separator();
                                if ui.button("Copy download URL").clicked() {
                                    if let Some(d) = self.downloads.iter().find(|d| d.id == id) {
                                        ui.ctx().copy_text(d.url.clone());
                                    }
                                    ui.close_menu();
                                }
                                if !complete && ui.button("Restart download").clicked() {
                                    actions.push(RowAction::Restart(id));
                                    ui.close_menu();
                                }
                            });
                        });
                    });
                if let Some(c) = sort_click {
                    self.set_sort(c);
                }
                for a in actions {
                    self.apply_row_action(a, ctx);
                }

            });
    }

    fn apply_row_action(&mut self, a: RowAction, _ctx: &egui::Context) {
        match a {
            RowAction::Select(id, ctrl, shift) => {
                if shift {
                    if let Some(anchor) = self.anchor {
                        let ids = self.visible_ids();
                        if let (Some(a), Some(b)) = (
                            ids.iter().position(|x| *x == anchor),
                            ids.iter().position(|x| *x == id),
                        ) {
                            let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
                            for x in &ids[lo..=hi] {
                                self.selected.insert(*x);
                            }
                        }
                    } else {
                        self.selected.insert(id);
                    }
                } else if ctrl {
                    if !self.selected.remove(&id) {
                        self.selected.insert(id);
                    }
                    self.anchor = Some(id);
                } else {
                    self.selected.clear();
                    self.selected.insert(id);
                    self.anchor = Some(id);
                }
            }
            RowAction::Open(id) => {
                if let Some(d) = self.downloads.iter().find(|d| d.id == id) {
                    if d.status == Status::Complete {
                        let _ = open::that(d.dest_path());
                    } else {
                        self.open_progress(id);
                    }
                }
            }
            RowAction::Start(id) => self.engine.start(id),
            RowAction::Stop(id) => self.engine.stop(id, false),
            RowAction::Pause(id) => self.engine.stop(id, true),
            RowAction::Remove(id) => {
                if self.cfg.confirm_delete {
                    self.dialogs.confirm_delete_files = self.cfg.delete_file_with_item;
                    self.dialogs.confirm_delete = Some(vec![id]);
                } else {
                    self.engine.remove(vec![id], self.cfg.delete_file_with_item);
                    self.selected.remove(&id);
                }
            }
            RowAction::Properties(id) => {
                if let Some(d) = self.downloads.iter().find(|d| d.id == id) {
                    self.dialogs.properties = Some(crate::ui::dialogs::PropertiesState::from(d));
                }
            }
            RowAction::Refresh(id) => {
                self.restart_download(id);
            }
            RowAction::Restart(id) => {
                self.restart_download(id);
            }
        }
    }

    fn restart_selected(&mut self) {
        let ids: Vec<u64> = self
            .selected_downloads()
            .iter()
            .filter(|d| matches!(d.status, Status::Complete | Status::Error))
            .map(|d| d.id)
            .collect();
        for id in ids {
            self.restart_download(id);
        }
    }

    fn restart_download(&mut self, id: u64) {
        let Some(d) = self.downloads.iter().find(|d| d.id == id).cloned() else {
            return;
        };
        self.engine.remove(vec![id], false);
        self.add_download(AddRequest {
            url: d.url,
            save_dir: Some(d.save_dir),
            file_name: Some(d.file_name),
            description: d.description,
            referer: d.referer,
            cookies: d.cookies,
            user_agent: Some(d.user_agent),
            username: d.username,
            password: d.password,
            post_data: d.post_data,
            category: Some(d.category),
            queue_id: Some(d.queue_id),
            start: true,
            silent: false,
        });
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let (new, find, find_next, del, all, open) = ctx.input(|i| {
            (
                i.modifiers.ctrl && i.key_pressed(Key::N),
                i.modifiers.ctrl && i.key_pressed(Key::F),
                i.key_pressed(Key::F3),
                i.key_pressed(Key::Delete),
                i.modifiers.ctrl && i.key_pressed(Key::A),
                i.key_pressed(Key::Enter),
            )
        });
        if new {
            self.open_add();
        }
        if find {
            self.dialogs.find = Some(Default::default());
        }
        if find_next {
            self.find_next();
        }
        if del {
            self.remove_selected();
        }
        if all {
            self.select_all();
        }
        if open {
            self.open_selected();
        }
    }

    fn find_next(&mut self) {
        let text = self
            .dialogs
            .find
            .as_ref()
            .map(|f| f.text.clone())
            .filter(|t| !t.is_empty())
            .unwrap_or_default();
        if text.is_empty() {
            self.dialogs.find = Some(Default::default());
            return;
        }
        let matches: Vec<u64> = self
            .downloads
            .iter()
            .filter(|d| {
                d.file_name.to_ascii_lowercase().contains(&text.to_ascii_lowercase())
                    || d.url.to_ascii_lowercase().contains(&text.to_ascii_lowercase())
                    || d.description.to_ascii_lowercase().contains(&text.to_ascii_lowercase())
            })
            .map(|d| d.id)
            .collect();
        if matches.is_empty() {
            self.status = "No such files found".into();
            return;
        }
        let pos = matches
            .iter()
            .position(|id| self.selected.contains(id))
            .map(|p| (p + 1) % matches.len())
            .unwrap_or(0);
        self.selected.clear();
        self.selected.insert(matches[pos]);
    }

    fn import_links(&mut self, export: bool) {
        let filter = if export {
            vec![("ODM export files", vec!["txt", "lst"]), ("All files", vec!["*"])]
        } else {
            vec![("Text files", vec!["txt", "lst"]), ("All files", vec!["*"])]
        };
        if let Some(path) = rfd::FileDialog::new().add_filter(filter[0].0, &filter[0].1).pick_file() {
            if let Ok(text) = std::fs::read_to_string(&path) {
                let links = if export {
                    odm_core::batch::parse_export(&text)
                } else {
                    odm_core::batch::parse_links(&text)
                        .into_iter()
                        .map(|u| (u, String::new()))
                        .collect()
                };
                let mut n = 0;
                for (url, desc) in links {
                    self.engine.add(AddRequest {
                        url,
                        description: desc,
                        start: false,
                        ..Default::default()
                    });
                    n += 1;
                }
                self.status = format!("{n} links imported.");
            }
        }
    }

    fn export_links(&mut self, _export: bool) {
        let items: Vec<(String, String)> = self
            .downloads
            .iter()
            .map(|d| (d.url.clone(), d.description.clone()))
            .collect();
        if let Some(path) = rfd::FileDialog::new()
            .set_file_name("odm-export.txt")
            .add_filter("Text files", &["txt"])
            .save_file()
        {
            let _ = std::fs::write(path, odm_core::batch::export_text(&items));
            self.status = "Download list exported.".into();
        }
    }

    fn open_options(&mut self) {
        self.dialogs.options = Some(crate::ui::dialogs::OptionsState::new(&self.cfg));
    }

    fn open_grabber(&mut self) {
        if self.grabber.is_none() {
            self.grabber = Some(GrabberState::default());
        }
    }

    fn clipboard_tick(&mut self, ctx: &egui::Context) {
        if !self.cfg.monitor_clipboard || self.last_clip_check.elapsed() < Duration::from_secs(1) {
            return;
        }
        self.last_clip_check = Instant::now();
        if let Some(c) = self.clip.as_mut() {
            if let Ok(t) = c.get_text() {
                let t = t.trim().to_string();
                if t != self.last_clip
                    && (t.starts_with("http://") || t.starts_with("https://"))
                    && self.url_has_captured_type(&t)
                {
                    self.last_clip = t.clone();
                    self.clip_prompt = Some(t);
                    ctx.request_repaint();
                }
            }
        }
    }

    fn url_has_captured_type(&self, url: &str) -> bool {
        let path = url.split('?').next().unwrap_or(url);
        let ext = std::path::Path::new(path)
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        self.cfg
            .file_types
            .iter()
            .any(|f| f.eq_ignore_ascii_case(&ext))
    }

    fn restore(&mut self, ctx: &egui::Context) {
        self.hidden = false;
        #[cfg(windows)]
        if let Some(h) = self.hwnd {
            unsafe {
                win::ShowWindow(h, win::SW_RESTORE);
                win::ShowWindow(h, win::SW_SHOW);
                win::SetForegroundWindow(h);
            }
        }
        ctx.send_viewport_cmd(ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(ViewportCommand::Focus);
        ctx.request_repaint();
    }
}

#[derive(Clone, Copy)]
enum RowAction {
    Select(u64, bool, bool),
    Open(u64),
    Start(u64),
    Stop(u64),
    Pause(u64),
    Remove(u64),
    Properties(u64),
    Refresh(u64),
    Restart(u64),
}

fn cell_text(ui: &mut egui::Ui, text: &str, size: f32) {
    let color = theme::text(ui.visuals().dark_mode);
    cell_text_colored(ui, text, color, size);
}

fn cell_text_colored(ui: &mut egui::Ui, text: &str, color: Color32, size: f32) {
    let rect = ui.max_rect();
    ui.painter().with_clip_rect(rect).text(
        Pos2::new(rect.left() + 2.0, rect.center().y),
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(size),
        color,
    );
}

fn status_text(d: &Download) -> (String, Color32) {
    match d.status {
        Status::Downloading => (
            format!("Downloading {:.0}%", d.percent()),
            Color32::from_rgb(0, 120, 0),
        ),
        Status::Connecting => ("Connecting...".into(), Color32::from_rgb(0, 100, 180)),
        Status::Assembling => ("Assembling...".into(), Color32::from_rgb(150, 100, 0)),
        Status::Queued => ("Queued".into(), Color32::from_gray(110)),
        Status::Paused => ("Paused".into(), Color32::from_rgb(180, 120, 0)),
        Status::Stopped => ("Stopped".into(), Color32::from_gray(110)),
        Status::Complete => ("Complete".into(), Color32::from_rgb(0, 130, 0)),
        Status::Error => ("Error".into(), theme::ODM_RED),
        Status::Scanning => ("Virus checking...".into(), Color32::from_rgb(120, 80, 160)),
    }
}

fn tree_row(
    ui: &mut egui::Ui,
    icon: Icon,
    label: &str,
    count: usize,
    selected: bool,
    indent: f32,
    dark: bool,
) -> egui::Response {
    let width = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, 24.0), Sense::click());
    let hover_t = ui.ctx().animate_bool_with_time(
        egui::Id::new(("tree-row", label)),
        resp.hovered() && !selected,
        0.10,
    );
    if selected {
        ui.painter()
            .rect_filled(rect, egui::Rounding::same(4), theme::ACCENT);
    } else if hover_t > 0.0 {
        let hover_bg = if dark {
            Color32::from_rgb(58, 60, 65)
        } else {
            Color32::from_rgb(232, 241, 250)
        };
        ui.painter().rect_filled(
            rect,
            egui::Rounding::same(4),
            theme::mix(Color32::TRANSPARENT, hover_bg, hover_t),
        );
    }
    let fg = if selected {
        Color32::WHITE
    } else {
        theme::text(dark)
    };
    let icon_rect = Rect::from_center_size(
        Pos2::new(rect.left() + 13.0 + indent * 12.0, rect.center().y),
        Vec2::splat(15.0),
    );
    icons::draw(
        ui.painter(),
        icon_rect,
        icon,
        if selected {
            fg
        } else {
            theme::ODM_AMBER
        },
    );
    ui.painter().text(
        Pos2::new(rect.left() + 26.0 + indent * 12.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(12.5),
        fg,
    );
    if count > 0 {
        ui.painter().text(
            Pos2::new(rect.right() - 8.0, rect.center().y),
            Align2::RIGHT_CENTER,
            count.to_string(),
            FontId::proportional(11.0),
            if selected {
                fg
            } else {
                theme::muted(dark)
            },
        );
    }
    resp
}

#[cfg(windows)]
fn native_hwnd(frame: &eframe::Frame) -> Option<isize> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    match frame.window_handle().ok()?.as_raw() {
        RawWindowHandle::Win32(h) => Some(h.hwnd.get()),
        _ => None,
    }
}

#[cfg(windows)]
mod win {
    #[link(name = "user32")]
    extern "system" {
        pub fn ShowWindow(hwnd: isize, cmd: i32) -> i32;
        pub fn SetForegroundWindow(hwnd: isize) -> i32;
        pub fn IsWindowVisible(hwnd: isize) -> i32;
    }
    pub const SW_HIDE: i32 = 0;
    pub const SW_SHOW: i32 = 5;
    pub const SW_RESTORE: i32 = 9;
}

fn load_tips() -> Vec<String> {    vec![
        "Open Download Manager downloads files using several connections at the same time. Use Options -> Connection to change the number.".into(),
        "You can drag and drop links onto the main window.".into(),
        "The scheduler can start and stop the download queue at a chosen time.".into(),
    ]
}

impl eframe::App for IdmApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        #[cfg(windows)]
        if self.hwnd.is_none() {
            self.hwnd = native_hwnd(frame);
        }
        #[cfg(windows)]
        if let Some(h) = self.hwnd {
            self.hwnd_shared
                .store(h, std::sync::atomic::Ordering::Relaxed);
            if !self.watcher_started {
                if let Some(tray) = &self.tray {
                    crate::ui::tray::spawn_restore_watcher(
                        self.hwnd_shared.clone(),
                        tray.show.id().clone(),
                        tray.exit.id().clone(),
                    );
                    self.watcher_started = true;
                }
            }
        }
        self.poll(ctx);
        #[cfg(windows)]
        if self.hidden {
            if let Some(h) = self.hwnd {
                if unsafe { win::IsWindowVisible(h) } != 0 {
                    // Restored by the tray watcher or another instance.
                    self.hidden = false;
                    ctx.request_repaint();
                }
            }
        }
        self.handle_shortcuts(ctx);
        self.clipboard_tick(ctx);

        if ctx.input(|i| i.viewport().close_requested())
            && self.cfg.close_to_tray
            && self.tray.is_some()
            && !self.hidden
        {
            ctx.send_viewport_cmd(ViewportCommand::CancelClose);
            #[cfg(windows)]
            if let Some(h) = self.hwnd {
                unsafe {
                    win::ShowWindow(h, win::SW_HIDE);
                }
            } else {
                ctx.send_viewport_cmd(ViewportCommand::Visible(false));
            }
            #[cfg(not(windows))]
            ctx.send_viewport_cmd(ViewportCommand::Visible(false));
            self.hidden = true;
        }

        self.menu_bar(ctx);
        self.toolbar(ctx);
        self.status_bar(ctx);
        self.side_panel(ctx);
        self.main_list(ctx);
        self.show_dialogs(ctx);

        let active = self.downloads.iter().any(|d| d.status.is_active());
        let interval = if self.hidden {
            Duration::from_secs(2)
        } else if active {
            Duration::from_millis(250)
        } else {
            Duration::from_secs(1)
        };
        ctx.request_repaint_after(interval);
    }
}
