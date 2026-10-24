//! Core of the Old Brain Fix desktop app: a reimplementation of the Dart
//! core's signed, append-only entry log (ADR 0002) and of what the
//! screens read from it and write to it, Notes with their Sections, Areas
//! and Topics, Routines and Reminders with the alarm plan, the Nudge and
//! the Missed Summary, this Device's settings, and the question marks of
//! the background. The Dart core
//! writes the fixtures under `desktop/fixtures`; this crate must read them
//! the same and write the same entries.

pub mod alarms;
pub mod entry;
pub mod error;
pub mod notes;
pub mod nudges;
pub mod places;
pub mod question_marks;
pub mod routines;
pub mod schedule;
pub mod settings;
pub mod signing;
pub mod store;

pub use alarms::{Alarms, MissedSummary, PlannedAlarm};
pub use entry::{Entry, WireEntry, dart_iso, parse_time};
pub use error::{Error, Result};
pub use notes::{Importance, Note, Notes, Section};
pub use nudges::{PlannedNudge, QuietHours};
pub use places::{AREA_ICONS, Area, NotesGo, Places, STARTER_SET, StarterPlace, Topic};
pub use routines::{Agenda, AgendaRow, Routine, Routines};
pub use schedule::{
    ClockTime, Day, Occurrence, Recurrence, RepeatKind, RepeatUnit, Schedule, SimpleRepeat,
    WallClock,
};
pub use settings::DeviceSettings;
pub use signing::SigningKey;
pub use store::{Clock, Fields, IdMaker, Store};

/// The app's name, as the window title shows it.
pub const APP_NAME: &str = "Old Brain Fix";

/// The application id, shared with the phone.
pub const APP_ID: &str = "io.github.paxel.oldbrainfix";
