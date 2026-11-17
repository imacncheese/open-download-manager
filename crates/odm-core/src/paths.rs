use std::path::PathBuf;
use std::sync::OnceLock;

static APP_DIR: OnceLock<PathBuf> = OnceLock::new();

/// `%APPDATA%\OpenDownloadManager`, with fallbacks for locked-down profiles.
pub fn appdata_dir() -> PathBuf {
    APP_DIR.get_or_init(resolve_appdata_dir).clone()
}

fn resolve_appdata_dir() -> PathBuf {
    if let Ok(v) = std::env::var("ODM_HOME") {
        if !v.trim().is_empty() {
            return PathBuf::from(v);
        }
    }
    if let Ok(appdata) = std::env::var("APPDATA") {
        let p = PathBuf::from(appdata).join("OpenDownloadManager");
        if writable(&p) {
            return p;
        }
    }
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        let p = PathBuf::from(local).join("OpenDownloadManager");
        if writable(&p) {
            return p;
        }
    }
    // Sandboxed or locked-down profiles: keep state next to the executable.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let p = dir.join("OpenDownloadManager-data");
            if writable(&p) {
                return p;
            }
        }
    }
    PathBuf::from(".open-download-manager")
}

fn writable(p: &std::path::Path) -> bool {
    if std::fs::create_dir_all(p).is_err() {
        return false;
    }
    let probe = p.join(".write-test");
    match std::fs::write(&probe, b"1") {
        Ok(()) => {
            let _ = std::fs::remove_file(probe);
            true
        }
        Err(_) => false,
    }
}

pub fn default_download_dir() -> PathBuf {
    if let Ok(profile) = std::env::var("USERPROFILE") {
        let p = PathBuf::from(profile).join("Downloads");
        if p.exists() {
            return p;
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

pub fn config_file() -> PathBuf {
    appdata_dir().join("config.json")
}

pub fn state_file() -> PathBuf {
    appdata_dir().join("downloads.json")
}

pub fn ipc_file() -> PathBuf {
    appdata_dir().join("ipc.json")
}

pub fn log_file() -> PathBuf {
    appdata_dir().join("odm.log")
}

pub fn ensure_dirs() -> std::io::Result<()> {
    let dir = appdata_dir();
    std::fs::create_dir_all(&dir)?;
    std::fs::create_dir_all(dir.join("temp"))?;
    std::fs::create_dir_all(dir.join("Grabber"))?;
    std::fs::create_dir_all(dir.join("Scheduler"))?;
    Ok(())
}
