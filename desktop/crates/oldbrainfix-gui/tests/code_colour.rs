//! Code coloured by its language (obf-1036): a Code text by the language
//! the text format badge picks beside "Code", stored with the Note; a
//! fenced code block of a formatted text by the one its fence names; one
//! without a language, or with one not known, in one colour as before.
//! "Keep as code" picks the language guessed for the paste.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, Duration, Utc};
use egui::accesskit::Role;
use egui::epaint::Shape;
use egui::text::LayoutJob;
use egui::{Color32, FontId, TextFormat};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::highlight::{CodeColour, Grammars};
use oldbrainfix_core::text_format::{CODE, MARKDOWN, PLAIN};
use oldbrainfix_core::{Notes, OwnProfile, Places, SigningKey, Store};
use oldbrainfix_gui::text_marks_view;
use oldbrainfix_gui::{App, Selection, theme};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const RUST: &str = "// says hello\nfn main() {\n    let answer = 42;\n    println!(\"hi\");\n}";

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

/// A store with the Note `text` in `format` filed in "Work", coloured by
/// `language`, and its id.
fn with_note(
    text: &str,
    format: &str,
    language: Option<&str>,
) -> Result<(Arc<Store>, String), Box<dyn std::error::Error>> {
    let store = device()?;
    let work = Places::new(&store).add_area("Work")?;
    let notes = Notes::new(&store);
    let note = notes.capture_as(text, format, Some(&work.id), None)?;
    if language.is_some() {
        notes.set_code_language(&note.id, language)?;
    }
    Ok((Arc::new(store), note.id))
}

/// The window over `store` in English showing `selection`, the Note
/// `note` open in its dialog.
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

fn pick(harness: &mut Harness<'_, App>, label: &str) -> TestResult {
    harness
        .query_all_by_role_and_label(Role::RadioButton, label)
        .next()
        .ok_or(format!("{label:?}"))?
        .click_accesskit();
    harness.run();
    Ok(())
}

fn focus_field(harness: &mut Harness<'_, App>, text: &str) -> TestResult {
    let text = text.to_string();
    harness
        .query_all_by(move |n| {
            n.role() == Role::MultilineTextInput && n.value().as_deref() == Some(text.as_str())
        })
        .next()
        .ok_or("the field")?
        .focus();
    harness.run();
    Ok(())
}

fn language(store: &Store, note: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
    let note = Notes::new(store).by_id(note)?.ok_or("the Note")?;
    Ok(note.code_language)
}

/// The colours code's parts take in the look `ctx` shows.
fn palette(ctx: &egui::Context) -> Vec<Color32> {
    let dark = ctx.theme() == egui::Theme::Dark;
    CodeColour::ALL
        .iter()
        .map(|c| theme::code_colour(dark, *c))
        .collect()
}

/// Every run of text drawn in the last frame with its colour.
fn drawn_runs(shapes: &[egui::epaint::ClippedShape]) -> Vec<(String, Color32)> {
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
    let mut out = Vec::new();
    for clipped in shapes {
        collect(&clipped.shape, &mut out);
    }
    out
}

/// The runs drawn in one of code's colours.
fn coloured_runs(harness: &Harness<'_, App>) -> Vec<(String, Color32)> {
    let palette = palette(&harness.ctx);
    drawn_runs(&harness.output().shapes)
        .into_iter()
        .filter(|(run, colour)| !run.trim().is_empty() && palette.contains(colour))
        .collect()
}

#[test]
fn the_badge_picks_a_code_texts_language_and_stores_it() -> TestResult {
    let (store, note) = with_note(RUST, CODE, None)?;
    let mut harness = window(&store, Selection::Unsorted, Some(&note));
    click(&mut harness, "Text format: Code")?;
    // The language under the formats: none or each grammar's.
    assert!(
        harness.query_all_by_value("Language").next().is_some(),
        "its heading"
    );
    assert!(radio(&harness, "No language"));
    for grammar in &Grammars::built_in().all {
        assert!(radio(&harness, &grammar.name), "{}", grammar.name);
    }
    pick(&mut harness, "Rust")?;
    assert_eq!(language(&store, &note)?.as_deref(), Some("source.rust"));
    // Its format stays code; the badge closed with the pick.
    assert_eq!(
        Notes::new(&store)
            .by_id(&note)?
            .ok_or("the Note")?
            .text_format,
        CODE
    );
    assert!(!radio(&harness, "No language"));
    click(&mut harness, "Text format: Code")?;
    pick(&mut harness, "No language")?;
    assert_eq!(language(&store, &note)?, None);
    Ok(())
}

#[test]
fn only_a_code_text_offers_a_language() -> TestResult {
    for format in [PLAIN, MARKDOWN] {
        let (store, note) = with_note("# comment\necho hi", format, None)?;
        let mut harness = window(&store, Selection::Unsorted, Some(&note));
        let badge = match format {
            PLAIN => "Text format: Plain text",
            _ => "Text format: Formatted text",
        };
        click(&mut harness, badge)?;
        assert!(radio(&harness, "Code"), "{format}");
        assert!(!radio(&harness, "No language"), "{format}");
        assert!(!radio(&harness, "Rust"), "{format}");
    }
    Ok(())
}

#[test]
fn a_rust_code_text_is_coloured_on_its_card_and_in_its_dialog() -> TestResult {
    let (store, note) = with_note(RUST, CODE, Some("source.rust"))?;
    let harness = window(&store, Selection::All, None);
    let runs = coloured_runs(&harness);
    assert!(!runs.is_empty(), "coloured on its card");
    // The comment in the comment's colour, its words as written.
    let dark = harness.ctx.theme() == egui::Theme::Dark;
    let comment = theme::code_colour(dark, CodeColour::Comment);
    assert!(
        runs.iter()
            .any(|(run, colour)| run.contains("says hello") && *colour == comment),
        "{runs:?}"
    );
    // In the dialog's field too.
    let harness = window(&store, Selection::Unsorted, Some(&note));
    assert!(!coloured_runs(&harness).is_empty(), "coloured in its field");
    Ok(())
}

#[test]
fn a_code_text_without_a_language_or_with_one_not_known_is_one_colour() -> TestResult {
    for language in [None, Some("source.not-here")] {
        let (store, note) = with_note(RUST, CODE, language)?;
        let harness = window(&store, Selection::All, None);
        assert_eq!(coloured_runs(&harness), [], "{language:?} on its card");
        let harness = window(&store, Selection::Unsorted, Some(&note));
        assert_eq!(coloured_runs(&harness), [], "{language:?} in its dialog");
    }
    Ok(())
}

#[test]
fn a_fenced_block_is_coloured_by_the_language_its_fence_names() -> TestResult {
    let text = "Steps\n\n```rust\n// says hello\nfn main() {}\n```\n\nafter";
    let (store, _) = with_note(text, MARKDOWN, None)?;
    let harness = window(&store, Selection::All, None);
    let runs = coloured_runs(&harness);
    let dark = harness.ctx.theme() == egui::Theme::Dark;
    let comment = theme::code_colour(dark, CodeColour::Comment);
    assert!(
        runs.iter()
            .any(|(run, colour)| run.contains("says hello") && *colour == comment),
        "{runs:?}"
    );
    // The words around it keep the text's colour.
    assert!(
        !runs
            .iter()
            .any(|(run, _)| run.contains("Steps") || run.contains("after"))
    );
    // A fence naming none or one not known stays in one colour.
    for fence in ["```", "```klingon"] {
        let text = format!("Steps\n\n{fence}\n// says hello\nfn main() {{}}\n```\n\nafter");
        let (store, _) = with_note(&text, MARKDOWN, None)?;
        let harness = window(&store, Selection::All, None);
        assert_eq!(coloured_runs(&harness), [], "{fence}");
    }
    Ok(())
}

#[test]
fn keep_as_code_picks_the_language_guessed_for_the_paste() -> TestResult {
    let (store, note) = with_note("x", PLAIN, None)?;
    let mut harness = window(&store, Selection::Unsorted, Some(&note));
    focus_field(&mut harness, "x")?;
    harness.event(egui::Event::Paste("#!/bin/bash\necho hi".into()));
    harness.run();
    click(&mut harness, "Keep as code")?;
    let stored = Notes::new(&store).by_id(&note)?.ok_or("the Note")?;
    assert_eq!(stored.text_format, CODE);
    assert_eq!(stored.code_language.as_deref(), Some("source.shell"));
    Ok(())
}

#[test]
fn keep_as_code_without_a_guess_picks_no_language() -> TestResult {
    let (store, note) = with_note("Script:", PLAIN, None)?;
    let mut harness = window(&store, Selection::Unsorted, Some(&note));
    focus_field(&mut harness, "Script:")?;
    harness.event(egui::Event::Paste("\nint a;\nint b;\n# heading".into()));
    harness.run();
    click(&mut harness, "Keep as code")?;
    assert_eq!(language(&store, &note)?, None);
    Ok(())
}

#[test]
fn painting_code_changes_only_the_colour_of_its_parts() -> TestResult {
    let harness = Harness::builder().build_ui(|_| {});
    let ctx = harness.ctx.clone();
    let grammar = Grammars::built_in()
        .by_scope(Some("source.rust"))
        .ok_or("the Rust grammar")?;
    let mut format = TextFormat::simple(FontId::monospace(14.0), Color32::BLACK);
    format.background = Color32::LIGHT_GRAY;
    let mut job = LayoutJob::single_section(RUST.to_string(), format.clone());
    let spans = text_marks_view::code_colours(&ctx, grammar, RUST);
    assert!(!spans.is_empty());
    text_marks_view::paint_code(&mut job, 0, &spans, false);
    assert_eq!(job.text, RUST, "every byte stays");
    // The sections cover the text, one after another.
    let mut at = 0;
    for section in &job.sections {
        assert_eq!(section.byte_range.start.0, at);
        at = section.byte_range.end.0;
        assert_eq!(section.format.font_id, format.font_id);
        assert_eq!(section.format.background, format.background);
    }
    assert_eq!(at, RUST.len());
    // Each coloured part in its colour, the rest as it was.
    for span in spans.iter() {
        let section = job
            .sections
            .iter()
            .find(|s| s.byte_range.start.0 == span.start)
            .ok_or("a section where a part begins")?;
        assert_eq!(section.format.color, theme::code_colour(false, span.colour));
    }
    assert!(
        job.sections
            .iter()
            .any(|s| s.format.color == Color32::BLACK)
    );
    Ok(())
}

#[test]
fn the_palette_differs_by_look() {
    for part in CodeColour::ALL {
        assert_ne!(
            theme::code_colour(false, part),
            theme::code_colour(true, part),
            "{part:?}"
        );
    }
}
