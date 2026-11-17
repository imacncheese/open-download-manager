//! Small vector icons drawn with the egui painter, matching IDM's toolbar
//! pictograms (green download arrow, red stop square, gear, clock...).

use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    AddUrl,
    Resume,
    Stop,
    StopAll,
    Delete,
    Options,
    Scheduler,
    Grabber,
    QueueStart,
    QueueStop,
    Speed,
    Find,
    Folder,
    File,
    Music,
    Video,
    Program,
    Doc,
    Archive,
    Game,
    Globe,
}

pub fn draw(painter: &egui::Painter, rect: Rect, icon: Icon, color: Color32) {
    let c = rect.center();
    let s = rect.width().min(rect.height());
    let stroke = Stroke::new((s * 0.075).max(1.4), color);
    let green = Color32::from_rgb(0, 150, 0);
    let red = Color32::from_rgb(200, 30, 30);
    let blue = Color32::from_rgb(0, 110, 200);
    let amber = Color32::from_rgb(220, 150, 0);
    match icon {
        Icon::AddUrl => {
            // Green down arrow into a tray.
            painter.arrow(
                Pos2::new(c.x, rect.top() + s * 0.12),
                Vec2::new(0.0, s * 0.5),
                Stroke::new(s * 0.1, green),
            );
            painter.line_segment(
                [
                    Pos2::new(c.x - s * 0.17, rect.top() + s * 0.47),
                    Pos2::new(c.x, rect.top() + s * 0.66),
                ],
                Stroke::new(s * 0.09, green),
            );
            painter.line_segment(
                [
                    Pos2::new(c.x + s * 0.17, rect.top() + s * 0.47),
                    Pos2::new(c.x, rect.top() + s * 0.66),
                ],
                Stroke::new(s * 0.09, green),
            );
            painter.rect_stroke(
                Rect::from_min_max(
                    Pos2::new(rect.left() + s * 0.14, rect.top() + s * 0.74),
                    Pos2::new(rect.right() - s * 0.14, rect.bottom() - s * 0.12),
                ),
                egui::Rounding::same(1),
                Stroke::new(s * 0.07, green),
                egui::StrokeKind::Middle,
            );
        }
        Icon::Resume => {
            painter.add(egui::Shape::convex_polygon(
                vec![
                    Pos2::new(rect.left() + s * 0.26, rect.top() + s * 0.14),
                    Pos2::new(rect.right() - s * 0.16, c.y),
                    Pos2::new(rect.left() + s * 0.26, rect.bottom() - s * 0.14),
                ],
                green,
                Stroke::NONE,
            ));
        }
        Icon::Stop => {
            painter.rect_filled(
                Rect::from_center_size(c, Vec2::splat(s * 0.62)),
                egui::Rounding::same(2),
                red,
            );
        }
        Icon::StopAll => {
            let r = Rect::from_center_size(c, Vec2::splat(s * 0.62));
            painter.rect_filled(r, egui::Rounding::same(2), red);
            painter.line_segment(
                [
                    Pos2::new(r.left() + s * 0.1, r.top() + s * 0.1),
                    Pos2::new(r.right() - s * 0.1, r.bottom() - s * 0.1),
                ],
                Stroke::new(s * 0.06, Color32::WHITE),
            );
            painter.line_segment(
                [
                    Pos2::new(r.left() + s * 0.1, r.bottom() - s * 0.1),
                    Pos2::new(r.right() - s * 0.1, r.top() + s * 0.1),
                ],
                Stroke::new(s * 0.06, Color32::WHITE),
            );
        }
        Icon::Delete => {
            // Red X over a page.
            painter.rect_stroke(
                Rect::from_center_size(
                    Pos2::new(c.x, c.y + s * 0.06),
                    Vec2::new(s * 0.5, s * 0.6),
                ),
                egui::Rounding::same(1),
                Stroke::new(s * 0.055, color),
                egui::StrokeKind::Middle,
            );
            let d = s * 0.24;
            painter.line_segment(
                [
                    Pos2::new(c.x - d, c.y - d),
                    Pos2::new(c.x + d, c.y + d),
                ],
                Stroke::new(s * 0.09, red),
            );
            painter.line_segment(
                [
                    Pos2::new(c.x + d, c.y - d),
                    Pos2::new(c.x - d, c.y + d),
                ],
                Stroke::new(s * 0.09, red),
            );
        }
        Icon::Options => {
            let r = s * 0.30;
            for i in 0..6 {
                let a = std::f32::consts::FRAC_PI_3 * i as f32;
                let p1 = c + Vec2::new(a.cos(), a.sin()) * r;
                let p2 = c + Vec2::new(a.cos(), a.sin()) * (r + s * 0.12);
                painter.line_segment([p1, p2], Stroke::new(s * 0.11, color));
            }
            painter.circle_stroke(c, r * 0.62, Stroke::new(s * 0.11, color));
            painter.circle_filled(c, r * 0.22, color);
        }
        Icon::Scheduler => {
            painter.circle_stroke(c, s * 0.32, Stroke::new(s * 0.075, color));
            painter.line_segment(
                [c, c + Vec2::new(0.0, -s * 0.22)],
                Stroke::new(s * 0.06, blue),
            );
            painter.line_segment(
                [c, c + Vec2::new(s * 0.16, s * 0.06)],
                Stroke::new(s * 0.06, blue),
            );
        }
        Icon::Grabber => {
            painter.circle_stroke(c, s * 0.32, Stroke::new(s * 0.06, blue));
            painter.line_segment(
                [c - Vec2::new(s * 0.32, 0.0), c + Vec2::new(s * 0.32, 0.0)],
                Stroke::new(s * 0.05, blue),
            );
            painter.line_segment(
                [c - Vec2::new(0.0, s * 0.32), c + Vec2::new(0.0, s * 0.32)],
                Stroke::new(s * 0.05, blue),
            );
            painter.circle_stroke(c, s * 0.32, Stroke::new(s * 0.04, blue));
        }
        Icon::QueueStart => {
            painter.add(egui::Shape::convex_polygon(
                vec![
                    Pos2::new(rect.left() + s * 0.2, rect.top() + s * 0.18),
                    Pos2::new(rect.left() + s * 0.55, c.y),
                    Pos2::new(rect.left() + s * 0.2, rect.bottom() - s * 0.18),
                ],
                green,
                Stroke::NONE,
            ));
            for i in 0..3 {
                let y = rect.top() + s * (0.28 + i as f32 * 0.2);
                painter.line_segment(
                    [Pos2::new(rect.right() - s * 0.34, y), Pos2::new(rect.right() - s * 0.1, y)],
                    Stroke::new(s * 0.06, color),
                );
            }
        }
        Icon::QueueStop => {
            painter.rect_filled(
                Rect::from_min_size(
                    Pos2::new(rect.left() + s * 0.16, rect.top() + s * 0.22),
                    Vec2::new(s * 0.34, s * 0.56),
                ),
                egui::Rounding::same(1),
                red,
            );
            for i in 0..3 {
                let y = rect.top() + s * (0.28 + i as f32 * 0.2);
                painter.line_segment(
                    [Pos2::new(rect.right() - s * 0.34, y), Pos2::new(rect.right() - s * 0.1, y)],
                    Stroke::new(s * 0.06, color),
                );
            }
        }
        Icon::Speed => {
            painter.circle_stroke(c, s * 0.3, Stroke::new(s * 0.07, color));
            painter.line_segment(
                [c, c + Vec2::new(s * 0.18, -s * 0.18)],
                Stroke::new(s * 0.07, red),
            );
            painter.circle_filled(c, s * 0.06, color);
        }
        Icon::Find => {
            painter.circle_stroke(c - Vec2::new(s * 0.06, s * 0.06), s * 0.2, Stroke::new(s * 0.08, color));
            painter.line_segment(
                [c + Vec2::new(s * 0.1, s * 0.1), c + Vec2::new(s * 0.3, s * 0.3)],
                Stroke::new(s * 0.1, color),
            );
        }
        Icon::Folder => {
            let r = Rect::from_center_size(c, Vec2::new(s * 0.62, s * 0.46));
            painter.rect_filled(r, egui::Rounding::same(2), amber);
            painter.rect_filled(
                Rect::from_min_size(
                    Pos2::new(r.left(), r.top() - s * 0.1),
                    Vec2::new(s * 0.28, s * 0.14),
                ),
                egui::Rounding::same(2),
                amber,
            );
        }
        Icon::File => {
            painter.rect_filled(
                Rect::from_center_size(c, Vec2::new(s * 0.46, s * 0.58)),
                egui::Rounding::same(1),
                Color32::from_gray(220),
            );
            painter.rect_stroke(
                Rect::from_center_size(c, Vec2::new(s * 0.46, s * 0.58)),
                egui::Rounding::same(1),
                Stroke::new(1.0, Color32::from_gray(120)),
                egui::StrokeKind::Middle,
            );
        }
        Icon::Music => {
            painter.circle_filled(c + Vec2::new(-s * 0.12, s * 0.18), s * 0.1, blue);
            painter.line_segment(
                [
                    c + Vec2::new(-s * 0.03, s * 0.16),
                    c + Vec2::new(-s * 0.03, -s * 0.26),
                ],
                Stroke::new(s * 0.06, blue),
            );
            painter.line_segment(
                [
                    c + Vec2::new(-s * 0.03, -s * 0.26),
                    c + Vec2::new(s * 0.18, -s * 0.34),
                ],
                Stroke::new(s * 0.06, blue),
            );
        }
        Icon::Video => {
            painter.rect_filled(
                Rect::from_center_size(c, Vec2::new(s * 0.58, s * 0.4)),
                egui::Rounding::same(1),
                Color32::from_gray(60),
            );
            painter.add(egui::Shape::convex_polygon(
                vec![
                    c + Vec2::new(-s * 0.12, -s * 0.12),
                    c + Vec2::new(s * 0.14, 0.0),
                    c + Vec2::new(-s * 0.12, s * 0.12),
                ],
                Color32::WHITE,
                Stroke::NONE,
            ));
        }
        Icon::Program => {
            painter.rect_filled(
                Rect::from_center_size(c, Vec2::splat(s * 0.5)),
                egui::Rounding::same(2),
                Color32::from_rgb(60, 120, 200),
            );
            painter.text(
                c,
                egui::Align2::CENTER_CENTER,
                "EXE",
                egui::FontId::proportional(s * 0.22),
                Color32::WHITE,
            );
        }
        Icon::Doc => {
            painter.rect_filled(
                Rect::from_center_size(c, Vec2::new(s * 0.44, s * 0.58)),
                egui::Rounding::same(1),
                Color32::WHITE,
            );
            for i in 0..4 {
                let y = rust_top(c.y) + s * (0.16 + i as f32 * 0.1);
                painter.line_segment(
                    [Pos2::new(c.x - s * 0.15, y), Pos2::new(c.x + s * 0.15, y)],
                    Stroke::new(1.0, Color32::from_gray(120)),
                );
            }
        }
        Icon::Archive => {
            painter.rect_filled(
                Rect::from_center_size(c, Vec2::new(s * 0.5, s * 0.56)),
                egui::Rounding::same(1),
                Color32::from_rgb(190, 150, 60),
            );
            painter.line_segment(
                [c + Vec2::new(0.0, -s * 0.26), c + Vec2::new(0.0, s * 0.08)],
                Stroke::new(s * 0.08, Color32::from_gray(60)),
            );
        }
        Icon::Game => {
            painter.circle_filled(c, s * 0.3, Color32::from_rgb(120, 60, 180));
            painter.circle_filled(c + Vec2::new(-s * 0.1, 0.0), s * 0.06, Color32::WHITE);
            painter.circle_filled(c + Vec2::new(s * 0.1, 0.0), s * 0.06, Color32::WHITE);
        }
        Icon::Globe => {
            painter.circle_stroke(c, s * 0.3, Stroke::new(s * 0.06, blue));
            painter.line_segment(
                [c - Vec2::new(s * 0.3, 0.0), c + Vec2::new(s * 0.3, 0.0)],
                Stroke::new(s * 0.05, blue),
            );
            painter.circle_stroke(c, s * 0.3, Stroke::new(s * 0.04, blue));
        }
    }
}

fn rust_top(y: f32) -> f32 {
    y - 12.0
}

/// Map a file extension to the folder icon used in the list.
pub fn icon_for_file(name: &str) -> Icon {
    let ext = std::path::Path::new(name)
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "mp3" | "wav" | "wma" | "aac" | "m4a" | "ogg" | "flac" => Icon::Music,
        "mp4" | "mkv" | "avi" | "mov" | "wmv" | "flv" | "webm" | "mpg" | "mpeg" => Icon::Video,
        "exe" | "msi" | "msu" => Icon::Program,
        "zip" | "rar" | "7z" | "gz" | "bz2" | "tar" | "iso" => Icon::Archive,
        "pdf" | "doc" | "docx" | "ppt" | "pptx" | "txt" => Icon::Doc,
        _ => Icon::File,
    }
}
