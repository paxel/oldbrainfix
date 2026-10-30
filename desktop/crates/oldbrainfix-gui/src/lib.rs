//! The Old Brain Fix desktop app: an egui shell over `oldbrainfix_core`.
//!
//! The native window and the eframe glue live in the `oldbrainfix`
//! binary; everything that can be tested headless lives here so the
//! kittest harness can drive it: the window, the alarm layer behind the
//! notifications, the tray, the login start, the one-instance rule, the
//! command line, the capture popup and its hotkey, voice recordings and
//! photos, Paste in the capture, the Transcripts and Photo Texts made of
//! them, the LAN sync it hosts, for pairing in the Sync view and for paired Devices on their
//! own, the shared folder it syncs through, the Backups it writes and
//! restores, the About view with every third-party component's
//! licence, and the view in place of the window for a database of a
//! newer version.

pub mod about_view;
pub mod ai_view;
pub mod alerts;
pub mod all_routines_view;
pub mod app;
pub mod auto_sync;
pub mod autostart;
pub mod background;
pub mod backup_view;
pub mod backups;
pub mod capture_choices;
pub mod checklist_view;
pub mod cli;
pub mod enrich;
pub mod folder_sync;
pub mod format_bar;
pub mod frogs_view;
pub mod history_view;
pub mod hotkey;
pub mod icons;
pub mod instance;
pub mod l10n;
#[cfg(target_os = "linux")]
pub mod launcher;
pub mod login;
pub mod media;
pub mod media_view;
pub mod names;
pub mod notify;
pub mod paste;
pub mod person_view;
pub mod popup;
pub mod profile_view;
pub mod ring;
pub mod routine_editor;
pub mod schedule_text;
pub mod session_editor;
pub mod session_view;
pub mod share_note;
pub mod shopping_view;
pub mod speech;
pub mod starter_set_view;
pub mod store_needs_update;
pub mod sync_view;
pub mod table_view;
pub mod tally_view;
pub mod text_marks_view;
pub mod theme;
pub mod tray;

pub use app::{App, Selection, Wall, local_wall};

use std::path::PathBuf;

/// The icon's theme name in launcher entries and notifications, backed
/// by `icons/hicolor/256x256/apps/oldbrainfix.png`.
pub const ICON_NAME: &str = "oldbrainfix";

/// The icon, 256 pixels: what the launcher entry installs and the tray
/// scales down.
pub const ICON: &[u8] = include_bytes!("../../../packaging/icon/icon_256.png");

/// Where the desktop keeps its store: the system's data directory for
/// the app, e.g. `~/.local/share/oldbrainfix` on Linux.
pub fn data_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("io.github", "paxel", "oldbrainfix")
        .map(|d| d.data_dir().to_path_buf())
}
