//! The window (UI laws, "Desktop"): the Agenda, Areas and Topics on the
//! left, the capture field and the Agenda or the Notes of the selection
//! in the middle as embossed cards, the selected Note's editor or a
//! Routine's editor on the right.
//!
//! The window opens on the Agenda (UI laws, "Agenda"): the Missed
//! Summary until "Seen", the "x Unsorted" banner, today's all-day and
//! then timed Occurrences, a gap, and tomorrow's dimmed, all of them
//! checkable. Its gear leads to this Device's Settings, its alarm button
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
//! at once and one big stop button ends it and keeps the Note, as do the
//! Nudge times (UI laws, "Capture"). A picture pasted from the clipboard
//! or dropped onto the window (where the system hands drops over: not on
//! Wayland) becomes a photo Note, with what is typed; a sync bundle
//! dropped there is imported, what came in announced at the top; a
//! Sent Item dropped there, opened or handed over by `oldbrainfix open`
//! shows its preview to accept or decline. A Note is sent from its
//! editor, saved where the user says.
//! The editor shows a Note's photos and plays its recordings, with their
//! Transcripts. Transcripts and Photo Texts are made in the background
//! and found by the search over All Notes, an Area or a Topic.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashSet};
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
use oldbrainfix_core::backup;
use oldbrainfix_core::profile::ProfileChoice;
use oldbrainfix_core::sent::{self, OpenError, Opened, Recipient, SenderTrust, SentItemState};
use oldbrainfix_core::{
    AREA_ICONS, AgendaRow, Alarms, Area, AreaAccess, AreaRole, ClockTime, Day, DeviceSettings,
    Importance, Invite, MissedSummary, Note, Notes, NotesGo, Occurrence, OwnProfile, Person,
    PersonFilter, Places, Profile, QuietHours, Routine, Routines, Search, Section, Store, Topic,
    bundle,
};

use crate::about_view;
use crate::alerts::Event;
use crate::autostart::Autostart;
use crate::background;
use crate::backup_view::{self, BackupAction, Passphrase, Welcome, WelcomeAction};
use crate::backups;
use crate::enrich::Enricher;
use crate::folder_sync;
use crate::hotkey::{self, Chord};
use crate::icons;
use crate::l10n::L10n;
use crate::media::{self, Clipboard, Player, Recorder, photo_from_file};
use crate::media_view::{self, PhotoCache, Recording, Speech, Voice};
use crate::names::{area_name, topic_name};
use crate::person_view::{self, Comparing, PersonAction, PersonView};
use crate::profile_view::{self, ProfileAction, ProfileView, Square};
use crate::routine_editor::{Intent, RENAG_CHOICES, RoutineEditor};
use crate::schedule_text::{day_text, occurrence_time, schedule_text};
use crate::speech::{SpeechModel, Status as SpeechStatus};
use crate::sync_view::{SyncAction, SyncHost, device_name, import_bundle_text, update_notice_text};
use crate::theme::{self, AREA_PALETTE, Tones};

/// What the middle pane shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection {
    /// Today's and tomorrow's Occurrences, the Missed Summary and the
    /// Unsorted count.
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
    /// Every Note neither in the trash nor merged.
    All,
    Unsorted,
    /// The Area or Topic with this id; an Area holds its Topics' Notes.
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
}

/// What the Agenda shows.
#[derive(Debug, Clone, Default)]
struct AgendaView {
    today: Vec<Row>,
    tomorrow: Vec<Row>,
    missed: MissedSummary,
    unsorted: usize,
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
    /// People (#52): every Profile kept here, as the user sees them.
    people_shown: Vec<Profile>,
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
    notes: Vec<Placed>,
    /// The selected Note with its Sections.
    detail: Option<(Placed, Vec<Section>)>,
    /// The Routines and Reminders the selected Note spawned.
    routines: Vec<Routine>,
    /// While the Agenda shows.
    agenda: Option<AgendaView>,
    /// The Routines and Reminders in the trash on their own, with their
    /// Notes, while the trash shows.
    trashed_routines: Vec<(Routine, Option<Note>)>,
    /// The user's own Profile and its short code.
    profile: Option<Profile>,
    profile_code: Option<String>,
    /// The other Profile "Which one is you?" asks about, if it does.
    profile_question: Option<ProfileChoice>,
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
    /// Whether the Routines and Reminders of the Note being deleted go
    /// to the trash too.
    TrashNote {
        question: String,
    },
    /// Whether the edited Routine or Reminder goes to the trash.
    DeleteRoutine {
        question: String,
    },
    /// Sharing the Area `area`: the role the other person gets.
    Share {
        area: String,
        role: AreaRole,
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
    /// sender's short code came to.
    SentItem {
        opened: Box<Opened>,
        compared: Compared,
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
    /// "Which one is you?" (ADR 0017): two Profiles of the user's own
    /// Devices, both changed by the user, met.
    WhichProfile,
    /// Another person's Profile (#52), with where comparing their code
    /// stands and the name for the user being typed.
    Person {
        id: String,
        comparing: Comparing,
        name: String,
    },
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
    SelectNote(String),
    /// Captures the capture field's text and the recording running,
    /// nudged this long from now.
    Capture(Duration),
    /// Starts recording voice into the Note being captured.
    Record,
    /// Plays the voice recording with this hash, or stops it playing.
    Play(String),
    /// Plays recordings at this speed from now on, the one playing
    /// included, and keeps it on this Device.
    Speed(f64),
    /// Keeps the picture on the clipboard as a photo Note.
    PastePhoto,
    /// Keeps these photos, already made the phone's way, as a photo
    /// Note.
    Photos(Vec<Vec<u8>>),
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
    Editor(Intent),
    /// Deletes the edited Routine, asked.
    DeleteRoutine,
    /// Deletes the selected Note, with or without its Routines, asked.
    TrashNote(bool),
    RestoreRoutine(String),
    Setting(Setting),
    Sync(SyncAction),
    /// Imports the bundle file dropped onto the window or handed over,
    /// or shows the preview of the Sent Item.
    OpenFile(std::path::PathBuf),
    /// Asks whom the selected Note is sent to.
    StartSend,
    /// Saves the Note of the send dialog as a Sent Item.
    SendNote,
    /// Accepts the Sent Item previewed, or declines it.
    AcceptSent,
    DeclineSent,
    /// Asks to compare the sender's short code in person, and what that
    /// came to: whether the codes match.
    CompareCodes,
    CodesCompared(bool),
    /// Asks how the Area is to be shared.
    Share(String),
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
    trail: Trail,
    /// The capture field's text.
    capture: String,
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
    /// A recording that ended but could not be kept yet.
    recorded: Option<Vec<u8>>,
    clipboard: Option<Box<dyn Clipboard>>,
    /// The background threads making Transcripts and Photo Texts.
    transcriber: Option<Enricher>,
    reader: Option<Enricher>,
    /// The speech model the Transcripts need, and whether it was ready
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
    /// The picture of the other Profile "Which one is you?" shows.
    question_view: ProfileView,
    /// The square of a new picture for the person the dialog shows.
    person_square: Option<Square>,
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
}

/// Asks for a folder, or a file to open, with the dialog's title.
pub type FolderPicker = Box<dyn Fn(&str) -> Option<std::path::PathBuf>>;

/// Asks where to save a file, with the dialog's title and the name it
/// proposes.
pub type FileSaver = Box<dyn Fn(&str, &str) -> Option<std::path::PathBuf>>;

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

impl App {
    /// The window over `store`, speaking `locale`, showing the Agenda on
    /// the system's local time.
    pub fn new(store: impl Into<Arc<Store>>, locale: &str) -> App {
        let store = store.into();
        let read_at = store.now();
        let speed = media::playback_speed(&store);
        let mut app = App {
            store,
            wall: Arc::new(local_wall),
            t: L10n::new(locale),
            selection: Selection::Agenda,
            selected_note: None,
            view: View::default(),
            error: None,
            scrolled: 0.0,
            trail: Trail::default(),
            capture: String::new(),
            text: String::new(),
            text_of: None,
            amend: String::new(),
            dialog: None,
            focus: false,
            editor: None,
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
            recorded: None,
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
            bundle_saver: Box::new(|title, name| {
                bundle_dialog(title).set_file_name(name).save_file()
            }),
            bundle_opener: Box::new(|title| bundle_dialog(title).pick_file()),
            item_saver: Box::new(|title, name| item_dialog(title).set_file_name(name).save_file()),
            handed: None,
            notice: None,
            updates: std::collections::VecDeque::new(),
            profile_view: ProfileView::default(),
            question_view: ProfileView::default(),
            person_square: None,
            dialog_behind: None,
            question_later: false,
            picture_picker: Box::new(|title| picture_dialog(title).pick_file()),
            backup_form: Passphrase::default(),
            welcome: None,
            sheet_saver: Box::new(|title, name| {
                sheet_dialog(title).set_file_name(name).save_file()
            }),
            backup_opener: Box::new(|title| backup_dialog(title).pick_file()),
        };
        app.hotkey_text = DeviceSettings::new(&app.store)
            .capture_hotkey()
            .unwrap_or_else(|_| oldbrainfix_core::settings::DEFAULT_CAPTURE_HOTKEY.into());
        app.refresh();
        app
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
            WelcomeAction::Fresh => {
                backup_view::restore_answered(&self.store)?;
                self.welcome = None;
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
                        backup_view::restore_answered(&self.store)?;
                        self.welcome = None;
                        self.notice = Some(t.restore_done(restored.entries as i64));
                    }
                    Err(e) => self.notice = Some(backup_view::restore_failure(&t, &e)),
                }
            }
        }
        Ok(())
    }

    /// Hands in the dialog asking where to save a Sent Item.
    pub fn set_item_saver(&mut self, saver: FileSaver) {
        self.item_saver = saver;
    }

    /// Opens the files put into `handed` from outside the window, as
    /// `oldbrainfix open` does, at the next frame.
    pub fn set_handed_files(&mut self, handed: Arc<std::sync::Mutex<Vec<std::path::PathBuf>>>) {
        self.handed = Some(handed);
    }

    /// Whether a Sent Item's preview shows.
    pub fn previewing(&self) -> bool {
        matches!(self.dialog, Some(Dialog::SentItem { .. }))
    }

    /// What a bundle saved or imported did, as announced.
    pub fn notice(&self) -> Option<&str> {
        self.notice.as_deref()
    }

    /// Saves a bundle for the paired Device `device`, or of everything,
    /// where the user says; nothing asked when there is nothing new.
    fn save_bundle(&mut self, device: Option<String>) -> oldbrainfix_core::Result<()> {
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
                Err(e) => format!("{} ({e})", self.t.bundle_failed()),
            },
        );
        Ok(())
    }

    /// Opens the file at `path`: a Sent Item shows its preview, anything
    /// else is imported as a bundle.
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
        match self.store.open_sent_item(&bytes) {
            Ok(Ok(opened)) => {
                self.dialog = Some(Dialog::SentItem {
                    opened: Box::new(opened),
                    compared: Compared::Not,
                });
            }
            Ok(Err(OpenError::NotForUs)) => {
                self.notice = Some(self.t.sent_item_not_for_us().to_string());
            }
            Ok(Err(OpenError::NotASentItem)) => {
                self.notice = Some(self.t.sent_item_unreadable().to_string());
            }
            Ok(Err(OpenError::NotFromSender)) => {
                self.notice = Some(self.t.sent_item_not_from_sender().to_string());
            }
            Err(e) => self.notice = Some(format!("{} ({e})", self.t.sent_item_unreadable())),
        }
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

    /// Imports the bundle at `path` and announces what came in.
    fn import_bundle(&mut self, path: &std::path::Path) {
        let said = import_bundle_text(&self.t, &self.store, path);
        self.notice = Some(said.unwrap_or_else(|e| e));
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
    /// Transcripts only once the speech model is ready.
    fn kick(&mut self) {
        if let Some(reader) = &self.reader {
            reader.kick();
        }
        let ready = self
            .speech
            .as_ref()
            .is_none_or(|s| s.status() == SpeechStatus::Ready);
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
        if !self.capture.trim().is_empty() || self.recording() || self.recorded.is_some() {
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
        let shown = self.view.detail.as_ref().map(|(p, _)| &p.note);
        if shown.map(|n| &n.id) != self.text_of.as_ref() {
            self.text = shown.map(|n| n.text.clone()).unwrap_or_default();
            self.text_of = shown.map(|n| n.id.clone());
            self.amend.clear();
        }
        self.kick();
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
            | Selection::Licences => Vec::new(),
            Selection::All => self
                .store
                .notes_by(search.notes(&self.query, None)?, &self.people)?,
            Selection::Unsorted => self.store.notes_by(notes.unsorted()?, &self.people)?,
            Selection::Place(id) => self
                .store
                .notes_by(search.notes(&self.query, Some(id))?, &self.people)?,
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
        let note_routines = match &detail {
            Some((p, _)) if !p.note.is_trashed() => routines.of_note(&p.note.id)?,
            _ => Vec::new(),
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
        if let Some((p, sections)) = &detail
            && !p.note.is_trashed()
        {
            let note = (&p.note.id, &p.note.photos, &p.note.recordings);
            let merged = sections.iter().map(|s| (&s.id, &s.photos, &s.recordings));
            for (id, photos, recordings) in std::iter::once(note).chain(merged) {
                for hash in photos {
                    if notes.may_remove_photo(id, hash)? {
                        removable.insert((id.clone(), hash.clone()));
                    }
                }
                for hash in recordings {
                    if notes.may_remove_recording(id, hash)? {
                        removable.insert((id.clone(), hash.clone()));
                    }
                }
            }
        }
        let agenda = match self.selection {
            Selection::Agenda => {
                let wall = &*self.wall;
                let agenda = routines.agenda(wall)?;
                let row = |row: AgendaRow| -> oldbrainfix_core::Result<Row> {
                    let area = match &row.note {
                        Some(note) => place(note.clone())?.area,
                        None => None,
                    };
                    Ok(Row { row, area })
                };
                Some(AgendaView {
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
                })
            }
            _ => None,
        };
        let mut trashed_routines = Vec::new();
        if self.selection == Selection::Trash {
            for routine in routines.trashed()? {
                let note = routines.note(&routine)?;
                trashed_routines.push((routine, note));
            }
        }
        Ok(View {
            areas,
            access,
            people,
            names,
            shown: shown_people,
            people_shown: self.store.people_shown()?,
            person: match &self.dialog {
                Some(Dialog::Person { id, .. }) => Some(PersonView::read(&self.store, id)?),
                _ => None,
            },
            mine,
            may,
            removable,
            notes: placed,
            detail,
            routines: note_routines,
            agenda,
            trashed_routines,
            profile: OwnProfile::new(&self.store).get()?,
            profile_code: OwnProfile::new(&self.store).code()?,
            profile_question: OwnProfile::new(&self.store).question()?,
        })
    }

    /// Does `action`, reads the log again and announces what went wrong.
    fn apply(&mut self, action: Action) {
        let done = self.act(action);
        // The session runs exactly while the Sync view shows.
        if (self.selection == Selection::Sync) != self.sync.hosting() {
            if self.sync.hosting() {
                self.sync.stop();
            } else {
                self.sync.start(&self.store);
            }
        }

        self.refresh();
        if let Err(e) = done {
            self.error = Some(e.to_string());
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
            }
            Action::Capture(nudge_in) => {
                // A running recording is ended and kept first; held
                // until it is, so a failed save loses nothing.
                if let Some(r) = &mut self.recorder
                    && r.recording()
                    && let Some(bytes) = r.stop()
                {
                    self.recorded = Some(bytes);
                }
                let recording = self.recorded.clone();
                self.capture_note(nudge_in, &[], recording.as_slice())?;
                self.recorded = None;
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
                media::keep_playback_speed(&self.store, speed)?;
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
            Action::Photos(photos) => {
                self.capture_note(Notes::default_nudge(), &photos, &[])?;
            }
            Action::Search => {}
            Action::FilterPeople(people) => self.people = people,
            Action::Speech(asked) => {
                if let Some(speech) = &self.speech {
                    match asked {
                        Speech::Download => speech.download(),
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
                let linked = Routines::new(&self.store).of_note(&selected)?;
                if linked.is_empty() {
                    Routines::new(&self.store).trash_note(&selected, false)?;
                } else {
                    let count = linked.len() as i64;
                    let reminders = linked.iter().filter(|r| r.is_reminder()).count() as i64;
                    let question = if reminders == count {
                        self.t.delete_note_reminders(count)
                    } else if reminders == 0 {
                        self.t.delete_note_routines(count)
                    } else {
                        self.t.delete_note_routines_and_reminders(count)
                    };
                    self.dialog = Some(Dialog::TrashNote { question });
                }
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
                    self.editor = Some(RoutineEditor::edit(&routine, note, today, renag));
                }
            }
            Action::NewRoutine { note, reminder } => {
                self.save_text()?;
                let note = match note {
                    Some(id) => Notes::new(&self.store).by_id(&id)?,
                    None => None,
                };
                let renag = DeviceSettings::new(&self.store).renag_interval()?;
                self.editor = Some(RoutineEditor::new(
                    note,
                    reminder,
                    self.store.now(),
                    &*self.wall,
                    renag,
                ));
            }
            Action::Editor(intent) => match intent {
                Intent::Save => {
                    if let Some(editor) = &self.editor {
                        editor.save(&self.store)?;
                        self.editor = None;
                    }
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
            Action::Setting(setting) => self.set(setting)?,
            Action::Sync(SyncAction::Trust(trust)) => {
                if self.sync.answer(&self.store, trust, &self.sync_name) {
                    // A Moment and the new entries: read again below.
                    self.refresh();
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
            Action::Backup(asked) => self.backup(asked)?,
            Action::Profile(asked) => self.profile_action(asked)?,
            Action::OpenPerson(id) => self.open_person(id)?,
            Action::Person(asked) => self.person_action(asked)?,
            Action::Welcome(asked) => self.welcome_action(asked)?,
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
            Action::AcceptSent => {
                if let Some(Dialog::SentItem { opened, compared }) = self.dialog.take() {
                    // The alerts plan its alarms once the action is done.
                    self.store
                        .accept_sent_item(&opened.item, compared == Compared::Match)?;
                    let said = match opened.state {
                        SentItemState::Update => self.t.sent_item_applied(),
                        SentItemState::Cancel => self.t.sent_item_trashed(),
                        _ => self.t.sent_item_accepted(),
                    };
                    self.notice = Some(said.to_string());
                }
            }
            Action::DeclineSent => {
                let previewed = matches!(&self.dialog, Some(Dialog::SentItem { opened, .. }) if acceptable(opened.state));
                self.dialog = None;
                if previewed {
                    self.notice = Some(self.t.sent_item_declined().to_string());
                }
            }
            Action::CompareCodes => {
                if let Some(Dialog::SentItem { compared, .. }) = &mut self.dialog {
                    *compared = Compared::Asking;
                }
            }
            Action::CodesCompared(matched) => {
                if let Some(Dialog::SentItem { compared, .. }) = &mut self.dialog {
                    *compared = if matched {
                        Compared::Match
                    } else {
                        Compared::Differ
                    };
                }
            }
            Action::Share(area) => {
                self.dialog = Some(Dialog::Share {
                    area,
                    role: AreaRole::Member,
                });
            }
            Action::StartInvite => {
                if let Some(Dialog::Share { area, role }) = self.dialog.clone() {
                    let area = self.store.share(&area)?;
                    self.dialog = None;
                    self.save_text()?;
                    self.query.clear();
                    self.selection = Selection::Sync;
                    self.sync.start_inviting(&self.store, Invite { area, role });
                }
            }
            Action::Sync(SyncAction::StopInviting) => self.sync.stop_inviting(&self.store),
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
    fn capture_note(
        &mut self,
        nudge_in: Duration,
        photos: &[Vec<u8>],
        recordings: &[Vec<u8>],
    ) -> oldbrainfix_core::Result<()> {
        let text = self.capture.trim();
        if text.is_empty() && photos.is_empty() && recordings.is_empty() {
            return Ok(());
        }
        // Captured inside an Area or Topic, the Note is filed there, as
        // the phone's "Note here".
        let place = match &self.selection {
            Selection::Place(id) => Some(id.as_str()),
            _ => None,
        };
        let note = Notes::new(&self.store).capture_with(
            text,
            place,
            Some(nudge_in),
            photos,
            recordings,
        )?;
        self.capture.clear();
        self.save_text()?;
        self.selected_note = Some(note.id);
        self.editor = None;
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
            Selection::Profile => Some(Selection::Agenda),
            Selection::Settings | Selection::Sync => Some(Selection::Profile),
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
        // The speech model came: what waited is transcribed now.
        let ready = self
            .speech
            .as_ref()
            .is_none_or(|s| s.status() == SpeechStatus::Ready);
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
        // Pictures dropped onto the window become a photo Note; a bundle
        // file is imported, a Sent Item previewed, as are the files
        // handed over from outside.
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
        let photos: Vec<Vec<u8>> = dropped
            .iter()
            .filter_map(|file| match (&file.bytes, &file.path) {
                (Some(bytes), _) => Some(bytes.to_vec()),
                (None, Some(path)) => std::fs::read(path).ok(),
                (None, None) => None,
            })
            .filter_map(|bytes| photo_from_file(&bytes))
            .collect();
        if !photos.is_empty() {
            actions.push(Action::Photos(photos));
        }
        if self.dialog.is_none() {
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
        }
        // The first start's view, instead of the panes, until answered.
        if self.welcome.is_some() {
            let (t, wall) = (self.t, self.wall.clone());
            let mut asked = Vec::new();
            if let Some(welcome) = &mut self.welcome {
                egui::CentralPanel::default()
                    .frame(Frame::NONE.inner_margin(Margin::same(24)))
                    .show(ui, |ui| {
                        ui.set_max_width(640.0);
                        embossed(ui, tones, false, None, |ui| {
                            backup_view::welcome(ui, &t, &wall, welcome, &mut asked);
                        });
                    });
            }
            for asked in asked {
                self.apply(Action::Welcome(asked));
            }
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
        self.dragged(ui.ctx(), tones);
        for action in actions {
            self.apply(action);
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
    /// Once something is typed, the Nudge times show; Ctrl+Enter takes
    /// the 1 h one.
    fn capture_card(&mut self, ui: &mut Ui, tones: Tones, actions: &mut Vec<Action>) {
        let t = self.t;
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
                && !self.capture.trim().is_empty()
                && ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Enter))
            {
                actions.push(Action::Capture(Notes::default_nudge()));
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
            if has_recorder {
                match media_view::voice_button(ui, &t, recording) {
                    Some(Voice::Record) => actions.push(Action::Record),
                    Some(Voice::StopAndSave) => {
                        actions.push(Action::Capture(Notes::default_nudge()));
                    }
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
            if self.capture.trim().is_empty() && recording.is_none() {
                return;
            }
            ui.add_space(4.0);
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
        if self.view.notes.is_empty() {
            let empty = if trash {
                self.t.trash_empty()
            } else {
                self.t.no_notes_yet()
            };
            if !(trash && !self.view.trashed_routines.is_empty()) {
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
                    let selected = self.selected_note.as_deref() == Some(placed.note.id.as_str());
                    // A Note in the trash is restored, not filed.
                    let draggable = !placed.note.is_trashed();
                    let media = CardMedia {
                        photos: &mut self.photos,
                        pictures: &mut self.pictures,
                        store: &self.store,
                        transcribing: transcribing.as_deref(),
                    };
                    let (card, author) =
                        note_card(ui, &self.t, tones, placed, selected, draggable, media);
                    if let (Some(author), Some(profile)) = (author, &placed.author)
                        && author.clicked()
                    {
                        actions.push(Action::OpenPerson(profile.id.clone()));
                    } else if card.clicked() {
                        actions.push(Action::SelectNote(placed.note.id.clone()));
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
        let Some((placed, sections)) = &self.view.detail else {
            return;
        };
        let linked = &self.view.routines;
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
        let removable = &self.view.removable;
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
                        let shown = (&mut *cache, &**store, removable);
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
                    ui.label(RichText::new(when(&t, note.captured)).color(text));
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
                    ui.menu_button(RichText::new(t.file_into()).color(text), |ui| {
                        for (area, topics) in areas {
                            let mut file = |ui: &mut Ui, id: &str, label: String, indent| {
                                ui.horizontal(|ui| {
                                    ui.add_space(indent);
                                    if ui.button(RichText::new(label).color(text)).clicked() {
                                        actions.push(Action::File {
                                            note: note.id.clone(),
                                            place: id.to_string(),
                                        });
                                    }
                                });
                            };
                            file(ui, &area.id, area_name(&t, area), 0.0);
                            for topic in topics {
                                file(ui, &topic.id, topic_name(&t, topic), 16.0);
                            }
                        }
                        if areas.is_empty() {
                            ui.label(RichText::new(t.no_areas_yet()).color(weak));
                        }
                    });
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
                        let shown = (&mut *cache, &**store, removable);
                        editor_media(
                            ui,
                            &t,
                            media,
                            shown,
                            (playing, transcribing, can_play, speed),
                            actions,
                        );
                        ui.label(
                            RichText::new(when(&t, section.captured))
                                .small()
                                .color(weak),
                        );
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
                    if !may.schedules {
                        ui.label(RichText::new(t.only_managers()).small().color(weak));
                        return;
                    }
                    ui.horizontal(|ui| {
                        for (reminder, label) in [(false, t.routine()), (true, t.reminder())] {
                            if ui.button(RichText::new(label).color(text)).clicked() {
                                actions.push(Action::NewRoutine {
                                    note: Some(note.id.clone()),
                                    reminder,
                                });
                            }
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
                    Dialog::TrashNote { question } => {
                        title(ui, question);
                        let text = ui.visuals().text_color();
                        ui.label(RichText::new(t.delete_note_linked_detail()).color(text));
                        ui.horizontal(|ui| {
                            cancel(ui, &t, actions);
                            if ui.button(RichText::new(t.no()).color(text)).clicked() {
                                actions.push(Action::TrashNote(false));
                            }
                            if destructive(ui, t.yes()) {
                                actions.push(Action::TrashNote(true));
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
                    Dialog::Share { area, role } => {
                        let shown = view
                            .area(area)
                            .map(|a| area_name(&t, a))
                            .unwrap_or_default();
                        share_form(ui, &t, &shown, role, actions);
                    }
                    Dialog::Send {
                        recipients,
                        picked,
                        update,
                        cancel,
                        ..
                    } => send_form(ui, &t, recipients, (picked, *update, cancel), actions),
                    Dialog::SentItem { opened, compared } => {
                        sent_item_form(ui, &t, tones, (opened, *compared, pictures), actions);
                    }
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
                    Dialog::Person {
                        comparing, name, ..
                    } => {
                        let mut asked = Vec::new();
                        if let Some(square) = person_square.as_mut() {
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
                                (*comparing, name),
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
                _ => Action::CloseDialog,
            });
        }
    }

    /// The Agenda (UI laws, "Agenda"): the capture field, the Missed
    /// Summary until "Seen", the "x Unsorted" banner, today's
    /// Occurrences, a gap, and tomorrow's dimmed; each checkable on its
    /// own, a click on one opens its Routine. The Profile picture in the
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
                    let banner = embossed(ui, tones, false, None, |ui| {
                        ui.horizontal(|ui| {
                            icons::show(ui, icons::INBOX, 20.0, tones.accent);
                            ui.label(
                                RichText::new(t.unsorted_banner(agenda.unsorted as i64))
                                    .color(text),
                            );
                        });
                    });
                    let banner =
                        ui.interact(banner.rect, Id::new("unsorted banner"), Sense::click());
                    banner.widget_info(|| {
                        WidgetInfo::labeled(
                            WidgetType::Button,
                            true,
                            t.unsorted_banner(agenda.unsorted as i64),
                        )
                    });
                    if banner
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .clicked()
                    {
                        actions.push(Action::Select(Selection::Unsorted));
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
                ui.add_space(8.0);
            });
        if output.state.offset.y != self.scrolled {
            self.scrolled = output.state.offset.y;
            ui.ctx().request_repaint();
        }
    }

    /// The Profile page (ADR 0017): the own Profile's picture, name and
    /// short code; Backups and recovery; the user's Devices; and the way
    /// to this Device's settings. While a new picture's square is picked,
    /// that alone.
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
        let (store, wall, backup_form, view) = (
            &self.store,
            &self.wall,
            &mut self.backup_form,
            &mut self.profile_view,
        );
        let (people, pictures) = (&self.view.people_shown, &mut self.pictures);
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
                // People (#52): every Profile kept here, each opening it.
                embossed(ui, tones, false, None, |ui| {
                    ui.label(RichText::new(t.people_title()).strong().color(text));
                    if people.is_empty() {
                        ui.label(RichText::new(t.profile_people_empty()).small().color(weak));
                    }
                    for person in people {
                        let name = profile_view::display_name(&t, &person.name).to_string();
                        ui.horizontal(|ui| {
                            let open = t.person_open(&name);
                            let picture =
                                profile_view::person_button(ui, pictures, person, 32.0, &open);
                            let line = ui.add(
                                egui::Button::new(RichText::new(&name).color(text)).frame(false),
                            );
                            if picture.clicked() || line.clicked() {
                                people_asked.push(person.id.clone());
                            }
                        });
                    }
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
    fn open_person(&mut self, id: String) -> oldbrainfix_core::Result<()> {
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
        });
        Ok(())
    }

    /// Closes the dialog. A person's Profile keeps the name typed last
    /// and shows the dialog it opened over again, a Sent Item's sender as
    /// the user sees them now.
    fn close_dialog(&mut self) -> oldbrainfix_core::Result<()> {
        let behind = self.dialog_behind.take();
        if let Some(Dialog::Person { id, name, .. }) = self.dialog.take() {
            self.store.set_override_name(&id, &name)?;
            self.person_square = None;
            self.dialog = behind;
            if let Some(Dialog::SentItem { opened, .. }) = &mut self.dialog
                && opened.trust != SenderTrust::Unknown
            {
                opened.sender = self.store.person_shown(&opened.sender.id)?;
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
            _ => {}
        }
        let Some(Dialog::Person {
            id,
            comparing,
            name,
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
            PersonAction::UseSquare | PersonAction::CancelSquare | PersonAction::TheirNotes => {}
            PersonAction::CompareCodes => *comparing = Comparing::Asking,
            PersonAction::CodesCompared(false) => *comparing = Comparing::Differ,
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
        }
        Ok(())
    }

    /// This Device's settings (ADR 0008): its notifications and login
    /// start, its Re-nag interval, and the Nudges' repeat and Quiet
    /// Hours.
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
                    minutes_choice(
                        ui,
                        "renag device",
                        t.renag_device(),
                        &t,
                        renag,
                        &choices,
                        |m| {
                            actions.push(Action::Setting(Setting::Renag(m)));
                        },
                    );
                    ui.label(RichText::new(t.settings_renag_detail()).small().color(weak));
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
                    minutes_choice(
                        ui,
                        "nudge repeat",
                        t.nudge_repeat(),
                        &t,
                        repeat,
                        &choices,
                        |m| {
                            actions.push(Action::Setting(Setting::NudgeRepeat(m)));
                        },
                    );
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
                    let what = note.as_ref().map_or(t.note_gone(), |n| n.text.as_str());
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

/// Cancel, which closes the dialog without a change.
fn cancel(ui: &mut Ui, t: &L10n, actions: &mut Vec<Action>) {
    let text = ui.visuals().text_color();
    if ui.button(RichText::new(t.cancel()).color(text)).clicked() {
        actions.push(Action::CloseDialog);
    }
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
/// when their phone joins, to confirm (ADR 0017).
fn share_form(ui: &mut Ui, t: &L10n, area: &str, role: &mut AreaRole, actions: &mut Vec<Action>) {
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
    (opened, compared, pictures): (&Opened, Compared, &mut profile_view::Pictures),
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
/// How much tomorrow is faded on the Agenda (the phone's
/// `tomorrowOpacity`).
pub const TOMORROW_OPACITY: f32 = 0.55;

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

/// The Nudge repeats offered, in minutes (the phone's choices).
pub const NUDGE_REPEAT_CHOICES: [i64; 3] = [10, 30, 60];

/// A row of `label` and a choice of `minutes`; `picked` gets a new one.
fn minutes_choice(
    ui: &mut Ui,
    id: &str,
    label: &str,
    t: &L10n,
    minutes: i64,
    choices: &[i64],
    mut picked: impl FnMut(i64),
) {
    let text = ui.visuals().text_color();
    ui.horizontal(|ui| {
        ui.label(RichText::new(label).color(text));
        let mut chosen = minutes;
        egui::ComboBox::from_id_salt(id)
            .selected_text(RichText::new(t.renag_minutes(minutes)).color(text))
            .show_ui(ui, |ui| {
                for m in choices {
                    ui.selectable_value(
                        &mut chosen,
                        *m,
                        RichText::new(t.renag_minutes(*m)).color(text),
                    );
                }
            });
        if chosen != minutes {
            picked(chosen);
        }
    });
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
fn check(ui: &mut Ui, tones: Tones, done: bool, label: &str) -> egui::Response {
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

/// What an Occurrence of a Routine shows as: its Note's text, or that the
/// Note is gone.
fn row_title(t: &L10n, note: Option<&Note>) -> String {
    match note {
        Some(note) if !note.text.trim().is_empty() => note.text.trim().to_string(),
        Some(note) if note.photos.is_empty() && !note.recordings.is_empty() => {
            t.voice_recording().to_string()
        }
        Some(_) => t.photo().to_string(),
        None => t.note_gone().to_string(),
    }
}

/// One Occurrence on the Agenda as an embossed row: the box to check it
/// off, its time, its Note's text, and its Area's colour and icon. A
/// click on the text opens its Routine.
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
        ui.horizontal(|ui| {
            let label = format!("{}: {time} {title}", t.mark_done());
            if check(ui, tones, done, &label).clicked() {
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

/// What the editor shows of `media`: the photos, the recordings with
/// their play buttons and playback `speed` while `can_play`, the button under each one the
/// Device may remove while its text stays, and the texts of those
/// removed where the files were (ADR 0020).
fn editor_media(
    ui: &mut Ui,
    t: &L10n,
    media: Media,
    (cache, store, removable): (&mut PhotoCache, &Store, &HashSet<(String, String)>),
    (playing, transcribing, can_play, speed): (Option<&str>, Option<&str>, bool, f64),
    actions: &mut Vec<Action>,
) {
    let id = media.id;
    let (photos, removed_photos, photo_texts) = media.photos;
    let (recordings, removed_recordings, transcripts) = media.recordings;
    let may = |hash: &str| removable.contains(&(id.to_string(), hash.to_string()));
    let ask = |hash: String, photo: bool| Action::AskRemoveMedia {
        note: id.to_string(),
        hash,
        photo,
    };
    if let Some(hash) = media_view::photos(ui, t, cache, store, photos, &may) {
        actions.push(ask(hash, true));
    }
    let asked = media_view::recordings(
        ui,
        t,
        recordings,
        transcripts,
        (playing, transcribing, speed),
        can_play,
        &may,
    );
    match asked {
        Some(Recording::Play(hash)) => actions.push(Action::Play(hash)),
        Some(Recording::Remove(hash)) => actions.push(ask(hash, false)),
        Some(Recording::Speed(speed)) => actions.push(Action::Speed(speed)),
        None => {}
    }
    media_view::removed_texts(
        ui,
        (removed_photos, photo_texts),
        (removed_recordings, transcripts),
    );
}

/// A Note as a card (UI laws, "Cards"): its text, when it was captured
/// and where it is filed, its Area's colour down the left edge and icon
/// before the place. Important cards carry the accent marker and a
/// stronger emboss; unimportant ones are dimmed. A `draggable` card can
/// be dropped onto an Area or Topic.
/// What a card needs to show its photos and recordings.
struct CardMedia<'a> {
    photos: &'a mut PhotoCache,
    pictures: &'a mut profile_view::Pictures,
    store: &'a Store,
    /// The recording being transcribed, by its hash.
    transcribing: Option<&'a str>,
}

fn note_card(
    ui: &mut Ui,
    t: &L10n,
    tones: Tones,
    placed: &Placed,
    selected: bool,
    draggable: bool,
    media: CardMedia,
) -> (egui::Response, Option<egui::Response>) {
    let note = &placed.note;
    let important = note.importance == Importance::Important;
    let colour = placed
        .area
        .as_ref()
        .and_then(|a| area_colour(a.colour.as_deref()));
    let mut author_rect = None;
    let inner = ui.scope(|ui| {
        if note.importance == Importance::Unimportant {
            ui.multiply_opacity(theme::UNIMPORTANT_OPACITY);
        }
        let text = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        let card = embossed(ui, tones, important, colour, |ui| {
            if important {
                // Room for the marker on the right.
                ui.set_max_width(ui.available_width() - 36.0);
            }
            // A photo Note may have no words.
            if !note.text.is_empty() {
                ui.label(RichText::new(&note.text).size(16.0).color(text));
            }
            if !note.photos.is_empty() {
                media_view::thumbnails(ui, t, media.photos, media.store, &note.photos, 64.0, 3);
            }
            media_view::recordings(
                ui,
                t,
                &note.recordings,
                &note.transcripts,
                (None, media.transcribing, 1.0),
                false,
                &|_| false,
            );
            media_view::removed_texts(
                ui,
                (&note.removed_photos, &note.photo_texts),
                (&note.removed_recordings, &note.transcripts),
            );
            // Another person's Note shows their picture (ADR 0017); the
            // user's own none.
            ui.horizontal(|ui| {
                if let Some(author) = &placed.author {
                    let name = profile_view::display_name(t, &author.name);
                    let picture = profile_view::person_picture(
                        ui,
                        media.pictures,
                        author,
                        20.0,
                        &t.author_picture(name),
                    );
                    author_rect = Some(picture.rect);
                }
                ui.label(RichText::new(when(t, note.captured)).small().color(weak));
            });
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
    // The author's picture lies over the card and opens their Profile
    // (#52): registered after the card, so it takes the click.
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
    (resp.on_hover_cursor(egui::CursorIcon::PointingHand), author)
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
