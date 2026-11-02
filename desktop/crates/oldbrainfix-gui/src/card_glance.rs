//! What the compact view of a card says under the Note's name of its
//! Tally or of the Session it runs (#189), as the phone's
//! `card_glance.dart` writes it.

use std::collections::HashMap;

use oldbrainfix_core::{
    Day, PersonFilter, Result, Routine, Routines, Sessions, Store, Tallies, TallyField,
    TallyPeriod, UnitSystem, WallClock,
};

use crate::history_view;
use crate::l10n::L10n;
use crate::schedule_text::{day_text, occurrence_time};
use crate::tally_view::{decimal_sign, shown_value, typed_value};

/// The lines of each Note of `listed` that is a Tally or runs a Session,
/// by its id: a Tally's fields shown, each with its total today and its
/// target ("Pages 30 / 50 pages"); else for a Note that runs a Session its
/// next run not Done and how many runs were Done and Missed this week.
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
    // The Sessions not in the trash, by the Note they show under.
    let mut running: HashMap<String, Routine> = HashMap::new();
    for routine in routines.all()? {
        if !sessions.is_session(&routine.id)? {
            continue;
        }
        if let Some(note) = routines.note(&routine)? {
            // #191 makes the Session a kind of its own; until then its
            // first.
            running.entry(note.id).or_insert(routine);
        }
    }
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
                    &PersonFilter::default(),
                )?;
                let total = bars
                    .last()
                    .filter(|b| b.start == today)
                    .and_then(|b| b.value)
                    .unwrap_or(0.0);
                lines.push(field_line(t, field, total, system, decimal));
            }
            glances.insert(id.clone(), lines);
        } else if let Some(routine) = running.get(id) {
            let history = routines.history(routine, wall, history_view::week_start(t))?;
            let next = routine
                .schedule
                .next(history.today, 64)
                .into_iter()
                .find(|o| !routine.done.contains_key(&o.key()));
            let mut lines = Vec::new();
            if let Some(next) = next {
                let when = format!("{} {}", day_text(t, next.day), occurrence_time(t, &next));
                lines.push(t.session_card_next(&when));
            }
            let week = &history.this_week;
            lines.push(t.session_card_week(week.done as i64, week.missed as i64));
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
