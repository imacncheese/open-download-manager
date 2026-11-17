//! Visual theme: a clean, modern take on the classic download-manager look.
//! Segoe UI typography, soft surfaces, accent-blue selection and smooth
//! state transitions. Dark mode follows the Windows app mode.

use eframe::egui::{
    self, Color32, FontData, FontDefinitions, FontFamily, Rounding, Shadow, Stroke, Visuals,
};
use std::sync::atomic::{AtomicU32, Ordering};

pub const ACCENT: Color32 = Color32::from_rgb(0, 120, 212);
pub const ACCENT_HOVER: Color32 = Color32::from_rgb(22, 110, 190);
static ACCENT_RGB: AtomicU32 = AtomicU32::new(0x0078D4);
pub const ODM_BLUE: Color32 = ACCENT;
pub const ODM_GREEN: Color32 = Color32::from_rgb(16, 124, 64);
pub const ODM_RED: Color32 = Color32::from_rgb(196, 43, 28);
pub const ODM_AMBER: Color32 = Color32::from_rgb(157, 93, 0);

pub fn set_accent(accent: odm_core::config::AccentColor) {
    use odm_core::config::AccentColor;
    let rgb = match accent {
        AccentColor::Blue => 0x0078D4,
        AccentColor::Violet => 0x7C4DFF,
        AccentColor::Teal => 0x009688,
        AccentColor::Orange => 0xE67E22,
        AccentColor::Rose => 0xD43C78,
        AccentColor::Green => 0x109650,
    };
    ACCENT_RGB.store(rgb, Ordering::Relaxed);
}

pub fn accent() -> Color32 {
    let v = ACCENT_RGB.load(Ordering::Relaxed);
    Color32::from_rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
}

pub fn accent_hover() -> Color32 {
    let c = accent();
    Color32::from_rgb(
        (c.r() as f32 * 0.82) as u8,
        (c.g() as f32 * 0.82) as u8,
        (c.b() as f32 * 0.82) as u8,
    )
}

pub const PANEL_LIGHT: Color32 = Color32::from_rgb(243, 243, 243);
pub const PANEL_DARK: Color32 = Color32::from_rgb(32, 33, 36);
pub const TOOLBAR_LIGHT: Color32 = Color32::from_rgb(236, 237, 240);
pub const TOOLBAR_DARK: Color32 = Color32::from_rgb(43, 44, 47);
pub const LIST_LIGHT: Color32 = Color32::from_rgb(255, 255, 255);
pub const LIST_DARK: Color32 = Color32::from_rgb(30, 31, 34);

pub fn surface(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(38, 39, 43)
    } else {
        Color32::from_rgb(250, 250, 251)
    }
}

pub fn border(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(64, 66, 71)
    } else {
        Color32::from_rgb(201, 203, 208)
    }
}

pub fn muted(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(154, 160, 166)
    } else {
        Color32::from_rgb(96, 99, 104)
    }
}

pub fn text(dark: bool) -> Color32 {
    if dark {
        Color32::from_rgb(232, 234, 237)
    } else {
        Color32::from_rgb(32, 33, 36)
    }
}

pub fn toolbar_bg(dark: bool) -> Color32 {
    if dark {
        TOOLBAR_DARK
    } else {
        TOOLBAR_LIGHT
    }
}

pub fn list_bg(dark: bool) -> Color32 {
    if dark {
        LIST_DARK
    } else {
        LIST_LIGHT
    }
}

pub fn windows_dark_mode() -> bool {
    #[cfg(windows)]
    {
        use winreg::enums::HKEY_CURRENT_USER;
        use winreg::RegKey;
        if let Ok(key) = RegKey::predef(HKEY_CURRENT_USER).open_subkey(
            "Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize",
        ) {
            if let Ok(v) = key.get_value::<u32, _>("AppsUseLightTheme") {
                return v == 0;
            }
        }
    }
    false
}

pub fn install(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    for path in [
        r"C:\Windows\Fonts\segoeui.ttf",
        r"C:\Windows\Fonts\segoeuib.ttf",
        r"C:\Windows\Fonts\tahoma.ttf",
    ] {
        if let Ok(bytes) = std::fs::read(path) {
            let name = std::path::Path::new(path)
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .to_string();
            fonts
                .font_data
                .insert(name.clone(), FontData::from_owned(bytes).into());
            if name == "segoeui" {
                fonts
                    .families
                    .entry(FontFamily::Proportional)
                    .or_default()
                    .insert(0, name.clone());
                fonts
                    .families
                    .entry(FontFamily::Monospace)
                    .or_default()
                    .insert(0, name);
            }
        }
    }
    ctx.set_fonts(fonts);
    apply(ctx, false);
}

pub fn apply(ctx: &egui::Context, dark: bool) {
    let mut style = (*ctx.style()).clone();
    style.animation_time = 0.18;
    style.spacing.item_spacing = egui::vec2(7.0, 5.0);
    style.spacing.button_padding = egui::vec2(10.0, 4.0);
    style.spacing.interact_size = egui::vec2(26.0, 24.0);
    style.spacing.indent = 18.0;
    style.text_styles = [
        (egui::TextStyle::Small, egui::FontId::proportional(11.0)),
        (egui::TextStyle::Body, egui::FontId::proportional(13.0)),
        (egui::TextStyle::Button, egui::FontId::proportional(13.0)),
        (egui::TextStyle::Heading, egui::FontId::proportional(16.0)),
        (egui::TextStyle::Monospace, egui::FontId::monospace(12.0)),
    ]
    .into();

    let mut v = if dark { Visuals::dark() } else { Visuals::light() };
    v.panel_fill = if dark { PANEL_DARK } else { PANEL_LIGHT };
    v.window_fill = surface(dark);
    v.extreme_bg_color = list_bg(dark);
    v.faint_bg_color = if dark {
        Color32::from_rgb(35, 36, 40)
    } else {
        Color32::from_rgb(247, 248, 250)
    };
    v.window_stroke = Stroke::new(1.0, border(dark));
    v.window_corner_radius = Rounding::same(8);
    v.menu_corner_radius = Rounding::same(6);
    v.window_shadow = Shadow {
        offset: [0, 5],
        blur: 22,
        spread: 0,
        color: if dark {
            Color32::from_black_alpha(110)
        } else {
            Color32::from_black_alpha(45)
        },
    };
    v.popup_shadow = v.window_shadow;

    v.widgets.noninteractive.bg_fill = surface(dark);
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, border(dark));
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, text(dark));
    v.widgets.noninteractive.corner_radius = Rounding::same(4);

    v.widgets.inactive.bg_fill = if dark {
        Color32::from_rgb(52, 54, 58)
    } else {
        Color32::from_rgb(250, 250, 251)
    };
    v.widgets.inactive.weak_bg_fill = v.widgets.inactive.bg_fill;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, border(dark));
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, text(dark));
    v.widgets.inactive.corner_radius = Rounding::same(4);

    v.widgets.hovered.bg_fill = if dark {
        Color32::from_rgb(63, 66, 72)
    } else {
        Color32::from_rgb(235, 244, 252)
    };
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, accent());
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, if dark { Color32::WHITE } else { text(false) });
    v.widgets.hovered.corner_radius = Rounding::same(4);

    v.widgets.active.bg_fill = if dark {
        Color32::from_rgb(0, 90, 170)
    } else {
        Color32::from_rgb(204, 228, 247)
    };
    v.widgets.active.bg_stroke = Stroke::new(1.0, accent_hover());
    v.widgets.active.fg_stroke = Stroke::new(1.0, text(dark));
    v.widgets.active.corner_radius = Rounding::same(4);

    v.selection.bg_fill = if dark {
        Color32::from_rgb(11, 87, 208)
    } else {
        accent()
    };
    v.selection.stroke = Stroke::new(1.0, Color32::WHITE);
    v.hyperlink_color = accent();
    v.override_text_color = Some(text(dark));
    style.visuals = v;
    ctx.set_style(style);
}

pub fn panel_frame(dark: bool) -> egui::Frame {
    egui::Frame::none()
        .fill(if dark { PANEL_DARK } else { PANEL_LIGHT })
        .inner_margin(egui::Margin::symmetric(0, 0))
}

/// Sunken 1px border used around list views and group boxes.
pub fn sunken(ui: &mut egui::Ui, dark: bool) -> egui::Response {
    let rect = ui.available_rect_before_wrap();
    let resp = ui.allocate_rect(rect, egui::Sense::hover());
    ui.painter().rect_stroke(
        rect,
        Rounding::ZERO,
        Stroke::new(1.0, border(dark)),
        egui::StrokeKind::Inside,
    );
    resp
}

/// Blend two colors (gamma space is fine for UI accents).
pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let f = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgba_unmultiplied(
        f(a.r(), b.r()),
        f(a.g(), b.g()),
        f(a.b(), b.b()),
        f(a.a(), b.a()),
    )
}
