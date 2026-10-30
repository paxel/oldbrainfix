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

use std::sync::Arc;

use egui::text::{CCursor, CharIndex, Galley};
use egui::{
    Color32, FontSelection, Id, Popup, Rect, RichText, Sense, TextWrapMode, Ui, WidgetInfo,
    WidgetText, WidgetType,
};
use oldbrainfix_core::TextMark;
use oldbrainfix_core::text_marks::find;

use crate::l10n::L10n;

/// A status mark as it is drawn: the line it begins in the whole text,
/// counted from 0, the state it shows and where it lies on the screen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MarkSpot {
    pub line: usize,
    pub mark: TextMark,
    pub rect: Rect,
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
/// while `None`) in `colour`, cut off at the edge with `truncate` and
/// wrapped otherwise, and gives where its marks lie. Without a mark it
/// is the plain label it was.
pub fn marked_label(
    ui: &mut Ui,
    text: &str,
    shown: (usize, usize),
    (size, colour): (Option<f32>, Color32),
    truncate: bool,
) -> Vec<MarkSpot> {
    let (from, to) = shown;
    let Some(part) = text.get(from..to) else {
        return Vec::new();
    };
    let mut rich = RichText::new(part).color(colour);
    if let Some(size) = size {
        rich = rich.size(size);
    }
    let found: Vec<_> = find(text)
        .into_iter()
        .filter(|m| m.start >= from && m.end <= to)
        .collect();
    if found.is_empty() {
        let label = egui::Label::new(rich);
        ui.add(if truncate { label.truncate() } else { label });
        return Vec::new();
    }
    let wrap = if truncate {
        TextWrapMode::Truncate
    } else {
        TextWrapMode::Wrap
    };
    let galley = WidgetText::from(rich).into_galley(
        ui,
        Some(wrap),
        ui.available_width(),
        FontSelection::Default,
    );
    let response = ui.add(egui::Label::new(galley.clone()));
    spots(&galley, response.rect.min, text, shown)
}

/// Where the marks of `text` lie in `galley`, laid out at `origin` from
/// its bytes `shown`.
pub fn spots(
    galley: &Arc<Galley>,
    origin: egui::Pos2,
    text: &str,
    (from, to): (usize, usize),
) -> Vec<MarkSpot> {
    // What a cut-off galley still shows of it, in characters.
    let shown = galley.end().index.0;
    let mut out = Vec::new();
    for found in find(text) {
        if found.start < from || found.end > to {
            continue;
        }
        let (Some(before), Some(glyph)) = (
            text.get(from..found.start),
            text.get(found.start..found.end),
        ) else {
            continue;
        };
        let start = before.chars().count();
        let end = start + glyph.chars().count();
        if end > shown || (galley.elided && end >= shown) {
            continue;
        }
        let at = |index: usize| {
            galley.pos_from_cursor(CCursor {
                index: CharIndex(index),
                prefer_next_row: true,
            })
        };
        let (a, b) = (at(start), at(end));
        let rect = Rect::from_min_max(a.min, egui::pos2(b.max.x.max(a.min.x + 4.0), a.max.y));
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
}
