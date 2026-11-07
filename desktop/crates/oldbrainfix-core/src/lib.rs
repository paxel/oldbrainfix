//! Core of the Old Brain Fix desktop app: a reimplementation of the Dart
//! core's signed, append-only entry log (ADR 0002) and of what the
//! screens read from it and write to it, Notes with their Sections, Areas
//! and Topics, Routines and Reminders with the alarm plan, the Nudge,
//! the Missed Summary, their history and their calendar file, Frogs with today's pick, Tallies with their entries and charts, Checklists and Shopping Lists with their Lines, this Device's settings, and the question marks of
//! the background; and of the photos and voice recordings kept beside
//! the log, with their Photo Texts and Transcripts and the search over
//! them; and the LAN sync between Devices with its pair codes, keys and
//! paired Devices, the Area keys and the shared folder and bundle files they seal, the Sent Items one person sends another, the Packs of copied Notes one hands another, the encrypted Backups with their Recovery Sheet, the user's own Profile, and the Imports of a folder of files and of a Google Keep export. The Dart core
//! writes the fixtures under `desktop/fixtures`; this crate must read them
//! the same and write the same entries.

pub mod ai_port;
pub mod ai_task;
pub mod ai_tools;
pub mod alarms;
pub mod app_link;
pub mod backup;
pub mod blobs;
pub mod blocks;
pub mod box_keys;
pub mod boxes;
pub mod bundle;
pub mod cards;
pub mod changes_nudges;
pub mod checklists;
pub mod conflicts;
pub mod discovery;
pub mod entry;
pub mod error;
pub mod folder;
pub mod frogs;
pub mod help_notes;
pub mod history;
pub mod ics;
pub mod imports;
pub mod join;
pub mod keys;
pub mod lan;
pub mod markdown;
pub mod named_places;
pub mod note_history;
pub mod note_kinds;
pub mod notes;
pub mod nudges;
pub mod offers;
pub mod overrides;
pub mod packs;
pub mod pairing;
pub mod pending;
pub mod people;
pub mod person_settings;
pub mod photos;
pub mod places;
pub mod profile;
pub mod question_marks;
pub mod recovery;
pub mod routines;
pub mod schedule;
pub mod schedule_sentence;
pub mod search;
pub mod sent;
pub mod sessions;
pub mod settings;
pub mod share;
pub mod shared;
pub mod shopping_lists;
pub mod signing;
pub mod store;
pub mod sync;
pub mod tallies;
pub mod tally_units;
pub mod text_boxes;
pub mod text_format;
pub mod text_tags;
pub mod transcript_prompts;

/// The symbol of `alphabet` for the low five bits of `value`, as the
/// pair codes, the Recovery Sheet and the Profile codes write base32.
pub(crate) fn base32_symbol(alphabet: &[u8; 32], value: u64) -> char {
    // Proof: the mask keeps the index in 0..=31, and the alphabet's type
    // holds exactly 32 symbols, so the index is always in bounds.
    #[allow(clippy::indexing_slicing)]
    let symbol = alphabet[(value & 31) as usize];
    char::from(symbol)
}

pub use alarms::{Alarms, MissedSummary, OpenOccurrence, PlannedAlarm};
pub use boxes::Boxes;
pub use cards::{CardOpened, CardTaken};
pub use checklists::{
    Checklist, ChecklistAgendaRow, Checklists, Line, LineChecks, RoutineOccurrence,
};
pub use conflicts::{Conflict, ConflictVersion};
pub use entry::{Entry, WireEntry, dart_iso, parse_time};
pub use error::{Error, Result};
pub use frogs::{Frog, Frogs, FrogsToday};
pub use help_notes::{HelpNotes, HelpTexts, WhatsNew, compare_versions, parse_whats_new};
pub use history::{
    Adherence, DayTally, DoneTiming, HistoryEntry, OccurrenceStatus, RoutineHistory, ScheduleSpan,
};
pub use imports::{ImportFile, ImportSummary, ImportedLine, ImportedNote, Imports};
pub use named_places::{Coordinates, NamedPlace, NamedPlaces};
pub use note_history::{NoteChange, NoteChangeKind, NoteHistory};
pub use notes::{
    Blob, CaptureContext, CapturedDetail, Importance, Note, Notes, Section, SourceApp, Wordless,
};
pub use nudges::{PlannedChangesNudge, PlannedNudge, QuietHours};
pub use overrides::PersonOverride;
pub use packs::{
    OpenedPack, Pack, PackAdded, PackError, PackFrom, PackItem, PackItemKind, PackNote, PackTicks,
};
pub use pairing::PairedDevice;
pub use pending::{
    AcceptedChanges, KeptEdit, PendingChange, PendingDraft, PendingEntry, PendingSet,
};
pub use people::{PersonFilter, PersonTaken};
pub use person_settings::PersonSettings;
pub use places::{AREA_ICONS, Area, NotesGo, Places, STARTER_SET, StarterPlace, Topic};
pub use profile::{OwnProfile, Profile, ProfileChoice, SentProfile};
pub use routines::{Agenda, AgendaRow, NewerRoutine, Routine, Routines};
pub use schedule::{
    ClockTime, Day, Occurrence, Recurrence, RepeatEnd, RepeatKind, RepeatUnit, Schedule,
    SimpleRepeat, WallClock,
};
pub use search::Search;
pub use sessions::{
    Session, SessionMarks, SessionOccurrence, SessionPick, SessionRule, SessionSetup, Sessions,
};
pub use settings::{DeviceSettings, QuickNoteControl, ThemeChoice};
pub use shared::{AreaAccess, AreaRole, Grant, Invite, Person};
pub use shopping_lists::{
    ListOnAgenda, Reappear, ReappearUnit, ShoppingAgenda, ShoppingAgendaRow, ShoppingLine,
    ShoppingList, ShoppingLists, ShoppingOnAgenda,
};
pub use signing::{ImportReport, KeyRecord, KeyTrust, PinnedKey, SigningKey};
pub use store::{Clock, Fields, IdMaker, MomentRow, RefusalMark, RefusedRun, STORE_VERSION, Store};
pub use sync::Moment;
pub use tallies::{Tallies, Tally, TallyBucket, TallyEntry, TallyField, TallyPeriod};
pub use tally_units::{TallyRule, UnitKind, UnitSystem};
pub use text_boxes::{BoxState, BoxTick, FoundBox, ShownBox, ShownText};
pub use text_format::StoredText;
pub use text_tags::FoundTag;
pub use transcript_prompts::TranscriptPrompts;

/// The app's name, as the window title shows it.
pub const APP_NAME: &str = "Old Brain Fix";

/// The application id, shared with the phone.
pub const APP_ID: &str = "io.github.paxel.oldbrainfix";
