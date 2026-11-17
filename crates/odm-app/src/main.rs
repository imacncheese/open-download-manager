#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ui;

use eframe::egui;
use odm_core::ipc::IpcRequest;
use odm_core::{AddRequest, Engine, EngineEvent};
use ui::IdmApp;

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let urls: Vec<String> = args
        .iter()
        .filter(|a| a.starts_with("http://") || a.starts_with("https://") || a.starts_with("ftp://"))
        .cloned()
        .collect();

    // Single instance: forward links to the running window and exit.
    let instance = single_instance::SingleInstance::new("open-download-manager-1.0").ok();
    let is_single = instance.as_ref().map(|i| i.is_single()).unwrap_or(true);
    if !is_single {
        if let Ok(text) = std::fs::read_to_string(odm_core::paths::ipc_file()) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
                let port = v.get("port").and_then(|p| p.as_u64()).unwrap_or(0) as u16;
                let token = v.get("token").and_then(|t| t.as_str()).unwrap_or("").to_string();
                if port > 0 {
                    for u in &urls {
                        let req = IpcRequest {
                            token: token.clone(),
                            cmd: "add".into(),
                            url: u.clone(),
                            ..Default::default()
                        };
                        let _ = odm_core::ipc::send_request(port, &req);
                    }
                    // Bring the existing (possibly tray-hidden) window forward.
                    let req = IpcRequest {
                        token,
                        cmd: "focus".into(),
                        ..Default::default()
                    };
                    let _ = odm_core::ipc::send_request(port, &req);
                }
            }
        }
        // The existing instance may be hidden in the tray and not processing
        // UI updates; restore its window directly.
        #[cfg(windows)]
        restore_existing_window();
        return Ok(());
    }
    // Keep the single-instance handle alive for the process lifetime.
    std::mem::forget(instance);

    let (tx, rx) = std::sync::mpsc::channel::<EngineEvent>();
    let engine = Engine::new(Some(tx)).expect("engine init");
    for u in urls {
        engine.add(AddRequest {
            url: u,
            start: true,
            ..Default::default()
        });
    }

    let icon = make_icon(32);
    let icon = egui::IconData {
        rgba: icon,
        width: 32,
        height: 32,
    };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1219.0, 679.0])
            .with_min_inner_size([760.0, 430.0])
            .with_title("Open Download Manager")
            .with_icon(std::sync::Arc::new(icon)),
        ..Default::default()
    };
    eframe::run_native(
        "Open Download Manager",
        options,
        Box::new(move |cc| Ok(Box::new(IdmApp::new(cc, engine, rx)))),
    )
}

/// Find the running main window by title and bring it back from the tray.
#[cfg(windows)]
fn restore_existing_window() {
    #[link(name = "user32")]
    extern "system" {
        fn FindWindowW(class: *const u16, title: *const u16) -> isize;
        fn ShowWindow(hwnd: isize, cmd: i32) -> i32;
        fn SetForegroundWindow(hwnd: isize) -> i32;
    }
    const SW_SHOW: i32 = 5;
    const SW_RESTORE: i32 = 9;
    let title: Vec<u16> = "Open Download Manager\0".encode_utf16().collect();
    unsafe {
        let hwnd = FindWindowW(std::ptr::null(), title.as_ptr());
        if hwnd != 0 {
            ShowWindow(hwnd, SW_RESTORE);
            ShowWindow(hwnd, SW_SHOW);
            SetForegroundWindow(hwnd);
        }
    }
}

/// Blue tile + green arrow, same artwork as the tray icon.
fn make_icon(size: u32) -> Vec<u8> {
    let mut buf = vec![0u8; (size * size * 4) as usize];
    let s = size as f32;
    for y in 0..size {
        for x in 0..size {
            let fx = x as f32 / s;
            let fy = y as f32 / s;
            let i = ((y * size + x) * 4) as usize;
            let dx = (fx - 0.5).abs() * 2.0;
            let dy = (fy - 0.5).abs() * 2.0;
            let inside = dx < 0.94 && dy < 0.94;
            let mut px = if inside { [20u8, 90, 170, 255] } else { [0, 0, 0, 0] };
            let shaft = fx > 0.41 && fx < 0.59 && fy > 0.12 && fy < 0.52;
            let head = fy >= 0.52 && fy < 0.72 && (fx - 0.5).abs() < (0.72 - fy) * 1.55;
            let tray = fy > 0.78 && fy < 0.9 && fx > 0.2 && fx < 0.8;
            if shaft || head || tray {
                px = [60, 200, 70, 255];
            }
            buf[i..i + 4].copy_from_slice(&px);
        }
    }
    buf
}
