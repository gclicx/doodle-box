//! Doodle Jumper – springe immer höher auf Plattformen (Doodle-Jump-Prinzip).
//!
//! Steuerung: Pfeiltasten / A+D, ESC = beenden. Das Auto-Springen passiert
//! bei jeder Berührung einer Plattform von oben.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

doodle_meta!(
    "Doodle Jumper",
    "Springe auf Plattformen immer höher – wie weit kommst du?",
    env!("CARGO_PKG_VERSION")
);

use doodle_core::highscore;
use doodle_core::palette::{self, BLUE, MUTED, FG, GREEN, ORANGE};
use doodle_core::{aero_draw, alpha, doodle_meta, game_over_screen, seed_rng, text_centered, title_screen};
use macroquad::prelude::*;

const HIGHSCORE_ID: &str = "doodle-jumper";
const PLAYER_R: f32 = 14.0;
const GRAVITY: f32 = 1500.0;
const JUMP_V: f32 = -690.0;
const MOVE_SPEED: f32 = 360.0;
// Erreichbarkeitsgarantien: Sprunghöhe = JUMP_V^2/(2*GRAVITY) ~ 159 px,
// max. horizontale Flugweite ~ 331 px.
const MAX_GAP: f32 = 96.0;
const MAX_GAP_AFTER_BREAKABLE: f32 = 84.0;
const MAX_X_DIST: f32 = 190.0;

fn conf() -> Conf {
    Conf {
        window_title: "Doodle Jumper".to_owned(),
        window_width: 480,
        window_height: 720,
        window_resizable: false,
        high_dpi: true,
        ..Default::default()
    }
}

struct Player {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Normal,
    Moving,
    Breakable,
}

struct Platform {
    x: f32,
    y: f32,
    w: f32,
    kind: Kind,
    vx: f32,
    used: bool,
}

impl Platform {
    fn color(&self) -> Color {
        match self.kind {
            Kind::Normal => GREEN,
            Kind::Moving => BLUE,
            Kind::Breakable => ORANGE,
        }
    }
}

#[macroquad::main(conf)]
async fn main() {
    seed_rng();

    if !title_screen(
            "Doodle Jumper",
            "Immer höher springen",
            &[
                "Pfeiltasten oder A/D zum Bewegen",
                "Die Figur springt automatisch auf jeder Plattform",
                "Orange Plattformen zerbrechen nach dem ersten Sprung",
            ],
        )
        .await
    {
        return;
    }

    // Nach "Leertaste = Nochmal" direkt eine neue Runde (ohne Titelscreen).
    loop {
        let score = run_game().await;

        let best = highscore::load(HIGHSCORE_ID);
        if score > best {
            highscore::save(HIGHSCORE_ID, score);
        }

        if !game_over_screen(score, best.max(score)).await {
            return;
        }
    }
}

async fn run_game() -> u32 {
    let w = screen_width();
    let h = screen_height();

    let mut plats: Vec<Platform> = vec![Platform {
        x: w * 0.5,
        y: h * 0.80,
        w: 230.0,
        kind: Kind::Normal,
        vx: 0.0,
        used: false,
    }];
    let base_y = plats[0].y;
    let mut next_y = base_y - 70.0;

    let mut p = Player {
        x: w * 0.5,
        y: base_y - PLAYER_R,
        vx: 0.0,
        vy: 0.0,
    };
    let start_y = p.y;
    let mut cam_y = p.y - h * 0.55; // Fensterkante oben in Weltkoordinaten
    let mut score = 0_u32;
    let best = highscore::load(HIGHSCORE_ID);
    let bg_tex = aero_draw::wallpaper();

    loop {
        let dt = get_frame_time().min(0.05);

        if is_key_pressed(KeyCode::Escape) {
            return score;
        }

        // Horizontale Steuerung (weich an Zielgeschwindigkeit)
        let input = if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
            1.0
        } else if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
            -1.0
        } else {
            0.0
        };
        p.vx += (input * MOVE_SPEED - p.vx) * (10.0 * dt).min(1.0);

        // Physik
        p.vy += GRAVITY * dt;
        let prev_bottom = p.y + PLAYER_R;
        p.y += p.vy * dt;
        p.x += p.vx * dt;

        // Randlose Welt: horizontal umbrechen
        if p.x < -PLAYER_R {
            p.x = w + PLAYER_R;
        } else if p.x > w + PLAYER_R {
            p.x = -PLAYER_R;
        }

        // Kollision nur beim Fallen
        if p.vy > 0.0 {
            for pl in &mut plats {
                if pl.used {
                    continue;
                }
                let bottom = p.y + PLAYER_R;
                let half = PLAYER_R * 0.7;
                if prev_bottom <= pl.y && bottom >= pl.y
                    && p.x + half > pl.x - pl.w * 0.5
                    && p.x - half < pl.x + pl.w * 0.5
                {
                    if pl.kind == Kind::Breakable {
                        pl.used = true;
                        p.vy = JUMP_V * 0.85;
                    } else {
                        p.vy = JUMP_V;
                    }
                    break;
                }
            }
        }

        // Bewegliche Plattformen
        for pl in &mut plats {
            if pl.kind == Kind::Moving {
                pl.x += pl.vx * dt;
                let half = pl.w * 0.5;
                if pl.x - half < 8.0 || pl.x + half > w - 8.0 {
                    pl.vx = -pl.vx;
                    pl.x = pl.x.clamp(8.0 + half, w - 8.0 - half);
                }
            }
        }

        // Nach oben weiter Plattformen erzeugen – mit Erreichbarkeitsgarantien:
        // - vertikale Lücken bleiben unter der Sprunghöhe
        // - auf eine zerbrechliche Plattform folgt immer eine sichere (normal,
        //   kleinere Lücke), damit keine unlösbaren Sequenzen entstehen
        // - horizontale Distanz benachbarter Plattformen ist begrenzt
        let mut prev_x = plats.last().map(|p| p.x).unwrap_or(w * 0.5);
        while next_y > cam_y - 60.0 {
            let after_breakable = plats
                .last()
                .map(|p| p.kind == Kind::Breakable)
                .unwrap_or(false);
            let roll = rand::gen_range(0.0_f32, 1.0);
            let kind = if after_breakable || roll < 0.62 {
                Kind::Normal
            } else if roll < 0.85 {
                Kind::Moving
            } else {
                Kind::Breakable
            };
            let (w_p, speed) = match kind {
                Kind::Normal => (rand::gen_range(74.0, 100.0), 0.0),
                Kind::Moving => (
                    80.0,
                    rand::gen_range(55.0, 130.0)
                        * if rand::gen_range(0.0_f32, 1.0) < 0.5 { -1.0 } else { 1.0 },
                ),
                Kind::Breakable => (rand::gen_range(62.0, 80.0), 0.0),
            };
            let gap = if after_breakable {
                rand::gen_range(58.0, MAX_GAP_AFTER_BREAKABLE)
            } else {
                rand::gen_range(58.0, MAX_GAP)
            };
            // Horizontale Distanz zur vorherigen Plattform begrenzen;
            // das Clamping verkürzt nur, verlängert nie.
            let minx = w_p * 0.5 + 10.0;
            let maxx = w - w_p * 0.5 - 10.0;
            let x = rand::gen_range(prev_x - MAX_X_DIST, prev_x + MAX_X_DIST).clamp(minx, maxx);
            plats.push(Platform { x, y: next_y, w: w_p, kind, vx: speed, used: false });
            prev_x = x;
            next_y -= gap;
        }

        // Sichtbare Region aufräumen
        plats.retain(|pl| pl.y < cam_y + h + 80.0 && !pl.used);

        // Kamera folgt nach oben
        cam_y = cam_y.min(p.y - h * 0.55);

        // Punktestand = maximale Steighöhe
        score = score.max(((start_y - p.y).max(0.0) * 0.12) as u32);

        // Absturz unter das sichtbare Fenster -> Ende
        if p.y - cam_y > h + 40.0 {
            return score;
        }

        // Darstellung
        aero_draw::background(&bg_tex);
        // dezente Linien als Höhenreferenz (Navy, sehr blass)
        let grid_world_start = (cam_y / 44.0).floor() * 44.0;
        let mut gy = grid_world_start;
        while gy < cam_y + h + 44.0 {
            draw_line(0.0, gy - cam_y, w, gy - cam_y, 1.0, alpha(FG, 0.08));
            gy += 44.0;
        }

        for pl in &plats {
            let sy = pl.y - cam_y;
            let col = pl.color();
            if pl.kind == Kind::Breakable {
                // gebrochene Optik: zwei getrennte Hälften
                draw_rectangle(pl.x - pl.w * 0.5, sy, pl.w * 0.42, 10.0, col);
                draw_rectangle(pl.x + pl.w * 0.08, sy, pl.w * 0.42, 10.0, col);
            } else {
                draw_rectangle(pl.x - pl.w * 0.5, sy, pl.w, 10.0, col);
                draw_rectangle(pl.x - pl.w * 0.5, sy + 2.0, pl.w, 3.0, alpha(col, 0.5));
            }
        }

        // Spiel-Figur: schattierter Kreis mit Augen
        let sy = p.y - cam_y;
        draw_circle(p.x + 1.0, sy + 3.0, PLAYER_R, alpha(palette::MUTED, 0.25));
        draw_circle(p.x, sy, PLAYER_R, palette::YELLOW);
        draw_circle(p.x, sy, PLAYER_R - 6.0, alpha(WHITE, 0.55));
        let look = if p.vx.abs() > 30.0 { (p.vx / MOVE_SPEED) * 3.0 } else { 0.0 };
        for side in [-1.0_f32, 1.0] {
            draw_circle(p.x + 5.0 * side, sy - 4.0, 3.5, WHITE);
            draw_circle(p.x + 5.0 * side + look, sy - 4.0, 1.6, FG);
        }

        // HUD
        draw_text(&format!("Punkte: {score}"), 14.0, 34.0, 20.0, FG);
        let btxt = format!("Best: {}", best.max(score));
        let bdim = measure_text(&btxt, None, 20_u16, 1.0);
        draw_text(&btxt, w - bdim.width - 14.0, 34.0, 20.0, MUTED);

        if score == 0 {
            text_centered("← → zum Bewegen", w * 0.5, 80.0, 18.0, MUTED);
        }

        next_frame().await;
    }
}
