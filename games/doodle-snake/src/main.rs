//! Doodle Snake – klassische Schlangen-Spielre im Launcher.
//!
//! Steuerung: Pfeiltasten / WASD, ESC = beenden.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

doodle_meta!(
    "Doodle Snake",
    "Die klassische Schlange: fresse, wachse, verbiss dich nicht selbst",
    env!("CARGO_PKG_VERSION")
);

use doodle_core::highscore;
use doodle_core::palette::{self, BG, BORDER, MUTED, FG, GREEN, PANEL, RED};
use doodle_core::{alpha, doodle_meta, game_over_screen, seed_rng, text_centered, title_screen};
use macroquad::prelude::*;

const CELL: f32 = 24.0;
const COLS: i32 = 20;
const ROWS: i32 = 20;
const BOARD_W: f32 = CELL * COLS as f32; // 480 px
const BOARD_H: f32 = CELL * ROWS as f32;
const LEFT: f32 = 40.0;
const TOP: f32 = 120.0;
const HIGHSCORE_ID: &str = "doodle-snake";

fn conf() -> Conf {
    Conf {
        window_title: "Doodle Snake".to_owned(),
        window_width: 560,
        window_height: 660,
        window_resizable: false,
        high_dpi: true,
        ..Default::default()
    }
}

#[derive(Clone, Copy, PartialEq)]
struct Pos(i32, i32);

/// Neue Frucht auf einem zufälligen, freien Feld.
fn spawn_food(snake: &[Pos]) -> Pos {
    loop {
        let cand = Pos(
            rand::gen_range(0, COLS),
            rand::gen_range(0, ROWS),
        );
        if !snake.iter().any(|p| p == &cand) {
            return cand;
        }
    }
}

/// Richtung in die Warteschlange legen (ohne Duplikate, max. 3 voraus).
fn try_queue(pending: &mut Vec<(i32, i32)>, last: (i32, i32), d: (i32, i32)) {
    if pending.len() < 3 && pending.last().copied().unwrap_or(last) != d {
        pending.push(d);
    }
}

fn cell_center(p: Pos) -> (f32, f32) {
    (
        LEFT + (p.0 as f32 + 0.5) * CELL,
        TOP + (p.1 as f32 + 0.5) * CELL,
    )
}

#[macroquad::main(conf)]
async fn main() {
    seed_rng();

    if !title_screen(
            "Doodle Snake",
            "Die klassische Schlange",
            &[
                "Pfeiltasten oder WASD steuern",
                "Früchte fressen – nicht in die Wand und nicht in dich selbst beißen",
                "Mit steigendem Punktestand wird es schneller",
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
    let start_col = COLS / 2;
    let start_row = ROWS / 2;
    // Kopf (erste Zelle) liegt in Bewegungsrichtung vorne: Kopf rechts,
    // Körper dahinter nach links – sonst stößt die Schlange sofort in sich selbst.
    let mut snake: Vec<Pos> = (0..4).map(|i| Pos(start_col - i, start_row)).collect();
    let mut dir: (i32, i32) = (1, 0);
    let mut pending: Vec<(i32, i32)> = Vec::new();
    let mut food = spawn_food(&snake);
    let mut step_t = 0.115_f32;
    let mut acc = 0.0_f32;
    let mut score = 0_u32;
    let best = highscore::load(HIGHSCORE_ID);

    loop {
        let dt = get_frame_time().min(0.05);

        if is_key_pressed(KeyCode::Escape) {
            return score;
        }
        // Eingaben zwischenspeichern, damit schnelle Doppel-Eingaben nicht verloren gehen
        if is_key_pressed(KeyCode::Left) || is_key_pressed(KeyCode::A) {
            try_queue(&mut pending, dir, (-1, 0));
        }
        if is_key_pressed(KeyCode::Right) || is_key_pressed(KeyCode::D) {
            try_queue(&mut pending, dir, (1, 0));
        }
        if is_key_pressed(KeyCode::Up) || is_key_pressed(KeyCode::W) {
            try_queue(&mut pending, dir, (0, -1));
        }
        if is_key_pressed(KeyCode::Down) || is_key_pressed(KeyCode::S) {
            try_queue(&mut pending, dir, (0, 1));
        }

        // Spiellogik fester Schrittziten
        acc += dt;
        while acc >= step_t {
            acc -= step_t;

            if let Some(d) = pending.first().copied() {
                pending.remove(0);
                if d != dir && d != (-dir.0, -dir.1) {
                    dir = d;
                }
            }

            let head = snake[0];
            let next = Pos(head.0 + dir.0, head.1 + dir.1);

            // Wand oder Körper berührt -> Ende
            if next.0 < 0 || next.0 >= COLS || next.1 < 0 || next.1 >= ROWS {
                return score;
            }
            let eating = next == food;
            if !eating {
                snake.pop();
            }
            if snake.iter().any(|p| p == &next) {
                return score;
            }
            snake.insert(0, next);

            if eating {
                score += 1;
                // Randfall: Feld komplett voll – es gibt kein freies Feld
                // für eine neue Frucht mehr, also endet die Runde hier.
                if snake.len() >= (COLS * ROWS) as usize {
                    return score;
                }
                food = spawn_food(&snake);
                step_t = (0.115 - score as f32 * 0.0025).max(0.06);
            }
        }

        // Darstellung
        clear_background(BG);
        text_centered("Doodle Snake", LEFT + BOARD_W * 0.5, 52.0, 34.0, FG);
        draw_text(&format!("Punkte: {score}"), LEFT, 92.0, 18.0, MUTED);
        text_centered(
            &format!("Best: {}", best.max(score)),
            LEFT + BOARD_W * 0.5 + 130.0,
            92.0,
            18.0,
            MUTED,
        );

        // Spielfeld
        draw_rectangle(LEFT, TOP, BOARD_W, BOARD_H, PANEL);
        for i in 1..COLS {
            let x = LEFT + i as f32 * CELL;
            draw_line(x, TOP, x, TOP + BOARD_H, 1.0, alpha(FG, 0.05));
        }
        for j in 1..ROWS {
            let y = TOP + j as f32 * CELL;
            draw_line(LEFT, y, LEFT + BOARD_W, y, 1.0, alpha(FG, 0.05));
        }
        draw_rectangle_lines(LEFT, TOP, BOARD_W, BOARD_H, 2.0, BORDER);

        // Frucht
        let (fx, fy) = cell_center(food);
        draw_circle(fx, fy, CELL * 0.32, RED);
        draw_rectangle(fx - 1.5, fy - CELL * 0.32 - 4.0, 3.0, 5.0, GREEN);

        // Schlange
        for (i, p) in snake.iter().enumerate() {
            let (x, y) = cell_center(*p);
            let col = if i == 0 { palette::ACCENT } else { GREEN };
            draw_rectangle(x - CELL * 0.42, y - CELL * 0.42, CELL * 0.84, CELL * 0.84, col);
        }
        // Augen am Kopf (Richtung anzeigen)
        let (hx, hy) = cell_center(snake[0]);
        let perp = (dir.1 as f32, -dir.0 as f32);
        for side in [-1.0, 1.0] {
            draw_circle(
                hx + dir.0 as f32 * 5.0 + perp.0 * 6.0 * side,
                hy + dir.1 as f32 * 5.0 + perp.1 * 6.0 * side,
                3.0,
                BG,
            );
        }

        next_frame().await;
    }
}
