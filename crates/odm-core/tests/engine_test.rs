//! End-to-end engine test against a local HTTP server with Range support.

use odm_core::config::ProxyMode;
use odm_core::{AddRequest, Engine, Status};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn payload(n: usize) -> Vec<u8> {
    (0..n).map(|i| (i * 31 + 7) as u8).collect()
}

/// Tiny HTTP/1.1 server: HEAD, GET and single-range GET.
fn start_server(data: Vec<u8>) -> (u16, Arc<AtomicUsize>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let hits = Arc::new(AtomicUsize::new(0));
    let h2 = hits.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let mut s = match stream {
                Ok(s) => s,
                Err(_) => continue,
            };
            let data = data.clone();
            let hits = h2.clone();
            std::thread::spawn(move || {
                hits.fetch_add(1, Ordering::SeqCst);
                let mut buf = [0u8; 8192];
                let n = s.read(&mut buf).unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                let head_only = req.starts_with("HEAD");
                let range = req
                    .lines()
                    .find(|l| l.to_ascii_lowercase().starts_with("range:"))
                    .and_then(|l| l.split('=').nth(1))
                    .and_then(|r| {
                        let (a, b) = r.trim().split_once('-')?;
                        let a: usize = a.parse().ok()?;
                        let b = if b.trim().is_empty() {
                            data.len() - 1
                        } else {
                            b.trim().parse().ok()?
                        };
                        Some((a, b))
                    });
                let headers = match range {
                    Some((a, b)) => format!(
                        "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {}-{}/{}\r\nAccept-Ranges: bytes\r\nContent-Type: application/octet-stream\r\nConnection: close\r\n\r\n",
                        b - a + 1, a, b, data.len()
                    ),
                    None => format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nAccept-Ranges: bytes\r\nContent-Type: application/octet-stream\r\nConnection: close\r\n\r\n",
                        data.len()
                    ),
                };
                let _ = s.write_all(headers.as_bytes());
                if !head_only {
                    let (a, b) = range.unwrap_or((0, data.len() - 1));
                    let _ = s.write_all(&data[a..=b]);
                }
                let _ = s.flush();
            });
        }
    });
    (port, hits)
}

#[test]
fn segmented_download_completes_and_matches() {
    let tmp = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("odm-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    std::env::set_var("APPDATA", &tmp);

    let data = payload(1024 * 1024 + 12345);
    let (port, hits) = start_server(data.clone());
    let mut engine = Engine::new(None).unwrap();
    let mut cfg = engine.config();
    cfg.proxy.mode = ProxyMode::None;
    cfg.temp_dir = tmp.join("temp");
    cfg.download_dir = tmp.join("out");
    cfg.max_connections = 4;
    cfg.max_concurrent_downloads = 2;
    cfg.user_agent = "odm-test".into();
    for c in &mut cfg.categories {
        c.folder = tmp.join("out");
    }
    std::fs::create_dir_all(tmp.join("out")).unwrap();
    engine.set_config(cfg);

    let id = engine.add(AddRequest {
        url: format!("http://127.0.0.1:{port}/file.bin"),
        file_name: Some("file.bin".into()),
        start: true,
        ..Default::default()
    });
    assert!(id > 0, "download id");

    let deadline = Instant::now() + Duration::from_secs(30);
    let mut status = Status::Queued;
    while Instant::now() < deadline {
        if let Some(d) = engine.snapshot().into_iter().find(|d| d.id == id) {
            status = d.status;
            if status == Status::Complete || status == Status::Error {
                if status == Status::Error {
                    panic!("download failed: {:?}", d.error);
                }
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    assert_eq!(status, Status::Complete, "download should complete");
    let out = tmp.join("out").join("file.bin");
    let got = std::fs::read(&out).expect("output file");
    assert_eq!(got.len(), data.len(), "file length");
    assert_eq!(got, data, "file content");
    assert!(
        hits.load(Ordering::SeqCst) >= 3,
        "expected probe + multiple range requests, got {}",
        hits.load(Ordering::SeqCst)
    );
    engine.shutdown();
    let _ = std::fs::remove_dir_all(&tmp);
}
