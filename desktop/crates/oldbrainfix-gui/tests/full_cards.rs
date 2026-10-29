//! The Note lists' full and compact cards (#114), driven headless by
//! egui_kittest: every Note in full with its Sections and its labelled
//! Transcripts and Photo Texts by default, the full/compact switch kept
//! per list on this Device, a click unfolding a compact Note, a
//! double-click or Edit opening the editor, a photo of extreme
//! proportions capped on its card, the Capture Context of the Note and
//! of each Section, and a click on a photo showing it full screen to
//! zoom.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::Duration;
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::named_places::keys as place_keys;
use oldbrainfix_core::notes::keys as note_keys;
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
    // On a full card a click does nothing, but on a photo; here on its
    // text.
    pause(&mut harness);
    harness.get_by_role_and_label(Role::Label, TEXT).click();
    harness.run();
    assert!(shows(&harness, AMEND));
    assert_eq!(harness.state().selected_note(), None);
    assert_eq!(harness.state().viewed_photo(), None);
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

/// The Named Place Home around `centre`.
fn home(store: &Store, centre: &str) -> TestResult {
    let id = "namedplace:home";
    store.append(
        id,
        place_keys::TYPE,
        Some(place_keys::TYPE_NAMED_PLACE),
        None,
    )?;
    store.append(id, place_keys::NAME, Some("Home"), None)?;
    store.append(id, place_keys::CENTRE, Some(centre), None)?;
    store.append(id, place_keys::RADIUS, Some("100"), None)?;
    Ok(())
}

/// The Dentist Note with a Capture Context as a phone keeps it: at Home,
/// shared from WhatsApp during a call with two Bluetooth devices; its
/// Amend captured elsewhere during another event. The Note's id.
fn noted(store: &Store) -> Result<String, Box<dyn std::error::Error>> {
    let notes = Notes::new(store);
    let note = notes.capture(TEXT, None, None)?;
    home(store, "52.52,13.405")?;
    let at = |field: &str, value: &str| store.append(&note.id, field, Some(value), None);
    at(note_keys::LOCATION, "52.52,13.405")?;
    at(note_keys::SOURCE_PACKAGE, "com.whatsapp")?;
    at(note_keys::SOURCE_LABEL, "WhatsApp")?;
    at(note_keys::CALENDAR, r#"["Team call"]"#)?;
    at(note_keys::BLUETOOTH, r#"["Car kit","Headphones"]"#)?;
    let amend = notes.amend(&note.id, AMEND)?;
    store.append(
        &amend.id,
        note_keys::LOCATION,
        Some("48.13724,11.57561"),
        None,
    )?;
    store.append(&amend.id, note_keys::CALENDAR, Some(r#"["Dentist"]"#), None)?;
    Ok(note.id)
}

/// The Note's Capture Context and the Amend's, each line as it shows.
const NOTE_CONTEXT: [&str; 4] = [
    "Home (52.5200, 13.4050)",
    "Shared from WhatsApp",
    "During: Team call",
    "Connected: Car kit, Headphones",
];
const AMEND_CONTEXT: [&str; 2] = ["48.1372, 11.5756", "During: Dentist"];

#[test]
fn a_full_card_shows_the_capture_context_of_the_note_and_above_each_section() -> TestResult {
    let store = store()?;
    noted(&store)?;
    let harness = window(store, Selection::All);
    let dentist = card(&harness, TEXT).rect();
    for line in NOTE_CONTEXT.iter().chain(&AMEND_CONTEXT) {
        let shown = harness.get_by_label(line).rect();
        assert!(dentist.contains_rect(shown), "{line}");
    }
    // The Note's in its header, above its text; the Amend's above it.
    // The card is labelled with the text too.
    let text = harness.get_by_role_and_label(Role::Label, TEXT).rect();
    let amend = harness.get_by_label(AMEND).rect();
    for line in NOTE_CONTEXT {
        assert!(
            harness.get_by_label(line).rect().bottom() <= text.top(),
            "{line}"
        );
    }
    for line in AMEND_CONTEXT {
        let shown = harness.get_by_label(line).rect();
        assert!(text.bottom() <= shown.top(), "{line}");
        assert!(shown.bottom() <= amend.top(), "{line}");
    }
    Ok(())
}

#[test]
fn a_compact_card_shows_where_and_the_editor_shows_the_whole_capture_context() -> TestResult {
    let store = store()?;
    let id = noted(&store)?;
    store.set_local_setting("compactNotes:allNotes", "true")?;
    let mut harness = window(store, Selection::All);
    // Compact: where, by its Named Place; the rest in the editor.
    assert!(shows(&harness, "Home"));
    for line in NOTE_CONTEXT.iter().chain(&AMEND_CONTEXT) {
        assert!(!shows(&harness, line), "{line}");
    }
    edit(&harness, TEXT).click();
    harness.run();
    assert_eq!(harness.state().selected_note(), Some(id.as_str()));
    let card = card(&harness, TEXT).rect();
    for line in NOTE_CONTEXT.iter().chain(&AMEND_CONTEXT) {
        let shown = harness.get_by_label(line).rect();
        assert!(!card.contains_rect(shown), "{line} in the editor");
    }
    Ok(())
}

/// The photo with the largest place in the window: the full-screen one
/// while it shows.
fn largest_photo<'a>(harness: &'a Harness<'_, App>) -> egui_kittest::Node<'a> {
    harness
        .query_all_by_role_and_label(Role::Image, "Photo")
        .max_by(|a, b| a.rect().area().total_cmp(&b.rect().area()))
        .unwrap_or_else(|| panic!("no photo"))
}

#[test]
fn a_click_on_a_photo_shows_it_full_screen_to_zoom_until_closed() -> TestResult {
    let store = store()?;
    notes(&store)?;
    let photo = Notes::new(&store)
        .unsorted()?
        .into_iter()
        .find(|n| n.text == TEXT)
        .map(|n| n.photos[0].clone())
        .ok_or("no photo")?;
    let mut harness = window(store, Selection::All);
    harness.get_by_role_and_label(Role::Image, "Photo").click();
    harness.run();
    // Full screen, fitting the window; the card neither folds nor opens.
    assert_eq!(harness.state().viewed_photo(), Some((photo.as_str(), 1.0)));
    assert_eq!(harness.state().selected_note(), None);
    let full = largest_photo(&harness).rect();
    assert!(
        full.width() >= 1279.0 && full.height() >= 1399.0,
        "{full:?}"
    );
    // A pinch zooms in, and so does the wheel.
    harness.hover_at(full.center());
    harness.run();
    harness.event(egui::Event::Zoom(2.0));
    harness.run();
    let zoom = harness.state().viewed_photo().map_or(0.0, |(_, z)| z);
    assert!((zoom - 2.0).abs() < 0.01, "{zoom}");
    harness.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta: egui::vec2(0.0, 120.0),
        phase: egui::TouchPhase::Move,
        modifiers: egui::Modifiers::NONE,
    });
    // The wheel's step is spread over a few frames.
    harness.run_steps(30);
    let wheeled = harness.state().viewed_photo().map_or(0.0, |(_, z)| z);
    assert!(wheeled > zoom, "{wheeled}");
    // Zoomed in, a drag moves it; the list behind stays.
    harness.drag_at(full.center());
    harness.run();
    harness.hover_at(full.center() + egui::vec2(-200.0, 0.0));
    harness.run();
    harness.drop_at(full.center() + egui::vec2(-200.0, 0.0));
    harness.run();
    assert!(harness.state().viewed_photo().is_some());
    // Esc closes it, with the list as it was.
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert_eq!(harness.state().viewed_photo(), None);
    assert_eq!(harness.state().selection(), &Selection::All);
    assert_eq!(harness.state().selected_note(), None);
    // So do Alt+Left, the mouse's back button and the close button,
    // going nowhere else; a photo shows fitting the window again.
    let open = |harness: &mut Harness<'_, App>| {
        harness.get_by_role_and_label(Role::Image, "Photo").click();
        harness.run();
        assert_eq!(harness.state().viewed_photo(), Some((photo.as_str(), 1.0)));
    };
    open(&mut harness);
    harness.key_press_modifiers(egui::Modifiers::ALT, egui::Key::ArrowLeft);
    harness.run();
    assert_eq!(harness.state().viewed_photo(), None);
    assert_eq!(harness.state().selection(), &Selection::All);
    open(&mut harness);
    let at = full.center();
    for pressed in [true, false] {
        harness.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Extra1,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
    }
    harness.run();
    assert_eq!(harness.state().viewed_photo(), None);
    assert_eq!(harness.state().selection(), &Selection::All);
    open(&mut harness);
    press(&mut harness, "Close");
    assert_eq!(harness.state().viewed_photo(), None);
    Ok(())
}

#[test]
fn a_click_on_a_photo_in_the_editor_shows_it_full_screen() -> TestResult {
    let store = store()?;
    notes(&store)?;
    let mut harness = window(store, Selection::All);
    edit(&harness, TEXT).click();
    harness.run();
    // The editor's photo lies on the right.
    let editor = harness
        .query_all_by_role_and_label(Role::Image, "Photo")
        .max_by(|a, b| a.rect().left().total_cmp(&b.rect().left()))
        .ok_or("no photo in the editor")?;
    assert!(editor.rect().left() > 640.0, "{:?}", editor.rect());
    editor.click();
    harness.run();
    assert!(harness.state().viewed_photo().is_some());
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert_eq!(harness.state().viewed_photo(), None);
    Ok(())
}

#[test]
fn a_photo_of_extreme_proportions_shows_full_screen_decoded_bounded_once() -> TestResult {
    let store = store()?;
    Notes::new(&store).capture_with("Tall", None, None, &[png(12, 34_000)], &[])?;
    let mut harness = window(store, Selection::All);
    harness.get_by_role_and_label(Role::Image, "Photo").click();
    harness.run();
    assert!(harness.state().viewed_photo().is_some());
    let large = |harness: &Harness<'_, App>| -> Vec<[usize; 2]> {
        let manager = harness.ctx.tex_manager();
        let manager = manager.read();
        manager
            .allocated()
            .filter(|(_, meta)| meta.name.starts_with("photo-large:"))
            .map(|(_, meta)| meta.size)
            .collect()
    };
    let most = harness.ctx.input(|i| i.max_texture_side).min(4096);
    let decoded = large(&harness);
    assert_eq!(decoded.len(), 1);
    for size in &decoded {
        assert!(size[0] <= most && size[1] <= most, "{size:?}");
        assert!(size[0] >= 1 && size[1] >= 1, "{size:?}");
    }
    // Fitting the window, its own proportions; zoomed in far enough for
    // its short side to fill it.
    let full = largest_photo(&harness).rect();
    assert!(full.height() >= 1399.0, "{full:?}");
    harness.hover_at(full.center());
    harness.run();
    for _ in 0..3 {
        harness.event(egui::Event::Zoom(10.0));
        harness.run();
    }
    let zoom = harness.state().viewed_photo().map_or(0.0, |(_, z)| z);
    assert!(zoom > 6.0, "{zoom}");
    // Decoded once: more frames make no more.
    harness.run();
    harness.run();
    assert_eq!(large(&harness).len(), 1);
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert_eq!(harness.state().viewed_photo(), None);
    Ok(())
}
