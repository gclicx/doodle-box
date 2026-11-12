#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

doodle_meta!(
    "Doodle Flappy",
    "Flattere zwischen den Röhren durch – nur eine Berührung und es ist vorbei",
    env!("CARGO_PKG_VERSION")
);

use doodle_core::highscore;
use doodle_core::palette::{BORDER, MUTED, FG, GREEN, PANEL, YELLOW, WHITE};
use doodle_core::{aero_draw, alpha, doodle_meta, game_over_screen, seed_rng, text_centered, title_screen};
use macroquad::prelude::*;

const HIGHSCORE_ID: &str = "doodle-flappy";
const BIRD_X: f32 = 130.0;
const BIRD_R: f32 = 14.0;
const FLAP_V: f32 = -380.0;
const GRAVITY: f32 = 1300.0;
const PIPE_W: f32 = 64.0;
const PIPE_GAP: f32 = 155.0;
const PIPE_SPACING: f32 = 235.0;
const PIPE_SPEED: f32 = 150.0;

fn conf() -> Conf {
    Conf {
        window_title: "Doodle Flappy".to_owned(),
        window_width: 480,
        window_height: 640,
        window_resizable: false,
        high_dpi: true,
        ..Default::default()
    }
}

struct Pipe {
    x: f32,
    gap_y: f32,
    scored: bool,
}

/// Kreis schneidet Rechteck?
fn hits_rect(cx: f32, cy: f32, r: f32, rx: f32, ry: f32, rw: f32, rh: f32) -> bool {
    let nx = cx.clamp(rx, rx + rw);
    let ny = cy.clamp(ry, ry + rh);
    (cx - nx) * (cx - nx) + (cy - ny) * (cy - ny) <= r * r
}

#[macroquad::main(conf)]
async fn main() {
    seed_rng();

    if !title_screen(
        "Doodle Flappy",
        "Zwischen den Röhren durch",
        &[
            "Leertaste oder Klick flattert aufwärts",
            "Jede Röhre zählt einen Punkt",
            "Röhre, Boden oder Decke = Ende",
        ],
    )
    .await
    {
        return;
    }

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
    let ground_y = h - 60.0;

    let mut bird_y = h * 0.4;
    let mut bird_vy = 0.0_f32;
    let mut pipes: Vec<Pipe> = Vec::new();
    let mut score = 0_u32;
    let best = highscore::load(HIGHSCORE_ID);
    let bg_tex = aero_draw::wallpaper();

    loop {
        let dt = get_frame_time().min(0.033);

        if is_key_pressed(KeyCode::Escape) {
            return score;
        }
        if is_key_pressed(KeyCode::Space) || is_mouse_button_pressed(MouseButton::Left) {
            bird_vy = FLAP_V;
        }

        // Flugphysik
        bird_vy += GRAVITY * dt;
        bird_y += bird_vy * dt;

        // Röhren bewegen, nachlegen, wegwerfen
        for p in &mut pipes {
            p.x -= PIPE_SPEED * dt;
        }
        if pipes.is_empty() || pipes[pipes.len() - 1].x < w - PIPE_SPACING {
            let x = pipes
                .last()
                .map(|p| p.x + PIPE_SPACING)
                .unwrap_or(w + 240.0);
            let gap_y = rand::gen_range(
                PIPE_GAP * 0.5 + 40.0,
                ground_y - PIPE_GAP * 0.5 - 30.0,
            );
            pipes.push(Pipe { x, gap_y, scored: false });
        }
        pipes.retain(|p| p.x > -PIPE_W - 12.0);

        // Punktvergabe beim Passieren
        for p in &mut pipes {
            if !p.scored && p.x + PIPE_W < BIRD_X - BIRD_R {
                p.scored = true;
                score += 1;
            }
        }

        // Kollisionen: Röhren, Boden; Decke nur abbremsen
        let mut dead = bird_y + BIRD_R >= ground_y;
        for p in &pipes {
            let top_h = p.gap_y - PIPE_GAP * 0.5;
            let bot_y = p.gap_y + PIPE_GAP * 0.5;
            if hits_rect(BIRD_X, bird_y, BIRD_R, p.x, 0.0, PIPE_W, top_h)
                || hits_rect(BIRD_X, bird_y, BIRD_R, p.x, bot_y, PIPE_W, ground_y - bot_y)
            {
                dead = true;
                break;
            }
        }
        if dead {
            return score;
        }
        if bird_y - BIRD_R < 0.0 {
            bird_y = BIRD_R;
            bird_vy = bird_vy.max(0.0);
        }

        // Darstellung
        aero_draw::background(&bg_tex);
        for p in &pipes {
            let top_h = p.gap_y - PIPE_GAP * 0.5;
            let bot_y = p.gap_y + PIPE_GAP * 0.5;
            draw_rectangle(p.x, 0.0, PIPE_W, top_h, GREEN);
            draw_rectangle(p.x, bot_y, PIPE_W, ground_y - bot_y, GREEN);
            draw_rectangle(p.x - 4.0, top_h - 20.0, PIPE_W + 8.0, 20.0, GREEN);
            draw_rectangle(p.x - 4.0, bot_y, PIPE_W + 8.0, 20.0, GREEN);
        }
        // Boden
        draw_rectangle(0.0, ground_y, w, h - ground_y, PANEL);
        draw_line(0.0, ground_y, w, ground_y, 2.0, BORDER);

        // Vogel
        draw_circle(BIRD_X, bird_y, BIRD_R, YELLOW);
        draw_circle(BIRD_X, bird_y + 2.0, BIRD_R, alpha(MUTED, 0.12));
        draw_circle(BIRD_X - 0.5, bird_y - 0.5, BIRD_R, YELLOW);
        draw_circle(BIRD_X + 5.0, bird_y - 4.0, 4.5, WHITE);
        draw_circle(BIRD_X + 6.5, bird_y - 4.0, 2.0, FG);
        draw_circle(BIRD_X - 3.0, bird_y - 5.0, 3.0, alpha(WHITE, 0.85));
        draw_rectangle(BIRD_X - 12.0, bird_y + 1.0, 10.0, 7.0, alpha(WHITE, 0.65));

        // Punkte
        text_centered(&format!("{score}"), w * 0.5, 64.0, 40.0, FG);
        let btxt = format!("Best: {}", best.max(score));
        let bdim = measure_text(&btxt, None, 16_u16, 1.0);
        draw_text(&btxt, w - bdim.width - 14.0, 22.0, 16.0, MUTED);

        next_frame().await;
    }
}
