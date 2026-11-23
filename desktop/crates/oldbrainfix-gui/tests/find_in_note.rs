//! A search leads to what it found (obf-1047) on the desktop: under each
//! card of a search's list its first three places found, each its line
//! with the words found tinted, and "+n more"; a click on one opens the
//! Note at it, opening the card at its first place; a place in a long
//! table unfolds the table on the card. The opened Note shows every
//! place tinted, the current one stronger, with the find bar "2 of 4",
//! Previous and Next, Enter and Shift+Enter, its words editable there,
//! ✕ and Esc closing it as leaving the Note does. Ctrl+F opens the bar
//! over the Note dialog and in a Note's own window, its reading view
//! too; over the list it still searches the list.

// A test may unwrap, expect, panic and index, its helpers too (#229).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, Duration, Utc};
use egui::accesskit::Role;
use egui::{Color32, Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::text_format::{MARKDOWN, PLAIN};
use oldbrainfix_core::{Checklists, Notes, OwnProfile, SigningKey, Store};
use oldbrainfix_gui::{App, Selection, theme};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const SHOP: &str =
    "Buy milk at Müller\nThe milk at MÜLLER is cheap\nmuller's bread\nAsk Muller for eggs";

/// A store on a clock that moves a microsecond per read, with the Notes
/// `texts` captured into Unsorted in that order, plain or in `format`:
/// the store and their ids.
fn store(texts: &[(&str, &str)]) -> Result<(Arc<Store>, Vec<String>), Box<dyn std::error::Error>> {
    let start = DateTime::parse_from_rfc3339("2026-03-02T11:00:00Z")?.with_timezone(&Utc);
    let ticks = Arc::new(AtomicI64::new(0));
    let clock =
        Box::new(move || start + Duration::microseconds(ticks.fetch_add(1, Ordering::SeqCst)));
    let store = Store::in_memory(clock, || Ok(SigningKey::from_seed([9; 32])))?;
    OwnProfile::new(&store).apply_defaults("Desk", None)?;
    let notes = Notes::new(&store);
    let mut ids = Vec::new();
    for (text, format) in texts {
        ids.push(notes.capture_as(text, format, None, None)?.id);
    }
    Ok((Arc::new(store), ids))
}

/// The main window over `store` on All Notes.
fn window(store: &Arc<Store>) -> Harness<'static, App> {
    let mut app = App::new(store.clone(), "en");
    app.select(Selection::All);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 1000.0))
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

/// The Note `note` of `store` in a window of its own.
fn alone(store: &Arc<Store>, note: &str) -> Harness<'static, App> {
    let app = App::for_note_window(store.clone(), "en", note, true);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 900.0))
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show_note_window(ui);
            },
            app,
        );
    harness.run();
    harness
}

/// The words drawn on a find's tint, the current one's (`current`) or
/// the others', top first as they were drawn.
fn tinted(harness: &Harness<'_, App>, current: bool) -> Vec<String> {
    fn each(shape: &egui::Shape, tint: Color32, out: &mut Vec<String>) {
        match shape {
            egui::Shape::Text(text) => {
                let job = &text.galley.job;
                for section in &job.sections {
                    if section.format.background == tint {
                        let range = section.byte_range.start.0..section.byte_range.end.0;
                        out.push(job.text[range].to_string());
                    }
                }
            }
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| each(s, tint, out)),
            _ => {}
        }
    }
    let dark = harness.ctx.theme() == egui::Theme::Dark;
    let tint = theme::found_tint(dark, current);
    let mut out = Vec::new();
    for clipped in &harness.output().shapes {
        each(&clipped.shape, tint, &mut out);
    }
    out
}

/// How many buttons are labelled `label`.
fn buttons(harness: &Harness<'_, App>, label: &str) -> usize {
    harness
        .query_all_by(|n| n.role() == Role::Button && n.label().as_deref() == Some(label))
        .count()
}

fn press(harness: &mut Harness<'_, App>, modifiers: Modifiers, key: Key) {
    harness.key_press_modifiers(modifiers, key);
    harness.run();
}

/// What the widget with the keyboard is called, or its hint.
fn focused_label(harness: &Harness<'_, App>) -> Option<String> {
    let node = harness.query_by(|n| n.is_focused())?.accesskit_node();
    node.label()
        .or_else(|| node.placeholder().map(str::to_string))
}

#[test]
fn a_search_lists_its_places_under_the_card_and_a_click_opens_the_note_at_one() -> TestResult {
    let (store, ids) = store(&[(SHOP, PLAIN), ("Stamps", PLAIN)])?;
    let mut harness = window(&store);
    harness.state_mut().set_query("muller");
    harness.run();
    assert_eq!(harness.state().shown_notes(), [ids[0].clone()]);
    // Its first three places, each its line, then how many more.
    let found = [
        "Buy milk at Müller",
        "The milk at MÜLLER is cheap",
        "muller's bread",
    ];
    for line in found {
        assert_eq!(buttons(&harness, &format!("Open at “{line}”")), 1, "{line}");
    }
    assert_eq!(buttons(&harness, "Open at “Ask Muller for eggs”"), 0);
    assert_eq!(buttons(&harness, "+1 more"), 1);
    // The words found tinted on the card and under it.
    assert!(tinted(&harness, false).iter().any(|w| w == "MÜLLER"));
    harness
        .get_by_role_and_label(Role::Button, "Open at “The milk at MÜLLER is cheap”")
        .click();
    harness.run();
    assert!(harness.state().note_open());
    assert_eq!(harness.state().find_bar(), Some(("muller", 1, 4)));
    assert!(harness.query_all_by_value("2 of 4").next().is_some());
    harness.run_steps(4);
    // The list's cards behind the dialog keep theirs; the current one is
    // the dialog's alone.
    assert_eq!(tinted(&harness, true), ["MÜLLER"]);
    // "+1 more" opens it at the fourth.
    harness.key_press(Key::Escape);
    harness.run();
    harness.key_press(Key::Escape);
    harness.run();
    assert!(!harness.state().note_open());
    harness.get_by_label("+1 more").click();
    harness.run();
    assert_eq!(harness.state().find_bar(), Some(("muller", 3, 4)));
    Ok(())
}

#[test]
fn a_found_card_opens_at_its_first_place_and_leaving_closes_the_bar() -> TestResult {
    let (store, ids) = store(&[(SHOP, PLAIN)])?;
    let mut harness = window(&store);
    harness.state_mut().set_query("milk");
    harness.run();
    harness.state_mut().select_note(&ids[0]);
    harness.run();
    assert_eq!(harness.state().find_bar(), Some(("milk", 0, 2)));
    harness.state_mut().close_note();
    harness.run();
    assert_eq!(harness.state().find_bar(), None);
    // Without a search the Note opens without it.
    harness.state_mut().set_query("");
    harness.run();
    harness.state_mut().select_note(&ids[0]);
    harness.run();
    assert_eq!(harness.state().find_bar(), None);
    Ok(())
}

#[test]
fn the_bar_steps_with_its_buttons_and_keys_and_esc_closes_it_first() -> TestResult {
    let (store, ids) = store(&[("milk, eggs and more milk\nmilk again", PLAIN)])?;
    let mut harness = window(&store);
    harness.state_mut().select_note(&ids[0]);
    harness.run();
    assert_eq!(harness.state().find_bar(), None);
    // Ctrl+F over the Note opens its bar, the Note staying, the cursor
    // in the bar's field.
    press(&mut harness, Modifiers::COMMAND, Key::F);
    assert!(harness.state().note_open());
    assert_eq!(harness.state().find_bar(), Some(("", 0, 0)));
    assert_eq!(
        focused_label(&harness).as_deref(),
        Some("Find in this Note")
    );
    harness.get_by(|n| n.is_focused()).type_text("milk");
    harness.run();
    assert_eq!(harness.state().find_bar(), Some(("milk", 0, 3)));
    assert!(harness.query_all_by_value("1 of 3").next().is_some());
    // Enter goes on, Shift+Enter back, round at the ends; the field
    // keeps the cursor.
    press(&mut harness, Modifiers::NONE, Key::Enter);
    assert_eq!(harness.state().find_bar(), Some(("milk", 1, 3)));
    assert_eq!(
        focused_label(&harness).as_deref(),
        Some("Find in this Note")
    );
    press(&mut harness, Modifiers::SHIFT, Key::Enter);
    press(&mut harness, Modifiers::SHIFT, Key::Enter);
    assert_eq!(harness.state().find_bar(), Some(("milk", 2, 3)));
    harness.get_by_label("Next place found").click();
    harness.run();
    assert_eq!(harness.state().find_bar(), Some(("milk", 0, 3)));
    harness.get_by_label("Previous place found").click();
    harness.run();
    assert_eq!(harness.state().find_bar(), Some(("milk", 2, 3)));
    // In the Note's editable text, every place tinted, the current one
    // stronger.
    harness.run_steps(4);
    assert_eq!(tinted(&harness, true), ["milk"]);
    assert_eq!(tinted(&harness, false), ["milk", "milk"]);
    // Ctrl+F again puts the cursor back, its words selected to be typed
    // over; words found nowhere say so.
    press(&mut harness, Modifiers::COMMAND, Key::F);
    harness.get_by(|n| n.is_focused()).type_text("tea");
    harness.run();
    assert_eq!(harness.state().find_bar(), Some(("tea", 0, 0)));
    assert!(
        harness
            .query_all_by_value("Not in this Note")
            .next()
            .is_some()
    );
    // Esc closes the bar first, then the Note.
    press(&mut harness, Modifiers::NONE, Key::Escape);
    assert_eq!(harness.state().find_bar(), None);
    assert!(harness.state().note_open());
    assert!(tinted(&harness, false).is_empty());
    press(&mut harness, Modifiers::NONE, Key::Escape);
    assert!(!harness.state().note_open());
    // ✕ closes it too.
    harness.state_mut().select_note(&ids[0]);
    harness.run();
    press(&mut harness, Modifiers::COMMAND, Key::F);
    harness.get_by_label("Close the find bar").click();
    harness.run();
    assert_eq!(harness.state().find_bar(), None);
    assert!(harness.state().note_open());
    Ok(())
}

#[test]
fn over_the_list_ctrl_f_still_puts_the_cursor_into_its_search() -> TestResult {
    let (store, _) = store(&[("Milk", PLAIN)])?;
    let mut harness = window(&store);
    press(&mut harness, Modifiers::COMMAND, Key::F);
    assert_eq!(focused_label(&harness).as_deref(), Some("Search all Notes"));
    assert_eq!(harness.state().find_bar(), None);
    Ok(())
}

#[test]
fn a_place_in_a_long_table_unfolds_it_on_the_card_and_shows_tinted() -> TestResult {
    let mut table = String::from("| Day | Reading |\n| --- | --- |");
    for n in 1..=40 {
        table.push_str(&format!("\n| row {n} | {} |", n * 3));
    }
    let (store, ids) = store(&[(&table, MARKDOWN)])?;
    let mut harness = window(&store);
    // Folded, the card shows only its first rows.
    assert!(harness.query_all_by_value("row 30").next().is_none());
    harness.state_mut().set_query("row 30");
    harness.run();
    harness.run_steps(2);
    assert!(harness.query_all_by_value("row 30").next().is_some());
    assert!(tinted(&harness, false).iter().any(|w| w == "row 30"));
    assert_eq!(buttons(&harness, "Show all 40"), 0);
    // Opened at it, the table's row on the current tint.
    harness
        .get_by(|n| {
            n.role() == Role::Button && n.label().is_some_and(|l| l.starts_with("Open at “row 30"))
        })
        .click_accesskit();
    harness.run();
    harness.run_steps(4);
    assert_eq!(harness.state().find_bar(), Some(("row 30", 0, 1)));
    assert_eq!(harness.state().shown_notes(), [ids[0].clone()]);
    assert_eq!(tinted(&harness, true), ["row 30"]);
    Ok(())
}

#[test]
fn ctrl_f_opens_the_bar_in_a_notes_own_window_and_its_reading_view() -> TestResult {
    let (store, ids) = store(&[("# Pancakes\n\n200 g **flour**, more flour", MARKDOWN)])?;
    let mut harness = alone(&store, &ids[0]);
    press(&mut harness, Modifiers::COMMAND, Key::F);
    assert_eq!(
        focused_label(&harness).as_deref(),
        Some("Find in this Note")
    );
    harness.get_by(|n| n.is_focused()).type_text("flour");
    harness.run();
    assert_eq!(harness.state().find_bar(), Some(("flour", 0, 2)));
    harness.run_steps(4);
    assert_eq!(tinted(&harness, true), ["flour"]);
    // The reading view keeps it.
    press(&mut harness, Modifiers::NONE, Key::F9);
    assert_eq!(harness.state().reading(), Some(true));
    assert_eq!(harness.state().find_bar(), Some(("flour", 0, 2)));
    harness.run_steps(4);
    assert_eq!(tinted(&harness, false), ["flour"]);
    // Esc closes the bar before it leaves the reading view.
    press(&mut harness, Modifiers::NONE, Key::Escape);
    assert_eq!(harness.state().find_bar(), None);
    assert_eq!(harness.state().reading(), Some(true));
    Ok(())
}

/// Where the words on the current place's tint are drawn, wholly inside
/// what they are clipped to and the window.
fn current_shown(harness: &Harness<'_, App>) -> bool {
    fn each(shape: &egui::Shape, clip: egui::Rect, tint: Color32, out: &mut Vec<bool>) {
        match shape {
            egui::Shape::Text(text) => {
                let job = &text.galley.job;
                let screen =
                    egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 1000.0));
                for section in job.sections.iter().filter(|s| s.format.background == tint) {
                    // Where the place itself lies, not the whole text.
                    let chars = job.text[..section.byte_range.start.0].chars().count();
                    let at = text.galley.pos_from_cursor(egui::text::CCursor::new(chars));
                    let rect = at.translate(text.pos.to_vec2());
                    out.push(clip.contains(rect.center()) && screen.contains(rect.center()));
                }
            }
            egui::Shape::Vec(shapes) => shapes.iter().for_each(|s| each(s, clip, tint, out)),
            _ => {}
        }
    }
    let dark = harness.ctx.theme() == egui::Theme::Dark;
    let tint = theme::found_tint(dark, true);
    let mut out = Vec::new();
    for clipped in &harness.output().shapes {
        each(&clipped.shape, clipped.clip_rect, tint, &mut out);
    }
    !out.is_empty() && out.iter().all(|&shown| shown)
}

#[test]
fn the_current_place_comes_into_view_in_a_long_text_and_a_long_table() -> TestResult {
    let lines: Vec<String> = (1..=150).map(|n| format!("line {n}")).collect();
    let long = format!("{}\nthe needle\n", lines.join("\n"));
    let mut table = String::from("| Day | Reading |\n| --- | --- |");
    for n in 1..=200 {
        table.push_str(&format!("\n| day {n} | {} |", n * 3));
    }
    let (store, ids) = store(&[(&long, PLAIN), (&table, MARKDOWN)])?;
    let mut harness = window(&store);
    for (id, words) in [(&ids[0], "needle"), (&ids[1], "day 170")] {
        harness.state_mut().select_note(id);
        harness.run();
        press(&mut harness, Modifiers::COMMAND, Key::F);
        harness.get_by(|n| n.is_focused()).type_text(words);
        // Settled: the place scrolled to once, nothing scrolling on.
        harness.run();
        harness.run();
        assert_eq!(harness.state().find_bar().map(|b| b.2), Some(1), "{words}");
        assert!(current_shown(&harness), "{words} in view");
        harness.state_mut().close_note();
        harness.run();
    }
    Ok(())
}

#[test]
fn in_a_list_edited_here_the_bar_finds_in_its_name_and_its_description() -> TestResult {
    let (store, ids) = store(&[("Packing for Köln", PLAIN)])?;
    Checklists::new(&store).make(&ids[0])?;
    // Its description under its name.
    Notes::new(&store).edit(&ids[0], "Packing for Köln\nThe tickets for Köln first")?;
    let mut harness = window(&store);
    harness.state_mut().select_note(&ids[0]);
    harness.run();
    press(&mut harness, Modifiers::COMMAND, Key::F);
    harness.get_by(|n| n.is_focused()).type_text("koln");
    harness.run();
    assert_eq!(harness.state().find_bar(), Some(("koln", 0, 2)));
    // The current one in the name's field, then in the description's.
    harness.run_steps(4);
    assert_eq!(tinted(&harness, true), ["Köln"]);
    assert_eq!(tinted(&harness, false), ["Köln"]);
    press(&mut harness, Modifiers::NONE, Key::Enter);
    harness.run_steps(4);
    assert_eq!(harness.state().find_bar(), Some(("koln", 1, 2)));
    assert_eq!(tinted(&harness, true), ["Köln"]);
    assert_eq!(tinted(&harness, false), ["Köln"]);
    Ok(())
}

#[test]
fn a_bar_never_follows_onto_another_note() -> TestResult {
    let (store, ids) = store(&[("milk and bread", PLAIN), ("milk for tea", PLAIN)])?;
    let mut harness = window(&store);
    harness.state_mut().set_query("milk");
    harness.run();
    harness.state_mut().select_note(&ids[0]);
    harness.run();
    assert_eq!(harness.state().find_bar(), Some(("milk", 0, 1)));
    // Another Note opened over it, the search gone: no bar.
    harness.state_mut().set_query("");
    harness.run();
    harness.state_mut().select_note(&ids[1]);
    harness.run();
    assert_eq!(harness.state().find_bar(), None);
    assert!(tinted(&harness, false).is_empty());
    Ok(())
}

#[test]
fn the_bars_buttons_fit_beside_words_found_nowhere_in_german() -> TestResult {
    let (store, ids) = store(&[("Milch", PLAIN)])?;
    let app = App::for_note_window(store.clone(), "de", &ids[0], true);
    let size = oldbrainfix_gui::window_sizes::MIN_NOTE_DIALOG;
    let mut harness = Harness::builder().with_size(size).build_ui_state(
        |ui, app: &mut App| {
            App::install(ui.ctx());
            app.show_note_window(ui);
        },
        app,
    );
    harness.run();
    press(&mut harness, Modifiers::COMMAND, Key::F);
    harness
        .get_by(|n| n.is_focused())
        .type_text("Tee mit Zitrone");
    harness.run();
    assert!(
        harness
            .query_all_by_value("Nicht in dieser Notiz")
            .next()
            .is_some()
    );
    let window = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
    for label in ["Suchleiste schließen", "Nächste Stelle", "Vorherige Stelle"] {
        let rect = harness.get_by_label(label).rect();
        assert!(window.contains_rect(rect), "{label} at {rect:?}");
    }
    harness.get_by_label("Suchleiste schließen").click();
    harness.run();
    assert_eq!(harness.state().find_bar(), None);
    Ok(())
}
