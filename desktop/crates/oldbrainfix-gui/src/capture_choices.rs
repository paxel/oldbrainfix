//! What is set up for a Note while it is captured (#113), as the phone's
//! `capture_choices.dart` holds it: the Area or Topic it goes into, its
//! Importance, the first field of the Tally it becomes, the Routines,
//! Reminders and Sessions spawned from it and a Frog. None of it is
//! written before the Note is: [`capture`] writes the Note and all of it
//! in one batch, so a write that fails on the way leaves nothing, and an
//! empty Note writes nothing at all.
//!
//! The capture popup and the window's capture field share it, with the
//! Note card's buttons ([`action_row`]) and the choices shown at the top
//! ([`chips`]), each with a button that drops it again. Merging, sending
//! and the trash need a saved Note and are not offered here.
//!
//! "Add to a Shopping List" in "More" (#124) makes each line typed a Line
//! of the list picked instead, and keeps no Note; the other choices wait
//! unshown and unwritten meanwhile, and come back when its chip is
//! dropped. A recording gives the lines of its Transcript: as the desktop
//! transcribes only what is written, such a capture is written Unsorted
//! with nothing else and goes into the list once the Transcripts are
//! there ([`pour_later`]); without one it stays a Note, announced at the
//! top. No Line holds a photo, so a photo keeps it a Note.

use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::Duration;
use egui::{Frame, Margin, RichText, Stroke, Ui};
use oldbrainfix_core::{
    Area, Error, Frogs, Importance, Note, Notes, Routines, Schedule, SessionRule, Sessions,
    ShoppingLists, Store, Tallies, TallyRule, Topic, WallClock, markdown, text_format,
};

use crate::app::{Wall, icon_button};
use crate::enrich::{Enricher, Said};
use crate::icons;
use crate::l10n::L10n;
use crate::names::{area_name, topic_name};
use crate::schedule_text::schedule_text;

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
        let id = sessions
            .spawn(note_id, schedule, &self.pool, self.count, self.rule)?
            .routine
            .id;
        for (day, own) in &self.counts {
            sessions.set_count_on(&id, *day, Some(*own))?;
        }
        Ok(id)
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
}

/// Everything set up for the Note being captured.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CaptureChoices {
    /// The Area or Topic picked with "File into…".
    pub place: Option<String>,
    pub importance: Importance,
    pub tally: Option<TallyFieldChoice>,
    pub schedules: Vec<ScheduleChoice>,
    pub frog: bool,
    /// The Note of the Shopping List the lines go into instead of a
    /// Note; while one is, the rest waits unwritten.
    pub shopping: Option<String>,
}

/// Something on the capture's buttons or its choices to spawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Spawn {
    Routine,
    Reminder,
    Frog,
    Session,
}

/// One of the choices shown at the top.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chip {
    Place,
    Importance,
    Tally,
    /// The Routine, Reminder or Session at this index.
    Schedule(usize),
    Frog,
    Shopping,
}

/// What was asked of the capture's buttons or its choices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ask {
    FileInto(String),
    Spawn(Spawn),
    Importance(Importance),
    MakeTally,
    /// The lines go into the Shopping List of this Note.
    Shopping(String),
    /// Drops the choice again.
    Remove(Chip),
}

/// The page a button opens, which hands back its choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opens {
    Schedule { reminder: bool, session: bool },
    Tally,
}

impl CaptureChoices {
    /// Whether the Note leaves Unsorted going into `place`: filed, with
    /// something spawned from it or made of it, or going into a Shopping
    /// List. It needs no Nudge then.
    pub fn sorted(&self, place: Option<&str>) -> bool {
        place.is_some()
            || self.tally.is_some()
            || !self.schedules.is_empty()
            || self.frog
            || self.shopping.is_some()
    }

    /// Writes all of it into the Note `note_id`, which holds its place
    /// already, one after the other; meant for the batch [`capture`]
    /// runs, as anything failing on the way is to undo the rest.
    pub fn write(&self, store: &Store, note_id: &str) -> oldbrainfix_core::Result<()> {
        if self.importance != Importance::Normal {
            Notes::new(store).set_importance(note_id, self.importance)?;
        }
        if let Some(tally) = &self.tally {
            Tallies::new(store).add_field(
                note_id,
                &tally.name,
                &tally.unit,
                tally.target,
                Some(tally.rule),
            )?;
        }
        for schedule in &self.schedules {
            schedule.spawn_from(store, note_id)?;
        }
        if self.frog {
            Frogs::new(store).spawn(note_id)?;
        }
        Ok(())
    }

    /// Drops what may not go into `place`: in a Shared Area only a
    /// Manager spawns (ADR 0013), and a Session there picks only Tallies
    /// of that Area (ADR 0029).
    pub fn keep_what_fits(
        &mut self,
        store: &Store,
        place: Option<&str>,
    ) -> oldbrainfix_core::Result<()> {
        if !may_spawn(store, place)? {
            self.schedules.clear();
            self.frog = false;
            return Ok(());
        }
        let fits = Sessions::new(store).choices(place.unwrap_or(""))?;
        self.schedules.retain(|choice| {
            choice
                .session
                .as_ref()
                .is_none_or(|s| s.pool.iter().all(|id| fits.contains(id)))
        });
        Ok(())
    }

    /// Takes what was asked; the page to open for it, if any.
    pub fn take(&mut self, asked: Ask, store: &Store) -> oldbrainfix_core::Result<Option<Opens>> {
        match asked {
            Ask::FileInto(picked) => {
                self.keep_what_fits(store, Some(&picked))?;
                self.place = Some(picked);
            }
            Ask::Spawn(Spawn::Frog) => self.frog = true,
            Ask::Spawn(spawn) => {
                return Ok(Some(Opens::Schedule {
                    reminder: spawn == Spawn::Reminder,
                    session: spawn == Spawn::Session,
                }));
            }
            Ask::Importance(importance) => self.importance = importance,
            Ask::MakeTally => return Ok(Some(Opens::Tally)),
            Ask::Shopping(list) => self.shopping = Some(list),
            Ask::Remove(chip) => match chip {
                Chip::Place => self.place = None,
                Chip::Importance => self.importance = Importance::Normal,
                Chip::Tally => self.tally = None,
                Chip::Schedule(i) if i < self.schedules.len() => drop(self.schedules.remove(i)),
                Chip::Schedule(_) => {}
                Chip::Frog => self.frog = false,
                Chip::Shopping => self.shopping = None,
            },
        }
        Ok(None)
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

/// What the capture's buttons offer: the Areas with their Topics to
/// file into, whether the Note may spawn where it goes, and the Shopping
/// Lists it can go into.
pub type Offer<'a> = (&'a [(Area, Vec<Topic>)], bool, &'a [ListTitle]);

/// What a capture came to.
#[derive(Debug, Clone, PartialEq)]
pub enum Captured {
    /// Nothing, as the Note was empty.
    Nothing,
    /// This Note, with all that was set up for it.
    Note(Note),
    /// Its lines went into the Shopping List; no Note is kept.
    Poured,
    /// This Note, Unsorted with nothing else, until the Transcripts of
    /// its recordings are there to go into the Shopping List with it.
    Waiting(Note),
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
/// recordings, the Note waits Unsorted for their Transcripts. A photo
/// keeps it a Note, as does a list that can no longer be added to.
pub fn capture(
    store: &Store,
    text: &str,
    place: Option<&str>,
    nudge_in: Duration,
    (photos, recordings): (&[Vec<u8>], &[Vec<u8>]),
    (choices, wall): (&CaptureChoices, &WallClock),
) -> oldbrainfix_core::Result<Captured> {
    let text = text.trim();
    if text.is_empty() && photos.is_empty() && recordings.is_empty() {
        return Ok(Captured::Nothing);
    }
    let note = |choices: &CaptureChoices| {
        let mut choices = choices.clone();
        choices.shopping = None;
        // A sorted Note needs no Nudge (#113).
        let nudge = (!choices.sorted(place)).then_some(nudge_in);
        store.batch(|| {
            let note = Notes::new(store).capture_with(text, place, nudge, photos, recordings)?;
            formatted(store, &note)?;
            choices.write(store, &note.id)?;
            Ok(note)
        })
    };
    // No Line holds a photo: the capture stays a Note, to keep it.
    let Some(list) = choices.shopping.as_deref().filter(|_| photos.is_empty()) else {
        return note(choices).map(Captured::Note);
    };
    if recordings.is_empty() {
        return match ShoppingLists::new(store).add_lines(list, text, wall) {
            Ok(_) => Ok(Captured::Poured),
            // The list went meanwhile: a Note with all that was set up.
            Err(Error::Invalid(_)) => note(choices).map(Captured::Kept),
            Err(e) => Err(e),
        };
    }
    // Until the Transcripts are there, the Note holds all of it.
    let note = Notes::new(store).capture_with(text, None, None, &[], recordings)?;
    formatted(store, &note)?;
    Ok(Captured::Waiting(note))
}

/// Makes the text of the Note `note`, just captured, a formatted text
/// when it was typed or pasted with a format (#127, ADR 0033).
fn formatted(store: &Store, note: &Note) -> oldbrainfix_core::Result<()> {
    if markdown::uses(&note.text) {
        Notes::new(store).set_format(&note.id, text_format::MARKDOWN)?;
    }
    Ok(())
}

/// Makes each line of the text of the Note `note_id` and of its
/// recordings' Transcripts a Line of the Shopping List `list_id`, a name
/// the list has bringing its Line back, and discards the Note, all in one
/// batch (#124). `Some(true)` when it did; `Some(false)` when the list can
/// no longer be added to, and `None` when the Note may no longer go:
/// deleted, merged or with something spawned from it meanwhile. Either
/// way the Note stays.
pub fn pour(
    store: &Store,
    note_id: &str,
    list_id: &str,
    wall: &WallClock,
) -> oldbrainfix_core::Result<Option<bool>> {
    let notes = Notes::new(store);
    let Some(note) = notes.by_id(note_id)? else {
        return Ok(None);
    };
    if note.is_trashed() || note.is_merged() || note.spawned {
        return Ok(None);
    }
    let mut lines = vec![note.text.as_str()];
    lines.extend(
        note.recordings
            .iter()
            .filter_map(|hash| note.transcripts.get(hash).map(String::as_str)),
    );
    let text = lines.join("\n");
    let poured = store.batch(|| {
        ShoppingLists::new(store).add_lines(list_id, &text, wall)?;
        notes.discard(note_id)
    });
    match poured {
        Ok(()) => Ok(Some(true)),
        Err(Error::Invalid(_)) => Ok(Some(false)),
        Err(e) => Err(e),
    }
}

/// Pours the Note `note_id` into the Shopping List `list_id` as [`pour`]
/// does once `transcriber` has read its recordings. Until then the Note
/// holds all of it, so nothing is lost when the app ends meanwhile.
/// Without a Transcript, or when the list can no longer be added to, the
/// Note stays and the window announces why at the top.
pub fn pour_later(
    transcriber: &Enricher,
    (store, wall): (Arc<Store>, Wall),
    t: L10n,
    note_id: &str,
    list_id: &str,
) {
    let (note, list) = (note_id.to_string(), list_id.to_string());
    transcriber.when_read(
        note_id,
        Box::new(move |read| -> Option<Said> {
            if !read {
                // Said unless it went meanwhile.
                let stays = Notes::new(&store)
                    .by_id(&note)
                    .ok()
                    .flatten()
                    .is_some_and(|n| !n.is_trashed());
                let details = format!("shopping list {list}: no Transcript for {note}");
                return stays.then_some((t.capture_shopping_untranscribed(), details));
            }
            match pour(&store, &note, &list, &*wall) {
                Ok(Some(false)) => Some((
                    t.capture_shopping_gone(),
                    format!("shopping list {list}: it can no longer be added to"),
                )),
                Ok(_) => None,
                Err(e) => Some((t.action_failed(), e.to_string())),
            }
        }),
    );
}

/// What a Shopping List is called where it is picked: the first line of
/// its text, "Shopping List" while that is blank.
pub fn shopping_title(t: &L10n, note: &Note) -> String {
    match note.visible_text().trim().split('\n').next().map(str::trim) {
        Some(title) if !title.is_empty() => title.to_string(),
        _ => t.shopping_heading().to_string(),
    }
}

/// The Shopping Lists a capture can go into, by their Note's id with
/// their title: each one this Device may add Lines to, oldest capture
/// first.
pub fn shopping_lists(store: &Store, t: &L10n) -> oldbrainfix_core::Result<Vec<ListTitle>> {
    Ok(ShoppingLists::new(store)
        .addable()?
        .iter()
        .map(|note| (note.id.clone(), shopping_title(t, note)))
        .collect())
}

/// The title of the Shopping List `id` among `lists`; "Shopping List"
/// for one no longer there.
pub fn shopping_name(t: &L10n, lists: &[ListTitle], id: &str) -> String {
    lists.iter().find(|(list, _)| list == id).map_or_else(
        || t.shopping_heading().to_string(),
        |(_, title)| title.clone(),
    )
}

/// The Area or Topic `id` as the user reads it, "Area / Topic" for a
/// Topic; `None` when `areas` has no such place.
pub fn place_name(t: &L10n, areas: &[(Area, Vec<Topic>)], id: &str) -> Option<String> {
    for (area, topics) in areas {
        if area.id == id {
            return Some(area_name(t, area));
        }
        if let Some(topic) = topics.iter().find(|topic| topic.id == id) {
            return Some(format!("{} / {}", area_name(t, area), topic_name(t, topic)));
        }
    }
    None
}

/// "File into…" as a menu of the Areas with their Topics indented; the
/// one picked.
pub fn file_into_menu(ui: &mut Ui, t: &L10n, areas: &[(Area, Vec<Topic>)]) -> Option<String> {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let mut picked = None;
    ui.menu_button(RichText::new(t.file_into()).color(text), |ui| {
        for (area, topics) in areas {
            let mut file = |ui: &mut Ui, id: &str, label: String, indent| {
                ui.horizontal(|ui| {
                    ui.add_space(indent);
                    if ui.button(RichText::new(label).color(text)).clicked() {
                        picked = Some(id.to_string());
                    }
                });
            };
            file(ui, &area.id, area_name(t, area), 0.0);
            for topic in topics {
                file(ui, &topic.id, topic_name(t, topic), 16.0);
            }
        }
        if areas.is_empty() {
            ui.label(RichText::new(t.no_areas_yet()).color(weak));
        }
    });
    picked
}

/// "Add to a Shopping List" in a menu: its heading over `lists`, by
/// their Note's id with their title, as radio buttons, the one picked
/// before on; the one picked.
fn shopping_picker(
    ui: &mut Ui,
    t: &L10n,
    current: Option<&str>,
    lists: &[ListTitle],
) -> Option<String> {
    let text = ui.visuals().text_color();
    ui.label(RichText::new(t.add_to_shopping_list()).strong().color(text));
    let mut picked = None;
    for (id, title) in lists {
        let on = current == Some(id.as_str());
        if ui.radio(on, RichText::new(title).color(text)).clicked() && !on {
            picked = Some(id.clone());
        }
    }
    picked
}

/// The Note card's buttons on a capture: "File into…" over `areas`,
/// Routine, Reminder, Frog and Session while `spawns`, else why not, and
/// "More" with the Importance, "Make it a Tally" and "Add to a Shopping
/// List" over `lists`, offered while there is one. While a Shopping List
/// is picked, "More" holds only that.
pub fn action_row(
    ui: &mut Ui,
    t: &L10n,
    choices: &CaptureChoices,
    (areas, spawns, lists): Offer<'_>,
) -> Option<Ask> {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let mut asked = None;
    // Lines need none of the rest: it waits while a list is picked.
    let shopping = choices.shopping.as_deref();
    if shopping.is_some() {
        ui.horizontal_wrapped(|ui| {
            ui.menu_button(RichText::new(t.more()).color(text), |ui| {
                if let Some(list) = shopping_picker(ui, t, shopping, lists) {
                    asked = Some(Ask::Shopping(list));
                }
            });
        });
        return asked;
    }
    ui.horizontal_wrapped(|ui| {
        if let Some(place) = file_into_menu(ui, t, areas) {
            asked = Some(Ask::FileInto(place));
        }
        if spawns {
            let offered = [
                (Spawn::Routine, t.routine()),
                (Spawn::Reminder, t.reminder()),
                (Spawn::Frog, t.frog()),
                (Spawn::Session, t.session()),
            ];
            for (spawn, label) in offered {
                // A Note has one open Frog at a time.
                if spawn == Spawn::Frog && choices.frog {
                    continue;
                }
                if ui.button(RichText::new(label).color(text)).clicked() {
                    asked = Some(Ask::Spawn(spawn));
                }
            }
        }
        ui.menu_button(RichText::new(t.more()).color(text), |ui| {
            ui.label(RichText::new(t.importance()).strong().color(text));
            let levels = [
                (Importance::Important, t.importance_important()),
                (Importance::Normal, t.importance_normal()),
                (Importance::Unimportant, t.importance_unimportant()),
            ];
            for (importance, label) in levels {
                let on = choices.importance == importance;
                if ui.radio(on, RichText::new(label).color(text)).clicked() && !on {
                    asked = Some(Ask::Importance(importance));
                }
            }
            if choices.tally.is_none() {
                ui.separator();
                if ui
                    .button(RichText::new(t.make_tally()).color(text))
                    .clicked()
                {
                    asked = Some(Ask::MakeTally);
                }
            }
            if !lists.is_empty() {
                ui.separator();
                if let Some(list) = shopping_picker(ui, t, None, lists) {
                    asked = Some(Ask::Shopping(list));
                }
            }
        });
    });
    if !spawns {
        ui.label(RichText::new(t.only_managers()).small().color(weak));
    }
    asked
}

/// What was set up for the Note, at the top, not saved yet: where it
/// goes (`place`, as the user reads it, and whether it can be dropped),
/// its Importance, its Tally, Routines, Reminders and Sessions and its
/// Frog, each with a button that drops it; nothing while there is none.
/// While a Shopping List is picked, only it shows, by its title `lists`
/// gives.
pub fn chips(
    ui: &mut Ui,
    t: &L10n,
    choices: &CaptureChoices,
    (place, lists): (Option<(String, bool)>, &[ListTitle]),
) -> Option<Ask> {
    let mut shown: Vec<(Chip, String, bool)> = Vec::new();
    // The other choices wait unshown while the lines go into a list.
    if let Some(list) = &choices.shopping {
        let title = shopping_name(t, lists, list);
        shown.push((Chip::Shopping, t.capture_choice_shopping(&title), true));
        return chip_row(ui, t, shown);
    }
    if let Some((name, removable)) = place {
        shown.push((Chip::Place, t.capture_choice_place_desk(&name), removable));
    }
    match choices.importance {
        Importance::Normal => {}
        Importance::Important => {
            shown.push((Chip::Importance, t.importance_important().into(), true));
        }
        Importance::Unimportant => {
            shown.push((Chip::Importance, t.importance_unimportant().into(), true));
        }
    }
    if let Some(tally) = &choices.tally {
        shown.push((Chip::Tally, t.capture_choice_tally(&tally.name), true));
    }
    for (i, choice) in choices.schedules.iter().enumerate() {
        let when = schedule_text(t, &choice.schedule);
        let label = if choice.session.is_some() {
            t.capture_choice_session(&when)
        } else if choice.schedule.once() {
            t.capture_choice_reminder(&when)
        } else {
            t.capture_choice_routine(&when)
        };
        shown.push((Chip::Schedule(i), label, true));
    }
    if choices.frog {
        shown.push((Chip::Frog, t.frog().into(), true));
    }
    chip_row(ui, t, shown)
}

/// The chips `shown`, each with its label and whether a button drops it;
/// what was asked of them.
fn chip_row(ui: &mut Ui, t: &L10n, shown: Vec<(Chip, String, bool)>) -> Option<Ask> {
    if shown.is_empty() {
        return None;
    }
    let text = ui.visuals().text_color();
    let stroke = Stroke::new(1.0, ui.visuals().weak_text_color());
    let mut asked = None;
    ui.horizontal_wrapped(|ui| {
        for (chip, label, removable) in shown {
            Frame::NONE
                .stroke(stroke)
                .corner_radius(8.0)
                .inner_margin(Margin::symmetric(8, 2))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&label).color(text));
                        if removable
                            && icon_button(ui, icons::CLOSE, &t.remove_choice(&label), text)
                                .clicked()
                        {
                            asked = Some(Ask::Remove(chip));
                        }
                    });
                });
        }
    });
    asked
}
