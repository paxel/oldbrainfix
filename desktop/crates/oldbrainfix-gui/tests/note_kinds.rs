//! A Note is its kind (#188), driven headless by egui_kittest: in the
//! Note dialog a Checklist, a Shopping List and a Tally show their text
//! as a name on top, one line, and a description under it folded to one
//! line, a click unfolding it, then the kind's content, the badges and
//! the Sections; a plain Note keeps its one text field; the name and the
//! description are written back as one text, nothing when neither
//! changed; the kind badge of every Note, a plain one's too, changes the
//! kind with radio buttons, the Lines carried over between the two
//! lists; read only, the description unfolds and folds again; and a
//! full card shows the name as its title and the description folded,
//! a click on it unfolding it without taking the card's click.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, Duration, Utc};
use egui::Key;
use egui::accesskit::Role;
use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::{Harness, Node};
use oldbrainfix_core::notes::keys as note_keys;
use oldbrainfix_core::{
    Checklists, Notes, OwnProfile, Places, ShoppingLists, SigningKey, Store, Tallies,
};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A store on a clock that moves a microsecond per read, with the Area
/// Elsewhere, which none of the Notes is in: the store and the Area.
fn store() -> Result<(Arc<Store>, String), Box<dyn std::error::Error>> {
    let start = DateTime::parse_from_rfc3339("2026-03-02T11:00:00Z")?.with_timezone(&Utc);
    let ticks = Arc::new(AtomicI64::new(0));
    let clock =
        Box::new(move || start + Duration::microseconds(ticks.fetch_add(1, Ordering::SeqCst)));
    let store = Store::in_memory(clock, || Ok(SigningKey::from_seed([6; 32])))?;
    OwnProfile::new(&store).apply_defaults("Desk", None)?;
    let elsewhere = Places::new(&store).add_area("Elsewhere")?.id;
    Ok((Arc::new(store), elsewhere))
}

/// The window over `store` in English showing `selection`.
fn window(store: &Arc<Store>, selection: Selection) -> Harness<'static, App> {
    let mut app = App::new(store.clone(), "en");
    app.set_region(Some("DE"));
    app.select(selection);
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
    harness
}

/// The Note dialog open on `note` over a list that does not show it.
fn dialog(store: &Arc<Store>, elsewhere: &str, note: &str) -> Harness<'static, App> {
    let mut harness = window(store, Selection::Place(elsewhere.to_string()));
    harness.state_mut().select_note(note);
    harness.run();
    harness
}

fn shows(harness: &Harness<'_, App>, label: &str) -> bool {
    harness.query_all_by_label(label).next().is_some()
}

fn click(harness: &mut Harness<'_, App>, label: &str) -> TestResult {
    harness
        .query_all_by_label(label)
        .last()
        .ok_or(format!("{label:?}"))?
        .click_accesskit();
    harness.run();
    Ok(())
}

/// Picks the radio button `label` of the open small editor.
fn pick(harness: &mut Harness<'_, App>, label: &str) {
    harness
        .get_by_role_and_label(Role::RadioButton, label)
        .click_accesskit();
    harness.run();
}

fn typed(harness: &mut Harness<'_, App>, text: &str) {
    harness.get_by(|n| n.is_focused()).type_text(text);
    harness.run();
}

fn text_of(store: &Store, id: &str) -> Result<String, Box<dyn std::error::Error>> {
    Ok(Notes::new(store).by_id(id)?.ok_or("the Note")?.text)
}

/// How many texts the log holds for the Note `id`.
fn text_entries(store: &Store, id: &str) -> Result<usize, Box<dyn std::error::Error>> {
    Ok(store
        .history(id)?
        .iter()
        .filter(|e| e.field == note_keys::TEXT)
        .count())
}

/// The name field of the Note dialog.
fn name_field<'a>(harness: &'a Harness<'_, App>) -> Node<'a> {
    harness.get_by_role_and_label(Role::TextInput, "Name")
}

/// The top of the first widget labelled or holding `text`.
fn top(harness: &Harness<'_, App>, text: &str) -> Result<f32, Box<dyn std::error::Error>> {
    Ok(harness
        .query_all_by(|n| n.label().as_deref() == Some(text) || n.value().as_deref() == Some(text))
        .map(|n| n.rect().top())
        .reduce(f32::min)
        .ok_or(format!("{text:?}"))?)
}

/// What a Note of each kind is made with: the kind's badge value and a
/// way to make the Note `id` one, its content showing "Passport".
type Making = (&'static str, fn(&Store, &str) -> TestResult);

const KINDS: [Making; 3] = [
    (
        "Checklist",
        |store, id| Ok(Checklists::new(store).make(id)?),
    ),
    ("Shopping List", |store, id| {
        Ok(ShoppingLists::new(store).make(id)?)
    }),
    ("Tally", |store, id| {
        Tallies::new(store).add_field(id, "Passport", "reps", None, None)?;
        Ok(())
    }),
];

/// A Note "Packing" of the kind `making`, its description "for the
/// trip" and "and the way back" on the next line.
fn packing(store: &Store, making: &Making) -> Result<String, Box<dyn std::error::Error>> {
    let notes = Notes::new(store);
    let id = notes.capture("Packing\nPassport", None, None)?.id;
    if making.0 == "Tally" {
        notes.edit(&id, "Packing")?;
    }
    (making.1)(store, &id)?;
    notes.edit(&id, "Packing\nfor the trip\nand the way back")?;
    Ok(id)
}

#[test]
fn each_kind_shows_its_name_its_folded_description_its_content_then_the_rest() -> TestResult {
    for making in &KINDS {
        let (store, elsewhere) = store()?;
        let id = packing(&store, making)?;
        Notes::new(&store).amend(&id, "Socks bought")?;
        let mut harness = dialog(&store, &elsewhere, &id);
        let kind = making.0;
        // The name, one line, on top; no field of the whole text.
        assert_eq!(name_field(&harness).value().as_deref(), Some("Packing"));
        assert!(!shows(&harness, "Note text"), "{kind}");
        // The description folded to its first line.
        assert!(shows(&harness, "for the trip"), "{kind}");
        assert!(!shows(&harness, "and the way back"), "{kind}");
        let name = name_field(&harness).rect().bottom();
        let description = top(&harness, "for the trip")?;
        let content = top(&harness, "Passport")?;
        let badge = top(&harness, &format!("Kind: {kind}"))?;
        let section = top(&harness, "Socks bought")?;
        assert!(name <= description, "{kind}");
        assert!(description < content, "{kind}");
        assert!(content < badge, "{kind}");
        assert!(badge < section, "{kind}");
        // A click unfolds it into its field.
        click(&mut harness, "Show the whole description")?;
        let field = harness.get_by_role_and_label(Role::MultilineTextInput, "Description");
        assert_eq!(
            field.value().as_deref(),
            Some("for the trip\nand the way back"),
            "{kind}"
        );
        // Left, it folds again.
        name_field(&harness).click();
        harness.run();
        assert!(shows(&harness, "Show the whole description"), "{kind}");
        assert!(
            harness
                .query_all_by_role_and_label(Role::MultilineTextInput, "Description")
                .next()
                .is_none(),
            "{kind}"
        );
    }
    Ok(())
}

#[test]
fn a_plain_note_keeps_its_one_text_field_and_shows_its_kind() -> TestResult {
    let (store, elsewhere) = store()?;
    let id = Notes::new(&store)
        .capture("Call Ben\nabout the tap", None, None)?
        .id;
    let harness = dialog(&store, &elsewhere, &id);
    let field = harness.get_by_label("Note text");
    assert_eq!(field.value().as_deref(), Some("Call Ben\nabout the tap"));
    assert!(
        harness
            .query_all_by_role_and_label(Role::TextInput, "Name")
            .next()
            .is_none()
    );
    assert!(!shows(&harness, "Show the whole description"));
    assert!(shows(&harness, "Kind: Plain Note"));
    // Its kind badge is no longer above a "Make it a Checklist".
    assert!(!shows(&harness, "Make it a Checklist"));
    assert!(!shows(&harness, "Make it a Shopping List"));
    Ok(())
}

#[test]
fn the_name_and_the_description_are_written_back_as_one_text() -> TestResult {
    let (store, elsewhere) = store()?;
    let id = packing(&store, &KINDS[0])?;
    let mut harness = dialog(&store, &elsewhere, &id);
    name_field(&harness).click();
    harness.run();
    harness.key_press(Key::End);
    harness.run();
    typed(&mut harness, " list");
    // Enter goes on at the start of the description, no break in the
    // name.
    harness.key_press(Key::Enter);
    harness.run();
    let focused = harness.get_by(|n| n.is_focused());
    assert_eq!(
        focused.accesskit_node().label().as_deref(),
        Some("Description")
    );
    typed(&mut harness, "All ");
    harness.state_mut().close_note();
    harness.run();
    assert_eq!(
        text_of(&store, &id)?,
        "Packing list\nAll for the trip\nand the way back"
    );
    // An empty description leaves the name alone as the text.
    let mut harness = dialog(&store, &elsewhere, &id);
    click(&mut harness, "Show the whole description")?;
    harness.key_press_modifiers(egui::Modifiers::COMMAND, Key::A);
    harness.run();
    harness.key_press(Key::Backspace);
    harness.run();
    harness.state_mut().close_note();
    harness.run();
    assert_eq!(text_of(&store, &id)?, "Packing list");
    Ok(())
}

#[test]
fn an_emptied_name_gets_the_stored_one_back() -> TestResult {
    let (store, elsewhere) = store()?;
    let id = packing(&store, &KINDS[0])?;
    let stored = text_of(&store, &id)?;
    let mut harness = dialog(&store, &elsewhere, &id);
    name_field(&harness).click();
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::COMMAND, Key::A);
    harness.run();
    harness.key_press(Key::Backspace);
    harness.run();
    let emptied = name_field(&harness).accesskit_node().value();
    assert_eq!(emptied.as_deref().unwrap_or(""), "");
    harness.state_mut().close_note();
    harness.run();
    // Nothing of the description becomes the name.
    assert_eq!(text_of(&store, &id)?, stored);
    Ok(())
}

#[test]
fn opening_and_leaving_a_note_of_a_kind_writes_nothing() -> TestResult {
    let (store, elsewhere) = store()?;
    let id = packing(&store, &KINDS[1])?;
    // A text whose break the join would write otherwise.
    Notes::new(&store).edit(&id, "Packing\r\nfor the trip")?;
    let before = text_entries(&store, &id)?;
    let mut harness = dialog(&store, &elsewhere, &id);
    name_field(&harness).click();
    harness.run();
    click(&mut harness, "Show the whole description")?;
    harness.state_mut().close_note();
    harness.run();
    assert_eq!(text_entries(&store, &id)?, before);
    assert_eq!(text_of(&store, &id)?, "Packing\r\nfor the trip");
    Ok(())
}

#[test]
fn the_kind_badge_changes_the_kind_the_lines_carried_over() -> TestResult {
    let (store, elsewhere) = store()?;
    let id = Notes::new(&store)
        .capture("Packing\nPassport\nTicket", None, None)?
        .id;
    let wall = |at: DateTime<Utc>| at.naive_utc();
    let mut harness = dialog(&store, &elsewhere, &id);
    // A plain Note's kinds as radio buttons, its own picked.
    click(&mut harness, "Kind: Plain Note")?;
    for kind in ["Plain Note", "Checklist", "Shopping List", "Tally"] {
        assert!(
            harness
                .query_all_by_role_and_label(Role::RadioButton, kind)
                .next()
                .is_some(),
            "{kind}"
        );
    }
    pick(&mut harness, "Checklist");
    let list = Checklists::new(&store).by_note(&id)?.ok_or("a Checklist")?;
    let lines: Vec<&str> = list.lines.iter().map(|l| l.text.as_str()).collect();
    assert_eq!(lines, ["Passport", "Ticket"]);
    assert!(shows(&harness, "Kind: Checklist"));
    assert_eq!(name_field(&harness).value().as_deref(), Some("Packing"));
    // A Checklist becomes a Shopping List, its Lines carried over, a
    // checked one checked.
    Checklists::new(&store).check(&list.lines[1].id)?;
    harness.state_mut().close_note();
    harness.state_mut().select_note(&id);
    harness.run();
    click(&mut harness, "Kind: Checklist")?;
    assert!(
        harness
            .query_all_by_role_and_label(Role::RadioButton, "Tally")
            .next()
            .is_none()
    );
    pick(&mut harness, "Shopping List");
    assert!(Checklists::new(&store).by_note(&id)?.is_none());
    let list = ShoppingLists::new(&store)
        .by_note(&id, &wall)?
        .ok_or("a Shopping List")?;
    let lines: Vec<(&str, bool)> = list
        .lines
        .iter()
        .map(|l| (l.text.as_str(), l.struck))
        .collect();
    assert_eq!(lines, [("Passport", false), ("Ticket", true)]);
    assert!(shows(&harness, "Kind: Shopping List"));
    // Each Line once, in its new list.
    assert_eq!(
        harness
            .query_all_by(
                |n| n.role() == Role::TextInput && n.value().as_deref() == Some("Passport")
            )
            .count(),
        1
    );
    // And back to a Checklist.
    click(&mut harness, "Kind: Shopping List")?;
    pick(&mut harness, "Checklist");
    let list = Checklists::new(&store).by_note(&id)?.ok_or("a Checklist")?;
    let lines: Vec<&str> = list.lines.iter().map(|l| l.text.as_str()).collect();
    assert_eq!(lines, ["Passport", "Ticket"]);
    // And a plain Note again, its one field holding the whole text.
    click(&mut harness, "Kind: Checklist")?;
    pick(&mut harness, "Plain Note");
    assert!(Checklists::new(&store).by_note(&id)?.is_none());
    assert!(shows(&harness, "Kind: Plain Note"));
    let field = harness.get_by_label("Note text");
    assert_eq!(
        field.value().as_deref(),
        Some(text_of(&store, &id)?.as_str())
    );
    Ok(())
}

#[test]
fn a_tallys_kind_badge_only_shows_it() -> TestResult {
    let (store, elsewhere) = store()?;
    let id = packing(&store, &KINDS[2])?;
    let harness = dialog(&store, &elsewhere, &id);
    let badge = harness.get_by_label("Kind: Tally");
    assert_eq!(badge.accesskit_node().role(), Role::Label);
    Ok(())
}

#[test]
fn read_only_the_description_unfolds_and_folds_again() -> TestResult {
    let (store, _) = store()?;
    let id = packing(&store, &KINDS[0])?;
    Notes::new(&store).trash(&id)?;
    let mut harness = window(&store, Selection::Trash);
    harness.state_mut().select_note(&id);
    harness.run();
    assert!(
        harness
            .query_all_by_role_and_label(Role::TextInput, "Name")
            .next()
            .is_none()
    );
    assert!(shows(&harness, "for the trip"));
    assert!(!shows(&harness, "and the way back"));
    click(&mut harness, "Show the whole description")?;
    assert!(
        harness
            .query_all_by(|n| n.value().is_some_and(|v| v.contains("and the way back")))
            .next()
            .is_some()
    );
    click(&mut harness, "Fold the description")?;
    assert!(shows(&harness, "Show the whole description"));
    Ok(())
}

#[test]
fn a_full_card_shows_the_name_as_its_title_and_unfolds_the_description() -> TestResult {
    let (store, _) = store()?;
    packing(&store, &KINDS[0])?;
    let mut harness = window(&store, Selection::All);
    assert!(shows(&harness, "Packing"));
    assert!(shows(&harness, "for the trip"));
    assert!(!shows(&harness, "and the way back"));
    // A pointer click on it unfolds it, no click of the card's.
    harness.get_by_label("Show the whole description").click();
    harness.run();
    assert!(
        harness
            .query_all_by(|n| n.value().is_some_and(|v| v.contains("and the way back")))
            .next()
            .is_some()
    );
    assert!(harness.state().selected_cards().is_empty());
    assert!(!harness.state().note_open());
    harness.get_by_label("Fold the description").click();
    harness.run();
    assert!(!shows(&harness, "and the way back"));
    assert!(harness.state().selected_cards().is_empty());
    // The name is the card's title, above the description and the Lines.
    assert!(top(&harness, "Packing")? < top(&harness, "for the trip")?);
    Ok(())
}
