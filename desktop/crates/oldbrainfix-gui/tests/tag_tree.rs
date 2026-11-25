//! The tag tree on the desktop (#267, Q623, Q639): "All tags…" at the
//! end of the row of the most used tags under All Notes' search
//! (obf-1074) lists every tag by its levels with its count of Notes, the
//! levels below unfolding by the arrow, a click on a tag searching All
//! Notes for it; and "#" typed into a list's search offers the tags
//! starting with it as buttons under the field. Driven headless by
//! egui_kittest.

// A test may unwrap, expect, panic and index, its helpers too (#229).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use egui::accesskit::Role;
use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::{Notes, SigningKey, Store, parse_time};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn store(texts: &[&str]) -> Result<Store, Box<dyn std::error::Error>> {
    let now = parse_time("2026-11-13T09:00:00.000Z")?;
    let store = Store::in_memory(Box::new(move || now), || Ok(SigningKey::from_seed([5; 32])))?;
    for text in texts {
        Notes::new(&store).capture(text, None, None)?;
    }
    Ok(store)
}

fn harness(store: Store) -> Harness<'static, App> {
    window(store, Selection::Agenda, 1280.0)
}

/// The window `width` wide over `store`, showing `selection`.
fn window(store: Store, selection: Selection, width: f32) -> Harness<'static, App> {
    let mut app = App::new(store, "en");
    app.select(selection);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(width, 900.0))
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

fn shows<S>(harness: &Harness<'_, S>, label: &str) -> bool {
    harness.query_all_by_label(label).next().is_some()
}

/// How many buttons named `label` show.
fn buttons<S>(harness: &Harness<'_, S>, label: &str) -> usize {
    harness
        .query_all_by_role_and_label(Role::Button, label)
        .count()
}

const NOTES: [&str; 4] = [
    "Back exercises #life/health/back",
    "Dentist #Life/health",
    "Seeds #garden",
    "Tax #life",
];

/// The row of the most used tags' chips, by the tags they search for,
/// in their order.
fn chips<S>(harness: &Harness<'_, S>) -> Vec<String> {
    harness
        .query_all_by_role(Role::Button)
        .filter_map(|n| n.accesskit_node().label())
        .filter_map(|l| l.strip_prefix("Search for ").map(str::to_string))
        .collect()
}

const ALL_TAGS: &str = "All tags…";

#[test]
fn all_tags_at_the_end_of_the_row_lists_every_tag_by_its_levels() -> TestResult {
    let mut harness = window(store(&NOTES)?, Selection::All, 1280.0);
    harness.get_by_label(ALL_TAGS).click();
    harness.run();
    assert_eq!(harness.state().selection(), &Selection::Tags);
    // The top level alone at first, each with its count of Notes.
    assert!(shows(&harness, "#garden") && shows(&harness, "#life"));
    assert!(shows(&harness, "1 Note") && shows(&harness, "3 Notes"));
    assert!(!shows(&harness, "health"));
    // Only a tag with levels below has the arrow.
    assert_eq!(harness.query_all_by_label("Show the tags below").count(), 1);
    harness.get_by_label("Show the tags below").click();
    harness.run();
    assert!(shows(&harness, "health") && shows(&harness, "2 Notes"));
    assert!(!shows(&harness, "back"));
    harness.get_by_label("Show the tags below").click();
    harness.run();
    assert!(shows(&harness, "back"));
    // A click on a level searches All Notes for the tag down to it.
    harness.get_by_label("health").click();
    harness.run();
    assert_eq!(harness.state().selection(), &Selection::All);
    assert_eq!(harness.state().query(), "#life/health");
    assert_eq!(harness.state().shown_notes().len(), 2);
    // Back to the tree, still unfolded.
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowLeft);
    harness.run();
    assert_eq!(harness.state().selection(), &Selection::Tags);
    assert!(shows(&harness, "back"));
    // Folding #life hides all below it.
    assert_eq!(harness.query_all_by_label("Hide the tags below").count(), 2);
    harness
        .query_all_by_label("Hide the tags below")
        .next()
        .ok_or("the arrow of #life")?
        .click();
    harness.run();
    assert!(!shows(&harness, "health") && !shows(&harness, "back"));
    Ok(())
}

#[test]
fn without_tags_the_tree_says_how_one_comes() -> TestResult {
    let mut harness = window(store(&["Milk"])?, Selection::All, 1280.0);
    // The row is "All tags…" alone.
    assert!(chips(&harness).is_empty());
    harness.get_by_label(ALL_TAGS).click();
    harness.run();
    assert!(shows(
        &harness,
        "No tags yet. A word with # in front in a Note, such as #garden or #life/health, shows here."
    ));
    Ok(())
}

#[test]
fn a_hash_typed_into_the_search_offers_the_tags_starting_with_it() -> TestResult {
    let mut harness = harness(store(&NOTES)?);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::F);
    harness.run();
    harness.get_by(|n| n.is_focused()).type_text("#li");
    harness.run();
    // Read with the search, shown the frame after.
    harness.run();
    for tag in ["#life", "#life/health", "#life/health/back"] {
        assert_eq!(buttons(&harness, tag), 1, "{tag}");
    }
    assert_eq!(buttons(&harness, "#garden"), 0);
    harness
        .get_by_role_and_label(Role::Button, "#life/health")
        .click();
    harness.run();
    assert_eq!(harness.state().query(), "#life/health");
    harness.run();
    // The cursor stands after it, to type on.
    harness.get_by(|n| n.is_focused()).type_text("/b");
    harness.run();
    assert_eq!(harness.state().query(), "#life/health/b");
    harness.state_mut().set_query("#life/health");
    harness.run();
    // The one typed exactly is left out; those below it stay.
    assert_eq!(buttons(&harness, "#life/health/back"), 1);
    assert_eq!(buttons(&harness, "#life/health"), 0);
    // Without a "#", none.
    harness.state_mut().set_query("life");
    harness.run();
    assert_eq!(buttons(&harness, "#life/health/back"), 0);
    Ok(())
}

/// Fifteen tags: #life on three Notes, #life/health on two, and #tag01
/// to #tag13 on one each.
fn many() -> Result<Store, Box<dyn std::error::Error>> {
    let mut texts = vec![
        "Tax #life".to_string(),
        "Dentist #life/health".to_string(),
        "Back #life/health".to_string(),
    ];
    texts.extend((1..=13).map(|n| format!("Note {n} #tag{n:02}")));
    let texts: Vec<&str> = texts.iter().map(String::as_str).collect();
    store(&texts)
}

#[test]
fn all_notes_shows_the_twelve_most_used_tags_under_its_search() -> TestResult {
    let mut harness = window(many()?, Selection::All, 1800.0);
    // Those more Notes carry first, of every level; of as many those
    // first in the tree; twelve at most.
    let mut wanted = vec!["#life".to_string(), "#life/health".to_string()];
    wanted.extend((1..=10).map(|n| format!("#tag{n:02}")));
    assert_eq!(chips(&harness), wanted);
    // "All tags…" ends the row, under the search field and over the
    // first card.
    let all = harness.get_by_role_and_label(Role::Button, ALL_TAGS).rect();
    let last = harness
        .get_by_role_and_label(Role::Button, "Search for #tag10")
        .rect();
    assert!(all.min.x > last.max.x && (all.center().y - last.center().y).abs() < 1.0);
    let field = harness.get_by_role(Role::TextInput).rect();
    assert!(all.min.y >= field.max.y, "{all:?} under {field:?}");
    let card = harness
        .query_all_by_role(Role::Button)
        .find(|n| {
            n.accesskit_node()
                .label()
                .is_some_and(|l| l.starts_with("Note 1 "))
        })
        .ok_or("the card of Note 1")?
        .rect();
    assert!(all.max.y <= card.min.y, "{all:?} over {card:?}");
    // A click searches for the tag; the row goes with the text.
    harness
        .get_by_role_and_label(Role::Button, "Search for #life/health")
        .click();
    harness.run();
    assert_eq!(harness.state().selection(), &Selection::All);
    assert_eq!(harness.state().query(), "#life/health");
    assert_eq!(harness.state().shown_notes().len(), 2);
    assert!(!shows(&harness, ALL_TAGS) && chips(&harness).is_empty());
    Ok(())
}

#[test]
fn the_row_goes_at_the_first_letter_typed_and_shows_in_all_notes_alone() -> TestResult {
    let mut harness = window(store(&NOTES)?, Selection::All, 1280.0);
    assert!(shows(&harness, ALL_TAGS));
    harness.key_press_modifiers(Modifiers::COMMAND, Key::F);
    harness.run();
    harness.get_by(|n| n.is_focused()).type_text("t");
    harness.run();
    assert!(!shows(&harness, ALL_TAGS) && chips(&harness).is_empty());
    // Emptied, it is back.
    harness.state_mut().set_query("");
    harness.run();
    assert!(shows(&harness, ALL_TAGS));
    assert_eq!(chips(&harness).len(), 4);
    for other in [Selection::Unsorted, Selection::Trash, Selection::Agenda] {
        harness.state_mut().select(other.clone());
        harness.run();
        assert!(!shows(&harness, ALL_TAGS), "{other:?}");
    }
    // Nor does the sidebar list the tag tree any more.
    assert!(!shows(&harness, "Tags"));
    Ok(())
}

#[test]
fn a_narrow_window_leaves_out_the_least_used_tags_never_all_tags() -> TestResult {
    for width in [640.0, 760.0, 900.0, 1100.0] {
        let mut harness = window(many()?, Selection::All, width);
        let shown = chips(&harness);
        assert!(!shown.is_empty() && shown.len() < 12, "{width}: {shown:?}");
        assert_eq!(shown.first().map(String::as_str), Some("#life"));
        // In one row, the last inside the window.
        let all = harness.get_by_role_and_label(Role::Button, ALL_TAGS).rect();
        assert!(all.max.x <= width, "{width}: {all:?}");
        let first = harness
            .get_by_role_and_label(Role::Button, "Search for #life")
            .rect();
        assert!((all.center().y - first.center().y).abs() < 1.0, "{width}");
        harness.get_by_label(ALL_TAGS).click();
        harness.run();
        assert_eq!(harness.state().selection(), &Selection::Tags);
    }
    Ok(())
}

#[test]
fn with_the_agendas_opener_the_row_is_read_on_a_thread() -> TestResult {
    let store = std::sync::Arc::new(store(&NOTES)?);
    let mut app = App::new(store.clone(), "en");
    app.select(Selection::All);
    // The thread on the window's own connection: an in-memory store has
    // no other.
    let aside = store.clone();
    app.set_agenda_store(move || aside.clone());
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
    assert_eq!(chips(&harness).first().map(String::as_str), Some("#life"));
    // Two more Notes make #garden the most used, read once the window
    // reads again.
    let notes = Notes::new(&store);
    notes.capture("Pears #garden", None, None)?;
    notes.capture("Plums #garden", None, None)?;
    notes.capture("Apples #garden", None, None)?;
    harness.state_mut().set_query("");
    let start = std::time::Instant::now();
    while chips(&harness).first().map(String::as_str) != Some("#garden") {
        assert!(start.elapsed().as_secs() < 10, "the row's tags never came");
        harness.step();
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(shows(&harness, ALL_TAGS));
    Ok(())
}

#[test]
fn a_connection_of_its_own_reads_what_the_window_wrote() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("oldbrainfix.db");
    let open = |path: &std::path::Path| -> Result<Store, Box<dyn std::error::Error>> {
        let now = parse_time("2026-11-13T09:00:00.000Z")?;
        Ok(Store::open(path, Box::new(move || now), || {
            Ok(SigningKey::from_seed([5; 32]))
        })?)
    };
    let shown = std::sync::Arc::new(open(&path)?);
    for text in NOTES {
        Notes::new(&shown).capture(text, None, None)?;
    }
    let own = std::sync::Arc::new(open(&path)?);
    let mut app = App::new(shown.clone(), "en");
    app.select(Selection::All);
    app.set_agenda_store(move || own.clone());
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
    // Each tag new to the thread's connection, written by the window's,
    // the second after the first was read there.
    for (text, tag) in [("Plums #plum", "#plum"), ("Pears #orchard", "#orchard")] {
        Notes::new(&shown).capture(text, None, None)?;
        harness.state_mut().set_query("");
        let start = std::time::Instant::now();
        while !chips(&harness).iter().any(|shown| shown == tag) {
            assert!(start.elapsed().as_secs() < 10, "{tag} never came");
            harness.step();
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }
    Ok(())
}
