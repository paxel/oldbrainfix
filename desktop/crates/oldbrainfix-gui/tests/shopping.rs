//! Shopping Lists in the window (#122, ADR 0030): "Make it a Shopping
//! List" in a Note's editor and in its card's ⋮ turns its lines into
//! Lines; checking strikes a Line and moves it to the end with the day
//! it comes back, and at the start of that day it is open at the top
//! again; the field at the top adds Lines, a name already there bringing
//! its Line back; the Line menu renames, sets the reappear time and
//! deletes for good after asking once; full cards check, compact cards
//! count the open Lines; a Member checks and adds but changes only their
//! own Lines; and a Line Conflict is named as a Shopping List's.

use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, Utc};
use egui::accesskit::{Role, Toggled};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::lan::{Joining, answer_sync_inviting, apply_answer_as, sync_request_as};
use oldbrainfix_core::{
    AreaRole, ImportReport, Invite, Notes, OwnProfile, PairedDevice, Places, Reappear,
    ShoppingList, ShoppingLists, SigningKey, Store,
};
use oldbrainfix_gui::l10n::L10n;
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// 07:00 on Saturday, 24 October 2026, the wall clock on UTC.
const NOW: &str = "2026-10-24T07:00:00Z";

/// The text of the Note every test shops with.
const SHOPPING: &str = "Weekly\nMilk\nBread\nApples";

/// The moment the clocks of the tests show, in seconds since 1970: it
/// moves a second per read and can be set.
type Moment = Arc<AtomicI64>;

/// A store on a clock that moves a second per read from [`NOW`], with
/// its own Profile named `name`, and the clock.
fn device(seed: u8, name: &str) -> Result<(Store, Moment), Box<dyn std::error::Error>> {
    let start = DateTime::parse_from_rfc3339(NOW)?.timestamp();
    let moment = Arc::new(AtomicI64::new(start));
    let ticks = moment.clone();
    let clock = Box::new(move || {
        DateTime::<Utc>::from_timestamp(ticks.fetch_add(1, Ordering::SeqCst), 0).unwrap_or_default()
    });
    let store = Store::in_memory(clock, move || Ok(SigningKey::from_seed([seed; 32])))?;
    store.set_local_setting("author", name)?;
    OwnProfile::new(&store).apply_defaults(name, None)?;
    Ok((store, moment))
}

/// A store with the Note [`SHOPPING`] in the Area "Home": the store, the
/// Note's id and the clock.
fn with_note() -> Result<(Arc<Store>, String, Moment), Box<dyn std::error::Error>> {
    let (store, moment) = device(3, "Desk")?;
    let home = Places::new(&store).add_area("Home")?;
    let note = Notes::new(&store).capture(SHOPPING, Some(&home.id), None)?;
    Ok((Arc::new(store), note.id, moment))
}

/// The window over `store` in English listing All Notes, its days on
/// UTC, the Note `note` open in its editor.
fn window(store: &Arc<Store>, note: Option<&str>) -> Harness<'static, App> {
    let mut app = App::new(store.clone(), "en");
    app.set_wall_clock(Arc::new(|utc: DateTime<Utc>| utc.naive_utc()));
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

/// The checkboxes of the Line `text`.
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

/// The days of the tests, on UTC.
fn utc(utc: DateTime<Utc>) -> chrono::NaiveDateTime {
    utc.naive_utc()
}

/// The Shopping List of `note` in `store` as the core reads it now.
fn list_of(store: &Store, note: &str) -> Result<ShoppingList, Box<dyn std::error::Error>> {
    Ok(ShoppingLists::new(store)
        .by_note(note, &utc)?
        .ok_or("a Shopping List")?)
}

fn list(
    harness: &Harness<'_, App>,
    note: &str,
) -> Result<ShoppingList, Box<dyn std::error::Error>> {
    list_of(harness.state().store(), note)
}

/// The texts of the Lines of `note` in the order they show, a struck one
/// marked "✓ ".
fn texts(
    harness: &Harness<'_, App>,
    note: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    Ok(list(harness, note)?
        .lines
        .iter()
        .map(|l| {
            let mark = if l.struck { "✓ " } else { "" };
            format!("{mark}{}", l.text)
        })
        .collect())
}

/// The id of the Line `text` of `note`.
fn line_id(store: &Store, note: &str, text: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(list_of(store, note)?
        .lines
        .iter()
        .find(|l| l.text == text)
        .ok_or(format!("{text:?}"))?
        .id
        .clone())
}

/// Types `text` into the add field, line by line.
fn type_lines(harness: &mut Harness<'_, App>, lines: &[&str]) {
    harness
        .get_by(|n| n.placeholder() == Some("New Line"))
        .click();
    harness.run();
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            harness.key_press(egui::Key::Enter);
            harness.run();
        }
        harness.get_by(|n| n.is_focused()).type_text(line);
        harness.run();
    }
}

#[test]
fn make_it_a_shopping_list_in_the_editor_turns_the_lines_into_lines() -> TestResult {
    let (store, note, _) = with_note()?;
    let mut harness = window(&store, Some(&note));
    assert!(!shows(&harness, "Shopping List"));
    click(&mut harness, "Make it a Shopping List")?;
    assert_eq!(texts(&harness, &note)?, ["Milk", "Bread", "Apples"]);
    let now = Notes::new(harness.state().store())
        .by_id(&note)?
        .ok_or("note")?;
    assert_eq!(now.text, "Weekly");
    // The field holds the title now, so leaving it writes nothing old.
    assert!(
        harness
            .query_by(
                |n| n.role() == Role::MultilineTextInput && n.value().as_deref() == Some("Weekly")
            )
            .is_some()
    );
    assert!(shows(&harness, "Shopping List"));
    assert!(!shows(&harness, "Make it a Shopping List"));
    // A Shopping List is neither a Checklist nor a Tally.
    assert!(!shows(&harness, "Make it a Checklist"));
    assert!(!shows(&harness, "Make it a Tally"));
    assert!(!shows(&harness, "Uncheck all"));
    Ok(())
}

#[test]
fn a_checked_line_goes_to_the_end_with_its_day_back_and_unchecked_to_the_top() -> TestResult {
    let (store, note, _) = with_note()?;
    ShoppingLists::new(&store).make(&note)?;
    let mut harness = window(&store, Some(&note));
    tick(&mut harness, "Milk")?;
    assert!(checked(&harness, "Milk")?);
    assert_eq!(texts(&harness, &note)?, ["Bread", "Apples", "✓ Milk"]);
    // A week on, the same weekday, in the locale's short date.
    assert!(shows(&harness, "back Sat, Oct 31"));
    tick(&mut harness, "Milk")?;
    assert!(!checked(&harness, "Milk")?);
    assert_eq!(texts(&harness, &note)?, ["Milk", "Bread", "Apples"]);
    assert!(!shows_part(&harness, "back Sat"));
    Ok(())
}

#[test]
fn a_struck_line_is_open_again_at_the_top_from_the_start_of_its_day() -> TestResult {
    let (store, note, moment) = with_note()?;
    let lists = ShoppingLists::new(&store);
    lists.make(&note)?;
    let bread = line_id(&store, &note, "Bread")?;
    lists.check(&bread, &utc)?;
    let mut harness = window(&store, None);
    assert!(checked(&harness, "Bread")?);
    assert!(shows(&harness, "back Sat, Oct 31"));
    // Late on the day before: still struck.
    let eve = DateTime::parse_from_rfc3339("2026-10-30T23:50:00Z")?.timestamp();
    moment.store(eve, Ordering::SeqCst);
    harness.run();
    assert!(checked(&harness, "Bread")?);
    // Its day has begun: the window reads it again by itself.
    let midnight = DateTime::parse_from_rfc3339("2026-10-31T00:00:01Z")?.timestamp();
    moment.store(midnight, Ordering::SeqCst);
    harness.run();
    assert!(!checked(&harness, "Bread")?);
    assert!(!shows_part(&harness, "back Sat"));
    assert_eq!(texts(&harness, &note)?, ["Bread", "Milk", "Apples"]);
    Ok(())
}

#[test]
fn the_add_field_adds_at_the_top_and_a_name_there_brings_its_line_back() -> TestResult {
    let (store, note, _) = with_note()?;
    let lists = ShoppingLists::new(&store);
    lists.make(&note)?;
    lists.check(&line_id(&store, &note, "Bread")?, &utc)?;
    let mut harness = window(&store, Some(&note));
    // The field and "Add" sit above the Lines.
    let field = harness
        .get_by(|n| n.placeholder() == Some("New Line"))
        .rect()
        .top();
    // The editor's, right of the card's.
    let milk = boxes(&harness, "Milk")
        .into_iter()
        .max_by(|a, b| a.rect().left().total_cmp(&b.rect().left()))
        .ok_or("Milk")?;
    assert!(field < milk.rect().top());
    type_lines(&mut harness, &["Eggs", "  bread "]);
    click(&mut harness, "Add")?;
    // The first typed ends at the top; Bread comes back, not twice.
    assert_eq!(texts(&harness, &note)?, ["Eggs", "Bread", "Milk", "Apples"]);
    assert!(
        harness
            .query_by(|n| n.role() == Role::MultilineTextInput
                && n.value().is_some_and(|v| v.contains("Eggs")))
            .is_none(),
        "the field is empty again"
    );
    // An open name stays as it is.
    type_lines(&mut harness, &["MILK"]);
    click(&mut harness, "Add")?;
    assert_eq!(texts(&harness, &note)?, ["Eggs", "Bread", "Milk", "Apples"]);
    Ok(())
}

#[test]
fn the_line_menu_renames_sets_the_reappear_time_and_deletes_for_good() -> TestResult {
    let (store, note, _) = with_note()?;
    ShoppingLists::new(&store).make(&note)?;
    let mut harness = window(&store, Some(&note));
    // A new Line comes back after a week.
    click(&mut harness, "More: Milk")?;
    assert!(shows(&harness, "Reappear time"));
    let radios: Vec<(String, bool)> = harness
        .query_all_by_role(Role::RadioButton)
        .filter(|n| {
            let label = n.accesskit_node().label().unwrap_or_default();
            label == "Once" || label.ends_with("week") || label.ends_with("weeks")
        })
        .map(|n| {
            let label = n.accesskit_node().label().unwrap_or_default();
            (label, n.accesskit_node().toggled() == Some(Toggled::True))
        })
        .collect();
    let want = [
        ("Once", false),
        ("1 week", true),
        ("2 weeks", false),
        ("3 weeks", false),
        ("4 weeks", false),
    ];
    assert_eq!(
        radios,
        want.map(|(l, on)| (l.to_string(), on)).to_vec(),
        "the reappear times"
    );
    assert!(!shows(&harness, "Move up"));
    assert!(!shows(&harness, "Put under the Line above"));
    click(&mut harness, "3 weeks")?;
    let milk = line_id(&store, &note, "Milk")?;
    let reappear = |harness: &Harness<'_, App>| -> Result<Reappear, Box<dyn std::error::Error>> {
        Ok(list(harness, &note)?.line(&milk).ok_or("Milk")?.reappear)
    };
    assert_eq!(reappear(&harness)?, Reappear::Week3);
    harness.key_press(egui::Key::Escape);
    harness.run();
    tick(&mut harness, "Milk")?;
    assert!(shows(&harness, "back Sat, Nov 14"));

    // Once: gone when checked.
    click(&mut harness, "More: Apples")?;
    click(&mut harness, "Once")?;
    harness.key_press(egui::Key::Escape);
    harness.run();
    tick(&mut harness, "Apples")?;
    assert_eq!(texts(&harness, &note)?, ["Bread", "✓ Milk"]);

    click(&mut harness, "More: Bread")?;
    click(&mut harness, "Rename")?;
    assert!(shows(&harness, "Rename Line"));
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness.run();
    harness
        .get_by(|n| n.role() == Role::TextInput && n.value().as_deref() == Some("Bread"))
        .type_text("Rye bread");
    harness.run();
    click(&mut harness, "Save")?;
    assert_eq!(texts(&harness, &note)?, ["Rye bread", "✓ Milk"]);

    // Deleting asks once, with the menu's words; Cancel keeps it.
    click(&mut harness, "More: Rye bread")?;
    click(&mut harness, "Delete for good")?;
    assert!(shows(&harness, "Delete this Line?"));
    click(&mut harness, "Cancel")?;
    assert_eq!(list(&harness, &note)?.count(), 2);
    click(&mut harness, "More: Rye bread")?;
    click(&mut harness, "Delete for good")?;
    click(&mut harness, "Delete for good")?;
    assert_eq!(texts(&harness, &note)?, ["✓ Milk"]);
    Ok(())
}

#[test]
fn back_to_a_plain_note_marks_the_struck_lines() -> TestResult {
    let (store, note, _) = with_note()?;
    let lists = ShoppingLists::new(&store);
    lists.make(&note)?;
    lists.check(&line_id(&store, &note, "Bread")?, &utc)?;
    let mut harness = window(&store, Some(&note));
    click(&mut harness, "Back to a plain Note")?;
    let now = Notes::new(harness.state().store())
        .by_id(&note)?
        .ok_or("note")?;
    assert_eq!(now.text, "Weekly\nMilk\nApples\n✓ Bread");
    assert!(
        !ShoppingLists::new(harness.state().store()).is_shopping_list(&note)?,
        "a plain Note again"
    );
    // The field holds the whole text again.
    assert!(
        harness
            .query_by(|n| n.role() == Role::MultilineTextInput
                && n.value().as_deref() == Some(now.text.as_str()))
            .is_some()
    );
    assert!(shows(&harness, "Make it a Shopping List"));
    Ok(())
}

#[test]
fn the_cards_menu_makes_a_shopping_list_and_turns_it_back() -> TestResult {
    let (store, note, _) = with_note()?;
    let mut harness = window(&store, None);
    click(&mut harness, "More: Weekly")?;
    assert!(shows(&harness, "Make it a Checklist"));
    click(&mut harness, "Make it a Shopping List")?;
    assert_eq!(texts(&harness, &note)?, ["Milk", "Bread", "Apples"]);
    click(&mut harness, "More: Weekly")?;
    assert!(!shows(&harness, "Make it a Shopping List"));
    assert!(!shows(&harness, "Make it a Checklist"));
    assert!(!shows(&harness, "Uncheck all"));
    click(&mut harness, "Back to a plain Note")?;
    let now = Notes::new(harness.state().store())
        .by_id(&note)?
        .ok_or("note")?;
    assert_eq!(now.text, SHOPPING);
    Ok(())
}

#[test]
fn full_cards_check_lines_and_compact_cards_count_the_open_ones() -> TestResult {
    let (store, note, _) = with_note()?;
    ShoppingLists::new(&store).make(&note)?;
    let mut harness = window(&store, None);
    assert!(!checked(&harness, "Bread")?);
    tick(&mut harness, "Bread")?;
    assert!(checked(&harness, "Bread")?);
    assert_eq!(texts(&harness, &note)?, ["Milk", "Apples", "✓ Bread"]);
    assert!(shows(&harness, "back Sat, Oct 31"));
    // The card's checkbox writes a check, never the Note's text.
    assert_eq!(
        Notes::new(harness.state().store())
            .by_id(&note)?
            .ok_or("note")?
            .text,
        "Weekly"
    );
    let compact = L10n::new("en").show_notes_compact();
    click(&mut harness, compact)?;
    assert!(shows(&harness, "2 open"));
    assert!(boxes(&harness, "Bread").is_empty());
    Ok(())
}

/// Ann's store, Ben's and the Note's id.
type Shared = (Arc<Store>, Arc<Store>, String);

/// Ann's Household with her Shopping List [`SHOPPING`] in it, shared
/// with Ben as a Member: both stores and the Note's id.
fn shared() -> Result<Shared, Box<dyn std::error::Error>> {
    let (ann, _) = device(1, "Ann")?;
    let (ben, _) = device(2, "Ben")?;
    let house = Places::new(&ann).add_area("Household")?;
    let note = Notes::new(&ann).capture(SHOPPING, Some(&house.id), None)?;
    ShoppingLists::new(&ann).make(&note.id)?;
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
    assert!(shows(&harness, "Shopping List"));
    // Ann's Lines: no ⋮, and the kind stays hers.
    assert!(!shows_part(&harness, "More: Milk"));
    assert!(!shows(&harness, "Back to a plain Note"));
    tick(&mut harness, "Milk")?;
    assert!(list(&harness, &note)?.lines[2].struck);
    type_lines(&mut harness, &["Water"]);
    click(&mut harness, "Add")?;
    // His own Line is his to change.
    assert!(shows(&harness, "More: Water"));
    click(&mut harness, "More: Water")?;
    assert!(shows(&harness, "Reappear time"));
    harness.key_press(egui::Key::Escape);
    harness.run();
    // His card's menu offers nothing of the kind.
    harness.state_mut().select(Selection::All);
    harness.run();
    click(&mut harness, "More: Weekly")?;
    assert!(!shows(&harness, "Back to a plain Note"));
    assert!(!shows(&harness, "Make it a Shopping List"));
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
fn two_renames_apart_are_a_conflict_on_a_line_of_a_shopping_list() -> TestResult {
    let (desk, _) = device(1, "Desk")?;
    let (phone, _) = device(2, "Phone")?;
    let note = Notes::new(&desk).capture(SHOPPING, None, None)?;
    ShoppingLists::new(&desk).make(&note.id)?;
    send(&desk, &phone)?;
    let bread = line_id(&desk, &note.id, "Bread")?;
    ShoppingLists::new(&phone).rename(&bread, "Rye bread")?;
    ShoppingLists::new(&desk).rename(&bread, "White bread")?;
    send(&phone, &desk)?;
    assert_eq!(desk.conflicts()?.len(), 1);

    let mut app = App::new(Arc::new(desk), "en");
    app.set_wall_clock(Arc::new(|utc: DateTime<Utc>| utc.naive_utc()));
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
    assert!(shows(&harness, "Line of a Shopping List"));
    assert!(!shows(&harness, "Line of a Checklist"));
    // The Note it belongs to, by its title.
    assert!(shows(&harness, "Weekly"));
    harness
        .query_all_by_label_contains("Rye bread")
        .next()
        .ok_or("the phone's version")?
        .click_accesskit();
    harness.step();
    harness.step();
    assert!(shows(&harness, "No conflicts. Every Device agrees."));
    let desk = harness.state().store();
    assert_eq!(
        list_of(desk, &note.id)?.line(&bread).ok_or("Bread")?.text,
        "Rye bread"
    );
    Ok(())
}

#[test]
fn the_reappear_time_reads_in_both_languages() {
    use oldbrainfix_gui::shopping_view::reappear_label;
    let en = L10n::new("en");
    let de = L10n::new("de");
    assert_eq!(reappear_label(&en, Reappear::Once), "Once");
    assert_eq!(reappear_label(&en, Reappear::Week1), "1 week");
    assert_eq!(reappear_label(&de, Reappear::Week4), "4 Wochen");
    assert_eq!(reappear_label(&de, Reappear::Once), "Einmal");
}
