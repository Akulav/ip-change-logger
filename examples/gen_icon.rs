//! Renders the IPTrail logo into `assets/` (PNG sizes + a multi-size Windows `.ico`).
//!
//! The artwork mirrors `assets/logo.svg` and is drawn with signed distance
//! functions so it stays crisp at every size, including 16x16 tray icons.
//!
//! Run with: `cargo run --example gen_icon`

use image::codecs::ico::{IcoEncoder, IcoFrame};
use image::{ExtendedColorType, Rgba, RgbaImage};
use std::path::Path;

/// Logo geometry in a 256x256 design space (kept in sync with `assets/logo.svg`).
const RADIUS: f32 = 56.0;
const TRAIL: [(f32, f32); 4] = [(66.0, 186.0), (110.0, 128.0), (146.0, 160.0), (188.0, 86.0)];
const LINE_HALF_WIDTH: f32 = 7.0;
const DOT_RADIUS: f32 = 13.0;
const PIN_OUTER: f32 = 34.0;
const PIN_INNER: f32 = 19.0;
const PIN_CORE: f32 = 9.0;

const TOP_LEFT: [f32; 3] = [34.0, 211.0, 238.0]; // #22D3EE
const BOTTOM_RIGHT: [f32; 3] = [99.0, 102.0, 241.0]; // #6366F1
const BACK_DEEP: [f32; 3] = [76.0, 29.0, 149.0]; // #4C1D95

fn main() {
    let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("assets");
    std::fs::create_dir_all(&out).expect("create assets dir");

    let mut frames = Vec::new();
    for size in [16u32, 24, 32, 48, 64, 128, 256] {
        let img = render(size);
        if matches!(size, 32 | 64 | 128 | 256) {
            img.save(out.join(format!("icon-{size}.png"))).expect("write png");
        }
        frames.push(IcoFrame::as_png(img.as_raw(), size, size, ExtendedColorType::Rgba8).expect("ico frame"));
    }
    let file = std::fs::File::create(out.join("icon.ico")).expect("create ico");
    IcoEncoder::new(file).encode_images(&frames).expect("write ico");
    println!("wrote icons to {}", out.display());
}

fn render(size: u32) -> RgbaImage {
    const SS: u32 = 4; // supersampling per axis
    let scale = 256.0 / size as f32;
    let mut img = RgbaImage::new(size, size);
    for py in 0..size {
        for px in 0..size {
            let mut acc = [0.0f32; 4];
            for sy in 0..SS {
                for sx in 0..SS {
                    let x = (px as f32 + (sx as f32 + 0.5) / SS as f32) * scale;
                    let y = (py as f32 + (sy as f32 + 0.5) / SS as f32) * scale;
                    let c = sample(x, y);
                    // premultiplied accumulation
                    acc[0] += c[0] * c[3];
                    acc[1] += c[1] * c[3];
                    acc[2] += c[2] * c[3];
                    acc[3] += c[3];
                }
            }
            let n = (SS * SS) as f32;
            let a = acc[3] / n;
            let px_color =
                if a > 0.0 { [acc[0] / acc[3], acc[1] / acc[3], acc[2] / acc[3], a] } else { [0.0; 4] };
            img.put_pixel(
                px,
                py,
                Rgba([
                    px_color[0].round().clamp(0.0, 255.0) as u8,
                    px_color[1].round().clamp(0.0, 255.0) as u8,
                    px_color[2].round().clamp(0.0, 255.0) as u8,
                    (px_color[3] * 255.0).round().clamp(0.0, 255.0) as u8,
                ]),
            );
        }
    }
    img
}

/// Returns straight (non-premultiplied) RGB in 0..255 and alpha in 0..1.
fn sample(x: f32, y: f32) -> [f32; 4] {
    if sd_round_rect(x, y) > 0.0 {
        return [0.0; 4];
    }

    // Diagonal gradient with a deep violet bottom-right corner.
    let t = ((x + y) / 512.0).clamp(0.0, 1.0);
    let mut c = if t < 0.6 {
        mix(TOP_LEFT, BOTTOM_RIGHT, t / 0.6)
    } else {
        mix(BOTTOM_RIGHT, BACK_DEEP, (t - 0.6) / 0.4)
    };

    // Soft top highlight.
    let glow = (1.0 - y / 140.0).clamp(0.0, 1.0) * 0.10;
    c = mix(c, [255.0; 3], glow);

    let white = [255.0, 255.0, 255.0];
    let pin = TRAIL[3];

    // Trail line (faded slightly) and stop dots.
    let mut line = f32::MAX;
    for w in TRAIL.windows(2) {
        line = line.min(sd_segment(x, y, w[0], w[1]) - LINE_HALF_WIDTH);
    }
    if line <= 0.0 {
        c = mix(c, white, 0.78);
    }
    for &(cx, cy) in &TRAIL[..3] {
        if dist(x, y, cx, cy) <= DOT_RADIUS {
            c = white;
        }
    }

    // Destination pin: punch a ring out of everything below it, then draw it.
    let d = dist(x, y, pin.0, pin.1);
    if d <= PIN_OUTER + 6.0 && d > PIN_OUTER {
        // halo cut so the ring separates from the line
        let back = mix(TOP_LEFT, BOTTOM_RIGHT, (((x + y) / 512.0) / 0.6).clamp(0.0, 1.0));
        c = mix(back, [255.0; 3], glow);
    }
    if (PIN_INNER..=PIN_OUTER).contains(&d) {
        c = white;
    } else if d < PIN_INNER {
        let back = mix(TOP_LEFT, BOTTOM_RIGHT, (((x + y) / 512.0) / 0.6).clamp(0.0, 1.0));
        c = mix(back, [255.0; 3], glow);
        if d <= PIN_CORE {
            c = white;
        }
    }

    [c[0], c[1], c[2], 1.0]
}

fn sd_round_rect(x: f32, y: f32) -> f32 {
    let (hx, hy) = (128.0 - RADIUS, 128.0 - RADIUS);
    let qx = (x - 128.0).abs() - hx;
    let qy = (y - 128.0).abs() - hy;
    let ox = qx.max(0.0);
    let oy = qy.max(0.0);
    (ox * ox + oy * oy).sqrt() + qx.max(qy).min(0.0) - RADIUS
}

fn sd_segment(px: f32, py: f32, a: (f32, f32), b: (f32, f32)) -> f32 {
    let (pax, pay) = (px - a.0, py - a.1);
    let (bax, bay) = (b.0 - a.0, b.1 - a.1);
    let h = ((pax * bax + pay * bay) / (bax * bax + bay * bay)).clamp(0.0, 1.0);
    dist(pax, pay, bax * h, bay * h)
}

fn dist(x: f32, y: f32, cx: f32, cy: f32) -> f32 {
    ((x - cx).powi(2) + (y - cy).powi(2)).sqrt()
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
}
