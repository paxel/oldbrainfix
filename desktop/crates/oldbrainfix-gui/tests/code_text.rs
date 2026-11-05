//! A Code text in the window (#213): a Plain Note's whole text shown as
//! written, unwrapped, nothing in it a format, a box or a tag; the Note
//! dialog's text format badge sets it, a formatted text never back to
//! plain, and a paste that looks like code offers "Keep as code".

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, Duration, Utc};
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::text_format::{CODE, MARKDOWN, PLAIN};
use oldbrainfix_core::{Notes, OwnProfile, Places, SigningKey, Store};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const SCRIPT: &str = "#!/bin/sh\n# **not bold** #tag\necho \"a long line that runs on and on, \
                      far wider than the window, and is never wrapped at all, whatever the \
                      width of the dialog it shows in, as code is read line by line\"";

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

/// A store with the Note `text` in `format` filed in "Work", and its id.
fn with_note(text: &str, format: &str) -> Result<(Arc<Store>, String), Box<dyn std::error::Error>> {
    let store = device()?;
    let work = Places::new(&store).add_area("Work")?;
    let note = Notes::new(&store).capture_as(text, format, Some(&work.id), None)?;
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

/// Where the Note's field holding `text` lies.
fn field_rect(harness: &Harness<'_, App>, text: &str) -> Option<egui::Rect> {
    let text = text.to_string();
    harness
        .query_all_by(move |n| {
            n.role() == Role::MultilineTextInput && n.value().as_deref() == Some(text.as_str())
        })
        .next()
        .map(|n| n.rect())
}

/// The width of the Note's field holding `text`.
fn field_width(harness: &Harness<'_, App>, text: &str) -> Option<f32> {
    field_rect(harness, text).map(|r| r.width())
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

fn stored(store: &Store, note: &str) -> Result<(String, String), Box<dyn std::error::Error>> {
    let note = Notes::new(store).by_id(note)?.ok_or("the Note")?;
    Ok((note.text, note.text_format))
}

#[test]
fn a_code_note_reads_as_written_on_its_card_with_no_tag_or_format() -> TestResult {
    let (store, _) = with_note(SCRIPT, CODE)?;
    let harness = window(&store, Selection::All, None);
    let written = harness
        .query_all_by(|n| {
            n.label()
                .is_some_and(|l| l.starts_with("#!/bin/sh\n# **not bold** #tag"))
        })
        .count();
    assert!(written > 0, "its text as written");
    assert!(!shows(&harness, "not bold"), "no format read in it");
    assert_eq!(
        harness
            .query_all_by_role_and_label(Role::Link, "#tag")
            .count(),
        0
    );
    Ok(())
}

#[test]
fn its_field_is_unwrapped_sideways_and_has_no_format_bar() -> TestResult {
    let (store, note) = with_note(SCRIPT, CODE)?;
    let mut harness = window(&store, Selection::Unsorted, Some(&note));
    let width = field_width(&harness, SCRIPT).ok_or("the field")?;
    assert!(width > 1280.0, "{width}");
    // Scrolled over its start, which shows, it moves sideways; its middle
    // lies beyond the window.
    let rect = field_rect(&harness, SCRIPT).ok_or("the field")?;
    let before = rect.left();
    harness.hover_at(egui::pos2(rect.left() + 40.0, rect.top() + 8.0));
    harness.run();
    harness
        .query_all_by(|n| {
            n.role() == Role::MultilineTextInput && n.value().as_deref() == Some(SCRIPT)
        })
        .next()
        .ok_or("the field")?
        .scroll_left();
    harness.run_steps(30);
    let after = field_rect(&harness, SCRIPT).ok_or("the field")?.left();
    assert!(after < before - 50.0, "{before} {after}");
    // Only Add to Note's format bar: the text's has none.
    let bars = |harness: &Harness<'_, App>| {
        harness
            .query_all_by_role_and_label(Role::Button, "Bold")
            .count()
    };
    assert_eq!(bars(&harness), 1);
    // Its badges right under it, no room left between.
    let badge = harness.get_by_label("Text format: Code").rect();
    let field = field_rect(&harness, SCRIPT).ok_or("the field")?;
    assert!(badge.top() - field.bottom() < 120.0, "{field:?} {badge:?}");
    // Not unwrapped as plain text, with its format bar.
    let (store, note) = with_note(SCRIPT, PLAIN)?;
    let harness = window(&store, Selection::Unsorted, Some(&note));
    let width = field_width(&harness, SCRIPT).ok_or("the plain field")?;
    assert!(width < 1280.0, "{width}");
    assert_eq!(bars(&harness), 2);
    Ok(())
}

#[test]
fn a_task_list_item_typed_into_code_stays_as_written() -> TestResult {
    let (store, note) = with_note("x", CODE)?;
    let mut harness = window(&store, Selection::Unsorted, Some(&note));
    focus_field(&mut harness, "x")?;
    harness.event(egui::Event::Text("\n- [ ] y".into()));
    harness.run();
    harness.state_mut().select(Selection::All);
    harness.run();
    let (text, format) = stored(&store, &note)?;
    assert!(text.contains("- [ ] y"), "{text:?}");
    assert_eq!(format, CODE);
    Ok(())
}

#[test]
fn the_text_format_badge_sets_code_and_back_never_formatted_to_plain() -> TestResult {
    let (store, note) = with_note("# comment\necho hi", PLAIN)?;
    let mut harness = window(&store, Selection::Unsorted, Some(&note));
    click(&mut harness, "Text format: Plain text")?;
    for format in ["Plain text", "Formatted text", "Code"] {
        assert!(radio(&harness, format), "{format}");
    }
    pick(&mut harness, "Code")?;
    assert_eq!(
        stored(&store, &note)?,
        ("# comment\necho hi".into(), CODE.into())
    );
    assert!(shows(&harness, "Text format: Code"));
    // A Code text becomes no other kind: its kind badge only shows.
    click(&mut harness, "Kind: Plain Note")?;
    for kind in ["Checklist", "Tally", "Session"] {
        assert!(!radio(&harness, kind), "{kind}");
    }
    // Nor a Session from "+", which still offers a Routine.
    click(&mut harness, "Routine, Reminder, Frog or Session")?;
    assert!(shows(&harness, "Routine"));
    assert!(!shows(&harness, "Session"));
    harness.key_press(egui::Key::Escape);
    harness.run();
    click(&mut harness, "Text format: Code")?;
    pick(&mut harness, "Formatted text")?;
    assert_eq!(stored(&store, &note)?.1, MARKDOWN);
    click(&mut harness, "Text format: Formatted text")?;
    assert!(!radio(&harness, "Plain text"));
    assert!(radio(&harness, "Code"));
    Ok(())
}

#[test]
fn a_paste_that_looks_like_code_offers_keep_as_code() -> TestResult {
    let (store, note) = with_note("Script:", PLAIN)?;
    let mut harness = window(&store, Selection::Unsorted, Some(&note));
    assert!(!shows(&harness, "Keep as code"));
    focus_field(&mut harness, "Script:")?;
    harness.event(egui::Event::Paste("\nint a;\nint b;\n# heading".into()));
    harness.run();
    assert!(shows(&harness, "This looks like code."));
    // The paste formats nothing, though it holds a heading.
    assert_eq!(stored(&store, &note)?.1, PLAIN);
    click(&mut harness, "Keep as code")?;
    assert_eq!(
        stored(&store, &note)?,
        ("Script:\nint a;\nint b;\n# heading".into(), CODE.into())
    );
    assert!(!shows(&harness, "Keep as code"));
    Ok(())
}

#[test]
fn closing_the_offer_lets_the_paste_be() -> TestResult {
    let (store, note) = with_note("x", PLAIN)?;
    let mut harness = window(&store, Selection::Unsorted, Some(&note));
    focus_field(&mut harness, "x")?;
    harness.event(egui::Event::Paste("#!/bin/sh\nls".into()));
    harness.run();
    assert!(shows(&harness, "Keep as code"));
    click(&mut harness, "Close")?;
    assert!(!shows(&harness, "Keep as code"));
    harness.state_mut().select(Selection::All);
    harness.run();
    assert_eq!(stored(&store, &note)?.1, PLAIN);
    Ok(())
}
