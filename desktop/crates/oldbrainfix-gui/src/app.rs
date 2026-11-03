//! The window (UI laws, "Desktop"), a classic desktop window (#145): the
//! menu bar on top; the sidebar on the left with the Agenda, Unsorted,
//! All Notes, Frogs, the Areas and their Topics, marking the place shown;
//! the list in the middle, under the back and forward buttons and the
//! path to the place. A Note opens as a dialog over the window, one at a
//! time (#146), its settings as badges under its text (#160); closing it
//! saves. File ▸ New Note, Ctrl+N and "+" open the quick note as a dialog
//! too; "+" at the top of All Notes, an Area or a Topic also makes a
//! Checklist, a Shopping List, a Tally, a Session or a Frog there (#161).
//! The Routine editor, the New Tally page and a Tally's pages open as
//! dialogs over what they were opened from. The Settings and everything
//! under Sync (Sync, Pair a Device, People, Profile, Backups) open as
//! dialogs over the window from the menu bar.
//!
//! The window opens on the Agenda (UI laws, "Agenda"): the Missed
//! Summary until "Seen", the "x Unsorted" banner, the "x Sent Items
//! waiting" line while a stranger's Sent Item waits for "Show", the "x
//! shared Areas waiting" line while an Area shared from afar waits for
//! Accept or Decline, today's
//! all-day and then timed Occurrences, a gap, and tomorrow's dimmed,
//! then the five days after it under their weekdays and dates, all of them checkable. Its alarm button leads
//! to a new Routine. A Note spawns Routines and Reminders from its
//! editor and from the Note menu; deleting a Note that spawned some asks
//! whether they go too.
//!
//! Every change goes through `oldbrainfix_core`, which writes the entries
//! the phone's core writes. A card dragged onto an Area or Topic on the
//! left is filed there; Ctrl-click and Shift-click select several cards,
//! which a drag, File into… and Ctrl+M then move together
//! ([`CardSelection`], #148). Back and forward go through the places the list
//! showed, as a browser's do ([`Navigation`]): the buttons, Alt+Left and
//! Alt+Right, and the mouse's back and forward buttons; the path leads up
//! the levels. The keys rest while text is being typed, where Alt+arrows
//! move the cursor; over a dialog opened from another they lead back to
//! that one.
//!
//! The quick note records voice too: the microphone's button starts
//! at once and one big stop button ends it and leaves it on the field,
//! saved with the Note by the Nudge times or Save (UI laws, "Capture").
//! Once something is typed, the Note card's buttons show above them;
//! what they set up shows at the top of the field and is saved with the
//! Note, all of it or nothing (#113). A picture pasted from the clipboard,
//! and pictures and sound files chosen with "From file…" or dropped onto
//! the window (where the system hands drops over: not on Wayland), wait
//! in the quick note, which they open, as its photos and voice recordings and are saved with
//! what is typed into one Note, as a share does on the phone; a sound
//! file that cannot be transcribed here is kept and announced once the
//! Note is saved; a sync bundle
//! dropped there is imported, what came in announced at the top; a
//! Sent Item dropped there, opened or handed over by `oldbrainfix open`
//! shows its preview to accept or decline, a stranger's hidden until
//! "Show" and waiting behind the Agenda's "x Sent Items waiting" until
//! decided. A bundle offering an Area shared from afar (#108) shows who
//! shares which Area as what, to accept or decline; an Area is shared so
//! from its Share dialog with a person in People, the offer saved where
//! the user says, and later changes go from that person's Profile as a
//! bundle when they travel as files.
//! A Note is sent from its dialog, saved where the user says.
//! The dialog shows a Note's photos and plays its recordings, with their
//! Transcripts; a right-click on a photo there or full screen saves it
//! where the user says, copies it or opens it in another program (#149). Transcripts and Photo Texts are made in the background
//! and found by the search over All Notes, an Area or a Topic.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
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
use oldbrainfix_core::ai_port::{AiPort, Reach, docker_bridge};
use oldbrainfix_core::backup;
use oldbrainfix_core::bundle::BundleResult;
use oldbrainfix_core::cards::{self, CardOpened};
use oldbrainfix_core::checklists::keys as checklist_keys;
use oldbrainfix_core::ics::routines_ics;
use oldbrainfix_core::note_kinds::{self, NoteKind};
use oldbrainfix_core::offers::{self, HeldAreaOffer, OfferError, Transport};
use oldbrainfix_core::profile::ProfileChoice;
use oldbrainfix_core::routines::routines_by_name;
use oldbrainfix_core::sent::{
    self, HeldSentItem, OpenError, Opened, Received, Recipient, SenderTrust, SentItemState,
};
use oldbrainfix_core::share::{self, NoteShare};
use oldbrainfix_core::sync::SETTINGS_SOURCE;
use oldbrainfix_core::{
    AREA_ICONS, AgendaRow, Alarms, Area, AreaAccess, AreaRole, Blob, BoxState, Boxes,
    CaptureContext, CapturedDetail, CardTaken, Checklist, ChecklistAgendaRow, Checklists,
    ClockTime, Coordinates, Day, DeviceSettings, Frog, Frogs, FrogsToday, Importance, Invite, Line,
    ListOnAgenda, MissedSummary, Note, NoteHistory, Notes, NotesGo, Occurrence, OpenOccurrence,
    OwnProfile, Person, PersonFilter, PersonSettings, Places, Profile, QuickNoteControl,
    QuietHours, Routine, RoutineOccurrence, Routines, Search, Section, SessionSetup, Sessions,
    ShoppingLine, ShoppingList, ShoppingLists, ShoppingOnAgenda, ShownBox, Store, Tallies,
    ThemeChoice, Topic, UnitSystem, bundle, markdown, text_format,
};

use crate::about_view;
use crate::ai_view::{self, AiAction, Client, ReviewShown};
use crate::alerts::Event;
use crate::all_routines_view::{self, AllRoutinesAction};
use crate::autostart::Autostart;
use crate::background;
use crate::backup_view::{self, BackupAction, Passphrase, Welcome, WelcomeAction};
use crate::backups;
use crate::capture_choices::{
    self, Ask, CaptureChoices, Captured, Held, ListTitle, Opens, QuickNote, RowAsk,
};
use crate::card_glance;
use crate::card_occurrences::{self, CardOccurrence, OpenAsked};
use crate::card_selection::CardSelection;
use crate::checklist_view::{self, CardMenu, ChecklistAction, ChecklistShown, LineStep, ShownLine};
use crate::enrich::Enricher;
use crate::folder_import::{FolderImport, Imported, Importing, folder_name};
use crate::folder_sync;
use crate::format_bar::{self, Format};
use crate::frogs_view::{self, FrogAction, FrogShown};
use crate::history_view::{self, HistoryShown};
use crate::hotkey::{self, Chord};
use crate::icons;
use crate::keep_import::KeepSource;
use crate::l10n::{self, L10n};
use crate::line_editing::{Listed, TypedLines};
use crate::login;
use crate::media::{
    Clipboard, FromFile, PICTURE_EXTENSIONS, Player, Recorder, SOUND_EXTENSIONS, from_file,
    photo_extension, photo_from_file,
};
use crate::media_view::{
    self, Generated, Offers, Out, Photo, PhotoCache, Recording, Speech, Viewed, Viewer, Voice,
};
use crate::names::{area_name, offered_area_name, topic_name};
use crate::navigation::Navigation;
use crate::note_history_view;
use crate::paste;
use crate::person_view::{self, Comparing, PersonAction, PersonView, SendBack};
use crate::photo_out;
use crate::profile_view::{self, CardForm, ProfileAction, ProfileView, Square};
use crate::ring::{self, Rung};
use crate::routine_editor::{Intent, RENAG_CHOICES, RoutineEditor};
use crate::schedule_text::{agenda_day_text, day_text, occurrence_time, schedule_text};
use crate::session_editor::{SessionDraft, SessionIntent, WEEKDAYS_OPEN};
use crate::session_view::{self, PicksShown, SessionAction, SessionCardShown, SessionPart};
use crate::setting_badges::{
    BadgeValue, badge, badge_dialog, badge_row, badge_switch, menu_entry, switch_icon,
};
use crate::share_note;
use crate::shopping_view::{self, ShopStep, ShoppingAction, ShoppingShown};
use crate::speech::SpeechModel;
use crate::starter_set_view;
use crate::sync_view::{
    JoinFailure, Joiner, SyncAction, SyncHost, bundle_import_why, bundle_result_text, device_name,
    import_bundle_result, offer_refusal, update_notice_text,
};
use crate::table_view::{self, Opening, TableView};
use crate::tally_page::{PageIntent, TallyPage};
use crate::tally_view::{
    self, EntryForm, FieldForm, FormIntent, TallyAction, TallyForm, TallyShown, Whose,
};
use crate::text_marks_view::{self, BoxAsk, BoxLook, Spots};
use crate::theme::{self, AREA_PALETTE, Tones};

/// What the middle pane shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection {
    /// The Occurrences of today and the six days after, the Missed
    /// Summary and the Unsorted count.
    Agenda,
    /// The strangers' Sent Items waiting for "Show", Accept or Decline
    /// (#91), reached from the Agenda's "x Sent Items waiting" line.
    SentItemsWaiting,
    /// The Areas shared from afar waiting for Accept or Decline (#108),
    /// reached from the Agenda's "x shared Areas waiting" line.
    AreaOffersWaiting,
    /// The review of the AI Task (#112): its changes by kind, Accept all
    /// and Reject all; reached by "Done, review" in the Settings dialog,
    /// and on start while one waits.
    AiReview,
    /// The Frog view (#101): the open Frogs, oldest first, and the
    /// folded "Done" section; reached from the Agenda's "Eat a frog".
    Frogs,
    /// Every Routine and Reminder by name, also those beyond the
    /// Agenda's week (#115), with Export; reached from the Agenda's bar.
    AllRoutines,
    /// The History of the Routine with this id (Q400): its Streak, its
    /// Adherence and its Log; reached from an Occurrence's menu and from
    /// the Routine's editor.
    History(String),
    /// Every Note neither in the trash nor merged.
    All,
    Unsorted,
    /// The Area or Topic with this id; an Area lists its Notes without a
    /// Topic, and its search also finds its Topics' Notes.
    Place(String),
    Trash,
}

/// A dialog over the window that holds what was a view of its own
/// (#145): opened from the menu bar, the Settings and Sync dialogs among
/// them, while the list stays where it was below.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    /// This Device's settings: Edit ▸ Settings… (Ctrl+,).
    Settings,
    /// The version and the source: Help ▸ About, and from the Settings.
    About,
    /// Every third-party component and its licence, reached from About.
    Licences,
    /// The user's own Profile: its picture, name and short code, and its
    /// card; Sync ▸ Profile.
    Profile,
    /// Every Profile kept here, the blocked ones apart, and "Add a person
    /// from a link"; Sync ▸ People.
    People,
    /// The Backups and the recovery sheet; Sync ▸ Backups….
    Backups,
    /// The LAN sync this Device hosts while it shows, the Conflicts, the
    /// shared folder, the bundles and the paired Devices; Sync ▸ Sync now.
    Sync,
    /// Pairing either way round: the question about a Device asking,
    /// this Device's code and the field for the other's; Sync ▸ Pair a
    /// Device. It hosts while it shows, as Sync does.
    Pair,
}

/// How a click on a card selects (#148): a plain click this card alone,
/// Ctrl-click (Cmd on macOS) in or out, Shift-click the run up to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pick {
    One,
    Toggle,
    Run,
}

/// Which of Edit's Cut, Copy and Paste the field the cursor was in gets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Clip {
    Cut,
    Copy,
    Paste,
}

/// What a card carries while it is dragged: the ids of the Notes that
/// move, the selected ones in the list's order when the card is one of
/// them (#148), else its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraggedNotes(pub Vec<String>);

/// A Note with the Area and Topic it is filed in, and the Profile of the
/// other person who wrote it; none for the user's own.
#[derive(Debug, Clone)]
struct Placed {
    note: Note,
    area: Option<Area>,
    topic: Option<Topic>,
    author: Option<Profile>,
}

/// What a Note is made: a Checklist, a Shopping List or a plain Note
/// again (#120, #122).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Checklist,
    ShoppingList,
    Plain,
}

/// One Agenda row with the Area its Note is filed in.
#[derive(Debug, Clone)]
struct Row {
    row: AgendaRow,
    area: Option<Area>,
    /// For a Session what shows under its title.
    session: Option<SessionPart>,
    /// For a Routine spawned from a Checklist its Lines as checked for
    /// this Occurrence, and whether this Device may check them (#121).
    lines: Option<(Checklist, bool)>,
}

/// A Shopping List's group on today's Agenda (#123, #186, #208): its
/// heading, the list's title, and its open `lines` under it; or, as
/// chosen for it, the heading alone saying how many are open,
/// "Groceries: 3 open". With the Area its Note is filed in and whether
/// this Device may check its Lines.
#[derive(Debug, Clone)]
struct ShoppingRow {
    note_id: String,
    area: Option<Area>,
    /// What the heading says.
    text: String,
    lines: Vec<ShoppingLine>,
    checks: bool,
}

/// A Checklist's group on today's Agenda (#143, #208), shaped as a
/// Shopping List's: its Note, Area, what its heading says and its open
/// Lines under it; with no Lines the heading says how many are open,
/// "Packing: 3 of 7 open". With whether this Device may check them.
#[derive(Debug, Clone)]
struct ChecklistRow {
    note_id: String,
    area: Option<Area>,
    text: String,
    lines: Vec<ShownLine>,
    checks: bool,
}

/// The Lines a Checklist shows under its heading on the Agenda (#208): a
/// Checklist Routine's Occurrence every Line, checked ones struck
/// through, or only its open ones where this Device `hides_checked`
/// (#217); a Checklist without a Routine its open ones. The one place
/// that decides it.
fn agenda_lines(list: &Checklist, hides_checked: bool) -> Vec<ShownLine> {
    checklist_view::shown_lines(list)
        .into_iter()
        .filter(|line| (list.on.is_some() && !hides_checked) || !line.checked)
        .collect()
}

/// Whether `row` shows as Done: checked off, or Done from the entries of
/// every pick or from every Line checked.
fn shown_done(row: &Row) -> bool {
    row.row.done().is_some()
        || matches!(&row.session, Some(SessionPart::Today(s)) if s.today.complete())
        || row
            .lines
            .as_ref()
            .is_some_and(|(list, _)| list.all_checked())
}

/// What the Agenda shows.
#[derive(Debug, Clone, Default)]
struct AgendaView {
    today: Vec<Row>,
    /// Today's rows of the Shopping Lists, as Settings of this Device
    /// choose (#123).
    shopping: Vec<ShoppingRow>,
    /// Today's rows of the Checklists without a Routine that show on this
    /// person's Agenda and have open Lines, oldest capture first (#143).
    checklists: Vec<ChecklistRow>,
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
    /// Whether this Device hides what is checked instead of striking it
    /// through (#217): the rows shown as Done are left out already; the
    /// checked Lines and the picks with an entry go as they are drawn.
    hides_checked: bool,
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

/// How the boxes of texts show, by id, the texts whose boxes this Device
/// may tick and the Notes with Show ticks on, as [`App::box_looks`] reads
/// them.
type BoxLooks =
    oldbrainfix_core::Result<(HashMap<String, BoxLook>, HashSet<String>, HashSet<String>)>;

/// The local setting of this Device that says whether the Note `note`
/// shows after each box who ticked it last and when (ADR 0035), the
/// phone's `showTicksKey`.
fn show_ticks_key(note: &str) -> String {
    format!("showTicks:{note}")
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
    /// Those of `people_shown` a newer version of the app saved a Profile
    /// or an Override of in a form this one does not know (ADR 0034).
    people_newer: std::collections::HashSet<String>,
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
    /// The captured details of the selected Note and its Sections this
    /// Device may remove while the capture time stays (ADR 0037), by
    /// Note or Section id.
    captured_removable: HashSet<(String, CapturedDetail)>,
    notes: Vec<Placed>,
    /// The Sections of each Note listed, oldest first, read in one pass
    /// for the full cards (#114).
    sections: BTreeMap<String, Vec<Section>>,
    /// The Checklists of the Notes listed that are one, read in one pass
    /// for the cards as they show now, each with whether this Device may
    /// check its Lines (#120, #121).
    checklists: BTreeMap<String, (Checklist, bool)>,
    /// The Shopping Lists of the Notes listed that are one, read in one
    /// pass for the cards, each with whether this Device may check its
    /// Lines (#122).
    shopping_lists: BTreeMap<String, (ShoppingList, bool)>,
    /// The Notes listed that are a Tally, read in one pass for the
    /// cards, which show their name and description (#188).
    tallies: HashSet<String>,
    /// The Tallies listed whose card shows its detailed view, read for
    /// their chart and entries on it (#190), by their Note.
    card_tallies: HashMap<String, TallyShown>,
    /// The Notes listed that are a Session, which show their name and
    /// description (#191), and those whose card shows its detailed view,
    /// read for their pool, today's picks and last weeks on it.
    sessions: HashSet<String>,
    card_sessions: HashMap<String, SessionCardShown>,
    /// The Notes listed whose text this Device may change from their
    /// cards, a table full screen (#128): its to change, not in the
    /// trash, in a format known here.
    card_changes: HashSet<String>,
    /// How the boxes of each Note's and Section's text listed or shown
    /// show (ADR 0035), by its id: their states, and with Show ticks on
    /// what each says after it.
    box_looks: HashMap<String, BoxLook>,
    /// The Notes and Sections listed or shown whose boxes this Device may
    /// tick: where it holds a role, not in the trash.
    box_ticks: HashSet<String>,
    /// The Notes listed or shown with Show ticks on, on this Device.
    show_ticks: HashSet<String>,
    /// How the boxes of the table shown full screen show, while it is
    /// one of a stored text.
    table_ticking: Option<table_view::Ticking>,
    /// Whether the list shows its cards in the compact view, as this
    /// Device keeps it for that list (#114, #189).
    compact: bool,
    /// The Notes of the list whose card a click on its view button
    /// switched to the other view, kept on this Device until the list's
    /// switch changes (#189).
    switched: HashSet<String>,
    /// What the compact view says of each listed Tally and Note running a
    /// Session, by its id (#189).
    glances: HashMap<String, Vec<String>>,
    /// The Notes whose card this Device keeps with its struck Lines, or
    /// its description, unfolded (#189).
    struck_shown: HashSet<String>,
    descriptions_unfolded: HashSet<String>,
    /// The selected Note with its Sections.
    detail: Option<(Placed, Vec<Section>)>,
    /// Whether the Note being captured may spawn where it goes (#113).
    capture_spawns: bool,
    /// Whether this Device may set up Sessions and Frogs where the list
    /// shows, for what "+" offers (#161).
    list_manages: bool,
    /// What this Device's quick note shows (#142).
    capture_quick: QuickNote,
    /// The Shopping Lists the Note being captured can go into, by their
    /// Note's id with their title (#124).
    capture_lists: Vec<ListTitle>,
    /// The Routines and Reminders the selected Note spawned.
    routines: Vec<Routine>,
    /// What the selected Note asks to check off now (#215): of each of
    /// them this Device may check off, the Occurrence due, else the
    /// latest Missed one.
    open: Vec<OpenOccurrence>,
    /// Today's Occurrences of the Routines and Reminders of each Note
    /// listed, read in one pass for the boxes on their cards (#215).
    card_today: HashMap<String, Vec<CardOccurrence>>,
    /// How many of them a newer version of the app saved the schedule of,
    /// which shows only that (ADR 0034).
    newer_routines: usize,
    /// The Frogs the selected Note spawned, oldest first, each with
    /// whether this Device may check it off.
    note_frogs: Vec<(Frog, bool)>,
    /// The Tally of the selected Note, while it is one (#103).
    tally: Option<TallyShown>,
    /// The Checklist of the selected Note, while it is one, and whether
    /// this Device may make the selected Note one (#120).
    checklist: Option<ChecklistShown>,
    makes_checklist: bool,
    /// The Shopping List of the selected Note, while it is one, and
    /// whether this Device may make the selected Note one (#122).
    shopping: Option<ShoppingShown>,
    makes_shopping: bool,
    /// Whether this Device may switch the selected Note's list, a
    /// Checklist to a Shopping List or back (#188).
    switches_list: bool,
    /// The kind of the selected Note (#188), plain while none is.
    kind: Option<NoteKind>,
    /// The setup of the selected Note while it is a Session (#191), and
    /// whether this Device may make it one, change its setup or make it
    /// a plain Note again: as it may change the Note, in the lists.
    session_setup: Option<SessionSetup>,
    makes_session: bool,
    /// The Frog view's open and Done Frogs, while it shows, and whether
    /// its "Done" section is open.
    frogs_open: Vec<FrogShown>,
    frogs_done: Vec<FrogShown>,
    frogs_done_open: bool,
    /// Every Routine and Reminder in alphabetical order with its name
    /// and its Note's Area, while their page shows (#115).
    all_routines: Vec<(Routine, String, Option<Area>)>,
    /// Those whose schedule a newer version of the app saved, with the
    /// name and Area of their Note, which a click opens (ADR 0034).
    all_newer: Vec<all_routines_view::NewerRow>,
    /// The Routine's History, while it shows.
    history: Option<HistoryShown>,
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
    /// Whether a newer version of the app saved the own Profile in a form
    /// this one does not know: it shows, and only that version changes it
    /// (ADR 0034).
    profile_newer: bool,
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

/// A notice that an update is needed, announced at the top once.
#[derive(Debug, Clone)]
struct UpdateNotice {
    /// Whether a setting a newer version saved was kept (ADR 0034),
    /// rather than a sync needing the update.
    settings: bool,
    /// What needs the update, and the full text.
    cause: String,
    details: String,
}

/// A dialog over the window.
#[derive(Debug, Clone, PartialEq)]
enum Dialog {
    /// Every tick of a box, the oldest first: the state it set and who
    /// set it when (ADR 0035).
    BoxHistory {
        lines: Vec<(BoxState, String)>,
    },
    /// Every change of the Note `note`, the newest first (#193), each
    /// with "Use this version" where it brings an old text back.
    NoteHistory {
        note: String,
        rows: Vec<note_history_view::ChangeRow>,
    },
    /// The address of a link on the bytes `start..end` of the text being
    /// edited, `linked` when they are in a link already (#127).
    Link {
        address: String,
        start: usize,
        end: usize,
        linked: bool,
        /// Whether it goes on the Amend field's text.
        amend: bool,
        /// The Section whose field's text it goes on (#183).
        section: Option<String>,
    },
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
    /// Whether the Line `id` of a Checklist goes, or of a Shopping List
    /// `for_good` (#122).
    DeleteLine {
        id: String,
        for_good: bool,
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
    /// Whether the captured `detail` of the Note or Section `id` goes
    /// while the capture time stays (ADR 0037).
    RemoveCaptured {
        id: String,
        detail: CapturedDetail,
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
    /// What a job announced at the top as finished brought (UI laws,
    /// "Announcements"): its `details` under `title`, with Close.
    Done {
        title: String,
        details: String,
    },
    /// What an Import announced as finished made (ADR 0036): its
    /// `details`, with Close and a way into the new Area `area`.
    Imported {
        details: String,
        area: String,
    },
    /// Whether the Google Takeout export to import from Google Keep is
    /// still its zip or a folder unpacked from it (ADR 0036).
    ImportKeep,
    /// File ▸ Import… (#145): a folder of Markdown or text files, or a
    /// Google Keep export, each with what it does, or that it reads
    /// while it runs.
    Import,
    /// Note ▸ File into… or Ctrl+M (#145): the Area or Topic the Notes
    /// `notes` go into, the selected ones together (#148).
    FileInto {
        notes: Vec<String>,
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
    /// The name of a new Checklist, Shopping List or Frog made from "+"
    /// (#161).
    NewKind {
        kind: NewKind,
        name: String,
    },
}

/// The badge of the Note dialog whose small editor is open (#160).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NoteBadge {
    Importance,
    Captured,
    /// The Captured details of the Note's Section at this place in its
    /// list, opened from its line (#170).
    SectionCaptured(usize),
    /// The kind, with "Back to a plain Note".
    Kind,
    /// A Plain Note's text format: plain, formatted or code (#213).
    Format,
    OnAgenda,
    /// A Shopping List's "Show on the Agenda", its three ways (#186).
    ShoppingOnAgenda,
    Ticks,
    /// A Session's rule and its weekdays' own counts (#191).
    SessionRule,
    SessionWeekdays,
    /// The reappear time of the Shopping List's Line in `reappear_line`
    /// (#195).
    Reappear,
}

/// The dialogs back closes, or leads back from, on top of the others
/// (#145, #146).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TopDialog {
    Note,
    QuickNote,
    Editor,
    TallyForm,
    TallyPage,
    Page,
}

impl TopDialog {
    /// What back asks of it.
    fn back(self) -> Action {
        match self {
            TopDialog::Note => Action::CloseNote,
            TopDialog::QuickNote => Action::CloseQuickNote,
            TopDialog::Editor => Action::Editor(Intent::Close),
            TopDialog::TallyForm => Action::TallyForm(FormIntent::Close),
            TopDialog::TallyPage => Action::TallyPage(PageIntent::Close),
            TopDialog::Page => Action::PageBack,
        }
    }
}

/// Where the announcements show (UI laws, "Desktop").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Announcer {
    Window,
    Page,
    Quick,
    Note,
}

/// What "+" at the top of a list makes (#161), as the phone's
/// `NewKind`: a Note through the quick note, a Checklist, a Shopping
/// List, a Tally, a Session or a Frog, and in an Area a Topic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewKind {
    Note,
    Checklist,
    Shopping,
    Tally,
    Session,
    Frog,
    Topic,
}

/// What came of saving the changes for one person (#109).
#[derive(Debug, Clone, PartialEq)]
enum ChangesSent {
    Saved,
    NothingNew,
    /// Refused while an AI Task waits.
    Paused,
    /// Not saved: whose, and the full text of why.
    Failed(String),
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
    /// Opens this dialog over the window, in place of any open (#145).
    OpenPage(Page),
    /// Opens this dialog from the one open, which back leads to again.
    PageInto(Page),
    /// Back to the dialog this one was opened from, or none.
    PageBack,
    ClosePage,
    /// File ▸ New Note (Ctrl+N) and "+": the quick note's dialog
    /// (#146), the cursor in its field.
    NewNote,
    /// Closes the quick note's dialog, keeping what is typed with a
    /// Nudge in 1 h, as leaving it does.
    CloseQuickNote,
    /// Closes the Note dialog, keeping what was typed (#146).
    CloseNote,
    /// Opens the small editor of a badge of the Note dialog, or closes
    /// it (#160).
    Badge(Option<NoteBadge>),
    /// Makes what "+" at the top of the list offers (#161), filed where
    /// the list is.
    Add(NewKind),
    /// Makes the Checklist, Shopping List or Frog the name dialog asks
    /// for.
    CreateNamed,
    /// Something asked of the New Tally page (#161).
    TallyPage(PageIntent),
    /// File ▸ Quick note: the capture popup, as the hotkey opens it.
    QuickNote,
    /// File ▸ Quit: the app ends, also where the tray would keep it.
    Quit,
    /// Edit ▸ Find (Ctrl+F): the cursor into the list's search, of All
    /// Notes where the list has none.
    Find,
    /// Edit ▸ Cut, Copy or Paste, on the field the cursor was in.
    Clip(Clip),
    /// Help ▸ User guide: the guide's desktop page in the browser.
    OpenGuide,
    /// Sync ▸ Sync now: the Sync dialog, and the shared folder synced at
    /// once if one is chosen.
    SyncNow,
    /// Asks where the Notes with these ids are filed (Note ▸ File into…,
    /// Ctrl+M, a card's right-click), together (#148).
    AskFileInto(Vec<String>),
    /// A click on a card (#148): as its [`Pick`] says, it selects this
    /// card alone, adds or takes it away, or selects the run up to it.
    PickCard(String, Pick),
    /// Esc over the list: no card selected.
    ClearCards,
    /// File ▸ Import…: asks which Import.
    AskImport,
    /// Opens the Note in the Note dialog (#146): a double-click on its
    /// card or Edit (#114).
    SelectNote(String),
    /// Switches the card of this Note to its other view, in place, and
    /// keeps that on this Device until the list's switch changes (#189).
    SwitchView(String),
    /// Keeps a fold of a card open, by its setting, or forgets it (#189).
    Fold(String, bool),
    /// Switches every card of the list shown to the compact view or the
    /// detailed one and keeps the choice for that list on this Device
    /// (#114, #189).
    Compact(bool),
    /// Shows the theme chosen in View ▸ Theme at once and keeps it on
    /// this Device (#147).
    Theme(ThemeChoice),
    /// Shows this photo full screen, to zoom and move (#114).
    ViewPhoto(String),
    /// Closes the photo shown full screen.
    ClosePhoto,
    /// Hands this photo out of the Note (#149).
    PhotoOut(String, Out),
    /// Shows a table of a formatted text full screen (#128).
    OpenTable(Opening),
    /// Something asked of the table shown full screen.
    Table(table_view::Asked),
    /// Captures the capture field's text and the recording running,
    /// nudged this long from now.
    Capture(Duration),
    /// Something asked of the capture's buttons or its choices (#113).
    CaptureChoice(Ask),
    /// "To full editor" (#142): captures as leaving does and opens the
    /// Note in the Note dialog (#146), its text taking the keyboard.
    CaptureToEditor,
    /// Starts recording voice into the Note being captured.
    Record,
    /// Ends the recording and holds it on the capture field.
    StopRecording,
    /// Plays the voice recording with this hash, or stops it playing.
    Play(String),
    /// Plays recordings at this speed from now on, the one playing
    /// included, and keeps it on this Device.
    Speed(f64),
    /// Asks the system's dialog for pictures and sound files and holds
    /// them on the capture field.
    PickFiles,
    /// Holds these files, by name and content, on the capture field for
    /// one Note with what is typed: the pictures as photos, the sound
    /// files as voice recordings; anything else is left alone.
    Files(Vec<(String, Vec<u8>)>),
    /// Shows why the recordings announced could not be transcribed.
    ShowUntranscribable,
    /// The search over the Notes shown changed.
    Search,
    /// Searches All Notes for this, a tag clicked as written (#126).
    SearchFor(String),
    /// Shows only the Notes of these people (#53).
    FilterPeople(PersonFilter),
    Speech(Speech),
    /// Keeps the selected Note's edited text.
    SaveText,
    /// Keeps it as a formatted text, its first format just applied
    /// (#127, ADR 0033): the Note's, or the Section's of this id (#183).
    FormatText(Option<String>),
    /// Puts the Note's text, as typed, into this format (#213).
    SetTextFormat(&'static str),
    /// A paste that looks like code into the Note's text: "Keep as code"
    /// under it (#213), or, without, that offer closed.
    OfferCode(bool),
    /// Asks for the address of a link on the bytes `start..end` of the
    /// edited text, of the Amend field's with `amend` or of the field of
    /// the Section `section` (#183), `url` the link's there is one
    /// already.
    AskLink {
        start: usize,
        end: usize,
        url: Option<String>,
        amend: bool,
        section: Option<String>,
    },
    /// Puts the link asked for on the text it was asked for.
    SetLink,
    Amend,
    /// Opens the table of the Amend field's text at this byte full
    /// screen, putting a new one in there when there is none.
    AmendTable(usize),
    Importance(Importance),
    /// Files these Notes into the Area or Topic `place`, together; the
    /// ones this Device may not move stay (#148).
    File {
        notes: Vec<String>,
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
    /// "Done and move to trash" at the top of the Note dialog (#215):
    /// checks off the Occurrence of the open Note's one Reminder, moves
    /// the Note with it to the trash and closes the dialog.
    DoneAndTrash(String, Occurrence),
    /// "Seen" on the Missed Summary.
    Seen,
    OpenRoutine(String),
    /// Saves one calendar file of every Routine and Reminder, in the
    /// order their page lists them (#115).
    ExportRoutines,
    /// Saves the Routine with this id as a calendar file, its whole
    /// series with its end, or the Reminder (#115).
    ShareRoutine(String),
    /// A new Routine, or Reminder, from the Note, or with its Note made
    /// behind the scenes.
    NewRoutine {
        note: Option<String>,
        reminder: bool,
    },
    /// A new Session from the Note with this id (#104), or with its Note
    /// made behind the scenes.
    NewSession(Option<String>),
    /// Something asked of a Session's row on the Agenda, or of the picks
    /// on a Session Note's card (#191).
    Session(SessionAction),
    /// The setup's page of a Session made of the plain Note with this id,
    /// from its kind badge (#191).
    MakeSession(String),
    /// Makes the Session Note with this id a plain Note again (#191).
    UnmakeSession(String),
    /// Writes what the open Session Note's page or one of its badges
    /// changed of its setup, at once (#191): the draft as changed, and
    /// the setup it was read from, which it is compared with. Carried
    /// along, so a read of the view in between loses nothing.
    SessionSetup(Box<(SessionDraft, SessionSetup)>),
    /// "New Tally" in the open Session Note's pool (#191): the New Tally
    /// page, filed where the Note is, its Tally coming back first and
    /// ticked.
    SessionPoolTally,
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
    /// Something asked of the Tally setup in the Note dialog (#103,
    /// #190).
    Tally(TallyAction),
    /// Something asked of the Tally on the detailed card of the Note with
    /// this id (#190).
    CardTally(String, TallyAction),
    /// Something asked of the field page or the entry page.
    TallyForm(FormIntent),
    /// Removes the entry the dialog asks about.
    DeleteEntry,
    /// Something asked of a Checklist, on a card or in the editor (#120).
    Checklist(ChecklistAction),
    /// Something asked of a Shopping List, on a card or in the editor
    /// (#122).
    Shopping(ShoppingAction),
    /// Ticks the box `id` in the text of `entity`, a Note's or a
    /// Section's, to `to` (ADR 0035): an entry beside the text, no edit.
    Tick {
        entity: String,
        id: String,
        to: BoxState,
    },
    /// Shows every tick of the box `id` in the text of `entity`.
    BoxHistory {
        entity: String,
        id: String,
    },
    /// Shows the History of the Note with this id (#193).
    NoteHistory(String),
    /// Writes the text of the change at this place in the open Note
    /// History again, as a new change (#193).
    UseVersion(usize),
    /// Shows after each box of the Note's text, and its Sections', who
    /// ticked it last and when, or no more, on this Device.
    ShowTicks {
        note: String,
        on: bool,
    },
    /// Stores what was typed into the Lines of the Note shown and drops
    /// those left empty, as the editor is left (#141).
    LeaveLines,
    /// Deletes the Line the dialog asks about.
    DeleteLine,
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
    /// Asks whom the Note is sent to.
    StartSend(String),
    /// Puts the Note's text on the clipboard, with HTML beside formatted
    /// text (#129).
    CopyAsText(String),
    /// Saves the Note as a Markdown file with its photos and recordings
    /// beside it, where the user says (#129).
    SaveAsMarkdown(String),
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
    /// Asks for the Google Takeout export the Import dialog says, its
    /// zip (`true`) or the folder unpacked from it, and imports it.
    ImportKeep(bool),
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
    /// Asks whether the captured `detail` of the Note or Section `id`,
    /// the selected Note or one of its Sections, goes (ADR 0037).
    AskRemoveCaptured {
        id: String,
        detail: CapturedDetail,
    },
    /// Removes the captured detail asked about; the capture time stays.
    RemoveCaptured,
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
    /// Shows the dialog behind the notice at the top.
    ShowNoticeDialog,
    /// Shows what went wrong, announced at the top.
    ShowError,
    /// Shows which Device needs an update, announced at the top.
    ShowUpdate,
    /// Something asked of the Backups dialog.
    Backup(BackupAction),
    /// Something asked of the own Profile in the Profile dialog.
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
    /// From when all-day Occurrences ring (#201).
    AllDayFrom(ClockTime),
    /// The capture popup's key chord, as the settings show it.
    Hotkey(String),
    /// The sound file Occurrences ring with; `None` for the app's own
    /// (#102).
    AlarmSound(Option<String>),
    /// The system Tallies show known units in; `None` follows the
    /// region (#103).
    Units(Option<UnitSystem>),
    /// Whether a Routine's History shows its Streak (Q400).
    Streaks(bool),
    /// Whether the Agenda hides what is checked instead of striking it
    /// through (#217).
    HideChecked(bool),
    /// Whether Done on a plain Note's only Reminder moves the Note to the
    /// trash: the person's, on all of their own Devices (#211).
    TrashDoneReminder(bool),
    /// Whether the quick note shows this control (#142).
    QuickNote(QuickNoteControl, bool),
    /// Whether the quick note shows the three Nudge buttons (#142).
    QuickNoteNudgeButtons(bool),
}

/// Opens an address in the browser; tests hand in one that keeps it.
pub type UrlOpener = Box<dyn Fn(&str)>;

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
    /// The cards selected in the list shown, to move together (#148).
    cards: CardSelection,
    view: View,
    /// What went wrong reading or writing the log, in full: announced at
    /// the top without it, its click opening it with the cause and fix.
    error: Option<String>,
    /// How far the Notes list is scrolled, for the parallax.
    scrolled: f32,
    /// The photo shown full screen over the window, while one is (#114).
    viewer: Option<Viewer>,
    /// The table shown full screen over the window, while one is (#128).
    table: Option<TableView>,
    /// Whether that table is in the Amend field's text, which it is
    /// written back into, instead of a Note's.
    table_amends: bool,
    /// Where back and forward lead through the places the list showed
    /// (#145).
    nav: Navigation<Selection>,
    /// The dialogs holding what were views of their own, the one shown
    /// last; back leads through the others (#145).
    pages: Vec<Page>,
    /// The text field the cursor was in last, for Edit's Cut, Copy and
    /// Paste, which take the cursor out of it, and what of them waits.
    last_field: Option<Id>,
    clip: Option<Clip>,
    /// Whether File ▸ New Note or Edit ▸ Find wants the cursor in the
    /// quick note's field or the search, until it has it.
    capture_focus: bool,
    search_focus: bool,
    /// File ▸ Quit and File ▸ Quick note, for the native window to do.
    quit_wanted: bool,
    quick_note_wanted: bool,
    /// The theme this Device chose (#147), and whether the window shows
    /// it yet.
    theme: ThemeChoice,
    theme_shown: bool,
    /// The address the browser is to open, and who opens it; egui's
    /// `open_url` when none.
    open_url: Option<String>,
    url_opener: Option<UrlOpener>,
    /// The capture field's text, and what was set up for its Note, not
    /// written yet (#113).
    capture: String,
    capture_choices: CaptureChoices,
    /// The Note "To full editor" opens, until its text takes the
    /// keyboard in the Note dialog (#142).
    edit_wanted: Option<String>,
    /// The selected Note's text as it is being edited, and that Note;
    /// of a Note of a kind its description, its name in `name` (#188),
    /// whether it is one in `text_kinded`.
    text: String,
    text_of: Option<String>,
    name: String,
    text_kinded: bool,
    /// Whether its text is being edited: a formatted text shows its
    /// source in the field meanwhile, and as it reads otherwise (#127).
    text_editing: bool,
    /// Whether the field is to take the keyboard, once, with the
    /// selection `text_select` in bytes when there is one.
    text_focus: bool,
    text_select: Option<(usize, usize)>,
    /// The text of each Section of the selected Note this Device may
    /// edit in place (#183), by its id, as it is being edited.
    section_texts: BTreeMap<String, SectionField>,
    /// The text of the Section being added, and whether its field is to
    /// take the keyboard, once, with the selection `amend_select` in
    /// bytes when there is one.
    amend: String,
    amend_focus: bool,
    amend_select: Option<(usize, usize)>,
    dialog: Option<Dialog>,
    /// Whether the dialog's text field is to take the keyboard, once, as
    /// it opens.
    focus: bool,
    /// The Routine or Reminder being made or changed, or the New
    /// Session page (#161), in its dialog.
    editor: Option<RoutineEditor>,
    /// The field page or the entry page of a Tally in its dialog, and how
    /// the selected Note's Tally is looked at (#103).
    tally_form: Option<TallyForm>,
    /// The New Tally page (#161), in its dialog.
    tally_page: Option<TallyPage>,
    /// The pool and the count of the selected Session Note as its page
    /// edits them (#191), read again with the view.
    session_draft: Option<SessionDraft>,
    /// Whether the quick note's dialog shows (#146).
    quick_open: bool,
    /// The badge of the Note dialog whose small editor is open (#160).
    badge: Option<NoteBadge>,
    /// The Shopping List Line whose reappear time dialog is open (#195).
    reappear_line: Option<String>,
    /// The Note whose text took a paste that looks like code, while
    /// "Keep as code" waits under it (#213).
    code_offer: Option<String>,
    /// Whether the Lines of the Note opened next are to be typed in at
    /// once: a Checklist or Shopping List just made (#161).
    start_lines: bool,
    /// Whose entries each Tally shows, by its Note, once picked (#190);
    /// the default until then.
    tally_whose: HashMap<String, Whose>,
    /// The Lines of the Note shown as they are typed in place, and what
    /// the Shopping List's field holds (#141); what the menu of the card
    /// last opened offers (#120).
    lines: TypedLines,
    card_menu: Option<CardMenu>,
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
    /// The settings' "All-day items remind from" field as typed (#201).
    all_day_text: String,
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
    /// The pictures pasted, chosen or dropped, held on the capture field
    /// until its Note is saved.
    held_photos: Vec<Vec<u8>>,
    /// Why recordings held from files cannot be transcribed here, said
    /// once their Note is saved.
    held_problems: Vec<(oldbrainfix_whisper::Problem, String)>,
    clipboard: Option<Box<dyn Clipboard>>,
    /// Whether the last Paste found nothing, said in grey above the
    /// capture field until something is pasted or typed (#130).
    paste_empty: bool,
    /// Whether Ctrl+V's paste was taken, until its keys are let go.
    paste_held: bool,
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
    /// The LAN session hosted while the Sync dialogs show, and the name
    /// this computer goes by in it.
    sync: SyncHost,
    sync_name: String,
    /// The Conflicts an automatic sync brought, to be announced at the
    /// top until the Sync dialog shows them.
    arrived: usize,
    /// Asks the system's dialog for the shared folder, titled as given.
    folder_picker: FolderPicker,
    /// Tells the folder thread to sync now; without one, the window
    /// syncs itself.
    folder_wanted: Option<Arc<AtomicBool>>,
    /// Raised by a click on the Nudge: Unsorted is to show, as the
    /// phone's Nudge screen.
    unsorted_wanted: Option<Arc<AtomicBool>>,
    /// Raised by a click on the Changes Nudge: the bundles of the people
    /// waiting are to be saved (#109).
    changes_wanted: Option<Arc<AtomicBool>>,
    /// Ask the system's dialogs where to save a bundle and which one to
    /// import.
    bundle_saver: FileSaver,
    bundle_opener: FolderPicker,
    /// Asks the system's dialog for a folder to import (ADR 0036), and
    /// the Import reading it off the window's thread.
    import_picker: FolderPicker,
    /// Asks the system's dialog for the zip of a Google Takeout export.
    import_zip_picker: FolderPicker,
    import: FolderImport,
    /// Asks where to save a Sent Item.
    item_saver: FileSaver,
    /// Asks where to save the own Profile card file (#144).
    card_saver: FileSaver,
    /// Asks where to save a calendar file, of one Routine or of all.
    calendar_saver: FileSaver,
    /// Asks where to save a Note as Markdown (#129).
    markdown_saver: FileSaver,
    /// Asks where to save a photo (#149).
    photo_saver: FileSaver,
    /// Opens a file in the system's program for it, and the folder the
    /// copies of photos to open are written to (#149).
    photo_opener: (FileOpener, std::path::PathBuf),
    /// Files handed over from outside the window, to open.
    handed: Option<Arc<std::sync::Mutex<Vec<std::path::PathBuf>>>>,
    /// What a bundle saved or imported did, announced at the top until
    /// clicked away.
    notice: Option<String>,
    /// The dialog behind the notice it was announced with (UI laws,
    /// "Announcements"), a failure's or what a finished job brought: the
    /// click on that notice opens it; a later notice says something else
    /// and leaves it closed.
    notice_dialog: Option<(String, Dialog)>,
    /// Syncs that need an update of the app here or on another Device,
    /// and settings a newer version saved that were kept (ADR 0034), each
    /// announced once, which Device behind the click (ADR 0019).
    updates: std::collections::VecDeque<UpdateNotice>,
    /// The own Profile's picture, name field and new picture's square.
    profile_view: ProfileView,
    /// The own Profile card saved (#144), and another person's being
    /// pasted (#107).
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
    /// "Which one is you?" was put off until the Profile dialog asks again.
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

/// Opens a file in another program; an error when none opened it.
pub type FileOpener = Box<dyn Fn(&std::path::Path) -> std::io::Result<()>>;

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

/// The dialogs' filter for bundle files, and Sent Items and Profile
/// card files to open.
fn bundle_dialog(title: &str) -> rfd::FileDialog {
    rfd::FileDialog::new().set_title(title).add_filter(
        oldbrainfix_core::APP_NAME,
        &[bundle::EXTENSION, sent::EXTENSION, cards::EXTENSION],
    )
}

/// The open dialog's filter for the zip of a Google Takeout export.
fn zip_dialog(title: &str) -> rfd::FileDialog {
    rfd::FileDialog::new()
        .set_title(title)
        .add_filter(title, &["zip"])
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

/// The save dialog's filter for a Profile card file (#144).
fn card_dialog(title: &str) -> rfd::FileDialog {
    rfd::FileDialog::new()
        .set_title(title)
        .add_filter(oldbrainfix_core::APP_NAME, &[cards::EXTENSION])
}

/// The save dialog for the photo to be saved as `name`, filtered by its
/// ending (#149).
fn photo_dialog(title: &str, name: &str) -> rfd::FileDialog {
    let dialog = rfd::FileDialog::new().set_title(title).set_file_name(name);
    match std::path::Path::new(name)
        .extension()
        .and_then(|e| e.to_str())
    {
        Some(ending) => dialog.add_filter(ending.to_uppercase(), &[ending]),
        None => dialog,
    }
}

/// What a Note saved as Markdown ends in (#129).
const MARKDOWN_EXTENSION: &str = "md";

/// The save dialog's filter for a calendar file.
fn calendar_dialog(title: &str) -> rfd::FileDialog {
    rfd::FileDialog::new()
        .set_title(title)
        .add_filter("iCalendar", &[CALENDAR_EXTENSION])
}

/// What a calendar file ends in, and the name one is proposed under, as
/// the phone's `calendarFileName`.
const CALENDAR_EXTENSION: &str = "ics";
const CALENDAR_FILE_NAME: &str = "oldbrainfix.ics";

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
        // The system's when it cannot be read.
        let theme = DeviceSettings::new(&store).theme().unwrap_or_default();
        let mut app = App {
            store,
            wall: Arc::new(local_wall),
            t: L10n::new(locale),
            selection: Selection::Agenda,
            selected_note: None,
            cards: CardSelection::default(),
            view: View::default(),
            error: None,
            scrolled: 0.0,
            viewer: None,
            table: None,
            table_amends: false,
            nav: Navigation::default(),
            pages: Vec::new(),
            last_field: None,
            clip: None,
            capture_focus: false,
            search_focus: false,
            quit_wanted: false,
            quick_note_wanted: false,
            theme,
            theme_shown: false,
            open_url: None,
            url_opener: None,
            capture: String::new(),
            capture_choices: CaptureChoices::default(),
            edit_wanted: None,
            text: String::new(),
            text_of: None,
            name: String::new(),
            text_kinded: false,
            text_editing: false,
            text_focus: false,
            text_select: None,
            section_texts: BTreeMap::new(),
            amend: String::new(),
            amend_focus: false,
            amend_select: None,
            dialog: None,
            focus: false,
            editor: None,
            tally_form: None,
            tally_page: None,
            session_draft: None,
            quick_open: false,
            badge: None,
            reappear_line: None,
            code_offer: None,
            start_lines: false,
            tally_whose: HashMap::new(),
            lines: TypedLines::default(),
            card_menu: None,
            region: region_of(sys_locale::get_locale().as_deref()),
            changes: None,
            seen_changes: 0,
            wake: None,
            autostart: None,
            read_at,
            quiet_text: (String::new(), String::new()),
            all_day_text: String::new(),
            hotkey: None,
            hotkey_text: String::new(),
            recorder: None,
            player: None,
            playing: None,
            speed,
            recorded: Vec::new(),
            held_photos: Vec::new(),
            held_problems: Vec::new(),
            clipboard: None,
            paste_empty: false,
            paste_held: false,
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
            unsorted_wanted: None,
            changes_wanted: None,
            bundle_saver: Box::new(|title, name| {
                bundle_dialog(title).set_file_name(name).save_file()
            }),
            bundle_opener: Box::new(|title| bundle_dialog(title).pick_file()),
            import_picker: Box::new(|title| rfd::FileDialog::new().set_title(title).pick_folder()),
            import_zip_picker: Box::new(|title| zip_dialog(title).pick_file()),
            import: FolderImport::default(),
            item_saver: Box::new(|title, name| item_dialog(title).set_file_name(name).save_file()),
            card_saver: Box::new(|title, name| card_dialog(title).set_file_name(name).save_file()),
            calendar_saver: Box::new(|title, name| {
                calendar_dialog(title).set_file_name(name).save_file()
            }),
            markdown_saver: Box::new(|title, name| {
                rfd::FileDialog::new()
                    .set_title(title)
                    .add_filter("Markdown", &[MARKDOWN_EXTENSION])
                    .set_file_name(name)
                    .save_file()
            }),
            photo_saver: Box::new(|title, name| photo_dialog(title, name).save_file()),
            photo_opener: (
                Box::new(photo_out::open_with_system),
                directories::ProjectDirs::from("io.github", "paxel", "oldbrainfix").map_or_else(
                    || std::env::temp_dir().join("oldbrainfix-opened"),
                    |d| d.cache_dir().join("opened"),
                ),
            ),
            handed: None,
            notice: None,
            notice_dialog: None,
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

    /// Hosts the Sync dialogs on `bind` instead of the LAN address, as this
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

    /// Asks `picker` for the folder to import instead of the system's
    /// dialog; tests hand in a path.
    pub fn set_import_picker(&mut self, picker: FolderPicker) {
        self.import_picker = picker;
    }

    /// Asks `picker` for the zip of a Google Takeout export instead of
    /// the system's dialog; tests hand in a path.
    pub fn set_import_zip_picker(&mut self, picker: FolderPicker) {
        self.import_zip_picker = picker;
    }

    /// Whether an Import is reading its folder or zip.
    pub fn importing(&self) -> bool {
        self.import.running()
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
                match std::fs::write(&path, pdf) {
                    Ok(()) => self.notice = Some(t.backup_sheet_saved().to_string()),
                    Err(e) => self.announce_failure(
                        t.backup_sheet_not_out(),
                        t.backup_sheet_not_out_title(),
                        (t.backup_sheet_failed_cause(), t.nothing_lost_fix()),
                        format!("recovery sheet: {e}"),
                    ),
                }
            }
            BackupAction::PickFolder => {
                if let Some(path) = (self.folder_picker)(t.backup_folder_pick_title()) {
                    self.store
                        .set_backup_folder(Some(&path.to_string_lossy()))?;
                    self.backup(BackupAction::Now)?;
                }
            }
            BackupAction::Weekly(on) => self.store.set_backups_weekly(on)?,
            BackupAction::Now => match backups::now(&self.store) {
                // Shown as the last Backup on the card: no announcement
                // (UI laws).
                Ok(_) => {}
                Err(e) => self.announce_failure(
                    t.backup_not_written(),
                    t.backup_not_written_title(),
                    (t.backup_failed_cause(), t.backup_failed_fix()),
                    format!("backup: {e}"),
                ),
            },
        }
        Ok(())
    }

    /// Does what the first start's view asked.
    fn welcome_action(&mut self, asked: WelcomeAction) -> oldbrainfix_core::Result<()> {
        let t = self.t;
        match asked {
            WelcomeAction::New => {
                if let Some(welcome) = &mut self.welcome {
                    welcome.starter = Some(starter_set_view::all());
                }
                self.notice = None;
            }
            WelcomeAction::Plant => {
                let picked = self
                    .welcome
                    .as_ref()
                    .and_then(|w| w.starter.clone())
                    .unwrap_or_default();
                let picked: Vec<&str> = picked.iter().map(String::as_str).collect();
                Places::new(&self.store).plant_starter_set(&picked)?;
                self.welcomed()?;
            }
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
                        Err(oldbrainfix_core::Error::Backup(backup::Problem::NeedsUpdate)) => self
                            .announce_failure(
                                t.restore_not_done(),
                                t.restore_not_done_title(),
                                (t.restore_needs_update(), t.update_open_again_fix()),
                                "backup: a Backup of a later format".into(),
                            ),
                        _ => self.announce_failure(
                            t.restore_not_done(),
                            t.restore_not_done_title(),
                            (t.restore_not_a_backup(), t.restore_pick_fix()),
                            "backup: no Backup header in the file picked".into(),
                        ),
                    }
                }
            }
            WelcomeAction::Restore => {
                let Some(welcome) = &self.welcome else {
                    return Ok(());
                };
                let key = match backup_view::unlock(&t, welcome) {
                    Ok(key) => key,
                    // Said red under the field.
                    Err(backup_view::Unlocked::NotAKey) => {
                        if let Some(welcome) = &mut self.welcome {
                            welcome.code_invalid = true;
                        }
                        return Ok(());
                    }
                    Err(backup_view::Unlocked::Failed(cause, fix, details)) => {
                        self.announce_failure(
                            t.restore_not_done(),
                            t.restore_not_done_title(),
                            (cause, fix),
                            details,
                        );
                        return Ok(());
                    }
                };
                let Some((path, _)) = &welcome.file else {
                    return Ok(());
                };
                match backup::restore(&self.store, path, &key) {
                    Ok(restored) => {
                        self.welcomed()?;
                        self.announce_done(
                            t.restore_done_short(),
                            t.restore_done(restored.entries as i64),
                        );
                        // The playback speed came with it; one that cannot
                        // be read leaves the speed as it was.
                        if let Ok(speed) = DeviceSettings::new(&self.store).playback_speed() {
                            self.speed = speed;
                            if let Some(player) = &mut self.player {
                                player.set_speed(speed);
                            }
                        }
                    }
                    Err(e) => {
                        let (cause, fix) = backup_view::restore_failure(&t, &e);
                        self.announce_failure(
                            t.restore_not_done(),
                            t.restore_not_done_title(),
                            (cause, fix),
                            format!("restore: {e}"),
                        );
                    }
                }
            }
        }
        Ok(())
    }

    /// The first start's view is answered: new with the Starter Set
    /// planted, restored or synced. Only
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

    /// Hands in the dialog asking where to save the own Profile card
    /// file (#144).
    pub fn set_card_saver(&mut self, saver: FileSaver) {
        self.card_saver = saver;
    }

    /// Hands in the dialog asking where to save a calendar file.
    pub fn set_calendar_saver(&mut self, saver: FileSaver) {
        self.calendar_saver = saver;
    }

    /// Hands in the dialog asking where to save a Note as Markdown.
    pub fn set_markdown_saver(&mut self, saver: FileSaver) {
        self.markdown_saver = saver;
    }

    /// Hands in the dialog asking where to save a photo (#149).
    pub fn set_photo_saver(&mut self, saver: FileSaver) {
        self.photo_saver = saver;
    }

    /// Opens photos with `opener` instead of the system's program, their
    /// copies written to `dir` (#149).
    pub fn set_photo_opener(&mut self, dir: std::path::PathBuf, opener: FileOpener) {
        self.photo_opener = (opener, dir);
    }

    /// Hands the photo `hash` out of the Note as `out` asks (#149): saved
    /// where the user says, as its file is; on the clipboard as a
    /// picture; or a copy of it opened in another program. Done says
    /// nothing (UI laws); a failure, or a photo not here, is announced,
    /// the latter before the dialog.
    fn photo_out(&mut self, hash: &str, out: Out) {
        let t = self.t;
        let (said, title) = match out {
            Out::SaveAs => (t.photo_not_saved(), t.photo_not_saved_title()),
            Out::Copy => (t.photo_not_copied(), t.photo_not_copied_title()),
            Out::Open => (t.photo_not_opened(), t.photo_not_opened_title()),
        };
        let Some(bytes) = self.store.photo(hash) else {
            self.announce_failure(
                said,
                title,
                (t.photo_out_missing_cause(), t.photo_out_missing_fix()),
                format!("photo {hash}: not on this device"),
            );
            return;
        };
        let name = share_note::photo_file_name(hash, &bytes);
        let failed = match out {
            Out::SaveAs => {
                let Some(mut path) = (self.photo_saver)(t.save_image_title(), &name) else {
                    return;
                };
                if path.extension().is_none() {
                    path.set_extension(photo_extension(&bytes));
                }
                std::fs::write(&path, &bytes).err().map(|e| {
                    (
                        (t.photo_not_saved_cause(), t.photo_not_saved_fix()),
                        format!("save photo: {e}"),
                    )
                })
            }
            Out::Copy => {
                let put = self.clipboard.as_mut().is_some_and(|c| c.put_photo(&bytes));
                (!put).then(|| {
                    (
                        (t.photo_not_copied_cause(), t.photo_not_copied_fix()),
                        "copy photo: the system clipboard took no picture".to_string(),
                    )
                })
            }
            Out::Open => {
                let (opener, dir) = &self.photo_opener;
                let opened =
                    photo_out::copy_to_open(dir, &name, &bytes).and_then(|path| opener(&path));
                opened.err().map(|e| {
                    (
                        (t.photo_not_opened_cause(), t.photo_not_opened_fix()),
                        format!("open photo: {e}"),
                    )
                })
            }
        };
        if let Some((why, details)) = failed {
            self.announce_failure(said, title, why, details);
        }
    }

    /// What the Note `id` hands to another app (#129), its Sections'
    /// dates as the cards show them; `None` when it is gone.
    fn note_share(&self, id: &str) -> oldbrainfix_core::Result<Option<NoteShare>> {
        let notes = Notes::new(&self.store);
        let Some(note) = notes.by_id(id)? else {
            return Ok(None);
        };
        let t = self.t;
        let sections = notes.sections(id)?;
        Ok(Some(share::note_share(&note, &sections, |at| when(&t, at))))
    }

    /// Puts the Note `id`'s text on the clipboard, with its HTML when it
    /// is formatted; through the window's clipboard, as plain text, when
    /// the system's cannot be reached. Copied says nothing (UI laws); a
    /// Note without words is announced.
    fn copy_as_text(&mut self, id: &str) -> oldbrainfix_core::Result<()> {
        let Some(share) = self.note_share(id)? else {
            return Ok(());
        };
        let t = self.t;
        if share.text.is_empty() {
            self.announce_failure(
                t.note_not_shared(),
                t.note_not_shared_title(),
                (t.copy_nothing_cause(), t.copy_nothing_fix()),
                "copy: no words".to_string(),
            );
            return Ok(());
        }
        let put = self
            .clipboard
            .as_mut()
            .is_some_and(|c| c.put_text(&share.text, share.html.as_deref()));
        if !put {
            self.copy = Some(share.text);
        }
        Ok(())
    }

    /// Saves the Note `id` as a Markdown file where the user says, with
    /// its photos and recordings beside it (#129). Saved says nothing (UI
    /// laws); a failure, or nothing of the Note here, is announced, the
    /// latter before the dialog.
    fn save_as_markdown(&mut self, id: &str) -> oldbrainfix_core::Result<()> {
        let Some(share) = self.note_share(id)? else {
            return Ok(());
        };
        let t = self.t;
        let nothing = |app: &mut App| {
            app.announce_failure(
                t.note_not_shared(),
                t.note_not_shared_title(),
                (t.share_nothing_cause(), t.share_nothing_fix()),
                "markdown: no words and no photo or recording here".to_string(),
            );
        };
        // Said before the dialog, so no place is picked for nothing.
        if !share_note::has_anything(&share, &Notes::new(&self.store)) {
            nothing(self);
            return Ok(());
        }
        let title = row_title(&t, Notes::new(&self.store).by_id(id)?.as_ref());
        let Some(mut path) =
            (self.markdown_saver)(t.save_markdown_title(), &share_note::file_name(&title))
        else {
            return Ok(());
        };
        if path.extension().is_none_or(|e| e != MARKDOWN_EXTENSION) {
            path.set_extension(MARKDOWN_EXTENSION);
        }
        match share_note::save_markdown(&path, &share, &Notes::new(&self.store)) {
            Ok(true) => {}
            Ok(false) => nothing(self),
            Err(e) => self.announce_failure(
                t.note_not_shared(),
                t.note_not_shared_title(),
                (t.share_not_written_cause(), t.nothing_lost_fix()),
                format!("markdown: {e}"),
            ),
        }
        Ok(())
    }

    /// Saves `list` as one calendar file where the user says, made at the
    /// store's now, each titled with its Note's text: every one for
    /// Export, or one Routine as its whole series or one Reminder for
    /// Share. The desktop has no share sheet; the file is what a calendar
    /// app imports. Backing out of the dialog writes nothing.
    fn save_calendar_file(&mut self, list: &[Routine]) -> oldbrainfix_core::Result<()> {
        let t = self.t;
        let names = all_routines_view::names(&t, &Routines::new(&self.store), list)?;
        let ics = routines_ics(
            list,
            |r| names.get(&r.id).cloned().unwrap_or_default(),
            self.store.now(),
            DeviceSettings::new(&self.store).all_day_from()?,
        );
        let Some(mut path) =
            (self.calendar_saver)(t.calendar_file_save_title(), CALENDAR_FILE_NAME)
        else {
            return Ok(());
        };
        if path.extension().is_none_or(|e| e != CALENDAR_EXTENSION) {
            path.set_extension(CALENDAR_EXTENSION);
        }
        match std::fs::write(&path, ics) {
            Ok(()) => self.notice = Some(t.calendar_file_saved().to_string()),
            Err(e) => self.announce_failure(
                t.calendar_file_not_saved(),
                t.calendar_file_not_saved_title(),
                (
                    t.calendar_file_not_saved_cause(),
                    t.calendar_file_not_saved_fix(),
                ),
                format!("calendar file: {e}"),
            ),
        }
        Ok(())
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

    /// What a bundle saved or imported did, or that a job failed, as
    /// announced.
    pub fn notice(&self) -> Option<&str> {
        self.notice.as_deref()
    }

    /// Announces at the top that a job failed, in `said`, which carries
    /// no values (UI laws, "Announcements"); its click opens the failure
    /// dialog with `title`, the cause and the fix, and the full
    /// `details`.
    fn announce_failure(
        &mut self,
        said: &str,
        title: &str,
        (cause, fix): (&str, &str),
        details: String,
    ) {
        self.notice = Some(said.to_string());
        self.notice_dialog = Some((
            said.to_string(),
            Dialog::Failure {
                title: title.to_string(),
                cause: cause.to_string(),
                fix: fix.to_string(),
                details,
            },
        ));
    }

    /// Announces at the top that a job finished, in `said`, which
    /// carries no values; its click opens `details` under `said`, with
    /// Close.
    fn announce_done(&mut self, said: &str, details: String) {
        self.notice = Some(said.to_string());
        self.notice_dialog = Some((
            said.to_string(),
            Dialog::Done {
                title: said.to_string(),
                details,
            },
        ));
    }

    /// Removes the captured `detail` of the Note or Section `id`, asked
    /// about, while the capture time stays (ADR 0037). A detail a sync
    /// took meanwhile is gone already; a refusal, as when a role in the
    /// Area changed, is announced at the top with its cause and fix.
    fn remove_captured(
        &mut self,
        id: &str,
        detail: CapturedDetail,
    ) -> oldbrainfix_core::Result<()> {
        let notes = Notes::new(&self.store);
        let shown = match notes.by_id(id)? {
            Some(note) => Some(note.context),
            None => match &self.view.detail {
                Some((p, _)) => notes
                    .sections(&p.note.id)?
                    .into_iter()
                    .find(|section| section.id == id)
                    .map(|section| section.context),
                None => None,
            },
        };
        if !shown.is_some_and(|context| context.holds(detail)) {
            return Ok(());
        }
        if let Err(refused) = notes.remove_captured(id, detail) {
            let t = self.t;
            self.announce_failure(
                t.remove_captured_failed(),
                t.remove_captured_failed_title(),
                (
                    t.remove_captured_failed_cause(),
                    t.remove_captured_failed_fix(),
                ),
                format!("remove captured {}: {refused}", detail.name()),
            );
        }
        Ok(())
    }

    /// Announces that what was asked was refused for the reason `said`
    /// gives, a sentence without values; the click opens it with what to
    /// do and `details`.
    fn announce_refusal(&mut self, said: &str, details: &str) {
        let t = self.t;
        self.announce_failure(
            said,
            t.action_failed_title(),
            (said, t.action_failed_fix()),
            details.to_string(),
        );
    }

    /// Announces that the Profile picture was not set, as one that cannot
    /// be read is not; the click opens why and what to do.
    fn announce_picture_unreadable(&mut self) {
        let t = self.t;
        self.announce_failure(
            t.profile_picture_not_set(),
            t.profile_picture_not_set_title(),
            (t.profile_picture_unreadable(), t.profile_picture_fix()),
            "profile picture: it cannot be read".into(),
        );
    }

    /// Announces that joining a typed code failed, as `failure` says: the
    /// cause, the fix and the full text (UI laws, "Announcements").
    fn announce_join_failed(&mut self, failure: JoinFailure) {
        let t = self.t;
        match failure {
            JoinFailure::Paused => self.notice = Some(t.sync_paused_error().to_string()),
            JoinFailure::Failed {
                cause,
                fix,
                details,
            } => self.announce_failure(
                t.sync_not_done(),
                t.sync_not_done_title(),
                (cause, fix),
                details,
            ),
        }
    }

    /// Announces that a Sent Item was not taken, refused as `error`
    /// says; the click opens why and what to do.
    fn announce_not_taken(&mut self, error: OpenError) {
        let t = self.t;
        self.announce_failure(
            t.sent_item_not_taken(),
            t.sent_item_not_taken_title(),
            refused(&t, error),
            format!("sent item: {error:?}"),
        );
    }

    /// Announces that an Area shared from afar was not taken, refused as
    /// `error` says; the click opens why and what to do.
    fn announce_offer_not_taken(&mut self, error: OfferError) {
        let t = self.t;
        self.announce_failure(
            t.area_offer_not_taken(),
            t.area_offer_not_taken_title(),
            offer_refusal(&t, error),
            format!("area offer: {error:?}"),
        );
    }

    /// Announces that no Profile card can be made yet, as before this
    /// Device synced with another of the user's; the click opens why.
    fn announce_card_not_made(&mut self) {
        let t = self.t;
        self.announce_failure(
            t.profile_card_not_made(),
            t.profile_card_not_made_title(),
            (t.profile_card_not_yet_cause(), t.profile_card_not_yet()),
            "profile card: this Device is not bound to the Profile yet".into(),
        );
    }

    /// The dialog behind the notice shown, if it was announced with one.
    fn dialog_behind_notice(&self) -> Option<&Dialog> {
        match (&self.notice, &self.notice_dialog) {
            (Some(notice), Some((said, dialog))) if notice == said => Some(dialog),
            _ => None,
        }
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
        let t = self.t;
        if self.store.entries_since(&since)?.is_empty() {
            self.notice = Some(t.bundle_nothing_to_send().to_string());
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
        match bundle::write_bundle_now(&self.store, &path, device.as_deref()) {
            Ok(Some(_)) => self.notice = Some(t.bundle_saved().to_string()),
            Ok(None) => self.notice = Some(t.bundle_nothing_to_send().to_string()),
            Err(oldbrainfix_core::Error::SyncPaused) => {
                self.notice = Some(t.sync_paused_error().to_string());
            }
            Err(e) => self.announce_failure(
                t.bundle_not_saved(),
                t.bundle_not_saved_title(),
                (t.bundle_not_written_cause(), t.nothing_lost_fix()),
                format!("bundle: {e}"),
            ),
        }
        Ok(())
    }

    /// Opens the file at `path`: a Profile card file (#144) opens the
    /// person's Profile or says why nobody was added, a Sent Item shows
    /// its preview, a stranger's held and hidden until "Show" (#91);
    /// anything else is imported as a bundle. A card file is JSON as a
    /// Sent Item is, so it is asked for first.
    fn open_file(&mut self, path: &std::path::Path) {
        let most = sent::MAX_BYTES.max(cards::MAX_FILE_BYTES) as u64;
        let small = std::fs::metadata(path).is_ok_and(|m| m.len() <= most);
        let bytes = if small {
            std::fs::read(path).unwrap_or_default()
        } else {
            Vec::new()
        };
        if cards::looks_like_profile_card(&bytes) {
            let shown = self
                .store
                .open_profile_card_file(&bytes)
                .and_then(|opened| self.card_opened(opened));
            if let Err(e) = shown {
                self.announce_card_not_added(CardTaken::Unreadable, e.to_string());
            }
            return;
        }
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
            Ok(Err(e)) => self.announce_not_taken(e),
            Err(e) => {
                let t = self.t;
                self.announce_failure(
                    t.sent_item_not_taken(),
                    t.sent_item_not_taken_title(),
                    (t.sent_item_unreadable(), t.sent_item_ask_again_fix()),
                    e.to_string(),
                );
            }
        }
    }

    /// Shows what opening a Profile card file came to: the person's
    /// Profile when they are in People now, offering "Send yours back"
    /// when the card asks for it, else that nobody was added, why behind
    /// the click.
    fn card_opened(&mut self, opened: CardOpened) -> oldbrainfix_core::Result<()> {
        match (opened.taken, opened.id) {
            (CardTaken::Added | CardTaken::Updated | CardTaken::Unchanged, Some(id)) => {
                let back = if opened.offer_back {
                    SendBack::Offered
                } else {
                    SendBack::No
                };
                self.open_person(id, back)
            }
            (taken, _) => {
                self.announce_card_not_added(taken, format!("profile card: {}", taken.name()));
                Ok(())
            }
        }
    }

    /// Announces that a Profile card file added nobody, as `taken` says;
    /// the click opens why and what to do.
    fn announce_card_not_added(&mut self, taken: CardTaken, details: String) {
        let t = self.t;
        let why = match taken {
            CardTaken::Own => (t.card_own(), t.card_own_fix()),
            CardTaken::Blocked => (t.card_blocked(), t.blocked_fix()),
            CardTaken::Refused => (t.card_refused(), t.card_refused_fix()),
            CardTaken::NeedsUpdate => (t.sent_item_needs_update(), t.update_open_again_fix()),
            CardTaken::Added
            | CardTaken::Updated
            | CardTaken::Unchanged
            | CardTaken::Unreadable => (t.card_unreadable_cause(), t.card_ask_again_fix()),
        };
        self.announce_failure(t.card_not_added(), t.card_not_added_title(), why, details);
    }

    /// Saves the own Profile card as a file where the user says (#144),
    /// answering a card the user opened when `back`; true once saved.
    /// Nothing asked when no card can be made yet, which is announced.
    fn save_card(&mut self, back: bool) -> oldbrainfix_core::Result<bool> {
        let Some(bytes) = self.store.profile_card_file(back)? else {
            self.announce_card_not_made();
            return Ok(false);
        };
        let t = self.t;
        let name = OwnProfile::new(&self.store)
            .get()?
            .map(|p| p.name)
            .unwrap_or_default();
        let file = cards::file_name(profile_view::display_name(&t, &name));
        let Some(mut path) = (self.card_saver)(t.profile_card_save_title(), &file) else {
            return Ok(false);
        };
        if path.extension().is_none_or(|e| e != cards::EXTENSION) {
            path.set_extension(cards::EXTENSION);
        }
        match std::fs::write(&path, bytes) {
            Ok(()) => Ok(true),
            Err(e) => {
                self.announce_failure(
                    t.profile_card_not_sent(),
                    t.profile_card_not_sent_title(),
                    (t.profile_card_not_written_cause(), t.nothing_lost_fix()),
                    format!("profile card: {e}"),
                );
                Ok(false)
            }
        }
    }

    /// "Send yours back" on the person's Profile the dialog shows: the
    /// own card file, answering theirs; once saved, what to do with it
    /// shows instead.
    fn send_back(&mut self) -> oldbrainfix_core::Result<()> {
        if self.save_card(true)?
            && let Some(Dialog::Person { back, .. }) = &mut self.dialog
        {
            *back = SendBack::Saved;
        }
        Ok(())
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
                self.announce_not_taken(e);
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
                self.announce_not_taken(e);
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
            Err(oldbrainfix_core::Error::FromBlocked) => {
                self.announce_not_taken(OpenError::FromBlocked);
                return Ok(());
            }
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
        let t = self.t;
        match std::fs::write(&path, bytes) {
            Ok(()) => self.notice = Some(t.sent_saved().to_string()),
            Err(e) => self.announce_failure(
                t.sent_item_not_sent(),
                t.sent_item_not_sent_title(),
                (t.sent_item_not_written_cause(), t.nothing_lost_fix()),
                format!("sent item: {e}"),
            ),
        }
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
                self.announce_afar_failed(&e);
                return Ok(());
            }
        };
        let (old, new) = (
            Selection::Place(area.clone()),
            Selection::Place(shared.clone()),
        );
        if self.selection == old {
            self.selection = new.clone();
        }
        self.nav.replace(&old, &new);
        match bundle::write_area_offer_now(&self.store, &path, &shared, &person) {
            Ok(_) => self.notice = Some(t.sent_saved().to_string()),
            Err(e) => self.announce_afar_failed(&e),
        }
        // The Area goes into the folder as well.
        if transport == Transport::Folder {
            self.folder_now()?;
        }
        Ok(())
    }

    /// Announces that sharing from afar failed as `error` says; the click
    /// opens the cause, the fix and the full text. A refusal while an AI
    /// Task waits says so instead, as it does everywhere.
    fn announce_afar_failed(&mut self, error: &oldbrainfix_core::Error) {
        let t = self.t;
        if let oldbrainfix_core::Error::SyncPaused = error {
            self.notice = Some(t.sync_paused_error().to_string());
            return;
        }
        self.announce_failure(
            t.share_afar_failed(),
            t.share_afar_failed_title(),
            (t.share_afar_failed_cause(), t.share_afar_failed_fix()),
            format!("share from afar: {error}"),
        );
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
                self.announce_offer_not_taken(e);
                return Ok(());
            }
            // Decided meanwhile; a block drops it too.
            Err(oldbrainfix_core::Error::Invalid(_)) => {
                if self.store.is_blocked(&held.sender.id)? {
                    self.announce_offer_not_taken(OfferError::FromBlocked);
                } else {
                    self.notice = Some(t.area_offer_gone().to_string());
                }
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
        if let Some(sent) = self.send_changes_to(&id, &title)? {
            self.announce_changes(&[sent]);
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
        let mut sent = Vec::new();
        for id in &planned.people {
            let shown = self.store.person_shown(id)?;
            let name = profile_view::display_name(&self.t, &shown.name).to_string();
            let title = self.t.changes_save_title(&name);
            if let Some(one) = self.send_changes_to(id, &title)? {
                sent.push(one);
            }
        }
        self.announce_changes(&sent);
        Ok(())
    }

    /// Announces what came of saving the changes `sent` for one person or
    /// more, once (UI laws, "Announcements"): a failure whose click opens
    /// each one's full text, else that they are saved, else that nothing
    /// was new; nothing when every dialog was cancelled.
    fn announce_changes(&mut self, sent: &[ChangesSent]) {
        let t = self.t;
        let failed: Vec<&str> = sent
            .iter()
            .filter_map(|s| match s {
                ChangesSent::Failed(details) => Some(details.as_str()),
                _ => None,
            })
            .collect();
        if !failed.is_empty() {
            self.announce_failure(
                t.changes_not_saved(),
                t.changes_not_saved_title(),
                (t.changes_not_saved_cause(), t.changes_not_saved_fix()),
                failed.join("\n"),
            );
        } else if sent.contains(&ChangesSent::Paused) {
            self.notice = Some(t.sync_paused_error().to_string());
        } else if sent.contains(&ChangesSent::Saved) {
            self.notice = Some(t.sent_saved().to_string());
        } else if !sent.is_empty() {
            self.notice = Some(t.changes_nothing_new().to_string());
        }
    }

    /// Saves a bundle of what is new for the person `id` where the save
    /// dialog titled `title` says, and only once it is saved counts the
    /// changes up to the log's mark, read before writing, as sent to them
    /// (#109). What came of it; `None` when the dialog was cancelled.
    fn send_changes_to(
        &mut self,
        id: &str,
        title: &str,
    ) -> oldbrainfix_core::Result<Option<ChangesSent>> {
        if self.store.sync_paused()? {
            return Ok(Some(ChangesSent::Paused));
        }
        if self.store.entries_for_person(id)?.is_empty() {
            return Ok(Some(ChangesSent::NothingNew));
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
                    ChangesSent::Saved
                }
                Ok(None) => ChangesSent::NothingNew,
                Err(oldbrainfix_core::Error::SyncPaused) => ChangesSent::Paused,
                Err(e) => {
                    let name =
                        profile_view::display_name(&self.t, &self.store.person_shown(id)?.name)
                            .to_string();
                    ChangesSent::Failed(format!("{name}: {e}"))
                }
            },
        ))
    }

    /// Imports the bundle at `path` and announces what came in; one
    /// offering an Area shared from afar (#108) shows its preview instead.
    fn import_bundle(&mut self, path: &std::path::Path) {
        match import_bundle_result(&self.store, path) {
            Ok(result) => match result.offer {
                Some(held) => {
                    self.dialog = Some(Dialog::AreaOffer {
                        held,
                        compared: Compared::Not,
                        blocking: None,
                    });
                }
                None => self.announce_imported(&result),
            },
            Err(oldbrainfix_core::Error::Offer(e)) => self.announce_offer_not_taken(e),
            Err(oldbrainfix_core::Error::SyncPaused) => {
                self.notice = Some(self.t.sync_paused_error().to_string());
            }
            Err(e) => {
                let t = self.t;
                self.announce_failure(
                    t.bundle_not_imported(),
                    t.bundle_not_imported_title(),
                    bundle_import_why(&t, &e),
                    format!("bundle: {e}"),
                );
            }
        }
        self.refresh();
        // The alerts plan again: a Done in the bundle closes its alarm.
        if let Some(wake) = &self.wake {
            drop(wake.send(Event::Wake));
        }
    }

    /// Announces what the bundle import `result` brought (UI laws,
    /// "Announcements"): that it came in, how much behind the click; that
    /// changes were refused, why behind the click; or that nothing could
    /// be read, as from a Device not paired with this one.
    fn announce_imported(&mut self, result: &BundleResult) {
        let t = self.t;
        if result.needs_pairing() {
            self.announce_failure(
                t.bundle_not_imported(),
                t.bundle_not_imported_title(),
                (t.bundle_pair_first_cause(), t.bundle_pair_first_fix()),
                "bundle: from a Device not paired with this one".into(),
            );
            return;
        }
        let refused = result.report.refused_count() as i64;
        if refused > 0 {
            self.announce_failure(
                &t.host_refused(refused),
                t.host_refused_title(),
                (&t.sync_refused(refused), t.host_refused_fix()),
                bundle_result_text(&t, result),
            );
        } else {
            self.announce_done(t.bundle_imported(), bundle_result_text(&t, result));
        }
    }

    /// Announces how the Import of the folder `name` ended (UI laws,
    /// "Announcements"): that it finished, what went where behind the
    /// click with a way into the new Area; that nothing in it was
    /// usable; or that it failed, each with the cause and the fix.
    fn announce_folder_imported(&mut self, name: &str, imported: Imported) {
        let t = self.t;
        let summary = match imported {
            Ok(summary) => summary,
            Err(details) => {
                self.announce_failure(
                    t.import_failed(),
                    t.import_failed_title(),
                    (t.import_failed_cause(), t.import_failed_fix()),
                    details,
                );
                return;
            }
        };
        let skipped =
            t.import_skipped_details(name, summary.not_text as i64, summary.too_large as i64);
        let Some(area) = summary.area_id.clone() else {
            self.announce_failure(
                t.import_nothing(),
                t.import_nothing_title(),
                (t.import_nothing_cause(), t.import_nothing_fix()),
                skipped,
            );
            return;
        };
        // The Area just made, by the name it shows under; the name asked
        // for should it not be read.
        let shown = match Places::new(&self.store).area_by_id(&area) {
            Ok(Some(made)) => area_name(&t, &made),
            _ => t.import_area_name(name),
        };
        let mut details = t.import_finished_notes(summary.notes as i64, &shown);
        if summary.skipped() > 0 {
            details.push_str("\n\n");
            details.push_str(&t.import_finished_skipped(summary.skipped() as i64));
            details.push('\n');
            details.push_str(&skipped);
        }
        self.notice = Some(t.import_finished().to_string());
        self.notice_dialog = Some((
            t.import_finished().to_string(),
            Dialog::Imported { details, area },
        ));
    }

    /// Announces how the import from Google Keep ended, as
    /// [`App::announce_folder_imported`] does: what went where behind the
    /// click, the Checklists and photos among it and what was skipped
    /// and why, in the phone's order; that the export held no note to
    /// take; or that it failed.
    fn announce_keep_imported(&mut self, imported: Imported) {
        let t = self.t;
        let summary = match imported {
            Ok(summary) => summary,
            Err(details) => {
                self.announce_failure(
                    t.import_keep_failed(),
                    t.import_failed_title(),
                    (t.import_keep_failed_cause(), t.import_keep_failed_fix()),
                    details,
                );
                return;
            }
        };
        let skipped = t.import_keep_skipped_details(
            summary.trashed as i64,
            summary.missing as i64,
            summary.not_photo as i64,
            summary.too_large as i64,
            summary.not_text as i64,
        );
        let Some(area) = summary.area_id.clone() else {
            self.announce_failure(
                t.import_keep_nothing(),
                t.import_nothing_title(),
                (t.import_keep_nothing_cause(), t.import_keep_nothing_fix()),
                skipped,
            );
            return;
        };
        let shown = match Places::new(&self.store).area_by_id(&area) {
            Ok(Some(made)) => area_name(&t, &made),
            _ => t.import_keep_area_name().to_string(),
        };
        let mut details = vec![t.import_finished_notes(summary.notes as i64, &shown)];
        if summary.checklists > 0 {
            details.push(t.import_keep_checklists(summary.checklists as i64));
        }
        if summary.photos > 0 {
            details.push(t.import_keep_photos(summary.photos as i64));
        }
        if summary.skipped() > 0 {
            details.push(String::new());
            details.push(t.import_keep_skipped(summary.skipped() as i64));
            details.push(skipped);
        }
        self.notice = Some(t.import_finished().to_string());
        self.notice_dialog = Some((
            t.import_finished().to_string(),
            Dialog::Imported {
                details: details.join("\n"),
                area,
            },
        ));
    }

    /// Leaves folder syncs to the thread that `wanted` wakes.
    pub fn set_folder_wanted(&mut self, wanted: Arc<AtomicBool>) {
        self.folder_wanted = Some(wanted);
    }

    /// Shows Unsorted, what the phone's Nudge screen shows, whenever a
    /// click on the Nudge raised `wanted`.
    pub fn set_unsorted_wanted(&mut self, wanted: Arc<AtomicBool>) {
        self.unsorted_wanted = Some(wanted);
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
                self.go(Selection::Agenda);
            }
        } else {
            // In the list, the dialogs over it closed.
            self.pages.clear();
            self.go(Selection::AiReview);
        }
        Ok(())
    }

    /// What was asked of the AI port's card or of the review (#112).
    fn ai_action(&mut self, asked: AiAction) -> oldbrainfix_core::Result<()> {
        let t = self.t;
        match asked {
            AiAction::Open => {
                if self.ai_port.is_none() {
                    match AiPort::open(&self.store) {
                        Ok(port) => self.ai_port = Some(port),
                        // The card says so; nothing opened.
                        Err(oldbrainfix_core::Error::NoDockerBridge) => return Ok(()),
                        Err(e) => return Err(e),
                    }
                    self.ai_copied = None;
                }
                // The review waits again until "Done, review".
                if self.selection == Selection::AiReview {
                    self.go(Selection::Agenda);
                }
            }
            AiAction::Reach(reach) => {
                if self.ai_port.is_none() {
                    self.store.set_ai_reach(reach)?;
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
                    port.answer_waiting(&self.store, &*self.wall);
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
                self.go(Selection::Agenda);
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
                self.go(Selection::Agenda);
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

    /// The LAN session the Sync dialogs host.
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

    /// The editor of the Routine `id`, in its dialog.
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

    /// The ids of the cards selected in the list (#148), in its order.
    pub fn selected_cards(&self) -> Vec<String> {
        self.cards.in_order(&self.shown_cards())
    }

    /// What went wrong last, as announced at the top.
    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Shows `selection` in the middle pane, where back then leads from.
    pub fn select(&mut self, selection: Selection) {
        self.apply(Action::Select(selection));
    }

    /// Opens the dialog `page` over the window, in place of any open, as
    /// the menu bar does (#145).
    pub fn open(&mut self, page: Page) {
        self.apply(Action::OpenPage(page));
    }

    /// Closes the dialogs opened with [`App::open`], as their close
    /// button does.
    pub fn close_page(&mut self) {
        self.apply(Action::ClosePage);
    }

    /// The dialog shown over the window, if one is (#145).
    pub fn page(&self) -> Option<Page> {
        self.pages.last().copied()
    }

    /// Whether back and forward lead anywhere (#145).
    pub fn can_go_back(&self) -> bool {
        self.nav.can_go_back()
    }

    pub fn can_go_forward(&self) -> bool {
        self.nav.can_go_forward()
    }

    /// Whether File ▸ Quit was picked since asked last: the app is to
    /// end, also where closing the window leaves it in the tray.
    pub fn take_quit(&mut self) -> bool {
        std::mem::take(&mut self.quit_wanted)
    }

    /// Shows the theme chosen, once on start and once after each choice:
    /// set every frame it would undo a theme set from outside.
    fn show_theme(&mut self, ctx: &egui::Context) {
        if !std::mem::replace(&mut self.theme_shown, true) {
            theme::prefer(ctx, self.theme);
            ctx.request_repaint();
        }
    }

    /// Whether File ▸ Quick note was picked since asked last: the capture
    /// popup is to open, as the hotkey opens it.
    pub fn take_quick_note(&mut self) -> bool {
        std::mem::take(&mut self.quick_note_wanted)
    }

    /// Opens Help ▸ User guide's address with `opener` instead of egui's
    /// `open_url`.
    pub fn set_url_opener(&mut self, opener: UrlOpener) {
        self.url_opener = Some(opener);
    }

    /// Opens the Note `id` in the Note dialog (#146).
    pub fn select_note(&mut self, id: &str) {
        self.apply(Action::SelectNote(id.to_string()));
    }

    /// Opens the Note `id` in the Note dialog, its text taking the
    /// keyboard, as the capture popup's "To full editor" asks (#142,
    /// #146).
    pub fn edit_note(&mut self, id: &str) {
        self.edit_wanted = Some(id.to_string());
        self.apply(Action::SelectNote(id.to_string()));
    }

    /// Keeps what is typed but not saved yet, as the window closes: the
    /// quick note's text as a new Note nudged in 1 h, as leaving the
    /// phone's capture screen does, and the open Note's edited text.
    pub fn flush(&mut self) {
        if !self.capture.trim().is_empty() || self.recording() || self.holds_media() {
            self.apply(Action::Capture(Notes::default_nudge()));
        }
        self.apply(Action::LeaveLines);
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
        // The Lines of a Note left: stored, those left empty dropped
        // (#141).
        let left = match &self.lines.note {
            Some(_) if self.lines.note != self.selected_note => {
                self.lines.leave(&self.store, &*self.wall).err()
            }
            _ => None,
        };
        // A page of a Note's Tally goes with its Note, opened from its
        // card or its dialog (#190).
        let form_note = self
            .tally_form
            .as_ref()
            .filter(|f| !f.capturing())
            .map(|f| f.note_id().to_string());
        if let Some(id) = form_note
            && !Notes::new(&self.store)
                .by_id(&id)
                .ok()
                .flatten()
                .is_some_and(|n| !n.is_trashed() && !n.is_merged())
        {
            self.tally_form = None;
        }
        let mut read = self.read();
        if let (Ok(view), Selection::Place(id)) = (&read, &self.selection)
            && view.area(id).is_none()
            && view.topic(id).is_none()
        {
            self.arrive(Selection::All);
            read = self.read();
        }
        // A Routine gone meanwhile has no History to show: back where it
        // was opened from.
        if let (Ok(view), Selection::History(_)) = (&read, &self.selection)
            && view.history.is_none()
        {
            let back = self.nav.back(self.selection.clone());
            self.arrive(back.unwrap_or(Selection::Agenda));
            read = self.read();
        }
        // Back and forward lead to no place gone meanwhile.
        if let Ok(view) = &read {
            let routines = Routines::new(&self.store);
            let current = self.selection.clone();
            self.nav.retain(&current, |place| match place {
                Selection::Place(id) => view.area(id).is_some() || view.topic(id).is_some(),
                Selection::History(id) => routines.by_id(id).ok().flatten().is_some(),
                _ => true,
            });
        }
        match read {
            Ok(view) => {
                self.view = view;
                self.error = None;
                // The page's draft of a Session Note, read again with it
                // (#191); its tag filter stays while the page is open
                // (#220).
                let picked = self
                    .session_draft
                    .take()
                    .map(|d| d.picked)
                    .unwrap_or_default();
                self.session_draft = match &self.view.session_setup {
                    Some(setup) => {
                        match SessionDraft::of(&self.store, &self.t, &setup.note_id, Some(setup)) {
                            Ok(mut draft) => {
                                draft.picked = picked;
                                Some(draft)
                            }
                            Err(e) => {
                                self.error = Some(e.to_string());
                                None
                            }
                        }
                    }
                    None => None,
                };
                // Only cards still shown stay selected (#148): none hidden
                // by a search, or filed out of the list, moves unseen.
                let shown = self.shown_cards();
                self.cards.retain(&shown);
            }
            Err(e) => self.error = Some(e.to_string()),
        }
        if let Some(e) = left {
            self.error = Some(e.to_string());
        }
        // The Lines shown keep what is typed into them.
        let view = &self.view;
        let detail = view.detail.as_ref().map(|(p, _)| p.note.id.as_str());
        let none = HashSet::new();
        let (listed, rights) = match (&view.checklist, &view.shopping) {
            (Some(s), _) => (Some(Listed::Checklist(&s.list)), (s.adds, &s.editable)),
            (None, Some(s)) => (Some(Listed::Shopping(&s.list)), (s.checks, &s.editable)),
            (None, None) => (None, (false, &none)),
        };
        self.lines.arrange(detail, listed, rights);
        // A list just made: its first Line takes the keyboard (#161).
        if self.start_lines && self.lines.note.is_some() && self.lines.note == self.selected_note {
            self.start_lines = false;
            self.lines.focus_empty_row();
        }
        if self.view.detail.is_none() {
            self.badge = None;
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
            let details = format!("update needed: {update:?} ({source})");
            self.updates.push_back(UpdateNotice {
                settings: source == SETTINGS_SOURCE,
                cause: text,
                details,
            });
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
        if self.pages.contains(&Page::Sync) {
            // What an automatic sync brought meanwhile.
            self.sync.reload_conflicts(&self.store);
        }
        if let Ok(quiet) = DeviceSettings::new(&self.store).quiet_hours() {
            self.quiet_text = (quiet.start.to_string(), quiet.end.to_string());
        }
        if let Ok(from) = DeviceSettings::new(&self.store).all_day_from() {
            self.all_day_text = from.to_string();
        }
        let shown = self.view.detail.as_ref().map(|(p, _)| &p.note);
        let kinded = self.view.kind.is_some_and(|k| k != NoteKind::Plain);
        if shown.map(|n| &n.id) != self.text_of.as_ref() {
            let stored = shown.map(|n| n.text.clone()).unwrap_or_default();
            self.text_of = shown.map(|n| n.id.clone());
            self.text_kinded = kinded;
            self.put_typed(stored);
            self.text_editing = false;
            self.text_focus = false;
            self.text_select = None;
            self.amend.clear();
            self.amend_focus = false;
            self.amend_select = None;
            self.section_texts.clear();
        } else if kinded != self.text_kinded {
            // Its kind changed while it shows: what is typed stays, now
            // as the other kind holds it.
            let typed = self.typed_text();
            self.text_kinded = kinded;
            self.put_typed(typed);
        }
        // Each Section this Device may edit has its field (#183), none
        // while the Note is in the trash; what is typed in one stays as
        // the Sections are read again, only a save puts the stored text
        // back into it.
        let editable = &self.view.text_changeable;
        let sections = self.view.detail.as_ref().map_or(&[][..], |(_, s)| s);
        self.section_texts
            .retain(|id, _| editable.contains(id) && sections.iter().any(|s| &s.id == id));
        for section in sections {
            if editable.contains(&section.id) {
                self.section_texts
                    .entry(section.id.clone())
                    .or_insert_with(|| SectionField {
                        text: section.text.clone(),
                        ..SectionField::default()
                    });
            }
        }
        // "To full editor": the Note's text takes the keyboard (#142).
        if let Some(wanted) = self.edit_wanted.take()
            && self.text_of.as_ref() == Some(&wanted)
        {
            self.text_editing = true;
            self.text_focus = true;
        }
        self.kick();
    }

    /// Which Note list shows, by the phone's names (#114).
    /// The History of the Routine `id` as its page shows it, at the
    /// store's now; `None` when there is no such Routine.
    fn history_shown(&self, id: &str) -> oldbrainfix_core::Result<Option<HistoryShown>> {
        let routines = Routines::new(&self.store);
        let Some(routine) = routines.by_id(id)? else {
            return Ok(None);
        };
        let note = routines.note(&routine)?;
        let week_start = history_view::week_start(&self.t);
        Ok(Some(HistoryShown {
            title: all_routines_view::routine_name(&self.t, note.as_ref()),
            schedule: schedule_text(&self.t, &routine.schedule),
            history: routines.history(&routine, &*self.wall, week_start)?,
            streaks: DeviceSettings::new(&self.store).streaks_on()?,
        }))
    }

    fn cards_key(&self) -> Option<String> {
        let area = matches!(&self.selection, Selection::Place(id) if self.view.area(id).is_some());
        cards_key(&self.selection, &self.people, area)
    }

    /// The theme this Device chose (#147).
    pub fn theme(&self) -> ThemeChoice {
        self.theme
    }

    /// Whether the Note list shown shows its Notes compact (#114).
    pub fn compact(&self) -> bool {
        self.view.compact
    }

    /// The text of the table shown full screen, as edited there, while
    /// one is (#128).
    pub fn shown_table(&self) -> Option<&str> {
        self.table.as_ref().map(TableView::text)
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
        // The Sync dialog lists them: announced.
        if self.pages.contains(&Page::Sync) {
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
            | Selection::SentItemsWaiting
            | Selection::AreaOffersWaiting
            | Selection::AiReview
            | Selection::Frogs
            | Selection::AllRoutines
            | Selection::History(_) => Vec::new(),
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
        // A Note in the trash ticks nothing.
        let card_today = if listed.is_empty() || self.selection == Selection::Trash {
            HashMap::new()
        } else {
            let today = routines.today_by_note(&*self.wall)?;
            card_occurrences::read(store, today, &listed)?
        };
        let lists = Checklists::new(store);
        let mut checklists = BTreeMap::new();
        if !listed.is_empty() {
            for (id, list) in lists.of_notes_now(&listed, &*self.wall)? {
                let checks = checklist_view::may_check_shown(store, &list)?;
                checklists.insert(id, (list, checks));
            }
        }
        let shopping = ShoppingLists::new(store);
        let mut shopping_lists = BTreeMap::new();
        if !listed.is_empty() {
            for (id, list) in shopping.of_notes(&listed, &*self.wall)? {
                let checks = shopping.may_check(&id)?;
                shopping_lists.insert(id, (list, checks));
            }
        }
        // A Tally wins over a list (ADR 0030).
        let mut tallies = HashSet::new();
        let tally_notes = Tallies::new(store);
        for id in &listed {
            if tally_notes.is_tally(id)? {
                tallies.insert(id.clone());
            }
        }
        // A Tally wins over a Session too (ADR 0039).
        let session_notes = Sessions::new(store);
        let mut sessions = HashSet::new();
        for id in listed.iter().filter(|id| !tallies.contains(*id)) {
            if session_notes.is_session_note(id)? {
                sessions.insert(id.clone());
            }
        }
        let mut card_changes = HashSet::new();
        for p in &placed {
            let note = &p.note;
            if !note.is_trashed() && note.text_editable && store.may_change(&note.id)? {
                card_changes.insert(note.id.clone());
            }
        }
        // Every text with boxes listed or shown, with the Note it is of
        // and whether that is in the trash.
        let mut texts: Vec<(&str, &[ShownBox], &str, bool)> = Vec::new();
        for p in placed.iter().chain(detail.as_ref().map(|(p, _)| p)) {
            let note = &p.note;
            texts.push((&note.id, &note.boxes, &note.id, note.is_trashed()));
            let of_note = sections.get(&note.id).into_iter().flatten();
            let detailed = detail
                .as_ref()
                .filter(|(d, _)| d.note.id == note.id)
                .map(|(_, s)| s.as_slice())
                .unwrap_or_default();
            for section in of_note.chain(detailed) {
                texts.push((&section.id, &section.boxes, &note.id, note.is_trashed()));
            }
        }
        let (box_looks, box_ticks, show_ticks) = self.box_looks(&texts)?;
        let is_area = matches!(&self.selection, Selection::Place(id) if areas.iter().any(|(a, _)| a.id == *id));
        let (compact, switched) = match cards_key(&self.selection, &self.people, is_area) {
            Some(key) => (
                self.store.local_setting(&compact_setting(&key))?.as_deref() == Some("true"),
                self.store
                    .local_setting(&switched_setting(&key))?
                    .unwrap_or_default()
                    .split(',')
                    .filter(|id| !id.is_empty())
                    .map(str::to_string)
                    .collect(),
            ),
            None => (false, HashSet::new()),
        };
        let glances = card_glance::read(store, &self.t, &*self.wall, self.unit_system()?, &listed)?;
        // The Tallies on a detailed card, with their chart and entries
        // (#190).
        let mut card_tallies = HashMap::new();
        for id in listed.iter().filter(|id| tallies.contains(*id)) {
            if compact != switched.contains(id) {
                continue;
            }
            let whose = self.tally_whose.get(id);
            if let Some(shown) =
                tally_view::read(store, id, whose, self.unit_system()?, &*self.wall)?
            {
                card_tallies.insert(id.clone(), shown);
            }
        }
        // The Sessions on a detailed card, with their pool, today's picks
        // and last weeks (#191); today's picks as the Agenda shows them,
        // written there, not here.
        let detailed: Vec<&String> = listed
            .iter()
            .filter(|id| sessions.contains(*id) && compact == switched.contains(*id))
            .collect();
        let mut card_sessions = HashMap::new();
        if !detailed.is_empty() {
            let mut today = session_notes.today(&*self.wall)?;
            for id in detailed {
                let Some(setup) = session_notes.setup_of(id)? else {
                    continue;
                };
                let (own, rest) = today.into_iter().partition(|o| o.session.note_id() == *id);
                today = rest;
                let wall: &oldbrainfix_core::WallClock = &*self.wall;
                let shown = (self.unit_system()?, wall);
                let card = SessionCardShown::read(store, &self.t, &setup, own, shown)?;
                card_sessions.insert(id.clone(), card);
            }
        }
        let kept_open = |prefix: &str| -> oldbrainfix_core::Result<HashSet<String>> {
            Ok(store
                .local_settings_with_prefix(prefix)?
                .into_iter()
                .filter(|(_, v)| v == "true")
                .filter_map(|(k, _)| k.strip_prefix(prefix).map(str::to_string))
                .collect())
        };
        let struck_shown = kept_open(STRUCK_SHOWN)?;
        let descriptions_unfolded = kept_open(DESCRIPTION_UNFOLDED)?;
        let (note_routines, newer_routines) = match &detail {
            Some((p, _)) if !p.note.is_trashed() => (
                routines.of_note(&p.note.id)?,
                routines.newer_of_note(&p.note.id)?.len(),
            ),
            _ => (Vec::new(), 0),
        };
        let mut open = Vec::new();
        if let Some((p, _)) = &detail
            && !p.note.is_trashed()
        {
            for o in Alarms::new(store).open_of(&p.note.id, &*self.wall)? {
                if store.may_check_off(&o.routine.id)? {
                    open.push(o);
                }
            }
        }
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
                self.tally_whose.get(&p.note.id),
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
        let (checklist, makes_checklist) = match &detail {
            Some((p, _)) if !p.note.is_trashed() => (
                checklist_view::read(store, &p.note, may.changes, &*self.wall)?,
                lists.may_make(&p.note.id)?,
            ),
            _ => (None, false),
        };
        let (shopping_shown, makes_shopping) = match &detail {
            Some((p, _)) if !p.note.is_trashed() => (
                shopping_view::read(store, &p.note, may.changes, &*self.wall)?,
                shopping.may_make(&p.note.id)?,
            ),
            _ => (None, false),
        };
        let switches_list = match &detail {
            Some((p, _)) => shopping.may_switch(&p.note.id)?,
            None => false,
        };
        let kind = match &detail {
            Some((p, _)) => Some(NoteKind::of(store, &p.note.id)?),
            None => None,
        };
        // A Session Note becomes no list (ADR 0039).
        let makes_checklist = makes_checklist && kind != Some(NoteKind::Session);
        let makes_shopping = makes_shopping && kind != Some(NoteKind::Session);
        // Its setup only while it is a Session: an older one spawned from
        // a Tally stays a Tally (ADR 0039).
        let session_setup = match &detail {
            Some((p, _)) if kind == Some(NoteKind::Session) && !p.note.is_trashed() => {
                session_notes.setup_of(&p.note.id)?
            }
            _ => None,
        };
        let makes_session = match &detail {
            Some((p, _)) => may.changes && !p.note.is_trashed() && !p.note.is_merged(),
            None => false,
        };
        let mut removable = HashSet::new();
        let mut text_removable = HashSet::new();
        let mut remakeable = HashSet::new();
        let mut text_changeable = HashSet::new();
        let mut captured_removable = HashSet::new();
        if let Some((p, sections)) = &detail
            && !p.note.is_trashed()
        {
            let ids = std::iter::once(&p.note.id).chain(sections.iter().map(|s| &s.id));
            for id in ids {
                for detail in CapturedDetail::ALL {
                    if notes.may_remove_captured(id, detail)? {
                        captured_removable.insert((id.clone(), detail));
                    }
                }
            }
            if may.changes && p.note.text_editable {
                text_changeable.insert(p.note.id.clone());
            }
            for section in sections {
                if notes.may_edit_section(&section.id)? {
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
                // The rows' Notes that are Checklists, read in one pass so
                // only their rows read their Lines for the Occurrence.
                let row_notes: Vec<String> = agenda
                    .today_rows
                    .iter()
                    .chain(&agenda.tomorrow_rows)
                    .chain(agenda.later_rows.iter().flatten())
                    .filter_map(|row| row.note.as_ref().map(|n| n.id.clone()))
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect();
                let list_notes: HashSet<String> = if row_notes.is_empty() {
                    HashSet::new()
                } else {
                    lists.of_notes(&row_notes)?.into_keys().collect()
                };
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
                    // A Session's picks win over Lines.
                    let mut lines = None;
                    if session.is_none()
                        && row
                            .note
                            .as_ref()
                            .is_some_and(|n| list_notes.contains(&n.id))
                    {
                        let on = RoutineOccurrence {
                            routine_id: row.routine.id.clone(),
                            occurrence: row.occurrence,
                        };
                        if let Some(list) = lists.for_occurrence(&on)?
                            && list.count() > 0
                        {
                            lines = Some((list, store.may_check_off(&row.routine.id)?));
                        }
                    }
                    Ok(Row {
                        row,
                        area,
                        session,
                        lines,
                    })
                };
                // What shows as Done leaves where this Device hides it
                // (#217).
                let hides_checked = DeviceSettings::new(store).agenda_hides_checked()?;
                let rows = |rows: Vec<AgendaRow>| -> oldbrainfix_core::Result<Vec<Row>> {
                    let mut shown = Vec::new();
                    for placed in rows.into_iter().map(row) {
                        let placed = placed?;
                        if !hides_checked || !shown_done(&placed) {
                            shown.push(placed);
                        }
                    }
                    Ok(shown)
                };
                let mut later = Vec::new();
                for (i, day) in agenda.later_rows.iter().enumerate() {
                    later.push((agenda.later(i), rows(day.clone())?));
                }
                // Read as the Agenda shows, never while it is drawn:
                // today's pick is written for the user's own Devices when
                // it is new, and reading again writes nothing.
                let today = frogs.pick_today(wall)?;
                let frog_rows = shown_frogs(today.shown.clone())?;
                let mut shopping_rows = Vec::new();
                for shown in shopping.agenda_rows(wall)? {
                    let Some(note) = notes.by_id(&shown.list.note_id)? else {
                        continue;
                    };
                    let title = match row_title(&self.t, Some(&note)).lines().next() {
                        Some(first) if !first.trim().is_empty() => first.trim().to_string(),
                        _ => self.t.shopping_heading().to_string(),
                    };
                    // As this person chose for the list (#186); one kept
                    // off is not among them.
                    let count = shown.lines.len() as i64;
                    let (lines, text) = match shown.shows {
                        ShoppingOnAgenda::Lines => (shown.lines, title),
                        ShoppingOnAgenda::Count => {
                            (Vec::new(), self.t.shopping_row_open(&title, count))
                        }
                        ShoppingOnAgenda::Off => continue,
                    };
                    shopping_rows.push(ShoppingRow {
                        area: place(note.clone())?.area,
                        checks: shopping.may_check(&note.id)?,
                        note_id: note.id,
                        text,
                        lines,
                    });
                }
                let mut checklist_rows = Vec::new();
                for row in lists.agenda_rows()? {
                    let Some(note) = notes.by_id(&row.list.note_id)? else {
                        continue;
                    };
                    let title = match row_title(&self.t, Some(&note)).lines().next() {
                        Some(first) if !first.trim().is_empty() => first.trim().to_string(),
                        _ => self.t.checklist_heading().to_string(),
                    };
                    // As this person chose for the list (#219): every
                    // open Line, or none, which leaves the count.
                    let ChecklistAgendaRow { list, shows } = row;
                    let lines = match shows {
                        ListOnAgenda::Count => Vec::new(),
                        // Its checked Lines never show (#143).
                        _ => agenda_lines(&list, true),
                    };
                    let text = if lines.is_empty() {
                        let (open, count) = (list.open_count() as i64, list.count() as i64);
                        self.t.checklist_row_open(&title, open, count)
                    } else {
                        title
                    };
                    checklist_rows.push(ChecklistRow {
                        area: place(note.clone())?.area,
                        text,
                        lines,
                        checks: lists.may_check(&note.id)?,
                        note_id: note.id,
                    });
                }
                Some(AgendaView {
                    later,
                    shopping: shopping_rows,
                    checklists: checklist_rows,
                    today: rows(agenda.today_rows)?,
                    tomorrow: rows(agenda.tomorrow_rows)?,
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
                    hides_checked,
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
        let mut all_routines = Vec::new();
        if self.selection == Selection::AllRoutines {
            let all = routines.all()?;
            let names = all_routines_view::names(&self.t, &routines, &all)?;
            let name = |r: &Routine| names.get(&r.id).cloned().unwrap_or_default();
            for routine in routines_by_name(all, name) {
                let name = names.get(&routine.id).cloned().unwrap_or_default();
                let area = match routines.note(&routine)? {
                    Some(note) => place(note)?.area,
                    None => None,
                };
                all_routines.push((routine, name, area));
            }
        }
        let mut all_newer = Vec::new();
        if self.selection == Selection::AllRoutines {
            for routine in routines.newer()? {
                if let Some(note) = routines.note_of_newer(&routine)? {
                    let name = all_routines_view::routine_name(&self.t, Some(&note));
                    let area = place(note.clone())?.area;
                    all_newer.push(all_routines_view::NewerRow {
                        id: routine.id,
                        note: note.id,
                        name,
                        area,
                    });
                }
            }
        }
        let history = match &self.selection {
            Selection::History(id) => self.history_shown(id)?,
            _ => None,
        };
        let (frogs_open, frogs_done) = if self.selection == Selection::Frogs {
            (shown_frogs(frogs.open()?)?, shown_frogs(frogs.done()?)?)
        } else {
            (Vec::new(), Vec::new())
        };
        let people_shown = self.store.people_shown()?;
        let mut people_newer = std::collections::HashSet::new();
        for person in &people_shown {
            if self.store.person_is_newer(&person.id)?
                || self.store.override_is_newer(&person.id)?
            {
                people_newer.insert(person.id.clone());
            }
        }
        Ok(View {
            areas,
            access,
            people,
            names,
            shown: shown_people,
            people_shown,
            people_newer,
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
            captured_removable,
            notes: placed,
            table_ticking: self.table_ticking(&box_looks, &box_ticks)?,
            sections,
            checklists,
            shopping_lists,
            tallies,
            card_tallies,
            sessions,
            card_sessions,
            card_changes,
            box_looks,
            box_ticks,
            show_ticks,
            compact,
            switched,
            glances,
            struck_shown,
            descriptions_unfolded,
            detail,
            capture_spawns: capture_choices::may_spawn(
                &self.store,
                self.capture_place().as_deref(),
            )?,
            list_manages: capture_choices::may_spawn(&self.store, self.list_place().as_deref())?,
            capture_lists: capture_choices::shopping_lists(&self.store, &self.t)?,
            capture_quick: QuickNote::read(&self.store)?,
            routines: note_routines,
            open,
            card_today,
            newer_routines,
            note_frogs,
            tally,
            checklist,
            makes_checklist,
            shopping: shopping_shown,
            makes_shopping,
            switches_list,
            kind,
            session_setup,
            makes_session,
            frogs_open,
            all_routines,
            all_newer,
            history,
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
            profile_newer: OwnProfile::new(&self.store).is_newer()?,
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
        // The session runs exactly while the Sync or Pair dialog shows,
        // or the first start's adding of a Device of one's own.
        let adding = self.welcome.as_ref().is_some_and(|w| w.adding);
        let shown = self
            .pages
            .iter()
            .any(|p| matches!(p, Page::Sync | Page::Pair));
        if (shown || adding) != self.sync.hosting() {
            if self.sync.hosting() {
                self.sync.stop();
            } else {
                self.sync.start(&self.store);
            }
        }

        self.refresh();
        match done {
            Ok(()) => {}
            // Waiting, not failed: said with what to do.
            Err(oldbrainfix_core::Error::SyncPaused) => {
                self.notice = Some(self.t.sync_paused_error().to_string());
            }
            Err(e) => self.error = Some(e.to_string()),
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
                // The place shows, not the Note over it (#146).
                self.leave_note()?;
                self.go(selection);
            }
            Action::OpenPage(page) => {
                self.save_text()?;
                self.pages = vec![page];
            }
            Action::PageInto(page) => self.pages.push(page),
            Action::PageBack => {
                self.pages.pop();
            }
            Action::ClosePage => self.pages.clear(),
            Action::NewNote => {
                // One dialog at a time: the Note open is kept and closed.
                self.leave_note()?;
                self.quick_open = true;
                self.capture_focus = true;
            }
            Action::CloseQuickNote => {
                // Leaving keeps what is typed, as on the phone; a failed
                // save leaves it open, so nothing is lost.
                if !self.capture.trim().is_empty() || self.recording() || self.holds_media() {
                    self.act(Action::Capture(Notes::default_nudge()))?;
                }
                self.quick_open = false;
                self.capture_focus = false;
            }
            Action::CloseNote => self.leave_note()?,
            Action::Badge(open) => self.badge = open,
            Action::Add(kind) => self.add(kind)?,
            Action::CreateNamed => self.create_named()?,
            Action::TallyPage(intent) => self.tally_page_action(intent)?,
            Action::QuickNote => self.quick_note_wanted = true,
            Action::Quit => self.quit_wanted = true,
            Action::Find => {
                self.leave_note()?;
                // All Notes, an Area and a Topic have the search.
                if !matches!(self.selection, Selection::All | Selection::Place(_)) {
                    self.go(Selection::All);
                }
                self.search_focus = true;
            }
            Action::Clip(clip) => self.clip = Some(clip),
            Action::OpenGuide => self.open_url = Some(about_view::guide_url(self.t.locale())),
            Action::SyncNow => {
                self.save_text()?;
                self.pages = vec![Page::Sync];
                if self.store.sync_folder()?.is_some() {
                    self.folder_now()?;
                }
            }
            Action::AskFileInto(notes) => self.dialog = Some(Dialog::FileInto { notes }),
            Action::PickCard(id, pick) => {
                let order = self.shown_cards();
                match pick {
                    Pick::One => self.cards.click(&id),
                    Pick::Toggle => self.cards.toggle(&id),
                    Pick::Run => self.cards.extend(&id, &order),
                }
            }
            Action::ClearCards => self.cards.clear(),
            Action::AskImport => self.dialog = Some(Dialog::Import),
            Action::SelectNote(id) => {
                // One dialog at a time: the quick note is kept and closed.
                if self.quick_open {
                    self.act(Action::CloseQuickNote)?;
                }
                if self.selected_note.as_deref() != Some(id.as_str()) {
                    self.leave_note()?;
                }
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
                // Saved, the quick note's dialog goes (#146).
                self.quick_open = false;
                self.capture_focus = false;
            }
            Action::CaptureToEditor => {
                if let Some(r) = &mut self.recorder
                    && r.recording()
                    && let Some(bytes) = r.stop()
                {
                    self.recorded.push(bytes);
                }
                // The quick note goes and the Note dialog opens on the Note
                // saved, its text taking the keyboard (#142, #146).
                if let Some(id) = self.capture_note(Notes::default_nudge(), &[], &[])? {
                    self.quick_open = false;
                    self.capture_focus = false;
                    self.selected_note = Some(id.clone());
                    self.edit_wanted = Some(id);
                }
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
                    let t = self.t;
                    self.announce_refusal(t.microphone_missing(), "record: no microphone");
                    return Ok(());
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
            Action::PickFiles => {
                let files = (self.files_picker)(self.t.from_file_title())
                    .into_iter()
                    .filter_map(|path| {
                        let bytes = std::fs::read(&path).ok()?;
                        Some((file_name(&path), bytes))
                    })
                    .collect();
                self.hold_files(files);
            }
            Action::Files(files) => {
                // They wait in the quick note, with what is typed (#146).
                self.hold_files(files);
                if self.holds_media() && !self.quick_open {
                    self.act(Action::NewNote)?;
                }
            }
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
            Action::SearchFor(query) => {
                // All Notes' search, whatever list shows: the tag is
                // found in every Note holding it (ADR 0032), the Note it
                // was clicked in closed.
                self.leave_note()?;
                if self.selection != Selection::All {
                    self.act(Action::Select(Selection::All))?;
                }
                self.query = query;
            }
            Action::FilterPeople(people) => self.people = people,
            Action::ViewPhoto(hash) => self.viewer = Some(Viewer::new(hash)),
            Action::ClosePhoto => self.viewer = None,
            Action::PhotoOut(hash, out) => self.photo_out(&hash, out),
            Action::OpenTable(opening) => self.open_table(opening)?,
            Action::Table(asked) => self.table_asked(asked)?,
            Action::Fold(setting, open) => {
                // Nothing kept means folded, the default.
                if open {
                    self.store.set_local_setting(&setting, "true")?;
                } else {
                    self.store.remove_local_setting(&setting)?;
                }
            }
            Action::SwitchView(id) => {
                if let Some(key) = self.cards_key() {
                    let mut switched = self.view.switched.clone();
                    if !switched.remove(&id) {
                        switched.insert(id);
                    }
                    let setting = switched_setting(&key);
                    if switched.is_empty() {
                        self.store.remove_local_setting(&setting)?;
                    } else {
                        let mut ids: Vec<&str> = switched.iter().map(String::as_str).collect();
                        ids.sort_unstable();
                        self.store.set_local_setting(&setting, &ids.join(","))?;
                    }
                }
            }
            Action::Theme(choice) => {
                // Shown even when it cannot be kept.
                self.theme = choice;
                self.theme_shown = false;
                DeviceSettings::new(&self.store).set_theme(choice)?;
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
                    // Every card shows that view again.
                    self.store.remove_local_setting(&switched_setting(&key))?;
                }
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
            Action::FormatText(None) => self.save_formatted()?,
            Action::FormatText(Some(section)) => self.save_section_formatted(&section)?,
            Action::SetTextFormat(format) => self.set_text_format(format)?,
            Action::OfferCode(offer) => {
                self.code_offer = offer.then(|| self.text_of.clone()).flatten()
            }
            Action::AskLink {
                start,
                end,
                url,
                amend,
                section,
            } => {
                self.dialog = Some(Dialog::Link {
                    linked: url.is_some(),
                    address: url.unwrap_or_default(),
                    start,
                    end,
                    amend,
                    section,
                });
                self.focus = true;
            }
            Action::SetLink => {
                if let Some(Dialog::Link {
                    address,
                    start,
                    end,
                    amend: true,
                    ..
                }) = &self.dialog
                {
                    // Into the Amend field, kept with the Section.
                    let url = format_bar::address(address);
                    let linked = markdown::link(&self.amend, *start, *end, &url);
                    self.amend = linked.text;
                    self.amend_select = Some((linked.start, linked.end));
                    self.amend_focus = true;
                    self.dialog = None;
                } else if let Some(Dialog::Link {
                    address,
                    start,
                    end,
                    section: Some(section),
                    ..
                }) = &self.dialog
                {
                    // Into the Section's field, kept at once (#183).
                    let section = section.clone();
                    let url = format_bar::address(address);
                    if let Some(field) = self.section_texts.get_mut(&section) {
                        let linked = markdown::link(&field.text, *start, *end, &url);
                        field.text = linked.text;
                        field.select = Some((linked.start, linked.end));
                        field.editing = true;
                        field.focus = true;
                    }
                    self.dialog = None;
                    self.save_section_formatted(&section)?;
                } else if let Some(Dialog::Link {
                    address,
                    start,
                    end,
                    ..
                }) = self.dialog.take()
                {
                    let url = format_bar::address(&address);
                    let linked = markdown::link(&self.text, start, end, &url);
                    self.text = linked.text;
                    self.text_select = Some((linked.start, linked.end));
                    self.text_editing = true;
                    self.text_focus = true;
                    self.save_formatted()?;
                }
            }
            Action::Amend => {
                // Typed or pasted with a format, it is a formatted text.
                let format = if markdown::uses(&self.amend) {
                    text_format::MARKDOWN
                } else {
                    text_format::PLAIN
                };
                notes.amend_as(&selected, &self.amend, format)?;
                self.amend.clear();
            }
            Action::AmendTable(at) => {
                let opening = match markdown::table::at(&self.amend, at) {
                    Some(table) => Opening {
                        note: Some(selected),
                        text: self.amend.clone(),
                        at: table.start,
                        cell: None,
                        tags: true,
                        boxes: None,
                    },
                    None => {
                        let change = markdown::table::insert(&self.amend, at);
                        self.amend = change.text;
                        Opening {
                            note: Some(selected),
                            text: self.amend.clone(),
                            at: change.start,
                            cell: Some((0, 0)),
                            tags: true,
                            boxes: None,
                        }
                    }
                };
                // Edited there, it is written back into the field.
                self.table = Some(TableView::new(opening));
                self.table_amends = true;
            }
            Action::Importance(importance) => notes.set_importance(&selected, importance)?,
            Action::File { notes: ids, place } => self.file_notes(&ids, &place)?,
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
                self.save_text()?;
                if let Some(to) = self.nav.back(self.selection.clone()) {
                    self.arrive(to);
                }
            }
            Action::Forward => {
                self.save_text()?;
                if let Some(to) = self.nav.forward(self.selection.clone()) {
                    self.arrive(to);
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
                    self.check_off(&routine, &occurrence)?;
                }
            }
            Action::CheckMissed(routine, occurrence) => self.check_off(&routine, &occurrence)?,
            Action::DoneAndTrash(routine, occurrence) => {
                self.leave_note()?;
                let routines = Routines::new(&self.store);
                routines.mark_done(&routine, &occurrence, None)?;
                // The Reminder goes with it, and comes back with it.
                routines.trash_note(&selected, true)?;
            }
            Action::Seen => Alarms::new(&self.store).mark_seen()?,
            Action::ExportRoutines => {
                let all: Vec<Routine> = self
                    .view
                    .all_routines
                    .iter()
                    .map(|(routine, ..)| routine.clone())
                    .collect();
                self.save_calendar_file(&all)?;
            }
            Action::ShareRoutine(id) => {
                if let Some(routine) = Routines::new(&self.store).by_id(&id)? {
                    self.save_calendar_file(&[routine])?;
                }
            }
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
                    // A Routine of a Session Note only schedules it: the
                    // setup lies on the Note, on its page (#191).
                    self.editor = Some(RoutineEditor::edit(&routine, note, today, renag));
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
                // A Routine of a Session Note runs it already (#191).
                if let Some(note) = &id
                    && Sessions::new(&self.store).is_session_note(note)?
                {
                    return self.act(Action::NewRoutine {
                        note: id,
                        reminder: false,
                    });
                }
                self.save_text()?;
                let note = match &id {
                    Some(id) => Notes::new(&self.store).by_id(id)?,
                    None => None,
                };
                let renag = DeviceSettings::new(&self.store).renag_interval()?;
                // Without a Note yet, any of the user's Tallies.
                let entity = id.as_deref().unwrap_or("");
                let draft = SessionDraft::of(&self.store, &self.t, entity, None)?;
                self.tally_form = None;
                let editor = RoutineEditor::new(note, false, self.store.now(), &*self.wall, renag);
                self.editor = Some(editor.with_session(draft));
            }
            Action::Session(asked) => self.session_action(asked)?,
            Action::MakeSession(id) => {
                self.save_text()?;
                let Some(note) = Notes::new(&self.store).by_id(&id)? else {
                    return Ok(());
                };
                let draft = SessionDraft::of(&self.store, &self.t, &id, None)?;
                let renag = DeviceSettings::new(&self.store).renag_interval()?;
                let now = self.store.now();
                self.tally_form = None;
                self.editor = Some(RoutineEditor::make_session(
                    note,
                    draft,
                    now,
                    &*self.wall,
                    renag,
                ));
            }
            Action::UnmakeSession(id) => {
                // A sync while the dialog was open may have changed it.
                if Sessions::new(&self.store).is_session_note(&id)? {
                    self.save_text()?;
                    Sessions::new(&self.store).unmake(&id)?;
                }
            }
            Action::SessionSetup(changed) => {
                let (draft, setup) = *changed;
                draft.write(&self.store, &setup)?;
            }
            Action::SessionPoolTally => {
                let Some(setup) = &self.view.session_setup else {
                    return Ok(());
                };
                let place = Notes::new(&self.store)
                    .by_id(&setup.note_id)?
                    .and_then(|n| n.place);
                self.save_text()?;
                let system = self.unit_system()?;
                let text = self
                    .session_draft
                    .as_ref()
                    .map(SessionDraft::new_tally_text)
                    .unwrap_or_default();
                self.tally_page = Some(TallyPage::new(place, true, system).with_name(text));
            }
            Action::Editor(intent) => match intent {
                Intent::Save => match &mut self.editor {
                    // Set up for the Note being captured: held with it.
                    Some(editor) if editor.capturing() => {
                        if let Some(choice) = editor.choice() {
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
                // The New Tally page over it, filed where the Session
                // goes; its Tally comes back ticked (#161).
                Intent::Session(SessionIntent::NewTally) => {
                    let place = self.editor.as_ref().and_then(|e| e.place.clone());
                    let system = self.unit_system()?;
                    let text = self
                        .editor
                        .as_ref()
                        .and_then(RoutineEditor::session)
                        .map(SessionDraft::new_tally_text)
                        .unwrap_or_default();
                    self.tally_page = Some(TallyPage::new(place, true, system).with_name(text));
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
                // The History shows in the list, the editor left.
                Intent::History(id) => {
                    self.editor = None;
                    return self.act(Action::Select(Selection::History(id)));
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
            Action::Tally(asked) => self.tally_action(None, asked)?,
            Action::CardTally(note, asked) => self.tally_action(Some(note), asked)?,
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
            Action::Checklist(asked) => self.checklist_action(asked)?,
            Action::Shopping(asked) => self.shopping_action(asked)?,
            Action::Tick { entity, id, to } => {
                // A sync may have taken the box or the role meanwhile.
                let boxes = Boxes::new(&self.store);
                if boxes.may_tick(&entity)? && boxes.of(&entity)?.iter().any(|b| b.id == id) {
                    boxes.tick(&entity, &id, to)?;
                }
            }
            Action::BoxHistory { entity, id } => {
                let now = self.store.now();
                let mut lines = Vec::new();
                for tick in Boxes::new(&self.store).history(&entity, &id)? {
                    let by = self.store.ticked_by(&entity, &tick)?;
                    let name = by
                        .as_ref()
                        .map(|p| profile_view::display_name(&self.t, &p.name));
                    let said = text_marks_view::tick_text(&self.t, &tick, name, (now, true));
                    lines.push((tick.state, said));
                }
                self.dialog = Some(Dialog::BoxHistory { lines });
            }
            Action::NoteHistory(note) => {
                self.save_text()?;
                let rows = note_history_view::rows(&self.store, &self.t, &note)?;
                self.dialog = Some(Dialog::NoteHistory { note, rows });
            }
            Action::UseVersion(at) => {
                let Some(Dialog::NoteHistory { note, rows }) = &self.dialog else {
                    return Ok(());
                };
                let Some(row) = rows.get(at) else {
                    return Ok(());
                };
                let (note, change) = (note.clone(), row.change.clone());
                let history = NoteHistory::new(&self.store);
                // A sync since the dialog opened may have changed it.
                if !history.may_use(&note, &change)? {
                    return Ok(());
                }
                history.use_version(&note, &change)?;
                let now = self
                    .store
                    .current(&change.entity, oldbrainfix_core::notes::keys::TEXT)?;
                let now = oldbrainfix_core::text_format::StoredText::read(now.as_deref()).text;
                if self.text_of.as_deref() == Some(change.entity.as_str()) {
                    self.put_typed(now);
                } else if let Some(field) = self.section_texts.get_mut(&change.entity) {
                    // A Section's field shows the text it has now, or the
                    // next save would put the old one back (#183).
                    field.text = now;
                }
                let rows = note_history_view::rows(&self.store, &self.t, &note)?;
                self.dialog = Some(Dialog::NoteHistory { note, rows });
            }
            Action::ShowTicks { note, on } => {
                self.store
                    .set_local_setting(&show_ticks_key(&note), if on { "true" } else { "false" })?;
            }
            Action::LeaveLines => self.lines.leave(&self.store, &*self.wall)?,
            Action::DeleteLine => {
                if let Some(Dialog::DeleteLine { id, .. }) = self.dialog.take() {
                    if self.line_of(&id)?.is_some() {
                        Checklists::new(&self.store).delete(&id)?;
                    } else if self.shopping_line_of(&id)?.is_some() {
                        ShoppingLists::new(&self.store).delete(&id)?;
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
            Action::Sync(SyncAction::ImportFolder) => {
                // One Import at a time; its button is greyed meanwhile.
                if !self.import.running()
                    && let Some(folder) = (self.import_picker)(self.t.import_folder_title())
                {
                    let name = folder_name(&folder);
                    let area = self.t.import_area_name(&name);
                    self.import.start(self.store.clone(), folder, name, area);
                }
            }
            Action::Sync(SyncAction::ImportKeep) => {
                if !self.import.running() {
                    self.dialog = Some(Dialog::ImportKeep);
                }
            }
            Action::ImportKeep(zip) => {
                self.dialog = None;
                // One Import at a time; its button is greyed meanwhile.
                if !self.import.running() {
                    let t = self.t;
                    let source = if zip {
                        (self.import_zip_picker)(t.import_keep_zip_title()).map(KeepSource::Zip)
                    } else {
                        (self.import_picker)(t.import_keep_folder_title()).map(KeepSource::Folder)
                    };
                    if let Some(source) = source {
                        let area = t.import_keep_area_name().to_string();
                        self.import.start_keep(self.store.clone(), source, area);
                        // The Import dialog says it reads meanwhile.
                        self.dialog = Some(Dialog::Import);
                    }
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
            Action::StartSend(id) => {
                self.save_text()?;
                let recipients = self.store.sent_item_recipients()?;
                let picked = (!recipients.is_empty()).then_some(0);
                let update = self.store.was_sent(&id)?;
                self.dialog = Some(Dialog::Send {
                    note: id,
                    recipients,
                    picked,
                    update,
                    cancel: false,
                });
            }
            Action::SendNote => self.send_note()?,
            Action::CopyAsText(id) => {
                self.save_text()?;
                self.copy_as_text(&id)?;
            }
            Action::SaveAsMarkdown(id) => {
                self.save_text()?;
                self.save_as_markdown(&id)?;
            }
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
                        self.announce_not_taken(OpenError::FromBlocked);
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
                    // Whoever joins with this Device's code gets the Area.
                    self.pages = vec![Page::Pair];
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
            Action::AskRemoveCaptured { id, detail } => {
                self.save_text()?;
                self.dialog = Some(Dialog::RemoveCaptured { id, detail });
            }
            Action::RemoveCaptured => {
                if let Some(Dialog::RemoveCaptured { id, detail }) = self.dialog.take() {
                    self.remove_captured(&id, detail)?;
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
                // As the phone joins them, escaped into a formatted text
                // (ADR 0033).
                let written = notes.take_over(&note, generated, append)?;
                if self.text_of.as_deref() == Some(note.as_str()) {
                    self.put_typed(written);
                } else if let Some(field) = self.section_texts.get_mut(&note) {
                    // A Section's field shows the text it has now, or the
                    // next save would put the old one back (#183).
                    field.text = written;
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
            // Never over another dialog: the notice waits until it is
            // closed.
            Action::ShowNoticeDialog if self.dialog.is_none() => {
                self.dialog = self.dialog_behind_notice().cloned();
                self.notice = None;
                self.notice_dialog = None;
            }
            Action::ShowError if self.dialog.is_none() => {
                if let Some(details) = self.error.take() {
                    let t = self.t;
                    self.dialog = Some(Dialog::Failure {
                        title: t.action_failed_title().into(),
                        cause: t.action_failed_cause().into(),
                        fix: t.action_failed_fix().into(),
                        details,
                    });
                }
            }
            Action::ShowUpdate if self.dialog.is_none() => {
                if let Some(notice) = self.updates.pop_front() {
                    let t = self.t;
                    let fix = if notice.settings {
                        t.settings_update_fix()
                    } else {
                        t.update_fix()
                    };
                    self.dialog = Some(Dialog::Failure {
                        title: t.update_needed_title().into(),
                        cause: notice.cause,
                        fix: fix.into(),
                        details: notice.details,
                    });
                }
            }
            Action::ShowNoticeDialog | Action::ShowError | Action::ShowUpdate => {}
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
    /// pick's entry form opens in its dialog over its Tally's Note,
    /// which shows once it is saved.
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

    /// Does what was asked of a Checklist, on a card or in the editor
    /// (#120).
    fn checklist_action(&mut self, asked: ChecklistAction) -> oldbrainfix_core::Result<()> {
        match asked {
            ChecklistAction::Make(note) => self.change_kind(&note, Kind::Checklist)?,
            ChecklistAction::ToPlain(note) => self.change_kind(&note, Kind::Plain)?,
            ChecklistAction::UncheckAll(note) => {
                let lists = Checklists::new(&self.store);
                if let Some(list) = lists.shown_now(&note, &*self.wall)?
                    && checklist_view::may_check_shown(&self.store, &list)?
                {
                    match &list.on {
                        Some(on) => lists.uncheck_all_for(&note, on)?,
                        None => lists.uncheck_all(&note)?,
                    }
                }
            }
            ChecklistAction::OnAgenda(note, shows) => {
                let lists = Checklists::new(&self.store);
                // A Routine may have come meanwhile.
                if lists.may_show_on_agenda(&note)? {
                    lists.set_on_agenda(&note, shows)?;
                }
            }
            ChecklistAction::Check(line, checked, on) => {
                // A new Line is stored first; an empty one has nothing to
                // check.
                let Some(line) = self.lines.stored_id(&self.store, &*self.wall, &line)? else {
                    return Ok(());
                };
                // A sync meanwhile may have taken it.
                if let Some((list, _)) = self.line_of(&line)? {
                    let lists = Checklists::new(&self.store);
                    let list = Checklist { on, ..list };
                    if checklist_view::may_check_shown(&self.store, &list)? {
                        match (&list.on, checked) {
                            (Some(on), true) => lists.check_for(&line, on)?,
                            (Some(on), false) => lists.uncheck_for(&line, on)?,
                            (None, true) => lists.check(&line)?,
                            (None, false) => lists.uncheck(&line)?,
                        }
                    }
                }
            }
            ChecklistAction::Typed(asked) => self.lines.typing(&self.store, &*self.wall, asked)?,
            ChecklistAction::Step(id, step) => {
                // A new Line is stored first, and what was typed, so the
                // step sees the list as shown (#202).
                let Some(id) = self.lines.stored_id(&self.store, &*self.wall, &id)? else {
                    return Ok(());
                };
                self.lines.keep(&self.store, &*self.wall)?;
                let Some((list, line)) = self.line_of(&id)? else {
                    return Ok(());
                };
                let lists = Checklists::new(&self.store);
                match step {
                    LineStep::Up => checklist_view::move_up(&self.store, &list, &line)?,
                    LineStep::Down => checklist_view::move_down(&self.store, &list, &line)?,
                    LineStep::Indent => lists.indent(&id, true)?,
                    LineStep::Outdent => lists.indent(&id, false)?,
                    LineStep::Delete => {
                        self.dialog = Some(Dialog::DeleteLine {
                            id,
                            for_good: false,
                        });
                    }
                }
            }
            ChecklistAction::Menu(note) => {
                self.card_menu = match Notes::new(&self.store).by_id(&note)? {
                    Some(note) => Some(checklist_view::card_menu(&self.store, &note, &*self.wall)?),
                    None => None,
                };
            }
        }
        Ok(())
    }

    /// Does what was asked of a Shopping List, on a card or in the
    /// editor (#122).
    fn shopping_action(&mut self, asked: ShoppingAction) -> oldbrainfix_core::Result<()> {
        let lists = ShoppingLists::new(&self.store);
        match asked {
            ShoppingAction::Make(note) => self.change_kind(&note, Kind::ShoppingList)?,
            ShoppingAction::ToPlain(note) => self.change_kind(&note, Kind::Plain)?,
            ShoppingAction::Check(line, checked) => {
                // A new Line is stored first; an empty one has nothing to
                // check.
                let Some(line) = self.lines.stored_id(&self.store, &*self.wall, &line)? else {
                    return Ok(());
                };
                // A sync meanwhile may have taken it.
                if let Some(shown) = self.shopping_line_of(&line)?
                    && lists.may_check(&shown.note_id)?
                {
                    if checked {
                        lists.check(&line, &*self.wall)?;
                    } else {
                        lists.uncheck(&line, &*self.wall)?;
                    }
                }
            }
            ShoppingAction::Typed(asked) => self.lines.typing(&self.store, &*self.wall, asked)?,
            ShoppingAction::Step(id, step) => {
                // A new Line is stored first, and what was typed (#202).
                let Some(id) = self.lines.stored_id(&self.store, &*self.wall, &id)? else {
                    return Ok(());
                };
                self.lines.keep(&self.store, &*self.wall)?;
                if self.shopping_line_of(&id)?.is_none() || !lists.may_edit(&id)? {
                    return Ok(());
                }
                match step {
                    ShopStep::PickReappear => {
                        self.reappear_line = Some(id);
                        self.badge = Some(NoteBadge::Reappear);
                    }
                    ShopStep::Reappear(reappear) => lists.set_reappear(&id, reappear)?,
                    ShopStep::Delete => {
                        self.dialog = Some(Dialog::DeleteLine { id, for_good: true });
                    }
                }
            }
            ShoppingAction::OnAgenda(note, shows) => {
                // It may have gone to the trash or back to a plain Note
                // meanwhile.
                if lists.may_show_on_agenda(&note)? {
                    lists.set_on_agenda(&note, shows)?;
                }
            }
        }
        Ok(())
    }

    /// Makes the Note `note` a Checklist or a Shopping List, or a plain
    /// Note again, where this Device may. On the Note shown, what is
    /// typed into its text goes in first, and the field then holds the
    /// text it has now, so that leaving it writes nothing old back.
    fn change_kind(&mut self, note: &str, kind: Kind) -> oldbrainfix_core::Result<()> {
        let shown = self.selected_note.as_deref() == Some(note);
        if shown {
            self.save_text()?;
        }
        let lists = Checklists::new(&self.store);
        let shopping = ShoppingLists::new(&self.store);
        // The other list switches, its Lines carried over (#188).
        let other_list = match kind {
            Kind::Checklist => shopping.is_shopping_list(note)?,
            Kind::ShoppingList => lists.is_checklist(note)?,
            Kind::Plain => false,
        };
        match kind {
            Kind::Checklist | Kind::ShoppingList if other_list => {
                if !shopping.may_switch(note)? {
                    return Ok(());
                }
                shopping.switch_list(note, &*self.wall)?;
                // Its rows start again from the Lines of the new list, what
                // was typed kept above.
                if shown {
                    self.lines = TypedLines::default();
                }
            }
            Kind::Checklist => {
                if !lists.may_make(note)? {
                    return Ok(());
                }
                lists.make(note)?;
            }
            Kind::ShoppingList => {
                if !shopping.may_make(note)? {
                    return Ok(());
                }
                shopping.make(note)?;
            }
            Kind::Plain => {
                if lists.is_checklist(note)? {
                    lists.to_plain(note)?;
                } else if shopping.is_shopping_list(note)?
                    && self.store.may_change(note)?
                    && Notes::new(&self.store)
                        .by_id(note)?
                        .is_some_and(|n| n.text_editable)
                {
                    shopping.to_plain(note, &*self.wall)?;
                } else {
                    return Ok(());
                }
            }
        }
        if shown && let Some(now) = Notes::new(&self.store).by_id(note)? {
            // Of the kind it is now, the view read again only after.
            self.text_kinded = NoteKind::of(&self.store, note)? != NoteKind::Plain;
            self.put_typed(now.text);
        }
        Ok(())
    }

    /// How the boxes of each of `texts` show, by its id (ADR 0035): a
    /// text's id, its boxes, the Note it is of and whether that is in the
    /// trash. Gives them, the texts whose boxes this Device may tick and
    /// the Notes with Show ticks on.
    fn box_looks(&self, texts: &[(&str, &[ShownBox], &str, bool)]) -> BoxLooks {
        let boxes = Boxes::new(&self.store);
        let now = self.store.now();
        let (mut looks, mut ticks, mut shows) = (HashMap::new(), HashSet::new(), HashSet::new());
        for &(entity, shown, note, trashed) in texts {
            if shown.is_empty() || looks.contains_key(entity) {
                continue;
            }
            let show = self.store.local_setting(&show_ticks_key(note))?.as_deref() == Some("true");
            if show {
                shows.insert(note.to_string());
            }
            if !trashed && boxes.may_tick(entity)? {
                ticks.insert(entity.to_string());
            }
            let mut look = BoxLook::of(shown);
            if show {
                for b in shown {
                    let Some(last) = &b.last else {
                        continue;
                    };
                    let by = self.store.ticked_by(entity, last)?;
                    let name = by
                        .as_ref()
                        .map(|p| profile_view::display_name(&self.t, &p.name));
                    let said = text_marks_view::tick_text(&self.t, last, name, (now, false));
                    look.after.insert(b.id.clone(), said);
                }
            }
            looks.insert(entity.to_string(), look);
        }
        Ok((looks, ticks, shows))
    }

    /// How the boxes of the table shown full screen show, while it is one
    /// of a stored text: read anew when it is no text `looks` holds.
    fn table_ticking(
        &self,
        looks: &HashMap<String, BoxLook>,
        ticks: &HashSet<String>,
    ) -> oldbrainfix_core::Result<Option<table_view::Ticking>> {
        let Some(entity) = self.table.as_ref().and_then(TableView::boxes) else {
            return Ok(None);
        };
        let boxes = Boxes::new(&self.store);
        let shown = boxes.of(entity)?;
        let look = match looks.get(entity) {
            Some(look) => look.clone(),
            None => BoxLook::of(&shown),
        };
        let may_tick = if looks.contains_key(entity) {
            ticks.contains(entity)
        } else {
            boxes.may_tick(entity)?
        };
        Ok(Some(table_view::Ticking {
            look,
            may_tick,
            ticked: shown
                .iter()
                .filter(|b| b.last.is_some())
                .map(|b| b.id.clone())
                .collect(),
        }))
    }

    /// The stored text and its format of the Note or Section `id`
    /// (#183) where this Device may write it as it is stored now: not in
    /// the trash, in a format known here, its to change.
    fn writable_text(&self, id: &str) -> oldbrainfix_core::Result<Option<(String, String)>> {
        let notes = Notes::new(&self.store);
        // A merged Note is a Section of the one it went into.
        if let Some(section) = notes.section_by_id(id)? {
            return Ok(Self::section_writable(&notes, &section)?
                .then_some((section.text, section.text_format)));
        }
        Ok(match notes.by_id(id)? {
            Some(now) if !now.is_trashed() && now.text_editable && self.store.may_change(id)? => {
                Some((now.text, now.text_format))
            }
            _ => None,
        })
    }

    /// Shows the table `opening` says full screen (#128). A table of a
    /// Note or Section (#183) is edited there only where this Device may
    /// change its text as it is stored now; what is typed into the Note
    /// shown goes in first, and the view holds the text as stored, at the
    /// same table.
    fn open_table(&mut self, mut opening: Opening) -> oldbrainfix_core::Result<()> {
        self.table_amends = false;
        if let Some(note) = opening.note.take() {
            if self.text_of.as_deref() == Some(note.as_str())
                || self.section_texts.contains_key(&note)
            {
                self.save_text()?;
            }
            if let Some((now, format)) = self.writable_text(&note)?
                && format == text_format::MARKDOWN
                && let Some(at) = same_table((&opening.text, opening.at), &now)
            {
                opening.note = Some(note);
                opening.text = now;
                opening.at = at;
            } else {
                opening.cell = None;
            }
        }
        if markdown::table::at(&opening.text, opening.at).is_some() {
            self.table = Some(TableView::new(opening));
        }
        Ok(())
    }

    /// Does what the table shown full screen asked (#128).
    fn table_asked(&mut self, asked: table_view::Asked) -> oldbrainfix_core::Result<()> {
        match asked {
            table_view::Asked::Write(text) => self.write_table(&text)?,
            table_view::Asked::Close => {
                // The cell being edited is written as it is left.
                if let Some(text) = self.table.as_mut().and_then(TableView::commit) {
                    self.write_table(&text)?;
                }
                self.table = None;
                // The Amend field has the keyboard again.
                if std::mem::take(&mut self.table_amends) {
                    self.amend_focus = true;
                }
            }
            table_view::Asked::SearchFor(tag) => {
                self.table_asked(table_view::Asked::Close)?;
                self.act(Action::SearchFor(tag))?;
            }
            table_view::Asked::Tick(id, to) => {
                if let Some(entity) = self.table.as_ref().and_then(TableView::boxes) {
                    let entity = entity.to_string();
                    self.act(Action::Tick { entity, id, to })?;
                }
            }
            table_view::Asked::History(id) => {
                if let Some(entity) = self.table.as_ref().and_then(TableView::boxes) {
                    let entity = entity.to_string();
                    self.act(Action::BoxHistory { entity, id })?;
                }
            }
        }
        Ok(())
    }

    /// Keeps `text`, its table edited full screen, as the text of the
    /// Note or Section (#183) the table is in, where this Device may
    /// change it as it is stored now; a text left blank is not kept. The
    /// field of the Note or Section shown then holds it.
    fn write_table(&mut self, text: &str) -> oldbrainfix_core::Result<()> {
        // A table of the Amend field goes back into it, kept with the
        // Section.
        if self.table_amends && self.table.is_some() {
            self.amend = text.to_string();
            return Ok(());
        }
        let Some(note) = self
            .table
            .as_ref()
            .and_then(TableView::note)
            .map(str::to_string)
        else {
            return Ok(());
        };
        let Some((now, _)) = self.writable_text(&note)? else {
            return Ok(());
        };
        let text = text.trim();
        if text.is_empty() || text == now {
            return Ok(());
        }
        let notes = Notes::new(&self.store);
        let section = notes.section_by_id(&note)?.is_some();
        // As stored, its new boxes with their ids (ADR 0035), here and in
        // the view, so the next write keeps them.
        let stored = if section {
            notes.edit_section_as(&note, text, text_format::MARKDOWN)?;
            notes.section_by_id(&note)?.map(|s| s.text)
        } else {
            notes.edit_as(&note, text, text_format::MARKDOWN)?;
            notes.by_id(&note)?.map(|n| n.text)
        };
        let Some(stored) = stored else {
            return Ok(());
        };
        if let Some(table) = &mut self.table
            && let Some(at) = same_table((table.text(), table.at()), &stored)
        {
            table.take_stored(stored.clone(), at);
        }
        if self.text_of.as_deref() == Some(note.as_str()) {
            self.put_typed(stored);
        } else if let Some(field) = self.section_texts.get_mut(&note) {
            field.text = stored;
        }
        Ok(())
    }

    /// The Line `id` of a Shopping List, while it shows there (#122).
    fn shopping_line_of(&self, id: &str) -> oldbrainfix_core::Result<Option<ShoppingLine>> {
        let Some(note) = self.store.current(id, checklist_keys::NOTE)? else {
            return Ok(None);
        };
        let list = ShoppingLists::new(&self.store).by_note(&note, &*self.wall)?;
        Ok(list.and_then(|list| list.line(id).cloned()))
    }

    /// The Line `id` with its Checklist, while it shows in one.
    fn line_of(&self, id: &str) -> oldbrainfix_core::Result<Option<(Checklist, Line)>> {
        let Some(note) = self.store.current(id, checklist_keys::NOTE)? else {
            return Ok(None);
        };
        let Some(list) = Checklists::new(&self.store).by_note(&note)? else {
            return Ok(None);
        };
        let line = list.line(id).cloned();
        Ok(line.map(|line| (list, line)))
    }

    /// Does what was asked of the Tally of the Note `on`, of its card,
    /// or of the selected Note's setup (#103, #190).
    fn tally_action(
        &mut self,
        on: Option<String>,
        asked: TallyAction,
    ) -> oldbrainfix_core::Result<()> {
        let tallies = Tallies::new(&self.store);
        let system = self.unit_system()?;
        let note = on
            .or_else(|| self.selected_note.clone())
            .unwrap_or_default();
        match asked {
            TallyAction::Make(id) => {
                self.save_text()?;
                self.editor = None;
                self.tally_form =
                    Some(TallyForm::Field(FieldForm::new(&id, None, system, &self.t)));
            }
            TallyAction::OpenField(field) => {
                let tally = tallies.by_note(&note)?;
                let field = match (field, &tally) {
                    (Some(id), Some(tally)) => tally.field(&id).cloned(),
                    _ => None,
                };
                self.save_text()?;
                self.editor = None;
                let form = FieldForm::new(&note, field, system, &self.t);
                let form = match &tally {
                    Some(tally) => form.multiplying(tally),
                    None => form,
                };
                self.tally_form = Some(TallyForm::Field(form));
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
            TallyAction::Whose(whose) => {
                self.tally_whose.insert(note, whose);
            }
            TallyAction::Period(period) => {
                let setting = tally_view::period_setting(&note);
                if period == oldbrainfix_core::TallyPeriod::Day {
                    self.store.remove_local_setting(&setting)?;
                } else {
                    self.store
                        .set_local_setting(&setting, tally_view::period_name(period))?;
                }
            }
            TallyAction::AllShown(shown) => {
                let setting = tally_view::entries_shown_setting(&note);
                if shown {
                    self.store.set_local_setting(&setting, "true")?;
                } else {
                    self.store.remove_local_setting(&setting)?;
                }
            }
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

    /// Captures what the quick note holds; the Note it kept, if it kept
    /// one.
    fn capture_note(
        &mut self,
        nudge_in: Duration,
        photos: &[Vec<u8>],
        recordings: &[Vec<u8>],
    ) -> oldbrainfix_core::Result<Option<String>> {
        let place = self.capture_place();
        // The photos and recordings held on the field go into the same
        // Note.
        let mut photos = photos.to_vec();
        photos.extend(self.held_photos.iter().cloned());
        let photos = photos.as_slice();
        let mut recordings = recordings.to_vec();
        recordings.extend(self.recorded.iter().cloned());
        let held = Held {
            photos,
            recordings: &recordings,
            transcribes: self.transcribes(),
        };
        let (captured, said) = capture_choices::save(
            (&self.store, &self.wall),
            &self.capture,
            place.as_deref(),
            nudge_in,
            held,
            &self.capture_choices,
            (self.transcriber.as_ref(), self.t),
        )?;
        // Kept as a Note instead of going into the list: said at the top.
        if let Some((said, details)) = said {
            self.announce_done(said, details);
        }
        let shown = match captured {
            Captured::Nothing => return Ok(None),
            Captured::Note(note) | Captured::Kept(note) => Some(note),
            Captured::Poured | Captured::Waiting(_) => None,
        };
        // Only once all of it is written: a failed save keeps it here.
        self.capture.clear();
        self.recorded.clear();
        self.held_photos.clear();
        // Added to what still waits to be clicked, so none goes unsaid.
        self.untranscribable.append(&mut self.held_problems);
        self.capture_choices = CaptureChoices::default();
        let Some(note) = shown else {
            return Ok(None);
        };
        Ok(Some(note.id))
    }

    /// Pastes into the capture field `id` (#130): the clipboard's text,
    /// or `text` when Ctrl+V brought it, at its cursor, and its picture
    /// held for the Note, as files are; with neither, the grey line says
    /// so.
    fn paste(&mut self, ui: &Ui, id: Id, text: Option<String>) {
        let Some(clipboard) = self.clipboard.as_deref_mut() else {
            return;
        };
        let pasted = paste::take(clipboard, text);
        self.paste_empty = pasted.is_empty();
        if let Some(text) = &pasted.text {
            paste::insert(ui, id, &mut self.capture, text);
        }
        if let Some(photo) = pasted.photo {
            self.hold_photo(photo);
        }
    }

    /// Holds `photo` on the capture field for its Note. No Line holds a
    /// photo, so a Shopping List picked is dropped (UI laws, "Capture").
    fn hold_photo(&mut self, photo: Vec<u8>) {
        capture_choices::hold_photo(&mut self.held_photos, &mut self.capture_choices, photo);
    }

    /// Whether photos or recordings wait on the capture field.
    fn holds_media(&self) -> bool {
        !self.recorded.is_empty() || !self.held_photos.is_empty()
    }

    /// Whether a recording captured now gets its Transcript here: with
    /// the thread making them and a speech model in use.
    fn transcribes(&self) -> bool {
        self.transcriber.is_some() && self.speech.as_ref().is_none_or(|s| s.current().is_some())
    }

    /// The Shopping Lists the Note being captured can go into: none while
    /// it holds a photo or a recording that gets no Transcript here.
    fn capture_lists(&self) -> &[ListTitle] {
        let voice = self.recording() || !self.recorded.is_empty();
        let photo = !self.held_photos.is_empty();
        capture_choices::shopping_offer(&self.view.capture_lists, photo, voice, self.transcribes())
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
    /// Routine, Reminder, Session or Tally opens its page in a dialog over
    /// the quick note, which hands its choice back to the capture (#113).
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
                    editor = editor.with_session(draft);
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

    /// The user's Done on `occurrence` of the Routine `routine`, from the
    /// Agenda, a card or the Note dialog: its Note may go to the trash
    /// with it by the person's setting (#211), and the dialog showing it
    /// closes first, as for "Done and move to trash".
    fn check_off(
        &mut self,
        routine: &str,
        occurrence: &Occurrence,
    ) -> oldbrainfix_core::Result<()> {
        let trashes = Routines::new(&self.store).trashes_when_done(routine)?;
        if let Some(note) = trashes
            && self.selected_note.as_deref() == Some(note.as_str())
        {
            self.leave_note()?;
        }
        Routines::new(&self.store).check_off(routine, occurrence, None)?;
        Ok(())
    }

    /// Closes the Note dialog, keeping what was typed into it: its text,
    /// its Lines and a Section typed into Amend (#146).
    fn leave_note(&mut self) -> oldbrainfix_core::Result<()> {
        if self.selected_note.is_none() {
            return Ok(());
        }
        self.save_text()?;
        if !self.amend.trim().is_empty() && self.view.may.amends {
            self.act(Action::Amend)?;
        }
        self.lines.leave(&self.store, &*self.wall)?;
        self.selected_note = None;
        self.badge = None;
        self.start_lines = false;
        Ok(())
    }

    /// Where what "+" makes goes: the Area or Topic the list shows, none
    /// from All Notes, Unsorted and the Agenda (#161).
    fn list_place(&self) -> Option<String> {
        match &self.selection {
            Selection::Place(id) => Some(id.clone()),
            _ => None,
        }
    }

    /// Makes `kind` as "+" asked, each on its own form: the Note through
    /// the quick note; a Checklist, a Shopping List or a Frog from its
    /// name; a Tally or a Session on its one page; a Topic as the Area
    /// adds one (#161).
    fn add(&mut self, kind: NewKind) -> oldbrainfix_core::Result<()> {
        let place = self.list_place();
        match kind {
            NewKind::Note => self.act(Action::NewNote)?,
            NewKind::Checklist | NewKind::Shopping | NewKind::Frog => {
                self.dialog = Some(Dialog::NewKind {
                    kind,
                    name: String::new(),
                });
                self.focus = true;
            }
            NewKind::Tally => {
                let system = self.unit_system()?;
                self.tally_page = Some(TallyPage::new(place, false, system));
            }
            NewKind::Session => {
                // The Tallies of where it goes; from All Notes, any.
                let entity = place.as_deref().unwrap_or("");
                let draft = SessionDraft::of(&self.store, &self.t, entity, None)?;
                let renag = DeviceSettings::new(&self.store).renag_interval()?;
                let now = self.store.now();
                self.tally_form = None;
                self.editor = Some(RoutineEditor::new_session_page(
                    place,
                    draft,
                    now,
                    &*self.wall,
                    renag,
                ));
            }
            NewKind::Topic => {
                if let Some(area) = place {
                    self.act(Action::NewTopic(area))?;
                }
            }
        }
        Ok(())
    }

    /// Makes the Checklist, Shopping List or Frog the name dialog asked
    /// for, filed where the list is: a list then opens in the Note dialog
    /// with the cursor in its first Line.
    fn create_named(&mut self) -> oldbrainfix_core::Result<()> {
        let Some(Dialog::NewKind { kind, name }) = self.dialog.take() else {
            return Ok(());
        };
        let name = name.trim().to_string();
        if name.is_empty() {
            self.dialog = Some(Dialog::NewKind { kind, name });
            return Ok(());
        }
        let place = self.list_place();
        let store = &*self.store;
        let made = store.batch(|| {
            let note = Notes::new(store).capture(&name, place.as_deref(), None)?;
            match kind {
                NewKind::Checklist => Checklists::new(store).make(&note.id)?,
                NewKind::Shopping => ShoppingLists::new(store).make(&note.id)?,
                _ => {
                    Frogs::new(store).spawn(&note.id)?;
                }
            }
            Ok(note.id)
        })?;
        if matches!(kind, NewKind::Checklist | NewKind::Shopping) {
            self.act(Action::SelectNote(made))?;
            self.start_lines = true;
        }
        Ok(())
    }

    /// Does what was asked of the New Tally page: Save makes the Tally
    /// and hands it to the New Session page it was opened from, ticked,
    /// or puts it first in the pool of the Session Note whose page it was
    /// opened from (#191).
    fn tally_page_action(&mut self, intent: PageIntent) -> oldbrainfix_core::Result<()> {
        let Some(page) = &mut self.tally_page else {
            return Ok(());
        };
        match intent {
            PageIntent::Close => self.tally_page = None,
            PageIntent::Save => {
                let Some(id) = page.save(&self.store)? else {
                    return Ok(());
                };
                if page.for_session {
                    match (&mut self.editor, &self.view.session_setup) {
                        (Some(editor), _) => editor.add_made_tally(&id, page.name()),
                        // From a Session Note's page: first in its pool.
                        (None, Some(setup)) => {
                            let mut pool = vec![id.clone()];
                            pool.extend(setup.pool.iter().filter(|p| **p != id).cloned());
                            Sessions::new(&self.store).set_pool(&setup.note_id, &pool)?;
                        }
                        (None, None) => {}
                    }
                }
                self.tally_page = None;
            }
        }
        Ok(())
    }

    /// Holds `files`, by name and content, on the capture field for one
    /// Note with its text, as a share does on the phone: the pictures as
    /// photos, the sound files as voice recordings, each kept even when
    /// it cannot be transcribed here, which is announced at the top once
    /// the Note is saved. Anything else is left alone.
    fn hold_files(&mut self, files: Vec<(String, Vec<u8>)>) {
        for (name, bytes) in files {
            match from_file(&name, &bytes) {
                Some(FromFile::Photo(photo)) => self.hold_photo(photo),
                Some(FromFile::Recording { bytes, problem }) => {
                    if let Some(problem) = problem {
                        self.held_problems
                            .push((problem, problem_line(problem, &name)));
                    }
                    self.recorded.push(bytes);
                }
                None => {}
            }
        }
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
                    let t = self.t;
                    self.announce_failure(
                        t.autostart_failed(),
                        t.autostart_failed_title(),
                        (t.autostart_failed_cause(), t.autostart_failed_fix()),
                        format!("autostart: {e}"),
                    );
                }
            }
            Setting::Renag(minutes) => settings.set_renag_interval(Duration::minutes(minutes))?,
            Setting::NudgeRepeat(minutes) => {
                settings.set_nudge_repeat(Duration::minutes(minutes))?;
            }
            Setting::QuietOn(on) => settings.set_quiet_hours_on(on)?,
            Setting::Quiet(quiet) => settings.set_quiet_hours(quiet)?,
            Setting::AllDayFrom(from) => settings.set_all_day_from(from)?,
            Setting::Hotkey(chord) => {
                settings.set_capture_hotkey(&chord)?;
                if let (Some(hotkey), Ok(chord)) = (&self.hotkey, chord.parse::<Chord>()) {
                    hotkey.borrow_mut().rebind(&chord);
                }
            }
            Setting::AlarmSound(sound) => settings.set_alarm_sound(sound.as_deref())?,
            Setting::Units(system) => settings.set_chosen_unit_system(system)?,
            Setting::Streaks(on) => settings.set_streaks_on(on)?,
            Setting::HideChecked(on) => settings.set_agenda_hides_checked(on)?,
            Setting::TrashDoneReminder(on) => {
                PersonSettings::new(&self.store).set_trashes_done_reminder(on)?;
            }
            Setting::QuickNote(control, on) => settings.set_quick_note_shows(control, on)?,
            Setting::QuickNoteNudgeButtons(on) => settings.set_quick_note_nudge_buttons_on(on)?,
        }
        Ok(())
    }

    /// Keeps the edited text of the shown Note when it changed, then
    /// each of its Sections' (#183); an emptied text goes back to what it
    /// was.
    fn save_text(&mut self) -> oldbrainfix_core::Result<()> {
        // What was typed into its Lines first, by anyone who may add them
        // (#141).
        self.lines.keep(&self.store, &*self.wall)?;
        self.save_note_text()?;
        self.save_sections()
    }

    /// Keeps the edited text of each Section of the shown Note whose
    /// field changed (#183), fixed in place as the Note's is, where this
    /// Device may edit it as it is stored now; an emptied text goes back
    /// to what it was. Each field then holds the text as it was kept, its
    /// new boxes with their ids (ADR 0035).
    fn save_sections(&mut self) -> oldbrainfix_core::Result<()> {
        let notes = Notes::new(&self.store);
        for (id, field) in &mut self.section_texts {
            let Some(section) = notes.section_by_id(id)? else {
                continue;
            };
            let text = field.text.trim();
            if text.is_empty() {
                field.text = section.text;
                continue;
            }
            if text == section.text || !Self::section_writable(&notes, &section)? {
                continue;
            }
            notes.edit_section(id, text)?;
            if let Some(now) = notes.section_by_id(id)? {
                field.text = now.text;
            }
        }
        Ok(())
    }

    /// Whether this Device may write the text of `section` now: its
    /// Note is not in the trash and [`Notes::may_edit_section`] (#183).
    fn section_writable(notes: &Notes<'_>, section: &Section) -> oldbrainfix_core::Result<bool> {
        let trashed = notes
            .by_id(&section.note_id)?
            .is_none_or(|n| n.is_trashed());
        Ok(!trashed && notes.may_edit_section(&section.id)?)
    }

    /// Keeps the Section `id`'s edited text as a formatted text, at once,
    /// as its first format is applied (#183, ADR 0033), as
    /// [`App::save_formatted`] keeps the Note's.
    fn save_section_formatted(&mut self, id: &str) -> oldbrainfix_core::Result<()> {
        let notes = Notes::new(&self.store);
        let Some(section) = notes.section_by_id(id)? else {
            return Ok(());
        };
        let Some(field) = self.section_texts.get_mut(id) else {
            return Ok(());
        };
        let text = field.text.trim();
        if text.is_empty() || !Self::section_writable(&notes, &section)? {
            return Ok(());
        }
        if section.text_format != text_format::MARKDOWN || text != section.text {
            notes.edit_section_as(id, text, text_format::MARKDOWN)?;
            if let Some(now) = notes.section_by_id(id)?
                && now.text != text
            {
                let typed = field.text.clone();
                let before = &typed[..typed.len() - typed.trim_start().len()];
                let after = &typed[typed.trim_end().len()..];
                field.text = format!("{before}{}{after}", now.text);
            }
        }
        Ok(())
    }

    /// The shown Note's text as typed: of a Note of a kind its name and
    /// its description joined (#188).
    fn typed_text(&self) -> String {
        if self.text_kinded {
            note_kinds::join(&self.name, &self.text)
        } else {
            self.text.clone()
        }
    }

    /// Puts the text `whole` into the field of the shown Note, of a Note
    /// of a kind split into its name and its description (#188).
    fn put_typed(&mut self, whole: String) {
        if self.text_kinded {
            self.name = note_kinds::name(&whole).to_string();
            self.text = note_kinds::description(&whole).to_string();
        } else {
            self.name.clear();
            self.text = whole;
        }
    }

    /// Whether the name and the description typed are those of `stored`
    /// (#188): leaving them writes nothing, the text as stored kept.
    fn kinded_unchanged(&self, stored: &str) -> bool {
        self.text_kinded
            && self.name == note_kinds::name(stored)
            && self.text == note_kinds::description(stored)
    }

    /// Keeps the edited text of the shown Note when it changed; an
    /// emptied text goes back to what it was, and so does an emptied
    /// name of a Note of a kind (#188).
    fn save_note_text(&mut self) -> oldbrainfix_core::Result<()> {
        let Some((placed, _)) = &self.view.detail else {
            return Ok(());
        };
        let note = &placed.note;
        // A text in a format not known here is never overwritten.
        if self.text_of.as_deref() != Some(note.id.as_str())
            || note.is_trashed()
            || !note.text_editable
            || self.kinded_unchanged(&note.text)
        {
            return Ok(());
        }
        if self.text_kinded && self.name.trim().is_empty() {
            self.name = note_kinds::name(&note.text).to_string();
        }
        let typed = self.typed_text();
        let text = typed.trim();
        if text.is_empty() {
            let stored = note.text.clone();
            self.put_typed(stored);
        } else if text != note.text {
            let notes = Notes::new(&self.store);
            notes.edit(&note.id, text)?;
            // As stored: its new boxes with their ids (ADR 0035), or the
            // next save gives them new ones and their ticks are lost.
            let stored = match notes.by_id(&note.id)? {
                Some(now) => now.text,
                None => text.to_string(),
            };
            self.put_typed(stored);
        }
        Ok(())
    }

    /// Puts the shown Note's text into `format` (#213), what is typed kept
    /// first: a plain or formatted text becomes code and back, a formatted
    /// one never plain ([`Notes::set_format`]); the field then holds it as
    /// stored.
    fn set_text_format(&mut self, format: &str) -> oldbrainfix_core::Result<()> {
        self.save_text()?;
        self.code_offer = None;
        let Some(id) = self.text_of.clone() else {
            return Ok(());
        };
        let notes = Notes::new(&self.store);
        notes.set_format(&id, format)?;
        if let Some(now) = notes.by_id(&id)? {
            self.put_typed(now.text);
        }
        Ok(())
    }

    /// Keeps the selected Note's edited text as a formatted text, at
    /// once, as its first format is applied (ADR 0033); the field keeps
    /// what is typed as it is, spaces at its end included.
    fn save_formatted(&mut self) -> oldbrainfix_core::Result<()> {
        let Some((placed, _)) = &self.view.detail else {
            return Ok(());
        };
        let note = &placed.note;
        if self.text_of.as_deref() != Some(note.id.as_str())
            || note.is_trashed()
            || !note.text_editable
        {
            return Ok(());
        }
        if self.text_kinded && self.name.trim().is_empty() {
            self.name = note_kinds::name(&note.text).to_string();
        }
        let typed = self.typed_text();
        let text = typed.trim();
        if text.is_empty() {
            return Ok(());
        }
        if note.text_format != text_format::MARKDOWN || text != note.text {
            let notes = Notes::new(&self.store);
            notes.edit_as(&note.id, text, text_format::MARKDOWN)?;
            // As stored, its new boxes with their ids, what is typed
            // around it kept.
            if let Some(now) = notes.by_id(&note.id)?
                && now.text != text
            {
                let before = &typed[..typed.len() - typed.trim_start().len()];
                let after = &typed[typed.trim_end().len()..];
                let kept = format!("{before}{}{after}", now.text);
                self.put_typed(kept);
            }
        }
        Ok(())
    }

    /// Shows `to` in the list, back leading to the place it leaves
    /// (#145).
    fn go(&mut self, to: Selection) {
        self.nav.went(self.selection.clone(), &to);
        self.arrive(to);
    }

    /// Shows `to` in the list; every level has a search and a person
    /// filter of its own, as on the phone.
    fn arrive(&mut self, to: Selection) {
        if to != self.selection {
            self.query.clear();
            self.people = PersonFilter::default();
            // The cards selected were those of the list left (#148).
            self.cards.clear();
        }
        self.selection = to;
    }

    /// The way up to the place the list shows, as the path above it shows
    /// it (#145): each level with where a click on it leads, the place
    /// itself last, leading nowhere.
    fn path(&self) -> Vec<(String, Option<Selection>)> {
        let t = &self.t;
        let agenda = || (t.agenda_title().to_string(), Some(Selection::Agenda));
        let here = |name: &str| (name.to_string(), None);
        match &self.selection {
            Selection::Agenda => vec![here(t.agenda_title())],
            Selection::All => vec![here(t.all_notes_title())],
            Selection::Unsorted => vec![here(t.unsorted())],
            Selection::Frogs => vec![here(t.frogs_title())],
            Selection::Trash => vec![here(t.trash_title())],
            Selection::AllRoutines => vec![agenda(), here(t.all_routines_title())],
            Selection::History(_) => vec![
                agenda(),
                (
                    t.all_routines_title().to_string(),
                    Some(Selection::AllRoutines),
                ),
                here(t.history_title()),
            ],
            Selection::SentItemsWaiting => vec![agenda(), here(t.sent_items_waiting_title())],
            Selection::AreaOffersWaiting => vec![agenda(), here(t.area_offers_waiting_title())],
            Selection::AiReview => vec![agenda(), here(t.ai_review_title())],
            Selection::Place(id) => {
                // The Areas live in the sidebar; their Notes in All Notes.
                let mut path = vec![(t.areas_title().to_string(), Some(Selection::All))];
                for (area, topics) in &self.view.areas {
                    if area.id == *id {
                        path.push(here(&area_name(t, area)));
                    } else if let Some(topic) = topics.iter().find(|p| p.id == *id) {
                        path.push((area_name(t, area), Some(Selection::Place(area.id.clone()))));
                        path.push(here(&topic_name(t, topic)));
                    }
                }
                path
            }
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

    /// The first start's view instead of the panes: the three choices,
    /// the Starter Set, a Backup being restored, or a Device of one's own
    /// being added with
    /// the Sync dialogs' ways to pair. Back, as key or mouse button, goes
    /// to the choices.
    fn welcome_view(&mut self, ui: &mut Ui, tones: Tones, actions: Vec<Action>) {
        let (t, wall) = (self.t, self.wall.clone());
        let mut asked = Vec::new();
        let mut synced = Vec::new();
        if actions.contains(&Action::Back) {
            asked.push(WelcomeAction::Back);
        }
        // What the earlier part of the frame decided about a join, and
        // a click on the announcement at the top.
        let actions = actions.into_iter().filter(|a| {
            matches!(
                a,
                Action::Welcome(_)
                    | Action::ShowNoticeDialog
                    | Action::ShowError
                    | Action::ShowUpdate
            )
        });
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
                                        && theme::primary(ui, true, t.host_continue()).clicked()
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
        // What the announcement at the top opened shows over it too.
        let mut closed = Vec::new();
        if matches!(
            self.dialog,
            Some(Dialog::Failure { .. } | Dialog::Done { .. } | Dialog::Imported { .. })
        ) {
            self.dialog(ui.ctx(), tones, &mut closed);
        }
        let all: Vec<Action> = actions
            .chain(synced)
            .chain(asked.into_iter().map(Action::Welcome))
            .chain(closed)
            .collect();
        for action in all {
            self.apply(action);
        }
    }

    /// Draws one frame into `ui` and does what was asked in it.
    pub fn show(&mut self, ui: &mut Ui) {
        self.show_theme(ui.ctx());
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
        // A voice capture went into its Shopping List, or stayed a Note.
        let said = self.transcriber.as_ref().map(Enricher::take_said);
        if let Some(said) = said.filter(|said| !said.is_empty()) {
            for (said, details) in said {
                self.announce_done(said, details);
            }
            self.refresh();
        }
        // A phone's requests are answered while the Sync dialogs show.
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
            if port.answer_waiting(&self.store, &*self.wall) > 0 {
                self.refresh();
            }
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(100));
        }
        // A typed code being joined, also after leaving the Sync dialogs: its
        // answer is taken in when it comes.
        if self.sync.joining() {
            match self.sync.joined(&self.store, &self.t) {
                Some(_) if self.welcome.is_some() => {
                    // The first start goes on once synced, else back to
                    // its choices, saying why at the top.
                    if self.sync.synced() {
                        actions.push(Action::Welcome(WelcomeAction::Synced));
                    } else {
                        if let Some(failure) = self.sync.take_join_failure() {
                            self.announce_join_failed(failure);
                        }
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
        // An Import reading its folder, also after its dialog closed:
        // its answer is announced when it comes.
        if self.import.running() {
            match self.import.finished() {
                Some((kind, imported)) => {
                    // What it made is announced at the top, clickable once
                    // the Import dialog is gone.
                    if self.dialog == Some(Dialog::Import) {
                        self.dialog = None;
                    }
                    let made = imported.as_ref().is_ok_and(|s| s.area_id.is_some());
                    match kind {
                        Importing::Folder(name) => self.announce_folder_imported(&name, imported),
                        Importing::Keep => self.announce_keep_imported(imported),
                    }
                    if made {
                        self.refresh();
                        if let Some(wake) = &self.wake {
                            drop(wake.send(Event::Wake));
                        }
                    }
                    ui.ctx().request_repaint();
                }
                None => ui
                    .ctx()
                    .request_repaint_after(std::time::Duration::from_millis(100)),
            }
        }
        // Hosting that could not start, or a session that choked, is
        // announced once, its full text behind the click.
        if let Some((started, details)) = self.sync.take_error() {
            let t = self.t;
            let why = if started {
                (t.sync_host_failed_cause(), t.sync_host_failed_fix())
            } else {
                (t.sync_broke_off_cause(), t.nothing_lost_fix())
            };
            self.announce_failure(t.sync_not_done(), t.sync_not_done_title(), why, details);
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
        // Note; a bundle file is imported, a Sent Item previewed, a
        // Profile card file opened, as are the files handed over from
        // outside.
        let dropped = ui.input(|i| i.raw.dropped_files.clone());
        let (files, dropped): (Vec<_>, Vec<_>) = dropped.into_iter().partition(|file| {
            file.path.as_ref().is_some_and(|p| {
                p.extension().is_some_and(|e| {
                    e == bundle::EXTENSION || e == sent::EXTENSION || e == cards::EXTENSION
                })
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
            .unsorted_wanted
            .as_ref()
            .is_some_and(|wanted| wanted.swap(false, Ordering::SeqCst))
        {
            actions.push(Action::Select(Selection::Unsorted));
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
        if self.viewer.is_none() && self.table.is_some() {
            // Back closes the table shown full screen, the cell being
            // edited written; Alt+Left rests while a cell is typed into.
            let typing = ui.ctx().text_edit_focused();
            let back = ui.input_mut(|i| {
                i.pointer.button_pressed(PointerButton::Extra1)
                    || (!typing && i.consume_key(Modifiers::ALT, Key::ArrowLeft))
            });
            if back {
                actions.push(Action::Table(table_view::Asked::Close));
            }
        } else if self.viewer.is_some() {
            // Back closes the photo shown full screen, and goes nowhere
            // else.
            let back = ui.input_mut(|i| {
                i.pointer.button_pressed(PointerButton::Extra1)
                    | i.consume_key(Modifiers::ALT, Key::ArrowLeft)
            });
            if back {
                actions.push(Action::ClosePhoto);
            }
        } else if self.dialog.is_none() && self.badge.is_none() && self.top_dialog().is_some() {
            // Back leads from a dialog opened from another to that one,
            // and closes the first; forward goes nowhere there.
            let typing = ui.ctx().text_edit_focused();
            let back = ui.input_mut(|i| {
                i.pointer.button_pressed(PointerButton::Extra1)
                    || (!typing && i.consume_key(Modifiers::ALT, Key::ArrowLeft))
            });
            if let Some(top) = self.top_dialog().filter(|_| back) {
                actions.push(top.back());
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
        // The menu bar's keys, over the window only; they are no text
        // keys, so they work while typing too.
        // Over the Note dialog they work, its Note the Note menu's.
        let over = self.viewer.is_some()
            || self.table.is_some()
            || self.dialog.is_some()
            || self.badge.is_some()
            || self.top_dialog().is_some_and(|top| top != TopDialog::Note);
        if self.welcome.is_none() && !over {
            let filing = self.file_into_notes();
            // Esc over the list drops the cards selected (#148); over a
            // dialog or menu it is theirs, in a field the field's.
            let clears = self.top_dialog().is_none()
                && !self.cards.is_empty()
                && !ui.ctx().text_edit_focused()
                && !egui::Popup::is_any_open(ui.ctx());
            ui.input_mut(|i| {
                if clears && i.consume_key(Modifiers::NONE, Key::Escape) {
                    actions.push(Action::ClearCards);
                }
                if i.consume_key(Modifiers::COMMAND, Key::N) {
                    actions.push(Action::NewNote);
                }
                if i.consume_key(Modifiers::COMMAND, Key::F) {
                    actions.push(Action::Find);
                }
                if i.consume_key(Modifiers::COMMAND, Key::Comma) {
                    actions.push(Action::OpenPage(Page::Settings));
                }
                if let Some(notes) = filing.filter(|_| i.consume_key(Modifiers::COMMAND, Key::M)) {
                    actions.push(Action::AskFileInto(notes));
                }
            });
        }
        // The field the cursor is in, for Edit's Cut, Copy and Paste,
        // which take the cursor out of it.
        if ui.ctx().text_edit_focused() {
            self.last_field = ui.ctx().memory(|m| m.focused());
        }
        background::paint(
            ui.painter(),
            ui.max_rect(),
            tones.background,
            tones.mark,
            self.scrolled * background::PARALLAX,
        );
        // The menu bar tops the window, over the announcements (#145).
        if self.welcome.is_none() {
            egui::Panel::top("menu bar")
                .frame(
                    Frame::NONE
                        .inner_margin(Margin::symmetric(8, 4))
                        .fill(tones.card),
                )
                .show(ui, |ui| self.menu_bar(ui, &mut actions));
        }
        // What the Note dialog lies over: all but the menu bar, which
        // stays at hand for its Note (#146).
        let below_menu = ui.available_rect_before_wrap();
        // Announcements go to the top (UI laws): of the window, or of the
        // dialog over it, where they can be clicked.
        if self.announces_in() == Announcer::Window {
            self.announcements(ui, tones, &mut actions);
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
        egui::CentralPanel::default()
            .frame(Frame::NONE.inner_margin(Margin::same(12)))
            .show(ui, |ui| {
                self.nav_row(ui, &mut actions);
                self.notes_pane(ui, tones, &mut actions);
            });
        // The dialogs of what were views, under the dialogs they open.
        self.page_dialog(ui.ctx(), tones, &mut actions);
        // The Note open (#146), and the quick note, under the pages and
        // dialogs they open.
        self.note_dialog(ui.ctx(), below_menu, tones, &mut actions);
        self.quick_note_dialog(ui.ctx(), tones, &mut actions);
        self.editor_dialog(ui.ctx(), tones, &mut actions);
        self.tally_page_dialog(ui.ctx(), tones, &mut actions);
        if !matches!(self.dialog, Some(Dialog::BoxHistory { .. })) {
            self.dialog(ui.ctx(), tones, &mut actions);
        }
        // A table full screen over the dialog it may be opened from.
        if let Some(table) = &mut self.table {
            if let Some(ticking) = &self.view.table_ticking {
                table.set_ticking(ticking.clone());
            }
            let asked = table.show(ui.ctx(), &self.t, tones);
            actions.extend(asked.into_iter().map(Action::Table));
        }
        // A box's History over the table it may be opened from.
        if matches!(self.dialog, Some(Dialog::BoxHistory { .. })) {
            self.dialog(ui.ctx(), tones, &mut actions);
        }
        // The photo full screen, over everything else.
        if let Some(viewer) = &mut self.viewer {
            match media_view::photo_viewer(
                ui.ctx(),
                &self.t,
                (&mut self.photos, &self.store),
                viewer,
            ) {
                Some(Viewed::Close) => actions.push(Action::ClosePhoto),
                Some(Viewed::Out(out)) => actions.push(Action::PhotoOut(viewer.hash.clone(), out)),
                None => {}
            }
        }
        self.dragged(ui.ctx(), tones);
        for action in actions {
            self.apply(action);
        }
        self.show_theme(ui.ctx());
        if let Some(text) = self.copy.take() {
            ui.ctx().copy_text(text);
        }
        // Edit's Cut, Copy and Paste: the cursor back into its field, and
        // the system's clipboard asked as the keys ask it.
        if let Some(clip) = self.clip.take()
            && let Some(field) = self.last_field
        {
            ui.ctx().memory_mut(|m| m.request_focus(field));
            ui.ctx().send_viewport_cmd(match clip {
                Clip::Cut => egui::ViewportCommand::RequestCut,
                Clip::Copy => egui::ViewportCommand::RequestCopy,
                Clip::Paste => egui::ViewportCommand::RequestPaste,
            });
        }
        if let Some(url) = self.open_url.take() {
            match &self.url_opener {
                Some(opener) => opener(&url),
                None => ui.ctx().open_url(egui::OpenUrl::new_tab(url)),
            }
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
                    (Selection::Unsorted, icons::UNSORTED, t.unsorted()),
                    (Selection::All, icons::ALL_NOTES, t.all_notes_title()),
                    (Selection::Frogs, icons::FROG, t.frogs_title()),
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
                                // Also a pick by keyboard or screen reader.
                                ui.close();
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
                                    ui.close();
                                }
                            }
                        });
                    }
                }
                ui.add_space(12.0);
                let on = self.selection == Selection::Trash;
                let trash = Some((icons::OPEN_TRASH, text));
                if row(ui, tones, on, 0.0, trash, t.trash_title(), false).clicked() {
                    actions.push(Action::Select(Selection::Trash));
                }
            });
    }

    /// The one announcement at the top (UI laws, "Announcements"), the
    /// most pressing first: of the window, or of the dialog shown over it
    /// (#145).
    fn announcements(&mut self, ui: &mut Ui, tones: Tones, actions: &mut Vec<Action>) {
        if self.error.is_some() {
            // Announcements go to the top (UI laws), without the error's
            // text: the click shows it, with the cause and the fix.
            egui::Panel::top("announcement")
                .frame(Frame::NONE.inner_margin(Margin::same(12)).fill(tones.card))
                .show(ui, |ui| {
                    let colour = ui.visuals().error_fg_color;
                    let text = RichText::new(self.t.action_failed()).color(colour);
                    let label = egui::Label::new(text).sense(egui::Sense::click());
                    if ui.add(label).clicked() {
                        actions.push(Action::ShowError);
                    }
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
            // What a job did, until clicked away; the click opens the
            // details, a failure's why.
            let behind = self.dialog_behind_notice().is_some();
            egui::Panel::top("announcement")
                .frame(Frame::NONE.inner_margin(Margin::same(12)).fill(tones.card))
                .show(ui, |ui| {
                    let colour = ui.visuals().text_color();
                    let label = egui::Label::new(RichText::new(notice).color(colour))
                        .sense(egui::Sense::click());
                    let mut response = ui.add(label);
                    if !behind {
                        response = response.on_hover_text(self.t.close());
                    }
                    if response.clicked() {
                        if behind {
                            actions.push(Action::ShowNoticeDialog);
                        } else {
                            self.notice = None;
                        }
                    }
                });
        } else if !self.updates.is_empty() {
            // A sync that needs an update, until clicked; the click shows
            // which Device.
            egui::Panel::top("announcement")
                .frame(Frame::NONE.inner_margin(Margin::same(12)).fill(tones.card))
                .show(ui, |ui| {
                    let colour = ui.visuals().text_color();
                    let headline = if self.updates.front().is_some_and(|n| n.settings) {
                        self.t.settings_update_needed()
                    } else {
                        self.t.update_needed()
                    };
                    let text = RichText::new(headline).color(colour);
                    let label = egui::Label::new(text).sense(egui::Sense::click());
                    if ui.add(label).clicked() {
                        actions.push(Action::ShowUpdate);
                    }
                });
        } else if self.arrived > 0 && !self.pages.contains(&Page::Sync) {
            // One at a time: Conflicts from an automatic sync, a click
            // away from the Sync dialog that lists them.
            let text = self.t.sync_conflicts_arrived(self.arrived as i64);
            egui::Panel::top("announcement")
                .frame(Frame::NONE.inner_margin(Margin::same(12)).fill(tones.card))
                .show(ui, |ui| {
                    let colour = ui.visuals().text_color();
                    let label = egui::Label::new(RichText::new(text).strong().color(colour))
                        .sense(egui::Sense::click());
                    if ui.add(label).clicked() {
                        actions.push(Action::OpenPage(Page::Sync));
                    }
                });
        } else if self.view.sync_paused {
            // Sync waits for the AI Task, open or waiting for its review,
            // for as long as it does, below every announcement that goes
            // by a click; the click leads to where the wait ends.
            let (text, asked) = if self.ai_port.is_some() {
                (
                    self.t.ai_port_open_banner(),
                    Action::OpenPage(Page::Settings),
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
    }

    /// What the Note menu may do to the Note in the Note dialog (#145,
    /// #146), as the dialog's buttons may; none while no Note is open, or
    /// one in the trash.
    fn note_menu(&self) -> Option<NoteMenu> {
        let (placed, _) = self.view.detail.as_ref()?;
        let note = &placed.note;
        if note.is_trashed() {
            return None;
        }
        let may = self.view.may;
        let frog_open = self.view.note_frogs.iter().any(|(f, _)| !f.is_done());
        // A Code text becomes no Tally or Session (#213).
        let code = note.text_format == text_format::CODE;
        let plain = self.view.kind == Some(NoteKind::Plain) && !note.is_merged() && !code;
        Some(NoteMenu {
            id: note.id.clone(),
            changes: may.changes,
            schedules: may.schedules,
            frog: may.schedules && !frog_open,
            // A Routine of a Session Note runs it already (#191).
            session: may.schedules && self.view.kind == Some(NoteKind::Plain) && !code,
            amends: may.amends,
            checklist: may.amends && self.view.makes_checklist,
            shopping: may.amends && self.view.makes_shopping,
            tally: may.amends && may.changes && plain,
        })
    }

    /// The Note Ctrl+M files: the one shown, where it may be filed.
    fn note_menu_note(&self) -> Option<String> {
        self.note_menu().filter(|n| n.changes).map(|n| n.id)
    }

    /// The Notes Note ▸ File into… and Ctrl+M file (#148): the one in the
    /// Note dialog, where it may be filed, else the cards selected in the
    /// list, in its order; none in the trash, whose Notes are restored.
    fn file_into_notes(&self) -> Option<Vec<String>> {
        if self.note_open() {
            return self.note_menu_note().map(|id| vec![id]);
        }
        if self.selection == Selection::Trash || self.cards.is_empty() {
            return None;
        }
        Some(self.cards.in_order(&self.shown_cards()))
    }

    /// The ids of the Notes the list shows, in its order.
    fn shown_cards(&self) -> Vec<String> {
        self.view.notes.iter().map(|p| p.note.id.clone()).collect()
    }

    /// Files the Notes `ids` into the Area or Topic `place` in one go
    /// (#148), as File into… and a drop on the sidebar ask, Unsorted ones
    /// leaving Unsorted. One this Device may not move stays: another
    /// person's in a Shared Area where it is a Member, one in the trash,
    /// and every one where this Device may not write in `place`, such as
    /// an Area it was taken out of (ADR 0013); how many stayed is told.
    fn file_notes(&mut self, ids: &[String], place: &str) -> oldbrainfix_core::Result<()> {
        let notes = Notes::new(&self.store);
        let into = self.store.may_write_in(place)?;
        let mut moving = Vec::new();
        for id in ids {
            let trashed = notes.by_id(id)?.is_none_or(|n| n.is_trashed());
            if into && !trashed && self.store.may_change(id)? {
                moving.push(id.as_str());
            }
        }
        self.store
            .batch(|| moving.iter().try_for_each(|id| notes.file(id, place)))?;
        self.cards.clear();
        let stayed = ids.len() - moving.len();
        let t = self.t;
        if stayed == 0 {
            return Ok(());
        }
        if ids.len() == 1 {
            self.announce_refusal(t.only_managers(), "file: not a Manager of the Area");
            return Ok(());
        }
        let details = t.file_some_stayed_details(moving.len() as i64, stayed as i64);
        self.announce_failure(
            t.file_some_stayed(),
            t.file_some_stayed_title(),
            (t.file_some_stayed_cause(), t.file_some_stayed_fix()),
            details,
        );
        Ok(())
    }

    /// The menu bar (#145): File, Edit, View, Note, Sync and Help, each
    /// entry with its keys where it has some, the Note menu's greyed
    /// while no Note is open, but File into… for the cards selected
    /// (#148). A pick closes its menu, also one by keyboard or screen
    /// reader.
    fn menu_bar(&self, ui: &mut Ui, actions: &mut Vec<Action>) {
        let t = self.t;
        let text = ui.visuals().text_color();
        let ctx = ui.ctx().clone();
        // As the system writes them: Ctrl+N, or ⌘N on macOS; the comma as
        // a comma.
        let keys = |key: Key| {
            let shortcut = egui::KeyboardShortcut::new(Modifiers::COMMAND, key);
            Some(ctx.format_shortcut(&shortcut).replace("Comma", ","))
        };
        let note = self.note_menu();
        let id = note.as_ref().map(|n| n.id.clone()).unwrap_or_default();
        // File into… also files the cards selected in the list (#148).
        let filing = self.file_into_notes();
        let may = |f: fn(&NoteMenu) -> bool| note.as_ref().is_some_and(f);
        let field = self.last_field.is_some();
        // Full or compact where the list has cards.
        let cards = self.cards_key().is_some();
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button(RichText::new(t.menu_file()).color(text), |ui| {
                let items = [
                    (t.menu_new_note(), keys(Key::N), true, Action::NewNote),
                    (t.settings_quick_note(), None, true, Action::QuickNote),
                    (t.menu_import(), None, true, Action::AskImport),
                    (
                        t.save_as_markdown(),
                        None,
                        may(|n| n.amends),
                        Action::SaveAsMarkdown(id.clone()),
                    ),
                ];
                menu_entries(ui, items, actions);
                ui.separator();
                menu_entries(ui, [(t.tray_quit(), None, true, Action::Quit)], actions);
            });
            ui.menu_button(RichText::new(t.menu_edit()).color(text), |ui| {
                let items = [
                    (t.menu_cut(), None, field, Action::Clip(Clip::Cut)),
                    (t.menu_copy(), None, field, Action::Clip(Clip::Copy)),
                    (t.paste(), None, field, Action::Clip(Clip::Paste)),
                    (t.menu_find(), keys(Key::F), true, Action::Find),
                ];
                menu_entries(ui, items, actions);
                ui.separator();
                let settings = Action::OpenPage(Page::Settings);
                menu_entries(
                    ui,
                    [(t.menu_settings(), keys(Key::Comma), true, settings)],
                    actions,
                );
            });
            ui.menu_button(RichText::new(t.menu_view()).color(text), |ui| {
                let places = [
                    (t.agenda_title(), Selection::Agenda),
                    (t.unsorted(), Selection::Unsorted),
                    (t.all_notes_title(), Selection::All),
                    (t.frogs_title(), Selection::Frogs),
                    (t.all_routines_title(), Selection::AllRoutines),
                ];
                for (label, place) in places {
                    let on = self.selection == place;
                    let button = egui::Button::selectable(on, RichText::new(label).color(text));
                    if ui.add(button).clicked() {
                        actions.push(Action::Select(place));
                        ui.close();
                    }
                }
                ui.separator();
                let compact = self.view.compact;
                for (label, wanted) in [
                    (t.menu_detailed_views(), false),
                    (t.menu_compact_views(), true),
                ] {
                    let on = cards && compact == wanted;
                    let button = egui::Button::selectable(on, RichText::new(label).color(text));
                    if ui.add_enabled(cards, button).clicked() {
                        actions.push(Action::Compact(wanted));
                        ui.close();
                    }
                }
                ui.separator();
                // The system's light or dark, or one of them (#147).
                ui.menu_button(RichText::new(t.menu_theme()).color(text), |ui| {
                    for (label, choice) in [
                        (t.theme_system(), ThemeChoice::System),
                        (t.theme_light(), ThemeChoice::Light),
                        (t.theme_dark(), ThemeChoice::Dark),
                    ] {
                        let on = self.theme == choice;
                        let button = egui::Button::selectable(on, RichText::new(label).color(text));
                        if ui.add(button).clicked() {
                            actions.push(Action::Theme(choice));
                            ui.close();
                        }
                    }
                });
            });
            ui.menu_button(RichText::new(t.menu_note()).color(text), |ui| {
                let items = [
                    (
                        t.file_into(),
                        keys(Key::M),
                        filing.is_some(),
                        Action::AskFileInto(filing.clone().unwrap_or_default()),
                    ),
                    (
                        t.menu_routine(),
                        None,
                        may(|n| n.schedules),
                        Action::NewRoutine {
                            note: Some(id.clone()),
                            reminder: false,
                        },
                    ),
                    (
                        t.menu_reminder(),
                        None,
                        may(|n| n.schedules),
                        Action::NewRoutine {
                            note: Some(id.clone()),
                            reminder: true,
                        },
                    ),
                    (
                        t.frog(),
                        None,
                        may(|n| n.frog),
                        Action::SpawnFrog(id.clone()),
                    ),
                    (
                        t.session(),
                        None,
                        may(|n| n.session),
                        Action::NewSession(Some(id.clone())),
                    ),
                ];
                menu_entries(ui, items, actions);
                ui.separator();
                let items = [
                    (
                        t.make_checklist(),
                        None,
                        may(|n| n.checklist),
                        Action::Checklist(ChecklistAction::Make(id.clone())),
                    ),
                    (
                        t.make_shopping_list(),
                        None,
                        may(|n| n.shopping),
                        Action::Shopping(ShoppingAction::Make(id.clone())),
                    ),
                    (
                        t.make_tally(),
                        None,
                        may(|n| n.tally),
                        Action::Tally(TallyAction::Make(id.clone())),
                    ),
                ];
                menu_entries(ui, items, actions);
                ui.separator();
                let items = [
                    (
                        t.send_to_person(),
                        None,
                        may(|n| n.amends),
                        Action::StartSend(id.clone()),
                    ),
                    (
                        t.copy_as_text(),
                        None,
                        may(|n| n.amends),
                        Action::CopyAsText(id.clone()),
                    ),
                ];
                menu_entries(ui, items, actions);
                ui.separator();
                menu_entries(
                    ui,
                    [(
                        t.history_title(),
                        None,
                        note.is_some(),
                        Action::NoteHistory(id.clone()),
                    )],
                    actions,
                );
                ui.separator();
                menu_entries(
                    ui,
                    [(t.delete(), None, may(|n| n.changes), Action::Trash)],
                    actions,
                );
            });
            ui.menu_button(RichText::new(t.menu_sync()).color(text), |ui| {
                let items = [
                    (t.menu_sync_now(), None, true, Action::SyncNow),
                    (t.menu_pair(), None, true, Action::OpenPage(Page::Pair)),
                    (t.people_title(), None, true, Action::OpenPage(Page::People)),
                    (
                        t.profile_title(),
                        None,
                        true,
                        Action::OpenPage(Page::Profile),
                    ),
                    (
                        t.menu_backups(),
                        None,
                        true,
                        Action::OpenPage(Page::Backups),
                    ),
                ];
                menu_entries(ui, items, actions);
            });
            ui.menu_button(RichText::new(t.menu_help()).color(text), |ui| {
                let items = [
                    (t.menu_user_guide(), None, true, Action::OpenGuide),
                    (t.about_title(), None, true, Action::OpenPage(Page::About)),
                ];
                menu_entries(ui, items, actions);
            });
        });
    }

    /// Back and forward at the top of the list, and the path to the place
    /// it shows, each level above it a click away (#145).
    fn nav_row(&self, ui: &mut Ui, actions: &mut Vec<Action>) {
        let t = self.t;
        let text = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        ui.horizontal(|ui| {
            let arrows = [
                (
                    self.nav.can_go_back(),
                    icons::ARROW_BACK,
                    t.nav_back(),
                    Action::Back,
                ),
                (
                    self.nav.can_go_forward(),
                    icons::ARROW_FORWARD,
                    t.nav_forward(),
                    Action::Forward,
                ),
            ];
            for (may, icon, label, action) in arrows {
                let colour = if may { text } else { weak };
                let arrow = ui.add_enabled_ui(may, |ui| icon_button(ui, icon, label, colour));
                if arrow.inner.clicked() {
                    actions.push(action);
                }
            }
            ui.add_space(8.0);
            for (i, (name, to)) in self.path().into_iter().enumerate() {
                if i > 0 {
                    crumb(ui, "›", weak, None);
                }
                match to {
                    Some(to) => {
                        if crumb(ui, &name, text, Some(&t.path_up(&name))) {
                            actions.push(Action::Select(to));
                        }
                    }
                    None => {
                        crumb(ui, &name, weak, None);
                    }
                }
            }
        });
        ui.add_space(4.0);
    }

    /// The dialog of what was a view of its own (#145), over the window
    /// and under the dialogs it opens: on top the way back to the dialog it
    /// was opened from and its close button; Esc and a click beside it
    /// close it.
    fn page_dialog(&mut self, ctx: &egui::Context, tones: Tones, actions: &mut Vec<Action>) {
        let Some(page) = self.page() else {
            return;
        };
        let t = self.t;
        let parent = self
            .pages
            .len()
            .checked_sub(2)
            .and_then(|i| self.pages.get(i))
            .copied();
        let screen = ctx.content_rect().size();
        let size = egui::vec2(
            (screen.x - 120.0).clamp(320.0, 760.0),
            (screen.y - 100.0).max(240.0),
        );
        let frame = Frame::NONE
            .fill(tones.background)
            .corner_radius(theme::CARD_RADIUS)
            .inner_margin(Margin::same(16));
        let modal = egui::Modal::new(Id::new("page"))
            .frame(frame)
            .show(ctx, |ui| {
                ui.set_width(size.x);
                ui.set_height(size.y);
                self.announcements(ui, tones, actions);
                let text = ui.visuals().text_color();
                ui.horizontal(|ui| {
                    if let Some(parent) = parent {
                        let back = t.page_back(page_title(&t, parent));
                        if icon_button(ui, icons::ARROW_BACK, &back, text).clicked() {
                            actions.push(Action::PageBack);
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let close = t.page_close(page_title(&t, page));
                        if icon_button(ui, icons::CLOSE, &close, text).clicked() {
                            actions.push(Action::ClosePage);
                        }
                    });
                });
                match page {
                    Page::Settings => self.settings_pane(ui, tones, actions),
                    Page::About => {
                        if about_view::about(ui, &t, tones) {
                            actions.push(Action::PageInto(Page::Licences));
                        }
                    }
                    Page::Licences => about_view::licences_page(ui, &t),
                    Page::Profile | Page::People | Page::Backups => {
                        self.profile_pane(ui, tones, actions, page);
                    }
                    Page::Sync | Page::Pair => {
                        let wall = self.wall.clone();
                        let pair = page == Page::Pair;
                        let shown = self.sync.show(ui, &t, tones, (&self.store, &wall), pair);
                        if let Some(asked) = shown {
                            actions.push(Action::Sync(asked));
                        }
                    }
                }
            });
        if modal.should_close() {
            actions.push(Action::ClosePage);
        }
    }

    /// The Note open (#146), as a dialog over the window, one at a time:
    /// its editor, the badges of its settings (#160) and their small
    /// editors. The menu bar stays at hand over it, its Note menu acting
    /// on this Note. Closing it saves: the cross, Esc, Alt+Left, the
    /// mouse's back button and a click beside it.
    fn note_dialog(
        &mut self,
        ctx: &egui::Context,
        under: egui::Rect,
        tones: Tones,
        actions: &mut Vec<Action>,
    ) {
        if !self.note_open() {
            return;
        }
        let t = self.t;
        // The keyboard stays off the list and the sidebar beneath, as the
        // pointer does: Tab does not walk onto them.
        if let Some(focused) = ctx.memory(|m| m.focused())
            && let Some(beneath) = ctx.read_response(focused)
            && beneath.layer_id.order == egui::Order::Background
            && under.contains(beneath.rect.center())
        {
            ctx.memory_mut(|m| m.surrender_focus(focused));
        }
        // Esc is the dialog's while nothing lies over it.
        let top = self.top_dialog() == Some(TopDialog::Note)
            && self.dialog.is_none()
            && self.badge.is_none()
            && self.viewer.is_none()
            && self.table.is_none();
        if top
            && !egui::Popup::is_any_open(ctx)
            && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
        {
            actions.push(Action::CloseNote);
        }
        let size = egui::vec2(
            (under.width() - 120.0).clamp(320.0, 760.0),
            (under.height() - 60.0).max(240.0),
        );
        let frame = Frame::NONE
            .fill(tones.background)
            .corner_radius(theme::CARD_RADIUS)
            .inner_margin(Margin::same(16));
        egui::Area::new(Id::new("note dialog"))
            .order(egui::Order::Foreground)
            .fixed_pos(under.min)
            .constrain(false)
            .show(ctx, |ui| {
                // The backdrop over the list and the sidebar, as a modal
                // dialog's; a click on it closes the dialog.
                let mut backdrop = ui.new_child(
                    egui::UiBuilder::new()
                        .sense(Sense::click() | Sense::drag())
                        .max_rect(under),
                );
                backdrop.set_min_size(under.size());
                ui.painter()
                    .rect_filled(under, 0.0, Color32::from_black_alpha(100));
                if backdrop.response().clicked() {
                    actions.push(Action::CloseNote);
                }
                let shown =
                    egui::Rect::from_center_size(under.center(), size + egui::vec2(32.0, 32.0));
                ui.scope_builder(
                    egui::UiBuilder::new()
                        .max_rect(shown)
                        .sense(Sense::click() | Sense::drag()),
                    |ui| {
                        frame.show(ui, |ui| {
                            ui.set_width(size.x);
                            ui.set_height(size.y);
                            if self.announces_in() == Announcer::Note {
                                self.announcements(ui, tones, actions);
                            }
                            let text = ui.visuals().text_color();
                            ui.horizontal(|ui| {
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if icon_button(ui, icons::CLOSE, t.close(), text).clicked()
                                        {
                                            actions.push(Action::CloseNote);
                                        }
                                    },
                                );
                            });
                            self.note_editor(ui, tones, actions);
                        });
                    },
                );
            });
        self.badge_dialog(ctx, tones, actions);
    }

    /// The small editor of the Note's badge clicked (#160), while it is
    /// open: each changes its setting at once, and the badge follows.
    fn badge_dialog(&mut self, ctx: &egui::Context, tones: Tones, actions: &mut Vec<Action>) {
        let Some(open) = self.badge else {
            return;
        };
        let Some((placed, sections)) = &self.view.detail else {
            self.badge = None;
            return;
        };
        let t = self.t;
        let note = &placed.note;
        let captured_removable = &self.view.captured_removable;
        let shown = (&t, tones);
        let closed = match open {
            NoteBadge::Importance => badge_dialog(
                ctx,
                shown,
                (icons::IMPORTANCE, None),
                t.importance(),
                |ui| {
                    let mut picked = note.importance;
                    let options = [
                        (
                            Importance::Unimportant,
                            t.importance_unimportant().to_string(),
                        ),
                        (Importance::Normal, t.importance_normal().to_string()),
                        (Importance::Important, t.importance_important().to_string()),
                    ];
                    if radio_column(ui, &mut picked, options) {
                        if picked != note.importance {
                            actions.push(Action::Importance(picked));
                        }
                        actions.push(Action::Badge(None));
                    }
                },
            ),
            NoteBadge::Captured => badge_dialog(
                ctx,
                shown,
                (icons::CONTEXT_WHEN, None),
                t.captured_heading(),
                // Each detail on a line of its own.
                |ui| {
                    let id = &note.id;
                    let removable = |detail| captured_removable.contains(&(id.clone(), detail));
                    let shown = (note.captured, &note.context);
                    if let Some(detail) = capture_context(ui, &t, shown, removable) {
                        let id = id.clone();
                        actions.push(Action::AskRemoveCaptured { id, detail });
                    }
                },
            ),
            NoteBadge::SectionCaptured(at) => match sections.get(at) {
                Some(section) => badge_dialog(
                    ctx,
                    shown,
                    (icons::CONTEXT_WHEN, None),
                    t.captured_heading(),
                    |ui| {
                        let id = &section.id;
                        let removable = |detail| captured_removable.contains(&(id.clone(), detail));
                        let shown = (section.captured, &section.context);
                        if let Some(detail) = capture_context(ui, &t, shown, removable) {
                            let id = id.clone();
                            actions.push(Action::AskRemoveCaptured { id, detail });
                        }
                    },
                ),
                // Gone meanwhile.
                None => true,
            },
            NoteBadge::Kind => {
                let shown_kind = KindShown::of(&self.view);
                let current = shown_kind.kind;
                badge_dialog(ctx, shown, (icons::KIND, None), t.badge_kind(), |ui| {
                    // The kinds as radio buttons, the current one picked;
                    // a pick changes it at once and closes (#188).
                    let mut picked = current;
                    let options = std::iter::once(current)
                        .chain(shown_kind.choices.iter().copied())
                        .map(|kind| (kind, kind_look(&t, kind).1.to_string()));
                    if radio_column(ui, &mut picked, options) {
                        if picked != current {
                            actions.push(kind_action(&note.id, current, picked));
                        }
                        actions.push(Action::Badge(None));
                    }
                })
            }
            // The formats the text may take, a formatted one never plain
            // again (ADR 0033); a pick sets it at once and closes (#213).
            NoteBadge::Format => {
                let known = [text_format::PLAIN, text_format::MARKDOWN, text_format::CODE];
                let current = known
                    .into_iter()
                    .find(|f| *f == note.text_format)
                    .unwrap_or(text_format::PLAIN);
                let setting = t.badge_text_format();
                badge_dialog(ctx, shown, (icons::TEXT_FORMAT, None), setting, |ui| {
                    let mut picked = current;
                    let options = known
                        .into_iter()
                        .filter(|f| *f != text_format::PLAIN || current != text_format::MARKDOWN)
                        .map(|f| (f, text_format_look(&t, f).1.to_string()));
                    if radio_column(ui, &mut picked, options) {
                        if picked != current {
                            actions.push(Action::SetTextFormat(picked));
                        }
                        actions.push(Action::Badge(None));
                    }
                })
            }
            // A Checklist's (#219) or a Shopping List's (#186) three ways.
            NoteBadge::OnAgenda | NoteBadge::ShoppingOnAgenda => {
                let checklist = open == NoteBadge::OnAgenda;
                let shows = match checklist {
                    true => self.view.checklist.as_ref().and_then(|c| c.on_agenda),
                    false => self.view.shopping.as_ref().and_then(|s| s.on_agenda),
                };
                let setting = t.checklist_show_on_agenda();
                badge_dialog(ctx, shown, (icons::ON_AGENDA, None), setting, |ui| {
                    let Some(shows) = shows else {
                        return;
                    };
                    let mut picked = shows;
                    let options = ShoppingOnAgenda::ALL
                        .map(|way| (way, shopping_view::on_agenda_label(&t, way).to_string()));
                    // A pick writes it and closes, as on the phone.
                    if radio_column(ui, &mut picked, options) {
                        if picked != shows {
                            let note = note.id.clone();
                            actions.push(match checklist {
                                true => Action::Checklist(ChecklistAction::OnAgenda(note, picked)),
                                false => Action::Shopping(ShoppingAction::OnAgenda(note, picked)),
                            });
                        }
                        actions.push(Action::Badge(None));
                    }
                })
            }
            // Each pick written at once, the rule's closing (#191).
            NoteBadge::SessionRule => match (&mut self.session_draft, &self.view.session_setup) {
                (Some(draft), Some(setup)) => {
                    let icon = (icons::SESSION_RULE_BADGE, None);
                    badge_dialog(ctx, shown, icon, t.session_rule(), |ui| {
                        if draft.rules(ui, &t) {
                            let changed = Box::new((draft.clone(), setup.clone()));
                            actions.push(Action::SessionSetup(changed));
                            actions.push(Action::Badge(None));
                        }
                    })
                }
                // No Session meanwhile.
                _ => true,
            },
            NoteBadge::SessionWeekdays => match (&mut self.session_draft, &self.view.session_setup)
            {
                (Some(draft), Some(setup)) => {
                    let icon = (icons::SESSION_WEEKDAYS, None);
                    let setting = t.session_weekday_counts();
                    badge_dialog(ctx, shown, icon, setting, |ui| {
                        let before = draft.counts;
                        draft.weekdays(ui, &t);
                        if draft.counts != before {
                            let changed = Box::new((draft.clone(), setup.clone()));
                            actions.push(Action::SessionSetup(changed));
                        }
                    })
                }
                _ => true,
            },
            // Each pick written at once, as on the phone; gone with its Line.
            NoteBadge::Reappear => {
                let line = self.reappear_line.clone();
                let current = line.as_ref().and_then(|id| {
                    let shopping = self.view.shopping.as_ref()?;
                    shopping
                        .editable
                        .contains(id)
                        .then(|| shopping.list.line(id).map(|l| l.reappear))
                        .flatten()
                });
                match (line, current) {
                    (Some(id), Some(current)) => {
                        let icon = (icons::REAPPEAR, None);
                        badge_dialog(ctx, shown, icon, t.shopping_reappear(), |ui| {
                            if let Some(picked) = shopping_view::reappear_editor(ui, &t, current)
                                && picked != current
                            {
                                let step = ShopStep::Reappear(picked);
                                actions.push(Action::Shopping(ShoppingAction::Step(id, step)));
                            }
                        })
                    }
                    _ => true,
                }
            }
            NoteBadge::Ticks => {
                let mut on = self.view.show_ticks.contains(&note.id);
                let setting = t.box_show_ticks();
                badge_dialog(ctx, shown, (icons::SHOW_TICKS, None), setting, |ui| {
                    if badge_switch(ui, &t, setting, &mut on) {
                        actions.push(Action::ShowTicks {
                            note: note.id.clone(),
                            on,
                        });
                    }
                })
            }
        };
        if closed {
            actions.push(Action::Badge(None));
        }
    }

    /// The quick note (#142, #146), as a dialog over the window: what
    /// File ▸ New Note, Ctrl+N and "+" open. Closing it keeps what is
    /// typed, as leaving the phone's capture screen does: the cross, Esc,
    /// Alt+Left, the mouse's back button and a click beside it.
    fn quick_note_dialog(&mut self, ctx: &egui::Context, tones: Tones, actions: &mut Vec<Action>) {
        if !self.quick_open {
            return;
        }
        let t = self.t;
        let screen = ctx.content_rect().size();
        let width = (screen.x - 120.0).clamp(320.0, 600.0);
        let frame = Frame::NONE
            .fill(tones.background)
            .corner_radius(theme::CARD_RADIUS)
            .inner_margin(Margin::same(16));
        // Its top stays put as it grows with what is typed and set up.
        let id = Id::new("quick note");
        let area = egui::Modal::default_area(id).anchor(egui::Align2::CENTER_TOP, [0.0, 80.0]);
        let modal = egui::Modal::new(id)
            .area(area)
            .frame(frame)
            .show(ctx, |ui| {
                ui.set_width(width);
                if self.announces_in() == Announcer::Quick {
                    self.announcements(ui, tones, actions);
                }
                let text = ui.visuals().text_color();
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(t.menu_new_note())
                            .size(18.0)
                            .strong()
                            .color(text),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if icon_button(ui, icons::CLOSE, t.close(), text).clicked() {
                            actions.push(Action::CloseQuickNote);
                        }
                    });
                });
                // No scrolling: an Area's content cannot grow past its size
                // of the frame before, and the quick note grows with what
                // is typed and set up.
                ui.spacing_mut().item_spacing.y = 16.0;
                self.capture_card(ui, tones, actions);
            });
        if modal.should_close() {
            actions.push(Action::CloseQuickNote);
        }
    }

    /// The Routine or Reminder editor, the New Session page and a Tally's
    /// field or entry page, each as a dialog over the window, over the
    /// Note or the quick note they were opened from. Esc, a click beside
    /// it and Cancel leave it without saving.
    fn editor_dialog(&mut self, ctx: &egui::Context, tones: Tones, actions: &mut Vec<Action>) {
        if self.editor.is_none() && self.tally_form.is_none() {
            return;
        }
        let t = self.t;
        let screen = ctx.content_rect().size();
        let size = egui::vec2(
            (screen.x - 120.0).clamp(320.0, 720.0),
            (screen.y - 100.0).max(240.0),
        );
        let frame = Frame::NONE
            .fill(tones.background)
            .corner_radius(theme::CARD_RADIUS)
            .inner_margin(Margin::same(16));
        let shown_now = (self.wall)(self.store.now());
        let mut asked = None;
        let modal = egui::Modal::new(Id::new("editor"))
            .frame(frame)
            .show(ctx, |ui| {
                ui.set_width(size.x);
                ui.set_height(size.y);
                if let Some(editor) = &mut self.editor {
                    asked = editor.show(ui, &t, tones, shown_now).map(Action::Editor);
                } else if let Some(form) = &mut self.tally_form {
                    asked = form.show(ui, &t, tones).map(Action::TallyForm);
                }
            });
        actions.extend(asked);
        if modal.should_close() {
            actions.push(if self.editor.is_some() {
                Action::Editor(Intent::Close)
            } else {
                Action::TallyForm(FormIntent::Close)
            });
        }
    }

    /// The New Tally page (#161), as a dialog over the window or over the
    /// New Session page it was opened from. Leaving it without Save makes
    /// nothing.
    fn tally_page_dialog(&mut self, ctx: &egui::Context, tones: Tones, actions: &mut Vec<Action>) {
        let Some(page) = &mut self.tally_page else {
            return;
        };
        let t = self.t;
        let screen = ctx.content_rect().size();
        let size = egui::vec2(
            (screen.x - 140.0).clamp(320.0, 680.0),
            (screen.y - 120.0).max(240.0),
        );
        let frame = Frame::NONE
            .fill(tones.background)
            .corner_radius(theme::CARD_RADIUS)
            .inner_margin(Margin::same(16));
        let mut asked = None;
        let modal = egui::Modal::new(Id::new("new tally"))
            .frame(frame)
            .show(ctx, |ui| {
                ui.set_width(size.x);
                ui.set_height(size.y);
                asked = page.show(ui, &t, tones);
            });
        actions.extend(asked.map(Action::TallyPage));
        if modal.should_close() {
            actions.push(Action::TallyPage(PageIntent::Close));
        }
    }

    /// Whether a Note shows in the Note dialog.
    pub fn note_open(&self) -> bool {
        self.view.detail.is_some()
    }

    /// Whether the quick note's dialog shows (#146).
    pub fn quick_note_open(&self) -> bool {
        self.quick_open
    }

    /// File ▸ New Note: the quick note's dialog, as Ctrl+N opens it.
    pub fn new_note(&mut self) {
        self.apply(Action::NewNote);
    }

    /// Closes the Note dialog, keeping what was typed, as Esc does.
    pub fn close_note(&mut self) {
        self.apply(Action::CloseNote);
    }

    /// The dialog on top of the others that back closes or leads back
    /// from, but for the small ones.
    fn top_dialog(&self) -> Option<TopDialog> {
        if self.tally_page.is_some() {
            Some(TopDialog::TallyPage)
        } else if self.editor.is_some() {
            Some(TopDialog::Editor)
        } else if self.tally_form.is_some() {
            Some(TopDialog::TallyForm)
        } else if self.quick_open {
            Some(TopDialog::QuickNote)
        } else if !self.pages.is_empty() {
            Some(TopDialog::Page)
        } else if self.note_open() {
            Some(TopDialog::Note)
        } else {
            None
        }
    }

    /// Where the announcements show: at the top of the window, or of the
    /// dialog over it (UI laws, "Desktop").
    fn announces_in(&self) -> Announcer {
        if !self.pages.is_empty() {
            Announcer::Page
        } else if self.quick_open {
            Announcer::Quick
        } else if self.note_open() {
            Announcer::Note
        } else {
            Announcer::Window
        }
    }

    /// The title of the middle pane.
    fn title(&self) -> String {
        match &self.selection {
            Selection::Agenda => self.t.agenda_title().to_string(),
            Selection::SentItemsWaiting => self.t.sent_items_waiting_title().to_string(),
            Selection::AreaOffersWaiting => self.t.area_offers_waiting_title().to_string(),
            Selection::AiReview => self.t.ai_review_title().to_string(),
            Selection::Frogs => self.t.frogs_title().to_string(),
            Selection::AllRoutines => self.t.all_routines_title().to_string(),
            Selection::History(_) => self.t.history_title().to_string(),
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

    /// The quick note's content (UI laws, "Capture"), in its dialog
    /// (#146): it never asks anything.
    /// Once something is typed, "To full editor" shows (#142) and beside
    /// it what of the Note card's buttons (#113) this Device switched on
    /// in Settings ▸ Quick note, whose choices show at the top, not saved
    /// yet, and under them the Nudge times, Ctrl+Enter taking the 1 h
    /// one; a Note filed or with something set up, or where the Nudge
    /// buttons are switched off, gets one Save instead, nudging only an
    /// Unsorted Note. With a Shopping List picked in "More", the lines go
    /// into it instead of a Note (#124). The format bar shows over the
    /// field where it is switched on.
    fn capture_card(&mut self, ui: &mut Ui, tones: Tones, actions: &mut Vec<Action>) {
        let t = self.t;
        let place = self.capture_place();
        let sorted = self.capture_choices.sorted(place.as_deref());
        // The Area shown is where it goes, not a choice to drop.
        let place_chip = self.capture_choices.place.as_ref().and_then(|id| {
            capture_choices::place_name(&t, &self.view.areas, id).map(|name| (name, true))
        });
        let spawns = self.view.capture_spawns;
        let quick = self.view.capture_quick;
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
                && (!self.capture.trim().is_empty() || self.holds_media())
                && ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Enter))
            {
                actions.push(Action::Capture(Notes::default_nudge()));
            }
            // Announcements and choices go to the top (UI laws).
            let shown = (place_chip, self.view.capture_lists.as_slice());
            if let Some(asked) = capture_choices::chips(ui, &t, &self.capture_choices, shown) {
                actions.push(Action::CaptureChoice(asked));
            }
            // Ctrl+V in the empty field takes text and picture alike,
            // before the field takes the text alone (#130).
            let blank = self.capture.trim().is_empty();
            if has_clipboard
                && recording.is_none()
                && let Some(text) = paste::ctrl_v(ui, id, blank, &mut self.paste_held)
            {
                self.paste(ui, id, text);
            }
            if self.paste_empty {
                paste::empty_line(ui, &t);
            }
            if quick.format_bar {
                format_bar::capture_bar(ui, &t, id, &mut self.capture, (text, tones.accent));
            }
            let field = ui.add(
                TextEdit::multiline(&mut self.capture)
                    .id(id)
                    .hint_text(t.capture_hint())
                    .text_color(text)
                    .font(FontId::proportional(16.0))
                    .desired_rows(2)
                    .desired_width(f32::INFINITY)
                    .frame(Frame::NONE),
            );
            name_field(ui, &field, t.capture_field_name());
            // File ▸ New Note: after the field, so the menu's click does
            // not take it out again; asked again past the dialog's first,
            // sizing pass, which drops it.
            if self.capture_focus {
                field.request_focus();
                self.capture_focus = ui.is_sizing_pass() || !field.has_focus();
            }
            if field.changed() {
                self.paste_empty = false;
            }
            // A stopped recording, a picture pasted and the files chosen
            // or dropped stay here until the Note is saved.
            for _ in &self.recorded {
                ui.horizontal(|ui| {
                    icons::show(ui, icons::MIC, 18.0, text);
                    ui.label(RichText::new(t.voice_recording()).color(text));
                });
            }
            for _ in &self.held_photos {
                ui.horizontal(|ui| {
                    icons::show(ui, icons::PHOTO, 18.0, text);
                    ui.label(RichText::new(t.photo()).color(text));
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
                if paste::button(ui, &t) {
                    self.paste(ui, id, None);
                }
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
            if self.capture.trim().is_empty() && recording.is_none() && !self.holds_media() {
                return;
            }
            ui.add_space(4.0);
            let offer = (self.view.areas.as_slice(), spawns, self.capture_lists());
            let row = (&quick, true);
            match capture_choices::action_row(ui, &t, &self.capture_choices, offer, row) {
                Some(RowAsk::Choice(asked)) => actions.push(Action::CaptureChoice(asked)),
                Some(RowAsk::ToEditor) => actions.push(Action::CaptureToEditor),
                None => {}
            }
            ui.add_space(4.0);
            if sorted || !quick.nudge_buttons {
                // A sorted Note needs no Nudge (#113); without the Nudge
                // buttons, Save nudges in 1 h (#142).
                if theme::primary(ui, true, t.save()).clicked() {
                    actions.push(Action::Capture(Notes::default_nudge()));
                }
                let hint = if sorted {
                    t.capture_keys_hint_sorted()
                } else {
                    t.capture_keys_hint()
                };
                ui.label(RichText::new(hint).small().color(weak));
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
            Selection::AllRoutines => {
                let mut asked = Vec::new();
                let rows = (&self.view.all_routines[..], &self.view.all_newer[..]);
                all_routines_view::page(ui, &self.t, tones, rows, &mut asked);
                actions.extend(asked.into_iter().map(|asked| match asked {
                    AllRoutinesAction::Open(id) => Action::OpenRoutine(id),
                    AllRoutinesAction::OpenNote(id) => Action::SelectNote(id),
                    AllRoutinesAction::NewSession => Action::NewSession(None),
                    AllRoutinesAction::Export => Action::ExportRoutines,
                }));
                return;
            }
            Selection::History(_) => {
                history_view::page(ui, &self.t, tones, self.view.history.as_ref(), &*self.wall);
                return;
            }
            _ => {}
        }
        let text = ui.visuals().text_color();
        ui.horizontal(|ui| {
            ui.heading(RichText::new(self.title()).color(text));
            // Every Note list but the trash has its compact/detailed switch.
            if self.cards_key().is_some() && compact_button(ui, tones, &self.t, self.view.compact) {
                actions.push(Action::Compact(!self.view.compact));
            }
            if self.selection != Selection::Trash {
                self.person_filter_menu(ui, actions);
            }
            // "+" makes what the list holds (#161), an Area's level its
            // Topics too (UI laws, "Navigation"); last, so the buttons
            // before it keep their place while it goes.
            if let Some(kinds) = self.new_kinds()
                && let Some(kind) = add_button(ui, &self.t, &kinds)
            {
                actions.push(Action::Add(kind));
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
                // Edit ▸ Find: after the field, so the menu's click does
                // not take it out again.
                if std::mem::take(&mut self.search_focus) {
                    field.request_focus();
                }
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
        let output = list_scroll(ui, "notes", |ui| {
            // Room for the emboss around every card.
            ui.spacing_mut().item_spacing.y = 16.0;
            ui.add_space(4.0);
            let transcribing = self.transcriber.as_ref().and_then(|e| e.current());
            let order: Vec<String> = self.view.notes.iter().map(|p| p.note.id.clone()).collect();
            // Ctrl (Cmd on macOS) and Shift as the card is clicked (#148).
            let modifiers = ui.input(|i| i.modifiers);
            // A list with its switch has the view button on its cards
            // (#189).
            let switches = self.cards_key().is_some();
            for placed in &self.view.notes {
                let id = &placed.note.id;
                let selected = self.selected_note.as_deref() == Some(id.as_str());
                let picked = self.cards.contains(id);
                // A Note in the trash is restored, not filed.
                let draggable = !placed.note.is_trashed();
                let media = CardMedia {
                    photos: &mut self.photos,
                    pictures: &mut self.pictures,
                    store: &self.store,
                    transcribing: transcribing.as_deref(),
                };
                let look = CardLook {
                    full: self.view.compact == self.view.switched.contains(id),
                    glance: self.view.glances.get(id).map_or(&[], Vec::as_slice),
                    switches,
                    folds: CardFolds {
                        struck: self.view.struck_shown.contains(id),
                        description: self.view.descriptions_unfolded.contains(id),
                    },
                    sections: self.view.sections.get(id).map_or(&[], Vec::as_slice),
                    checklist: self.view.checklists.get(id).map(|(l, c)| (l, *c)),
                    shopping: self.view.shopping_lists.get(id).map(|(l, c)| (l, *c)),
                    tally: self.view.tallies.contains(id),
                    tally_shown: self.view.card_tallies.get(id),
                    session: self.view.sessions.contains(id),
                    session_shown: self.view.card_sessions.get(id),
                    menu: self.card_menu.as_ref(),
                    changes: self.view.card_changes.contains(id),
                    boxes: CardBoxes {
                        looks: &self.view.box_looks,
                        ticks: &self.view.box_ticks,
                        show: self.view.show_ticks.contains(id),
                    },
                    today: self.view.card_today.get(id).map_or(&[], Vec::as_slice),
                };
                let clicks = note_card(
                    ui,
                    &self.t,
                    tones,
                    placed,
                    (selected, picked, draggable),
                    look,
                    media,
                );
                // Dragged, the card takes the cards selected with it when
                // it is one of them (#148).
                if draggable && clicks.card.drag_started() {
                    let moving = self.cards.moving(id, &order);
                    egui::DragAndDrop::set_payload(ui.ctx(), DraggedNotes(moving));
                }
                // A click selects the card, Ctrl-click and Shift-click
                // select more (#148); its view button switches its view
                // (#189); a double-click or Edit opens the Note dialog (UI
                // laws, "Desktop").
                if clicks.menu_opened {
                    actions.push(Action::Checklist(ChecklistAction::Menu(id.clone())));
                }
                actions.extend(
                    clicks
                        .tally
                        .into_iter()
                        .map(|asked| Action::CardTally(id.clone(), asked)),
                );
                actions.extend(clicks.session.into_iter().map(Action::Session));
                if let Some(Action::AskFileInto(_)) = clicks.picked {
                    // The right-click's File into… files the cards selected
                    // with it.
                    actions.push(Action::AskFileInto(self.cards.moving(id, &order)));
                } else if let Some(picked) = clicks.picked {
                    actions.push(picked);
                } else if let Some(tag) = clicks.tag {
                    actions.push(Action::SearchFor(tag));
                } else if let Some((routine, occurrence)) = clicks.occurrence {
                    // As the Agenda row's box: it opens nothing (#215).
                    actions.push(Action::Toggle(routine, occurrence));
                } else if let Some((entity, asked)) = clicks.tick {
                    actions.push(box_action(entity, asked));
                } else if let Some(opening) = clicks.table {
                    actions.push(Action::OpenTable(opening));
                } else if let Some((line, checked)) = clicks.check {
                    if self.view.shopping_lists.contains_key(id) {
                        actions.push(Action::Shopping(ShoppingAction::Check(line, checked)));
                    } else {
                        // For the Occurrence the card shows it on.
                        let on = self.view.checklists.get(id).and_then(|(l, _)| l.on.clone());
                        let check = ChecklistAction::Check(line, checked, on);
                        actions.push(Action::Checklist(check));
                    }
                } else if let Some(hash) = clicks.photo {
                    actions.push(Action::ViewPhoto(hash));
                } else if let (Some(author), Some(profile)) = (clicks.author, &placed.author)
                    && author.clicked()
                {
                    actions.push(Action::OpenPerson(profile.id.clone()));
                } else if let Some((setting, open)) = clicks.folded {
                    actions.push(Action::Fold(setting, open));
                } else if clicks.view.clicked() {
                    actions.push(Action::SwitchView(id.clone()));
                } else if clicks.edit.clicked()
                        // A click soon after two more counts as well.
                        || clicks.card.double_clicked()
                        || clicks.card.triple_clicked()
                {
                    actions.push(Action::SelectNote(id.clone()));
                } else if clicks.card.clicked() {
                    let pick = if modifiers.shift {
                        Pick::Run
                    } else if modifiers.command {
                        Pick::Toggle
                    } else {
                        Pick::One
                    };
                    actions.push(Action::PickCard(id.clone(), pick));
                }
            }
        });
        if output.state.offset.y != self.scrolled {
            // The marks follow on the next pass.
            self.scrolled = output.state.offset.y;
            ui.ctx().request_repaint();
        }
    }

    /// What "+" at the top of the list makes (#161), in the order its
    /// menu shows them: in All Notes, an Area and a Topic the six kinds,
    /// Session and Frog only for who may set them up there, in an Area
    /// a Topic too for who may add one; in Unsorted a Note. Nothing among
    /// another person's Notes, nor in the trash.
    fn new_kinds(&self) -> Option<Vec<NewKind>> {
        if !self.people.is_everyone() {
            return None;
        }
        match &self.selection {
            Selection::Unsorted => Some(vec![NewKind::Note]),
            Selection::All | Selection::Place(_) => {
                let mut kinds = vec![
                    NewKind::Note,
                    NewKind::Checklist,
                    NewKind::Shopping,
                    NewKind::Tally,
                ];
                if self.view.list_manages {
                    kinds.extend([NewKind::Session, NewKind::Frog]);
                }
                if let Selection::Place(id) = &self.selection
                    && self.view.area(id).is_some()
                    && matches!(
                        self.view.access.get(id),
                        None | Some(AreaAccess::Owner | AreaAccess::Manager)
                    )
                {
                    kinds.push(NewKind::Topic);
                }
                Some(kinds)
            }
            _ => None,
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
                if theme::checkbox(ui, &mut me, RichText::new(t.person_filter_me()).color(text))
                    .changed()
                {
                    let mut next = filter.clone();
                    next.me = me;
                    actions.push(Action::FilterPeople(next));
                }
                for person in &self.view.people_shown {
                    let mut on = filter.people.contains(&person.id);
                    let name = profile_view::display_name(t, &person.name);
                    if theme::checkbox(ui, &mut on, RichText::new(name).color(text)).changed() {
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
    /// The Note dialog's content (#146): the text, the badges of the
    /// Note's settings under it (#160), its Lines, media, Tally and
    /// Sections, Amend and the rest of what can be done to it. A Section
    /// this Device may edit has a field of its own, as the Note's text
    /// (#183); a text only read selects to be copied.
    fn note_editor(&mut self, ui: &mut Ui, tones: Tones, actions: &mut Vec<Action>) {
        let Some((placed, sections)) = &self.view.detail else {
            return;
        };
        let kind = KindShown::of(&self.view);
        // A Checklist, a Shopping List or a Tally shows its text as its
        // name and its description, its content under them (#188).
        let kinded = self.view.kind.is_some_and(|k| k != NoteKind::Plain);
        let linked = &self.view.routines;
        let newer_routines = self.view.newer_routines;
        let tally = self.view.tally.as_ref();
        let checklist = self.view.checklist.as_ref();
        let shopping = self.view.shopping.as_ref();
        let session_setup = self.view.session_setup.as_ref();
        let makes_session = self.view.makes_session;
        let session_draft = &mut self.session_draft;
        let typed = &mut self.lines;
        let note_frogs = &self.view.note_frogs;
        let may = self.view.may;
        let t = self.t;
        let note = &placed.note;
        let trashed = note.is_trashed();
        // "Keep as code" under a paste that looks like code (#213).
        let offers_code = self.code_offer.is_some()
            && self.code_offer == self.text_of
            && note.text_format != text_format::CODE;
        let text = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        let edited = &mut self.text;
        let name = &mut self.name;
        let editing = &mut self.text_editing;
        let focus = &mut self.text_focus;
        let select = &mut self.text_select;
        let section_texts = &mut self.section_texts;
        let amend = &mut self.amend;
        let amend_focus = &mut self.amend_focus;
        let amend_select = &mut self.amend_select;
        let cache = &mut self.photos;
        let store = &self.store;
        let playing = self.playing.as_deref();
        let can_play = self.player.is_some();
        let speed = self.speed;
        let transcribing = self.transcriber.as_ref().and_then(|e| e.current());
        let transcribing = transcribing.as_deref();
        // Whisper needs its model; RustO is built in.
        let heard = self.speech.as_ref().is_none_or(|s| s.current().is_some());
        let offering = Offering {
            view: &self.view,
            transcriber: self.transcriber.as_ref().filter(|_| heard),
            reader: self.reader.as_ref(),
        };
        let offering = &offering;
        // Its boxes and its Sections' (ADR 0035): how they show, and
        // whether this Device may tick them.
        let no_boxes = BoxLook::default();
        let box_looks = &self.view.box_looks;
        let box_ticks = &self.view.box_ticks;
        let look_of = |id: &str| box_looks.get(id).unwrap_or(&no_boxes);
        let boxes = look_of(&note.id);
        let ticks = box_ticks.contains(&note.id);
        let ticked = ticked_in(&note.boxes);
        let show_ticks = self.view.show_ticks.contains(&note.id);
        let has_boxes = !note.boxes.is_empty() || sections.iter().any(|s| !s.boxes.is_empty());
        let open = &self.view.open;
        let today = Day::today(store.now(), &*self.wall);
        // Its one Reminder goes with it, where this Device moves it.
        let trashes = may.changes && linked.len() == 1;
        egui::ScrollArea::vertical()
            .id_salt("note")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 16.0;
                ui.add_space(4.0);
                // At the top, over its name (#215).
                match card_occurrences::open_lines(ui, &t, open, (today, trashes)) {
                    Some(OpenAsked::Done(routine, occurrence)) => {
                        actions.push(Action::CheckMissed(routine, occurrence));
                    }
                    Some(OpenAsked::DoneAndTrash(routine, occurrence)) => {
                        actions.push(Action::DoneAndTrash(routine, occurrence));
                    }
                    None => {}
                }
                embossed(ui, tones, false, None, |ui| {
                    // A Member reads someone else's Note (ADR 0013); a text
                    // in a format not known here shows as it is stored
                    // (ADR 0018).
                    if !note.text_editable {
                        ui.label(RichText::new(t.text_newer_format()).color(weak));
                    }
                    let formatted = note.text_format == text_format::MARKDOWN;
                    let writes = !trashed && may.changes && note.text_editable;
                    if kinded {
                        let shown = KindedText {
                            note,
                            writes,
                            boxes,
                            ticks: (ticks, &ticked),
                        };
                        let state = (&mut *edited, &mut *editing, &mut *focus, &mut *select);
                        kinded_text(ui, (t, tones), shown, (&mut *name, state), actions);
                        return;
                    }
                    if !writes {
                        // Its marks show, to read (#125), and a formatted
                        // text reads as it is formatted (#127); it selects
                        // to be copied (#183).
                        ui.style_mut().interaction.selectable_labels = true;
                        let all = (0, note.text.len());
                        // A Code text moves sideways (#213).
                        if note.text_format == text_format::CODE {
                            let look = (Some(18.0), text);
                            text_marks_view::code_text_label(
                                ui,
                                &note.text,
                                all,
                                look,
                                (false, true),
                            );
                            return;
                        }
                        let spots = text_marks_view::formatted_label(
                            ui,
                            (&note.text, &note.text_format, boxes),
                            all,
                            (Some(18.0), text),
                            false,
                        );
                        let of = Id::new(("text", &note.id));
                        // Its tables open to read (#128).
                        if let Some(at) = text_marks_view::table_clicks(ui, &t, of, &spots.tables) {
                            actions.push(Action::OpenTable(Opening {
                                note: None,
                                text: note.text.clone(),
                                at,
                                cell: None,
                                tags: true,
                                boxes: Some(note.id.clone()),
                            }));
                        }
                        // A Member ticks the boxes of a Note not theirs.
                        if let Some(asked) =
                            text_marks_view::box_clicks(ui, &t, of, &spots.boxes, (ticks, &ticked))
                        {
                            actions.push(box_action(note.id.clone(), asked));
                        }
                        if let Some(tag) = text_marks_view::tag_clicks(ui, &t, of, &spots.tags) {
                            actions.push(Action::SearchFor(tag));
                        }
                        text_marks_view::link_clicks(ui, of, &spots.links);
                        return;
                    }
                    let code = note.text_format == text_format::CODE;
                    let field = EditedText {
                        id: Id::new("note text"),
                        of: &note.id,
                        section: false,
                        stored: &note.text,
                        formatted,
                        code,
                        // A paste that looks like code offers it (#213).
                        offers_code: !code,
                        boxes,
                        ticks: (ticks, &ticked),
                        name: t.note_text_field_name(),
                        look: (18.0, text),
                        hint: None,
                    };
                    let state = (&mut *edited, &mut *editing, &mut *focus, &mut *select);
                    edited_text(ui, (t, tones), field, state, actions);
                    if offers_code {
                        code_offer(ui, &t, actions);
                    }
                });
                // The kind's content under its name and description (#188),
                // as the Lines are under them on a card; nothing while the
                // Note is no Checklist.
                if let Some(shown) = checklist {
                    let mut asked = Vec::new();
                    checklist_view::section(ui, (&t, tones), shown, typed, &mut asked);
                    actions.extend(asked.into_iter().map(Action::Checklist));
                }
                // Nothing while the Note is no Shopping List.
                if let Some(shown) = shopping {
                    let mut asked = Vec::new();
                    shopping_view::section(ui, (&t, tones), shown, typed, &mut asked);
                    actions.extend(asked.into_iter().map(Action::Shopping));
                }
                // Only its setup, nothing while the Note is no Tally; its
                // chart and entries show on its card (#190).
                if let Some(tally) = tally {
                    let mut asked = Vec::new();
                    tally_view::setup(ui, &t, tones, tally, &mut asked);
                    actions.extend(asked.into_iter().map(Action::Tally));
                }
                // A Session's pool and count, each change written at once;
                // its picks and history show on its card (#191).
                if let (Some(draft), Some(setup)) = (session_draft.as_mut(), session_setup) {
                    draft.new_tally = makes_session;
                    let (changed, asked) = draft.note_page(ui, &t, tones, makes_session);
                    if changed {
                        let changed = Box::new((draft.clone(), setup.clone()));
                        actions.push(Action::SessionSetup(changed));
                    }
                    if asked == Some(SessionIntent::NewTally) {
                        actions.push(Action::SessionPoolTally);
                    }
                }
                // Each setting as its badge, in one wrapping row under the
                // text and the kind's content (#160).
                let shown = BadgesShown {
                    placed,
                    routines: linked,
                    newer: newer_routines,
                    kind: &kind,
                    on_agenda: checklist.and_then(|c| c.on_agenda),
                    shopping_on_agenda: shopping.and_then(|s| s.on_agenda),
                    ticks: has_boxes.then_some(show_ticks),
                    session: session_setup.map(|s| (s, makes_session)),
                    may,
                    frog_open: note_frogs.iter().any(|(f, _)| !f.is_done()),
                };
                note_badges(ui, (&t, tones), &shown, actions);
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
                for (at, section) in sections.iter().enumerate() {
                    embossed(ui, tones, false, None, |ui| {
                        if !section.text_editable {
                            ui.label(RichText::new(t.text_newer_format()).color(weak));
                        }
                        let look = look_of(&section.id);
                        let section_ticks = box_ticks.contains(&section.id);
                        let section_ticked = ticked_in(&section.boxes);
                        if let Some(field) = section_texts.get_mut(&section.id) {
                            // Edited in place as the Note's text is, by the
                            // same people (#183).
                            let shown = EditedText {
                                id: Id::new(("section text", &section.id)),
                                of: &section.id,
                                section: true,
                                stored: &section.text,
                                formatted: section.text_format == text_format::MARKDOWN,
                                code: false,
                                offers_code: false,
                                boxes: look,
                                ticks: (section_ticks, &section_ticked),
                                name: t.section_text_field_name(),
                                look: (18.0, text),
                                hint: None,
                            };
                            let state = (
                                &mut field.text,
                                &mut field.editing,
                                &mut field.focus,
                                &mut field.select,
                            );
                            edited_text(ui, (t, tones), shown, state, actions);
                        } else if !section.text.is_empty() {
                            // Its boxes tick; only read, its text selects
                            // to be copied (#183). A formatted one reads
                            // formatted.
                            ui.style_mut().interaction.selectable_labels = true;
                            let all = (0, section.text.len());
                            let shown = (None, text);
                            let spots = text_marks_view::formatted_label(
                                ui,
                                (&section.text, &section.text_format, look),
                                all,
                                shown,
                                false,
                            );
                            let of = Id::new(("text", &section.id));
                            if let Some(at) =
                                text_marks_view::table_clicks(ui, &t, of, &spots.tables)
                            {
                                actions.push(Action::OpenTable(Opening {
                                    note: None,
                                    text: section.text.clone(),
                                    at,
                                    cell: None,
                                    tags: true,
                                    boxes: Some(section.id.clone()),
                                }));
                            }
                            if let Some(asked) = text_marks_view::box_clicks(
                                ui,
                                &t,
                                of,
                                &spots.boxes,
                                (section_ticks, &section_ticked),
                            ) {
                                actions.push(box_action(section.id.clone(), asked));
                            }
                            if let Some(tag) = text_marks_view::tag_clicks(ui, &t, of, &spots.tags)
                            {
                                actions.push(Action::SearchFor(tag));
                            }
                            text_marks_view::link_clicks(ui, of, &spots.links);
                            ui.style_mut().interaction.selectable_labels = false;
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
                        // Its details, each removable on its own, in the
                        // same dialog as the Note's Captured badge (#170).
                        let before = match at {
                            0 => note.captured,
                            _ => sections[at - 1].captured,
                        };
                        let line =
                            capture_line(&t, (section.captured, &section.context), Some(before));
                        let edited = section_edited(&t, section);
                        if capture_meta(ui, &t, &line, edited.as_deref(), true) {
                            actions.push(Action::Badge(Some(NoteBadge::SectionCaptured(at))));
                        }
                    });
                }
                if trashed {
                    // Where it is now, to be restored at once.
                    ui.horizontal(|ui| {
                        icons::show(ui, icons::TRASH, 18.0, weak);
                        ui.label(RichText::new(t.trash_title()).color(text));
                    });
                    if may.changes && ui.button(RichText::new(t.restore()).color(text)).clicked() {
                        actions.push(Action::Restore);
                    }
                    return;
                }
                if !may.amends {
                    return;
                }
                embossed(ui, tones, false, None, |ui| {
                    // The format bar goes with the field (UI laws), as on
                    // the phone's "Add to Note": without ☐, which the
                    // Note's text puts in.
                    let id = Id::new("amend");
                    let focused = ui.memory(|m| m.has_focus(id));
                    let selected = format_bar::selection(ui, id, amend);
                    let at = format_bar::At::of(amend, selected, markdown::uses(amend));
                    let formats = &Format::ALL[1..];
                    let mut picked = format_bar::bar(ui, &t, &at, (text, tones.accent), formats);
                    if focused {
                        ui.input_mut(|i| {
                            if i.consume_key(Modifiers::COMMAND, Key::B) {
                                picked = Some(Format::Bold);
                            } else if i.consume_key(Modifiers::COMMAND, Key::I) {
                                picked = Some(Format::Italic);
                            }
                        });
                    }
                    if std::mem::take(amend_focus) {
                        match amend_select.take() {
                            Some(range) => format_bar::select(ui, id, amend, range),
                            None => ui.memory_mut(|m| m.request_focus(id)),
                        }
                    }
                    ui.add(
                        TextEdit::multiline(amend)
                            .id(id)
                            .hint_text(t.amend_hint())
                            .text_color(text)
                            .desired_rows(1)
                            .desired_width(f32::INFINITY)
                            .frame(Frame::NONE),
                    );
                    match picked {
                        Some(Format::Table) => actions.push(Action::AmendTable(selected.0)),
                        Some(Format::Link) => actions.push(Action::AskLink {
                            start: selected.0,
                            end: selected.1,
                            url: at.url.clone(),
                            amend: true,
                            section: None,
                        }),
                        Some(format) => match format_bar::change(format, amend, selected) {
                            Some(changed) => {
                                *amend = changed.text;
                                let range = (changed.start, changed.end);
                                format_bar::select(ui, id, amend, range);
                            }
                            None => ui.memory_mut(|m| m.request_focus(id)),
                        },
                        None => {}
                    }
                    let ready = !amend.trim().is_empty();
                    ui.horizontal_wrapped(|ui| {
                        let add = ui.add_enabled(
                            ready,
                            egui::Button::new(RichText::new(t.amend()).color(text)),
                        );
                        if add.clicked() {
                            actions.push(Action::Amend);
                        }
                    });
                });
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .button(RichText::new(t.send_button()).color(text))
                        .on_hover_text(t.send_to_person())
                        .clicked()
                    {
                        actions.push(Action::StartSend(note.id.clone()));
                    }
                    // To another app (#129).
                    if ui
                        .button(RichText::new(t.copy_as_text()).color(text))
                        .clicked()
                    {
                        actions.push(Action::CopyAsText(note.id.clone()));
                    }
                    if ui
                        .button(RichText::new(t.save_as_markdown()).color(text))
                        .clicked()
                    {
                        actions.push(Action::SaveAsMarkdown(note.id.clone()));
                    }
                });
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
                    if theme::destructive(ui, true, t.move_to_trash()).clicked() {
                        actions.push(Action::Trash);
                    }
                });
                // Only a Note in the lists becomes a Tally (ADR 0028); one
                // that is adds entries in its Tally part above. A
                // Checklist, a Shopping List or a Session is no Tally (ADR
                // 0030, ADR 0039).
                if tally.is_none()
                    && checklist.is_none()
                    && shopping.is_none()
                    && session_setup.is_none()
                    && !note.is_merged()
                    && note.text_format != text_format::CODE
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
        let focus = self.focus;
        let Some(dialog) = &mut self.dialog else {
            self.focus = false;
            return;
        };
        // A dialog's first, sizing pass drops a focus: it is asked again.
        let mut sizing = false;
        let t = self.t;
        let view = &self.view;
        let (own_view, question_view) = (&mut self.profile_view, &mut self.question_view);
        let pictures = &mut self.pictures;
        let person_square = &mut self.person_square;
        let importing = self.import.kind();
        let frame = Frame::NONE
            .fill(tones.card)
            .corner_radius(theme::CARD_RADIUS)
            .inner_margin(Margin::same(20));
        let modal = egui::Modal::new(Id::new("dialog"))
            .frame(frame)
            .show(ctx, |ui| {
                sizing = ui.is_sizing_pass();
                ui.set_width(400.0);
                ui.spacing_mut().item_spacing.y = 10.0;
                match dialog {
                    Dialog::BoxHistory { lines } => {
                        title(ui, t.history_title());
                        let text = ui.visuals().text_color();
                        let weak = ui.visuals().weak_text_color();
                        ui.label(RichText::new(t.box_history_hint()).color(text));
                        egui::ScrollArea::vertical()
                            .max_height(360.0)
                            .show(ui, |ui| {
                                for (state, said) in lines.iter() {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(state.glyph().to_string())
                                                .size(20.0)
                                                .color(text),
                                        );
                                        ui.vertical(|ui| {
                                            let label = text_marks_view::box_label(&t, *state);
                                            ui.label(RichText::new(label).color(text));
                                            ui.label(RichText::new(said).small().color(weak));
                                        });
                                    });
                                }
                            });
                        if ui.button(RichText::new(t.close()).color(text)).clicked() {
                            actions.push(Action::CloseDialog);
                        }
                    }
                    Dialog::NoteHistory { rows, .. } => {
                        title(ui, t.history_title());
                        let text = ui.visuals().text_color();
                        let weak = ui.visuals().weak_text_color();
                        ui.label(RichText::new(t.note_history_hint()).color(text));
                        // As high as the window allows, the title, the
                        // hint and Close around it.
                        let high = (ui.ctx().content_rect().height() - 220.0).max(240.0);
                        egui::ScrollArea::vertical()
                            .max_height(high)
                            .show(ui, |ui| {
                                for (i, row) in rows.iter().enumerate() {
                                    note_change(ui, &t, (i, row), (text, weak), actions);
                                }
                            });
                        if ui.button(RichText::new(t.close()).color(text)).clicked() {
                            actions.push(Action::CloseDialog);
                        }
                    }
                    Dialog::Link {
                        address, linked, ..
                    } => link_form(ui, &t, (address, focus), *linked, actions),
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
                    Dialog::NewKind { kind, name } => {
                        let title = match kind {
                            NewKind::Checklist => t.new_checklist(),
                            NewKind::Shopping => t.new_shopping_list(),
                            _ => t.new_frog(),
                        };
                        let form = (title, t.create(), Action::CreateNamed);
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
                    Dialog::DeleteLine { for_good, .. } => {
                        title(ui, t.checklist_delete_question());
                        // The verb of the menu it came from.
                        let verb = if *for_good {
                            t.shopping_delete_for_good()
                        } else {
                            t.delete()
                        };
                        ui.horizontal(|ui| {
                            cancel(ui, &t, actions);
                            if destructive(ui, verb) {
                                actions.push(Action::DeleteLine);
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
                    } => area_offer_form(ui, &t, (held, *compared), pictures, actions),
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
                    } => held_sent_item_form(ui, &t, held, actions),
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
                    Dialog::RemoveCaptured { id, detail } => {
                        let of_note = view.detail.as_ref().is_some_and(|(p, _)| p.note.id == *id);
                        title(
                            ui,
                            &if of_note {
                                t.remove_captured_question(detail.name())
                            } else {
                                t.remove_captured_section_question(detail.name())
                            },
                        );
                        let text = ui.visuals().text_color();
                        ui.label(RichText::new(t.remove_captured_detail()).color(text));
                        ui.horizontal(|ui| {
                            cancel(ui, &t, actions);
                            if destructive(ui, t.remove()) {
                                actions.push(Action::RemoveCaptured);
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
                    } => {
                        if failure_form(ui, &t, heading, (cause, fix), details) {
                            actions.push(Action::CloseDialog);
                        }
                    }
                    Dialog::Done {
                        title: heading,
                        details,
                    } => {
                        if done_form(ui, &t, heading, details) {
                            actions.push(Action::CloseDialog);
                        }
                    }
                    Dialog::Imported { details, area } => {
                        let (close, open) = imported_form(ui, &t, details);
                        if close || open {
                            actions.push(Action::CloseDialog);
                        }
                        if open {
                            actions.push(Action::Select(Selection::Place(area.clone())));
                        }
                    }
                    Dialog::ImportKeep => {
                        title(ui, t.import_keep());
                        let text = ui.visuals().text_color();
                        ui.label(RichText::new(t.import_keep_ask()).color(text));
                        // The two answers stacked full width, Cancel below
                        // (UI laws), as on the phone.
                        let wide = |ui: &mut Ui, label: &str| {
                            ui.add_sized(
                                [ui.available_width(), 32.0],
                                egui::Button::new(RichText::new(label).color(text)),
                            )
                            .clicked()
                        };
                        if wide(ui, t.import_keep_zip()) {
                            actions.push(Action::ImportKeep(true));
                        }
                        if wide(ui, t.import_keep_folder()) {
                            actions.push(Action::ImportKeep(false));
                        }
                        cancel(ui, &t, actions);
                    }
                    Dialog::Import => {
                        title(ui, t.settings_import());
                        let text = ui.visuals().text_color();
                        let weak = ui.visuals().weak_text_color();
                        // Each with what it does, or that it reads; both
                        // greyed while either runs.
                        let imports = [
                            (
                                matches!(importing, Some(Importing::Folder(_))),
                                (t.import_folder_detail(), t.import_reading()),
                                t.import_folder_open(),
                                Action::Sync(SyncAction::ImportFolder),
                            ),
                            (
                                importing == Some(&Importing::Keep),
                                (t.import_keep_detail(), t.import_keep_reading()),
                                t.import_keep_open(),
                                Action::Sync(SyncAction::ImportKeep),
                            ),
                        ];
                        for (running, (detail, reading), label, asked) in imports {
                            let button = egui::Button::new(RichText::new(label).color(text));
                            if ui
                                .add_enabled_ui(importing.is_none(), |ui| {
                                    ui.add_sized([ui.available_width(), 32.0], button)
                                })
                                .inner
                                .clicked()
                            {
                                actions.push(asked);
                            }
                            let said = if running { reading } else { detail };
                            ui.label(RichText::new(said).small().color(weak));
                        }
                        if ui.button(RichText::new(t.close()).color(text)).clicked() {
                            actions.push(Action::CloseDialog);
                        }
                    }
                    Dialog::FileInto { notes } => {
                        // Several selected: how many go (#148).
                        match notes.len() {
                            1 => title(ui, t.file_into()),
                            n => title(ui, &t.file_notes_into(n as i64)),
                        }
                        let text = ui.visuals().text_color();
                        let weak = ui.visuals().weak_text_color();
                        egui::ScrollArea::vertical()
                            .max_height(420.0)
                            .show(ui, |ui| {
                                for (area, topics) in &view.areas {
                                    let places = std::iter::once((
                                        area.id.clone(),
                                        area_name(&t, area),
                                        0.0,
                                    ))
                                    .chain(
                                        topics
                                            .iter()
                                            .map(|p| (p.id.clone(), topic_name(&t, p), 24.0)),
                                    );
                                    for (place, name, indent) in places {
                                        ui.horizontal(|ui| {
                                            ui.add_space(indent);
                                            let button =
                                                egui::Button::new(RichText::new(name).color(text));
                                            if ui.add(button).clicked() {
                                                actions.push(Action::CloseDialog);
                                                actions.push(Action::File {
                                                    notes: notes.clone(),
                                                    place,
                                                });
                                            }
                                        });
                                    }
                                }
                                if view.areas.is_empty() {
                                    ui.label(RichText::new(t.no_areas_yet()).color(weak));
                                }
                            });
                        cancel(ui, &t, actions);
                    }
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
        if !sizing {
            self.focus = false;
        }
        if modal.should_close() {
            // Put off, not lost: it waits in the Profile dialog.
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

    /// The Agenda (UI laws, "Agenda"): "+" for the quick note, the Missed
    /// Summary until "Seen", the "x Unsorted" banner, the "x Sent Items
    /// waiting" line opening their list, the "x shared Areas waiting" line
    /// opening theirs, today's Occurrences with the Shopping Lists' and
    /// the Checklists' rows after the all-day ones, a gap, and tomorrow's dimmed, then the five days after
    /// the same way, each under its weekday and date; each checkable on
    /// its own, a click on one opens its Routine. Its bar leads to All
    /// Routines and Reminders and to a new Routine.
    fn agenda_pane(&mut self, ui: &mut Ui, tones: Tones, actions: &mut Vec<Action>) {
        let t = self.t;
        let text = ui.visuals().text_color();
        ui.horizontal(|ui| {
            ui.heading(RichText::new(t.agenda_title()).color(text));
            // As the phone's bar, without its Calendar (UI laws, "The
            // Agenda's bar").
            // "+" captures, as the phone's Agenda's does (#146).
            if add_button(ui, &t, &[NewKind::Note]).is_some() {
                actions.push(Action::NewNote);
            }
            ui.add_space((ui.available_width() - 60.0).max(0.0));
            if icon_button(ui, icons::ALL_ROUTINES, t.all_routines_title(), text).clicked() {
                actions.push(Action::Select(Selection::AllRoutines));
            }
            if icon_button(ui, icons::ADD_ALARM, t.new_routine_tooltip(), text).clicked() {
                actions.push(Action::NewRoutine {
                    note: None,
                    reminder: false,
                });
            }
        });
        ui.add_space(8.0);
        ui.spacing_mut().item_spacing.y = 16.0;
        let Some(agenda) = self.view.agenda.clone() else {
            return;
        };
        let output = list_scroll(ui, "agenda", |ui| {
            // Tighter than 0.3.1's 12 (#208).
            ui.spacing_mut().item_spacing.y = 6.0;
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
            if agenda.today.is_empty() && agenda.shopping.is_empty() && agenda.checklists.is_empty()
            {
                ui.label(RichText::new(t.nothing_today()).color(ui.visuals().weak_text_color()));
            }
            // The Shopping Lists' rows after the all-day Occurrences,
            // then the Checklists', before the timed ones (#123, #143).
            let timed = agenda
                .today
                .iter()
                .position(|r| !r.row.occurrence.all_day())
                .unwrap_or(agenda.today.len());
            let (all_day, timed) = agenda.today.split_at(timed);
            for row in all_day {
                agenda_row(ui, &t, tones, (row, agenda.hides_checked), actions);
            }
            for row in &agenda.shopping {
                shopping_row(ui, tones, row, actions);
            }
            for row in &agenda.checklists {
                checklist_row(ui, tones, row, actions);
            }
            for row in timed {
                agenda_row(ui, &t, tones, (row, agenda.hides_checked), actions);
            }
            // Under what is due, never above it (#101); gone once today's
            // Frog is eaten where this Device hides what is checked (#217).
            if let Some(frogs) = agenda
                .frogs
                .as_ref()
                .filter(|f| f.visible && !(agenda.hides_checked && f.eaten))
            {
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
                // Only today's empty day tells how to capture.
                if agenda.tomorrow.is_empty() {
                    ui.label(
                        RichText::new(t.nothing_planned()).color(ui.visuals().weak_text_color()),
                    );
                }
                for row in &agenda.tomorrow {
                    agenda_row(ui, &t, tones, (row, agenda.hides_checked), actions);
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
                        agenda_row(ui, &t, tones, (row, agenda.hides_checked), actions);
                    }
                });
            }
        });
        if output.state.offset.y != self.scrolled {
            self.scrolled = output.state.offset.y;
            ui.ctx().request_repaint();
        }
    }

    /// The dialogs of the Profile page (ADR 0017), as the Sync menu opens
    /// them (#145): `Page::Profile` the own Profile's picture, name and
    /// short code and its card as a link or a QR (#107), and "Which one
    /// is you?" while it waits; `Page::People` People, with "Add a person
    /// from a link"; `Page::Backups` the Backups and recovery. While a new
    /// picture's square is picked, that alone.
    fn profile_pane(&mut self, ui: &mut Ui, tones: Tones, actions: &mut Vec<Action>, page: Page) {
        let t = self.t;
        let text = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        let mut asked = Vec::new();
        if page == Page::Profile
            && let Some(square) = self.profile_view.square.as_mut()
        {
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
        let profile_newer = self.view.profile_newer;
        let people_newer = &self.view.people_newer;
        let question = self.view.profile_question.is_some();
        ui.heading(RichText::new(page_title(&t, page)).color(text));
        ui.add_space(8.0);
        egui::ScrollArea::vertical()
            .id_salt(("profile", page_title(&t, page)))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 16.0;
                ui.add_space(4.0);
                match page {
                    Page::Profile => {
                        embossed(ui, tones, false, None, |ui| {
                            let shown = (profile, profile_newer);
                            profile_view::header(ui, &t, view, shown, code, &mut asked);
                        });
                        if profile.is_some() {
                            embossed(ui, tones, false, None, |ui| {
                                profile_view::card_section(ui, &t, card_form.saved, &mut asked);
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
                    }
                    Page::Backups => {
                        embossed(ui, tones, false, None, |ui| {
                            let mut backup = Vec::new();
                            backup_view::card(ui, &t, store, wall, backup_form, &mut backup);
                            actions.extend(backup.into_iter().map(Action::Backup));
                        });
                    }
                    // People (#52): every Profile kept here, each opening
                    // it; the blocked ones (#90) in a fold of their own,
                    // closed until opened, which this Device remembers.
                    _ => {
                        embossed(ui, tones, false, None, |ui| {
                            if people.is_empty() && blocked.is_empty() {
                                ui.label(
                                    RichText::new(t.profile_people_empty()).small().color(weak),
                                );
                            }
                            for person in people {
                                let newer = people_newer.contains(&person.id);
                                people_row(ui, &t, pictures, (person, newer), &mut people_asked);
                            }
                            if !blocked.is_empty() {
                                let fold = egui::CollapsingHeader::new(
                                    RichText::new(t.people_blocked()).color(text),
                                )
                                .id_salt("people blocked")
                                .open(Some(blocked_open))
                                .show(ui, |ui| {
                                    for person in blocked {
                                        let shown = (person, false);
                                        people_row(ui, &t, pictures, shown, &mut people_asked);
                                    }
                                });
                                if fold.header_response.clicked() {
                                    actions.push(Action::BlockedOpen(!blocked_open));
                                }
                            }
                            ui.separator();
                            profile_view::add_person(ui, &t, card_form, &mut asked);
                        });
                    }
                }
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
        // The link not set: the field has the keyboard again.
        match &self.dialog {
            Some(Dialog::Link { amend: true, .. }) => self.amend_focus = true,
            Some(Dialog::Link {
                section: Some(section),
                ..
            }) => {
                if let Some(field) = self.section_texts.get_mut(section) {
                    field.focus = true;
                }
            }
            Some(Dialog::Link { .. }) => self.text_focus = true,
            _ => {}
        }
        let behind = self.dialog_behind.take();
        if let Some(Dialog::Person { id, name, .. }) = self.dialog.take() {
            self.store.set_override_name(&id, &name)?;
            self.person_square = None;
            self.dialog = behind;
            if let Some(Dialog::SentItem { opened, .. }) = &mut self.dialog {
                if self.store.is_blocked(&opened.sender.id)? {
                    self.dialog = None;
                    self.announce_not_taken(OpenError::FromBlocked);
                } else if opened.trust != SenderTrust::Unknown {
                    opened.sender = self.store.person_shown(&opened.sender.id)?;
                }
            }
            // A block drops the offer with it (ADR 0021).
            if let Some(Dialog::AreaOffer { held, .. }) = &mut self.dialog {
                if self.store.is_blocked(&held.sender.id)? {
                    self.dialog = None;
                    self.announce_offer_not_taken(OfferError::FromBlocked);
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
            PersonAction::SendBack => return self.send_back(),
            _ => {}
        }
        let Some(Dialog::Person {
            id,
            comparing,
            name,
            asking,
            ..
        }) = &mut self.dialog
        else {
            return Ok(());
        };
        // An Override a newer version of the app saved is changed only
        // there (ADR 0034), whatever a stale form still asks.
        let locked = self.store.override_is_newer(id)?;
        match asked {
            PersonAction::Rename(_) | PersonAction::TheirPicture if locked => {}
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
                    None => self.announce_picture_unreadable(),
                }
            }
            PersonAction::UseSquare
            | PersonAction::CancelSquare
            | PersonAction::TheirNotes
            | PersonAction::SendChanges
            | PersonAction::SendBack => {}
            PersonAction::Block => {
                *asking = Some(Asking::Block(self.store.areas_to_leave_for_blocking(id)?));
            }
            PersonAction::Unblock => *asking = Some(Asking::Unblock),
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
        if !take || self.store.override_is_newer(id)? {
            return Ok(());
        }
        match square.make() {
            Some(picture) => self.store.set_override_picture(id, Some(&picture))?,
            None => self.announce_picture_unreadable(),
        }
        Ok(())
    }

    /// Does what was asked of the own Profile in the Profile dialog.
    fn profile_action(&mut self, asked: ProfileAction) -> oldbrainfix_core::Result<()> {
        let own = OwnProfile::new(&self.store);
        // A Profile a newer version of the app saved is changed only there
        // (ADR 0034), whatever a stale form still asks.
        let locked = own.is_newer()?;
        match asked {
            ProfileAction::Rename(_) | ProfileAction::Initials | ProfileAction::UseSquare
                if locked =>
            {
                self.profile_view.square = None;
            }
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
                    None => self.announce_picture_unreadable(),
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
                    None => self.announce_picture_unreadable(),
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
            ProfileAction::SendCard => {
                if self.save_card(false)? {
                    self.card_form.saved = true;
                }
            }
            ProfileAction::ShowQr => match self.store.profile_card(false)? {
                Some(link) => self.dialog = Some(Dialog::ProfileQr { link }),
                None => self.announce_card_not_made(),
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
        list_scroll(ui, "sent items waiting", |ui| {
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
        list_scroll(ui, "shared areas waiting", |ui| {
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
        let all_day_text = &mut self.all_day_text;
        let all_day_from = settings
            .all_day_from()
            .unwrap_or(oldbrainfix_core::settings::DEFAULT_ALL_DAY_FROM);
        let hotkey_text = &mut self.hotkey_text;
        let stored_hotkey = settings
            .capture_hotkey()
            .unwrap_or_else(|_| oldbrainfix_core::settings::DEFAULT_CAPTURE_HOTKEY.into());
        let hotkey_status = self.hotkey.as_ref().map(|h| h.borrow().status());
        let speech = self.speech.as_ref();
        let ai_url = self.ai_port.as_ref().map(AiPort::url);
        let ai_waiting = self.ai_port.is_none() && self.view.ai_task.is_some();
        let ai_copied = self.ai_copied;
        let ai_reach = self.store.ai_reach().unwrap_or_default();
        // Looked for every frame while it matters: Docker started
        // meanwhile shows at once.
        let ai_no_bridge = self.ai_port.is_none()
            && ai_reach == Reach::Docker
            && cfg!(target_os = "linux")
            && docker_bridge().is_none();
        let units = settings.chosen_unit_system().ok().flatten();
        let region_units = oldbrainfix_core::tally_units::system_for_region(self.region.as_deref());
        let streaks = settings.streaks_on().unwrap_or(true);
        let hides_checked = settings.agenda_hides_checked().unwrap_or(false);
        let trashes_done = PersonSettings::new(&self.store)
            .trashes_done_reminder()
            .unwrap_or(false);
        let quick = self.view.capture_quick;
        egui::ScrollArea::vertical()
            .id_salt("settings")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 16.0;
                ui.add_space(4.0);
                embossed(ui, tones, false, None, |ui| {
                    ui.label(RichText::new(t.settings_computer()).strong().color(text));
                    let mut on = alerts;
                    if theme::checkbox(ui, &mut on, RichText::new(t.alerts_switch()).color(text))
                        .changed()
                    {
                        actions.push(Action::Setting(Setting::Alerts(on)));
                    }
                    ui.label(RichText::new(t.alerts_detail()).small().color(weak));
                    let mut on = autostart;
                    if theme::checkbox(ui, &mut on, RichText::new(t.autostart_switch()).color(text))
                        .changed()
                    {
                        actions.push(Action::Setting(Setting::Autostart(on)));
                    }
                    ui.label(RichText::new(t.autostart_detail()).small().color(weak));
                });
                let ai_shown = ai_view::PortShown {
                    url: ai_url.as_deref(),
                    waiting: ai_waiting,
                    copied: ai_copied,
                    reach: ai_reach,
                    no_bridge: ai_no_bridge,
                };
                if let Some(asked) = ai_view::port_card(ui, &t, tones, ai_shown) {
                    actions.push(Action::Ai(asked));
                }
                embossed(ui, tones, false, None, |ui| {
                    if ui
                        .button(RichText::new(t.about_title()).color(text))
                        .clicked()
                    {
                        actions.push(Action::PageInto(Page::About));
                    }
                    ui.label(RichText::new(t.about_settings_detail()).small().color(weak));
                });
                embossed(ui, tones, false, None, |ui| {
                    quick_capture_card(ui, &t, hotkey_text, &stored_hotkey, hotkey_status, actions);
                });
                embossed(ui, tones, false, None, |ui| {
                    quick_note_card(ui, &t, &quick, actions);
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
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(t.all_day_from()).color(text));
                        let field = ui.add(
                            TextEdit::singleline(all_day_text)
                                .id(Id::new("all day from"))
                                .hint_text(t.time_hint())
                                .text_color(text)
                                .desired_width(60.0),
                        );
                        if field.changed()
                            && let Some(time) = ClockTime::parse(all_day_text.trim())
                            && time != all_day_from
                        {
                            actions.push(Action::Setting(Setting::AllDayFrom(time)));
                        }
                    });
                    if ClockTime::parse(all_day_text.trim()).is_none() {
                        ui.label(
                            RichText::new(t.time_invalid()).color(ui.visuals().error_fg_color),
                        );
                    }
                    ui.label(RichText::new(t.all_day_from_detail()).small().color(weak));
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
                    if theme::checkbox(ui, &mut on, RichText::new(t.quiet_hours()).color(text))
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
                embossed(ui, tones, false, None, |ui| {
                    ui.label(RichText::new(t.settings_agenda()).strong().color(text));
                    let mut on = hides_checked;
                    if theme::checkbox(
                        ui,
                        &mut on,
                        RichText::new(t.hide_checked_on_agenda()).color(text),
                    )
                    .changed()
                    {
                        actions.push(Action::Setting(Setting::HideChecked(on)));
                    }
                    ui.label(
                        RichText::new(t.hide_checked_on_agenda_detail_device())
                            .small()
                            .color(weak),
                    );
                    let mut on = trashes_done;
                    if theme::checkbox(
                        ui,
                        &mut on,
                        RichText::new(t.trash_done_reminder()).color(text),
                    )
                    .changed()
                    {
                        actions.push(Action::Setting(Setting::TrashDoneReminder(on)));
                    }
                    ui.label(
                        RichText::new(t.trash_done_reminder_detail())
                            .small()
                            .color(weak),
                    );
                });
                embossed(ui, tones, false, None, |ui| {
                    ui.label(RichText::new(t.settings_history()).strong().color(text));
                    let mut on = streaks;
                    if theme::checkbox(
                        ui,
                        &mut on,
                        RichText::new(t.show_streaks_device()).color(text),
                    )
                    .changed()
                    {
                        actions.push(Action::Setting(Setting::Streaks(on)));
                    }
                    ui.label(RichText::new(t.show_streaks_detail()).small().color(weak));
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

    /// The card's text next to the pointer while a card is dragged, and
    /// how many go while several are (#148).
    fn dragged(&self, ctx: &egui::Context, tones: Tones) {
        let Some(dragged) = egui::DragAndDrop::payload::<DraggedNotes>(ctx) else {
            return;
        };
        let Some(pos) = ctx.pointer_interact_pos() else {
            return;
        };
        let Some(first) = dragged.0.first() else {
            return;
        };
        let Some(placed) = self.view.notes.iter().find(|p| &p.note.id == first) else {
            return;
        };
        let count = dragged.0.len();
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
                        if count > 1 {
                            let many = self.t.notes_count(count as i64);
                            ui.label(RichText::new(many).strong().color(text));
                        }
                        ui.label(RichText::new(placed.note.visible_text()).color(text));
                    });
            });
    }
}

/// Selects the Area or Topic `place` when its row is clicked, and files
/// the cards dropped on it there, all those dragged together (#148).
fn place_row(resp: &egui::Response, place: &str, actions: &mut Vec<Action>) {
    if resp.clicked() {
        actions.push(Action::Select(Selection::Place(place.to_string())));
    }
    if let Some(dragged) = resp.dnd_release_payload::<DraggedNotes>() {
        actions.push(Action::File {
            notes: dragged.0.clone(),
            place: place.to_string(),
        });
    }
}

/// What the Note menu may do to the Note shown (#145), by its id.
struct NoteMenu {
    id: String,
    changes: bool,
    schedules: bool,
    frog: bool,
    session: bool,
    amends: bool,
    checklist: bool,
    shopping: bool,
    tally: bool,
}

/// The entries of a menu of the menu bar: each label with its keys, if
/// any, at its end, greyed unless enabled; a pick asks its action and
/// closes the menu, also one by keyboard or screen reader.
fn menu_entries<const N: usize>(
    ui: &mut Ui,
    items: [(&str, Option<String>, bool, Action); N],
    actions: &mut Vec<Action>,
) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    for (label, keys, enabled, action) in items {
        let mut button = egui::Button::new(RichText::new(label).color(text));
        if let Some(keys) = keys {
            button = button.shortcut_text(RichText::new(keys).color(weak));
        }
        if ui.add_enabled(enabled, button).clicked() {
            actions.push(action);
            ui.close();
        }
    }
}

/// One level of the path above the list, or its sign between two (#145):
/// drawn, so a screen reader hears the places as the sidebar and the
/// heading name them, and a level above as the way `up` there, clicked
/// when it says so. The place shown leads nowhere.
fn crumb(ui: &mut Ui, name: &str, colour: Color32, up: Option<&str>) -> bool {
    let galley = ui
        .painter()
        .layout_no_wrap(name.to_string(), FontId::proportional(14.0), colour);
    let sense = if up.is_some() {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(galley.size(), sense);
    if let Some(label) = up {
        resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, label));
    }
    if ui.is_rect_visible(rect) {
        ui.painter().galley(rect.min, galley, colour);
        if up.is_some() && resp.hovered() {
            let line = egui::Stroke::new(1.0, colour);
            ui.painter().hline(rect.x_range(), rect.bottom(), line);
        }
    }
    if up.is_none() {
        return false;
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}

/// The title of the dialog `page` (#145).
fn page_title(t: &L10n, page: Page) -> &'static str {
    match page {
        Page::Settings => t.settings_title(),
        Page::About => t.about_title(),
        Page::Licences => t.open_source_licenses(),
        Page::Profile => t.profile_title(),
        Page::People => t.people_title(),
        Page::Backups => t.profile_backups(),
        Page::Sync => t.sync_title(),
        Page::Pair => t.menu_pair(),
    }
}

/// A square button that shows `icon`, called `label`.
pub(crate) fn icon_button(ui: &mut Ui, icon: char, label: &str, colour: Color32) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), Sense::click());
    resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, label));
    theme::icon_button_back(ui, &resp, rect);
    icons::paint(ui.painter(), rect.shrink(2.0), icon, colour);
    resp.on_hover_text(label)
}

/// Gives the text field `field` the name a screen reader says for it,
/// which its hint no longer gives once it holds text.
pub(crate) fn name_field(ui: &Ui, field: &egui::Response, name: &str) {
    ui.ctx()
        .accesskit_node_builder(field.id, |node| node.set_label(name));
}

/// The heading of a dialog.
/// One change of a Note's History (#193): what it changed, who made it
/// and when, the values before and after it, and "Use this version"
/// where it brings an old text back.
fn note_change(
    ui: &mut Ui,
    t: &L10n,
    (i, row): (usize, &note_history_view::ChangeRow),
    (text, weak): (Color32, Color32),
    actions: &mut Vec<Action>,
) {
    ui.vertical(|ui| {
        ui.label(RichText::new(&row.title).strong().color(text));
        ui.label(RichText::new(&row.said).small().color(weak));
        if let Some(before) = &row.before {
            ui.label(RichText::new(t.note_history_before()).small().color(weak));
            ui.add(egui::Label::new(RichText::new(before).color(text)).wrap());
        }
        if let Some(after) = &row.after {
            if row.before.is_some() {
                ui.label(RichText::new(t.note_history_after()).small().color(weak));
            }
            ui.add(egui::Label::new(RichText::new(after).color(text)).wrap());
        }
        if row.may_use
            && ui
                .button(RichText::new(t.use_this_version()).color(text))
                .clicked()
        {
            actions.push(Action::UseVersion(i));
        }
    });
    ui.separator();
}

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
/// with the details to the developer that the user still sends. Whether
/// it was closed.
pub(crate) fn failure_form(
    ui: &mut Ui,
    t: &L10n,
    heading: &str,
    (cause, fix): (&str, &str),
    details: &str,
) -> bool {
    let mut closed = false;
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
            closed = true;
        }
        if ui.button(RichText::new(t.report()).color(text)).clicked() {
            ui.ctx()
                .open_url(egui::OpenUrl::new_tab(report_url(details)));
            closed = true;
        }
    });
    closed
}

/// What a job announced at the top did (UI laws, "Announcements"):
/// `heading` over the full `details`, and Close. Whether it was closed.
pub(crate) fn done_form(ui: &mut Ui, t: &L10n, heading: &str, details: &str) -> bool {
    title(ui, heading);
    let text = ui.visuals().text_color();
    ui.label(RichText::new(details).color(text));
    ui.button(RichText::new(t.close()).color(text)).clicked()
}

/// What an Import made, `details` under "Import finished.", with Close
/// and "Open the Area"; returns whether each was clicked.
fn imported_form(ui: &mut Ui, t: &L10n, details: &str) -> (bool, bool) {
    title(ui, t.import_finished());
    let text = ui.visuals().text_color();
    ui.label(RichText::new(details).color(text));
    ui.horizontal(|ui| {
        let close = ui.button(RichText::new(t.close()).color(text)).clicked();
        let open = ui
            .button(RichText::new(t.import_open_area()).color(text))
            .clicked();
        (close, open)
    })
    .inner
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

/// What the settings of this Device start with that keep a Shopping
/// List's struck Lines, or the description of a Note of a kind, unfolded
/// on its card, the Note's id after it, as the phone names them (#189).
const STRUCK_SHOWN: &str = "shoppingStruckShown:";
const DESCRIPTION_UNFOLDED: &str = "descriptionUnfolded:";

/// The setting that keeps the struck Lines of the Shopping List `note`
/// unfolded on its card.
fn struck_shown_setting(note: &str) -> String {
    format!("{STRUCK_SHOWN}{note}")
}

/// The setting that keeps the description of the Note `note` unfolded on
/// its card.
fn description_unfolded_setting(note: &str) -> String {
    format!("{DESCRIPTION_UNFOLDED}{note}")
}

/// The setting of this Device that keeps the Notes of the list `key`
/// whose card shows the view the list does not start in, joined by commas
/// (#189).
fn switched_setting(key: &str) -> String {
    format!("switchedNotes:{key}")
}

/// The setting of this Device that keeps the Note list `key` compact
/// (#114), the phone's; none while it shows in full, the default.
fn compact_setting(key: &str) -> String {
    format!("compactNotes:{key}")
}

/// Which Note list `selection` shows, by the phone's names, each keeping
/// its own compact/detailed choice (#114): all Notes, or a person's while
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
    (person, newer): (&Profile, bool),
    asked: &mut Vec<String>,
) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let name = profile_view::display_name(t, &person.name).to_string();
    ui.horizontal(|ui| {
        let open = t.person_open(&name);
        let picture = profile_view::person_button(ui, pictures, person, 32.0, &open);
        let line = ui.add(egui::Button::new(RichText::new(&name).color(text)).frame(false));
        if picture.clicked() || line.clicked() {
            asked.push(person.id.clone());
        }
        // Saved by a newer version of the app (ADR 0034).
        if newer {
            ui.label(RichText::new(t.newer_version_needed()).small().color(weak));
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

/// A confirm button for a destructive step: its verb on the error
/// colour (UI laws, #218).
fn destructive(ui: &mut Ui, label: &str) -> bool {
    theme::destructive(ui, true, label).clicked()
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
        if theme::primary(ui, ready, verb).clicked() {
            actions.push(done);
        }
    });
}

/// Asks for the address of a link (#127): one already there is filled
/// in, `linked`, and taken off when left empty. Enter takes it.
fn link_form(
    ui: &mut Ui,
    t: &L10n,
    (address, focus): (&mut String, bool),
    linked: bool,
    actions: &mut Vec<Action>,
) {
    title(ui, t.format_link());
    let field = focused_field(ui, (&mut *address, focus), "link", t.link_address());
    if linked {
        let weak = ui.visuals().weak_text_color();
        ui.label(RichText::new(t.link_remove_hint()).small().color(weak));
    }
    let ready = linked || !address.trim().is_empty();
    if ready && field.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
        actions.push(Action::SetLink);
    }
    ui.horizontal(|ui| {
        cancel(ui, t, actions);
        if theme::primary(ui, ready, t.link_set()).clicked() {
            actions.push(Action::SetLink);
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
            ui.label(
                RichText::new(t.sent_item_codes_differed()).color(ui.visuals().hyperlink_color),
            );
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
        theme::checkbox(ui, cancelling, RichText::new(t.send_cancel()).color(text));
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

/// Why a Sent Item was refused as `error` says, and what to do, as the
/// phone's `_refused`.
fn refused(t: &L10n, error: OpenError) -> (&'static str, &'static str) {
    match error {
        OpenError::NotForUs => (t.sent_item_not_for_us(), t.sent_item_ask_again_fix()),
        OpenError::NotASentItem => (t.sent_item_unreadable(), t.sent_item_ask_again_fix()),
        OpenError::NeedsUpdate => (t.sent_item_needs_update(), t.update_open_again_fix()),
        OpenError::NotFromSender => (t.sent_item_not_from_sender(), t.sent_item_ask_again_fix()),
        OpenError::FromBlocked => (t.sent_item_from_blocked(), t.blocked_fix()),
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
            ui.label(
                RichText::new(t.sent_item_codes_differed()).color(ui.visuals().hyperlink_color),
            );
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
            ui.label(RichText::new(t.sent_item_update()).color(ui.visuals().hyperlink_color));
        }
        SentItemState::Cancel => {
            ui.label(
                RichText::new(
                    t.sent_item_cancelled(&text_format::visible(&item.text_format, &item.text)),
                )
                .strong()
                .color(ui.visuals().hyperlink_color),
            );
            ui.label(RichText::new(t.sent_item_cancel_detail()).color(text));
        }
        SentItemState::UpToDate => {
            ui.label(RichText::new(t.sent_item_already()).color(ui.visuals().hyperlink_color));
        }
        SentItemState::NothingToCancel => {
            ui.label(
                RichText::new(t.sent_item_nothing_to_cancel()).color(ui.visuals().hyperlink_color),
            );
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
                    line(
                        t.sent_item_text_now(&text_format::visible(&item.text_format, &item.text)),
                    );
                    if changes.text_edited {
                        line(t.sent_item_text_yours().to_string());
                    }
                }
                if changes.makes_checklist {
                    line(t.sent_item_makes_checklist().to_string());
                }
                if changes.makes_shopping_list {
                    line(t.sent_item_makes_shopping_list().to_string());
                }
                if changes.makes_plain {
                    line(t.sent_item_makes_plain().to_string());
                }
                for l in &changes.lines_added {
                    line(t.sent_item_line_added(&l.text));
                }
                for r in &changes.lines_renamed {
                    line(t.sent_item_line_renamed(&r.before, &r.after));
                }
                for l in &changes.lines_removed {
                    line(t.sent_item_line_removed(&l.text));
                }
                for s in &changes.sections_added {
                    line(t.sent_item_section_added(&text_format::visible(&s.format, &s.text)));
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
            // Formatted as it reads; its tags are not followed before it
            // is taken, its links open (#127).
            let shown = (0, item.text.len());
            // Its boxes as the sender left them, never ticked here.
            let unticked = BoxLook::default();
            let spots = text_marks_view::formatted_label(
                ui,
                (&item.text, &item.text_format, &unticked),
                shown,
                (None, text),
                false,
            );
            let of = Id::new("sent text");
            if let Some(at) = text_marks_view::table_clicks(ui, t, of, &spots.tables) {
                actions.push(Action::OpenTable(Opening {
                    note: None,
                    text: item.text.clone(),
                    at,
                    cell: None,
                    tags: false,
                    boxes: None,
                }));
            }
            text_marks_view::link_clicks(ui, of, &spots.links);
            if !full {
                return;
            }
            // The Lines of a Checklist or a Shopping List, to look at,
            // not to check; a Shopping List's with its reappear time.
            if let Some(lines) = &item.lines {
                checklist_view::preview(ui, (t, tones), lines);
            }
            if !item.routines.is_empty() {
                ui.label(RichText::new(t.routines_heading()).strong().color(text));
                for r in &item.routines {
                    ui.label(RichText::new(schedule_text(t, &r.schedule)).color(text));
                }
            }
            for (i, section) in item.sections.iter().enumerate() {
                ui.separator();
                let shown = (0, section.text.len());
                let spots = text_marks_view::formatted_label(
                    ui,
                    (&section.text, &section.format, &unticked),
                    shown,
                    (None, text),
                    false,
                );
                let of = Id::new(("sent section", i));
                if let Some(at) = text_marks_view::table_clicks(ui, t, of, &spots.tables) {
                    actions.push(Action::OpenTable(Opening {
                        note: None,
                        text: section.text.clone(),
                        at,
                        cell: None,
                        tags: false,
                        boxes: None,
                    }));
                }
                text_marks_view::link_clicks(ui, of, &spots.links);
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
    banded_clickable_card(ui, tones, (id, label), None, content)
}

/// A [`clickable_card`] with an Area's colour `band` down its left side,
/// as an Agenda row has it.
pub(crate) fn banded_clickable_card(
    ui: &mut Ui,
    tones: Tones,
    (id, label): (Id, &str),
    band: Option<Color32>,
    content: impl FnOnce(&mut Ui),
) -> bool {
    let card = embossed(ui, tones, false, band, content);
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
fn held_sent_item_form(ui: &mut Ui, t: &L10n, held: &HeldSentItem, actions: &mut Vec<Action>) {
    let text = ui.visuals().text_color();
    let name = profile_view::display_name(t, &held.sender.name);
    title(ui, &t.sent_item_from(name));
    ui.label(RichText::new(t.sent_item_stranger()).color(ui.visuals().hyperlink_color));
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
        // A role a newer version of the app gave is kept as it is, and
        // nothing here changes it (ADR 0034).
        let role = match person.role {
            Some(AreaRole::Manager) => t.role_manager(),
            Some(AreaRole::Member) => t.role_member(),
            None => t.newer_version_needed(),
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
            let (label, next) = match person.role {
                _ if !manages || own => return,
                Some(AreaRole::Member) => (t.make_manager(), AreaRole::Manager),
                Some(AreaRole::Manager) => (t.make_member(), AreaRole::Member),
                None => return,
            };
            if ui.button(RichText::new(label).color(text)).clicked() {
                actions.push(Action::SetRole {
                    area: area.to_string(),
                    person: person.id.clone(),
                    role: next,
                });
            }
            if theme::destructive(ui, true, t.remove_person()).clicked() {
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
                if merge_row(ui, tones, text).clicked() {
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

/// The Settings' card on what the quick note, its dialog in the window
/// and the popup, shows besides its text (#142): one switch for each of the Note
/// card's buttons and the format bar, off until switched on, and the
/// Nudge buttons, on until switched off.
fn quick_note_card(ui: &mut Ui, t: &L10n, quick: &QuickNote, actions: &mut Vec<Action>) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    ui.label(RichText::new(t.settings_quick_note()).strong().color(text));
    ui.label(
        RichText::new(t.settings_quick_note_detail_device())
            .small()
            .color(weak),
    );
    let switches = [
        (QuickNoteControl::FileInto, quick.file_into, t.file_into()),
        (
            QuickNoteControl::Schedule,
            quick.schedule,
            t.quick_note_schedule(),
        ),
        (
            QuickNoteControl::Shopping,
            quick.shopping,
            t.add_to_shopping_list(),
        ),
        (
            QuickNoteControl::Importance,
            quick.importance,
            t.importance(),
        ),
        (
            QuickNoteControl::FormatBar,
            quick.format_bar,
            t.quick_note_format_bar(),
        ),
    ];
    for (control, shows, label) in switches {
        let mut on = shows;
        if theme::checkbox(ui, &mut on, RichText::new(label).color(text)).changed() {
            actions.push(Action::Setting(Setting::QuickNote(control, on)));
        }
    }
    let mut on = quick.nudge_buttons;
    if theme::checkbox(
        ui,
        &mut on,
        RichText::new(t.quick_note_nudge_buttons()).color(text),
    )
    .changed()
    {
        actions.push(Action::Setting(Setting::QuickNoteNudgeButtons(on)));
    }
    ui.label(
        RichText::new(t.quick_note_nudge_buttons_detail())
            .small()
            .color(weak),
    );
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
        Some(note) if !note.text.trim().is_empty() => note.visible_text().trim().to_string(),
        Some(note) => l10n::wordless(t, note.wordless()).to_string(),
        None => t.note_gone().to_string(),
    }
}

/// How far an entry under an Agenda group's heading is indented, beside
/// the item spacing: its box stands where the heading's words begin, as
/// far in as a box and the room after it (#208).
const ENTRY_INDENT: f32 = 28.0 + 4.0;

/// How much further a Line under another Line is indented.
const ENTRY_STEP: f32 = 20.0;

/// The room above and below an Agenda row's content in its card, tighter
/// than a card's elsewhere (#208).
const AGENDA_INSIDE: i8 = 6;

/// The box of an Agenda heading that only shows Done, a Session's
/// (#208): drawn as [`check`] in its usual colours, but nothing clicks
/// it.
fn shown_check(ui: &mut Ui, tones: Tones, done: bool, label: &str) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), Sense::hover());
    resp.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, false, done, label));
    let (icon, colour) = if done {
        (icons::CHECKED, tones.accent)
    } else {
        (icons::UNCHECKED, ui.visuals().text_color())
    };
    icons::paint(ui.painter(), rect.shrink(2.0), icon, colour);
    resp
}

/// The icon that leads an Agenda heading without a box, in a box's room.
fn heading_icon(ui: &mut Ui, icon: char) {
    let weak = ui.visuals().weak_text_color();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), Sense::hover());
    icons::paint(ui.painter(), rect.shrink(4.0), icon, weak);
}

/// What was clicked on an entry of an Agenda group.
#[derive(Debug, Clone, Copy, Default)]
struct EntryClicks {
    boxed: bool,
    words: bool,
}

/// One entry under an Agenda group's heading (#208): a Line, a Shopping
/// List's Line or a Session's pick, all alike: its box, `checked`, and its
/// words, struck through and dimmed when `struck`, indented under the
/// heading, `depth` steps further for a Line under another. The box
/// clicks while `boxes`, the words while `opens`, as a button named by
/// them; `id` keeps their click apart from every other's.
fn agenda_entry(
    ui: &mut Ui,
    tones: Tones,
    (id, words, depth): (Id, &str, usize),
    (checked, struck): (bool, bool),
    (boxes, opens): (bool, bool),
) -> EntryClicks {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    ui.horizontal(|ui| {
        let spacing = ui.spacing().item_spacing.x;
        ui.add_space(ENTRY_INDENT + spacing + depth as f32 * ENTRY_STEP);
        let boxed = ui
            .add_enabled_ui(boxes, |ui| check(ui, tones, checked, words))
            .inner
            .clicked();
        ui.add_space(4.0);
        let mut shown = RichText::new(words).size(15.0).color(text);
        if struck {
            shown = shown.strikethrough().color(weak);
        }
        let label = ui.add(egui::Label::new(shown).truncate());
        let sense = if opens {
            Sense::click()
        } else {
            Sense::hover()
        };
        let mut open = ui.interact(label.rect, id, sense);
        open.widget_info(|| WidgetInfo::labeled(WidgetType::Button, opens, words));
        if opens {
            open = open.on_hover_cursor(egui::CursorIcon::PointingHand);
        }
        EntryClicks {
            boxed,
            words: open.clicked(),
        }
    })
    .inner
}

/// One Occurrence on the Agenda as a group (#208): the heading an
/// embossed card's top, with the box to check it off, its time, its
/// Note's text, its Area's colour and icon, and its ⋮; a click on the
/// text opens its Routine, a right-click on it the menu of ⋮. Under it a
/// Session's picks today, each with its box showing whether it has an
/// entry, its box and words opening its entry form, and "Another one"
/// under each without one; how many it picks on later days. A Session's
/// own box only shows Done: once every pick has an entry it checks
/// itself, and nothing else checks it. A Routine spawned from a
/// Checklist lists its Lines under it, checked for this Occurrence; once
/// every Line is checked, its box stays checked too (#121). Where
/// `hides_checked` (#217), the checked Lines and the picks with an entry
/// are left out.
fn agenda_row(
    ui: &mut Ui,
    t: &L10n,
    tones: Tones,
    (row, hides_checked): (&Row, bool),
    actions: &mut Vec<Action>,
) {
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
    agenda_card(ui, tones, colour, |ui| {
        // Done from the entries of every pick, or from every Line
        // checked, stays checked.
        let complete = matches!(&row.session, Some(SessionPart::Today(s)) if s.today.complete())
            || row
                .lines
                .as_ref()
                .is_some_and(|(list, _)| list.all_checked());
        ui.horizontal(|ui| {
            let label = format!("{}: {time} {title}", t.mark_done());
            if row.session.is_some() {
                // Its picks make it Done (#208).
                shown_check(ui, tones, done || complete, &label);
            } else {
                let toggled = ui
                    .add_enabled_ui(!complete, |ui| check(ui, tones, done, &label))
                    .inner;
                if toggled.clicked() {
                    actions.push(Action::Toggle(routine.id.clone(), occurrence));
                }
            }
            ui.add_space(4.0);
            let width = (ui.available_width() - 84.0).max(40.0);
            let body = ui.vertical(|ui| {
                ui.set_max_width(width);
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.label(RichText::new(&time).strong().color(text));
                let mut line = RichText::new(&title).size(15.0).color(text);
                if done {
                    line = line.strikethrough();
                }
                ui.add(egui::Label::new(line).truncate());
                match &row.session {
                    Some(SessionPart::Later(count)) => {
                        ui.label(RichText::new(t.session_picks(*count)).color(text));
                    }
                    Some(SessionPart::Today(shown)) if shown.today.picks.is_empty() => {
                        ui.label(RichText::new(t.session_nothing_to_pick()).color(text));
                    }
                    _ => {}
                }
            });
            let open = body
                .response
                .interact(Sense::click())
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            open.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &title));
            if open.clicked() {
                actions.push(Action::OpenRoutine(routine.id.clone()));
            }
            // The same menu from a right-click and from ⋮.
            let history = !routine.is_reminder();
            let mut picked = None;
            open.context_menu(|ui| picked = picked.or(occurrence_menu(ui, t, history)));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let more = icon_button(ui, icons::MORE_VERT, &t.occurrence_menu(&title), weak);
                egui::Popup::menu(&more)
                    .show(|ui| picked = picked.or(occurrence_menu(ui, t, history)));
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
            match picked {
                Some(OccurrencePick::History) => {
                    actions.push(Action::Select(Selection::History(routine.id.clone())));
                }
                Some(OccurrencePick::Share) => {
                    actions.push(Action::ShareRoutine(routine.id.clone()));
                }
                None => {}
            }
        });
        // The entries go under the heading, outside its click.
        if let Some(SessionPart::Today(shown)) = &row.session {
            session_picks(ui, t, tones, (shown, hides_checked), actions);
        } else if let Some((list, checks)) = &row.lines {
            // The Lines go there too, checked for this Occurrence.
            for line in agenda_lines(list, hides_checked) {
                let id = Id::new(("row line", &routine.id, occurrence.key(), &line.id));
                let shown = (id, line.text.as_str(), usize::from(line.in_group));
                let clicks = agenda_entry(
                    ui,
                    tones,
                    shown,
                    (line.checked, line.checked),
                    (*checks, true),
                );
                if clicks.boxed {
                    actions.push(Action::Checklist(ChecklistAction::Check(
                        line.id.clone(),
                        !line.checked,
                        list.on.clone(),
                    )));
                }
                if clicks.words {
                    actions.push(Action::OpenRoutine(routine.id.clone()));
                }
            }
        }
    });
}

/// Today's picks of a Session under its heading (#208), in the order
/// picked, each an entry with its box, checked once it has an entry and
/// struck through; its box and its words open its entry form where this
/// Device may; "Another one" goes under each without an entry while they
/// swap (the control goes below its text). Where `hides_checked`
/// (#217), a pick with an entry is left out.
fn session_picks(
    ui: &mut Ui,
    t: &L10n,
    tones: Tones,
    (shown, hides_checked): (&PicksShown, bool),
    actions: &mut Vec<Action>,
) {
    let text = ui.visuals().text_color();
    let today = &shown.today;
    let session = today.session.id();
    let occurrence = today.occurrence;
    for (i, pick) in today.picks.iter().enumerate() {
        if hides_checked && pick.done() {
            continue;
        }
        let words = shown.texts.get(i).map(String::as_str).unwrap_or_default();
        let opens = shown.opens.get(i).copied().unwrap_or(false);
        let id = Id::new(("session pick", session, occurrence.key(), &pick.note_id));
        let clicks = agenda_entry(
            ui,
            tones,
            (id, words, 0),
            (pick.done(), pick.done()),
            (opens, opens),
        );
        if clicks.boxed || clicks.words {
            actions.push(Action::Session(SessionAction::Open {
                session: session.to_string(),
                occurrence,
                note_id: pick.note_id.clone(),
                entry: pick.entries.last().map(|e| e.id.clone()),
            }));
        }
        if shown.swaps && !pick.done() {
            let another = ui.horizontal(|ui| {
                ui.add_space(ENTRY_INDENT + ui.spacing().item_spacing.x);
                icons::show(ui, icons::SWAP, 18.0, text);
                ui.button(RichText::new(t.session_another()).color(text))
            });
            if another.inner.clicked() {
                actions.push(Action::Session(SessionAction::Another {
                    session: session.to_string(),
                    occurrence,
                    note_id: pick.note_id.clone(),
                }));
            }
        }
    }
}

/// The heading of a Shopping List's or Checklist's group: its kind's
/// icon where a box would be, its `words`, a click on which opens the
/// Note `note_id`, and its Area's icon.
fn list_heading(
    ui: &mut Ui,
    (icon, area, colour): (char, Option<&Area>, Option<Color32>),
    (note_id, words, salt): (&str, &str, &str),
    actions: &mut Vec<Action>,
) {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    ui.horizontal(|ui| {
        heading_icon(ui, icon);
        ui.add_space(4.0);
        let width = (ui.available_width() - 32.0).max(40.0);
        let body = ui.vertical(|ui| {
            ui.set_max_width(width);
            ui.add(egui::Label::new(RichText::new(words).size(15.0).color(text)).truncate());
        });
        let open = ui
            .interact(body.response.rect, Id::new((salt, note_id)), Sense::click())
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        open.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, words));
        if open.clicked() {
            actions.push(Action::SelectNote(note_id.to_string()));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if let Some(icon) = area.and_then(|a| icons::area_icon(a.icon.as_deref())) {
                icons::show(ui, icon, 20.0, colour.unwrap_or(weak));
            }
        });
    });
}

/// A Shopping List's group today (#123, #208): its title as the heading,
/// with the cart and no box, and the Lines that came back today under it,
/// each with its box, which strikes it and takes it off the Agenda; or,
/// as chosen for it, the heading alone with how many are open. A click
/// on a heading's or Line's words opens the Note.
fn shopping_row(ui: &mut Ui, tones: Tones, row: &ShoppingRow, actions: &mut Vec<Action>) {
    let colour = row
        .area
        .as_ref()
        .and_then(|a| area_colour(a.colour.as_deref()));
    agenda_card(ui, tones, colour, |ui| {
        let icon = (icons::SHOPPING_CART, row.area.as_ref(), colour);
        list_heading(ui, icon, (&row.note_id, &row.text, "shopping row"), actions);
        for line in &row.lines {
            let id = Id::new(("shopping line", line.id.as_str()));
            let clicks = agenda_entry(
                ui,
                tones,
                (id, &line.text, 0),
                (false, false),
                (row.checks, true),
            );
            if clicks.boxed {
                let check = ShoppingAction::Check(line.id.clone(), true);
                actions.push(Action::Shopping(check));
            }
            if clicks.words {
                actions.push(Action::SelectNote(row.note_id.clone()));
            }
        }
    });
}

/// A Checklist's group today (#143, #208): its title as the heading,
/// with the list's icon and no box, and its open Lines under it, each
/// with its box, which checks it. A click on a heading's or Line's words
/// opens the Note.
fn checklist_row(ui: &mut Ui, tones: Tones, row: &ChecklistRow, actions: &mut Vec<Action>) {
    let colour = row
        .area
        .as_ref()
        .and_then(|a| area_colour(a.colour.as_deref()));
    agenda_card(ui, tones, colour, |ui| {
        let icon = (icons::CHECKLIST, row.area.as_ref(), colour);
        list_heading(
            ui,
            icon,
            (&row.note_id, &row.text, "checklist row"),
            actions,
        );
        for line in &row.lines {
            let id = Id::new(("checklist line", line.id.as_str()));
            let shown = (id, line.text.as_str(), usize::from(line.in_group));
            let clicks = agenda_entry(
                ui,
                tones,
                shown,
                (line.checked, line.checked),
                (row.checks, true),
            );
            if clicks.boxed {
                let check = ChecklistAction::Check(line.id.clone(), !line.checked, None);
                actions.push(Action::Checklist(check));
            }
            if clicks.words {
                actions.push(Action::SelectNote(row.note_id.clone()));
            }
        }
    });
}

/// What was picked in an Occurrence's menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OccurrencePick {
    History,
    Share,
}

/// An Occurrence's menu, from a right-click on it or its ⋮, as the
/// phone's: History for a Routine (`history`), and Share, which saves
/// its Routine as the whole series with its end, or its Reminder, as a
/// calendar file. What was picked.
fn occurrence_menu(ui: &mut Ui, t: &L10n, history: bool) -> Option<OccurrencePick> {
    let text = ui.visuals().text_color();
    let mut picked = None;
    if history
        && ui
            .button(RichText::new(t.history_title()).color(text))
            .clicked()
    {
        picked = Some(OccurrencePick::History);
    }
    if ui.button(RichText::new(t.share()).color(text)).clicked() {
        picked = Some(OccurrencePick::Share);
    }
    // A pick closes the menu, also one by keyboard or screen reader,
    // which no pointer click closes.
    if picked.is_some() {
        ui.close();
    }
    picked
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
/// line's icon, as the phone's `CaptureContextLines`: where, by its
/// Named Place, else its coordinates, the app it was shared in from,
/// the calendar events running and the Bluetooth devices connected;
/// each with the detail it shows.
fn context_lines(t: &L10n, context: &CaptureContext) -> Vec<(char, String, CapturedDetail)> {
    let mut lines = Vec::new();
    if context.location.is_some()
        && let Some(place) = where_captured(context)
    {
        lines.push((icons::CONTEXT_WHERE, place, CapturedDetail::Place));
    }
    if let Some(app) = &context.source_app {
        let line = t.context_source_app(app.name());
        lines.push((icons::CONTEXT_APP, line, CapturedDetail::App));
    }
    if !context.calendar_events.is_empty() {
        let events = context.calendar_events.join(", ");
        let line = t.context_during(&events);
        lines.push((icons::CONTEXT_CALENDAR, line, CapturedDetail::Event));
    }
    if !context.bluetooth_devices.is_empty() {
        let devices = context.bluetooth_devices.join(", ");
        let line = t.context_bluetooth(&devices);
        lines.push((icons::CONTEXT_BLUETOOTH, line, CapturedDetail::Device));
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

/// When the text of `section` was last edited and by whom (#183), as
/// the phone's `CaptureContextLines.ofSection` says it: no "by" for a
/// writer not known or a Profile without a name; `None` while it was
/// never edited.
fn section_edited(t: &L10n, section: &Section) -> Option<String> {
    let at = when(t, section.modified?);
    let name = section.edited_by.as_ref().map(|p| p.name.trim());
    Some(match name.filter(|n| !n.is_empty()) {
        Some(name) => t.section_edited_by(&at, name),
        None => t.section_edited(&at),
    })
}

/// The whole Capture Context of a Note or Section `captured` then, line
/// by line, in its Captured dialog, as the phone's `CaptureContextLines`:
/// when, then [`context_lines`]. Each detail `removable` says may go
/// ends in a Remove (ADR 0037), the time in none; the detail whose
/// Remove was clicked. Beside its text the same reads as one
/// [`capture_line`] (#170).
fn capture_context(
    ui: &mut Ui,
    t: &L10n,
    (captured, context): (DateTime<Utc>, &CaptureContext),
    removable: impl Fn(CapturedDetail) -> bool,
) -> Option<CapturedDetail> {
    let mut asked = None;
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 4.0;
        context_line(ui, icons::CONTEXT_WHEN, &when(t, captured));
        for (icon, line, detail) in context_lines(t, context) {
            if !removable(detail) {
                context_line(ui, icon, &line);
                continue;
            }
            // The Remove first, at the end of the line, so the line
            // wraps in what is left beside it.
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                let remove = theme::destructive(ui, true, t.remove());
                let label = t.remove_captured_label(detail.name());
                remove.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &label));
                if remove.clicked() {
                    asked = Some(detail);
                }
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
                    context_line(ui, icon, &line);
                });
            });
        }
    });
    asked
}

/// A Capture Context `captured` then as one line read beside its text
/// (#170), the phone's `captureLine`: when, then where (its Named Place,
/// else its coordinates), the calendar events running, the Bluetooth
/// devices connected and the app it was shared in from, joined by
/// " · ". A part captured the same local day as the one before it
/// (`after`) shows only its time; another day the date too.
fn capture_line(
    t: &L10n,
    (captured, context): (DateTime<Utc>, &CaptureContext),
    after: Option<DateTime<Utc>>,
) -> String {
    let local = captured.with_timezone(&Local);
    let same_day =
        after.is_some_and(|a| a.with_timezone(&Local).date_naive() == local.date_naive());
    let mut parts = vec![if same_day {
        local.format("%H:%M").to_string()
    } else {
        when(t, captured)
    }];
    parts.extend(where_captured(context));
    if !context.calendar_events.is_empty() {
        parts.push(context.calendar_events.join(", "));
    }
    if !context.bluetooth_devices.is_empty() {
        parts.push(context.bluetooth_devices.join(", "));
    }
    if let Some(app) = &context.source_app {
        parts.push(app.name().to_string());
    }
    parts.join(" · ")
}

/// A part's [`capture_line`] under its text, in the background (#170):
/// small and in the weak colour, wrapped only where it must, and a
/// Section's `edited` line (#183, [`section_edited`]) under it the same
/// way. A `clickable` one is a button, read as "Captured: <line>", that
/// opens the details; whether it was clicked.
fn capture_meta(ui: &mut Ui, t: &L10n, line: &str, edited: Option<&str>, clickable: bool) -> bool {
    let weak = ui.visuals().weak_text_color();
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        let label = egui::Label::new(RichText::new(line).size(12.0).color(weak)).wrap();
        let clicked = if clickable {
            let shown = ui
                .add(label.sense(Sense::click()))
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            let name = t.setting_badge_label(t.captured_heading(), line);
            shown.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &name));
            shown.clicked()
        } else {
            ui.add(label);
            false
        };
        if let Some(edited) = edited {
            ui.add(egui::Label::new(RichText::new(edited).size(12.0).color(weak)).wrap());
        }
        clicked
    })
    .inner
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
        if droppable && resp.dnd_hover_payload::<DraggedNotes>().is_some() {
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

/// A Note the merge picks as a row, as [`row`] draws one, its tags in the
/// accent colour: a click on one picks the Note, as anywhere on the row
/// (#126).
fn merge_row(ui: &mut Ui, tones: Tones, label: &str) -> egui::Response {
    let height = 30.0;
    let width = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, height), Sense::click());
    resp.widget_info(|| WidgetInfo::selected(WidgetType::SelectableLabel, true, false, label));
    if ui.is_rect_visible(rect) {
        if resp.hovered() {
            ui.painter()
                .rect_filled(rect, 8.0, tones.card.gamma_multiply(0.7));
        }
        let x = rect.left() + 8.0;
        let colours = (ui.visuals().text_color(), ui.visuals().hyperlink_color);
        let font = FontId::proportional(15.0);
        let mut job = text_marks_view::tag_job(label, (0, label.len()), font, colours);
        job.wrap.max_width = (rect.right() - x - 8.0).max(0.0);
        let text = ui.painter().layout_job(job);
        let y = rect.center().y - text.size().y / 2.0;
        ui.painter().galley(egui::pos2(x, y), text, colours.0);
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
    embossed_inside(ui, (tones, strong), (band, 14), content)
}

/// An Agenda row's card (#208): as [`embossed`], plain, with less room
/// above and below its content.
pub(crate) fn agenda_card(
    ui: &mut Ui,
    tones: Tones,
    band: Option<Color32>,
    content: impl FnOnce(&mut Ui),
) -> egui::Response {
    embossed_inside(ui, (tones, false), (band, AGENDA_INSIDE), content)
}

/// An embossed card with `inside` room above and below its content.
fn embossed_inside(
    ui: &mut Ui,
    (tones, strong): (Tones, bool),
    (band, inside): (Option<Color32>, i8),
    content: impl FnOnce(&mut Ui),
) -> egui::Response {
    let under = ui.painter().add(egui::Shape::Noop);
    let left = if band.is_some() { 22 } else { 16 };
    let inner = Frame::NONE
        .inner_margin(Margin {
            left,
            right: 16,
            top: inside,
            bottom: inside,
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

/// What the Note dialog's badges show (#160): the Note where it is filed,
/// its Routines and Reminders and how many of them a newer version saved,
/// its kind with whether it may go back to a plain Note, Show on the
/// Agenda of a Checklist or a Shopping List and Show ticks where they
/// apply, a Session's setup, what this Device may do to it and whether it
/// has an open Frog.
struct BadgesShown<'a> {
    placed: &'a Placed,
    routines: &'a [Routine],
    newer: usize,
    kind: &'a KindShown,
    on_agenda: Option<ListOnAgenda>,
    shopping_on_agenda: Option<ShoppingOnAgenda>,
    ticks: Option<bool>,
    /// A Session Note's setup, for its rule and weekdays' badges, and
    /// whether this Device may change them (#191).
    session: Option<(&'a SessionSetup, bool)>,
    may: NoteMay,
    frog_open: bool,
}

/// The kind of the Note open (#188), and the kinds this Device may make
/// it from its kind badge, in the order offered: a plain Note a
/// Checklist, a Shopping List, a Tally or a Session (#191), as it may make
/// it one; a list a plain Note again or the other list; a Session a plain
/// Note again; a Tally none.
#[derive(Debug, Clone, PartialEq)]
struct KindShown {
    kind: NoteKind,
    choices: Vec<NoteKind>,
}

impl KindShown {
    fn of(view: &View) -> KindShown {
        let kind = view.kind.unwrap_or(NoteKind::Plain);
        let Some((placed, _)) = &view.detail else {
            return KindShown {
                kind,
                choices: Vec::new(),
            };
        };
        let note = &placed.note;
        let may = view.may;
        let mut choices = Vec::new();
        if !note.is_trashed() {
            let plain = |list_plain: bool| list_plain.then_some(NoteKind::Plain);
            let switch = |to: NoteKind| view.switches_list.then_some(to);
            match kind {
                // A Code text becomes no other kind (#213).
                NoteKind::Plain if note.text_format == text_format::CODE => {}
                NoteKind::Plain => {
                    // As the Note menu offers them (#145).
                    let tally = may.amends && may.changes && !note.is_merged();
                    choices.extend(view.makes_checklist.then_some(NoteKind::Checklist));
                    choices.extend(view.makes_shopping.then_some(NoteKind::ShoppingList));
                    choices.extend(tally.then_some(NoteKind::Tally));
                    choices.extend(view.makes_session.then_some(NoteKind::Session));
                }
                NoteKind::Checklist => {
                    choices.extend(plain(view.checklist.as_ref().is_some_and(|l| l.plain)));
                    choices.extend(switch(NoteKind::ShoppingList));
                }
                NoteKind::ShoppingList => {
                    choices.extend(plain(view.shopping.as_ref().is_some_and(|l| l.plain)));
                    choices.extend(switch(NoteKind::Checklist));
                }
                // No way out of a Tally.
                NoteKind::Tally => {}
                NoteKind::Session => {
                    choices.extend(view.makes_session.then_some(NoteKind::Plain));
                }
            }
        }
        KindShown { kind, choices }
    }
}

/// What making the Note `id` of the kind `from` one of the kind `to`
/// asks (#188): the existing ways a Note becomes a list, a Tally or a
/// plain Note again, a list the other list; a Session's setup page, and
/// a Session a plain Note again (#191).
fn kind_action(id: &str, from: NoteKind, to: NoteKind) -> Action {
    let id = id.to_string();
    match to {
        NoteKind::Plain if from == NoteKind::Session => Action::UnmakeSession(id),
        NoteKind::Plain if from == NoteKind::ShoppingList => {
            Action::Shopping(ShoppingAction::ToPlain(id))
        }
        NoteKind::Plain => Action::Checklist(ChecklistAction::ToPlain(id)),
        NoteKind::Checklist => Action::Checklist(ChecklistAction::Make(id)),
        NoteKind::ShoppingList => Action::Shopping(ShoppingAction::Make(id)),
        NoteKind::Tally => Action::Tally(TallyAction::Make(id)),
        NoteKind::Session => Action::MakeSession(id),
    }
}

/// The icon and the name of the kind `kind` (#188), the value of its
/// badge (#192); the phone's `kindIcon` and `kindName`.
/// Under the Note's text after a paste that looks like code (#213): says
/// so and offers to keep the whole text as code; the cross lets it be.
fn code_offer(ui: &mut Ui, t: &L10n, actions: &mut Vec<Action>) {
    let text = ui.visuals().text_color();
    ui.horizontal(|ui| {
        icons::show(ui, icons::CODE, 18.0, text);
        ui.label(RichText::new(t.code_pasted()).color(text));
        if ui
            .button(RichText::new(t.keep_as_code()).color(text))
            .clicked()
        {
            actions.push(Action::SetTextFormat(text_format::CODE));
        }
        if icon_button(ui, icons::CLOSE, t.close(), text).clicked() {
            actions.push(Action::OfferCode(false));
        }
    });
}

/// The value icon of the text format `format` in its badge, and the
/// format in words (#213).
fn text_format_look(t: &L10n, format: &str) -> (char, &'static str) {
    match format {
        text_format::MARKDOWN => (icons::FORMAT_BOLD, t.text_format_formatted()),
        text_format::CODE => (icons::CODE, t.text_format_code()),
        _ => (icons::SHORT_TEXT, t.text_format_plain()),
    }
}

fn kind_look(t: &L10n, kind: NoteKind) -> (char, &'static str) {
    match kind {
        NoteKind::Plain => (icons::ALL_NOTES, t.kind_plain()),
        NoteKind::Checklist => (icons::CHECKLIST, t.checklist_heading()),
        NoteKind::ShoppingList => (icons::SHOPPING_CART, t.shopping_heading()),
        NoteKind::Tally => (icons::TALLY, t.tally_heading()),
        NoteKind::Session => (icons::SESSION, t.session()),
    }
}

/// The row of badges under the Note's text (#160), each the setting's
/// icon and its value, an icon for a fixed set of values, short text
/// otherwise (#192), in the phone's order:
/// the Importance; Area and Topic in one, opening File into…; Captured,
/// to read its details; each Routine and Reminder with its schedule,
/// opening its editor, and those a newer version saved, only shown; the
/// kind; a Session's rule and weekdays' counts (#191); Show on the
/// Agenda; Show ticks; and "+" with Routine, Reminder, Frog and, on a
/// plain Note, Session, for who may set them up. A setting that does not
/// apply has no badge; one this Device may not change only shows.
fn note_badges(
    ui: &mut Ui,
    (t, tones): (&L10n, Tones),
    shown: &BadgesShown,
    actions: &mut Vec<Action>,
) {
    let placed = shown.placed;
    let note = &placed.note;
    let trashed = note.is_trashed();
    let changes = shown.may.changes && !trashed;
    let look = (t, tones);
    badge_row(ui, |ui| {
        let importance = match note.importance {
            Importance::Unimportant => (icons::IMPORTANCE_LOW, t.importance_unimportant()),
            Importance::Normal => (icons::IMPORTANCE_NORMAL, t.importance_normal()),
            Importance::Important => (icons::IMPORTANCE_HIGH, t.importance_important()),
        };
        let icon = (icons::IMPORTANCE, None);
        let value = BadgeValue::Icon(importance.0, importance.1);
        if badge(ui, look, icon, t.importance(), Some(value), changes).clicked() {
            actions.push(Action::Badge(Some(NoteBadge::Importance)));
        }
        // Area and Topic in one: File into… picks both at once.
        // The folder for every place, in the Area's colour.
        let area = placed.area.as_ref();
        let colour = area.and_then(|a| area_colour(a.colour.as_deref()));
        let place = place_line(t, placed).unwrap_or_else(|| t.unsorted().to_string());
        if badge(
            ui,
            look,
            (icons::NO_AREA_ICON, colour),
            t.badge_place(),
            Some(BadgeValue::Text(&place)),
            changes,
        )
        .clicked()
        {
            actions.push(Action::AskFileInto(vec![note.id.clone()]));
        }
        let captured = when(t, note.captured);
        let icon = (icons::CONTEXT_WHEN, None);
        let value = Some(BadgeValue::Text(&captured));
        if badge(ui, look, icon, t.captured_heading(), value, true).clicked() {
            actions.push(Action::Badge(Some(NoteBadge::Captured)));
        }
        for routine in shown.routines {
            let (icon, setting) = if routine.is_reminder() {
                (icons::REMINDER, t.reminder())
            } else {
                (icons::REPEAT, t.routine())
            };
            let schedule = schedule_text(t, &routine.schedule);
            let value = Some(BadgeValue::Text(&schedule));
            if badge(ui, look, (icon, None), setting, value, true).clicked() {
                actions.push(Action::OpenRoutine(routine.id.clone()));
            }
        }
        for _ in 0..shown.newer {
            let icon = (icons::REPEAT, None);
            let newer = Some(BadgeValue::Text(t.newer_version_needed()));
            badge(ui, look, icon, t.routine(), newer, false);
        }
        // Every Note has its kind, a plain Note too (#188); it opens
        // where there is another to make it.
        let (icon, name) = kind_look(t, shown.kind.kind);
        let choosing = !shown.kind.choices.is_empty();
        let value = Some(BadgeValue::Icon(icon, name));
        let kind = (icons::KIND, None);
        if badge(ui, look, kind, t.badge_kind(), value, choosing).clicked() {
            actions.push(Action::Badge(Some(NoteBadge::Kind)));
        }
        // A Plain Note's text is plain, formatted or code (#213).
        if shown.kind.kind == NoteKind::Plain && note.text_editable {
            let (icon, name) = text_format_look(t, &note.text_format);
            let value = Some(BadgeValue::Icon(icon, name));
            let setting = (icons::TEXT_FORMAT, None);
            if badge(ui, look, setting, t.badge_text_format(), value, changes).clicked() {
                actions.push(Action::Badge(Some(NoteBadge::Format)));
            }
        }
        // A Session's rule and its weekdays' counts (#191).
        if let Some((setup, changes)) = shown.session {
            let rule = session_view::rule_name(t, setup.rule);
            let rule = Some(BadgeValue::Icon(session_view::rule_icon(setup.rule), rule));
            let icon = (icons::SESSION_RULE_BADGE, None);
            if badge(ui, look, icon, t.session_rule(), rule, changes).clicked() {
                actions.push(Action::Badge(Some(NoteBadge::SessionRule)));
            }
            let own = t.session_weekday_counts_summary(setup.counts.len() as i64);
            let icon = (icons::SESSION_WEEKDAYS, None);
            let setting = t.session_weekday_counts();
            let own = Some(BadgeValue::Text(&own));
            if badge(ui, look, icon, setting, own, changes).clicked() {
                actions.push(Action::Badge(Some(NoteBadge::SessionWeekdays)));
            }
        }
        // A Shopping List's (#186) or a Checklist's (#219) three ways.
        let ways = [
            (shown.on_agenda, NoteBadge::OnAgenda),
            (shown.shopping_on_agenda, NoteBadge::ShoppingOnAgenda),
        ];
        for (shows, open) in ways {
            let Some(shows) = shows else {
                continue;
            };
            let value = shopping_view::on_agenda_label(t, shows);
            let value = Some(BadgeValue::Icon(
                shopping_view::on_agenda_icon(shows),
                value,
            ));
            let icon = (icons::ON_AGENDA, None);
            let setting = t.checklist_show_on_agenda();
            if badge(ui, look, icon, setting, value, true).clicked() {
                actions.push(Action::Badge(Some(open)));
            }
        }
        let switches = [(
            shown.ticks,
            icons::SHOW_TICKS,
            t.box_show_ticks(),
            NoteBadge::Ticks,
        )];
        for (on, icon, setting, open) in switches {
            let Some(on) = on else {
                continue;
            };
            let value = if on { t.setting_on() } else { t.setting_off() };
            let value = Some(BadgeValue::Icon(switch_icon(on), value));
            if badge(ui, look, (icon, None), setting, value, true).clicked() {
                actions.push(Action::Badge(Some(open)));
            }
        }
        // In a Shared Area only a Manager sets them up (ADR 0013).
        if !shown.may.schedules || trashed {
            return;
        }
        let add = badge(
            ui,
            look,
            (icons::ADD, None),
            t.routine_or_reminder(),
            None,
            true,
        );
        let id = note.id.clone();
        let text = ui.visuals().text_color();
        egui::Popup::menu(&add).show(|ui| {
            let mut entries = vec![
                (
                    icons::REPEAT,
                    t.routine(),
                    Action::NewRoutine {
                        note: Some(id.clone()),
                        reminder: false,
                    },
                ),
                (
                    icons::REMINDER,
                    t.reminder(),
                    Action::NewRoutine {
                        note: Some(id.clone()),
                        reminder: true,
                    },
                ),
            ];
            // A Note has one open Frog at a time.
            if !shown.frog_open {
                entries.push((icons::FROG, t.frog(), Action::SpawnFrog(id.clone())));
            }
            // Only a plain Note becomes a Session, and no Code text
            // (#213); a Routine of a Session Note runs it already (#191).
            if shown.kind.kind == NoteKind::Plain && note.text_format != text_format::CODE {
                entries.push((
                    icons::SESSION,
                    t.session(),
                    Action::NewSession(Some(id.clone())),
                ));
            }
            for (icon, label, action) in entries {
                if menu_entry(ui, (icon, text), label).clicked() {
                    actions.push(action);
                    ui.close();
                }
            }
        });
    });
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
        Some(Photo::Out(hash, out)) => actions.push(Action::PhotoOut(hash, out)),
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

/// How a card shows its Note (#114, #189): `full`, its detailed view,
/// the whole Note with its `sections`, or its compact view, by its kind,
/// with the `glance` of its Tally or of the Session it runs; whether it
/// has the view button that switches it (`switches`); its Checklist or
/// Shopping List, if it is one, as it shows now, with whether this Device
/// may check its Lines, and what the card's menu offers of it once read
/// for this Note (#120, #121, #122).
#[derive(Debug, Clone, Copy)]
struct CardLook<'a> {
    full: bool,
    glance: &'a [String],
    switches: bool,
    /// Which of its folds this Device keeps open (#189).
    folds: CardFolds,
    sections: &'a [Section],
    checklist: Option<(&'a Checklist, bool)>,
    shopping: Option<(&'a ShoppingList, bool)>,
    /// Whether the Note is a Tally, whose full card shows its name and
    /// description as a list's does (#188).
    tally: bool,
    /// The Tally a full card shows under them: its chart, its entries and
    /// "Add entry" (#190).
    tally_shown: Option<&'a TallyShown>,
    /// Whether the Note is a Session, shown by its name and description as
    /// a Tally is, and what a full card shows under them: its pool, today's
    /// picks and last weeks (#191).
    session: bool,
    session_shown: Option<&'a SessionCardShown>,
    menu: Option<&'a CardMenu>,
    /// Whether this Device may change its text, its tables full screen
    /// (#128).
    changes: bool,
    /// How the boxes of its text and its Sections' show, by id, those of
    /// them this Device may tick, and whether Show ticks is on for it
    /// (ADR 0035).
    boxes: CardBoxes<'a>,
    /// Today's Occurrences of its Routines and Reminders, a box for each
    /// that ticks it as on the Agenda (#215).
    today: &'a [CardOccurrence],
}

/// The folds of a card this Device keeps open (UI laws, "Lists"; #189):
/// a Shopping List's struck Lines and the description of a Note of a
/// kind.
#[derive(Debug, Clone, Copy, Default)]
struct CardFolds {
    struck: bool,
    description: bool,
}

/// How the boxes of the texts listed show, by id, the texts whose boxes
/// this Device may tick and whether the Note has Show ticks on.
#[derive(Debug, Clone, Copy)]
struct CardBoxes<'a> {
    looks: &'a HashMap<String, BoxLook>,
    ticks: &'a HashSet<String>,
    show: bool,
}

impl CardBoxes<'_> {
    /// How the boxes of the text `id` show.
    fn look(&self, id: &str) -> &BoxLook {
        static NONE: std::sync::OnceLock<BoxLook> = std::sync::OnceLock::new();
        self.looks
            .get(id)
            .unwrap_or_else(|| NONE.get_or_init(BoxLook::default))
    }
}

/// What can be clicked on a card: the card, the author's picture on
/// another person's Note, Edit, the photo of a full card clicked, by
/// hash, the Line checked or unchecked on it, whether its menu opened
/// and what was picked in it (#120).
struct CardClicks {
    card: egui::Response,
    author: Option<egui::Response>,
    edit: egui::Response,
    /// The view button, switching the card's view (#189).
    view: egui::Response,
    /// The setting of a fold clicked, a Shopping List's struck Lines or a
    /// description, and whether it is open now (#189).
    folded: Option<(String, bool)>,
    photo: Option<String>,
    check: Option<(String, bool)>,
    /// The box of today's Occurrence clicked, by its Routine (#215).
    occurrence: Option<(String, Occurrence)>,
    /// What a click on a box of the Note's text or a Section's asked,
    /// with that text's id (ADR 0035).
    tick: Option<(String, BoxAsk)>,
    /// The table clicked, to show full screen (#128).
    table: Option<Opening>,
    /// The tag clicked, as written (#126).
    tag: Option<String>,
    menu_opened: bool,
    picked: Option<Action>,
    /// What was asked of the Tally on a full card (#190).
    tally: Vec<TallyAction>,
    /// What was asked of the picks on a Session Note's full card (#191).
    session: Vec<SessionAction>,
}

/// The room after the last row of every list (#158): its last card,
/// emboss and shadow included, scrolls fully into view, clear of the
/// window's edge, the list's "+" being at its top.
pub(crate) const LIST_END: f32 = 24.0;

/// A list's vertical scrolling, as every list of the window has it: the
/// one place its end gets [`LIST_END`] of room (#158).
pub(crate) fn list_scroll<R>(
    ui: &mut Ui,
    id: &str,
    content: impl FnOnce(&mut Ui) -> R,
) -> egui::scroll_area::ScrollAreaOutput<R> {
    egui::ScrollArea::vertical()
        .id_salt(id)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let shown = content(ui);
            ui.add_space(LIST_END);
            shown
        })
}

/// "+" at the top of a list (#146, #161): with one kind it makes that at
/// once, with more its menu offers them, Note first, each with its icon
/// and words; the kind picked.
fn add_button(ui: &mut Ui, t: &L10n, kinds: &[NewKind]) -> Option<NewKind> {
    let text = ui.visuals().text_color();
    if let [kind] = kinds {
        let add = icon_button(ui, icons::ADD, t.menu_new_note(), text);
        return add.clicked().then_some(*kind);
    }
    let add = icon_button(ui, icons::ADD, t.add_here(), text);
    let mut picked = None;
    egui::Popup::menu(&add).show(|ui| {
        for kind in kinds {
            let (icon, label) = match kind {
                NewKind::Note => (icons::NEW_NOTE, t.add_fan_note()),
                NewKind::Checklist => (icons::CHECKLIST, t.checklist_heading()),
                NewKind::Shopping => (icons::SHOPPING_CART, t.shopping_heading()),
                NewKind::Tally => (icons::TALLY, t.tally_heading()),
                NewKind::Session => (icons::SESSION, t.session()),
                NewKind::Frog => (icons::FROG, t.frog()),
                NewKind::Topic => (icons::NO_AREA_ICON, t.topic_choice()),
            };
            if menu_entry(ui, (icon, text), label).clicked() {
                picked = Some(*kind);
                ui.close();
            }
        }
    });
    picked
}

/// The bar button of a Note list's compact/detailed switch (#114, #189):
/// it says what clicking it does and shows as selected while the list is
/// `compact`; whether it was clicked.
fn compact_button(ui: &mut Ui, tones: Tones, t: &L10n, compact: bool) -> bool {
    let (icon, label) = if compact {
        (icons::NOTES_COMPACT, t.show_notes_full())
    } else {
        (icons::NOTES_FULL, t.show_notes_compact())
    };
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), Sense::click());
    resp.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, compact, label));
    // The hover under the tint, so the selected look stays.
    theme::icon_button_back(ui, &resp, rect);
    if compact {
        let fill = tones.accent.gamma_multiply(0.3);
        ui.painter().rect_filled(rect, theme::BUTTON_RADIUS, fill);
    }
    let text = ui.visuals().text_color();
    icons::paint(ui.painter(), rect.shrink(2.0), icon, text);
    resp.on_hover_text(label).clicked()
}

/// A text edited in the Note dialog: the Note's, or a Section's (#183)
/// in the same field, with the same format bar and the same rules.
struct EditedText<'a> {
    /// The field's id.
    id: Id,
    /// The Note or Section whose text it is, and whether a Section's.
    of: &'a str,
    section: bool,
    /// Its text as stored, and whether it is a formatted one.
    stored: &'a str,
    formatted: bool,
    /// Whether it is a Code text (#213), and whether a paste that looks
    /// like code offers it.
    code: bool,
    offers_code: bool,
    /// How its boxes show, whether this Device may tick them and which
    /// were ticked (ADR 0035).
    boxes: &'a BoxLook,
    ticks: (bool, &'a dyn Fn(&str) -> bool),
    /// What a screen reader calls the field.
    name: &'a str,
    /// The size and colour of its text: a description's smaller and
    /// weaker (#188).
    look: (f32, Color32),
    /// What it says while empty.
    hint: Option<&'a str>,
}

/// The field of a text of the Note dialog, `edit`, its format bar above
/// it (#127): `edited` as it is being edited, whether it is (a formatted
/// text shows its source meanwhile, as it reads otherwise), and whether
/// it is to take the keyboard, once, with the selection `select`; a Save
/// under it while it changed.
fn edited_text(
    ui: &mut Ui,
    (t, tones): (L10n, Tones),
    edit: EditedText<'_>,
    (edited, editing, focus, select): (
        &mut String,
        &mut bool,
        &mut bool,
        &mut Option<(usize, usize)>,
    ),
    actions: &mut Vec<Action>,
) {
    let text = ui.visuals().text_color();
    let (size, written) = edit.look;
    let (formatted, boxes) = (edit.formatted, edit.boxes);
    // Kept as a formatted text at once: the Note's, or the Section's.
    let format_text = || Action::FormatText(edit.section.then(|| edit.of.to_string()));
    let id = edit.id;
    let of = edit.of;
    let focused = ui.memory(|m| m.has_focus(id));
    if focused {
        *editing = true;
        if ui.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::Enter)) {
            actions.push(Action::SaveText);
        }
    }
    // The format bar above the field (#127), on its
    // selection as the field last had it; Ctrl+B and
    // Ctrl+I as its bold and italic while typing.
    let selected = format_bar::selection(ui, id, edited);
    let at = format_bar::At::of(edited, selected, formatted);
    // A Code text has none (#213).
    let as_code = edit.code;
    let mut picked = match as_code {
        true => None,
        false => format_bar::bar(ui, &t, &at, (text, tones.accent), &Format::ALL),
    };
    if focused && !as_code {
        ui.input_mut(|i| {
            if i.consume_key(Modifiers::COMMAND, Key::B) {
                picked = Some(Format::Bold);
            } else if i.consume_key(Modifiers::COMMAND, Key::I) {
                picked = Some(Format::Italic);
            }
        });
    }
    if formatted && !*editing {
        // As it reads; a click on it edits its source.
        let all = (0, edited.len());
        let shown = ui.scope(|ui| {
            text_marks_view::formatted_label(
                ui,
                (edited, text_format::MARKDOWN, boxes),
                all,
                (Some(size), written),
                false,
            )
        });
        let read = Id::new(("read", of));
        let open = ui.interact(shown.response.rect, read, Sense::click());
        open.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, t.edit()));
        if open.on_hover_cursor(egui::CursorIcon::Text).clicked() {
            *editing = true;
            *focus = true;
        }
        let spots = shown.inner;
        let marked = Id::new(("field", of));
        // A click on a table opens it full screen, to
        // edit (#128).
        if let Some(at) = text_marks_view::table_clicks(ui, &t, marked, &spots.tables) {
            actions.push(Action::OpenTable(Opening {
                note: Some(of.to_string()),
                text: edited.clone(),
                at,
                cell: None,
                tags: true,
                boxes: Some(of.to_string()),
            }));
        }
        // A tick is no edit: what is typed stays.
        if let Some(asked) = text_marks_view::box_clicks(ui, &t, marked, &spots.boxes, edit.ticks) {
            actions.push(box_action(of.to_string(), asked));
        }
        if let Some(tag) = text_marks_view::tag_clicks(ui, &t, marked, &spots.tags) {
            actions.push(Action::SearchFor(tag));
        }
        text_marks_view::link_clicks(ui, marked, &spots.links);
    } else {
        // Its tags in the accent colour, a formatted text's
        // marks dimmed; a click on a tag places the cursor,
        // as anywhere in the field (#126). Its boxes as they
        // show, their ids not drawn (ADR 0035), with Show
        // ticks room after each for who ticked it.
        let accent = ui.visuals().hyperlink_color;
        let weak = ui.visuals().weak_text_color();
        let code = ui.visuals().code_bg_color;
        let small = FontId::proportional(13.0);
        let mut layouter = |ui: &Ui, typed: &dyn egui::TextBuffer, width: f32| {
            let typed = typed.as_str();
            let font = FontId::proportional(size);
            // A Code text as written in the fixed-width font, unwrapped.
            if as_code {
                let format = egui::TextFormat::simple(FontId::monospace(size * 0.9), written);
                let job = egui::text::LayoutJob::single_section(typed.to_string(), format);
                return ui.fonts_mut(|f| f.layout_job(job));
            }
            let job = if formatted {
                text_marks_view::source_job(typed, font, (written, accent, weak, code))
            } else {
                text_marks_view::tag_job(typed, (0, typed.len()), font, (written, accent))
            };
            let gap = |after: &str| {
                let said = after.to_string();
                let galley = ui.fonts_mut(|f| f.layout_no_wrap(said, small.clone(), weak));
                galley.size().x + 8.0
            };
            let mut job = text_marks_view::field_job(job, boxes, &gap);
            job.wrap.max_width = width;
            ui.fonts_mut(|f| f.layout_job(job))
        };
        // Pasted, a text that looks like code offers it and formats
        // nothing (#213); else pasted with a format, a plain text
        // becomes a formatted one (ADR 0033).
        let pasted_code = edit.offers_code
            && focused
            && ui.input(|i| {
                i.events.iter().any(|e| {
                    matches!(e, egui::Event::Paste(p)
                        if p.chars().nth(1).is_some() && text_format::looks_like_code(p))
                })
            });
        let pasted = !pasted_code
            && focused
            && ui.input(|i| {
                i.events
                    .iter()
                    .any(|e| matches!(e, egui::Event::Paste(p) if markdown::uses(p)))
            });
        let placed = *focus && select.is_some();
        // Not in the dialog's first, sizing pass, which
        // drops a focus.
        if !ui.is_sizing_pass() && std::mem::take(focus) {
            let typed = egui::widgets::text_edit::TextEditState::load(ui.ctx(), id)
                .and_then(|s| s.cursor.char_range())
                .is_some();
            match select.take() {
                Some(range) => format_bar::select(ui, id, edited, range),
                // Never typed into: the cursor at the end.
                None if !typed => {
                    let end = edited.len();
                    format_bar::select(ui, id, edited, (end, end));
                }
                None => ui.memory_mut(|m| m.request_focus(id)),
            }
        }
        // A box goes whole, its id with it, and the cursor
        // stays where it was as a save gives boxes ids.
        let cut = text_marks_view::guard_field(ui, id, edited, !placed);
        // As wide as its longest line, for a Code text, which moves
        // sideways (#213).
        let widest = as_code.then(|| {
            let font = FontId::monospace(size * 0.9);
            let galley = ui.fonts_mut(|f| f.layout_no_wrap(edited.clone(), font, written));
            galley.size().x + 8.0
        });
        let mut field = TextEdit::multiline(edited)
            .id(id)
            .font(FontId::proportional(size))
            .layouter(&mut layouter)
            .desired_rows(1)
            .desired_width(f32::INFINITY)
            .frame(Frame::NONE);
        if let Some(hint) = edit.hint {
            field = field.hint_text(RichText::new(hint).size(size).color(weak));
        }
        let output = match widest {
            None => field.show(ui),
            Some(widest) => {
                Frame::NONE
                    .fill(code)
                    .corner_radius(4.0)
                    .inner_margin(egui::Margin::symmetric(8, 6))
                    .show(ui, |ui| {
                        egui::ScrollArea::horizontal()
                            .id_salt(("code", of))
                            .show(ui, |ui| {
                                let width = widest.max(ui.available_width());
                                let room = egui::vec2(width, ui.available_height());
                                let rect = egui::Rect::from_min_size(ui.cursor().min, room);
                                ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                                    field.show(ui)
                                })
                                .inner
                            })
                            .inner
                    })
                    .inner
            }
        };
        text_marks_view::guard_cursor(ui, id, edited);
        if cut {
            ui.ctx().request_repaint();
        }
        name_field(ui, &output.response.response, edit.name);
        // A click on the bar keeps the field being edited.
        if output.response.response.lost_focus() && picked.is_none() {
            *editing = false;
            actions.push(Action::SaveText);
        }
        if output.response.response.changed() && pasted_code {
            actions.push(Action::OfferCode(true));
        }
        // A shortcut typed makes a plain text a formatted
        // one, as the first format from the bar does.
        if output.response.response.changed() && !formatted && !as_code {
            let cursor = output.cursor_range.map_or(0, |range| range.primary.index.0);
            let byte = edited
                .char_indices()
                .nth(cursor)
                .map_or(edited.len(), |(b, _)| b);
            if pasted || markdown::shortcut_at(edited, byte) {
                actions.push(format_text());
            }
        }
        // With Show ticks, who ticked each box, after it.
        let place = (&output.galley, output.galley_pos);
        text_marks_view::paint_after(ui, place, (edited, boxes), (small, weak));
        // A click on a box ticks it, no edit of the text
        // (ADR 0035).
        let spots = text_marks_view::field_spots(&output.galley, output.galley_pos, edited, boxes);
        let marked = Id::new(("field", of));
        if let Some(asked) = text_marks_view::box_clicks(ui, &t, marked, &spots, edit.ticks) {
            actions.push(box_action(of.to_string(), asked));
        }
    }
    match picked {
        // In a table it opens it; elsewhere it puts a new
        // one in and edits its first cell (#128), the
        // text formatted from then on.
        Some(Format::Table) => {
            let opening = match markdown::table::at(edited, selected.0) {
                Some(table) => Opening {
                    note: Some(of.to_string()),
                    text: edited.clone(),
                    at: table.start,
                    cell: None,
                    tags: true,
                    boxes: Some(of.to_string()),
                },
                None => {
                    let change = markdown::table::insert(edited, selected.0);
                    *edited = change.text;
                    actions.push(if formatted {
                        Action::SaveText
                    } else {
                        format_text()
                    });
                    Opening {
                        note: Some(of.to_string()),
                        text: edited.clone(),
                        at: change.start,
                        cell: Some((0, 0)),
                        tags: true,
                        boxes: Some(of.to_string()),
                    }
                }
            };
            actions.push(Action::OpenTable(opening));
        }
        Some(Format::Link) => {
            *editing = true;
            actions.push(Action::AskLink {
                start: selected.0,
                end: selected.1,
                url: at.url.clone(),
                amend: false,
                section: edit.section.then(|| of.to_string()),
            });
        }
        Some(format) => {
            *editing = true;
            match format_bar::change(format, edited, selected) {
                Some(changed) => {
                    *edited = changed.text;
                    let range = (changed.start, changed.end);
                    format_bar::select(ui, id, edited, range);
                    // The first format makes it a formatted
                    // text, kept at once.
                    if format != Format::Mark && !formatted {
                        actions.push(format_text());
                    }
                }
                None => ui.memory_mut(|m| m.request_focus(id)),
            }
        }
        None => {}
    }
    text_marks_view::remember_text(ui, id, edited);
    ui.horizontal(|ui| {
        let changed = edited.trim();
        if !changed.is_empty()
            && changed != edit.stored
            && theme::primary(ui, true, t.save()).clicked()
        {
            actions.push(Action::SaveText);
        }
    });
}

/// The size of the name of a Note of a kind (#188), a heading, and of
/// its description, smaller than a text.
const NAME_SIZE: f32 = 22.0;
const DESCRIPTION_SIZE: f32 = 14.0;

/// The text of a Note of a kind in the Note dialog (#188): the Note,
/// whether this Device may change its text here, how its boxes show,
/// whether it may tick them and which were ticked (ADR 0035).
struct KindedText<'a> {
    note: &'a Note,
    writes: bool,
    boxes: &'a BoxLook,
    ticks: (bool, &'a dyn Fn(&str) -> bool),
}

/// What is typed into the text of a Note of a kind: its name, and its
/// description as [`edited_text`] edits it.
type KindedState<'a> = (
    &'a mut String,
    (
        &'a mut String,
        &'a mut bool,
        &'a mut bool,
        &'a mut Option<(usize, usize)>,
    ),
);

/// The text of a Note of a kind as its name, a heading on one line, and
/// its description under it (#188), small and weak, folded to its first
/// line while it is not edited: a click unfolds it, into its field where
/// this Device may change it, which an empty one always shows. Read only,
/// both select to be copied (#183) and a click folds the description
/// again.
fn kinded_text(
    ui: &mut Ui,
    (t, tones): (L10n, Tones),
    shown: KindedText<'_>,
    (name, (edited, editing, focus, select)): KindedState<'_>,
    actions: &mut Vec<Action>,
) {
    let note = shown.note;
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let formatted = note.text_format == text_format::MARKDOWN;
    if !shown.writes {
        ui.style_mut().interaction.selectable_labels = true;
        let whole = note.text.as_str();
        let look = (whole, note.text_format.as_str(), shown.boxes);
        // In the offsets of the whole text, as its boxes, tables, tags
        // and links are found.
        let named = note_kinds::name(whole).len();
        let mut spots = Spots::default();
        if named > 0 {
            let heading = (Some(NAME_SIZE), text);
            let label = text_marks_view::formatted_label(ui, look, (0, named), heading, false);
            spots.extend(label);
        }
        let description = note_kinds::description(whole);
        let start = whole.len() - description.len();
        if let Some((from, to)) = first_line(description) {
            let id = Id::new(("description open", &note.id));
            let open = ui.data(|d| d.get_temp::<bool>(id)).unwrap_or(false);
            let range = if open {
                (start, whole.len())
            } else {
                (start + from, start + to)
            };
            let small = (Some(DESCRIPTION_SIZE), weak);
            let drawn = folded_row(ui, open, |ui| {
                text_marks_view::formatted_label(ui, look, range, small, !open)
            });
            if description_fold(ui, &t, (drawn.response.rect, id), open) {
                ui.data_mut(|d| d.insert_temp(id, !open));
            }
            spots.extend(drawn.inner);
        }
        let of = Id::new(("text", &note.id));
        // Its tables open to read (#128).
        if let Some(at) = text_marks_view::table_clicks(ui, &t, of, &spots.tables) {
            actions.push(Action::OpenTable(Opening {
                note: None,
                text: note.text.clone(),
                at,
                cell: None,
                tags: true,
                boxes: Some(note.id.clone()),
            }));
        }
        // A Member ticks the boxes of a Note not theirs.
        if let Some(asked) = text_marks_view::box_clicks(ui, &t, of, &spots.boxes, shown.ticks) {
            actions.push(box_action(note.id.clone(), asked));
        }
        if let Some(tag) = text_marks_view::tag_clicks(ui, &t, of, &spots.tags) {
            actions.push(Action::SearchFor(tag));
        }
        text_marks_view::link_clicks(ui, of, &spots.links);
        return;
    }
    // Enter in the name goes on at the start of the description.
    if name_edit(ui, &t, (name, formatted, shown.boxes), actions) {
        *editing = true;
        *focus = true;
        *select = Some((0, 0));
    }
    let id = Id::new("note text");
    let open = *editing || ui.memory(|m| m.has_focus(id));
    if open {
        let field = EditedText {
            id,
            of: &note.id,
            section: false,
            stored: note_kinds::description(&note.text),
            formatted,
            code: false,
            offers_code: false,
            boxes: shown.boxes,
            ticks: shown.ticks,
            name: t.note_description(),
            look: (DESCRIPTION_SIZE, weak),
            hint: Some(t.note_description()),
        };
        edited_text(
            ui,
            (t, tones),
            field,
            (edited, editing, focus, select),
            actions,
        );
    } else if edited.trim().is_empty() {
        // Empty, its field with its hint; typed into, it is edited.
        let hint = RichText::new(t.note_description())
            .size(DESCRIPTION_SIZE)
            .color(weak);
        let field = ui.add(
            TextEdit::multiline(edited)
                .id(id)
                .font(FontId::proportional(DESCRIPTION_SIZE))
                .text_color(weak)
                .hint_text(hint)
                .desired_rows(1)
                .desired_width(f32::INFINITY)
                .frame(Frame::NONE),
        );
        name_field(ui, &field, t.note_description());
        if field.has_focus() {
            *editing = true;
        }
    } else {
        // Folded to its first line with words, as typed.
        let range = first_line(edited).unwrap_or((0, 0));
        let look = (edited.as_str(), note.text_format.as_str(), shown.boxes);
        let small = (Some(DESCRIPTION_SIZE), weak);
        let drawn = folded_row(ui, false, |ui| {
            text_marks_view::formatted_label(ui, look, range, small, true)
        });
        let fold = Id::new(("description open", &note.id));
        if description_fold(ui, &t, (drawn.response.rect, fold), false) {
            *editing = true;
            *focus = true;
        }
        // Its boxes tick and its tags and links open, folded too.
        let spots = drawn.inner;
        let marked = Id::new(("field", &note.id));
        if let Some(asked) = text_marks_view::box_clicks(ui, &t, marked, &spots.boxes, shown.ticks)
        {
            actions.push(box_action(note.id.clone(), asked));
        }
        if let Some(tag) = text_marks_view::tag_clicks(ui, &t, marked, &spots.tags) {
            actions.push(Action::SearchFor(tag));
        }
        text_marks_view::link_clicks(ui, marked, &spots.links);
    }
}

/// The name of a Note of a kind being edited (#188): one line, as a
/// heading, its tags marked and its boxes as they show (ADR 0035), a
/// formatted text's marks dimmed. Leaving it keeps it; Enter goes on in
/// the description, which `true` says. A line break pasted becomes a
/// space, as in any one-line field.
fn name_edit(
    ui: &mut Ui,
    t: &L10n,
    (name, formatted, boxes): (&mut String, bool, &BoxLook),
    actions: &mut Vec<Action>,
) -> bool {
    let id = Id::new("note name");
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let accent = ui.visuals().hyperlink_color;
    let code = ui.visuals().code_bg_color;
    let mut moved = false;
    if ui.memory(|m| m.has_focus(id)) {
        ui.input_mut(|i| {
            if i.consume_key(Modifiers::COMMAND, Key::Enter) {
                actions.push(Action::SaveText);
            } else if i.consume_key(Modifiers::NONE, Key::Enter) {
                moved = true;
            }
        });
    }
    let mut layouter = |ui: &Ui, typed: &dyn egui::TextBuffer, width: f32| {
        let typed = typed.as_str();
        let font = FontId::proportional(NAME_SIZE);
        let job = if formatted {
            text_marks_view::source_job(typed, font, (text, accent, weak, code))
        } else {
            text_marks_view::tag_job(typed, (0, typed.len()), font, (text, accent))
        };
        let mut job = text_marks_view::field_job(job, boxes, &|_: &str| 0.0);
        job.wrap.max_width = width;
        ui.fonts_mut(|f| f.layout_job(job))
    };
    let cut = text_marks_view::guard_field(ui, id, name, true);
    let output = TextEdit::singleline(name)
        .id(id)
        .font(FontId::proportional(NAME_SIZE))
        .layouter(&mut layouter)
        .hint_text(RichText::new(t.note_name()).size(NAME_SIZE).color(weak))
        .desired_width(f32::INFINITY)
        .return_key(None)
        .frame(Frame::NONE)
        .show(ui);
    text_marks_view::guard_cursor(ui, id, name);
    if cut {
        ui.ctx().request_repaint();
    }
    text_marks_view::remember_text(ui, id, name);
    let field = &output.response.response;
    name_field(ui, field, t.note_name());
    if moved {
        ui.memory_mut(|m| m.surrender_focus(id));
    } else if field.lost_focus() {
        actions.push(Action::SaveText);
    }
    moved
}

/// A description of a Note of a kind as `add` draws it, folded or
/// `open`, after the sign of which it is (#188).
fn folded_row<R>(
    ui: &mut Ui,
    open: bool,
    add: impl FnOnce(&mut Ui) -> R,
) -> egui::InnerResponse<R> {
    let weak = ui.visuals().weak_text_color();
    ui.horizontal_top(|ui| {
        let icon = if open {
            icons::EXPAND_LESS
        } else {
            icons::EXPAND_MORE
        };
        icons::show(ui, icon, 16.0, weak);
        ui.vertical(add).inner
    })
}

/// Whether the description drawn at `rect` was clicked to unfold it, or
/// to fold it while `open` (#188); a screen reader reads what the click
/// does.
fn description_fold(ui: &mut Ui, t: &L10n, (rect, id): (egui::Rect, Id), open: bool) -> bool {
    let said = if open {
        t.note_description_fold()
    } else {
        t.note_description_unfold()
    };
    let fold = ui.interact(rect, id.with("fold"), Sense::click());
    fold.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, said));
    fold.on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(said)
        .clicked()
}

/// What a click on a box of the text `entity` asked, as an action.
fn box_action(entity: String, asked: BoxAsk) -> Action {
    match asked {
        BoxAsk::Tick(id, to) => Action::Tick { entity, id, to },
        BoxAsk::History(id) => Action::BoxHistory { entity, id },
    }
}

/// Whether a box of `look` has been ticked: it offers its History then.
fn ticked_in(boxes: &[ShownBox]) -> impl Fn(&str) -> bool + '_ {
    move |id| boxes.iter().any(|b| b.id == id && b.last.is_some())
}

/// A Section's text being edited in the Note dialog (#183), as the
/// Note's own is: with the ids of its boxes, whether it is being edited
/// (a formatted text shows its source meanwhile), and whether the field
/// is to take the keyboard, once, with the selection `select` in bytes.
#[derive(Debug, Clone, Default)]
struct SectionField {
    text: String,
    editing: bool,
    focus: bool,
    select: Option<(usize, usize)>,
}

/// The text of a Note or Section and its photos and voice recordings,
/// each as `(shown, removed, texts)`.
struct Part<'a> {
    id: &'a str,
    text: &'a str,
    format: &'a str,
    photos: (&'a [String], &'a [String], &'a BTreeMap<String, String>),
    recordings: (&'a [String], &'a [String], &'a BTreeMap<String, String>),
}

impl<'a> Part<'a> {
    fn of_note(note: &'a Note) -> Part<'a> {
        Part {
            id: &note.id,
            text: &note.text,
            format: &note.text_format,
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
            id: &section.id,
            text: &section.text,
            format: &section.text_format,
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
    (look, marks): (&BoxLook, &mut Spots),
) -> Vec<(String, egui::Rect)> {
    card_text(ui, (part.text, part.format, look), marks);
    media_view::whole_media(
        ui,
        t,
        (cache, store),
        part.photos,
        part.recordings,
        transcribing,
    )
}

/// The whole text `text` of a Note or Section in `format` on a full
/// card, its boxes as `look` says, where its boxes (ADR 0035), tags
/// (#126) and links (#127) lie added to `marks`.
fn card_text(ui: &mut Ui, (text, format, look): (&str, &str, &BoxLook), marks: &mut Spots) {
    if !text.is_empty() {
        let colour = ui.visuals().text_color();
        let shown = (0, text.len());
        marks.extend(text_marks_view::formatted_label(
            ui,
            (text, format, look),
            shown,
            (Some(16.0), colour),
            false,
        ));
    }
}

/// The text `text` of a Note of a kind in `format` on its full card
/// (#188): its name as the card's title, its description under it,
/// small, folded to its first line with words unless `unfolded`; where
/// its boxes, tags and links lie added to `marks`, and where the
/// description shows.
fn card_kinded_text(
    ui: &mut Ui,
    (text, format, look): (&str, &str, &BoxLook),
    unfolded: bool,
    marks: &mut Spots,
) -> Option<egui::Rect> {
    let colour = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let named = note_kinds::name(text).len();
    if named > 0 {
        let heading = (Some(20.0), colour);
        let label =
            text_marks_view::formatted_label(ui, (text, format, look), (0, named), heading, false);
        marks.extend(label);
    }
    let description = note_kinds::description(text);
    let start = text.len() - description.len();
    let (from, to) = first_line(description)?;
    let range = if unfolded {
        (start, text.len())
    } else {
        (start + from, start + to)
    };
    let small = (Some(13.0), weak);
    let drawn = folded_row(ui, unfolded, |ui| {
        text_marks_view::formatted_label(ui, (text, format, look), range, small, !unfolded)
    });
    marks.extend(drawn.inner);
    Some(drawn.response.rect)
}

/// The photo cache, the store the photos are read from and the
/// recording being transcribed, for [`whole`].
type ShownMedia<'a, 'b> = (&'b mut PhotoCache, &'a Store, Option<&'a str>);

/// A Note as a card (UI laws, "Cards"): Edit, ⋮ (but in the trash) and,
/// in a list with its switch, the view button top right (#189), and where
/// it is filed, its Area's colour down the left edge and icon before the
/// place; then, as `look` says, its detailed view: the whole Note with its
/// Capture Context as one line under its text (#170), a Checklist's or a
/// Shopping List's Lines under its name and description (#188), checkable,
/// a Shopping List's struck ones folded under its open ones, with every
/// Section oldest first, each with its own line, its photos opening full
/// screen on a click; or its compact view, a glance by its kind (#189): a
/// plain Note's first line and the two lines after it or the `glance` of
/// the Session it runs, a list's or a Tally's name with how many of its
/// Lines are open or the Tally's `glance`, a Note without words its
/// media; no Capture Context. ⋮ and a right-click open
/// the same menu: Edit, then making it a Checklist, "Uncheck all", "Back
/// to a plain Note" and making it a Shopping List as this Device may
/// (#120, #122). Important cards carry the accent marker and a stronger
/// emboss; unimportant ones are dimmed. A `draggable` card can be
/// dropped onto an Area or Topic, its drag started by the list; a
/// `selected` one is outlined while its editor shows; a `chosen` one,
/// among the cards selected (#148), carries a check mark and a thicker
/// outline, and says so to a screen reader.
fn note_card(
    ui: &mut Ui,
    t: &L10n,
    tones: Tones,
    placed: &Placed,
    (selected, chosen, draggable): (bool, bool, bool),
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
    let mut more_rect = None;
    let mut view_rect = None;
    // Where the fold of a Shopping List's struck Lines shows in the
    // detailed view, how many, and whether it is open (#189).
    let struck_id = Id::new(("card struck", &note.id));
    let struck_open = look.folds.struck;
    let mut struck_rect = None;
    // Where the description of a Note of a kind shows on a full card, and
    // whether it is unfolded (#188).
    let fold_id = Id::new(("card description", &note.id));
    let unfolded = look.folds.description;
    let mut description_rect = None;
    // The photos shown in full, where they lie, the checkboxes of its
    // Lines and the status marks and tags of its text and of its
    // Sections'.
    let mut shown = Vec::new();
    let mut boxes = Vec::new();
    let mut marks = Spots::default();
    let mut section_marks: Vec<(&Section, Spots)> = Vec::new();
    // Where the Tally or the Session of a full card lies, out of the
    // card's own click, and what was asked of it (#190, #191).
    let mut tally_rect = None;
    let mut tally_asked = Vec::new();
    let mut session_asked = Vec::new();
    // Where the boxes of today's Occurrences lie (#215).
    let mut today_rects = Vec::new();
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
                if chosen {
                    icons::show(ui, icons::STATUS_DONE, 18.0, tones.accent);
                }
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
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    edit_rect = ui
                        .allocate_exact_size(egui::vec2(24.0, 24.0), Sense::hover())
                        .0;
                    // A Note in the trash is restored, not changed.
                    if !note.is_trashed() {
                        more_rect = Some(
                            ui.allocate_exact_size(egui::vec2(24.0, 24.0), Sense::hover())
                                .0,
                        );
                    }
                    // Its view shows under the Note's text, its time and
                    // place in the detailed view (#170, #189).
                    if look.switches {
                        view_rect = Some(
                            ui.allocate_exact_size(egui::vec2(24.0, 24.0), Sense::hover())
                                .0,
                        );
                    }
                });
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
            ui.separator();
            // Under where it is filed, over its text (#215).
            today_rects = card_occurrences::card_rows(ui, t, tones, look.today);
            if look.full {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 8.0;
                    let part = Part::of_note(note);
                    let lines = match (look.checklist, look.shopping) {
                        (Some((list, _)), _) => Some(checklist_view::shown_lines(list)),
                        (None, Some((list, _))) => Some(shopping_view::shown_lines(t, list)),
                        (None, None) => None,
                    };
                    if lines.is_some() || look.tally || look.session {
                        // Its name as its title, its description folded
                        // under it (#188), its Lines right under them.
                        let text = (part.text, part.format, look.boxes.look(part.id));
                        description_rect = card_kinded_text(ui, text, unfolded, &mut marks);
                        if let Some(lines) = &lines {
                            // A Shopping List's struck Lines fold under its
                            // open ones (#189).
                            let (open, struck): (Vec<_>, Vec<_>) = lines
                                .iter()
                                .cloned()
                                .partition(|l| look.shopping.is_none() || !l.checked);
                            boxes = checklist_view::card_lines(ui, tones, &open);
                            if !struck.is_empty() {
                                let said = t.shopping_struck_count(struck.len() as i64);
                                let row = folded_row(ui, struck_open, |ui| {
                                    ui.label(RichText::new(&said).size(13.0).color(weak));
                                });
                                struck_rect = Some((row.response.rect, said));
                                if struck_open {
                                    boxes.extend(checklist_view::card_lines(ui, tones, &struck));
                                }
                            }
                        }
                        if let Some(shown) = look.tally_shown {
                            let drawn =
                                ui.scope(|ui| tally_view::card(ui, t, shown, &mut tally_asked));
                            tally_rect = Some(drawn.response.rect);
                        }
                        if let Some(shown) = look.session_shown {
                            let drawn =
                                ui.scope(|ui| session_view::card(ui, t, shown, &mut session_asked));
                            tally_rect = Some(drawn.response.rect);
                        }
                        shown.extend(media_view::whole_media(
                            ui,
                            t,
                            (&mut *photos, store),
                            part.photos,
                            part.recordings,
                            transcribing,
                        ));
                    } else {
                        let media = (&mut *photos, store, transcribing);
                        let boxes = look.boxes.look(part.id);
                        shown.extend(whole(ui, t, part, media, (boxes, &mut marks)));
                    }
                    // Each part's Capture Context under its text (#170).
                    let line = capture_line(t, (note.captured, &note.context), None);
                    capture_meta(ui, t, &line, None, false);
                    let mut before = note.captured;
                    for section in look.sections {
                        ui.separator();
                        let part = Part::of_section(section);
                        let mut spots = Spots::default();
                        let media = (&mut *photos, store, transcribing);
                        let boxes = look.boxes.look(part.id);
                        shown.extend(whole(ui, t, part, media, (boxes, &mut spots)));
                        section_marks.push((section, spots));
                        let line =
                            capture_line(t, (section.captured, &section.context), Some(before));
                        let edited = section_edited(t, section);
                        capture_meta(ui, t, &line, edited.as_deref(), false);
                        before = section.captured;
                    }
                });
                return;
            }
            // The compact view, by the Note's kind (#189).
            let text_of = (
                note.text.as_str(),
                note.text_format.as_str(),
                look.boxes.look(&note.id),
            );
            if look.checklist.is_some() || look.shopping.is_some() || look.tally || look.session {
                // Its name, then how many Lines are open, or the Tally's
                // totals today, or the Session's next run and week (#191).
                let named = note_kinds::name(&note.text).len();
                if named > 0 {
                    marks = text_marks_view::formatted_label(
                        ui,
                        text_of,
                        (0, named),
                        (Some(20.0), text),
                        true,
                    );
                }
                if let Some((list, _)) = look.checklist {
                    checklist_view::card_count(ui, t, list);
                } else if let Some((list, _)) = look.shopping {
                    shopping_view::card_count(ui, t, list);
                }
                card_glance_lines(ui, look.glance);
                return;
            }
            // A plain Note: its first line and the two after it, or, running
            // a Session, its next run and this week (#189).
            if let Some(line) =
                card_first_line(&note.text, note.text_format == text_format::MARKDOWN)
            {
                marks =
                    text_marks_view::formatted_label(ui, text_of, line, (Some(16.0), text), true);
                if look.glance.is_empty() {
                    if let Some(next) = next_lines(&note.text, line.1) {
                        let more = text_marks_view::formatted_label(
                            ui,
                            text_of,
                            next,
                            (Some(14.0), text),
                            false,
                        );
                        marks.extend(more);
                    }
                } else {
                    card_glance_lines(ui, look.glance);
                }
                return;
            }
            card_glance_lines(ui, look.glance);
            // Without words it shows what it holds instead.
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
        if chosen {
            ui.painter().rect_stroke(
                rect,
                theme::CARD_RADIUS,
                egui::Stroke::new(3.0, tones.accent),
                egui::StrokeKind::Inside,
            );
        } else if selected {
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
    // Around the Tally or the Session of a full card, whose own buttons
    // take its clicks (#190, #191).
    let resp = match tally_rect {
        Some(hole) => {
            let above = egui::Rect::from_min_max(rect.min, egui::pos2(rect.max.x, hole.min.y));
            let below = egui::Rect::from_min_max(egui::pos2(rect.min.x, hole.max.y), rect.max);
            let top = ui.interact(above, Id::new(("card", &note.id)), sense);
            top.union(ui.interact(below, Id::new(("card below", &note.id)), sense))
        }
        None => ui.interact(rect, Id::new(("card", &note.id)), sense),
    };
    let label = note.visible_text();
    resp.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, chosen, &label));
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
    // A fold clicked open or closed, kept per Note on this Device (#189).
    let mut folded = None;
    // The view button over the card, switching its view (#189).
    let view_said = if look.full {
        t.card_show_compact()
    } else {
        t.card_show_detailed()
    };
    let view = ui.interact(
        view_rect.unwrap_or(egui::Rect::NOTHING),
        Id::new(("view", &note.id)),
        Sense::click(),
    );
    if let Some(view_rect) = view_rect {
        view.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, view_said));
        if ui.is_rect_visible(view_rect) {
            if view.hovered() {
                let fill = ui.visuals().widgets.hovered.weak_bg_fill;
                ui.painter().rect_filled(view_rect, 6.0, fill);
            }
            let icon = if look.full {
                icons::EXPAND_LESS
            } else {
                icons::EXPAND_MORE
            };
            icons::paint(
                ui.painter(),
                view_rect.shrink(2.0),
                icon,
                ui.visuals().weak_text_color(),
            );
        }
    }
    // A click on the struck Lines' fold shows or hides them (#189).
    if let Some((rect, said)) = &struck_rect {
        let fold = ui.interact(*rect, struck_id.with("fold"), Sense::click());
        fold.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, said));
        if fold
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            folded = Some((struck_shown_setting(&note.id), !struck_open));
        }
    }
    // Over the card too: a click on the description folds or unfolds it,
    // not the card's own (#188).
    if let Some(rect) = description_rect
        && description_fold(ui, t, (rect, fold_id), unfolded)
    {
        folded = Some((description_unfolded_setting(&note.id), !unfolded));
    }
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
    // ⋮ and a right-click on the card open the same menu, read as it
    // opens (#120).
    let mut menu_opened = resp.secondary_clicked();
    let mut picked = None;
    let menu = look.menu.filter(|m| m.note == note.id);
    let has_boxes = !note.boxes.is_empty() || look.sections.iter().any(|s| !s.boxes.is_empty());
    let mut entries = |ui: &mut Ui| {
        let text = ui.visuals().text_color();
        if ui.button(RichText::new(t.edit()).color(text)).clicked() {
            picked = Some(Action::SelectNote(note.id.clone()));
        }
        // With the cards selected along, as the list makes it (#148).
        if look.changes
            && ui
                .button(RichText::new(t.file_into()).color(text))
                .clicked()
        {
            picked = Some(Action::AskFileInto(vec![note.id.clone()]));
        }
        // A Shopping List's ways onto the Agenda before "Back to a plain
        // Note", as on the phone (#186).
        if let Some(asked) = menu.and_then(|m| shopping_view::card_on_agenda(ui, t, m)) {
            picked = Some(Action::Shopping(asked));
        }
        if let Some(asked) = menu.and_then(|m| checklist_view::card_menu_entries(ui, t, m)) {
            picked = Some(Action::Checklist(asked));
        }
        if let Some(asked) = menu.and_then(|m| shopping_view::card_menu_entry(ui, t, m)) {
            picked = Some(Action::Shopping(asked));
        }
        // To a person as a Sent Item, or to another app (#129).
        if ui
            .button(RichText::new(t.send_to_person()).color(text))
            .clicked()
        {
            picked = Some(Action::StartSend(note.id.clone()));
        }
        if ui
            .button(RichText::new(t.copy_as_text()).color(text))
            .clicked()
        {
            picked = Some(Action::CopyAsText(note.id.clone()));
        }
        if ui
            .button(RichText::new(t.save_as_markdown()).color(text))
            .clicked()
        {
            picked = Some(Action::SaveAsMarkdown(note.id.clone()));
        }
        // Who ticked each box last and when (ADR 0035), where it has any.
        if has_boxes {
            let mut on = look.boxes.show;
            let label = RichText::new(t.box_show_ticks()).color(text);
            if theme::checkbox(ui, &mut on, label).clicked() {
                picked = Some(Action::ShowTicks {
                    note: note.id.clone(),
                    on,
                });
            }
        }
        // A pick closes the menu, also one made with the keyboard or a
        // screen reader, which no pointer click closes.
        if picked.is_some() {
            ui.close();
        }
    };
    if let Some(more_rect) = more_rect {
        let title = row_title(t, Some(note));
        let label = checklist_view::menu_label(t, title.lines().next().unwrap_or_default());
        let more = ui.interact(more_rect, Id::new(("more", &note.id)), Sense::click());
        more.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &label));
        if ui.is_rect_visible(more_rect) {
            if more.hovered() {
                let fill = ui.visuals().widgets.hovered.weak_bg_fill;
                ui.painter().rect_filled(more_rect, 6.0, fill);
            }
            let fg = ui.visuals().weak_text_color();
            icons::paint(ui.painter(), more_rect.shrink(2.0), icons::MORE_VERT, fg);
        }
        menu_opened |= more.clicked();
        let more = more.on_hover_text(&label);
        egui::Popup::menu(&more).show(&mut entries);
        resp.context_menu(&mut entries);
    }
    if menu_opened {
        ui.ctx().request_repaint();
    }
    // A click on a photo opens it full screen (UI laws, "Cards").
    let mut photo = None;
    for (i, (hash, rect)) in shown.into_iter().enumerate() {
        let id = Id::new(("photo", &note.id, i));
        if media_view::photo_click(ui, t, rect, id).clicked() {
            photo = Some(hash);
        }
    }
    let checks = look.checklist.is_some_and(|(_, checks)| checks)
        || look.shopping.is_some_and(|(_, checks)| checks);
    let check = checklist_view::card_checks(ui, tones, &note.id, &boxes, checks);
    let occurrence =
        card_occurrences::card_clicks(ui, (t, tones), &note.id, look.today, &today_rects);
    // Sections have no edit of their text: their marks only show. A
    // table opens full screen, to edit where its marks may change (#128);
    // taken first, its marks, tags and links keep their clicks.
    let of = Id::new(("card", &note.id));
    let mut table = text_marks_view::table_clicks(ui, t, of, &marks.tables).map(|at| Opening {
        note: look.changes.then(|| note.id.clone()),
        text: note.text.clone(),
        at,
        cell: None,
        tags: true,
        boxes: Some(note.id.clone()),
    });
    for (section, spots) in &section_marks {
        let of = Id::new(("card", &section.id));
        if let Some(at) = text_marks_view::table_clicks(ui, t, of, &spots.tables) {
            table = Some(Opening {
                note: None,
                text: section.text.clone(),
                at,
                cell: None,
                tags: true,
                boxes: Some(section.id.clone()),
            });
        }
    }
    // A box of its text, or of a Section's, ticks where this Device may
    // (ADR 0035).
    let ticks = look.boxes.ticks.contains(&note.id);
    let mut tick =
        text_marks_view::box_clicks(ui, t, of, &marks.boxes, (ticks, &ticked_in(&note.boxes)))
            .map(|asked| (note.id.clone(), asked));
    // A click on a tag searches for it, and opens no card (#126).
    let mut tag = text_marks_view::tag_clicks(ui, t, of, &marks.tags);
    // A click on a link opens it, and no card (#127).
    text_marks_view::link_clicks(ui, of, &marks.links);
    for (section, spots) in &section_marks {
        let of = Id::new(("card", &section.id));
        let ticks = look.boxes.ticks.contains(&section.id);
        let ticked = ticked_in(&section.boxes);
        if let Some(asked) = text_marks_view::box_clicks(ui, t, of, &spots.boxes, (ticks, &ticked))
        {
            tick = Some((section.id.clone(), asked));
        }
        tag = text_marks_view::tag_clicks(ui, t, of, &spots.tags).or(tag);
        text_marks_view::link_clicks(ui, of, &spots.links);
    }
    CardClicks {
        card: resp.on_hover_cursor(egui::CursorIcon::PointingHand),
        author,
        edit: edit.on_hover_text(t.edit()),
        view: view.on_hover_text(view_said),
        folded,
        photo,
        check,
        occurrence,
        tick,
        table,
        tag,
        menu_opened,
        picked,
        tally: tally_asked,
        session: session_asked,
    }
}

/// Where the table beginning at the byte `at` of `from` begins in `to`:
/// the table in the same place among the tables there, for `to` the same
/// text written again (its ends trimmed, say).
fn same_table((from, at): (&str, usize), to: &str) -> Option<usize> {
    let index = markdown::table::all(from)
        .iter()
        .position(|t| t.start == at)?;
    markdown::table::all(to).get(index).map(|t| t.start)
}

/// Where the `2` lines after the byte `from` of `text` lie in it, from the
/// first with words, without the spaces around them, as a compact card
/// shows them under a plain Note's first line (#189); none when no words
/// follow.
fn next_lines(text: &str, from: usize) -> Option<(usize, usize)> {
    let rest = text.get(from..)?;
    let start = from + rest.len() - rest.trim_start().len();
    let after = text.get(start..)?;
    if after.is_empty() {
        return None;
    }
    let end = match after.match_indices('\n').nth(1) {
        Some((at, _)) => start + at,
        None => text.len(),
    };
    let lines = text.get(start..end)?.trim_end();
    Some((start, start + lines.len()))
}

/// Where the first line a compact card shows of `text` lies, in bytes:
/// [`first_line`], or, a formatted text opening with a code block, the
/// first line with words after its opening fence, which shows nothing
/// (#212).
fn card_first_line(text: &str, formatted: bool) -> Option<(usize, usize)> {
    let line = first_line(text)?;
    if formatted
        && let Some(block) = markdown::parse(text).get(markdown::line_at(text, line.0))
        && block
            .code
            .as_ref()
            .is_some_and(|c| c.first_line == block.line)
        && let Some((a, b)) = text.get(block.end..).and_then(first_line)
    {
        return Some((block.end + a, block.end + b));
    }
    Some(line)
}

/// The lines the compact view says of a Tally or of a Session (#189).
fn card_glance_lines(ui: &mut Ui, glance: &[String]) {
    let text = ui.visuals().text_color();
    for line in glance {
        ui.label(RichText::new(line).size(14.0).color(text));
    }
}

/// Where the first line of `text` that a compact card shows lies in it,
/// in bytes: the text's first line with words, none for a text without
/// any.
fn first_line(text: &str) -> Option<(usize, usize)> {
    let start = text.len() - text.trim_start().len();
    let rest = text.get(start..)?;
    let line = rest.trim_end().lines().next()?;
    Some((start, start + line.len()))
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
