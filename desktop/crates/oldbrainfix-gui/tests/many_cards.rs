//! A long list answers a click at once (obf-1069): selecting a card,
//! switching a card's view and opening and closing a Note read none of
//! the cards again, as a change of the list does; only the cards in and
//! near view are laid out, those in view moving only as the list scrolls
//! while the others find their room; a text drawn frame after frame is
//! parsed once.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, Duration, Utc};
use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, MouseWheelUnit, TouchPhase};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::text_format::{MARKDOWN, PLAIN};
use oldbrainfix_core::{Notes, OwnProfile, SigningKey, Store};
use oldbrainfix_gui::{App, Selection, text_marks_view};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A store holding `count` Notes "Note 0", "Note 1", …, captured a
/// second after one another, the newest first in the list, on a clock
/// that stands still after them: the clock reads nothing again.
fn store(count: usize) -> Result<Store, Box<dyn std::error::Error>> {
    store_with(count, |n| format!("Note {n}"), PLAIN)
}

/// As [`store`], each Note's text `text` of its number, in `format`.
fn store_with(
    count: usize,
    text: impl Fn(usize) -> String,
    format: &str,
) -> Result<Store, Box<dyn std::error::Error>> {
    let start = DateTime::parse_from_rfc3339("2026-03-01T09:00:00Z")?.with_timezone(&Utc);
    let at = Arc::new(AtomicI64::new(0));
    let read = at.clone();
    let clock = Box::new(move || start + Duration::seconds(read.load(Ordering::SeqCst)));
    let store = Store::in_memory(clock, || Ok(SigningKey::from_seed([3; 32])))?;
    OwnProfile::new(&store).apply_defaults("Ann", None)?;
    let notes = Notes::new(&store);
    for n in 0..count {
        at.fetch_add(1, Ordering::SeqCst);
        notes.capture_as(&text(n), format, None, None)?;
    }
    Ok(store)
}

fn harness(store: Store) -> Harness<'static, App> {
    let mut app = App::new(store, "en");
    app.select(Selection::All);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
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

/// Lets more than a double-click's time pass.
fn pause(harness: &mut Harness<'_, App>) {
    harness.run_steps(50);
}

fn id_of(harness: &Harness<'_, App>, text: &str) -> Result<String, Box<dyn std::error::Error>> {
    Notes::new(harness.state().store())
        .all()?
        .into_iter()
        .find(|n| n.text == text)
        .map(|n| n.id)
        .ok_or_else(|| format!("no Note {text:?}").into())
}

#[test]
fn a_click_on_a_card_reads_nothing_again() -> TestResult {
    let mut harness = harness(store(20)?);
    let id = id_of(&harness, "Note 18")?;
    let before = harness.state().reads();
    harness
        .get_by_role_and_label(Role::Button, "Note 18")
        .click();
    harness.run();
    pause(&mut harness);
    assert_eq!(harness.state().selected_cards(), vec![id]);
    assert_eq!(harness.state().reads(), before, "nothing read");
    // Esc drops the selection, reading nothing either.
    harness.key_press(Key::Escape);
    harness.run();
    assert!(harness.state().selected_cards().is_empty());
    assert_eq!(harness.state().reads(), before, "nothing read");
    // Nor does Ctrl+F, the cursor going into the search.
    harness.key_press_modifiers(Modifiers::COMMAND, Key::F);
    harness.run();
    assert_eq!(
        harness.get_by(|n| n.is_focused()).accesskit_node().role(),
        Role::TextInput,
        "the search has the keyboard"
    );
    assert_eq!(harness.state().reads(), before, "nothing read");
    Ok(())
}

#[test]
fn opening_and_closing_a_note_reads_it_alone_and_a_new_list_its_cards() -> TestResult {
    let mut harness = harness(store(20)?);
    let id = id_of(&harness, "Note 18")?;
    let (_, cards) = harness.state().reads();
    harness.state_mut().select_note(&id);
    harness.run();
    assert!(harness.state().note_open());
    assert_eq!(harness.state().reads().1, cards, "the cards kept");
    harness.key_press(Key::Escape);
    harness.run();
    assert!(!harness.state().note_open());
    assert_eq!(harness.state().reads().1, cards, "the cards kept");
    // Another list reads its cards.
    harness.state_mut().select(Selection::Unsorted);
    harness.run();
    assert_eq!(harness.state().reads().1, cards + 1);
    // A Note written reads them again.
    harness.state_mut().select(Selection::All);
    harness.run();
    let (_, cards) = harness.state().reads();
    harness.state_mut().edit_note(&id);
    harness.run();
    harness.get_by(|n| n.is_focused()).type_text(" changed");
    harness.run();
    harness.state_mut().close_note();
    harness.run();
    assert_eq!(harness.state().reads().1, cards + 1, "read again");
    let text = Notes::new(harness.state().store())
        .by_id(&id)?
        .ok_or("no Note")?
        .text;
    assert!(text.contains("changed"), "{text:?}");
    assert!(
        harness
            .query_all_by_role_and_label(Role::Button, &text)
            .next()
            .is_some(),
        "its card shows it"
    );
    Ok(())
}

#[test]
fn a_cards_view_switched_reads_no_cards_again() -> TestResult {
    let mut harness = harness(store(20)?);
    let before = harness.state().reads();
    let card = harness
        .get_by_role_and_label(Role::Button, "Note 18")
        .rect();
    harness
        .query_all_by_role_and_label(Role::Button, "Show compact view")
        .find(|n| card.contains_rect(n.rect()))
        .ok_or("no view button")?
        .click();
    harness.run();
    pause(&mut harness);
    let card = harness
        .get_by_role_and_label(Role::Button, "Note 18")
        .rect();
    assert!(
        harness
            .query_all_by_role_and_label(Role::Button, "Show detailed view")
            .any(|n| card.contains_rect(n.rect())),
        "compact now"
    );
    assert!(
        harness
            .state()
            .store()
            .local_setting("switchedNotes:allNotes")?
            .is_some()
    );
    assert_eq!(harness.state().reads(), before, "nothing read");
    Ok(())
}

#[test]
fn only_the_cards_in_and_near_view_are_laid_out() -> TestResult {
    let harness = harness(store(300)?);
    let laid_out = harness.state().cards_laid_out();
    assert!(laid_out > 0, "the cards in view");
    assert!(laid_out < 60, "{laid_out} of 300 laid out");
    // The newest first: the oldest is far out of view.
    assert!(
        harness
            .query_all_by_role_and_label(Role::Button, "Note 0")
            .next()
            .is_none()
    );
    assert!(
        harness
            .query_all_by_role_and_label(Role::Button, "Note 299")
            .next()
            .is_some()
    );
    Ok(())
}

#[test]
fn a_card_stepped_to_out_of_view_is_laid_out() -> TestResult {
    let mut harness = harness(store(300)?);
    let id = id_of(&harness, "Note 179")?;
    harness
        .get_by_role_and_label(Role::Button, "Note 299")
        .click();
    harness.run();
    pause(&mut harness);
    // Down a card at a time, far below the cards laid out at first.
    for _ in 0..120 {
        harness.key_press(Key::ArrowDown);
        harness.run();
    }
    assert_eq!(harness.state().selected_cards(), vec![id]);
    assert!(
        harness
            .query_all_by_role_and_label(Role::Button, "Note 179")
            .next()
            .is_some(),
        "the card stepped to shows"
    );
    Ok(())
}

#[test]
fn a_text_drawn_frame_after_frame_is_parsed_once() -> TestResult {
    let text = |n| format!("# Note {n}\n\nA **bold** line and a #tag\n\n- one\n- two");
    let mut harness = harness(store_with(20, text, MARKDOWN)?);
    let parsed = text_marks_view::parses(&harness.ctx);
    assert!(parsed > 0, "the cards' texts parsed");
    harness.run_steps(5);
    assert_eq!(text_marks_view::parses(&harness.ctx), parsed);
    Ok(())
}

#[test]
fn the_cards_in_view_move_only_as_far_as_the_list_is_scrolled() -> TestResult {
    // Short and long cards in turn, the room of those not drawn yet only
    // guessed.
    let text = |n: usize| match n % 2 {
        0 => format!("Note {n}"),
        _ => format!("Note {n}{}", "\nanother line".repeat(39)),
    };
    let mut harness = harness(store_with(60, text, PLAIN)?);
    // Up from nothing: the last card, the oldest, at the list's end.
    harness.key_press(Key::ArrowUp);
    harness.run_steps(60);
    let at = |harness: &Harness<'_, App>| {
        harness
            .get_by_role_and_label(Role::Button, "Note 0")
            .rect()
            .top()
    };
    let end = harness
        .get_by_role_and_label(Role::Button, "Note 0")
        .rect()
        .bottom();
    assert!(end <= 900.0, "the last card ends at {end}");
    harness.hover_at(egui::pos2(700.0, 450.0));
    harness.run();
    for _ in 0..8 {
        let (before, scrolled) = (at(&harness), harness.state().list_scrolled());
        harness.event(Event::MouseWheel {
            unit: MouseWheelUnit::Point,
            delta: egui::vec2(0.0, 150.0),
            phase: TouchPhase::Move,
            modifiers: Modifiers::NONE,
        });
        // The wheel scrolls smoothly, over some frames.
        harness.run_steps(30);
        let moved = at(&harness) - before;
        assert!(harness.state().list_scrolled() < scrolled, "scrolled up");
        assert!((moved - 150.0).abs() < 2.0, "moved {moved}");
    }
    Ok(())
}
