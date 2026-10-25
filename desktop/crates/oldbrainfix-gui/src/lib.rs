//! The Old Brain Fix desktop app: an egui shell over `oldbrainfix_core`.
//!
//! The native window and the eframe glue live in the `oldbrainfix`
//! binary; everything that can be tested headless lives here so the
//! kittest harness can drive it: the window, the alarm layer behind the
//! notifications, the tray, the login start, the one-instance rule, the
//! command line, the capture popup and its hotkey, voice recordings and
//! photos, the Transcripts and Photo Texts made of them, the LAN sync it
//! hosts, for pairing in the Sync view and for paired Devices on their
//! own, the shared folder it syncs through, the Backups it writes and
//! restores, and the About view with every third-party component's
//! licence.

pub mod about_view;
pub mod alerts;
pub mod app;
pub mod auto_sync;
pub mod autostart;
pub mod background;
pub mod backup_view;
pub mod backups;
pub mod cli;
pub mod enrich;
pub mod folder_sync;
pub mod hotkey;
pub mod icons;
pub mod instance;
pub mod l10n;
pub mod media;
pub mod media_view;
pub mod names;
pub mod notify;
pub mod popup;
pub mod routine_editor;
pub mod schedule_text;
pub mod speech;
pub mod sync_view;
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
