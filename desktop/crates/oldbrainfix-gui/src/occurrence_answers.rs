//! Snooze and Not today wherever the desktop answers an Occurrence
//! (#269), as the phone's `occurrence_answers.dart`: the Agenda row's ⋮
//! and right-click, and the Note dialog's Due line, as the notification
//! does. Snooze waits 10 min, 30 min or 1 h; Not today offers Skip, and
//! Tomorrow, which skips it and adds a one-time Occurrence of the same
//! Routine the day after at the same time. Neither changes the schedule
//! (ADR 0006).

use chrono::Duration;
use egui::{RichText, Ui};
use oldbrainfix_core::alarms::open_occurrences;
use oldbrainfix_core::{
    Alarms, Day, DeviceSettings, Occurrence, Result, Routine, Routines, Store, WallClock,
};

use crate::l10n::L10n;
use crate::undo::Undo;

/// What can be answered for an Occurrence now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Answers {
    /// While it is due, started and not yet Missed: a snooze rings from
    /// now on.
    pub snooze: bool,
    /// Where this Device may check it off and it is neither Done nor
    /// skipped.
    pub not_today: bool,
    /// Not today's Tomorrow, only for an Occurrence of today or later.
    pub tomorrow: bool,
}

impl Answers {
    pub fn any(&self) -> bool {
        self.snooze || self.not_today
    }
}

/// One answer picked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Answer {
    /// Snooze for this many minutes.
    Snooze(i64),
    Skip,
    Tomorrow,
}

/// The snoozes offered, in minutes.
pub const SNOOZES: [i64; 3] = [10, 30, 60];

/// What can be answered for `occurrence` of `routine` at the store's
/// now, as the log has it (the phone's `occurrenceAnswers`).
pub fn answers(
    store: &Store,
    routine: &Routine,
    occurrence: &Occurrence,
    wall: &WallClock,
) -> Result<Answers> {
    let open = routine.done_at(occurrence).is_none() && routine.skipped_at(occurrence).is_none();
    if !open {
        return Ok(Answers::default());
    }
    let now = store.now();
    let all_day_from = DeviceSettings::new(store).all_day_from()?;
    let snooze = open_occurrences(std::slice::from_ref(routine), now, wall, all_day_from)
        .iter()
        .any(|o| !o.missed && o.occurrence == *occurrence);
    let not_today = store.may_check_off(&routine.id)?;
    let tomorrow = not_today && occurrence.day >= Day::today(now, wall);
    Ok(Answers {
        snooze,
        not_today,
        tomorrow,
    })
}

/// The label of a snooze of `minutes`.
fn snooze_label(t: &L10n, minutes: i64) -> &'static str {
    match minutes {
        10 => t.snooze10(),
        30 => t.snooze30(),
        _ => t.snooze60(),
    }
}

/// The entries of a menu for `answers`: Snooze with its three choices
/// and Not today with Skip and Tomorrow, each under its heading, as many
/// as `answers` allows. What was picked; a pick closes the menu, also
/// one by keyboard or screen reader.
pub fn entries(ui: &mut Ui, t: &L10n, answers: Answers) -> Option<Answer> {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    let mut picked = None;
    if answers.snooze {
        ui.label(RichText::new(t.snooze()).small().color(weak));
        for minutes in SNOOZES {
            let label = RichText::new(snooze_label(t, minutes)).color(text);
            if ui.button(label).clicked() {
                picked = Some(Answer::Snooze(minutes));
            }
        }
    }
    if answers.snooze && answers.not_today {
        ui.separator();
    }
    if answers.not_today {
        ui.label(RichText::new(t.not_today()).small().color(weak));
        if ui.button(RichText::new(t.skip()).color(text)).clicked() {
            picked = Some(Answer::Skip);
        }
        if answers.tomorrow
            && ui
                .button(RichText::new(t.not_today_tomorrow()).color(text))
                .clicked()
        {
            picked = Some(Answer::Tomorrow);
        }
    }
    if picked.is_some() {
        ui.close();
    }
    picked
}

/// Answers `occurrence` of the Routine `routine` with `answer`, as far
/// as the log still allows it, a menu having been read a moment before:
/// a snooze rings again then; Skip and Tomorrow go at once, and what
/// Ctrl+Z takes back, when they changed anything (#268).
pub fn answer(
    store: &Store,
    wall: &WallClock,
    (routine, occurrence): (&str, &Occurrence),
    answer: Answer,
) -> Result<Option<Undo>> {
    let Some(held) = Routines::new(store).by_id(routine)? else {
        return Ok(None);
    };
    let now = answers(store, &held, occurrence, wall)?;
    let alarms = Alarms::new(store);
    let changed = match answer {
        Answer::Snooze(minutes) => {
            if now.snooze {
                let until = store.now() + Duration::minutes(minutes);
                alarms.apply_snooze(routine, &occurrence.key(), until)?;
            }
            return Ok(None);
        }
        Answer::Skip => {
            now.not_today && alarms.apply_skip(routine, &occurrence.key(), store.now())?
        }
        Answer::Tomorrow => {
            now.tomorrow && alarms.apply_tomorrow(routine, &occurrence.key(), store.now())?
        }
    };
    Ok(changed.then(|| Undo::Skip {
        routine: routine.to_string(),
        occurrence: *occurrence,
    }))
}
