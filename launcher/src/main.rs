//! Doodle Box – Launcher für kleine Rust-Spiele aus diesem Workspace.
//!
//! Der Launcher sucht neben seiner eigenen Datei (bzw. in `games/`-Ordnern)
//! nach Programmdateien, die mit `doodle-` beginnen, fragt jede mit dem
//! Argument `--doodle-meta` kurz nach Name/Beschreibung/Version und listet
//! sie auf. Ein Klick auf die Karte startet das Spiel als eigener Prozess.
//!
//! Neues Spiel hinzufügen:
//! 1. Neuen Crate im Ordner `games/` anlegen (Crate-Name beginnt mit `doodle-`).
//! 2. `doodle_meta!("Name", "Beschreibung", env!("CARGO_PKG_VERSION"));` in `main.rs`.
//! 3. `cargo run -p doodle-launcher` – das Spiel erscheint automatisch.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use doodle_meta::{GameMeta, META_ARG};
use eframe::egui;
use egui::{
    pos2, vec2, Align, Align2, Color32, CornerRadius, CursorIcon, FontId, Key, Layout, Rect,
    RichText, ScrollArea, Sense, Stroke, TextStyle,
};
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

// ---- Farben (dunkles, modernes Slate-Thema passend zu den Spielen) ----
const BG: Color32 = Color32::from_rgb(0x1a, 0x1b, 0x22);
const CARD: Color32 = Color32::from_rgb(0x24, 0x26, 0x33);
const CARD_HOVER: Color32 = Color32::from_rgb(0x2c, 0x2f, 0x3e);
const CHIP: Color32 = Color32::from_rgb(0x2c, 0x2f, 0x3e);
const BORDER: Color32 = Color32::from_rgb(0x32, 0x36, 0x47);
const TEXT: Color32 = Color32::from_rgb(0xe9, 0xe6, 0xdf);
const MUTED: Color32 = Color32::from_rgb(0x90, 0x95, 0xa5);
const ACCENT: Color32 = Color32::from_rgb(0x63, 0xe6, 0xbe);
/// Weicher Akzentton für Hover-Kanten (Alpha).
fn accent_soft() -> Color32 {
    Color32::from_rgba_unmultiplied(0x63, 0xe6, 0xbe, 110)
}
const ON_ACCENT: Color32 = Color32::from_rgb(0x0b, 0x14, 0x10);

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
            // ---------------- Kopfzeile ----------------
            ui.horizontal(|ui| {
                // Logo-Badge
                let (badge, _) = ui.allocate_exact_size(vec2(46.0, 46.0), Sense::hover());
                let p = ui.painter();
                p.rect(
                    badge,
                    14.0,
                    ACCENT,
                    Stroke::NONE,
                    egui::StrokeKind::Inside,
                );
                p.text(
                    badge.center(),
                    Align2::CENTER_CENTER,
                    "D",
                    FontId::proportional(24.0),
                    ON_ACCENT,
                );

                ui.add_space(4.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new("Doodle Box").size(23.0).strong());
                    ui.label(
                        RichText::new("Kleine Rust-Spiele · ein Klick zum Starten")
                            .weak()
                            .size(13.0),
                    );
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    // Zähler-Pill
                    let count = format!(
                        "{} {}",
                        self.games.len(),
                        if self.games.len() == 1 { "Spiel" } else { "Spiele" }
                    );
                    // Breite erst per temporärem Painter messen,
                    // damit der Borrow vor dem allozieren endet.
                    let (gal, pill_w) = {
                        let p0 = ui.painter();
                        let g = p0.layout_no_wrap(count, FontId::proportional(13.0), MUTED);
                        let w = g.size().x + 26.0;
                        (g, w)
                    };
                    let pill = ui
                        .allocate_exact_size(vec2(pill_w, 30.0), Sense::hover())
                        .1
                        .rect;
                    let p = ui.painter();
                    p.rect(
                        pill,
                        15.0,
                        CHIP,
                        Stroke::new(1.0_f32, BORDER),
                        egui::StrokeKind::Inside,
                    );
                    p.galley(
                        pos2(pill.left() + 13.0, pill.center().y - gal.size().y / 2.0),
                        gal,
                        MUTED,
                    );

                    ui.add_space(6.0);
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("Aktualisieren").weak().size(14.0),
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
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() * 0.16);
                    ui.label(RichText::new("Noch keine Spiele gefunden").size(21.0));
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(
                            "Baue die Spiele mit  cargo build  oder lege ein doodle-* Programm\n\
                             neben die Launcher-Datei bzw. in einen games/-Ordner.",
                        )
                        .weak()
                        .size(14.0),
                    );
                });
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
                    ui.label(RichText::new(&self.status).weak().size(13.0));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Ordner öffnen").weak().size(13.5),
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
                    BORDER,
                    Stroke::NONE,
                    egui::StrokeKind::Outside,
                );
            });
        });
    }
}

/// Eine modern gestylte Karte: komplett klickbar, Hover-Effekt,
/// Versions-Chip, Pfad als Tooltip, Spiel-Button rechtsbündig.
fn draw_game_card(ui: &mut egui::Ui, entry: &GameEntry, index: usize, launch: &mut Option<usize>) {
    let (card, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 96.0), Sense::click());
    let hovered = resp.hovered();

    let p = ui.painter();
    p.rect(
        card,
        14.0,
        if hovered { CARD_HOVER } else { CARD },
        Stroke::new(1.2_f32, if hovered { accent_soft() } else { BORDER }),
        egui::StrokeKind::Inside,
    );

    let resp = resp
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(format!("Datei: {}", entry.path.display()));

    // Name
    p.text(
        pos2(card.left() + 20.0, card.top() + 16.0),
        Align2::LEFT_TOP,
        &entry.name,
        FontId::proportional(19.0),
        TEXT,
    );

    // Versions-Chip
    let ver = format!("v{}", entry.version);
    let vgal = p.layout_no_wrap(ver, FontId::proportional(11.0), MUTED);
    let chip = Rect::from_min_size(
        pos2(card.left() + 20.0, card.top() + 48.0),
        vec2(vgal.size().x + 18.0, 22.0),
    );
    p.rect(chip, 7.0, CHIP, Stroke::NONE, egui::StrokeKind::Inside);
    p.galley(
        pos2(chip.left() + 9.0, chip.center().y - vgal.size().y / 2.0),
        vgal,
        MUTED,
    );

    // Beschreibung (gekürzt, passt in Zeile neben Button)
    let max_chars = ((card.width() - 340.0) / 7.4).max(24.0) as usize;
    let desc = clamp_chars(&entry.description, max_chars);
    p.text(
        pos2(chip.right() + 14.0, card.top() + 50.0),
        Align2::LEFT_TOP,
        desc,
        FontId::proportional(14.0),
        MUTED,
    );

    // Spiel-Button rechtsbündig, vertikal zentriert
    let bw = 122.0_f32;
    let bh = 46.0_f32;
    let btn_rect = egui::Rect::from_min_size(
        pos2(card.right() - 20.0 - bw, card.center().y - bh / 2.0),
        vec2(bw, bh),
    );
    let button = play_button(entry);
    if ui.put(btn_rect, button).clicked() {
        *launch = Some(index);
    }
    // Ganze Karte klickbar
    if resp.clicked() {
        *launch = Some(index);
    }
}

fn play_button(entry: &GameEntry) -> egui::Button<'static> {
    if entry.has_meta {
        egui::Button::new(RichText::new("Spielen").color(ON_ACCENT).size(16.0))
            .fill(ACCENT)
            .min_size(vec2(122.0, 46.0))
    } else {
        egui::Button::new(RichText::new("Starten").color(MUTED).size(15.0))
            .fill(CARD)
            .stroke(Stroke::new(1.0_f32, BORDER))
            .min_size(vec2(122.0, 46.0))
    }
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

/// Dunkles, modernes Design.
fn apply_theme(ctx: &egui::Context) {
    let mut s = (*ctx.style()).clone();
    s.visuals.dark_mode = true;
    s.visuals.panel_fill = BG;
    s.visuals.window_fill = BG;
    s.visuals.extreme_bg_color = CARD;
    s.visuals.override_text_color = Some(TEXT);
    s.visuals.window_corner_radius = CornerRadius::same(10);
    s.visuals.menu_corner_radius = CornerRadius::same(10);
    s.visuals.interact_cursor = Some(CursorIcon::PointingHand);

    for st in [
        &mut s.visuals.widgets.noninteractive,
        &mut s.visuals.widgets.inactive,
        &mut s.visuals.widgets.hovered,
        &mut s.visuals.widgets.active,
        &mut s.visuals.widgets.open,
    ] {
        st.bg_fill = CARD;
        st.weak_bg_fill = CARD;
        st.bg_stroke = Stroke::new(1.0_f32, BORDER);
        st.fg_stroke = Stroke::new(1.0_f32, TEXT);
        st.corner_radius = CornerRadius::same(9);
    }
    s.visuals.widgets.hovered.bg_fill = CARD_HOVER;
    s.visuals.widgets.active.bg_fill = CARD_HOVER;

    s.text_styles.entry(TextStyle::Heading).or_default().size = 26.0;
    s.text_styles.entry(TextStyle::Body).or_default().size = 16.0;
    s.text_styles.entry(TextStyle::Button).or_default().size = 16.0;
    s.text_styles.entry(TextStyle::Small).or_default().size = 12.0;

    s.spacing.item_spacing = vec2(10.0, 10.0);
    s.spacing.button_padding = vec2(14.0, 9.0);
    ctx.set_style(s);
}
