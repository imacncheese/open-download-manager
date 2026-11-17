//! Download engine: segmented HTTP/HTTPS transfers with resume, per-part
//! files, retries, queues, scheduling and speed limiting.
//!
//! The architecture mirrors IDM's observable design: one part file per
//! connection in a temporary folder, assembly into the final file, a single
//! flat download list, and queue/scheduler driven start and stop.

use crate::config::{CompletionAction, Config, Queue};
use crate::httpmeta::{self, Probe};
use crate::limiter::SpeedLimiter;
use crate::model::{Download, DownloadId, Part, Status};
use crate::persist::{self, Store};
use crate::util;
use anyhow::{anyhow, Context, Result};
use futures_util::StreamExt;
use reqwest::header::RANGE;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::task::JoinSet;

#[derive(Clone, Debug, Default)]
pub struct AddRequest {
    pub url: String,
    pub save_dir: Option<PathBuf>,
    pub file_name: Option<String>,
    pub description: String,
    pub referer: String,
    pub cookies: Option<String>,
    pub user_agent: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub post_data: Option<String>,
    pub category: Option<String>,
    pub queue_id: Option<u64>,
    pub start: bool,
    pub silent: bool,
}

#[derive(Clone, Debug)]
pub enum EngineEvent {
    Changed,
    Added(DownloadId),
    Completed(DownloadId),
    Failed(DownloadId, String),
    QueueStarted(u64),
    QueueStopped(u64),
    ShowWindow,
    Started(DownloadId),
}

struct PartAtom {
    start: u64,
    end: u64,
    pos: AtomicU64,
    done: AtomicBool,
}

struct RunCtl {
    cancel: AtomicBool,
    parts: Mutex<Vec<Arc<PartAtom>>>,
    downloaded: AtomicU64,
    speed: AtomicU64,
}

impl RunCtl {
    fn new(parts: &[Part]) -> Self {
        Self {
            cancel: AtomicBool::new(false),
            parts: Mutex::new(
                parts
                    .iter()
                    .map(|p| {
                        Arc::new(PartAtom {
                            start: p.start,
                            end: p.end,
                            pos: AtomicU64::new(p.pos),
                            done: AtomicBool::new(p.done),
                        })
                    })
                    .collect(),
            ),
            downloaded: AtomicU64::new(parts.iter().map(|p| p.pos).sum()),
            speed: AtomicU64::new(0),
        }
    }
    fn atoms(&self) -> Vec<Arc<PartAtom>> {
        self.parts.lock().map(|v| v.clone()).unwrap_or_default()
    }
    fn total_downloaded(&self) -> u64 {
        let mut total = 0;
        if let Ok(parts) = self.parts.lock() {
            for p in parts.iter() {
                total += p.pos.load(Ordering::Relaxed);
            }
        }
        total
    }
    fn all_done(&self) -> bool {
        self.parts
            .lock()
            .map(|p| p.iter().all(|x| x.done.load(Ordering::Relaxed)))
            .unwrap_or(false)
    }
}

pub struct Shared {
    pub store: RwLock<Store>,
    pub limiter: Arc<SpeedLimiter>,
    pub controls: Mutex<HashMap<u64, Arc<RunCtl>>>,
    pub active: AtomicU32,
    pub ipc_port: AtomicU32,
    pub ipc_token: String,
    pub events: Mutex<Option<std::sync::mpsc::Sender<EngineEvent>>>,
}

impl Shared {
    pub(crate) fn emit(&self, ev: EngineEvent) {
        if let Ok(g) = self.events.lock() {
            if let Some(tx) = g.as_ref() {
                let _ = tx.send(ev);
            }
        }
    }
    fn snapshot(&self, id: u64) -> Option<Download> {
        self.store.read().ok()?.get(id).cloned()
    }
    fn update<F: FnOnce(&mut Download)>(&self, id: u64, f: F) {
        if let Ok(mut st) = self.store.write() {
            if let Some(d) = st.get_mut(id) {
                f(d);
            }
        }
    }
    fn set_status(&self, id: u64, status: Status) {
        self.update(id, |d| {
            d.status = status;
            if !status.is_active() {
                d.speed = 0;
                d.active_connections = 0;
            }
        });
        self.emit(EngineEvent::Changed);
    }
}

pub(crate) enum Cmd {
    Add(AddRequest, std::sync::mpsc::SyncSender<u64>),
    AddFire(AddRequest),
    Start(u64),
    StartAll(Vec<u64>),
    Stop(u64, bool),
    StopMany(Vec<u64>, bool),
    StopAll,
    Remove(Vec<u64>, bool),
    SetConfig(Box<Config>),
    SetSpeed(u32),
    Save,
    StartQueue(u64),
    StopQueue(u64),
    Shutdown(std::sync::mpsc::SyncSender<()>),
}

pub struct Engine {
    shared: Arc<Shared>,
    tx: UnboundedSender<Cmd>,
    join: Option<std::thread::JoinHandle<()>>,
}

impl Engine {
    pub fn new(events: Option<std::sync::mpsc::Sender<EngineEvent>>) -> Result<Self> {
        crate::paths::ensure_dirs().ok();
        let store = Store::load();
        let limiter = Arc::new(SpeedLimiter::new(store.config.speed_limit_kbps));
        let mut ipc_token = store.config.ipc_token.clone();
        if ipc_token.is_empty() {
            ipc_token = format!(
                "{:x}{:x}",
                std::process::id(),
                chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0)
            );
        }
        let shared = Arc::new(Shared {
            store: RwLock::new(store),
            limiter,
            controls: Mutex::new(HashMap::new()),
            active: AtomicU32::new(0),
            ipc_port: AtomicU32::new(0),
            ipc_token,
            events: Mutex::new(events),
        });
        let (tx, rx) = unbounded_channel();
        let (ready_tx, ready_rx) = std::sync::mpsc::channel::<u16>();
        let s2 = shared.clone();
        let tx2 = tx.clone();
        let join = std::thread::Builder::new()
            .name("odm-engine".into())
            .spawn(move || {
                let rt = match tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(4)
                    .enable_all()
                    .thread_name("odm-worker")
                    .build()
                {
                    Ok(rt) => rt,
                    Err(e) => {
                        persist::log(&format!("engine runtime failed: {e}"));
                        let _ = ready_tx.send(0);
                        return;
                    }
                };
                rt.block_on(async move {
                    let controller = tokio::spawn(controller_loop(s2.clone(), rx, tx2.clone()));
                    let port = crate::ipc::serve(s2.clone(), tx2.clone()).await;
                    s2.ipc_port.store(port as u32, Ordering::SeqCst);
                    if let Ok(mut st) = s2.store.write() {
                        st.config.ipc_port = port;
                        if st.config.ipc_token.is_empty() {
                            st.config.ipc_token = s2.ipc_token.clone();
                        }
                        let _ = &st.config;
                    }
                    let _ = ready_tx.send(port);
                    // Keep the runtime alive until shutdown.
                    let _ = controller.await;
                });
            })?;
        let port = ready_rx.recv_timeout(Duration::from_secs(5)).unwrap_or(0);
        Ok(Engine {
            shared,
            tx,
            join: Some(join),
        })
    }

    pub fn config(&self) -> Config {
        self.shared
            .store
            .read()
            .map(|s| s.config.clone())
            .unwrap_or_default()
    }

    pub fn snapshot(&self) -> Vec<Download> {
        self.shared
            .store
            .read()
            .map(|s| {
                let mut v = s.downloads.clone();
                for d in &mut v {
                    if let Ok(c) = self.shared.controls.lock() {
                        if let Some(ctl) = c.get(&d.id) {
                            d.speed = ctl.speed.load(Ordering::Relaxed);
                            d.downloaded = ctl.total_downloaded().max(d.downloaded);
                            if let Ok(parts) = ctl.parts.lock() {
                                d.active_connections = parts
                                    .iter()
                                    .filter(|p| !p.done.load(Ordering::Relaxed))
                                    .count()
                                    as u32;
                                for (i, p) in parts.iter().enumerate() {
                                    if let Some(dp) = d.parts.get_mut(i) {
                                        dp.pos = p.pos.load(Ordering::Relaxed);
                                        dp.done = p.done.load(Ordering::Relaxed);
                                    }
                                }
                            }
                        }
                    }
                }
                v
            })
            .unwrap_or_default()
    }

    pub fn add(&self, req: AddRequest) -> u64 {
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        if self.tx.send(Cmd::Add(req, tx)).is_err() {
            return 0;
        }
        rx.recv_timeout(Duration::from_secs(3)).unwrap_or(0)
    }
    pub fn add_many(&self, reqs: Vec<AddRequest>) -> Vec<u64> {
        reqs.into_iter().map(|r| self.add(r)).collect()
    }
    pub fn start(&self, id: u64) {
        let _ = self.tx.send(Cmd::Start(id));
    }
    pub fn start_all(&self, ids: Vec<u64>) {
        let _ = self.tx.send(Cmd::StartAll(ids));
    }
    pub fn stop(&self, id: u64, pause: bool) {
        let _ = self.tx.send(Cmd::Stop(id, pause));
    }
    pub fn stop_many(&self, ids: Vec<u64>, pause: bool) {
        let _ = self.tx.send(Cmd::StopMany(ids, pause));
    }
    pub fn stop_all(&self) {
        let _ = self.tx.send(Cmd::StopAll);
    }
    pub fn remove(&self, ids: Vec<u64>, delete_file: bool) {
        let _ = self.tx.send(Cmd::Remove(ids, delete_file));
    }
    pub fn set_config(&self, cfg: Config) {
        let _ = self.tx.send(Cmd::SetConfig(Box::new(cfg)));
    }
    pub fn set_speed_limit(&self, kbps: u32) {
        let _ = self.tx.send(Cmd::SetSpeed(kbps));
    }
    pub fn save(&self) {
        let _ = self.tx.send(Cmd::Save);
    }
    pub fn start_queue(&self, id: u64) {
        let _ = self.tx.send(Cmd::StartQueue(id));
    }
    pub fn stop_queue(&self, id: u64) {
        let _ = self.tx.send(Cmd::StopQueue(id));
    }
    pub fn ipc_port(&self) -> u16 {
        self.shared.ipc_port.load(Ordering::SeqCst) as u16
    }
    pub fn ipc_token(&self) -> String {
        self.shared.ipc_token.clone()
    }
    pub fn shutdown(&mut self) {
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        if self.tx.send(Cmd::Shutdown(tx)).is_ok() {
            let _ = rx.recv_timeout(Duration::from_secs(5));
        }
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        if self.join.is_some() {
            self.shutdown();
        }
    }
}

async fn controller_loop(
    shared: Arc<Shared>,
    mut rx: tokio::sync::mpsc::UnboundedReceiver<Cmd>,
    tx: UnboundedSender<Cmd>,
) {
    let mut tick = tokio::time::interval(Duration::from_millis(700));
    let mut last_limit_check = Instant::now();
    loop {
        tokio::select! {
            cmd = rx.recv() => {
                match cmd {
                    Some(Cmd::Add(req, resp)) => {
                        let id = do_add(&shared, req);
                        let _ = resp.send(id);
                    }
                    Some(Cmd::AddFire(req)) => {
                        do_add(&shared, req);
                    }
                    Some(Cmd::Start(id)) => {
                        start_one(&shared, id);
                    }
                    Some(Cmd::StartAll(ids)) => {
                        for id in ids { start_one(&shared, id); }
                    }
                    Some(Cmd::Stop(id, pause)) => stop_one(&shared, id, pause),
                    Some(Cmd::StopMany(ids, pause)) => for id in ids { stop_one(&shared, id, pause); },
                    Some(Cmd::StopAll) => {
                        let ids: Vec<u64> = shared.store.read().map(|s| s.downloads.iter()
                            .filter(|d| d.status.is_active()).map(|d| d.id).collect()).unwrap_or_default();
                        for id in ids { stop_one(&shared, id, false); }
                    }
                    Some(Cmd::Remove(ids, del)) => remove_many(&shared, ids, del),
                    Some(Cmd::SetConfig(cfg)) => {
                        if let Ok(mut st) = shared.store.write() {
                            st.config = *cfg;
                            shared.limiter.set_rate(st.config.speed_limit_kbps);
                            st.save();
                        }
                        shared.emit(EngineEvent::Changed);
                    }
                    Some(Cmd::SetSpeed(k)) => {
                        shared.limiter.set_rate(k);
                        if let Ok(mut st) = shared.store.write() {
                            st.config.speed_limit_kbps = k;
                            st.save();
                        }
                    }
                    Some(Cmd::Save) => { if let Ok(st) = shared.store.read() { st.save(); } }
                    Some(Cmd::StartQueue(qid)) => start_queue(&shared, qid),
                    Some(Cmd::StopQueue(qid)) => stop_queue(&shared, qid),
                    Some(Cmd::Shutdown(done)) => {
                        if let Ok(st) = shared.store.read() { st.save(); }
                        let _ = done.send(());
                        break;
                    }
                    None => break,
                }
            }
            _ = tick.tick() => {
                scheduler_tick(&shared).await;
                let _ = &tx;
                if last_limit_check.elapsed() > Duration::from_secs(5) {
                    last_limit_check = Instant::now();
                    enforce_limits(&shared);
                }
            }
        }
    }
}

fn do_add(shared: &Arc<Shared>, req: AddRequest) -> u64 {
    let url = req.url.trim().to_string();
    if url.is_empty() {
        return 0;
    }
    let mut st = match shared.store.write() {
        Ok(s) => s,
        Err(_) => return 0,
    };
    // Refuse exact duplicates of an active download (IDM's duplicate dialog
    // default is "ask"; the UI performs the prompt and decides).
    let file_name = req
        .file_name
        .clone()
        .filter(|s| !s.is_empty())
        .or_else(|| util::filename_from_url(&url))
        .unwrap_or_else(|| "download".into());
    let file_name = util::sanitize_filename(&file_name);
    let category = req
        .category
        .clone()
        .filter(|c| st.config.category(c).is_some())
        .unwrap_or_else(|| st.config.pick_category(&file_name, &url));
    let save_dir = req
        .save_dir
        .clone()
        .or_else(|| st.config.category(&category).map(|c| c.folder.clone()))
        .unwrap_or_else(|| st.config.download_dir.clone());
    let mut dl = Download::new(0, url, file_name, save_dir);
    dl.category = category;
    dl.queue_id = req.queue_id.unwrap_or(1);
    dl.description = req.description;
    dl.referer = req.referer;
    dl.cookies = req.cookies;
    dl.user_agent = req.user_agent.unwrap_or_default();
    dl.username = req.username;
    dl.password = req.password;
    dl.post_data = req.post_data;
    if dl.user_agent.is_empty() {
        dl.user_agent = st.config.user_agent.clone();
    }
    let id = st.add(dl);
    let start = req.start;
    let _ = st.save_state_only();
    drop(st);
    shared.emit(EngineEvent::Added(id));
    let t = shared.clone();
    if start {
        tokio::spawn(async move {
            start_one(&t, id);
        });
    }
    id
}

fn start_one(shared: &Arc<Shared>, id: u64) {
    // A download may only run once at a time.
    if let Ok(c) = shared.controls.lock() {
        if c.contains_key(&id) {
            return;
        }
    }
    let dl = match shared.snapshot(id) {
        Some(d) => d,
        None => return,
    };
    if dl.status == Status::Complete {
        return;
    }
    let max = shared
        .store
        .read()
        .map(|s| s.config.max_concurrent_downloads.max(1))
        .unwrap_or(4);
    if shared.active.load(Ordering::Relaxed) >= max {
        shared.set_status(id, Status::Queued);
        return;
    }
    shared.active.fetch_add(1, Ordering::SeqCst);
    shared.set_status(id, Status::Connecting);
    shared.update(id, |d| {
        d.error = None;
        d.last_try = Some(util::now());
    });
    shared.emit(EngineEvent::Started(id));
    shared.emit(EngineEvent::Changed);
    let sh = shared.clone();
    tokio::spawn(async move {
        let result = run_download(sh.clone(), id).await;
        sh.controls.lock().ok().map(|mut c| c.remove(&id));
        sh.active.fetch_sub(1, Ordering::SeqCst);
        match result {
            Ok(true) => {
                sh.set_status(id, Status::Complete);
                sh.update(id, |d| {
                    d.completed = Some(util::now());
                    d.downloaded = d.size.unwrap_or(d.downloaded);
                    d.speed = 0;
                    d.active_connections = 0;
                    for p in &mut d.parts {
                        p.done = true;
                        p.pos = p.len();
                    }
                });
                if let Ok(st) = sh.store.read() {
                    st.save_state_only();
                }
                sh.emit(EngineEvent::Completed(id));
            }
            Ok(false) => {
                // Cancelled: status already set by stop_one.
                if let Ok(st) = sh.store.read() {
                    st.save_state_only();
                }
            }
            Err(e) => {
                let msg = format!("{e:#}");
                persist::log(&format!("download {id} failed: {msg}"));
                sh.set_status(id, Status::Error);
                sh.update(id, |d| {
                    d.error = Some(msg.clone());
                    d.speed = 0;
                });
                if let Ok(st) = sh.store.read() {
                    st.save_state_only();
                }
                sh.emit(EngineEvent::Failed(id, msg));
            }
        }
        sh.emit(EngineEvent::Changed);
        // Free slot: promote the next queued item.
        tokio::spawn(async move { promote_next(&sh).await });
    });
}

async fn promote_next(shared: &Arc<Shared>) {
    let max = shared
        .store
        .read()
        .map(|s| s.config.max_concurrent_downloads.max(1))
        .unwrap_or(4);
    loop {
        if shared.active.load(Ordering::Relaxed) >= max {
            return;
        }
        let next = {
            let st = match shared.store.read() {
                Ok(s) => s,
                Err(_) => return,
            };
            st.downloads
                .iter()
                .find(|d| d.status == Status::Queued)
                .map(|d| d.id)
        };
        let Some(id) = next else { return };
        start_one(shared, id);
    }
}

fn stop_one(shared: &Arc<Shared>, id: u64, pause: bool) {
    if let Ok(c) = shared.controls.lock() {
        if let Some(ctl) = c.get(&id) {
            ctl.cancel.store(true, Ordering::SeqCst);
        }
    }
    let st = if pause { Status::Paused } else { Status::Stopped };
    let active = shared
        .snapshot(id)
        .map(|d| d.status.is_active())
        .unwrap_or(false);
    shared.update(id, |d| {
        if d.status.is_active() || d.status == Status::Queued {
            d.status = st;
        }
        d.speed = 0;
        d.active_connections = 0;
    });
    if active {
        if let Ok(st) = shared.store.read() {
            st.save_state_only();
        }
    }
    shared.emit(EngineEvent::Changed);
}

fn remove_many(shared: &Arc<Shared>, ids: Vec<u64>, delete_file: bool) {
    for id in &ids {
        if let Ok(c) = shared.controls.lock() {
            if let Some(ctl) = c.get(id) {
                ctl.cancel.store(true, Ordering::SeqCst);
            }
        }
    }
    std::thread::sleep(Duration::from_millis(120));
    let mut st = match shared.store.write() {
        Ok(s) => s,
        Err(_) => return,
    };
    for id in &ids {
        if let Some(d) = st.get(*id) {
            if delete_file {
                let _ = std::fs::remove_file(d.dest_path());
                let _ = std::fs::remove_file(d.save_dir.join(format!("{}.idm", d.file_name)));
            }
            let tmp = st.config.temp_dir.join(format!("{}", id));
            let _ = std::fs::remove_dir_all(tmp);
        }
        st.remove(*id);
    }
    let _ = st.save_state_only();
    drop(st);
    shared.emit(EngineEvent::Changed);
}

fn start_queue(shared: &Arc<Shared>, qid: u64) {
    if let Ok(mut st) = shared.store.write() {
        if let Some(q) = st.config.queues.iter_mut().find(|q| q.id == qid) {
            q.running = true;
        } else {
            return;
        }
        for d in &mut st.downloads {
            if d.queue_id == qid && matches!(d.status, Status::Stopped | Status::Error | Status::Paused)
            {
                d.status = Status::Queued;
            }
        }
        st.save();
    }
    shared.emit(EngineEvent::QueueStarted(qid));
    let sh = shared.clone();
    tokio::spawn(async move { promote_next(&sh).await });
}

fn stop_queue(shared: &Arc<Shared>, qid: u64) {
    let ids: Vec<u64> = shared
        .store
        .read()
        .map(|s| {
            s.downloads
                .iter()
                .filter(|d| d.queue_id == qid && d.status.is_active())
                .map(|d| d.id)
                .collect()
        })
        .unwrap_or_default();
    for id in ids {
        stop_one(shared, id, false);
    }
    if let Ok(mut st) = shared.store.write() {
        if let Some(q) = st.config.queues.iter_mut().find(|q| q.id == qid) {
            q.running = false;
        }
        for d in &mut st.downloads {
            if d.queue_id == qid && d.status == Status::Queued {
                d.status = Status::Stopped;
            }
        }
        st.save();
    }
    shared.emit(EngineEvent::QueueStopped(qid));
}

async fn scheduler_tick(shared: &Arc<Shared>) {
    let now = chrono::Local::now();
    let hhmm = now.format("%H:%M").to_string();
    let weekday = now.format("%w").to_string().parse::<usize>().unwrap_or(0);
    let mut start_queues = Vec::new();
    let mut stop_queues = Vec::new();
    let mut action = CompletionAction::None;
    if let Ok(st) = shared.store.read() {
        for q in &st.config.queues {
            let s = &q.schedule;
            if !s.enabled || s.once {
                continue;
            }
            if !s.days[weekday.min(6)] {
                continue;
            }
            if s.start_time == hhmm && !q.running {
                start_queues.push(q.id);
            }
            if s.stop_time == hhmm && q.running {
                stop_queues.push(q.id);
                action = s.action;
            }
        }
    }
    for q in start_queues {
        start_queue(shared, q);
    }
    for q in stop_queues {
        stop_queue(shared, q);
    }
    if action != CompletionAction::None {
        let any_active = shared
            .store
            .read()
            .map(|s| s.downloads.iter().any(|d| d.status.is_active()))
            .unwrap_or(false);
        if !any_active {
            perform_action(action);
        }
    }
}

fn perform_action(action: CompletionAction) {
    if action == CompletionAction::None {
        return;
    }
    #[cfg(windows)]
    {
        let cmd = match action {
            CompletionAction::Exit => return,
            CompletionAction::Shutdown => "shutdown /s /t 0",
            CompletionAction::Sleep => "rundll32.exe powrprof.dll,SetSuspendState 0,1,0",
            CompletionAction::Hibernate => "shutdown /h",
            CompletionAction::Hangup => "rasdial /disconnect",
            CompletionAction::None => return,
        };
        let _ = std::process::Command::new("cmd")
            .args(["/C", cmd])
            .spawn();
    }
}

fn enforce_limits(shared: &Arc<Shared>) {
    let (enabled, limit_mb) = match shared.store.read() {
        Ok(st) => st
            .config
            .queues
            .iter()
            .find(|q| q.running)
            .map(|q| (q.schedule.limit_enabled, q.schedule.limit_mb))
            .unwrap_or((false, 0)),
        Err(_) => (false, 0),
    };
    if !enabled || limit_mb == 0 {
        return;
    }
    let total: u64 = shared
        .store
        .read()
        .map(|s| s.downloads.iter().map(|d| d.downloaded).sum())
        .unwrap_or(0);
    if total >= limit_mb as u64 * 1024 * 1024 {
        shared.emit(EngineEvent::Changed);
        let ids: Vec<u64> = shared
            .store
            .read()
            .map(|s| {
                s.downloads
                    .iter()
                    .filter(|d| d.status.is_active())
                    .map(|d| d.id)
                    .collect()
            })
            .unwrap_or_default();
        for id in ids {
            stop_one(shared, id, false);
        }
    }
}

// ---------------------------------------------------------------------------
// Transfer implementation
// ---------------------------------------------------------------------------

async fn run_download(shared: Arc<Shared>, id: u64) -> Result<bool> {
    let dl = shared
        .snapshot(id)
        .ok_or_else(|| anyhow!("download {id} disappeared"))?;
    let cfg = shared.store.read().map(|s| s.config.clone()).unwrap_or_default();
    let client = httpmeta::build_client(&cfg, &dl)?;
    shared.set_status(id, Status::Connecting);
    let probe = httpmeta::probe(&client, &cfg, &dl)
        .await
        .with_context(|| format!("probe {}", dl.url))?;

    // Apply server metadata.
    let mut file_name = dl.file_name.clone();
    if let Some(n) = &probe.file_name {
        let n = util::sanitize_filename(n);
        if !n.is_empty() {
            file_name = n;
        }
    }
    shared.update(id, |d| {
        d.final_url = Some(probe.final_url.clone());
        d.size = probe.size;
        d.resumable = probe.resumable;
        d.etag = probe.etag.clone();
        d.last_modified = probe.last_modified.clone();
        d.content_type = probe.content_type.clone();
        d.file_name = file_name.clone();
        d.started = Some(util::now());
        d.last_try = Some(util::now());
        d.error = None;
    });
    shared.set_status(id, Status::Downloading);

    let total = probe.size;
    let resumable = probe.resumable && total.is_some();
    let n_conn = if resumable {
        let mut maxc = cfg.max_connections.clamp(1, 32) as u64;
        if let Ok(u) = url::Url::parse(&probe.final_url) {
            if let Some(host) = u.host_str() {
                for e in &cfg.connection_exceptions {
                    if crate::config::wildcard_match(&e.server, host) {
                        maxc = maxc.min(e.max.clamp(1, 32) as u64);
                        break;
                    }
                }
            }
        }
        let size = total.unwrap_or(0);
        let by_size = (size / (512 * 1024)).max(1);
        maxc.min(by_size).min(32).max(1) as usize
    } else {
        1
    };

    // Build or restore parts.
    let mut parts = dl.parts.clone();
    let existing_total: u64 = parts
        .iter()
        .map(|p| p.end.saturating_sub(p.start) + 1)
        .sum();
    let reuse = !parts.is_empty()
        && parts.len() == n_conn
        && resumable
        && total.map(|t| existing_total == t).unwrap_or(false);
    if !reuse {
        parts = make_parts(total, n_conn);
        // Fresh start: clear old part files.
        let dir = cfg.temp_dir.join(format!("{}", id));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("cannot create temp dir {}", dir.display()))?;
    } else {
        // Trust a part file that is larger than the recorded position
        // (a crash can lose up to one progress write).
        let dir = cfg.temp_dir.join(format!("{}", id));
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("cannot create temp dir {}", dir.display()))?;
        for (i, p) in parts.iter_mut().enumerate() {
            let path = dir.join(format!("part{}", i));
            if let Ok(meta) = std::fs::metadata(&path) {
                let len = meta.len().min(p.len());
                if len > p.pos {
                    p.pos = len;
                }
            }
            if p.pos >= p.len() {
                p.done = true;
            }
        }
    }
    shared.update(id, |d| {
        d.parts = parts.clone();
        d.connections = parts.len() as u32;
    });
    if total == Some(0) {
        return finish_download(&shared, id, &cfg, total).await;
    }

    let ctl = Arc::new(RunCtl::new(&parts));
    if let Ok(mut c) = shared.controls.lock() {
        c.insert(id, ctl.clone());
    }
    let dir = cfg.temp_dir.join(format!("{}", id));
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("cannot create part dir {}", dir.display()))?;

    let mut set: JoinSet<Result<()>> = JoinSet::new();
    for (i, atom) in ctl.atoms().iter().enumerate() {
        set.spawn(fetch_part(
            shared.clone(),
            id,
            i,
            atom.clone(),
            ctl.clone(),
            client.clone(),
            cfg.clone(),
            dir.clone(),
        ));
    }

    let reporter = tokio::spawn(report_loop(shared.clone(), id, ctl.clone()));

    let mut first_err: Option<anyhow::Error> = None;
    while let Some(joined) = set.join_next().await {
        match joined {
            Ok(Ok(())) => {}
            Ok(Err(e)) => {
                if !ctl.cancel.load(Ordering::SeqCst) {
                    first_err = Some(e);
                }
                ctl.cancel.store(true, Ordering::SeqCst);
            }
            Err(e) => {
                if !ctl.cancel.load(Ordering::SeqCst) {
                    first_err = Some(anyhow!("transfer task failed: {e}"));
                }
                ctl.cancel.store(true, Ordering::SeqCst);
            }
        }
    }
    reporter.abort();
    final_sync(&shared, id, &ctl);
    if let Some(e) = first_err {
        return Err(e);
    }
    if ctl.cancel.load(Ordering::SeqCst) {
        return Ok(false);
    }
    if !ctl.all_done() {
        return Err(anyhow!("Download failed. Downloaded: {} Bytes Total", ctl.total_downloaded()));
    }
    shared.set_status(id, Status::Assembling);
    let sh = shared.clone();
    let c = cfg.clone();
    let ok = tokio::task::spawn_blocking(move || assemble(&sh, id, &c))
        .await
        .map_err(|e| anyhow!("assembly task failed: {e}"))??;
    let _ = ok;
    Ok(true)
}

fn make_parts(total: Option<u64>, n: usize) -> Vec<Part> {
    match total {
        Some(size) if size > 0 => {
            let n = n.min(size as usize).max(1);
            let chunk = size / n as u64;
            (0..n)
                .map(|i| {
                    let start = i as u64 * chunk;
                    let end = if i == n - 1 { size - 1 } else { (i as u64 + 1) * chunk - 1 };
                    Part {
                        start,
                        end,
                        pos: 0,
                        done: false,
                    }
                })
                .collect()
        }
        _ => vec![Part {
            start: 0,
            end: u64::MAX,
            pos: 0,
            done: false,
        }],
    }
}

async fn fetch_part(
    shared: Arc<Shared>,
    id: u64,
    index: usize,
    atom: Arc<PartAtom>,
    ctl: Arc<RunCtl>,
    client: reqwest::Client,
    cfg: Config,
    dir: PathBuf,
) -> Result<()> {
    let path = dir.join(format!("part{}", index));
    let len = if atom.end == u64::MAX {
        u64::MAX
    } else {
        atom.end - atom.start + 1
    };
    if len != u64::MAX && atom.pos.load(Ordering::Relaxed) >= len {
        atom.done.store(true, Ordering::Relaxed);
        return Ok(());
    }
    let file = std::fs::OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(&path)
        .with_context(|| format!("cannot open part file {}", path.display()))?;
    if atom.pos.load(Ordering::Relaxed) == 0 {
        file.set_len(0).ok();
    }
    let dl = shared.snapshot(id).ok_or_else(|| anyhow!("download disappeared"))?;
    let mut attempt = 0u32;
    loop {
        if ctl.cancel.load(Ordering::SeqCst) {
            return Ok(());
        }
        let pos = atom.pos.load(Ordering::Relaxed);
        if len != u64::MAX && pos >= len {
            atom.done.store(true, Ordering::Relaxed);
            return Ok(());
        }
        let start_abs = atom.start.saturating_add(pos);
        let range = if atom.end == u64::MAX {
            format!("bytes={}-", start_abs)
        } else {
            format!("bytes={}-{}", start_abs, atom.end)
        };
        let mut req = client
            .get(dl.final_url.as_deref().unwrap_or(&dl.url))
            .header(RANGE, range.clone());
        req = httpmeta::apply_headers(req, &cfg, &dl);
        if let Some(body) = &dl.post_data {
            if !body.is_empty() && start_abs == 0 {
                req = req
                    .header(reqwest::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(body.clone());
            }
        }
        let resp = match req.send().await {
            Ok(r) => r,
            Err(e) => {
                attempt += 1;
                if attempt > 5 {
                    return Err(anyhow!("Connection failed, restarting... ({e})"));
                }
                tokio::time::sleep(Duration::from_secs(2u64.pow(attempt.min(3)))).await;
                continue;
            }
        };
        let status = resp.status();
        if status == reqwest::StatusCode::RANGE_NOT_SATISFIABLE {
            if len != u64::MAX {
                atom.pos.store(len, Ordering::Relaxed);
                atom.done.store(true, Ordering::Relaxed);
            }
            return Ok(());
        }
        let partial = status == reqwest::StatusCode::PARTIAL_CONTENT;
        if !status.is_success() {
            attempt += 1;
            if attempt > 5 {
                return Err(anyhow!(
                    "Cannot download this file. Invalid http server reply (HTTP {}).",
                    status.as_u16()
                ));
            }
            tokio::time::sleep(Duration::from_secs(2u64.pow(attempt.min(3)))).await;
            continue;
        }
        if !partial && start_abs > 0 {
            return Err(anyhow!(
                "The data transfer has been interrupted and the server does not support \"resume\"."
            ));
        }
        if partial {
            if let Some(cr) = resp
                .headers()
                .get(reqwest::header::CONTENT_RANGE)
                .and_then(|v| v.to_str().ok())
            {
                if let Some((_, _, total)) = util::parse_content_range(cr) {
                    if let (Some(expected), Some(got)) = (dl.size, total) {
                        if expected != got {
                            return Err(anyhow!(
                                "The file size has been changed on the server.\nIt will be downloaded from the beginning."
                            ));
                        }
                    }
                }
            }
        }
        let mut stream = resp.bytes_stream();
        let mut wrote_any = false;
        while let Some(chunk) = stream.next().await {
            if ctl.cancel.load(Ordering::SeqCst) {
                return Ok(());
            }
            let chunk = match chunk {
                Ok(c) => c,
                Err(e) => {
                    attempt += 1;
                    if attempt > 6 {
                        return Err(anyhow!("An error occurred while receiving data ({e})"));
                    }
                    break;
                }
            };
            if chunk.is_empty() {
                continue;
            }
            use std::os::windows::fs::FileExt;
            let mut off = atom.pos.load(Ordering::Relaxed);
            file.seek_write(&chunk, off)
                .map_err(|e| anyhow!("File write error: {e}"))?;
            off += chunk.len() as u64;
            atom.pos.store(
                off.min(if len == u64::MAX { u64::MAX } else { len }),
                Ordering::Relaxed,
            );
            ctl.downloaded.fetch_add(chunk.len() as u64, Ordering::Relaxed);
            wrote_any = true;
            shared.limiter.consume(chunk.len()).await;
            if len != u64::MAX && off >= len {
                break;
            }
        }
        if len != u64::MAX && atom.pos.load(Ordering::Relaxed) >= len {
            atom.done.store(true, Ordering::Relaxed);
            return Ok(());
        }
        if atom.end == u64::MAX {
            // Open-ended stream ended: the file is complete.
            atom.done.store(true, Ordering::Relaxed);
            return Ok(());
        }
        if !wrote_any {
            attempt += 1;
            if attempt > 6 {
                return Err(anyhow!("Connection has been closed by server."));
            }
        }
        attempt = 0;
        tokio::time::sleep(Duration::from_millis(150)).await;
    }
}

fn pos_offset(_pos: u64, _atom: &PartAtom, _start_abs: u64) -> u64 {
    0
}

async fn report_loop(shared: Arc<Shared>, id: u64, ctl: Arc<RunCtl>) {
    let mut last = Instant::now();
    let mut last_bytes = ctl.total_downloaded();
    let mut save_at = Instant::now();
    loop {
        tokio::time::sleep(Duration::from_millis(400)).await;
        let now = Instant::now();
        let total = ctl.total_downloaded();
        let dt = now.duration_since(last).as_secs_f64().max(0.001);
        let speed = ((total.saturating_sub(last_bytes)) as f64 / dt) as u64;
        last = now;
        last_bytes = total;
        ctl.speed.store(speed, Ordering::Relaxed);
        let atoms = ctl.atoms();
        if let Ok(mut st) = shared.store.write() {
            if let Some(d) = st.get_mut(id) {
                d.downloaded = total;
                d.speed = speed;
                d.active_connections = atoms
                    .iter()
                    .filter(|p| !p.done.load(Ordering::Relaxed))
                    .count() as u32;
                for (i, a) in atoms.iter().enumerate() {
                    if let Some(p) = d.parts.get_mut(i) {
                        p.pos = a.pos.load(Ordering::Relaxed);
                        p.done = a.done.load(Ordering::Relaxed);
                    }
                }
            }
            if save_at.elapsed() > Duration::from_secs(3) {
                st.save_state_only();
                save_at = Instant::now();
            }
        }
    }
}

fn final_sync(shared: &Arc<Shared>, id: u64, ctl: &Arc<RunCtl>) {
    let atoms = ctl.atoms();
    if let Ok(mut st) = shared.store.write() {
        if let Some(d) = st.get_mut(id) {
            d.downloaded = ctl.total_downloaded();
            d.speed = 0;
            d.active_connections = 0;
            for (i, a) in atoms.iter().enumerate() {
                if let Some(p) = d.parts.get_mut(i) {
                    p.pos = a.pos.load(Ordering::Relaxed);
                    p.done = a.done.load(Ordering::Relaxed);
                }
            }
        }
        st.save_state_only();
    }
}

async fn finish_download(
    shared: &Arc<Shared>,
    id: u64,
    cfg: &Config,
    _total: Option<u64>,
) -> Result<bool> {
    let sh = shared.clone();
    let c = cfg.clone();
    tokio::task::spawn_blocking(move || assemble(&sh, id, &c))
        .await
        .map_err(|e| anyhow!("assembly task failed: {e}"))??;
    Ok(true)
}

fn assemble(shared: &Arc<Shared>, id: u64, cfg: &Config) -> Result<()> {
    let dl = shared
        .snapshot(id)
        .ok_or_else(|| anyhow!("download disappeared"))?;
    let dir = cfg.temp_dir.join(format!("{}", id));
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("cannot create assembly dir {}", dir.display()))?;
    let assembled = dir.join("assembled");
    {
        use std::io::{Read, Write};
        let mut out = std::fs::File::create(&assembled)
            .with_context(|| format!("cannot create {}", assembled.display()))?;
        let mut buf = vec![0u8; 1024 * 256];
        for (i, p) in dl.parts.iter().enumerate() {
            let path = dir.join(format!("part{}", i));
            let mut f = match std::fs::File::open(&path) {
                Ok(f) => f,
                Err(_) => {
                    if p.len() == 0 || p.pos == 0 {
                        continue;
                    }
                    return Err(anyhow!(
                        "Cannot rebuild the downloaded file. A part file is missing."
                    ));
                }
            };
            let mut remaining = if p.end == u64::MAX {
                u64::MAX
            } else {
                p.len()
            };
            loop {
                let want = buf.len().min(if remaining == u64::MAX { buf.len() } else { remaining as usize });
                if want == 0 {
                    break;
                }
                let n = f.read(&mut buf[..want])?;
                if n == 0 {
                    break;
                }
                out.write_all(&buf[..n])?;
                if remaining != u64::MAX {
                    remaining -= n as u64;
                }
            }
        }
        out.flush()?;
    }
    let final_len = std::fs::metadata(&assembled)?.len();
    if let Some(size) = dl.size {
        if final_len != size && size > 0 {
            // Try to recover from a stale part position.
            return Err(anyhow!(
                "An error has occurred during assembling the file (size {} != {})",
                final_len,
                size
            ));
        }
    }
    let dest_dir = if dl.save_dir.as_os_str().is_empty() {
        cfg.download_dir.clone()
    } else {
        dl.save_dir.clone()
    };
    std::fs::create_dir_all(&dest_dir)
        .with_context(|| format!("cannot create save dir {}", dest_dir.display()))?;
    let dest = match cfg.duplicate_mode {
        2 => dest_dir.join(&dl.file_name),
        _ => util::unique_path(&dest_dir, &dl.file_name),
    };
    if dest.exists() && cfg.duplicate_mode != 2 {
        let _ = std::fs::remove_file(&dest);
    }
    match std::fs::rename(&assembled, &dest) {
        Ok(()) => {}
        Err(_) => {
            // Different volume (temp dir vs. save dir).
            std::fs::copy(&assembled, &dest)
                .with_context(|| format!("cannot copy to {}", dest.display()))?;
            let _ = std::fs::remove_file(&assembled);
        }
    }
    if let Some(lm) = &dl.last_modified {
        if let Ok(t) = httpdate::parse_http_date(lm) {
            if let Ok(ft) = filetime_from_system(t) {
                set_file_times(&dest, ft);
            }
        }
    }
    shared.update(id, |d| {
        d.downloaded = final_len;
        d.completed = Some(util::now());
        d.status = Status::Complete;
        d.speed = 0;
        d.active_connections = 0;
    });
    let _ = std::fs::remove_dir_all(&dir);
    if let Ok(st) = shared.store.read() {
        st.save_state_only();
    }
    Ok(())
}

fn filetime_from_system(t: std::time::SystemTime) -> Result<u64> {
    let d = t
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| anyhow!("bad time: {e}"))?;
    // 100ns intervals since 1601-01-01.
    Ok(d.as_secs() * 10_000_000 + 116_444_736_000_000_000 + d.subsec_nanos() as u64 / 100)
}

#[cfg(windows)]
fn set_file_times(path: &std::path::Path, ft: u64) {
    use std::os::windows::ffi::OsStrExt;
    #[repr(C)]
    struct FileTime {
        low: u32,
        high: u32,
    }
    extern "system" {
        fn SetFileTime(h: isize, c: *const FileTime, a: *const FileTime, w: *const FileTime)
            -> i32;
        fn CreateFileW(
            name: *const u16,
            access: u32,
            share: u32,
            sec: *const u8,
            disposition: u32,
            flags: u32,
            template: isize,
        ) -> isize;
        fn CloseHandle(h: isize) -> i32;
    }
    let mut wide: Vec<u16> = path.as_os_str().encode_wide().collect();
    wide.push(0);
    let ft = FileTime {
        low: ft as u32,
        high: (ft >> 32) as u32,
    };
    unsafe {
        // FILE_WRITE_ATTRIBUTES = 0x100, share all, OPEN_EXISTING = 3
        let h = CreateFileW(wide.as_ptr(), 0x100, 7, std::ptr::null(), 3, 0x80, 0);
        if h != -1 && h != 0 {
            let _ = SetFileTime(h, std::ptr::null(), std::ptr::null(), &ft);
            let _ = CloseHandle(h);
        }
    }
}

#[cfg(not(windows))]
fn set_file_times(_path: &std::path::Path, _ft: u64) {}

/// Helper used by the UI to re-plan a stopped download from scratch.
pub fn reset_download(shared: &Arc<Shared>, id: u64) {
    shared.update(id, |d| {
        d.parts.clear();
        d.downloaded = 0;
        d.size = None;
        d.error = None;
    });
}

/// Convenience used by the browser integration to enqueue a captured link.
pub fn add_from_browser(engine: &Engine, url: &str, referer: &str, cookies: Option<String>) -> u64 {
    engine.add(AddRequest {
        url: url.into(),
        referer: referer.into(),
        cookies,
        start: true,
        ..Default::default()
    })
}

/// Ensure all internal queues have sane ids after loading older configs.
pub fn normalize_queues(cfg: &mut Config) {
    if !cfg.queues.iter().any(|q| q.id == 1) {
        cfg.queues.push(Queue::main());
    }
}
