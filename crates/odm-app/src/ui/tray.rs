//! System tray icon and menu (IDM's "tray icon" feature).

use tray_icon::menu::{Menu, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

pub struct Tray {
    pub icon: TrayIcon,
    pub show: MenuItem,
    pub exit: MenuItem,
}

pub fn create() -> Option<Tray> {
    let show = MenuItem::new("Show Open Download Manager", true, None);
    let pause = MenuItem::new("Pause all downloads", true, None);
    let stop = MenuItem::new("Stop all downloads", true, None);
    let exit = MenuItem::new("Exit", true, None);
    let menu = Menu::new();
    let _ = menu.append(&show);
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&pause);
    let _ = menu.append(&stop);
    let _ = menu.append(&PredefinedMenuItem::separator());
    let _ = menu.append(&exit);
    let icon = Icon::from_rgba(make_icon(32), 32, 32).ok()?;
    let tray = TrayIconBuilder::new()
        .with_menu(Box::new(menu))
        .with_tooltip("Open Download Manager")
        .with_icon(icon)
        .build()
        .ok()?;
    Some(Tray {
        icon: tray,
        show,
        exit,
    })
}

/// Blue rounded square with the IDM green download arrow.
fn make_icon(size: u32) -> Vec<u8> {
    let mut buf = vec![0u8; (size * size * 4) as usize];
    let s = size as f32;
    for y in 0..size {
        for x in 0..size {
            let fx = x as f32 / s;
            let fy = y as f32 / s;
            let i = ((y * size + x) * 4) as usize;
            // Background: rounded blue square.
            let dx = (fx - 0.5).abs() * 2.0;
            let dy = (fy - 0.5).abs() * 2.0;
            let inside = dx < 0.92 && dy < 0.92;
            let (r, g, b, a) = if !inside {
                (0, 0, 0, 0)
            } else {
                (20, 90, 170, 255)
            };
            // Arrow shaft (x 0.42..0.58, y 0.12..0.55) and head.
            let shaft = fx > 0.41 && fx < 0.59 && fy > 0.12 && fy < 0.52;
            let head = fy >= 0.52
                && fy < 0.72
                && (fx - 0.5).abs() < (0.72 - fy) * 1.55;
            let tray = fy > 0.78 && fy < 0.9 && fx > 0.2 && fx < 0.8;
            let (ar, ag, ab) = if shaft || head {
                (60, 200, 70)
            } else if tray {
                (60, 200, 70)
            } else {
                (r, g, b)
            };
            buf[i] = ar;
            buf[i + 1] = ag;
            buf[i + 2] = ab;
            buf[i + 3] = a;
        }
    }
    buf
}
