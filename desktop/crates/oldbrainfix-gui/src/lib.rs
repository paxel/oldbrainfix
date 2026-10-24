//! The Old Brain Fix desktop app: an egui shell over `oldbrainfix_core`.
//!
//! The native window and the eframe glue live in the `oldbrainfix`
//! binary; everything that can be tested headless lives here so the
//! kittest harness can drive it.

pub mod app;
pub mod background;
pub mod icons;
pub mod l10n;
pub mod names;
pub mod theme;

pub use app::{App, Selection};

use std::path::PathBuf;

/// Where the desktop keeps its store: the system's data directory for
/// the app, e.g. `~/.local/share/oldbrainfix` on Linux.
pub fn data_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("io.github", "paxel", "oldbrainfix")
        .map(|d| d.data_dir().to_path_buf())
}
