//! Doodle Box – Launcher für kleine Rust-Spiele aus diesem Workspace.
//!
//! Der Launcher sucht neben seiner eigenen Datei (bzw. in `games/`-Ordnern)
//! nach Programmdateien, die mit `doodle-` beginnen, fragt jede mit dem
//! Argument `--doodle-meta` kurz nach Name/Beschreibung/Version und listet
//! sie auf. Ein Klick auf die Karte startet das Spiel als eigener Prozess.
//!
//! Design: Frutiger Aero (Kit "tauri2-frutiger-aero-kit") – transparentes
//! Glas über prozeduralem Wallpaper, 2-stufige Gel-Knöpfe, Navy-Text.
//!
//! Neues Spiel hinzufügen:
//! 1. Neuen Crate im Ordner `games/` anlegen (Crate-Name beginnt mit `doodle-`).
//! 2. `doodle_meta!("Name", "Beschreibung", env!("CARGO_PKG_VERSION"));` in `main.rs`.
//! 3. `cargo run -p doodle-launcher` – das Spiel erscheint automatisch.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use aero;
use doodle_meta::{GameMeta, META_ARG};
use eframe::egui;
use egui::{
    pos2, vec2, Align, Align2, Color32, CornerRadius, CursorIcon, FontId, Key, Layout, Rect,
    RichText, ScrollArea, Sense, Stroke, TextStyle, TextureHandle,
};
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

// ---- Farben aus dem Aero-Kit ----
const NAVY: Color32 = Color32::from_rgb(0x09, 0x21, 0x3d);
const MUTED: Color32 = Color32::from_rgb(0x2c, 0x4d, 0x6e);
const PRIMARY: Color32 = Color32::from_rgb(0x00, 0x78, 0xd7);
const BTN_BORDER: Color32 = Color32::from_rgb(0x00, 0x58, 0xad);
const DESTRUCTIVE: Color32 = Color32::from_rgb(0xd9, 0x26, 0x26);
const WHITE: Color32 = Color32::WHITE;

/// Ein gefundenes Spiel.
struct GameEntry {
    name: String,
    description: String,
    version: String,
    path: PathBuf,
    has_meta: bool,
}

/// Laufendes Spiel (für Statusanzeige beim Beenden).
struct RunningGame {
    name: String,
    child: Child,
}

struct LauncherApp {
    games: Vec<GameEntry>,
    running: Vec<RunningGame>,
    status: String,
    bg: Option<TextureHandle>,
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([920.0, 640.0])
            .with_min_inner_size([580.0, 440.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Doodle Box",
        options,
        Box::new(|cc| {
            apply_theme(&cc.egui_ctx);
            Ok(Box::new(LauncherApp {
                games: scan_games(),
                running: Vec::new(),
                status: String::new(),
                bg: Some(make_background(&cc.egui_ctx)),
            }))
        }),
    )
}

/// Prozeduralen Aero-Wallpaper als egui-Textur.
fn make_background(ctx: &egui::Context) -> TextureHandle {
    let pixels = aero::aero_pixels(768, 512);
    let image = egui::ColorImage::from_rgba_unmultiplied([768, 512], &pixels);
    ctx.load_texture("aero_hintergrund", image, egui::TextureOptions::LINEAR)
}

// ---------------------------------------------------------------- scanning

/// Kandidaten-Ordner für Spiele, in der Reihenfolge der Priorität.
fn candidate_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(d) = std::env::var_os("DOODLE_GAMES_DIR") {
        dirs.push(PathBuf::from(d));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            dirs.push(dir.to_path_buf());
            dirs.push(dir.join("games"));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        dirs.push(cwd.join("games"));
    }
    dirs
}

/// Sieht der Dateipfad wie ein Spiel aus? (`doodle-*`, ausführbare Erweiterung)
fn is_game_file(path: &Path) -> bool {
    let Some(name) = path.file_stem().and_then(|s| s.to_str()) else {
        return false;
    };
    if !name.starts_with("doodle-") || name == "doodle-launcher" {
        return false;
    }
    if cfg!(windows) {
        path.extension().and_then(|e| e.to_str()) == Some("exe")
    } else {
        // Unter Unix: keine Build-Datendateien (.d, .pdb, .so …), nur Programme.
        !matches!(
            path.extension().and_then(|e| e.to_str()),
            Some("d") | Some("pdb") | Some("so") | Some("json")
        )
    }
}

/// Fragt eine Spieledatei nach ihren Metadaten (kurzer Prozess-Handshake).
/// Gibt `None` zurück, wenn die Datei nicht antwortet oder nicht rechtzeitig endet.
fn query_meta(path: &Path) -> Option<GameMeta> {
    let mut cmd = Command::new(path);
    cmd.arg(META_ARG)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW: kein Konsolen-Flash
    }
    let mut child = cmd.spawn().ok()?;
    let stdout = child.stdout.take()?;

    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut line = String::new();
        if std::io::BufReader::new(stdout).read_line(&mut line).is_ok() {
            let _ = tx.send(line);
        }
    });

    let line = rx.recv_timeout(Duration::from_millis(900)).ok()?;

    // Falls das Spiel hängen geblieben ist: abschießen.
    if child.try_wait().map(|st| st.is_none()).unwrap_or(true) {
        let _ = child.kill();
        let _ = child.wait();
    }

    serde_json::from_str(line.trim()).ok()
}

/// Dateiname ohne `doodle-`-Präfix als lesbarer Name ("doodle-jumper" -> "Jumper").
fn pretty_name(stem: &str) -> String {
    let rest = stem.trim_start_matches("doodle-").replace('-', " ");
    let mut c = rest.chars();
    match c.next() {
        Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
        None => stem.to_string(),
    }
}

/// Alle Programme im Workspace-Sammeln und Metadaten abfragen.
fn scan_games() -> Vec<GameEntry> {
    let mut seen = std::collections::HashSet::new();
    let mut games = Vec::new();

    for dir in candidate_dirs() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !is_game_file(&path) {
                continue;
            }
            let key = path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            if !seen.insert(key.clone()) {
                continue;
            }

            let stem = key.clone();
            let entry = match query_meta(&path) {
                Some(meta) => GameEntry {
                    name: meta.name,
                    description: meta.description,
                    version: meta.version,
                    path,
                    has_meta: true,
                },
                None => GameEntry {
                    name: pretty_name(&stem),
                    description: "Kein --doodle-meta-Handshake (doodle_meta! fehlt?)".to_string(),
                    version: "?".to_string(),
                    path,
                    has_meta: false,
                },
            };
            games.push(entry);
        }
    }

    games.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    games
}

// ---------------------------------------------------------------- app

impl LauncherApp {
    fn rescan(&mut self) {
        let started = Instant::now();
        self.games = scan_games();
        self.status = format!(
            "{} Spiel(er) gefunden ({} ms)",
            self.games.len(),
            started.elapsed().as_millis()
        );
    }

    fn launch(&mut self, index: usize) {
        let Some(entry) = self.games.get(index) else {
            return;
        };
        let mut cmd = Command::new(&entry.path);
        #[cfg(windows)]
        {
            // Im Release-Launcher (GUI-Subsystem) kein Konsolen-Flash.
            if !cfg!(debug_assertions) {
                use std::os::windows::process::CommandExt;
                cmd.creation_flags(0x0800_0000);
            }
        }
        match cmd.spawn() {
            Ok(child) => {
                self.status = format!("Gestartet: {}", entry.name);
                self.running.push(RunningGame {
                    name: entry.name.clone(),
                    child,
                });
            }
            Err(err) => {
                self.status = format!("Fehler beim Starten von {}: {err}", entry.name);
            }
        }
    }

    /// Beendete Spiele austragen (kostet nichts, kein Polling-Networking).
    fn poll_running(&mut self) {
        let mut exited = Vec::new();
        self.running.retain_mut(|r| match r.child.try_wait() {
            Ok(Some(_)) => {
                exited.push(r.name.clone());
                false
            }
            Ok(None) => true,
            Err(_) => {
                exited.push(r.name.clone());
                false
            }
        });
        if !exited.is_empty() {
            self.status = format!("Beendet: {}", exited.join(", "));
        }
    }

    fn open_games_folder(&self) {
        let dir = candidate_dirs()
            .into_iter()
            .find(|d| d.is_dir())
            .unwrap_or_else(|| PathBuf::from("."));
        #[cfg(windows)]
        let _ = Command::new("explorer").arg(&dir).spawn();
        #[cfg(target_os = "macos")]
        let _ = Command::new("open").arg(&dir).spawn();
        #[cfg(target_os = "linux")]
        let _ = Command::new("xdg-open").arg(&dir).spawn();
    }
}

impl eframe::App for LauncherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_running();

        let f5 = ctx.input(|i| i.key_pressed(Key::F5));
        if f5 {
            self.rescan();
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(Color32::TRANSPARENT))
            .show(ctx, |ui| {
                // Wallpaper über die ganze Fläche
                if let Some(bg) = self.bg.as_ref() {
                    ui.painter().image(
                        bg.id(),
                        ui.max_rect(),
                        Rect::from_min_size(egui::Pos2::ZERO, vec2(1.0, 1.0)),
                        Color32::WHITE,
                    );
                }

                // ---------------- Kopfzeile ----------------
                ui.horizontal(|ui| {
                    // Glaskugel-Logo
                    let (badge, _) = ui.allocate_exact_size(vec2(46.0, 46.0), Sense::hover());
                    {
                        let p = ui.painter();
                        p.circle_filled(
                            badge.center() + vec2(0.0, 2.0),
                            21.0,
                            Color32::from_rgba_unmultiplied(0x00, 0x37, 0x6e, 60),
                        );
                        p.circle_filled(badge.center(), 21.0, PRIMARY);
                        p.circle_filled(
                            badge.center() - vec2(5.0, 7.0),
                            11.0,
                            Color32::from_white_alpha(90),
                        );
                        p.circle_filled(
                            badge.center() - vec2(6.5, 8.5),
                            5.0,
                            Color32::from_white_alpha(210),
                        );
                        p.circle_stroke(
                            badge.center(),
                            21.0,
                            Stroke::new(1.5_f32, Color32::from_white_alpha(235)),
                        );
                        p.text(
                            badge.center() + vec2(0.0, 1.0),
                            Align2::CENTER_CENTER,
                            "D",
                            FontId::proportional(19.0),
                            WHITE,
                        );
                    }

                    ui.add_space(4.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Doodle Box").size(23.0).strong().color(NAVY));
                        ui.label(
                            RichText::new("Kleine Rust-Spiele · ein Klick zum Starten")
                                .weak()
                                .size(13.0)
                                .color(MUTED),
                        );
                    });

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        // Zähler-Pill (Aero: weiße Glas-Pille mit blauer Kante)
                        let count = format!(
                            "{} {}",
                            self.games.len(),
                            if self.games.len() == 1 { "Spiel" } else { "Spiele" }
                        );
                        let pill_w = {
                            let p0 = ui.painter();
                            let gal =
                                p0.layout_no_wrap(count.clone(), FontId::proportional(13.0), NAVY);
                            gal.size().x + 26.0
                        };
                        let (pill, _) = ui
                            .allocate_exact_size(vec2(pill_w, 30.0), Sense::hover());
                        {
                            let p = ui.painter();
                            p.rect(pill, 15.0, Color32::from_white_alpha(200), Stroke::NONE, egui::StrokeKind::Inside);
                            p.rect_stroke(pill, 15.0, Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(0x6e, 0xb4, 0xeb, 210)), egui::StrokeKind::Inside);
                            p.line_segment(
                                [pos2(pill.left() + 4.0, pill.top() + 1.5), pos2(pill.right() - 4.0, pill.top() + 1.5)],
                                Stroke::new(1.2_f32, Color32::from_white_alpha(235)),
                            );
                            let gal = p.layout_no_wrap(count.clone(), FontId::proportional(13.0), NAVY);
                            p.galley(pos2(pill.center().x - gal.size().x / 2.0, pill.center().y - gal.size().y / 2.0), gal, NAVY);
                        }

                        ui.add_space(6.0);
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Aktualisieren").color(NAVY).size(13.5),
                                )
                                .frame(false)
                                .min_size(vec2(0.0, 30.0)),
                            )
                            .clicked()
                        {
                            self.rescan();
                        }
                    });
                });

                ui.add_space(14.0);

                // ---------------- Karteileiste ----------------
                let mut launch_index: Option<usize> = None;

                if self.games.is_empty() {
                    let (rect, _) =
                        ui.allocate_exact_size(vec2(ui.available_width(), 150.0), Sense::hover());
                    {
                        let p = ui.painter();
                        p.rect(rect, 14.0, Color32::from_white_alpha(120), Stroke::NONE, egui::StrokeKind::Inside);
                        p.rect(
                            Rect::from_min_size(rect.left_top(), vec2(rect.width(), rect.height() * 0.5)),
                            CornerRadius { nw: 14, ne: 14, sw: 0, se: 0 },
                            Color32::from_white_alpha(50),
                            Stroke::NONE,
                            egui::StrokeKind::Inside,
                        );
                        p.rect_stroke(rect, 14, Stroke::new(1.2_f32, Color32::from_white_alpha(206)), egui::StrokeKind::Inside);
                        p.line_segment(
                            [pos2(rect.left() + 4.0, rect.top() + 1.5), pos2(rect.right() - 4.0, rect.top() + 1.5)],
                            Stroke::new(1.2_f32, Color32::from_white_alpha(235)),
                        );
                        p.text(
                            pos2(rect.center().x, rect.top() + 48.0),
                            Align2::CENTER_CENTER,
                            "Noch keine Spiele gefunden",
                            FontId::proportional(20.0),
                            NAVY,
                        );
                        p.text(
                            pos2(rect.center().x, rect.top() + 80.0),
                            Align2::CENTER_CENTER,
                            "Baue die Spiele mit  cargo build  oder lege ein doodle-* Programm",
                            FontId::proportional(14.0),
                            MUTED,
                        );
                        p.text(
                            pos2(rect.center().x, rect.top() + 102.0),
                            Align2::CENTER_CENTER,
                            "neben die Launcher-Datei bzw. in einen games/-Ordner.",
                            FontId::proportional(14.0),
                            MUTED,
                        );
                    }
                } else {
                    ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            for (i, entry) in self.games.iter().enumerate() {
                                draw_game_card(ui, entry, i, &mut launch_index);
                                ui.add_space(12.0);
                            }
                        });
                }

                if let Some(i) = launch_index {
                    self.launch(i);
                }

                // ---------------- Fußzeile ----------------
                ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&self.status).weak().size(13.0).color(NAVY));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("Ordner öffnen").color(NAVY).size(13.5),
                                    )
                                    .frame(false),
                                )
                                .clicked()
                            {
                                self.open_games_folder();
                            }
                        });
                    });
                    ui.add_space(8.0);
                    let (line, _) = ui
                        .allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
                    ui.painter().rect(
                        line,
                        0.0,
                        Color32::from_white_alpha(150),
                        Stroke::NONE,
                        egui::StrokeKind::Outside,
                    );
                });
            });
    }
}

/// Aero-Glas-Karte: komplett klickbar, Hover-Glanz, Versions-Chip,
/// Pfad als Tooltip, Gel-Knopf rechtsbündig.
fn draw_game_card(ui: &mut egui::Ui, entry: &GameEntry, index: usize, launch: &mut Option<usize>) {
    let (card, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 96.0), Sense::click());
    let hovered = resp.hovered();
    let resp = resp.on_hover_cursor(CursorIcon::PointingHand);

    let p = ui.painter();
    // Schatten
    p.rect(
        card.translate(vec2(0.0, 5.0)),
        14.0,
        Color32::from_rgba_unmultiplied(0x00, 0x37, 0x6e, 30),
        Stroke::NONE,
        egui::StrokeKind::Outside,
    );
    // Glas
    p.rect(
        card,
        14.0,
        Color32::from_white_alpha(if hovered { 150 } else { 112 }),
        Stroke::NONE,
        egui::StrokeKind::Inside,
    );
    p.rect(
        Rect::from_min_size(card.left_top(), vec2(card.width(), card.height() * 0.5)),
        CornerRadius { nw: 14, ne: 14, sw: 0, se: 0 },
        Color32::from_white_alpha(58),
        Stroke::NONE,
        egui::StrokeKind::Inside,
    );
    p.rect(
        Rect::from_min_size(
            pos2(card.left(), card.top() + card.height() * 0.5),
            vec2(card.width(), card.height() * 0.5),
        ),
        CornerRadius { nw: 0, ne: 0, sw: 14, se: 14 },
        Color32::from_rgba_unmultiplied(0xd7, 0xf0, 0xff, 64),
        Stroke::NONE,
        egui::StrokeKind::Inside,
    );
    // Glaskante + Lichtsaum
    p.rect(
        card,
        14.0,
        Color32::TRANSPARENT,
        Stroke::new(1_f32, if hovered { Color32::from_rgba_unmultiplied(0x00, 0x78, 0xd7, 220) } else { Color32::from_white_alpha(215) }),
        egui::StrokeKind::Inside,
    );
    p.line_segment(
        [pos2(card.left() + 4.0, card.top() + 1.5), pos2(card.right() - 4.0, card.top() + 1.5)],
        Stroke::new(1.2_f32, Color32::from_white_alpha(240)),
    );

    let resp = resp.on_hover_text(format!("Datei: {}", entry.path.display()));

    // Name
    p.text(
        pos2(card.left() + 20.0, card.top() + 16.0),
        Align2::LEFT_TOP,
        &entry.name,
        FontId::proportional(19.0),
        NAVY,
    );

    // Versions-Chip
    let ver = format!("v{}", entry.version);
    let vgal = p.layout_no_wrap(ver, FontId::proportional(11.0), NAVY);
    let chip = Rect::from_min_size(
        pos2(card.left() + 20.0, card.top() + 48.0),
        vec2(vgal.size().x + 18.0, 22.0),
    );
    p.rect(chip, 7.0, Color32::from_white_alpha(190), Stroke::NONE, egui::StrokeKind::Inside);
    p.rect_stroke(chip, 7.0, Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(0x6e, 0xb4, 0xeb, 210)), egui::StrokeKind::Inside);
    p.galley(
        pos2(chip.left() + 9.0, chip.center().y - vgal.size().y / 2.0),
        vgal,
        NAVY,
    );

    // Beschreibung (gekürzt), rot wenn kein Meta-Handshake
    let max_chars = ((card.width() - 340.0) / 7.4).max(24.0) as usize;
    let desc = clamp_chars(&entry.description, max_chars);
    p.text(
        pos2(chip.right() + 14.0, card.top() + 50.0),
        Align2::LEFT_TOP,
        desc,
        FontId::proportional(14.0),
        if entry.has_meta { MUTED } else { DESTRUCTIVE },
    );

    // Knopf rechtsbündig, vertikal zentriert
    let bw = 122.0_f32;
    let bh = 46.0_f32;
    let btn_rect = Rect::from_min_size(
        pos2(card.right() - 20.0 - bw, card.center().y - bh / 2.0),
        vec2(bw, bh),
    );
    let resp_btn = if entry.has_meta {
        gel_button(ui, btn_rect, "Spielen", true)
    } else {
        glass_button(ui, btn_rect, "Starten")
    };
    if resp_btn.clicked() {
        *launch = Some(index);
    }
    // Ganze Karte klickbar
    if resp.clicked() {
        *launch = Some(index);
    }
}

/// Frutiger-Aero-Gel-Knopf (2-stufiger Glanz, Navy-Schatten der Schrift).
fn gel_button(ui: &mut egui::Ui, rect: Rect, label: &str, green: bool) -> egui::Response {
    let (top, bottom, border) = if green {
        (
            Color32::from_rgb(0x63, 0xcd, 0x20),
            Color32::from_rgb(0x3e, 0xa3, 0x00),
            Color32::from_rgb(0x2d, 0x7a, 0x00),
        )
    } else {
        (
            Color32::from_rgb(0x45, 0xa9, 0xf7),
            Color32::from_rgb(0x06, 0x7b, 0xe2),
            BTN_BORDER,
        )
    };
    let resp = ui.put(
        rect,
        egui::Button::new(RichText::new(label).color(WHITE).size(16.0))
            .fill(bottom)
            .min_size(rect.size()),
    );

    let p = ui.painter();
    let hover = resp.hovered();
    // Glanz-Oberzone
    p.rect(
        Rect::from_min_size(rect.left_top(), vec2(rect.width(), rect.height() * 0.49)),
        CornerRadius { nw: 10, ne: 10, sw: 0, se: 0 },
        top,
        Stroke::NONE,
        egui::StrokeKind::Inside,
    );
    if hover {
        p.rect(
            Rect::from_min_size(rect.left_top(), vec2(rect.width(), rect.height() * 0.49)),
            CornerRadius { nw: 10, ne: 10, sw: 0, se: 0 },
            Color32::from_white_alpha(70),
            Stroke::NONE,
            egui::StrokeKind::Inside,
        );
    }
    // Naht + Innenlicht
    p.line_segment(
        [pos2(rect.left() + 2.0, rect.top() + rect.height() * 0.49), pos2(rect.right() - 2.0, rect.top() + rect.height() * 0.49)],
        Stroke::new(1.3_f32, Color32::from_white_alpha(205)),
    );
    p.line_segment(
        [pos2(rect.left() + 3.0, rect.top() + 1.2), pos2(rect.right() - 3.0, rect.top() + 1.2)],
        Stroke::new(1.3_f32, Color32::from_white_alpha(230)),
    );
    // Rahmen
    p.rect(
        rect,
        10,
        Color32::TRANSPARENT,
        Stroke::new(1.2_f32, border),
        egui::StrokeKind::Inside,
    );
    resp
}

/// Glas-Knopf für Spiele ohne Metadaten-Handshake.
fn glass_button(ui: &mut egui::Ui, rect: Rect, label: &str) -> egui::Response {
    let resp = ui.put(
        rect,
        egui::Button::new(RichText::new(label).color(MUTED).size(15.0))
            .fill(Color32::from_white_alpha(160))
            .min_size(rect.size()),
    );
    let p = ui.painter();
    p.rect(
        Rect::from_min_size(rect.left_top(), vec2(rect.width(), rect.height() * 0.5)),
        CornerRadius { nw: 10, ne: 10, sw: 0, se: 0 },
        Color32::from_white_alpha(60),
        Stroke::NONE,
        egui::StrokeKind::Inside,
    );
    p.rect(
        rect,
        10,
        Color32::TRANSPARENT,
        Stroke::new(1.0_f32, Color32::from_white_alpha(215)),
        egui::StrokeKind::Inside,
    );
    resp
}

fn clamp_chars(s: &str, max: usize) -> String {
    let n = s.chars().count();
    if n <= max {
        s.to_string()
    } else {
        let c: String = s.chars().take(max.saturating_sub(1)).collect();
        format!("{c}…")
    }
}

/// Aero-Hellthema: Glas-Widgets, Navy-Text, Pointer-Cursor.
fn apply_theme(ctx: &egui::Context) {
    let mut s = (*ctx.style()).clone();
    s.visuals.dark_mode = false;
    s.visuals.panel_fill = Color32::TRANSPARENT;
    s.visuals.window_fill = Color32::TRANSPARENT;
    s.visuals.extreme_bg_color = Color32::from_white_alpha(150);
    s.visuals.override_text_color = Some(NAVY);
    s.visuals.window_corner_radius = CornerRadius::same(12);
    s.visuals.menu_corner_radius = CornerRadius::same(12);
    s.visuals.interact_cursor = Some(CursorIcon::PointingHand);
    s.visuals.selection.bg_fill = Color32::from_rgba_unmultiplied(0x00, 0x78, 0xd7, 110);

    for st in [
        &mut s.visuals.widgets.noninteractive,
        &mut s.visuals.widgets.inactive,
        &mut s.visuals.widgets.hovered,
        &mut s.visuals.widgets.active,
        &mut s.visuals.widgets.open,
    ] {
        st.bg_fill = Color32::from_white_alpha(120);
        st.weak_bg_fill = Color32::from_white_alpha(120);
        st.bg_stroke = Stroke::new(1.0_f32, Color32::from_white_alpha(200));
        st.fg_stroke = Stroke::new(1.0_f32, NAVY);
        st.corner_radius = CornerRadius::same(9);
    }
    s.visuals.widgets.hovered.bg_fill = Color32::from_white_alpha(170);
    s.visuals.widgets.active.bg_fill = Color32::from_white_alpha(200);

    s.text_styles.entry(TextStyle::Heading).or_default().size = 26.0;
    s.text_styles.entry(TextStyle::Body).or_default().size = 16.0;
    s.text_styles.entry(TextStyle::Button).or_default().size = 16.0;
    s.text_styles.entry(TextStyle::Small).or_default().size = 12.0;

    s.spacing.item_spacing = vec2(10.0, 10.0);
    s.spacing.button_padding = vec2(14.0, 9.0);
    ctx.set_style(s);
}
