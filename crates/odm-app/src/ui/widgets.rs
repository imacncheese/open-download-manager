//! Reusable widgets with smooth hover/press transitions and animated bars.

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
    let height = if large { 54.0 } else { 28.0 };
    let width: f32 = if large { 62.0 } else { 0.0 };
    let text = egui::WidgetText::from(egui::RichText::new(label).size(11.0));
    let galley = text.into_galley(
        ui,
        Some(egui::TextWrapMode::Truncate),
        width.max(40.0),
        egui::TextStyle::Button,
    );
    let w = if large {
        width.max(galley.size().x + 10.0).max(46.0)
    } else {
        galley.size().x + 32.0
    };
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(w, height), Sense::click());
    let hovered = enabled && resp.hovered();
    let pressed = enabled && resp.is_pointer_button_down_on();

    let hover_t = ui.ctx().animate_bool_with_time(
        egui::Id::new(("toolbar-hover", label, large)),
        hovered && !pressed,
        0.12,
    );
    let press_t = ui.ctx().animate_bool_with_time(
        egui::Id::new(("toolbar-press", label, large)),
        pressed,
        0.07,
    );
    if hover_t > 0.0 && hover_t < 1.0 || press_t > 0.0 && press_t < 1.0 {
        ui.ctx().request_repaint();
    }

    let hover_fill = if dark {
        Color32::from_rgb(63, 66, 72)
    } else {
        Color32::from_rgb(226, 238, 250)
    };
    let press_fill = if dark {
        Color32::from_rgb(0, 84, 158)
    } else {
        Color32::from_rgb(204, 228, 247)
    };
    let mut bg = theme::mix(Color32::TRANSPARENT, hover_fill, hover_t);
    bg = theme::mix(bg, press_fill, press_t);

    if bg.a() > 0 {
        ui.painter()
            .rect_filled(rect.shrink(1.0), egui::Rounding::same(5), bg);
        let border_a = ((hover_t.max(press_t) * 170.0) as u8).max(0);
        if border_a > 0 {
            ui.painter().rect_stroke(
                rect.shrink(1.0),
                egui::Rounding::same(5),
                Stroke::new(
                    1.0,
                    theme::ACCENT.gamma_multiply(border_a as f32 / 255.0),
                ),
                egui::StrokeKind::Inside,
            );
        }
    }

    let color = if enabled {
        if pressed {
            theme::ACCENT
        } else {
            theme::text(dark)
        }
    } else {
        if dark {
            Color32::from_gray(105)
        } else {
            Color32::from_gray(165)
        }
    };

    let lift = if pressed { 1.0 } else { 0.0 };
    if large {
        let icon_rect = Rect::from_center_size(
            Pos2::new(rect.center().x, rect.top() + 20.0 + lift),
            Vec2::splat(26.0),
        );
        icons::draw(ui.painter(), icon_rect, icon, color);
        ui.painter().galley(
            Pos2::new(
                rect.center().x - galley.size().x / 2.0,
                rect.bottom() - galley.size().y - 3.0 + lift,
            ),
            galley,
            color,
        );
    } else {
        let icon_rect = Rect::from_center_size(
            Pos2::new(rect.left() + 15.0, rect.center().y + lift),
            Vec2::splat(16.0),
        );
        icons::draw(ui.painter(), icon_rect, icon, color);
        ui.painter().galley(
            Pos2::new(rect.left() + 28.0, rect.center().y - galley.size().y / 2.0 + lift),
            galley,
            color,
        );
    }

    if enabled {
        resp.on_hover_text(label)
    } else {
        resp.on_disabled_hover_text(label)
    }
}

/// Animated, full-width progress bar.
pub fn progress_bar_animated(
    ui: &mut Ui,
    id: impl std::hash::Hash,
    fraction: f32,
    height: f32,
    show_text: bool,
    dark: bool,
) -> Response {
    let width = ui.available_width().max(60.0);
    progress_bar_sized(ui, id, fraction, width, height, show_text, dark)
}

/// Animated progress bar with an explicit width (used for per-connection rows).
pub fn progress_bar_sized(
    ui: &mut Ui,
    id: impl std::hash::Hash,
    fraction: f32,
    width: f32,
    height: f32,
    show_text: bool,
    dark: bool,
) -> Response {
    let target = fraction.clamp(0.0, 1.0);
    let anim = ui
        .ctx()
        .animate_value_with_time(egui::Id::new(id), target, 0.30)
        .clamp(0.0, 1.0);
    if (anim - target).abs() > 0.001 {
        ui.ctx().request_repaint();
    }
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());

    let track = if dark {
        Color32::from_rgb(55, 57, 62)
    } else {
        Color32::from_rgb(226, 228, 233)
    };
    let p = ui.painter();
    p.rect_filled(rect, egui::Rounding::same(4), track);
    p.rect_stroke(
        rect,
        egui::Rounding::same(4),
        Stroke::new(1.0, theme::border(dark)),
        egui::StrokeKind::Inside,
    );

    if anim > 0.0 {
        let mut fill = rect.shrink(1.0);
        fill.set_width(((rect.width() - 2.0) * anim).max(2.0));
        p.rect_filled(fill, egui::Rounding::same(3), theme::ACCENT);
        // Soft top highlight for a glossy, modern fill.
        let mut hi = fill;
        hi.set_height((fill.height() * 0.42).max(1.0));
        p.rect_filled(
            hi,
            egui::Rounding::same(3),
            Color32::from_rgba_unmultiplied(255, 255, 255, 34),
        );
        // Brighter leading edge.
        if fill.width() > 4.0 {
            let mut edge = fill;
            edge.set_left(edge.right() - 3.0);
            p.rect_filled(edge, egui::Rounding::same(2), Color32::from_white_alpha(70));
        }
    }

    if show_text {
        let color = if anim > 0.55 {
            Color32::WHITE
        } else {
            theme::muted(dark)
        };
        p.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            format!("{:.0}%", anim * 100.0),
            egui::FontId::proportional(11.5),
            color,
        );
    }
    resp
}

/// Compact animated bar without labels for connection rows.
pub fn mini_progress_bar(
    ui: &mut Ui,
    id: impl std::hash::Hash,
    fraction: f32,
    width: f32,
    dark: bool,
) -> Response {
    progress_bar_sized(ui, id, fraction, width, 12.0, false, dark)
}

pub fn group_title(ui: &mut Ui, text: &str) {
    ui.add_space(3.0);
    ui.label(
        egui::RichText::new(text)
            .strong()
            .size(12.0)
            .color(theme::muted(ui.visuals().dark_mode)),
    );
}

pub fn kv(ui: &mut Ui, key: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.add_sized(
            [110.0, 18.0],
            egui::Label::new(
                egui::RichText::new(key).color(theme::muted(ui.visuals().dark_mode)),
            ),
        );
        ui.label(value);
    });
}

pub fn section(ui: &mut Ui, dark: bool, add: impl FnOnce(&mut Ui)) {
    let frame = egui::Frame::none()
        .fill(theme::surface(dark))
        .stroke(Stroke::new(1.0, theme::border(dark)))
        .inner_margin(egui::Margin::same(10))
        .rounding(egui::Rounding::same(6));
    frame.show(ui, |ui| add(ui));
}

/// Right-aligned OK/Cancel row used by dialogs.
pub fn dialog_buttons(ui: &mut Ui, ok: &str, cancel: &str) -> (bool, bool) {
    let mut ok_clicked = false;
    let mut cancel_clicked = false;
    ui.add_space(4.0);
    ui.separator();
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add(egui::Button::new(cancel).min_size(egui::vec2(86.0, 26.0)))
                .clicked()
            {
                cancel_clicked = true;
            }
            if ui
                .add(
                    egui::Button::new(egui::RichText::new(ok).strong())
                        .min_size(egui::vec2(86.0, 26.0)),
                )
                .clicked()
            {
                ok_clicked = true;
            }
        });
    });
    (ok_clicked, cancel_clicked)
}
