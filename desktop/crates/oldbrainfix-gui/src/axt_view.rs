//! AXT files in the window (obf-1145, ADR 0056): "Export as AXT file…"
//! beside every "Export as Pack…" writes the Notes of a card, the cards
//! selected with it, the Note dialog's Note, or an Area's or a Topic's
//! Notes as one AXT file for another app, where the user says. What goes
//! into the file is the core's ([`Store::axt_write`]).

use oldbrainfix_core::{Day, Note, Notes, Places, Result, Store};

/// What an AXT file ends in (AXT `spec/envelope.md`, section 9).
pub const EXTENSION: &str = "axt";

/// The longest stem of an AXT file's name, in characters.
const STEM_MAX: usize = 60;

/// The names Windows keeps for its devices, with any ending.
const RESERVED: [&str; 4] = ["CON", "PRN", "AUX", "NUL"];

/// What "Export as AXT file…" writes: Notes picked, or the Notes of an
/// Area, its Topics' with them, or of a Topic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AxtOf {
    Notes(Vec<String>),
    Place(String),
}

/// The name an AXT file of `title` goes by, as the phone's `axtFileName`
/// names it: `Buy milk.axt`, each character a file name cannot hold on
/// any system made `_`, without blanks and full stops at either end, at
/// most 60 characters, `_` before a name Windows keeps for a device;
/// `oldbrainfix-notes-2026-11-25.axt` of `day` when nothing is left.
pub fn file_name(title: Option<&str>, day: Day) -> String {
    let first = title
        .unwrap_or_default()
        .split('\n')
        .next()
        .unwrap_or_default();
    let kept: String = first
        .chars()
        .map(|c| {
            if "\\/:*?\"<>|".contains(c) || c.is_ascii_control() {
                '_'
            } else {
                c
            }
        })
        .take(STEM_MAX)
        .collect();
    let mut stem = kept
        .trim_matches(|c: char| c.is_whitespace() || c == '.')
        .to_string();
    if stem.is_empty() {
        stem = format!("oldbrainfix-notes-{day}");
    }
    let base = stem.split('.').next().unwrap_or_default().to_uppercase();
    let numbered = |prefix: &str| {
        base.strip_prefix(prefix)
            .is_some_and(|n| n.len() == 1 && n.chars().all(|c| ('1'..='9').contains(&c)))
    };
    if RESERVED.contains(&base.as_str()) || numbered("COM") || numbered("LPT") {
        stem.insert(0, '_');
    }
    format!("{stem}.{EXTENSION}")
}

/// The title the Note `note` goes by in an AXT file, its record's: the
/// first line of its text as it reads, trimmed.
pub fn note_title(note: &Note) -> String {
    let text = note.visible_text();
    text.trim()
        .split('\n')
        .next()
        .unwrap_or_default()
        .trim()
        .to_string()
}

/// The Notes of the Area or Topic `place`, as an export of it as a Pack
/// lists them: the place's own first, then each Topic's of an Area, in
/// the Topics' order; nothing in the trash or merged.
pub fn notes_of(store: &Store, place: &str) -> Result<Vec<String>> {
    let places = Places::new(store);
    let mut ids = vec![place.to_string()];
    if places.area_by_id(place)?.is_some() {
        ids.extend(places.topics(place)?.into_iter().map(|t| t.id));
    }
    let all = Notes::new(store).all()?;
    Ok(ids
        .iter()
        .flat_map(|id| {
            all.iter()
                .filter(move |n| n.place.as_deref() == Some(id.as_str()))
        })
        .filter(|n| !n.is_trashed() && !n.is_merged())
        .map(|n| n.id.clone())
        .collect())
}

/// Writes the AXT file of the Notes `ids` to `path`.
pub fn write_file(
    store: &Store,
    ids: &[String],
    path: &std::path::Path,
) -> std::result::Result<(), String> {
    let bytes = store.axt_write(ids).map_err(|e| e.to_string())?;
    std::fs::write(path, bytes).map_err(|e| e.to_string())
}
