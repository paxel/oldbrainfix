//! Core of the Old Brain Fix desktop app: a reimplementation of the Dart
//! core's signed, append-only entry log (ADR 0002) and of what the
//! screens read from it and write to it, Notes with their Sections, Areas
//! and Topics, Routines and Reminders with the alarm plan, the Nudge and
//! the Missed Summary, this Device's settings, and the question marks of
//! the background; and of the photos and voice recordings kept beside
//! the log, with their Photo Texts and Transcripts and the search over
//! them; and the LAN sync between Devices with its pair codes, keys and
//! paired Devices, the Area keys and the shared folder and bundle files they seal, the Sent Items one person sends another, the encrypted Backups with their Recovery Sheet, and the user's own Profile. The Dart core
//! writes the fixtures under `desktop/fixtures`; this crate must read them
//! the same and write the same entries.

pub mod alarms;
pub mod backup;
pub mod blobs;
pub mod box_keys;
pub mod bundle;
pub mod conflicts;
pub mod discovery;
pub mod entry;
pub mod error;
pub mod folder;
pub mod keys;
pub mod lan;
pub mod notes;
pub mod nudges;
pub mod overrides;
pub mod pairing;
pub mod people;
pub mod photos;
pub mod places;
pub mod profile;
pub mod question_marks;
pub mod recovery;
pub mod routines;
pub mod schedule;
pub mod search;
pub mod sent;
pub mod settings;
pub mod shared;
pub mod signing;
pub mod store;
pub mod sync;
pub mod text_format;
pub mod transcript_prompts;

pub use alarms::{Alarms, MissedSummary, PlannedAlarm};
pub use conflicts::{Conflict, ConflictVersion};
pub use entry::{Entry, WireEntry, dart_iso, parse_time};
pub use error::{Error, Result};
pub use notes::{Blob, Importance, Note, Notes, Section};
pub use nudges::{PlannedNudge, QuietHours};
pub use overrides::PersonOverride;
pub use pairing::PairedDevice;
pub use people::{PersonFilter, PersonTaken};
pub use places::{AREA_ICONS, Area, NotesGo, Places, STARTER_SET, StarterPlace, Topic};
pub use profile::{OwnProfile, Profile, ProfileChoice, SentProfile};
pub use routines::{Agenda, AgendaRow, Routine, Routines};
pub use schedule::{
    ClockTime, Day, Occurrence, Recurrence, RepeatEnd, RepeatKind, RepeatUnit, Schedule,
    SimpleRepeat, WallClock,
};
pub use search::Search;
pub use settings::DeviceSettings;
pub use shared::{AreaAccess, AreaRole, Grant, Invite, Person};
pub use signing::{ImportReport, KeyRecord, KeyTrust, PinnedKey, SigningKey};
pub use store::{Clock, Fields, IdMaker, MomentRow, Store};
pub use sync::Moment;
pub use text_format::StoredText;
pub use transcript_prompts::TranscriptPrompts;

/// The app's name, as the window title shows it.
pub const APP_NAME: &str = "Old Brain Fix";

/// The application id, shared with the phone.
pub const APP_ID: &str = "io.github.paxel.oldbrainfix";
