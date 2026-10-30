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
    picked: &mut BTreeSet<String>,
    actions: &mut Vec<WelcomeAction>,
) {
    let text = ui.visuals().text_color();
    ui.heading(RichText::new(t.starter_set_title()).color(text));
    ui.label(RichText::new(t.starter_set_intro()).color(text));
    for (area, topics) in STARTER_SET {
        ui.add_space(8.0);
        let mut on = picked.contains(area.slug);
        let name = offered_area_name(t, Some(area.slug), area.name);
        if ui
            .checkbox(&mut on, RichText::new(name).strong().color(text))
            .changed()
        {
            tick_area(picked, area.slug, on);
        }
        for topic in topics {
            ui.horizontal(|ui| {
                ui.add_space(24.0);
                let mut on = picked.contains(topic.slug);
                let name = offered_area_name(t, Some(topic.slug), topic.name);
                if ui
                    .checkbox(&mut on, RichText::new(name).color(text))
                    .changed()
                {
                    tick_topic(picked, area.slug, topic.slug, on);
                }
            });
        }
    }
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        if ui
            .button(RichText::new(t.starter_set_done()).strong().color(text))
            .clicked()
        {
            actions.push(WelcomeAction::Plant);
        }
        if ui.button(RichText::new(t.cancel()).color(text)).clicked() {
            actions.push(WelcomeAction::Back);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_area_unticked_takes_its_topics_and_a_topic_ticked_brings_its_area() {
        let mut picked = all();
        assert!(picked.contains("household") && picked.contains("kitchen"));
        tick_area(&mut picked, "household", false);
        for slug in ["household", "kitchen", "garden", "cleaning"] {
            assert!(!picked.contains(slug), "{slug}");
        }
        assert!(picked.contains("work"));
        tick_topic(&mut picked, "household", "garden", true);
        assert!(picked.contains("household") && picked.contains("garden"));
        assert!(!picked.contains("kitchen"));
        tick_topic(&mut picked, "household", "garden", false);
        assert!(picked.contains("household") && !picked.contains("garden"));
    }
}
