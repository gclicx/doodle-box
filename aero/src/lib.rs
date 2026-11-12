//! Frutiger-Aero-Kit – prozedural erzeugter Wallpaper und gemeinsame Farben.
//!
//! Framework-neutral (nur `u8`-Tripel), damit der Launcher (egui) und
//! `doodle-core` (macroquad) dieselbe Designquelle nutzen.

/// Text auf Glasflächen (dunkles Navy).
pub const NAVY: [u8; 3] = [0x09, 0x21, 0x3d];
/// Gedämpfter Text auf Glas.
pub const MUTED: [u8; 3] = [0x2c, 0x4d, 0x6e];
/// Aero-Hauptblau.
pub const PRIMARY: [u8; 3] = [0x00, 0x78, 0xd7];
/// Knopfrahmen.
pub const BTN_BORDER: [u8; 3] = [0x00, 0x58, 0xad];
pub const SKY_TOP: [u8; 3] = [0x4f, 0xa8, 0xf0];
pub const SKY_LOW: [u8; 3] = [0xd9, 0xf1, 0xfe];
pub const GRASS_TOP: [u8; 3] = [0x8f, 0xd4, 0x5a];
pub const GRASS_LOW: [u8; 3] = [0x3f, 0x9e, 0x2f];
pub const AERO_BLUE: [u8; 3] = [0x2e, 0xa0, 0xff];
pub const AERO_GREEN: [u8; 3] = [0x5c, 0xc9, 0x18];
pub const YELLOW: [u8; 3] = [0xff, 0xd4, 0x5e];
pub const ORANGE: [u8; 3] = [0xff, 0x8a, 0x3d];
pub const RED: [u8; 3] = [0xff, 0x5f, 0x57];

/// Glatter Übergang.
fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn mix(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    [
        (a[0] as f32 + (b[0] as f32 - a[0] as f32) * t) as u8,
        (a[1] as f32 + (b[1] as f32 - a[1] as f32) * t) as u8,
        (a[2] as f32 + (b[2] as f32 - a[2] as f32) * t) as u8,
    ]
}

/// Erstellt den Frutiger-Aero-Hintergrund (Himmel, Sonnenglühen,
/// geschwungener Horizont mit Dunst und Gras) als RGBA8-Daten.
pub fn aero_pixels(w: usize, h: usize) -> Vec<u8> {
    let mut buf = vec![0u8; w * h * 4];
    let (sw, sh) = (w as f32, h as f32);

    for y in 0..h {
        let yf = y as f32 / sh;
        for x in 0..w {
            let xf = x as f32 / sw;

            // Geschwungene Hügel-Linie
            let horizon = 0.64 + 0.012 * (xf * 7.0).sin();
            let mut col = if yf < horizon {
                let t = smooth((yf / horizon).powf(1.2));
                mix(SKY_TOP, SKY_LOW, t)
            } else {
                let t = smooth(((yf - horizon) / (1.0 - horizon)).min(1.0));
                mix(GRASS_TOP, GRASS_LOW, t)
            };

            // Dunst direkt am Horizont
            let d = (yf - horizon).abs();
            let haze = (1.0 - d / 0.055).clamp(0.0, 1.0);
            col = mix(col, [0xfa, 0xfd, 0xff], haze * 0.55);

            // Sonnenglühen oben rechts
            let dx = xf - 0.82;
            let dy = yf - 0.10;
            let dd = (dx * dx + dy * dy).sqrt();
            let glow = (1.0 - dd / 0.42).clamp(0.0, 1.0);
            let glow = glow * glow;
            col = mix(col, [255, 246, 224], glow * 0.65);

            let i = (y * w + x) * 4;
            buf[i] = col[0];
            buf[i + 1] = col[1];
            buf[i + 2] = col[2];
            buf[i + 3] = 255;
        }
    }
    buf
}
