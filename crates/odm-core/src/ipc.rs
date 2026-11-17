//! Local IPC used by the browser native-messaging host and secondary app
//! instances. Loopback-only, token authenticated.

use crate::engine::{AddRequest, Cmd, EngineEvent, Shared};
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct IpcRequest {
    #[serde(default)]
    pub token: String,
    pub cmd: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub referer: String,
    #[serde(default)]
    pub cookies: Option<String>,
    #[serde(default)]
    pub user_agent: Option<String>,
    #[serde(default)]
    pub file_name: Option<String>,
    #[serde(default)]
    pub save_dir: Option<String>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub post_data: Option<String>,
    #[serde(default = "default_true")]
    pub start: bool,
    #[serde(default)]
    pub queue: Option<u64>,
    #[serde(default)]
    pub urls: Vec<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct IpcResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
}

pub async fn serve(shared: Arc<Shared>, tx: tokio::sync::mpsc::UnboundedSender<Cmd>) -> u16 {
    let wanted = shared
        .store
        .read()
        .map(|s| s.config.ipc_port)
        .unwrap_or(57443);
    let mut listener = None;
    let mut port = wanted;
    for candidate in wanted..wanted.saturating_add(24) {
        if let Ok(l) = TcpListener::bind(("127.0.0.1", candidate)).await {
            listener = Some(l);
            port = candidate;
            break;
        }
    }
    let Some(listener) = listener else {
        crate::persist::log("IPC: no loopback port available");
        return 0;
    };
    let info = serde_json::json!({
        "port": port,
        "token": shared.ipc_token,
        "pid": std::process::id(),
        "app": "Open Download Manager",
    });
    let _ = crate::persist::write_atomic(
        &crate::paths::ipc_file(),
        &serde_json::to_vec_pretty(&info).unwrap_or_default(),
    );
    let sh = shared.clone();
    tokio::spawn(async move {
        loop {
            match listener.accept().await {
                Ok((sock, _addr)) => {
                    let sh = sh.clone();
                    let tx = tx.clone();
                    tokio::spawn(async move {
                        let _ = handle(sock, sh, tx).await;
                    });
                }
                Err(_) => break,
            }
        }
    });
    port
}

async fn handle(
    sock: tokio::net::TcpStream,
    shared: Arc<Shared>,
    tx: tokio::sync::mpsc::UnboundedSender<Cmd>,
) -> std::io::Result<()> {
    let (rd, mut wr) = sock.into_split();
    let mut reader = BufReader::new(rd);
    let mut line = String::new();
    loop {
        line.clear();
        let n = tokio::time::timeout(std::time::Duration::from_secs(30), reader.read_line(&mut line))
            .await
            .ok()
            .and_then(|r| r.ok())
            .unwrap_or(0);
        if n == 0 {
            break;
        }
        let req: IpcRequest = match serde_json::from_str(line.trim()) {
            Ok(r) => r,
            Err(e) => {
                let resp = IpcResponse {
                    ok: false,
                    error: Some(format!("bad request: {e}")),
                    ..Default::default()
                };
                wr.write_all(format!("{}\n", serde_json::to_string(&resp).unwrap()).as_bytes())
                    .await?;
                continue;
            }
        };
        if !shared.ipc_token.is_empty() && req.token != shared.ipc_token {
            let resp = IpcResponse {
                ok: false,
                error: Some("unauthorized".into()),
                ..Default::default()
            };
            wr.write_all(format!("{}\n", serde_json::to_string(&resp).unwrap()).as_bytes())
                .await?;
            continue;
        }
        let resp = match req.cmd.as_str() {
            "ping" => IpcResponse {
                ok: true,
                version: Some(env!("CARGO_PKG_VERSION").into()),
                port: Some(shared.ipc_port.load(Ordering::SeqCst) as u16),
                ..Default::default()
            },
            "add" | "add_url" => {
                if req.url.trim().is_empty() {
                    IpcResponse {
                        ok: false,
                        error: Some("empty url".into()),
                        ..Default::default()
                    }
                } else {
                    let mut add = AddRequest {
                        url: req.url.clone(),
                        referer: req.referer.clone(),
                        cookies: req.cookies.clone(),
                        user_agent: req.user_agent.clone(),
                        file_name: req.file_name.clone(),
                        description: req.description.clone(),
                        post_data: req.post_data.clone(),
                        start: req.start,
                        silent: true,
                        ..Default::default()
                    };
                    if let Some(dir) = &req.save_dir {
                        if !dir.trim().is_empty() {
                            add.save_dir = Some(std::path::PathBuf::from(dir));
                        }
                    }
                    if let Some(cat) = &req.category {
                        if !cat.trim().is_empty() {
                            add.category = Some(cat.clone());
                        }
                    }
                    if let Some(q) = req.queue {
                        add.queue_id = Some(q);
                    }
                    let _ = tx.send(Cmd::AddFire(add));
                    IpcResponse {
                        ok: true,
                        ..Default::default()
                    }
                }
            }
            "add_many" => {
                for u in &req.urls {
                    if u.trim().is_empty() {
                        continue;
                    }
                    let _ = tx.send(Cmd::AddFire(AddRequest {
                        url: u.clone(),
                        referer: req.referer.clone(),
                        cookies: req.cookies.clone(),
                        start: req.start,
                        silent: true,
                        ..Default::default()
                    }));
                }
                IpcResponse {
                    ok: true,
                    ..Default::default()
                }
            }
            "status" => {
                let (total, active) = shared
                    .store
                    .read()
                    .map(|s| (s.downloads.len(), s.active_count()))
                    .unwrap_or((0, 0));
                let _ = (total, active);
                IpcResponse {
                    ok: true,
                    ..Default::default()
                }
            }
            "focus" => {
                let _ = shared.emit(EngineEvent::Changed);
                IpcResponse {
                    ok: true,
                    ..Default::default()
                }
            }
            other => IpcResponse {
                ok: false,
                error: Some(format!("unknown command: {other}")),
                ..Default::default()
            },
        };
        wr.write_all(format!("{}\n", serde_json::to_string(&resp).unwrap()).as_bytes())
            .await?;
    }
    Ok(())
}

/// Blocking helper for the native host.
pub fn send_request(port: u16, req: &IpcRequest) -> std::io::Result<IpcResponse> {
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpStream;
    let mut s = TcpStream::connect_timeout(
        &format!("127.0.0.1:{}", port)
            .parse()
            .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidInput, "bad port"))?,
        std::time::Duration::from_secs(5),
    )?;
    s.set_read_timeout(Some(std::time::Duration::from_secs(10)))?;
    let line = serde_json::to_string(req).unwrap();
    s.write_all(line.as_bytes())?;
    s.write_all(b"\n")?;
    let mut reader = BufReader::new(s);
    let mut out = String::new();
    reader.read_line(&mut out)?;
    serde_json::from_str(out.trim())
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}
