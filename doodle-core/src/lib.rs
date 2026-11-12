//! `doodle-core` – gemeinsame Basis für alle Spiele der Sammlung:
//! einfache Farbpalette, Text-/Bildschirm-Helfer, Bestwert-Speicherung
//! und der Metadaten-Handshake für den Launcher.
//!
//! Spiele hängen nur an diesem Crate (plus `macroquad` für den
//! `#[macroquad::main]`-Attribute-Makro).

pub use doodle_meta::{doodle_meta, GameMeta, META_ARG};
pub use macroquad::prelude::*;

/// Neutrale Grundfarben (dunkles Arcade-Schema).
pub mod palette {
    use super::Color;

    /// Hintergrund.
    pub const BG: Color = Color::from_rgba(0x21, 0x22, 0x2b, 255);
    /// Etwas hellerer Flächenuntergrund (Brett, Panels).
    pub const PANEL: Color = Color::from_rgba(0x2c, 0x2e, 0x3b, 255);
    /// Text / helle Elemente.
    pub const FG: Color = Color::from_rgba(0xe9, 0xe6, 0xdf, 255);
    /// Gedämpfter Text.
    pub const DIM: Color = Color::from_rgba(0xe9, 0xe6, 0xdf, 130);
    pub const GREEN: Color = Color::from_rgba(0x51, 0xcf, 0x66, 255);
    pub const BLUE: Color = Color::from_rgba(0x4d, 0xab, 0xf7, 255);
    pub const ORANGE: Color = Color::from_rgba(0xff, 0xa9, 0x4d, 255);
    pub const YELLOW: Color = Color::from_rgba(0xff, 0xd4, 0x5e, 255);
    pub const RED: Color = Color::from_rgba(0xff, 0x6b, 0x6b, 255);
    /// Akzent (Bergteal) für Hauptelemente.
    pub const ACCENT: Color = Color::from_rgba(0x63, 0xe6, 0xbe, 255);
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

/// Titelbildschirm mit Titel, Untertitel und Steuerungshinweisen.
/// Blockiert in eigener Schleife. `true` = Spiel starten, `false` = Fenster schließen.
pub async fn title_screen(title: &str, subtitle: &str, lines: &[&str]) -> bool {
    loop {
        let w = screen_width();
        let h = screen_height();
        clear_background(palette::BG);

        // Dünne Zierrahmen-Linien oben und unten
        draw_rectangle(0.0, 12.0, w, 2.0, alpha(palette::ACCENT, 0.6));
        draw_rectangle(0.0, h - 14.0, w, 2.0, alpha(palette::ACCENT, 0.6));

        text_centered(title, w * 0.5, h * 0.30, 56.0, palette::FG);
        text_centered(subtitle, w * 0.5, h * 0.40, 24.0, palette::ACCENT);

        let mut y = h * 0.55;
        for line in lines {
            text_centered(line, w * 0.5, y, 19.0, palette::DIM);
            y += 32.0;
        }

        let pulse = (get_time() as f32 * 3.0).sin() * 0.25 + 0.75;
        text_centered(
            "Leertaste oder Klick zum Starten",
            w * 0.5,
            h * 0.86,
            23.0,
            alpha(palette::FG, pulse),
        );
        text_centered("ESC = Beenden", w * 0.5, h * 0.94, 15.0, palette::DIM);

        if is_key_pressed(KeyCode::Space) || is_mouse_button_pressed(MouseButton::Left) {
            return true;
        }
        if is_key_pressed(KeyCode::Escape) {
            return false;
        }
        next_frame().await;
    }
}

/// Game-Over-Bildschirm. `true` = nochmal spielen, `false` = beenden.
/// `best` ist der bisherige Bestwert (vor dem Speichern des neuen).
pub async fn game_over_screen(score: u32, best: u32) -> bool {
    loop {
        let w = screen_width();
        let h = screen_height();
        clear_background(palette::BG);

        text_centered("GAME OVER", w * 0.5, h * 0.36, 54.0, palette::RED);
        draw_rectangle(w * 0.5 - 140.0, h * 0.385, 280.0, 3.0, alpha(palette::RED, 0.7));

        text_centered(&format!("Punkte: {score}"), w * 0.5, h * 0.50, 30.0, palette::FG);
        if score >= best && score > 0 {
            let pulse = (get_time() as f32 * 4.0).sin() * 0.3 + 0.7;
            text_centered("Neuer Bestwert!", w * 0.5, h * 0.585, 24.0, alpha(palette::ACCENT, pulse));
        } else {
            text_centered(&format!("Bestwert: {best}"), w * 0.5, h * 0.585, 22.0, palette::DIM);
        }

        let pulse = (get_time() as f32 * 3.0).sin() * 0.25 + 0.75;
        text_centered(
            "Leertaste = Nochmal · ESC = Beenden",
            w * 0.5,
            h * 0.84,
            22.0,
            alpha(palette::FG, pulse),
        );

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
