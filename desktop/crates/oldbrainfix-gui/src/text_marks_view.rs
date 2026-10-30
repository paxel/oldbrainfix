//! Status marks on the desktop's screen (#125, ADR 0031), as the phone's
//! `text_marks_view.dart` draws them: a ☐, ✅ or ❌ at the start of a
//! line of a text is read by a screen reader as the state it shows, and
//! where the text may be changed a click on it opens Check, Fail and
//! Open, the current state left out. A mark in the middle of a line is
//! plain text.
//!
//! The marks are found in the text as it is drawn and taken as widgets
//! of their own afterwards, over whatever lies under them (a card, the
//! editor's field), so that they get the click.
//!
//! A `#tag` in a text (#126, ADR 0032) is drawn in the accent colour,
//! and where the text is read, on a card or in a Section, a click on it
//! opens the search with the tag as written; a screen reader reads it as
//! a link. In the editor's field it is only coloured: a click there
//! places the cursor.
//!
//! A formatted text (#127, ADR 0033) is drawn as it reads, without its
//! marks: bold, italic, struck through, two sizes of heading, bulleted
//! and numbered items, quotes, links and code, line by line as the core
//! reads it. Its status marks and tags are found in its source and take
//! clicks as in a plain text; a link to a web or e-mail address opens in
//! the system's browser or mail program on a click, and any other stays
//! text.
//!
//! A table of a formatted text (#128, ADR 0033) is drawn as a grid,
//! scaled down to the width there is: its header row bold on grey, its
//! first column bold while it holds headings, each cell one line of the
//! formats above with its status mark and tags taking clicks. A click
//! elsewhere on it opens it in the full-screen table view.

use std::sync::Arc;

use egui::text::{CCursor, CharIndex, Galley, LayoutJob, TextFormat};
use egui::{
    Color32, FontId, FontSelection, Id, Popup, Pos2, Rect, RichText, Sense, Stroke, TextStyle,
    TextWrapMode, Ui, Vec2, WidgetInfo, WidgetText, WidgetType,
};
use oldbrainfix_core::markdown::table::{Align, Cell, Table};
use oldbrainfix_core::markdown::{self, Block, BlockKind, Mark, Span, SpanKind};
use oldbrainfix_core::text_marks::{find, line_at};
use oldbrainfix_core::{FoundTag, TextMark, text_format, text_tags};

use crate::l10n::L10n;

/// A status mark as it is drawn: the line it begins in the whole text,
/// counted from 0, the column of the table's cell it begins while it is
/// in one, the state it shows and where it lies on the screen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MarkSpot {
    pub line: usize,
    pub column: Option<usize>,
    pub mark: TextMark,
    pub rect: Rect,
}

/// A tag as it is drawn: where it begins in the whole text, in bytes,
/// the tag as written ("#Garten") and where it lies on the screen.
#[derive(Debug, Clone, PartialEq)]
pub struct TagSpot {
    pub start: usize,
    pub written: String,
    pub rect: Rect,
}

/// A link of a formatted text that may be opened, as it is drawn: where
/// it begins in the whole text, in bytes, where it goes, the words it
/// shows and where they lie on the screen, a rectangle for each row.
#[derive(Debug, Clone, PartialEq)]
pub struct LinkSpot {
    pub start: usize,
    pub url: String,
    pub label: String,
    pub rects: Vec<Rect>,
}

/// A table of a formatted text as it is drawn: where it begins in the
/// whole text, in bytes, and where it lies on the screen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TableSpot {
    pub at: usize,
    pub rect: Rect,
}

/// The status marks, the tags, the links and the tables of a text as it
/// is drawn.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Spots {
    pub marks: Vec<MarkSpot>,
    pub tags: Vec<TagSpot>,
    pub links: Vec<LinkSpot>,
    pub tables: Vec<TableSpot>,
}

impl Spots {
    /// Adds the spots of another text drawn.
    pub fn extend(&mut self, other: Spots) {
        self.marks.extend(other.marks);
        self.tags.extend(other.tags);
        self.links.extend(other.links);
        self.tables.extend(other.tables);
    }
}

/// The bytes `from..to` of `text` laid out in `font` and `colour`, each
/// tag in it in `accent`, unwrapped.
pub fn tag_job(
    text: &str,
    (from, to): (usize, usize),
    font: FontId,
    (colour, accent): (Color32, Color32),
) -> LayoutJob {
    let mut job = LayoutJob::default();
    let plain = TextFormat::simple(font.clone(), colour);
    let tagged = TextFormat::simple(font, accent);
    let mut at = from;
    for tag in text_tags::find(text) {
        if tag.start < from || tag.end > to {
            continue;
        }
        if let (Some(before), Some(written)) =
            (text.get(at..tag.start), text.get(tag.start..tag.end))
        {
            job.append(before, 0.0, plain.clone());
            job.append(written, 0.0, tagged.clone());
            at = tag.end;
        }
    }
    job.append(text.get(at..to).unwrap_or_default(), 0.0, plain);
    job
}

/// What a screen reader says for `mark`: the state it shows.
pub fn mark_label(t: &L10n, mark: TextMark) -> &'static str {
    match mark {
        TextMark::Open => t.text_mark_open(),
        TextMark::Checked => t.text_mark_checked(),
        TextMark::Failed => t.text_mark_failed(),
    }
}

/// The menu entry that changes a mark to `to`, its character first.
pub fn menu_entry(t: &L10n, to: TextMark) -> String {
    let word = match to {
        TextMark::Checked => t.text_mark_check(),
        TextMark::Failed => t.text_mark_fail(),
        TextMark::Open => t.text_mark_reopen(),
    };
    format!("{} {word}", to.glyph())
}

/// Draws the bytes `shown` of `text` as a label of `size` (the body's
/// while `None`) in `colour`, its tags in the accent colour, cut off at
/// the edge with `truncate` and wrapped otherwise, and gives where its
/// marks and tags lie. Without either it is the plain label it was.
pub fn marked_label(
    ui: &mut Ui,
    text: &str,
    shown: (usize, usize),
    (size, colour): (Option<f32>, Color32),
    truncate: bool,
) -> Spots {
    let (from, to) = shown;
    let Some(part) = text.get(from..to) else {
        return Spots::default();
    };
    let marked = find(text).iter().any(|m| m.start >= from && m.end <= to);
    let tagged = text_tags::find(text)
        .iter()
        .any(|t| t.start >= from && t.end <= to);
    if !marked && !tagged {
        let mut rich = RichText::new(part).color(colour);
        if let Some(size) = size {
            rich = rich.size(size);
        }
        let label = egui::Label::new(rich);
        ui.add(if truncate { label.truncate() } else { label });
        return Spots::default();
    }
    let wrap = if truncate {
        TextWrapMode::Truncate
    } else {
        TextWrapMode::Wrap
    };
    let mut font = TextStyle::Body.resolve(ui.style());
    if let Some(size) = size {
        font.size = size;
    }
    let accent = ui.visuals().hyperlink_color;
    let job = tag_job(text, shown, font, (colour, accent));
    let galley = WidgetText::from(job).into_galley(
        ui,
        Some(wrap),
        ui.available_width(),
        FontSelection::Default,
    );
    let response = ui.add(egui::Label::new(galley.clone()));
    let origin = response.rect.min;
    Spots {
        marks: spots(&galley, origin, text, shown),
        tags: tag_spots(&galley, origin, text, shown),
        ..Spots::default()
    }
}

/// Draws the bytes `shown` of `text`, in `format`, as [`marked_label`]
/// draws them: a formatted text as it reads, line by line, a table as a
/// grid (a row's cells one after another while cut off), a plain one as
/// it is written. Gives where its marks, tags, links and tables lie.
pub fn formatted_label(
    ui: &mut Ui,
    (text, format): (&str, &str),
    shown: (usize, usize),
    look: (Option<f32>, Color32),
    truncate: bool,
) -> Spots {
    if format != text_format::MARKDOWN {
        return marked_label(ui, text, shown, look, truncate);
    }
    let (from, to) = shown;
    if text.get(from..to).is_none() {
        return Spots::default();
    }
    let look = Look::of(ui, look);
    let found = Found::of(text);
    let blocks = markdown::parse(text);
    let (first, last) = (line_at(text, from), line_at(text, to));
    let mut spots = Spots::default();
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        for block in blocks.get(first..=last).unwrap_or_default() {
            match &block.table {
                // A table is drawn once, at its first line shown; cut
                // off, each line as its row reads.
                Some(table) if !truncate => {
                    if block.line == table.first_line.max(first) {
                        spots.extend(table_label(ui, text, table, (&found, &look)));
                    }
                }
                _ => spots.extend(block_label(ui, (text, block), &found, &look, truncate)),
            }
        }
    });
    spots
}

/// How a formatted text is drawn: its font, its colour, the accent for
/// links, tags and a quote's bar, the dimmer colour of a quote's words
/// and the background of code.
#[derive(Clone)]
pub(crate) struct Look {
    font: FontId,
    colour: Color32,
    accent: Color32,
    weak: Color32,
    code: Color32,
}

impl Look {
    /// The body's font of `size` (its own while `None`) in `colour`, the
    /// rest as `ui`'s visuals have it.
    pub(crate) fn of(ui: &Ui, (size, colour): (Option<f32>, Color32)) -> Look {
        let mut font = TextStyle::Body.resolve(ui.style());
        if let Some(size) = size {
            font.size = size;
        }
        let visuals = ui.visuals();
        Look {
            font,
            colour,
            accent: visuals.hyperlink_color,
            weak: visuals.weak_text_color(),
            code: visuals.code_bg_color,
        }
    }

    /// The same, `scale` times as large.
    fn scaled(&self, scale: f32) -> Look {
        let mut look = self.clone();
        look.font.size *= scale;
        look
    }
}

/// The status marks and the tags of a formatted text, in its source: a
/// table's cell's marks among them.
pub(crate) struct Found {
    marks: Vec<Mark>,
    tags: Vec<FoundTag>,
}

impl Found {
    pub(crate) fn of(text: &str) -> Found {
        Found {
            marks: markdown::marks(text),
            tags: markdown::tags(text),
        }
    }
}

/// What a part of a line is inside of.
#[derive(Clone, Copy, Default)]
struct Style {
    bold: bool,
    italic: bool,
    strike: bool,
    /// Inside a link that may be opened.
    link: bool,
    quote: bool,
}

/// Where a status mark is: its line, and the column of its table's cell.
type MarkAt = (usize, Option<usize>);

/// A line of a formatted text laid out: what it shows, in `job`; its bold
/// parts once more, in `ghost`, drawn a little to the right over them, as
/// egui's fonts have no bold; and where its marks, tags and links lie in
/// it, in characters.
#[derive(Default)]
struct Line {
    job: LayoutJob,
    ghost: LayoutJob,
    bold: bool,
    chars: usize,
    marks: Vec<(MarkAt, TextMark, (usize, usize))>,
    tags: Vec<(usize, String, (usize, usize))>,
    links: Vec<(usize, String, (usize, usize))>,
}

impl Line {
    /// Adds `shown` in `format`; gives where it lies, in characters.
    fn push(&mut self, shown: &str, format: TextFormat, bold: bool) -> (usize, usize) {
        let from = self.chars;
        if shown.is_empty() {
            return (from, from);
        }
        let mut ghost = format.clone();
        ghost.background = Color32::TRANSPARENT;
        ghost.underline = Stroke::NONE;
        ghost.strikethrough = Stroke::NONE;
        if !bold {
            ghost.color = Color32::TRANSPARENT;
        }
        self.bold |= bold;
        self.job.append(shown, 0.0, format);
        self.ghost.append(shown, 0.0, ghost);
        self.chars += shown.chars().count();
        (from, self.chars)
    }
}

/// The format of a part inside `style`, code in the fixed-width font on
/// its background.
fn part_format(look: &Look, style: Style, code: bool) -> TextFormat {
    let font = if code {
        FontId::monospace(look.font.size * 0.9)
    } else {
        look.font.clone()
    };
    let colour = if style.link {
        look.accent
    } else if style.quote {
        look.weak
    } else {
        look.colour
    };
    let mut format = TextFormat::simple(font, colour);
    format.italics = style.italic;
    if style.strike {
        format.strikethrough = Stroke::new(1.0, colour);
    }
    if style.link {
        format.underline = Stroke::new(1.0, colour);
    }
    if code {
        format.background = look.code;
    }
    format
}

/// Adds the parts `spans` of `text` to `line`.
fn add_spans(line: &mut Line, text: &str, spans: &[Span], style: Style, ctx: (&Found, &Look)) {
    let look = ctx.1;
    for span in spans {
        match span.kind {
            SpanKind::Text if span.content_start == span.start => {
                add_written(line, text, (span.start, span.end), style, ctx);
            }
            SpanKind::Text => {
                line.push(&span.text, part_format(look, style, false), style.bold);
            }
            SpanKind::Code => {
                line.push(&span.text, part_format(look, style, true), style.bold);
            }
            SpanKind::Bold => {
                let inner = Style {
                    bold: true,
                    ..style
                };
                add_spans(line, text, &span.children, inner, ctx);
            }
            SpanKind::Italic => {
                let inner = Style {
                    italic: true,
                    ..style
                };
                add_spans(line, text, &span.children, inner, ctx);
            }
            SpanKind::Strike => {
                let inner = Style {
                    strike: true,
                    ..style
                };
                add_spans(line, text, &span.children, inner, ctx);
            }
            SpanKind::Link => {
                // Only to a web or e-mail address; any other is its words.
                let opens = !style.link && markdown::may_open(&span.url);
                let from = line.chars;
                let inner = Style {
                    link: style.link || opens,
                    ..style
                };
                add_spans(line, text, &span.children, inner, ctx);
                if opens {
                    line.links
                        .push((span.start, span.url.clone(), (from, line.chars)));
                }
            }
        }
    }
}

/// What a run of characters shown as written holds: a status mark of a
/// line, or a tag.
enum Cut {
    Mark(usize, Option<usize>, TextMark),
    Tag,
}

/// Adds the bytes `start..end` of `text`, shown as written, to `line`,
/// its status marks and tags cut out of it; a tag in a link's words is
/// part of the link.
fn add_written(
    line: &mut Line,
    text: &str,
    (start, end): (usize, usize),
    style: Style,
    (found, look): (&Found, &Look),
) {
    let format = part_format(look, style, false);
    let mut cuts: Vec<(usize, usize, Cut)> = found
        .marks
        .iter()
        .filter(|m| m.start >= start && m.end <= end)
        .map(|m| (m.start, m.end, Cut::Mark(m.line, m.column, m.mark)))
        .collect();
    if !style.link {
        cuts.extend(
            found
                .tags
                .iter()
                .filter(|t| t.start >= start && t.end <= end)
                .map(|t| (t.start, t.end, Cut::Tag)),
        );
    }
    cuts.sort_by_key(|c| c.0);
    let mut at = start;
    for (a, b, cut) in cuts {
        let (Some(before), Some(piece)) = (text.get(at..a), text.get(a..b)) else {
            continue;
        };
        line.push(before, format.clone(), style.bold);
        match cut {
            Cut::Mark(number, column, mark) => {
                let chars = line.push(piece, format.clone(), style.bold);
                line.marks.push(((number, column), mark, chars));
            }
            Cut::Tag => {
                let mut tagged = format.clone();
                tagged.color = look.accent;
                let chars = line.push(piece, tagged, style.bold);
                line.tags.push((a, piece.to_string(), chars));
            }
        }
        at = b;
    }
    line.push(text.get(at..end).unwrap_or_default(), format, style.bold);
}

/// What goes before an item's words: its bullet or its number.
fn item_prefix(block: &Block, look: &Look, colour: Color32) -> Option<LayoutJob> {
    let (shown, font) = match block.kind {
        // The proportional font has no "◦"; the fixed-width one has.
        BlockKind::Bullet if block.level > 0 => {
            ("◦".to_string(), FontId::monospace(look.font.size))
        }
        BlockKind::Bullet => ("•".to_string(), look.font.clone()),
        BlockKind::Numbered => (format!("{}.", block.number), look.font.clone()),
        _ => return None,
    };
    Some(LayoutJob::simple_singleline(shown, font, colour))
}

/// Draws the line `block` of `text`, cut off at the edge with `truncate`
/// and wrapped otherwise, and gives where its marks, tags and links lie.
/// A heading cut off keeps the size of the words around it.
fn block_label(
    ui: &mut Ui,
    (text, block): (&str, &Block),
    found: &Found,
    look: &Look,
    truncate: bool,
) -> Spots {
    let mut font = look.font.clone();
    let heading = match block.kind {
        BlockKind::Heading1 => Some(1.5),
        BlockKind::Heading2 => Some(1.25),
        _ => None,
    };
    if let Some(scale) = heading
        && !truncate
    {
        font.size *= scale;
    }
    let look = Look {
        font,
        colour: look.colour,
        accent: look.accent,
        weak: look.weak,
        code: look.code,
    };
    let style = Style {
        bold: heading.is_some(),
        quote: block.kind == BlockKind::Quote,
        ..Style::default()
    };
    let mut line = Line::default();
    match &block.table {
        // A row cut off: its cells that show something, one after
        // another; the line marking the first column as headings as the
        // header row, the delimiter row as nothing.
        Some(table) => {
            let shown = block
                .row
                .or((block.line + 2 == table.delimiter_line).then_some(0))
                .and_then(|row| Some((row, table.rows.get(row)?)));
            if let Some((number, row)) = shown {
                let cells = row.cells.iter().filter(|c| c.end > c.start);
                for (i, cell) in cells.enumerate() {
                    if i > 0 {
                        line.push(" ", part_format(&look, style, false), false);
                    }
                    let style = Style {
                        bold: number == 0,
                        ..style
                    };
                    add_spans(&mut line, text, &cell.spans, style, (found, &look));
                }
            }
        }
        None => add_spans(&mut line, text, &block.spans, style, (found, &look)),
    }
    let quote = block.kind == BlockKind::Quote;
    let indent = match block.kind {
        BlockKind::Bullet | BlockKind::Numbered if block.level > 0 => 20.0,
        _ if quote => 12.0,
        _ => 0.0,
    };
    let wrap = if truncate {
        TextWrapMode::Truncate
    } else {
        TextWrapMode::Wrap
    };
    let row = ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.add_space(indent);
        if let Some(prefix) = item_prefix(block, &look, look.colour) {
            let galley = WidgetText::from(prefix).into_galley(
                ui,
                Some(TextWrapMode::Extend),
                f32::INFINITY,
                FontSelection::Default,
            );
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(galley.size().x.max(14.0) + 6.0, galley.size().y),
                Sense::hover(),
            );
            ui.painter().galley(rect.min, galley, look.colour);
        }
        if line.chars == 0 {
            // A blank line keeps its height.
            let height = ui.fonts_mut(|f| f.row_height(&look.font));
            ui.allocate_exact_size(egui::vec2(1.0, height), Sense::hover());
            return Spots::default();
        }
        let galley = WidgetText::from(std::mem::take(&mut line.job)).into_galley(
            ui,
            Some(wrap),
            ui.available_width(),
            FontSelection::Default,
        );
        let response = ui.add(egui::Label::new(galley.clone()));
        let origin = response.rect.min;
        if line.bold && ui.is_rect_visible(response.rect) {
            let mut ghost = std::mem::take(&mut line.ghost);
            ghost.wrap = galley.job.wrap.clone();
            ghost.halign = galley.job.halign;
            let ghost = ui.fonts_mut(|f| f.layout_job(ghost));
            ui.painter()
                .galley(origin + egui::vec2(0.6, 0.0), ghost, look.colour);
        }
        line_spots(&galley, origin, &line)
    });
    if quote && ui.is_rect_visible(row.response.rect) {
        let rect = row.response.rect;
        let x = rect.left() + 4.0;
        ui.painter()
            .vline(x, rect.y_range(), Stroke::new(3.0, look.accent));
    }
    row.inner
}

/// A cell of a table laid out on one line: what it shows, its bold parts
/// once more for the ghost, and where its marks, tags and links lie in
/// it.
pub(crate) struct LaidCell {
    galley: Arc<Galley>,
    ghost: Option<Arc<Galley>>,
    line: Line,
}

impl LaidCell {
    /// How much room its words take, at least a row of `look`'s font.
    pub(crate) fn size(&self, ui: &Ui, look: &Look) -> Vec2 {
        let height = ui.fonts_mut(|f| f.row_height(&look.font));
        let ghost = if self.ghost.is_some() { 0.6 } else { 0.0 };
        let size = self.galley.size();
        egui::vec2(size.x + ghost, size.y.max(height))
    }

    /// Draws it with its top left at `pos`, as a label to a screen reader
    /// while `label`, and gives where its marks, tags and links lie.
    pub(crate) fn paint(&self, ui: &mut Ui, pos: Pos2, colour: Color32, label: bool) -> Spots {
        let rect = Rect::from_min_size(pos, self.galley.size());
        if label {
            ui.put(rect, egui::Label::new(self.galley.clone()));
        } else if ui.is_rect_visible(rect) {
            ui.painter().galley(pos, self.galley.clone(), colour);
        }
        if let Some(ghost) = &self.ghost
            && ui.is_rect_visible(rect)
        {
            ui.painter()
                .galley(pos + egui::vec2(0.6, 0.0), ghost.clone(), colour);
        }
        line_spots(&self.galley, pos, &self.line)
    }
}

/// Lays out `cell` of `text`, bold while `bold`, on one line.
pub(crate) fn lay_cell(
    ui: &Ui,
    (text, cell): (&str, &Cell),
    (found, look): (&Found, &Look),
    bold: bool,
) -> LaidCell {
    let mut line = Line::default();
    let style = Style {
        bold,
        ..Style::default()
    };
    add_spans(&mut line, text, &cell.spans, style, (found, look));
    let job = std::mem::take(&mut line.job);
    let galley = ui.fonts_mut(|f| f.layout_job(job));
    let ghost = line.bold.then(|| {
        let ghost = std::mem::take(&mut line.ghost);
        ui.fonts_mut(|f| f.layout_job(ghost))
    });
    LaidCell {
        galley,
        ghost,
        line,
    }
}

/// Every cell of `table` laid out, row by row: the header row bold, and
/// the first column too while it holds headings.
pub(crate) fn lay_table(
    ui: &Ui,
    (text, table): (&str, &Table),
    ctx: (&Found, &Look),
) -> Vec<Vec<LaidCell>> {
    table
        .rows
        .iter()
        .enumerate()
        .map(|(r, row)| {
            row.cells
                .iter()
                .enumerate()
                .map(|(c, cell)| {
                    let bold = r == 0 || (table.heading_column && c == 0);
                    lay_cell(ui, (text, cell), ctx, bold)
                })
                .collect()
        })
        .collect()
}

/// How wide each column of `laid` is and how high each row, each with
/// `pad` around its words.
pub(crate) fn grid(
    ui: &Ui,
    laid: &[Vec<LaidCell>],
    look: &Look,
    pad: Vec2,
) -> (Vec<f32>, Vec<f32>) {
    let columns = laid.first().map_or(0, Vec::len);
    let mut widths = vec![0.0f32; columns];
    let mut heights = Vec::with_capacity(laid.len());
    for row in laid {
        let mut height = 0.0f32;
        for (c, cell) in row.iter().enumerate() {
            let size = cell.size(ui, look) + pad * 2.0;
            if let Some(width) = widths.get_mut(c) {
                *width = width.max(size.x);
            }
            height = height.max(size.y);
        }
        heights.push(height);
    }
    (widths, heights)
}

/// Where in a column `width` wide the words of a cell `size` wide begin,
/// from its left edge, as the column lines them up, `pad` in from either
/// edge.
pub(crate) fn aligned(align: Align, width: f32, size: f32, pad: f32) -> f32 {
    match align {
        Align::None | Align::Left => pad,
        Align::Center => (width - size) / 2.0,
        Align::Right => width - size - pad,
    }
}

/// How the lines of a table are drawn and its header row filled, in a
/// text drawn in `colour`.
pub(crate) fn table_tones(colour: Color32) -> (Stroke, Color32) {
    (
        Stroke::new(1.0, colour.gamma_multiply(0.35)),
        colour.gamma_multiply(0.12),
    )
}

/// The room around a cell's words in a table, at full size.
const CELL_PAD: Vec2 = egui::vec2(6.0, 3.0);

/// Draws `table` of `text` as a grid, scaled down to the width there is,
/// and gives where its marks, tags and links lie and where it does.
fn table_label(ui: &mut Ui, text: &str, table: &Table, (found, look): (&Found, &Look)) -> Spots {
    let room = ui.available_width().max(1.0);
    let mut scale = 1.0f32;
    let mut scaled = look.clone();
    let mut laid = lay_table(ui, (text, table), (found, &scaled));
    let (mut widths, mut heights) = grid(ui, &laid, &scaled, CELL_PAD);
    // Smaller till it fits; a font does not shrink quite in proportion.
    for _ in 0..3 {
        let wide: f32 = widths.iter().sum();
        if wide <= room {
            break;
        }
        scale = (scale * room / wide).max(0.2);
        scaled = look.scaled(scale);
        laid = lay_table(ui, (text, table), (found, &scaled));
        (widths, heights) = grid(ui, &laid, &scaled, CELL_PAD * scale);
    }
    let pad = CELL_PAD * scale;
    let size = egui::vec2(widths.iter().sum(), heights.iter().sum());
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    // The cells' labels in a part of their own, which moves nothing after
    // the table.
    let mut cells = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect)
            .id_salt(("table", table.start)),
    );
    let (stroke, header) = table_tones(look.colour);
    let mut spots = Spots::default();
    let mut y = rect.top();
    for (r, row) in laid.iter().enumerate() {
        let height = heights.get(r).copied().unwrap_or_default();
        let mut x = rect.left();
        for (c, cell) in row.iter().enumerate() {
            let width = widths.get(c).copied().unwrap_or_default();
            let at = Rect::from_min_size(egui::pos2(x, y), egui::vec2(width, height));
            if r == 0 && ui.is_rect_visible(at) {
                ui.painter().rect_filled(at, 0.0, header);
            }
            let align = table.aligns.get(c).copied().unwrap_or(Align::None);
            let words = cell.size(ui, &scaled);
            let left = aligned(align, width, words.x, pad.x);
            let pos = egui::pos2(x + left, y + pad.y);
            spots.extend(cell.paint(&mut cells, pos, scaled.colour, true));
            if ui.is_rect_visible(at) {
                ui.painter()
                    .rect_stroke(at, 0.0, stroke, egui::StrokeKind::Inside);
            }
            x += width;
        }
        y += height;
    }
    spots.tables.push(TableSpot {
        at: table.start,
        rect,
    });
    spots
}

/// Takes the tables `spots` of the text `of`, by id, as buttons over
/// what was drawn of them, read as "Open the table". Gives where the
/// table clicked begins in the text. Taken before the marks, tags and
/// links in them, those keep their clicks.
pub fn table_clicks(ui: &mut Ui, t: &L10n, of: Id, spots: &[TableSpot]) -> Option<usize> {
    let mut picked = None;
    for spot in spots {
        let resp = ui.interact(spot.rect, of.with(("table", spot.at)), Sense::click());
        resp.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, t.table_open()));
        if resp
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text(t.table_open())
            .clicked()
        {
            picked = Some(spot.at);
        }
    }
    picked
}

/// Where the marks, tags and links of `line`, laid out in `galley` at
/// `origin`, lie on the screen.
fn line_spots(galley: &Galley, origin: Pos2, line: &Line) -> Spots {
    let at =
        |chars: (usize, usize)| place_chars(galley, chars).map(|r| r.translate(origin.to_vec2()));
    Spots {
        marks: line
            .marks
            .iter()
            .filter_map(|&((line, column), mark, chars)| {
                Some(MarkSpot {
                    line,
                    column,
                    mark,
                    rect: at(chars)?,
                })
            })
            .collect(),
        tags: line
            .tags
            .iter()
            .filter_map(|(start, written, chars)| {
                Some(TagSpot {
                    start: *start,
                    written: written.clone(),
                    rect: at(*chars)?,
                })
            })
            .collect(),
        links: line
            .links
            .iter()
            .filter_map(|(start, url, chars)| {
                let rects: Vec<Rect> = row_rects(galley, *chars)
                    .into_iter()
                    .map(|r| r.translate(origin.to_vec2()))
                    .collect();
                if rects.is_empty() {
                    return None;
                }
                let label = galley
                    .job
                    .text
                    .chars()
                    .skip(chars.0)
                    .take(chars.1 - chars.0)
                    .collect();
                Some(LinkSpot {
                    start: *start,
                    url: url.clone(),
                    label,
                    rects,
                })
            })
            .collect(),
        tables: Vec::new(),
    }
}

/// Where the characters `start..end` of `galley` lie, a rectangle on each
/// row they are on.
fn row_rects(galley: &Galley, (start, end): (usize, usize)) -> Vec<Rect> {
    let mut out = Vec::new();
    let mut at = 0;
    for row in &galley.rows {
        let count = row.char_count_excluding_newline().0;
        let (a, b) = (start.max(at), end.min(at + count));
        if a < b {
            let left = row.pos.x + row.x_offset(CharIndex(a - at));
            let right = row.pos.x + row.x_offset(CharIndex(b - at));
            out.push(Rect::from_min_max(
                egui::pos2(left, row.min_y()),
                egui::pos2(right.max(left + 4.0), row.max_y()),
            ));
        }
        at += row.char_count_including_newline().0;
    }
    out
}

/// Takes the links `spots` of the text `of`, by id, as links over what
/// was drawn under them; a click opens one in the system's browser or
/// mail program. Only web and e-mail addresses are ever among them.
pub fn link_clicks(ui: &mut Ui, of: Id, spots: &[LinkSpot]) {
    for spot in spots {
        for (i, rect) in spot.rects.iter().enumerate() {
            let resp = ui.interact(*rect, of.with(("link", spot.start, i)), Sense::click());
            resp.widget_info(|| WidgetInfo::labeled(WidgetType::Link, true, &spot.label));
            if resp
                .on_hover_cursor(egui::CursorIcon::PointingHand)
                .on_hover_text(&spot.url)
                .clicked()
                && markdown::may_open(&spot.url)
            {
                ui.ctx().open_url(egui::OpenUrl::new_tab(&spot.url));
            }
        }
    }
}

/// The source `text` of a formatted text laid out for its edit field in
/// `font` and `colour`: every character where it is, its marks in `dim`
/// and its tags in `accent`, unwrapped. A table's pipes, its delimiter
/// row and the line marking its first column as headings are marks.
pub fn source_job(
    text: &str,
    font: FontId,
    (colour, accent, dim): (Color32, Color32, Color32),
) -> LayoutJob {
    // What each byte is: 0 words, 1 a mark, 2 a tag.
    let mut kind = vec![0u8; text.len()];
    let mut mark = |a: usize, b: usize, k: u8| {
        for slot in kind.get_mut(a..b).unwrap_or_default() {
            *slot = k;
        }
    };
    fn marks_of(spans: &[Span], out: &mut Vec<(usize, usize)>) {
        for s in spans {
            match s.kind {
                SpanKind::Text => out.push((s.start, s.content_start)),
                SpanKind::Code | SpanKind::Bold | SpanKind::Italic | SpanKind::Strike => {
                    out.push((s.start, s.content_start));
                    out.push((s.content_end, s.end));
                }
                SpanKind::Link => {
                    out.push((s.start, s.content_start));
                    out.push((s.content_end, s.end));
                }
            }
            marks_of(&s.children, out);
        }
    }
    // A line's mark, or a table's line whole; then its cells' words
    // again; then the marks of its words.
    let (mut lines, mut cells, mut inline) = (Vec::new(), Vec::new(), Vec::new());
    for block in markdown::parse(text) {
        match (&block.table, block.row) {
            (Some(table), row) => {
                lines.push((block.start, block.end));
                let row = row.and_then(|r| table.rows.get(r));
                cells.extend(
                    row.iter()
                        .flat_map(|r| r.cells.iter().map(|c| (c.start, c.end))),
                );
            }
            (None, _) => lines.push((block.start, block.content_start)),
        }
        marks_of(&block.spans, &mut inline);
    }
    for (a, b) in lines {
        mark(a, b, 1);
    }
    for (a, b) in cells {
        mark(a, b, 0);
    }
    for (a, b) in inline {
        mark(a, b, 1);
    }
    for tag in markdown::tags(text) {
        mark(tag.start, tag.end, 2);
    }
    let format = |k: u8| {
        let c = match k {
            1 => dim,
            2 => accent,
            _ => colour,
        };
        TextFormat::simple(font.clone(), c)
    };
    let mut job = LayoutJob::default();
    let mut run = 0;
    let mut run_kind = kind.first().copied().unwrap_or(0);
    for (i, _) in text.char_indices() {
        let k = kind.get(i).copied().unwrap_or(0);
        if k != run_kind {
            job.append(text.get(run..i).unwrap_or_default(), 0.0, format(run_kind));
            run = i;
            run_kind = k;
        }
    }
    job.append(text.get(run..).unwrap_or_default(), 0.0, format(run_kind));
    job
}

/// Where the bytes `start..end` of `text`, laid out in `galley` from its
/// bytes `from..`, lie in it; none while a cut-off galley does not show
/// them.
fn place(galley: &Galley, text: &str, from: usize, (start, end): (usize, usize)) -> Option<Rect> {
    let start_at = text.get(from..start)?.chars().count();
    let end_at = start_at + text.get(start..end)?.chars().count();
    place_chars(galley, (start_at, end_at))
}

/// Where the characters `start_at..end_at` of `galley` lie in it, on the
/// row the first is on; none while a cut-off galley does not show them.
fn place_chars(galley: &Galley, (start_at, end_at): (usize, usize)) -> Option<Rect> {
    // What a cut-off galley still shows of it, in characters.
    let shown = galley.end().index.0;
    if end_at > shown || (galley.elided && end_at >= shown) {
        return None;
    }
    let at = |index: usize| {
        galley.pos_from_cursor(CCursor {
            index: CharIndex(index),
            prefer_next_row: true,
        })
    };
    let (a, b) = (at(start_at), at(end_at));
    if b.min.y > a.min.y {
        // Broken over two rows: the part on the first.
        return Some(Rect::from_min_max(
            a.min,
            egui::pos2(galley.rect.max.x.max(a.min.x + 4.0), a.max.y),
        ));
    }
    Some(Rect::from_min_max(
        a.min,
        egui::pos2(b.max.x.max(a.min.x + 4.0), a.max.y),
    ))
}

/// Where the tags of `text` lie in `galley`, laid out at `origin` from
/// its bytes `shown`.
pub fn tag_spots(
    galley: &Arc<Galley>,
    origin: egui::Pos2,
    text: &str,
    (from, to): (usize, usize),
) -> Vec<TagSpot> {
    text_tags::find(text)
        .into_iter()
        .filter(|tag| tag.start >= from && tag.end <= to)
        .filter_map(|tag| {
            let rect = place(galley, text, from, (tag.start, tag.end))?;
            Some(TagSpot {
                start: tag.start,
                written: text.get(tag.start..tag.end)?.to_string(),
                rect: rect.translate(origin.to_vec2()),
            })
        })
        .collect()
}

/// Takes the tags `spots` of the text `of`, by id, as links over what was
/// drawn under them, each read as "Search for" the tag. Gives the tag
/// clicked, as written.
pub fn tag_clicks(ui: &mut Ui, t: &L10n, of: Id, spots: &[TagSpot]) -> Option<String> {
    let mut picked = None;
    for spot in spots {
        let resp = ui.interact(spot.rect, of.with(("tag", spot.start)), Sense::click());
        let label = t.search_for_tag(&spot.written);
        resp.widget_info(|| WidgetInfo::labeled(WidgetType::Link, true, &label));
        if resp.hovered() && ui.is_rect_visible(spot.rect) {
            let stroke = egui::Stroke::new(1.0, ui.visuals().hyperlink_color);
            let y = spot.rect.bottom();
            ui.painter().hline(spot.rect.x_range(), y, stroke);
        }
        if resp
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            picked = Some(spot.written.clone());
        }
    }
    picked
}

/// Where the marks of `text` lie in `galley`, laid out at `origin` from
/// its bytes `shown`.
pub fn spots(
    galley: &Arc<Galley>,
    origin: egui::Pos2,
    text: &str,
    (from, to): (usize, usize),
) -> Vec<MarkSpot> {
    let mut out = Vec::new();
    for found in find(text) {
        if found.start < from || found.end > to {
            continue;
        }
        let Some(rect) = place(galley, text, from, (found.start, found.end)) else {
            continue;
        };
        out.push(MarkSpot {
            line: found.line,
            column: None,
            mark: found.mark,
            rect: rect.translate(origin.to_vec2()),
        });
    }
    out
}

/// Takes the marks `spots` of the text `of`, by id, as widgets over what was
/// drawn under them: read as their state, and while `changes` each a
/// button opening Check, Fail and Open at the mark, the current state
/// left out. Gives the line of the mark picked, the column of its cell in
/// a table, and the state it is to show.
pub fn mark_clicks(
    ui: &mut Ui,
    t: &L10n,
    of: Id,
    spots: &[MarkSpot],
    changes: bool,
) -> Option<(usize, Option<usize>, TextMark)> {
    let mut picked = None;
    let sense = if changes {
        Sense::click()
    } else {
        Sense::hover()
    };
    for spot in spots {
        let resp = ui.interact(spot.rect, of.with(("mark", spot.line, spot.column)), sense);
        let label = mark_label(t, spot.mark);
        let typ = if changes {
            WidgetType::Button
        } else {
            WidgetType::Label
        };
        resp.widget_info(|| WidgetInfo::labeled(typ, true, label));
        if !changes {
            continue;
        }
        if resp.hovered() && ui.is_rect_visible(spot.rect) {
            let stroke = ui.visuals().widgets.hovered.fg_stroke;
            ui.painter().rect_stroke(
                spot.rect.expand(1.0),
                3.0,
                stroke,
                egui::StrokeKind::Outside,
            );
        }
        let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
        // At the mark: where the pointer is, and where a screen reader's
        // click lands.
        Popup::menu(&resp).show(|ui| {
            let text = ui.visuals().text_color();
            for to in TextMark::ALL {
                if to != spot.mark
                    && ui
                        .button(RichText::new(menu_entry(t, to)).color(text))
                        .clicked()
                {
                    picked = Some((spot.line, spot.column, to));
                    ui.close();
                }
            }
        });
    }
    picked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_menu_names_each_state_after_its_character() {
        let en = L10n::new("en");
        assert_eq!(menu_entry(&en, TextMark::Checked), "✅ Check");
        assert_eq!(menu_entry(&en, TextMark::Failed), "❌ Fail");
        assert_eq!(menu_entry(&en, TextMark::Open), "☐ Open");
        assert_eq!(mark_label(&en, TextMark::Checked), "Checked");
        let de = L10n::new("de");
        assert_eq!(menu_entry(&de, TextMark::Open), "☐ Wieder öffnen");
        assert_eq!(mark_label(&de, TextMark::Failed), "Gescheitert");
    }

    #[test]
    fn a_tag_is_read_as_a_search_for_it_as_written() {
        assert_eq!(
            L10n::new("en").search_for_tag("#Garten"),
            "Search for #Garten"
        );
        assert_eq!(
            L10n::new("de").search_for_tag("#Garten"),
            "Nach #Garten suchen"
        );
    }

    #[test]
    fn a_tags_job_colours_the_tag_alone() {
        let text = "Seeds #Garten- und #1 now";
        let font = FontId::proportional(14.0);
        let job = tag_job(text, (0, text.len()), font, (Color32::WHITE, Color32::RED));
        assert_eq!(job.text, text);
        let red: Vec<_> = job
            .sections
            .iter()
            .filter(|s| s.format.color == Color32::RED)
            .filter_map(|s| job.text.get(s.byte_range.start.0..s.byte_range.end.0))
            .collect();
        assert_eq!(red, ["#Garten"]);
        let part = tag_job(
            text,
            (6, 13),
            FontId::proportional(14.0),
            (Color32::WHITE, Color32::RED),
        );
        assert_eq!(part.text, "#Garten");
    }
}
