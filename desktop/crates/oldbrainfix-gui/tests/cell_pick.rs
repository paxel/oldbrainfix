//! Cells of a table picked and copied (obf-1054): in a text only read, a
//! drag that starts on a cell picks the cells from it to the one under
//! the pointer and Shift+click picks too, in the rows as they show;
//! Ctrl+C and the right-click menu's Copy put them on the clipboard as a
//! table of their own, HTML beside Markdown. A drag that starts outside
//! a table selects its words as text; a click on the table while cells
//! are picked only ends the pick; nothing is picked on a card. The table
//! full screen, the editor's grid and the Markdown viewer pick so too.

// A test may unwrap, expect, panic and index, its helpers too (#229).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, Duration, Utc};
use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, OutputCommand, PointerButton, Pos2, Rect};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::markdown::table;
use oldbrainfix_core::text_format::MARKDOWN;
use oldbrainfix_core::{Notes, OwnProfile, Places, SigningKey, Store};
use oldbrainfix_gui::l10n::L10n;
use oldbrainfix_gui::md_view::{Asked, Viewer};
use oldbrainfix_gui::media::Clipboard;
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const TABLE: &str = "| Item | Count |\n| --- | --: |\n| milk | two |\n| oat | one |";

fn shop() -> String {
    format!("Shop list\n\n{TABLE}\n\nBread too")
}

fn store() -> Result<Arc<Store>, Box<dyn std::error::Error>> {
    let start = DateTime::parse_from_rfc3339("2026-03-02T11:00:00Z")?.with_timezone(&Utc);
    let ticks = Arc::new(AtomicI64::new(0));
    let clock = Box::new(move || start + Duration::seconds(ticks.fetch_add(1, Ordering::SeqCst)));
    let store = Store::in_memory(clock, || Ok(SigningKey::from_seed([6; 32])))?;
    OwnProfile::new(&store).apply_defaults("Ann", None)?;
    Ok(Arc::new(store))
}

/// What went to the fake clipboard: each text with its HTML.
type Copied = Rc<RefCell<Vec<(String, Option<String>)>>>;

struct FakeClipboard(Copied);

impl Clipboard for FakeClipboard {
    fn photo(&mut self) -> Option<Vec<u8>> {
        None
    }

    fn put_text(&mut self, text: &str, html: Option<&str>) -> bool {
        self.0
            .borrow_mut()
            .push((text.to_string(), html.map(str::to_string)));
        true
    }
}

/// The window over `store` showing `place`, its clipboard a fake one.
fn window(store: &Arc<Store>, place: Selection) -> (Harness<'static, App>, Copied) {
    let mut app = App::new(store.clone(), "en");
    app.set_region(Some("DE"));
    app.select(place);
    let copied = Copied::default();
    app.set_clipboard(Box::new(FakeClipboard(copied.clone())));
    let harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 1400.0))
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            app,
        );
    (harness, copied)
}

/// The window with the Note `text`, in the trash, open in its dialog,
/// where its text is only read.
fn trashed(text: &str) -> Result<(Harness<'static, App>, Copied), Box<dyn std::error::Error>> {
    let store = store()?;
    let notes = Notes::new(&store);
    let note = notes.capture_as(text, MARKDOWN, None, None)?.id;
    notes.trash(&note)?;
    let (mut harness, copied) = window(&store, Selection::Trash);
    harness.run();
    harness.state_mut().select_note(&note);
    harness.run();
    assert!(harness.state().note_open());
    Ok((harness, copied))
}

/// Where the last label reading `words` lies: in the Note dialog, over
/// the card of the list.
fn at(harness: &Harness<'_, App>, words: &str) -> Pos2 {
    harness
        .query_all_by_role(Role::Label)
        .rfind(|n| {
            let node = n.accesskit_node();
            node.label().as_deref() == Some(words) || node.value().as_deref() == Some(words)
        })
        .unwrap_or_else(|| panic!("no {words:?}"))
        .rect()
        .center()
}

fn press<S>(harness: &mut Harness<'_, S>, pos: Pos2, button: PointerButton, pressed: bool) {
    let modifiers = harness.input().modifiers;
    harness.event(Event::PointerButton {
        pos,
        button,
        pressed,
        modifiers,
    });
    harness.step();
}

/// A drag of the mouse from `from` to `to`.
fn drag<S>(harness: &mut Harness<'_, S>, from: Pos2, to: Pos2) {
    harness.event(Event::PointerMoved(from));
    harness.step();
    press(harness, from, PointerButton::Primary, true);
    harness.event(Event::PointerMoved(to));
    harness.step();
    press(harness, to, PointerButton::Primary, false);
    harness.run();
}

fn click<S>(harness: &mut Harness<'_, S>, at: Pos2) {
    harness.event(Event::PointerMoved(at));
    harness.step();
    press(harness, at, PointerButton::Primary, true);
    press(harness, at, PointerButton::Primary, false);
    harness.run();
}

fn shift_click<S>(harness: &mut Harness<'_, S>, at: Pos2) {
    harness.input_mut().modifiers = Modifiers::SHIFT;
    click(harness, at);
    harness.input_mut().modifiers = Modifiers::NONE;
    harness.run();
}

fn right_click<S>(harness: &mut Harness<'_, S>, at: Pos2) {
    harness.event(Event::PointerMoved(at));
    press(harness, at, PointerButton::Secondary, true);
    press(harness, at, PointerButton::Secondary, false);
    harness.run();
}

/// Ctrl+C, as the window brings it; what the frames then copied as text.
fn ctrl_c<S>(harness: &mut Harness<'_, S>) -> String {
    harness.event(Event::Copy);
    let mut copied = String::new();
    for _ in 0..4 {
        harness.step();
        for command in &harness.output().platform_output.commands {
            if let OutputCommand::CopyText(text) = command {
                copied.clone_from(text);
            }
        }
    }
    harness.run();
    copied
}

/// The table `markdown` as Copy puts it on the clipboard, with its
/// HTML.
fn cells(markdown: &str) -> (String, Option<String>) {
    let copy = table::copy(markdown, 0, &Default::default()).unwrap();
    (copy.markdown, Some(copy.html))
}

#[test]
fn a_drag_from_a_cell_picks_cells_and_ctrl_c_copies_them_as_a_table() -> TestResult {
    let (mut harness, copied) = trashed(&shop())?;
    let (from, to) = (at(&harness, "milk"), at(&harness, "one"));
    drag(&mut harness, from, to);
    let said = ctrl_c(&mut harness);
    // As a table of their own, the first row its header row; no words
    // selected beside it.
    let picked = "| milk | two |\n| --- | --: |\n| oat | one |";
    assert_eq!(copied.borrow().as_slice(), [cells(picked)]);
    assert!(copied.borrow()[0].1.as_deref().unwrap().contains("<table"));
    assert_eq!(said, "");
    assert_eq!(harness.state().shown_table(), None);
    assert!(harness.state().note_open());
    // Esc ends the pick alone: the Note stays open, Ctrl+C copies none.
    harness.key_press(Key::Escape);
    harness.run();
    assert!(harness.state().note_open());
    ctrl_c(&mut harness);
    assert_eq!(copied.borrow().len(), 1);
    Ok(())
}

#[test]
fn a_drag_from_the_words_before_a_table_selects_through_it_as_text() -> TestResult {
    let (mut harness, copied) = trashed(&shop())?;
    let (from, to) = (at(&harness, "Shop list"), at(&harness, "Bread too"));
    drag(&mut harness, from, to);
    let said = ctrl_c(&mut harness);
    assert!(said.contains("list"), "{said:?}");
    assert!(said.contains("milk"), "{said:?}");
    assert!(said.contains("one"), "{said:?}");
    assert!(copied.borrow().is_empty(), "{:?}", copied.borrow());
    Ok(())
}

#[test]
fn a_click_while_cells_are_picked_only_ends_the_pick() -> TestResult {
    let (mut harness, copied) = trashed(&shop())?;
    let (from, to) = (at(&harness, "milk"), at(&harness, "two"));
    drag(&mut harness, from, to);
    let pos = at(&harness, "oat");
    click(&mut harness, pos);
    assert_eq!(harness.state().shown_table(), None);
    ctrl_c(&mut harness);
    assert!(copied.borrow().is_empty(), "{:?}", copied.borrow());
    // The next click opens the table full screen, as before.
    let pos = at(&harness, "oat");
    click(&mut harness, pos);
    assert!(harness.state().shown_table().is_some());
    Ok(())
}

#[test]
fn shift_click_picks_from_the_cell_picked_first_and_sorts_nothing() -> TestResult {
    let (mut harness, copied) = trashed(&shop())?;
    // A single cell, a table of one cell.
    let pos = at(&harness, "two");
    shift_click(&mut harness, pos);
    ctrl_c(&mut harness);
    assert_eq!(copied.borrow().last(), Some(&cells("| two |\n| --: |")));
    // On to a cell of the header row, which sorts nothing.
    let pos = at(&harness, "Item");
    shift_click(&mut harness, pos);
    assert_eq!(harness.state().shown_table(), None);
    let sorted = harness
        .query_all_by_role_and_label(Role::Button, "Item: Sort by this column")
        .last()
        .and_then(|n| n.value());
    assert_eq!(sorted, None);
    ctrl_c(&mut harness);
    let picked = "| Item | Count |\n| --- | --: |\n| milk | two |";
    assert_eq!(copied.borrow().last(), Some(&cells(picked)));
    Ok(())
}

#[test]
fn a_table_shown_sorted_copies_its_rows_as_they_show() -> TestResult {
    let (mut harness, copied) = trashed(&shop())?;
    // Sorted by Item, descending: oat before milk.
    let pos = at(&harness, "Item");
    click(&mut harness, pos);
    click(&mut harness, pos);
    let (from, to) = (at(&harness, "Item"), at(&harness, "milk"));
    drag(&mut harness, from, to);
    ctrl_c(&mut harness);
    let picked = "| Item |\n| --- |\n| oat |\n| milk |";
    assert_eq!(copied.borrow().last(), Some(&cells(picked)));
    Ok(())
}

#[test]
fn the_right_click_menu_copies_the_cells_picked_and_searches_one() -> TestResult {
    let (mut harness, copied) = trashed(&shop())?;
    let (from, to) = (at(&harness, "milk"), at(&harness, "two"));
    drag(&mut harness, from, to);
    let pos = at(&harness, "two");
    right_click(&mut harness, pos);
    // The right-click itself copies nothing.
    assert!(copied.borrow().is_empty(), "{:?}", copied.borrow());
    let entry = |harness: &Harness<'_, App>, label: &str| {
        harness
            .query_all_by_role_and_label(Role::Button, label)
            .last()
            .map(|n| !n.accesskit_node().is_disabled())
    };
    assert_eq!(entry(&harness, "Copy"), Some(true));
    // Find in Note takes the words of a single cell only.
    assert_eq!(entry(&harness, "Find in Note"), Some(false));
    harness
        .query_all_by_role_and_label(Role::Button, "Copy")
        .last()
        .unwrap()
        .click();
    harness.run();
    assert_eq!(
        copied.borrow().as_slice(),
        [cells("| milk | two |\n| --- | --: |")]
    );
    // One cell: Find in Note is offered.
    let pos = at(&harness, "Bread too");
    click(&mut harness, pos);
    let pos = at(&harness, "oat");
    shift_click(&mut harness, pos);
    let pos = at(&harness, "oat");
    right_click(&mut harness, pos);
    assert_eq!(entry(&harness, "Find in Note"), Some(true));
    Ok(())
}

#[test]
fn select_all_on_a_text_that_begins_with_a_table_selects_its_words() -> TestResult {
    let (mut harness, copied) = trashed(&format!("{TABLE}\n\nBread too"))?;
    let pos = at(&harness, "Bread too");
    right_click(&mut harness, pos);
    harness
        .query_all_by_role_and_label(Role::Button, "Select all")
        .last()
        .unwrap()
        .click_accesskit();
    harness.run_steps(6);
    harness.run();
    let said = ctrl_c(&mut harness);
    assert!(said.contains("Item"), "{said:?}");
    assert!(said.contains("Bread too"), "{said:?}");
    assert!(copied.borrow().is_empty(), "{:?}", copied.borrow());
    Ok(())
}

#[test]
fn nothing_is_picked_on_a_card() -> TestResult {
    let store = store()?;
    let work = Places::new(&store).add_area("Work")?;
    Notes::new(&store).capture_as(&shop(), MARKDOWN, Some(&work.id), None)?;
    let (mut harness, copied) = window(&store, Selection::All);
    harness.run();
    let (from, to) = (at(&harness, "milk"), at(&harness, "one"));
    drag(&mut harness, from, to);
    let pos = at(&harness, "two");
    shift_click(&mut harness, pos);
    ctrl_c(&mut harness);
    assert!(copied.borrow().is_empty(), "{:?}", copied.borrow());
    Ok(())
}

/// Where the cell named `label` lies full screen: the widest of them.
fn full_cell(harness: &Harness<'_, App>, label: &str) -> Rect {
    harness
        .query_all_by_label(label)
        .map(|n| n.rect())
        .max_by(|a, b| a.width().total_cmp(&b.width()))
        .unwrap_or_else(|| panic!("no {label:?}"))
}

#[test]
fn full_screen_a_drag_picks_cells_ctrl_c_copies_them_and_esc_ends_it() -> TestResult {
    let (mut harness, copied) = trashed(&shop())?;
    let pos = at(&harness, "oat");
    click(&mut harness, pos);
    assert!(harness.state().shown_table().is_some());
    let (from, to) = (
        full_cell(&harness, "Count: Sort by this column"),
        full_cell(&harness, "two"),
    );
    drag(&mut harness, from.center(), to.center());
    ctrl_c(&mut harness);
    assert_eq!(
        copied.borrow().last(),
        Some(&cells("| Count |\n| --: |\n| two |"))
    );
    // Esc ends the pick; the view stays.
    harness.key_press(Key::Escape);
    harness.run();
    assert!(harness.state().shown_table().is_some());
    ctrl_c(&mut harness);
    assert_eq!(copied.borrow().len(), 1);
    Ok(())
}

#[test]
fn on_the_editors_grid_a_drag_picks_cells_and_a_click_then_edits() -> TestResult {
    let store = store()?;
    let work = Places::new(&store).add_area("Work")?;
    let note = Notes::new(&store).capture_as(TABLE, MARKDOWN, Some(&work.id), None)?;
    let (mut harness, copied) = window(&store, Selection::Unsorted);
    harness.state_mut().select_note(&note.id);
    harness.run();
    harness
        .query_all_by_role_and_label(Role::Button, "Open the table")
        .next()
        .ok_or("the table")?
        .click_accesskit();
    harness.run();
    let cell = |harness: &Harness<'_, App>, label: &str| {
        harness
            .query_all_by_role_and_label(Role::Button, label)
            .next()
            .unwrap_or_else(|| panic!("no {label:?}"))
            .rect()
            .center()
    };
    let (from, to) = (
        cell(&harness, "Row 2, column A: milk"),
        cell(&harness, "Row 3, column B: one"),
    );
    drag(&mut harness, from, to);
    ctrl_c(&mut harness);
    let picked = "| milk | two |\n| --- | --: |\n| oat | one |";
    assert_eq!(copied.borrow().last(), Some(&cells(picked)));
    // The right-click menu of a cell picked copies them too.
    let pos = cell(&harness, "Row 3, column A: oat");
    harness.event(Event::PointerMoved(pos));
    press(&mut harness, pos, PointerButton::Secondary, true);
    press(&mut harness, pos, PointerButton::Secondary, false);
    harness.run();
    harness
        .query_all_by_role_and_label(Role::Button, "Copy")
        .last()
        .ok_or("the menu's Copy")?
        .click();
    harness.run();
    assert_eq!(copied.borrow().len(), 2);
    assert_eq!(copied.borrow().last(), Some(&cells(picked)));
    // A click ends it and edits nothing; the next edits the cell.
    let pos = cell(&harness, "Row 2, column A: milk");
    click(&mut harness, pos);
    assert!(
        harness
            .query_all_by_role_and_label(Role::TextInput, "Row 2, column A")
            .next()
            .is_none()
    );
    let pos = cell(&harness, "Row 2, column A: milk");
    click(&mut harness, pos);
    assert!(
        harness
            .query_all_by_role_and_label(Role::TextInput, "Row 2, column A")
            .next()
            .is_some()
    );
    // The text stays as it was.
    let kept = Notes::new(&store).by_id(&note.id)?.ok_or("the Note")?;
    assert_eq!(kept.text, TABLE);
    Ok(())
}

#[test]
fn the_markdown_viewer_picks_cells_and_asks_the_window_to_copy_them() -> TestResult {
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("Shop.md");
    std::fs::write(&file, shop())?;
    let shown = Viewer::open(file.clone(), 1, egui::vec2(900.0, 700.0), None);
    let t = L10n::new("en");
    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 700.0))
        .build_ui_state(
            move |ui, (viewer, asked): &mut (Viewer, Vec<Asked>)| {
                App::install(ui.ctx());
                let (_, wanted) = viewer.frame(ui, &t, false, false);
                asked.extend(wanted);
            },
            (shown, Vec::new()),
        );
    for _ in 0..300 {
        harness.step();
        if harness.state().0.loaded() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    harness.run_steps(4);
    let at = |harness: &Harness<'_, (Viewer, Vec<Asked>)>, words: &str| {
        harness
            .query_all_by_role(Role::Label)
            .rfind(|n| {
                let node = n.accesskit_node();
                node.label().as_deref() == Some(words) || node.value().as_deref() == Some(words)
            })
            .unwrap_or_else(|| panic!("no {words:?}"))
            .rect()
            .center()
    };
    let (from, to) = (at(&harness, "Item"), at(&harness, "milk"));
    drag(&mut harness, from, to);
    ctrl_c(&mut harness);
    let copies: Vec<_> = harness
        .state()
        .1
        .iter()
        .filter_map(|a| match a {
            Asked::Copy(copy) => Some(copy.markdown.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(copies, ["| Item |\n| --- |\n| milk |"]);
    // Its right-click finds in the file, so it reads Find, not Find in
    // Note (obf-1014, C3).
    let milk = at(&harness, "milk");
    right_click(&mut harness, milk);
    let entry = |harness: &Harness<'_, (Viewer, Vec<Asked>)>, label: &str| {
        harness
            .query_all_by_role_and_label(Role::Button, label)
            .next()
            .is_some()
    };
    assert!(entry(&harness, "Find"));
    assert!(!entry(&harness, "Find in Note"));
    Ok(())
}

#[test]
fn the_reading_view_picks_the_cells_of_a_sections_table() -> TestResult {
    let store = store()?;
    let notes = Notes::new(&store);
    let id = notes.capture("draft", None, None)?.id;
    notes.edit_as(&id, "Shop list", MARKDOWN)?;
    notes.amend_as(&id, TABLE, MARKDOWN)?;
    let mut app = App::for_note_window(store.clone(), "en", &id, true);
    let copied = Copied::default();
    app.set_clipboard(Box::new(FakeClipboard(copied.clone())));
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 900.0))
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show_note_window(ui);
            },
            app,
        );
    harness.run();
    harness.key_press(Key::F9);
    harness.run();
    assert_eq!(harness.state().reading(), Some(true));
    let (from, to) = (at(&harness, "Count"), at(&harness, "one"));
    drag(&mut harness, from, to);
    ctrl_c(&mut harness);
    let picked = "| Count |\n| --: |\n| two |\n| one |";
    assert_eq!(copied.borrow().as_slice(), [cells(picked)]);
    Ok(())
}

#[test]
fn a_drag_on_past_the_view_scrolls_the_table_away_unharmed() -> TestResult {
    let mut text = shop();
    for n in 0..80 {
        text.push_str(&format!("\n\nParagraph {n} with some words to read."));
    }
    let (mut harness, _) = trashed(&text)?;
    let from = at(&harness, "milk");
    harness.event(Event::PointerMoved(from));
    harness.step();
    press(&mut harness, from, PointerButton::Primary, true);
    let below = egui::pos2(from.x, 1390.0);
    harness.event(Event::PointerMoved(below));
    harness.run_steps(60);
    press(&mut harness, below, PointerButton::Primary, false);
    harness.run();
    assert!(harness.state().note_open());
    Ok(())
}
