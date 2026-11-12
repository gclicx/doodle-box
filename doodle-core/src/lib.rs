//! `doodle-core` – gemeinsame Basis für alle Spiele der Sammlung:
//! Designsprache von shadcn/ui (Neutral/Zinc, hell), Bildschirme,
//! Bestwert-Speicherung und der Metadaten-Handshake für den Launcher.
//!
//! Spiele hängen nur an diesem Crate (plus `macroquad` für den
//! `#[macroquad::main]`-Attribute-Makro).

pub use doodle_meta::{doodle_meta, GameMeta, META_ARG};
pub use macroquad::prelude::*;

/// shadcn/ui-Palette (helles Zink-Theme von ui.shadcn.com) plus
/// Tailwind-500er-Sprite-Farben für Spielfiguren.
pub mod palette {
    use super::Color;

    /// --background
    pub const BG: Color = Color::from_rgba(255, 255, 255, 255);
    /// --card
    pub const CARD: Color = Color::from_rgba(255, 255, 255, 255);
    /// --border
    pub const BORDER: Color = Color::from_rgba(0xe4, 0xe4, 0xe7, 255);
    /// --foreground
    pub const FG: Color = Color::from_rgba(0x09, 0x09, 0x0b, 255);
    /// --muted-foreground (gedämpfter Text)
    pub const MUTED: Color = Color::from_rgba(0x71, 0x71, 0x7a, 255);
    /// --primary (fast schwarz)
    pub const PRIMARY: Color = Color::from_rgba(0x18, 0x18, 0x1b, 255);
    /// --primary-foreground
    pub const PRIMARY_FG: Color = Color::from_rgba(0xfa, 0xfa, 0xfa, 255);
    /// --secondary (gedämpfte Fläche)
    pub const SECONDARY: Color = Color::from_rgba(0xf4, 0xf4, 0xf5, 255);
    /// --destructive
    pub const DESTRUCTIVE: Color = Color::from_rgba(0xef, 0x44, 0x44, 255);
    /// --ring
    pub const RING: Color = Color::from_rgba(0xa1, 0xa1, 0xaa, 255);
    /// --radius in Pixeln (0.5rem).
    pub const RADIUS: f32 = 8.0;

    // Sprites (Tailwind-Palette, passt zur neutralen UI)
    pub const GREEN: Color = Color::from_rgba(0x22, 0xc5, 0x5e, 255);
    pub const BLUE: Color = Color::from_rgba(0x3b, 0x82, 0xf6, 255);
    pub const ORANGE: Color = Color::from_rgba(0xf9, 0x73, 0x16, 255);
    pub const YELLOW: Color = Color::from_rgba(0xfa, 0xcc, 0x15, 255);
    pub const RED: Color = Color::from_rgba(0xef, 0x44, 0x44, 255);
    /// Akzent für Spielfiguren (blau-500).
    pub const ACCENT: Color = Color::from_rgba(0x3b, 0x82, 0xf6, 255);
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

/// shadcn/ui-Zeichenhelfer (macroquad-Seite): Karten, Knöpfe, Badges, Kreise.
pub mod ui {
    use super::*;

    /// --card: weiße Fläche mit 1px-Border.
    pub fn card(x: f32, y: f32, w: f32, h: f32) {
        draw_rectangle(x, y, w, h, palette::CARD);
        draw_rectangle_lines(x, y, w, h, 1.0, palette::BORDER);
    }

    /// Schlichter Knopf. Varianten: "primary", "outline", "ghost".
    pub fn button(x: f32, y: f32, w: f32, h: f32, label: &str, variant: &str, font_size: f32) {
        let text = match variant {
            "primary" => {
                draw_rectangle(x, y, w, h, palette::PRIMARY);
                palette::PRIMARY_FG
            }
            "outline" => {
                draw_rectangle(x, y, w, h, palette::CARD);
                draw_rectangle_lines(x, y, w, h, 1.0, palette::BORDER);
                palette::FG
            }
            // ghost: nur Text, keine Fläche
            _ => palette::FG,
        };
        text_centered(
            label,
            x + w * 0.5,
            y + h * 0.5 + font_size * 0.36,
            font_size,
            text,
        );
    }

    /// Badge (sekundär): gedämpfte Fläche, gedämpfter Text.
    pub fn badge(x: f32, y: f32, w: f32, h: f32, label: &str, font_size: f32) {
        draw_rectangle(x, y, w, h, palette::SECONDARY);
        text_centered(label, x + w * 0.5, y + h * 0.5 + font_size * 0.36, font_size, palette::MUTED);
    }

    /// Flacher Kreis für Bälle mit dezenter Umrandung und Lichtpunkt.
    pub fn orb(x: f32, y: f32, r: f32, color: Color) {
        draw_circle(x, y, r, color);
        draw_circle_lines(x, y, r, 1.0, alpha(palette::FG, 0.35));
        draw_circle(x - r * 0.30, y - r * 0.35, r * 0.28, alpha(WHITE, 0.55));
    }

    /// Dünne Trennlinie über die volle Breite.
    pub fn separator(y: f32, w: f32) {
        draw_line(0.0, y, w, y, 1.0, palette::BORDER);
    }
}

/// Titelbildschirm im shadcn-Stil: weiße Fläche, Steuerungs-Karte,
/// primärer Knopf als CTA. Blockiert in eigener Schleife.
/// `true` = Spiel starten, `false` = Fenster schließen.
pub async fn title_screen(title: &str, subtitle: &str, lines: &[&str]) -> bool {
    loop {
        let w = screen_width();
        let h = screen_height();
        clear_background(palette::BG);
        ui::separator(0.0, w);
        ui::separator(h - 1.0, w);

        text_centered("DOODLE BOX", w * 0.5, h * 0.26, 14.0, palette::MUTED);
        text_centered(title, w * 0.5, h * 0.32, 54.0, palette::FG);
        text_centered(subtitle, w * 0.5, h * 0.375, 22.0, palette::MUTED);

        // Steuerungshinweise auf einer Karte mit Punkt-Bullets
        let mut maxw = 120.0_f32;
        for line in lines {
            maxw = maxw.max(measure_text(line, None, 19_u16, 1.0).width);
        }
        let py = h * 0.46;
        let ph = lines.len() as f32 * 30.0 + 30.0;
        let pw = maxw + 78.0;
        ui::card(w * 0.5 - pw * 0.5, py, pw, ph);
        let mut y = py + 34.0;
        for line in lines {
            let dim = measure_text(line, None, 19_u16, 1.0);
            let left = w * 0.5 - (dim.width + 24.0) * 0.5;
            draw_circle(left + 4.0, y - 6.0, 3.0, palette::PRIMARY);
            draw_text(line, left + 24.0, y, 19.0, palette::MUTED);
            y += 30.0;
        }

        // CTA: primärer Knopf
        let bw = 430.0_f32;
        let bh = 48.0_f32;
        let bx = w * 0.5 - bw / 2.0;
        let by = h * 0.75;
        ui::button(bx, by, bw, bh, "Leertaste oder Klick zum Starten", "primary", 21.0);

        text_centered("ESC = Beenden", w * 0.5, h * 0.94, 13.0, palette::MUTED);

        if is_key_pressed(KeyCode::Space) || is_mouse_button_pressed(MouseButton::Left) {
            return true;
        }
        if is_key_pressed(KeyCode::Escape) {
            return false;
        }
        next_frame().await;
    }
}

/// Game-Over-Bildschirm (shadcn): Karte mit Punktestand und Bestwert,
/// primärer Knopf zum Neustart. `true` = nochmal, `false` = beenden.
pub async fn game_over_screen(score: u32, best: u32) -> bool {
    loop {
        let w = screen_width();
        let h = screen_height();
        clear_background(palette::BG);
        ui::separator(0.0, w);
        ui::separator(h - 1.0, w);

        text_centered("GAME OVER", w * 0.5, h * 0.28, 52.0, palette::DESTRUCTIVE);

        // Ergebnis-Karte
        let pw = 380.0_f32;
        let ph = 132.0_f32;
        let px = w * 0.5 - pw / 2.0;
        let py = h * 0.38;
        ui::card(px, py, pw, ph);
        draw_rectangle(px, py, pw, 3.0, palette::PRIMARY);

        text_centered(&format!("Punkte: {score}"), w * 0.5, py + 58.0, 30.0, palette::FG);
        if score >= best && score > 0 {
            text_centered("Neuer Bestwert!", w * 0.5, py + 100.0, 21.0, palette::GREEN);
        } else {
            text_centered(
                &format!("Bestwert: {best}"),
                w * 0.5,
                py + 100.0,
                20.0,
                palette::MUTED,
            );
        }

        // CTA: primärer Knopf
        let bw = 430.0_f32;
        let bh = 48.0_f32;
        let bx = w * 0.5 - bw / 2.0;
        let by = h * 0.76;
        ui::button(bx, by, bw, bh, "Leertaste = Nochmal", "primary", 21.0);

        text_centered("ESC = Beenden", w * 0.5, h * 0.94, 13.0, palette::MUTED);

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
