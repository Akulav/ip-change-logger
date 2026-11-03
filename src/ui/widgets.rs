//! Small reusable pieces of the IPTrail look.

use super::theme::{self, BORDER, FAINT, MUTED, SURFACE, TEXT};
use eframe::egui::{
    self, Color32, CornerRadius, FontId, Frame, Margin, Response, RichText, Sense, Stroke, Ui,
};

/// Rounded surface used for every section.
pub fn card<R>(ui: &mut Ui, add: impl FnOnce(&mut Ui) -> R) -> R {
    Frame::new()
        .fill(SURFACE)
        .stroke(Stroke::new(1.0, BORDER))
        .corner_radius(CornerRadius::same(theme::RADIUS))
        .inner_margin(Margin::same(18))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui)
        })
        .inner
}

/// Small upper-case caption above a value.
pub fn caption(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text.to_uppercase()).size(11.0).color(FAINT).extra_letter_spacing(0.8));
}

pub fn section_title(ui: &mut Ui, icon: &str, text: &str) {
    ui.horizontal(|ui| {
        ui.label(RichText::new(icon).size(15.0).color(theme::CYAN));
        ui.label(RichText::new(text).font(FontId::new(15.5, theme::strong())).color(TEXT));
    });
}

/// Rounded pill with a colored dot and label. Painted directly so it keeps
/// its compact size and reading order inside right-to-left layouts.
pub fn pill(ui: &mut Ui, text: &str, color: Color32) -> Response {
    let galley = ui.painter().layout_no_wrap(text.to_owned(), FontId::proportional(12.5), color);
    let pad = egui::vec2(10.0, 4.0);
    let dot = 7.0;
    let gap = 6.0;
    let size = egui::vec2(pad.x * 2.0 + dot + gap + galley.size().x, pad.y * 2.0 + galley.size().y);
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    if ui.is_rect_visible(rect) {
        let p = ui.painter();
        p.rect(
            rect,
            CornerRadius::same(255),
            theme::tint(color, 0.14),
            Stroke::new(1.0, theme::tint(color, 0.45)),
            egui::StrokeKind::Inside,
        );
        p.circle_filled(egui::pos2(rect.left() + pad.x + dot / 2.0, rect.center().y), dot / 2.0, color);
        let text_pos = egui::pos2(rect.left() + pad.x + dot + gap, rect.center().y - galley.size().y / 2.0);
        p.galley(text_pos, galley, color);
    }
    response
}

/// Text-only chip for compact flags in lists.
pub fn chip(ui: &mut Ui, text: &str, color: Color32) {
    let galley = ui.painter().layout_no_wrap(text.to_owned(), FontId::proportional(11.0), color);
    let pad = egui::vec2(6.0, 2.0);
    let (rect, _) = ui.allocate_exact_size(galley.size() + pad * 2.0, Sense::hover());
    if ui.is_rect_visible(rect) {
        ui.painter().rect_filled(rect, CornerRadius::same(6), theme::tint(color, 0.13));
        ui.painter().galley(rect.min + pad, galley, color);
    }
}

/// Circular status glyph drawn with vector strokes (no emoji fonts involved).
#[derive(Clone, Copy)]
pub enum Glyph {
    Check,
    Shield,
    Alert,
    Question,
}

pub fn glyph(ui: &mut Ui, kind: Glyph, color: Color32, diameter: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(diameter, diameter), Sense::hover());
    let p = ui.painter();
    let c = rect.center();
    let r = diameter / 2.0;
    p.circle(c, r, theme::tint(color, 0.18), Stroke::new(1.5, theme::tint(color, 0.6)));
    let s = Stroke::new(diameter * 0.085, color);
    let at = |x: f32, y: f32| c + egui::vec2(x * r, y * r);
    match kind {
        Glyph::Check => {
            p.line(vec![at(-0.42, 0.02), at(-0.12, 0.32), at(0.44, -0.30)], s);
        }
        Glyph::Shield => {
            let pts = vec![
                at(0.0, -0.50),
                at(0.40, -0.34),
                at(0.36, 0.08),
                at(0.0, 0.50),
                at(-0.36, 0.08),
                at(-0.40, -0.34),
                at(0.0, -0.50),
            ];
            p.line(pts, s);
        }
        Glyph::Alert => {
            p.line_segment([at(0.0, -0.42), at(0.0, 0.12)], s);
            p.circle_filled(at(0.0, 0.38), s.width * 0.7, color);
        }
        Glyph::Question => {
            let q =
                ui.painter().layout_no_wrap("?".into(), FontId::new(diameter * 0.6, theme::strong()), color);
            let pos = c - q.size() / 2.0;
            ui.painter().galley(pos, q, color);
        }
    }
}

/// Filled accent button.
pub fn primary_button(ui: &mut Ui, text: &str, color: Color32) -> Response {
    let button = egui::Button::new(
        RichText::new(text)
            .color(Color32::from_rgb(0x07, 0x0B, 0x14))
            .font(FontId::new(14.0, theme::strong())),
    )
    .fill(color)
    .stroke(Stroke::NONE)
    .corner_radius(CornerRadius::same(9));
    ui.add(button)
}

/// Quiet button with an icon and label.
pub fn ghost_button(ui: &mut Ui, text: &str) -> Response {
    ui.add(egui::Button::new(RichText::new(text).color(TEXT)).corner_radius(CornerRadius::same(9)))
}

/// Label/value row used in the details grid.
pub fn kv_row(ui: &mut Ui, key: &str, value: Option<&str>) {
    ui.label(RichText::new(key).color(MUTED));
    match value {
        Some(v) => ui.add(egui::Label::new(RichText::new(v).color(TEXT)).truncate()),
        None => ui.label(RichText::new("—").color(FAINT)),
    };
    ui.end_row();
}

/// A labelled yes/no/unknown signal. `bad_when_true` picks the color for "Yes".
pub fn signal_row(ui: &mut Ui, label: &str, hint: &str, value: Option<bool>, yes_color: Color32) {
    ui.label(RichText::new(label).color(TEXT)).on_hover_text(hint);
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| match value {
        Some(true) => {
            pill(ui, "Yes", yes_color);
        }
        Some(false) => {
            pill(ui, "No", theme::GREEN);
        }
        None => {
            pill(ui, "Unknown", FAINT);
        }
    });
    ui.end_row();
}

/// Compact stat tile: caption, big value and a secondary line.
pub fn stat(ui: &mut Ui, label: &str, value: &str, sub: &str, mono: bool) {
    card(ui, |ui| {
        caption(ui, label);
        ui.add_space(2.0);
        let font = if mono { FontId::monospace(19.0) } else { FontId::new(19.0, theme::strong()) };
        ui.add(egui::Label::new(RichText::new(value).font(font).color(TEXT)).truncate());
        ui.add(egui::Label::new(RichText::new(sub).size(12.0).color(MUTED)).truncate());
    });
}
