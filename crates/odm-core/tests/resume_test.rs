//! Resume test: stop a running transfer, then resume it from saved part
//! positions and verify the assembled file byte-for-byte.

use odm_core::config::ProxyMode;
use odm_core::{AddRequest, Engine, Status};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::{Duration, Instant};

fn payload(n: usize) -> Vec<u8> {
    (0..n).map(|i| (i * 17 + 3) as u8).collect()
}

/// Range-capable server that writes the body slowly enough to interrupt.
fn slow_server(data: Vec<u8>) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let mut s = match stream {
                Ok(s) => s,
                Err(_) => continue,
            };
            let data = data.clone();
            std::thread::spawn(move || {
                let mut buf = [0u8; 8192];
                let n = s.read(&mut buf).unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]).to_string();
                let head = req.starts_with("HEAD");
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
                let (a, b) = range.unwrap_or((0, data.len() - 1));
                let headers = if range.is_some() {
                    format!(
                        "HTTP/1.1 206 Partial Content\r\nContent-Length: {}\r\nContent-Range: bytes {}-{}/{}\r\nAccept-Ranges: bytes\r\nConnection: close\r\n\r\n",
                        b - a + 1, a, b, data.len()
                    )
                } else {
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nAccept-Ranges: bytes\r\nConnection: close\r\n\r\n",
                        data.len()
                    )
                };
                let _ = s.write_all(headers.as_bytes());
                if head {
                    return;
                }
                let body = &data[a..=b];
                let mut off = 0;
                while off < body.len() {
                    let end = (off + 32768).min(body.len());
                    if s.write_all(&body[off..end]).is_err() {
                        return;
                    }
                    let _ = s.flush();
                    off = end;
                    std::thread::sleep(Duration::from_millis(12));
                }
            });
        }
    });
    port
}

#[test]
fn resume_after_stop() {
    let tmp = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("odm-resume-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).unwrap();
    std::env::set_var("ODM_HOME", &tmp);

    let data = payload(700_000);
    let port = slow_server(data.clone());
    let mut engine = Engine::new(None).unwrap();
    let mut cfg = engine.config();
    cfg.proxy.mode = ProxyMode::None;
    cfg.temp_dir = tmp.join("temp");
    cfg.download_dir = tmp.join("out");
    cfg.max_connections = 2;
    cfg.user_agent = "odm-test".into();
    for c in &mut cfg.categories {
        c.folder = tmp.join("out");
    }
    std::fs::create_dir_all(tmp.join("out")).unwrap();
    engine.set_config(cfg);

    let id = engine.add(AddRequest {
        url: format!("http://127.0.0.1:{port}/resume.bin"),
        file_name: Some("resume.bin".into()),
        start: true,
        ..Default::default()
    });
    assert!(id > 0);

    // Wait until some bytes have arrived, then stop.
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut downloaded = 0;
    while Instant::now() < deadline {
        if let Some(d) = engine.snapshot().into_iter().find(|d| d.id == id) {
            downloaded = d.downloaded;
            if downloaded > 100_000 {
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(downloaded > 0, "some bytes should arrive before stop");
    engine.stop(id, false);
    std::thread::sleep(Duration::from_millis(500));
    let stopped_pos = engine
        .snapshot()
        .into_iter()
        .find(|d| d.id == id)
        .map(|d| d.downloaded)
        .unwrap_or(0);
    assert!(stopped_pos > 0, "partial data expected");

    engine.start(id);
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut final_status = Status::Stopped;
    while Instant::now() < deadline {
        if let Some(d) = engine.snapshot().into_iter().find(|d| d.id == id) {
            final_status = d.status;
            if final_status == Status::Error {
                panic!("resume failed: {:?}", d.error);
            }
            if final_status == Status::Complete {
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    assert_eq!(final_status, Status::Complete, "resume should complete");
    let got = std::fs::read(tmp.join("out").join("resume.bin")).expect("output");
    assert_eq!(got.len(), data.len());
    assert_eq!(got, data, "resumed content must match exactly");
    engine.shutdown();
    let _ = std::fs::remove_dir_all(&tmp);
}
