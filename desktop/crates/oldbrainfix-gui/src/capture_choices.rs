//! What a Note is written with as it is captured (#113), as the phone's
//! `capture_choices.dart` holds it: the Area or Topic it goes into, its
//! Kind, its Importance, the first field of the Tally it becomes, the
//! Routines, Reminders and Sessions spawned from it, a Frog and the
//! Shopping List its lines go into. None of it is written before the
//! Note is: [`capture`] writes the Note and all of it in one batch, so a
//! write that fails on the way leaves nothing, and an empty Note writes
//! nothing at all.
//!
//! The quick note, the capture popup and the window's quick note dialog,
//! shows only the text, the Nudge buttons and Open in the editor (#352,
//! obf-1080); the Note dialog does the rest once the Note is saved. Both
//! save through
//! [`save`], so the two cannot tell this apart. The Kind (#310) applies
//! to the typed text only: with nothing typed the Note stays a Plain
//! Note. With a Shopping List set (#124), each line typed becomes a Line
//! of it instead and no Note is kept; a voice capture (#311) stays on
//! the list as a Section, its recording to be listened to
//! (`ShoppingLists::add_voice_capture`). No Line holds a photo or a file:
//! a photo held drops the list ([`hold_photo`]).
//!
//! [`pour_into_shopping`] does the same for a Note already saved, as the
//! Note dialog's Add to a Shopping List asks (#352).

use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::Duration;
use egui::{Atom, Button, Id, Key, Modifiers, Response, RichText, Ui, Vec2};
use oldbrainfix_core::{
    Checklists, DeviceSettings, Error, Frogs, Importance, NewFile, Note, Notes, Routines, Schedule,
    SessionRule, Sessions, ShoppingLists, Store, Tallies, TallyRule, WallClock, markdown,
    text_format,
};

use crate::app::Wall;
use crate::attachment_view::HeldFile;
use crate::enrich::Said;
use crate::icons;
use crate::l10n::L10n;

/// A Session's pool, count, weekdays' own counts and rule as plain
/// values, kept after its editor is gone.
#[derive(Debug, Clone, PartialEq)]
pub struct SessionChoice {
    pub pool: Vec<String>,
    pub count: i64,
    /// The own count of each weekday that has one, 1 for Monday.
    pub counts: BTreeMap<u32, i64>,
    pub rule: SessionRule,
}

impl SessionChoice {
    /// Spawns it from the Note `note_id` on `schedule`; the Session's id.
    pub fn spawn(
        &self,
        store: &Store,
        note_id: &str,
        schedule: &Schedule,
    ) -> oldbrainfix_core::Result<String> {
        let sessions = Sessions::new(store);
        let made = sessions.spawn(note_id, schedule, &self.pool, self.count, self.rule)?;
        // The weekdays' counts lie on the Note (ADR 0039).
        for (day, own) in &self.counts {
            sessions.set_count_on(made.note_id(), *day, Some(*own))?;
        }
        Ok(made.routine.id)
    }

    /// Makes the plain Note `note_id` a Session of it, without a Routine
    /// (#191), in one batch.
    pub fn make(&self, store: &Store, note_id: &str) -> oldbrainfix_core::Result<()> {
        let sessions = Sessions::new(store);
        store.batch(|| {
            sessions.make(note_id, &self.pool, self.count, self.rule)?;
            // Making it takes away any weekday's count left before.
            for (day, own) in &self.counts {
                sessions.set_count_on(note_id, *day, Some(*own))?;
            }
            Ok(())
        })
    }
}

/// A Routine, Reminder or Session set up before its Note is written:
/// what its editor saves, held until the Note is.
#[derive(Debug, Clone, PartialEq)]
pub struct ScheduleChoice {
    pub schedule: Schedule,
    /// Its own Re-nag interval; `None` for the Device's.
    pub renag: Option<Duration>,
    /// After how many ignored Re-nags it escalates; `None` when it does
    /// not.
    pub escalation: Option<i64>,
    /// The Session's part; `None` for a plain Routine or Reminder.
    pub session: Option<SessionChoice>,
    /// The person's own Heads-ups, in minutes before an Occurrence
    /// (obf-1046); none for none.
    pub heads_ups: Vec<i64>,
}

impl ScheduleChoice {
    /// Spawns it from the Note `note_id`; the Routine's id.
    pub fn spawn_from(&self, store: &Store, note_id: &str) -> oldbrainfix_core::Result<String> {
        let routines = Routines::new(store);
        let id = match &self.session {
            None => routines.spawn(note_id, &self.schedule)?.id,
            Some(session) => session.spawn(store, note_id, &self.schedule)?,
        };
        routines.set_renag(&id, self.renag)?;
        routines.set_escalation(&id, self.escalation)?;
        routines.set_heads_ups(&id, &self.heads_ups)?;
        Ok(id)
    }
}

/// The first field of the Tally the Note becomes, as its page saves it;
/// `target` in the base unit.
#[derive(Debug, Clone, PartialEq)]
pub struct TallyFieldChoice {
    pub name: String,
    pub unit: String,
    pub target: Option<f64>,
    pub rule: TallyRule,
    /// Whether it is a Check field (#264).
    pub check: bool,
}

/// The Kind a capture makes its Note (#310): a Plain Note of its text,
/// or a Checklist or Shopping List of its typed lines.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CaptureKind {
    #[default]
    Text,
    Checklist,
    ShoppingList,
}

/// Everything set up for the Note being captured.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CaptureChoices {
    /// The Kind it is made as it is written (#310).
    pub kind: CaptureKind,
    /// The Area or Topic it goes into.
    pub place: Option<String>,
    pub importance: Importance,
    pub tally: Option<TallyFieldChoice>,
    pub schedules: Vec<ScheduleChoice>,
    pub frog: bool,
    /// The Note of the Shopping List the lines go into instead of a
    /// Note; while one is, the rest waits unwritten.
    pub shopping: Option<String>,
}

impl CaptureChoices {
    /// Whether the Note leaves Unsorted going into `place`: filed, with
    /// something spawned from it or made of it, made a list (#310), or
    /// going into a Shopping List. It needs no Nudge then.
    pub fn sorted(&self, place: Option<&str>) -> bool {
        place.is_some()
            || self.list_kind()
            || self.tally.is_some()
            || !self.schedules.is_empty()
            || self.frog
            || self.shopping.is_some()
    }

    /// Whether a Checklist or Shopping List is chosen as the Kind (#310).
    pub fn list_kind(&self) -> bool {
        self.kind != CaptureKind::Text
    }

    /// Writes all of it into the Note `note_id`, which holds its place
    /// and its format already, one after the other: first the Kind, made
    /// where this Device may, else it stays a Plain Note, as Capture asks
    /// nothing (#310). Meant for the batch [`capture`] runs, as anything
    /// failing on the way is to undo the rest.
    pub fn write(&self, store: &Store, note_id: &str) -> oldbrainfix_core::Result<()> {
        match self.kind {
            CaptureKind::Text => {}
            CaptureKind::Checklist => {
                let lists = Checklists::new(store);
                if lists.may_make(note_id)? {
                    lists.make_from_text(note_id)?;
                }
            }
            CaptureKind::ShoppingList => {
                let lists = ShoppingLists::new(store);
                if lists.may_make(note_id)? {
                    lists.make_from_text(note_id)?;
                }
            }
        }
        if self.importance != Importance::Normal {
            Notes::new(store).set_importance(note_id, self.importance)?;
        }
        if let Some(tally) = &self.tally {
            let tallies = Tallies::new(store);
            if tally.check {
                tallies.add_check(note_id, &tally.name, tally.target)?;
            } else {
                tallies.add_field(
                    note_id,
                    &tally.name,
                    &tally.unit,
                    tally.target,
                    Some(tally.rule),
                )?;
            }
        }
        for schedule in &self.schedules {
            schedule.spawn_from(store, note_id)?;
        }
        if self.frog {
            Frogs::new(store).spawn(note_id)?;
        }
        Ok(())
    }
}

/// Whether this Device may spawn from a Note in `place`: anywhere but in
/// a Shared Area it does not manage (ADR 0013).
pub fn may_spawn(store: &Store, place: Option<&str>) -> oldbrainfix_core::Result<bool> {
    let area = match place {
        Some(place) => store.area_of(place)?,
        None => None,
    };
    store.may_manage(area.as_deref())
}

/// A Shopping List a capture can go into: its Note's id and its title.
pub type ListTitle = (String, String);

/// What a capture came to.
#[derive(Debug, Clone, PartialEq)]
pub enum Captured {
    /// Nothing, as the Note was empty.
    Nothing,
    /// This Note, with all that was set up for it.
    Note(Note),
    /// Its lines went into the Shopping List; no Note is kept but a voice
    /// capture's, merged into the list as a Section (#311).
    Poured,
    /// This Note, with all that was set up for it but the Shopping List,
    /// which can no longer be added to.
    Kept(Note),
}

/// Captures `text` with `photos` and `recordings` as a new Note in
/// `place`, nudged `nudge_in` from now unless it is sorted, and writes
/// `choices` into it, all in one batch: when a write on the way fails,
/// none of it is written. Nothing is written when the Note is empty.
///
/// With a Shopping List picked, each line of `text` becomes a Line of
/// it, days as `wall` shows them, and no Note is written; with
/// recordings, the Note is merged into the list, owing it the items said
/// in their Transcripts (#311). A photo keeps it a Note, as does a list
/// that can no longer be added to.
pub fn capture(
    store: &Store,
    text: &str,
    place: Option<&str>,
    nudge_in: Duration,
    (photos, recordings, files): (&[Vec<u8>], &[Vec<u8>], &[HeldFile]),
    (choices, wall): (&CaptureChoices, &WallClock),
) -> oldbrainfix_core::Result<Captured> {
    let text = text.trim();
    if text.is_empty() && photos.is_empty() && recordings.is_empty() && files.is_empty() {
        return Ok(Captured::Nothing);
    }
    let attached: Vec<NewFile> = files.iter().map(HeldFile::new_file).collect();
    let note = |choices: &CaptureChoices| {
        let mut choices = choices.clone();
        choices.shopping = None;
        // The Kind is made of the typed lines: nothing typed, it stays a
        // Plain Note (#310).
        if text.is_empty() {
            choices.kind = CaptureKind::Text;
        }
        // A sorted Note needs no Nudge (#113).
        let nudge = (!choices.sorted(place)).then_some(nudge_in);
        store.batch(|| {
            let note = Notes::new(store)
                .capture_with_files(text, place, nudge, photos, recordings, &attached)?;
            formatted(store, &note)?;
            choices.write(store, &note.id)?;
            Ok(note)
        })
    };
    // No Line holds a photo or a file: the capture stays a Note, to keep
    // it.
    let Some(list) = choices
        .shopping
        .as_deref()
        .filter(|_| photos.is_empty() && files.is_empty())
    else {
        return note(choices).map(Captured::Note);
    };
    // The list used last goes to the top of the pick (#276).
    let used = || DeviceSettings::new(store).set_last_shopping_list(Some(list));
    if recordings.is_empty() {
        return match ShoppingLists::new(store).add_lines(list, text, wall) {
            Ok(added) => {
                if !added.is_empty() {
                    used()?;
                }
                Ok(Captured::Poured)
            }
            // The list went meanwhile: a Note with all that was set up.
            Err(Error::Invalid(_)) => note(choices).map(Captured::Kept),
            Err(e) => Err(e),
        };
    }
    // The recordings stay on the list as a Section, to be listened to.
    let voiced = store.batch(|| {
        let note = Notes::new(store).capture_with(text, None, None, &[], recordings)?;
        formatted(store, &note)?;
        ShoppingLists::new(store).add_voice_capture(list, &note.id, wall)
    });
    match voiced {
        Ok(_) => {
            used()?;
            Ok(Captured::Poured)
        }
        // The list went meanwhile, or it lies in a Shared Area a Note
        // Hidden from AI goes into none of (#267): a Note with all that
        // was set up.
        Err(Error::Invalid(_) | Error::HiddenFromAi(_)) => note(choices).map(Captured::Kept),
        Err(e) => Err(e),
    }
}

/// The full text behind "kept as a Note": the Shopping List `list` can
/// no longer be added to.
fn gone_details(list: &str) -> String {
    format!("shopping list {list}: it can no longer be added to")
}

/// What is held with the text being captured: the photos, the
/// recordings and the files.
#[derive(Debug, Clone, Copy)]
pub struct Held<'a> {
    pub photos: &'a [Vec<u8>],
    pub recordings: &'a [Vec<u8>],
    /// The files to attach (#267).
    pub files: &'a [HeldFile],
}

/// Saves a capture as the popup and the window do: [`capture`] of `text`
/// with what is `held` and `choices`. With it, what to announce at the
/// top, in the words of `t`, when the capture was kept as a Note instead
/// of going into the list picked.
pub fn save(
    (store, wall): (&Arc<Store>, &Wall),
    text: &str,
    place: Option<&str>,
    nudge_in: Duration,
    held: Held<'_>,
    choices: &CaptureChoices,
    t: L10n,
) -> oldbrainfix_core::Result<(Captured, Option<Said>)> {
    let captured = capture(
        store,
        text,
        place,
        nudge_in,
        (held.photos, held.recordings, held.files),
        (choices, &**wall),
    )?;
    let list = choices.shopping.as_deref().unwrap_or("");
    let said = match &captured {
        Captured::Kept(_) => Some((t.capture_shopping_gone(), gone_details(list))),
        _ => None,
    };
    Ok((captured, said))
}

/// Holds `photo` with the `photos` of the Note being captured. No Line
/// holds a photo, so the Shopping List picked in `choices` is dropped
/// (UI laws, "Capture").
pub fn hold_photo(photos: &mut Vec<Vec<u8>>, choices: &mut CaptureChoices, photo: Vec<u8>) {
    photos.push(photo);
    choices.shopping = None;
}

/// Makes the text of the Note `note`, just captured, a formatted text
/// when it was typed or pasted with a format (#127, ADR 0033).
fn formatted(store: &Store, note: &Note) -> oldbrainfix_core::Result<()> {
    if markdown::uses(&note.text) {
        Notes::new(store).set_format(&note.id, text_format::MARKDOWN)?;
    }
    Ok(())
}

/// What a Shopping List is called where it is picked: the first line of
/// its text, else of its Transcript or Photo Text ([`Note::shown_name`],
/// obf-1049), "Shopping List" while that is blank.
pub fn shopping_title(t: &L10n, note: &Note) -> String {
    match note.shown_name().trim().split('\n').next().map(str::trim) {
        Some(title) if !title.is_empty() => title.to_string(),
        _ => t.shopping_heading().to_string(),
    }
}

/// The Shopping Lists a capture can go into, by their Note's id with
/// their title: each one this Device may add Lines to, oldest capture
/// first, but the one a capture on this Device went into last at the top
/// (#276).
pub fn shopping_lists(store: &Store, t: &L10n) -> oldbrainfix_core::Result<Vec<ListTitle>> {
    let mut lists: Vec<ListTitle> = ShoppingLists::new(store)
        .addable()?
        .iter()
        .map(|note| (note.id.clone(), shopping_title(t, note)))
        .collect();
    if let Some(last) = DeviceSettings::new(store).last_shopping_list()?
        && let Some(at) = lists.iter().position(|(id, _)| *id == last)
    {
        let used = lists.remove(at);
        lists.insert(0, used);
    }
    Ok(lists)
}

/// "Add to a Shopping List" in a menu: its heading over `lists`, by
/// their Note's id with their title, as radio buttons, the one picked
/// before on; with one list a single entry naming it, "Add to Weekly"
/// (#276). The one picked.
pub fn shopping_picker(
    ui: &mut Ui,
    t: &L10n,
    current: Option<&str>,
    lists: &[ListTitle],
) -> Option<String> {
    let text = ui.visuals().text_color();
    if let [(id, title)] = lists {
        let on = current == Some(id.as_str());
        let words = t.add_to_named_shopping_list(title);
        let entry = if on {
            ui.radio(true, RichText::new(words).color(text))
        } else {
            ui.button(RichText::new(words).color(text))
        };
        if entry.clicked() {
            ui.close();
            return (!on).then(|| id.clone());
        }
        return None;
    }
    ui.label(RichText::new(t.add_to_shopping_list()).strong().color(text));
    let mut picked = None;
    for (id, title) in lists {
        let on = current == Some(id.as_str());
        if ui.radio(on, RichText::new(title).color(text)).clicked() {
            if !on {
                picked = Some(id.clone());
            }
            // A pick closes the menu, also one by keyboard or screen
            // reader, which no pointer click closes.
            ui.close();
        }
    }
    picked
}

/// What this Device's quick note, its dialog in the window and the
/// popup, shows besides its text and Open in the editor (#142, #352,
/// ADR 0008): the Nudge buttons until switched off in Settings ▸ Quick
/// note.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuickNote {
    pub nudge_buttons: bool,
}

impl Default for QuickNote {
    /// As a Device that switched nothing.
    fn default() -> QuickNote {
        QuickNote {
            nudge_buttons: true,
        }
    }
}

impl QuickNote {
    /// What this Device's settings switched on.
    pub fn read(store: &Store) -> oldbrainfix_core::Result<QuickNote> {
        let settings = DeviceSettings::new(store);
        Ok(QuickNote {
            nudge_buttons: settings.quick_note_nudge_buttons_on()?,
        })
    }
}

/// The Nudge times of the capture's buttons, 10 min, 30 min and 1 h.
pub fn nudge_times() -> [Duration; 3] {
    [
        Duration::minutes(10),
        Duration::minutes(30),
        Notes::default_nudge(),
    ]
}

/// The Nudge a key in the capture asks to save with, taken (#277): Ctrl+1,
/// Ctrl+2 and Ctrl+3 (Cmd on macOS) the Nudge buttons' times while
/// `nudges` they show, Ctrl+Enter the 1 h. Only in the dialog on top,
/// so a dialog over the capture keeps its keys.
pub fn nudge_key(ui: &Ui, nudges: bool) -> Option<Duration> {
    if !crate::theme::on_top(ui) {
        return None;
    }
    ui.input_mut(|i| {
        if nudges {
            let keys = [Key::Num1, Key::Num2, Key::Num3];
            for (key, nudge_in) in keys.into_iter().zip(nudge_times()) {
                if i.consume_key(Modifiers::COMMAND, key) {
                    return Some(nudge_in);
                }
            }
        }
        i.consume_key(Modifiers::COMMAND, Key::Enter)
            .then(Notes::default_nudge)
    })
}

/// The quick note's Open in the editor (obf-1080), under the Nudge
/// buttons or their Save: a full-width outlined button, the tonal fill
/// with the outline (obf-1081), with the editor's pencil before its
/// words, greyed unless `enabled`. Its caller saves
/// the Note and opens it in the Note dialog.
pub fn open_in_editor(ui: &mut Ui, t: &L10n, enabled: bool) -> Response {
    let text = ui.visuals().text_color();
    let pencil = Id::new("open in the editor pencil");
    ui.add_enabled_ui(enabled, |ui| {
        // The pencil and the words in the middle of the width.
        let atoms = (
            Atom::grow(),
            Atom::custom(pencil, Vec2::splat(18.0)),
            RichText::new(t.capture_open_in_editor()).color(text),
            Atom::grow(),
        );
        let shown = Button::new(atoms)
            .min_size(egui::vec2(ui.available_width(), 0.0))
            .atom_ui(ui);
        if let Some(rect) = shown.rect(pencil) {
            icons::paint(ui.painter(), rect, icons::EDIT, text);
        }
        shown.response
    })
    .inner
}

/// Makes each line of the text of the saved Note `note_id` a Line of the
/// Shopping List `list`, a name the list has bringing its Line back, and
/// lets the Note go, all in one batch, as the phone's `pourIntoShopping`
/// does (#124, #352): the Note dialog's Add to a Shopping List. A Note
/// with a recording stays on the list as a Section instead, owing it the
/// items said in its Transcripts (#311). The list goes to the top of the
/// pick (#276). Whether it went; false when the list can no longer be
/// added to, or the Note may no longer go (deleted, merged or with
/// something spawned from it meanwhile): the Note stays then.
pub fn pour_into_shopping(
    store: &Store,
    note_id: &str,
    list: &str,
    wall: &WallClock,
) -> oldbrainfix_core::Result<bool> {
    let notes = Notes::new(store);
    let Some(note) = notes.by_id(note_id)? else {
        return Ok(false);
    };
    if note.is_trashed() || note.is_merged() || note.spawned {
        return Ok(false);
    }
    let lists = ShoppingLists::new(store);
    let poured = if note.recordings.is_empty() {
        store.batch(|| {
            // Its boxes as they show, without their ids (ADR 0035).
            lists.add_lines(list, &note.shown_text(), wall)?;
            notes.discard(note_id)
        })
    } else {
        lists.add_voice_capture(list, note_id, wall).map(drop)
    };
    match poured {
        Ok(()) => {
            DeviceSettings::new(store).set_last_shopping_list(Some(list))?;
            Ok(true)
        }
        Err(Error::Invalid(_) | Error::HiddenFromAi(_)) => Ok(false),
        Err(e) => Err(e),
    }
}
