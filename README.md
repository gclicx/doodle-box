# Doodle Box

Eine kleine Spielesammlung in Rust mit eigenem **Launcher**: Der Launcher
findet alle Spiele dieses Workspaces automatisch und startet sie als eigene
Fenster. Neue Spiele werden einfach als Crate im Ordner `games/` angelegt –
ohne Änderung am Launcher.

## Spielesammlung

| Spiel | Beschreibung | Steuerung |
|---|---|---|
| **Doodle Snake** | Klassische Schlange: fressen, wachsen, sich nicht selbst beißen | Pfeiltasten / WASD, ESC |
| **Doodle Jumper** | Springe auf Plattformen immer höher | Pfeiltasten / A+D, ESC |
| **Doodle Flappy** | Flattere zwischen den Röhren durch | Leertaste / Klick, ESC |
| **Doodle Pong** | Pong gegen den CPU-Spieler, bis 7 Punkte | Pfeiltasten H/R / W+S, ESC |
| **Doodle Breakout** | Zerlege die Mauer mit Ball und Schläger | Pfeiltasten / A+D, ESC |

Der Launcher zeigt Name, Beschreibung, Version und Programmdateipfad jedes
Spiels an. Bestwerte werden pro Spiel im Benutzerprofil gespeichert.

## Download & spielen (ohne Rust)

Fertige Windows-Builds gibt es unter
[Releases](https://github.com/gclicx/doodle-box/releases/latest)
(Windows 10/11, 64-Bit): ZIP entpacken, `Doodle Box.exe` doppelklicken,
Spiel anklicken – fertig. Die Exe ist nicht signiert; falls SmartScreen beim
ersten Start warnen sollte: *Weitere Informationen* → *Trotzdem ausführen*.
Neue Versionen erscheinen immer dort, sobald `scripts\bundle.ps1` neu gebaut
und ein Release hochgeladen wurde.

## Bauen & Starten

```sh
cargo run -p doodle-launcher
```

(Der erste Build dauert einige Minuten.) Für einen schlanken Release-Build:

```sh
cargo build --release
cargo run -p doodle-launcher --release
```

Alle Spiele liegen nach dem Build neben der Launcher-Programmdatei unter
`target/release/` (bzw. `target/debug/`) – genau dort sucht der Launcher.

## Fertige Exe (Doppelklicken)

Ein verschiebbarer, startfertiger Ordner mit Launcher und allen Spielen
entsteht so:

```sh
powershell -ExecutionPolicy Bypass -File scripts\bundle.ps1
```

Danach liegt im Projektordner:

```
Doodle Box\
├── Doodle Box.exe        ← einfach doppelklicken
├── doodle-jumper.exe
├── doodle-snake.exe
├── doodle-flappy.exe
├── doodle-pong.exe
└── doodle-breakout.exe
```

Der Ordner darf beliebig verschoben werden – der Launcher findet die
`doodle-*.exe` immer neben seiner eigenen Datei. Nach dem Hinzufügen neuer
Spiele einfach das Bundle-Skript erneut ausführen, dann erscheinen sie dort
und in der Liste.

## Neues Spiel hinzufügen (Checkliste)

1. Neuen Crate unter `games/` anlegen, der Name beginnt mit `doodle-`
   (z. B. `games/doodle-flappy`), und ihn in die `[workspace] members`-Liste
   der `Cargo.toml` im Hauptordner ergänzen:
   ```sh
   cargo new games/doodle-flappy
   ```
2. In `games/doodle-flappy/Cargo.toml` die Abhängigkeiten eintragen:
   ```toml
   [dependencies]
   macroquad = { workspace = true }
   doodle-core = { workspace = true }
   ```
3. In `main.rs` ganz oben das Metadaten-Makro einbinden (liefert dem Launcher
   Name, Beschreibung und Version):
   ```rust
   doodle_meta!("Doodle Flappy", "Beschreibung …", env!("CARGO_PKG_VERSION"));
   ```
4. Ein `#[macroquad::main]`-Fenster schreiben. Titelbildschirm, Game-Over,
   Farbpalette und Bestwert-Speicherung liefert `doodle-core` fertig mit
   (siehe `games/doodle-snake` als Vorlage).
5. `cargo run -p doodle-launcher` – das Spiel erscheint automatisch in der
   Liste.

## Wie der Launcher Spiele findet

- Suchorte (in dieser Reihenfolge, doppelte Einträge werden entfernt):
  1. Ordner aus der Umgebungsvariable `DOODLE_GAMES_DIR` (optional)
  2. Ordner neben der Launcher-Programmdatei + dortiger `games/`-Unterordner
  3. `games/` relativ zum aktuellen Verzeichnis
- Kriterium: reguläre Programmdatei, deren Name mit `doodle-` beginnt
  (der Launcher selbst wird ausgenommen).
- Jede Datei wird einmal mit dem Argument `--doodle-meta` gestartet und
  antwortet mit einer JSON-Zeile: `{"name":…,"description":…,"version":…}`.
  Der Makro `doodle_meta!` aus `doodle-meta` erledigt das **vor** der
  Fenstereröffnung, damit kein Fenster aufblinkt.
  Findet sich ein Spiel ohne diesen Handshake, erscheint es trotzdem in der
  Liste – mit grauem „Starten“-Button und Hinweistext.

## Workspace-Struktur

```
.
├── Cargo.toml            # Workspace (alle Crates + gemeinsame Versionen)
├── doodle-meta/          # Metadaten-Handshake zwischen Launcher und Spiel
├── doodle-core/          # Palette, Titelbildschirm, Game-Over, Bestwerte, Seed
├── launcher/             # GUI des Launchers (egui/eframe)
└── games/                # Spielesammlung (je ein Crate pro Spiel)
    ├── doodle-snake/
    ├── doodle-jumper/
    ├── doodle-flappy/
    ├── doodle-pong/
    └── doodle-breakout/
```

## Tastenkürzel

- **F5** – Spieleliste im Launcher aktualisieren
- **ESC** – laufendes Spiel schließen (zurück zum Launcher)
