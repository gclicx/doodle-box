//! Doodle Box – Launcher für kleine Rust-Spiele aus diesem Workspace.
//!
//! Der Launcher sucht neben seiner eigenen Datei (bzw. in `games/`-Ordnern)
//! nach Programmdateien, die mit `doodle-` beginnen, fragt jede mit dem
//! Argument `--doodle-meta` kurz nach Name/Beschreibung/Version und listet
//! sie auf. Ein Klick auf "Spielen" startet das Spiel als eigener Prozess.
//!
//! Neues Spiel hinzufügen:
//! 1. Neuen Crate im Ordner `games/` anlegen (Crate-Name beginnt mit `doodle-`).
//! 2. `doodle_meta!("Name", "Beschreibung", env!("CARGO_PKG_VERSION"));` in `main.rs`.
//! 3. `cargo run -p doodle-launcher` – das Spiel erscheint automatisch.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use doodle_meta::{GameMeta, META_ARG};
use eframe::egui;
use egui::{Align, Color32, Key, Layout, RichText, ScrollArea, TextStyle};
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

// ---- Farben (neutral, hell) ----
const BG: Color32 = Color32::from_rgb(0xf4, 0xf5, 0xf2);
const CARD: Color32 = Color32::from_rgb(0xff, 0xff, 0xff);
const ACCENT: Color32 = Color32::from_rgb(0x2f, 0x9e, 0x44);
const DISABLED: Color32 = Color32::from_rgb(0xb8, 0xbc, 0xc2);

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
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([880.0, 640.0])
            .with_min_inner_size([560.0, 420.0]),
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
            }))
        }),
    )
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

        egui::CentralPanel::default().show(ctx, |ui| {
            // Kopf
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.heading("Doodle Box");
                ui.label(
                    RichText::new("kleine Rust-Spiele aus diesem Workspace")
                        .weak()
                        .size(14.0),
                );
            });
            ui.add_space(2.0);
            ui.separator();
            ui.add_space(8.0);

            if self.games.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() * 0.25);
                    ui.label(RichText::new("Keine Spiele gefunden").size(20.0));
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(
                            "Spiele liegen neben der launcher-Programmdatei oder in einem games/-Ordner.\n\
                             Neues Spiel anlegen: games/<doodle-name>/ mit doodle_meta!(…) in main.rs.",
                        )
                        .weak()
                        .size(14.0),
                    );
                });
            } else {
                let mut launch_index: Option<usize> = None;

                ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for (i, entry) in self.games.iter().enumerate() {
                            ui.group(|ui| {
                                ui.set_min_height(78.0);
                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        ui.label(RichText::new(&entry.name).size(21.0).strong());
                                        ui.label(RichText::new(&entry.description).weak());
                                        ui.label(
                                            RichText::new(format!(
                                                "v{} · {}",
                                                entry.version,
                                                entry.path.display()
                                            ))
                                            .small()
                                            .weak(),
                                        );
                                    });
                                    ui.with_layout(
                                        Layout::right_to_left(Align::Center),
                                        |ui| {
                                            let button = if entry.has_meta {
                                                egui::Button::new(
                                                    RichText::new("Spielen").color(Color32::WHITE),
                                                )
                                                .fill(ACCENT)
                                                .min_size(egui::vec2(110.0, 40.0))
                                            } else {
                                                egui::Button::new("Starten")
                                                    .fill(DISABLED)
                                                    .min_size(egui::vec2(110.0, 40.0))
                                            };
                                            if ui.add(button).clicked() {
                                                launch_index = Some(i);
                                            }
                                        },
                                    );
                                });
                            });
                            ui.add_space(10.0);
                        }
                    });

                // Klicks erst nach der Liste verarbeiten (Borrowing).
                if let Some(i) = launch_index {
                    self.launch(i);
                }
            }

            // Fußzeile (bottom_up: zuerst Hinzugefügtes liegt unten)
            ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&self.status).weak().size(13.0));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button("Ordner öffnen").clicked() {
                            self.open_games_folder();
                        }
                        if ui.button("Aktualisieren (F5)").clicked() {
                            self.rescan();
                        }
                    });
                });
                ui.add_space(6.0);
                ui.separator();
            });
        });
    }
}

/// Helliges, neutrales Launcher-Design.
fn apply_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.visuals.panel_fill = BG;
    style.visuals.window_fill = BG;
    style.visuals.extreme_bg_color = CARD;
    style.visuals.widgets.noninteractive.bg_fill = CARD;
    style.visuals.widgets.inactive.bg_fill = CARD;
    style.visuals.widgets.hovered.bg_fill = BG;
    style.text_styles.entry(TextStyle::Heading).or_default().size = 28.0;
    style.text_styles.entry(TextStyle::Body).or_default().size = 16.0;
    style.text_styles.entry(TextStyle::Button).or_default().size = 17.0;
    ctx.set_style(style);
}
