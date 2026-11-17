use crate::config::Config;
use crate::model::Download;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::atomic::{AtomicI64, Ordering};

#[derive(Serialize, Deserialize, Clone)]
pub struct Store {
    pub config: Config,
    pub downloads: Vec<Download>,
    pub next_id: u64,
}

impl Default for Store {
    fn default() -> Self {
        Self {
            config: Config::default(),
            downloads: Vec::new(),
            next_id: 1,
        }
    }
}

#[derive(Serialize, Deserialize, Default)]
struct StateFile {
    #[serde(default)]
    next_id: u64,
    #[serde(default)]
    downloads: Vec<Download>,
}

impl Store {
    pub fn load() -> Self {
        crate::paths::ensure_dirs().ok();
        let config = match std::fs::read_to_string(crate::paths::config_file()) {
            Ok(s) => serde_json::from_str::<Config>(&s).unwrap_or_default(),
            Err(_) => Config::default(),
        };
        let (downloads, next_id) = match std::fs::read_to_string(crate::paths::state_file()) {
            Ok(s) => match serde_json::from_str::<StateFile>(&s) {
                Ok(mut st) => {
                    // Any download that was active when the app stopped is
                    // shown as paused, exactly like IDM after a restart.
                    for d in &mut st.downloads {
                        if d.status.is_active() {
                            d.status = crate::model::Status::Paused;
                            d.speed = 0;
                        }
                    }
                    let max = st.downloads.iter().map(|d| d.id).max().unwrap_or(0);
                    if st.next_id <= max {
                        st.next_id = max + 1;
                    }
                    (st.downloads, st.next_id)
                }
                Err(_) => (Vec::new(), 1),
            },
            Err(_) => (Vec::new(), 1),
        };
        let mut store = Self {
            config,
            downloads,
            next_id: next_id.max(1),
        };
        store.normalize();
        store
    }

    fn normalize(&mut self) {
        if self.config.categories.is_empty() {
            self.config.categories =
                crate::config::default_categories(&self.config.download_dir);
        }
        if self.config.queues.is_empty() {
            self.config.queues.push(crate::config::Queue::main());
        }
        let max = self.downloads.iter().map(|d| d.id).max().unwrap_or(0);
        if self.next_id <= max {
            self.next_id = max + 1;
        }
        for d in &mut self.downloads {
            if d.save_dir.as_os_str().is_empty() {
                d.save_dir = self.config.download_dir.clone();
            }
        }
    }

    pub fn config_path() -> std::path::PathBuf {
        crate::paths::config_file()
    }

    pub fn save(&self) {
        let _ = write_atomic(
            &crate::paths::config_file(),
            &serde_json::to_vec_pretty(&self.config).unwrap_or_default(),
        );
        let st = StateFile {
            next_id: self.next_id,
            downloads: self.downloads.clone(),
        };
        let _ = write_atomic(
            &crate::paths::state_file(),
            &serde_json::to_vec(&st).unwrap_or_default(),
        );
    }

    pub fn save_state_only(&self) {
        let st = StateFile {
            next_id: self.next_id,
            downloads: self.downloads.clone(),
        };
        let _ = write_atomic(
            &crate::paths::state_file(),
            &serde_json::to_vec(&st).unwrap_or_default(),
        );
    }

    pub fn add(&mut self, mut d: Download) -> u64 {
        let id = self.next_id.max(1);
        self.next_id = id + 1;
        d.id = id;
        self.downloads.push(d);
        id
    }

    pub fn get(&self, id: u64) -> Option<&Download> {
        self.downloads.iter().find(|d| d.id == id)
    }

    pub fn get_mut(&mut self, id: u64) -> Option<&mut Download> {
        self.downloads.iter_mut().find(|d| d.id == id)
    }

    pub fn remove(&mut self, id: u64) -> Option<Download> {
        if let Some(pos) = self.downloads.iter().position(|d| d.id == id) {
            Some(self.downloads.remove(pos))
        } else {
            None
        }
    }

    pub fn active_count(&self) -> usize {
        self.downloads.iter().filter(|d| d.status.is_active()).count()
    }
}

fn next_id_fix(next_id: &mut u64, max: u64) {
    *next_id = max + 1;
}

pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension(format!(
        "{}.tmp",
        path.extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_default()
    ));
    std::fs::write(&tmp, bytes)?;
    // Windows rename fails when the target exists.
    let _ = std::fs::remove_file(path);
    std::fs::rename(&tmp, path)
}

static LAST_SAVE: AtomicI64 = AtomicI64::new(0);

/// Debounced state save used by the transfer threads.
pub fn save_throttled(store: &Store, min_secs: i64) {
    let now = crate::util::now();
    let last = LAST_SAVE.load(Ordering::Relaxed);
    if now - last >= min_secs && LAST_SAVE.swap(now, Ordering::Relaxed) != now {
        store.save_state_only();
    }
}

pub fn log(msg: &str) {
    use std::io::Write;
    let path = crate::paths::log_file();
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(f, "[{}] {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), msg);
    }
}
