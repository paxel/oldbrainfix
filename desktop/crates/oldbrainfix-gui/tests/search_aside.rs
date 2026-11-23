//! Typing into a search never waits (obf-1068, C11): every letter shows
//! in its field at once, the list as it was and nothing read; the search
//! runs on a thread of its own with a connection of its own once typing
//! stopped for about 300 ms, and what it found shows once it is in. What
//! a search brings for a text no longer typed never shows. Merge into…
//! and Add to Note… search alike. Driven headless by egui_kittest over a
//! store on disk; each frame of the harness is a quarter of a second.

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::Utc;
use egui::accesskit::Role;
use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::{Notes, Places, SigningKey, Store};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const MILK: &str = "Buy milk";
const MINT: &str = "Mint tea";
const BIKE: &str = "Fix the bike";

fn open(path: &Path) -> Result<Store, Box<dyn std::error::Error>> {
    Ok(Store::open(path, Box::new(Utc::now), || {
        Ok(SigningKey::from_seed([3; 32]))
    })?)
}

/// The window's store on disk, "Buy milk" and "Mint tea" Unsorted and
/// "Fix the bike" in Home, and a connection of its own to it.
fn stores(dir: &Path) -> Result<(Arc<Store>, Arc<Store>), Box<dyn std::error::Error>> {
    let path = dir.join("oldbrainfix.db");
    let shown = Arc::new(open(&path)?);
    let home = Places::new(&shown).add_area("Home")?;
    let notes = Notes::new(&shown);
    notes.capture(MILK, None, None)?;
    notes.capture(MINT, None, None)?;
    notes.capture(BIKE, Some(&home.id), None)?;
    Ok((shown, Arc::new(open(&path)?)))
}

fn window(app: App) -> Harness<'static, App> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
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

/// All Notes, its search searching on the connection `open` hands it.
fn all_notes(
    shown: Arc<Store>,
    open: impl Fn() -> Arc<Store> + Send + Sync + 'static,
) -> Harness<'static, App> {
    let mut app = App::new(shown, "en");
    app.select(Selection::All);
    app.set_agenda_store(open);
    window(app)
}

/// What the cards shown say, in order.
fn shown(harness: &Harness<'_, App>) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let notes = Notes::new(harness.state().store());
    let mut texts = Vec::new();
    for id in harness.state().shown_notes() {
        texts.push(notes.by_id(&id)?.ok_or("a Note shown")?.text);
    }
    texts.sort();
    Ok(texts)
}

fn sorted(texts: &[&str]) -> Vec<String> {
    let mut texts: Vec<String> = texts.iter().map(|t| t.to_string()).collect();
    texts.sort();
    texts
}

/// Types `text` where the keyboard is, in one frame.
fn type_here(harness: &mut Harness<'_, App>, text: &str) {
    harness.get_by(|n| n.is_focused()).type_text(text);
    harness.step();
}

/// What the field the keyboard is in holds.
fn typed(harness: &Harness<'_, App>) -> Option<String> {
    harness.get_by(|n| n.is_focused()).value()
}

/// Steps the window until `done`.
fn until(harness: &mut Harness<'_, App>, done: impl Fn(&Harness<'_, App>) -> bool) -> TestResult {
    let started = Instant::now();
    while !done(harness) {
        if started.elapsed() > Duration::from_secs(60) {
            return Err("it never came".into());
        }
        harness.step();
        std::thread::sleep(Duration::from_millis(5));
    }
    Ok(())
}

/// How many are named `label`: its card's and a dialog's row.
fn named(harness: &Harness<'_, App>, label: &str) -> usize {
    harness.query_all_by_label(label).count()
}

#[test]
fn a_letter_shows_at_once_and_the_results_follow_once_typing_stops() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (shown_store, own) = stores(dir.path())?;
    let mut harness = all_notes(shown_store, move || own.clone());
    assert_eq!(shown(&harness)?, sorted(&[MILK, MINT, BIKE]));
    harness.key_press_modifiers(Modifiers::COMMAND, Key::F);
    harness.run();
    let reads = harness.state().reads();

    // In the field at once; the list as it was, nothing read for it.
    type_here(&mut harness, "mi");
    assert_eq!(typed(&harness).as_deref(), Some("mi"));
    assert_eq!(harness.state().query(), "mi");
    assert_eq!(harness.state().searched(), "");
    assert_eq!(shown(&harness)?, sorted(&[MILK, MINT, BIKE]));
    assert_eq!(harness.state().reads(), reads);
    // A quarter of a second on, typing has not stopped long enough.
    harness.step();
    assert!(!harness.state().searching());
    assert_eq!(harness.state().searched(), "");

    // Then it is searched on its thread, and what it found shows.
    until(&mut harness, |h| h.state().searched() == "mi")?;
    assert_eq!(shown(&harness)?, sorted(&[MILK, MINT]));
    assert_eq!(typed(&harness).as_deref(), Some("mi"));
    Ok(())
}

/// The search's connection, handed over once per search the test lets
/// go, and how many searches asked for it.
struct Gate {
    go: Sender<()>,
    asked: Arc<AtomicUsize>,
}

impl Gate {
    fn opener(own: Arc<Store>) -> (Gate, impl Fn() -> Arc<Store> + Send + Sync + 'static) {
        let (go, gate) = channel::<()>();
        let gate = Arc::new(Mutex::new(gate));
        let asked = Arc::new(AtomicUsize::new(0));
        let counted = asked.clone();
        let open = move || {
            counted.fetch_add(1, Ordering::SeqCst);
            if let Ok(gate) = gate.lock() {
                let _ = gate.recv();
            }
            own.clone()
        };
        (Gate { go, asked }, open)
    }

    fn asked(&self) -> usize {
        self.asked.load(Ordering::SeqCst)
    }
}

#[test]
fn what_a_search_finds_for_a_text_no_longer_typed_never_shows() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (shown_store, own) = stores(dir.path())?;
    let (gate, open) = Gate::opener(own);
    let mut harness = all_notes(shown_store, open);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::F);
    harness.run();

    // "mi" is searched, held at the gate; typing goes on meanwhile.
    type_here(&mut harness, "mi");
    until(&mut harness, |h| h.state().searching() && gate.asked() == 1)?;
    type_here(&mut harness, "nt");
    harness.run_steps(4);
    assert_eq!(typed(&harness).as_deref(), Some("mint"));
    assert_eq!(gate.asked(), 1, "one search at a time");

    // What "mi" found is dropped, "mint" searched after it.
    gate.go.send(())?;
    until(&mut harness, |_| gate.asked() == 2)?;
    assert_eq!(harness.state().searched(), "");
    assert_eq!(shown(&harness)?, sorted(&[MILK, MINT, BIKE]));
    gate.go.send(())?;
    until(&mut harness, |h| h.state().searched() == "mint")?;
    assert_eq!(shown(&harness)?, [MINT]);
    Ok(())
}

/// Opens `item` from the menu of the card `label`.
fn from_card_menu(harness: &mut Harness<'_, App>, label: &str, item: &str) -> TestResult {
    harness
        .get_by_role_and_label(Role::Button, label)
        .click_secondary();
    harness.run();
    harness
        .query_all_by_label(item)
        .next_back()
        .ok_or_else(|| format!("no {item:?} in the menu"))?
        .click();
    harness.run_steps(8);
    Ok(())
}

/// Types "tea" into the dialog open: it shows at once with `gone` still
/// offered, and once typing stopped only "Mint tea" is.
fn searches_in_the_dialog(harness: &mut Harness<'_, App>, gone: &str) -> TestResult {
    let (mint, offered) = (named(harness, MINT), named(harness, gone));
    let reads = harness.state().reads();
    type_here(harness, "tea");
    assert_eq!(typed(harness).as_deref(), Some("tea"));
    assert_eq!(named(harness, gone), offered, "still offered");
    assert_eq!(harness.state().reads(), reads);
    // Its row goes, its card stays.
    until(harness, |h| {
        named(h, gone) == offered - 1 && !h.state().searching()
    })?;
    assert_eq!(named(harness, MINT), mint);
    Ok(())
}

#[test]
fn merge_into_searches_once_typing_stops() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (shown_store, own) = stores(dir.path())?;
    let mut harness = all_notes(shown_store, move || own.clone());
    from_card_menu(&mut harness, BIKE, "Merge into…")?;
    searches_in_the_dialog(&mut harness, MILK)
}

#[test]
fn add_to_note_searches_once_typing_stops() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (shown_store, own) = stores(dir.path())?;
    let mut harness = all_notes(shown_store, move || own.clone());
    from_card_menu(&mut harness, MILK, "Add to Note…")?;
    searches_in_the_dialog(&mut harness, BIKE)
}

/// Types `text` into the dialog open and presses Ctrl+Enter at once, its
/// search held at the gate: what was typed is searched at once, and the
/// first Note it finds is picked, not the first offered before.
fn picks_what_is_typed_at_once(harness: &mut Harness<'_, App>, gate: &Gate, text: &str) {
    type_here(harness, text);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Enter);
    harness.run_steps(4);
    assert_eq!(gate.asked(), 0, "nothing searched on the thread");
}

/// The texts of the Sections of the Note `text`.
fn sections_of(store: &Store, text: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let notes = Notes::new(store);
    let note = notes
        .all()?
        .into_iter()
        .find(|n| n.text == text)
        .ok_or_else(|| format!("no {text:?}"))?;
    Ok(notes
        .sections(&note.id)?
        .into_iter()
        .map(|s| s.text)
        .collect())
}

#[test]
fn ctrl_enter_in_merge_into_merges_into_what_is_typed_before_its_search_is_in() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (shown_store, own) = stores(dir.path())?;
    let (gate, open) = Gate::opener(own);
    let mut harness = all_notes(shown_store.clone(), open);
    from_card_menu(&mut harness, BIKE, "Merge into…")?;
    // "Mint tea", the newer, offered first.
    picks_what_is_typed_at_once(&mut harness, &gate, "milk");
    assert_eq!(sections_of(&shown_store, MILK)?, [BIKE]);
    assert!(sections_of(&shown_store, MINT)?.is_empty());
    Ok(())
}

#[test]
fn ctrl_enter_in_add_to_note_adds_to_what_is_typed_before_its_search_is_in() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (shown_store, own) = stores(dir.path())?;
    let (gate, open) = Gate::opener(own);
    let mut harness = all_notes(shown_store.clone(), open);
    from_card_menu(&mut harness, MILK, "Add to Note…")?;
    // "Fix the bike", the newer, offered first.
    picks_what_is_typed_at_once(&mut harness, &gate, "tea");
    assert_eq!(sections_of(&shown_store, MINT)?, [MILK]);
    assert!(sections_of(&shown_store, BIKE)?.is_empty());
    Ok(())
}
