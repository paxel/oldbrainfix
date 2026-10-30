//! Boxes in the window (ADR 0035): a ☐, ✅ or ❌ anywhere in a Note's
//! text, on its card and in its editor, opens Check, Fail and Open, the
//! current state left out, and the one picked is a tick beside the text,
//! never an edit of it; a ticked box offers its History; a ☐ typed or put
//! in by the format bar, also in the middle of a word, is a box once
//! saved; Backspace takes a box whole; Show ticks says who ticked a box
//! and when; a Member ticks every Note's and Section's boxes in a Shared
//! Area; a Section's boxes tick though its text has no edit.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, Duration, Utc};
use egui::Key;
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::lan::{Joining, answer_sync_inviting, apply_answer_as, sync_request_as};
use oldbrainfix_core::{
    AreaRole, BoxState, Boxes, Invite, Notes, OwnProfile, PairedDevice, Places, SigningKey, Store,
    text_boxes,
};
use oldbrainfix_gui::l10n::L10n;
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A store on a clock that moves a second per read, with its own
/// Profile named `name`.
fn device(seed: u8, name: &str) -> Result<Store, Box<dyn std::error::Error>> {
    let start = DateTime::parse_from_rfc3339("2026-03-02T11:00:00Z")?.with_timezone(&Utc);
    let ticks = Arc::new(AtomicI64::new(0));
    let clock = Box::new(move || start + Duration::seconds(ticks.fetch_add(1, Ordering::SeqCst)));
    let store = Store::in_memory(clock, move || Ok(SigningKey::from_seed([seed; 32])))?;
    store.set_local_setting("author", name)?;
    OwnProfile::new(&store).apply_defaults(name, None)?;
    Ok(store)
}

/// A store with a Note of `text` in the Area "Work": the store and the
/// Note's id.
fn with_note(text: &str) -> Result<(Arc<Store>, String), Box<dyn std::error::Error>> {
    let store = device(3, "Desk")?;
    let work = Places::new(&store).add_area("Work")?;
    let note = Notes::new(&store).capture(text, Some(&work.id), None)?;
    Ok((Arc::new(store), note.id))
}

/// The window over `store` in English listing All Notes in full, the
/// Note `note` open in its editor.
fn window(store: &Arc<Store>, note: Option<&str>) -> Harness<'static, App> {
    let mut app = App::new(store.clone(), "en");
    app.select(Selection::All);
    if let Some(note) = note {
        app.select_note(note);
    }
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 1600.0))
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

fn shows(harness: &Harness<'_, App>, label: &str) -> bool {
    harness.query_all_by_label(label).next().is_some()
}

/// How many boxes read as `state` can be clicked.
fn buttons(harness: &Harness<'_, App>, state: &str) -> usize {
    harness
        .query_all_by_role_and_label(Role::Button, state)
        .count()
}

/// How many boxes read as `state` are only there to read.
fn read_only(harness: &Harness<'_, App>, state: &str) -> usize {
    harness
        .query_all_by_role_and_label(Role::Label, state)
        .count()
}

/// Clicks the last widget labelled `label`: a menu's over the window.
fn click(harness: &mut Harness<'_, App>, label: &str) -> TestResult {
    harness
        .query_all_by_label(label)
        .last()
        .ok_or(format!("{label:?}"))?
        .click_accesskit();
    harness.run();
    Ok(())
}

/// Clicks the first box read as `state`.
fn click_box(harness: &mut Harness<'_, App>, state: &str) -> TestResult {
    harness
        .query_all_by_role_and_label(Role::Button, state)
        .next()
        .ok_or(format!("{state:?}"))?
        .click_accesskit();
    harness.run();
    Ok(())
}

/// The text of `note` as it is stored, its boxes with their ids.
fn stored(store: &Store, note: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(Notes::new(store).by_id(note)?.ok_or("the Note")?.text)
}

/// The text of `note` as it shows: each box in the glyph of its state.
fn shown(store: &Store, note: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(Notes::new(store)
        .by_id(note)?
        .ok_or("the Note")?
        .shown_text())
}

/// The text the editor's field holds: the field of many lines with a
/// text, as the empty capture field above the cards is one too.
fn field(harness: &Harness<'_, App>) -> Option<String> {
    harness
        .query_by(|n| {
            n.role() == Role::MultilineTextInput && n.value().is_some_and(|v| !v.is_empty())
        })
        .and_then(|n| n.value())
}

/// Gives the editor's field the keyboard, its cursor at the end.
fn focus_field(harness: &mut Harness<'_, App>) -> TestResult {
    harness
        .query_all_by(|n| {
            n.role() == Role::MultilineTextInput && n.value().is_some_and(|v| !v.is_empty())
        })
        .next()
        .ok_or("the field")?
        .focus();
    harness.run();
    Ok(())
}

/// Leaves the editor, which saves what was typed.
fn leave(harness: &mut Harness<'_, App>) {
    harness.state_mut().select(Selection::Unsorted);
    harness.run();
}

#[test]
fn a_cards_box_offers_the_other_two_states_and_ticks_beside_the_text() -> TestResult {
    let (store, note) = with_note("Release\n✅ v2 supported, docs ☐ later")?;
    let before = stored(&store, &note)?;
    assert_eq!(text_boxes::find(&before).len(), 2);
    let mut harness = window(&store, None);
    assert_eq!(buttons(&harness, "Checked"), 1);
    assert_eq!(buttons(&harness, "Open"), 1);
    click_box(&mut harness, "Checked")?;
    assert!(shows(&harness, "❌ Fail"));
    assert!(shows(&harness, "☐ Open"));
    assert!(!shows(&harness, "✅ Check"));
    // Never ticked: no History yet.
    assert!(!shows(&harness, "History"));
    click(&mut harness, "❌ Fail")?;
    // The text stays as it is; the tick is an entry beside it.
    assert_eq!(stored(&store, &note)?, before);
    assert_eq!(
        shown(&store, &note)?,
        "Release\n❌ v2 supported, docs ☐ later"
    );
    assert_eq!(buttons(&harness, "Failed"), 1);
    // The box in the middle of the line is a box as well.
    click_box(&mut harness, "Open")?;
    click(&mut harness, "✅ Check")?;
    assert_eq!(
        shown(&store, &note)?,
        "Release\n❌ v2 supported, docs ✅ later"
    );
    Ok(())
}

#[test]
fn a_compact_card_offers_the_box_of_its_first_line() -> TestResult {
    let (store, note) = with_note("\n☐ Call Ann\n✅ later")?;
    let mut harness = window(&store, None);
    let compact = L10n::new("en").show_notes_compact();
    click(&mut harness, compact)?;
    // Only the first line shows, and the box is its own.
    assert_eq!(buttons(&harness, "Checked"), 0);
    click_box(&mut harness, "Open")?;
    click(&mut harness, "✅ Check")?;
    assert_eq!(shown(&store, &note)?, "\n✅ Call Ann\n✅ later");
    Ok(())
}

#[test]
fn a_glyph_in_another_emoji_is_no_box() -> TestResult {
    let (store, _) = with_note("Joined: ✅\u{200D}🔥 and ☑ other box")?;
    let harness = window(&store, None);
    for state in ["Checked", "Failed", "Open"] {
        assert_eq!(buttons(&harness, state), 0, "{state}");
        assert_eq!(read_only(&harness, state), 0, "{state}");
    }
    Ok(())
}

#[test]
fn a_box_in_the_editors_field_ticks_and_the_field_keeps_its_source() -> TestResult {
    let (store, note) = with_note("Release\n✅ v2 supported")?;
    let before = stored(&store, &note)?;
    let mut harness = window(&store, Some(&note));
    // On the card and in the field.
    assert_eq!(buttons(&harness, "Checked"), 2);
    let editor = harness
        .query_all_by_role_and_label(Role::Button, "Checked")
        .last()
        .ok_or("the field's box")?;
    editor.click_accesskit();
    harness.run();
    click(&mut harness, "☐ Open")?;
    assert_eq!(stored(&store, &note)?, before);
    assert_eq!(shown(&store, &note)?, "Release\n☐ v2 supported");
    // The field holds the text as stored, and its box shows the tick.
    assert_eq!(field(&harness), Some(before));
    assert_eq!(buttons(&harness, "Open"), 2);
    assert_eq!(buttons(&harness, "Checked"), 0);
    Ok(())
}

#[test]
fn the_format_bar_puts_a_box_in_at_the_cursor_mid_word() -> TestResult {
    let (store, note) = with_note("Passport")?;
    let mut harness = window(&store, Some(&note));
    focus_field(&mut harness)?;
    for _ in 0..4 {
        harness.key_press(Key::ArrowLeft);
    }
    harness.run();
    click(&mut harness, "Put in ☐")?;
    assert_eq!(field(&harness).as_deref(), Some("Pass☐port"));
    leave(&mut harness);
    let text = stored(&store, &note)?;
    assert_eq!(text_boxes::find(&text).len(), 1);
    assert_eq!(shown(&store, &note)?, "Pass☐port");
    Ok(())
}

#[test]
fn a_typed_glyph_becomes_a_box_on_save_and_the_field_takes_its_id() -> TestResult {
    let (store, note) = with_note("Boiler")?;
    let mut harness = window(&store, Some(&note));
    focus_field(&mut harness)?;
    harness.event(egui::Event::Text(" ✅ serviced".into()));
    harness.run();
    // Ctrl+Enter saves and keeps the field.
    harness.key_press_modifiers(egui::Modifiers::COMMAND, Key::Enter);
    harness.run();
    let text = stored(&store, &note)?;
    let boxes = text_boxes::find(&text);
    assert_eq!(boxes.len(), 1);
    assert_eq!(boxes.first().map(|b| b.state), Some(BoxState::Done));
    assert_eq!(field(&harness), Some(text.clone()));
    // Typed on, the cursor after the box: the id stays the same.
    harness.event(egui::Event::Text("!".into()));
    harness.run();
    leave(&mut harness);
    let again = stored(&store, &note)?;
    assert_eq!(text_boxes::strip(&again), "Boiler ✅ serviced!");
    assert_eq!(
        text_boxes::find(&again).first().map(|b| b.id.clone()),
        boxes.first().map(|b| b.id.clone())
    );
    Ok(())
}

#[test]
fn backspace_after_a_box_takes_it_whole() -> TestResult {
    let (store, note) = with_note("Pack ☐")?;
    let mut harness = window(&store, Some(&note));
    focus_field(&mut harness)?;
    harness.key_press(Key::Backspace);
    harness.run();
    assert_eq!(field(&harness).as_deref(), Some("Pack "));
    leave(&mut harness);
    assert_eq!(stored(&store, &note)?, "Pack");
    Ok(())
}

#[test]
fn a_ticked_box_shows_its_history() -> TestResult {
    let (store, note) = with_note("☐ Call Ann")?;
    let id = text_boxes::find(&stored(&store, &note)?)
        .first()
        .map(|b| b.id.clone())
        .ok_or("a box")?;
    let boxes = Boxes::new(&store);
    boxes.tick(&note, &id, BoxState::Done)?;
    boxes.tick(&note, &id, BoxState::Failed)?;
    let mut harness = window(&store, None);
    click_box(&mut harness, "Failed")?;
    click(&mut harness, "History")?;
    assert!(shows(
        &harness,
        "Every change of this box, the oldest first."
    ));
    assert_eq!(read_only(&harness, "Checked"), 1);
    assert_eq!(harness.query_all_by_label_contains("You, ").count(), 2);
    Ok(())
}

#[test]
fn show_ticks_says_who_ticked_each_box_and_is_kept_per_note() -> TestResult {
    let (store, note) = with_note("☐ Call Ann")?;
    let mut harness = window(&store, None);
    click_box(&mut harness, "Open")?;
    click(&mut harness, "✅ Check")?;
    assert!(
        harness
            .query_all_by_label_contains("You, ")
            .next()
            .is_none()
    );
    let more = format!("{}: ", L10n::new("en").more());
    let menu = harness
        .query_all_by(move |n| {
            n.role() == Role::Button && n.label().is_some_and(|l| l.starts_with(&more))
        })
        .next()
        .ok_or("the card's ⋮")?;
    menu.click_accesskit();
    harness.run();
    click(&mut harness, "Show ticks")?;
    assert!(
        harness
            .query_all_by_label_contains("You, ")
            .next()
            .is_some()
    );
    assert_eq!(
        store
            .local_setting(&format!("showTicks:{note}"))?
            .as_deref(),
        Some("true")
    );
    Ok(())
}

#[test]
fn a_sections_box_ticks_though_its_text_has_no_edit() -> TestResult {
    let (store, note) = with_note("Trip")?;
    let section = Notes::new(&store).amend(&note, "☐ Passport")?;
    let mut harness = window(&store, Some(&note));
    // On the card and in the editor.
    assert_eq!(buttons(&harness, "Open"), 2);
    click_box(&mut harness, "Open")?;
    click(&mut harness, "✅ Check")?;
    let shown = Notes::new(&store)
        .section_by_id(&section.id)?
        .ok_or("the Section")?;
    assert_eq!(shown.shown_text(), "✅ Passport");
    assert_eq!(text_boxes::strip(&shown.text), "☐ Passport");
    Ok(())
}

/// Ann's store, Ben's, Ann's Note's id, its Section's id and Ben's
/// Note's id.
type Shared = (Arc<Store>, Arc<Store>, String, String, String);

/// Ann's Household with her Note "✅ Boiler serviced" in it and its
/// Section "☐ Filter", shared with Ben as a Member, and his own Note
/// "☐ Buy filters" there.
fn shared() -> Result<Shared, Box<dyn std::error::Error>> {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let house = Places::new(&ann).add_area("Household")?;
    let notes = Notes::new(&ann);
    let note = notes.capture("✅ Boiler serviced", Some(&house.id), None)?;
    let section = notes.amend(&note.id, "☐ Filter")?;
    let area = ann.share(&house.id)?;
    let request = sync_request_as(
        &ben,
        &ann.version_vector()?,
        "Phone",
        &ann.device_id()?,
        "",
        Joining::Invited,
    )?;
    let invite = Invite {
        area,
        role: AreaRole::Member,
    };
    let (answer, _) =
        answer_sync_inviting(&ann, &request, "Desk", Some(&invite))?.ok_or("an answer")?;
    apply_answer_as(&ben, &answer, true, None)?;
    for (store, peer) in [(&ann, &ben), (&ben, &ann)] {
        store.remember_device(&PairedDevice {
            id: peer.device_id()?,
            name: String::new(),
            fingerprint: String::new(),
            host: None,
            port: None,
            last_sync: store.now(),
            shared: true,
        })?;
    }
    let own = Notes::new(&ben).capture("☐ Buy filters", Some(&house.id), None)?;
    Ok((Arc::new(ann), Arc::new(ben), note.id, section.id, own.id))
}

#[test]
fn a_member_ticks_the_boxes_of_every_note_and_section() -> TestResult {
    let (_ann, ben, note, section, own) = shared()?;
    let before = stored(&ben, &note)?;
    let mut harness = window(&ben, Some(&note));
    // Ann's Note: no edit of its text, but its boxes tick.
    assert!(!shows(&harness, "Put in ☐"));
    assert_eq!(buttons(&harness, "Checked"), 2);
    click_box(&mut harness, "Checked")?;
    click(&mut harness, "❌ Fail")?;
    assert_eq!(stored(&ben, &note)?, before);
    assert_eq!(shown(&ben, &note)?, "❌ Boiler serviced");
    // Her Section's box, and his own Note's, one click each.
    for _ in 0..2 {
        click_box(&mut harness, "Open")?;
        click(&mut harness, "✅ Check")?;
    }
    assert_eq!(buttons(&harness, "Open"), 0);
    let section = Notes::new(&ben)
        .section_by_id(&section)?
        .ok_or("the Section")?;
    assert_eq!(section.shown_text(), "✅ Filter");
    assert_eq!(shown(&ben, &own)?, "✅ Buy filters");
    Ok(())
}
