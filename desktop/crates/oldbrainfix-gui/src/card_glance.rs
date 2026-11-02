//! What the compact view of a card says under the Note's name of its
//! Tally or of its Session (#189, #191), as the phone's `card_glance.dart`
//! writes it.

use std::collections::HashMap;

use oldbrainfix_core::{
    Day, Occurrence, Result, Routines, Sessions, Store, Tallies, TallyField, TallyPeriod,
    UnitSystem, WallClock,
};

use crate::history_view;
use crate::l10n::L10n;
use crate::schedule_text::{day_text, occurrence_time};
use crate::tally_view::{self, decimal_sign, shown_value, typed_value};

/// The lines of each Note of `listed` that is a Tally or a Session, by
/// its id: a Tally's fields shown, each with its total today and its
/// target ("Pages 30 / 50 pages"), of the user's own entries in a Shared
/// Area as its detailed view counts unless another choice is picked
/// (#190); else of a Session Note (#191) the next run of any of its
/// Routines not Done and how many runs of all of them were Done and
/// Missed this week, or without a Routine how many it picks.
pub fn read(
    store: &Store,
    t: &L10n,
    wall: &WallClock,
    system: UnitSystem,
    listed: &[String],
) -> Result<HashMap<String, Vec<String>>> {
    let mut glances = HashMap::new();
    if listed.is_empty() {
        return Ok(glances);
    }
    let tallies = Tallies::new(store);
    let routines = Routines::new(store);
    let sessions = Sessions::new(store);
    let decimal = decimal_sign(t);
    let today = Day::today(store.now(), wall);
    for id in listed {
        if let Some(tally) = tallies.by_note(id)? {
            let mut lines = Vec::new();
            for field in tally.shown() {
                let bars = tallies.chart(
                    id,
                    &field.id,
                    TallyPeriod::Day,
                    wall,
                    &tally_view::default_filter(store, id)?,
                )?;
                let total = bars
                    .last()
                    .filter(|b| b.start == today)
                    .and_then(|b| b.value)
                    .unwrap_or(0.0);
                lines.push(field_line(t, field, total, system, decimal));
            }
            glances.insert(id.clone(), lines);
        } else if let Some(setup) = sessions.setup_of(id)? {
            let runs = sessions.of_note(id)?;
            // Without a Routine it runs never; its count says what it is.
            if runs.is_empty() {
                glances.insert(id.clone(), vec![t.session_picks(setup.count)]);
                continue;
            }
            let mut next: Option<Occurrence> = None;
            let (mut done, mut missed) = (0, 0);
            for run in &runs {
                let routine = &run.routine;
                let history = routines.history(routine, wall, history_view::week_start(t))?;
                done += history.this_week.done as i64;
                missed += history.this_week.missed as i64;
                let first = routine
                    .schedule
                    .next(history.today, 64)
                    .into_iter()
                    .find(|o| !routine.done.contains_key(&o.key()));
                if let Some(first) = first
                    && next.is_none_or(|n| first < n)
                {
                    next = Some(first);
                }
            }
            let mut lines = Vec::new();
            if let Some(next) = next {
                let when = format!("{} {}", day_text(t, next.day), occurrence_time(t, &next));
                lines.push(t.session_card_next(&when));
            }
            lines.push(t.session_card_week(done, missed));
            glances.insert(id.clone(), lines);
        }
    }
    Ok(glances)
}

/// `field`'s `total` today against its target.
fn field_line(
    t: &L10n,
    field: &TallyField,
    total: f64,
    system: UnitSystem,
    decimal: &str,
) -> String {
    match field.target {
        Some(target) => t.tally_card_total(
            &field.name,
            &typed_value(Some(total), &field.unit, system, decimal),
            &shown_value(target, &field.unit, system, decimal),
        ),
        None => t.tally_card_total_only(
            &field.name,
            &shown_value(total, &field.unit, system, decimal),
        ),
    }
}
