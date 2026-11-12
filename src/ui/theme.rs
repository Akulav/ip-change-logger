//! Colors, fonts and widget styling. The look follows Akulav's newer apps (Hyve, Lumen): a deep blue-black
//! backdrop with soft glows ([`paint_backdrop`]) and glass cards with hairline borders ([`GLASS`]).

use eframe::egui::{
    self,
    epaint::{Mesh, Vertex, WHITE_UV},
    Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Margin, Pos2, Rect, Stroke,
    TextStyle,
};
use std::sync::Arc;

pub const BG: Color32 = Color32::from_rgb(0x09, 0x0B, 0x14);
pub const SURFACE: Color32 = Color32::from_rgb(0x10, 0x13, 0x1F);
pub const SURFACE_HI: Color32 = Color32::from_rgb(0x17, 0x1B, 0x2A);
pub const BORDER: Color32 = Color32::from_rgb(0x2A, 0x2F, 0x45);
pub const TEXT: Color32 = Color32::from_rgb(0xE8, 0xEA, 0xF6);
pub const MUTED: Color32 = Color32::from_rgb(0x8F, 0x95, 0xB2);
pub const FAINT: Color32 = Color32::from_rgb(0x5D, 0x63, 0x80);
/// Glass: translucent white over the backdrop, so the glows show through cards.
pub const GLASS: Color32 = Color32::from_rgba_premultiplied(14, 14, 15, 14);
pub const GLASS_BORDER: Color32 = Color32::from_rgba_premultiplied(24, 24, 26, 24);

pub const CYAN: Color32 = Color32::from_rgb(0x22, 0xD3, 0xEE);
pub const INDIGO: Color32 = Color32::from_rgb(0x63, 0x66, 0xF1);
pub const GREEN: Color32 = Color32::from_rgb(0x34, 0xD3, 0x99);
pub const AMBER: Color32 = Color32::from_rgb(0xFB, 0xBF, 0x24);
pub const RED: Color32 = Color32::from_rgb(0xF8, 0x71, 0x71);

pub const RADIUS: u8 = 16;
/// Buttons and fields (Hyve's radius-sm).
pub const RADIUS_SM: u8 = 10;

/// Soft radial glow: `color` in the middle, fading to nothing at `radius`.
fn radial(mesh: &mut Mesh, center: Pos2, radius: egui::Vec2, color: Color32) {
    let base = mesh.vertices.len() as u32;
    mesh.vertices.push(Vertex { pos: center, uv: WHITE_UV, color });
    let n = 48;
    for i in 0..=n {
        let a = i as f32 / n as f32 * std::f32::consts::TAU;
        let pos = center + egui::vec2(a.cos() * radius.x, a.sin() * radius.y);
        mesh.vertices.push(Vertex { pos, uv: WHITE_UV, color: Color32::TRANSPARENT });
    }
    for i in 0..n {
        mesh.add_triangle(base, base + 1 + i, base + 2 + i);
    }
}

/// The window backdrop in `rect`: the base color with an indigo glow top-left, a cyan glow top-right and a faint
/// one below.
pub fn paint_backdrop(painter: &egui::Painter, rect: Rect) {
    painter.rect_filled(rect, CornerRadius::ZERO, BG);
    let (w, h) = (rect.width(), rect.height());
    let mut mesh = Mesh::default();
    radial(
        &mut mesh,
        rect.min + egui::vec2(w * 0.08, -h * 0.10),
        egui::vec2(w * 0.62, h * 0.85),
        tint(INDIGO, 0.18),
    );
    radial(
        &mut mesh,
        rect.min + egui::vec2(w * 1.05, h * 0.10),
        egui::vec2(w * 0.48, h * 0.75),
        tint(CYAN, 0.11),
    );
    radial(
        &mut mesh,
        rect.min + egui::vec2(w * 0.5, h * 1.2),
        egui::vec2(w * 0.45, h * 0.6),
        tint(INDIGO, 0.08),
    );
    painter.add(mesh);
}

/// Semi-bold UI font (falls back to the regular one when unavailable).
pub fn strong() -> FontFamily {
    FontFamily::Name("strong".into())
}

pub fn tint(c: Color32, alpha: f32) -> Color32 {
    c.gamma_multiply(alpha)
}

pub fn apply(ctx: &egui::Context) {
    ctx.set_fonts(fonts());
    ctx.set_theme(egui::Theme::Dark);
    ctx.global_style_mut(|style| {
        style.text_styles = [
            (TextStyle::Small, FontId::proportional(11.5)),
            (TextStyle::Body, FontId::proportional(14.0)),
            (TextStyle::Button, FontId::proportional(14.0)),
            (TextStyle::Heading, FontId::new(20.0, strong())),
            (TextStyle::Monospace, FontId::monospace(13.5)),
        ]
        .into();
        style.spacing.item_spacing = egui::vec2(10.0, 8.0);
        style.spacing.button_padding = egui::vec2(14.0, 7.0);
        style.spacing.interact_size.y = 32.0;
        style.spacing.window_margin = Margin::same(20);

        let v = &mut style.visuals;
        v.dark_mode = true;
        v.panel_fill = BG;
        v.window_fill = SURFACE;
        v.extreme_bg_color = Color32::from_rgb(0x07, 0x08, 0x0F); // text fields: darker than the glass around them
        v.faint_bg_color = SURFACE_HI;
        v.window_stroke = Stroke::new(1.0, BORDER);
        v.window_corner_radius = CornerRadius::same(RADIUS);
        v.menu_corner_radius = CornerRadius::same(10);
        v.override_text_color = Some(TEXT);
        v.hyperlink_color = CYAN;
        v.selection.bg_fill = tint(INDIGO, 0.55);
        v.selection.stroke = Stroke::new(1.0, CYAN);
        v.warn_fg_color = AMBER;
        v.error_fg_color = RED;

        // Hyve's buttons: translucent white with a light hairline, brighter on hover.
        let r = CornerRadius::same(RADIUS_SM);
        for (w, fill, stroke) in [
            (&mut v.widgets.noninteractive, SURFACE, BORDER),
            (
                &mut v.widgets.inactive,
                Color32::from_rgba_premultiplied(15, 15, 16, 15),
                Color32::from_rgba_premultiplied(40, 40, 42, 40),
            ),
            (
                &mut v.widgets.hovered,
                Color32::from_rgba_premultiplied(28, 28, 30, 28),
                Color32::from_rgba_premultiplied(62, 62, 66, 62),
            ),
            (&mut v.widgets.active, tint(INDIGO, 0.35), INDIGO),
            (&mut v.widgets.open, SURFACE_HI, BORDER),
        ] {
            w.bg_fill = fill;
            w.weak_bg_fill = fill;
            w.bg_stroke = Stroke::new(1.0, stroke);
            w.corner_radius = r;
        }
        v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, MUTED);
        v.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT);
        v.widgets.hovered.fg_stroke = Stroke::new(1.5, Color32::WHITE);
        v.widgets.active.fg_stroke = Stroke::new(1.5, Color32::WHITE);
    });
}

/// Uses the platform's UI font when one is installed, keeping egui's bundled
/// fonts (and their emoji/symbol coverage) as fallbacks.
fn fonts() -> FontDefinitions {
    let mut defs = FontDefinitions::default();

    let regular = first_existing(REGULAR_CANDIDATES);
    let strong = first_existing(STRONG_CANDIDATES);
    let mono = first_existing(MONO_CANDIDATES);

    let mut prepend = |family: FontFamily, name: &str, bytes: Option<Vec<u8>>| {
        if let Some(bytes) = bytes {
            defs.font_data.insert(name.to_owned(), Arc::new(FontData::from_owned(bytes)));
            defs.families.entry(family).or_default().insert(0, name.to_owned());
        }
    };

    prepend(FontFamily::Proportional, "ui-regular", regular);
    prepend(FontFamily::Monospace, "ui-mono", mono);

    // "strong" = semibold UI font → regular UI font → egui defaults.
    let mut strong_chain = defs.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    if let Some(bytes) = strong {
        defs.font_data.insert("ui-strong".into(), Arc::new(FontData::from_owned(bytes)));
        strong_chain.insert(0, "ui-strong".into());
    }
    defs.families.insert(self::strong(), strong_chain);
    defs
}

fn first_existing(candidates: &[&str]) -> Option<Vec<u8>> {
    candidates.iter().find_map(|p| std::fs::read(p).ok())
}

#[cfg(windows)]
const REGULAR_CANDIDATES: &[&str] = &[r"C:\Windows\Fonts\segoeui.ttf"];
#[cfg(windows)]
const STRONG_CANDIDATES: &[&str] = &[r"C:\Windows\Fonts\seguisb.ttf", r"C:\Windows\Fonts\segoeuib.ttf"];
#[cfg(windows)]
const MONO_CANDIDATES: &[&str] = &[r"C:\Windows\Fonts\CascadiaMono.ttf", r"C:\Windows\Fonts\consola.ttf"];

#[cfg(not(windows))]
const REGULAR_CANDIDATES: &[&str] = &[
    "/usr/share/fonts/truetype/inter/Inter-Regular.ttf",
    "/usr/share/fonts/inter/Inter-Regular.ttf",
    "/usr/share/fonts/opentype/inter/Inter-Regular.otf",
    "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
    "/usr/share/fonts/noto/NotoSans-Regular.ttf",
    "/usr/share/fonts/google-noto/NotoSans-Regular.ttf",
    "/usr/share/fonts/truetype/cantarell/Cantarell-Regular.ttf",
];
#[cfg(not(windows))]
const STRONG_CANDIDATES: &[&str] = &[
    "/usr/share/fonts/truetype/inter/Inter-SemiBold.ttf",
    "/usr/share/fonts/inter/Inter-SemiBold.ttf",
    "/usr/share/fonts/opentype/inter/Inter-SemiBold.otf",
    "/usr/share/fonts/truetype/noto/NotoSans-SemiBold.ttf",
    "/usr/share/fonts/noto/NotoSans-SemiBold.ttf",
    "/usr/share/fonts/google-noto/NotoSans-SemiBold.ttf",
    "/usr/share/fonts/truetype/noto/NotoSans-Bold.ttf",
];
#[cfg(not(windows))]
const MONO_CANDIDATES: &[&str] = &[
    "/usr/share/fonts/truetype/jetbrains-mono/JetBrainsMono-Regular.ttf",
    "/usr/share/fonts/truetype/noto/NotoSansMono-Regular.ttf",
    "/usr/share/fonts/noto/NotoSansMono-Regular.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
];
