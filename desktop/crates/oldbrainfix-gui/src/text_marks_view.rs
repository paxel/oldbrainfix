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

use std::sync::Arc;

use egui::text::{CCursor, CharIndex, Galley, LayoutJob, TextFormat};
use egui::{
    Color32, FontId, FontSelection, Id, Popup, Rect, RichText, Sense, TextStyle, TextWrapMode, Ui,
    WidgetInfo, WidgetText, WidgetType,
};
use oldbrainfix_core::TextMark;
use oldbrainfix_core::text_marks::find;
use oldbrainfix_core::text_tags;

use crate::l10n::L10n;

/// A status mark as it is drawn: the line it begins in the whole text,
/// counted from 0, the state it shows and where it lies on the screen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MarkSpot {
    pub line: usize,
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

/// The status marks and the tags of a text as it is drawn.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Spots {
    pub marks: Vec<MarkSpot>,
    pub tags: Vec<TagSpot>,
}

impl Spots {
    /// Adds the spots of another text drawn.
    pub fn extend(&mut self, other: Spots) {
        self.marks.extend(other.marks);
        self.tags.extend(other.tags);
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
    }
}

/// Where the bytes `start..end` of `text`, laid out in `galley` from its
/// bytes `from..`, lie in it; none while a cut-off galley does not show
/// them.
fn place(galley: &Galley, text: &str, from: usize, (start, end): (usize, usize)) -> Option<Rect> {
    // What a cut-off galley still shows of it, in characters.
    let shown = galley.end().index.0;
    let start_at = text.get(from..start)?.chars().count();
    let end_at = start_at + text.get(start..end)?.chars().count();
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
            mark: found.mark,
            rect: rect.translate(origin.to_vec2()),
        });
    }
    out
}

/// Takes the marks `spots` of the text `of`, by id, as widgets over what was
/// drawn under them: read as their state, and while `changes` each a
/// button opening Check, Fail and Open at the mark, the current state
/// left out. Gives the line of the mark picked and the state it is to
/// show.
pub fn mark_clicks(
    ui: &mut Ui,
    t: &L10n,
    of: Id,
    spots: &[MarkSpot],
    changes: bool,
) -> Option<(usize, TextMark)> {
    let mut picked = None;
    let sense = if changes {
        Sense::click()
    } else {
        Sense::hover()
    };
    for spot in spots {
        let resp = ui.interact(spot.rect, of.with(("mark", spot.line)), sense);
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
                    picked = Some((spot.line, to));
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
