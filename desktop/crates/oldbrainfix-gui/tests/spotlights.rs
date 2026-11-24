//! The tips that explain each screen (obf-1043), driven headless by
//! egui_kittest with the tours switched on, as only the desktop binary
//! does: which tips are due, a Device updated from 0.6.1 counting what it
//! had as shown while 0.7.0's tips stay due, each tip marked before it
//! shows, Next chaining, Skip, Esc and a click beside the card ending the
//! tour and counting the rest as shown, a tip whose element is not on the
//! screen staying due, Show tips again bringing them back, the card's
//! place, and the guide's frames marking every tip; "Show tips" off
//! showing none and marking none, and No more tips switching it off at
//! once (obf-1060).

// A test may unwrap, expect, panic and index, its helpers too (#229).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::sync::Arc;

use chrono::{DateTime, NaiveDateTime, Utc};
use egui::accesskit::Role;
use egui::{Key, Pos2, Rect, Vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::NodeT;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::{DeviceSettings, Places, SigningKey, Store, parse_time};
use oldbrainfix_gui::spotlight::{self, BEFORE_070, MANIFEST};
use oldbrainfix_gui::{App, Page, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn utc(at: DateTime<Utc>) -> NaiveDateTime {
    at.naive_utc()
}

fn store() -> Result<Arc<Store>, Box<dyn std::error::Error>> {
    let now = parse_time("2026-10-26T08:30:00.000Z")?;
    let store = Store::in_memory(Box::new(move || now), || {
        Ok(SigningKey::from_seed([43; 32]))
    })?;
    Ok(Arc::new(store))
}

/// The ids of `screen`'s tips not shown yet.
fn due(store: &Store, screen: &str) -> Result<Vec<&'static str>, Box<dyn std::error::Error>> {
    let seen = spotlight::seen(store, screen)?;
    Ok(spotlight::due(&seen, spotlight::tips(screen))
        .into_iter()
        .map(|tip| tip.id)
        .collect())
}

/// The window over `store` on `selection`, its tours on and every tip
/// due, as on a fresh Device.
fn window(
    store: &Arc<Store>,
    selection: Selection,
) -> Result<Harness<'static, App>, Box<dyn std::error::Error>> {
    let mut app = App::new(store.clone(), "en");
    app.set_wall_clock(Arc::new(utc));
    app.select(selection);
    app.set_spotlights(true)?;
    spotlight::show_again(store)?;
    Ok(shown(app))
}

/// `app` in a window of 1280 by 800, drawn.
fn shown(app: App) -> Harness<'static, App> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            app,
        );
    harness.run();
    harness
}

/// Whether the Settings' switch "Show tips" shows ticked.
fn switch_on(harness: &Harness<'_, App>) -> Result<bool, Box<dyn std::error::Error>> {
    let switch = harness
        .query_all_by_role_and_label(Role::CheckBox, "Show tips")
        .next()
        .ok_or("no Show tips")?;
    Ok(switch.accesskit_node().toggled() == Some(egui::accesskit::Toggled::True))
}

/// Whether a Label says `text` (egui gives a Label its text as the
/// value).
fn says(harness: &Harness<'_, App>, text: &str) -> bool {
    harness.query_all_by_value(text).next().is_some()
}

fn click(harness: &mut Harness<'_, App>, label: &str) -> TestResult {
    harness
        .get_all_by_role_and_label(Role::Button, label)
        .last()
        .ok_or(format!("no {label}"))?
        .click();
    harness.run();
    Ok(())
}

/// A click at `at`, away from every button.
fn click_at(harness: &mut Harness<'_, App>, at: Pos2) {
    harness.hover_at(at);
    harness.step();
    harness.drag_at(at);
    harness.step();
    harness.drop_at(at);
    harness.run();
}

#[test]
fn due_tips_are_those_not_shown_in_the_manifests_order() {
    let tips = spotlight::tips(spotlight::AGENDA);
    let all: Vec<&str> = spotlight::due("", tips).iter().map(|t| t.id).collect();
    assert_eq!(all.first(), Some(&"agenda-add"));
    assert_eq!(all.len(), tips.len());
    let rest: Vec<&str> = spotlight::due("agenda-add, agenda-back", tips)
        .iter()
        .map(|t| t.id)
        .collect();
    assert!(!rest.contains(&"agenda-add") && !rest.contains(&"agenda-back"));
    assert_eq!(rest.first(), Some(&"agenda-places"));
    assert!(spotlight::tips("no such screen").is_empty());
}

#[test]
fn every_tip_id_is_unique_and_every_frozen_one_is_in_its_manifest() {
    for screen in MANIFEST {
        let mut ids: Vec<&str> = screen.tips.iter().map(|t| t.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), screen.tips.len(), "{}", screen.id);
    }
    for (screen, frozen) in BEFORE_070 {
        let tips = spotlight::tips(screen);
        for id in *frozen {
            assert!(tips.iter().any(|t| t.id == *id), "{screen}: {id}");
        }
    }
}

#[test]
fn an_updated_device_counts_what_it_had_as_shown_and_keeps_the_new_tips() -> TestResult {
    let store = store()?;
    assert!(spotlight::seed(&store, false)?);
    assert_eq!(due(&store, spotlight::AGENDA)?, ["agenda-quiet"]);
    assert_eq!(due(&store, spotlight::QUICK_NOTE)?, ["quick-more"]);
    assert_eq!(
        due(&store, spotlight::NOTE_DIALOG)?,
        ["note-code-language", "note-remind", "note-linked-from"]
    );
    assert_eq!(
        due(&store, spotlight::ROUTINE_EDITOR)?,
        ["routine-heads-ups"]
    );
    assert_eq!(
        due(&store, spotlight::SETTINGS)?,
        [
            "settings-show-tips",
            "settings-grammar",
            "settings-tips-switch"
        ]
    );
    // obf-1065's ⋮ of an Area or a Topic came after.
    assert_eq!(due(&store, spotlight::LIST)?, ["list-place-more"]);
    assert!(due(&store, spotlight::UNSORTED)?.is_empty());
    // Once only.
    spotlight::show_again(&store)?;
    assert!(!spotlight::seed(&store, false)?);
    assert_eq!(
        due(&store, spotlight::AGENDA)?.len(),
        spotlight::tips(spotlight::AGENDA).len()
    );
    Ok(())
}

#[test]
fn a_fresh_device_gets_every_tip() -> TestResult {
    let store = store()?;
    assert!(spotlight::seed(&store, true)?);
    assert_eq!(
        store.local_setting(spotlight::STARTED)?.as_deref(),
        Some("1")
    );
    for screen in MANIFEST {
        assert_eq!(due(&store, screen.id)?.len(), screen.tips.len());
    }
    Ok(())
}

#[test]
fn the_guide_frames_mark_every_tip_and_show_tips_again_brings_them_back() -> TestResult {
    let store = store()?;
    spotlight::seed(&store, true)?;
    spotlight::mark_all_seen(&store)?;
    for screen in MANIFEST {
        assert!(due(&store, screen.id)?.is_empty(), "{}", screen.id);
    }
    spotlight::show_again(&store)?;
    for screen in MANIFEST {
        assert_eq!(due(&store, screen.id)?.len(), screen.tips.len());
    }
    assert!(store.local_setting(spotlight::STARTED)?.is_some());
    Ok(())
}

#[test]
fn the_card_stays_inside_the_window_and_off_what_it_explains() {
    let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 800.0));
    let size = Vec2::new(400.0, 120.0);
    let targets = [
        // A "+" at the top right, an arrow at the top left.
        Rect::from_min_size(Pos2::new(1240.0, 40.0), Vec2::splat(24.0)),
        Rect::from_min_size(Pos2::new(0.0, 0.0), Vec2::splat(24.0)),
        // A button near the bottom, past the edge.
        Rect::from_min_size(Pos2::new(600.0, 790.0), Vec2::new(80.0, 30.0)),
        // The sidebar, as high as the window.
        Rect::from_min_size(Pos2::new(0.0, 30.0), Vec2::new(260.0, 770.0)),
        // A panel in the middle.
        Rect::from_center_size(screen.center(), Vec2::new(600.0, 300.0)),
    ];
    for target in targets {
        let card = spotlight::card_rect(target, screen, size);
        assert!(
            screen.shrink(spotlight::MARGIN).contains_rect(card),
            "{target:?} {card:?}"
        );
        assert!(!card.intersects(target), "{target:?} {card:?}");
        assert!(card.width() <= spotlight::CARD_WIDTH);
    }
    // A window too small for anything beside it: inside it all the same.
    let small = Rect::from_min_size(Pos2::ZERO, Vec2::new(300.0, 200.0));
    let card = spotlight::card_rect(small.shrink(10.0), small, size);
    assert!(small.contains_rect(card));
}

#[test]
fn the_window_shows_no_tip_unless_they_are_switched_on() -> TestResult {
    let store = store()?;
    spotlight::show_again(&store)?;
    let mut app = App::new(store.clone(), "en");
    app.set_wall_clock(Arc::new(utc));
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            app,
        );
    harness.run();
    assert_eq!(harness.state().spotlight_showing(), None);
    assert!(store.local_setting(spotlight::STARTED)?.is_none());
    Ok(())
}

#[test]
fn the_agendas_tour_marks_each_tip_before_it_shows_and_next_chains() -> TestResult {
    let store = store()?;
    let mut harness = window(&store, Selection::Agenda)?;
    assert_eq!(harness.state().spotlight_showing(), Some("agenda-add"));
    assert!(says(
        &harness,
        "Writes a thought down at once; you sort it in later."
    ));
    assert!(!due(&store, spotlight::AGENDA)?.contains(&"agenda-add"));
    assert!(due(&store, spotlight::AGENDA)?.contains(&"agenda-places"));

    click(&mut harness, "Next")?;
    assert_eq!(harness.state().spotlight_showing(), Some("agenda-places"));
    assert!(!due(&store, spotlight::AGENDA)?.contains(&"agenda-places"));

    // To the last, whose button says Got it and ends the tour.
    for _ in 0..10 {
        if harness
            .query_all_by_role_and_label(Role::Button, "Got it")
            .next()
            .is_some()
        {
            break;
        }
        click(&mut harness, "Next")?;
    }
    click(&mut harness, "Got it")?;
    assert_eq!(harness.state().spotlight_showing(), None);
    assert!(due(&store, spotlight::AGENDA)?.is_empty());
    Ok(())
}

#[test]
fn skip_ends_the_tour_and_counts_the_rest_as_shown() -> TestResult {
    let store = store()?;
    let mut harness = window(&store, Selection::Agenda)?;
    assert!(harness.state().spotlight_showing().is_some());
    click(&mut harness, "Skip")?;
    assert_eq!(harness.state().spotlight_showing(), None);
    assert!(due(&store, spotlight::AGENDA)?.is_empty());
    // And it does not come back.
    harness.run_steps(4);
    assert_eq!(harness.state().spotlight_showing(), None);
    Ok(())
}

#[test]
fn a_click_beside_the_card_ends_the_tour_and_nothing_beneath_takes_it() -> TestResult {
    let store = store()?;
    let mut harness = window(&store, Selection::Agenda)?;
    assert_eq!(harness.state().spotlight_showing(), Some("agenda-add"));
    click_at(&mut harness, Pos2::new(1270.0, 790.0));
    assert_eq!(harness.state().spotlight_showing(), None);
    assert!(due(&store, spotlight::AGENDA)?.is_empty());
    assert!(!harness.state().quick_note_open());
    Ok(())
}

#[test]
fn esc_ends_the_tour_and_leaves_the_dialog_beneath_open() -> TestResult {
    let store = store()?;
    let mut harness = window(&store, Selection::Agenda)?;
    click(&mut harness, "Skip")?;
    harness.state_mut().new_note();
    harness.run();
    assert_eq!(harness.state().spotlight_showing(), Some("quick-text"));
    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!(harness.state().spotlight_showing(), None);
    assert!(harness.state().quick_note_open());
    Ok(())
}

#[test]
fn a_tip_whose_element_is_missing_stays_due() -> TestResult {
    let store = store()?;
    // An Area without Notes: no first card to point at.
    let house = Places::new(&store).add_area("Household")?;
    let mut harness = window(&store, Selection::Place(house.id))?;
    assert_eq!(harness.state().spotlight_showing(), Some("list-path"));
    click(&mut harness, "Skip")?;
    assert_eq!(due(&store, spotlight::LIST)?, ["list-card"]);
    Ok(())
}

#[test]
fn a_tip_of_what_comes_up_while_typing_shows_once_the_keys_rest() -> TestResult {
    let store = store()?;
    let mut harness = window(&store, Selection::Agenda)?;
    click(&mut harness, "Skip")?;
    harness.state_mut().new_note();
    harness.run();
    click(&mut harness, "Skip")?;
    assert_eq!(
        due(&store, spotlight::QUICK_NOTE)?,
        ["quick-more", "quick-nudges"]
    );
    harness.event(egui::Event::Text("Milk".into()));
    harness.step();
    assert_eq!(harness.state().spotlight_showing(), None);
    harness.run_steps(10);
    assert_eq!(harness.state().spotlight_showing(), Some("quick-more"));
    Ok(())
}

#[test]
fn show_tips_again_in_the_settings_brings_them_back_on_the_next_visit() -> TestResult {
    let store = store()?;
    let mut harness = window(&store, Selection::Agenda)?;
    click(&mut harness, "Skip")?;
    harness.state_mut().open(Page::Settings);
    harness.run();
    assert_eq!(harness.state().spotlight_showing(), Some("settings-search"));
    click(&mut harness, "Skip")?;
    click(&mut harness, "Show tips again")?;
    assert_eq!(
        harness.state().notice(),
        Some("Each screen shows its tips again.")
    );
    // Not on this visit; every screen's on its next.
    harness.run_steps(4);
    assert_eq!(harness.state().spotlight_showing(), None);
    assert_eq!(
        due(&store, spotlight::AGENDA)?.len(),
        spotlight::tips(spotlight::AGENDA).len()
    );
    assert!(store.local_setting(spotlight::STARTED)?.is_some());
    // Back on the Agenda, its tour starts again.
    harness.state_mut().close_page();
    harness.run();
    assert_eq!(harness.state().spotlight_showing(), Some("agenda-add"));
    Ok(())
}

#[test]
fn switched_on_a_first_start_gets_every_tip_and_an_updated_device_the_new_ones() -> TestResult {
    // Nothing written yet: the first start is due.
    let fresh = store()?;
    let mut app = App::new(fresh.clone(), "en");
    app.offer_restore();
    assert!(app.welcoming());
    app.set_spotlights(true)?;
    for screen in MANIFEST {
        assert_eq!(due(&fresh, screen.id)?.len(), screen.tips.len());
    }
    // A Device that ran before.
    let updated = store()?;
    let mut app = App::new(updated.clone(), "en");
    app.set_spotlights(true)?;
    assert_eq!(due(&updated, spotlight::AGENDA)?, ["agenda-quiet"]);
    Ok(())
}

#[test]
fn show_tips_off_shows_no_tip_and_counts_none_as_shown() -> TestResult {
    let store = store()?;
    let mut app = App::new(store.clone(), "en");
    app.set_wall_clock(Arc::new(utc));
    app.select(Selection::Agenda);
    app.set_spotlights(true)?;
    spotlight::show_again(&store)?;
    DeviceSettings::new(&store).set_tips_on(false)?;
    // As the binary reads it at its start.
    app.set_spotlights(true)?;
    let mut harness = shown(app);
    harness.run_steps(4);
    assert_eq!(harness.state().spotlight_showing(), None);
    assert!(store.local_setting(spotlight::STARTED)?.is_some());
    for screen in MANIFEST {
        assert_eq!(due(&store, screen.id)?.len(), screen.tips.len());
    }
    // Nor in the Settings, where the switch shows off.
    harness.state_mut().open(Page::Settings);
    harness.run();
    assert_eq!(harness.state().spotlight_showing(), None);
    assert!(!switch_on(&harness)?);
    // Switched on: not on this visit, on the next screen's.
    harness
        .get_all_by_role_and_label(Role::CheckBox, "Show tips")
        .next()
        .ok_or("no Show tips")?
        .click();
    harness.run();
    assert!(switch_on(&harness)?);
    assert_eq!(store.local_setting("tipsOff")?, None);
    harness.run_steps(4);
    assert_eq!(harness.state().spotlight_showing(), None);
    harness.state_mut().close_page();
    harness.run();
    assert_eq!(harness.state().spotlight_showing(), Some("agenda-add"));
    Ok(())
}

#[test]
fn no_more_tips_switches_them_off_at_once_and_show_tips_again_on() -> TestResult {
    let store = store()?;
    let mut harness = window(&store, Selection::Agenda)?;
    assert_eq!(harness.state().spotlight_showing(), Some("agenda-add"));
    click(&mut harness, "No more tips")?;
    assert_eq!(harness.state().spotlight_showing(), None);
    assert_eq!(store.local_setting("tipsOff")?.as_deref(), Some("true"));
    assert!(!DeviceSettings::new(&store).tips_on()?);
    // The rest stays due for when they are on again.
    assert!(due(&store, spotlight::AGENDA)?.contains(&"agenda-places"));
    harness.state_mut().open(Page::Settings);
    harness.run();
    harness.run_steps(4);
    assert_eq!(harness.state().spotlight_showing(), None);
    assert!(!switch_on(&harness)?);
    click(&mut harness, "Show tips again")?;
    assert!(switch_on(&harness)?);
    assert!(DeviceSettings::new(&store).tips_on()?);
    harness.state_mut().close_page();
    harness.run();
    assert_eq!(harness.state().spotlight_showing(), Some("agenda-add"));
    Ok(())
}

#[test]
fn the_settings_tour_points_at_the_switch_once() -> TestResult {
    let store = store()?;
    let mut harness = window(&store, Selection::Agenda)?;
    click(&mut harness, "Skip")?;
    harness.state_mut().open(Page::Settings);
    harness.run();
    for _ in 0..10 {
        if harness.state().spotlight_showing() == Some("settings-tips-switch") {
            break;
        }
        // The switch lies below the fold: the Settings scroll to it.
        harness
            .get_all_by_role_and_label(Role::Button, "Next")
            .last()
            .ok_or("no Next")?
            .click();
        harness.run_steps(8);
    }
    assert_eq!(
        harness.state().spotlight_showing(),
        Some("settings-tips-switch")
    );
    assert!(says(
        &harness,
        "Switched off, no screen shows a tip; No more tips does the same."
    ));
    assert!(!due(&store, spotlight::SETTINGS)?.contains(&"settings-tips-switch"));
    Ok(())
}

/// Whether the card's No more tips, Skip and `next` sit side by side,
/// in that order, under its sentence and inside its width.
fn the_row_fits(harness: &Harness<'_, App>, next: &str) -> TestResult {
    let t = harness.state().t();
    let tip = harness.state().spotlight_showing().ok_or("no tip")?;
    let text = MANIFEST
        .iter()
        .flat_map(|screen| screen.tips)
        .find(|candidate| candidate.id == tip)
        .map(|candidate| (candidate.text)(t))
        .ok_or("no such tip")?;
    let sentence = harness
        .query_all_by_value(text)
        .next()
        .ok_or("no sentence")?
        .rect();
    let button = |label: &str| -> Result<Rect, Box<dyn std::error::Error>> {
        Ok(harness
            .query_all_by_role_and_label(Role::Button, label)
            .last()
            .ok_or(format!("no {label}"))?
            .rect())
    };
    let off = button(t.spot_no_more_tips())?;
    let skip = button(t.spot_skip())?;
    let next = button(next)?;
    assert!(off.max.x <= skip.min.x && skip.max.x <= next.min.x);
    assert!(off.min.x >= sentence.min.x - 1.0, "{off:?} {sentence:?}");
    assert!(next.max.x <= sentence.min.x + spotlight::CARD_WIDTH);
    assert!(off.min.y > sentence.max.y);
    Ok(())
}

#[test]
fn the_cards_three_buttons_sit_side_by_side_under_its_sentence_in_german() -> TestResult {
    let store = store()?;
    let mut app = App::new(store.clone(), "de");
    app.set_wall_clock(Arc::new(utc));
    app.select(Selection::Agenda);
    app.set_spotlights(true)?;
    spotlight::show_again(&store)?;
    let mut harness = shown(app);
    assert_eq!(harness.state().spotlight_showing(), Some("agenda-add"));
    the_row_fits(&harness, "Weiter")?;
    // The last tip's Verstanden, wider than Weiter.
    for _ in 0..10 {
        if harness
            .query_all_by_role_and_label(Role::Button, "Verstanden")
            .next()
            .is_some()
        {
            break;
        }
        click(&mut harness, "Weiter")?;
    }
    the_row_fits(&harness, "Verstanden")
}
