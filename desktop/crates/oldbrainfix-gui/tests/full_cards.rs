//! The Note lists' full and compact cards (#114), driven headless by
//! egui_kittest: every Note in full with its Sections and its labelled
//! Transcripts and Photo Texts by default, the full/compact switch kept
//! per list on this Device, a click unfolding a compact Note, a
//! double-click or Edit opening the editor, and a photo of extreme
//! proportions capped on its card.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::Duration;
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::{Notes, Places, SigningKey, Store, parse_time};
use oldbrainfix_gui::media;
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const TEXT: &str = "Dentist number\nCall before 9";
const AMEND: &str = "Moved to Wednesday";

/// An empty store whose clock moves a second on every look, so each
/// Note and Section has a time of its own.
fn store() -> Result<Arc<Store>, Box<dyn std::error::Error>> {
    let start = parse_time("2026-10-26T09:00:00.000Z")?;
    let tick = AtomicI64::new(0);
    let clock = move || start + Duration::seconds(tick.fetch_add(1, Ordering::Relaxed));
    let store = Store::in_memory(Box::new(clock), || Ok(SigningKey::from_seed([9; 32])))?;
    Ok(Arc::new(store))
}

/// A picture of `width` × `height` pixels, as a PNG.
fn png(width: u32, height: u32) -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(width, height, image::Rgba([20, 120, 200, 255]));
    let mut out = std::io::Cursor::new(Vec::new());
    let written =
        image::DynamicImage::ImageRgba8(image).write_to(&mut out, image::ImageFormat::Png);
    assert!(written.is_ok());
    out.into_inner()
}

/// The Dentist Note, Unsorted: two lines, a photo with its Photo Text, a
/// recording with its Transcript and an Amend; and a second one-line
/// Note. Their ids.
fn notes(store: &Store) -> Result<(String, String), Box<dyn std::error::Error>> {
    let notes = Notes::new(store);
    let recording = media::wav(&[0.0; 160]);
    let dentist = notes.capture_with(TEXT, None, None, &[png(40, 30)], &[recording])?;
    notes.set_photo_text(&dentist.id, &dentist.photos[0], "RECEIPT 12,50")?;
    notes.set_transcript(&dentist.id, &dentist.recordings[0], "Ask about the crown")?;
    notes.amend(&dentist.id, AMEND)?;
    let milk = notes.capture("Buy milk", None, None)?;
    Ok((dentist.id, milk.id))
}

/// The window over `store` on `selection`.
fn window(store: Arc<Store>, selection: Selection) -> Harness<'static, App> {
    let mut app = App::new(store, "en");
    app.select(selection);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 1400.0))
        // A frame per event: two clicks this quick make a double-click.
        .with_step_dt(0.02)
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

/// Whether anything in the window is labelled `label`.
fn shows(harness: &Harness<'_, App>, label: &str) -> bool {
    harness.query_all_by_label(label).next().is_some()
}

/// The card of the Note with `text`.
fn card<'a>(harness: &'a Harness<'_, App>, text: &'a str) -> egui_kittest::Node<'a> {
    harness.get_by_role_and_label(Role::Button, text)
}

/// Lets more than a double-click's time pass, so the next click is one
/// of its own.
fn pause(harness: &mut Harness<'_, App>) {
    harness.run_steps(50);
}

/// Clicks the button `label`.
fn press(harness: &mut Harness<'_, App>, label: &str) {
    harness.get_by_role_and_label(Role::Button, label).click();
    harness.run();
}

/// The Edit button on the card of the Note with `text`.
fn edit<'a>(harness: &'a Harness<'_, App>, text: &str) -> egui_kittest::Node<'a> {
    let card = card(harness, text).rect();
    harness
        .query_all_by_role_and_label(Role::Button, "Edit")
        .find(|n| card.contains_rect(n.rect()))
        .unwrap_or_else(|| panic!("no Edit on the card of {text:?}"))
}

/// The setting this Device keeps for the Note list `key`.
fn kept(store: &Store, key: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
    Ok(store.local_setting(&format!("compactNotes:{key}"))?)
}

#[test]
fn a_note_list_shows_every_note_in_full_with_its_sections_and_labelled_texts() -> TestResult {
    let store = store()?;
    notes(&store)?;
    let harness = window(store, Selection::All);
    assert!(!harness.state().compact());
    // The whole text, not only its first line.
    assert_eq!(harness.query_all_by_label(TEXT).count(), 2, "card and text");
    // The photo at the card's width, its Photo Text and the Transcript
    // labelled under them.
    let photo = harness.get_by_role_and_label(Role::Image, "Photo").rect();
    assert!(photo.width() > 200.0, "{photo:?}");
    for label in [
        "Photo Text",
        "RECEIPT 12,50",
        "Transcript",
        "Ask about the crown",
    ] {
        assert!(shows(&harness, label), "{label}");
    }
    let photo_text = harness.get_by_label("RECEIPT 12,50").rect();
    assert!(photo.bottom() <= photo_text.top());
    // The Amend under the Note, on its card.
    let amend = harness.get_by_label(AMEND).rect();
    let dentist = card(&harness, TEXT).rect();
    assert!(dentist.contains_rect(amend));
    assert!(harness.get_by_label("Ask about the crown").rect().bottom() <= amend.top());
    // Nothing opened the editor.
    assert_eq!(harness.state().selected_note(), None);
    Ok(())
}

#[test]
fn the_switch_shows_a_list_compact_and_each_list_keeps_its_own() -> TestResult {
    let store = store()?;
    let (dentist, _) = notes(&store)?;
    let area = Places::new(&store).add_area("Household")?;
    Notes::new(&store).capture("Fix the tap", Some(&area.id), None)?;
    let mut harness = window(store.clone(), Selection::All);
    press(&mut harness, "Show Notes compact");
    assert!(harness.state().compact());
    assert_eq!(kept(&store, "allNotes")?.as_deref(), Some("true"));
    // Compact: the first line and the media, not the Sections.
    assert!(shows(&harness, "Dentist number"));
    assert!(!shows(&harness, AMEND));
    assert!(!shows(&harness, "Photo Text"));
    assert!(shows(&harness, "Show Notes in full"));
    // Unsorted and the Area keep their own choice: full.
    harness.state_mut().select(Selection::Unsorted);
    harness.run();
    assert!(!harness.state().compact());
    assert!(shows(&harness, AMEND));
    harness
        .state_mut()
        .select(Selection::Place(area.id.clone()));
    harness.run();
    assert!(!harness.state().compact());
    press(&mut harness, "Show Notes compact");
    assert_eq!(
        kept(&store, &format!("area:{}", area.id))?.as_deref(),
        Some("true")
    );
    // Back on all Notes it is still compact; full again, nothing is kept.
    harness.state_mut().select(Selection::All);
    harness.run();
    assert!(harness.state().compact());
    press(&mut harness, "Show Notes in full");
    assert!(!harness.state().compact());
    assert_eq!(kept(&store, "allNotes")?, None);
    assert!(shows(&harness, AMEND));
    // The trash shows its Notes in full, without a switch.
    Notes::new(&store).trash(&dentist)?;
    harness.state_mut().select(Selection::Trash);
    harness.run();
    assert!(shows(&harness, AMEND));
    assert!(!shows(&harness, "Show Notes compact"));
    assert!(!shows(&harness, "Show Notes in full"));
    Ok(())
}

#[test]
fn a_click_unfolds_a_compact_note_and_a_double_click_or_edit_opens_the_editor() -> TestResult {
    let store = store()?;
    let (dentist, milk) = notes(&store)?;
    store.set_local_setting("compactNotes:allNotes", "true")?;
    let mut harness = window(store, Selection::All);
    assert!(harness.state().compact());
    assert!(!shows(&harness, AMEND));
    // A click unfolds it in place and opens nothing.
    card(&harness, TEXT).click();
    harness.run();
    assert!(shows(&harness, AMEND));
    assert!(shows(&harness, "Photo Text"));
    assert!(harness.state().compact());
    assert_eq!(harness.state().selected_note(), None);
    // On a full card a click does nothing.
    pause(&mut harness);
    card(&harness, TEXT).click();
    harness.run();
    assert!(shows(&harness, AMEND));
    assert_eq!(harness.state().selected_note(), None);
    // A double-click opens the editor on the right.
    pause(&mut harness);
    let node = card(&harness, "Buy milk");
    node.click();
    node.click();
    harness.run();
    assert_eq!(harness.state().selected_note(), Some(milk.as_str()));
    // So does Edit.
    edit(&harness, TEXT).click();
    harness.run();
    assert_eq!(harness.state().selected_note(), Some(dentist.as_str()));
    // Another list folds what was unfolded.
    harness.state_mut().select(Selection::Unsorted);
    harness.run();
    harness.state_mut().select(Selection::All);
    harness.run();
    assert!(!harness.query_all_by_label(AMEND).any(|n| {
        // The editor on the right shows the Section; the card does not.
        card(&harness, TEXT).rect().contains_rect(n.rect())
    }));
    Ok(())
}

#[test]
fn a_photo_of_extreme_proportions_shows_capped_and_decoded_small() -> TestResult {
    let store = store()?;
    let notes = Notes::new(&store);
    notes.capture_with("Tall", None, None, &[png(12, 34_000)], &[])?;
    notes.capture_with("Wide", None, None, &[png(34_000, 12)], &[])?;
    let mut harness = window(store, Selection::All);
    let photos: Vec<egui::Rect> = harness
        .query_all_by_role_and_label(Role::Image, "Photo")
        .map(|n| n.rect())
        .collect();
    assert_eq!(photos.len(), 2);
    for rect in &photos {
        assert!(rect.height() <= 480.5, "{rect:?}");
        assert!(rect.height() >= 47.5, "{rect:?}");
        assert!(rect.width() <= 1280.0, "{rect:?}");
    }
    let textures = |harness: &Harness<'_, App>| -> Vec<[usize; 2]> {
        let manager = harness.ctx.tex_manager();
        let manager = manager.read();
        manager
            .allocated()
            .filter(|(_, meta)| meta.name.starts_with("photo:"))
            .map(|(_, meta)| meta.size)
            .collect()
    };
    let decoded = textures(&harness);
    assert_eq!(decoded.len(), 2);
    for size in &decoded {
        assert!(size[0] <= 1024 && size[1] <= 1024, "{size:?}");
        assert!(size[0] >= 1 && size[1] >= 1, "{size:?}");
    }
    // Decoded once: more frames make no more.
    harness.run();
    harness.run();
    assert_eq!(textures(&harness).len(), 2);
    Ok(())
}
