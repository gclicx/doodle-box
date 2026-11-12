//! Doodle Box – Launcher für kleine Rust-Spiele aus diesem Workspace.
//!
//! Der Launcher sucht neben seiner eigenen Datei (bzw. in `games/`-Ordnern)
//! nach Programmdateien, die mit `doodle-` beginnen, fragt jede mit dem
//! Argument `--doodle-meta` kurz nach Name/Beschreibung/Version und listet
//! sie auf. Ein Klick auf die Karte startet das Spiel als eigener Prozess.
//!
//! Design: Designsprache von shadcn/ui (Neutral/Zinc, hell) – flache Karten
//! mit 1px-Border, primärer Knopf in fast schwarz, gedämpfte Badges.
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

// ---- Farben (shadcn/ui, Hellethema Zinc) ----
const BG: Color32 = Color32::from_rgb(0xfa, 0xfa, 0xfa);
const CARD: Color32 = Color32::from_rgb(0xff, 0xff, 0xff);
const BORDER: Color32 = Color32::from_rgb(0xe4, 0xe4, 0xe7);
const BORDER_HOVER: Color32 = Color32::from_rgb(0xd4, 0xd4, 0xd8);
const FG: Color32 = Color32::from_rgb(0x09, 0x09, 0x0b);
const MUTED: Color32 = Color32::from_rgb(0x71, 0x71, 0x7a);
const MUTED_BG: Color32 = Color32::from_rgb(0xf4, 0xf4, 0xf5);
const PRIMARY: Color32 = Color32::from_rgb(0x18, 0x18, 0x1b);
const PRIMARY_HOVER: Color32 = Color32::from_rgb(0x27, 0x27, 0x2a);
const PRIMARY_FG: Color32 = Color32::from_rgb(0xfa, 0xfa, 0xfa);
const DESTRUCTIVE: Color32 = Color32::from_rgb(0xef, 0x44, 0x44);
/// Radius (0.5rem)
const RADIUS: f32 = 8.0;

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
                // Logo: primäres Quadrat mit „D"
                let (badge, _) = ui.allocate_exact_size(vec2(40.0, 40.0), Sense::hover());
                ui.painter().rect(badge, RADIUS, PRIMARY, Stroke::NONE, egui::StrokeKind::Inside);
                ui.painter().text(
                    badge.center(),
                    Align2::CENTER_CENTER,
                    "D",
                    FontId::proportional(18.0),
                    PRIMARY_FG,
                );

                ui.add_space(4.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new("Doodle Box").size(22.0).strong().color(FG));
                    ui.label(
                        RichText::new("Kleine Rust-Spiele · ein Klick zum Starten")
                            .size(13.0)
                            .color(MUTED),
                    );
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    // Zähler-Badge (secondary)
                    let count = format!(
                        "{} {}",
                        self.games.len(),
                        if self.games.len() == 1 { "Spiel" } else { "Spiele" }
                    );
                    let pill_w = {
                        let p0 = ui.painter();
                        p0.layout_no_wrap(count.clone(), FontId::proportional(13.0), MUTED)
                            .size()
                            .x
                            + 24.0
                    };
                    let (pill, _) = ui.allocate_exact_size(vec2(pill_w, 30.0), Sense::hover());
                    ui.painter().rect(pill, 15.0, MUTED_BG, Stroke::NONE, egui::StrokeKind::Inside);
                    ui.painter().text(
                        pill.center(),
                        Align2::CENTER_CENTER,
                        count,
                        FontId::proportional(13.0),
                        FG,
                    );

                    ui.add_space(6.0);
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("Aktualisieren (F5)").color(FG).size(13.5),
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
                    p.rect(rect, RADIUS, CARD, Stroke::NONE, egui::StrokeKind::Inside);
                    p.rect_stroke(rect, RADIUS, Stroke::new(1.0_f32, BORDER), egui::StrokeKind::Inside);
                    p.text(
                        pos2(rect.center().x, rect.top() + 48.0),
                        Align2::CENTER_CENTER,
                        "Noch keine Spiele gefunden",
                        FontId::proportional(20.0),
                        FG,
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
                            ui.add_space(10.0);
                        }
                    });
            }

            if let Some(i) = launch_index {
                self.launch(i);
            }

            // ---------------- Fußzeile ----------------
            ui.with_layout(Layout::bottom_up(Align::LEFT), |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&self.status).size(13.0).color(MUTED));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new("Ordner öffnen").color(FG).size(13.5),
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

/// shadcn-Karte: weiß, 1px-Border, 8px-Radius, komplette Karte klickbar,
/// Version als Badge, Pfad als Tooltip, primärer Knopf rechtsbündig.
fn draw_game_card(ui: &mut egui::Ui, entry: &GameEntry, index: usize, launch: &mut Option<usize>) {
    let (card, resp) = ui.allocate_exact_size(vec2(ui.available_width(), 92.0), Sense::click());
    let hovered = resp.hovered();

    let p = ui.painter();
    // Karte
    p.rect(card, RADIUS, if hovered { MUTED_BG } else { CARD }, Stroke::NONE, egui::StrokeKind::Inside);
    p.rect(
        card,
        RADIUS,
        Color32::TRANSPARENT,
        Stroke::new(1.0_f32, if hovered { BORDER_HOVER } else { BORDER }),
        egui::StrokeKind::Inside,
    );

    let resp = resp
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(format!("Datei: {}", entry.path.display()));

    // Name
    p.text(
        pos2(card.left() + 20.0, card.top() + 17.0),
        Align2::LEFT_TOP,
        &entry.name,
        FontId::proportional(18.0),
        FG,
    );

    // Versions-Badge
    let ver = format!("v{}", entry.version);
    let vgal = p.layout_no_wrap(ver, FontId::proportional(11.0), MUTED);
    let chip = Rect::from_min_size(
        pos2(card.left() + 20.0, card.top() + 46.0),
        vec2(vgal.size().x + 16.0, 20.0),
    );
    p.rect(chip, 6.0, MUTED_BG, Stroke::NONE, egui::StrokeKind::Inside);
    p.galley(
        pos2(chip.left() + 8.0, chip.center().y - vgal.size().y / 2.0),
        vgal,
        MUTED,
    );

    // Beschreibung (gekürzt), rot wenn kein Meta-Handshake
    let max_chars = ((card.width() - 340.0) / 7.4).max(24.0) as usize;
    let desc = clamp_chars(&entry.description, max_chars);
    p.text(
        pos2(chip.right() + 14.0, card.top() + 48.0),
        Align2::LEFT_TOP,
        desc,
        FontId::proportional(14.0),
        if entry.has_meta { MUTED } else { DESTRUCTIVE },
    );

    // Knopf rechtsbündig, vertikal zentriert
    let bw = 112.0_f32;
    let bh = 40.0_f32;
    let btn_rect = Rect::from_min_size(
        pos2(card.right() - 20.0 - bw, card.center().y - bh / 2.0),
        vec2(bw, bh),
    );
    let resp_btn = if entry.has_meta {
        button_primary(ui, btn_rect, "Spielen")
    } else {
        button_outline(ui, btn_rect, "Starten")
    };
    if resp_btn.clicked() {
        *launch = Some(index);
    }
    // Ganze Karte klickbar
    if resp.clicked() {
        *launch = Some(index);
    }
}

/// Primärknopf (shadcn default): dunkle Fläche, heller Text, Hover heller.
fn button_primary(ui: &mut egui::Ui, rect: Rect, label: &str) -> egui::Response {
    let resp = ui.put(
        rect,
        egui::Button::new(RichText::new(label).color(PRIMARY_FG).size(15.0))
            .fill(PRIMARY)
            .min_size(rect.size()),
    );
    let p = ui.painter();
    if resp.hovered() {
        p.rect(rect, RADIUS, PRIMARY_HOVER, Stroke::NONE, egui::StrokeKind::Inside);
    }
    if resp.is_pointer_button_down_on() {
        p.rect(rect, RADIUS, Color32::from_black_alpha(50), Stroke::NONE, egui::StrokeKind::Inside);
    }
    resp
}

/// Outline-Knopf (shadcn outline): weiße Fläche, Border, Hover gedämpft.
fn button_outline(ui: &mut egui::Ui, rect: Rect, label: &str) -> egui::Response {
    let resp = ui.put(
        rect,
        egui::Button::new(RichText::new(label).color(FG).size(14.0))
            .fill(CARD)
            .min_size(rect.size()),
    );
    let p = ui.painter();
    p.rect(rect, RADIUS, Color32::TRANSPARENT, Stroke::new(1.0_f32, BORDER), egui::StrokeKind::Inside);
    if resp.hovered() {
        p.rect(rect, RADIUS, MUTED_BG, Stroke::NONE, egui::StrokeKind::Inside);
        p.rect(rect, RADIUS, Color32::TRANSPARENT, Stroke::new(1.0_f32, BORDER), egui::StrokeKind::Inside);
    }
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

/// shadcn-Hellthema: Zink-Palette, 8px-Radius, dezente Widgets.
fn apply_theme(ctx: &egui::Context) {
    let mut s = (*ctx.style()).clone();
    s.visuals.dark_mode = false;
    s.visuals.panel_fill = BG;
    s.visuals.window_fill = CARD;
    s.visuals.extreme_bg_color = MUTED_BG;
    s.visuals.override_text_color = Some(FG);
    s.visuals.window_corner_radius = CornerRadius::same(10);
    s.visuals.menu_corner_radius = CornerRadius::same(10);
    s.visuals.interact_cursor = Some(CursorIcon::PointingHand);
    s.visuals.selection.bg_fill = PRIMARY;
    s.visuals.selection.stroke = Stroke::new(1.0_f32, PRIMARY_FG);

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
        st.fg_stroke = Stroke::new(1.0_f32, FG);
        st.corner_radius = CornerRadius::same(RADIUS as u8);
    }
    s.visuals.widgets.hovered.bg_fill = MUTED_BG;
    s.visuals.widgets.active.bg_fill = MUTED_BG;
    s.visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, FG);

    s.text_styles.entry(TextStyle::Heading).or_default().size = 22.0;
    s.text_styles.entry(TextStyle::Body).or_default().size = 15.0;
    s.text_styles.entry(TextStyle::Button).or_default().size = 15.0;
    s.text_styles.entry(TextStyle::Small).or_default().size = 12.0;

    s.spacing.item_spacing = vec2(10.0, 10.0);
    s.spacing.button_padding = vec2(12.0, 8.0);
    ctx.set_style(s);
}
