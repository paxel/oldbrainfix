//! The format bar of the desktop's Note editor (#127, ADR 0033), with the
//! phone's buttons: ☐ at the start of the line, bold, italic,
//! strike-through, the two headings, bulleted and numbered items, a
//! quote, a link, code, an item one level in and out, and a table (#128),
//! which the window puts in or opens. Each works on the selection of the
//! edit field, or on the word at its cursor, with the core's helpers; a
//! button whose format holds there shows it. The
//! bar sits above the field and never keeps the keyboard: the field has
//! it again after a click, its selection on what was formatted.

use egui::text::{CCursor, CCursorRange};
use egui::widgets::text_edit::TextEditState;
use egui::{Color32, Id, Sense, Ui, WidgetInfo, WidgetType};
use oldbrainfix_core::markdown::{self, BlockKind, Change, Span, SpanKind};
use oldbrainfix_core::text_marks;

use crate::icons;
use crate::l10n::L10n;

/// A button of the format bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Mark,
    Bold,
    Italic,
    Strike,
    Heading1,
    Heading2,
    Bullets,
    Numbers,
    Quote,
    Link,
    Code,
    Indent,
    Outdent,
    Table,
}

impl Format {
    /// The buttons in the phone's order.
    pub const ALL: [Format; 14] = [
        Format::Mark,
        Format::Bold,
        Format::Italic,
        Format::Strike,
        Format::Heading1,
        Format::Heading2,
        Format::Bullets,
        Format::Numbers,
        Format::Quote,
        Format::Link,
        Format::Code,
        Format::Indent,
        Format::Outdent,
        Format::Table,
    ];

    /// What a screen reader and the tooltip say for it.
    pub fn label(self, t: &L10n) -> &'static str {
        match self {
            Format::Mark => t.text_mark_insert(),
            Format::Bold => t.format_bold(),
            Format::Italic => t.format_italic(),
            Format::Strike => t.format_strike(),
            Format::Heading1 => t.format_heading1(),
            Format::Heading2 => t.format_heading2(),
            Format::Bullets => t.format_bullets(),
            Format::Numbers => t.format_numbers(),
            Format::Quote => t.format_quote(),
            Format::Link => t.format_link(),
            Format::Code => t.format_code(),
            Format::Indent => t.format_indent(),
            Format::Outdent => t.format_outdent(),
            Format::Table => t.format_table(),
        }
    }

    /// The phone's icon, or the words it shows instead.
    fn glyph(self) -> Result<char, &'static str> {
        match self {
            Format::Mark => Ok(icons::UNCHECKED),
            Format::Bold => Ok(icons::FORMAT_BOLD),
            Format::Italic => Ok(icons::FORMAT_ITALIC),
            Format::Strike => Ok(icons::FORMAT_STRIKE),
            Format::Heading1 => Err("H1"),
            Format::Heading2 => Err("H2"),
            Format::Bullets => Ok(icons::ALL_ROUTINES),
            Format::Numbers => Ok(icons::FORMAT_NUMBERS),
            Format::Quote => Ok(icons::FORMAT_QUOTE),
            Format::Link => Ok(icons::LINK),
            Format::Code => Ok(icons::CODE),
            Format::Indent => Ok(icons::INDENT),
            Format::Outdent => Ok(icons::OUTDENT),
            Format::Table => Ok(icons::TABLE),
        }
    }

    /// The line format it puts on, for the line buttons.
    fn block(self) -> Option<BlockKind> {
        Some(match self {
            Format::Heading1 => BlockKind::Heading1,
            Format::Heading2 => BlockKind::Heading2,
            Format::Bullets => BlockKind::Bullet,
            Format::Numbers => BlockKind::Numbered,
            Format::Quote => BlockKind::Quote,
            _ => return None,
        })
    }

    /// The format it puts on words, for the word buttons.
    fn span(self) -> Option<SpanKind> {
        Some(match self {
            Format::Bold => SpanKind::Bold,
            Format::Italic => SpanKind::Italic,
            Format::Strike => SpanKind::Strike,
            Format::Code => SpanKind::Code,
            _ => return None,
        })
    }
}

/// The formats that hold at a selection: its first line's, those of the
/// parts holding all of it, and the address of the link it is in.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct At {
    pub line: Option<BlockKind>,
    pub spans: Vec<SpanKind>,
    pub url: Option<String>,
}

impl At {
    /// What holds at the bytes `start..end` of the formatted text `text`;
    /// nothing in a plain one.
    pub fn of(text: &str, (start, end): (usize, usize), formatted: bool) -> At {
        if !formatted {
            return At::default();
        }
        fn inside(spans: &[Span], (start, end): (usize, usize), at: &mut At) {
            for span in spans {
                if span.content_start <= start && end <= span.content_end {
                    at.spans.push(span.kind);
                    if span.kind == SpanKind::Link {
                        at.url = Some(span.url.clone());
                    }
                    inside(&span.children, (start, end), at);
                }
            }
        }
        let blocks = markdown::parse(text);
        let mut at = At::default();
        if let Some(block) = blocks.get(text_marks::line_at(text, start)) {
            at.line = Some(block.kind);
            inside(&block.spans, (start, end), &mut at);
        }
        at
    }

    /// Whether `format` holds here.
    pub fn holds(&self, format: Format) -> bool {
        if format == Format::Link {
            return self.url.is_some();
        }
        if format == Format::Table {
            return self.line == Some(BlockKind::Table);
        }
        if let Some(kind) = format.block() {
            return self.line == Some(kind);
        }
        format.span().is_some_and(|kind| self.spans.contains(&kind))
    }
}

/// What `format` makes of `text` with the bytes `start..end` selected;
/// none for the link, which asks for its address first, for the table,
/// which the table view takes, and for ☐ on a line with a mark.
pub fn change(format: Format, text: &str, (start, end): (usize, usize)) -> Option<Change> {
    if let Some(kind) = format.span() {
        return markdown::toggle(text, start, end, kind).ok();
    }
    if let Some(kind) = format.block() {
        return Some(markdown::toggle_line(text, start, end, kind));
    }
    match format {
        Format::Indent => Some(markdown::indent(text, start, end, false)),
        Format::Outdent => Some(markdown::indent(text, start, end, true)),
        Format::Mark => {
            let inserted =
                text_marks::insert_open(text, text_marks::line_at(text, start.min(end)))?;
            // Where "☐ " went in: the selection after it moves with it.
            let at = text
                .char_indices()
                .zip(inserted.chars())
                .find(|((_, a), b)| a != b)
                .map_or(text.len(), |((i, _), _)| i);
            let moved = inserted.len() - text.len();
            let shift = |b: usize| if b >= at { b + moved } else { b };
            Some(Change {
                text: inserted,
                start: shift(start),
                end: shift(end),
            })
        }
        _ => None,
    }
}

/// The address typed for a link as it is written: one without a scheme
/// goes to an e-mail address when it has an "@" and no "/", and to a web
/// address otherwise. Blank stays blank, which takes a link off.
pub fn address(typed: &str) -> String {
    let typed = typed.trim();
    if typed.is_empty() || typed.contains(':') {
        return typed.to_string();
    }
    if typed.contains('@') && !typed.contains('/') {
        format!("mailto:{typed}")
    } else {
        format!("https://{typed}")
    }
}

/// The selection of the edit field `id` holding `text`, in bytes, its
/// start first; the start of the text in a field never typed into.
pub fn selection(ui: &Ui, id: Id, text: &str) -> (usize, usize) {
    let range = TextEditState::load(ui.ctx(), id).and_then(|s| s.cursor.char_range());
    let byte = |chars: usize| {
        text.char_indices()
            .nth(chars)
            .map_or(text.len(), |(b, _)| b)
    };
    match range {
        Some(range) => {
            let (a, b) = (byte(range.primary.index.0), byte(range.secondary.index.0));
            (a.min(b), a.max(b))
        }
        None => (0, 0),
    }
}

/// Selects the bytes `start..end` of `text` in the edit field `id` and
/// gives it the keyboard.
pub fn select(ui: &Ui, id: Id, text: &str, (start, end): (usize, usize)) {
    let chars = |b: usize| {
        text.get(..b)
            .map_or(text.chars().count(), |s| s.chars().count())
    };
    let mut state = TextEditState::load(ui.ctx(), id).unwrap_or_default();
    state.cursor.set_char_range(Some(CCursorRange::two(
        CCursor::new(chars(start)),
        CCursor::new(chars(end)),
    )));
    state.store(ui.ctx(), id);
    ui.memory_mut(|m| m.request_focus(id));
}

/// Draws the bar, each button showing whether its format holds `at`;
/// gives the button clicked.
pub fn bar(ui: &mut Ui, t: &L10n, at: &At, (colour, accent): (Color32, Color32)) -> Option<Format> {
    let mut picked = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        for format in Format::ALL {
            if button(ui, t, format, at.holds(format), (colour, accent)).clicked() {
                picked = Some(format);
            }
        }
    });
    picked
}

/// One button of the bar: its icon, on the accent while its format holds.
fn button(
    ui: &mut Ui,
    t: &L10n,
    format: Format,
    holds: bool,
    (colour, accent): (Color32, Color32),
) -> egui::Response {
    let label = format.label(t);
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), Sense::click());
    let toggles = format.block().is_some()
        || format.span().is_some()
        || matches!(format, Format::Link | Format::Table);
    resp.widget_info(|| {
        if toggles {
            WidgetInfo::selected(WidgetType::Button, true, holds, label)
        } else {
            WidgetInfo::labeled(WidgetType::Button, true, label)
        }
    });
    if ui.is_rect_visible(rect) {
        let fg = if holds {
            ui.painter().rect_filled(rect, 6.0, accent);
            Color32::BLACK
        } else {
            if resp.hovered() {
                let fill = ui.visuals().widgets.hovered.weak_bg_fill;
                ui.painter().rect_filled(rect, 6.0, fill);
            }
            colour
        };
        match format.glyph() {
            Ok(icon) => icons::paint(ui.painter(), rect.shrink(4.0), icon, fg),
            Err(words) => {
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    words,
                    egui::FontId::proportional(14.0),
                    fg,
                );
            }
        }
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(label)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_address_without_a_scheme_gets_the_one_it_reads_as() {
        assert_eq!(address(" ann@example.org "), "mailto:ann@example.org");
        assert_eq!(address("example.org/a@b"), "https://example.org/a@b");
        assert_eq!(address("example.org"), "https://example.org");
        assert_eq!(address("https://example.org"), "https://example.org");
        assert_eq!(address("  "), "");
    }

    #[test]
    fn the_buttons_work_on_the_word_or_the_line_at_the_cursor() {
        let bold = change(Format::Bold, "buy milk", (5, 5));
        assert_eq!(bold.map(|c| c.text).as_deref(), Some("buy **milk**"));
        let list = change(Format::Bullets, "milk", (0, 0));
        assert_eq!(list.map(|c| c.text).as_deref(), Some("- milk"));
        assert_eq!(change(Format::Link, "milk", (0, 0)), None);
        assert_eq!(change(Format::Table, "milk", (0, 0)), None);
        let marked = change(Format::Mark, "Pack\nPassport", (7, 7));
        assert_eq!(
            marked,
            Some(Change {
                text: "Pack\n☐ Passport".into(),
                start: 11,
                end: 11,
            })
        );
    }

    #[test]
    fn a_button_shows_the_format_at_the_cursor() {
        let text = "## Shop **milk**";
        let at = At::of(text, (11, 11), true);
        assert!(at.holds(Format::Heading2));
        assert!(at.holds(Format::Bold));
        assert!(!at.holds(Format::Italic));
        assert_eq!(At::of(text, (11, 11), false), At::default());
        let table = At::of("| a | b |\n| --- | --- |", (2, 2), true);
        assert!(table.holds(Format::Table));
        assert!(!at.holds(Format::Table));
        let link = At::of("[a](https://x.org)", (1, 1), true);
        assert_eq!(link.url.as_deref(), Some("https://x.org"));
    }
}
