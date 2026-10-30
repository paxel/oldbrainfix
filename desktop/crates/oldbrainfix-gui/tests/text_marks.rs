//! Status marks in the window (#125, ADR 0031): a ☐, ✅ or ❌ at the
//! start of a line of a Note's text, on its card and in its editor,
//! opens Check, Fail and Open, the current state left out, and the one
//! picked changes the text; a mark in the middle of a line is plain
//! text; the editor puts ☐ at the start of the cursor's line; and a
//! Member changes the marks of their own Notes only.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, Duration, Utc};
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::lan::{Joining, answer_sync_inviting, apply_answer_as, sync_request_as};
use oldbrainfix_core::{
    AreaRole, Invite, Notes, OwnProfile, PairedDevice, Places, SigningKey, Store,
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

/// How many marks read as `state` can be clicked.
fn buttons(harness: &Harness<'_, App>, state: &str) -> usize {
    harness
        .query_all_by_role_and_label(Role::Button, state)
        .count()
}

/// How many marks read as `state` are only there to read.
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

/// Clicks the last mark read as `state`.
fn click_mark(harness: &mut Harness<'_, App>, state: &str) -> TestResult {
    harness
        .query_all_by_role_and_label(Role::Button, state)
        .last()
        .ok_or(format!("{state:?}"))?
        .click_accesskit();
    harness.run();
    Ok(())
}

/// The text of `note` as it is stored.
fn stored(store: &Store, note: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(Notes::new(store).by_id(note)?.ok_or("the Note")?.text)
}

/// The text the editor's field holds.
fn field(harness: &Harness<'_, App>) -> Option<String> {
    harness
        .query_by(|n| n.role() == Role::MultilineTextInput)
        .and_then(|n| n.value())
}

#[test]
fn a_cards_mark_offers_the_other_two_states_and_changes_the_text() -> TestResult {
    let (store, note) = with_note("Release\n✅ v2 supported\n  - ☐ docs")?;
    let mut harness = window(&store, None);
    assert_eq!(buttons(&harness, "Checked"), 1);
    assert_eq!(buttons(&harness, "Open"), 1);
    click_mark(&mut harness, "Checked")?;
    assert!(shows(&harness, "❌ Fail"));
    assert!(shows(&harness, "☐ Open"));
    assert!(!shows(&harness, "✅ Check"));
    click(&mut harness, "❌ Fail")?;
    assert_eq!(
        stored(&store, &note)?,
        "Release\n❌ v2 supported\n  - ☐ docs"
    );
    assert_eq!(buttons(&harness, "Failed"), 1);
    // The indented bullet's mark, the line counted in the whole text.
    click_mark(&mut harness, "Open")?;
    assert!(!shows(&harness, "☐ Open"));
    click(&mut harness, "✅ Check")?;
    assert_eq!(
        stored(&store, &note)?,
        "Release\n❌ v2 supported\n  - ✅ docs"
    );
    Ok(())
}

#[test]
fn a_compact_card_offers_the_mark_of_its_first_line() -> TestResult {
    let (store, note) = with_note("\n☐ Call Ann\n✅ later")?;
    let mut harness = window(&store, None);
    let compact = L10n::new("en").show_notes_compact();
    click(&mut harness, compact)?;
    // Only the first line shows, and the mark is its own.
    assert_eq!(buttons(&harness, "Checked"), 0);
    click_mark(&mut harness, "Open")?;
    click(&mut harness, "✅ Check")?;
    assert_eq!(stored(&store, &note)?, "✅ Call Ann\n✅ later");
    Ok(())
}

#[test]
fn an_emoji_in_the_middle_of_a_line_is_plain_text() -> TestResult {
    let (store, _) = with_note("v2 at customer: ✅\n-❌ no space\n1. ☐ numbered\n☑ other box")?;
    let harness = window(&store, None);
    for state in ["Checked", "Failed", "Open"] {
        assert_eq!(buttons(&harness, state), 0, "{state}");
        assert_eq!(read_only(&harness, state), 0, "{state}");
    }
    Ok(())
}

#[test]
fn a_mark_in_the_editors_field_changes_and_is_saved() -> TestResult {
    let (store, note) = with_note("Release\n✅ v2 supported")?;
    let mut harness = window(&store, Some(&note));
    // On the card and in the field.
    assert_eq!(buttons(&harness, "Checked"), 2);
    click_mark(&mut harness, "Checked")?;
    click(&mut harness, "☐ Open")?;
    assert_eq!(stored(&store, &note)?, "Release\n☐ v2 supported");
    assert_eq!(field(&harness).as_deref(), Some("Release\n☐ v2 supported"));
    Ok(())
}

#[test]
fn the_editor_puts_an_open_mark_at_the_start_of_the_cursors_line() -> TestResult {
    let (store, note) = with_note("Pack\nPassport")?;
    let mut harness = window(&store, Some(&note));
    let insert = "Put ☐ at the start of the line";
    // A field not typed into has its cursor at the start.
    click(&mut harness, insert)?;
    assert_eq!(field(&harness).as_deref(), Some("☐ Pack\nPassport"));
    // A line with a mark gets no second one.
    click(&mut harness, insert)?;
    assert_eq!(field(&harness).as_deref(), Some("☐ Pack\nPassport"));
    // The field has the keyboard again; leaving it saves.
    harness.state_mut().select(Selection::Unsorted);
    harness.run();
    assert_eq!(stored(&store, &note)?, "☐ Pack\nPassport");
    Ok(())
}

/// Ann's store, Ben's, Ann's Note's id and Ben's.
type Shared = (Arc<Store>, Arc<Store>, String, String);

/// Ann's Household with her Note "✅ Boiler serviced" in it, shared with
/// Ben as a Member, and his own Note "☐ Buy filters" there.
fn shared() -> Result<Shared, Box<dyn std::error::Error>> {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let house = Places::new(&ann).add_area("Household")?;
    let note = Notes::new(&ann).capture("✅ Boiler serviced", Some(&house.id), None)?;
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
    Ok((Arc::new(ann), Arc::new(ben), note.id, own.id))
}

#[test]
fn a_member_changes_the_marks_of_their_own_notes_only() -> TestResult {
    let (_ann, ben, note, own) = shared()?;
    let mut harness = window(&ben, Some(&note));
    // Ann's Note: read on the card and in the editor, never a button.
    assert_eq!(buttons(&harness, "Checked"), 0);
    assert_eq!(read_only(&harness, "Checked"), 2);
    assert!(!shows(&harness, "Put ☐ at the start of the line"));
    // His own is his to change.
    click_mark(&mut harness, "Open")?;
    click(&mut harness, "✅ Check")?;
    assert_eq!(stored(&ben, &own)?, "✅ Buy filters");
    assert_eq!(stored(&ben, &note)?, "✅ Boiler serviced");
    Ok(())
}
