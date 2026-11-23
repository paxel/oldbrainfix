//! What a find highlights in the texts drawn (obf-1047): while a
//! [`FindMarks`] is set around what is drawn ([`with`], [`set`]), every
//! place its query is found in a text drawn there shows on a tint, the
//! current one, the find bar's, on a stronger one; the current one comes
//! into view once while it is to be revealed. The places are found as
//! the core's [`findings`] finds them, in the text as it shows (its
//! boxes without their ids), and drawn where they lie in its source.
//!
//! The search's list sets one with the query searched and none current,
//! so a card shows where its Note was found; the Note dialog, a Note's
//! own window and its reading view set the find bar's.

use std::sync::Arc;

use egui::cache::{ComputerMut, FrameCache};
use egui::text::{ByteIndex, LayoutJob, LayoutSection};
use egui::{Color32, Id, Rect, Ui};
use oldbrainfix_core::findings;

use crate::theme;

/// What is highlighted where it is set: the places `query` is found, the
/// `current` one marked more strongly, and whether that one is still to
/// come into view (`reveal`), as the find bar `bar` asks.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FindMarks {
    pub bar: u64,
    pub query: String,
    pub current: Option<Current>,
    pub reveal: bool,
}

/// The current place: in the text whose source hashes to `text`
/// ([`text_key`]), from the byte `start` of that text as it shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Current {
    pub text: u64,
    pub start: usize,
}

/// What [`Current::text`] names a source text by.
pub fn text_key(text: &str) -> u64 {
    Id::new(text).value()
}

/// Where the marks set lie in the context's data.
fn scope_id() -> Id {
    Id::new("find marks")
}

/// Where whether the current place of the find bar `bar` came into view
/// lies.
fn revealed_id(bar: u64) -> Id {
    Id::new(("find revealed", bar))
}

/// Sets `marks` for what is drawn from now on and gives those set
/// before, to be set again after: [`with`] without a closure.
pub fn set(ctx: &egui::Context, marks: Option<Arc<FindMarks>>) -> Option<Arc<FindMarks>> {
    ctx.data_mut(|d| {
        let before = d.get_temp::<Option<Arc<FindMarks>>>(scope_id()).flatten();
        d.insert_temp(scope_id(), marks);
        before
    })
}

/// Draws `add` with `marks` set, those set before set again after.
pub fn with<R>(ui: &mut Ui, marks: Option<Arc<FindMarks>>, add: impl FnOnce(&mut Ui) -> R) -> R {
    let before = set(ui.ctx(), marks);
    let drawn = add(ui);
    set(ui.ctx(), before);
    drawn
}

/// The marks set, if any with a query that finds something.
pub fn marks(ctx: &egui::Context) -> Option<Arc<FindMarks>> {
    ctx.data(|d| d.get_temp::<Option<Arc<FindMarks>>>(scope_id()))
        .flatten()
        .filter(|m| !findings::fold(m.query.trim()).is_empty())
}

/// Whether the current place of the find bar `bar` came into view since
/// [`take_revealed`] was asked last; asking forgets it.
pub fn take_revealed(ctx: &egui::Context, bar: u64) -> bool {
    ctx.data_mut(|d| d.remove_temp::<bool>(revealed_id(bar)))
        .unwrap_or(false)
}

/// Brings `rect`, where the current place is drawn, into view while the
/// marks ask it to, and tells [`take_revealed`] so.
pub(crate) fn reveal(ui: &Ui, marks: &FindMarks, rect: Rect) {
    // The Note dialog's first pass only measures.
    if !marks.reveal || ui.is_sizing_pass() {
        return;
    }
    ui.scroll_to_rect_animation(
        rect.expand(8.0),
        Some(egui::Align::Center),
        egui::style::ScrollAnimation::none(),
    );
    ui.data_mut(|d| d.insert_temp(revealed_id(marks.bar), true));
}

/// The tint behind a place found, the current one's stronger.
pub fn tint(dark: bool, current: bool) -> Color32 {
    theme::found_tint(dark, current)
}

/// A place found in a source text: where it lies in the source, in
/// bytes, and where it begins in the text as it shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hit {
    pub source: (usize, usize),
    pub shown: usize,
}

/// The places found in one source text, top first, and its key.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Hits {
    pub text: u64,
    pub found: Vec<Hit>,
}

impl Hits {
    /// Which of them is the current one of `marks`.
    pub fn current(&self, marks: &FindMarks) -> Option<usize> {
        let current = marks.current?;
        if current.text != self.text {
            return None;
        }
        self.found.iter().position(|h| h.shown == current.start)
    }

    /// Those lying across the source bytes `from..to`, each with whether
    /// it is the current one of `marks`.
    pub fn across(&self, (from, to): (usize, usize), current: Option<usize>) -> Vec<(Hit, bool)> {
        self.found
            .iter()
            .enumerate()
            .filter(|(_, h)| h.source.0 < to && from < h.source.1)
            .map(|(i, h)| (*h, Some(i) == current))
            .collect()
    }
}

/// The places `query` is found in the source `text` in `format`, as its
/// shown text has them ([`shown_of`]).
pub fn hits_in(text: &str, format: &str, query: &str) -> Hits {
    let (shown, cuts) = shown_of(text);
    let found = findings::in_text(&shown, format, query)
        .into_iter()
        .map(|f| Hit {
            source: (source_at(&cuts, f.start), source_at(&cuts, f.end)),
            shown: f.start,
        })
        .collect();
    Hits {
        text: text_key(text),
        found,
    }
}

/// Finds each text's places once while it is drawn frame after frame.
#[derive(Default)]
struct Finder;

impl ComputerMut<(&str, &str, &str), Arc<Hits>> for Finder {
    fn compute(&mut self, (text, format, query): (&str, &str, &str)) -> Arc<Hits> {
        Arc::new(hits_in(text, format, query))
    }
}

type FoundTexts = FrameCache<Arc<Hits>, Finder>;

/// The places the marks set find in the source `text` in `format`, with
/// the marks; none while none are set or nothing is found.
pub fn hits(ctx: &egui::Context, text: &str, format: &str) -> Option<(Arc<Hits>, Arc<FindMarks>)> {
    let marks = marks(ctx)?;
    let hits = ctx.memory_mut(|m| {
        m.caches
            .cache::<FoundTexts>()
            .get((text, format, marks.query.as_str()))
            .clone()
    });
    (!hits.found.is_empty()).then_some((hits, marks))
}

/// Where each id of a box was cut from the source `text` as it shows:
/// the byte of the shown text it was cut at, and how many bytes of the
/// source it took. With them, the shown text: each box with an id as
/// its glyph alone, as [`oldbrainfix_core::text_boxes::shown`] and
/// `strip` show it (every glyph is three bytes, whichever state it
/// shows), a box whose id came before too.
pub fn shown_of(text: &str) -> (String, Vec<(usize, usize)>) {
    const TAG_BASE: u32 = 0xE0000;
    const CANCEL: char = '\u{E007F}';
    const ID_LENGTH: usize = 4;
    let glyphs = ['\u{2610}', '\u{2705}', '\u{274C}'];
    let id_char = |c: char| {
        (c as u32)
            .checked_sub(TAG_BASE)
            .and_then(char::from_u32)
            .is_some_and(|a| a.is_ascii_digit() || a.is_ascii_lowercase())
    };
    // The bytes of an id after a glyph at `at`, its variation selector
    // with it, where one follows.
    let id_after = |at: usize| -> Option<usize> {
        let rest = text.get(at..)?;
        let mut chars = rest.char_indices();
        let mut len = 0;
        let mut first = chars.next()?;
        if matches!(first.1, '\u{FE0E}' | '\u{FE0F}') {
            len = first.1.len_utf8();
            first = chars.next()?;
        }
        if first.1 == '\u{200D}' {
            return None;
        }
        let mut c = first;
        for _ in 0..ID_LENGTH {
            if !id_char(c.1) {
                return None;
            }
            len += c.1.len_utf8();
            c = chars.next()?;
        }
        (c.1 == CANCEL).then_some(len + CANCEL.len_utf8())
    };
    if !text.chars().any(|c| (c as u32) >= TAG_BASE && c <= CANCEL) {
        return (text.to_string(), Vec::new());
    }
    let mut shown = String::with_capacity(text.len());
    let mut cuts = Vec::new();
    let mut from = 0;
    let mut at = 0;
    while let Some(c) = text.get(at..).and_then(|s| s.chars().next()) {
        at += c.len_utf8();
        if !glyphs.contains(&c) {
            continue;
        }
        if let Some(len) = id_after(at) {
            shown.push_str(text.get(from..at).unwrap_or_default());
            cuts.push((shown.len(), len));
            at += len;
            from = at;
        }
    }
    shown.push_str(text.get(from..).unwrap_or_default());
    (shown, cuts)
}

/// The byte of the source a byte `at` of its shown text came from, by
/// the `cuts` of [`shown_of`]: past every id cut before or at it.
pub fn source_at(cuts: &[(usize, usize)], at: usize) -> usize {
    at + cuts
        .iter()
        .take_while(|(cut, _)| *cut <= at)
        .map(|(_, len)| len)
        .sum::<usize>()
}

/// `job`'s sections cut at the bytes `ranges` of its text, each range on
/// the tint of a place found, the current one's (its `bool`) stronger. A
/// part drawn invisibly, such as a box's id in an edit field, stays so.
pub fn mark_job(job: &mut LayoutJob, ranges: &[(usize, usize, bool)], dark: bool) {
    if ranges.is_empty() {
        return;
    }
    let sections = std::mem::take(&mut job.sections);
    let mut out = Vec::with_capacity(sections.len() + ranges.len() * 2);
    for section in sections {
        let (start, end) = (section.byte_range.start.0, section.byte_range.end.0);
        let hidden = section.format.color == Color32::TRANSPARENT;
        let mut at = start;
        let mut piece = |a: usize, b: usize, format: egui::TextFormat| {
            if a < b {
                let lead = if a == start {
                    section.leading_space
                } else {
                    0.0
                };
                out.push(LayoutSection {
                    leading_space: lead,
                    byte_range: ByteIndex(a)..ByteIndex(b),
                    format,
                });
            }
        };
        if !hidden {
            for &(from, to, current) in ranges {
                let (a, b) = (from.max(at), to.min(end));
                if a >= b {
                    continue;
                }
                piece(at, a, section.format.clone());
                piece(a, b, marked(&section.format, dark, current));
                at = b;
            }
        }
        piece(at, end, section.format);
    }
    job.sections = out;
}

/// `format` on the tint of a place found.
pub(crate) fn marked(format: &egui::TextFormat, dark: bool, current: bool) -> egui::TextFormat {
    let mut marked = format.clone();
    marked.background = tint(dark, current);
    marked.expand_bg = 1.0;
    marked
}

/// The places the marks set find in the text `typed` of an edit field in
/// `format`, laid out in `job` byte for byte: on their tints. Gives where
/// the current one lies in `typed`, while it is there.
pub fn mark_typed(
    ctx: &egui::Context,
    job: &mut LayoutJob,
    (typed, format): (&str, &str),
    dark: bool,
) -> Option<(usize, usize)> {
    let (hits, marks) = hits(ctx, typed, format)?;
    let current = hits.current(&marks);
    let ranges: Vec<(usize, usize, bool)> = hits
        .found
        .iter()
        .enumerate()
        .map(|(i, h)| (h.source.0, h.source.1, Some(i) == current))
        .collect();
    mark_job(job, &ranges, dark);
    current
        .and_then(|i| hits.found.get(i))
        .filter(|_| marks.reveal)
        .map(|h| h.source)
}

/// A text of a recording, photo or file, a Transcript, a Photo Text, an
/// Attachment Text or a file's name, as it shows: `said` in `colour`,
/// at `size` (the body's while `None`), the places the marks set find in
/// it on their tints; with whether the current one is in it, to be
/// revealed.
pub fn said_text(
    ui: &Ui,
    said: &str,
    (size, colour): (Option<f32>, Color32),
) -> (egui::WidgetText, bool) {
    let mut rich = egui::RichText::new(said).color(colour);
    if let Some(size) = size {
        rich = rich.size(size);
    }
    let Some((hits, marks)) = hits(ui.ctx(), said, oldbrainfix_core::text_format::PLAIN) else {
        return (rich.into(), false);
    };
    let mut font = egui::TextStyle::Body.resolve(ui.style());
    if let Some(size) = size {
        font.size = size;
    }
    let format = egui::TextFormat::simple(font, colour);
    let mut job = LayoutJob::single_section(said.to_string(), format);
    let current = hits.current(&marks);
    let ranges: Vec<(usize, usize, bool)> = hits
        .found
        .iter()
        .enumerate()
        .map(|(i, h)| (h.source.0, h.source.1, Some(i) == current))
        .collect();
    mark_job(&mut job, &ranges, ui.visuals().dark_mode);
    (job.into(), current.is_some() && marks.reveal)
}

/// Draws a text of a recording, photo or file as [`said_text`] lays it
/// out, selecting to be copied while `selects`, and brings it into view
/// while it holds the current place to be revealed.
pub fn said_label(ui: &mut Ui, said: &str, colour: Color32, selects: bool) -> egui::Response {
    let (shown, current) = said_text(ui, said, (None, colour));
    let label = egui::Label::new(shown);
    let response = if selects {
        crate::read_text::shown(ui, |ui| crate::read_text::label(ui, label))
    } else {
        ui.add(label)
    };
    if current && let Some(marks) = marks(ui.ctx()) {
        reveal(ui, &marks, response.rect);
    }
    response
}

#[cfg(test)]
mod tests;
