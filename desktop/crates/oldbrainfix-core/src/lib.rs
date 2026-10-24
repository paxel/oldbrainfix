//! Core of the Old Brain Fix desktop app: a reimplementation of the Dart
//! core's signed, append-only entry log (ADR 0002) and of what the
//! screens read from it and write to it, Notes with their Sections, Areas
//! and Topics, and the question marks of the background. The Dart core
//! writes the fixtures under `desktop/fixtures`; this crate must read them
//! the same and write the same entries.

pub mod entry;
pub mod error;
pub mod notes;
pub mod places;
pub mod question_marks;
pub mod signing;
pub mod store;

pub use entry::{Entry, WireEntry, dart_iso, parse_time};
pub use error::{Error, Result};
pub use notes::{Importance, Note, Notes, Section};
pub use places::{AREA_ICONS, Area, NotesGo, Places, STARTER_SET, StarterPlace, Topic};
pub use signing::SigningKey;
pub use store::{Clock, Fields, IdMaker, Store};

/// The app's name, as the window title shows it.
pub const APP_NAME: &str = "Old Brain Fix";

/// The application id, shared with the phone.
pub const APP_ID: &str = "io.github.paxel.oldbrainfix";
