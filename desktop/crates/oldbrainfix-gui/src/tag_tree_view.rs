//! The tag tree on the desktop (#267, Q623, Q639): "All tags…" at the
//! end of the row of the most used tags under All Notes' search
//! (obf-1074) lists every tag of the Notes by its levels, the top level
//! as written (`#life`), a level below by its own part (`health`),
//! indented, each with how many Notes carry it or a tag below it. The
//! levels below a tag unfold by its arrow, the top level alone at first;
//! a click on a tag searches All Notes for it, as a click on a tag in a
//! text does. Typing `#` into a list's search offers the tags starting
//! with what is typed as buttons under the field.

use std::collections::HashSet;

use egui::{RichText, Sense, Ui, WidgetInfo, WidgetType};
use oldbrainfix_core::tag_tree::TagNode;

use crate::icons;
use crate::l10n::L10n;
use crate::theme::{self, Tones};

/// How many tags the search offers at most, as on the phone.
pub const SUGGESTIONS: usize = 12;

/// How many of the most used tags show under All Notes' search at
/// most, as on the phone (obf-1074).
pub const MOST_USED: usize = 12;

/// What was asked of the tag tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagAsked {
    /// Searching All Notes for this tag, as written.
    Search(String),
    /// Unfolding (`true`) or folding the levels below the tag with this
    /// key ([`key`]).
    Fold(String, bool),
}

/// The key a tag's fold is kept by while the tree shows: the tag in
/// lower case, the same however most Notes write it.
pub fn key(node: &TagNode) -> String {
    node.tag.to_lowercase()
}

/// The tag tree's page: its heading, every tag of `tree` by its levels,
/// those below a tag in `unfolded` shown, or a line saying there are
/// none yet.
pub fn page(
    ui: &mut Ui,
    t: &L10n,
    tones: Tones,
    (tree, unfolded): (&[TagNode], &HashSet<String>),
    asked: &mut Vec<TagAsked>,
) {
    let text = ui.visuals().text_color();
    ui.heading(RichText::new(t.tags_title()).color(text));
    ui.add_space(8.0);
    if tree.is_empty() {
        let weak = ui.visuals().weak_text_color();
        ui.label(RichText::new(t.tags_none()).color(weak));
        return;
    }
    egui::ScrollArea::vertical()
        .id_salt("tag tree")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for node in tree {
                rows(ui, t, tones, (node, 0), unfolded, asked);
            }
            ui.add_space(crate::app::LIST_END);
        });
}

/// The row of `node` at `depth`, and its levels below while unfolded.
fn rows(
    ui: &mut Ui,
    t: &L10n,
    tones: Tones,
    (node, depth): (&TagNode, usize),
    unfolded: &HashSet<String>,
    asked: &mut Vec<TagAsked>,
) {
    let open = unfolded.contains(&key(node));
    let name = if depth == 0 { &node.tag } else { &node.name };
    ui.horizontal(|ui| {
        ui.add_space(24.0 * depth as f32);
        // A tag with no levels below keeps the arrow's room, unclickable.
        let sense = if node.children.is_empty() {
            Sense::hover()
        } else {
            Sense::click()
        };
        let (rect, arrow) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), sense);
        if !node.children.is_empty() {
            let said = if open { t.tag_fold() } else { t.tag_unfold() };
            arrow.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, said));
            theme::icon_button_back(ui, &arrow, rect);
            let icon = if open {
                icons::EXPAND_MORE
            } else {
                icons::NEXT
            };
            icons::paint(
                ui.painter(),
                rect.shrink(4.0),
                icon,
                ui.visuals().text_color(),
            );
            if arrow.on_hover_text(said).clicked() {
                asked.push(TagAsked::Fold(key(node), !open));
            }
        }
        let tag =
            egui::Button::new(RichText::new(name).size(15.0).color(tones.accent)).frame(false);
        if ui
            .add(tag)
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .clicked()
        {
            asked.push(TagAsked::Search(node.tag.clone()));
        }
        let weak = ui.visuals().weak_text_color();
        let count = i64::try_from(node.notes).unwrap_or(i64::MAX);
        ui.label(RichText::new(t.tag_notes(count)).size(13.0).color(weak));
    });
    if open {
        for child in &node.children {
            rows(ui, t, tones, (child, depth + 1), unfolded, asked);
        }
    }
}

/// The tags `found` for what is typed into a search, as buttons under its
/// field; the one clicked, to fill in.
pub fn suggestions(ui: &mut Ui, t: &L10n, found: &[String]) -> Option<String> {
    if found.is_empty() {
        return None;
    }
    let text = ui.visuals().text_color();
    let mut picked = None;
    let row = ui.horizontal_wrapped(|ui| {
        for tag in found {
            if ui.button(RichText::new(tag).color(text)).clicked() {
                picked = Some(tag.clone());
            }
        }
    });
    row.response
        .widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, t.tag_suggestions_label()));
    picked
}

/// The most used tags of `tree` (obf-1074), as the phone's
/// `mostUsedTags`: the tags of every level, those more Notes carry first,
/// of as many the one first in the tree, at most `count`, each as
/// written.
pub fn most_used(tree: &[TagNode], count: usize) -> Vec<String> {
    fn walk<'a>(level: &'a [TagNode], all: &mut Vec<&'a TagNode>) {
        for node in level {
            all.push(node);
            walk(&node.children, all);
        }
    }
    let mut all = Vec::new();
    walk(tree, &mut all);
    // Stable: of as many Notes the tree's order stays.
    all.sort_by_key(|node| std::cmp::Reverse(node.notes));
    all.into_iter()
        .take(count)
        .map(|node| node.tag.clone())
        .collect()
}

/// What was asked of the row of the most used tags.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowAsked {
    /// Searching All Notes for this tag, as written.
    Search(String),
    /// Opening the tag tree, by "All tags…".
    All,
}

/// The most used tags `tags` as one row of buttons under All Notes'
/// search field (obf-1074), ending in "All tags…", shown also without a
/// tag. A row wider than the band leaves out the tags at its end, the
/// least used, so "All tags…" always shows. Each tag reads as "Search
/// for" it to a screen reader, as a tag in a text does.
pub fn most_used_row(ui: &mut Ui, t: &L10n, tags: &[String]) -> Option<RowAsked> {
    let text = ui.visuals().text_color();
    let font = egui::TextStyle::Button.resolve(ui.style());
    let pad = ui.spacing().button_padding.x * 2.0;
    let gap = ui.spacing().item_spacing.x;
    let width = |words: &str| {
        let galley = ui.fonts_mut(|f| f.layout_no_wrap(words.to_string(), font.clone(), text));
        galley.size().x + pad
    };
    let all = t.all_tags();
    let mut room = ui.available_width() - width(all);
    let mut fit = 0;
    for tag in tags {
        let wide = width(tag) + gap;
        if wide > room {
            break;
        }
        room -= wide;
        fit += 1;
    }
    let mut asked = None;
    let row = ui.horizontal(|ui| {
        for tag in tags.iter().take(fit) {
            let said = t.search_for_tag(tag);
            let chip = ui.button(RichText::new(tag).color(text));
            chip.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &said));
            if chip.on_hover_text(&said).clicked() {
                asked = Some(RowAsked::Search(tag.clone()));
            }
        }
        if ui.button(RichText::new(all).color(text)).clicked() {
            asked = Some(RowAsked::All);
        }
    });
    row.response
        .widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, t.most_used_tags_label()));
    asked
}

#[cfg(test)]
mod tests;
