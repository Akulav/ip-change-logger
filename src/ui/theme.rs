//! Colors, fonts and widget styling.

use eframe::egui::{
    self, Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Margin, Stroke, TextStyle,
};
use std::sync::Arc;

pub const BG: Color32 = Color32::from_rgb(0x0B, 0x10, 0x1C);
pub const SURFACE: Color32 = Color32::from_rgb(0x12, 0x18, 0x27);
pub const SURFACE_HI: Color32 = Color32::from_rgb(0x19, 0x21, 0x33);
pub const BORDER: Color32 = Color32::from_rgb(0x23, 0x2D, 0x42);
pub const TEXT: Color32 = Color32::from_rgb(0xE6, 0xEA, 0xF2);
pub const MUTED: Color32 = Color32::from_rgb(0x94, 0xA0, 0xB8);
pub const FAINT: Color32 = Color32::from_rgb(0x5F, 0x6B, 0x82);

pub const CYAN: Color32 = Color32::from_rgb(0x22, 0xD3, 0xEE);
pub const INDIGO: Color32 = Color32::from_rgb(0x63, 0x66, 0xF1);
pub const GREEN: Color32 = Color32::from_rgb(0x34, 0xD3, 0x99);
pub const AMBER: Color32 = Color32::from_rgb(0xFB, 0xBF, 0x24);
pub const RED: Color32 = Color32::from_rgb(0xF8, 0x71, 0x71);

pub const RADIUS: u8 = 14;

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
        v.extreme_bg_color = Color32::from_rgb(0x0E, 0x14, 0x22);
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

        let r = CornerRadius::same(9);
        for (w, fill, stroke) in [
            (&mut v.widgets.noninteractive, SURFACE, BORDER),
            (&mut v.widgets.inactive, SURFACE_HI, BORDER),
            (
                &mut v.widgets.hovered,
                Color32::from_rgb(0x22, 0x2C, 0x44),
                Color32::from_rgb(0x34, 0x40, 0x5C),
            ),
            (&mut v.widgets.active, Color32::from_rgb(0x2A, 0x35, 0x52), INDIGO),
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
