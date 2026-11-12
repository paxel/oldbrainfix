//! Shopping Lists in the window (#122, ADR 0030): the kind badge of a
//! Note's editor (#188) and its card's ⋮ turn its lines into
//! Lines; checking strikes a Line and moves it to the end with the day
//! it comes back, and at the start of that day it is open in its place
//! again; the Lines are typed in place as one block of text, as a
//! Checklist's (#141, #184): no field and no "Add" at the top, Enter adds
//! a Line where it was typed, a name already there bringing its Line
//! back, Backspace at the start joins, the rows stay where they are while
//! the editor is open, and the typed order is kept the next time; the
//! Line menu sets the reappear time and deletes without asking, "Removed
//! Lines" and Ctrl+Z bringing it back (#268); full cards check, compact cards count the open Lines; a Member
//! checks and adds but changes only their own Lines; and a Line Conflict
//! is named as a Shopping List's. Today's
//! Agenda shows each Shopping List with open Lines as chosen for it
//! (#186), in the card's ⋮ and with the Note dialog's badge: one row with
//! the number of open Lines, the default, each open Line with its box, or
//! none; a Line checked there stays struck under the open ones for the
//! day, also one checked Once, and its box brings it back, unless this
//! Device hides what is checked (#276); a list nobody chose for shows as
//! this Device showed Shopping Lists before, the Settings no longer
//! choose it, and a click on a Line's row opens the Note, on the heading
//! with the keyboard in a new Line after the open ones (#276).

use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, Utc};
use egui::accesskit::{Role, Toggled};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::lan::{Joining, answer_sync_inviting, apply_answer_as, sync_request_as};
use oldbrainfix_core::{
    AreaRole, ClockTime, Day, ImportReport, Invite, Notes, OwnProfile, PairedDevice, Places,
    Reappear, ReappearUnit, Routines, Schedule, ShoppingList, ShoppingLists, ShoppingOnAgenda,
    SigningKey, Store,
};
use oldbrainfix_gui::l10n::L10n;
use oldbrainfix_gui::{App, Page, Selection};

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

/// Whether the Agenda shows the list "Weekly", as its heading alone or
/// with its Lines under it (#208).
fn shows_weekly(harness: &Harness<'_, App>) -> bool {
    shows_part(harness, "Weekly:")
        || harness
            .query_all_by_role_and_label(Role::Button, "Weekly")
            .next()
            .is_some()
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

/// Types `text` into the Line with the keyboard while it is empty, else
/// into the first empty Line.
fn type_line(harness: &mut Harness<'_, App>, text: &str) {
    let field = |n: &egui_kittest::kittest::AccessKitNode<'_>| {
        n.role() == Role::TextInput
            && n.placeholder().unwrap_or_default().is_empty()
            && n.value().as_deref() == Some("")
    };
    if !harness
        .query_all_by(|n| n.is_focused() && field(n))
        .any(|_| true)
    {
        if let Some(empty) = harness.query_all_by(field).next() {
            empty.click();
        }
        harness.run();
    }
    harness.get_by(|n| n.is_focused()).type_text(text);
    harness.run();
}

/// What the Lines' fields in the editor hold, top to bottom.
fn rows(harness: &Harness<'_, App>) -> Vec<String> {
    let mut fields: Vec<(f32, String)> = harness
        .query_all_by(|n| {
            // Not the Note's name (#188).
            n.role() == Role::TextInput
                && n.placeholder().unwrap_or_default().is_empty()
                && n.label().as_deref() != Some("Name")
        })
        .map(|n| (n.rect().top(), n.value().unwrap_or_default()))
        .collect();
    fields.sort_by(|a, b| a.0.total_cmp(&b.0));
    fields.into_iter().map(|(_, value)| value).collect()
}

/// Clicks into the field of the Line `value`, the cursor at its end.
fn to_end_of(harness: &mut Harness<'_, App>, value: &str) -> TestResult {
    line_field(harness, value)?.click();
    harness.run();
    press(harness, egui::Key::End);
    assert_eq!(focused(harness).as_deref(), Some(value));
    Ok(())
}

/// What the field with the keyboard holds.
fn focused(harness: &Harness<'_, App>) -> Option<String> {
    harness
        .query_all_by(|n| n.is_focused())
        .next()
        .and_then(|n| n.value())
}

fn press(harness: &mut Harness<'_, App>, key: egui::Key) {
    harness.key_press(key);
    harness.run();
}

/// The field a Line is renamed in, holding `value`.
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

/// Clicks into the field of the Line `value` and selects all of it.
fn into_line(harness: &mut Harness<'_, App>, value: &str) -> TestResult {
    line_field(harness, value)?.click();
    harness.run();
    assert_eq!(focused(harness).as_deref(), Some(value));
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::A);
    harness.run();
    Ok(())
}

#[test]
fn make_it_a_shopping_list_in_the_editor_turns_the_lines_into_lines() -> TestResult {
    let (store, note, _) = with_note()?;
    let mut harness = window(&store, Some(&note));
    assert!(!shows(&harness, "Shopping List"));
    click(&mut harness, "Kind: Plain Note")?;
    pick(&mut harness, "Shopping List")?;
    assert_eq!(texts(&harness, &note)?, ["Milk", "Bread", "Apples"]);
    let now = Notes::new(harness.state().store())
        .by_id(&note)?
        .ok_or("note")?;
    assert_eq!(now.text, "Weekly");
    // The name holds the title now, so leaving it writes nothing old
    // (#188).
    assert_eq!(
        harness
            .get_by_role_and_label(Role::TextInput, "Name")
            .value()
            .as_deref(),
        Some("Weekly")
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
fn a_checked_line_goes_to_the_end_with_its_day_back_and_unchecked_into_its_place() -> TestResult {
    let (store, note, _) = with_note()?;
    ShoppingLists::new(&store).make(&note)?;
    weekly(&store, &note)?;
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
fn a_struck_line_is_open_again_in_its_place_from_the_start_of_its_day() -> TestResult {
    let (store, note, moment) = with_note()?;
    let lists = ShoppingLists::new(&store);
    lists.make(&note)?;
    weekly(&store, &note)?;
    let bread = line_id(&store, &note, "Bread")?;
    lists.check(&bread, &utc)?;
    let mut harness = window(&store, None);
    // Folded under the open Lines until a click shows it (#189).
    assert!(boxes(&harness, "Bread").is_empty());
    click(&mut harness, "1 struck through")?;
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
    assert_eq!(texts(&harness, &note)?, ["Milk", "Bread", "Apples"]);
    Ok(())
}

#[test]
fn enter_adds_a_line_where_it_was_typed_and_a_name_there_brings_its_line_back() -> TestResult {
    let (store, note, _) = with_note()?;
    let lists = ShoppingLists::new(&store);
    lists.make(&note)?;
    lists.check(&line_id(&store, &note, "Bread")?, &utc)?;
    let mut harness = window(&store, Some(&note));
    // No field and no "Add" at the top: the Lines are typed in place.
    assert!(!shows(&harness, "Add"));
    assert!(!shows(&harness, "Add Line"));
    assert_eq!(rows(&harness), ["Milk", "Apples", "Bread"]);
    // Enter at the end of a Line starts an empty one under it, and the
    // Line typed there is added among the open Lines where it stands.
    to_end_of(&mut harness, "Apples")?;
    press(&mut harness, egui::Key::Enter);
    assert_eq!(focused(&harness).as_deref(), Some(""));
    assert_eq!(list(&harness, &note)?.count(), 3);
    type_line(&mut harness, "Eggs");
    press(&mut harness, egui::Key::Enter);
    assert_eq!(
        texts(&harness, &note)?,
        ["Milk", "Apples", "Eggs", "✓ Bread"]
    );
    assert_eq!(focused(&harness).as_deref(), Some(""));
    // Bread comes back, in its place, not twice; its row stays where it
    // is while the editor is open.
    type_line(&mut harness, "  bread ");
    press(&mut harness, egui::Key::Enter);
    assert_eq!(texts(&harness, &note)?, ["Milk", "Bread", "Apples", "Eggs"]);
    assert_eq!(rows(&harness), ["Milk", "Apples", "Eggs", "", "Bread"]);
    assert_eq!(focused(&harness).as_deref(), Some(""));
    // An open name stays as it is.
    type_line(&mut harness, "MILK");
    press(&mut harness, egui::Key::Enter);
    assert_eq!(texts(&harness, &note)?, ["Milk", "Bread", "Apples", "Eggs"]);
    assert_eq!(rows(&harness), ["Milk", "Apples", "Eggs", "", "Bread"]);
    Ok(())
}

#[test]
fn enter_mid_line_moves_the_rest_down_and_backspace_at_the_start_joins() -> TestResult {
    let (store, note, _) = with_note()?;
    ShoppingLists::new(&store).make(&note)?;
    let mut harness = window(&store, Some(&note));
    to_end_of(&mut harness, "Bread")?;
    harness.get_by(|n| n.is_focused()).type_text("Butter");
    harness.run();
    for _ in 0.."Butter".len() {
        press(&mut harness, egui::Key::ArrowLeft);
    }
    press(&mut harness, egui::Key::Enter);
    assert_eq!(
        texts(&harness, &note)?,
        ["Milk", "Bread", "Butter", "Apples"]
    );
    assert_eq!(focused(&harness).as_deref(), Some("Butter"));
    // Up and Down go between the Lines.
    press(&mut harness, egui::Key::ArrowUp);
    assert_eq!(focused(&harness).as_deref(), Some("Bread"));
    press(&mut harness, egui::Key::ArrowDown);
    assert_eq!(focused(&harness).as_deref(), Some("Butter"));
    // Backspace at the start: the text goes to the end of the Line above,
    // and the Line joined goes, its checks staying in the log.
    let lists = ShoppingLists::new(&store);
    let butter = line_id(&store, &note, "Butter")?;
    lists.check(&butter, &utc)?;
    harness.run();
    press(&mut harness, egui::Key::Home);
    press(&mut harness, egui::Key::Backspace);
    assert_eq!(texts(&harness, &note)?, ["Milk", "BreadButter", "Apples"]);
    assert_eq!(focused(&harness).as_deref(), Some("BreadButter"));
    assert!(store.current(&butter, "checked")?.is_some());
    assert!(store.current(&butter, "trashed")?.is_some());
    // At the very top there is nothing to join with.
    to_end_of(&mut harness, "Milk")?;
    press(&mut harness, egui::Key::Home);
    press(&mut harness, egui::Key::Backspace);
    assert_eq!(texts(&harness, &note)?, ["Milk", "BreadButter", "Apples"]);
    Ok(())
}

#[test]
fn a_new_list_starts_with_one_line_and_keeps_the_order_typed_the_next_time() -> TestResult {
    let (store, _, _) = with_note()?;
    let lists = ShoppingLists::new(&store);
    let note = Notes::new(&store).capture("Market", None, None)?.id;
    lists.make(&note)?;
    let other = Notes::new(&store).capture("Other", None, None)?.id;
    let mut harness = window(&store, Some(&note));
    assert_eq!(rows(&harness), [""]);
    for (i, text) in ["Leeks", "Pears", "Cheese"].into_iter().enumerate() {
        if i > 0 {
            press(&mut harness, egui::Key::Enter);
        }
        type_line(&mut harness, text);
    }
    // One more, typed between the first two.
    to_end_of(&mut harness, "Leeks")?;
    press(&mut harness, egui::Key::Enter);
    type_line(&mut harness, "Eggs");
    // Checked in the editor, a Line stays in its row while it is open.
    tick(&mut harness, "Pears")?;
    assert_eq!(rows(&harness), ["Leeks", "Eggs", "Pears", "Cheese"]);
    harness.state_mut().select_note(&other);
    harness.run();
    assert_eq!(
        texts(&harness, &note)?,
        ["Leeks", "Eggs", "Cheese", "✓ Pears"]
    );
    // The next time the open Lines show in the order typed, the checked
    // one below them.
    harness.state_mut().select_note(&note);
    harness.run();
    assert_eq!(rows(&harness), ["Leeks", "Eggs", "Cheese", "Pears"]);
    Ok(())
}

#[test]
fn lines_are_renamed_in_place_and_leaving_keeps_what_was_typed() -> TestResult {
    let (store, note, _) = with_note()?;
    ShoppingLists::new(&store).make(&note)?;
    let other = Notes::new(&store).capture("Other", None, None)?.id;
    let mut harness = window(&store, Some(&note));
    // Enter at the end stores the rename and starts a new Line.
    into_line(&mut harness, "Bread")?;
    harness.get_by(|n| n.is_focused()).type_text("Rye bread");
    harness.run();
    press(&mut harness, egui::Key::Enter);
    assert_eq!(texts(&harness, &note)?, ["Milk", "Rye bread", "Apples"]);
    assert_eq!(focused(&harness).as_deref(), Some(""));
    // A rename the cursor leaves is stored; a Line left empty goes on
    // leaving, and a new one typed is added.
    into_line(&mut harness, "Milk")?;
    harness.get_by(|n| n.is_focused()).type_text("Oat milk");
    harness.run();
    into_line(&mut harness, "Apples")?;
    assert_eq!(texts(&harness, &note)?, ["Oat milk", "Rye bread", "Apples"]);
    press(&mut harness, egui::Key::Backspace);
    assert_eq!(rows(&harness), ["Oat milk", "Rye bread", "", ""]);
    line_field(&harness, "")?.click();
    harness.run();
    type_line(&mut harness, "Butter");
    harness.state_mut().select_note(&other);
    harness.run();
    assert_eq!(texts(&harness, &note)?, ["Oat milk", "Rye bread", "Butter"]);
    Ok(())
}

/// Gives each Line of the Shopping List of `note` a week (#195).
fn weekly(store: &Store, note: &str) -> TestResult {
    let lists = ShoppingLists::new(store);
    for line in list_of(store, note)?.lines {
        lists.set_reappear(&line.id, Reappear::FALLBACK)?;
    }
    Ok(())
}

/// The reappear time badges of the Lines in the editor, in the order of
/// their rows (#195).
fn badges(harness: &Harness<'_, App>) -> Vec<String> {
    harness
        .query_all_by_label_contains("Reappear time: ")
        .filter(|n| n.accesskit_node().role() == Role::Button)
        .map(|n| n.accesskit_node().label().unwrap_or_default())
        .collect()
}

/// Clicks the reappear time badge of the row `row` of the editor.
fn click_badge(harness: &mut Harness<'_, App>, row: usize) -> TestResult {
    harness
        .query_all_by_label_contains("Reappear time: ")
        .filter(|n| n.accesskit_node().role() == Role::Button)
        .nth(row)
        .ok_or(format!("badge {row}"))?
        .click_accesskit();
    harness.run();
    Ok(())
}

#[test]
fn the_line_badge_sets_the_reappear_time_and_the_menu_deletes_for_good() -> TestResult {
    let (store, note, _) = with_note()?;
    ShoppingLists::new(&store).make(&note)?;
    let mut harness = window(&store, Some(&note));
    // A new Line has none (#195).
    assert_eq!(badges(&harness), ["Reappear time: None"; 3]);
    click_badge(&mut harness, 0)?;
    assert!(shows(&harness, "Reappear time"));
    for label in ["None", "Once", "After"] {
        assert!(shows(&harness, label), "{label}");
    }
    assert!(!shows(&harness, "week"), "units only for After");
    let milk = line_id(&store, &note, "Milk")?;
    let reappear = |harness: &Harness<'_, App>| -> Result<Reappear, Box<dyn std::error::Error>> {
        Ok(list(harness, &note)?.line(&milk).ok_or("Milk")?.reappear)
    };
    // After starts at a week, each pick written at once.
    click(&mut harness, "After")?;
    assert_eq!(reappear(&harness)?, Reappear::FALLBACK);
    for unit in ["day", "week", "month", "year"] {
        assert!(shows(&harness, unit), "{unit}");
    }
    click(&mut harness, "More")?;
    click(&mut harness, "More")?;
    assert_eq!(reappear(&harness)?, Reappear::After(3, ReappearUnit::Week));
    click(&mut harness, "months")?;
    assert_eq!(reappear(&harness)?, Reappear::After(3, ReappearUnit::Month));
    click(&mut harness, "Close")?;
    assert!(!shows(&harness, "After"));
    assert_eq!(badges(&harness)[0], "Reappear time: 3 months");
    tick(&mut harness, "Milk")?;
    assert!(shows(&harness, "back Sun, Jan 24"));
    // Back to none: struck until a time is set.
    click_badge(&mut harness, 0)?;
    click(&mut harness, "None")?;
    assert_eq!(reappear(&harness)?, Reappear::Unset);
    click(&mut harness, "Close")?;
    assert!(list(&harness, &note)?.line(&milk).ok_or("Milk")?.struck);
    assert!(!shows_part(&harness, "back "));

    // Once: gone when checked.
    click_badge(&mut harness, 2)?;
    click(&mut harness, "Once")?;
    click(&mut harness, "Close")?;
    tick(&mut harness, "Apples")?;
    assert_eq!(texts(&harness, &note)?, ["Bread", "✓ Milk"]);

    // Renaming is typing in place, the reappear time is the badge's: the
    // menu has neither.
    click(&mut harness, "More: Bread")?;
    assert!(!shows(&harness, "Rename"));
    assert!(!shows(&harness, "Once"));
    harness.key_press(egui::Key::Escape);
    harness.run();

    // Deleting asks nothing (#268): the Line folds under "Removed
    // Lines", whose Restore brings it back, and so does Ctrl+Z.
    click(&mut harness, "More: Bread")?;
    click(&mut harness, "Delete")?;
    assert!(!shows(&harness, "Delete this Line?"));
    assert_eq!(texts(&harness, &note)?, ["✓ Milk"]);
    click(&mut harness, "Removed Lines")?;
    assert!(shows(&harness, "Bread"));
    click(&mut harness, "Restore")?;
    assert_eq!(texts(&harness, &note)?, ["Bread", "✓ Milk"]);
    click(&mut harness, "More: Bread")?;
    click(&mut harness, "Delete")?;
    assert_eq!(texts(&harness, &note)?, ["✓ Milk"]);
    harness.key_press_modifiers(egui::Modifiers::COMMAND, egui::Key::Z);
    harness.run();
    assert_eq!(texts(&harness, &note)?, ["Bread", "✓ Milk"]);
    Ok(())
}

#[test]
fn a_line_just_typed_has_its_menu_once_it_holds_text_and_each_entry_stores_it_first() -> TestResult
{
    let (store, note, _) = with_note()?;
    ShoppingLists::new(&store).make(&note)?;
    let mut harness = window(&store, Some(&note));
    // An empty new Line has nothing to act on (#202).
    to_end_of(&mut harness, "Apples")?;
    press(&mut harness, egui::Key::Enter);
    assert_eq!(focused(&harness).as_deref(), Some(""));
    assert!(!shows(&harness, "More: "));
    type_line(&mut harness, "Tea");
    assert_eq!(list(&harness, &note)?.count(), 3);
    click_badge(&mut harness, 3)?;
    click(&mut harness, "Once")?;
    click(&mut harness, "Close")?;
    let tea = line_id(&store, &note, "Tea")?;
    assert_eq!(
        list(&harness, &note)?.line(&tea).ok_or("Tea")?.reappear,
        Reappear::Once
    );

    to_end_of(&mut harness, "Tea")?;
    press(&mut harness, egui::Key::Enter);
    type_line(&mut harness, "Salt");
    click(&mut harness, "More: Salt")?;
    click(&mut harness, "Delete")?;
    assert_eq!(texts(&harness, &note)?, ["Milk", "Bread", "Apples", "Tea"]);
    assert!(!shows(&harness, "More: Salt"));
    Ok(())
}

#[test]
fn back_to_a_plain_note_marks_the_struck_lines() -> TestResult {
    let (store, note, _) = with_note()?;
    let lists = ShoppingLists::new(&store);
    lists.make(&note)?;
    lists.check(&line_id(&store, &note, "Bread")?, &utc)?;
    let mut harness = window(&store, Some(&note));
    // In the Note dialog's kind badge (#160).
    click(&mut harness, "Kind: Shopping List")?;
    pick(&mut harness, "Plain Note")?;
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
    assert!(shows(&harness, "Kind: Plain Note"));
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
    weekly(&store, &note)?;
    let mut harness = window(&store, None);
    assert!(!checked(&harness, "Bread")?);
    tick(&mut harness, "Bread")?;
    assert_eq!(texts(&harness, &note)?, ["Milk", "Apples", "✓ Bread"]);
    // Struck, it folds under the open Lines until a click shows it (#189).
    assert!(boxes(&harness, "Bread").is_empty());
    click(&mut harness, "1 struck through")?;
    assert!(checked(&harness, "Bread")?);
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
    // Ann's Lines show as words, not to be renamed.
    assert!(line_field(&harness, "Bread").is_err());
    type_line(&mut harness, "Water");
    press(&mut harness, egui::Key::Enter);
    // His own Line is his to change, in place too.
    assert!(shows(&harness, "More: Water"));
    assert!(line_field(&harness, "Water").is_ok());
    // His badge of Ann's Lines only shows; his own opens its dialog.
    assert_eq!(badges(&harness), ["Reappear time: None"]);
    assert!(harness.query_all_by_label("Reappear time: None").count() > 3);
    click_badge(&mut harness, 0)?;
    assert!(shows(&harness, "After"));
    click(&mut harness, "Close")?;
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

fn drop_file(harness: &mut Harness<'_, App>, path: &std::path::Path) {
    harness.input_mut().dropped_files = vec![egui::DroppedFile {
        path: Some(path.to_path_buf()),
        ..Default::default()
    }];
    harness.run();
}

#[test]
fn an_update_that_makes_a_shopping_list_says_so_and_shows_each_reappear_time() -> TestResult {
    let (ann, _) = device(4, "Ann")?;
    let note = Notes::new(&ann).capture(SHOPPING, None, None)?;
    let dir = tempfile::tempdir()?;
    let first = dir.path().join("first.oldbrain");
    std::fs::write(&first, ann.send_note(&note.id, None, false)?)?;
    let (desk, _) = device(1, "Ada")?;
    let desk = Arc::new(desk);
    let mut harness = window(&desk, None);
    drop_file(&mut harness, &first);
    click(&mut harness, "Show")?;
    click(&mut harness, "Accept")?;
    assert!(!harness.state().previewing());

    let lists = ShoppingLists::new(&ann);
    lists.make(&note.id)?;
    lists.set_reappear(&line_id(&ann, &note.id, "Bread")?, Reappear::Once)?;
    let update = dir.path().join("update.oldbrain");
    std::fs::write(&update, ann.send_note(&note.id, None, false)?)?;
    drop_file(&mut harness, &update);
    assert!(harness.state().previewing());
    assert!(shows(&harness, "It becomes a Shopping List."));
    // Its Lines, each with the reappear time it comes with.
    for line in ["Milk", "Bread", "Apples"] {
        assert!(!boxes(&harness, line).is_empty(), "{line}");
    }
    assert!(shows_part(&harness, "Reappear time: Once"));
    assert_eq!(
        harness
            .query_all_by_label_contains("Reappear time: None")
            .count(),
        2
    );
    Ok(())
}

#[test]
fn the_reappear_time_reads_in_both_languages() {
    use oldbrainfix_gui::shopping_view::reappear_label;
    let en = L10n::new("en");
    let de = L10n::new("de");
    use oldbrainfix_gui::shopping_view::reappear_short;
    assert_eq!(reappear_label(&en, Reappear::Once), "Once");
    assert_eq!(reappear_label(&en, Reappear::Unset), "None");
    assert_eq!(
        reappear_label(&en, Reappear::After(1, ReappearUnit::Week)),
        "1 week"
    );
    assert_eq!(
        reappear_label(&en, Reappear::After(3, ReappearUnit::Month)),
        "3 months"
    );
    assert_eq!(
        reappear_label(&de, Reappear::After(4, ReappearUnit::Week)),
        "4 Wochen"
    );
    assert_eq!(
        reappear_label(&de, Reappear::After(2, ReappearUnit::Day)),
        "2 Tage"
    );
    assert_eq!(
        reappear_label(&de, Reappear::After(1, ReappearUnit::Year)),
        "1 Jahr"
    );
    assert_eq!(reappear_label(&de, Reappear::Once), "Einmal");
    assert_eq!(reappear_label(&de, Reappear::Unset), "Keine");
    let short = |t: &L10n, r| reappear_short(t, r);
    assert_eq!(short(&en, Reappear::Unset), "—");
    assert_eq!(short(&en, Reappear::After(4, ReappearUnit::Week)), "4w");
    assert_eq!(short(&en, Reappear::After(3, ReappearUnit::Month)), "3m");
    assert_eq!(short(&en, Reappear::After(1, ReappearUnit::Day)), "1d");
    assert_eq!(short(&en, Reappear::After(20, ReappearUnit::Year)), "20y");
    assert_eq!(short(&de, Reappear::After(4, ReappearUnit::Week)), "4W");
    assert_eq!(short(&de, Reappear::After(1, ReappearUnit::Year)), "1J");
    assert_eq!(short(&de, Reappear::Once), "Einmal");
}

/// The start of Saturday, 31 October 2026, on UTC: the day a Line
/// checked at [`NOW`] comes back.
const BACK: &str = "2026-10-31T00:00:01Z";

/// [`SHOPPING`] made a Shopping List with "Bread" checked at [`NOW`],
/// the clock at [`BACK`] so all three Lines are open again, and
/// `prepare` run on the store and the Note: the window on the Agenda and
/// the Note's id.
fn agenda_with(
    prepare: impl FnOnce(&Store, &str) -> TestResult,
) -> Result<(Harness<'static, App>, String), Box<dyn std::error::Error>> {
    let (store, note, moment) = with_note()?;
    let lists = ShoppingLists::new(&store);
    lists.make(&note)?;
    weekly(&store, &note)?;
    lists.check(&line_id(&store, &note, "Bread")?, &utc)?;
    moment.store(
        DateTime::parse_from_rfc3339(BACK)?.timestamp(),
        Ordering::SeqCst,
    );
    prepare(&store, &note)?;
    let mut harness = window(&store, None);
    harness.state_mut().select(Selection::Agenda);
    harness.run();
    Ok((harness, note))
}

/// As [`agenda_with`], the list showing on the Agenda as `shows` was
/// chosen for it, or as nobody chose.
fn agenda(
    shows: Option<ShoppingOnAgenda>,
) -> Result<(Harness<'static, App>, String), Box<dyn std::error::Error>> {
    agenda_with(|store, note| {
        if let Some(shows) = shows {
            ShoppingLists::new(store).set_on_agenda(note, shows)?;
        }
        Ok(())
    })
}

/// The Agenda's message for a day with nothing due.
const NOTHING_TODAY: &str = "Nothing due today. Click + to capture a thought.";

/// The words of the three ways onto the Agenda.
const OFF: &str = "Off";
const COUNT: &str = "One row with the number of open Lines";
const LINES: &str = "Each open Line on its own row";

#[test]
fn by_default_the_agenda_has_one_row_with_the_open_lines_of_the_list() -> TestResult {
    let (mut harness, note) = agenda(None)?;
    assert!(shows(&harness, "Weekly: 3 open"));
    assert!(boxes(&harness, "Bread").is_empty());
    assert!(!shows(&harness, NOTHING_TODAY));
    harness
        .get_by_role_and_label(Role::Button, "Weekly: 3 open")
        .click();
    harness.run();
    assert_eq!(harness.state().selected_note(), Some(note.as_str()));
    Ok(())
}

#[test]
fn the_lines_choice_checks_a_line_on_the_agenda_and_it_stays_struck_for_the_day() -> TestResult {
    let (mut harness, note) = agenda(Some(ShoppingOnAgenda::Lines))?;
    // The list's name as the heading, its open Lines under it (#208).
    let heading = harness.get_by_role_and_label(Role::Button, "Weekly").rect();
    for line in ["Milk", "Bread", "Apples"] {
        let words = harness.get_by_role_and_label(Role::Button, line).rect();
        let tick = boxes(&harness, line).pop().ok_or(line)?.rect();
        assert!(words.top() > heading.top(), "{line} under the heading");
        assert!(tick.left() >= heading.left() - 0.5, "{line} indented");
    }
    assert!(!shows_part(&harness, "open"));
    assert!(!checked(&harness, "Bread")?);
    tick(&mut harness, "Bread")?;
    assert_eq!(texts(&harness, &note)?, ["Milk", "Apples", "✓ Bread"]);
    // Struck under the open ones (#276); its box brings it back.
    assert!(checked(&harness, "Bread")?);
    assert!(!checked(&harness, "Milk")?);
    let bread = harness.get_by_role_and_label(Role::Button, "Bread").rect();
    let apples = harness.get_by_role_and_label(Role::Button, "Apples").rect();
    assert!(apples.top() < bread.top());
    tick(&mut harness, "Bread")?;
    assert_eq!(texts(&harness, &note)?, ["Milk", "Bread", "Apples"]);
    assert!(!checked(&harness, "Bread")?);
    Ok(())
}

#[test]
fn a_line_checked_once_on_the_agenda_stays_struck_and_its_box_brings_it_back() -> TestResult {
    let (mut harness, note) = agenda_with(|store, note| {
        let lists = ShoppingLists::new(store);
        lists.set_on_agenda(note, ShoppingOnAgenda::Lines)?;
        lists.set_reappear(&line_id(store, note, "Milk")?, Reappear::Once)?;
        Ok(())
    })?;
    tick(&mut harness, "Milk")?;
    // Gone from the list, still struck on the Agenda for the day.
    assert_eq!(texts(&harness, &note)?, ["Bread", "Apples"]);
    assert!(checked(&harness, "Milk")?);
    tick(&mut harness, "Milk")?;
    assert_eq!(texts(&harness, &note)?, ["Milk", "Bread", "Apples"]);
    assert!(!checked(&harness, "Milk")?);
    Ok(())
}

#[test]
fn every_line_checked_today_stays_struck_but_its_count_does_not_show() -> TestResult {
    let (mut harness, note) = agenda(Some(ShoppingOnAgenda::Lines))?;
    for line in ["Milk", "Bread", "Apples"] {
        tick(&mut harness, line)?;
    }
    assert!(shows_weekly(&harness));
    for line in ["Milk", "Bread", "Apples"] {
        assert!(checked(&harness, line)?, "{line}");
    }
    assert!(!shows(&harness, NOTHING_TODAY));
    // As its count, with none open, the list does not show.
    ShoppingLists::new(harness.state().store()).set_on_agenda(&note, ShoppingOnAgenda::Count)?;
    harness.state_mut().select(Selection::All);
    harness.run();
    harness.state_mut().select(Selection::Agenda);
    harness.run();
    assert!(!shows_weekly(&harness));
    assert!(shows(&harness, NOTHING_TODAY));
    Ok(())
}

#[test]
fn hiding_whats_checked_takes_a_checked_line_off_the_agenda_at_once() -> TestResult {
    let (mut harness, note) = agenda_with(|store, note| {
        ShoppingLists::new(store).set_on_agenda(note, ShoppingOnAgenda::Lines)?;
        oldbrainfix_core::DeviceSettings::new(store).set_agenda_hides_checked(true)?;
        Ok(())
    })?;
    tick(&mut harness, "Bread")?;
    assert_eq!(texts(&harness, &note)?, ["Milk", "Apples", "✓ Bread"]);
    assert!(boxes(&harness, "Bread").is_empty());
    assert!(!boxes(&harness, "Milk").is_empty());
    Ok(())
}

#[test]
fn the_heading_opens_the_list_in_a_new_line_before_the_struck_ones() -> TestResult {
    let (mut harness, note) = agenda(Some(ShoppingOnAgenda::Lines))?;
    tick(&mut harness, "Bread")?;
    harness
        .get_by_role_and_label(Role::Button, "Weekly")
        .click();
    harness.run();
    assert_eq!(harness.state().selected_note(), Some(note.as_str()));
    assert_eq!(focused(&harness).as_deref(), Some(""));
    assert_eq!(rows(&harness), ["Milk", "Apples", "", "Bread"]);
    harness.get_by(|n| n.is_focused()).type_text("Tea");
    harness.run();
    harness.state_mut().select(Selection::Agenda);
    harness.run();
    assert_eq!(
        texts(&harness, &note)?,
        ["Milk", "Apples", "Tea", "✓ Bread"]
    );
    // Left unused, the new Line is dropped.
    harness
        .get_by_role_and_label(Role::Button, "Weekly")
        .click();
    harness.run();
    assert_eq!(focused(&harness).as_deref(), Some(""));
    harness.state_mut().select(Selection::Agenda);
    harness.run();
    assert_eq!(
        texts(&harness, &note)?,
        ["Milk", "Apples", "Tea", "✓ Bread"]
    );
    Ok(())
}

#[test]
fn a_line_row_opens_its_note() -> TestResult {
    let (mut harness, note) = agenda(Some(ShoppingOnAgenda::Lines))?;
    harness.get_by_role_and_label(Role::Button, "Milk").click();
    harness.run();
    assert_eq!(harness.state().selected_note(), Some(note.as_str()));
    Ok(())
}

#[test]
fn the_heading_of_a_list_with_its_lines_opens_its_note() -> TestResult {
    let (mut harness, note) = agenda(Some(ShoppingOnAgenda::Lines))?;
    harness
        .get_by_role_and_label(Role::Button, "Weekly")
        .click();
    harness.run();
    assert_eq!(harness.state().selected_note(), Some(note.as_str()));
    Ok(())
}

#[test]
fn the_off_choice_shows_no_shopping_list() -> TestResult {
    let (harness, _) = agenda(Some(ShoppingOnAgenda::Off))?;
    assert!(!shows_weekly(&harness));
    assert!(boxes(&harness, "Bread").is_empty());
    assert!(shows(&harness, NOTHING_TODAY));
    Ok(())
}

#[test]
fn a_list_without_open_lines_shows_only_its_lines_checked_today_one_by_one() -> TestResult {
    let ways = [
        (None, false),
        (Some(ShoppingOnAgenda::Lines), false),
        (Some(ShoppingOnAgenda::Lines), true),
    ];
    for (shows_as, hides) in ways {
        let (harness, _) = agenda_with(|store, note| {
            let lists = ShoppingLists::new(store);
            if let Some(shows) = shows_as {
                lists.set_on_agenda(note, shows)?;
            }
            oldbrainfix_core::DeviceSettings::new(store).set_agenda_hides_checked(hides)?;
            for line in ["Milk", "Bread", "Apples"] {
                lists.check(&line_id(store, note, line)?, &utc)?;
            }
            Ok(())
        })?;
        // Struck for the day where each Line shows and nothing checked
        // is hidden (#276); never as a count of none open.
        let shown = shows_as.is_some() && !hides;
        let way = format!("{shows_as:?}, hiding {hides}");
        assert_eq!(shows_weekly(&harness), shown, "{way}");
        assert_eq!(shows(&harness, NOTHING_TODAY), !shown, "{way}");
        if shown {
            assert!(checked(&harness, "Milk")?, "{way}");
        }
    }
    Ok(())
}

#[test]
fn a_list_nobody_chose_for_shows_as_this_device_showed_shopping_lists() -> TestResult {
    // As a version before #186 kept it on this Device.
    let old = |way: &'static str| {
        move |store: &Store, _: &str| -> TestResult {
            store.set_local_setting("shoppingAgenda", way)?;
            Ok(())
        }
    };
    let (harness, _) = agenda_with(old("off"))?;
    assert!(!shows_weekly(&harness));
    let (harness, _) = agenda_with(old("lines"))?;
    assert!(shows(&harness, "Weekly"));
    assert!(!boxes(&harness, "Bread").is_empty());
    let (harness, _) = agenda_with(old("back"))?;
    assert!(shows(&harness, "Weekly: 3 open"));
    Ok(())
}

#[test]
fn the_shopping_rows_sit_after_the_all_day_occurrences_before_the_timed() -> TestResult {
    let (mut harness, _) = agenda(None)?;
    let day = Day::new(2026, 10, 31);
    let nine: Vec<ClockTime> = ClockTime::new(9, 0).into_iter().collect();
    let routines = Routines::new(harness.state().store());
    routines.create(
        "Water the plants",
        &Schedule::new(day, &[], None).ok_or("all day")?,
    )?;
    routines.create(
        "Call Dana",
        &Schedule::new(day, &nine, None).ok_or("at nine")?,
    )?;
    harness.state_mut().select(Selection::All);
    harness.run();
    harness.state_mut().select(Selection::Agenda);
    harness.run();
    let shopping = top(&harness, "Weekly: 3 open")?;
    assert!(top(&harness, "Water the plants")? < shopping);
    assert!(shopping < top(&harness, "Call Dana")?);
    Ok(())
}

/// The top of the first widget labelled `label`.
fn top(harness: &Harness<'_, App>, label: &str) -> Result<f32, Box<dyn std::error::Error>> {
    Ok(harness
        .query_all_by_label(label)
        .next()
        .ok_or(format!("{label:?}"))?
        .rect()
        .top())
}

/// Whether the last radio button `label` shows chosen, or `None` while
/// none shows.
fn radio(harness: &Harness<'_, App>, label: &str) -> Option<bool> {
    harness
        .query_all_by_role_and_label(Role::RadioButton, label)
        .last()
        .map(|n| n.accesskit_node().toggled() == Some(Toggled::True))
}

/// Clicks the last radio button `label`.
fn pick(harness: &mut Harness<'_, App>, label: &str) -> TestResult {
    harness
        .query_all_by_role_and_label(Role::RadioButton, label)
        .last()
        .ok_or(format!("{label:?}"))?
        .click_accesskit();
    harness.run();
    Ok(())
}

#[test]
fn the_cards_menu_chooses_how_the_list_shows_on_the_agenda() -> TestResult {
    let (store, note, _) = with_note()?;
    let lists = ShoppingLists::new(&store);
    lists.make(&note)?;
    let mut harness = window(&store, None);
    click(&mut harness, "More: Weekly")?;
    assert!(shows(&harness, "Show on the Agenda"));
    assert_eq!(radio(&harness, OFF), Some(false));
    assert_eq!(radio(&harness, COUNT), Some(true));
    assert_eq!(radio(&harness, LINES), Some(false));
    // No switch as a Checklist's, and before "Back to a plain Note".
    assert!(
        harness
            .query_all_by_role_and_label(Role::CheckBox, "Show on the Agenda")
            .next()
            .is_none()
    );
    let plain = top(&harness, "Back to a plain Note")?;
    let off = harness
        .query_all_by_role_and_label(Role::RadioButton, OFF)
        .last()
        .ok_or("Off")?
        .rect()
        .top();
    assert!(off < plain);
    pick(&mut harness, LINES)?;
    assert_eq!(lists.on_agenda(&note)?, ShoppingOnAgenda::Lines);
    assert_eq!(radio(&harness, LINES), None, "the menu closed");
    harness.state_mut().select(Selection::Agenda);
    harness.run();
    assert!(!boxes(&harness, "Milk").is_empty());
    harness.state_mut().select(Selection::All);
    harness.run();
    click(&mut harness, "More: Weekly")?;
    assert_eq!(radio(&harness, LINES), Some(true));
    pick(&mut harness, OFF)?;
    assert_eq!(lists.on_agenda(&note)?, ShoppingOnAgenda::Off);
    harness.state_mut().select(Selection::Agenda);
    harness.run();
    assert!(!shows_weekly(&harness));
    Ok(())
}

#[test]
fn the_note_dialogs_badge_chooses_how_the_list_shows_on_the_agenda() -> TestResult {
    let (store, note, _) = with_note()?;
    let lists = ShoppingLists::new(&store);
    lists.make(&note)?;
    let mut harness = window(&store, Some(&note));
    let badge = |words: &str| format!("Show on the Agenda: {words}");
    assert!(shows(&harness, &badge(COUNT)));
    click(&mut harness, &badge(COUNT))?;
    assert_eq!(radio(&harness, COUNT), Some(true));
    pick(&mut harness, LINES)?;
    assert_eq!(lists.on_agenda(&note)?, ShoppingOnAgenda::Lines);
    // A pick closes it, and the badge follows.
    assert_eq!(radio(&harness, COUNT), None);
    assert!(shows(&harness, &badge(LINES)));
    click(&mut harness, &badge(LINES))?;
    pick(&mut harness, OFF)?;
    assert!(shows(&harness, &badge(OFF)));
    assert_eq!(lists.on_agenda(&note)?, ShoppingOnAgenda::Off);
    Ok(())
}

#[test]
fn a_plain_note_has_no_shopping_choice() -> TestResult {
    let (store, note, _) = with_note()?;
    let harness = window(&store, Some(&note));
    assert!(!shows_part(&harness, "Show on the Agenda"));
    let mut harness = window(&store, None);
    click(&mut harness, "More: Weekly")?;
    assert_eq!(radio(&harness, COUNT), None);
    Ok(())
}

#[test]
fn the_settings_have_no_shopping_lists_on_the_agenda_any_more() -> TestResult {
    let (store, _, _) = with_note()?;
    let mut harness = window(&store, None);
    harness.state_mut().open(Page::Settings);
    harness.run();
    assert!(!shows(&harness, "Shopping Lists on the Agenda"));
    assert!(!shows_part(&harness, "One row per Shopping List"));
    Ok(())
}
