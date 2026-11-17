//! Configuration model. Defaults are taken from the observed IDM 6.43
//! registry layout (`HKCU\Software\DownloadManager`) and language template.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub folder: PathBuf,
    #[serde(default)]
    pub masks: Vec<String>,
    #[serde(default)]
    pub sites: Vec<String>,
    #[serde(default)]
    pub builtin: bool,
    #[serde(default)]
    pub remember_last: bool,
}

impl Category {
    pub fn matches_ext(&self, ext: &str) -> bool {
        let ext = ext.trim_start_matches('.').to_ascii_lowercase();
        self.masks.iter().any(|m| {
            let m = m.trim_start_matches('.').to_ascii_lowercase();
            m == ext || wildcard_match(&m, &ext)
        })
    }
    pub fn matches_site(&self, url: &str) -> bool {
        if self.sites.is_empty() {
            return false;
        }
        self.sites.iter().any(|s| wildcard_match(s, url))
    }
}

pub fn wildcard_match(pattern: &str, value: &str) -> bool {
    let p = pattern.to_ascii_lowercase();
    let v = value.to_ascii_lowercase();
    if !p.contains('*') && !p.contains('?') {
        return p.is_empty() || v.contains(&p) || v == p;
    }
    let pc: Vec<char> = p.chars().collect();
    let vc: Vec<char> = v.chars().collect();
    let (mut pi, mut vi) = (0usize, 0usize);
    let (mut star, mut mark) = (None, 0usize);
    while vi < vc.len() {
        if pi < pc.len() && (pc[pi] == '?' || pc[pi] == vc[vi]) {
            pi += 1;
            vi += 1;
        } else if pi < pc.len() && pc[pi] == '*' {
            star = Some(pi);
            mark = vi;
            pi += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            vi = mark;
        } else {
            return false;
        }
    }
    while pi < pc.len() && pc[pi] == '*' {
        pi += 1;
    }
    pi == pc.len()
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ProxyMode {
    None,
    #[default]
    System,
    Manual,
    Auto,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProxyConfig {
    pub mode: ProxyMode,
    pub http_host: String,
    pub http_port: u16,
    pub http_user: String,
    pub http_pass: String,
    pub https_host: String,
    pub https_port: u16,
    pub ftp_host: String,
    pub ftp_port: u16,
    pub socks_host: String,
    pub socks_port: u16,
    pub socks_user: String,
    pub socks_pass: String,
    pub use_socks: bool,
    pub no_proxy: String,
    pub pac_url: String,
    pub use_proxy_for_http: bool,
    pub use_proxy_for_https: bool,
    pub use_proxy_for_ftp: bool,
    pub try_browser_proxy_on_error: bool,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self {
            mode: ProxyMode::System,
            http_host: String::new(),
            http_port: 8080,
            http_user: String::new(),
            http_pass: String::new(),
            https_host: String::new(),
            https_port: 8080,
            ftp_host: String::new(),
            ftp_port: 8080,
            socks_host: String::new(),
            socks_port: 1080,
            socks_user: String::new(),
            socks_pass: String::new(),
            use_socks: false,
            no_proxy: String::new(),
            pac_url: String::new(),
            use_proxy_for_http: true,
            use_proxy_for_https: true,
            use_proxy_for_ftp: true,
            try_browser_proxy_on_error: true,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum CompletionAction {
    #[default]
    None,
    Exit,
    Shutdown,
    Sleep,
    Hibernate,
    Hangup,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Schedule {
    pub enabled: bool,
    pub start_time: String, // "HH:MM"
    pub stop_time: String,  // "HH:MM"
    pub once: bool,
    pub days: [bool; 7], // Sunday..Saturday
    pub action: CompletionAction,
    pub limit_enabled: bool,
    pub limit_hours: u32,
    pub limit_mb: u32,
}

impl Default for Schedule {
    fn default() -> Self {
        Self {
            enabled: false,
            start_time: "01:00".into(),
            stop_time: "07:00".into(),
            once: false,
            days: [true; 7],
            action: CompletionAction::None,
            limit_enabled: false,
            limit_hours: 0,
            limit_mb: 0,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Queue {
    pub id: u64,
    pub name: String,
    #[serde(default)]
    pub running: bool,
    #[serde(default)]
    pub schedule: Schedule,
    #[serde(default)]
    pub retries: u32,
    #[serde(default)]
    pub start_on_startup: bool,
}

impl Queue {
    pub fn main() -> Self {
        Self {
            id: 1,
            name: "Main download queue".into(),
            running: false,
            schedule: Schedule::default(),
            retries: 3,
            start_on_startup: false,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ThemeMode {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum AccentColor {
    #[default]
    Blue,
    Violet,
    Teal,
    Orange,
    Rose,
    Green,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ToolbarStyle {
    Classic,
    Small,
    #[default]
    Large,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SoundSettings {
    pub enabled: bool,
    pub complete: Option<PathBuf>,
    pub failed: Option<PathBuf>,
    pub queue_started: Option<PathBuf>,
    pub queue_stopped: Option<PathBuf>,
}

impl Default for SoundSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            complete: None,
            failed: None,
            queue_started: None,
            queue_stopped: None,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    pub download_dir: PathBuf,
    pub temp_dir: PathBuf,
    pub categories: Vec<Category>,
    pub max_connections: u32,
    pub max_concurrent_downloads: u32,
    pub speed_limit_kbps: u32,
    pub speed_limiter_on_startup: bool,
    pub user_agent: String,
    pub proxy: ProxyConfig,
    pub queues: Vec<Queue>,
    pub file_types: Vec<String>,
    pub monitor_clipboard: bool,
    pub launch_on_startup: bool,
    pub minimize_to_tray: bool,
    pub close_to_tray: bool,
    pub start_minimized: bool,
    pub show_complete_dialog: bool,
    pub show_start_dialog: bool,
    pub start_immediately_with_info: bool,
    pub show_progress_dialog: bool,
    pub theme: ThemeMode,
    #[serde(default)]
    pub accent: AccentColor,
    pub toolbar_style: ToolbarStyle,
    pub large_buttons: bool,
    pub show_categories: bool,
    pub remember_last_save: bool,
    pub sounds: SoundSettings,
    pub virus_scanner: String,
    pub virus_args: String,
    pub virus_enabled: bool,
    pub confirm_delete: bool,
    pub delete_file_with_item: bool,
    pub duplicate_mode: u8, // 0 ask, 1 numbered, 2 overwrite, 3 resume
    pub dial_up_enabled: bool,
    pub dial_up_connection: String,
    pub dial_up_user: String,
    pub dial_up_pass: String,
    pub dial_up_retries: u32,
    pub dial_up_interval: u32,
    pub dial_up_hangup: bool,
    pub sites: Vec<SiteLogin>,
    pub connection_exceptions: Vec<ConnectionException>,
    pub last_save_dir: PathBuf,
    pub last_category: String,
    pub check_updates: bool,
    pub last_update_check: i64,
    pub ipc_port: u16,
    pub ipc_token: String,
    pub first_run: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct SiteLogin {
    pub site: String,
    pub user: String,
    pub password: String,
}

/// Per-server connection cap ("Max. connections number for a server",
/// registry `Software\DownloadManager\MCN`).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ConnectionException {
    pub server: String,
    pub max: u32,
}

impl Default for ConnectionException {
    fn default() -> Self {
        Self {
            server: String::new(),
            max: 4,
        }
    }
}

pub fn default_file_types() -> Vec<String> {
    // Exact list observed in HKCU\Software\DownloadManager\Extensions.
    "3GP 7Z AAC ACE AIF APK ARJ ASF AVI BIN BZ2 EXE GZ GZIP IMG ISO LZH M4A M4V MKV MOV MP3 MP4 MPA MPE MPEG MPG MSI MSU OGG OGV PDF PLJ PPS PPT QT R0* R1* RA RAR RM RMVB SEA SIT SITX TAR TIF TIFF WAV WMA WMV Z ZIP"
        .split_whitespace()
        .map(|s| s.to_string())
        .collect()
}

pub fn default_categories(download_dir: &PathBuf) -> Vec<Category> {
    let mk = |id: &str, name: &str, sub: &str, masks: &str| Category {
        id: id.into(),
        name: name.into(),
        folder: download_dir.join(sub),
        masks: masks.split_whitespace().map(|s| s.to_string()).collect(),
        sites: vec![],
        builtin: true,
        remember_last: false,
    };
    vec![
        Category {
            id: "general".into(),
            name: "General".into(),
            folder: download_dir.clone(),
            masks: vec![],
            sites: vec![],
            builtin: true,
            remember_last: false,
        },
        mk("programs", "Programs", "Programs", "exe msi"),
        mk(
            "music",
            "Music",
            "Music",
            "mp3 wav wma mpa ram ra aac aif m4a tsa",
        ),
        mk(
            "video",
            "Video",
            "Video",
            "avi mpg mpe mpeg asf wmv mov qt rm mp4 flv m4v webm ogv ogg mkv ts tsv",
        ),
        mk("games", "Games", "Games", "exe msi"),
        mk("documents", "Documents", "Documents", "doc pdf ppt pps docx pptx"),
        mk(
            "compressed",
            "Compressed",
            "Compressed",
            "zip rar r0* r1* arj gz sit sitx sea ace bz2 7z",
        ),
    ]
}

impl Default for Config {
    fn default() -> Self {
        let download_dir = crate::paths::default_download_dir();
        let mut queues = vec![Queue::main()];
        queues.push(Queue {
            id: 2,
            name: "Synchronization queue".into(),
            running: false,
            schedule: Schedule::default(),
            retries: 3,
            start_on_startup: false,
        });
        Self {
            categories: default_categories(&download_dir),
            download_dir: download_dir.clone(),
            temp_dir: crate::paths::appdata_dir().join("temp"),
            max_connections: 8,
            max_concurrent_downloads: 4,
            speed_limit_kbps: 0,
            speed_limiter_on_startup: false,
            user_agent: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36".into(),
            proxy: ProxyConfig::default(),
            queues,
            file_types: default_file_types(),
            monitor_clipboard: false,
            launch_on_startup: false,
            minimize_to_tray: true,
            close_to_tray: true,
            start_minimized: false,
            show_complete_dialog: true,
            show_start_dialog: true,
            start_immediately_with_info: true,
            show_progress_dialog: true,
            theme: ThemeMode::System,
            accent: AccentColor::Blue,
            toolbar_style: ToolbarStyle::Large,
            large_buttons: true,
            show_categories: true,
            remember_last_save: true,
            sounds: SoundSettings::default(),
            virus_scanner: String::new(),
            virus_args: String::new(),
            virus_enabled: false,
            confirm_delete: true,
            delete_file_with_item: false,
            duplicate_mode: 0,
            dial_up_enabled: false,
            dial_up_connection: String::new(),
            dial_up_user: String::new(),
            dial_up_pass: String::new(),
            dial_up_retries: 3,
            dial_up_interval: 5,
            dial_up_hangup: false,
            sites: vec![],
            connection_exceptions: vec![],
            last_save_dir: download_dir,
            last_category: "general".into(),
            check_updates: true,
            last_update_check: 0,
            ipc_port: 57443,
            ipc_token: String::new(),
            first_run: true,
        }
    }
}

impl Config {
    pub fn category(&self, id: &str) -> Option<&Category> {
        self.categories.iter().find(|c| c.id == id)
    }
    pub fn category_mut(&mut self, id: &str) -> Option<&mut Category> {
        self.categories.iter_mut().find(|c| c.id == id)
    }
    /// Pick the category for a file extension / URL, mirroring IDM's
    /// "automatically put in this category the following file types".
    pub fn pick_category(&self, file_name: &str, url: &str) -> String {
        let ext = std::path::Path::new(file_name)
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default();
        for c in &self.categories {
            if c.id == "general" {
                continue;
            }
            if c.matches_site(url) {
                return c.id.clone();
            }
        }
        for c in &self.categories {
            if c.id == "general" {
                continue;
            }
            if c.matches_ext(&ext) {
                return c.id.clone();
            }
        }
        "general".into()
    }
}
