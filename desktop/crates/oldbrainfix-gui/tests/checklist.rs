//! Checklists in the window (#120, ADR 0030): "Make it a Checklist" in
//! a Note's editor and in its card's ⋮ turns its lines into Lines, one
//! indented under the Line above; the editor checks them, adds them,
//! renames, moves, indents and deletes them after asking once, unchecks
//! all and goes back to a plain Note; full cards check them in place,
//! compact cards count them; a Member checks and adds Lines but changes
//! only their own; a Line Conflict is offered on the Conflicts view; and
//! a Sent Item's preview shows its Lines and what an update changes.

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
    click(&mut harness, "Make it a Checklist")?;
    assert_eq!(texts(&harness, &note)?, ["Passport", "  Charger", "Ticket"]);
    let now = Notes::new(harness.state().store())
        .by_id(&note)?
        .ok_or("note")?;
    assert_eq!(now.text, "Packing");
    // The field holds the title now, so leaving it writes nothing old.
    assert!(
        harness
            .query_by(
                |n| n.role() == Role::MultilineTextInput && n.value().as_deref() == Some("Packing")
            )
            .is_some()
    );
    assert!(shows(&harness, "Checklist"));
    assert!(!shows(&harness, "Make it a Checklist"));
    // A Checklist is no Tally.
    assert!(!shows(&harness, "Make it a Tally"));
    // The group lies in its frame under its parent, further in.
    let x = |text: &str| -> Result<f32, Box<dyn std::error::Error>> {
        let node = boxes(&harness, text).pop().ok_or(format!("{text:?}"))?;
        Ok(node.rect().left())
    };
    assert!(x("Charger")? > x("Passport")? + 10.0);
    assert_eq!(x("Ticket")?, x("Passport")?);
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

#[test]
fn the_add_field_adds_every_line_typed_one_indented_under_the_line_above() -> TestResult {
    let (store, note) = with_note()?;
    Checklists::new(&store).make(&note)?;
    let mut harness = window(&store, Some(&note));
    harness
        .get_by(|n| n.placeholder() == Some("New Line"))
        .click();
    harness.run();
    harness.get_by(|n| n.is_focused()).type_text("Seat");
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    harness.get_by(|n| n.is_focused()).type_text("  Window");
    harness.run();
    click(&mut harness, "Add Line")?;
    assert_eq!(
        texts(&harness, &note)?,
        ["Passport", "  Charger", "Ticket", "Seat", "  Window"]
    );
    assert!(
        harness
            .query_by(|n| n.role() == Role::MultilineTextInput
                && n.value().is_some_and(|v| v.contains("Window")))
            .is_none(),
        "the field is empty again"
    );
    Ok(())
}

#[test]
fn the_line_menu_renames_moves_indents_and_deletes_after_asking() -> TestResult {
    let (store, note) = with_note()?;
    Checklists::new(&store).make(&note)?;
    let mut harness = window(&store, Some(&note));
    // At the top of the top level: no Move up, no indent.
    click(&mut harness, "More: Passport")?;
    assert!(!shows(&harness, "Move up"));
    assert!(!shows(&harness, "Put under the Line above"));
    assert!(shows(&harness, "Move down"));
    harness.key_press(egui::Key::Escape);
    harness.run();

    // A parent moves with its group.
    line_step(&mut harness, "Ticket", "Move up")?;
    assert_eq!(texts(&harness, &note)?, ["Ticket", "Passport", "  Charger"]);
    line_step(&mut harness, "Ticket", "Move down")?;
    assert_eq!(texts(&harness, &note)?, ["Passport", "  Charger", "Ticket"]);

    line_step(&mut harness, "Ticket", "Put under the Line above")?;
    assert_eq!(
        texts(&harness, &note)?,
        ["Passport", "  Charger", "  Ticket"]
    );
    // Within its group, a Line stays on its level.
    line_step(&mut harness, "Ticket", "Move up")?;
    assert_eq!(
        texts(&harness, &note)?,
        ["Passport", "  Ticket", "  Charger"]
    );
    line_step(&mut harness, "Ticket", "Move down")?;
    assert_eq!(
        texts(&harness, &note)?,
        ["Passport", "  Charger", "  Ticket"]
    );
    line_step(&mut harness, "Ticket", "Back to the top level")?;
    assert_eq!(texts(&harness, &note)?, ["Passport", "  Charger", "Ticket"]);

    line_step(&mut harness, "Ticket", "Rename")?;
    assert!(shows(&harness, "Rename Line"));
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness.run();
    harness
        .get_by(|n| n.role() == Role::TextInput && n.value().as_deref() == Some("Ticket"))
        .type_text("Train ticket");
    harness.run();
    click(&mut harness, "Save")?;
    assert_eq!(
        texts(&harness, &note)?,
        ["Passport", "  Charger", "Train ticket"]
    );

    // Deleting asks once; Cancel keeps it.
    line_step(&mut harness, "Charger", "Delete")?;
    assert!(shows(&harness, "Delete this Line?"));
    click(&mut harness, "Cancel")?;
    assert_eq!(list(&harness, &note)?.count(), 3);
    line_step(&mut harness, "Charger", "Delete")?;
    click(&mut harness, "Delete")?;
    assert_eq!(texts(&harness, &note)?, ["Passport", "Train ticket"]);
    Ok(())
}

#[test]
fn back_to_a_plain_note_marks_the_checked_lines_and_indents_the_groups() -> TestResult {
    let (store, note) = with_note()?;
    Checklists::new(&store).make(&note)?;
    let charger = list_of(&store, &note)?.lines[1].id.clone();
    Checklists::new(&store).check(&charger)?;
    let mut harness = window(&store, Some(&note));
    click(&mut harness, "Back to a plain Note")?;
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
    assert!(shows(&harness, "Make it a Checklist"));
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
    assert!(shows(&harness, "1 of 3 checked"));
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
    harness
        .get_by(|n| n.placeholder() == Some("New Line"))
        .click();
    harness.run();
    harness.get_by(|n| n.is_focused()).type_text("Water");
    harness.run();
    click(&mut harness, "Add Line")?;
    // His own Line is his to change.
    assert!(shows(&harness, "More: Water"));
    line_step(&mut harness, "Water", "Rename")?;
    assert!(shows(&harness, "Rename Line"));
    click(&mut harness, "Cancel")?;
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
    app.select(Selection::Sync);
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
    drop_file(&mut harness, &update);
    assert!(shows(&harness, "Line: Ticket → Train ticket"));
    assert!(shows(&harness, "New Line: Water"));
    Ok(())
}
