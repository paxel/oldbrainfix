//! The texts the Note dialog only shows (#255): a Note's in the trash,
//! another person's, Transcripts, Photo Texts and the History select by
//! a drag, also over their formats, and Ctrl+C copies them; a
//! right-click on one opens Copy, greyed while nothing is selected, and
//! Select all. A click on a link in one still opens it, a drag from it
//! selects.

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
use egui::{Event, Modifiers, OutputCommand, PointerButton, Pos2, Rect};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::text_format::MARKDOWN;
use oldbrainfix_core::{Notes, OwnProfile, SigningKey, Store};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn store() -> Result<Arc<Store>, Box<dyn std::error::Error>> {
    let start = DateTime::parse_from_rfc3339("2026-03-02T11:00:00Z")?.with_timezone(&Utc);
    let ticks = Arc::new(AtomicI64::new(0));
    let clock = Box::new(move || start + Duration::seconds(ticks.fetch_add(1, Ordering::SeqCst)));
    let store = Store::in_memory(clock, || Ok(SigningKey::from_seed([6; 32])))?;
    OwnProfile::new(&store).apply_defaults("Ann", None)?;
    Ok(Arc::new(store))
}

/// The window on `place` with the Note `note` open in its dialog.
fn opened(store: &Arc<Store>, place: Selection, note: &str) -> Harness<'static, App> {
    let mut app = App::new(store.clone(), "en");
    app.set_region(Some("DE"));
    app.select(place);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 1400.0))
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            app,
        );
    harness.run();
    harness.state_mut().select_note(note);
    harness.run();
    assert!(harness.state().note_open());
    harness
}

/// A Note `text` formatted, in the trash, where it is only read.
fn trashed(store: &Store, text: &str) -> Result<String, Box<dyn std::error::Error>> {
    let notes = Notes::new(store);
    let note = notes.capture_as(text, MARKDOWN, None, None)?.id;
    notes.trash(&note)?;
    Ok(note)
}

/// Where the last label holding `text` lies: in the Note dialog, over
/// the card of the list.
fn rect(harness: &Harness<'_, App>, text: &str) -> Rect {
    harness
        .query_all_by_label_contains(text)
        .rfind(|n| n.accesskit_node().role() == Role::Label)
        .unwrap_or_else(|| panic!("no {text:?}"))
        .rect()
}

fn press(harness: &mut Harness<'_, App>, pos: Pos2, button: PointerButton, pressed: bool) {
    harness.event(Event::PointerButton {
        pos,
        button,
        pressed,
        modifiers: Modifiers::NONE,
    });
    harness.step();
}

/// A drag of the mouse from `from` to `to`.
fn drag(harness: &mut Harness<'_, App>, from: Pos2, to: Pos2) {
    harness.event(Event::PointerMoved(from));
    press(harness, from, PointerButton::Primary, true);
    harness.event(Event::PointerMoved(to));
    harness.step();
    press(harness, to, PointerButton::Primary, false);
    harness.run();
}

fn right_click(harness: &mut Harness<'_, App>, at: Pos2) {
    harness.event(Event::PointerMoved(at));
    press(harness, at, PointerButton::Secondary, true);
    press(harness, at, PointerButton::Secondary, false);
    harness.run();
}

/// What the frames from now on copy.
fn copied(harness: &mut Harness<'_, App>) -> String {
    let mut copied = String::new();
    for _ in 0..4 {
        harness.step();
        for command in &harness.output().platform_output.commands {
            if let OutputCommand::CopyText(text) = command {
                copied.clone_from(text);
            }
        }
    }
    copied
}

/// Ctrl+C, as the window brings it.
fn ctrl_c(harness: &mut Harness<'_, App>) -> String {
    harness.event(Event::Copy);
    copied(harness)
}

fn entry_enabled(harness: &Harness<'_, App>, label: &str) -> bool {
    harness
        .query_all_by_role_and_label(Role::Button, label)
        .last()
        .is_some_and(|n| !n.accesskit_node().is_disabled())
}

fn pick(harness: &mut Harness<'_, App>, label: &str) -> String {
    harness
        .query_all_by_role_and_label(Role::Button, label)
        .last()
        .unwrap_or_else(|| panic!("no entry {label:?}"))
        .click();
    copied(harness)
}

#[test]
fn a_drag_over_a_formatted_text_only_read_selects_it_and_ctrl_c_copies_it() -> TestResult {
    let store = store()?;
    let note = trashed(&store, "Buy **fresh milk** for #home")?;
    let mut harness = opened(&store, Selection::Trash, &note);
    let words = rect(&harness, "Buy fresh milk");
    let from = egui::pos2(words.left() + 1.0, words.center().y);
    let to = egui::pos2(words.right() - 1.0, words.center().y);
    drag(&mut harness, from, to);
    let copied = ctrl_c(&mut harness);
    assert!(copied.contains("fresh milk"), "{copied:?}");
    assert!(harness.state().note_open());
    Ok(())
}

#[test]
fn a_right_click_on_a_text_only_read_offers_copy_and_select_all() -> TestResult {
    let store = store()?;
    let note = trashed(&store, "Buy **milk**\n\nand *eggs* too")?;
    let mut harness = opened(&store, Selection::Trash, &note);
    let first = rect(&harness, "Buy milk");
    right_click(&mut harness, first.center());
    // Nothing selected yet.
    assert!(!entry_enabled(&harness, "Copy"));
    assert!(entry_enabled(&harness, "Select all"));
    pick(&mut harness, "Select all");
    harness.run();
    assert!(harness.state().note_open());
    // The whole text selected, over both its lines; the right-click
    // keeps it, and Copy copies it.
    right_click(&mut harness, first.center());
    assert!(entry_enabled(&harness, "Copy"));
    let copied = pick(&mut harness, "Copy");
    assert!(
        copied.contains("Buy milk") && copied.contains("and eggs too"),
        "{copied:?}"
    );
    assert!(harness.state().note_open());
    // The menu is gone, the selection is not: Ctrl+C copies it too.
    assert!(
        harness
            .query_all_by_role_and_label(Role::Button, "Select all")
            .next()
            .is_none()
    );
    let again = ctrl_c(&mut harness);
    assert!(again.contains("and eggs too"), "{again:?}");
    Ok(())
}

#[test]
fn transcripts_and_photo_texts_select_to_be_copied() -> TestResult {
    let store = store()?;
    let notes = Notes::new(&store);
    let note = notes.capture("Shed", None, None)?.id;
    let wav = oldbrainfix_gui::media::wav(&[0.0; 1600]);
    let recording = notes.add_recording(&note, &wav)?;
    notes.set_transcript(&note, &recording, "Lock code one two three four")?;
    let image = image::RgbaImage::from_pixel(8, 8, image::Rgba([20, 120, 200, 255]));
    let mut png = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image).write_to(&mut png, image::ImageFormat::Png)?;
    let photo = notes.add_photo(&note, &png.into_inner())?;
    notes.set_photo_text(&note, &photo, "Spare key under the pot")?;
    let mut harness = opened(&store, Selection::All, &note);

    let said = rect(&harness, "Lock code");
    right_click(&mut harness, said.center());
    pick(&mut harness, "Select all");
    harness.run();
    right_click(&mut harness, said.center());
    assert_eq!(pick(&mut harness, "Copy"), "Lock code one two three four");

    let read = rect(&harness, "Spare key");
    let from = egui::pos2(read.left() + 1.0, read.center().y);
    drag(&mut harness, from, read.right_center());
    assert_eq!(ctrl_c(&mut harness), "Spare key under the pot");
    Ok(())
}

#[test]
fn the_history_selects_to_be_copied() -> TestResult {
    let store = store()?;
    let notes = Notes::new(&store);
    let note = notes.capture("Dentist on Tuesday", None, None)?.id;
    notes.edit(&note, "Dentist on Wednesday")?;
    let mut harness = opened(&store, Selection::All, &note);
    harness
        .get_all_by_label("Note")
        .next()
        .ok_or("the Note menu")?
        .click();
    harness.run();
    harness
        .get_all_by_label("History")
        .last()
        .ok_or("History")?
        .click();
    harness.run();
    let before = rect(&harness, "Text changed");
    right_click(&mut harness, before.center());
    pick(&mut harness, "Select all");
    harness.run();
    right_click(&mut harness, before.center());
    let copied = pick(&mut harness, "Copy");
    assert!(
        copied.contains("Text changed")
            && copied.contains("Dentist on Tuesday")
            && copied.contains("Dentist on Wednesday"),
        "{copied:?}"
    );
    Ok(())
}

#[test]
fn a_click_on_a_link_opens_it_and_a_drag_from_it_selects() -> TestResult {
    let store = store()?;
    let note = trashed(&store, "[Shop](https://example.org) for milk")?;
    let mut harness = opened(&store, Selection::Trash, &note);
    let line = rect(&harness, "Shop for milk");
    let on_link = egui::pos2(line.left() + 8.0, line.center().y);
    drag(&mut harness, on_link, line.right_center());
    let copied = ctrl_c(&mut harness);
    assert!(copied.contains("for milk"), "{copied:?}");
    harness.event(Event::PointerMoved(on_link));
    press(&mut harness, on_link, PointerButton::Primary, true);
    harness.event(Event::PointerButton {
        pos: on_link,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    let mut opened = Vec::new();
    for _ in 0..3 {
        harness.step();
        for command in &harness.output().platform_output.commands {
            if let OutputCommand::OpenUrl(open) = command {
                opened.push(open.url.clone());
            }
        }
    }
    assert_eq!(opened, ["https://example.org"]);
    Ok(())
}
