//! Core data model for Open Download Manager.
//!
//! The types mirror the observable behaviour of Open Download Manager
//! 6.43 (Tonec): a flat download list, per-connection part files, queues,
//! categories, scheduler and site logins.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub type DownloadId = u64;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Status {
    #[default]
    Queued,
    Connecting,
    Downloading,
    Paused,
    Stopped,
    Assembling,
    Complete,
    Error,
    Scanning,
}

impl Status {
    /// Text shown in the "Status" column of the main list.
    pub fn label(self) -> &'static str {
        match self {
            Status::Queued => "Queued",
            Status::Connecting => "Connecting...",
            Status::Downloading => "Downloading",
            Status::Paused => "Paused",
            Status::Stopped => "Stopped",
            Status::Assembling => "Assembling...",
            Status::Complete => "Complete",
            Status::Error => "Error",
            Status::Scanning => "Virus checking...",
        }
    }

    pub fn is_active(self) -> bool {
        matches!(
            self,
            Status::Connecting | Status::Downloading | Status::Assembling | Status::Scanning
        )
    }

    pub fn is_final(self) -> bool {
        matches!(self, Status::Complete | Status::Error | Status::Stopped)
    }

    /// Sort key used by "Arrange files -> By Status".
    pub fn order(self) -> u8 {
        match self {
            Status::Downloading | Status::Connecting | Status::Assembling | Status::Scanning => 0,
            Status::Queued => 1,
            Status::Paused => 2,
            Status::Stopped => 3,
            Status::Error => 4,
            Status::Complete => 5,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Part {
    /// Absolute byte offset in the remote file where this connection starts.
    pub start: u64,
    /// Inclusive end offset, or u64::MAX for an open-ended stream.
    pub end: u64,
    /// Bytes already stored in the part file.
    pub pos: u64,
    pub done: bool,
}

impl Part {
    pub fn len(&self) -> u64 {
        if self.end == u64::MAX {
            u64::MAX
        } else {
            self.end.saturating_sub(self.start) + 1
        }
    }
    pub fn remaining(&self) -> u64 {
        self.len().saturating_sub(self.pos)
    }
    pub fn percent(&self) -> f32 {
        let len = self.len();
        if len == 0 || len == u64::MAX {
            0.0
        } else {
            (self.pos as f64 / len as f64 * 100.0) as f32
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Download {
    pub id: DownloadId,
    pub url: String,
    #[serde(default)]
    pub final_url: Option<String>,
    pub file_name: String,
    pub save_dir: PathBuf,
    #[serde(default = "default_category")]
    pub category: String,
    #[serde(default)]
    pub queue_id: u64,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub referer: String,
    #[serde(default)]
    pub user_agent: String,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    /// Cookie header captured from the browser, if any.
    #[serde(default)]
    pub cookies: Option<String>,
    /// HTTP method / body for POST captures from the browser.
    #[serde(default)]
    pub post_data: Option<String>,

    pub size: Option<u64>,
    pub downloaded: u64,
    pub status: Status,
    #[serde(default)]
    pub error: Option<String>,

    pub created: i64,
    #[serde(default)]
    pub started: Option<i64>,
    #[serde(default)]
    pub completed: Option<i64>,
    #[serde(default)]
    pub last_try: Option<i64>,

    #[serde(default)]
    pub parts: Vec<Part>,
    #[serde(default)]
    pub resumable: bool,
    #[serde(default)]
    pub connections: u32,

    #[serde(default)]
    pub etag: Option<String>,
    #[serde(default)]
    pub last_modified: Option<String>,
    #[serde(default)]
    pub content_type: Option<String>,

    /// Optional per-download speed limit (KB/s, 0 = use global).
    #[serde(default)]
    pub speed_limit: u32,
    #[serde(default)]
    pub note: String,

    // ---- runtime-only ----
    #[serde(skip)]
    pub speed: u64,
    #[serde(skip)]
    pub active_connections: u32,
    #[serde(skip)]
    pub selected: bool,
}

fn default_category() -> String {
    "general".into()
}

impl Download {
    pub fn new(id: DownloadId, url: String, file_name: String, save_dir: PathBuf) -> Self {
        Self {
            id,
            url,
            final_url: None,
            file_name,
            save_dir,
            category: "general".into(),
            queue_id: 0,
            description: String::new(),
            referer: String::new(),
            user_agent: String::new(),
            username: None,
            password: None,
            cookies: None,
            post_data: None,
            size: None,
            downloaded: 0,
            status: Status::Queued,
            error: None,
            created: chrono::Utc::now().timestamp(),
            started: None,
            completed: None,
            last_try: None,
            parts: Vec::new(),
            resumable: false,
            connections: 0,
            etag: None,
            last_modified: None,
            content_type: None,
            speed_limit: 0,
            note: String::new(),
            speed: 0,
            active_connections: 0,
            selected: false,
        }
    }

    pub fn dest_path(&self) -> PathBuf {
        self.save_dir.join(&self.file_name)
    }

    pub fn extension(&self) -> String {
        Path::new(&self.file_name)
            .extension()
            .map(|e| e.to_string_lossy().to_ascii_lowercase())
            .unwrap_or_default()
    }

    pub fn percent(&self) -> f32 {
        match self.size {
            Some(size) if size > 0 => (self.downloaded as f64 / size as f64 * 100.0) as f32,
            _ => 0.0,
        }
    }

    /// Remaining seconds at the current transfer rate; None when unknown.
    pub fn time_left(&self) -> Option<u64> {
        let size = self.size?;
        if self.speed == 0 || self.downloaded >= size {
            return None;
        }
        Some((size - self.downloaded) / self.speed)
    }

    pub fn is_complete(&self) -> bool {
        self.status == Status::Complete
    }
}
