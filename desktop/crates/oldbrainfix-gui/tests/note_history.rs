//! A Note's History on the desktop (#193), driven headless by
//! egui_kittest: Note ▸ History over the Note dialog lists every change
//! of the Note and its Amends, the newest first, with who made it, when
//! and its text before and after; "Use this version" brings an old text
//! back as a new change, the dialog's fields showing it, so closing the
//! dialog keeps it.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, Duration, Utc};
use egui::Key;
use egui_kittest::kittest::Queryable;
use egui_kittest::{Harness, Node};
use oldbrainfix_core::{Importance, Notes, OwnProfile, SigningKey, Store};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A store on a clock that moves a second per read, its Profile Ann.
fn store() -> Result<Arc<Store>, Box<dyn std::error::Error>> {
    let start = DateTime::parse_from_rfc3339("2026-03-02T11:00:00Z")?.with_timezone(&Utc);
    let ticks = Arc::new(AtomicI64::new(0));
    let clock = Box::new(move || start + Duration::seconds(ticks.fetch_add(1, Ordering::SeqCst)));
    let store = Store::in_memory(clock, || Ok(SigningKey::from_seed([4; 32])))?;
    OwnProfile::new(&store).apply_defaults("Ann", None)?;
    Ok(Arc::new(store))
}

/// The window over `store` with the Note `id` open in its dialog.
fn opened(store: &Arc<Store>, id: &str) -> Harness<'static, App> {
    let mut app = App::new(store.clone(), "en");
    app.set_region(Some("DE"));
    app.select(Selection::All);
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
    harness.state_mut().select_note(id);
    harness.run();
    harness
}

fn count(harness: &Harness<'_, App>, label: &str) -> usize {
    harness.query_all_by_label(label).count()
}

/// The last widget labelled `label`: a menu's entry, drawn over the
/// window's buttons of the same name.
fn last<'a>(harness: &'a Harness<'_, App>, label: &'a str) -> Node<'a> {
    match harness.get_all_by_label(label).last() {
        Some(node) => node,
        None => panic!("no {label}"),
    }
}

/// Picks Note ▸ History.
fn open_history(harness: &mut Harness<'_, App>) {
    match harness.get_all_by_label("Note").next() {
        Some(menu) => menu.click(),
        None => panic!("no Note menu"),
    }
    harness.run();
    last(harness, "History").click();
    harness.run();
}

#[test]
fn history_lists_every_change_and_brings_an_old_text_back() -> TestResult {
    let store = store()?;
    let notes = Notes::new(&store);
    let note = notes.capture("Dentist on Tuesday", None, None)?;
    notes.edit(&note.id, "Dentist on Wednesday")?;
    let amend = notes.amend(&note.id, "Bring the X-ray")?;
    notes.edit_section(&amend.id, "Bring nothing")?;
    notes.set_importance(&note.id, Importance::Important)?;
    let mut harness = opened(&store, &note.id);

    open_history(&mut harness);
    assert_eq!(
        count(
            &harness,
            "Every change of this Note, its Amends and its Lines, the newest first."
        ),
        1
    );
    for (label, n) in [
        ("Captured", 1),
        ("Text changed", 1),
        ("Amended", 1),
        ("Amend changed", 1),
        ("Importance", 1),
        ("Before", 3),
        ("After", 3),
        ("Normal", 1),
        ("Important", 1),
        // In the History and in its field beneath.
        ("Bring nothing", 2),
        // Each version that is not the text showing now.
        ("Use this version", 2),
    ] {
        assert_eq!(count(&harness, label), n, "{label}");
    }
    assert!(
        harness
            .query_all_by_label_contains("Ann, ")
            .next()
            .is_some(),
        "who and when"
    );

    // The newest that may come back first: the Amend as it was added.
    let written = store.current_seq()?;
    match harness.get_all_by_label("Use this version").next() {
        // Read and clicked as a screen reader does: the list scrolls.
        Some(button) => button.click_accesskit(),
        None => panic!("no Use this version"),
    }
    harness.run();
    assert_eq!(store.entries_after(written)?.len(), 1);
    assert_eq!(
        notes.section_by_id(&amend.id)?.ok_or("the Amend")?.text,
        "Bring the X-ray"
    );
    // Now the text it had before may come back, and the Note's own.
    assert_eq!(count(&harness, "Use this version"), 2);
    last(&harness, "Use this version").click_accesskit();
    harness.run();
    assert_eq!(
        notes.by_id(&note.id)?.ok_or("the Note")?.text,
        "Dentist on Tuesday"
    );

    last(&harness, "Close").click_accesskit();
    harness.run();
    // Closing the Note dialog saves its fields, which show the texts
    // brought back.
    harness.key_press(Key::Escape);
    harness.run();
    assert!(!harness.state().note_open());
    assert_eq!(
        notes.by_id(&note.id)?.ok_or("the Note")?.text,
        "Dentist on Tuesday"
    );
    assert_eq!(
        notes.section_by_id(&amend.id)?.ok_or("the Amend")?.text,
        "Bring the X-ray"
    );
    Ok(())
}
