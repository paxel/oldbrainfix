//! The window (UI laws, "Desktop"): the Agenda, Areas and Topics on the
//! left, the capture field and the Agenda or the Notes of the selection
//! in the middle as embossed cards, the selected Note's editor or a
//! Routine's editor on the right.
//!
//! The window opens on the Agenda (UI laws, "Agenda"): the Missed
//! Summary until "Seen", the "x Unsorted" banner, the "x Sent Items
//! waiting" line while a stranger's Sent Item waits for "Show", the "x
//! shared Areas waiting" line while an Area shared from afar waits for
//! Accept or Decline, today's
//! all-day and then timed Occurrences, a gap, and tomorrow's dimmed,
//! then the five days after it under their weekdays and dates, all of them checkable. Its gear leads to this Device's Settings, its alarm button
//! to a new Routine. A Note spawns Routines and Reminders from its
//! editor; deleting a Note that spawned some asks whether they go too.
//!
//! Every change goes through `oldbrainfix_core`, which writes the entries
//! the phone's core writes. A card dragged onto an Area or Topic on the
//! left is filed there. Keys replace swipes (UI laws, "Navigation"):
//! Alt+Left or the mouse's back button goes back one level, a Topic to
//! its Area and an Area, Unsorted or the trash to All Notes; Alt+Right or
//! the mouse's forward button goes forward into the last selected Area,
//! and from an Area into its last selected Topic. The keys rest while
//! text is being typed, where Alt+arrows move the cursor.
//!
//! The capture field records voice too: the microphone's button starts
//! at once and one big stop button ends it and leaves it on the field,
//! saved with the Note by the Nudge times or Save (UI laws, "Capture").
//! Once something is typed, the Note card's buttons show above them;
//! what they set up shows at the top of the field and is saved with the
//! Note, all of it or nothing (#113). A picture pasted from the clipboard
//! becomes a photo Note, with what is typed; pictures and sound files
//! chosen with "From file…" or dropped onto the window (where the system
//! hands drops over: not on Wayland) become one Note with their photos
//! and voice recordings, as a share does on the phone, and a sound file
//! that cannot be transcribed here is kept and announced; a sync bundle
//! dropped there is imported, what came in announced at the top; a
//! Sent Item dropped there, opened or handed over by `oldbrainfix open`
//! shows its preview to accept or decline, a stranger's hidden until
//! "Show" and waiting behind the Agenda's "x Sent Items waiting" until
//! decided. A bundle offering an Area shared from afar (#108) shows who
//! shares which Area as what, to accept or decline; an Area is shared so
//! from its Share dialog with a person in People, the offer saved where
//! the user says, and later changes go from that person's Profile as a
//! bundle when they travel as files.
//! A Note is sent from its editor, saved where the user says.
//! The editor shows a Note's photos and plays its recordings, with their
//! Transcripts. Transcripts and Photo Texts are made in the background
//! and found by the search over All Notes, an Area or a Topic.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::Sender;

use chrono::{DateTime, Duration, Local, NaiveDateTime, Utc};
use egui::containers::menu::{MenuButton, MenuConfig};
use egui::{
    Color32, FontId, Frame, Id, Key, Margin, Modifiers, PointerButton, PopupCloseBehavior,
    RichText, Sense, TextEdit, Ui, WidgetInfo, WidgetType,
};
use oldbrainfix_core::ai_port::AiPort;
use oldbrainfix_core::backup;
use oldbrainfix_core::offers::{self, HeldAreaOffer, Transport};
use oldbrainfix_core::profile::ProfileChoice;
use oldbrainfix_core::sent::{
    self, HeldSentItem, OpenError, Opened, Received, Recipient, SenderTrust, SentItemState,
};
use oldbrainfix_core::{
    AREA_ICONS, AgendaRow, Alarms, Area, AreaAccess, AreaRole, Blob, CaptureContext, CardTaken,
    ClockTime, Coordinates, Day, DeviceSettings, Frog, Frogs, FrogsToday, Importance, Invite,
    MissedSummary, Note, Notes, NotesGo, Occurrence, OwnProfile, Person, PersonFilter, Places,
    Profile, QuietHours, Routine, Routines, Search, Section, Sessions, Store, Tallies, Topic,
    UnitSystem, bundle,
};

use crate::about_view;
use crate::ai_view::{self, AiAction, Client, ReviewShown};
use crate::alerts::Event;
use crate::autostart::Autostart;
use crate::background;
use crate::backup_view::{self, BackupAction, Passphrase, Welcome, WelcomeAction};
use crate::backups;
use crate::capture_choices::{self, Ask, CaptureChoices, Opens};
use crate::enrich::Enricher;
use crate::folder_sync;
use crate::frogs_view::{self, FrogAction, FrogShown};
use crate::hotkey::{self, Chord};
use crate::icons;
use crate::l10n::{self, L10n};
use crate::login;
use crate::media::{
    Clipboard, FromFile, PICTURE_EXTENSIONS, Player, Recorder, SOUND_EXTENSIONS, from_file,
    photo_from_file,
};
use crate::media_view::{
    self, Generated, Offers, Photo, PhotoCache, Recording, Speech, Viewer, Voice,
};
use crate::names::{area_name, offered_area_name, topic_name};
use crate::person_view::{self, Comparing, PersonAction, PersonView, SendBack};
use crate::profile_view::{self, CardForm, ProfileAction, ProfileView, Square};
use crate::ring::{self, Rung};
use crate::routine_editor::{Intent, RENAG_CHOICES, RoutineEditor};
use crate::schedule_text::{agenda_day_text, day_text, occurrence_time, schedule_text};
use crate::session_editor::{SessionDraft, SessionIntent, WEEKDAYS_OPEN};
use crate::session_view::{self, PicksShown, SessionAction, SessionPart};
use crate::speech::SpeechModel;
use crate::sync_view::{
    Joiner, SyncAction, SyncHost, bundle_result_text, device_name, import_bundle_result,
    offer_refusal, update_notice_text,
};
use crate::tally_view::{
    self, EntryForm, FieldForm, FormIntent, TallyAction, TallyForm, TallyLook, TallyShown,
};
use crate::theme::{self, AREA_PALETTE, Tones};

/// What the middle pane shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection {
    /// The Occurrences of today and the six days after, the Missed
    /// Summary and the Unsorted count.
    Agenda,
    /// The user's own Profile with Backups, the user's Devices and the
    /// way to this Device's settings, reached from the Profile picture in
    /// the Agenda's corner, where the settings gear was.
    Profile,
    /// This Device's settings, reached from the Profile page.
    Settings,
    /// The LAN sync this Device hosts while it shows, reached from the
    /// Profile page.
    Sync,
    /// The version and the source, reached from the Settings.
    About,
    /// Every third-party component and its licence, reached from About.
    Licences,
    /// The strangers' Sent Items waiting for "Show", Accept or Decline
    /// (#91), reached from the Agenda's "x Sent Items waiting" line.
    SentItemsWaiting,
    /// The Areas shared from afar waiting for Accept or Decline (#108),
    /// reached from the Agenda's "x shared Areas waiting" line.
    AreaOffersWaiting,
    /// The review of the AI Task (#112): its changes by kind, Accept all
    /// and Reject all; reached by "Done, review" in the Settings, and on
    /// start while one waits.
    AiReview,
    /// The Frog view (#101): the open Frogs, oldest first, and the
    /// folded "Done" section; reached from the Agenda's "Eat a frog".
    Frogs,
    /// Every Note neither in the trash nor merged.
    All,
    Unsorted,
    /// The Area or Topic with this id; an Area lists its Notes without a
    /// Topic, and its search also finds its Topics' Notes.
    Place(String),
    Trash,
}

/// What a card carries while it is dragged: the Note's id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraggedNote(pub String);

/// A Note with the Area and Topic it is filed in, and the Profile of the
/// other person who wrote it; none for the user's own.
#[derive(Debug, Clone)]
struct Placed {
    note: Note,
    area: Option<Area>,
    topic: Option<Topic>,
    author: Option<Profile>,
}

/// One Agenda row with the Area its Note is filed in.
#[derive(Debug, Clone)]
struct Row {
    row: AgendaRow,
    area: Option<Area>,
    /// For a Session what shows under its title.
    session: Option<SessionPart>,
}

/// What the Agenda shows.
#[derive(Debug, Clone, Default)]
struct AgendaView {
    today: Vec<Row>,
    tomorrow: Vec<Row>,
    /// The days after tomorrow, each with its rows.
    later: Vec<(Day, Vec<Row>)>,
    missed: MissedSummary,
    unsorted: usize,
    /// How many strangers' Sent Items wait (#91), counted from the
    /// pending sets without reading their files.
    waiting: usize,
    /// How many Areas shared from afar wait (#108), counted alike.
    offers: usize,
    /// Today's "Eat a frog" section (ADR 0027), and its Frogs as they
    /// show.
    frogs: Option<FrogsToday>,
    frog_rows: Vec<FrogShown>,
}

/// What this Device may do to the selected Note in a Shared Area
/// (ADR 0013): change it (edit, file, merge, delete), give it Routines,
/// and Amend it.
#[derive(Debug, Clone, Copy)]
struct NoteMay {
    changes: bool,
    schedules: bool,
    amends: bool,
}

impl Default for NoteMay {
    fn default() -> NoteMay {
        NoteMay {
            changes: true,
            schedules: true,
            amends: true,
        }
    }
}

/// What the panes show, read once per change.
#[derive(Debug, Clone, Default)]
struct View {
    areas: Vec<(Area, Vec<Topic>)>,
    /// What this Device may do in each Area, and who shares the shared
    /// ones.
    access: std::collections::HashMap<String, AreaAccess>,
    people: std::collections::HashMap<String, Vec<Person>>,
    /// The name each person's Profile shows, by Profile id.
    names: std::collections::HashMap<String, String>,
    /// Each person's Profile as it shows, Override applied, by Profile id.
    shown: std::collections::HashMap<String, Profile>,
    /// People (#52): every Profile kept here, as the user sees them,
    /// but the blocked ones, listed apart (#90), and whether their
    /// folded section is open.
    people_shown: Vec<Profile>,
    blocked_people: Vec<Profile>,
    blocked_open: bool,
    /// The person whose Profile the dialog shows.
    person: Option<PersonView>,
    /// The person this Device belongs to in each Shared Area.
    mine: std::collections::HashMap<String, String>,
    /// What this Device may do to the selected Note.
    may: NoteMay,
    /// The photos and voice recordings of the selected Note and its
    /// Sections this Device may remove while their texts stay
    /// (ADR 0020), by Note id and hash.
    removable: HashSet<(String, String)>,
    /// Those, as for `removable`, whose Transcripts and Photo Texts this
    /// Device may remove while they stay (ADR 0025).
    text_removable: HashSet<(String, String)>,
    /// Those, as for `removable`, whose texts this Device may make
    /// again: their files are here and the Note is its to change.
    remakeable: HashSet<(String, String)>,
    /// The selected Note and its Sections whose text this Device may
    /// change, by id.
    text_changeable: HashSet<String>,
    notes: Vec<Placed>,
    /// The Sections of each Note listed, oldest first, read in one pass
    /// for the full cards (#114).
    sections: BTreeMap<String, Vec<Section>>,
    /// Whether the list shows its Notes compact, by their first line,
    /// as this Device keeps it for that list (#114).
    compact: bool,
    /// The selected Note with its Sections.
    detail: Option<(Placed, Vec<Section>)>,
    /// Whether the Note being captured may spawn where it goes (#113).
    capture_spawns: bool,
    /// The Routines and Reminders the selected Note spawned.
    routines: Vec<Routine>,
    /// The Frogs the selected Note spawned, oldest first, each with
    /// whether this Device may check it off.
    note_frogs: Vec<(Frog, bool)>,
    /// The Tally of the selected Note, while it is one (#103).
    tally: Option<TallyShown>,
    /// The Frog view's open and Done Frogs, while it shows, and whether
    /// its "Done" section is open.
    frogs_open: Vec<FrogShown>,
    frogs_done: Vec<FrogShown>,
    frogs_done_open: bool,
    /// While the Agenda shows.
    agenda: Option<AgendaView>,
    /// The strangers' Sent Items waiting, while their list shows.
    held: Vec<HeldSentItem>,
    /// The Areas shared from afar waiting, while their list shows.
    offers: Vec<HeldAreaOffer>,
    /// The Routines and Reminders in the trash on their own, with their
    /// Notes, while the trash shows.
    trashed_routines: Vec<(Routine, Option<Note>)>,
    /// The Frogs in the trash, with their Notes and whether this Device
    /// may restore them, while the trash shows.
    trashed_frogs: Vec<(Frog, Option<Note>, bool)>,
    /// The user's own Profile and its short code.
    profile: Option<Profile>,
    profile_code: Option<String>,
    /// The other Profile "Which one is you?" asks about, if it does.
    profile_question: Option<ProfileChoice>,
    /// Whether sync waits for an AI Task (ADR 0026), and the AI Task
    /// open or waiting for its review, by id.
    sync_paused: bool,
    ai_task: Option<String>,
    /// The AI Task's review, while it shows.
    ai_review: Option<ReviewShown>,
}

impl View {
    fn area(&self, id: &str) -> Option<&Area> {
        self.areas.iter().map(|(a, _)| a).find(|a| a.id == id)
    }

    fn topic(&self, id: &str) -> Option<&Topic> {
        self.areas
            .iter()
            .flat_map(|(_, topics)| topics)
            .find(|t| t.id == id)
    }
}

/// The last selected Area and Topic, where forward leads, as the phone's
/// `BrowserTrail` keeps them.
#[derive(Debug, Clone, Default)]
struct Trail {
    area: Option<String>,
    topic: Option<String>,
}

/// A dialog over the window.
#[derive(Debug, Clone, PartialEq)]
enum Dialog {
    /// Naming a new Area, or a new Topic of the Area `area`.
    NewPlace {
        area: Option<String>,
        name: String,
    },
    /// Renaming the Area or Topic `id`, shown as `shown`.
    Rename {
        id: String,
        name: String,
        shown: String,
    },
    /// The colour and icon of an Area.
    Look {
        area: String,
    },
    Delete(Deleting),
    /// Picking the Note to merge the selected one into, by id and text.
    Merge {
        query: String,
        others: Vec<(String, String)>,
    },
    /// Whether the selected Note goes to the trash, and with it its
    /// `linked` Routines and Reminders.
    TrashNote {
        linked: i64,
    },
    /// Whether the edited Routine or Reminder goes to the trash.
    DeleteRoutine {
        question: String,
    },
    /// Whether the Frog `id` goes to the trash; its Note stays.
    DeleteFrog {
        id: String,
    },
    /// Whether the Tally's entry `id` goes; it can be restored.
    DeleteEntry {
        id: String,
    },
    /// Sharing the Area `area`: the role the other person gets, and the
    /// People it can go to from afar (#108).
    Share {
        area: String,
        role: AreaRole,
        afar: Vec<AfarPerson>,
    },
    /// Sharing the Area `area` from afar as `role` (#108): whom, picked
    /// from `people`, then, for a person never asked about, how changes
    /// travel with them.
    ShareAfar {
        area: String,
        role: AreaRole,
        people: Vec<AfarPerson>,
        person: Option<String>,
        transport: Transport,
    },
    /// An Area shared with the user from afar (#108, ADR 0023), held
    /// until Accept or Decline, with what comparing its sharer's short
    /// code came to and, while "Decline and block" asks, the Shared Areas
    /// blocking them leaves (#90).
    AreaOffer {
        held: HeldAreaOffer,
        compared: Compared,
        blocking: Option<Vec<String>>,
    },
    /// "Choose the shared folder?" after the Area `area` was accepted
    /// with its changes travelling by a folder, while none is chosen.
    OfferFolder {
        area: String,
    },
    /// Sending the Note `note`: to one of `recipients`, encrypted, or to
    /// anyone (`None`), signed only. One sent before (`update`) goes as
    /// an update, or as its cancellation with `cancel`.
    Send {
        note: String,
        recipients: Vec<Recipient>,
        picked: Option<usize>,
        update: bool,
        cancel: bool,
    },
    /// The preview of a Sent Item opened here, with what comparing its
    /// sender's short code came to; whether its sender may be blocked,
    /// not the user's own Profile, and, while "Decline and block" asks,
    /// the Shared Areas blocking them leaves (#90). A stranger's item
    /// shown with "Show" names the pending set it is held in (#91):
    /// Accept, Decline and Close decide that one.
    SentItem {
        opened: Box<Opened>,
        compared: Compared,
        may_block: bool,
        blocking: Option<Vec<String>>,
        held: Option<String>,
    },
    /// A stranger's Sent Item held until "Show" (#91): only who it is
    /// from, and, while "Decline and block" asks, the Shared Areas
    /// blocking them leaves.
    HeldSentItem {
        held: HeldSentItem,
        blocking: Option<Vec<String>>,
    },
    /// The people of the Shared Area `area`.
    People {
        area: String,
    },
    /// Whether `person` goes from the Shared Area `area`.
    RemovePerson {
        area: String,
        person: String,
        question: String,
    },
    /// Whether the photo (`photo`) or voice recording `hash` of the Note
    /// `note` goes while its text stays (ADR 0020).
    RemoveMedia {
        note: String,
        hash: String,
        photo: bool,
    },
    /// Whether the Photo Text (`photo`) or Transcript of `hash` of the
    /// Note `note` goes while its file stays (ADR 0025).
    RemoveText {
        note: String,
        hash: String,
        photo: bool,
    },
    /// What a failure announced at the top was (UI laws,
    /// "Announcements"): its cause and fix, one sentence each, and the
    /// full text, with Close and Report.
    Failure {
        title: String,
        cause: String,
        fix: String,
        details: String,
    },
    /// "Which one is you?" (ADR 0017): two Profiles of the user's own
    /// Devices, both changed by the user, met.
    WhichProfile,
    /// Another person's Profile (#52), with where comparing their code
    /// stands, the name for the user being typed, whether blocking
    /// them or ending their block is being asked (#90), and where "Send
    /// yours back" stands (#107).
    Person {
        id: String,
        comparing: Comparing,
        name: String,
        asking: Option<Asking>,
        back: SendBack,
    },
    /// The own Profile card as a QR (#107).
    ProfileQr {
        link: String,
    },
    /// Whether every change of the AI Task is rejected (#112).
    RejectAi,
}

/// A person in People an Area can go to from afar (#108): with a Device
/// whose key is known here, not blocked; as the user sees them, and
/// whether their code was compared in person.
#[derive(Debug, Clone, PartialEq)]
struct AfarPerson {
    id: String,
    shown: Profile,
    verified: bool,
}

/// What a person's Profile asks before it blocks them or ends their
/// block (#90, ADR 0021).
#[derive(Debug, Clone, PartialEq)]
enum Asking {
    /// Whether to block them, leaving these Shared Areas they manage.
    Block(Vec<String>),
    Unblock,
}

/// Where comparing the short code of a Sent Item's sender stands
/// (ADR 0017, #51): a match marks their Profile verified on accept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compared {
    Not,
    Asking,
    Match,
    Differ,
}

/// The question before an Area or Topic is deleted.
#[derive(Debug, Clone, PartialEq)]
struct Deleting {
    id: String,
    question: String,
    lines: Vec<String>,
    /// The shown name of the Area a deleted Topic's Notes can go into;
    /// `None` for an Area.
    area: Option<String>,
    has_notes: bool,
}

/// What the user asked for in a frame, done after it is drawn.
#[derive(Debug, Clone, PartialEq)]
enum Action {
    Select(Selection),
    /// Opens the Note's editor on the right: a double-click on its card
    /// or Edit (#114).
    SelectNote(String),
    /// Shows the compact card of this Note in full, in place (#114).
    Unfold(String),
    /// Switches the list shown to compact cards or full ones and keeps
    /// the choice for that list on this Device (#114).
    Compact(bool),
    /// Shows this photo full screen, to zoom and move (#114).
    ViewPhoto(String),
    /// Closes the photo shown full screen.
    ClosePhoto,
    /// Captures the capture field's text and the recording running,
    /// nudged this long from now.
    Capture(Duration),
    /// Something asked of the capture's buttons or its choices (#113).
    CaptureChoice(Ask),
    /// Starts recording voice into the Note being captured.
    Record,
    /// Ends the recording and holds it on the capture field.
    StopRecording,
    /// Plays the voice recording with this hash, or stops it playing.
    Play(String),
    /// Plays recordings at this speed from now on, the one playing
    /// included, and keeps it on this Device.
    Speed(f64),
    /// Keeps the picture on the clipboard as a photo Note.
    PastePhoto,
    /// Asks the system's dialog for pictures and sound files and makes
    /// them into a Note.
    PickFiles,
    /// Makes these files, by name and content, into one Note with what
    /// is typed: the pictures as photos, the sound files as voice
    /// recordings; anything else is left alone.
    Files(Vec<(String, Vec<u8>)>),
    /// Shows why the recordings announced could not be transcribed.
    ShowUntranscribable,
    /// The search over the Notes shown changed.
    Search,
    /// Shows only the Notes of these people (#53).
    FilterPeople(PersonFilter),
    Speech(Speech),
    /// Keeps the selected Note's edited text.
    SaveText,
    Amend,
    Importance(Importance),
    File {
        note: String,
        place: String,
    },
    Trash,
    Restore,
    /// Merges the selected Note into this one.
    Merge(String),
    NewArea,
    NewTopic(String),
    Rename(String),
    Look(String),
    Delete(String),
    StartMerge,
    /// The merge dialog's search changed.
    MergeQuery,
    CloseDialog,
    AddPlace,
    SaveName,
    Colour(String, Option<String>),
    Icon(String, Option<String>),
    DeletePlace(NotesGo),
    Back,
    Forward,
    /// Checks the Occurrence of the Routine off, or unchecks it.
    Toggle(String, Occurrence),
    /// Checks off a Missed Occurrence: Done, late.
    CheckMissed(String, Occurrence),
    /// "Seen" on the Missed Summary.
    Seen,
    OpenRoutine(String),
    /// A new Routine, or Reminder, from the Note, or with its Note made
    /// behind the scenes.
    NewRoutine {
        note: Option<String>,
        reminder: bool,
    },
    /// A new Session from the Note with this id (#104).
    NewSession(String),
    /// Something asked of a Session's row on the Agenda.
    Session(SessionAction),
    Editor(Intent),
    /// Deletes the edited Routine, asked.
    DeleteRoutine,
    /// Deletes the selected Note, with or without its Routines, asked.
    TrashNote(bool),
    RestoreRoutine(String),
    /// Something asked of a Frog or the Frog view (#101).
    Frog(FrogAction),
    /// Spawns a Frog from the Note with this id.
    SpawnFrog(String),
    /// Moves the Frog the dialog asks about to the trash.
    DeleteFrog,
    RestoreFrog(String),
    /// Something asked of the Tally part of the Note's editor (#103).
    Tally(TallyAction),
    /// Something asked of the field page or the entry page.
    TallyForm(FormIntent),
    /// Removes the entry the dialog asks about.
    DeleteEntry,
    Setting(Setting),
    /// Asks the system's dialog for the sound file Occurrences ring with.
    PickAlarmSound,
    Sync(SyncAction),
    /// Imports the bundle file dropped onto the window or handed over,
    /// or shows the preview of the Sent Item.
    OpenFile(std::path::PathBuf),
    /// Saves a bundle for each person the Changes Nudge covers, one
    /// after another, as its click asked (#109).
    SendWaitingChanges,
    /// Asks whom the selected Note is sent to.
    StartSend,
    /// Saves the Note of the send dialog as a Sent Item.
    SendNote,
    /// Accepts the Sent Item previewed, or declines it.
    AcceptSent,
    DeclineSent,
    /// Shows what the stranger's Sent Item held in the dialog holds.
    ShowSent,
    /// Opens the stranger's Sent Item held as this pending set, hidden.
    OpenHeld(String),
    /// Asks whether to decline the Sent Item previewed and block its
    /// sender (#90).
    AskBlockSender,
    /// Blocks the person the dialog asks about, declining the Sent Item
    /// previewed; ends the block of the person whose Profile asks; or
    /// takes the question back.
    Block,
    Unblock,
    CancelBlock,
    /// Keeps whether the "Blocked" section of People is open.
    BlockedOpen(bool),
    /// Asks to compare the sender's short code in person, and what that
    /// came to: whether the codes match.
    CompareCodes,
    CodesCompared(bool),
    /// Asks how the Area is to be shared.
    Share(String),
    /// Asks whom the Area of the Share dialog goes to from afar (#108).
    ShareAfar,
    /// The person the Area goes to from afar was picked.
    ShareTo(String),
    /// Shares the Area from afar as the dialog says and saves the offer.
    ShareAfarNow,
    /// Opens the Area shared from afar held as this pending set.
    OpenOffer(String),
    /// Accepts the Area shared from afar the dialog shows, or declines
    /// it.
    AcceptOffer,
    DeclineOffer,
    /// Asks for the shared folder after an Area was accepted.
    OfferFolder,
    /// Shares the Area of the Share dialog and hosts its invite.
    StartInvite,
    /// Shows who shares the Area.
    People(String),
    SetRole {
        area: String,
        person: String,
        role: AreaRole,
    },
    /// Asks whether the person goes from the Area.
    AskRemovePerson {
        area: String,
        person: String,
    },
    /// Takes the person asked about out of the Area.
    RemovePerson,
    /// Asks whether the photo (`photo`) or voice recording `hash` of the
    /// Note `note`, the selected one or one merged into it, goes while
    /// its text stays.
    AskRemoveMedia {
        note: String,
        hash: String,
        photo: bool,
    },
    /// Removes the photo or recording asked about and keeps its text.
    RemoveMedia,
    /// Makes the Photo Text (`photo`) or Transcript of `hash` the text of
    /// the Note `note`, the selected one or one merged into it, or adds
    /// it on a line of its own after that text (`append`) (ADR 0025).
    TakeOver {
        note: String,
        hash: String,
        photo: bool,
        append: bool,
    },
    /// Asks whether the Photo Text (`photo`) or Transcript of `hash` of
    /// the Note `note` goes while its file stays.
    AskRemoveText {
        note: String,
        hash: String,
        photo: bool,
    },
    /// Removes the Photo Text or Transcript asked about.
    RemoveText,
    /// Reads the photo (`photo`) or transcribes the recording `hash` of
    /// the Note `note` again.
    Again {
        note: String,
        hash: String,
        photo: bool,
    },
    /// Shows why reading a photo (`true`) or transcribing a recording
    /// again brought no text.
    ShowAgainFailed(bool),
    /// Something asked of the Backups card on the Profile page.
    Backup(BackupAction),
    /// Something asked of the own Profile on the Profile page.
    Profile(ProfileAction),
    /// Opens another person's Profile (#52).
    OpenPerson(String),
    /// Something asked of the person's Profile the dialog shows.
    Person(PersonAction),
    /// Something asked of the first start's view.
    Welcome(WelcomeAction),
    /// Something asked of the AI port's card or the AI Task's review.
    Ai(AiAction),
}

/// A change in the Settings.
#[derive(Debug, Clone, PartialEq)]
enum Setting {
    Alerts(bool),
    Autostart(bool),
    Renag(i64),
    NudgeRepeat(i64),
    QuietOn(bool),
    Quiet(QuietHours),
    /// The capture popup's key chord, as the settings show it.
    Hotkey(String),
    /// The sound file Occurrences ring with; `None` for the app's own
    /// (#102).
    AlarmSound(Option<String>),
    /// The system Tallies show known units in; `None` follows the
    /// region (#103).
    Units(Option<UnitSystem>),
}

/// The wall clock the window shows days on.
pub type Wall = Arc<dyn Fn(DateTime<Utc>) -> NaiveDateTime + Send + Sync>;

/// The system's local time.
pub fn local_wall(utc: DateTime<Utc>) -> NaiveDateTime {
    utc.with_timezone(&Local).naive_local()
}

pub struct App {
    store: Arc<Store>,
    wall: Wall,
    t: L10n,
    selection: Selection,
    selected_note: Option<String>,
    view: View,
    /// What went wrong reading or writing the log, shown at the top.
    error: Option<String>,
    /// How far the Notes list is scrolled, for the parallax.
    scrolled: f32,
    /// The Notes a click unfolded on the compact list shown, and which
    /// list that is; they fold again once another list shows (#114).
    unfolded: HashSet<String>,
    unfolded_of: Option<String>,
    /// The photo shown full screen over the window, while one is (#114).
    viewer: Option<Viewer>,
    trail: Trail,
    /// The capture field's text, and what was set up for its Note, not
    /// written yet (#113).
    capture: String,
    capture_choices: CaptureChoices,
    /// The selected Note's text as it is being edited, and that Note.
    text: String,
    text_of: Option<String>,
    /// The text of the Section being added.
    amend: String,
    dialog: Option<Dialog>,
    /// Whether the dialog's text field is to take the keyboard, once, as
    /// it opens.
    focus: bool,
    /// The Routine or Reminder being made or changed on the right.
    editor: Option<RoutineEditor>,
    /// The field page or the entry page of a Tally on the right, and how
    /// the selected Note's Tally is looked at (#103).
    tally_form: Option<TallyForm>,
    tally_look: TallyLook,
    /// The region the system reports, an ISO country code, which picks
    /// the units Tallies show unless this Device chose them.
    region: Option<String>,
    /// Bumped by the alerts when a notification changed the log.
    changes: Option<Arc<AtomicU64>>,
    seen_changes: u64,
    /// Tells the alerts to look again after a change here.
    wake: Option<Sender<Event>>,
    /// The login start the Settings switch.
    autostart: Option<Autostart>,
    /// When the panes were last read, to follow the clock.
    read_at: DateTime<Utc>,
    /// The Quiet Hours as typed in the Settings.
    quiet_text: (String, String),
    /// The capture popup's hotkey, which the Settings move.
    hotkey: Option<Rc<RefCell<dyn hotkey::Control>>>,
    /// The hotkey as typed in the Settings.
    hotkey_text: String,
    /// The microphone, the speakers and the clipboard, where there are.
    recorder: Option<Box<dyn Recorder>>,
    player: Option<Box<dyn Player>>,
    /// The recording playing, by its hash.
    playing: Option<String>,
    /// How fast recordings play, 1 being normal speed; a Device setting.
    speed: f64,
    /// The recordings that ended, held on the capture field until its
    /// Note is saved.
    recorded: Vec<Vec<u8>>,
    clipboard: Option<Box<dyn Clipboard>>,
    /// The background threads making Transcripts and Photo Texts.
    transcriber: Option<Enricher>,
    reader: Option<Enricher>,
    /// The speech model the Transcripts need, and whether one was in use
    /// when last looked at.
    speech: Option<SpeechModel>,
    speech_ready: bool,
    /// A speech model file as typed into the Settings.
    /// Asks for a speech model file of the user's own.
    model_picker: FolderPicker,
    photos: PhotoCache,
    /// Other people's pictures on the cards.
    pictures: profile_view::Pictures,
    /// What the Notes shown are searched for.
    query: String,
    /// Whose Notes show (#53); each view has its own, as its search.
    people: PersonFilter,
    /// The LAN session hosted while the Sync view shows, and the name
    /// this computer goes by in it.
    sync: SyncHost,
    sync_name: String,
    /// The Conflicts an automatic sync brought, to be announced at the
    /// top until the Sync view shows them.
    arrived: usize,
    /// Asks the system's dialog for the shared folder, titled as given.
    folder_picker: FolderPicker,
    /// Tells the folder thread to sync now; without one, the window
    /// syncs itself.
    folder_wanted: Option<Arc<AtomicBool>>,
    /// Raised by a click on the Changes Nudge: the bundles of the people
    /// waiting are to be saved (#109).
    changes_wanted: Option<Arc<AtomicBool>>,
    /// Ask the system's dialogs where to save a bundle and which one to
    /// import.
    bundle_saver: FileSaver,
    bundle_opener: FolderPicker,
    /// Asks where to save a Sent Item.
    item_saver: FileSaver,
    /// Files handed over from outside the window, to open.
    handed: Option<Arc<std::sync::Mutex<Vec<std::path::PathBuf>>>>,
    /// What a bundle saved or imported did, announced at the top until
    /// clicked away.
    notice: Option<String>,
    /// Syncs that need an update of the app here or on another Device,
    /// each announced once until clicked away (ADR 0019).
    updates: std::collections::VecDeque<String>,
    /// The own Profile's picture, name field and new picture's square.
    profile_view: ProfileView,
    /// The own Profile card copied, and another person's being pasted
    /// (#107).
    card_form: CardForm,
    /// Text to put on the clipboard at the end of this frame.
    copy: Option<String>,
    /// The picture of the other Profile "Which one is you?" shows.
    question_view: ProfileView,
    /// The square of a new picture for the person the dialog shows.
    person_square: Option<Square>,
    /// Asks the system's dialog for pictures and sound files, titled as
    /// given.
    files_picker: FilesPicker,
    /// Why the recordings from files last made into a Note cannot be
    /// transcribed, one line each, announced at the top until clicked.
    untranscribable: Vec<(oldbrainfix_whisper::Problem, String)>,
    /// Whether reading a photo (`true`) or transcribing a recording again
    /// last brought no text, announced at the top until clicked.
    again_failed: Option<bool>,
    /// The dialog a person's Profile opened over, shown again after it.
    dialog_behind: Option<Dialog>,
    /// "Which one is you?" was put off until the Profile page asks again.
    question_later: bool,
    /// Asks the system's dialog for a picture file, titled as given.
    picture_picker: FolderPicker,
    /// The passphrase being typed into the Backups card.
    backup_form: Passphrase,
    /// The first start's view, while it shows instead of the panes.
    welcome: Option<Welcome>,
    /// Ask where to save the Recovery Sheet and which Backup to restore.
    sheet_saver: FileSaver,
    backup_opener: FolderPicker,
    /// Makes the own Profile from this computer's login, or fills in the
    /// defaults of one there (ADR 0017).
    profile_defaults: ProfileDefaults,
    /// The AI port while it is open (#112, ADR 0026): closed by "Done,
    /// review" or when the window goes.
    ai_port: Option<AiPort>,
    /// The client whose config was copied last while the port is open.
    ai_copied: Option<Client>,
    /// Asks the system's dialog for an alarm sound file, titled as given.
    sound_picker: FolderPicker,
    /// The alarm sound file last looked at and whether it rings or the
    /// own chime rings in its place, decoded once per file chosen.
    sound_rung: Option<(String, Rung)>,
}

/// Makes or fills in the own Profile in a store with this computer's
/// defaults, as [`login::apply_system_defaults`] does.
pub type ProfileDefaults = Box<dyn Fn(&Store) -> oldbrainfix_core::Result<Profile>>;

/// Asks for a folder, or a file to open, with the dialog's title.
pub type FolderPicker = Box<dyn Fn(&str) -> Option<std::path::PathBuf>>;

/// Asks for files to open, with the dialog's title; none when cancelled.
pub type FilesPicker = Box<dyn Fn(&str) -> Vec<std::path::PathBuf>>;

/// Asks where to save a file, with the dialog's title and the name it
/// proposes.
pub type FileSaver = Box<dyn Fn(&str, &str) -> Option<std::path::PathBuf>>;

/// The name a bundle for the person `person`, whose own Profile name is
/// `name`, is proposed under on `day`: the day's bundle name with theirs
/// in it, so the bundles for several people, saved one after another,
/// never overwrite each other (the phone's `changesFileName`).
fn changes_file_name(day: chrono::NaiveDate, person: &str, name: &str) -> String {
    let part = |text: &str| {
        text.split(|c: char| !c.is_alphanumeric())
            .filter(|w| !w.is_empty())
            .collect::<Vec<_>>()
            .join("-")
    };
    let who = Some(part(name))
        .filter(|w| !w.is_empty())
        .unwrap_or_else(|| part(person));
    let short: String = who.chars().take(32).collect();
    let base = bundle::file_name(day);
    match base.rsplit_once('.') {
        Some((stem, ext)) => format!("{stem}-{short}.{ext}"),
        None => base,
    }
}

/// The dialogs' filter for bundle files, and Sent Items to open.
fn bundle_dialog(title: &str) -> rfd::FileDialog {
    rfd::FileDialog::new().set_title(title).add_filter(
        oldbrainfix_core::APP_NAME,
        &[bundle::EXTENSION, sent::EXTENSION],
    )
}

/// The open dialog's filter for a Profile picture.
fn picture_dialog(title: &str) -> rfd::FileDialog {
    rfd::FileDialog::new()
        .set_title(title)
        .add_filter(title, &["jpg", "jpeg", "png", "webp"])
}

/// The open dialog's filter for "From file…": pictures and sound files.
fn files_dialog(title: &str) -> rfd::FileDialog {
    let both: Vec<&str> = PICTURE_EXTENSIONS
        .iter()
        .chain(SOUND_EXTENSIONS.iter())
        .copied()
        .collect();
    rfd::FileDialog::new()
        .set_title(title)
        .add_filter(title, &both)
}

/// The open dialog's filter for an alarm sound.
fn sound_dialog(title: &str) -> rfd::FileDialog {
    rfd::FileDialog::new()
        .set_title(title)
        .add_filter(title, &SOUND_EXTENSIONS)
}

/// The save dialog's filter for the Recovery Sheet.
fn sheet_dialog(title: &str) -> rfd::FileDialog {
    rfd::FileDialog::new()
        .set_title(title)
        .add_filter("PDF", &["pdf"])
}

/// The open dialog's filter for a Backup.
fn backup_dialog(title: &str) -> rfd::FileDialog {
    rfd::FileDialog::new()
        .set_title(title)
        .add_filter(oldbrainfix_core::APP_NAME, &[backup::EXTENSION])
}

/// The save dialog's filter for a Sent Item.
fn item_dialog(title: &str) -> rfd::FileDialog {
    rfd::FileDialog::new()
        .set_title(title)
        .add_filter(oldbrainfix_core::APP_NAME, &[sent::EXTENSION])
}

/// The region of a system locale such as `en-US`, `de_DE.UTF-8` or
/// `zh-Hans-CN`: its two-letter country code, or `None` when it names
/// none.
pub fn region_of(locale: Option<&str>) -> Option<String> {
    let tag = locale?.split(['.', '@']).next()?;
    tag.split(['-', '_'])
        .skip(1)
        .find(|part| part.len() == 2 && part.chars().all(|c| c.is_ascii_alphabetic()))
        .map(str::to_uppercase)
}

impl App {
    /// The window over `store`, speaking `locale`, showing the Agenda on
    /// the system's local time.
    pub fn new(store: impl Into<Arc<Store>>, locale: &str) -> App {
        let store = store.into();
        let read_at = store.now();
        // Normal speed when it cannot be read.
        let speed = DeviceSettings::new(&store).playback_speed().unwrap_or(1.0);
        let mut app = App {
            store,
            wall: Arc::new(local_wall),
            t: L10n::new(locale),
            selection: Selection::Agenda,
            selected_note: None,
            view: View::default(),
            error: None,
            scrolled: 0.0,
            unfolded: HashSet::new(),
            unfolded_of: None,
            viewer: None,
            trail: Trail::default(),
            capture: String::new(),
            capture_choices: CaptureChoices::default(),
            text: String::new(),
            text_of: None,
            amend: String::new(),
            dialog: None,
            focus: false,
            editor: None,
            tally_form: None,
            tally_look: TallyLook::new(None),
            region: region_of(sys_locale::get_locale().as_deref()),
            changes: None,
            seen_changes: 0,
            wake: None,
            autostart: None,
            read_at,
            quiet_text: (String::new(), String::new()),
            hotkey: None,
            hotkey_text: String::new(),
            recorder: None,
            player: None,
            playing: None,
            speed,
            recorded: Vec::new(),
            clipboard: None,
            transcriber: None,
            reader: None,
            speech: None,
            speech_ready: false,
            model_picker: Box::new(|title| rfd::FileDialog::new().set_title(title).pick_file()),
            photos: PhotoCache::default(),
            pictures: profile_view::Pictures::default(),
            query: String::new(),
            people: PersonFilter::default(),
            sync: SyncHost::default(),
            sync_name: device_name(),
            arrived: 0,
            folder_picker: Box::new(|title| rfd::FileDialog::new().set_title(title).pick_folder()),
            folder_wanted: None,
            changes_wanted: None,
            bundle_saver: Box::new(|title, name| {
                bundle_dialog(title).set_file_name(name).save_file()
            }),
            bundle_opener: Box::new(|title| bundle_dialog(title).pick_file()),
            item_saver: Box::new(|title, name| item_dialog(title).set_file_name(name).save_file()),
            handed: None,
            notice: None,
            updates: std::collections::VecDeque::new(),
            profile_view: ProfileView::default(),
            card_form: CardForm::default(),
            copy: None,
            question_view: ProfileView::default(),
            person_square: None,
            files_picker: Box::new(|title| files_dialog(title).pick_files().unwrap_or_default()),
            untranscribable: Vec::new(),
            again_failed: None,
            dialog_behind: None,
            question_later: false,
            picture_picker: Box::new(|title| picture_dialog(title).pick_file()),
            backup_form: Passphrase::default(),
            welcome: None,
            sheet_saver: Box::new(|title, name| {
                sheet_dialog(title).set_file_name(name).save_file()
            }),
            backup_opener: Box::new(|title| backup_dialog(title).pick_file()),
            profile_defaults: Box::new(login::apply_system_defaults),
            ai_port: None,
            ai_copied: None,
            sound_picker: Box::new(|title| sound_dialog(title).pick_file()),
            sound_rung: None,
        };
        app.hotkey_text = DeviceSettings::new(&app.store)
            .capture_hotkey()
            .unwrap_or_else(|_| oldbrainfix_core::settings::DEFAULT_CAPTURE_HOTKEY.into());
        app.refresh();
        // An AI Task left when the window went waits for its review.
        match app.review_ai() {
            Ok(()) => app.refresh(),
            Err(e) => app.error = Some(e.to_string()),
        }
        app
    }

    /// Shows the known units of Tallies as `region`, an ISO country code,
    /// does, unless this Device chose a system, instead of the system's
    /// region.
    pub fn set_region(&mut self, region: Option<&str>) {
        self.region = region.map(str::to_uppercase);
        self.refresh();
    }

    /// The system this Device shows the known units of Tallies in.
    fn unit_system(&self) -> oldbrainfix_core::Result<UnitSystem> {
        DeviceSettings::new(&self.store).unit_system(self.region.as_deref())
    }

    /// Shows days on `wall` instead of the system's local time.
    pub fn set_wall_clock(&mut self, wall: Wall) {
        self.wall = wall;
        self.refresh();
    }

    /// Hosts the Sync view on `bind` instead of the LAN address, as this
    /// computer named `name`; tests use the loopback.
    pub fn set_sync_host(&mut self, bind: Option<std::net::IpAddr>, name: &str) {
        self.sync.bind = bind;
        self.sync_name = name.to_string();
    }

    /// Joins a typed code with `joiner` instead of the LAN; tests hand
    /// one in.
    pub fn set_joiner(&mut self, joiner: Joiner) {
        self.sync.joiner = Some(joiner);
    }

    /// Asks `picker` for a Profile picture file instead of the system's
    /// dialog; tests hand one in.
    pub fn set_picture_picker(&mut self, picker: FolderPicker) {
        self.picture_picker = picker;
    }

    /// Asks `picker` for the shared folder instead of the system's
    /// dialog; tests hand in a path.
    pub fn set_folder_picker(&mut self, picker: FolderPicker) {
        self.folder_picker = picker;
    }

    /// Asks `picker` for a speech model file instead of the system's
    /// dialog; tests hand in a path.
    pub fn set_model_picker(&mut self, picker: FolderPicker) {
        self.model_picker = picker;
    }

    /// Asks `picker` for the alarm sound file instead of the system's
    /// dialog; tests hand in a path.
    pub fn set_sound_picker(&mut self, picker: FolderPicker) {
        self.sound_picker = picker;
    }

    /// Hands in the dialogs asking where to save a bundle and which one
    /// to import.
    pub fn set_bundle_dialogs(&mut self, saver: FileSaver, opener: FolderPicker) {
        self.bundle_saver = saver;
        self.bundle_opener = opener;
    }

    /// Hands in the dialogs asking where to save the Recovery Sheet and
    /// which Backup to restore.
    pub fn set_backup_dialogs(&mut self, saver: FileSaver, opener: FolderPicker) {
        self.sheet_saver = saver;
        self.backup_opener = opener;
    }

    /// Makes the own Profile with `defaults` instead of this computer's
    /// login once the first start's view is answered; tests hand one in.
    pub fn set_profile_defaults(&mut self, defaults: ProfileDefaults) {
        self.profile_defaults = defaults;
    }

    /// Shows the first start's view instead of the panes when it is due:
    /// nothing written yet here, and the view not answered before.
    pub fn offer_restore(&mut self) {
        if backup_view::restore_due(&self.store).unwrap_or(false) {
            self.welcome = Some(Welcome::default());
        }
    }

    /// Whether the first start's view shows.
    pub fn welcoming(&self) -> bool {
        self.welcome.is_some()
    }

    /// Does what the Backups card asked.
    fn backup(&mut self, asked: BackupAction) -> oldbrainfix_core::Result<()> {
        let t = self.t;
        match asked {
            BackupAction::SetUp => {
                let Some(passphrase) = self.backup_form.taken(&t) else {
                    return Ok(());
                };
                backup::set_up(
                    &self.store,
                    &passphrase,
                    backup::Kdf::STANDARD,
                    &mut backup::system_random,
                )?;
                self.backup_form = Passphrase::default();
                self.backup(BackupAction::SaveSheet)?;
            }
            BackupAction::SaveSheet => {
                let Some(pdf) = backup_view::sheet(&t, &self.store, &self.wall) else {
                    return Ok(());
                };
                let Some(mut path) = (self.sheet_saver)(
                    t.backup_sheet_save_title(),
                    "oldbrainfix-recovery-sheet.pdf",
                ) else {
                    return Ok(());
                };
                if path.extension().is_none_or(|e| e != "pdf") {
                    path.set_extension("pdf");
                }
                self.notice = Some(match std::fs::write(&path, pdf) {
                    Ok(()) => t.backup_sheet_saved().to_string(),
                    Err(e) => format!("{} ({e})", t.backup_sheet_failed()),
                });
            }
            BackupAction::PickFolder => {
                if let Some(path) = (self.folder_picker)(t.backup_folder_pick_title()) {
                    self.store
                        .set_backup_folder(Some(&path.to_string_lossy()))?;
                    self.backup(BackupAction::Now)?;
                }
            }
            BackupAction::Weekly(on) => self.store.set_backups_weekly(on)?,
            BackupAction::Now => {
                self.notice = Some(match backups::now(&self.store) {
                    Ok(_) => t.backup_done().to_string(),
                    Err(e) => format!("{} ({e})", t.backup_failed()),
                });
            }
        }
        Ok(())
    }

    /// Does what the first start's view asked.
    fn welcome_action(&mut self, asked: WelcomeAction) -> oldbrainfix_core::Result<()> {
        let t = self.t;
        match asked {
            WelcomeAction::New => self.welcomed()?,
            WelcomeAction::AddDevice => {
                if let Some(welcome) = &mut self.welcome {
                    welcome.adding = true;
                }
                self.notice = None;
            }
            WelcomeAction::Synced => self.welcomed()?,
            WelcomeAction::Back => {
                // A join under way finishes first: what it brings is kept.
                if self.sync.joining() {
                    return Ok(());
                }
                // Left after a sync went through: on, as with Continue.
                if self.welcome.as_ref().is_some_and(|w| w.adding) && self.sync.synced() {
                    return self.welcomed();
                }
                if let Some(welcome) = &mut self.welcome {
                    *welcome = Welcome::default();
                }
            }
            WelcomeAction::Pick => {
                if let Some(path) = (self.backup_opener)(t.restore_open_title()) {
                    match backup::read_header(&path) {
                        Ok(Some(header)) => {
                            if let Some(welcome) = &mut self.welcome {
                                welcome.file = Some((path, header));
                            }
                            self.notice = None;
                        }
                        _ => self.notice = Some(t.restore_not_a_backup().to_string()),
                    }
                }
            }
            WelcomeAction::Restore => {
                let Some(welcome) = &self.welcome else {
                    return Ok(());
                };
                let key = match backup_view::unlock(&t, welcome) {
                    Ok(key) => key,
                    Err(text) => {
                        self.notice = Some(text);
                        return Ok(());
                    }
                };
                let Some((path, _)) = &welcome.file else {
                    return Ok(());
                };
                match backup::restore(&self.store, path, &key) {
                    Ok(restored) => {
                        self.welcomed()?;
                        self.notice = Some(t.restore_done(restored.entries as i64));
                        // The playback speed came with it; one that cannot
                        // be read leaves the speed as it was.
                        if let Ok(speed) = DeviceSettings::new(&self.store).playback_speed() {
                            self.speed = speed;
                            if let Some(player) = &mut self.player {
                                player.set_speed(speed);
                            }
                        }
                    }
                    Err(e) => self.notice = Some(backup_view::restore_failure(&t, &e)),
                }
            }
        }
        Ok(())
    }

    /// The first start's view is answered: new, restored or synced. Only
    /// now is the own Profile made with this computer's defaults, so a
    /// Device being added takes the one its other Devices hold (ADR
    /// 0017); one that came with a restore or a sync stays as it is.
    fn welcomed(&mut self) -> oldbrainfix_core::Result<()> {
        backup_view::restore_answered(&self.store)?;
        self.welcome = None;
        (self.profile_defaults)(&self.store)?;
        Ok(())
    }

    /// Hands in the dialog asking where to save a Sent Item.
    pub fn set_item_saver(&mut self, saver: FileSaver) {
        self.item_saver = saver;
    }

    /// Asks `picker` for the files of "From file…" instead of the
    /// system's dialog; tests hand them in.
    pub fn set_files_picker(&mut self, picker: FilesPicker) {
        self.files_picker = picker;
    }

    /// Opens the files put into `handed` from outside the window, as
    /// `oldbrainfix open` does, at the next frame.
    pub fn set_handed_files(&mut self, handed: Arc<std::sync::Mutex<Vec<std::path::PathBuf>>>) {
        self.handed = Some(handed);
    }

    /// Whether a Sent Item's preview shows, a stranger's hidden one too,
    /// or that of an Area shared from afar.
    pub fn previewing(&self) -> bool {
        matches!(
            self.dialog,
            Some(Dialog::SentItem { .. } | Dialog::HeldSentItem { .. } | Dialog::AreaOffer { .. })
        )
    }

    /// What a bundle saved or imported did, as announced.
    pub fn notice(&self) -> Option<&str> {
        self.notice.as_deref()
    }

    /// Saves a bundle for the paired Device `device`, or of everything,
    /// where the user says; nothing asked when there is nothing new.
    fn save_bundle(&mut self, device: Option<String>) -> oldbrainfix_core::Result<()> {
        // Not even asked where to while sync waits for an AI Task.
        if self.store.sync_paused()? {
            self.notice = Some(self.t.sync_paused_error().to_string());
            return Ok(());
        }
        let since = match &device {
            Some(id) => self.store.held_by(id)?,
            None => Default::default(),
        };
        let name = device
            .as_ref()
            .and_then(|id| self.store.paired_device(id).ok().flatten())
            .map(|d| d.name)
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| self.t.unnamed_device().to_string());
        if self.store.entries_since(&since)?.is_empty() {
            self.notice = Some(self.t.bundle_nothing_new(&name));
            return Ok(());
        }
        let day = (self.wall)(self.store.now()).date();
        let Some(mut path) =
            (self.bundle_saver)(self.t.bundle_save_title(), &bundle::file_name(day))
        else {
            return Ok(());
        };
        if path.extension().is_none_or(|e| e != bundle::EXTENSION) {
            path.set_extension(bundle::EXTENSION);
        }
        self.notice = Some(
            match bundle::write_bundle_now(&self.store, &path, device.as_deref()) {
                Ok(Some((entries, _))) => self.t.bundle_saved(entries as i64),
                Ok(None) => self.t.bundle_nothing_new(&name),
                Err(e) => ai_view::failed_text(&self.t, self.t.bundle_failed(), &e),
            },
        );
        Ok(())
    }

    /// Opens the file at `path`: a Sent Item shows its preview, a
    /// stranger's held and hidden until "Show" (#91); anything else is
    /// imported as a bundle.
    fn open_file(&mut self, path: &std::path::Path) {
        let small = std::fs::metadata(path).is_ok_and(|m| m.len() <= sent::MAX_BYTES as u64);
        let bytes = if small {
            std::fs::read(path).unwrap_or_default()
        } else {
            Vec::new()
        };
        if !sent::looks_like_sent_item(&bytes) {
            self.import_bundle(path);
            return;
        }
        match self.store.receive_sent_item(&bytes) {
            Ok(Ok(Received::Opened(opened))) => self.preview(opened, None),
            Ok(Ok(Received::Held(held))) => {
                self.dialog = Some(Dialog::HeldSentItem {
                    held,
                    blocking: None,
                });
            }
            Ok(Err(e)) => self.notice = Some(refused(&self.t, e).to_string()),
            Err(e) => self.notice = Some(format!("{} ({e})", self.t.sent_item_unreadable())),
        }
    }

    /// Shows the preview of `opened`, held as the pending set `held` when
    /// a stranger's item was shown with "Show".
    fn preview(&mut self, opened: Box<Opened>, held: Option<String>) {
        let may_block = matches!(self.store.is_own_person(&opened.sender.id), Ok(false));
        self.dialog = Some(Dialog::SentItem {
            opened,
            compared: Compared::Not,
            may_block,
            blocking: None,
            held,
        });
    }

    /// "Show" on the stranger's Sent Item the dialog holds back: its
    /// preview, Compare codes included. Not remembered: opened again, it
    /// is hidden again.
    fn show_held(&mut self) -> oldbrainfix_core::Result<()> {
        let Some(Dialog::HeldSentItem { held, .. }) = &self.dialog else {
            return Ok(());
        };
        let id = held.id.clone();
        match self.store.show_held_sent_item(&id)? {
            Some(Ok(opened)) => self.preview(Box::new(opened), Some(id)),
            Some(Err(e)) => {
                self.dialog = None;
                self.notice = Some(refused(&self.t, e).to_string());
            }
            None => {
                self.dialog = None;
                self.notice = Some(self.t.sent_item_gone().to_string());
            }
        }
        Ok(())
    }

    /// Opens the stranger's Sent Item held as `id`, hidden until "Show".
    fn open_held(&mut self, id: &str) -> oldbrainfix_core::Result<()> {
        match self.store.held_sent_item(id)? {
            Some(held) => {
                self.dialog = Some(Dialog::HeldSentItem {
                    held,
                    blocking: None,
                });
            }
            None => self.notice = Some(self.t.sent_item_gone().to_string()),
        }
        Ok(())
    }

    /// Accepts the stranger's Sent Item held as `id`, shown or not,
    /// `verified` when the codes were compared, judged against the log
    /// now; announces what came of it. One gone meanwhile, or refused
    /// now, as from a person blocked since, is announced, not failed.
    fn accept_held(&mut self, id: &str, verified: bool) -> oldbrainfix_core::Result<()> {
        let t = self.t;
        let state = match self.store.show_held_sent_item(id)? {
            Some(Ok(opened)) => opened.state,
            Some(Err(e)) => {
                self.notice = Some(refused(&t, e).to_string());
                return Ok(());
            }
            None => {
                self.notice = Some(t.sent_item_gone().to_string());
                return Ok(());
            }
        };
        let said = match self.store.accept_held_sent_item(id, verified) {
            Ok(None) => t.sent_item_nothing_to_cancel(),
            Ok(Some(_)) => match state {
                SentItemState::Update => t.sent_item_applied(),
                SentItemState::Cancel => t.sent_item_trashed(),
                _ => t.sent_item_accepted(),
            },
            Err(oldbrainfix_core::Error::FromBlocked) => t.sent_item_from_blocked(),
            // Decided meanwhile.
            Err(oldbrainfix_core::Error::Invalid(_)) => t.sent_item_gone(),
            Err(e) => return Err(e),
        };
        self.notice = Some(said.to_string());
        Ok(())
    }

    /// Saves the Note of the send dialog as a Sent Item where the user
    /// says: encrypted to the person picked, else signed only.
    fn send_note(&mut self) -> oldbrainfix_core::Result<()> {
        let Some(Dialog::Send {
            note,
            recipients,
            picked,
            cancel,
            ..
        }) = self.dialog.take()
        else {
            return Ok(());
        };
        if self.store.sync_paused()? {
            self.notice = Some(self.t.sync_paused_error().to_string());
            return Ok(());
        }
        let day = (self.wall)(self.store.now()).date().format("%Y-%m-%d");
        let Some(mut path) =
            (self.item_saver)(self.t.send_save_title(), &sent::file_name(&day.to_string()))
        else {
            return Ok(());
        };
        if path.extension().is_none_or(|e| e != sent::EXTENSION) {
            path.set_extension(sent::EXTENSION);
        }
        let to = picked.and_then(|i| recipients.get(i));
        let bytes = self.store.send_note(&note, to, cancel)?;
        self.notice = Some(match std::fs::write(&path, bytes) {
            Ok(()) => self.t.sent_saved().to_string(),
            Err(e) => format!("{} ({e})", self.t.send_failed()),
        });
        Ok(())
    }

    /// The People an Area can go to from afar (#108): not blocked, with a
    /// Device whose key is known here; by the name the user sees.
    fn afar_people(&self) -> oldbrainfix_core::Result<Vec<AfarPerson>> {
        let mut out = Vec::new();
        for person in self.store.kept_people()? {
            if self.store.is_blocked(&person.id)?
                || self.store.offer_devices(&person.id)?.is_empty()
            {
                continue;
            }
            out.push(AfarPerson {
                shown: self.store.person_shown(&person.id)?,
                verified: self.store.person_verified(&person.id)?,
                id: person.id,
            });
        }
        let t = self.t;
        out.sort_by_cached_key(|p| profile_view::display_name(&t, &p.shown.name).to_lowercase());
        Ok(out)
    }

    /// Shares the Area of the dialog with the person picked from afar
    /// (#108): the shared folder first when changes travel by one and
    /// none is chosen, then where the offer is saved. Only then is
    /// anything written, so backing out of a dialog changes nothing. A
    /// Starter Set Area goes on under fresh ids, which the selection
    /// follows.
    fn share_afar(&mut self) -> oldbrainfix_core::Result<()> {
        let Some(Dialog::ShareAfar {
            area,
            role,
            person: Some(person),
            transport,
            ..
        }) = self.dialog.take()
        else {
            return Ok(());
        };
        let t = self.t;
        if self.store.sync_paused()? {
            self.notice = Some(t.sync_paused_error().to_string());
            return Ok(());
        }
        let transport = self.store.transport_with(&person)?.unwrap_or(transport);
        let mut folder = None;
        if transport == Transport::Folder && self.store.sync_folder()?.is_none() {
            let Some(path) = (self.folder_picker)(t.folder_pick_title()) else {
                return Ok(());
            };
            folder = Some(path);
        }
        let day = (self.wall)(self.store.now()).date();
        let Some(mut path) =
            (self.bundle_saver)(t.share_afar_save_title(), &bundle::file_name(day))
        else {
            return Ok(());
        };
        if path.extension().is_none_or(|e| e != bundle::EXTENSION) {
            path.set_extension(bundle::EXTENSION);
        }
        if let Some(folder) = &folder {
            self.store.choose_sync_folder(&folder.to_string_lossy())?;
        }
        let shared = match self.store.share_area_with(&area, &person, role, transport) {
            Ok(shared) => shared,
            Err(e) => {
                self.notice = Some(ai_view::failed_text(&t, t.share_afar_failed(), &e));
                return Ok(());
            }
        };
        if self.selection == Selection::Place(area.clone()) {
            self.selection = Selection::Place(shared.clone());
        }
        if self.trail.area.as_deref() == Some(area.as_str()) {
            self.trail.area = Some(shared.clone());
        }
        self.notice = Some(
            match bundle::write_area_offer_now(&self.store, &path, &shared, &person) {
                Ok(_) => t.sent_saved().to_string(),
                Err(e) => ai_view::failed_text(&t, t.share_afar_failed(), &e),
            },
        );
        // The Area goes into the folder as well.
        if transport == Transport::Folder {
            self.folder_now()?;
        }
        Ok(())
    }

    /// Accepts the Area shared from afar the dialog shows (#108), the
    /// sharer verified when the codes matched; asks for the shared folder
    /// when its changes travel by one and none is chosen here. One
    /// decided meanwhile, or refused now, is announced, not failed.
    fn accept_offer(&mut self) -> oldbrainfix_core::Result<()> {
        let Some(Dialog::AreaOffer { held, compared, .. }) = self.dialog.take() else {
            return Ok(());
        };
        let t = self.t;
        match self
            .store
            .accept_area_offer(&held.id, compared == Compared::Match)
        {
            Ok(_) => {}
            Err(oldbrainfix_core::Error::Offer(e)) => {
                self.notice = Some(offer_refusal(&t, e).to_string());
                return Ok(());
            }
            // Decided meanwhile; a block drops it too.
            Err(oldbrainfix_core::Error::Invalid(_)) => {
                self.notice = Some(if self.store.is_blocked(&held.sender.id)? {
                    t.area_offer_from_blocked().to_string()
                } else {
                    t.area_offer_gone().to_string()
                });
                return Ok(());
            }
            Err(e) => return Err(e),
        }
        self.notice = Some(t.area_offer_accepted().to_string());
        // The alerts plan the Area's Routines.
        if let Some(wake) = &self.wake {
            drop(wake.send(Event::Wake));
        }
        if held.transport == Transport::Folder && self.store.sync_folder()?.is_none() {
            self.dialog = Some(Dialog::OfferFolder {
                area: offer_area(&t, &held),
            });
        } else if held.transport == Transport::Folder {
            self.folder_now()?;
        }
        Ok(())
    }

    /// "Send changes to Ann" on the person's Profile the dialog shows
    /// (#108): saves a bundle of what is new in the Areas shared with
    /// them, for all of their Devices at once, where the user says;
    /// nothing asked when nothing is new for them.
    fn send_changes(&mut self) -> oldbrainfix_core::Result<()> {
        let Some(Dialog::Person { id, .. }) = &self.dialog else {
            return Ok(());
        };
        let id = id.clone();
        let title = self.t.person_send_changes_title().to_string();
        if let Some(said) = self.send_changes_to(&id, &title)? {
            self.notice = Some(said);
        }
        Ok(())
    }

    /// After a click on the Changes Nudge (#109): a save dialog for each
    /// person waiting, one after another, in the order the Changes Nudge
    /// names them. A dialog cancelled skips that person, who keeps the
    /// Changes Nudge.
    fn send_waiting_changes(&mut self) -> oldbrainfix_core::Result<()> {
        let Some(planned) = Alarms::new(&self.store).changes_nudge(&*self.wall)? else {
            return Ok(());
        };
        let mut said: Vec<String> = Vec::new();
        for id in &planned.people {
            let shown = self.store.person_shown(id)?;
            let name = profile_view::display_name(&self.t, &shown.name).to_string();
            let title = self.t.changes_save_title(&name);
            if let Some(text) = self.send_changes_to(id, &title)?
                && !said.contains(&text)
            {
                said.push(text);
            }
        }
        if !said.is_empty() {
            self.notice = Some(said.join("\n"));
        }
        Ok(())
    }

    /// Saves a bundle of what is new for the person `id` where the save
    /// dialog titled `title` says, and only once it is saved counts the
    /// changes up to the log's mark, read before writing, as sent to them
    /// (#109). What to announce; `None` when the dialog was cancelled.
    fn send_changes_to(
        &mut self,
        id: &str,
        title: &str,
    ) -> oldbrainfix_core::Result<Option<String>> {
        let t = self.t;
        if self.store.sync_paused()? {
            return Ok(Some(t.sync_paused_error().to_string()));
        }
        let name = profile_view::display_name(&t, &self.store.person_shown(id)?.name).to_string();
        if self.store.entries_for_person(id)?.is_empty() {
            return Ok(Some(t.bundle_nothing_new(&name)));
        }
        let day = (self.wall)(self.store.now()).date();
        let theirs = self.store.person_theirs(id)?.name;
        let Some(mut path) = (self.bundle_saver)(title, &changes_file_name(day, id, &theirs))
        else {
            return Ok(None);
        };
        if path.extension().is_none_or(|e| e != bundle::EXTENSION) {
            path.set_extension(bundle::EXTENSION);
        }
        let seq = self.store.current_seq()?;
        Ok(Some(
            match bundle::write_person_bundle_now(&self.store, &path, id) {
                Ok(Some(_)) => {
                    self.store.note_changes_sent(id, seq)?;
                    t.sent_saved().to_string()
                }
                Ok(None) => t.bundle_nothing_new(&name),
                Err(e) => ai_view::failed_text(&t, t.bundle_failed(), &e),
            },
        ))
    }

    /// Imports the bundle at `path` and announces what came in; one
    /// offering an Area shared from afar (#108) shows its preview instead.
    fn import_bundle(&mut self, path: &std::path::Path) {
        match import_bundle_result(&self.t, &self.store, path) {
            Ok(result) => match result.offer {
                Some(held) => {
                    self.dialog = Some(Dialog::AreaOffer {
                        held,
                        compared: Compared::Not,
                        blocking: None,
                    });
                }
                None => self.notice = Some(bundle_result_text(&self.t, &result)),
            },
            Err(said) => self.notice = Some(said),
        }
        self.refresh();
        // The alerts plan again: a Done in the bundle closes its alarm.
        if let Some(wake) = &self.wake {
            drop(wake.send(Event::Wake));
        }
    }

    /// Leaves folder syncs to the thread that `wanted` wakes.
    pub fn set_folder_wanted(&mut self, wanted: Arc<AtomicBool>) {
        self.folder_wanted = Some(wanted);
    }

    /// Saves the bundles of the people the Changes Nudge covers whenever
    /// a click on it raised `wanted` (#109).
    pub fn set_changes_wanted(&mut self, wanted: Arc<AtomicBool>) {
        self.changes_wanted = Some(wanted);
    }

    /// Syncs through the shared folder now: on the folder thread, or here
    /// when there is none.
    fn folder_now(&mut self) -> oldbrainfix_core::Result<()> {
        match &self.folder_wanted {
            Some(wanted) => wanted.store(true, Ordering::SeqCst),
            None => {
                folder_sync::round(&self.store)?;
            }
        }
        Ok(())
    }

    /// The AI port while it is open (#112).
    pub fn ai_port(&self) -> Option<&AiPort> {
        self.ai_port.as_ref()
    }

    /// Shows the review of the AI Task that waits, if one does; one that
    /// changed nothing is declined without asking.
    fn review_ai(&mut self) -> oldbrainfix_core::Result<()> {
        let Some(task) = self.store.ai_task()? else {
            return Ok(());
        };
        if self.store.ai_review(&task)?.is_empty() {
            self.store.decline_pending(&task)?;
            if self.selection == Selection::AiReview {
                self.selection = Selection::Settings;
            }
        } else {
            self.selection = Selection::AiReview;
        }
        Ok(())
    }

    /// What was asked of the AI port's card or of the review (#112).
    fn ai_action(&mut self, asked: AiAction) -> oldbrainfix_core::Result<()> {
        let t = self.t;
        match asked {
            AiAction::Open => {
                if self.ai_port.is_none() {
                    self.ai_port = Some(AiPort::open(&self.store)?);
                    self.ai_copied = None;
                }
                if self.selection == Selection::AiReview {
                    self.selection = Selection::Settings;
                }
            }
            AiAction::Copy(client) => {
                if let Some(port) = &self.ai_port {
                    self.copy = Some(ai_view::client_text(client, &port.url(), port.token()));
                    self.ai_copied = Some(client);
                }
            }
            AiAction::Done => {
                if let Some(mut port) = self.ai_port.take() {
                    // What the client asked before "Done" still counts.
                    port.answer_waiting(&self.store);
                    port.close();
                }
                self.ai_copied = None;
                self.review_ai()?;
            }
            AiAction::Review => {
                if self.ai_port.is_none() {
                    self.review_ai()?;
                }
            }
            AiAction::Accept => {
                if self.ai_port.is_some() {
                    return Ok(());
                }
                if let Some(task) = self.store.ai_task()? {
                    self.store.accept_pending(&task)?;
                    self.notice = Some(t.ai_accepted().to_string());
                }
                self.selection = Selection::Agenda;
            }
            AiAction::AskReject => self.dialog = Some(Dialog::RejectAi),
            AiAction::Reject => {
                self.dialog = None;
                if self.ai_port.is_some() {
                    return Ok(());
                }
                if let Some(task) = self.store.ai_task()? {
                    self.store.decline_pending(&task)?;
                    self.notice = Some(t.ai_rejected().to_string());
                }
                self.selection = Selection::Agenda;
            }
        }
        Ok(())
    }

    /// The AI Task's review (#112), or that none waits.
    fn ai_review_pane(&mut self, ui: &mut Ui, tones: Tones, actions: &mut Vec<Action>) {
        let t = self.t;
        let Some(shown) = &self.view.ai_review else {
            let text = ui.visuals().text_color();
            ui.heading(RichText::new(t.ai_review_title()).color(text));
            let weak = ui.visuals().weak_text_color();
            ui.label(RichText::new(t.ai_review_none()).color(weak));
            return;
        };
        let mut asked = None;
        if let Some(note) = ai_view::review_page(ui, &t, tones, shown, &mut asked) {
            actions.push(Action::SelectNote(note));
        }
        if let Some(asked) = asked {
            actions.push(Action::Ai(asked));
        }
    }

    /// The LAN session the Sync view hosts.
    pub fn sync_host(&self) -> &SyncHost {
        &self.sync
    }

    /// Reads the log again whenever `changes` moves on.
    pub fn watch_changes(&mut self, changes: Arc<AtomicU64>) {
        self.seen_changes = changes.load(Ordering::SeqCst);
        self.changes = Some(changes);
    }

    /// Tells `wake` after every change made here.
    pub fn set_wake(&mut self, wake: Sender<Event>) {
        self.wake = Some(wake);
    }

    /// The login start the Settings switch on and off.
    pub fn set_autostart(&mut self, autostart: Option<Autostart>) {
        self.autostart = autostart;
    }

    /// The capture popup's hotkey, which the Settings show and move.
    pub fn set_hotkey(&mut self, hotkey: Option<Rc<RefCell<dyn hotkey::Control>>>) {
        self.hotkey = hotkey;
    }

    /// The microphone voice Notes are recorded with.
    pub fn set_recorder(&mut self, recorder: Box<dyn Recorder>) {
        self.recorder = Some(recorder);
    }

    /// The speakers voice recordings are played on.
    pub fn set_player(&mut self, mut player: Box<dyn Player>) {
        player.set_speed(self.speed);
        self.player = Some(player);
    }

    /// The clipboard pictures are pasted from.
    pub fn set_clipboard(&mut self, clipboard: Box<dyn Clipboard>) {
        self.clipboard = Some(clipboard);
    }

    /// The threads making Transcripts and Photo Texts, and the speech
    /// model the Transcripts wait for; without one, they wait for none.
    pub fn set_enrichers(
        &mut self,
        transcriber: Option<Enricher>,
        reader: Option<Enricher>,
        speech: Option<SpeechModel>,
    ) {
        self.transcriber = transcriber;
        self.reader = reader;
        self.speech = speech;
        self.speech_ready = false;
        self.kick();
    }

    /// Whether a voice recording runs.
    pub fn recording(&self) -> bool {
        self.recorder.as_ref().is_some_and(|r| r.recording())
    }

    /// The text the Notes shown are searched for.
    pub fn set_query(&mut self, query: &str) {
        self.query = query.to_string();
        self.refresh();
    }

    /// Whose Notes the middle shows (#53).
    pub fn people_filter(&self) -> &PersonFilter {
        &self.people
    }

    /// The Notes shown in the middle, by id.
    pub fn shown_notes(&self) -> Vec<String> {
        self.view.notes.iter().map(|p| p.note.id.clone()).collect()
    }

    /// Tells the background threads to look for something to read; the
    /// Transcripts only once a speech model is in use.
    fn kick(&mut self) {
        if let Some(reader) = &self.reader {
            reader.kick();
        }
        let ready = self.speech.as_ref().is_none_or(|s| s.current().is_some());
        self.speech_ready = ready;
        if ready && let Some(transcriber) = &self.transcriber {
            transcriber.kick();
        }
    }

    /// The Routine or Reminder being made or changed, if any.
    pub fn routine_editor(&self) -> Option<&RoutineEditor> {
        self.editor.as_ref()
    }

    /// The editor of the Routine `id`, on the right.
    pub fn open_routine(&mut self, id: &str) {
        self.apply(Action::OpenRoutine(id.to_string()));
    }

    pub fn t(&self) -> &L10n {
        &self.t
    }

    /// The log, for looking at what was written.
    pub fn store(&self) -> &Store {
        &self.store
    }

    pub fn selection(&self) -> &Selection {
        &self.selection
    }

    pub fn selected_note(&self) -> Option<&str> {
        self.selected_note.as_deref()
    }

    /// What went wrong last, as announced at the top.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Shows `selection` in the middle pane.
    pub fn select(&mut self, selection: Selection) {
        self.apply(Action::Select(selection));
    }

    /// Shows the Note `id` on the right.
    pub fn select_note(&mut self, id: &str) {
        self.apply(Action::SelectNote(id.to_string()));
    }

    /// Keeps what is typed but not saved yet, as the window closes: the
    /// capture field's text as a new Note nudged in 1 h, as leaving the
    /// phone's capture screen does, and the selected Note's edited text.
    pub fn flush(&mut self) {
        if !self.capture.trim().is_empty() || self.recording() || !self.recorded.is_empty() {
            self.apply(Action::Capture(Notes::default_nudge()));
        }
        self.apply(Action::SaveText);
    }

    /// Installs the look and the fonts. Cheap enough to call every frame.
    pub fn install(ctx: &egui::Context) {
        theme::install(ctx);
        let bound = ctx.fonts(|f| {
            f.families()
                .contains(&egui::FontFamily::Name(icons::FAMILY.into()))
        });
        if !bound {
            ctx.set_fonts(icons::fonts());
        }
    }

    /// Reads what the panes show from the log. A selected place that is
    /// gone falls back to All Notes.
    fn refresh(&mut self) {
        // Each Note opened is looked at afresh; a page of another Note's
        // Tally goes.
        if self.tally_look.of != self.selected_note {
            self.tally_look = TallyLook::new(self.selected_note.clone());
        }
        if self
            .tally_form
            .as_ref()
            .is_some_and(|f| !f.capturing() && Some(f.note_id()) != self.selected_note.as_deref())
        {
            self.tally_form = None;
        }
        let mut read = self.read();
        if let (Ok(view), Selection::Place(id)) = (&read, &self.selection)
            && view.area(id).is_none()
            && view.topic(id).is_none()
        {
            self.selection = Selection::All;
            read = self.read();
        }
        match read {
            Ok(view) => {
                self.view = view;
                self.error = None;
            }
            Err(e) => self.error = Some(e.to_string()),
        }
        // An Area or Topic deleted meanwhile is no place to capture into.
        if let Some(place) = &self.capture_choices.place
            && self.view.area(place).is_none()
            && self.view.topic(place).is_none()
        {
            self.capture_choices.place = None;
            let place = self.capture_place();
            self.view.capture_spawns =
                capture_choices::may_spawn(&self.store, place.as_deref()).unwrap_or(true);
        }
        self.read_at = self.store.now();
        self.arrived = self.conflicts_arrived();
        // What the hosts and the folder found that needs an update.
        for (source, update) in self.store.take_update_notices().unwrap_or_default() {
            let text = update_notice_text(&self.t, &self.store, &source, update);
            self.updates.push_back(text);
        }
        // "Which one is you?" opens once it comes up, and goes once it was
        // answered here or on another Device.
        match (&self.view.profile_question, &self.dialog) {
            (Some(_), None) if !self.question_later => self.dialog = Some(Dialog::WhichProfile),
            (None, Some(Dialog::WhichProfile)) => self.dialog = None,
            _ => {}
        }
        if self.view.profile_question.is_none() {
            self.question_later = false;
        }
        if self.selection == Selection::Sync {
            // What an automatic sync brought meanwhile.
            self.sync.reload_conflicts(&self.store);
        }
        if let Ok(quiet) = DeviceSettings::new(&self.store).quiet_hours() {
            self.quiet_text = (quiet.start.to_string(), quiet.end.to_string());
        }
        // What a click unfolded stays while its list shows.
        let cards = self.cards_key();
        if cards != self.unfolded_of {
            self.unfolded.clear();
            self.unfolded_of = cards;
        }
        let shown = self.view.detail.as_ref().map(|(p, _)| &p.note);
        if shown.map(|n| &n.id) != self.text_of.as_ref() {
            self.text = shown.map(|n| n.text.clone()).unwrap_or_default();
            self.text_of = shown.map(|n| n.id.clone());
            self.amend.clear();
        }
        self.kick();
    }

    /// Which Note list shows, by the phone's names (#114).
    fn cards_key(&self) -> Option<String> {
        let area = matches!(&self.selection, Selection::Place(id) if self.view.area(id).is_some());
        cards_key(&self.selection, &self.people, area)
    }

    /// Whether the Note list shown shows its Notes compact (#114).
    pub fn compact(&self) -> bool {
        self.view.compact
    }

    /// The photo shown full screen, by hash, and how far it is zoomed in,
    /// 1 fitting the window (#114).
    pub fn viewed_photo(&self) -> Option<(&str, f32)> {
        self.viewer.as_ref().map(|v| (v.hash.as_str(), v.zoom))
    }

    /// How many Conflicts an automatic sync brought that are still open
    /// and not announced; none once they are settled elsewhere.
    fn conflicts_arrived(&self) -> usize {
        if !self.store.conflicts_arrived().unwrap_or(false) {
            return 0;
        }
        // The Sync view lists them: announced.
        if self.selection == Selection::Sync {
            drop(self.store.set_conflicts_arrived(false));
            return 0;
        }
        let open = self.store.conflicts().map(|c| c.len()).unwrap_or(0);
        if open == 0 {
            drop(self.store.set_conflicts_arrived(false));
        }
        open
    }

    /// The Conflicts an automatic sync brought, as announced at the top.
    pub fn arrived_conflicts(&self) -> usize {
        self.arrived
    }

    fn read(&self) -> oldbrainfix_core::Result<View> {
        let notes = Notes::new(&self.store);
        let places = Places::new(&self.store);
        let mut areas = Vec::new();
        let mut access = std::collections::HashMap::new();
        let mut people = std::collections::HashMap::new();
        let mut names = std::collections::HashMap::new();
        let mut shown_people = std::collections::HashMap::new();
        let mut mine = std::collections::HashMap::new();
        for area in places.areas()? {
            let topics = places.topics(&area.id)?;
            access.insert(area.id.clone(), self.store.access_in(&area.id, None)?);
            if let Some(grants) = self.store.grants_of(&area.id)? {
                let shared = self.store.people(&area.id)?;
                for person in &shared {
                    let shown = self.store.person_shown(&person.id)?;
                    names.insert(person.id.clone(), shown.name.clone());
                    shown_people.insert(person.id.clone(), shown);
                }
                people.insert(area.id.clone(), shared);
                if let Some(person) = grants.person_of(&self.store.device_id()?) {
                    mine.insert(area.id.clone(), person.to_string());
                }
            }
            areas.push((area, topics));
        }
        let routines = Routines::new(&self.store);
        let search = Search::new(&self.store);
        let shown = match &self.selection {
            Selection::Agenda
            | Selection::Profile
            | Selection::Settings
            | Selection::Sync
            | Selection::About
            | Selection::Licences
            | Selection::SentItemsWaiting
            | Selection::AreaOffersWaiting
            | Selection::AiReview
            | Selection::Frogs => Vec::new(),
            Selection::All => self
                .store
                .notes_by(search.notes(&self.query, None)?, &self.people)?,
            Selection::Unsorted => self.store.notes_by(notes.unsorted()?, &self.people)?,
            Selection::Place(id) => {
                let found = search.notes(&self.query, Some(id))?;
                // A Topic's Notes show inside the Topic; a search in its
                // Area still finds them, their cards naming the Topic.
                let area = areas.iter().any(|(a, _)| a.id == *id);
                let found = if self.query.trim().is_empty() && area {
                    found
                        .into_iter()
                        .filter(|n| n.place.as_deref() == Some(id.as_str()))
                        .collect()
                } else {
                    found
                };
                self.store.notes_by(found, &self.people)?
            }
            Selection::Trash => notes.trashed()?,
        };
        let store = &self.store;
        let place = |note: Note| -> oldbrainfix_core::Result<Placed> {
            let author = store.author_of(&note.id)?;
            let Some(id) = note.place.clone() else {
                return Ok(Placed {
                    note,
                    area: None,
                    topic: None,
                    author,
                });
            };
            let topic = places.topic_by_id(&id)?;
            let area_id = topic.as_ref().map_or(id, |t| t.area_id.clone());
            Ok(Placed {
                note,
                area: places.area_by_id(&area_id)?,
                topic,
                author,
            })
        };
        let frogs = Frogs::new(store);
        let show_frog = |frog: Frog| -> oldbrainfix_core::Result<Option<FrogShown>> {
            // A Frog whose Note is gone for good shows nowhere.
            let Some(note) = frogs.note(&frog)? else {
                return Ok(None);
            };
            let area = place(note.clone())?.area;
            Ok(Some(FrogShown {
                may_check: store.may_check_off(&frog.id)?,
                manages: store.may_manage(store.area_of(&frog.id)?.as_deref())?,
                done_on: frog.done.map(|at| Day::of((self.wall)(at))),
                frog,
                note,
                area,
            }))
        };
        let shown_frogs = |list: Vec<Frog>| -> oldbrainfix_core::Result<Vec<FrogShown>> {
            let mut out = Vec::new();
            for frog in list {
                out.extend(show_frog(frog)?);
            }
            Ok(out)
        };
        let detail = match &self.selected_note {
            Some(id) => match notes.by_id(id)? {
                Some(note) if !note.is_merged() => Some((place(note)?, notes.sections(id)?)),
                _ => None,
            },
            None => None,
        };
        let mut placed = Vec::new();
        for note in shown {
            placed.push(place(note)?);
        }
        // Once per read for every card, never per frame.
        let listed: Vec<String> = placed.iter().map(|p| p.note.id.clone()).collect();
        let sections = if listed.is_empty() {
            BTreeMap::new()
        } else {
            notes.sections_of(&listed)?
        };
        let is_area = matches!(&self.selection, Selection::Place(id) if areas.iter().any(|(a, _)| a.id == *id));
        let compact = match cards_key(&self.selection, &self.people, is_area) {
            Some(key) => {
                self.store.local_setting(&compact_setting(&key))?.as_deref() == Some("true")
            }
            None => false,
        };
        let note_routines = match &detail {
            Some((p, _)) if !p.note.is_trashed() => routines.of_note(&p.note.id)?,
            _ => Vec::new(),
        };
        let mut note_frogs = Vec::new();
        if let Some((p, _)) = &detail
            && !p.note.is_trashed()
        {
            for frog in frogs.of_note(&p.note.id)? {
                let may_check = store.may_check_off(&frog.id)?;
                note_frogs.push((frog, may_check));
            }
        }
        let tally = match &detail {
            Some((p, _)) if !p.note.is_trashed() => tally_view::read(
                store,
                &p.note.id,
                &self.tally_look,
                self.unit_system()?,
                &*self.wall,
            )?,
            _ => None,
        };
        let may = match &detail {
            Some((p, _)) => NoteMay {
                changes: self.store.may_change(&p.note.id)?,
                schedules: self
                    .store
                    .may_manage(self.store.area_of(&p.note.id)?.as_deref())?,
                amends: self.store.may_write_in(&p.note.id)?,
            },
            None => NoteMay::default(),
        };
        let mut removable = HashSet::new();
        let mut text_removable = HashSet::new();
        let mut remakeable = HashSet::new();
        let mut text_changeable = HashSet::new();
        if let Some((p, sections)) = &detail
            && !p.note.is_trashed()
        {
            if may.changes && p.note.text_editable {
                text_changeable.insert(p.note.id.clone());
            }
            for section in sections {
                if section.text_editable && self.store.may_change(&section.id)? {
                    text_changeable.insert(section.id.clone());
                }
            }
            let note = (&p.note.id, &p.note.photos, &p.note.recordings);
            let merged = sections.iter().map(|s| (&s.id, &s.photos, &s.recordings));
            for (id, photos, recordings) in std::iter::once(note).chain(merged) {
                let changes = self.store.may_change(id)?;
                for hash in photos {
                    let key = (id.clone(), hash.clone());
                    if notes.may_remove_photo(id, hash)? {
                        removable.insert(key.clone());
                    }
                    if notes.may_remove_photo_text(id, hash)? {
                        text_removable.insert(key.clone());
                    }
                    if changes && notes.photo_bytes(hash).is_some() {
                        remakeable.insert(key);
                    }
                }
                for hash in recordings {
                    let key = (id.clone(), hash.clone());
                    if notes.may_remove_recording(id, hash)? {
                        removable.insert(key.clone());
                    }
                    if notes.may_remove_transcript(id, hash)? {
                        text_removable.insert(key.clone());
                    }
                    if changes && notes.recording_bytes(hash).is_some() {
                        remakeable.insert(key);
                    }
                }
            }
        }
        let agenda = match self.selection {
            Selection::Agenda => {
                let wall = &*self.wall;
                // Read as the Agenda shows, never while it is drawn, and
                // before the rows read their Done: today's picks are
                // written for everyone the Session reaches when they are
                // new (ADR 0029), and reading again writes nothing.
                let sessions = Sessions::new(store);
                let system = self.unit_system()?;
                let mut picked = HashMap::new();
                for today in sessions.pick_today(wall)? {
                    let key = (today.session.id().to_string(), today.occurrence.key());
                    picked.insert(key, PicksShown::read(store, &self.t, today, system)?);
                }
                let counts: HashMap<String, _> = sessions
                    .all()?
                    .into_iter()
                    .map(|s| (s.id().to_string(), s))
                    .collect();
                let agenda = routines.agenda_over(wall, AGENDA_DAYS)?;
                let row = |row: AgendaRow| -> oldbrainfix_core::Result<Row> {
                    let area = match &row.note {
                        Some(note) => place(note.clone())?.area,
                        None => None,
                    };
                    let key = (row.routine.id.clone(), row.occurrence.key());
                    let session = match picked.get(&key) {
                        Some(shown) => Some(SessionPart::Today(Box::new(shown.clone()))),
                        None => counts
                            .get(&row.routine.id)
                            .map(|s| SessionPart::Later(s.count_on(row.occurrence.day))),
                    };
                    Ok(Row { row, area, session })
                };
                let mut later = Vec::new();
                for (i, rows) in agenda.later_rows.iter().enumerate() {
                    let rows = rows
                        .iter()
                        .cloned()
                        .map(row)
                        .collect::<oldbrainfix_core::Result<_>>()?;
                    later.push((agenda.later(i), rows));
                }
                // Read as the Agenda shows, never while it is drawn:
                // today's pick is written for the user's own Devices when
                // it is new, and reading again writes nothing.
                let today = frogs.pick_today(wall)?;
                let frog_rows = shown_frogs(today.shown.clone())?;
                Some(AgendaView {
                    later,
                    today: agenda
                        .today_rows
                        .into_iter()
                        .map(row)
                        .collect::<oldbrainfix_core::Result<_>>()?,
                    tomorrow: agenda
                        .tomorrow_rows
                        .into_iter()
                        .map(row)
                        .collect::<oldbrainfix_core::Result<_>>()?,
                    missed: Alarms::new(&self.store).missed(wall)?,
                    unsorted: notes.unsorted()?.len(),
                    // Counted from the pending sets, without reading each
                    // held file.
                    waiting: self
                        .store
                        .pending_sets()?
                        .iter()
                        .filter(|s| s.kind == sent::PENDING_KIND)
                        .count(),
                    offers: self
                        .store
                        .pending_sets()?
                        .iter()
                        .filter(|s| s.kind == offers::PENDING_KIND)
                        .count(),
                    frogs: Some(today),
                    frog_rows,
                })
            }
            _ => None,
        };
        let ai_task = self.store.ai_task()?;
        let mut trashed_routines = Vec::new();
        if self.selection == Selection::Trash {
            for routine in routines.trashed()? {
                let note = routines.note(&routine)?;
                trashed_routines.push((routine, note));
            }
        }
        let mut trashed_frogs = Vec::new();
        if self.selection == Selection::Trash {
            for id in store.entities_of_type(oldbrainfix_core::frogs::keys::TYPE_FROG)? {
                if let Some(frog) = frogs.by_id(&id)?
                    && frog.is_trashed()
                {
                    let note = frogs.note(&frog)?;
                    let manages = store.may_manage(store.area_of(&frog.id)?.as_deref())?;
                    trashed_frogs.push((frog, note, manages));
                }
            }
            trashed_frogs.sort_by_key(|(frog, ..)| std::cmp::Reverse(frog.trashed));
        }
        let (frogs_open, frogs_done) = if self.selection == Selection::Frogs {
            (shown_frogs(frogs.open()?)?, shown_frogs(frogs.done()?)?)
        } else {
            (Vec::new(), Vec::new())
        };
        Ok(View {
            areas,
            access,
            people,
            names,
            shown: shown_people,
            people_shown: self.store.people_shown()?,
            blocked_people: self.store.blocked_people()?,
            blocked_open: self.store.local_setting(BLOCKED_OPEN)?.as_deref() == Some("true"),
            person: match &self.dialog {
                Some(Dialog::Person { id, .. }) => Some(PersonView::read(&self.store, id)?),
                _ => None,
            },
            mine,
            may,
            removable,
            text_removable,
            remakeable,
            text_changeable,
            notes: placed,
            sections,
            compact,
            detail,
            capture_spawns: capture_choices::may_spawn(
                &self.store,
                self.capture_place().as_deref(),
            )?,
            routines: note_routines,
            note_frogs,
            tally,
            frogs_open,
            frogs_done,
            frogs_done_open: self.store.local_setting(FROGS_DONE_OPEN)?.as_deref() == Some("true"),
            agenda,
            // Each read checks every held file again: only while listed.
            held: if self.selection == Selection::SentItemsWaiting {
                self.store.held_sent_items()?
            } else {
                Vec::new()
            },
            offers: if self.selection == Selection::AreaOffersWaiting {
                self.store.held_area_offers()?
            } else {
                Vec::new()
            },
            trashed_routines,
            trashed_frogs,
            profile: OwnProfile::new(&self.store).get()?,
            profile_code: OwnProfile::new(&self.store).code()?,
            profile_question: OwnProfile::new(&self.store).question()?,
            sync_paused: self.store.sync_paused()?,
            ai_review: match &ai_task {
                Some(task) if self.selection == Selection::AiReview && self.ai_port.is_none() => {
                    Some(ai_view::read_review(&self.store, &self.t, task)?)
                }
                _ => None,
            },
            ai_task,
        })
    }

    /// Does `action`, reads the log again and announces what went wrong.
    fn apply(&mut self, action: Action) {
        let done = self.act(action);
        // The session runs exactly while the Sync view shows, or the
        // first start's adding of a Device of one's own.
        let adding = self.welcome.as_ref().is_some_and(|w| w.adding);
        if (self.selection == Selection::Sync || adding) != self.sync.hosting() {
            if self.sync.hosting() {
                self.sync.stop();
            } else {
                self.sync.start(&self.store);
            }
        }

        self.refresh();
        if let Err(e) = done {
            self.error = Some(ai_view::error_text(&self.t, &e));
        }
        if let Some(wake) = &self.wake {
            drop(wake.send(Event::Wake));
        }
    }

    fn act(&mut self, action: Action) -> oldbrainfix_core::Result<()> {
        if matches!(
            action,
            Action::NewArea
                | Action::NewTopic(_)
                | Action::Rename(_)
                | Action::StartMerge
                | Action::Share(_)
        ) {
            self.focus = true;
        }
        let notes = Notes::new(&self.store);
        let places = Places::new(&self.store);
        let selected = self.selected_note.clone().unwrap_or_default();
        match action {
            Action::Select(selection) => {
                self.save_text()?;
                // Every level has a search and a person filter of its
                // own, as on the phone.
                if selection != self.selection {
                    self.query.clear();
                    self.people = PersonFilter::default();
                }
                if let Selection::Place(id) = &selection {
                    if let Some(topic) = self.view.topic(id) {
                        self.trail.area = Some(topic.area_id.clone());
                        self.trail.topic = Some(topic.id.clone());
                    } else {
                        self.trail.area = Some(id.clone());
                    }
                }
                self.selection = selection;
            }
            Action::SelectNote(id) => {
                self.save_text()?;
                self.selected_note = Some(id);
                self.editor = None;
                self.tally_form = None;
            }
            Action::Capture(nudge_in) => {
                // A running recording is ended and kept first; held
                // until it is, so a failed save loses nothing.
                if let Some(r) = &mut self.recorder
                    && r.recording()
                    && let Some(bytes) = r.stop()
                {
                    self.recorded.push(bytes);
                }
                self.capture_note(nudge_in, &[], &[])?;
            }
            Action::CaptureChoice(asked) => self.capture_choice(asked)?,
            Action::StopRecording => {
                if let Some(bytes) = self.recorder.as_mut().and_then(|r| r.stop()) {
                    self.recorded.push(bytes);
                }
            }
            Action::Record => {
                if let Some(player) = &mut self.player {
                    player.stop();
                }
                self.playing = None;
                let started = self.recorder.as_mut().is_some_and(|r| r.start());
                if !started {
                    return Err(oldbrainfix_core::Error::Invalid(
                        self.t.microphone_missing().into(),
                    ));
                }
            }
            Action::Play(hash) => {
                let Some(player) = &mut self.player else {
                    return Ok(());
                };
                if self.playing.take().as_ref() == Some(&hash) {
                    player.stop();
                } else if let Some(bytes) = notes.recording_bytes(&hash)
                    && player.play(bytes)
                {
                    self.playing = Some(hash);
                }
            }
            Action::Speed(speed) => {
                // The one playing changes even if keeping it fails.
                self.speed = speed;
                if let Some(player) = &mut self.player {
                    player.set_speed(speed);
                }
                DeviceSettings::new(&self.store).set_playback_speed(speed)?;
            }
            Action::PastePhoto => {
                let photo = self.clipboard.as_mut().and_then(|c| c.photo());
                let Some(photo) = photo else {
                    return Err(oldbrainfix_core::Error::Invalid(
                        self.t.paste_no_picture().into(),
                    ));
                };
                self.capture_note(Notes::default_nudge(), &[photo], &[])?;
            }
            Action::PickFiles => {
                let files = (self.files_picker)(self.t.from_file_title())
                    .into_iter()
                    .filter_map(|path| {
                        let bytes = std::fs::read(&path).ok()?;
                        Some((file_name(&path), bytes))
                    })
                    .collect();
                self.capture_files(files)?;
            }
            Action::Files(files) => self.capture_files(files)?,
            Action::ShowUntranscribable => {
                let failed = std::mem::take(&mut self.untranscribable);
                if let Some((first, _)) = failed.first() {
                    let t = self.t;
                    self.dialog = Some(Dialog::Failure {
                        title: t.shared_recording_failed_title().into(),
                        cause: match first {
                            oldbrainfix_whisper::Problem::Unreadable => {
                                t.file_recording_unreadable()
                            }
                            oldbrainfix_whisper::Problem::Unsupported => {
                                t.file_recording_unsupported()
                            }
                        }
                        .into(),
                        fix: t.file_recording_fix().into(),
                        details: failed
                            .into_iter()
                            .map(|(_, line)| line)
                            .collect::<Vec<_>>()
                            .join("\n"),
                    });
                }
            }
            Action::Search => {}
            Action::FilterPeople(people) => self.people = people,
            Action::ViewPhoto(hash) => self.viewer = Some(Viewer::new(hash)),
            Action::ClosePhoto => self.viewer = None,
            Action::Unfold(id) => {
                self.unfolded.insert(id);
            }
            Action::Compact(on) => {
                if let Some(key) = self.cards_key() {
                    // Nothing kept means full, the default.
                    let setting = compact_setting(&key);
                    if on {
                        self.store.set_local_setting(&setting, "true")?;
                    } else {
                        self.store.remove_local_setting(&setting)?;
                    }
                }
                self.unfolded.clear();
            }
            Action::Speech(asked) => {
                if let Some(speech) = &self.speech {
                    match asked {
                        Speech::Download(chosen) => speech.download(chosen),
                        Speech::PickFile => {
                            if let Some(path) = (self.model_picker)(self.t.speech_model_pick()) {
                                speech.take_file(&path);
                            }
                        }
                        Speech::Cancel => speech.cancel(),
                    }
                }
            }
            Action::SaveText => self.save_text()?,
            Action::Amend => {
                notes.amend(&selected, &self.amend)?;
                self.amend.clear();
            }
            Action::Importance(importance) => notes.set_importance(&selected, importance)?,
            Action::File { note, place } => {
                // A card dragged by a Member that is not theirs, or into
                // an Area this Device was taken out of, stays (ADR 0013).
                if !self.store.may_change(&note)? || !self.store.may_write_in(&place)? {
                    return Err(oldbrainfix_core::Error::Invalid(
                        self.t.only_managers().into(),
                    ));
                }
                notes.file(&note, &place)?;
            }
            Action::Trash => {
                self.save_text()?;
                let linked = Routines::new(&self.store).of_note(&selected)?.len() as i64;
                self.dialog = Some(Dialog::TrashNote { linked });
            }
            Action::TrashNote(with_routines) => {
                Routines::new(&self.store).trash_note(&selected, with_routines)?;
                self.dialog = None;
            }
            Action::Restore => Routines::new(&self.store).restore_note(&selected)?,
            Action::Merge(into) => {
                self.save_text()?;
                Notes::new(&self.store).merge(&selected, &into)?;
                self.dialog = None;
                self.selected_note = Some(into);
            }
            Action::NewArea => {
                self.dialog = Some(Dialog::NewPlace {
                    area: None,
                    name: String::new(),
                });
            }
            Action::NewTopic(area) => {
                self.dialog = Some(Dialog::NewPlace {
                    area: Some(area),
                    name: String::new(),
                });
            }
            Action::Rename(id) => {
                let shown = self.shown_name(&id);
                self.dialog = Some(Dialog::Rename {
                    id,
                    name: shown.clone(),
                    shown,
                });
            }
            Action::Look(area) => self.dialog = Some(Dialog::Look { area }),
            Action::Delete(id) => self.dialog = Some(Dialog::Delete(self.deleting(&id)?)),
            Action::StartMerge => {
                let others = self.merge_candidates("", &selected)?;
                self.dialog = Some(Dialog::Merge {
                    query: String::new(),
                    others,
                });
            }
            Action::MergeQuery => {
                if let Some(Dialog::Merge { query, .. }) = &self.dialog {
                    let found = self.merge_candidates(&query.clone(), &selected)?;
                    if let Some(Dialog::Merge { others, .. }) = &mut self.dialog {
                        *others = found;
                    }
                }
            }
            Action::CloseDialog => self.close_dialog()?,
            Action::AddPlace => {
                if let Some(Dialog::NewPlace { area, name }) = &self.dialog {
                    match area {
                        Some(area) => drop(places.add_topic(area, name)?),
                        None => drop(places.add_area(name)?),
                    }
                    self.dialog = None;
                }
            }
            Action::SaveName => {
                if let Some(Dialog::Rename { id, name, shown }) = &self.dialog {
                    // An unchanged starter name stays translatable.
                    if name.trim() != shown {
                        places.rename(id, name)?;
                    }
                    self.dialog = None;
                }
            }
            Action::Colour(area, colour) => places.set_colour(&area, colour.as_deref())?,
            Action::Icon(area, icon) => places.set_icon(&area, icon.as_deref())?,
            Action::DeletePlace(go) => {
                if let Some(Dialog::Delete(deleting)) = &self.dialog {
                    if deleting.area.is_none() {
                        places.delete_area(&deleting.id, go)?;
                    } else {
                        places.delete_topic(&deleting.id, go)?;
                    }
                    self.dialog = None;
                }
            }
            Action::Back => {
                if let Some(back) = self.back() {
                    return self.act(Action::Select(back));
                }
            }
            Action::Forward => {
                if let Some(forward) = self.forward() {
                    return self.act(Action::Select(forward));
                }
            }
            Action::Toggle(routine, occurrence) => {
                let routines = Routines::new(&self.store);
                let done = routines
                    .by_id(&routine)?
                    .is_some_and(|r| r.done_at(&occurrence).is_some());
                if done {
                    routines.unmark_done(&routine, &occurrence)?;
                } else {
                    routines.mark_done(&routine, &occurrence, None)?;
                }
            }
            Action::CheckMissed(routine, occurrence) => {
                Routines::new(&self.store).mark_done(&routine, &occurrence, None)?;
            }
            Action::Seen => Alarms::new(&self.store).mark_seen()?,
            Action::OpenRoutine(id) => {
                self.save_text()?;
                let routines = Routines::new(&self.store);
                // What only a Manager may change shows its Note instead.
                if !self.store.may_change_routine(&id)? {
                    if let Some(routine) = routines.by_id(&id)? {
                        self.editor = None;
                        self.selected_note = Some(routine.note_id);
                    }
                    return Ok(());
                }
                if let Some(routine) = routines.by_id(&id)? {
                    let note = routines.note(&routine)?;
                    let today = Day::today(self.store.now(), &*self.wall);
                    let renag = DeviceSettings::new(&self.store).renag_interval()?;
                    let mut editor = RoutineEditor::edit(&routine, note, today, renag);
                    if let Some(session) = Sessions::new(&self.store).by_id(&id)? {
                        let draft = SessionDraft::of(&self.store, &self.t, &id, Some(&session))?;
                        editor = editor.with_session(draft, Some(session));
                    }
                    self.editor = Some(editor);
                    self.tally_form = None;
                }
            }
            Action::NewRoutine { note, reminder } => {
                self.save_text()?;
                let note = match note {
                    Some(id) => Notes::new(&self.store).by_id(&id)?,
                    None => None,
                };
                let renag = DeviceSettings::new(&self.store).renag_interval()?;
                self.tally_form = None;
                self.editor = Some(RoutineEditor::new(
                    note,
                    reminder,
                    self.store.now(),
                    &*self.wall,
                    renag,
                ));
            }
            Action::NewSession(id) => {
                self.save_text()?;
                let note = Notes::new(&self.store).by_id(&id)?;
                let renag = DeviceSettings::new(&self.store).renag_interval()?;
                let draft = SessionDraft::of(&self.store, &self.t, &id, None)?;
                self.tally_form = None;
                let editor = RoutineEditor::new(note, false, self.store.now(), &*self.wall, renag);
                self.editor = Some(editor.with_session(draft, None));
            }
            Action::Session(asked) => self.session_action(asked)?,
            Action::Editor(intent) => match intent {
                Intent::Save => match &mut self.editor {
                    // Set up for the Note being captured: held with it.
                    Some(editor) if editor.capturing() => {
                        if let Some(choice) = editor.choice(&self.store)? {
                            self.capture_choices.schedules.push(choice);
                            self.editor = None;
                        }
                    }
                    Some(editor) => {
                        if editor.save(&self.store)?.is_some() {
                            self.editor = None;
                        }
                    }
                    None => {}
                },
                Intent::Session(SessionIntent::WeekdaysOpen(open)) => {
                    let value = if open { "true" } else { "false" };
                    self.store.set_local_setting(WEEKDAYS_OPEN, value)?;
                }
                Intent::Delete => {
                    let reminder = self.editor.as_ref().is_some_and(RoutineEditor::once);
                    let question = if reminder {
                        self.t.delete_reminder_question()
                    } else {
                        self.t.delete_routine_question()
                    };
                    self.dialog = Some(Dialog::DeleteRoutine {
                        question: question.to_string(),
                    });
                }
                Intent::Close => self.editor = None,
                Intent::OpenNote(id) => {
                    self.editor = None;
                    self.selected_note = Some(id);
                }
            },
            Action::DeleteRoutine => {
                self.dialog = None;
                if let Some(id) = self.editor.take().and_then(|e| e.routine) {
                    Routines::new(&self.store).trash(&id)?;
                }
            }
            Action::RestoreRoutine(id) => Routines::new(&self.store).restore(&id)?,
            Action::Frog(asked) => self.frog_action(asked)?,
            Action::SpawnFrog(note) => {
                self.save_text()?;
                Frogs::new(&self.store).spawn(&note)?;
            }
            Action::DeleteFrog => {
                if let Some(Dialog::DeleteFrog { id }) = self.dialog.take() {
                    Frogs::new(&self.store).trash(&id)?;
                }
            }
            Action::RestoreFrog(id) => Frogs::new(&self.store).restore(&id)?,
            Action::Tally(asked) => self.tally_action(asked)?,
            Action::TallyForm(intent) => self.tally_form_action(intent)?,
            Action::DeleteEntry => {
                if let Some(Dialog::DeleteEntry { id }) = self.dialog.take() {
                    let tallies = Tallies::new(&self.store);
                    // A sync while the dialog was open may have taken it.
                    if tallies.entry_by_id(&id)?.is_some_and(|e| !e.is_trashed()) {
                        tallies.trash_entry(&id)?;
                    }
                    if let Some(TallyForm::Entry(form)) = &self.tally_form
                        && form.entry.as_ref().is_some_and(|e| e.id == id)
                    {
                        self.tally_form = None;
                    }
                }
            }
            Action::Setting(setting) => self.set(setting)?,
            Action::PickAlarmSound => {
                if let Some(path) = (self.sound_picker)(self.t.alarm_sound_pick()) {
                    let path = path.to_string_lossy().into_owned();
                    self.set(Setting::AlarmSound(Some(path)))?;
                }
            }
            Action::Sync(SyncAction::Trust(trust)) => {
                if self.sync.answer(&self.store, trust, &self.sync_name) {
                    // A Moment and the new entries: read again below.
                    self.refresh();
                }
                // Turned away on the first start: back to its choices.
                if !trust && self.welcome.as_ref().is_some_and(|w| w.adding) {
                    self.welcome_action(WelcomeAction::Back)?;
                }
            }
            Action::Sync(SyncAction::Forget(id)) => self.store.forget_device(&id)?,
            Action::Sync(SyncAction::PickFolder) => {
                if let Some(path) = (self.folder_picker)(self.t.folder_pick_title()) {
                    self.store.choose_sync_folder(&path.to_string_lossy())?;
                    self.folder_now()?;
                }
            }
            Action::Sync(SyncAction::ForgetFolder) => self.store.forget_sync_folder()?,
            Action::Sync(SyncAction::FolderNow) => self.folder_now()?,
            Action::Sync(SyncAction::SaveBundle(device)) => self.save_bundle(device)?,
            Action::Sync(SyncAction::OpenBundle) => {
                if let Some(path) = (self.bundle_opener)(self.t.bundle_open_title()) {
                    self.open_file(&path);
                }
            }
            Action::OpenFile(path) => self.open_file(&path),
            Action::SendWaitingChanges => self.send_waiting_changes()?,
            Action::Backup(asked) => self.backup(asked)?,
            Action::Profile(asked) => self.profile_action(asked)?,
            Action::OpenPerson(id) => self.open_person(id, SendBack::No)?,
            Action::Person(asked) => self.person_action(asked)?,
            Action::Welcome(asked) => self.welcome_action(asked)?,
            Action::Ai(asked) => self.ai_action(asked)?,
            Action::StartSend => {
                self.save_text()?;
                let recipients = self.store.sent_item_recipients()?;
                let picked = (!recipients.is_empty()).then_some(0);
                let update = self.store.was_sent(&selected)?;
                self.dialog = Some(Dialog::Send {
                    note: selected,
                    recipients,
                    picked,
                    update,
                    cancel: false,
                });
            }
            Action::SendNote => self.send_note()?,
            Action::AcceptSent => match self.dialog.take() {
                Some(Dialog::HeldSentItem { held, .. }) => self.accept_held(&held.id, false)?,
                Some(Dialog::SentItem {
                    compared,
                    held: Some(id),
                    ..
                }) => self.accept_held(&id, compared == Compared::Match)?,
                Some(Dialog::SentItem {
                    opened, compared, ..
                }) => {
                    // The alerts plan its alarms once the action is done.
                    let accepted = self
                        .store
                        .accept_sent_item(&opened.item, compared == Compared::Match);
                    if let Err(oldbrainfix_core::Error::FromBlocked) = accepted {
                        // Its sender was blocked since it opened.
                        self.notice = Some(self.t.sent_item_from_blocked().to_string());
                        return Ok(());
                    }
                    accepted?;
                    let said = match opened.state {
                        SentItemState::Update => self.t.sent_item_applied(),
                        SentItemState::Cancel => self.t.sent_item_trashed(),
                        _ => self.t.sent_item_accepted(),
                    };
                    self.notice = Some(said.to_string());
                }
                other => self.dialog = other,
            },
            Action::DeclineSent => {
                let previewed = match &self.dialog {
                    Some(Dialog::SentItem { opened, .. }) => acceptable(opened.state),
                    Some(Dialog::HeldSentItem { .. }) => true,
                    _ => false,
                };
                // A stranger's item goes, Close on one with nothing to
                // accept included.
                if let Some(
                    Dialog::SentItem { held: Some(id), .. }
                    | Dialog::HeldSentItem {
                        held: HeldSentItem { id, .. },
                        ..
                    },
                ) = self.dialog.take()
                {
                    self.store.decline_pending(&id)?;
                }
                if previewed {
                    self.notice = Some(self.t.sent_item_declined().to_string());
                }
            }
            Action::ShowSent => self.show_held()?,
            Action::OpenHeld(id) => self.open_held(&id)?,
            Action::AskBlockSender => match &mut self.dialog {
                Some(Dialog::SentItem {
                    opened, blocking, ..
                }) => {
                    *blocking = Some(self.store.areas_to_leave_for_blocking(&opened.sender.id)?);
                }
                Some(Dialog::HeldSentItem { held, blocking }) => {
                    *blocking = Some(self.store.areas_to_leave_for_blocking(&held.sender.id)?);
                }
                Some(Dialog::AreaOffer { held, blocking, .. }) => {
                    *blocking = Some(self.store.areas_to_leave_for_blocking(&held.sender.id)?);
                }
                _ => {}
            },
            Action::Block => self.block()?,
            Action::Unblock => {
                if let Some(Dialog::Person { id, asking, .. }) = &mut self.dialog {
                    self.store.unblock_person(id)?;
                    *asking = None;
                }
            }
            Action::CancelBlock => match &mut self.dialog {
                Some(
                    Dialog::SentItem { blocking, .. }
                    | Dialog::HeldSentItem { blocking, .. }
                    | Dialog::AreaOffer { blocking, .. },
                ) => *blocking = None,
                Some(Dialog::Person { asking, .. }) => *asking = None,
                _ => {}
            },
            Action::BlockedOpen(open) => {
                self.store
                    .set_local_setting(BLOCKED_OPEN, if open { "true" } else { "false" })?;
            }
            Action::CompareCodes => {
                if let Some(
                    Dialog::SentItem { compared, .. } | Dialog::AreaOffer { compared, .. },
                ) = &mut self.dialog
                {
                    *compared = Compared::Asking;
                }
            }
            Action::CodesCompared(matched) => {
                if let Some(
                    Dialog::SentItem { compared, .. } | Dialog::AreaOffer { compared, .. },
                ) = &mut self.dialog
                {
                    *compared = if matched {
                        Compared::Match
                    } else {
                        Compared::Differ
                    };
                }
            }
            Action::Share(area) => {
                let afar = self.afar_people()?;
                self.dialog = Some(Dialog::Share {
                    area,
                    role: AreaRole::Member,
                    afar,
                });
            }
            Action::ShareAfar => {
                if let Some(Dialog::Share { area, role, afar }) = self.dialog.take() {
                    self.dialog = Some(Dialog::ShareAfar {
                        area,
                        role,
                        people: afar,
                        person: None,
                        transport: Transport::Bundle,
                    });
                }
            }
            Action::ShareTo(id) => {
                let known = self.store.transport_with(&id)?;
                if let Some(Dialog::ShareAfar { person, .. }) = &mut self.dialog {
                    *person = Some(id);
                }
                // Asked once per person: a second Area goes the same way.
                if known.is_some() {
                    self.share_afar()?;
                }
            }
            Action::ShareAfarNow => self.share_afar()?,
            Action::OpenOffer(id) => match self.store.held_area_offer(&id)? {
                Some(held) => {
                    self.dialog = Some(Dialog::AreaOffer {
                        held,
                        compared: Compared::Not,
                        blocking: None,
                    });
                }
                None => self.notice = Some(self.t.area_offer_gone().to_string()),
            },
            Action::AcceptOffer => self.accept_offer()?,
            Action::DeclineOffer => {
                if let Some(Dialog::AreaOffer { held, .. }) = self.dialog.take() {
                    self.store.decline_area_offer(&held.id)?;
                    self.notice = Some(self.t.sent_item_declined().to_string());
                }
            }
            Action::OfferFolder => {
                self.dialog = None;
                self.act(Action::Sync(SyncAction::PickFolder))?;
            }
            Action::StartInvite => {
                if let Some(Dialog::Share { area, role, .. }) = self.dialog.clone() {
                    let area = self.store.share(&area)?;
                    self.dialog = None;
                    self.save_text()?;
                    self.query.clear();
                    self.selection = Selection::Sync;
                    self.sync.start_inviting(&self.store, Invite { area, role });
                }
            }
            Action::Sync(SyncAction::StopInviting) => self.sync.stop_inviting(&self.store),
            Action::Sync(SyncAction::Join) => {
                self.sync.join(self.store.clone(), &self.sync_name, &self.t)
            }
            Action::People(area) => self.dialog = Some(Dialog::People { area }),
            Action::SetRole { area, person, role } => self.store.set_role(&area, &person, role)?,
            Action::AskRemovePerson { area, person } => {
                let name = self
                    .view
                    .people
                    .get(&area)
                    .and_then(|people| people.iter().find(|p| p.id == person))
                    .map(|p| person_name(&self.t, &self.view.names, p))
                    .unwrap_or_default();
                let shown = self.shown_name(&area);
                self.dialog = Some(Dialog::RemovePerson {
                    question: self.t.remove_person_question(&name, &shown),
                    area,
                    person,
                });
            }
            Action::RemovePerson => {
                if let Some(Dialog::RemovePerson { area, person, .. }) = self.dialog.take() {
                    self.store.remove_person(&area, &person)?;
                    self.dialog = Some(Dialog::People { area });
                }
            }
            Action::AskRemoveMedia { note, hash, photo } => {
                self.save_text()?;
                self.dialog = Some(Dialog::RemoveMedia { note, hash, photo });
            }
            Action::RemoveMedia => {
                if let Some(Dialog::RemoveMedia { note, hash, photo }) = self.dialog.take() {
                    // A sync while the dialog was open may have taken it
                    // already.
                    if photo {
                        if notes.may_remove_photo(&note, &hash)? {
                            notes.remove_photo(&note, &hash)?;
                        }
                    } else if notes.may_remove_recording(&note, &hash)? {
                        if self.playing.as_deref() == Some(hash.as_str()) {
                            if let Some(player) = &mut self.player {
                                player.stop();
                            }
                            self.playing = None;
                        }
                        notes.remove_recording(&note, &hash)?;
                    }
                }
            }
            Action::TakeOver {
                note,
                hash,
                photo,
                append,
            } => {
                self.save_text()?;
                let notes = Notes::new(&self.store);
                let Some(target) = notes.by_id(&note)? else {
                    return Ok(());
                };
                let texts = if photo {
                    &target.photo_texts
                } else {
                    &target.transcripts
                };
                let Some(generated) = texts.get(&hash).filter(|g| !g.trim().is_empty()) else {
                    return Ok(());
                };
                if !target.text_editable || !self.store.may_change(&note)? {
                    return Ok(());
                }
                // As the phone joins them.
                let text = &target.text;
                let joined = if !append || text.trim().is_empty() {
                    generated.clone()
                } else {
                    format!("{}\n{}", text.trim_end(), generated)
                };
                notes.edit(&note, &joined)?;
                if self.text_of.as_deref() == Some(note.as_str()) {
                    self.text = joined.trim().to_string();
                }
            }
            Action::AskRemoveText { note, hash, photo } => {
                self.save_text()?;
                self.dialog = Some(Dialog::RemoveText { note, hash, photo });
            }
            Action::RemoveText => {
                if let Some(Dialog::RemoveText { note, hash, photo }) = self.dialog.take() {
                    // A sync while the dialog was open may have taken it
                    // already.
                    if photo {
                        if notes.may_remove_photo_text(&note, &hash)? {
                            notes.remove_photo_text(&note, &hash)?;
                        }
                    } else if notes.may_remove_transcript(&note, &hash)? {
                        notes.remove_transcript(&note, &hash)?;
                    }
                }
            }
            Action::Again { note, hash, photo } => {
                self.save_text()?;
                let enricher = if photo {
                    &self.reader
                } else {
                    &self.transcriber
                };
                if let Some(enricher) = enricher {
                    enricher.again(Blob {
                        note_id: note,
                        hash,
                    });
                }
            }
            Action::ShowAgainFailed(photo) => {
                self.again_failed = None;
                let t = self.t;
                self.dialog = Some(if photo {
                    Dialog::Failure {
                        title: t.read_text_again_failed_title().into(),
                        cause: t.read_text_again_failed_cause().into(),
                        fix: t.generated_text_again_fix().into(),
                        details: "read again: no Photo Text".into(),
                    }
                } else {
                    Dialog::Failure {
                        title: t.transcribe_again_failed_title().into(),
                        cause: t.transcribe_again_failed_cause().into(),
                        fix: t.generated_text_again_fix().into(),
                        details: "transcribe again: no Transcript".into(),
                    }
                });
            }
            Action::Sync(SyncAction::AutoSync(id, on)) => self.store.set_auto_sync(&id, on)?,
            Action::Sync(SyncAction::Keep {
                entity,
                unit,
                index,
            }) => {
                if let Some(conflict) = self.store.conflict_on(&entity, &unit)?
                    && let Some(version) = conflict.versions.get(index)
                {
                    self.store.resolve_conflict(&conflict, version)?;
                }
                self.sync.reload_conflicts(&self.store);
                self.refresh();
            }
        }
        Ok(())
    }

    /// Captures the capture field's text with `photos` and `recordings`
    /// as a new Note nudged `nudge_in` from now, and shows it; nothing
    /// when all of them are empty.
    /// Does what was asked of a Frog or the Frog view (#101).
    fn frog_action(&mut self, asked: FrogAction) -> oldbrainfix_core::Result<()> {
        let frogs = Frogs::new(&self.store);
        match asked {
            FrogAction::Toggle(id) => {
                if frogs.by_id(&id)?.is_some_and(|f| f.is_done()) {
                    frogs.unmark_done(&id)?;
                } else {
                    frogs.mark_done(&id, None)?;
                }
            }
            FrogAction::Another(id) => frogs.another(&*self.wall, &id)?,
            FrogAction::OpenNote(id) => return self.act(Action::SelectNote(id)),
            FrogAction::AskDelete(id) => self.dialog = Some(Dialog::DeleteFrog { id }),
            FrogAction::OpenView => return self.act(Action::Select(Selection::Frogs)),
            FrogAction::DoneOpen(open) => {
                let value = if open { "true" } else { "false" };
                self.store.set_local_setting(FROGS_DONE_OPEN, value)?;
            }
        }
        Ok(())
    }

    /// Does what was asked of a Session's row on the Agenda (#104): a
    /// pick's entry form opens on the right with its Tally's Note
    /// selected, which shows once it is saved.
    fn session_action(&mut self, asked: SessionAction) -> oldbrainfix_core::Result<()> {
        match asked {
            SessionAction::Open {
                session,
                occurrence,
                note_id,
                entry,
            } => {
                let tallies = Tallies::new(&self.store);
                let Some(tally) = tallies.by_note(&note_id)? else {
                    return Ok(());
                };
                let entry = match entry {
                    Some(id) => match tallies.entry_by_id(&id)? {
                        Some(entry) if !entry.is_trashed() => {
                            let changes = self.store.may_change(&id)?;
                            Some((entry, changes))
                        }
                        _ => return Ok(()),
                    },
                    None => None,
                };
                let today = Day::today(self.store.now(), &*self.wall);
                let system = self.unit_system()?;
                self.save_text()?;
                self.editor = None;
                self.selected_note = Some(note_id);
                let form = EntryForm::new(&tally, entry, (system, today), &*self.wall, &self.t);
                self.tally_form = Some(TallyForm::Entry(form.for_pick(&session, occurrence)));
            }
            SessionAction::Another {
                session,
                occurrence,
                note_id,
            } => Sessions::new(&self.store).another(&session, &occurrence, &note_id)?,
        }
        Ok(())
    }

    /// Does what was asked of the Tally part of the Note's editor (#103).
    fn tally_action(&mut self, asked: TallyAction) -> oldbrainfix_core::Result<()> {
        let tallies = Tallies::new(&self.store);
        let system = self.unit_system()?;
        let note = self.selected_note.clone().unwrap_or_default();
        match asked {
            TallyAction::Make(id) => {
                self.save_text()?;
                self.editor = None;
                self.tally_form =
                    Some(TallyForm::Field(FieldForm::new(&id, None, system, &self.t)));
            }
            TallyAction::OpenField(field) => {
                let field = match field {
                    Some(id) => tallies.by_note(&note)?.and_then(|t| t.field(&id).cloned()),
                    None => None,
                };
                self.save_text()?;
                self.editor = None;
                self.tally_form = Some(TallyForm::Field(FieldForm::new(
                    &note, field, system, &self.t,
                )));
            }
            TallyAction::AddEntry | TallyAction::EditEntry(_) => {
                let Some(tally) = tallies.by_note(&note)? else {
                    return Ok(());
                };
                let entry = match &asked {
                    TallyAction::EditEntry(id) => match tallies.entry_by_id(id)? {
                        Some(entry) if !entry.is_trashed() => {
                            let changes = self.store.may_change(id)?;
                            Some((entry, changes))
                        }
                        _ => return Ok(()),
                    },
                    _ => None,
                };
                let today = Day::today(self.store.now(), &*self.wall);
                self.save_text()?;
                self.editor = None;
                self.tally_form = Some(TallyForm::Entry(EntryForm::new(
                    &tally,
                    entry,
                    (system, today),
                    &*self.wall,
                    &self.t,
                )));
            }
            TallyAction::AskDelete(id) => self.dialog = Some(Dialog::DeleteEntry { id }),
            TallyAction::Restore(id) => tallies.restore_entry(&id)?,
            TallyAction::Whose(whose) => self.tally_look.whose = Some(whose),
            TallyAction::Period(period) => self.tally_look.period = period,
            TallyAction::More => self.tally_look.count += tally_view::PAGE,
            TallyAction::RemovedOpen(open) => {
                let value = if open { "true" } else { "false" };
                self.store
                    .set_local_setting(tally_view::REMOVED_OPEN, value)?;
            }
        }
        Ok(())
    }

    /// Does what was asked of the field page or the entry page.
    fn tally_form_action(&mut self, intent: FormIntent) -> oldbrainfix_core::Result<()> {
        let Some(form) = &mut self.tally_form else {
            return Ok(());
        };
        let form_capturing = form.capturing();
        match intent {
            FormIntent::Save => {
                let saved = match form {
                    // The first field of the Note being captured: held
                    // with it.
                    TallyForm::Field(form) if form_capturing => match form.choice() {
                        Some(field) => {
                            self.capture_choices.tally = Some(field);
                            true
                        }
                        None => false,
                    },
                    TallyForm::Field(form) => form.save(&self.store)?,
                    TallyForm::Entry(form) => form.save(&self.store, &*self.wall)?,
                };
                if saved {
                    self.tally_form = None;
                }
            }
            FormIntent::Close => self.tally_form = None,
            FormIntent::Hide(hidden) => {
                if let TallyForm::Field(form) = form {
                    form.set_hidden(&self.store, hidden)?;
                }
                self.tally_form = None;
            }
            FormIntent::Delete => {
                if let TallyForm::Entry(form) = form
                    && let Some(entry) = &form.entry
                {
                    self.dialog = Some(Dialog::DeleteEntry {
                        id: entry.id.clone(),
                    });
                }
            }
        }
        Ok(())
    }

    fn capture_note(
        &mut self,
        nudge_in: Duration,
        photos: &[Vec<u8>],
        recordings: &[Vec<u8>],
    ) -> oldbrainfix_core::Result<()> {
        let place = self.capture_place();
        // The recordings held on the field go into the same Note.
        let mut recordings = recordings.to_vec();
        recordings.extend(self.recorded.iter().cloned());
        let Some(note) = capture_choices::capture(
            &self.store,
            &self.capture,
            place.as_deref(),
            nudge_in,
            (photos, &recordings),
            &self.capture_choices,
        )?
        else {
            return Ok(());
        };
        // Only once all of it is written: a failed save keeps it here.
        self.capture.clear();
        self.recorded.clear();
        self.capture_choices = CaptureChoices::default();
        self.save_text()?;
        self.selected_note = Some(note.id);
        self.editor = None;
        self.tally_form = None;
        Ok(())
    }

    /// Where the Note being captured goes: the Area or Topic picked with
    /// "File into…", else the one shown, as the phone's "Note here";
    /// Unsorted without either.
    fn capture_place(&self) -> Option<String> {
        self.capture_choices
            .place
            .clone()
            .or_else(|| match &self.selection {
                Selection::Place(id) => Some(id.clone()),
                _ => None,
            })
    }

    /// Does what was asked of the capture's buttons or its choices: a
    /// Routine, Reminder, Session or Tally opens its page on the right,
    /// which hands its choice back to the capture (#113).
    fn capture_choice(&mut self, asked: Ask) -> oldbrainfix_core::Result<()> {
        let place = self.capture_place();
        match self.capture_choices.take(asked, &self.store)? {
            None => {}
            Some(Opens::Schedule { reminder, session }) => {
                self.save_text()?;
                let renag = DeviceSettings::new(&self.store).renag_interval()?;
                let now = self.store.now();
                let mut editor =
                    RoutineEditor::choose(&self.capture, reminder, now, &*self.wall, renag);
                if session {
                    // The Tallies of where it goes; while Unsorted, any.
                    let entity = place.as_deref().unwrap_or("");
                    let draft = SessionDraft::of(&self.store, &self.t, entity, None)?;
                    editor = editor.with_session(draft, None);
                }
                self.tally_form = None;
                self.editor = Some(editor);
            }
            Some(Opens::Tally) => {
                self.save_text()?;
                self.editor = None;
                let system = self.unit_system()?;
                self.tally_form = Some(TallyForm::Field(FieldForm::choose(system, &self.t)));
            }
        }
        Ok(())
    }

    /// Makes `files`, by name and content, into one Note with the capture
    /// field's text, as a share does on the phone: the pictures as
    /// photos, the sound files as voice recordings, each kept even when
    /// it cannot be transcribed here, which is announced at the top.
    /// Anything else is left alone; nothing when nothing is left.
    fn capture_files(&mut self, files: Vec<(String, Vec<u8>)>) -> oldbrainfix_core::Result<()> {
        let (mut photos, mut recordings, mut failed) = (Vec::new(), Vec::new(), Vec::new());
        for (name, bytes) in files {
            match from_file(&name, &bytes) {
                Some(FromFile::Photo(photo)) => photos.push(photo),
                Some(FromFile::Recording { bytes, problem }) => {
                    if let Some(problem) = problem {
                        failed.push((problem, problem_line(problem, &name)));
                    }
                    recordings.push(bytes);
                }
                None => {}
            }
        }
        if photos.is_empty() && recordings.is_empty() {
            return Ok(());
        }
        self.capture_note(Notes::default_nudge(), &photos, &recordings)?;
        // Added to what still waits to be clicked, so none goes unsaid.
        self.untranscribable.extend(failed);
        Ok(())
    }

    /// The Notes the Note `selected` can be merged into that the core's
    /// search finds for `query`, by id and the title they show under.
    fn merge_candidates(
        &self,
        query: &str,
        selected: &str,
    ) -> oldbrainfix_core::Result<Vec<(String, String)>> {
        Ok(Search::new(&self.store)
            .notes(query, None)?
            .into_iter()
            .filter(|n| n.id != selected)
            .map(|n| (n.id.clone(), row_title(&self.t, Some(&n))))
            .collect())
    }

    /// Keeps a change made in the Settings.
    fn set(&mut self, setting: Setting) -> oldbrainfix_core::Result<()> {
        let settings = DeviceSettings::new(&self.store);
        match setting {
            Setting::Alerts(on) => settings.set_alerts_on(on)?,
            Setting::Autostart(on) => {
                settings.set_autostart_on(on)?;
                if let Some(autostart) = &self.autostart
                    && let Err(e) = autostart.set(on)
                {
                    return Err(oldbrainfix_core::Error::Invalid(
                        self.t.autostart_failed(&e),
                    ));
                }
            }
            Setting::Renag(minutes) => settings.set_renag_interval(Duration::minutes(minutes))?,
            Setting::NudgeRepeat(minutes) => {
                settings.set_nudge_repeat(Duration::minutes(minutes))?;
            }
            Setting::QuietOn(on) => settings.set_quiet_hours_on(on)?,
            Setting::Quiet(quiet) => settings.set_quiet_hours(quiet)?,
            Setting::Hotkey(chord) => {
                settings.set_capture_hotkey(&chord)?;
                if let (Some(hotkey), Ok(chord)) = (&self.hotkey, chord.parse::<Chord>()) {
                    hotkey.borrow_mut().rebind(&chord);
                }
            }
            Setting::AlarmSound(sound) => settings.set_alarm_sound(sound.as_deref())?,
            Setting::Units(system) => settings.set_chosen_unit_system(system)?,
        }
        Ok(())
    }

    /// Keeps the edited text of the shown Note when it changed; an
    /// emptied text goes back to what it was.
    fn save_text(&mut self) -> oldbrainfix_core::Result<()> {
        let Some((placed, _)) = &self.view.detail else {
            return Ok(());
        };
        let note = &placed.note;
        // A text in a format not known here is never overwritten.
        if self.text_of.as_deref() != Some(note.id.as_str())
            || note.is_trashed()
            || !note.text_editable
        {
            return Ok(());
        }
        let text = self.text.trim();
        if text.is_empty() {
            self.text = note.text.clone();
        } else if text != note.text {
            Notes::new(&self.store).edit(&note.id, text)?;
            self.text = text.to_string();
        }
        Ok(())
    }

    /// One level up: a Topic to its Area; an Area, Unsorted or the trash
    /// to All Notes.
    fn back(&self) -> Option<Selection> {
        match &self.selection {
            Selection::All | Selection::Agenda => None,
            Selection::Profile | Selection::SentItemsWaiting | Selection::AreaOffersWaiting => {
                Some(Selection::Agenda)
            }
            Selection::Settings | Selection::Sync => Some(Selection::Profile),
            Selection::AiReview => Some(Selection::Settings),
            Selection::Frogs => Some(Selection::Agenda),
            Selection::About => Some(Selection::Settings),
            Selection::Licences => Some(Selection::About),
            Selection::Place(id) => Some(match self.view.topic(id) {
                Some(topic) => Selection::Place(topic.area_id.clone()),
                None => Selection::All,
            }),
            Selection::Unsorted | Selection::Trash => Some(Selection::All),
        }
    }

    /// Into the last selected Area, or from an Area into its last
    /// selected Topic.
    fn forward(&self) -> Option<Selection> {
        match &self.selection {
            Selection::Place(id) if self.view.area(id).is_some() => self
                .trail
                .topic
                .as_ref()
                .filter(|t| self.view.topic(t).is_some_and(|t| t.area_id == *id))
                .map(|t| Selection::Place(t.clone())),
            Selection::Place(_) => None,
            _ => self
                .trail
                .area
                .as_ref()
                .filter(|a| self.view.area(a).is_some())
                .map(|a| Selection::Place(a.clone())),
        }
    }

    /// The name the Area or Topic `id` shows under.
    fn shown_name(&self, id: &str) -> String {
        if let Some(area) = self.view.area(id) {
            return area_name(&self.t, area);
        }
        self.view
            .topic(id)
            .map(|t| topic_name(&self.t, t))
            .unwrap_or_default()
    }

    /// The question before the Area or Topic `id` goes, as the phone asks
    /// it.
    fn deleting(&self, id: &str) -> oldbrainfix_core::Result<Deleting> {
        let t = &self.t;
        let places = Places::new(&self.store);
        let count = places.notes_inside(id)?;
        let mut lines = Vec::new();
        let (question, area) = match self.view.topic(id) {
            Some(topic) => (
                t.delete_topic_question(&topic_name(t, topic)),
                Some(self.shown_name(&topic.area_id)),
            ),
            None => {
                if !places.topics(id)?.is_empty() {
                    lines.push(t.topics_go_too().to_string());
                }
                (t.delete_area_question(&self.shown_name(id)), None)
            }
        };
        if count > 0 {
            lines.push(t.notes_inside(count as i64));
            lines.push(t.where_notes_go().to_string());
        }
        Ok(Deleting {
            id: id.to_string(),
            question,
            lines,
            area,
            has_notes: count > 0,
        })
    }

    /// The first start's view instead of the panes: the three choices, a
    /// Backup being restored, or a Device of one's own being added with
    /// the Sync view's ways to pair. Back, as key or mouse button, goes
    /// to the choices.
    fn welcome_view(&mut self, ui: &mut Ui, tones: Tones, actions: Vec<Action>) {
        let (t, wall) = (self.t, self.wall.clone());
        let mut asked = Vec::new();
        let mut synced = Vec::new();
        if actions.contains(&Action::Back) {
            asked.push(WelcomeAction::Back);
        }
        // What the earlier part of the frame decided about a join.
        let actions = actions
            .into_iter()
            .filter(|a| matches!(a, Action::Welcome(_)));
        let (joining, hosted) = (self.sync.joining(), self.sync.synced());
        let sync = &mut self.sync;
        if let Some(welcome) = &mut self.welcome {
            egui::CentralPanel::default()
                .frame(Frame::NONE.inner_margin(Margin::same(24)))
                .show(ui, |ui| {
                    ui.set_max_width(640.0);
                    egui::ScrollArea::vertical()
                        .id_salt("welcome")
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            embossed(ui, tones, false, None, |ui| {
                                if !welcome.adding {
                                    backup_view::welcome(ui, &t, &wall, welcome, &mut asked);
                                    return;
                                }
                                let text = ui.visuals().text_color();
                                let weak = ui.visuals().weak_text_color();
                                ui.heading(RichText::new(t.first_start_device()).color(text));
                                ui.label(RichText::new(t.first_start_device_detail()).color(weak));
                                if let Some(action) = sync.pairing(ui, &t, tones) {
                                    synced.push(Action::Sync(action));
                                }
                                ui.horizontal(|ui| {
                                    // A session that went through: on, once
                                    // the other Device fetched what it needs.
                                    if hosted
                                        && !joining
                                        && ui
                                            .button(
                                                RichText::new(t.host_continue())
                                                    .strong()
                                                    .color(text),
                                            )
                                            .clicked()
                                    {
                                        asked.push(WelcomeAction::Synced);
                                    }
                                    if !hosted
                                        && ui
                                            .add_enabled(
                                                !joining,
                                                egui::Button::new(
                                                    RichText::new(t.cancel()).color(text),
                                                ),
                                            )
                                            .clicked()
                                    {
                                        asked.push(WelcomeAction::Back);
                                    }
                                });
                            });
                        });
                });
        }
        let all: Vec<Action> = actions
            .chain(synced)
            .chain(asked.into_iter().map(Action::Welcome))
            .collect();
        for action in all {
            self.apply(action);
        }
    }

    /// Draws one frame into `ui` and does what was asked in it.
    pub fn show(&mut self, ui: &mut Ui) {
        let tones = theme::tones(ui.visuals());
        let mut actions = Vec::new();
        // A notification's Done or a new minute changes what shows.
        let changed = self
            .changes
            .as_ref()
            .map(|c| c.load(Ordering::SeqCst))
            .is_some_and(|c| c != self.seen_changes);
        if changed || self.store.now() - self.read_at >= Duration::seconds(30) {
            if let Some(c) = &self.changes {
                self.seen_changes = c.load(Ordering::SeqCst);
            }
            self.refresh();
        }
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_secs(30));
        // What was asked to be read again and brought no text.
        for (enricher, photo) in [(&self.transcriber, false), (&self.reader, true)] {
            if enricher
                .as_ref()
                .is_some_and(|e| !e.take_failed().is_empty())
            {
                self.again_failed = Some(photo);
            }
        }
        // A phone's requests are answered while the Sync view shows.
        if self.sync.hosting() {
            let asked = self.sync.asking().is_some();
            if self.sync.poll(&self.store, &self.sync_name) {
                self.refresh();
                if let Some(wake) = &self.wake {
                    drop(wake.send(Event::Wake));
                }
            }
            if asked != self.sync.asking().is_some() {
                ui.ctx().request_repaint();
            }
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(100));
        }
        // The AI port's tool calls are answered on this thread, while the
        // window idles too; what they drafted shows at once.
        if let Some(port) = &self.ai_port {
            if port.answer_waiting(&self.store) > 0 {
                self.refresh();
            }
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(100));
        }
        // A typed code being joined, also after leaving the Sync view: its
        // answer is taken in when it comes.
        if self.sync.joining() {
            match self.sync.joined(&self.store, &self.t) {
                Some(_) if self.welcome.is_some() => {
                    // The first start goes on once synced, else back to
                    // its choices, saying why at the top.
                    if self.sync.synced() {
                        actions.push(Action::Welcome(WelcomeAction::Synced));
                    } else {
                        self.notice = self.sync.take_code_error();
                        actions.push(Action::Welcome(WelcomeAction::Back));
                    }
                    ui.ctx().request_repaint();
                }
                Some(changed) => {
                    // Paired Devices and Conflicts too: read again.
                    self.refresh();
                    if changed && let Some(wake) = &self.wake {
                        drop(wake.send(Event::Wake));
                    }
                    ui.ctx().request_repaint();
                }
                None => ui
                    .ctx()
                    .request_repaint_after(std::time::Duration::from_millis(100)),
            }
        }
        // A speech model came: what waited is transcribed now.
        let ready = self.speech.as_ref().is_none_or(|s| s.current().is_some());
        if ready != self.speech_ready {
            self.kick();
        }
        if self.playing.is_some() {
            if self.player.as_ref().is_some_and(|p| p.playing()) {
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(250));
            } else {
                self.playing = None;
            }
        }
        // Pictures and sound files dropped onto the window become a
        // Note; a bundle file is imported, a Sent Item previewed, as are
        // the files handed over from outside.
        let dropped = ui.input(|i| i.raw.dropped_files.clone());
        let (files, dropped): (Vec<_>, Vec<_>) = dropped.into_iter().partition(|file| {
            file.path.as_ref().is_some_and(|p| {
                p.extension()
                    .is_some_and(|e| e == bundle::EXTENSION || e == sent::EXTENSION)
            })
        });
        for file in files {
            if let Some(path) = file.path {
                actions.push(Action::OpenFile(path));
            }
        }
        if let Some(handed) = &self.handed
            && let Ok(mut handed) = handed.lock()
            && !handed.is_empty()
        {
            actions.extend(handed.drain(..).map(Action::OpenFile));
            // Nothing else stirs the window: what it opened shows at once.
            ui.ctx().request_repaint();
        }
        if self
            .changes_wanted
            .as_ref()
            .is_some_and(|wanted| wanted.swap(false, Ordering::SeqCst))
        {
            actions.push(Action::SendWaitingChanges);
            ui.ctx().request_repaint();
        }
        let files: Vec<(String, Vec<u8>)> = dropped
            .iter()
            .filter_map(|file| {
                let name = file
                    .path
                    .as_deref()
                    .map_or_else(|| file.name.clone(), file_name);
                match (&file.bytes, &file.path) {
                    (Some(bytes), _) => Some((name, bytes.to_vec())),
                    (None, Some(path)) => std::fs::read(path).ok().map(|bytes| (name, bytes)),
                    (None, None) => None,
                }
            })
            .collect();
        if !files.is_empty() {
            actions.push(Action::Files(files));
        }
        if self.viewer.is_some() {
            // Back closes the photo shown full screen, and goes nowhere
            // else.
            let back = ui.input_mut(|i| {
                i.pointer.button_pressed(PointerButton::Extra1)
                    | i.consume_key(Modifiers::ALT, Key::ArrowLeft)
            });
            if back {
                actions.push(Action::ClosePhoto);
            }
        } else if self.dialog.is_none() {
            let typing = ui.ctx().text_edit_focused();
            ui.input_mut(|i| {
                let back = i.pointer.button_pressed(PointerButton::Extra1)
                    || (!typing && i.consume_key(Modifiers::ALT, Key::ArrowLeft));
                let forward = i.pointer.button_pressed(PointerButton::Extra2)
                    || (!typing && i.consume_key(Modifiers::ALT, Key::ArrowRight));
                if back {
                    actions.push(Action::Back);
                } else if forward {
                    actions.push(Action::Forward);
                }
            });
        }
        background::paint(
            ui.painter(),
            ui.max_rect(),
            tones.background,
            tones.mark,
            self.scrolled * background::PARALLAX,
        );
        if let Some(error) = self.error.clone() {
            // Announcements go to the top (UI laws).
            egui::Panel::top("announcement")
                .frame(Frame::NONE.inner_margin(Margin::same(12)).fill(tones.card))
                .show(ui, |ui| {
                    ui.label(RichText::new(error).color(ui.visuals().error_fg_color));
                });
        } else if !self.untranscribable.is_empty() {
            // Recordings from files kept but not transcribable; the click
            // shows why (UI laws, "Announcements").
            let text = self
                .t
                .file_recording_failed(self.untranscribable.len() as i64);
            egui::Panel::top("announcement")
                .frame(Frame::NONE.inner_margin(Margin::same(12)).fill(tones.card))
                .show(ui, |ui| {
                    let colour = ui.visuals().text_color();
                    let label = egui::Label::new(RichText::new(text).color(colour))
                        .sense(egui::Sense::click());
                    if ui.add(label).clicked() {
                        actions.push(Action::ShowUntranscribable);
                    }
                });
        } else if let Some(photo) = self.again_failed {
            // Reading or transcribing again brought no text; the click
            // shows why.
            let text = if photo {
                self.t.read_text_again_failed()
            } else {
                self.t.transcribe_again_failed()
            };
            egui::Panel::top("announcement")
                .frame(Frame::NONE.inner_margin(Margin::same(12)).fill(tones.card))
                .show(ui, |ui| {
                    let colour = ui.visuals().text_color();
                    let label = egui::Label::new(RichText::new(text).color(colour))
                        .sense(egui::Sense::click());
                    if ui.add(label).clicked() {
                        actions.push(Action::ShowAgainFailed(photo));
                    }
                });
        } else if let Some(notice) = self.notice.clone() {
            // What a bundle did, until clicked away.
            egui::Panel::top("announcement")
                .frame(Frame::NONE.inner_margin(Margin::same(12)).fill(tones.card))
                .show(ui, |ui| {
                    let colour = ui.visuals().text_color();
                    let label = egui::Label::new(RichText::new(notice).color(colour))
                        .sense(egui::Sense::click());
                    if ui.add(label).on_hover_text(self.t.close()).clicked() {
                        self.notice = None;
                    }
                });
        } else if let Some(update) = self.updates.front().cloned() {
            // A sync that needs an update, until clicked away.
            egui::Panel::top("announcement")
                .frame(Frame::NONE.inner_margin(Margin::same(12)).fill(tones.card))
                .show(ui, |ui| {
                    let colour = ui.visuals().text_color();
                    let label = egui::Label::new(RichText::new(update).color(colour))
                        .sense(egui::Sense::click());
                    if ui.add(label).on_hover_text(self.t.close()).clicked() {
                        self.updates.pop_front();
                    }
                });
        } else if self.arrived > 0 && self.selection != Selection::Sync {
            // One at a time: Conflicts from an automatic sync, a click
            // away from the Sync view that lists them.
            let text = self.t.sync_conflicts_arrived(self.arrived as i64);
            egui::Panel::top("announcement")
                .frame(Frame::NONE.inner_margin(Margin::same(12)).fill(tones.card))
                .show(ui, |ui| {
                    let colour = ui.visuals().text_color();
                    let label = egui::Label::new(RichText::new(text).strong().color(colour))
                        .sense(egui::Sense::click());
                    if ui.add(label).clicked() {
                        actions.push(Action::Select(Selection::Sync));
                    }
                });
        } else if self.view.sync_paused {
            // Sync waits for the AI Task, open or waiting for its review,
            // for as long as it does, below every announcement that goes
            // by a click; the click leads to where the wait ends.
            let (text, asked) = if self.ai_port.is_some() {
                (
                    self.t.ai_port_open_banner(),
                    Action::Select(Selection::Settings),
                )
            } else {
                (self.t.ai_review_banner(), Action::Ai(AiAction::Review))
            };
            egui::Panel::top("announcement")
                .frame(Frame::NONE.inner_margin(Margin::same(12)).fill(tones.card))
                .show(ui, |ui| {
                    let colour = ui.visuals().text_color();
                    let label = egui::Label::new(RichText::new(text).strong().color(colour))
                        .sense(egui::Sense::click());
                    if ui.add(label).clicked() {
                        actions.push(asked);
                    }
                });
        }
        // The first start's view, instead of the panes, until answered.
        if self.welcome.is_some() {
            self.welcome_view(ui, tones, actions);
            return;
        }
        egui::Panel::left("places")
            .frame(Frame::NONE.inner_margin(Margin::same(12)))
            .resizable(true)
            .default_size(260.0)
            .min_size(180.0)
            .show(ui, |ui| self.places_pane(ui, tones, &mut actions));
        egui::Panel::right("note")
            .frame(Frame::NONE.inner_margin(Margin::same(12)))
            .resizable(true)
            .default_size(380.0)
            .min_size(240.0)
            .show(ui, |ui| self.note_pane(ui, tones, &mut actions));
        egui::CentralPanel::default()
            .frame(Frame::NONE.inner_margin(Margin::same(12)))
            .show(ui, |ui| self.notes_pane(ui, tones, &mut actions));
        self.dialog(ui.ctx(), tones, &mut actions);
        // The photo full screen, over everything else.
        if let Some(viewer) = &mut self.viewer
            && media_view::photo_viewer(ui.ctx(), &self.t, (&mut self.photos, &self.store), viewer)
        {
            actions.push(Action::ClosePhoto);
        }
        self.dragged(ui.ctx(), tones);
        for action in actions {
            self.apply(action);
        }
        if let Some(text) = self.copy.take() {
            ui.ctx().copy_text(text);
        }
    }

    fn places_pane(&mut self, ui: &mut Ui, tones: Tones, actions: &mut Vec<Action>) {
        let t = self.t;
        egui::ScrollArea::vertical()
            .id_salt("places")
            .show(ui, |ui| {
                let text = ui.visuals().text_color();
                let rows = [
                    (Selection::Agenda, icons::AGENDA, t.agenda_title()),
                    (Selection::All, icons::ALL_NOTES, t.all_notes_title()),
                    (Selection::Unsorted, icons::UNSORTED, t.unsorted()),
                ];
                for (selection, icon, label) in rows {
                    let on = self.selection == selection;
                    if row(ui, tones, on, 0.0, Some((icon, text)), label, false).clicked() {
                        actions.push(Action::Select(selection));
                    }
                }
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new(t.areas_title()).strong().color(text));
                    // "+" at the right end of the heading.
                    ui.add_space((ui.available_width() - 28.0).max(0.0));
                    if icon_button(ui, icons::ADD, t.new_area(), text).clicked() {
                        actions.push(Action::NewArea);
                    }
                });
                ui.add_space(4.0);
                for (area, topics) in &self.view.areas {
                    let colour = area_colour(area.colour.as_deref()).unwrap_or(text);
                    let icon = icons::area_icon(area.icon.as_deref()).map(|i| (i, colour));
                    let on = self.selection == Selection::Place(area.id.clone());
                    let resp = row(ui, tones, on, 0.0, icon, &area_name(&t, area), true);
                    place_row(&resp, &area.id, actions);
                    let access = self.view.access.get(&area.id).copied();
                    let manages =
                        matches!(access, None | Some(AreaAccess::Owner | AreaAccess::Manager));
                    let people = self.view.people.get(&area.id);
                    if let Some(people) = people {
                        let names: Vec<String> = people
                            .iter()
                            .map(|p| person_name(&t, &self.view.names, p))
                            .collect();
                        ui.horizontal(|ui| {
                            ui.add_space(32.0);
                            ui.label(
                                RichText::new(format!(
                                    "{} · {}",
                                    t.area_shared(),
                                    names.join(", ")
                                ))
                                .small()
                                .color(ui.visuals().weak_text_color()),
                            );
                        });
                    }
                    resp.context_menu(|ui| {
                        let mut items = Vec::new();
                        if manages {
                            items.extend([
                                (t.new_topic(), Action::NewTopic(area.id.clone())),
                                (t.rename(), Action::Rename(area.id.clone())),
                                (t.area_look(), Action::Look(area.id.clone())),
                                (t.share_area(), Action::Share(area.id.clone())),
                            ]);
                        }
                        if people.is_some() {
                            items.push((t.people_title(), Action::People(area.id.clone())));
                        }
                        if manages {
                            items.push((t.delete(), Action::Delete(area.id.clone())));
                        }
                        for (label, action) in items {
                            if ui.button(RichText::new(label).color(text)).clicked() {
                                actions.push(action);
                            }
                        }
                    });
                    for topic in topics {
                        let on = self.selection == Selection::Place(topic.id.clone());
                        let resp = row(ui, tones, on, 24.0, None, &topic_name(&t, topic), true);
                        place_row(&resp, &topic.id, actions);
                        if !manages {
                            continue;
                        }
                        resp.context_menu(|ui| {
                            let items = [
                                (t.rename(), Action::Rename(topic.id.clone())),
                                (t.delete(), Action::Delete(topic.id.clone())),
                            ];
                            for (label, action) in items {
                                if ui.button(RichText::new(label).color(text)).clicked() {
                                    actions.push(action);
                                }
                            }
                        });
                    }
                }
                ui.add_space(12.0);
                let on = self.selection == Selection::Trash;
                let trash = Some((icons::TRASH, text));
                if row(ui, tones, on, 0.0, trash, t.trash_title(), false).clicked() {
                    actions.push(Action::Select(Selection::Trash));
                }
            });
    }

    /// The title of the middle pane.
    fn title(&self) -> String {
        match &self.selection {
            Selection::Agenda => self.t.agenda_title().to_string(),
            Selection::Profile => self.t.profile_title().to_string(),
            Selection::Settings => self.t.settings_title().to_string(),
            Selection::Sync => self.t.sync_title().to_string(),
            Selection::About => self.t.about_title().to_string(),
            Selection::Licences => self.t.open_source_licenses().to_string(),
            Selection::SentItemsWaiting => self.t.sent_items_waiting_title().to_string(),
            Selection::AreaOffersWaiting => self.t.area_offers_waiting_title().to_string(),
            Selection::AiReview => self.t.ai_review_title().to_string(),
            Selection::Frogs => self.t.frogs_title().to_string(),
            Selection::All => self.t.all_notes_title().to_string(),
            Selection::Unsorted => self.t.unsorted().to_string(),
            Selection::Trash => self.t.trash_title().to_string(),
            Selection::Place(id) => self
                .view
                .areas
                .iter()
                .find_map(|(area, topics)| {
                    if area.id == *id {
                        return Some(area_name(&self.t, area));
                    }
                    topics.iter().find(|t| t.id == *id).map(|t| {
                        format!("{} / {}", area_name(&self.t, area), topic_name(&self.t, t))
                    })
                })
                .unwrap_or_default(),
        }
    }

    /// The capture field (UI laws, "Capture"): it never asks anything.
    /// Once something is typed, the Note card's buttons show (#113),
    /// whose choices show at the top, not saved yet, and under them the
    /// Nudge times, Ctrl+Enter taking the 1 h one; a Note filed or with
    /// something set up gets one Save instead and no Nudge.
    fn capture_card(&mut self, ui: &mut Ui, tones: Tones, actions: &mut Vec<Action>) {
        let t = self.t;
        let place = self.capture_place();
        let sorted = self.capture_choices.sorted(place.as_deref());
        // The Area shown is where it goes, not a choice to drop.
        let place_chip = self.capture_choices.place.as_ref().and_then(|id| {
            capture_choices::place_name(&t, &self.view.areas, id).map(|name| (name, true))
        });
        let spawns = self.view.capture_spawns;
        let text = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        let id = Id::new("capture");
        let recording = self
            .recorder
            .as_ref()
            .filter(|r| r.recording())
            .map(|r| r.elapsed());
        let has_recorder = self.recorder.is_some();
        let has_clipboard = self.clipboard.is_some();
        embossed(ui, tones, false, None, |ui| {
            let focused = ui.memory(|m| m.has_focus(id));
            if focused
                && (!self.capture.trim().is_empty() || !self.recorded.is_empty())
                && ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Enter))
            {
                actions.push(Action::Capture(Notes::default_nudge()));
            }
            // Announcements and choices go to the top (UI laws).
            if let Some(asked) = capture_choices::chips(ui, &t, &self.capture_choices, place_chip) {
                actions.push(Action::CaptureChoice(asked));
            }
            ui.add(
                TextEdit::multiline(&mut self.capture)
                    .id(id)
                    .hint_text(t.capture_hint())
                    .text_color(text)
                    .font(FontId::proportional(16.0))
                    .desired_rows(2)
                    .desired_width(f32::INFINITY)
                    .frame(Frame::NONE),
            );
            // A stopped recording stays here until the Note is saved.
            for _ in &self.recorded {
                ui.horizontal(|ui| {
                    icons::show(ui, icons::MIC, 18.0, text);
                    ui.label(RichText::new(t.voice_recording()).color(text));
                });
            }
            if has_recorder {
                match media_view::voice_button(ui, &t, recording) {
                    Some(Voice::Record) => actions.push(Action::Record),
                    Some(Voice::Stop) => actions.push(Action::StopRecording),
                    None => {}
                }
            }
            if has_clipboard && recording.is_none() {
                ui.horizontal(|ui| {
                    icons::show(ui, icons::PASTE, 20.0, text);
                    if ui
                        .button(RichText::new(t.paste_photo()).color(text))
                        .clicked()
                    {
                        actions.push(Action::PastePhoto);
                    }
                });
                ui.label(RichText::new(t.photos_hint()).small().color(weak));
            }
            if recording.is_none() {
                ui.horizontal(|ui| {
                    icons::show(ui, icons::FOLDER_OPEN, 20.0, text);
                    if ui
                        .button(RichText::new(t.from_file()).color(text))
                        .clicked()
                    {
                        actions.push(Action::PickFiles);
                    }
                });
                ui.label(RichText::new(t.from_file_hint()).small().color(weak));
            }
            if self.capture.trim().is_empty() && recording.is_none() && self.recorded.is_empty() {
                return;
            }
            ui.add_space(4.0);
            let offer = (self.view.areas.as_slice(), spawns);
            if let Some(asked) = capture_choices::action_row(ui, &t, &self.capture_choices, offer) {
                actions.push(Action::CaptureChoice(asked));
            }
            ui.add_space(4.0);
            if sorted {
                // A sorted Note needs no Nudge (#113).
                if ui.button(RichText::new(t.save()).color(text)).clicked() {
                    actions.push(Action::Capture(Notes::default_nudge()));
                }
                ui.label(
                    RichText::new(t.capture_keys_hint_sorted())
                        .small()
                        .color(weak),
                );
                return;
            }
            ui.label(RichText::new(t.nudge_heading()).strong().color(text));
            ui.horizontal(|ui| {
                let choices = [
                    (t.snooze10(), Duration::minutes(10)),
                    (t.snooze30(), Duration::minutes(30)),
                    (t.snooze60(), Notes::default_nudge()),
                ];
                for (label, nudge_in) in choices {
                    if ui.button(RichText::new(label).color(text)).clicked() {
                        actions.push(Action::Capture(nudge_in));
                    }
                }
            });
            ui.label(RichText::new(t.capture_keys_hint()).small().color(weak));
        });
    }

    fn notes_pane(&mut self, ui: &mut Ui, tones: Tones, actions: &mut Vec<Action>) {
        match self.selection {
            Selection::Agenda => return self.agenda_pane(ui, tones, actions),
            Selection::Profile => return self.profile_pane(ui, tones, actions),
            Selection::Settings => return self.settings_pane(ui, tones, actions),
            Selection::About => {
                if about_view::about(ui, &self.t, tones) {
                    actions.push(Action::Select(Selection::Licences));
                }
                return;
            }
            Selection::Licences => return about_view::licences_page(ui, &self.t),
            Selection::SentItemsWaiting => return self.held_pane(ui, tones, actions),
            Selection::AreaOffersWaiting => return self.offers_pane(ui, tones, actions),
            Selection::AiReview => return self.ai_review_pane(ui, tones, actions),
            Selection::Frogs => {
                let mut asked = Vec::new();
                let view = &self.view;
                let lists = (
                    view.frogs_open.as_slice(),
                    view.frogs_done.as_slice(),
                    view.frogs_done_open,
                );
                frogs_view::frogs_page(ui, &self.t, tones, lists, &mut asked);
                actions.extend(asked.into_iter().map(Action::Frog));
                return;
            }
            Selection::Sync => {
                let wall = self.wall.clone();
                if let Some(asked) = self.sync.show(ui, &self.t, tones, &self.store, &wall) {
                    actions.push(Action::Sync(asked));
                }
                return;
            }
            _ => {}
        }
        let text = ui.visuals().text_color();
        ui.horizontal(|ui| {
            ui.heading(RichText::new(self.title()).color(text));
            // An Area's level adds Topics (UI laws, "Navigation").
            if let Selection::Place(id) = &self.selection
                && self.view.area(id).is_some()
                && matches!(
                    self.view.access.get(id),
                    None | Some(AreaAccess::Owner | AreaAccess::Manager)
                )
                && icon_button(ui, icons::ADD, self.t.new_topic(), text).clicked()
            {
                actions.push(Action::NewTopic(id.clone()));
            }
            // Every Note list but the trash has its full/compact switch.
            if self.cards_key().is_some() && compact_button(ui, tones, &self.t, self.view.compact) {
                actions.push(Action::Compact(!self.view.compact));
            }
            if self.selection != Selection::Trash {
                self.person_filter_menu(ui, actions);
            }
        });
        if self.selection != Selection::Trash {
            self.person_filter_chips(ui, tones, actions);
        }
        ui.add_space(8.0);
        let hint = match &self.selection {
            Selection::All => Some(self.t.search_all_hint()),
            Selection::Place(id) if self.view.area(id).is_some() => Some(self.t.search_area_hint()),
            Selection::Place(_) => Some(self.t.search_topic_hint()),
            _ => None,
        };
        if let Some(hint) = hint {
            ui.horizontal(|ui| {
                icons::show(ui, icons::SEARCH, 20.0, ui.visuals().weak_text_color());
                let field = ui.add(
                    TextEdit::singleline(&mut self.query)
                        .id(Id::new("search"))
                        .hint_text(hint)
                        .text_color(text)
                        .desired_width(ui.available_width() - 32.0),
                );
                if field.changed() {
                    actions.push(Action::Search);
                }
                if !self.query.is_empty()
                    && icon_button(ui, icons::CLOSE, self.t.clear_search(), text).clicked()
                {
                    self.query.clear();
                    actions.push(Action::Search);
                }
            });
            ui.add_space(8.0);
        }
        let trash = self.selection == Selection::Trash;
        if !trash {
            ui.spacing_mut().item_spacing.y = 16.0;
            self.capture_card(ui, tones, actions);
        }
        if trash && !self.view.trashed_routines.is_empty() {
            self.trashed_routines(ui, tones, actions);
        }
        if trash && !self.view.trashed_frogs.is_empty() {
            self.trashed_frogs(ui, tones, actions);
        }
        if self.view.notes.is_empty() {
            let empty = if trash {
                self.t.trash_empty()
            } else {
                self.t.no_notes_yet()
            };
            let others =
                !self.view.trashed_routines.is_empty() || !self.view.trashed_frogs.is_empty();
            if !(trash && others) {
                ui.label(RichText::new(empty).color(ui.visuals().weak_text_color()));
            }
            return;
        }
        let output = egui::ScrollArea::vertical()
            .id_salt("notes")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                // Room for the emboss around every card.
                ui.spacing_mut().item_spacing.y = 16.0;
                ui.add_space(4.0);
                let transcribing = self.transcriber.as_ref().and_then(|e| e.current());
                for placed in &self.view.notes {
                    let id = &placed.note.id;
                    let selected = self.selected_note.as_deref() == Some(id.as_str());
                    // A Note in the trash is restored, not filed.
                    let draggable = !placed.note.is_trashed();
                    let media = CardMedia {
                        photos: &mut self.photos,
                        pictures: &mut self.pictures,
                        store: &self.store,
                        transcribing: transcribing.as_deref(),
                    };
                    let look = CardLook {
                        full: !self.view.compact || self.unfolded.contains(id),
                        sections: self.view.sections.get(id).map_or(&[], Vec::as_slice),
                    };
                    let clicks = note_card(
                        ui,
                        &self.t,
                        tones,
                        placed,
                        (selected, draggable),
                        look,
                        media,
                    );
                    // A click unfolds a compact Note and does nothing to a
                    // full one; a double-click or Edit opens the editor
                    // (UI laws, "Desktop").
                    if let Some(hash) = clicks.photo {
                        actions.push(Action::ViewPhoto(hash));
                    } else if let (Some(author), Some(profile)) = (clicks.author, &placed.author)
                        && author.clicked()
                    {
                        actions.push(Action::OpenPerson(profile.id.clone()));
                    } else if clicks.edit.clicked()
                        // A click soon after two more counts as well.
                        || clicks.card.double_clicked()
                        || clicks.card.triple_clicked()
                    {
                        actions.push(Action::SelectNote(id.clone()));
                    } else if clicks.card.clicked() && !look.full {
                        actions.push(Action::Unfold(id.clone()));
                    }
                }
                ui.add_space(8.0);
            });
        if output.state.offset.y != self.scrolled {
            // The marks follow on the next pass.
            self.scrolled = output.state.offset.y;
            ui.ctx().request_repaint();
        }
    }

    /// The person filter's menu (#53): "Me", every person of People and,
    /// while it filters, "Everyone"; it stays open while people are
    /// ticked.
    fn person_filter_menu(&self, ui: &mut Ui, actions: &mut Vec<Action>) {
        let t = &self.t;
        let text = ui.visuals().text_color();
        let filter = &self.people;
        let label = if filter.is_everyone() {
            RichText::new(t.person_filter()).color(text)
        } else {
            RichText::new(t.person_filter()).strong().color(text)
        };
        MenuButton::new(label)
            .config(MenuConfig::new().close_behavior(PopupCloseBehavior::CloseOnClickOutside))
            .ui(ui, |ui| {
                let mut me = filter.me;
                if ui
                    .checkbox(&mut me, RichText::new(t.person_filter_me()).color(text))
                    .changed()
                {
                    let mut next = filter.clone();
                    next.me = me;
                    actions.push(Action::FilterPeople(next));
                }
                for person in &self.view.people_shown {
                    let mut on = filter.people.contains(&person.id);
                    let name = profile_view::display_name(t, &person.name);
                    if ui
                        .checkbox(&mut on, RichText::new(name).color(text))
                        .changed()
                    {
                        let mut next = filter.clone();
                        next.toggle(&person.id);
                        actions.push(Action::FilterPeople(next));
                    }
                }
                if !filter.is_everyone()
                    && ui
                        .button(RichText::new(t.person_filter_everyone()).color(text))
                        .clicked()
                {
                    actions.push(Action::FilterPeople(PersonFilter::default()));
                    ui.close();
                }
            });
    }

    /// Who the person filter keeps, each with ✕ to take them out; nothing
    /// while it keeps everyone, so Notes never leave the list unseen.
    fn person_filter_chips(&self, ui: &mut Ui, tones: Tones, actions: &mut Vec<Action>) {
        let filter = &self.people;
        if filter.is_everyone() {
            return;
        }
        let t = &self.t;
        let text = ui.visuals().text_color();
        let mut kept: Vec<(Option<String>, String)> = Vec::new();
        if filter.me {
            kept.push((None, t.person_filter_me().to_string()));
        }
        for id in &filter.people {
            let name = self
                .view
                .people_shown
                .iter()
                .find(|p| p.id == *id)
                .map(|p| p.name.clone())
                .or_else(|| self.view.names.get(id).cloned())
                .unwrap_or_default();
            kept.push((
                Some(id.clone()),
                profile_view::display_name(t, &name).to_string(),
            ));
        }
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(t.person_filter_showing()).color(text));
            for (id, name) in kept {
                if pill(ui, tones, true, &name)
                    .on_hover_text(t.person_filter_remove(&name))
                    .clicked()
                {
                    let mut next = filter.clone();
                    match id {
                        Some(id) => next.toggle(&id),
                        None => next.me = false,
                    }
                    actions.push(Action::FilterPeople(next));
                }
            }
        });
    }

    /// The selected Note's editor (UI laws, "Desktop").
    fn note_pane(&mut self, ui: &mut Ui, tones: Tones, actions: &mut Vec<Action>) {
        if let Some(editor) = &mut self.editor {
            if let Some(intent) = editor.show(ui, &self.t, tones) {
                actions.push(Action::Editor(intent));
            }
            return;
        }
        if let Some(form) = &mut self.tally_form {
            if let Some(intent) = form.show(ui, &self.t, tones) {
                actions.push(Action::TallyForm(intent));
            }
            return;
        }
        let Some((placed, sections)) = &self.view.detail else {
            return;
        };
        let linked = &self.view.routines;
        let tally = self.view.tally.as_ref();
        let tally_count = self.tally_look.count;
        let note_frogs = &self.view.note_frogs;
        let may = self.view.may;
        let t = self.t;
        let note = &placed.note;
        let trashed = note.is_trashed();
        let text = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        let areas = &self.view.areas;
        let edited = &mut self.text;
        let amend = &mut self.amend;
        let cache = &mut self.photos;
        let store = &self.store;
        let playing = self.playing.as_deref();
        let can_play = self.player.is_some();
        let speed = self.speed;
        let transcribing = self.transcriber.as_ref().and_then(|e| e.current());
        let transcribing = transcribing.as_deref();
        // Whisper needs its model; ocrs is built in.
        let heard = self.speech.as_ref().is_none_or(|s| s.current().is_some());
        let offering = Offering {
            view: &self.view,
            transcriber: self.transcriber.as_ref().filter(|_| heard),
            reader: self.reader.as_ref(),
        };
        let offering = &offering;
        egui::ScrollArea::vertical()
            .id_salt("note")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 16.0;
                ui.add_space(4.0);
                embossed(ui, tones, false, None, |ui| {
                    // A Member reads someone else's Note (ADR 0013); a text
                    // in a format not known here shows as it is stored
                    // (ADR 0018).
                    if !note.text_editable {
                        ui.label(RichText::new(t.text_newer_format()).color(weak));
                    }
                    if trashed || !may.changes || !note.text_editable {
                        ui.label(RichText::new(&note.text).size(18.0).color(text));
                        return;
                    }
                    let id = Id::new("note text");
                    if ui.memory(|m| m.has_focus(id))
                        && ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Enter))
                    {
                        actions.push(Action::SaveText);
                    }
                    let field = ui.add(
                        TextEdit::multiline(edited)
                            .id(id)
                            .text_color(text)
                            .font(FontId::proportional(18.0))
                            .desired_rows(1)
                            .desired_width(f32::INFINITY)
                            .frame(Frame::NONE),
                    );
                    if field.lost_focus() {
                        actions.push(Action::SaveText);
                    }
                    let changed = edited.trim();
                    if !changed.is_empty()
                        && changed != note.text
                        && ui.button(RichText::new(t.save()).color(text)).clicked()
                    {
                        actions.push(Action::SaveText);
                    }
                });
                let media = [
                    &note.photos,
                    &note.recordings,
                    &note.removed_photos,
                    &note.removed_recordings,
                ];
                if media.iter().any(|m| !m.is_empty()) {
                    embossed(ui, tones, false, None, |ui| {
                        let media = Media {
                            id: &note.id,
                            photos: (&note.photos, &note.removed_photos, &note.photo_texts),
                            recordings: (
                                &note.recordings,
                                &note.removed_recordings,
                                &note.transcripts,
                            ),
                        };
                        let shown = (&mut *cache, &**store, offering);
                        editor_media(
                            ui,
                            &t,
                            media,
                            shown,
                            (playing, transcribing, can_play, speed),
                            actions,
                        );
                    });
                }
                // Nothing while the Note is no Tally.
                if let Some(tally) = tally {
                    let mut asked = Vec::new();
                    tally_view::section(ui, &t, tones, (tally, tally_count), &mut asked);
                    actions.extend(asked.into_iter().map(Action::Tally));
                }
                embossed(ui, tones, false, None, |ui| {
                    ui.label(RichText::new(t.importance()).strong().color(text));
                    let choices = [
                        (Importance::Unimportant, t.importance_unimportant()),
                        (Importance::Normal, t.importance_normal()),
                        (Importance::Important, t.importance_important()),
                    ];
                    for (importance, label) in choices {
                        let on = note.importance == importance;
                        if trashed || !may.changes {
                            if on {
                                ui.label(RichText::new(label).color(text));
                            }
                        } else if row(ui, tones, on, 0.0, None, label, false).clicked() && !on {
                            actions.push(Action::Importance(importance));
                        }
                    }
                });
                embossed(ui, tones, false, None, |ui| {
                    ui.label(RichText::new(t.captured_heading()).strong().color(text));
                    capture_context(ui, &t, note.captured, &note.context);
                    ui.horizontal(|ui| {
                        let colour = placed
                            .area
                            .as_ref()
                            .and_then(|a| area_colour(a.colour.as_deref()))
                            .unwrap_or(weak);
                        let icon = placed
                            .area
                            .as_ref()
                            .and_then(|a| icons::area_icon(a.icon.as_deref()))
                            .unwrap_or(icons::NO_AREA_ICON);
                        icons::show(ui, icon, 18.0, colour);
                        let place =
                            place_line(&t, placed).unwrap_or_else(|| t.unsorted().to_string());
                        ui.label(RichText::new(place).color(text));
                    });
                    if trashed {
                        ui.horizontal(|ui| {
                            icons::show(ui, icons::TRASH, 18.0, weak);
                            ui.label(RichText::new(t.trash_title()).color(text));
                        });
                        return;
                    }
                    if !may.changes {
                        return;
                    }
                    if let Some(place) = capture_choices::file_into_menu(ui, &t, areas) {
                        actions.push(Action::File {
                            note: note.id.clone(),
                            place,
                        });
                    }
                    ui.label(RichText::new(t.drag_to_file()).small().color(weak));
                });
                for section in sections {
                    embossed(ui, tones, false, None, |ui| {
                        if !section.text_editable {
                            ui.label(RichText::new(t.text_newer_format()).color(weak));
                        }
                        if !section.text.is_empty() {
                            ui.label(RichText::new(&section.text).color(text));
                        }
                        let media = Media {
                            id: &section.id,
                            photos: (
                                &section.photos,
                                &section.removed_photos,
                                &section.photo_texts,
                            ),
                            recordings: (
                                &section.recordings,
                                &section.removed_recordings,
                                &section.transcripts,
                            ),
                        };
                        let shown = (&mut *cache, &**store, offering);
                        editor_media(
                            ui,
                            &t,
                            media,
                            shown,
                            (playing, transcribing, can_play, speed),
                            actions,
                        );
                        capture_context(ui, &t, section.captured, &section.context);
                    });
                }
                if trashed {
                    if may.changes && ui.button(RichText::new(t.restore()).color(text)).clicked() {
                        actions.push(Action::Restore);
                    }
                    return;
                }
                embossed(ui, tones, false, None, |ui| {
                    ui.label(RichText::new(t.routines_heading()).strong().color(text));
                    for routine in linked {
                        let icon = if routine.is_reminder() {
                            icons::REMINDER
                        } else {
                            icons::REPEAT
                        };
                        let label = schedule_text(&t, &routine.schedule);
                        if row(ui, tones, false, 0.0, Some((icon, text)), &label, false).clicked() {
                            actions.push(Action::OpenRoutine(routine.id.clone()));
                        }
                    }
                    // Its Frogs, checked off here too; the words open the
                    // Frog view.
                    for (frog, may_check) in note_frogs {
                        ui.horizontal(|ui| {
                            let done = frog.is_done();
                            let label = format!("{}: {}", t.mark_done(), t.frog());
                            let toggled = ui
                                .add_enabled_ui(*may_check, |ui| check(ui, tones, done, &label))
                                .inner;
                            if toggled.clicked() {
                                actions.push(Action::Frog(FrogAction::Toggle(frog.id.clone())));
                            }
                            frogs_view::frog_mark(ui, 16.0, text);
                            let mut words = RichText::new(t.frog()).color(text);
                            if done {
                                words = words.strikethrough();
                            }
                            let open = ui
                                .add(egui::Label::new(words).sense(Sense::click()))
                                .on_hover_cursor(egui::CursorIcon::PointingHand);
                            if open.clicked() {
                                actions.push(Action::Frog(FrogAction::OpenView));
                            }
                        });
                    }
                    if !may.schedules {
                        ui.label(RichText::new(t.only_managers()).small().color(weak));
                        return;
                    }
                    // "Routine, Reminder, Frog or Session"; a Note has one
                    // open Frog at a time.
                    let frog_open = note_frogs.iter().any(|(f, _)| !f.is_done());
                    // Wrapped, so the four never widen the pane.
                    ui.horizontal_wrapped(|ui| {
                        for (reminder, label) in [(false, t.routine()), (true, t.reminder())] {
                            if ui.button(RichText::new(label).color(text)).clicked() {
                                actions.push(Action::NewRoutine {
                                    note: Some(note.id.clone()),
                                    reminder,
                                });
                            }
                        }
                        if !frog_open && ui.button(RichText::new(t.frog()).color(text)).clicked() {
                            actions.push(Action::SpawnFrog(note.id.clone()));
                        }
                        if ui.button(RichText::new(t.session()).color(text)).clicked() {
                            actions.push(Action::NewSession(note.id.clone()));
                        }
                    });
                });
                if !may.amends {
                    return;
                }
                embossed(ui, tones, false, None, |ui| {
                    ui.add(
                        TextEdit::multiline(amend)
                            .id(Id::new("amend"))
                            .hint_text(t.amend_hint())
                            .text_color(text)
                            .desired_rows(1)
                            .desired_width(f32::INFINITY)
                            .frame(Frame::NONE),
                    );
                    let ready = !amend.trim().is_empty();
                    let add = ui.add_enabled(
                        ready,
                        egui::Button::new(RichText::new(t.amend()).color(text)),
                    );
                    if add.clicked() {
                        actions.push(Action::Amend);
                    }
                });
                if ui
                    .button(RichText::new(t.send_button()).color(text))
                    .on_hover_text(t.send_note())
                    .clicked()
                {
                    actions.push(Action::StartSend);
                }
                if !may.changes {
                    return;
                }
                ui.horizontal(|ui| {
                    if ui
                        .button(RichText::new(t.merge_into()).color(text))
                        .clicked()
                    {
                        actions.push(Action::StartMerge);
                    }
                    if ui
                        .button(RichText::new(t.move_to_trash()).color(text))
                        .clicked()
                    {
                        actions.push(Action::Trash);
                    }
                });
                // Only a Note in the lists becomes a Tally (ADR 0028); one
                // that is adds entries in its Tally part above.
                if tally.is_none()
                    && !note.is_merged()
                    && ui
                        .button(RichText::new(t.make_tally()).color(text))
                        .clicked()
                {
                    actions.push(Action::Tally(TallyAction::Make(note.id.clone())));
                }
            });
    }

    /// The open dialog, over everything else.
    fn dialog(&mut self, ctx: &egui::Context, tones: Tones, actions: &mut Vec<Action>) {
        let focus = std::mem::take(&mut self.focus);
        let Some(dialog) = &mut self.dialog else {
            return;
        };
        let t = self.t;
        let view = &self.view;
        let (own_view, question_view) = (&mut self.profile_view, &mut self.question_view);
        let pictures = &mut self.pictures;
        let person_square = &mut self.person_square;
        let frame = Frame::NONE
            .fill(tones.card)
            .corner_radius(theme::CARD_RADIUS)
            .inner_margin(Margin::same(20));
        let modal = egui::Modal::new(Id::new("dialog"))
            .frame(frame)
            .show(ctx, |ui| {
                ui.set_width(400.0);
                ui.spacing_mut().item_spacing.y = 10.0;
                match dialog {
                    Dialog::NewPlace { area, name } => {
                        let title = if area.is_some() {
                            t.new_topic()
                        } else {
                            t.new_area()
                        };
                        let form = (title, t.create(), Action::AddPlace);
                        name_form(ui, &t, form, (name, focus), actions);
                    }
                    Dialog::Rename { name, .. } => {
                        let form = (t.rename(), t.save(), Action::SaveName);
                        name_form(ui, &t, form, (name, focus), actions);
                    }
                    Dialog::Look { area } => look_form(ui, &t, view.area(area), actions),
                    Dialog::Delete(deleting) => delete_form(ui, &t, deleting, actions),
                    Dialog::Merge { query, others } => {
                        merge_form(ui, &t, tones, (query, focus), others, actions);
                    }
                    Dialog::TrashNote { linked } => {
                        let linked = *linked;
                        title(ui, t.delete_note_question());
                        let text = ui.visuals().text_color();
                        let detail = if linked == 0 {
                            t.delete_note_detail()
                        } else {
                            t.delete_note_linked_detail()
                        };
                        ui.label(RichText::new(detail).color(text));
                        ui.horizontal_wrapped(|ui| {
                            cancel(ui, &t, actions);
                            if linked == 0 {
                                if destructive(ui, t.delete()) {
                                    actions.push(Action::TrashNote(false));
                                }
                            } else {
                                if destructive(ui, t.delete_note_only()) {
                                    actions.push(Action::TrashNote(false));
                                }
                                if destructive(ui, &t.delete_note_with_linked(linked)) {
                                    actions.push(Action::TrashNote(true));
                                }
                            }
                        });
                    }
                    Dialog::DeleteRoutine { question } => {
                        title(ui, question);
                        ui.horizontal(|ui| {
                            cancel(ui, &t, actions);
                            if destructive(ui, t.delete()) {
                                actions.push(Action::DeleteRoutine);
                            }
                        });
                    }
                    Dialog::DeleteFrog { .. } => {
                        title(ui, t.delete_frog_question());
                        ui.horizontal(|ui| {
                            cancel(ui, &t, actions);
                            if destructive(ui, t.delete()) {
                                actions.push(Action::DeleteFrog);
                            }
                        });
                    }
                    Dialog::DeleteEntry { .. } => {
                        title(ui, t.delete_entry_question());
                        ui.horizontal(|ui| {
                            cancel(ui, &t, actions);
                            if destructive(ui, t.delete()) {
                                actions.push(Action::DeleteEntry);
                            }
                        });
                    }
                    Dialog::Share { area, role, afar } => {
                        let shown = view
                            .area(area)
                            .map(|a| area_name(&t, a))
                            .unwrap_or_default();
                        share_form(ui, &t, &shown, (role, !afar.is_empty()), actions);
                    }
                    Dialog::ShareAfar {
                        area,
                        people,
                        person: None,
                        ..
                    } => {
                        let shown = view
                            .area(area)
                            .map(|a| area_name(&t, a))
                            .unwrap_or_default();
                        share_person_form(ui, &t, tones, (&shown, people), pictures, actions);
                    }
                    Dialog::ShareAfar {
                        people,
                        person: Some(person),
                        transport,
                        ..
                    } => {
                        let name = people
                            .iter()
                            .find(|p| p.id == *person)
                            .map(|p| profile_view::display_name(&t, &p.shown.name))
                            .unwrap_or_default();
                        transport_form(ui, &t, name, transport, actions);
                    }
                    Dialog::AreaOffer {
                        held,
                        blocking: Some(areas),
                        ..
                    } => {
                        let name = profile_view::display_name(&t, &held.sender.name);
                        block_form(ui, &t, name, &area_names(&t, view, areas), actions);
                    }
                    Dialog::AreaOffer {
                        held,
                        compared,
                        blocking: None,
                    } => area_offer_form(ui, &t, tones, (held, *compared), pictures, actions),
                    Dialog::OfferFolder { area } => {
                        title(ui, t.area_offer_folder_question());
                        let text = ui.visuals().text_color();
                        ui.label(RichText::new(t.area_offer_folder_detail(area)).color(text));
                        ui.horizontal(|ui| {
                            if ui
                                .button(RichText::new(t.area_offer_folder_later()).color(text))
                                .clicked()
                            {
                                actions.push(Action::CloseDialog);
                            }
                            if ui
                                .button(RichText::new(t.folder_pick()).strong().color(text))
                                .clicked()
                            {
                                actions.push(Action::OfferFolder);
                            }
                        });
                    }
                    Dialog::Send {
                        recipients,
                        picked,
                        update,
                        cancel,
                        ..
                    } => send_form(ui, &t, recipients, (picked, *update, cancel), actions),
                    Dialog::SentItem {
                        opened,
                        blocking: Some(areas),
                        ..
                    } => {
                        let name = profile_view::display_name(&t, &opened.sender.name);
                        block_form(ui, &t, name, &area_names(&t, view, areas), actions);
                    }
                    Dialog::SentItem {
                        opened,
                        compared,
                        may_block,
                        blocking: None,
                        ..
                    } => {
                        let shown = (opened.as_ref(), *compared, *may_block);
                        sent_item_form(ui, &t, tones, (shown, pictures), actions);
                    }
                    Dialog::HeldSentItem {
                        held,
                        blocking: Some(areas),
                    } => {
                        let name = profile_view::display_name(&t, &held.sender.name);
                        block_form(ui, &t, name, &area_names(&t, view, areas), actions);
                    }
                    Dialog::HeldSentItem {
                        held,
                        blocking: None,
                    } => held_sent_item_form(ui, &t, tones, held, actions),
                    Dialog::People { area } => {
                        let manages = view.access.get(area) == Some(&AreaAccess::Manager);
                        let people = view.people.get(area).map(Vec::as_slice).unwrap_or(&[]);
                        let mine = view.mine.get(area).map(String::as_str);
                        let names = &view.names;
                        let shown = (&view.shown, &mut *pictures);
                        people_form(ui, &t, area, (people, names, mine), shown, manages, actions);
                    }
                    Dialog::RemovePerson { question, .. } => {
                        title(ui, question);
                        let text = ui.visuals().text_color();
                        ui.label(RichText::new(t.remove_person_detail()).color(text));
                        ui.horizontal(|ui| {
                            cancel(ui, &t, actions);
                            if destructive(ui, t.remove_person()) {
                                actions.push(Action::RemovePerson);
                            }
                        });
                    }
                    Dialog::RemoveMedia { photo, .. } => {
                        title(
                            ui,
                            if *photo {
                                t.remove_photo_question()
                            } else {
                                t.remove_recording_question()
                            },
                        );
                        let text = ui.visuals().text_color();
                        ui.label(RichText::new(t.remove_media_detail()).color(text));
                        ui.horizontal(|ui| {
                            cancel(ui, &t, actions);
                            if destructive(ui, t.remove()) {
                                actions.push(Action::RemoveMedia);
                            }
                        });
                    }
                    Dialog::RemoveText { photo, .. } => {
                        title(
                            ui,
                            if *photo {
                                t.remove_photo_text_question()
                            } else {
                                t.remove_transcript_question()
                            },
                        );
                        let text = ui.visuals().text_color();
                        ui.label(RichText::new(t.remove_generated_text_detail()).color(text));
                        ui.horizontal(|ui| {
                            cancel(ui, &t, actions);
                            if destructive(ui, t.remove()) {
                                actions.push(Action::RemoveText);
                            }
                        });
                    }
                    Dialog::Person {
                        comparing,
                        name,
                        asking,
                        back,
                        ..
                    } => {
                        let mut asked = Vec::new();
                        if let (Some(asking), Some(person)) = (asking.as_ref(), &view.person) {
                            let name = profile_view::display_name(&t, &person.shown.name);
                            match asking {
                                Asking::Block(areas) => {
                                    let areas = area_names(&t, view, areas);
                                    block_form(ui, &t, name, &areas, actions);
                                }
                                Asking::Unblock => unblock_form(ui, &t, name, actions),
                            }
                        } else if let Some(square) = person_square.as_mut() {
                            let mut picked = Vec::new();
                            profile_view::square_page(ui, &t, square, &mut picked);
                            asked.extend(picked.into_iter().filter_map(|a| match a {
                                ProfileAction::UseSquare => Some(PersonAction::UseSquare),
                                ProfileAction::CancelSquare => Some(PersonAction::CancelSquare),
                                _ => None,
                            }));
                        } else if let Some(person) = &view.person {
                            person_view::form(
                                ui,
                                &t,
                                pictures,
                                person,
                                (*comparing, name, *back),
                                &mut asked,
                            );
                            if ui
                                .button(RichText::new(t.close()).color(ui.visuals().text_color()))
                                .clicked()
                            {
                                actions.push(Action::CloseDialog);
                            }
                        }
                        actions.extend(asked.into_iter().map(Action::Person));
                    }
                    Dialog::Failure {
                        title: heading,
                        cause,
                        fix,
                        details,
                    } => failure_form(ui, &t, heading, (cause, fix), details, actions),
                    Dialog::ProfileQr { link } => {
                        ui.set_width(500.0);
                        profile_view::qr_page(ui, &t, link);
                        if ui
                            .button(RichText::new(t.close()).color(ui.visuals().text_color()))
                            .clicked()
                        {
                            actions.push(Action::CloseDialog);
                        }
                    }
                    Dialog::RejectAi => {
                        title(ui, t.ai_reject_question());
                        let text = ui.visuals().text_color();
                        ui.label(RichText::new(t.ai_reject_detail()).color(text));
                        ui.horizontal_wrapped(|ui| {
                            cancel(ui, &t, actions);
                            if destructive(ui, t.ai_reject_all()) {
                                actions.push(Action::Ai(AiAction::Reject));
                            }
                        });
                    }
                    Dialog::WhichProfile => {
                        if let Some(other) = &view.profile_question {
                            let mut asked = Vec::new();
                            profile_view::question_form(
                                ui,
                                &t,
                                (own_view, question_view),
                                (view.profile.as_ref(), view.profile_code.as_deref()),
                                other,
                                &mut asked,
                            );
                            actions.extend(asked.into_iter().map(Action::Profile));
                        }
                    }
                }
            });
        if modal.should_close() {
            // Put off, not lost: it waits on the Profile page.
            actions.push(match dialog {
                Dialog::WhichProfile => Action::Profile(ProfileAction::Later),
                // A question about blocking goes back to what asked it.
                Dialog::SentItem {
                    blocking: Some(_), ..
                }
                | Dialog::HeldSentItem {
                    blocking: Some(_), ..
                }
                | Dialog::AreaOffer {
                    blocking: Some(_), ..
                }
                | Dialog::Person {
                    asking: Some(_), ..
                } => Action::CancelBlock,
                _ => Action::CloseDialog,
            });
        }
    }

    /// The Agenda (UI laws, "Agenda"): the capture field, the Missed
    /// Summary until "Seen", the "x Unsorted" banner, the "x Sent Items
    /// waiting" line opening their list, the "x shared Areas waiting" line
    /// opening theirs, today's Occurrences, a gap, and tomorrow's dimmed, then the five days after
    /// the same way, each under its weekday and date; each checkable on
    /// its own, a click on one opens its Routine. The Profile picture in the
    /// corner, where the settings gear was, leads to the Profile page.
    fn agenda_pane(&mut self, ui: &mut Ui, tones: Tones, actions: &mut Vec<Action>) {
        let t = self.t;
        let text = ui.visuals().text_color();
        ui.horizontal(|ui| {
            ui.heading(RichText::new(t.agenda_title()).color(text));
            ui.add_space((ui.available_width() - 60.0).max(0.0));
            if icon_button(ui, icons::ADD_ALARM, t.new_routine_tooltip(), text).clicked() {
                actions.push(Action::NewRoutine {
                    note: None,
                    reminder: false,
                });
            }
            let profile = self.view.profile.as_ref();
            if profile_view::avatar(ui, &mut self.profile_view, profile, 28.0, t.profile_title())
                .clicked()
            {
                actions.push(Action::Select(Selection::Profile));
            }
        });
        ui.add_space(8.0);
        ui.spacing_mut().item_spacing.y = 16.0;
        self.capture_card(ui, tones, actions);
        let Some(agenda) = self.view.agenda.clone() else {
            return;
        };
        let output = egui::ScrollArea::vertical()
            .id_salt("agenda")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 12.0;
                ui.add_space(4.0);
                if !agenda.missed.is_empty() {
                    missed_card(ui, &t, tones, &agenda.missed, actions);
                }
                if agenda.unsorted > 0 {
                    let said = t.unsorted_banner(agenda.unsorted as i64);
                    let id = Id::new("unsorted banner");
                    if count_banner(ui, tones, (id, icons::INBOX), &said) {
                        actions.push(Action::Select(Selection::Unsorted));
                    }
                }
                if agenda.waiting > 0 {
                    let said = t.sent_items_waiting(agenda.waiting as i64);
                    let id = Id::new("sent items waiting");
                    if count_banner(ui, tones, (id, icons::MAIL_UNREAD), &said) {
                        actions.push(Action::Select(Selection::SentItemsWaiting));
                    }
                }
                if agenda.offers > 0 {
                    let said = t.area_offers_waiting(agenda.offers as i64);
                    let id = Id::new("shared areas waiting");
                    if count_banner(ui, tones, (id, icons::GROUP_ADD), &said) {
                        actions.push(Action::Select(Selection::AreaOffersWaiting));
                    }
                }
                ui.label(
                    RichText::new(t.agenda_today())
                        .strong()
                        .size(16.0)
                        .color(text),
                );
                if agenda.today.is_empty() {
                    ui.label(
                        RichText::new(t.nothing_today()).color(ui.visuals().weak_text_color()),
                    );
                }
                for row in &agenda.today {
                    agenda_row(ui, &t, tones, row, actions);
                }
                // Under what is due, never above it (#101).
                if let Some(frogs) = agenda.frogs.as_ref().filter(|f| f.visible) {
                    let mut asked = Vec::new();
                    ui.add_space(4.0);
                    frogs_view::heading(ui, &t, &mut asked);
                    let another = frogs.can_swap && !frogs.eaten;
                    for shown in &agenda.frog_rows {
                        frogs_view::frog_row(ui, &t, tones, shown, (another, None), &mut asked);
                    }
                    actions.extend(asked.into_iter().map(Action::Frog));
                }
                // The visible gap between today and tomorrow.
                ui.add_space(24.0);
                ui.scope(|ui| {
                    ui.multiply_opacity(TOMORROW_OPACITY);
                    ui.label(
                        RichText::new(t.agenda_tomorrow())
                            .strong()
                            .size(16.0)
                            .color(text),
                    );
                    if agenda.tomorrow.is_empty() {
                        ui.label(
                            RichText::new(t.nothing_tomorrow())
                                .color(ui.visuals().weak_text_color()),
                        );
                    }
                    for row in &agenda.tomorrow {
                        agenda_row(ui, &t, tones, row, actions);
                    }
                });
                // The days after, each under its weekday and date; one
                // with nothing due still shows.
                for (day, rows) in &agenda.later {
                    ui.add_space(4.0);
                    ui.scope(|ui| {
                        ui.multiply_opacity(TOMORROW_OPACITY);
                        ui.label(
                            RichText::new(agenda_day_text(&t, *day))
                                .strong()
                                .size(16.0)
                                .color(text),
                        );
                        if rows.is_empty() {
                            ui.label(
                                RichText::new(t.nothing_planned())
                                    .color(ui.visuals().weak_text_color()),
                            );
                        }
                        for row in rows {
                            agenda_row(ui, &t, tones, row, actions);
                        }
                    });
                }
                ui.add_space(8.0);
            });
        if output.state.offset.y != self.scrolled {
            self.scrolled = output.state.offset.y;
            ui.ctx().request_repaint();
        }
    }

    /// The Profile page (ADR 0017): the own Profile's picture, name and
    /// short code; the own Profile card as a link or a QR (#107); Backups
    /// and recovery; the user's Devices; People, with "Add a person from
    /// a link"; and the way to this Device's settings. While a new
    /// picture's square is picked, that alone.
    fn profile_pane(&mut self, ui: &mut Ui, tones: Tones, actions: &mut Vec<Action>) {
        let t = self.t;
        let text = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        let mut asked = Vec::new();
        if let Some(square) = self.profile_view.square.as_mut() {
            embossed(ui, tones, false, None, |ui| {
                profile_view::square_page(ui, &t, square, &mut asked);
            });
            actions.extend(asked.into_iter().map(Action::Profile));
            return;
        }
        let (store, wall, backup_form, view, card_form) = (
            &self.store,
            &self.wall,
            &mut self.backup_form,
            &mut self.profile_view,
            &mut self.card_form,
        );
        let (people, pictures) = (&self.view.people_shown, &mut self.pictures);
        let (blocked, blocked_open) = (&self.view.blocked_people, self.view.blocked_open);
        let mut people_asked = Vec::new();
        let profile = self.view.profile.as_ref();
        let code = self.view.profile_code.as_deref();
        let question = self.view.profile_question.is_some();
        ui.heading(RichText::new(t.profile_title()).color(text));
        ui.add_space(8.0);
        egui::ScrollArea::vertical()
            .id_salt("profile")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 16.0;
                ui.add_space(4.0);
                embossed(ui, tones, false, None, |ui| {
                    profile_view::header(ui, &t, view, profile, code, &mut asked);
                });
                if profile.is_some() {
                    embossed(ui, tones, false, None, |ui| {
                        profile_view::card_section(ui, &t, card_form.copied, &mut asked);
                    });
                }
                if question {
                    embossed(ui, tones, false, None, |ui| {
                        ui.label(RichText::new(t.profile_which_waiting()).color(text));
                        if ui
                            .button(RichText::new(t.profile_which_title()).color(text))
                            .clicked()
                        {
                            asked.push(ProfileAction::Ask);
                        }
                    });
                }
                embossed(ui, tones, false, None, |ui| {
                    ui.label(RichText::new(t.profile_backups()).strong().color(text));
                    let mut backup = Vec::new();
                    backup_view::card(ui, &t, store, wall, backup_form, &mut backup);
                    actions.extend(backup.into_iter().map(Action::Backup));
                });
                embossed(ui, tones, false, None, |ui| {
                    ui.label(RichText::new(t.profile_devices()).strong().color(text));
                    if ui
                        .button(RichText::new(t.sync_settings_entry()).color(text))
                        .clicked()
                    {
                        actions.push(Action::Select(Selection::Sync));
                    }
                    ui.label(RichText::new(t.sync_settings_detail()).small().color(weak));
                });
                // People (#52): every Profile kept here, each opening it;
                // the blocked ones (#90) in a fold of their own, closed
                // until opened, which this Device remembers.
                embossed(ui, tones, false, None, |ui| {
                    ui.label(RichText::new(t.people_title()).strong().color(text));
                    if people.is_empty() && blocked.is_empty() {
                        ui.label(RichText::new(t.profile_people_empty()).small().color(weak));
                    }
                    for person in people {
                        people_row(ui, &t, pictures, person, &mut people_asked);
                    }
                    if !blocked.is_empty() {
                        let fold = egui::CollapsingHeader::new(
                            RichText::new(t.people_blocked()).color(text),
                        )
                        .id_salt("people blocked")
                        .open(Some(blocked_open))
                        .show(ui, |ui| {
                            for person in blocked {
                                people_row(ui, &t, pictures, person, &mut people_asked);
                            }
                        });
                        if fold.header_response.clicked() {
                            actions.push(Action::BlockedOpen(!blocked_open));
                        }
                    }
                    ui.separator();
                    profile_view::add_person(ui, &t, card_form, &mut asked);
                });
                embossed(ui, tones, false, None, |ui| {
                    ui.label(
                        RichText::new(t.profile_device_settings())
                            .strong()
                            .color(text),
                    );
                    if ui
                        .button(RichText::new(t.settings_title()).color(text))
                        .clicked()
                    {
                        actions.push(Action::Select(Selection::Settings));
                    }
                    ui.label(
                        RichText::new(t.profile_device_settings_detail())
                            .small()
                            .color(weak),
                    );
                });
            });
        actions.extend(asked.into_iter().map(Action::Profile));
        actions.extend(people_asked.into_iter().map(Action::OpenPerson));
    }

    /// Opens the Profile of the person `id` (#52) over the window; over
    /// a dialog, that one shows again once it closes. Nothing for the
    /// user's own.
    fn open_person(&mut self, id: String, back: SendBack) -> oldbrainfix_core::Result<()> {
        if self.store.is_own_person(&id)? {
            return Ok(());
        }
        let name = self
            .store
            .person_override(&id)?
            .and_then(|o| o.name)
            .unwrap_or_default();
        let behind = self.dialog.take();
        if !matches!(behind, Some(Dialog::Person { .. })) {
            self.dialog_behind = behind;
        }
        self.person_square = None;
        self.dialog = Some(Dialog::Person {
            id,
            comparing: Comparing::Not,
            name,
            asking: None,
            back,
        });
        Ok(())
    }

    /// Closes the dialog. A person's Profile keeps the name typed last
    /// and shows the dialog it opened over again, a Sent Item's or an Area
    /// offer's sender as the user sees them now; a Sent Item or an offer
    /// whose sender the user blocked there shows nothing more and closes
    /// too (#90).
    fn close_dialog(&mut self) -> oldbrainfix_core::Result<()> {
        let behind = self.dialog_behind.take();
        if let Some(Dialog::Person { id, name, .. }) = self.dialog.take() {
            self.store.set_override_name(&id, &name)?;
            self.person_square = None;
            self.dialog = behind;
            if let Some(Dialog::SentItem { opened, .. }) = &mut self.dialog {
                if self.store.is_blocked(&opened.sender.id)? {
                    self.dialog = None;
                    self.notice = Some(self.t.sent_item_from_blocked().to_string());
                } else if opened.trust != SenderTrust::Unknown {
                    opened.sender = self.store.person_shown(&opened.sender.id)?;
                }
            }
            // A block drops the offer with it (ADR 0021).
            if let Some(Dialog::AreaOffer { held, .. }) = &mut self.dialog {
                if self.store.is_blocked(&held.sender.id)? {
                    self.dialog = None;
                    self.notice = Some(self.t.area_offer_from_blocked().to_string());
                } else if held.trust != SenderTrust::Unknown {
                    held.sender = self.store.person_shown(&held.sender.id)?;
                }
            }
        }
        Ok(())
    }

    /// Blocks the person the dialog asks about (#90), leaving the Shared
    /// Areas it named; asks again, naming them, should one have come in
    /// by sync since. From a Sent Item's preview, a stranger's hidden one
    /// too, or an Area offer's (#108), the item is declined and the
    /// preview closes; a person's
    /// Profile stays and shows the block.
    fn block(&mut self) -> oldbrainfix_core::Result<()> {
        let done = match &mut self.dialog {
            Some(Dialog::Person {
                id,
                asking: Some(Asking::Block(areas)),
                ..
            }) => match self.store.block_person(id, !areas.is_empty()) {
                Ok(_) => None,
                Err(oldbrainfix_core::Error::MustLeaveAreas(more)) => {
                    *areas = more;
                    return Ok(());
                }
                Err(e) => return Err(e),
            },
            Some(Dialog::SentItem {
                opened,
                blocking: Some(areas),
                held,
                ..
            }) => {
                let leave = !areas.is_empty();
                let blocked = match held {
                    Some(id) => self.store.block_held_sent_item_sender(id, leave),
                    None => self.store.block_sent_item_sender(opened, leave),
                };
                match blocked {
                    Ok(blocked) => Some(blocked),
                    Err(oldbrainfix_core::Error::MustLeaveAreas(more)) => {
                        *areas = more;
                        return Ok(());
                    }
                    Err(e) => return Err(e),
                }
            }
            Some(Dialog::HeldSentItem {
                held,
                blocking: Some(areas),
            }) => match self
                .store
                .block_held_sent_item_sender(&held.id, !areas.is_empty())
            {
                Ok(blocked) => Some(blocked),
                Err(oldbrainfix_core::Error::MustLeaveAreas(more)) => {
                    *areas = more;
                    return Ok(());
                }
                Err(e) => return Err(e),
            },
            Some(Dialog::AreaOffer {
                held,
                blocking: Some(areas),
                ..
            }) => match self
                .store
                .block_area_offer_sender(&held.id, !areas.is_empty())
            {
                Ok(blocked) => Some(blocked),
                Err(oldbrainfix_core::Error::MustLeaveAreas(more)) => {
                    *areas = more;
                    return Ok(());
                }
                Err(e) => return Err(e),
            },
            _ => return Ok(()),
        };
        match done {
            None => {
                if let Some(Dialog::Person { asking, .. }) = &mut self.dialog {
                    *asking = None;
                }
            }
            // A copied id blocks nobody: the item is only declined.
            Some(blocked) => {
                self.dialog = None;
                self.notice = Some(if blocked {
                    self.t.sent_item_blocked().to_string()
                } else {
                    self.t.sent_item_declined().to_string()
                });
            }
        }
        Ok(())
    }

    /// Does what was asked of the person's Profile the dialog shows.
    fn person_action(&mut self, asked: PersonAction) -> oldbrainfix_core::Result<()> {
        match asked {
            PersonAction::UseSquare => return self.person_square_done(true),
            PersonAction::CancelSquare => return self.person_square_done(false),
            PersonAction::TheirNotes => return self.their_notes(),
            PersonAction::SendChanges => return self.send_changes(),
            _ => {}
        }
        let Some(Dialog::Person {
            id,
            comparing,
            name,
            asking,
            back,
        }) = &mut self.dialog
        else {
            return Ok(());
        };
        match asked {
            PersonAction::Rename(typed) => {
                self.store.set_override_name(id, &typed)?;
                *name = self
                    .store
                    .person_override(id)?
                    .and_then(|o| o.name)
                    .unwrap_or_default();
            }
            PersonAction::TheirPicture => self.store.set_override_picture(id, None)?,
            PersonAction::PickPicture => {
                let Some(path) = (self.picture_picker)(self.t.profile_picture_choose()) else {
                    return Ok(());
                };
                let square = std::fs::read(&path)
                    .ok()
                    .and_then(|bytes| photo_from_file(&bytes))
                    .and_then(Square::new);
                match square {
                    Some(square) => self.person_square = Some(square),
                    None => self.notice = Some(self.t.profile_picture_unreadable().to_string()),
                }
            }
            PersonAction::UseSquare
            | PersonAction::CancelSquare
            | PersonAction::TheirNotes
            | PersonAction::SendChanges => {}
            PersonAction::Block => {
                *asking = Some(Asking::Block(self.store.areas_to_leave_for_blocking(id)?));
            }
            PersonAction::Unblock => *asking = Some(Asking::Unblock),
            PersonAction::CompareCodes => *comparing = Comparing::Asking,
            PersonAction::CodesCompared(false) => *comparing = Comparing::Differ,
            PersonAction::SendBack => match self.store.profile_card(true)? {
                Some(link) => {
                    self.copy = Some(self.t.profile_card_text(&link));
                    *back = SendBack::Copied;
                }
                None => self.notice = Some(self.t.profile_card_not_yet().to_string()),
            },
            PersonAction::CodesCompared(true) => {
                *comparing = Comparing::Not;
                if let Some(code) = self.store.person_code(id)? {
                    self.store.verify_person(id, &code)?;
                }
            }
        }
        Ok(())
    }

    /// "Their Notes" (#53): closes the person's Profile, keeping the name
    /// typed last, and every dialog under it, and shows All Notes with
    /// only theirs.
    fn their_notes(&mut self) -> oldbrainfix_core::Result<()> {
        let Some(Dialog::Person { id, .. }) = &self.dialog else {
            return Ok(());
        };
        let id = id.clone();
        self.close_dialog()?;
        self.dialog = None;
        self.act(Action::Select(Selection::All))?;
        self.people = PersonFilter::person(&id);
        Ok(())
    }

    /// Takes the square picked as the picture for the person the dialog
    /// shows, or leaves it.
    fn person_square_done(&mut self, take: bool) -> oldbrainfix_core::Result<()> {
        let Some(square) = self.person_square.take() else {
            return Ok(());
        };
        let Some(Dialog::Person { id, .. }) = &self.dialog else {
            return Ok(());
        };
        if !take {
            return Ok(());
        }
        match square.make() {
            Some(picture) => self.store.set_override_picture(id, Some(&picture))?,
            None => self.notice = Some(self.t.profile_picture_unreadable().to_string()),
        }
        Ok(())
    }

    /// Does what was asked of the own Profile on the Profile page.
    fn profile_action(&mut self, asked: ProfileAction) -> oldbrainfix_core::Result<()> {
        let own = OwnProfile::new(&self.store);
        match asked {
            ProfileAction::Rename(name) => {
                if own.get()?.is_some() {
                    own.rename(&name)?;
                    // The field shows the name as kept.
                    self.profile_view.name_of = None;
                }
            }
            ProfileAction::Initials => {
                if own.get()?.is_some() {
                    own.set_picture(None)?;
                }
            }
            ProfileAction::PickPicture => {
                let Some(path) = (self.picture_picker)(self.t.profile_picture_choose()) else {
                    return Ok(());
                };
                // Upright and without metadata, as a pasted photo is kept.
                let square = std::fs::read(&path)
                    .ok()
                    .and_then(|bytes| photo_from_file(&bytes))
                    .and_then(Square::new);
                match square {
                    Some(square) => self.profile_view.square = Some(square),
                    None => self.notice = Some(self.t.profile_picture_unreadable().to_string()),
                }
            }
            ProfileAction::UseSquare => {
                let Some(square) = self.profile_view.square.take() else {
                    return Ok(());
                };
                match square.make() {
                    Some(picture) if own.get()?.is_some() => {
                        own.set_picture(Some(picture))?;
                    }
                    Some(_) => {}
                    None => self.notice = Some(self.t.profile_picture_unreadable().to_string()),
                }
            }
            ProfileAction::CancelSquare => self.profile_view.square = None,
            ProfileAction::Ask => {
                self.question_later = false;
                self.dialog = Some(Dialog::WhichProfile);
            }
            ProfileAction::Answer(keep_mine) => {
                self.dialog = None;
                own.answer(keep_mine)?;
            }
            ProfileAction::Later => {
                self.dialog = None;
                self.question_later = true;
            }
            ProfileAction::CopyCard => match self.store.profile_card(false)? {
                Some(link) => {
                    self.copy = Some(self.t.profile_card_text(&link));
                    self.card_form.copied = true;
                }
                None => self.notice = Some(self.t.profile_card_not_yet().to_string()),
            },
            ProfileAction::ShowQr => match self.store.profile_card(false)? {
                Some(link) => self.dialog = Some(Dialog::ProfileQr { link }),
                None => self.notice = Some(self.t.profile_card_not_yet().to_string()),
            },
            ProfileAction::OpenCard => {
                // Never in person here: the desktop scans nothing.
                let opened = self.store.open_profile_card(&self.card_form.link, false)?;
                match (opened.taken, opened.id) {
                    (CardTaken::Added | CardTaken::Updated | CardTaken::Unchanged, Some(id)) => {
                        self.card_form.link.clear();
                        self.card_form.answer = None;
                        let back = if opened.offer_back {
                            SendBack::Offered
                        } else {
                            SendBack::No
                        };
                        self.open_person(id, back)?;
                    }
                    (taken, _) => self.card_form.answer = Some(taken),
                }
            }
        }
        Ok(())
    }

    /// The strangers' Sent Items waiting (#91), the oldest first: who
    /// each is from and when it came, nothing of what it holds. A click
    /// opens its preview, hidden until "Show"; those left undecided, or
    /// by a restart, wait here until Accept or Decline.
    fn held_pane(&mut self, ui: &mut Ui, tones: Tones, actions: &mut Vec<Action>) {
        let t = self.t;
        let text = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        ui.heading(RichText::new(t.sent_items_waiting_title()).color(text));
        ui.add_space(8.0);
        if self.view.held.is_empty() {
            ui.label(RichText::new(t.sent_items_none_waiting()).color(weak));
            return;
        }
        egui::ScrollArea::vertical()
            .id_salt("sent items waiting")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 12.0;
                ui.add_space(4.0);
                for held in &self.view.held {
                    let from = t.sent_item_from(profile_view::display_name(&t, &held.sender.name));
                    let id = Id::new(("held", &held.id));
                    let clicked = clickable_card(ui, tones, (id, &from), |ui| {
                        ui.horizontal(|ui| {
                            icons::show(ui, icons::MAIL_UNREAD, 20.0, tones.accent);
                            ui.vertical(|ui| {
                                ui.label(RichText::new(&from).color(text));
                                ui.label(RichText::new(when(&t, held.held)).small().color(weak));
                            });
                        });
                    });
                    if clicked {
                        actions.push(Action::OpenHeld(held.id.clone()));
                    }
                }
            });
    }

    /// The Areas shared from afar waiting (#108), the oldest first: who
    /// shares which Area as what, and when it came. A click opens its
    /// preview; those left undecided, or by a restart, wait here until
    /// Accept or Decline.
    fn offers_pane(&mut self, ui: &mut Ui, tones: Tones, actions: &mut Vec<Action>) {
        let t = self.t;
        let text = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        ui.heading(RichText::new(t.area_offers_waiting_title()).color(text));
        ui.add_space(8.0);
        if self.view.offers.is_empty() {
            ui.label(RichText::new(t.area_offers_none_waiting()).color(weak));
            return;
        }
        egui::ScrollArea::vertical()
            .id_salt("shared areas waiting")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 12.0;
                ui.add_space(4.0);
                for held in &self.view.offers {
                    let line = offer_line(&t, held);
                    let id = Id::new(("offer", &held.id));
                    let clicked = clickable_card(ui, tones, (id, &line), |ui| {
                        ui.horizontal(|ui| {
                            icons::show(ui, icons::GROUP_ADD, 20.0, tones.accent);
                            ui.vertical(|ui| {
                                ui.label(RichText::new(&line).color(text));
                                ui.label(RichText::new(when(&t, held.held)).small().color(weak));
                            });
                        });
                    });
                    if clicked {
                        actions.push(Action::OpenOffer(held.id.clone()));
                    }
                }
            });
    }

    /// This Device's settings (ADR 0008): its notifications and login
    /// start, its Re-nag interval and alarm sound, and the Nudges' repeat
    /// and Quiet Hours.
    fn settings_pane(&mut self, ui: &mut Ui, tones: Tones, actions: &mut Vec<Action>) {
        let t = self.t;
        let text = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        let settings = DeviceSettings::new(&self.store);
        let alerts = settings.alerts_on().unwrap_or(true);
        let autostart = settings.autostart_on().unwrap_or(true);
        let renag = settings
            .renag_interval()
            .map(|d| d.num_minutes())
            .unwrap_or(10);
        let repeat = settings
            .nudge_repeat()
            .map(|d| d.num_minutes())
            .unwrap_or(60);
        let quiet_on = settings.quiet_hours_on().unwrap_or(true);
        let quiet = settings
            .quiet_hours()
            .unwrap_or(oldbrainfix_core::settings::DEFAULT_QUIET_HOURS);
        // Unreadable, the own chime rings (alerts.rs).
        let sound = settings.alarm_sound().ok().flatten();
        let sound_rung = match &sound {
            None => None,
            // Looked for every frame: a file deleted or a drive taken
            // out shows at once.
            Some(path) if !std::path::Path::new(path).exists() => Some(Rung::Own),
            Some(path) => {
                if self.sound_rung.as_ref().is_none_or(|(of, _)| of != path) {
                    self.sound_rung = Some((path.clone(), ring::alarm_sound(Some(path)).1));
                }
                self.sound_rung.as_ref().map(|(_, rung)| *rung)
            }
        };
        ui.heading(RichText::new(t.settings_title()).color(text));
        ui.label(RichText::new(t.settings_intro()).color(weak));
        ui.add_space(8.0);
        let quiet_text = &mut self.quiet_text;
        let hotkey_text = &mut self.hotkey_text;
        let stored_hotkey = settings
            .capture_hotkey()
            .unwrap_or_else(|_| oldbrainfix_core::settings::DEFAULT_CAPTURE_HOTKEY.into());
        let hotkey_status = self.hotkey.as_ref().map(|h| h.borrow().status());
        let speech = self.speech.as_ref();
        let ai_url = self.ai_port.as_ref().map(AiPort::url);
        let ai_waiting = self.ai_port.is_none() && self.view.ai_task.is_some();
        let ai_copied = self.ai_copied;
        let units = settings.chosen_unit_system().ok().flatten();
        let region_units = oldbrainfix_core::tally_units::system_for_region(self.region.as_deref());
        egui::ScrollArea::vertical()
            .id_salt("settings")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 16.0;
                ui.add_space(4.0);
                embossed(ui, tones, false, None, |ui| {
                    ui.label(RichText::new(t.settings_computer()).strong().color(text));
                    let mut on = alerts;
                    if ui
                        .checkbox(&mut on, RichText::new(t.alerts_switch()).color(text))
                        .changed()
                    {
                        actions.push(Action::Setting(Setting::Alerts(on)));
                    }
                    ui.label(RichText::new(t.alerts_detail()).small().color(weak));
                    let mut on = autostart;
                    if ui
                        .checkbox(&mut on, RichText::new(t.autostart_switch()).color(text))
                        .changed()
                    {
                        actions.push(Action::Setting(Setting::Autostart(on)));
                    }
                    ui.label(RichText::new(t.autostart_detail()).small().color(weak));
                });
                if let Some(asked) =
                    ai_view::port_card(ui, &t, tones, ai_url.as_deref(), ai_waiting, ai_copied)
                {
                    actions.push(Action::Ai(asked));
                }
                embossed(ui, tones, false, None, |ui| {
                    if ui
                        .button(RichText::new(t.about_title()).color(text))
                        .clicked()
                    {
                        actions.push(Action::Select(Selection::About));
                    }
                    ui.label(RichText::new(t.about_settings_detail()).small().color(weak));
                });
                embossed(ui, tones, false, None, |ui| {
                    quick_capture_card(ui, &t, hotkey_text, &stored_hotkey, hotkey_status, actions);
                });
                if let Some(speech) = speech {
                    embossed(ui, tones, false, None, |ui| {
                        if let Some(asked) = media_view::speech_card(ui, &t, speech) {
                            actions.push(Action::Speech(asked));
                        }
                    });
                }
                embossed(ui, tones, false, None, |ui| {
                    ui.label(RichText::new(t.settings_alarms()).strong().color(text));
                    let mut choices: Vec<i64> = RENAG_CHOICES.to_vec();
                    if !choices.contains(&renag) {
                        choices.push(renag);
                        choices.sort();
                    }
                    minutes_choice(ui, t.renag_device(), &t, renag, &choices, |m| {
                        actions.push(Action::Setting(Setting::Renag(m)));
                    });
                    ui.label(RichText::new(t.settings_renag_detail()).small().color(weak));
                    alarm_sound_choice(ui, &t, sound.as_deref(), sound_rung, actions);
                });
                embossed(ui, tones, false, None, |ui| {
                    ui.label(RichText::new(t.settings_nudges()).strong().color(text));
                    ui.label(
                        RichText::new(t.nudge_settings_explain())
                            .small()
                            .color(weak),
                    );
                    let mut choices: Vec<i64> = NUDGE_REPEAT_CHOICES.to_vec();
                    if !choices.contains(&repeat) {
                        choices.push(repeat);
                        choices.sort();
                    }
                    minutes_choice(ui, t.nudge_repeat(), &t, repeat, &choices, |m| {
                        actions.push(Action::Setting(Setting::NudgeRepeat(m)));
                    });
                    let mut on = quiet_on;
                    if ui
                        .checkbox(&mut on, RichText::new(t.quiet_hours()).color(text))
                        .changed()
                    {
                        actions.push(Action::Setting(Setting::QuietOn(on)));
                    }
                    if !quiet_on {
                        return;
                    }
                    let mut typed = None;
                    ui.horizontal(|ui| {
                        for (label, value, id, start) in [
                            (t.quiet_from(), &mut quiet_text.0, "quiet from", true),
                            (t.quiet_until(), &mut quiet_text.1, "quiet until", false),
                        ] {
                            ui.label(RichText::new(label).color(text));
                            let field = ui.add(
                                TextEdit::singleline(value)
                                    .id(Id::new(id))
                                    .hint_text(t.time_hint())
                                    .text_color(text)
                                    .desired_width(60.0),
                            );
                            if field.changed()
                                && let Some(time) = ClockTime::parse(value.trim())
                            {
                                typed = Some((start, time));
                            }
                        }
                    });
                    if let Some((start, time)) = typed {
                        let changed = if start {
                            QuietHours {
                                start: time,
                                ..quiet
                            }
                        } else {
                            QuietHours { end: time, ..quiet }
                        };
                        if changed != quiet {
                            actions.push(Action::Setting(Setting::Quiet(changed)));
                        }
                    }
                    let unreadable = [&quiet_text.0, &quiet_text.1]
                        .iter()
                        .any(|v| ClockTime::parse(v.trim()).is_none());
                    if unreadable {
                        ui.label(
                            RichText::new(t.time_invalid()).color(ui.visuals().error_fg_color),
                        );
                    }
                });
                embossed(ui, tones, false, None, |ui| {
                    ui.label(RichText::new(t.settings_units()).strong().color(text));
                    ui.label(RichText::new(t.settings_units_detail()).small().color(weak));
                    let region = match region_units {
                        UnitSystem::Metric => t.unit_system_metric_name(),
                        UnitSystem::Imperial => t.unit_system_imperial_name(),
                    };
                    let options = [
                        (None, t.unit_system_region(region)),
                        (Some(UnitSystem::Metric), t.unit_system_metric().to_string()),
                        (
                            Some(UnitSystem::Imperial),
                            t.unit_system_imperial().to_string(),
                        ),
                    ];
                    let mut chosen = units;
                    if radio_column(ui, &mut chosen, options) && chosen != units {
                        actions.push(Action::Setting(Setting::Units(chosen)));
                    }
                });
            });
    }

    /// The Routines and Reminders in the trash on their own, each with
    /// its Restore.
    fn trashed_routines(&self, ui: &mut Ui, tones: Tones, actions: &mut Vec<Action>) {
        let t = self.t;
        let text = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        embossed(ui, tones, false, None, |ui| {
            ui.label(RichText::new(t.routines_heading()).strong().color(text));
            for (routine, note) in &self.view.trashed_routines {
                ui.horizontal_wrapped(|ui| {
                    let what = row_title(&t, note.as_ref());
                    ui.label(RichText::new(what).color(text));
                    ui.label(
                        RichText::new(schedule_text(&t, &routine.schedule))
                            .small()
                            .color(weak),
                    );
                    if ui.button(RichText::new(t.restore()).color(text)).clicked() {
                        actions.push(Action::RestoreRoutine(routine.id.clone()));
                    }
                });
            }
        });
    }

    /// The Frogs in the trash, most recently deleted first, each with its
    /// Restore for who may manage it (ADR 0013); a Frog in the trash is
    /// never purged (ADR 0027).
    fn trashed_frogs(&self, ui: &mut Ui, tones: Tones, actions: &mut Vec<Action>) {
        let t = self.t;
        let text = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        embossed(ui, tones, false, None, |ui| {
            ui.horizontal(|ui| {
                frogs_view::frog_mark(ui, 16.0, text);
                ui.label(RichText::new(t.frogs_title()).strong().color(text));
            });
            for (frog, note, manages) in &self.view.trashed_frogs {
                ui.horizontal_wrapped(|ui| {
                    let what = row_title(&t, note.as_ref());
                    ui.label(RichText::new(what).color(text));
                    ui.label(RichText::new(t.frog()).small().color(weak));
                    if *manages && ui.button(RichText::new(t.restore()).color(text)).clicked() {
                        actions.push(Action::RestoreFrog(frog.id.clone()));
                    }
                });
            }
        });
    }

    /// The card's text next to the pointer while a card is dragged.
    fn dragged(&self, ctx: &egui::Context, tones: Tones) {
        let Some(dragged) = egui::DragAndDrop::payload::<DraggedNote>(ctx) else {
            return;
        };
        let Some(pos) = ctx.pointer_interact_pos() else {
            return;
        };
        let Some(placed) = self.view.notes.iter().find(|p| p.note.id == dragged.0) else {
            return;
        };
        ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
        egui::Area::new(Id::new("dragged"))
            .order(egui::Order::Tooltip)
            .fixed_pos(pos + egui::vec2(14.0, 14.0))
            .interactable(false)
            .show(ctx, |ui| {
                Frame::NONE
                    .fill(tones.card)
                    .corner_radius(8)
                    .inner_margin(Margin::same(8))
                    .show(ui, |ui| {
                        ui.set_max_width(240.0);
                        let text = ui.visuals().text_color();
                        ui.label(RichText::new(&placed.note.text).color(text));
                    });
            });
    }
}

/// Selects the Area or Topic `place` when its row is clicked, and files a
/// card dropped on it there.
fn place_row(resp: &egui::Response, place: &str, actions: &mut Vec<Action>) {
    if resp.clicked() {
        actions.push(Action::Select(Selection::Place(place.to_string())));
    }
    if let Some(dragged) = resp.dnd_release_payload::<DraggedNote>() {
        actions.push(Action::File {
            note: dragged.0.clone(),
            place: place.to_string(),
        });
    }
}

/// A square button that shows `icon`, called `label`.
pub(crate) fn icon_button(ui: &mut Ui, icon: char, label: &str, colour: Color32) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), Sense::click());
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, label));
    if resp.hovered() {
        ui.painter()
            .rect_filled(rect, 6.0, ui.visuals().widgets.hovered.weak_bg_fill);
    }
    icons::paint(ui.painter(), rect.shrink(2.0), icon, colour);
    resp.on_hover_text(label)
}

/// The heading of a dialog.
fn title(ui: &mut Ui, title: &str) {
    let text = ui.visuals().text_color();
    ui.label(RichText::new(title).size(18.0).strong().color(text));
}

/// A dialog's text field, taking the keyboard when `focus` says so.
fn focused_field(
    ui: &mut Ui,
    (value, focus): (&mut String, bool),
    id: &str,
    hint: &str,
) -> egui::Response {
    let text = ui.visuals().text_color();
    let field = ui.add(
        TextEdit::singleline(value)
            .id(Id::new(id))
            .hint_text(hint)
            .text_color(text)
            .desired_width(f32::INFINITY),
    );
    if focus {
        field.request_focus();
    }
    field
}

/// Where Report sends a problem's details: the developer's mail, as on
/// the phone (crash_guard.dart).
const REPORT_MAIL: &str = "taum@tuta.io";

/// What a failure announced at the top was (UI laws, "Announcements"):
/// `heading`, the cause and the fix in one sentence each, the full
/// `details` to select and copy, Close, and Report, which opens a mail
/// with the details to the developer that the user still sends.
fn failure_form(
    ui: &mut Ui,
    t: &L10n,
    heading: &str,
    (cause, fix): (&str, &str),
    details: &str,
    actions: &mut Vec<Action>,
) {
    title(ui, heading);
    let text = ui.visuals().text_color();
    ui.label(RichText::new(format!("{cause} {fix}")).color(text));
    Frame::NONE
        .fill(ui.visuals().extreme_bg_color)
        .corner_radius(8.0)
        .inner_margin(Margin::same(8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.add(
                egui::Label::new(RichText::new(details).monospace().color(text)).selectable(true),
            );
        });
    ui.horizontal(|ui| {
        if ui.button(RichText::new(t.close()).color(text)).clicked() {
            actions.push(Action::CloseDialog);
        }
        if ui.button(RichText::new(t.report()).color(text)).clicked() {
            ui.ctx()
                .open_url(egui::OpenUrl::new_tab(report_url(details)));
            actions.push(Action::CloseDialog);
        }
    });
}

/// A `mailto:` link to [`REPORT_MAIL`] carrying `details` and this
/// app's version.
fn report_url(details: &str) -> String {
    let body = format!(
        "{} {}\n{}\n\n{details}",
        oldbrainfix_core::APP_NAME,
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
    );
    format!(
        "mailto:{REPORT_MAIL}?subject={}&body={}",
        percent_encoded(&format!("{} problem report", oldbrainfix_core::APP_NAME)),
        percent_encoded(&body),
    )
}

/// `text` with everything but unreserved characters percent-encoded,
/// as a URL's query takes it.
fn percent_encoded(text: &str) -> String {
    text.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                char::from(b).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// The name of the file at `path`, as a drop or a dialog hands it over.
fn file_name(path: &std::path::Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// The details line of a recording from the file `name` that cannot be
/// transcribed: the problem and the file's ending, not its name, which
/// may say more than a report should.
fn problem_line(problem: oldbrainfix_whisper::Problem, name: &str) -> String {
    let kind = match problem {
        oldbrainfix_whisper::Problem::Unreadable => "unreadable",
        oldbrainfix_whisper::Problem::Unsupported => "unsupported",
    };
    match std::path::Path::new(name).extension() {
        Some(ending) => format!("{kind}: .{}", ending.to_string_lossy()),
        None => kind.to_string(),
    }
}

/// Cancel, which closes the dialog without a change.
fn cancel(ui: &mut Ui, t: &L10n, actions: &mut Vec<Action>) {
    let text = ui.visuals().text_color();
    if ui.button(RichText::new(t.cancel()).color(text)).clicked() {
        actions.push(Action::CloseDialog);
    }
}

/// The setting of this Device that keeps the "Blocked" section of People
/// open (#90), the phone's.
const BLOCKED_OPEN: &str = "peopleBlockedOpen";

/// The local setting that keeps the Frog view's "Done" section open, as
/// the phone names it.
const FROGS_DONE_OPEN: &str = "frogsDoneOpen";

/// The setting of this Device that keeps the Note list `key` compact
/// (#114), the phone's; none while it shows in full, the default.
fn compact_setting(key: &str) -> String {
    format!("compactNotes:{key}")
}

/// Which Note list `selection` shows, by the phone's names, each keeping
/// its own full/compact choice (#114): all Notes, or a person's while
/// `people` keeps only some, Unsorted, an Area (`area`) or a Topic. A
/// search inside one follows its choice; the trash has none.
fn cards_key(selection: &Selection, people: &PersonFilter, area: bool) -> Option<String> {
    Some(match selection {
        Selection::All if people.is_everyone() => "allNotes".to_string(),
        Selection::All => "personNotes".to_string(),
        Selection::Unsorted => "unsorted".to_string(),
        Selection::Place(id) if area => format!("area:{id}"),
        Selection::Place(id) => format!("topic:{id}"),
        _ => return None,
    })
}

/// A person in People: their picture and name, either opening their
/// Profile.
fn people_row(
    ui: &mut Ui,
    t: &L10n,
    pictures: &mut profile_view::Pictures,
    person: &Profile,
    asked: &mut Vec<String>,
) {
    let text = ui.visuals().text_color();
    let name = profile_view::display_name(t, &person.name).to_string();
    ui.horizontal(|ui| {
        let open = t.person_open(&name);
        let picture = profile_view::person_button(ui, pictures, person, 32.0, &open);
        let line = ui.add(egui::Button::new(RichText::new(&name).color(text)).frame(false));
        if picture.clicked() || line.clicked() {
            asked.push(person.id.clone());
        }
    });
}

/// The names of the Areas `ids` as shown, separated by commas.
fn area_names(t: &L10n, view: &View, ids: &[String]) -> String {
    ids.iter()
        .filter_map(|id| view.area(id))
        .map(|a| area_name(t, a))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Whether to block the person shown as `name` (#90, ADR 0021), as the
/// phone asks: when they manage Shared Areas this Device is in, `areas`
/// names them, and blocking leaves them.
fn block_form(ui: &mut Ui, t: &L10n, name: &str, areas: &str, actions: &mut Vec<Action>) {
    let text = ui.visuals().text_color();
    title(ui, &t.person_block_question(name));
    let (detail, verb) = if areas.is_empty() {
        (t.person_block_detail(name), t.person_block())
    } else {
        (
            t.person_block_leave_detail(name, areas),
            t.person_block_leave(),
        )
    };
    ui.label(RichText::new(detail).color(text));
    ui.horizontal(|ui| {
        if ui.button(RichText::new(t.cancel()).color(text)).clicked() {
            actions.push(Action::CancelBlock);
        }
        if destructive(ui, verb) {
            actions.push(Action::Block);
        }
    });
}

/// Whether to end the block of the person shown as `name` (#90).
fn unblock_form(ui: &mut Ui, t: &L10n, name: &str, actions: &mut Vec<Action>) {
    let text = ui.visuals().text_color();
    title(ui, &t.person_unblock_question(name));
    ui.label(RichText::new(t.person_unblock_detail()).color(text));
    ui.horizontal(|ui| {
        if ui.button(RichText::new(t.cancel()).color(text)).clicked() {
            actions.push(Action::CancelBlock);
        }
        if ui
            .button(RichText::new(t.person_unblock()).color(text))
            .clicked()
        {
            actions.push(Action::Unblock);
        }
    });
}

/// A confirm button for a destructive step: its verb in the error colour
/// (UI laws).
fn destructive(ui: &mut Ui, label: &str) -> bool {
    let error = ui.visuals().error_fg_color;
    ui.button(RichText::new(label).color(error)).clicked()
}

/// Asks for a name: `(title, verb, done)` say what for. Enter takes it.
fn name_form(
    ui: &mut Ui,
    t: &L10n,
    (heading, verb, done): (&str, &str, Action),
    (name, focus): (&mut String, bool),
    actions: &mut Vec<Action>,
) {
    title(ui, heading);
    let field = focused_field(ui, (&mut *name, focus), "name", t.name_hint());
    let ready = !name.trim().is_empty();
    if ready && field.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
        actions.push(done.clone());
    }
    ui.horizontal(|ui| {
        cancel(ui, t, actions);
        let text = ui.visuals().text_color();
        let button = egui::Button::new(RichText::new(verb).color(text));
        if ui.add_enabled(ready, button).clicked() {
            actions.push(done);
        }
    });
}

/// How an Area is shared: the role, and the button that shows the
/// invite's code. Nobody types a name: the other person's Profile shows
/// when their phone joins, to confirm (ADR 0017). Below, "Share with…"
/// gives it with the same role to a person in People who is not nearby
/// (#108), when there is anybody it can go to.
fn share_form(
    ui: &mut Ui,
    t: &L10n,
    area: &str,
    (role, afar): (&mut AreaRole, bool),
    actions: &mut Vec<Action>,
) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    title(ui, &t.share_title(area));
    ui.label(RichText::new(t.share_host_intro()).color(weak));
    ui.label(RichText::new(t.role_heading()).strong().color(text));
    for (choice, label, detail) in [
        (AreaRole::Member, t.role_member(), t.role_member_detail()),
        (AreaRole::Manager, t.role_manager(), t.role_manager_detail()),
    ] {
        ui.radio_value(role, choice, RichText::new(label).color(text));
        ui.label(RichText::new(detail).small().color(weak));
    }
    ui.horizontal(|ui| {
        cancel(ui, t, actions);
        if ui
            .button(RichText::new(t.share_show_code()).color(text))
            .clicked()
        {
            actions.push(Action::StartInvite);
        }
    });
    ui.separator();
    ui.label(RichText::new(t.share_afar_heading()).strong().color(text));
    let intro = if afar {
        t.share_afar_intro()
    } else {
        t.share_afar_nobody()
    };
    ui.label(RichText::new(intro).color(weak));
    if afar
        && ui
            .button(RichText::new(t.share_afar_pick()).color(text))
            .clicked()
    {
        actions.push(Action::ShareAfar);
    }
}

/// "Share Household with…" (#108): the People the Area can go to from
/// afar, each with their picture and name as the user sees them and "Not
/// verified" under those whose code was never compared in person; a
/// click picks them.
fn share_person_form(
    ui: &mut Ui,
    t: &L10n,
    tones: Tones,
    (area, people): (&str, &[AfarPerson]),
    pictures: &mut profile_view::Pictures,
    actions: &mut Vec<Action>,
) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    title(ui, &t.share_with_title(area));
    egui::ScrollArea::vertical()
        .id_salt("share with")
        .max_height(420.0)
        .show(ui, |ui| {
            for person in people {
                let name = profile_view::display_name(t, &person.shown.name);
                let id = Id::new(("share to", &person.id));
                let clicked = clickable_card(ui, tones, (id, name), |ui| {
                    ui.horizontal(|ui| {
                        profile_view::person_picture(ui, pictures, &person.shown, 40.0, name);
                        ui.add_space(8.0);
                        ui.vertical(|ui| {
                            ui.label(RichText::new(name).color(text));
                            if !person.verified {
                                ui.label(RichText::new(t.share_not_verified()).small().color(weak));
                            }
                        });
                    });
                });
                if clicked {
                    actions.push(Action::ShareTo(person.id.clone()));
                }
            }
        });
    cancel(ui, t, actions);
}

/// "How do changes travel with Ann?" (#108), asked once per person: a
/// shared folder both reach, or files through a messenger.
fn transport_form(
    ui: &mut Ui,
    t: &L10n,
    name: &str,
    transport: &mut Transport,
    actions: &mut Vec<Action>,
) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    title(ui, &t.transport_title(name));
    for (choice, label, detail) in [
        (
            Transport::Folder,
            t.transport_folder(),
            t.transport_folder_detail(),
        ),
        (
            Transport::Bundle,
            t.transport_bundle(),
            t.transport_bundle_detail(),
        ),
    ] {
        ui.radio_value(transport, choice, RichText::new(label).color(text));
        ui.label(RichText::new(detail).small().color(weak));
    }
    ui.horizontal(|ui| {
        cancel(ui, t, actions);
        if ui
            .button(RichText::new(t.invite_confirm_share()).strong().color(text))
            .clicked()
        {
            actions.push(Action::ShareAfarNow);
        }
    });
}

/// The name of the Area `held` offers as the user reads it, or "an
/// Area" when the roles left the name out.
fn offer_area(t: &L10n, held: &HeldAreaOffer) -> String {
    let area = offered_area_name(t, held.area_starter.as_deref(), &held.area_name);
    if area.is_empty() {
        t.area_offer_no_name().to_string()
    } else {
        area
    }
}

/// "Ann shares Household with you as Member" for the offer `held`.
fn offer_line(t: &L10n, held: &HeldAreaOffer) -> String {
    let role = match held.role {
        AreaRole::Manager => t.role_manager(),
        AreaRole::Member => t.role_member(),
    };
    t.area_offer_text(
        profile_view::display_name(t, &held.sender.name),
        &offer_area(t, held),
        role,
    )
}

/// The preview of an Area shared with the user from afar (#108, ADR
/// 0023): who shares it, with their picture, verified or not with its
/// short code, the Area and the role it comes with, and how changes
/// travel; nothing of it is kept before Accept. "Compare codes" marks the
/// sharer verified on Accept, as for a Sent Item (#51). Leaving decides
/// nothing: the offer waits behind the Agenda's "x shared Areas waiting".
fn area_offer_form(
    ui: &mut Ui,
    t: &L10n,
    tones: Tones,
    (held, compared): (&HeldAreaOffer, Compared),
    pictures: &mut profile_view::Pictures,
    actions: &mut Vec<Action>,
) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let name = profile_view::display_name(t, &held.sender.name).to_string();
    let line = offer_line(t, held);
    ui.horizontal(|ui| {
        // A sharer kept here opens their Profile (#52); one not kept yet
        // has nothing to open until accepted.
        if held.trust == SenderTrust::Unknown {
            profile_view::person_picture(ui, pictures, &held.sender, 40.0, &name);
        } else if profile_view::person_button(
            ui,
            pictures,
            &held.sender,
            40.0,
            &t.person_open(&name),
        )
        .clicked()
        {
            actions.push(Action::OpenPerson(held.sender.id.clone()));
        }
        ui.add_space(8.0);
        ui.add(egui::Label::new(RichText::new(&line).strong().size(18.0).color(text)).wrap());
    });
    let code = &held.code;
    let trust = if compared == Compared::Match {
        SenderTrust::Verified
    } else {
        held.trust
    };
    let said = match trust {
        SenderTrust::Verified => t.sent_item_verified(code),
        SenderTrust::Known => t.sent_item_known(code),
        SenderTrust::Unknown => t.sent_item_unknown(code),
    };
    ui.label(RichText::new(said).color(text));
    match compared {
        Compared::Match if held.trust != SenderTrust::Verified => {
            ui.label(
                RichText::new(t.sent_item_verified_on_accept())
                    .small()
                    .color(weak),
            );
        }
        Compared::Differ => {
            ui.label(RichText::new(t.sent_item_codes_differed()).color(tones.accent));
        }
        Compared::Asking => {
            ui.label(RichText::new(t.sent_item_compare_hint(&name, code)).color(text));
            ui.horizontal(|ui| {
                if ui
                    .button(RichText::new(t.sent_item_codes_match()).color(text))
                    .clicked()
                {
                    actions.push(Action::CodesCompared(true));
                }
                if ui
                    .button(RichText::new(t.sent_item_codes_differ()).color(text))
                    .clicked()
                {
                    actions.push(Action::CodesCompared(false));
                }
            });
        }
        _ => {}
    }
    if trust != SenderTrust::Verified
        && compared != Compared::Asking
        && ui
            .button(RichText::new(t.sent_item_compare()).color(text))
            .clicked()
    {
        actions.push(Action::CompareCodes);
    }
    let travel = match held.transport {
        Transport::Folder => t.area_offer_by_folder(),
        Transport::Bundle => t.area_offer_by_bundle(),
    };
    ui.label(RichText::new(travel).color(text));
    ui.label(
        RichText::new(t.area_offer_keeps_nothing())
            .small()
            .color(weak),
    );
    ui.horizontal(|ui| {
        if ui
            .button(RichText::new(t.sent_item_accept()).strong().color(text))
            .clicked()
        {
            actions.push(Action::AcceptOffer);
        }
        if ui
            .button(RichText::new(t.sent_item_decline()).color(text))
            .clicked()
        {
            actions.push(Action::DeclineOffer);
        }
    });
    if ui
        .button(RichText::new(t.sent_item_block()).color(text))
        .clicked()
    {
        actions.push(Action::AskBlockSender);
    }
}

/// Asks whom a Note is sent to: one of the people whose keys are known
/// here, encrypted to them, or anyone, signed only, with a hint that
/// whoever holds the file can read it.
fn send_form(
    ui: &mut Ui,
    t: &L10n,
    recipients: &[Recipient],
    (picked, update, cancelling): (&mut Option<usize>, bool, &mut bool),
    actions: &mut Vec<Action>,
) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    title(
        ui,
        if update {
            t.send_update()
        } else {
            t.send_title()
        },
    );
    ui.label(RichText::new(t.send_who()).color(text));
    for (i, r) in recipients.iter().enumerate() {
        ui.radio_value(
            picked,
            Some(i),
            RichText::new(t.send_encrypted(profile_view::display_name(t, &r.name))).color(text),
        );
    }
    ui.radio_value(picked, None, RichText::new(t.send_anyone()).color(text));
    if picked.is_none() {
        ui.label(RichText::new(t.send_plain_hint()).color(text));
    }
    if update {
        ui.checkbox(cancelling, RichText::new(t.send_cancel()).color(text));
        ui.label(RichText::new(t.send_cancel_hint()).small().color(weak));
    }
    ui.label(RichText::new(t.send_what()).small().color(weak));
    ui.horizontal(|ui| {
        cancel(ui, t, actions);
        if ui
            .button(RichText::new(t.send_action()).color(text))
            .clicked()
        {
            actions.push(Action::SendNote);
        }
    });
}

/// What a Sent Item refused as `error` is announced as.
fn refused(t: &L10n, error: OpenError) -> &'static str {
    match error {
        OpenError::NotForUs => t.sent_item_not_for_us(),
        OpenError::NotASentItem => t.sent_item_unreadable(),
        OpenError::NotFromSender => t.sent_item_not_from_sender(),
        OpenError::FromBlocked => t.sent_item_from_blocked(),
    }
}

/// Whether accepting a Sent Item in `state` does anything: a new one,
/// an update or a cancellation.
fn acceptable(state: SentItemState) -> bool {
    matches!(
        state,
        SentItemState::Fresh | SentItemState::Update | SentItemState::Cancel
    )
}

/// The preview of a Sent Item: who sent it, with their Profile's picture
/// and name, verified or not with its short code, whether it came
/// encrypted, its text, Routines and Reminders with their times and
/// Sections; Accept or Decline. "Compare codes" asks the user to compare
/// the code with the sender in person; a match marks the Profile verified
/// on accept (ADR 0017, #51). An update shows what changes, "Tue 10:00 →
/// Wed 14:00", and Apply; a cancellation what it cancels; one already up
/// to date or with nothing to cancel only closes.
fn sent_item_form(
    ui: &mut Ui,
    t: &L10n,
    tones: Tones,
    ((opened, compared, may_block), pictures): (
        (&Opened, Compared, bool),
        &mut profile_view::Pictures,
    ),
    actions: &mut Vec<Action>,
) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let item = &opened.item;
    let name = profile_view::display_name(t, &opened.sender.name).to_string();
    ui.horizontal(|ui| {
        // A sender kept here opens their Profile (#52); one not kept has
        // nothing to open until the item is accepted.
        if opened.trust == SenderTrust::Unknown {
            profile_view::person_picture(ui, pictures, &opened.sender, 40.0, &name);
        } else if profile_view::person_button(
            ui,
            pictures,
            &opened.sender,
            40.0,
            &t.person_open(&name),
        )
        .clicked()
        {
            actions.push(Action::OpenPerson(opened.sender.id.clone()));
        }
        ui.add_space(8.0);
        title(ui, &t.sent_item_from(&name));
    });
    let code = &opened.code;
    let trust = if compared == Compared::Match {
        SenderTrust::Verified
    } else {
        opened.trust
    };
    let said = match trust {
        SenderTrust::Verified => t.sent_item_verified(code),
        SenderTrust::Known => t.sent_item_known(code),
        SenderTrust::Unknown => t.sent_item_unknown(code),
    };
    ui.label(RichText::new(said).color(text));
    let acceptable = acceptable(opened.state);
    match compared {
        Compared::Match if opened.trust != SenderTrust::Verified => {
            ui.label(
                RichText::new(t.sent_item_verified_on_accept())
                    .small()
                    .color(weak),
            );
        }
        Compared::Differ => {
            ui.label(RichText::new(t.sent_item_codes_differed()).color(tones.accent));
        }
        Compared::Asking => {
            ui.label(RichText::new(t.sent_item_compare_hint(&name, code)).color(text));
            ui.horizontal(|ui| {
                if ui
                    .button(RichText::new(t.sent_item_codes_match()).color(text))
                    .clicked()
                {
                    actions.push(Action::CodesCompared(true));
                }
                if ui
                    .button(RichText::new(t.sent_item_codes_differ()).color(text))
                    .clicked()
                {
                    actions.push(Action::CodesCompared(false));
                }
            });
        }
        _ => {}
    }
    if acceptable
        && trust != SenderTrust::Verified
        && compared != Compared::Asking
        && ui
            .button(RichText::new(t.sent_item_compare()).color(text))
            .clicked()
    {
        actions.push(Action::CompareCodes);
    }
    let sealed = if opened.encrypted {
        t.sent_item_encrypted()
    } else {
        t.sent_item_plain()
    };
    ui.label(RichText::new(sealed).small().color(weak));
    match opened.state {
        SentItemState::Fresh => {}
        SentItemState::Update => {
            ui.label(RichText::new(t.sent_item_update()).color(tones.accent));
        }
        SentItemState::Cancel => {
            ui.label(
                RichText::new(t.sent_item_cancelled(&item.text))
                    .strong()
                    .color(tones.accent),
            );
            ui.label(RichText::new(t.sent_item_cancel_detail()).color(text));
        }
        SentItemState::UpToDate => {
            ui.label(RichText::new(t.sent_item_already()).color(tones.accent));
        }
        SentItemState::NothingToCancel => {
            ui.label(RichText::new(t.sent_item_nothing_to_cancel()).color(tones.accent));
        }
    }
    let accept = match opened.state {
        SentItemState::Fresh => Some(t.sent_item_accept()),
        SentItemState::Update => Some(t.sent_item_apply()),
        SentItemState::Cancel => Some(t.sent_item_accept_cancel()),
        SentItemState::UpToDate | SentItemState::NothingToCancel => None,
    };
    ui.horizontal(|ui| {
        if let Some(accept) = accept
            && ui
                .button(RichText::new(accept).strong().color(text))
                .clicked()
        {
            actions.push(Action::AcceptSent);
        }
        let decline = if accept.is_some() {
            t.sent_item_decline()
        } else {
            t.close()
        };
        if ui.button(RichText::new(decline).color(text)).clicked() {
            actions.push(Action::DeclineSent);
        }
    });
    if may_block
        && ui
            .button(RichText::new(t.sent_item_block()).color(text))
            .clicked()
    {
        actions.push(Action::AskBlockSender);
    }
    let full = matches!(opened.state, SentItemState::Fresh | SentItemState::Update);
    egui::ScrollArea::vertical()
        .id_salt("sent item")
        .max_height(360.0)
        .show(ui, |ui| {
            if let Some(changes) = &opened.changes {
                ui.label(RichText::new(t.sent_item_changes()).strong().color(text));
                let mut line = |said: String| {
                    ui.label(RichText::new(said).color(text));
                };
                if changes.restores {
                    line(t.sent_item_restores().to_string());
                }
                if let Some(before) = &changes.text_before {
                    line(t.sent_item_text_was(before));
                    line(t.sent_item_text_now(&item.text));
                    if changes.text_edited {
                        line(t.sent_item_text_yours().to_string());
                    }
                }
                for s in &changes.sections_added {
                    line(t.sent_item_section_added(&s.text));
                }
                for r in &changes.rescheduled {
                    line(t.sent_item_rescheduled(
                        &schedule_text(t, &r.before),
                        &schedule_text(t, &r.after),
                    ));
                }
                for r in &changes.routines_added {
                    line(t.sent_item_routine_added(&schedule_text(t, &r.schedule)));
                }
                for r in &changes.routines_removed {
                    line(t.sent_item_routine_removed(&schedule_text(t, &r.schedule)));
                }
                if changes.is_empty() {
                    line(t.sent_item_no_changes().to_string());
                }
                ui.label(RichText::new(t.sent_item_keeps()).small().color(weak));
                ui.separator();
            }
            ui.label(RichText::new(&item.text).color(text));
            if !full {
                return;
            }
            if !item.routines.is_empty() {
                ui.label(RichText::new(t.routines_heading()).strong().color(text));
                for r in &item.routines {
                    ui.label(RichText::new(schedule_text(t, &r.schedule)).color(text));
                }
            }
            for section in &item.sections {
                ui.separator();
                ui.label(RichText::new(&section.text).color(text));
            }
        });
}

/// A card with `icon` and `said` on the Agenda, "x Unsorted" or "x Sent
/// Items waiting"; whether it was clicked.
fn count_banner(ui: &mut Ui, tones: Tones, (id, icon): (Id, char), said: &str) -> bool {
    let text = ui.visuals().text_color();
    clickable_card(ui, tones, (id, said), |ui| {
        ui.horizontal(|ui| {
            icons::show(ui, icon, 20.0, tones.accent);
            ui.label(RichText::new(said).color(text));
        });
    })
}

/// An embossed card holding `content` that is clicked as a whole, a
/// button with `id` labelled `label`; whether it was clicked.
pub(crate) fn clickable_card(
    ui: &mut Ui,
    tones: Tones,
    (id, label): (Id, &str),
    content: impl FnOnce(&mut Ui),
) -> bool {
    let card = embossed(ui, tones, false, None, content);
    let card = ui.interact(card.rect, id, Sense::click());
    card.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, label));
    card.on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}

/// The preview of a stranger's Sent Item held until "Show" (#91, ADR
/// 0015, 0022): only who it is from and that they are unknown here, not
/// their picture or short code, nothing of what it holds. "Show" opens
/// the preview as for anybody else; Accept, Decline and "Decline and
/// block" work before it too. Leaving decides nothing: the item waits
/// behind the Agenda's "x Sent Items waiting".
fn held_sent_item_form(
    ui: &mut Ui,
    t: &L10n,
    tones: Tones,
    held: &HeldSentItem,
    actions: &mut Vec<Action>,
) {
    let text = ui.visuals().text_color();
    let name = profile_view::display_name(t, &held.sender.name);
    title(ui, &t.sent_item_from(name));
    ui.label(RichText::new(t.sent_item_stranger()).color(tones.accent));
    ui.label(RichText::new(t.sent_item_hidden()).color(text));
    let wide = |ui: &mut Ui, label: RichText| {
        ui.add_sized([ui.available_width(), 32.0], egui::Button::new(label))
            .clicked()
    };
    if wide(ui, RichText::new(t.sent_item_show()).strong().color(text)) {
        actions.push(Action::ShowSent);
    }
    ui.horizontal(|ui| {
        if ui
            .button(RichText::new(t.sent_item_accept()).color(text))
            .clicked()
        {
            actions.push(Action::AcceptSent);
        }
        if ui
            .button(RichText::new(t.sent_item_decline()).color(text))
            .clicked()
        {
            actions.push(Action::DeclineSent);
        }
    });
    if ui
        .button(RichText::new(t.sent_item_block()).color(text))
        .clicked()
    {
        actions.push(Action::AskBlockSender);
    }
}

/// The people of a Shared Area with their roles; a Manager changes them
/// and takes them out here.
fn people_form(
    ui: &mut Ui,
    t: &L10n,
    area: &str,
    (people, names, mine): (
        &[Person],
        &std::collections::HashMap<String, String>,
        Option<&str>,
    ),
    (shown, pictures): (
        &std::collections::HashMap<String, Profile>,
        &mut profile_view::Pictures,
    ),
    manages: bool,
    actions: &mut Vec<Action>,
) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    title(ui, t.people_title());
    ui.label(RichText::new(t.people_intro()).small().color(weak));
    for person in people {
        let own = Some(person.id.as_str()) == mine;
        let name = person_name(t, names, person);
        let name = if own { t.people_you(&name) } else { name };
        let role = match person.role {
            AreaRole::Manager => t.role_manager(),
            AreaRole::Member => t.role_member(),
        };
        ui.horizontal(|ui| {
            // Another person's picture opens their Profile (#52).
            if let Some(profile) = shown.get(&person.id) {
                if own {
                    profile_view::person_picture(ui, pictures, profile, 32.0, &name);
                } else if profile_view::person_button(
                    ui,
                    pictures,
                    profile,
                    32.0,
                    &t.person_open(&name),
                )
                .clicked()
                {
                    actions.push(Action::OpenPerson(person.id.clone()));
                }
            }
            ui.label(RichText::new(&name).color(text));
            ui.label(RichText::new(role).small().color(weak));
            if !manages || own {
                return;
            }
            let (label, next) = match person.role {
                AreaRole::Member => (t.make_manager(), AreaRole::Manager),
                AreaRole::Manager => (t.make_member(), AreaRole::Member),
            };
            if ui.button(RichText::new(label).color(text)).clicked() {
                actions.push(Action::SetRole {
                    area: area.to_string(),
                    person: person.id.clone(),
                    role: next,
                });
            }
            if ui
                .button(RichText::new(t.remove_person()).color(text))
                .clicked()
            {
                actions.push(Action::AskRemovePerson {
                    area: area.to_string(),
                    person: person.id.clone(),
                });
            }
        });
    }
    ui.horizontal(|ui| {
        if ui.button(RichText::new(t.close()).color(text)).clicked() {
            actions.push(Action::CloseDialog);
        }
    });
}

/// An Area's colour and icon, each kept as it is picked.
fn look_form(ui: &mut Ui, t: &L10n, area: Option<&Area>, actions: &mut Vec<Action>) {
    let Some(area) = area else {
        return;
    };
    let text = ui.visuals().text_color();
    title(ui, t.area_look());
    ui.label(RichText::new(t.colour_heading()).strong().color(text));
    ui.horizontal_wrapped(|ui| {
        if ui
            .button(RichText::new(t.no_colour()).color(text))
            .clicked()
        {
            actions.push(Action::Colour(area.id.clone(), None));
        }
        for hex in AREA_PALETTE {
            let on = area.colour.as_deref() == Some(hex);
            let colour = area_colour(Some(hex)).unwrap_or(text);
            if swatch(ui, hex, on, |painter, rect| {
                painter.circle_filled(rect.center(), rect.width() / 2.0 - 3.0, colour);
            })
            .clicked()
            {
                actions.push(Action::Colour(area.id.clone(), Some(hex.to_string())));
            }
        }
    });
    ui.label(RichText::new(t.icon_heading()).strong().color(text));
    ui.horizontal_wrapped(|ui| {
        if ui.button(RichText::new(t.no_icon()).color(text)).clicked() {
            actions.push(Action::Icon(area.id.clone(), None));
        }
        for key in AREA_ICONS {
            let Some(glyph) = icons::area_icon(Some(key)) else {
                continue;
            };
            let on = area.icon.as_deref() == Some(key);
            if swatch(ui, key, on, |painter, rect| {
                icons::paint(painter, rect.shrink(5.0), glyph, text);
            })
            .clicked()
            {
                actions.push(Action::Icon(area.id.clone(), Some(key.to_string())));
            }
        }
    });
    if ui.button(RichText::new(t.close()).color(text)).clicked() {
        actions.push(Action::CloseDialog);
    }
}

/// A round choice called `label`, ringed while it is `on`.
fn swatch(
    ui: &mut Ui,
    label: &str,
    on: bool,
    paint: impl FnOnce(&egui::Painter, egui::Rect),
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(32.0, 32.0), Sense::click());
    resp.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, on, label));
    paint(ui.painter(), rect);
    if on || resp.hovered() {
        let ring = if on {
            ui.visuals().text_color()
        } else {
            ui.visuals().weak_text_color()
        };
        ui.painter().circle_stroke(
            rect.center(),
            rect.width() / 2.0 - 1.0,
            egui::Stroke::new(2.0, ring),
        );
    }
    resp
}

/// Asks once whether to delete an Area or Topic and, with Notes inside,
/// where they go: Unsorted, the Topic's Area or the trash.
fn delete_form(ui: &mut Ui, t: &L10n, deleting: &Deleting, actions: &mut Vec<Action>) {
    let text = ui.visuals().text_color();
    title(ui, &deleting.question);
    for line in &deleting.lines {
        ui.label(RichText::new(line).color(text));
    }
    ui.horizontal_wrapped(|ui| {
        cancel(ui, t, actions);
        if !deleting.has_notes {
            if destructive(ui, t.delete()) {
                actions.push(Action::DeletePlace(NotesGo::Unsorted));
            }
            return;
        }
        if destructive(ui, t.move_to_unsorted()) {
            actions.push(Action::DeletePlace(NotesGo::Unsorted));
        }
        if let Some(area) = &deleting.area
            && destructive(ui, &t.move_to_area(area))
        {
            actions.push(Action::DeletePlace(NotesGo::Area));
        }
        if destructive(ui, t.trash_them()) {
            actions.push(Action::DeletePlace(NotesGo::Trash));
        }
    });
}

/// Picks the Note to merge the selected one into, from every other Note,
/// newest first, narrowed by what their text holds.
fn merge_form(
    ui: &mut Ui,
    t: &L10n,
    tones: Tones,
    (query, focus): (&mut String, bool),
    others: &[(String, String)],
    actions: &mut Vec<Action>,
) {
    let weak = ui.visuals().weak_text_color();
    title(ui, t.merge_into());
    let field = focused_field(ui, (&mut *query, focus), "merge query", t.search_all_hint());
    if field.changed() {
        actions.push(Action::MergeQuery);
    }
    // The core's search, as on the phone: Transcripts, Photo Texts and
    // Sections count too.
    let mut shown = others.iter().peekable();
    if shown.peek().is_none() {
        let empty = if query.trim().is_empty() {
            t.no_other_notes()
        } else {
            t.nothing_found()
        };
        ui.label(RichText::new(empty).color(weak));
    }
    egui::ScrollArea::vertical()
        .id_salt("merge")
        .max_height(320.0)
        .show(ui, |ui| {
            for (id, text) in shown {
                if row(ui, tones, false, 0.0, None, text, false).clicked() {
                    actions.push(Action::Merge(id.clone()));
                }
            }
        });
    cancel(ui, t, actions);
}
/// How much tomorrow and the days after are faded on the Agenda (the
/// phone's `tomorrowOpacity`).
pub const TOMORROW_OPACITY: f32 = 0.55;

/// How many days the Agenda shows, today included (the phone's
/// `AgendaScreen.days`).
const AGENDA_DAYS: usize = 7;

/// The name `person`'s Profile shows (`names`, by Profile id), or "No
/// name".
fn person_name(
    t: &L10n,
    names: &std::collections::HashMap<String, String>,
    person: &Person,
) -> String {
    let name = names.get(&person.id).map_or("", String::as_str);
    profile_view::display_name(t, name).to_string()
}

/// The Settings' card on the capture popup's hotkey and the
/// `oldbrainfix capture` command: the chord as typed, and how the hotkey
/// stands on this desktop.
fn quick_capture_card(
    ui: &mut Ui,
    t: &L10n,
    typed: &mut String,
    stored: &str,
    status: Option<hotkey::Status>,
    actions: &mut Vec<Action>,
) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    ui.label(
        RichText::new(t.quick_capture_heading())
            .strong()
            .color(text),
    );
    let mut chord = None;
    ui.horizontal(|ui| {
        ui.label(RichText::new(t.hotkey_label()).color(text));
        let field = ui.add(
            TextEdit::singleline(typed)
                .id(Id::new("hotkey"))
                .hint_text(oldbrainfix_core::settings::DEFAULT_CAPTURE_HOTKEY)
                .text_color(text)
                .desired_width(180.0),
        );
        chord = typed.parse::<Chord>().ok();
        // Kept once typed out, so no key on the way is grabbed.
        if field.lost_focus()
            && let Some(chord) = chord
            && chord.to_string() != stored
        {
            actions.push(Action::Setting(Setting::Hotkey(chord.to_string())));
        }
    });
    let line = match (chord, status) {
        (None, _) => {
            ui.label(RichText::new(t.hotkey_invalid()).color(ui.visuals().error_fg_color));
            None
        }
        (Some(_), None) => None,
        (Some(_), Some(hotkey::Status::Active)) => Some(t.hotkey_active().to_string()),
        (Some(_), Some(hotkey::Status::Waiting)) => Some(t.hotkey_waiting().to_string()),
        (Some(_), Some(hotkey::Status::Desktop(key))) => Some(match key {
            Some(key) => format!("{} {}", t.hotkey_desktop(), t.hotkey_desktop_key(&key)),
            None => t.hotkey_desktop().to_string(),
        }),
        (Some(_), Some(hotkey::Status::Taken)) => Some(t.hotkey_taken().to_string()),
        (Some(_), Some(hotkey::Status::Unavailable)) => Some(t.hotkey_unavailable().to_string()),
    };
    if let Some(line) = line {
        ui.label(RichText::new(line).small().color(weak));
    }
    ui.label(RichText::new(t.capture_command()).small().color(weak));
}

/// The Settings' alarm sound (#102): the own chime or the file chosen,
/// by its name, and whether that file plays (`rung`), with the choice of
/// another file and the way back to the own chime.
fn alarm_sound_choice(
    ui: &mut Ui,
    t: &L10n,
    sound: Option<&str>,
    rung: Option<Rung>,
    actions: &mut Vec<Action>,
) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    ui.label(RichText::new(t.alarm_sound()).color(text));
    match sound {
        None => {
            ui.label(RichText::new(t.alarm_sound_own()).color(text));
        }
        Some(path) => {
            let name = std::path::Path::new(path)
                .file_name()
                .map_or_else(|| path.into(), |n| n.to_string_lossy());
            ui.label(RichText::new(t.alarm_sound_file(&name)).color(text))
                .on_hover_text(path);
            if rung == Some(Rung::Own) {
                ui.label(
                    RichText::new(t.alarm_sound_unplayable()).color(ui.visuals().error_fg_color),
                );
            }
        }
    }
    ui.horizontal_wrapped(|ui| {
        if ui
            .button(RichText::new(t.alarm_sound_pick()).color(text))
            .clicked()
        {
            actions.push(Action::PickAlarmSound);
        }
        if sound.is_some()
            && ui
                .button(RichText::new(t.alarm_sound_use_own()).color(text))
                .clicked()
        {
            actions.push(Action::Setting(Setting::AlarmSound(None)));
        }
    });
    ui.label(RichText::new(t.alarm_sound_detail()).small().color(weak));
}

/// The Nudge repeats offered, in minutes (the phone's choices).
pub const NUDGE_REPEAT_CHOICES: [i64; 3] = [10, 30, 60];

/// `label`, and below it a column of radio buttons for `minutes`;
/// `picked` gets a new one.
fn minutes_choice(
    ui: &mut Ui,
    label: &str,
    t: &L10n,
    minutes: i64,
    choices: &[i64],
    mut picked: impl FnMut(i64),
) {
    let text = ui.visuals().text_color();
    ui.label(RichText::new(label).color(text));
    let mut chosen = minutes;
    let options = choices.iter().map(|m| (*m, t.renag_minutes(*m)));
    if radio_column(ui, &mut chosen, options) && chosen != minutes {
        picked(chosen);
    }
}

/// A single choice: one radio button per line, the `current` one marked.
/// True when a click picked one.
pub(crate) fn radio_column<T: PartialEq + Copy>(
    ui: &mut Ui,
    current: &mut T,
    options: impl IntoIterator<Item = (T, String)>,
) -> bool {
    let text = ui.visuals().text_color();
    let mut changed = false;
    for (value, label) in options {
        changed |= ui
            .radio_value(current, value, RichText::new(label).color(text))
            .clicked();
    }
    changed
}

/// A pill to pick one of a few choices: self-painted so its text reads in
/// every state, with the accent while `on`.
pub(crate) fn pill(ui: &mut Ui, tones: Tones, on: bool, label: &str) -> egui::Response {
    let font = FontId::proportional(14.0);
    let fg = ui.visuals().text_color();
    let galley = ui.painter().layout_no_wrap(label.to_string(), font, fg);
    let pad = egui::vec2(10.0, 5.0);
    let (rect, resp) = ui.allocate_exact_size(galley.size() + pad * 2.0, Sense::click());
    resp.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, on, label));
    if ui.is_rect_visible(rect) {
        let fill = if on {
            tones.accent.gamma_multiply(0.3)
        } else if resp.hovered() {
            tones.card.gamma_multiply(0.7)
        } else {
            Color32::TRANSPARENT
        };
        if fill.a() > 0 {
            ui.painter().rect_filled(rect, 12.0, fill);
        }
        ui.painter().rect_stroke(
            rect,
            12.0,
            egui::Stroke::new(
                1.0,
                if on {
                    tones.accent
                } else {
                    ui.visuals().weak_text_color()
                },
            ),
            egui::StrokeKind::Inside,
        );
        ui.painter().galley(rect.min + pad, galley, fg);
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// The box that checks an Occurrence off, called `label`.
pub(crate) fn check(ui: &mut Ui, tones: Tones, done: bool, label: &str) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), Sense::click());
    resp.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, true, done, label));
    if resp.hovered() {
        ui.painter()
            .rect_filled(rect, 6.0, ui.visuals().widgets.hovered.weak_bg_fill);
    }
    let (icon, colour) = if done {
        (icons::CHECKED, tones.accent)
    } else {
        (icons::UNCHECKED, ui.visuals().text_color())
    };
    icons::paint(ui.painter(), rect.shrink(2.0), icon, colour);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// What an Occurrence of a Routine shows as: its Note's text, what the
/// Note holds when it has no words, or that the Note is gone.
pub(crate) fn row_title(t: &L10n, note: Option<&Note>) -> String {
    match note {
        Some(note) if !note.text.trim().is_empty() => note.text.trim().to_string(),
        Some(note) => l10n::wordless(t, note.wordless()).to_string(),
        None => t.note_gone().to_string(),
    }
}

/// One Occurrence on the Agenda as an embossed row: the box to check it
/// off, its time, its Note's text, and its Area's colour and icon. A
/// click on the text opens its Routine. A Session's row lists its picks
/// under it today, and how many it picks on later days; once every pick
/// has an entry, its box stays checked.
fn agenda_row(ui: &mut Ui, t: &L10n, tones: Tones, row: &Row, actions: &mut Vec<Action>) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let routine = &row.row.routine;
    let occurrence = row.row.occurrence;
    let done = row.row.done().is_some();
    let colour = row
        .area
        .as_ref()
        .and_then(|a| area_colour(a.colour.as_deref()));
    let title = row_title(t, row.row.note.as_ref());
    let time = occurrence_time(t, &occurrence);
    embossed(ui, tones, false, colour, |ui| {
        // Done from the entries of every pick stays checked.
        let complete = matches!(&row.session, Some(SessionPart::Today(s)) if s.today.complete());
        ui.horizontal(|ui| {
            let label = format!("{}: {time} {title}", t.mark_done());
            let toggled = ui
                .add_enabled_ui(!complete, |ui| check(ui, tones, done, &label))
                .inner;
            if toggled.clicked() {
                actions.push(Action::Toggle(routine.id.clone(), occurrence));
            }
            ui.add_space(4.0);
            let width = (ui.available_width() - 56.0).max(40.0);
            let body = ui.vertical(|ui| {
                ui.set_max_width(width);
                ui.label(RichText::new(&time).strong().color(text));
                let mut line = RichText::new(&title).size(15.0).color(text);
                if done {
                    line = line.strikethrough();
                }
                ui.add(egui::Label::new(line).truncate());
            });
            let open = body
                .response
                .interact(Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            open.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &title));
            if open.clicked() {
                actions.push(Action::OpenRoutine(routine.id.clone()));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(icon) = row
                    .area
                    .as_ref()
                    .and_then(|a| icons::area_icon(a.icon.as_deref()))
                {
                    icons::show(ui, icon, 20.0, colour.unwrap_or(weak));
                }
                if routine.is_reminder() {
                    icons::show(ui, icons::REMINDER, 18.0, weak);
                }
            });
        });
        // The picks go under the title, outside its click.
        if let Some(part) = &row.session {
            let mut asked = Vec::new();
            ui.indent(("session part", &routine.id, occurrence.key()), |ui| {
                session_view::part(ui, t, part, &mut asked);
            });
            actions.extend(asked.into_iter().map(Action::Session));
        }
    });
}

/// How many Missed Occurrences of one Routine show to check off.
const MISSED_SHOWN: usize = 3;

/// The Missed Summary: each Routine that Missed something since "Seen",
/// how often, and its latest Missed Occurrences to check off late. It
/// stays until "Seen".
fn missed_card(
    ui: &mut Ui,
    t: &L10n,
    tones: Tones,
    missed: &MissedSummary,
    actions: &mut Vec<Action>,
) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let error = ui.visuals().error_fg_color;
    embossed(ui, tones, true, None, |ui| {
        ui.horizontal(|ui| {
            icons::show(ui, icons::HISTORY, 20.0, error);
            ui.label(
                RichText::new(t.missed_title())
                    .strong()
                    .size(16.0)
                    .color(text),
            );
            ui.add_space((ui.available_width() - 64.0).max(0.0));
            if ui
                .button(RichText::new(t.missed_seen()).color(text))
                .clicked()
            {
                actions.push(Action::Seen);
            }
        });
        for routine in &missed.routines {
            let what = row_title(t, routine.note.as_ref());
            let first = what.lines().next().unwrap_or_default();
            ui.label(
                RichText::new(format!(
                    "{first} · {}",
                    t.missed_times(routine.count() as i64)
                ))
                .color(text),
            );
            for m in routine.missed.iter().take(MISSED_SHOWN) {
                ui.horizontal(|ui| {
                    let when = format!(
                        "{}  {}",
                        day_text(t, m.occurrence.day),
                        occurrence_time(t, &m.occurrence)
                    );
                    let label = format!("{}: {when} {first}", t.mark_done());
                    if check(ui, tones, false, &label).clicked() {
                        actions.push(Action::CheckMissed(
                            routine.routine.id.clone(),
                            m.occurrence,
                        ));
                    }
                    ui.label(RichText::new(when).color(text));
                });
            }
            if routine.count() > MISSED_SHOWN {
                let more = t.missed_more((routine.count() - MISSED_SHOWN) as i64);
                ui.label(RichText::new(more).small().color(weak));
            }
        }
    });
}

/// An Area's stored colour (`RRGGBB`), or `None` when it has none or it
/// cannot be read.
pub fn area_colour(hex: Option<&str>) -> Option<Color32> {
    let value = u32::from_str_radix(hex?, 16).ok()?;
    let [_, r, g, b] = value.to_be_bytes();
    Some(Color32::from_rgb(r, g, b))
}

/// When something was captured, in the app's language and local time:
/// the phone's `yMd` and `Hm`.
pub fn when(t: &L10n, at: DateTime<Utc>) -> String {
    let local = at.with_timezone(&Local);
    match t.locale() {
        "de" => local.format("%-d.%-m.%Y %H:%M").to_string(),
        _ => local.format("%-m/%-d/%Y %H:%M").to_string(),
    }
}

/// A point as the phone shows it, `<latitude>, <longitude>` to four
/// decimals.
fn coordinates_text(at: Coordinates) -> String {
    format!("{:.4}, {:.4}", at.latitude(), at.longitude())
}

/// Where `context` was captured as a compact card shows it, the
/// phone's `whereCaptured`: its Named Place, else its coordinates;
/// `None` while unknown.
fn where_captured(context: &CaptureContext) -> Option<String> {
    match (&context.named_place, context.location) {
        (Some(place), _) => Some(place.name.clone()),
        (None, Some(at)) => Some(coordinates_text(at)),
        (None, None) => None,
    }
}

/// What a Capture Context says beyond the time, line by line with each
/// line's icon, as the phone's `CaptureContextLines`: where, with its
/// Named Place, the app it was shared in from, the calendar events
/// running and the Bluetooth devices connected.
fn context_lines(t: &L10n, context: &CaptureContext) -> Vec<(char, String)> {
    let mut lines = Vec::new();
    if let Some(at) = context.location {
        let at = coordinates_text(at);
        lines.push((
            icons::CONTEXT_WHERE,
            match &context.named_place {
                Some(place) => format!("{} ({at})", place.name),
                None => at,
            },
        ));
    }
    if let Some(app) = &context.source_app {
        lines.push((icons::CONTEXT_APP, t.context_source_app(app.name())));
    }
    if !context.calendar_events.is_empty() {
        let events = context.calendar_events.join(", ");
        lines.push((icons::CONTEXT_CALENDAR, t.context_during(&events)));
    }
    if !context.bluetooth_devices.is_empty() {
        let devices = context.bluetooth_devices.join(", ");
        lines.push((icons::CONTEXT_BLUETOOTH, t.context_bluetooth(&devices)));
    }
    lines
}

/// One line of a Capture Context: its icon, dimmed, and what it says,
/// wrapped where it is long.
fn context_line(ui: &mut Ui, icon: char, line: &str) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        icons::show(ui, icon, 16.0, weak);
        ui.add(egui::Label::new(RichText::new(line).size(13.0).color(text)).wrap());
    });
}

/// The whole Capture Context of a Note or Section `captured` then, line
/// by line, as the phone's `CaptureContextLines`: when, then
/// [`context_lines`].
fn capture_context(ui: &mut Ui, t: &L10n, captured: DateTime<Utc>, context: &CaptureContext) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 4.0;
        context_line(ui, icons::CONTEXT_WHEN, &when(t, captured));
        for (icon, line) in context_lines(t, context) {
            context_line(ui, icon, &line);
        }
    });
}

/// Where a Note is filed, `Area / Topic`; `None` while it is in no
/// place.
fn place_line(t: &L10n, placed: &Placed) -> Option<String> {
    let area = placed.area.as_ref()?;
    Some(match &placed.topic {
        Some(topic) => format!("{} / {}", area_name(t, area), topic_name(t, topic)),
        None => area_name(t, area),
    })
}

/// A row of the left pane: self-painted so its text reads in every
/// state, with the accent while selected. A `droppable` row is outlined
/// while a card is held over it.
fn row(
    ui: &mut Ui,
    tones: Tones,
    on: bool,
    indent: f32,
    icon: Option<(char, Color32)>,
    label: &str,
    droppable: bool,
) -> egui::Response {
    let font = egui::FontId::proportional(15.0);
    let height = 30.0;
    let width = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, height), Sense::click());
    resp.widget_info(|| WidgetInfo::selected(WidgetType::SelectableLabel, true, on, label));
    if ui.is_rect_visible(rect) {
        let fg = ui.visuals().text_color();
        if droppable && resp.dnd_hover_payload::<DraggedNote>().is_some() {
            ui.painter().rect_stroke(
                rect,
                8.0,
                egui::Stroke::new(2.0, tones.accent),
                egui::StrokeKind::Inside,
            );
        }
        let fill = if on {
            tones.accent.gamma_multiply(0.3)
        } else if resp.hovered() {
            tones.card.gamma_multiply(0.7)
        } else {
            Color32::TRANSPARENT
        };
        if fill.a() > 0 {
            ui.painter().rect_filled(rect, 8.0, fill);
        }
        let mut x = rect.left() + 8.0 + indent;
        if let Some((glyph, colour)) = icon {
            let icon_rect = egui::Rect::from_center_size(
                egui::pos2(x + 9.0, rect.center().y),
                egui::vec2(18.0, 18.0),
            );
            icons::paint(ui.painter(), icon_rect, glyph, colour);
            x += 26.0;
        }
        let text = ui.painter().layout(
            label.to_string(),
            font,
            fg,
            (rect.right() - x - 8.0).max(0.0),
        );
        let y = rect.center().y - text.size().y / 2.0;
        ui.painter().galley(egui::pos2(x, y), text, fg);
    }
    resp
}

/// Lays out `content` on a softly embossed card the width of `ui`;
/// `band` runs an Area's colour down its left edge.
pub(crate) fn embossed(
    ui: &mut Ui,
    tones: Tones,
    strong: bool,
    band: Option<Color32>,
    content: impl FnOnce(&mut Ui),
) -> egui::Response {
    let under = ui.painter().add(egui::Shape::Noop);
    let left = if band.is_some() { 22 } else { 16 };
    let inner = Frame::NONE
        .inner_margin(Margin {
            left,
            right: 16,
            top: 14,
            bottom: 14,
        })
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.style_mut().interaction.selectable_labels = false;
            content(ui);
        });
    let rect = inner.response.rect;
    let mut shapes = theme::embossed(rect, tones, strong);
    if let Some(colour) = band {
        shapes.push(theme::area_band(rect, colour));
    }
    ui.painter().set(under, egui::Shape::Vec(shapes));
    inner.response
}

/// The photos and voice recordings of the Note `id`, or of a Note merged
/// into it, each as `(shown, removed, texts)`.
struct Media<'a> {
    id: &'a str,
    photos: (&'a [String], &'a [String], &'a BTreeMap<String, String>),
    recordings: (&'a [String], &'a [String], &'a BTreeMap<String, String>),
}

/// What the editor may offer under the photos and recordings it shows
/// and their texts: the Transcripts are made again by `transcriber`,
/// the Photo Texts by `reader`, each one given only while it can.
struct Offering<'a> {
    view: &'a View,
    transcriber: Option<&'a Enricher>,
    reader: Option<&'a Enricher>,
}

impl Offering<'_> {
    /// What shows under the photo (`photo`) or recording `hash` of the
    /// Note or Section `id` (ADR 0020, ADR 0025).
    fn offers(&self, id: &str, hash: &str, photo: bool) -> Offers {
        let key = (id.to_string(), hash.to_string());
        let engine = if photo { self.reader } else { self.transcriber };
        Offers {
            take_over: self.view.text_changeable.contains(id),
            remove_text: self.view.text_removable.contains(&key),
            again: engine
                .filter(|_| self.view.remakeable.contains(&key))
                .map(|e| e.busy_with(hash)),
            remove_file: self.view.removable.contains(&key),
        }
    }
}

/// What the editor shows of `media`: the photos with their Photo Texts,
/// the recordings with their play buttons and playback `speed` while
/// `can_play`, the texts of those removed where the files were
/// (ADR 0020), and under each text and file the buttons `offering`
/// gives (ADR 0025).
fn editor_media(
    ui: &mut Ui,
    t: &L10n,
    media: Media,
    (cache, store, offering): (&mut PhotoCache, &Store, &Offering),
    (playing, transcribing, can_play, speed): (Option<&str>, Option<&str>, bool, f64),
    actions: &mut Vec<Action>,
) {
    let id = media.id;
    let (photos, removed_photos, photo_texts) = media.photos;
    let (recordings, removed_recordings, transcripts) = media.recordings;
    let ask = |hash: String, photo: bool| Action::AskRemoveMedia {
        note: id.to_string(),
        hash,
        photo,
    };
    let text = |asked: Generated, photo: bool| {
        let note = id.to_string();
        match asked {
            Generated::TakeOver { hash, append } => Action::TakeOver {
                note,
                hash,
                photo,
                append,
            },
            Generated::Remove(hash) => Action::AskRemoveText { note, hash, photo },
            Generated::Again(hash) => Action::Again { note, hash, photo },
        }
    };
    let photo_offers = |hash: &str| offering.offers(id, hash, true);
    match media_view::photos(ui, t, (cache, store), photos, photo_texts, &photo_offers) {
        Some(Photo::View(hash)) => actions.push(Action::ViewPhoto(hash)),
        Some(Photo::Remove(hash)) => actions.push(ask(hash, true)),
        Some(Photo::Text(asked)) => actions.push(text(asked, true)),
        None => {}
    }
    let asked = media_view::recordings(
        ui,
        t,
        recordings,
        transcripts,
        (playing, transcribing, speed),
        can_play,
        &|hash| offering.offers(id, hash, false),
    );
    match asked {
        Some(Recording::Play(hash)) => actions.push(Action::Play(hash)),
        Some(Recording::Remove(hash)) => actions.push(ask(hash, false)),
        Some(Recording::Text(asked)) => actions.push(text(asked, false)),
        Some(Recording::Speed(speed)) => actions.push(Action::Speed(speed)),
        None => {}
    }
    if let Some((photo, asked)) = media_view::removed_texts(
        ui,
        t,
        (removed_photos, photo_texts),
        (removed_recordings, transcripts),
        &|hash, photo| offering.offers(id, hash, photo),
    ) {
        actions.push(text(asked, photo));
    }
}

/// What a card needs to show its photos and recordings.
struct CardMedia<'a> {
    photos: &'a mut PhotoCache,
    pictures: &'a mut profile_view::Pictures,
    store: &'a Store,
    /// The recording being transcribed, by its hash.
    transcribing: Option<&'a str>,
}

/// How a card shows its Note (#114): `full`, the whole Note with its
/// `sections`, or compact, by its first line.
#[derive(Debug, Clone, Copy)]
struct CardLook<'a> {
    full: bool,
    sections: &'a [Section],
}

/// What can be clicked on a card: the card, the author's picture on
/// another person's Note, Edit, and the photo of a full card clicked, by
/// hash.
struct CardClicks {
    card: egui::Response,
    author: Option<egui::Response>,
    edit: egui::Response,
    photo: Option<String>,
}

/// The bar button of a Note list's full/compact switch (#114): it says
/// what clicking it does and shows as selected while the list is
/// `compact`; whether it was clicked.
fn compact_button(ui: &mut Ui, tones: Tones, t: &L10n, compact: bool) -> bool {
    let (icon, label) = if compact {
        (icons::NOTES_COMPACT, t.show_notes_full())
    } else {
        (icons::NOTES_FULL, t.show_notes_compact())
    };
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), Sense::click());
    resp.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, compact, label));
    let fill = if compact {
        tones.accent.gamma_multiply(0.3)
    } else if resp.hovered() {
        ui.visuals().widgets.hovered.weak_bg_fill
    } else {
        Color32::TRANSPARENT
    };
    if fill.a() > 0 {
        ui.painter().rect_filled(rect, 6.0, fill);
    }
    let text = ui.visuals().text_color();
    icons::paint(ui.painter(), rect.shrink(2.0), icon, text);
    resp.on_hover_text(label).clicked()
}

/// The text of a Note or Section and its photos and voice recordings,
/// each as `(shown, removed, texts)`.
struct Part<'a> {
    text: &'a str,
    photos: (&'a [String], &'a [String], &'a BTreeMap<String, String>),
    recordings: (&'a [String], &'a [String], &'a BTreeMap<String, String>),
}

impl<'a> Part<'a> {
    fn of_note(note: &'a Note) -> Part<'a> {
        Part {
            text: &note.text,
            photos: (&note.photos, &note.removed_photos, &note.photo_texts),
            recordings: (
                &note.recordings,
                &note.removed_recordings,
                &note.transcripts,
            ),
        }
    }

    fn of_section(section: &'a Section) -> Part<'a> {
        Part {
            text: &section.text,
            photos: (
                &section.photos,
                &section.removed_photos,
                &section.photo_texts,
            ),
            recordings: (
                &section.recordings,
                &section.removed_recordings,
                &section.transcripts,
            ),
        }
    }
}

/// The text, photos and recordings of a Note or Section in full on a
/// card (#114), with their Transcripts and Photo Texts; the photos shown
/// with where they lie.
fn whole(
    ui: &mut Ui,
    t: &L10n,
    part: Part,
    (cache, store, transcribing): ShownMedia,
) -> Vec<(String, egui::Rect)> {
    if !part.text.is_empty() {
        let text = ui.visuals().text_color();
        ui.label(RichText::new(part.text).size(16.0).color(text));
    }
    media_view::whole_media(
        ui,
        t,
        (cache, store),
        part.photos,
        part.recordings,
        transcribing,
    )
}

/// The photo cache, the store the photos are read from and the
/// recording being transcribed, for [`whole`].
type ShownMedia<'a, 'b> = (&'b mut PhotoCache, &'a Store, Option<&'a str>);

/// A Note as a card (UI laws, "Cards"): when it was captured, with Edit,
/// and where it is filed, its Area's colour down the left edge and icon
/// before the place; then, as `look` says, the whole Note under its
/// Capture Context with every Section oldest first, each under its own,
/// its photos opening full screen on a click; or, compact, where it was
/// captured, its first line and its media. Important cards carry the
/// accent marker and a stronger emboss; unimportant ones are dimmed. A
/// `draggable` card can be dropped onto an Area or Topic; a `selected`
/// one is outlined while its editor shows.
fn note_card(
    ui: &mut Ui,
    t: &L10n,
    tones: Tones,
    placed: &Placed,
    (selected, draggable): (bool, bool),
    look: CardLook,
    media: CardMedia,
) -> CardClicks {
    let note = &placed.note;
    let important = note.importance == Importance::Important;
    let unimportant = note.importance == Importance::Unimportant;
    let colour = placed
        .area
        .as_ref()
        .and_then(|a| area_colour(a.colour.as_deref()));
    let mut author_rect = None;
    let mut edit_rect = egui::Rect::NOTHING;
    // The photos shown in full, where they lie.
    let mut shown = Vec::new();
    let CardMedia {
        photos,
        pictures,
        store,
        transcribing,
    } = media;
    let inner = ui.scope(|ui| {
        if unimportant {
            ui.multiply_opacity(theme::UNIMPORTANT_OPACITY);
        }
        let text = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        let card = embossed(ui, tones, important, colour, |ui| {
            if important {
                // Room for the marker on the right.
                ui.set_max_width(ui.available_width() - 36.0);
            }
            // Another person's Note shows their picture (ADR 0017); the
            // user's own none.
            ui.horizontal(|ui| {
                if let Some(author) = &placed.author {
                    let name = profile_view::display_name(t, &author.name);
                    let picture = profile_view::person_picture(
                        ui,
                        pictures,
                        author,
                        20.0,
                        &t.author_picture(name),
                    );
                    author_rect = Some(picture.rect);
                }
                if look.full {
                    context_line(ui, icons::CONTEXT_WHEN, &when(t, note.captured));
                } else {
                    ui.label(RichText::new(when(t, note.captured)).small().color(weak));
                    // Where, and that the editor shows more (UI laws,
                    // "Cards"); room is left for Edit.
                    let room = (ui.available_width() - 56.0).max(0.0);
                    if let Some(at) = where_captured(&note.context) {
                        icons::show(ui, icons::CONTEXT_WHERE, 14.0, weak);
                        ui.allocate_ui(egui::vec2(room, 20.0), |ui| {
                            let at = RichText::new(at).small().color(weak);
                            ui.add(egui::Label::new(at).truncate());
                        });
                    }
                    if note.context.has_more() {
                        icons::show(ui, icons::MORE, 16.0, weak);
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    edit_rect = ui
                        .allocate_exact_size(egui::vec2(24.0, 24.0), Sense::hover())
                        .0;
                });
            });
            if look.full {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    for (icon, line) in context_lines(t, &note.context) {
                        context_line(ui, icon, &line);
                    }
                });
            }
            // A Note that spawned a Routine is not Unsorted, though not
            // filed.
            let place = match place_line(t, placed) {
                Some(line) => Some(line),
                None if note.spawned => None,
                None => Some(t.unsorted().to_string()),
            };
            if let Some(place) = place {
                ui.horizontal(|ui| {
                    if let Some(icon) = placed
                        .area
                        .as_ref()
                        .and_then(|a| icons::area_icon(a.icon.as_deref()))
                    {
                        icons::show(ui, icon, 16.0, colour.unwrap_or(weak));
                    }
                    ui.label(RichText::new(place).size(13.0).color(text));
                });
            }
            ui.separator();
            if look.full {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 8.0;
                    let part = Part::of_note(note);
                    shown.extend(whole(ui, t, part, (&mut *photos, store, transcribing)));
                    for section in look.sections {
                        ui.separator();
                        capture_context(ui, t, section.captured, &section.context);
                        let part = Part::of_section(section);
                        shown.extend(whole(ui, t, part, (&mut *photos, store, transcribing)));
                    }
                });
                return;
            }
            // A photo Note may have no words.
            if let Some(first) = note.text.trim().lines().next() {
                ui.add(egui::Label::new(RichText::new(first).size(16.0).color(text)).truncate());
            }
            if !note.photos.is_empty() {
                media_view::thumbnails(ui, t, photos, store, &note.photos, 64.0, 3);
            }
            media_view::recordings(
                ui,
                t,
                &note.recordings,
                &note.transcripts,
                (None, transcribing, 1.0),
                false,
                &|_| Offers::default(),
            );
            media_view::removed_texts(
                ui,
                t,
                (&note.removed_photos, &note.photo_texts),
                (&note.removed_recordings, &note.transcripts),
                &|_, _| Offers::default(),
            );
        });
        let rect = card.rect;
        if important {
            let marker = egui::Rect::from_center_size(
                egui::pos2(rect.right() - 28.0, rect.top() + 28.0),
                egui::vec2(24.0, 24.0),
            );
            icons::paint(ui.painter(), marker, icons::IMPORTANT, tones.accent);
        }
        if selected {
            ui.painter().rect_stroke(
                rect,
                theme::CARD_RADIUS,
                egui::Stroke::new(2.0, tones.accent),
                egui::StrokeKind::Outside,
            );
        }
        rect
    });
    let rect = inner.inner;
    let sense = if draggable {
        Sense::click_and_drag()
    } else {
        Sense::click()
    };
    let resp = ui.interact(rect, Id::new(("card", &note.id)), sense);
    resp.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, selected, &note.text));
    if draggable {
        resp.dnd_set_drag_payload(DraggedNote(note.id.clone()));
    }
    // The author's picture and Edit lie over the card: registered after
    // it, so they take the click.
    let author = author_rect
        .zip(placed.author.as_ref())
        .map(|(rect, author)| {
            let label = t.person_open(profile_view::display_name(t, &author.name));
            let picture = ui.interact(rect, Id::new(("author", &note.id)), Sense::click());
            picture.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &label));
            picture
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .on_hover_text(label)
        });
    let edit = ui.interact(edit_rect, Id::new(("edit", &note.id)), Sense::click());
    edit.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, t.edit()));
    if ui.is_rect_visible(edit_rect) {
        let dim = |c: Color32| {
            if unimportant {
                c.gamma_multiply(theme::UNIMPORTANT_OPACITY)
            } else {
                c
            }
        };
        if edit.hovered() {
            let fill = dim(ui.visuals().widgets.hovered.weak_bg_fill);
            ui.painter().rect_filled(edit_rect, 6.0, fill);
        }
        let fg = dim(ui.visuals().text_color());
        icons::paint(ui.painter(), edit_rect.shrink(2.0), icons::EDIT, fg);
    }
    // A click on a photo opens it full screen (UI laws, "Cards").
    let mut photo = None;
    for (i, (hash, rect)) in shown.into_iter().enumerate() {
        let id = Id::new(("photo", &note.id, i));
        if media_view::photo_click(ui, t, rect, id).clicked() {
            photo = Some(hash);
        }
    }
    CardClicks {
        card: resp.on_hover_cursor(egui::CursorIcon::PointingHand),
        author,
        edit: edit.on_hover_text(t.edit()),
        photo,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn area_colours_read_as_the_phone_reads_them() {
        assert_eq!(
            area_colour(Some("6A1B9A")),
            Some(Color32::from_rgb(0x6A, 0x1B, 0x9A))
        );
        assert_eq!(area_colour(Some("nope")), None);
        assert_eq!(area_colour(None), None);
    }

    #[test]
    fn a_bundle_for_a_person_is_named_after_them() {
        let Some(day) = chrono::NaiveDate::from_ymd_opt(2026, 3, 4) else {
            panic!("a valid date")
        };
        assert_eq!(
            changes_file_name(day, "profile:1", "Ann-Marie Öz!"),
            "oldbrainfix-2026-03-04-Ann-Marie-Öz.obfsync"
        );
        assert_eq!(
            changes_file_name(day, "profile:1", " "),
            "oldbrainfix-2026-03-04-profile-1.obfsync"
        );
    }

    #[test]
    fn a_report_mails_the_details_percent_encoded_and_names_no_file() {
        let url = report_url("unreadable: .ogg\nunsupported");
        assert!(
            url.starts_with(
                "mailto:taum@tuta.io?subject=Old%20Brain%20Fix%20problem%20report&body="
            )
        );
        assert!(
            url.ends_with("%0A%0Aunreadable%3A%20.ogg%0Aunsupported"),
            "{url}"
        );
        assert_eq!(percent_encoded("ä &"), "%C3%A4%20%26");
        let line = problem_line(oldbrainfix_whisper::Problem::Unsupported, "Dr. Ben.OGG");
        assert_eq!(line, "unsupported: .OGG");
        let line = problem_line(oldbrainfix_whisper::Problem::Unreadable, "voice");
        assert_eq!(line, "unreadable");
    }

    #[test]
    fn capture_times_read_in_the_apps_language() {
        let at = Utc.with_ymd_and_hms(2026, 3, 4, 12, 0, 0).single();
        let Some(at) = at else { panic!("a valid date") };
        let local = at.with_timezone(&Local);
        let en = when(&L10n::new("en"), at);
        let de = when(&L10n::new("de"), at);
        assert!(
            en.starts_with(&local.format("%-m/%-d/%Y").to_string()),
            "{en}"
        );
        assert!(
            de.starts_with(&local.format("%-d.%-m.%Y").to_string()),
            "{de}"
        );
    }
}
