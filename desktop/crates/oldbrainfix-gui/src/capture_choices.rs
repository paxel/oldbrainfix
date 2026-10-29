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

use std::collections::BTreeMap;

use chrono::Duration;
use egui::{Frame, Margin, RichText, Stroke, Ui};
use oldbrainfix_core::{
    Area, Frogs, Importance, Note, Notes, Routines, Schedule, SessionRule, Sessions, Store,
    Tallies, TallyRule, Topic,
};

use crate::app::icon_button;
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
}

/// What was asked of the capture's buttons or its choices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ask {
    FileInto(String),
    Spawn(Spawn),
    Importance(Importance),
    MakeTally,
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
    /// Whether the Note leaves Unsorted going into `place`: filed, or
    /// with something spawned from it or made of it. It needs no Nudge
    /// then.
    pub fn sorted(&self, place: Option<&str>) -> bool {
        place.is_some() || self.tally.is_some() || !self.schedules.is_empty() || self.frog
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
            Ask::Remove(chip) => match chip {
                Chip::Place => self.place = None,
                Chip::Importance => self.importance = Importance::Normal,
                Chip::Tally => self.tally = None,
                Chip::Schedule(i) if i < self.schedules.len() => drop(self.schedules.remove(i)),
                Chip::Schedule(_) => {}
                Chip::Frog => self.frog = false,
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

/// Captures `text` with `photos` and `recordings` as a new Note in
/// `place`, nudged `nudge_in` from now unless it is sorted, and writes
/// `choices` into it, all in one batch: when a write on the way fails,
/// none of it is written. `None`, and nothing written, when the Note is
/// empty.
pub fn capture(
    store: &Store,
    text: &str,
    place: Option<&str>,
    nudge_in: Duration,
    (photos, recordings): (&[Vec<u8>], &[Vec<u8>]),
    choices: &CaptureChoices,
) -> oldbrainfix_core::Result<Option<Note>> {
    let text = text.trim();
    if text.is_empty() && photos.is_empty() && recordings.is_empty() {
        return Ok(None);
    }
    // A sorted Note needs no Nudge (#113).
    let nudge = (!choices.sorted(place)).then_some(nudge_in);
    store.batch(|| {
        let note = Notes::new(store).capture_with(text, place, nudge, photos, recordings)?;
        choices.write(store, &note.id)?;
        Ok(Some(note))
    })
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

/// The Note card's buttons on a capture: "File into…" over `areas`,
/// Routine, Reminder, Frog and Session while `spawns`, else why not, and
/// "More" with the Importance and "Make it a Tally".
pub fn action_row(
    ui: &mut Ui,
    t: &L10n,
    choices: &CaptureChoices,
    (areas, spawns): (&[(Area, Vec<Topic>)], bool),
) -> Option<Ask> {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let mut asked = None;
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
pub fn chips(
    ui: &mut Ui,
    t: &L10n,
    choices: &CaptureChoices,
    place: Option<(String, bool)>,
) -> Option<Ask> {
    let mut shown: Vec<(Chip, String, bool)> = Vec::new();
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
