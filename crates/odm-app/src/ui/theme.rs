//! Visual theme reproducing IDM 6.43's Win32 look: light grey chrome, sunken
//! list, classic blue selection and Segoe UI typography. Dark mode follows
//! the Windows "app mode" setting.

use eframe::egui::{self, Color32, FontData, FontDefinitions, FontFamily, Rounding, Stroke, Visuals};

pub const ODM_BLUE: Color32 = Color32::from_rgb(0, 120, 215);
pub const ODM_GREEN: Color32 = Color32::from_rgb(0, 153, 0);
pub const ODM_RED: Color32 = Color32::from_rgb(200, 30, 30);
pub const PANEL_LIGHT: Color32 = Color32::from_rgb(240, 240, 240);
pub const PANEL_DARK: Color32 = Color32::from_rgb(32, 32, 32);
pub const TOOLBAR_LIGHT: Color32 = Color32::from_rgb(235, 236, 239);
pub const TOOLBAR_DARK: Color32 = Color32::from_rgb(45, 45, 48);
pub const LIST_LIGHT: Color32 = Color32::from_rgb(255, 255, 255);
pub const LIST_DARK: Color32 = Color32::from_rgb(30, 30, 30);

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
    style.spacing.item_spacing = egui::vec2(6.0, 4.0);
    style.spacing.button_padding = egui::vec2(8.0, 3.0);
    style.spacing.interact_size = egui::vec2(24.0, 22.0);
    style.text_styles = [
        (egui::TextStyle::Small, egui::FontId::proportional(11.0)),
        (egui::TextStyle::Body, egui::FontId::proportional(12.5)),
        (egui::TextStyle::Button, egui::FontId::proportional(12.5)),
        (egui::TextStyle::Heading, egui::FontId::proportional(15.0)),
        (egui::TextStyle::Monospace, egui::FontId::monospace(12.0)),
    ]
    .into();
    let mut v = if dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };
    v.panel_fill = if dark { PANEL_DARK } else { PANEL_LIGHT };
    v.window_fill = if dark {
        Color32::from_rgb(40, 40, 40)
    } else {
        Color32::from_rgb(247, 247, 247)
    };
    v.extreme_bg_color = if dark { LIST_DARK } else { LIST_LIGHT };
    v.faint_bg_color = if dark {
        Color32::from_rgb(38, 38, 38)
    } else {
        Color32::from_rgb(246, 246, 246)
    };
    v.window_stroke = Stroke::new(1.0, if dark { Color32::from_gray(70) } else { Color32::from_gray(160) });
    v.widgets.noninteractive.bg_fill = if dark { PANEL_DARK } else { PANEL_LIGHT };
    v.widgets.inactive.bg_fill = if dark {
        Color32::from_rgb(55, 55, 55)
    } else {
        Color32::from_rgb(236, 236, 236)
    };
    v.widgets.inactive.weak_bg_fill = v.widgets.inactive.bg_fill;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, if dark { Color32::from_gray(80) } else { Color32::from_gray(170) });
    v.widgets.hovered.bg_fill = if dark {
        Color32::from_rgb(70, 70, 70)
    } else {
        Color32::from_rgb(229, 241, 251)
    };
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, ODM_BLUE);
    v.widgets.active.bg_fill = if dark {
        Color32::from_rgb(0, 90, 170)
    } else {
        Color32::from_rgb(204, 228, 247)
    };
    v.selection.bg_fill = ODM_BLUE;
    v.selection.stroke = Stroke::new(1.0, Color32::WHITE);
    v.widgets.noninteractive.corner_radius = Rounding::same(2);
    v.widgets.inactive.corner_radius = Rounding::same(2);
    v.widgets.hovered.corner_radius = Rounding::same(2);
    v.widgets.active.corner_radius = Rounding::same(2);
    v.override_text_color = Some(if dark { Color32::from_gray(230) } else { Color32::from_gray(20) });
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
        Stroke::new(1.0, if dark { Color32::from_gray(80) } else { Color32::from_gray(130) }),
        egui::StrokeKind::Inside,
    );
    resp
}
