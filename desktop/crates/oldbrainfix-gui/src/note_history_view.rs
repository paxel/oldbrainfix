//! A Note's History (#193), as the phone's `NoteHistoryScreen`: every
//! change of the Note, its Amends, the Notes merged into it and its
//! Lines, the newest first, each with what it changed, who made it and
//! when, and "Use this version" on an old text of the Note or an Amend
//! this Device may edit, which writes it as a new change.

use chrono::{DateTime, Local, Utc};
use oldbrainfix_core::{NoteChange, NoteChangeKind, NoteHistory, Places, Store, text_format};

use crate::l10n::L10n;
use crate::names::{area_name, topic_name};
use crate::schedule_text;

/// One change as the dialog shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct ChangeRow {
    /// What it changed, in words.
    pub title: String,
    /// Who made it and when, as "Ann, Sat 24 Oct 2026 08:12", the time
    /// alone for a writer not known or a Profile without a name.
    pub said: String,
    /// The value before and after it, in words, where it has them.
    pub before: Option<String>,
    pub after: Option<String>,
    /// Whether "Use this version" brings its text back.
    pub may_use: bool,
    pub change: NoteChange,
}

/// The History of the Note `note_id` as the dialog shows it, newest
/// first.
pub fn rows(store: &Store, t: &L10n, note_id: &str) -> oldbrainfix_core::Result<Vec<ChangeRow>> {
    let history = NoteHistory::new(store);
    let mut out = Vec::new();
    for change in history.of(note_id)? {
        let at = when(t, change.at);
        let name = change.by.as_ref().map(|p| p.name.trim().to_string());
        let said = match name.filter(|n| !n.is_empty()) {
            Some(name) => t.box_tick_by(&name, &at),
            None => at,
        };
        let (title, before, after) = said_of(store, t, &change)?;
        out.push(ChangeRow {
            title,
            said,
            before,
            after,
            may_use: history.may_use(note_id, &change)?,
            change,
        });
    }
    Ok(out)
}

/// `at` as the dialog shows it: the whole day and the time, on this
/// computer's clock.
fn when(t: &L10n, at: DateTime<Utc>) -> String {
    let local = at.with_timezone(&Local);
    let day = oldbrainfix_core::Day {
        year: i64::from(chrono::Datelike::year(&local)),
        month: chrono::Datelike::month(&local),
        day: chrono::Datelike::day(&local),
    };
    format!(
        "{} {}",
        schedule_text::day_text(t, day),
        local.format("%H:%M")
    )
}

type Said = (String, Option<String>, Option<String>);

/// What `change` says: its title and the values before and after it, in
/// words; none where it has none to show.
fn said_of(store: &Store, t: &L10n, change: &NoteChange) -> oldbrainfix_core::Result<Said> {
    use NoteChangeKind::*;
    let (before, after) = (change.before.clone(), change.after.clone());
    let text = |title: &str, before: Option<String>| (title.to_string(), before, after.clone());
    let setting =
        |title: &str, before: String, after: String| (title.to_string(), Some(before), Some(after));
    Ok(match change.kind {
        Captured => text(t.note_history_captured(), None),
        Text => text(t.note_history_text(), before),
        Format => setting(
            t.badge_text_format(),
            format_name(t, before.as_deref()),
            format_name(t, after.as_deref()),
        ),
        Amended => text(t.note_history_amended(), None),
        AmendEdited => text(t.note_history_amend_edited(), before),
        MergedIn => text(t.note_history_merged_in(), None),
        LineAdded => text(t.note_history_line_added(), None),
        LineRenamed => text(t.note_history_line_renamed(), before),
        LineChecked => text(t.note_history_line_checked(), None),
        LineUnchecked => text(t.note_history_line_unchecked(), None),
        LineRemoved => text(t.note_history_line_removed(), None),
        LineRestored => text(t.note_history_line_restored(), None),
        Importance => setting(
            t.importance(),
            importance_name(t, before.as_deref()),
            importance_name(t, after.as_deref()),
        ),
        Filed => setting(
            t.badge_place(),
            place_name(store, t, before.as_deref())?,
            place_name(store, t, after.as_deref())?,
        ),
        Kind => setting(
            t.badge_kind(),
            kind_name(t, before.as_deref()),
            kind_name(t, after.as_deref()),
        ),
        Trashed => (t.note_history_trashed().to_string(), None, None),
        Restored => (t.note_history_restored().to_string(), None, None),
        PhotoAdded => (t.note_history_photo_added().to_string(), None, None),
        PhotoRemoved => (t.note_history_photo_removed().to_string(), None, None),
        RecordingAdded => (t.note_history_recording_added().to_string(), None, None),
        RecordingRemoved => (t.note_history_recording_removed().to_string(), None, None),
    })
}

fn format_name(t: &L10n, format: Option<&str>) -> String {
    match format {
        Some(text_format::MARKDOWN) => t.text_format_formatted(),
        Some(text_format::CODE) => t.text_format_code(),
        _ => t.text_format_plain(),
    }
    .to_string()
}

fn importance_name(t: &L10n, name: Option<&str>) -> String {
    match name {
        Some("important") => t.importance_important(),
        Some("unimportant") => t.importance_unimportant(),
        _ => t.importance_normal(),
    }
    .to_string()
}

fn kind_name(t: &L10n, list: Option<&str>) -> String {
    match list {
        Some("checklist") => t.checklist_heading(),
        Some("shopping") => t.shopping_heading(),
        _ => t.kind_plain(),
    }
    .to_string()
}

/// The Area or Topic `id` as it is named now: Unsorted for none, and no
/// name for one gone.
fn place_name(store: &Store, t: &L10n, id: Option<&str>) -> oldbrainfix_core::Result<String> {
    let Some(id) = id else {
        return Ok(t.unsorted().to_string());
    };
    let places = Places::new(store);
    let topic = places.topic_by_id(id)?;
    let area_id = topic.as_ref().map_or(id, |topic| topic.area_id.as_str());
    let Some(area) = places.area_by_id(area_id)? else {
        return Ok(t.note_history_place_gone().to_string());
    };
    Ok(match topic {
        Some(topic) => format!("{} / {}", area_name(t, &area), topic_name(t, &topic)),
        None => area_name(t, &area),
    })
}
