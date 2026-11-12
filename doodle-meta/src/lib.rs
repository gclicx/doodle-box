//! Metadaten-Handshake zwischen dem Launcher und den Spielen.
//!
//! Der Launcher startet jede `doodle-*`-Programmdatei einmal mit dem Argument
//! `--doodle-meta`. Spiele antworten mit einer JSON-Zeile und beenden sich
//! sofort – noch **bevor** das Fenster geöffnet wird (der [`doodle_meta!`]-Makro
//! läuft über `ctor` vor `main` und damit vor Macroquad).
//!
//! Ein Spiel binde das Makro deshalb genau einmal ganz oben in `main.rs` ein:
//!
//! ```ignore
//! doodle_meta!("Doodle Jumper", "Springe überall hin", env!("CARGO_PKG_VERSION"));
//! ```

use serde::{Deserialize, Serialize};

/// Argument, mit dem der Launcher Metadaten von einer Spieledatei abfragt.
pub const META_ARG: &str = "--doodle-meta";

/// Metadaten eines Spiels.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GameMeta {
    /// Anzeigename im Launcher.
    pub name: String,
    /// Kurzbeschreibung im Launcher.
    pub description: String,
    /// Versionsangabe.
    pub version: String,
}

/// Erzeugt die JSON-Zeile für den Metadaten-Handshake.
pub fn meta_json(name: &str, description: &str, version: &str) -> String {
    serde_json::to_string(&GameMeta {
        name: name.to_string(),
        description: description.to_string(),
        version: version.to_string(),
    })
    // Fallback: immer gültiges JSON ausgeben.
    .unwrap_or_else(|_| {
        format!(
            "{{\"name\":\"{}\",\"description\":\"\",\"version\":\"\"}}",
            name.replace('\\', "\\\\").replace('"', "\\\"")
        )
    })
}

/// Wiederveröffentlichtes `ctor`-Attribut, damit Spiel-Crates keine direkte
/// `ctor`-Abhängigkeit brauchen (das Makro oben greift über `$crate::ctor` darauf zu).
pub use ctor::ctor;

/// Registriert den Metadaten-Handshake für dieses Spiel.
///
/// Muss genau einmal pro Spiel-Crate verwendet werden (erzeugt eine
/// vor-`main`-Funktion mit eindeutigem Namen). Wird das Programm mit
/// `--doodle-meta` gestartet, antwortet es mit JSON und beendet sich.
#[macro_export]
macro_rules! doodle_meta {
    ($name:expr, $description:expr, $version:expr) => {
        #[$crate::ctor]
        fn doodle_meta_boot() {
            if std::env::args().nth(1).as_deref() == Some($crate::META_ARG) {
                println!("{}", $crate::meta_json($name, $description, $version));
                std::process::exit(0);
            }
        }
    };
}
