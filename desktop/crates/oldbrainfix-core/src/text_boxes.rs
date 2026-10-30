//! Boxes in user text (ADR 0035), as the Dart core's `text_boxes.dart`
//! finds, makes and shows them: a ☐, ✅ or ❌ anywhere in a Note's or a
//! Section's text that carries an id of its own, written right after it
//! in invisible tag characters, so a tick can name it. The glyph in the
//! text is the box's starting state; each tick is an entry of its own
//! beside the text ([`crate::boxes::Boxes`]). Saving a text gives every
//! bare glyph an id ([`convert`]).
//!
//! A box is ☐, ✅ or ❌, one variation selector after it (U+FE0E or
//! U+FE0F) belonging to it, then [`ID_LENGTH`] tag characters, each a
//! digit or a small letter (U+E0030–U+E0039, U+E0061–U+E007A), then a
//! cancel tag (U+E007F). A zero-width joiner after the glyph, or after
//! its selector, makes it part of another emoji, which is no box. An id
//! that shows up a second time in a text makes no second box. Offsets
//! are in UTF-8 bytes here, where the Dart core counts UTF-16 code
//! units.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};

use crate::error::Result;
use crate::markdown::{self, Span, SpanKind};
use crate::text_format;

/// The state a box shows, and the character it is written as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BoxState {
    /// ☐: not done yet.
    Open,
    /// ✅: done.
    Done,
    /// ❌: failed, or not to be done.
    Failed,
}

impl BoxState {
    /// Every state, in the Dart core's order.
    pub const ALL: [BoxState; 3] = [BoxState::Open, BoxState::Done, BoxState::Failed];

    /// The character the box is written as, without a variation
    /// selector.
    pub fn glyph(self) -> char {
        match self {
            BoxState::Open => '\u{2610}',
            BoxState::Done => '\u{2705}',
            BoxState::Failed => '\u{274C}',
        }
    }

    /// Its name, as a tick stores it.
    pub fn name(self) -> &'static str {
        match self {
            BoxState::Open => "open",
            BoxState::Done => "done",
            BoxState::Failed => "failed",
        }
    }

    /// The state named `name` as a tick stores it.
    pub fn named(name: Option<&str>) -> Option<BoxState> {
        match name {
            Some("open") => Some(BoxState::Open),
            Some("done") => Some(BoxState::Done),
            Some("failed") => Some(BoxState::Failed),
            _ => None,
        }
    }

    fn of_glyph(c: char) -> Option<BoxState> {
        BoxState::ALL.into_iter().find(|s| s.glyph() == c)
    }
}

/// A box as the source text holds it: its id, the state its glyph shows
/// (the starting state), and where it lies in bytes: the glyph from
/// `start`, a variation selector after it included, up to `id_start`;
/// the id from there up to `end`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoundBox {
    pub id: String,
    pub state: BoxState,
    pub start: usize,
    pub id_start: usize,
    pub end: usize,
}

/// One tick of a box: the state it set, when (`at`, its entry's date),
/// the Device that wrote it, and whether that is one of the user's own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoxTick {
    pub state: BoxState,
    pub at: DateTime<Utc>,
    pub device: String,
    pub own: bool,
}

/// A box as it shows: its id, the state it shows now and the one its
/// glyph started with (`starting`), its glyph `start..end` in the shown
/// text ([`shown`]), where the whole box lies in the source text,
/// `source_start..source_end` with its id from `id_start`, and its latest
/// tick, `None` while nobody ticked it. Offsets in bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShownBox {
    pub id: String,
    pub state: BoxState,
    pub starting: BoxState,
    pub start: usize,
    pub end: usize,
    pub source_start: usize,
    pub id_start: usize,
    pub source_end: usize,
    pub last: Option<BoxTick>,
}

/// A text as it shows: every box with the glyph of its current state,
/// without its id, and the boxes in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShownText {
    pub text: String,
    pub boxes: Vec<ShownBox>,
}

/// How many characters an id has.
pub const ID_LENGTH: usize = 4;

/// The characters of an id, as a tick names it.
pub const ID_ALPHABET: &str = "0123456789abcdefghijklmnopqrstuvwxyz";

const TEXT_STYLE: char = '\u{FE0E}';
const EMOJI_STYLE: char = '\u{FE0F}';
const JOINER: char = '\u{200D}';
const TAG_BASE: u32 = 0xE0000;
const CANCEL_TAG: char = '\u{E007F}';
/// The bytes of one tag character in UTF-8.
const TAG_BYTES: usize = 4;

fn id_char(c: char) -> bool {
    c.is_ascii_digit() || c.is_ascii_lowercase()
}

/// Whether `id` has the shape of a box id.
pub fn is_id(id: &str) -> bool {
    id.chars().count() == ID_LENGTH && id.chars().all(id_char)
}

/// A glyph [`scan`] found, as it describes.
struct Scanned {
    id: Option<String>,
    state: BoxState,
    start: usize,
    id_start: usize,
    end: usize,
    repeated: bool,
}

/// Every ☐, ✅ and ❌ of `text` that is no part of another emoji: from
/// `start`, to `id_start` after a variation selector, to `end` after its
/// id when it has one, else `end == id_start`. `repeated` when its id
/// came up before in the text.
fn scan(text: &str) -> Vec<Scanned> {
    let mut found = Vec::new();
    let mut seen = BTreeSet::new();
    let mut at = 0;
    while let Some(c) = text[at..].chars().next() {
        let start = at;
        at += c.len_utf8();
        let Some(state) = BoxState::of_glyph(c) else {
            continue;
        };
        let mut id_start = at;
        if let Some(next @ (TEXT_STYLE | EMOJI_STYLE)) = text[id_start..].chars().next() {
            id_start += next.len_utf8();
        }
        if text[id_start..].starts_with(JOINER) {
            // Part of another emoji; what follows is read on its own.
            at = id_start;
            continue;
        }
        let id = id_at(text, id_start);
        let end = if id.is_some() {
            id_start + TAG_BYTES * (ID_LENGTH + 1)
        } else {
            id_start
        };
        let repeated = id.as_ref().is_some_and(|id| !seen.insert(id.clone()));
        found.push(Scanned {
            id,
            state,
            start,
            id_start,
            end,
            repeated,
        });
        at = end;
    }
    found
}

/// The id written at `at` in `text`, or `None` when there is none.
fn id_at(text: &str, at: usize) -> Option<String> {
    let mut chars = text.get(at..)?.chars();
    let mut id = String::new();
    for _ in 0..ID_LENGTH {
        let c = chars.next()?;
        let ascii = char::from_u32((c as u32).checked_sub(TAG_BASE)?)?;
        if !id_char(ascii) {
            return None;
        }
        id.push(ascii);
    }
    (chars.next()? == CANCEL_TAG).then_some(id)
}

/// The id `id` as it is written after a glyph: its tag characters and
/// the cancel tag.
pub fn encode_id(id: &str) -> String {
    id.chars()
        .filter_map(|c| char::from_u32(TAG_BASE + c as u32))
        .chain(std::iter::once(CANCEL_TAG))
        .collect()
}

/// Every box in `text`, in the order they come.
pub fn find(text: &str) -> Vec<FoundBox> {
    scan(text)
        .into_iter()
        .filter(|b| !b.repeated)
        .filter_map(|b| {
            Some(FoundBox {
                id: b.id?,
                state: b.state,
                start: b.start,
                id_start: b.id_start,
                end: b.end,
            })
        })
        .collect()
}

/// `text` as it shows with `ticks`, the latest tick of each box by id:
/// every box with the bare glyph of its current state, the state of its
/// latest tick or else its starting state, and no id. A second box of
/// an id shows its own bare glyph and is no box.
pub fn shown(text: &str, ticks: &BTreeMap<String, BoxTick>) -> ShownText {
    let mut out = String::with_capacity(text.len());
    let mut boxes = Vec::new();
    let mut from = 0;
    for b in scan(text) {
        let Some(id) = b.id else {
            continue;
        };
        out.push_str(&text[from..b.start]);
        from = b.end;
        let last = if b.repeated {
            None
        } else {
            ticks.get(&id).cloned()
        };
        let state = last.as_ref().map_or(b.state, |t| t.state);
        if !b.repeated {
            boxes.push(ShownBox {
                id,
                state,
                starting: b.state,
                start: out.len(),
                end: out.len() + state.glyph().len_utf8(),
                source_start: b.start,
                id_start: b.id_start,
                source_end: b.end,
                last,
            });
        }
        out.push(state.glyph());
    }
    out.push_str(&text[from..]);
    ShownText { text: out, boxes }
}

/// `text` without the ids of its boxes, each with the glyph it was
/// written with: what search and the plain projections read when no
/// ticks are at hand.
pub fn strip(text: &str) -> String {
    if text
        .chars()
        .any(|c| (c as u32) >= TAG_BASE && (c as u32) <= CANCEL_TAG as u32)
    {
        shown(text, &BTreeMap::new()).text
    } else {
        text.to_string()
    }
}

/// `text` with every box written in the bare glyph of its current state
/// by `ticks`, its id kept: what a Sent Item carries, so its receiver
/// starts from the sender's states and the ids stay the same.
pub fn baked(text: &str, ticks: &BTreeMap<String, BoxTick>) -> String {
    let mut out = String::with_capacity(text.len());
    let mut from = 0;
    for b in scan(text) {
        let Some(id) = b.id else {
            continue;
        };
        if b.repeated {
            continue;
        }
        let state = ticks.get(&id).map_or(b.state, |t| t.state);
        out.push_str(&text[from..b.start]);
        out.push(state.glyph());
        from = b.id_start;
    }
    out.push_str(&text[from..]);
    out
}

/// `text` in `format` as it is saved: every bare ☐, ✅ or ❌, one not
/// part of another emoji, becomes a box with a new id, and a box whose
/// id came up before in the text gets a new one. In a formatted text a
/// glyph in inline code or in a link's address stays as it is. Each id
/// is drawn from `new_id`, the store's maker of entity ids, again while
/// it is in the text already.
pub fn convert(
    text: &str,
    format: &str,
    mut new_id: impl FnMut() -> Result<String>,
) -> Result<String> {
    let found = scan(text);
    if found.iter().all(|b| b.id.is_some() && !b.repeated) {
        return Ok(text.to_string());
    }
    let mut used: BTreeSet<String> = found
        .iter()
        .filter(|b| !b.repeated)
        .filter_map(|b| b.id.clone())
        .collect();
    let skipped = if format == text_format::MARKDOWN {
        literal_ranges(text)
    } else {
        Vec::new()
    };
    let mut out = String::with_capacity(text.len());
    let mut from = 0;
    for b in &found {
        if b.id.is_some() && !b.repeated {
            continue;
        }
        if b.id.is_none() && skipped.iter().any(|&(s, e)| b.start >= s && b.start < e) {
            continue;
        }
        let mut id = drawn(&mut new_id)?;
        while used.contains(&id) {
            id = drawn(&mut new_id)?;
        }
        out.push_str(&text[from..b.id_start]);
        out.push_str(&encode_id(&id));
        used.insert(id);
        from = b.end;
    }
    out.push_str(&text[from..]);
    Ok(out)
}

/// A new id from `new_id`: the first bytes of the SHA-256 of a fresh
/// entity id, each taken modulo the alphabet.
fn drawn(new_id: &mut impl FnMut() -> Result<String>) -> Result<String> {
    let digest = Sha256::digest(new_id()?.as_bytes());
    let alphabet = ID_ALPHABET.as_bytes();
    Ok(digest
        .iter()
        .take(ID_LENGTH)
        .map(|b| char::from(alphabet[usize::from(*b) % alphabet.len()]))
        .collect())
}

/// Where a glyph of the formatted text `text` is no box: inline code and
/// links' addresses, as (start, end).
fn literal_ranges(text: &str) -> Vec<(usize, usize)> {
    fn walk(spans: &[Span], ranges: &mut Vec<(usize, usize)>) {
        for s in spans {
            match s.kind {
                SpanKind::Code => ranges.push((s.start, s.end)),
                SpanKind::Link => {
                    ranges.push((s.content_end, s.end));
                    walk(&s.children, ranges);
                }
                _ => walk(&s.children, ranges),
            }
        }
    }
    let mut ranges = Vec::new();
    for block in markdown::parse(text) {
        walk(&block.spans, &mut ranges);
    }
    ranges
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boxed(id: &str, state: BoxState) -> String {
        format!("{}{}", state.glyph(), encode_id(id))
    }

    fn counted() -> impl FnMut() -> Result<String> {
        let mut n = 0;
        move || {
            n += 1;
            Ok(n.to_string())
        }
    }

    #[test]
    fn a_glyph_with_an_id_is_a_box_and_a_bare_one_becomes_one() -> Result<()> {
        let text = format!(
            "a{}b ✅\u{FE0F}{} ✅\u{200D}🔥 {}",
            boxed("a7k2", BoxState::Open),
            encode_id("0000"),
            boxed("a7k2", BoxState::Done)
        );
        let found = find(&text);
        assert_eq!(found.len(), 2);
        assert_eq!(
            (found[0].start, found[0].id_start, found[0].end),
            (1, 4, 24)
        );
        assert_eq!(found[1].id_start - found[1].start, 6);
        let saved = convert(&text, text_format::PLAIN, counted())?;
        assert_eq!(find(&saved).len(), 3);
        assert_eq!(strip(&saved), "a☐b ✅ ✅\u{200D}🔥 ✅");
        assert_eq!(
            convert(&saved, text_format::PLAIN, || Err(crate::Error::Invalid(
                "no id wanted".into()
            )))?,
            saved
        );
        for id in ["A7K2", "a7k", "a7k22", "a-k2", ""] {
            assert!(!is_id(id), "{id}");
        }
        Ok(())
    }

    #[test]
    fn an_id_the_text_holds_already_is_drawn_again() -> Result<()> {
        let mut ids = ["x", "x", "y"].into_iter();
        let mut drawn = 0;
        let saved = convert("☐ ☐", text_format::PLAIN, || {
            drawn += 1;
            Ok(ids.next().unwrap_or("z").to_string())
        })?;
        assert_eq!(drawn, 3);
        assert_eq!(
            find(&saved)
                .into_iter()
                .map(|b| b.id)
                .collect::<BTreeSet<_>>()
                .len(),
            2
        );
        Ok(())
    }

    #[test]
    fn formatted_text_keeps_glyphs_in_code_and_link_addresses() -> Result<()> {
        let text = "- ☐ a `☐` [☐ b](https://x.org/☐)\n\n| ☐ c | d |\n| --- | --- |";
        let saved = convert(text, text_format::MARKDOWN, counted())?;
        assert_eq!(find(&saved).len(), 3);
        assert_eq!(strip(&saved), text);
        assert_eq!(
            find(&convert(text, text_format::PLAIN, counted())?).len(),
            5
        );
        Ok(())
    }

    #[test]
    fn each_box_shows_its_latest_tick_without_its_id() {
        let text = format!(
            "Pack {} {} x",
            boxed("aaaa", BoxState::Open),
            boxed("bbbb", BoxState::Done)
        );
        let tick = BoxTick {
            state: BoxState::Failed,
            at: DateTime::<Utc>::UNIX_EPOCH,
            device: "d".into(),
            own: true,
        };
        let ticks = BTreeMap::from([("aaaa".to_string(), tick.clone())]);
        let shown = shown(&text, &ticks);
        assert_eq!(shown.text, "Pack ❌ ✅ x");
        for b in &shown.boxes {
            assert_eq!(&shown.text[b.start..b.end], b.state.glyph().to_string());
            assert_eq!(&text[b.id_start..b.source_end], encode_id(&b.id));
        }
        assert_eq!(shown.boxes[0].last.as_ref(), Some(&tick));
        assert_eq!(
            baked(&text, &ticks),
            format!(
                "Pack {} {} x",
                boxed("aaaa", BoxState::Failed),
                boxed("bbbb", BoxState::Done)
            )
        );
        assert_eq!(
            text_format::visible(text_format::PLAIN, &text),
            "Pack ☐ ✅ x"
        );
    }
}
