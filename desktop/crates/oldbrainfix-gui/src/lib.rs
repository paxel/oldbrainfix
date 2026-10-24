//! The Old Brain Fix desktop app: an egui shell over `oldbrainfix_core`.
//!
//! The native window and the eframe glue live in the `oldbrainfix`
//! binary; everything that can be tested headless lives here so the
//! kittest harness can drive it: the window, the alarm layer behind the
//! notifications, the tray, the login start and the one-instance rule.

pub mod alerts;
pub mod app;
pub mod autostart;
pub mod background;
pub mod icons;
pub mod instance;
pub mod l10n;
pub mod names;
pub mod notify;
pub mod routine_editor;
pub mod schedule_text;
pub mod theme;
pub mod tray;

pub use app::{App, Selection, Wall, local_wall};

use std::path::PathBuf;

/// Where the desktop keeps its store: the system's data directory for
/// the app, e.g. `~/.local/share/oldbrainfix` on Linux.
pub fn data_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("io.github", "paxel", "oldbrainfix")
        .map(|d| d.data_dir().to_path_buf())
}
