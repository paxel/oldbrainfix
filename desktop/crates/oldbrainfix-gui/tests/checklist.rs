//! Checklists in the window (#120, ADR 0030): the kind badge of a Note's
//! editor (#188) and its card's ⋮ turn its lines into Lines, one
//! indented under the Line above; the editor checks them, moves, indents
//! and deletes them after asking once, unchecks all and goes back to a
//! plain Note; the Lines are typed in place as one block of text (#141,
//! #184): no "Add Line", Enter at the end of a Line starts an empty one
//! and in its middle splits it, Backspace at the start joins, Tab and
//! Shift+Tab move a Line into a group and out, and leaving keeps what
//! was typed and drops the empty ones; the frame of a group holds the
//! group's own Line; full cards check them in place, compact cards count
//! them; a Member checks and adds Lines but changes only their own, an
//! empty Line waiting for him under the others'; a Line Conflict is
//! offered on the Conflicts view; and a Sent Item's preview shows its
//! Lines and what an update changes.

use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, Duration, Utc};
use egui::accesskit::{Role, Toggled};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::lan::{Joining, answer_sync_inviting, apply_answer_as, sync_request_as};
use oldbrainfix_core::{
    AreaRole, Checklist, Checklists, ImportReport, Invite, Notes, OwnProfile, PairedDevice, Places,
    SigningKey, Store,
};
use oldbrainfix_gui::l10n::L10n;
use oldbrainfix_gui::{App, Page, Selection};

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

/// The text of the Note every test packs with.
const PACKING: &str = "Packing\nPassport\n  Charger\nTicket";

/// A store with the Note [`PACKING`] in the Area "Travel": the store and
/// the Note's id.
fn with_note() -> Result<(Arc<Store>, String), Box<dyn std::error::Error>> {
    let store = device(3, "Desk")?;
    let travel = Places::new(&store).add_area("Travel")?;
    let note = Notes::new(&store).capture(PACKING, Some(&travel.id), None)?;
    Ok((Arc::new(store), note.id))
}

/// The window over `store` in English listing All Notes, the Note
/// `note` open in its editor.
fn window(store: &Arc<Store>, note: Option<&str>) -> Harness<'static, App> {
    let mut app = App::new(store.clone(), "en");
    app.select(Selection::All);
    if let Some(note) = note {
        app.select_note(note);
    }
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 2400.0))
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

fn shows_part(harness: &Harness<'_, App>, part: &str) -> bool {
    harness.query_all_by_label_contains(part).next().is_some()
}

/// Clicks the last widget labelled `label`: a menu's or a dialog's, over
/// the window.
fn click(harness: &mut Harness<'_, App>, label: &str) -> TestResult {
    harness
        .query_all_by_label(label)
        .last()
        .ok_or(format!("{label:?}"))?
        .click_accesskit();
    harness.run();
    Ok(())
}

/// The checkboxes of the Line `text`, the editor's first where both the
/// card and the editor show it.
fn boxes<'a>(harness: &'a Harness<'_, App>, text: &'a str) -> Vec<egui_kittest::Node<'a>> {
    harness
        .query_all_by_role_and_label(Role::CheckBox, text)
        .collect()
}

/// Whether the last checkbox of the Line `text` shows checked.
fn checked(harness: &Harness<'_, App>, text: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let node = boxes(harness, text).pop().ok_or(format!("{text:?}"))?;
    Ok(node.accesskit_node().toggled() == Some(Toggled::True))
}

/// Clicks the last checkbox of the Line `text`.
fn tick(harness: &mut Harness<'_, App>, text: &str) -> TestResult {
    boxes(harness, text)
        .pop()
        .ok_or(format!("{text:?}"))?
        .click_accesskit();
    harness.run();
    Ok(())
}

/// The Checklist of `note` as the core reads it.
fn list(harness: &Harness<'_, App>, note: &str) -> Result<Checklist, Box<dyn std::error::Error>> {
    Ok(Checklists::new(harness.state().store())
        .by_note(note)?
        .ok_or("a Checklist")?)
}

/// The texts of the Lines of `note`, a Line of a group after "  ".
fn texts(
    harness: &Harness<'_, App>,
    note: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    Ok(list(harness, note)?
        .lines
        .iter()
        .map(|l| {
            let indent = if l.in_group() { "  " } else { "" };
            format!("{indent}{}", l.text)
        })
        .collect())
}

/// Picks the radio button `label` of the open small editor (#188).
fn pick(harness: &mut Harness<'_, App>, label: &str) {
    harness
        .get_by_role_and_label(Role::RadioButton, label)
        .click_accesskit();
    harness.run();
}

/// Opens the ⋮ of the Line `text` in the editor and clicks `step`.
fn line_step(harness: &mut Harness<'_, App>, text: &str, step: &str) -> TestResult {
    click(harness, &format!("More: {text}"))?;
    click(harness, step)
}

#[test]
fn make_it_a_checklist_in_the_editor_turns_the_lines_into_lines() -> TestResult {
    let (store, note) = with_note()?;
    let mut harness = window(&store, Some(&note));
    assert!(!shows(&harness, "Checklist"));
    click(&mut harness, "Kind: Plain Note")?;
    pick(&mut harness, "Checklist");
    assert_eq!(texts(&harness, &note)?, ["Passport", "  Charger", "Ticket"]);
    let now = Notes::new(harness.state().store())
        .by_id(&note)?
        .ok_or("note")?;
    assert_eq!(now.text, "Packing");
    // The name holds the title now, so leaving it writes nothing old
    // (#188).
    assert_eq!(
        harness
            .get_by_role_and_label(Role::TextInput, "Name")
            .value()
            .as_deref(),
        Some("Packing")
    );
    assert!(shows(&harness, "Checklist"));
    assert!(!shows(&harness, "Make it a Checklist"));
    // A Checklist is no Tally.
    assert!(!shows(&harness, "Make it a Tally"));
    // The group lies in one frame with its parent, further in.
    let x = |text: &str| -> Result<f32, Box<dyn std::error::Error>> {
        let node = boxes(&harness, text).pop().ok_or(format!("{text:?}"))?;
        Ok(node.rect().left())
    };
    assert!(x("Charger")? > x("Passport")? + 10.0);
    assert!(x("Passport")? > x("Ticket")?);
    Ok(())
}

#[test]
fn checks_follow_the_group_and_uncheck_all_readies_it_again() -> TestResult {
    let (store, note) = with_note()?;
    Checklists::new(&store).make(&note)?;
    let mut harness = window(&store, Some(&note));
    assert!(!shows(&harness, "Uncheck all"));
    // The last Line of a group checks its parent.
    tick(&mut harness, "Charger")?;
    assert!(checked(&harness, "Charger")?);
    assert!(checked(&harness, "Passport")?);
    assert!(!checked(&harness, "Ticket")?);
    assert_eq!(list(&harness, &note)?.checked_count(), 2);
    // Unchecking the parent unchecks its group.
    tick(&mut harness, "Passport")?;
    assert!(!checked(&harness, "Charger")?);
    tick(&mut harness, "Ticket")?;
    // A checked Line stays where it is.
    assert_eq!(texts(&harness, &note)?, ["Passport", "  Charger", "Ticket"]);
    click(&mut harness, "Uncheck all")?;
    assert_eq!(list(&harness, &note)?.checked_count(), 0);
    assert!(!shows(&harness, "Uncheck all"));
    Ok(())
}

/// The field a Line is typed in, holding `value`.
fn line_field<'a>(
    harness: &'a Harness<'_, App>,
    value: &'a str,
) -> Result<egui_kittest::Node<'a>, Box<dyn std::error::Error>> {
    Ok(harness
        .query_all_by(|n| {
            // Not the search field, which has its hint.
            n.role() == Role::TextInput
                && n.placeholder().unwrap_or_default().is_empty()
                && n.value().as_deref() == Some(value)
        })
        .next()
        .ok_or(format!("a field holding {value:?}"))?)
}

/// What the field with the keyboard holds.
fn focused(harness: &Harness<'_, App>) -> Option<String> {
    harness
        .query_all_by(|n| n.is_focused())
        .next()
        .and_then(|n| n.value())
}

/// Clicks into the field of the Line `value`, the cursor at its end.
fn into_line(harness: &mut Harness<'_, App>, value: &str) -> TestResult {
    line_field(harness, value)?.click();
    harness.run();
    press(harness, egui::Key::End);
    assert_eq!(focused(harness).as_deref(), Some(value));
    Ok(())
}

fn press(harness: &mut Harness<'_, App>, key: egui::Key) {
    harness.key_press(key);
    harness.run();
}

fn type_in(harness: &mut Harness<'_, App>, text: &str) {
    harness.get_by(|n| n.is_focused()).type_text(text);
    harness.run();
}

/// Leaves the editor for the Note `other`.
fn leave_for(harness: &mut Harness<'_, App>, other: &str) {
    harness.state_mut().select_note(other);
    harness.run();
}

#[test]
fn enter_at_the_end_starts_an_empty_line_and_leaving_keeps_what_was_typed() -> TestResult {
    let (store, note) = with_note()?;
    Checklists::new(&store).make(&note)?;
    let other = Notes::new(&store).capture("Other", None, None)?.id;
    let mut harness = window(&store, Some(&note));
    // The Lines are typed in place; there is no button to add one, and
    // no empty Line while every Line can be typed in.
    assert!(line_field(&harness, "Passport").is_ok());
    assert!(!shows(&harness, "Add Line"));
    assert!(!shows(&harness, "Add"));
    assert!(line_field(&harness, "").is_err());
    into_line(&mut harness, "Ticket")?;
    press(&mut harness, egui::Key::Enter);
    // An empty Line exists only on screen until it holds text.
    assert_eq!(list(&harness, &note)?.count(), 3);
    assert_eq!(focused(&harness).as_deref(), Some(""));
    type_in(&mut harness, "Seat");
    // Enter at its end stores it and starts the next.
    press(&mut harness, egui::Key::Enter);
    assert_eq!(
        texts(&harness, &note)?,
        ["Passport", "  Charger", "Ticket", "Seat"]
    );
    assert_eq!(focused(&harness).as_deref(), Some(""));
    type_in(&mut harness, "Window");
    // Tab puts the new Line under the one above before it is stored.
    press(&mut harness, egui::Key::Tab);
    assert_eq!(focused(&harness).as_deref(), Some("Window"));
    // An emptied Line and a new empty one go on leaving; what was typed
    // is kept.
    press(&mut harness, egui::Key::End);
    press(&mut harness, egui::Key::Enter);
    assert_eq!(focused(&harness).as_deref(), Some(""));
    into_line(&mut harness, "Ticket")?;
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness.run();
    press(&mut harness, egui::Key::Backspace);
    assert_eq!(focused(&harness).as_deref(), Some(""));
    into_line(&mut harness, "Passport")?;
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness.run();
    type_in(&mut harness, "Passports");
    assert_eq!(list(&harness, &note)?.lines[0].text, "Passport");
    leave_for(&mut harness, &other);
    assert_eq!(
        texts(&harness, &note)?,
        ["Passports", "  Charger", "Seat", "  Window"]
    );
    Ok(())
}

#[test]
fn enter_splits_a_line_at_the_cursor_into_its_group() -> TestResult {
    let (store, note) = with_note()?;
    Checklists::new(&store).make(&note)?;
    let mut harness = window(&store, Some(&note));
    into_line(&mut harness, "Passport")?;
    for _ in 0..4 {
        press(&mut harness, egui::Key::ArrowLeft);
    }
    press(&mut harness, egui::Key::Enter);
    // A Line with a group gets the rest as the first of its group.
    assert_eq!(
        texts(&harness, &note)?,
        ["Pass", "  port", "  Charger", "Ticket"]
    );
    assert_eq!(focused(&harness).as_deref(), Some("port"));
    // At the start of a Line, an empty one goes in above; the cursor stays.
    into_line(&mut harness, "Ticket")?;
    press(&mut harness, egui::Key::Home);
    press(&mut harness, egui::Key::Enter);
    assert_eq!(focused(&harness).as_deref(), Some("Ticket"));
    assert_eq!(list(&harness, &note)?.count(), 4);
    // Up goes to the Line above, the new empty one, then to Charger;
    // Down back.
    press(&mut harness, egui::Key::ArrowUp);
    assert_eq!(focused(&harness).as_deref(), Some(""));
    press(&mut harness, egui::Key::ArrowUp);
    assert_eq!(focused(&harness).as_deref(), Some("Charger"));
    press(&mut harness, egui::Key::ArrowDown);
    assert_eq!(focused(&harness).as_deref(), Some(""));
    Ok(())
}

#[test]
fn backspace_at_the_start_joins_with_the_line_above_which_keeps_its_box() -> TestResult {
    let (store, note) = with_note()?;
    let lists = Checklists::new(&store);
    lists.make(&note)?;
    let ticket = list_of(&store, &note)?.lines[2].id.clone();
    lists.add(&note, "Rail card", None, true)?;
    lists.check(&ticket)?;
    let mut harness = window(&store, Some(&note));
    assert!(checked(&harness, "Ticket")?);
    into_line(&mut harness, "Ticket")?;
    press(&mut harness, egui::Key::Home);
    press(&mut harness, egui::Key::Backspace);
    // Its text goes to the end of the Line above, which keeps its state;
    // its group stays a group.
    assert_eq!(
        texts(&harness, &note)?,
        ["Passport", "  ChargerTicket", "  Rail card"]
    );
    let now = list(&harness, &note)?;
    assert!(!now.lines[1].checked);
    assert_eq!(focused(&harness).as_deref(), Some("ChargerTicket"));
    // The joined Line's checks stay in the log.
    let store = harness.state().store();
    assert!(store.current(&ticket, "checked")?.is_some());
    assert!(store.current(&ticket, "trashed")?.is_some());
    // The cursor stood where the two met: Enter splits them again.
    press(&mut harness, egui::Key::Enter);
    assert_eq!(
        texts(&harness, &note)?,
        ["Passport", "  Charger", "  Ticket", "  Rail card"]
    );
    // An empty Line simply goes, the cursor at the end of the one above.
    into_line(&mut harness, "Rail card")?;
    press(&mut harness, egui::Key::Enter);
    assert_eq!(focused(&harness).as_deref(), Some(""));
    press(&mut harness, egui::Key::Backspace);
    assert_eq!(focused(&harness).as_deref(), Some("Rail card"));
    assert_eq!(list(&harness, &note)?.count(), 4);
    // At the very top there is nothing to join with.
    into_line(&mut harness, "Passport")?;
    press(&mut harness, egui::Key::Home);
    press(&mut harness, egui::Key::Backspace);
    assert_eq!(list(&harness, &note)?.count(), 4);
    Ok(())
}

#[test]
fn tab_and_shift_tab_move_a_line_into_a_group_and_out() -> TestResult {
    let (store, note) = with_note()?;
    Checklists::new(&store).make(&note)?;
    let mut harness = window(&store, Some(&note));
    into_line(&mut harness, "Ticket")?;
    press(&mut harness, egui::Key::Tab);
    assert_eq!(
        texts(&harness, &note)?,
        ["Passport", "  Charger", "  Ticket"]
    );
    // The field keeps the keyboard while its Line moves.
    assert_eq!(focused(&harness).as_deref(), Some("Ticket"));
    harness.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::Tab);
    harness.run();
    assert_eq!(texts(&harness, &note)?, ["Passport", "  Charger", "Ticket"]);
    // The first Line has nothing to go under.
    into_line(&mut harness, "Passport")?;
    press(&mut harness, egui::Key::Tab);
    assert_eq!(texts(&harness, &note)?, ["Passport", "  Charger", "Ticket"]);
    assert_eq!(focused(&harness).as_deref(), Some("Passport"));
    Ok(())
}

#[test]
fn the_frame_of_a_group_holds_the_groups_own_line() -> TestResult {
    let (store, note) = with_note()?;
    Checklists::new(&store).make(&note)?;
    // On a full card, and in the editor over it.
    for open in [None, Some(note.as_str())] {
        let harness = window(&store, open);
        let left = |text: &str| -> Result<f32, Box<dyn std::error::Error>> {
            let found = boxes(&harness, text).pop().ok_or(format!("{text:?}"))?;
            Ok(found.rect().left())
        };
        let (passport, charger, ticket) = (left("Passport")?, left("Charger")?, left("Ticket")?);
        // Passport stands inside the frame of its group, a little in from
        // Ticket, which has none; Charger is indented under it.
        assert!(passport > ticket && passport < ticket + 10.0, "{open:?}");
        assert!(charger >= passport + 20.0, "{open:?}");
    }
    Ok(())
}

#[test]
fn a_line_typed_in_is_stored_as_the_cursor_leaves_it_and_as_the_window_closes() -> TestResult {
    let (store, note) = with_note()?;
    Checklists::new(&store).make(&note)?;
    let mut harness = window(&store, Some(&note));
    into_line(&mut harness, "Ticket")?;
    type_in(&mut harness, "s");
    into_line(&mut harness, "Charger")?;
    assert_eq!(
        texts(&harness, &note)?,
        ["Passport", "  Charger", "Tickets"]
    );
    type_in(&mut harness, "s");
    assert_eq!(list(&harness, &note)?.lines[1].text, "Charger");
    harness.state_mut().flush();
    harness.run();
    assert_eq!(
        texts(&harness, &note)?,
        ["Passport", "  Chargers", "Tickets"]
    );
    Ok(())
}

#[test]
fn the_line_menu_moves_indents_and_deletes_without_asking() -> TestResult {
    let (store, note) = with_note()?;
    Checklists::new(&store).make(&note)?;
    let mut harness = window(&store, Some(&note));
    // At the top of the top level: no Move up, no indent.
    click(&mut harness, "More: Passport")?;
    assert!(!shows(&harness, "Move up Alt+Up"));
    assert!(!shows(&harness, "Put under the Line above"));
    assert!(shows(&harness, "Move down Alt+Down"));
    harness.key_press(egui::Key::Escape);
    harness.run();

    // A parent moves with its group.
    line_step(&mut harness, "Ticket", "Move up Alt+Up")?;
    assert_eq!(texts(&harness, &note)?, ["Ticket", "Passport", "  Charger"]);
    line_step(&mut harness, "Ticket", "Move down Alt+Down")?;
    assert_eq!(texts(&harness, &note)?, ["Passport", "  Charger", "Ticket"]);

    line_step(&mut harness, "Ticket", "Put under the Line above")?;
    assert_eq!(
        texts(&harness, &note)?,
        ["Passport", "  Charger", "  Ticket"]
    );
    // Within its group, a Line stays on its level.
    line_step(&mut harness, "Ticket", "Move up Alt+Up")?;
    assert_eq!(
        texts(&harness, &note)?,
        ["Passport", "  Ticket", "  Charger"]
    );
    line_step(&mut harness, "Ticket", "Move down Alt+Down")?;
    assert_eq!(
        texts(&harness, &note)?,
        ["Passport", "  Charger", "  Ticket"]
    );
    line_step(&mut harness, "Ticket", "Back to the top level")?;
    assert_eq!(texts(&harness, &note)?, ["Passport", "  Charger", "Ticket"]);

    // Renaming is typing in place: the menu has no Rename.
    click(&mut harness, "More: Ticket")?;
    assert!(!shows(&harness, "Rename"));
    harness.key_press(egui::Key::Escape);
    harness.run();

    // Deleting asks nothing (#268): the Line folds under "Removed
    // Lines", whose Restore brings it back where it was.
    assert!(!shows(&harness, "Removed Lines"));
    line_step(&mut harness, "Charger", "Delete")?;
    assert!(!shows(&harness, "Delete this Line?"));
    assert_eq!(texts(&harness, &note)?, ["Passport", "Ticket"]);
    assert!(!shows(&harness, "Restore"));
    click(&mut harness, "Removed Lines")?;
    assert!(shows(&harness, "Charger"));
    click(&mut harness, "Restore")?;
    assert_eq!(texts(&harness, &note)?, ["Passport", "  Charger", "Ticket"]);
    assert!(!shows(&harness, "Removed Lines"));
    // Ctrl+Z brings back the one deleted last.
    line_step(&mut harness, "Charger", "Delete")?;
    assert_eq!(texts(&harness, &note)?, ["Passport", "Ticket"]);
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    harness.run();
    assert_eq!(texts(&harness, &note)?, ["Passport", "  Charger", "Ticket"]);
    Ok(())
}

#[test]
fn a_line_just_typed_has_its_menu_once_it_holds_text_and_each_step_stores_it_first() -> TestResult {
    let (store, note) = with_note()?;
    Checklists::new(&store).make(&note)?;
    let mut harness = window(&store, Some(&note));
    // An empty new Line has nothing to act on (#202).
    into_line(&mut harness, "Ticket")?;
    press(&mut harness, egui::Key::Enter);
    assert_eq!(focused(&harness).as_deref(), Some(""));
    assert!(!shows(&harness, "More: "));
    type_in(&mut harness, "Seat");
    assert_eq!(list(&harness, &note)?.count(), 3);
    line_step(&mut harness, "Seat", "Move up Alt+Up")?;
    assert_eq!(
        texts(&harness, &note)?,
        ["Passport", "  Charger", "Seat", "Ticket"]
    );

    into_line(&mut harness, "Ticket")?;
    press(&mut harness, egui::Key::Enter);
    type_in(&mut harness, "Window");
    line_step(&mut harness, "Window", "Put under the Line above")?;
    assert_eq!(
        texts(&harness, &note)?,
        ["Passport", "  Charger", "Seat", "Ticket", "  Window"]
    );

    into_line(&mut harness, "Window")?;
    press(&mut harness, egui::Key::Enter);
    type_in(&mut harness, "Bag");
    line_step(&mut harness, "Bag", "Delete")?;
    assert_eq!(
        texts(&harness, &note)?,
        ["Passport", "  Charger", "Seat", "Ticket", "  Window"]
    );
    assert!(!shows(&harness, "More: Bag"));
    Ok(())
}

#[test]
fn back_to_a_plain_note_marks_the_checked_lines_and_indents_the_groups() -> TestResult {
    let (store, note) = with_note()?;
    Checklists::new(&store).make(&note)?;
    let charger = list_of(&store, &note)?.lines[1].id.clone();
    Checklists::new(&store).check(&charger)?;
    let mut harness = window(&store, Some(&note));
    // In the Note dialog's kind badge (#160).
    click(&mut harness, "Kind: Checklist")?;
    pick(&mut harness, "Plain Note");
    let now = Notes::new(harness.state().store())
        .by_id(&note)?
        .ok_or("note")?;
    assert_eq!(now.text, "Packing\n✓ Passport\n  ✓ Charger\nTicket");
    assert!(
        Checklists::new(harness.state().store())
            .by_note(&note)?
            .is_none()
    );
    // The field holds the whole text again.
    assert!(
        harness
            .query_by(|n| n.role() == Role::MultilineTextInput
                && n.value().as_deref() == Some(now.text.as_str()))
            .is_some()
    );
    assert!(shows(&harness, "Kind: Plain Note"));
    Ok(())
}

fn list_of(store: &Store, note: &str) -> Result<Checklist, Box<dyn std::error::Error>> {
    Ok(Checklists::new(store).by_note(note)?.ok_or("a Checklist")?)
}

#[test]
fn the_cards_menu_makes_a_checklist_unchecks_all_and_goes_back() -> TestResult {
    let (store, note) = with_note()?;
    let mut harness = window(&store, None);
    click(&mut harness, "More: Packing")?;
    assert!(shows(&harness, "Edit"));
    click(&mut harness, "Make it a Checklist")?;
    assert_eq!(texts(&harness, &note)?, ["Passport", "  Charger", "Ticket"]);
    // Nothing checked: nothing to uncheck.
    click(&mut harness, "More: Packing")?;
    assert!(!shows(&harness, "Uncheck all"));
    assert!(!shows(&harness, "Make it a Checklist"));
    harness.key_press(egui::Key::Escape);
    harness.run();
    tick(&mut harness, "Ticket")?;
    click(&mut harness, "More: Packing")?;
    click(&mut harness, "Uncheck all")?;
    assert_eq!(list(&harness, &note)?.checked_count(), 0);
    click(&mut harness, "More: Packing")?;
    click(&mut harness, "Back to a plain Note")?;
    let now = Notes::new(harness.state().store())
        .by_id(&note)?
        .ok_or("note")?;
    assert_eq!(now.text, PACKING);
    // Edit in the menu opens the editor.
    click(&mut harness, "More: Packing")?;
    click(&mut harness, "Edit")?;
    assert_eq!(harness.state().selected_note(), Some(note.as_str()));
    Ok(())
}

#[test]
fn full_cards_check_lines_in_place_and_compact_cards_count_them() -> TestResult {
    let (store, note) = with_note()?;
    Checklists::new(&store).make(&note)?;
    let mut harness = window(&store, None);
    assert!(!checked(&harness, "Ticket")?);
    tick(&mut harness, "Ticket")?;
    assert!(checked(&harness, "Ticket")?);
    let ticket = list(&harness, &note)?;
    assert!(ticket.lines[2].checked);
    // The card's checkbox writes a check, never the Note's text.
    assert_eq!(
        Notes::new(harness.state().store())
            .by_id(&note)?
            .ok_or("note")?
            .text,
        "Packing"
    );
    let compact = L10n::new("en").show_notes_compact();
    click(&mut harness, compact)?;
    assert!(shows(&harness, "2 of 3 open"));
    assert!(boxes(&harness, "Ticket").is_empty());
    Ok(())
}

/// Ann's store, Ben's and the Note's id.
type Shared = (Arc<Store>, Arc<Store>, String);

/// Ann's Household with her Checklist [`PACKING`] in it, shared with Ben
/// as a Member: both stores and the Note's id.
fn shared() -> Result<Shared, Box<dyn std::error::Error>> {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let house = Places::new(&ann).add_area("Household")?;
    let note = Notes::new(&ann).capture(PACKING, Some(&house.id), None)?;
    Checklists::new(&ann).make(&note.id)?;
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
    Ok((Arc::new(ann), Arc::new(ben), note.id))
}

#[test]
fn a_member_checks_and_adds_lines_but_changes_only_their_own() -> TestResult {
    let (_ann, ben, note) = shared()?;
    let mut harness = window(&ben, Some(&note));
    assert!(shows(&harness, "Checklist"));
    // Ann's Lines: no ⋮, and the kind stays hers.
    assert!(!shows_part(&harness, "More: Passport"));
    assert!(!shows(&harness, "Back to a plain Note"));
    tick(&mut harness, "Ticket")?;
    assert!(list(&harness, &note)?.lines[2].checked);
    assert!(shows(&harness, "Uncheck all"));
    // Ann's Lines show as words, not to be typed in; an empty Line waits
    // under them.
    assert!(line_field(&harness, "Ticket").is_err());
    assert!(!shows(&harness, "Add Line"));
    line_field(&harness, "")?.click();
    harness.run();
    type_in(&mut harness, "Water");
    press(&mut harness, egui::Key::Enter);
    // His own Line is his to change.
    assert!(shows(&harness, "More: Water"));
    assert_eq!(list(&harness, &note)?.count(), 4);
    // Under Ann's Line nothing is joined; his empty Line goes.
    press(&mut harness, egui::Key::Backspace);
    assert_eq!(focused(&harness).as_deref(), Some("Water"));
    press(&mut harness, egui::Key::Home);
    press(&mut harness, egui::Key::Backspace);
    assert_eq!(
        texts(&harness, &note)?,
        ["Passport", "  Charger", "Ticket", "Water"]
    );
    assert_eq!(focused(&harness).as_deref(), Some("Water"));
    // He moves his own Line into Ann's group and out.
    press(&mut harness, egui::Key::Tab);
    assert_eq!(
        texts(&harness, &note)?,
        ["Passport", "  Charger", "Ticket", "  Water"]
    );
    harness.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::Tab);
    harness.run();
    // His card's menu offers nothing of the kind.
    harness.state_mut().select(Selection::All);
    harness.run();
    click(&mut harness, "More: Packing")?;
    assert!(shows(&harness, "Uncheck all"));
    assert!(!shows(&harness, "Back to a plain Note"));
    Ok(())
}

/// Everything `from` holds that `to` lacks, taken in as a session would.
fn send(from: &Store, to: &Store) -> TestResult {
    to.apply_entries(
        &oldbrainfix_core::sync::wire(&from.entries_since(&to.version_vector()?)?),
        &from.key_records()?,
        None,
        &mut ImportReport::default(),
    )?;
    Ok(())
}

#[test]
fn two_renames_apart_are_a_line_conflict_on_the_conflicts_view() -> TestResult {
    let desk = device(1, "Desk")?;
    let phone = device(2, "Phone")?;
    let note = Notes::new(&desk).capture(PACKING, None, None)?;
    Checklists::new(&desk).make(&note.id)?;
    send(&desk, &phone)?;
    let ticket = list_of(&desk, &note.id)?.lines[2].id.clone();
    Checklists::new(&phone).rename(&ticket, "Train ticket")?;
    Checklists::new(&desk).rename(&ticket, "Plane ticket")?;
    send(&phone, &desk)?;
    assert_eq!(desk.conflicts()?.len(), 1);

    let mut app = App::new(desk, "en");
    app.set_sync_host(Some(IpAddr::V4(Ipv4Addr::LOCALHOST)), "Desk");
    app.open(Page::Sync);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 1400.0))
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            app,
        );
    for _ in 0..400 {
        if harness.state().sync_host().hosting() {
            break;
        }
        harness.step();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    harness.step();
    assert!(shows(&harness, "Line of a Checklist"));
    // The Note it belongs to, by its title.
    assert!(shows(&harness, "Packing"));
    harness
        .query_all_by_label_contains("Train ticket")
        .next()
        .ok_or("the phone's version")?
        .click_accesskit();
    harness.step();
    harness.step();
    assert!(shows(&harness, "No conflicts. Every Device agrees."));
    let desk = harness.state().store();
    assert_eq!(list_of(desk, &note.id)?.lines[2].text, "Train ticket");
    Ok(())
}

fn drop_file(harness: &mut Harness<'_, App>, path: &std::path::Path) {
    harness.input_mut().dropped_files = vec![egui::DroppedFile {
        path: Some(path.to_path_buf()),
        ..Default::default()
    }];
    harness.run();
}

#[test]
fn a_sent_checklist_previews_its_lines_and_an_update_what_changes() -> TestResult {
    let ann = device(3, "Ann")?;
    let note = Notes::new(&ann).capture(PACKING, None, None)?;
    Checklists::new(&ann).make(&note.id)?;
    let dir = tempfile::tempdir()?;
    let first = dir.path().join("first.oldbrain");
    std::fs::write(&first, ann.send_note(&note.id, None, false)?)?;
    let desk = Arc::new(device(1, "Ada")?);
    let mut harness = window(&desk, None);
    drop_file(&mut harness, &first);
    assert!(harness.state().previewing());
    // From a stranger: held, nothing of it before "Show" (#91).
    assert!(boxes(&harness, "Charger").is_empty());
    click(&mut harness, "Show")?;
    // To look at, not to check.
    let charger = boxes(&harness, "Charger").pop().ok_or("Charger")?;
    assert!(charger.accesskit_node().is_disabled());
    assert!(shows(&harness, "Ticket"));
    click(&mut harness, "Accept")?;
    assert!(!harness.state().previewing());

    let lists = Checklists::new(&ann);
    let ticket = list_of(&ann, &note.id)?.lines[2].id.clone();
    lists.rename(&ticket, "Train ticket")?;
    lists.add(&note.id, "Water", None, false)?;
    let update = dir.path().join("update.oldbrain");
    std::fs::write(&update, ann.send_note(&note.id, None, false)?)?;
    // Ann is kept now: her update shows at once.
    drop_file(&mut harness, &update);
    assert!(shows(&harness, "Line: Ticket → Train ticket"));
    assert!(shows(&harness, "New Line: Water"));
    Ok(())
}

/// Presses Alt with `key`.
fn alt(harness: &mut Harness<'_, App>, key: egui::Key) {
    harness.key_press_modifiers(egui::Modifiers::ALT, key);
    harness.run();
}

#[test]
fn alt_up_and_alt_down_move_the_line_with_the_cursor_on_its_level() -> TestResult {
    let (store, note) = with_note()?;
    Checklists::new(&store).make(&note)?;
    let mut harness = window(&store, Some(&note));
    // A Line at the top passes the next one with its group (#283).
    into_line(&mut harness, "Ticket")?;
    alt(&mut harness, egui::Key::ArrowUp);
    assert_eq!(texts(&harness, &note)?, ["Ticket", "Passport", "  Charger"]);
    // The cursor stays in it, typing on where it was.
    assert_eq!(focused(&harness).as_deref(), Some("Ticket"));
    type_in(&mut harness, "s");
    assert_eq!(focused(&harness).as_deref(), Some("Tickets"));
    // At the top it stays, and so does the cursor.
    alt(&mut harness, egui::Key::ArrowUp);
    assert_eq!(texts(&harness, &note)?, ["Ticket", "Passport", "  Charger"]);
    assert_eq!(focused(&harness).as_deref(), Some("Tickets"));
    // A move keeps what was typed first.
    alt(&mut harness, egui::Key::ArrowDown);
    assert_eq!(
        texts(&harness, &note)?,
        ["Passport", "  Charger", "Tickets"]
    );
    // The only Line of a group stays in it; the cursor does not go up.
    into_line(&mut harness, "Charger")?;
    alt(&mut harness, egui::Key::ArrowUp);
    alt(&mut harness, egui::Key::ArrowDown);
    assert_eq!(
        texts(&harness, &note)?,
        ["Passport", "  Charger", "Tickets"]
    );
    assert_eq!(focused(&harness).as_deref(), Some("Charger"));
    // ⋮ names the keys.
    click(&mut harness, "More: Passport")?;
    assert!(shows(&harness, "Move down Alt+Down"));
    Ok(())
}

#[test]
fn in_a_line_alt_up_moves_the_line_and_not_the_area_shown() -> TestResult {
    let (store, note) = with_note()?;
    Checklists::new(&store).make(&note)?;
    let places = Places::new(&store);
    places.add_area("Garden")?;
    let travel = places
        .areas()?
        .into_iter()
        .find(|a| a.name == "Travel")
        .ok_or("Travel")?;
    let mut app = App::new(store.clone(), "en");
    app.select(Selection::Place(travel.id.clone()));
    app.select_note(&note);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 2400.0))
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            app,
        );
    harness.run();
    let order = |harness: &Harness<'_, App>| -> Result<Vec<String>, Box<dyn std::error::Error>> {
        Ok(
            oldbrainfix_core::PersonSettings::new(harness.state().store())
                .areas_in_order()?
                .into_iter()
                .map(|a| a.name)
                .collect(),
        )
    };
    let areas = order(&harness)?;
    into_line(&mut harness, "Ticket")?;
    alt(&mut harness, egui::Key::ArrowUp);
    assert_eq!(texts(&harness, &note)?, ["Ticket", "Passport", "  Charger"]);
    assert_eq!(order(&harness)?, areas);
    Ok(())
}

/// Presses at `from`, moves a little, then to `to`, and lets go there,
/// as a hand drags.
fn drag(harness: &mut Harness<'_, App>, from: egui::Pos2, to: egui::Pos2) {
    harness.run_steps(50);
    harness.drag_at(from);
    harness.run();
    harness.hover_at(from + egui::vec2(0.0, 8.0));
    harness.run();
    harness.hover_at(to);
    harness.run();
    harness.event(egui::Event::PointerButton {
        pos: to,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    harness.run();
}

/// The middle of the handle of the Line `text`.
fn handle(
    harness: &Harness<'_, App>,
    text: &str,
) -> Result<egui::Pos2, Box<dyn std::error::Error>> {
    let label = format!("Drag to move: {text}");
    let node = harness
        .query_all_by_label(&label)
        .last()
        .ok_or(format!("{label:?}"))?;
    Ok(node.rect().center())
}

/// Just inside the top of the editor's row of the Line `text`, or with
/// `under` just inside its bottom.
fn edge(
    harness: &Harness<'_, App>,
    text: &str,
    under: bool,
) -> Result<egui::Pos2, Box<dyn std::error::Error>> {
    let node = boxes(harness, text).pop().ok_or(format!("{text:?}"))?;
    let rect = node.rect();
    let y = if under {
        rect.bottom() - 3.0
    } else {
        rect.top() + 3.0
    };
    Ok(egui::pos2(rect.right() + 40.0, y))
}

/// The packing Note with two Lines in Passport's group, a Checklist.
fn with_group() -> Result<(Arc<Store>, String), Box<dyn std::error::Error>> {
    let store = device(3, "Desk")?;
    let travel = Places::new(&store).add_area("Travel")?;
    let text = "Packing\nPassport\n  Charger\n  Adapter\nTicket\nMap";
    let note = Notes::new(&store).capture(text, Some(&travel.id), None)?;
    Checklists::new(&store).make(&note.id)?;
    Ok((Arc::new(store), note.id))
}

#[test]
fn a_line_is_dragged_by_its_handle_within_its_group_or_the_top_level() -> TestResult {
    let (store, note) = with_group()?;
    let mut harness = window(&store, Some(&note));
    let start = ["Passport", "  Charger", "  Adapter", "Ticket", "Map"];
    assert_eq!(texts(&harness, &note)?, start);
    // One place up at the top, past Ticket (#283).
    let (from, to) = (handle(&harness, "Map")?, edge(&harness, "Ticket", false)?);
    drag(&mut harness, from, to);
    assert_eq!(
        texts(&harness, &note)?,
        ["Passport", "  Charger", "  Adapter", "Map", "Ticket"]
    );
    // A Line with a group goes with it, to the end.
    let (from, to) = (
        handle(&harness, "Passport")?,
        edge(&harness, "Ticket", true)?,
    );
    drag(&mut harness, from, to);
    assert_eq!(
        texts(&harness, &note)?,
        ["Map", "Ticket", "Passport", "  Charger", "  Adapter"]
    );
    // Within its group, up and back to its end.
    let (from, to) = (
        handle(&harness, "Adapter")?,
        edge(&harness, "Charger", false)?,
    );
    drag(&mut harness, from, to);
    assert_eq!(
        texts(&harness, &note)?,
        ["Map", "Ticket", "Passport", "  Adapter", "  Charger"]
    );
    let (from, to) = (
        handle(&harness, "Adapter")?,
        edge(&harness, "Charger", true)?,
    );
    drag(&mut harness, from, to);
    assert_eq!(
        texts(&harness, &note)?,
        ["Map", "Ticket", "Passport", "  Charger", "  Adapter"]
    );
    // Out of its group, where it stands and away from the Lines, it
    // stays.
    let (from, to) = (handle(&harness, "Charger")?, edge(&harness, "Map", false)?);
    drag(&mut harness, from, to);
    let (from, to) = (handle(&harness, "Ticket")?, edge(&harness, "Ticket", true)?);
    drag(&mut harness, from, to);
    let from = handle(&harness, "Ticket")?;
    drag(&mut harness, from, egui::pos2(from.x, 2300.0));
    assert_eq!(
        texts(&harness, &note)?,
        ["Map", "Ticket", "Passport", "  Charger", "  Adapter"]
    );
    Ok(())
}
