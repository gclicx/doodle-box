//! `doodle-core` – gemeinsame Basis für alle Spiele der Sammlung:
//! einfache Farbpalette, Text-/Bildschirm-Helfer, Bestwert-Speicherung
//! und der Metadaten-Handshake für den Launcher.
//!
//! Spiele hängen nur an diesem Crate (plus `macroquad` für den
//! `#[macroquad::main]`-Attribute-Makro).

pub use doodle_meta::{doodle_meta, GameMeta, META_ARG};
pub use macroquad::prelude::*;

/// Grundfarben – identisch zum dunklen Slate-Design des Launchers.
pub mod palette {
    use super::Color;

    /// Hintergrund.
    pub const BG: Color = Color::from_rgba(0x1a, 0x1b, 0x22, 255);
    /// Flächengrund (Spielfeld, Panels).
    pub const PANEL: Color = Color::from_rgba(0x24, 0x26, 0x33, 255);
    /// Feine Abgrenzungslinien.
    pub const BORDER: Color = Color::from_rgba(0x32, 0x36, 0x47, 255);
    /// Text / helle Elemente.
    pub const FG: Color = Color::from_rgba(0xe9, 0xe6, 0xdf, 255);
    /// Gedämpfter Text.
    pub const MUTED: Color = Color::from_rgba(0x90, 0x95, 0xa5, 255);
    pub const GREEN: Color = Color::from_rgba(0x51, 0xcf, 0x66, 255);
    pub const BLUE: Color = Color::from_rgba(0x4d, 0xab, 0xf7, 255);
    pub const ORANGE: Color = Color::from_rgba(0xff, 0xa9, 0x4d, 255);
    pub const YELLOW: Color = Color::from_rgba(0xff, 0xd4, 0x5e, 255);
    pub const RED: Color = Color::from_rgba(0xff, 0x6b, 0x6b, 255);
    /// Akzent (Bergteal) für Hauptelemente.
    pub const ACCENT: Color = Color::from_rgba(0x63, 0xe6, 0xbe, 255);
    /// Text auf Akzentflächen.
    pub const ON_ACCENT: Color = Color::from_rgba(0x0b, 0x14, 0x10, 255);
}

/// Farbe mit neuem Alphawert (0.0–1.0).
pub fn alpha(c: Color, a: f32) -> Color {
    Color::new(c.r, c.g, c.b, a)
}

/// Zufallsgenerator mit der Systemzeit beseceden.
///
/// Macroquad startet sonst **jedes Mal mit demselben festen Seed 0** – ohne
/// diesen Aufruf verhielte sich jedes Spiel bei jedem Start exakt gleich
/// (gleiche Fruchtfolge, gleiche Plattformreihenfolge).
/// Einmal zu Beginn von `main` aufrufen.
pub fn seed_rng() {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    rand::srand(nanos ^ nanos.rotate_left(29));
}

/// Horizontal zentrierter Text.
pub fn text_centered(text: &str, center_x: f32, baseline_y: f32, font_size: f32, color: Color) {
    let dim = measure_text(text, None, font_size as u16, 1.0);
    draw_text(text, center_x - dim.width * 0.5, baseline_y, font_size, color);
}

/// Titelbildschirm im Launcher-Look: Eyebrow, Farbmarken, Punkt-Bullets,
/// dünne Akzent-Linien und pulsierender Start-„Button".
/// Blockiert in eigener Schleife. `true` = Spiel starten, `false` = Fenster schließen.
pub async fn title_screen(title: &str, subtitle: &str, lines: &[&str]) -> bool {
    loop {
        let w = screen_width();
        let h = screen_height();
        let t = get_time() as f32;
        clear_background(palette::BG);

        // Feine Akzent-Linien oben und unten
        draw_rectangle(0.0, 10.0, w, 2.0, alpha(palette::ACCENT, 0.55));
        draw_rectangle(0.0, h - 12.0, w, 2.0, alpha(palette::ACCENT, 0.55));

        // Farbmarken der Spielesammlung über dem Titel
        let marks = [
            palette::GREEN,
            palette::BLUE,
            palette::ACCENT,
            palette::ORANGE,
            palette::RED,
        ];
        let mw = 10.0_f32;
        let gap = 8.0_f32;
        let row_w = marks.len() as f32 * mw + (marks.len() - 1) as f32 * gap;
        let mut mx = w * 0.5 - row_w * 0.5;
        for c in marks {
            draw_rectangle(mx, h * 0.16, mw, mw, c);
            mx += mw + gap;
        }

        text_centered("DOODLE BOX", w * 0.5, h * 0.28, 15.0, palette::MUTED);
        text_centered(title, w * 0.5, h * 0.335, 58.0, palette::FG);
        text_centered(subtitle, w * 0.5, h * 0.42, 23.0, palette::ACCENT);

        // Steuerungshinweise als zentrierte Punkt-Liste
        let mut y = h * 0.54;
        for line in lines {
            let dim = measure_text(line, None, 19_u16, 1.0);
            let left = w * 0.5 - (dim.width + 24.0) * 0.5;
            draw_circle(left + 4.0, y - 6.0, 3.0, alpha(palette::ACCENT, 0.9));
            draw_text(line, left + 24.0, y, 19.0, palette::MUTED);
            y += 30.0;
        }

        // Start-„Button“ (CTA)
        let bw = 430.0_f32;
        let bh = 48.0_f32;
        let bx = w * 0.5 - bw / 2.0;
        let by = h * 0.855 - bh;
        draw_rectangle(bx, by, bw, bh, palette::PANEL);
        let pulse = (t * 3.0).sin() * 0.25 + 0.75;
        draw_rectangle_lines(bx, by, bw, bh, 2.0, alpha(palette::ACCENT, pulse));
        text_centered(
            "Leertaste oder Klick zum Starten",
            w * 0.5,
            by + 31.0,
            22.0,
            alpha(palette::ACCENT, pulse),
        );

        text_centered("ESC = Beenden", w * 0.5, h * 0.965, 13.0, palette::MUTED);

        if is_key_pressed(KeyCode::Space) || is_mouse_button_pressed(MouseButton::Left) {
            return true;
        }
        if is_key_pressed(KeyCode::Escape) {
            return false;
        }
        next_frame().await;
    }
}

/// Game-Over-Bildschirm im Launcher-Look: großes Ergebnis-Panel mit
/// Akzent-Kante, Punktestand und Bestwert-Zeile. `true` = nochmal, `false` = beenden.
pub async fn game_over_screen(score: u32, best: u32) -> bool {
    loop {
        let w = screen_width();
        let h = screen_height();
        let t = get_time() as f32;
        clear_background(palette::BG);

        draw_rectangle(0.0, 10.0, w, 2.0, alpha(palette::ACCENT, 0.55));
        draw_rectangle(0.0, h - 12.0, w, 2.0, alpha(palette::ACCENT, 0.55));

        text_centered("GAME OVER", w * 0.5, h * 0.30, 54.0, palette::FG);
        draw_rectangle(
            w * 0.5 - 130.0,
            h * 0.325,
            260.0,
            3.0,
            alpha(palette::RED, 0.75),
        );

        // Ergebnis-Panel
        let pw = 380.0_f32;
        let ph = 132.0_f32;
        let px = w * 0.5 - pw / 2.0;
        let py = h * 0.40;
        draw_rectangle(px, py, pw, ph, palette::PANEL);
        draw_rectangle_lines(px, py, pw, ph, 1.5, palette::BORDER);
        draw_rectangle(px, py, pw, 3.0, palette::ACCENT);

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

        // CTA
        let bw = 430.0_f32;
        let bh = 48.0_f32;
        let bx = w * 0.5 - bw / 2.0;
        let by = h * 0.845 - bh;
        draw_rectangle(bx, by, bw, bh, palette::PANEL);
        let pulse = (t * 3.0).sin() * 0.25 + 0.75;
        draw_rectangle_lines(bx, by, bw, bh, 2.0, alpha(palette::ACCENT, pulse));
        text_centered(
            "Leertaste = Nochmal",
            w * 0.5,
            by + 31.0,
            22.0,
            alpha(palette::ACCENT, pulse),
        );
        text_centered("ESC = Beenden", w * 0.5, h * 0.965, 13.0, palette::MUTED);

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
