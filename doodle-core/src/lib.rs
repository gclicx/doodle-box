//! `doodle-core` – gemeinsame Basis für alle Spiele der Sammlung:
//! Frutiger-Aero-Design (Palette, Wallpaper, Glas-Panels, Gel-Knöpfe),
//! Bildschirme, Bestwert-Speicherung und der Metadaten-Handshake
//! für den Launcher.
//!
//! Spiele hängen nur an diesem Crate (plus `macroquad` für den
//! `#[macroquad::main]`-Attribute-Makro).

pub use aero;
pub use doodle_meta::{doodle_meta, GameMeta, META_ARG};
pub use macroquad::prelude::*;

/// Frutiger-Aero-Grundfarben (Werte aus dem Tauri-Aero-Kit).
pub mod palette {
    use super::Color;

    /// Fallback: Himmel oben aus dem Wallpaper-Verlauf.
    pub const BG: Color = Color::from_rgba(0x4f, 0xa8, 0xf0, 255);
    /// Glasfläche (weiß, halbdurchsichtig über dem Wallpaper).
    pub const PANEL: Color = Color::from_rgba(255, 255, 255, 116);
    /// Glanz-Oberseite der Glasfläche.
    pub const PANEL_GLOSS: Color = Color::from_rgba(255, 255, 255, 58);
    /// Blaustich der Glasunterseite.
    pub const PANEL_LOWER: Color = Color::from_rgba(0xd7, 0xf0, 0xff, 74);
    /// Weißer Glasrand.
    pub const BORDER: Color = Color::from_rgba(255, 255, 255, 206);
    /// Text auf Glasflächen (Navy aus dem Kit).
    pub const FG: Color = Color::from_rgba(0x09, 0x21, 0x3d, 255);
    /// Gedämpfter Text (aus dem Kit).
    pub const MUTED: Color = Color::from_rgba(0x2c, 0x4d, 0x6e, 255);
    /// Aero-Hauptblau (Gel-Elemente, Akzent).
    pub const ACCENT: Color = Color::from_rgba(0x00, 0x74, 0xe0, 255);
    /// Knopfrahmen.
    pub const BTN_BORDER: Color = Color::from_rgba(0x00, 0x58, 0xad, 255);
    pub const GREEN: Color = Color::from_rgba(0x5c, 0xc9, 0x18, 255);
    pub const BLUE: Color = Color::from_rgba(0x2e, 0xa0, 0xff, 255);
    pub const ORANGE: Color = Color::from_rgba(0xff, 0x8a, 0x3d, 255);
    pub const YELLOW: Color = Color::from_rgba(0xff, 0xd4, 0x5e, 255);
    pub const RED: Color = Color::from_rgba(0xff, 0x5f, 0x57, 255);
    /// Text auf Gel-Knopf: weiß.
    pub const ON_ACCENT: Color = Color::from_rgba(255, 255, 255, 255);
    /// Schlichtes Weiß (z. B. für Text-Schatten auf hellen Flächen).
    pub const WHITE: Color = Color::from_rgba(255, 255, 255, 255);
}

/// Farbe mit neuem Alphawert (0.0–1.0).
pub fn alpha(c: Color, a: f32) -> Color {
    Color::new(c.r, c.g, c.b, a)
}

/// Horizontal zentrierter Text.
pub fn text_centered(text: &str, center_x: f32, baseline_y: f32, font_size: f32, color: Color) {
    let dim = measure_text(text, None, font_size as u16, 1.0);
    draw_text(text, center_x - dim.width * 0.5, baseline_y, font_size, color);
}

/// Zufallsgenerator mit der Systemzeit beseeden.
///
/// Macroquad startet sonst **jedes Mal mit demselben festen Seed 0** – ohne
/// diesen Aufruf verhielte sich jedes Spiel bei jedem Start exakt gleich.
/// Einmal zu Beginn von `main` aufrufen.
pub fn seed_rng() {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    rand::srand(nanos ^ nanos.rotate_left(29));
}

/// Frutiger-Aero-Zeichenhelfer (macroquad-Seite des Kits).
pub mod aero_draw {
    use super::*;

    /// Prozeduralen Aero-Wallpaper als Textur erzeugen.
    pub fn wallpaper() -> Texture2D {
        let (w, h) = (768usize, 512usize);
        let px = aero::aero_pixels(w, h);
        Texture2D::from_rgba8(w as u16, h as u16, &px)
    }

    /// Wallpaper randlos auf das Fenster skalieren.
    pub fn background(tex: &Texture2D) {
        draw_texture_ex(
            tex,
            0.0,
            0.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(screen_width(), screen_height())),
                ..Default::default()
            },
        );
    }

    /// Gestreifte Glasfläche (weißer Rand, 1-px-Lichtsaum, 2-Zonen-Glanz).
    pub fn panel(x: f32, y: f32, w: f32, h: f32) {
        draw_rectangle(x + 2.0, y + 6.0, w, h, alpha(Color::from_rgba(0, 55, 110, 255), 0.10));
        draw_rectangle(x, y, w, h, palette::PANEL);
        draw_rectangle(x, y, w, h * 0.5, palette::PANEL_GLOSS);
        draw_rectangle(x, y + h * 0.5, w, h * 0.5, palette::PANEL_LOWER);
        draw_rectangle_lines(x, y, w, h, 1.5, palette::BORDER);
        draw_line(x + 3.0, y + 1.5, x + w - 3.0, y + 1.5, 1.5, alpha(WHITE, 0.92));
    }

    /// 2-stufiger Gel-Knopf ohne Text. `bright` 0.0–1.0 (Hover-/Puls-Glanz).
    pub fn gel(x: f32, y: f32, w: f32, h: f32, green: bool, bright: f32) {
        let (top, bottom, border) = if green {
            (
                Color::from_rgba(0x58, 0xc2, 0x2d, 255),
                Color::from_rgba(0x3a, 0xa1, 0x02, 255),
                Color::from_rgba(0x2d, 0x7a, 0x00, 255),
            )
        } else {
            (palette::BLUE, palette::ACCENT, palette::BTN_BORDER)
        };
        draw_rectangle(x + 1.0, y + 4.0, w, h, alpha(border, 0.24));
        draw_rectangle(x, y, w, h, bottom);
        draw_rectangle(x, y, w, h * 0.49, top);
        if bright > 0.0 {
            draw_rectangle(x, y, w, h * 0.49, alpha(WHITE, bright * 0.55));
        }
        draw_line(
            x + 2.0,
            y + h * 0.49,
            x + w - 2.0,
            y + h * 0.49,
            1.5,
            alpha(WHITE, 0.82),
        );
        draw_line(x + 2.0, y + 1.0, x + w - 2.0, y + 1.0, 1.5, alpha(WHITE, 0.86));
        draw_rectangle_lines(x, y, w, h, 1.5, border);
    }

    /// Glaskugel (z. B. für Bälle).
    pub fn orb(x: f32, y: f32, r: f32) {
        draw_circle(x + 1.0, y + 2.0, r, alpha(Color::from_rgba(0, 55, 110, 255), 0.30));
        draw_circle(x, y, r, WHITE);
        draw_circle(x, y - r * 0.30, r * 0.55, alpha(WHITE, 0.55));
        draw_circle(x - r * 0.30, y - r * 0.38, r * 0.22, alpha(WHITE, 0.85));
        draw_circle_lines(x, y, r, 1.5, alpha(palette::ACCENT, 0.85));
    }

    /// Weißer Text mit dunklem Schatten (Lesbarkeit ohne Glas).
    pub fn halo_text(text: &str, x: f32, y: f32, size: f32, color: Color) {
        draw_text(text, x + 1.0, y + 1.5, size, alpha(NAVY_GLOW, 0.35));
        draw_text(text, x, y, size, color);
    }

    /// Zentrierte Variante von [`halo_text`].
    pub fn halo_text_centered(text: &str, center_x: f32, baseline_y: f32, size: f32, color: Color) {
        let dim = measure_text(text, None, size as u16, 1.0);
        halo_text(text, center_x - dim.width * 0.5, baseline_y, size, color);
    }

    const NAVY_GLOW: Color = Color::from_rgba(0x0b, 0x2c, 0x55, 255);
}

/// Titelbildschirm im Frutiger-Aero-Stil: Wallpaper, Mini-Glaskugeln,
/// Glas-Panel für die Steuerungshinweise, pulsierender Gel-CTA.
/// Blockiert in eigener Schleife. `true` = Spiel starten, `false` = Fenster schließen.
pub async fn title_screen(title: &str, subtitle: &str, lines: &[&str]) -> bool {
    let tex = aero_draw::wallpaper();
    loop {
        let w = screen_width();
        let h = screen_height();
        let t = get_time() as f32;
        aero_draw::background(&tex);

        // Blickpunkt: kleine Glaskugeln über dem Titel
        let marks = [
            palette::GREEN,
            palette::BLUE,
            palette::ACCENT,
            palette::ORANGE,
            palette::RED,
        ];
        let r = 7.0_f32;
        let gap = 10.0_f32;
        let row_w = marks.len() as f32 * (r * 2.0) + (marks.len() - 1) as f32 * gap;
        let mut mx = w * 0.5 - row_w * 0.5 + r;
        for c in marks {
            draw_circle(mx, h * 0.155, r, c);
            draw_circle(mx - r * 0.3, h * 0.155 - r * 0.35, r * 0.45, alpha(WHITE, 0.75));
            draw_circle_lines(mx, h * 0.155, r, 1.5, alpha(WHITE, 0.85));
            mx += r * 2.0 + gap;
        }

        text_centered(&format!("DOODLE BOX"), w * 0.5, h * 0.28, 15.0, alpha(palette::FG, 0.55));
        aero_draw::halo_text_centered(title, w * 0.5, h * 0.335, 58.0, palette::WHITE);
        aero_draw::halo_text_centered(subtitle, w * 0.5, h * 0.42, 23.0, palette::WHITE);

        // Steuerungshinweise auf einem Glas-Panel mit Punkt-Bullets
        let mut maxw = 120.0_f32;
        for line in lines {
            maxw = maxw.max(measure_text(line, None, 19_u16, 1.0).width);
        }
        let py = h * 0.50;
        let ph = lines.len() as f32 * 30.0 + 30.0;
        let pw = maxw + 78.0;
        aero_draw::panel(w * 0.5 - pw * 0.5, py, pw, ph);
        let mut y = py + 34.0;
        for line in lines {
            let dim = measure_text(line, None, 19_u16, 1.0);
            let left = w * 0.5 - (dim.width + 24.0) * 0.5;
            draw_circle(left + 4.0, y - 6.0, 3.0, palette::ACCENT);
            draw_text(line, left + 24.0, y, 19.0, palette::FG);
            y += 30.0;
        }

        // Start-CTA: blauer Gel-Knopf mit Puls
        let bw = 430.0_f32;
        let bh = 48.0_f32;
        let bx = w * 0.5 - bw / 2.0;
        let by = h * 0.79;
        let bright = (t * 3.0).sin() * 0.25 + 0.55;
        aero_draw::gel(bx, by, bw, bh, false, bright);
        aero_draw::halo_text_centered(
            "Leertaste oder Klick zum Starten",
            w * 0.5,
            by + 31.0,
            22.0,
            palette::ON_ACCENT,
        );

        aero_draw::halo_text_centered("ESC = Beenden", w * 0.5, h * 0.955, 13.0, palette::WHITE);

        if is_key_pressed(KeyCode::Space) || is_mouse_button_pressed(MouseButton::Left) {
            return true;
        }
        if is_key_pressed(KeyCode::Escape) {
            return false;
        }
        next_frame().await;
    }
}

/// Game-Over-Bildschirm (Frutiger Aero): Ergebnis-Glas-Panel mit
/// Punktestand und Bestwert-Zeile, grüner Gel-CTA.
pub async fn game_over_screen(score: u32, best: u32) -> bool {
    let tex = aero_draw::wallpaper();
    loop {
        let w = screen_width();
        let h = screen_height();
        let t = get_time() as f32;
        aero_draw::background(&tex);

        aero_draw::halo_text_centered("GAME OVER", w * 0.5, h * 0.30, 54.0, palette::WHITE);
        draw_rectangle(
            w * 0.5 - 130.0,
            h * 0.325,
            260.0,
            3.0,
            alpha(palette::RED, 0.85),
        );

        // Ergebnis-Glas-Panel
        let pw = 380.0_f32;
        let ph = 132.0_f32;
        let px = w * 0.5 - pw / 2.0;
        let py = h * 0.40;
        aero_draw::panel(px, py, pw, ph);

        text_centered(&format!("Punkte: {score}"), w * 0.5, py + 58.0, 30.0, palette::FG);
        if score >= best && score > 0 {
            let pulse = (t * 4.0).sin() * 0.3 + 0.7;
            text_centered(
                "Neuer Bestwert!",
                w * 0.5,
                py + 100.0,
                22.0,
                alpha(palette::ACCENT, pulse),
            );
        } else {
            text_centered(
                &format!("Bestwert: {best}"),
                w * 0.5,
                py + 100.0,
                20.0,
                palette::MUTED,
            );
        }

        // CTA: grüner Gel-Knopf mit Puls
        let bw = 430.0_f32;
        let bh = 48.0_f32;
        let bx = w * 0.5 - bw / 2.0;
        let by = h * 0.80;
        let bright = (t * 3.0).sin() * 0.25 + 0.55;
        aero_draw::gel(bx, by, bw, bh, true, bright);
        aero_draw::halo_text_centered("Leertaste = Nochmal", w * 0.5, by + 31.0, 22.0, palette::ON_ACCENT);
        aero_draw::halo_text_centered("ESC = Beenden", w * 0.5, h * 0.955, 13.0, palette::WHITE);

        if is_key_pressed(KeyCode::Space) || is_mouse_button_pressed(MouseButton::Left) {
            return true;
        }
        if is_key_pressed(KeyCode::Escape) {
            return false;
        }
        next_frame().await;
    }
}

/// Bestwerte im Benutzerordner (`%LOCALAPPDATA%/doodle-box` bzw. XDG data home).
pub mod highscore {
    use std::path::PathBuf;

    fn file_for(id: &str) -> Option<PathBuf> {
        let base = if cfg!(windows) {
            std::env::var_os("LOCALAPPDATA").map(|d| PathBuf::from(d).join("doodle-box"))?
        } else {
            std::env::var_os("XDG_DATA_HOME")
                .map(PathBuf::from)
                .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
                .map(|p| p.join("doodle-box"))?
        };
        std::fs::create_dir_all(&base).ok()?;
        Some(base.join(format!("{id}.txt")))
    }

    /// Bestwert lesen (0, wenn keiner existiert).
    pub fn load(id: &str) -> u32 {
        file_for(id)
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0)
    }

    /// Bestwert speichern (Best-Effort, Fehler werden ignoriert).
    pub fn save(id: &str, score: u32) {
        if let Some(p) = file_for(id) {
            let _ = std::fs::write(p, score.to_string());
        }
    }
}
