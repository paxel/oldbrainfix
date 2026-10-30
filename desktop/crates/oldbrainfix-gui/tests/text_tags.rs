//! Tags in the window (#126, ADR 0032): a `#tag` in a Note's text and
//! in its Sections' is a link read as "Search for" it on a full and a
//! compact card and in the editor's Sections; a click on it searches All
//! Notes for the tag as written and opens no card; in the editor's field
//! and in the merge's picker it is only coloured.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, Duration, Utc};
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::{Notes, OwnProfile, Places, SigningKey, Store};
use oldbrainfix_gui::l10n::L10n;
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A store on a clock that moves a second per read, with its own
/// Profile.
fn device() -> Result<Store, Box<dyn std::error::Error>> {
    let start = DateTime::parse_from_rfc3339("2026-03-02T11:00:00Z")?.with_timezone(&Utc);
    let ticks = Arc::new(AtomicI64::new(0));
    let clock = Box::new(move || start + Duration::seconds(ticks.fetch_add(1, Ordering::SeqCst)));
    let store = Store::in_memory(clock, move || Ok(SigningKey::from_seed([3; 32])))?;
    store.set_local_setting("author", "Desk")?;
    OwnProfile::new(&store).apply_defaults("Desk", None)?;
    Ok(store)
}

/// The Notes of the tests, in the Area "Home": the store, the Area's id
/// and, by text, the Notes' ids: two with the tag in any case, one with
/// the word alone and one with a longer tag.
struct Garden {
    store: Arc<Store>,
    home: String,
    tagged: String,
    lower: String,
    word: String,
    longer: String,
}

fn garden() -> Result<Garden, Box<dyn std::error::Error>> {
    let store = device()?;
    let home = Places::new(&store).add_area("Home")?.id;
    let notes = Notes::new(&store);
    let tagged = notes
        .capture("Seeds for the #Garten", Some(&home), None)?
        .id;
    let lower = notes
        .capture("Water the #garten beds\nbefore noon", None, None)?
        .id;
    let word = notes.capture("Garten gate oil", Some(&home), None)?.id;
    let longer = notes.capture("Tools #Gartenarbeit", None, None)?.id;
    Ok(Garden {
        store: Arc::new(store),
        home,
        tagged,
        lower,
        word,
        longer,
    })
}

/// The window over `store` in English showing `selection`, the Note
/// `note` open in its editor.
fn window(store: &Arc<Store>, selection: Selection, note: Option<&str>) -> Harness<'static, App> {
    let mut app = App::new(store.clone(), "en");
    app.select(selection);
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

/// How many links read as `label` there are.
fn links(harness: &Harness<'_, App>, label: &str) -> usize {
    harness
        .query_all_by_role_and_label(Role::Link, label)
        .count()
}

/// Clicks the last link read as `label`.
fn click_link(harness: &mut Harness<'_, App>, label: &str) -> TestResult {
    harness
        .query_all_by_role_and_label(Role::Link, label)
        .last()
        .ok_or(format!("{label:?}"))?
        .click_accesskit();
    harness.run();
    Ok(())
}

/// Whether the search field holds `query`.
fn searched(harness: &Harness<'_, App>, query: &str) -> bool {
    harness
        .query_by(|n| n.role() == Role::TextInput && n.value().as_deref() == Some(query))
        .is_some()
}

fn sorted(mut ids: Vec<String>) -> Vec<String> {
    ids.sort();
    ids
}

#[test]
fn a_cards_tag_searches_all_notes_for_it_and_opens_no_card() -> TestResult {
    let g = garden()?;
    let mut harness = window(&g.store, Selection::Place(g.home.clone()), None);
    assert_eq!(links(&harness, "Search for #Garten"), 1);
    click_link(&mut harness, "Search for #Garten")?;
    assert_eq!(harness.state().selection(), &Selection::All);
    assert!(searched(&harness, "#Garten"));
    assert_eq!(
        sorted(harness.state().shown_notes()),
        sorted(vec![g.tagged.clone(), g.lower.clone(), g.longer.clone()])
    );
    assert!(!harness.state().shown_notes().contains(&g.word));
    assert_eq!(harness.state().selected_note(), None);
    // A tag clicked in the results replaces the query.
    click_link(&mut harness, "Search for #Gartenarbeit")?;
    assert!(searched(&harness, "#Gartenarbeit"));
    assert_eq!(harness.state().shown_notes(), vec![g.longer.clone()]);
    Ok(())
}

#[test]
fn a_compact_cards_tag_searches_for_it() -> TestResult {
    let g = garden()?;
    let mut harness = window(&g.store, Selection::All, None);
    harness
        .get_by_label(L10n::new("en").show_notes_compact())
        .click_accesskit();
    harness.run();
    assert!(harness.state().compact());
    assert_eq!(links(&harness, "Search for #garten"), 1);
    click_link(&mut harness, "Search for #garten")?;
    assert!(searched(&harness, "#garten"));
    assert!(harness.state().shown_notes().contains(&g.tagged));
    assert_eq!(harness.state().selected_note(), None);
    // The card stays compact, its first line alone shown.
    assert!(
        harness
            .query_all_by_label("Water the #garten beds")
            .next()
            .is_some()
    );
    Ok(())
}

#[test]
fn a_sections_tag_searches_for_it_on_the_card_and_in_the_editor() -> TestResult {
    let g = garden()?;
    Notes::new(&g.store).amend(&g.word, "Later: #Balkon")?;
    let mut harness = window(&g.store, Selection::All, Some(&g.word));
    // The card's Section and the editor's.
    assert_eq!(links(&harness, "Search for #Balkon"), 2);
    click_link(&mut harness, "Search for #Balkon")?;
    assert!(searched(&harness, "#Balkon"));
    assert_eq!(harness.state().shown_notes(), vec![g.word.clone()]);
    Ok(())
}

#[test]
fn the_editors_field_colours_a_tag_and_links_none() -> TestResult {
    let g = garden()?;
    let harness = window(&g.store, Selection::All, Some(&g.tagged));
    let field = harness
        .query_by(|n| n.role() == Role::MultilineTextInput)
        .and_then(|n| n.value());
    assert_eq!(field.as_deref(), Some("Seeds for the #Garten"));
    // The card's link alone.
    assert_eq!(links(&harness, "Search for #Garten"), 1);
    Ok(())
}

#[test]
fn the_merges_picker_links_no_tag() -> TestResult {
    let g = garden()?;
    let mut harness = window(&g.store, Selection::All, Some(&g.word));
    let before = links(&harness, "Search for #Garten");
    harness.get_by_label("Merge into…").click_accesskit();
    harness.run();
    // The picker lists the tagged Note; only the card behind links.
    assert!(
        harness
            .query_all_by_label("Seeds for the #Garten")
            .next()
            .is_some()
    );
    assert_eq!(links(&harness, "Search for #Garten"), before);
    Ok(())
}
