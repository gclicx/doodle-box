#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

doodle_meta!(
    "Doodle Breakout",
    "Zerlege die Mauer mit Ball und Schläger – klassisches Breakout",
    env!("CARGO_PKG_VERSION")
);

use doodle_core::highscore;
use doodle_core::palette::{self, ACCENT, BLUE, MUTED, FG, GREEN, ORANGE, RED, YELLOW};
use doodle_core::{alpha, doodle_meta, game_over_screen, seed_rng, text_centered, title_screen, ui};
use macroquad::prelude::*;

const HIGHSCORE_ID: &str = "doodle-breakout";
const PADDLE_W: f32 = 84.0;
const PADDLE_H: f32 = 13.0;
const BALL_R: f32 = 7.0;
const BALL_SPEED: f32 = 340.0;
const PADDLE_SPEED: f32 = 430.0;
const COLS: usize = 7;
const ROWS: usize = 6;

fn conf() -> Conf {
    Conf {
        window_title: "Doodle Breakout".to_owned(),
        window_width: 480,
        window_height: 640,
        window_resizable: false,
        high_dpi: true,
        ..Default::default()
    }
}

struct Brick {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: Color,
    alive: bool,
}

/// Kreis schneidet Rechteck?
fn hits_rect(cx: f32, cy: f32, r: f32, b: &Brick) -> bool {
    let nx = cx.clamp(b.x, b.x + b.w);
    let ny = cy.clamp(b.y, b.y + b.h);
    (cx - nx) * (cx - nx) + (cy - ny) * (cy - ny) <= r * r
}

#[macroquad::main(conf)]
async fn main() {
    seed_rng();

    if !title_screen(
        "Doodle Breakout",
        "Klassisches Breakout im Handumdrehen",
        &[
            "Pfeiltasten oder A/D bewegen den Schläger",
            "Jeder Block zählt 10 Punkte, alles abgeräumt + 50",
            "Fällt der Ball durch, ist die Runde vorbei",
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

    let paddle_y = h - 44.0;
    let mut paddle_x = w / 2.0;
    let mut ball = Vec2::new(w / 2.0, paddle_y - 22.0);
    let mut ball_v = Vec2::new(0.0, -BALL_SPEED);
    let mut score = 0_u32;
    let best = highscore::load(HIGHSCORE_ID);
    let start_time = get_time() as f32;

    // Mauer aufbauen
    let side = 18.0;
    let gap = 4.0;
    let bw = (w - 2.0 * side - (COLS - 1) as f32 * gap) / COLS as f32;
    let bh = 18.0;
    let top = 64.0;
    let row_colors = [RED, ORANGE, YELLOW, GREEN, BLUE, ACCENT];
    let mut bricks: Vec<Brick> = Vec::with_capacity(COLS * ROWS);
    for row in 0..ROWS {
        for col in 0..COLS {
            bricks.push(Brick {
                x: side + col as f32 * (bw + gap),
                y: top + row as f32 * (bh + gap),
                w: bw,
                h: bh,
                color: row_colors[row % row_colors.len()],
                alive: true,
            });
        }
    }

    loop {
        let dt = get_frame_time().min(0.033);

        if is_key_pressed(KeyCode::Escape) {
            return score;
        }

        // Schläger
        let dx = if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) {
            1.0
        } else if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) {
            -1.0
        } else {
            0.0
        };
        paddle_x = (paddle_x + dx * PADDLE_SPEED * dt)
            .clamp(PADDLE_W / 2.0 + 6.0, w - PADDLE_W / 2.0 - 6.0);

        // Ball
        ball += ball_v * dt;

        // Wände
        if ball.x - BALL_R < 4.0 {
            ball.x = 4.0 + BALL_R;
            ball_v.x = ball_v.x.abs();
        } else if ball.x + BALL_R > w - 4.0 {
            ball.x = w - 4.0 - BALL_R;
            ball_v.x = -ball_v.x.abs();
        }
        if ball.y - BALL_R < 10.0 {
            ball.y = 10.0 + BALL_R;
            ball_v.y = ball_v.y.abs();
        }

        // Schläger (nur wenn der Ball fällt)
        if ball_v.y > 0.0
            && ball.y + BALL_R >= paddle_y
            && ball.y + BALL_R <= paddle_y + PADDLE_H + 22.0
            && (ball.x - paddle_x).abs() <= PADDLE_W / 2.0 + BALL_R
        {
            let hit = (ball.x - paddle_x) / (PADDLE_W / 2.0);
            let hit = hit.clamp(-0.8, 0.8);
            ball_v.x = BALL_SPEED * 0.8 * hit;
            ball_v.y = -(BALL_SPEED * BALL_SPEED - ball_v.x * ball_v.x).sqrt();
            ball.y = paddle_y - BALL_R;
        }

        // Blöcke (maximal ein Treffer pro Bild)
        for b in &mut bricks {
            if b.alive && hits_rect(ball.x, ball.y, BALL_R, b) {
                b.alive = false;
                score += 10;
                // Abprall-Achse: Richtung der geringeren Überdeckung
                let nx = ball.x.clamp(b.x, b.x + b.w);
                let ny = ball.y.clamp(b.y, b.y + b.h);
                if (ball.y - ny).abs() > (ball.x - nx).abs() {
                    ball_v.y = -ball_v.y;
                } else {
                    ball_v.x = -ball_v.x;
                }
                break;
            }
        }

        // Verloren oder gewonnen?
        if ball.y - BALL_R > h + 4.0 {
            return score;
        }
        if bricks.iter().all(|b| !b.alive) {
            return score + 50;
        }

        // Darstellung
        clear_background(palette::BG);
        for b in &bricks {
            if b.alive {
                draw_rectangle(b.x, b.y, b.w, b.h, b.color);
                draw_rectangle(b.x, b.y + b.h - 4.0, b.w, 4.0, alpha(WHITE, 0.35));
                draw_line(b.x + 1.0, b.y + 1.0, b.x + b.w - 1.0, b.y + 1.0, 1.5, alpha(WHITE, 0.75));
            }
        }
        draw_rectangle(
            paddle_x - PADDLE_W / 2.0,
            paddle_y,
            PADDLE_W,
            PADDLE_H,
            palette::PRIMARY,
        );
        draw_rectangle_lines(paddle_x - PADDLE_W / 2.0, paddle_y, PADDLE_W, PADDLE_H, 1.0, palette::BORDER);
        draw_line(paddle_x - PADDLE_W / 2.0 + 2.0, paddle_y + 1.5, paddle_x + PADDLE_W / 2.0 - 2.0, paddle_y + 1.5, 1.5, alpha(WHITE, 0.4));
        ui::orb(ball.x, ball.y, BALL_R, ACCENT);

        draw_text(&format!("Punkte: {score}"), 14.0, 32.0, 20.0, FG);
        let btxt = format!("Best: {}", best.max(score));
        let bdim = measure_text(&btxt, None, 16_u16, 1.0);
        draw_text(&btxt, w - bdim.width - 14.0, 30.0, 16.0, MUTED);
        if get_time() as f32 - start_time < 3.0 {
            text_centered("Pfeiltasten oder A/D", w * 0.5, h - 12.0, 15.0, MUTED);
        }

        next_frame().await;
    }
}
