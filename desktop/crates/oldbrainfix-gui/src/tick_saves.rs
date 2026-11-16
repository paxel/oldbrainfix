//! Ticks shown before they are saved (#338, Q752): a click ticks its box,
//! and strikes its Line, in the frame it lands in; the save follows at the
//! start of the next frame, so that frame waits on neither the write nor
//! the reading again after it. The boxes are drawn from what was read;
//! while a tick waits, the place that draws its box asks [`done`] what it
//! shows, and a box in a text shows it in the looks read. The saves run
//! in the order clicked; a failed one shows the box as stored again and
//! says so.

use std::collections::BTreeMap;

use egui::{Context, Id};
use oldbrainfix_core::{BoxState, Occurrence, RoutineOccurrence};

/// What a waiting tick shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shown {
    /// A box that is checked or not: an Occurrence, a Line or a Frog.
    Done(bool),
    /// A box in a text, in one of its states.
    Box(BoxState),
}

#[derive(Clone, Default)]
struct Waiting(BTreeMap<String, Shown>);

fn id() -> Id {
    Id::new("tick saves waiting")
}

/// The key a box is ticked under, the same wherever it is drawn: a
/// Routine's Occurrence, a Line (on an Occurrence), a Frog, or a box in
/// the text of a Note or Section.
pub fn key(parts: [Option<&str>; 6]) -> String {
    parts.map(|p| p.unwrap_or_default()).join("|")
}

/// The key of the Occurrence `occurrence` of `routine`.
pub fn occurrence_key(routine: &str, occurrence: &Occurrence) -> String {
    key([
        Some(routine),
        Some(&occurrence.key()),
        None,
        None,
        None,
        None,
    ])
}

/// The key of the Line `line` checked on the Occurrence `on`, or on none.
pub fn line_key(line: &str, on: Option<&RoutineOccurrence>) -> String {
    let occurrence = on.map(|o| o.occurrence.key());
    key([
        on.map(|o| o.routine_id.as_str()),
        occurrence.as_deref(),
        Some(line),
        None,
        None,
        None,
    ])
}

/// The key of the Frog `frog`.
pub fn frog_key(frog: &str) -> String {
    key([None, None, None, Some(frog), None, None])
}

/// The key of the box `box_id` in the text of `entity`.
pub fn box_key(entity: &str, box_id: &str) -> String {
    key([None, None, None, None, Some(entity), Some(box_id)])
}

/// What the tick `key` shows while its save waits, if one does.
pub fn waiting(ctx: &Context, key: &str) -> Option<Shown> {
    ctx.data(|d| d.get_temp::<Waiting>(id()))
        .and_then(|w| w.0.get(key).copied())
}

/// Whether the box `key` shows checked: as clicked while its save waits,
/// else `stored`.
pub fn done(ctx: &Context, key: &str, stored: bool) -> bool {
    match waiting(ctx, key) {
        Some(Shown::Done(done)) => done,
        _ => stored,
    }
}

/// Shows `shown` on `key` until [`saved`].
pub fn wait(ctx: &Context, key: String, shown: Shown) {
    ctx.data_mut(|d| {
        d.get_temp_mut_or_default::<Waiting>(id())
            .0
            .insert(key, shown);
    });
}

/// The waiting ticks are saved, or failed: their boxes show what is
/// stored again.
pub fn saved(ctx: &Context) {
    ctx.data_mut(|d| d.remove::<Waiting>(id()));
}
