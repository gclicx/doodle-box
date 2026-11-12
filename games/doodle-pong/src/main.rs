#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

doodle_meta!(
    "Doodle Pong",
    "Klassisches Pong gegen den CPU-Spieler – wer zuerst 7 Punkte hat",
    env!("CARGO_PKG_VERSION")
);

use doodle_core::highscore;
use doodle_core::palette::{self, ACCENT, BORDER, MUTED, FG};
use doodle_core::{alpha, doodle_meta, game_over_screen, seed_rng, text_centered, title_screen, ui};
use macroquad::prelude::*;

const HIGHSCORE_ID: &str = "doodle-pong";
const PADDLE_W: f32 = 10.0;
const PADDLE_H: f32 = 70.0;
const MARGIN: f32 = 14.0;
const BALL_R: f32 = 8.0;
const BALL_SPEED: f32 = 300.0;
const MAX_SPEED: f32 = 580.0;
const PLAYER_SPEED: f32 = 330.0;
const CPU_SPEED: f32 = 235.0;
const WIN_SCORE: u32 = 7;

fn conf() -> Conf {
    Conf {
        window_title: "Doodle Pong".to_owned(),
        window_width: 640,
        window_height: 400,
        window_resizable: false,
        high_dpi: true,
        ..Default::default()
    }
}

#[macroquad::main(conf)]
async fn main() {
    seed_rng();

    if !title_screen(
        "Doodle Pong",
        "Du gegen den CPU-Spieler",
        &[
            "Pfeiltasten Hoch/Runter oder W/S bewegt den Schläger",
            "Der Trefferpunkt auf dem Schläger gelenkt den Ball",
            "Erster Spieler mit 7 Punkten gewinnt",
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

/// Ball nach Schläger-Treffer neu ausrichten (Winkel hängt am Trefferpunkt).
fn paddle_bounce(v: &mut Vec2, dir: f32, speed: &mut f32, hit: f32) {
    let angle = hit.clamp(-0.8, 0.8) * 0.8; // max. ~46 Grad
    *speed = (*speed * 1.045).min(MAX_SPEED);
    v.x = angle.cos() * *speed * dir;
    v.y = angle.sin() * *speed;
}

async fn run_game() -> u32 {
    let w = screen_width();
    let h = screen_height();
    let top = 10.0_f32;
    let bottom = h - 10.0_f32;
    let half_h = PADDLE_H / 2.0;

    let mut player_y = h / 2.0;
    let mut cpu_y = h / 2.0;
    let mut ball = Vec2::new(w / 2.0, h / 2.0);
    let mut ball_v = Vec2::ZERO;
    let mut speed = BALL_SPEED;
    let mut serve_t = 0.9_f32;
    let mut serve_dir = -1.0_f32; // erster Aufschlag zum Spieler
    let mut p_score = 0_u32;
    let mut c_score = 0_u32;
    let mut end_t = 0.0_f32;
    let mut player_won = false;
    let best = highscore::load(HIGHSCORE_ID);

    loop {
        let dt = get_frame_time().min(0.033);

        if is_key_pressed(KeyCode::Escape) {
            return p_score;
        }

        // Zielschläger des Spielers
        let dir_up = if is_key_down(KeyCode::Up) || is_key_down(KeyCode::W) {
            -1.0
        } else if is_key_down(KeyCode::Down) || is_key_down(KeyCode::S) {
            1.0
        } else {
            0.0
        };
        player_y = (player_y + dir_up * PLAYER_SPEED * dt)
            .clamp(top + half_h, bottom - half_h);

        // CPU verfolgt den Ball mit beschränkter Geschwindigkeit (schlächbar)
        if ball_v.x > 0.0 {
            let diff = ball.y - cpu_y;
            if diff.abs() > 12.0 {
                cpu_y += diff.clamp(-CPU_SPEED * dt, CPU_SPEED * dt);
            }
        } else {
            cpu_y += ((h / 2.0 - cpu_y) * 2.0 * dt).clamp(-CPU_SPEED * dt, CPU_SPEED * dt);
        }
        cpu_y = cpu_y.clamp(top + half_h, bottom - half_h);

        if end_t > 0.0 {
            // Endemeldung zeigen, dann beenden
            end_t -= dt;
            if end_t <= 0.0 {
                return p_score;
            }
        } else {
            // Aufschlagspause
            if serve_t > 0.0 {
                serve_t -= dt;
            } else {
                ball += ball_v * dt;

                // Ober- und Unterkante
                if ball.y - BALL_R < top {
                    ball.y = top + BALL_R;
                    ball_v.y = ball_v.y.abs();
                } else if ball.y + BALL_R > bottom {
                    ball.y = bottom - BALL_R;
                    ball_v.y = -ball_v.y.abs();
                }

                // Spieler-Schläger (links)
                if ball_v.x < 0.0
                    && ball.x - BALL_R <= MARGIN + PADDLE_W
                    && ball.y > player_y - half_h - BALL_R
                    && ball.y < player_y + half_h + BALL_R
                {
                    ball.x = MARGIN + PADDLE_W + BALL_R;
                    let hit = (ball.y - player_y) / half_h;
                    paddle_bounce(&mut ball_v, 1.0, &mut speed, hit);
                }

                // CPU-Schläger (rechts)
                if ball_v.x > 0.0
                    && ball.x + BALL_R >= w - MARGIN - PADDLE_W
                    && ball.y > cpu_y - half_h - BALL_R
                    && ball.y < cpu_y + half_h + BALL_R
                {
                    ball.x = w - MARGIN - PADDLE_W - BALL_R;
                    let hit = (ball.y - cpu_y) / half_h;
                    paddle_bounce(&mut ball_v, -1.0, &mut speed, hit);
                }

                // Punkte
                if ball.x < -BALL_R - 6.0 {
                    c_score += 1;
                    serve_dir = -1.0;
                    serve_t = 0.9;
                    speed = BALL_SPEED;
                    ball = Vec2::new(w / 2.0, h / 2.0);
                    ball_v = Vec2::ZERO;
                } else if ball.x > w + BALL_R + 6.0 {
                    p_score += 1;
                    serve_dir = 1.0;
                    serve_t = 0.9;
                    speed = BALL_SPEED;
                    ball = Vec2::new(w / 2.0, h / 2.0);
                    ball_v = Vec2::ZERO;
                }
            }

            if serve_t <= 0.0 && ball_v == Vec2::ZERO {
                let vy = rand::gen_range(-130.0_f32, 130.0);
                ball_v = Vec2::new(serve_dir * (speed * speed - vy * vy).sqrt(), vy);
            }

            if p_score >= WIN_SCORE || c_score >= WIN_SCORE {
                player_won = p_score >= WIN_SCORE;
                end_t = 2.0;
            }
        }

        // Darstellung
        clear_background(palette::BG);
        for y in (top as i32..bottom as i32).step_by(26) {
            draw_line(
                w / 2.0,
                y as f32,
                w / 2.0,
                (y + 12) as f32,
                2.0,
                alpha(palette::MUTED, 0.40),
            );
        }
        draw_rectangle(MARGIN, player_y - half_h, PADDLE_W, PADDLE_H, palette::PRIMARY);
        draw_line(MARGIN, player_y - half_h + 1.0, MARGIN + PADDLE_W, player_y - half_h + 1.0, 1.5, alpha(WHITE, 0.4));
        draw_rectangle(w - MARGIN - PADDLE_W, cpu_y - half_h, PADDLE_W, PADDLE_H, palette::PRIMARY);
        draw_line(w - MARGIN - PADDLE_W, cpu_y - half_h + 1.0, w - MARGIN, cpu_y - half_h + 1.0, 1.5, alpha(WHITE, 0.4));
        ui::orb(ball.x, ball.y, BALL_R, ACCENT);
        draw_rectangle(0.0, 0.0, w, top, alpha(palette::FG, 0.03));
        draw_rectangle(0.0, bottom, w, h - bottom, alpha(palette::FG, 0.03));
        draw_line(0.0, top, w, top, 1.5, BORDER);
        draw_line(0.0, bottom, w, bottom, 1.5, BORDER);
        draw_text(&format!("Du: {p_score}"), 16.0, 30.0, 20.0, FG);
        draw_text(&format!("Best: {best}"), 16.0, 50.0, 14.0, MUTED);
        let ctxt = format!("CPU: {c_score}");
        let cdim = measure_text(&ctxt, None, 20_u16, 1.0);
        draw_text(&ctxt, w - cdim.width - 16.0, 30.0, 20.0, FG);

        if end_t > 0.0 {
            let msg = if player_won {
                format!("Du gewinnst! {p_score} : {c_score}")
            } else {
                format!("CPU gewinnt. {p_score} : {c_score}")
            };
            ui::card(w * 0.5 - 210.0, h * 0.5 - 44.0, 420.0, 88.0);
            text_centered(&msg, w * 0.5, h * 0.5 + 6.0, 28.0, palette::FG);
        }

        next_frame().await;
    }
}
