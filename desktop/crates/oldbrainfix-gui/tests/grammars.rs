//! The person's own grammars (obf-1037): one added colours a Code text
//! and a fenced block of its language and is offered in the text format
//! badge; one of a built-in scope replaces the built-in one until Remove
//! brings it back; Settings ▸ Code languages lists each with Remove, adds
//! a file picked with "Add grammar…" and says under it why one was
//! refused, adding nothing.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, Duration, Utc};
use egui::accesskit::Role;
use egui::epaint::Shape;
use egui::{Color32, Theme};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::highlight::CodeColour;
use oldbrainfix_core::text_format::{CODE, MARKDOWN};
use oldbrainfix_core::{Notes, OwnProfile, PersonSettings, Places, SigningKey, Store};
use oldbrainfix_gui::{App, Page, Selection, theme};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A grammar of the subset for `scope`, colouring `MOVE` as a keyword,
/// as the core's tests write it.
fn cobol(scope: &str, name: &str) -> String {
    format!(
        "name: {name}\nscope: {scope}\nfile_extensions: [cbl]\n\
         contexts:\n  main:\n    - match: '\\bMOVE\\b'\n      scope: keyword\n"
    )
}

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

/// Adds the grammar `source` for the person, as "Add grammar…" does.
fn add(store: &Store, source: &str) -> TestResult {
    PersonSettings::new(store)
        .add_grammar(source.as_bytes())?
        .map_err(|refused| refused.0)?;
    Ok(())
}

/// A Note `text` in `format` filed in the Area `area` of `store`,
/// coloured by `language`; its id.
fn note(
    (store, area): (&Store, &str),
    text: &str,
    format: &str,
    language: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    let notes = Notes::new(store);
    let note = notes.capture_as(text, format, Some(area), None)?;
    if language.is_some() {
        notes.set_code_language(&note.id, language)?;
    }
    Ok(note.id)
}

/// The window over `store` in English showing `selection`, the Note
/// `note` open in its dialog, the grammar dialog handing back `picked`
/// in turn.
fn window(
    store: &Arc<Store>,
    selection: Selection,
    note: Option<&str>,
    picked: Vec<PathBuf>,
) -> Harness<'static, App> {
    let mut app = App::new(store.clone(), "en");
    let picked = Rc::new(RefCell::new(picked.into_iter()));
    app.set_grammar_picker(Box::new(move |_title| picked.borrow_mut().next()));
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

/// Whether a label or the words of one show `text`.
fn shows(harness: &Harness<'_, App>, text: &str) -> bool {
    let text = text.to_string();
    harness
        .query_all_by(move |n| {
            n.label().is_some_and(|l| l.contains(text.as_str()))
                || n.value().is_some_and(|v| v.contains(text.as_str()))
        })
        .next()
        .is_some()
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

fn radio(harness: &Harness<'_, App>, label: &str) -> bool {
    harness
        .query_all_by_role_and_label(Role::RadioButton, label)
        .next()
        .is_some()
}

fn added_scopes(store: &Store) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    Ok(PersonSettings::new(store)
        .grammars()?
        .added
        .iter()
        .map(|g| g.scope.clone())
        .collect())
}

/// The colour of `part` of code in the look `harness` shows.
fn colour(harness: &Harness<'_, App>, part: CodeColour) -> Color32 {
    theme::code_colour(harness.ctx.theme() == Theme::Dark, part)
}

/// The runs of text drawn in the last frame in one of code's colours.
fn coloured_runs(harness: &Harness<'_, App>) -> Vec<(String, Color32)> {
    fn collect(shape: &Shape, out: &mut Vec<(String, Color32)>) {
        match shape {
            Shape::Text(text) => {
                let job = &text.galley.job;
                for section in &job.sections {
                    let range = section.byte_range.start.0..section.byte_range.end.0;
                    let run = job.text.get(range).unwrap_or_default().to_string();
                    out.push((run, section.format.color));
                }
            }
            Shape::Vec(shapes) => shapes.iter().for_each(|s| collect(s, out)),
            _ => {}
        }
    }
    let palette: Vec<Color32> = CodeColour::ALL
        .iter()
        .map(|c| colour(harness, *c))
        .collect();
    let mut out = Vec::new();
    for clipped in &harness.output().shapes {
        collect(&clipped.shape, &mut out);
    }
    out.into_iter()
        .filter(|(run, colour)| !run.trim().is_empty() && palette.contains(colour))
        .collect()
}

/// Whether `run` is drawn in the colour of `part`.
fn drawn_as(harness: &Harness<'_, App>, run: &str, part: CodeColour) -> bool {
    let colour = colour(harness, part);
    coloured_runs(harness)
        .iter()
        .any(|(drawn, c)| drawn.contains(run) && *c == colour)
}

#[test]
fn an_added_grammar_colours_its_code_and_is_offered_in_the_badge() -> TestResult {
    let store = device()?;
    add(&store, &cobol("source.cobol", "COBOL"))?;
    let work = Places::new(&store).add_area("Work")?;
    let at = (&store, work.id.as_str());
    let code = note(at, "MOVE A TO B", CODE, Some("source.cobol"))?;
    note(
        at,
        "Steps\n\n```cbl\nMOVE C TO D\n```\n\nafter",
        MARKDOWN,
        None,
    )?;
    let store = Arc::new(store);
    // On the cards: the Code text and the block its fence names.
    let harness = window(&store, Selection::All, None, Vec::new());
    assert!(drawn_as(&harness, "MOVE", CodeColour::Keyword));
    let runs = coloured_runs(&harness);
    assert!(
        runs.iter().filter(|(run, _)| run.contains("MOVE")).count() >= 2,
        "{runs:?}"
    );
    assert!(!runs.iter().any(|(run, _)| run.contains("Steps")));
    // The badge offers it beside the built-in ones.
    let mut harness = window(&store, Selection::Unsorted, Some(&code), Vec::new());
    assert!(
        drawn_as(&harness, "MOVE", CodeColour::Keyword),
        "in its field"
    );
    click(&mut harness, "Text format: Code")?;
    assert!(radio(&harness, "COBOL"));
    assert!(radio(&harness, "Rust"));
    Ok(())
}

#[test]
fn a_grammar_of_a_built_in_scope_replaces_it_until_removed() -> TestResult {
    let store = device()?;
    add(&store, &cobol("source.rust", "My Rust"))?;
    let work = Places::new(&store).add_area("Work")?;
    // One line, as a compact card shows it whole.
    let text = "MOVE A // says hello";
    note((&store, work.id.as_str()), text, CODE, Some("source.rust"))?;
    let store = Arc::new(store);
    let mut harness = window(&store, Selection::All, None, Vec::new());
    // The person's own one colours: no comment it does not know of.
    assert!(drawn_as(&harness, "MOVE", CodeColour::Keyword));
    assert!(!drawn_as(&harness, "says hello", CodeColour::Comment));
    // Remove in the Settings brings the built-in one back at once.
    harness.state_mut().open(Page::Settings);
    harness.run();
    assert!(shows(&harness, "My Rust"));
    click(&mut harness, "Remove")?;
    assert_eq!(added_scopes(&store)?, Vec::<String>::new());
    assert!(!shows(&harness, "My Rust"));
    harness.state_mut().close_page();
    harness.run();
    assert!(drawn_as(&harness, "says hello", CodeColour::Comment));
    // Added again, as by sync arriving: it colours again in its place.
    add(&store, &cobol("source.rust", "My Rust"))?;
    harness.run();
    assert!(!drawn_as(&harness, "says hello", CodeColour::Comment));
    assert!(drawn_as(&harness, "MOVE", CodeColour::Keyword));
    Ok(())
}

#[test]
fn the_settings_list_each_added_grammar_with_remove() -> TestResult {
    let store = device()?;
    add(&store, &cobol("source.cobol", "COBOL"))?;
    add(&store, &cobol("source.rust", "My Rust"))?;
    let store = Arc::new(store);
    let mut harness = window(&store, Selection::Unsorted, None, Vec::new());
    harness.state_mut().open(Page::Settings);
    harness.run();
    let harness = &mut harness;
    assert!(shows(harness, "Code languages"));
    assert!(shows(harness, "Add a Sublime syntax file"));
    assert!(shows(harness, "COBOL"));
    assert!(shows(harness, "source.cobol"));
    assert!(shows(harness, "My Rust"));
    assert!(shows(harness, "Replaces the built-in grammar for Rust"));
    assert_eq!(harness.query_all_by_label("Remove").count(), 2);
    assert!(harness.query_all_by_label("Add grammar…").next().is_some());
    // Remove takes away the one beside it, at once.
    harness
        .query_all_by_label("Remove")
        .next()
        .ok_or("Remove")?
        .click_accesskit();
    harness.run();
    assert_eq!(added_scopes(&store)?.len(), 1);
    assert_eq!(harness.query_all_by_label("Remove").count(), 1);
    Ok(())
}

#[test]
fn add_grammar_adds_the_file_picked_and_says_why_one_is_refused() -> TestResult {
    let store = Arc::new(device()?);
    let dir = tempfile::tempdir()?;
    let broken = dir.path().join("broken.sublime-syntax");
    std::fs::write(&broken, "name: [x\n")?;
    let good = dir.path().join("COBOL.sublime-syntax");
    std::fs::write(&good, cobol("source.cobol", "COBOL"))?;
    let mut harness = window(
        &store,
        Selection::Unsorted,
        None,
        vec![broken.clone(), good.clone()],
    );
    harness.state_mut().open(Page::Settings);
    harness.run();
    click(&mut harness, "Add grammar…")?;
    // Refused: why, under the button; nothing added.
    assert!(shows(&harness, "broken.sublime-syntax was not added: "));
    assert!(shows(&harness, "not YAML"));
    assert_eq!(added_scopes(&store)?, Vec::<String>::new());
    assert_eq!(harness.query_all_by_label("Remove").count(), 0);
    // The next one added clears it and is listed.
    click(&mut harness, "Add grammar…")?;
    assert_eq!(added_scopes(&store)?, ["source.cobol"]);
    assert!(!shows(&harness, "was not added"));
    assert!(shows(&harness, "COBOL"));
    assert_eq!(harness.query_all_by_label("Remove").count(), 1);
    // The dialog cancelled changes nothing.
    click(&mut harness, "Add grammar…")?;
    assert_eq!(added_scopes(&store)?, ["source.cobol"]);
    Ok(())
}
