//! Reusable Win32-flavoured widgets.

use super::icons::{self, Icon};
use super::theme;
use eframe::egui::{self, Color32, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2};

pub fn toolbar_button(
    ui: &mut Ui,
    icon: Icon,
    label: &str,
    enabled: bool,
    large: bool,
    dark: bool,
) -> Response {
    let height = if large { 52.0 } else { 26.0 };
    let width: f32 = if large { 62.0 } else { 0.0 };
    let text = egui::WidgetText::from(egui::RichText::new(label).size(if large { 11.0 } else { 11.0 }));
    let galley = text.into_galley(ui, Some(egui::TextWrapMode::Truncate), width.max(40.0), egui::TextStyle::Button);
    let w = if large {
        width.max(galley.size().x + 8.0).max(44.0)
    } else {
        galley.size().x + 30.0
    };
    let (rect, mut resp) = ui.allocate_exact_size(Vec2::new(w, height), Sense::click());
    if !enabled {
        resp = resp.on_disabled_hover_text(label);
    }
    let hovered = enabled && resp.hovered();
    let pressed = enabled && resp.is_pointer_button_down_on();
    let bg = if pressed {
        if dark {
            Color32::from_rgb(0, 90, 170)
        } else {
            Color32::from_rgb(204, 228, 247)
        }
    } else if hovered {
        if dark {
            Color32::from_rgb(62, 62, 66)
        } else {
            Color32::from_rgb(229, 241, 251)
        }
    } else {
        Color32::TRANSPARENT
    };
    if bg != Color32::TRANSPARENT {
        ui.painter().rect_filled(rect.shrink(1.0), egui::Rounding::same(3), bg);
        ui.painter().rect_stroke(
            rect.shrink(1.0),
            egui::Rounding::same(3),
            Stroke::new(1.0, if dark { Color32::from_gray(90) } else { Color32::from_gray(170) }),
            egui::StrokeKind::Middle,
        );
    }
    let color = if enabled {
        if dark {
            Color32::from_gray(225)
        } else {
            Color32::from_gray(40)
        }
    } else {
        Color32::from_gray(150)
    };
    if large {
        let icon_rect = Rect::from_center_size(
            Pos2::new(rect.center().x, rect.top() + 20.0),
            Vec2::splat(26.0),
        );
        icons::draw(ui.painter(), icon_rect, icon, color);
        ui.painter().galley(
            Pos2::new(rect.center().x - galley.size().x / 2.0, rect.bottom() - galley.size().y - 3.0),
            galley,
            color,
        );
    } else {
        let icon_rect = Rect::from_center_size(
            Pos2::new(rect.left() + 14.0, rect.center().y),
            Vec2::splat(16.0),
        );
        icons::draw(ui.painter(), icon_rect, icon, color);
        ui.painter().galley(
            Pos2::new(rect.left() + 26.0, rect.center().y - galley.size().y / 2.0),
            galley,
            color,
        );
    }
    resp
}

pub fn progress_bar(ui: &mut Ui, fraction: f32, height: f32, show_text: bool) -> Response {
    let width = ui.available_width().max(60.0);
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, egui::Rounding::same(2), Color32::from_gray(232));
    p.rect_stroke(
        rect,
        egui::Rounding::same(2),
        Stroke::new(1.0, Color32::from_gray(150)),
        egui::StrokeKind::Middle,
    );
    let frac = fraction.clamp(0.0, 1.0);
    if frac > 0.0 {
        let mut fill = rect.shrink(1.0);
        fill.set_width((rect.width() - 2.0) * frac);
        p.rect_filled(fill, egui::Rounding::same(1), theme::ODM_BLUE);
        // subtle highlight stripe like the classic Win32 progress bar
        let mut hi = fill;
        hi.set_height((fill.height() * 0.45).max(1.0));
        p.rect_filled(hi, egui::Rounding::ZERO, Color32::from_rgba_unmultiplied(255, 255, 255, 40));
    }
    if show_text {
        p.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            format!("{:.0}%", frac * 100.0),
            egui::FontId::proportional(11.0),
            Color32::from_gray(30),
        );
    }
    resp
}

pub fn group_title(ui: &mut Ui, text: &str) {
    ui.add_space(2.0);
    ui.label(egui::RichText::new(text).strong());
}

pub fn kv(ui: &mut Ui, key: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.add_sized(
            [110.0, 18.0],
            egui::Label::new(egui::RichText::new(key).color(Color32::from_gray(90))),
        );
        ui.label(value);
    });
}

pub fn section(ui: &mut Ui, dark: bool, add: impl FnOnce(&mut Ui)) {
    let frame = egui::Frame::none()
        .fill(if dark { Color32::from_rgb(40, 40, 40) } else { Color32::from_rgb(250, 250, 250) })
        .stroke(Stroke::new(1.0, if dark { Color32::from_gray(75) } else { Color32::from_gray(190) }))
        .inner_margin(egui::Margin::same(8))
        .rounding(egui::Rounding::same(2));
    frame.show(ui, |ui| add(ui));
}
