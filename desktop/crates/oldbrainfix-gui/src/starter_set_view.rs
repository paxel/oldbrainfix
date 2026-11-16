//! The Starter Set on the first start, after "New to Old Brain Fix", as
//! the phone's `StarterSetScreen`: every Area and Topic of the core's
//! [`STARTER_SET`] ticked, a Topic only kept with its Area, Start planting
//! what is left ticked and Cancel going back to the three choices.

use std::collections::BTreeSet;

use egui::{RichText, Ui};
use oldbrainfix_core::places::STARTER_SET;

use crate::backup_view::WelcomeAction;
use crate::l10n::L10n;
use crate::names::offered_area_name;
use crate::theme;

/// Every slug of the Starter Set: what shows ticked at first.
pub fn all() -> BTreeSet<String> {
    STARTER_SET
        .iter()
        .flat_map(|(area, topics)| std::iter::once(area).chain(topics.iter()))
        .map(|place| place.slug.to_string())
        .collect()
}

/// Ticks the Area `area` on or off: off takes its Topics along.
pub fn tick_area(picked: &mut BTreeSet<String>, area: &str, on: bool) {
    if on {
        picked.insert(area.to_string());
        return;
    }
    picked.remove(area);
    for (starter, topics) in STARTER_SET {
        if starter.slug == area {
            for topic in topics {
                picked.remove(topic.slug);
            }
        }
    }
}

/// Ticks the Topic `topic` of the Area `area` on or off: on ticks its
/// Area too.
pub fn tick_topic(picked: &mut BTreeSet<String>, area: &str, topic: &str, on: bool) {
    if on {
        picked.insert(area.to_string());
        picked.insert(topic.to_string());
    } else {
        picked.remove(topic);
    }
}

/// The view's content: the heading, the intro, each Area with its
/// Topics indented under it, then Start and Cancel.
pub fn starter_set(
    ui: &mut Ui,
    t: &L10n,
    (picked, advanced): (&mut BTreeSet<String>, &mut bool),
    actions: &mut Vec<WelcomeAction>,
) {
    let text = ui.visuals().text_color();
    ui.heading(RichText::new(t.starter_set_title()).color(text));
    ui.label(RichText::new(t.features()).strong().color(text));
    features_choice(ui, t, advanced);
    ui.add_space(8.0);
    ui.label(RichText::new(t.starter_set_intro()).color(text));
    for (area, topics) in STARTER_SET {
        ui.add_space(8.0);
        let mut on = picked.contains(area.slug);
        let name = offered_area_name(t, Some(area.slug), area.name);
        if theme::checkbox(ui, &mut on, RichText::new(name).strong().color(text)).changed() {
            tick_area(picked, area.slug, on);
        }
        for topic in topics {
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                let mut on = picked.contains(topic.slug);
                let name = offered_area_name(t, Some(topic.slug), topic.name);
                if theme::checkbox(ui, &mut on, RichText::new(name).color(text)).changed() {
                    tick_topic(picked, area.slug, topic.slug, on);
                }
            });
        }
    }
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        if theme::primary(ui, true, t.starter_set_done()).clicked() {
            actions.push(WelcomeAction::Plant);
        }
        if ui.button(RichText::new(t.cancel()).color(text)).clicked() {
            actions.push(WelcomeAction::Back);
        }
    });
}

/// Simple or Advanced (#197) as two radio buttons, each with what it
/// means under it, under what the choice keeps: on the Starter Set and in
/// Settings. True when a click picked the other one.
pub fn features_choice(ui: &mut Ui, t: &L10n, advanced: &mut bool) -> bool {
    let text = ui.visuals().text_color();
    let weak = ui.visuals().weak_text_color();
    ui.label(RichText::new(t.features_detail()).small().color(weak));
    let was = *advanced;
    for (value, label, detail) in [
        (false, t.features_simple(), t.features_simple_detail()),
        (true, t.features_advanced(), t.features_advanced_detail()),
    ] {
        ui.radio_value(advanced, value, RichText::new(label).color(text));
        ui.horizontal(|ui| {
            ui.add_space(24.0);
            ui.label(RichText::new(detail).small().color(weak));
        });
    }
    *advanced != was
}

#[cfg(test)]
mod tests;
