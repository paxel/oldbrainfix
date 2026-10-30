//! Formatted text in the window (#127, ADR 0033): a formatted Note's
//! text and its Sections read without their marks on its card and in the
//! editor, a link opens only to a web or e-mail address, a click on the
//! text edits its Markdown, and the format bar, Ctrl+B, a shortcut typed
//! and a paste with a format make a plain text a formatted one, kept at
//! once. "Add to Note" has the format bar too, without ☐, and the text
//! fields have names a screen reader says once they hold text.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, Duration, Utc};
use egui::accesskit::Role;
use egui::{Key, Modifiers, OutputCommand};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::text_format::MARKDOWN;
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

/// A store with the Note `text` in `format` filed in "Work", and its id.
fn with_note(text: &str, format: &str) -> Result<(Arc<Store>, String), Box<dyn std::error::Error>> {
    let store = device()?;
    let work = Places::new(&store).add_area("Work")?;
    let note = Notes::new(&store).capture_as(text, format, Some(&work.id), None)?;
    Ok((Arc::new(store), note.id))
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

/// The editor alone: the Note is filed, the Unsorted list shows no card
/// of it.
fn editor(store: &Arc<Store>, note: &str) -> Harness<'static, App> {
    window(store, Selection::Unsorted, Some(note))
}

fn shows(harness: &Harness<'_, App>, label: &str) -> bool {
    harness.query_all_by_label(label).next().is_some()
}

fn links(harness: &Harness<'_, App>, label: &str) -> usize {
    harness
        .query_all_by_role_and_label(Role::Link, label)
        .count()
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

/// Clicks the format bar's button `label` of the Note's text, the first
/// bar, as the Amend field's has the same buttons.
fn format(harness: &mut Harness<'_, App>, label: &str) -> TestResult {
    harness
        .query_all_by_role_and_label(Role::Button, label)
        .next()
        .ok_or(format!("{label:?}"))?
        .click_accesskit();
    harness.run();
    Ok(())
}

/// Clicks the Amend field's format button `label`, the second bar's.
fn amend_format(harness: &mut Harness<'_, App>, label: &str) -> TestResult {
    harness
        .query_all_by_role_and_label(Role::Button, label)
        .nth(1)
        .ok_or(format!("{label:?} of Add to Note"))?
        .click_accesskit();
    harness.run();
    Ok(())
}

/// What the Amend field holds: the field without a name, as its hint
/// goes once it holds text, and the capture field and the Note's are
/// named.
fn amend_field(harness: &Harness<'_, App>) -> Option<String> {
    harness
        .query_by(|n| n.role() == Role::MultilineTextInput && n.label().is_none())
        .and_then(|n| n.value())
}

/// Whether a text field holds `text`: the editor's, as the Amend field
/// beside it holds nothing.
fn field(harness: &Harness<'_, App>, text: &str) -> bool {
    harness
        .query_all_by(move |n| {
            n.role() == Role::MultilineTextInput && n.value().as_deref() == Some(text)
        })
        .next()
        .is_some()
}

/// Gives the editor's field, holding `text`, the keyboard.
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

/// The text of `note` as it is stored, and its format.
fn stored(store: &Store, note: &str) -> Result<(String, String), Box<dyn std::error::Error>> {
    let note = Notes::new(store).by_id(note)?.ok_or("the Note")?;
    Ok((note.text, note.text_format))
}

#[test]
fn a_formatted_note_reads_without_its_marks_on_its_card_and_in_the_editor() -> TestResult {
    let (store, note) = with_note("# Shop\n- **milk**\n1. eggs", MARKDOWN)?;
    Notes::new(&store).amend_as(&note, "Also ~~bread~~", MARKDOWN)?;
    let harness = window(&store, Selection::All, Some(&note));
    for words in ["Shop", "milk", "eggs", "Also bread"] {
        assert!(shows(&harness, words), "{words}");
    }
    for source in ["# Shop", "- **milk**", "**milk**", "Also ~~bread~~"] {
        assert!(!shows(&harness, source), "{source}");
    }
    // Not being edited, the editor shows it as it reads, not its field.
    assert!(!field(&harness, "# Shop\n- **milk**\n1. eggs"));
    Ok(())
}

#[test]
fn a_compact_card_shows_its_first_line_as_it_reads() -> TestResult {
    let (store, _) = with_note("## **Big** plan\nsecond", MARKDOWN)?;
    let mut harness = window(&store, Selection::All, None);
    click(&mut harness, L10n::new("en").show_notes_compact())?;
    assert!(shows(&harness, "Big plan"));
    assert!(!shows(&harness, "second"));
    Ok(())
}

#[test]
fn a_link_opens_only_to_a_web_or_mail_address() -> TestResult {
    let (store, note) = with_note(
        "[site](https://example.org) [mail](mailto:ann@example.org) [app](intent:x)",
        MARKDOWN,
    )?;
    let mut harness = editor(&store, &note);
    assert_eq!(links(&harness, "site"), 1);
    assert_eq!(links(&harness, "mail"), 1);
    assert_eq!(links(&harness, "app"), 0);
    harness
        .query_all_by_role_and_label(Role::Link, "site")
        .next()
        .ok_or("the link")?
        .click_accesskit();
    harness.step();
    let opened: Vec<_> = harness
        .output()
        .platform_output
        .commands
        .iter()
        .filter_map(|c| match c {
            OutputCommand::OpenUrl(open) => Some(open.url.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(opened, ["https://example.org"]);
    Ok(())
}

#[test]
fn a_click_on_a_formatted_text_edits_its_markdown() -> TestResult {
    let (store, note) = with_note("Buy **milk**", MARKDOWN)?;
    let mut harness = editor(&store, &note);
    assert!(shows(&harness, "Buy milk"));
    harness
        .query_all_by_role_and_label(Role::Button, "Edit")
        .last()
        .ok_or("the text")?
        .click_accesskit();
    harness.run();
    assert!(field(&harness, "Buy **milk**"));
    Ok(())
}

#[test]
fn the_format_bar_makes_a_plain_text_formatted_at_once() -> TestResult {
    let (store, note) = with_note("buy milk", "plain")?;
    let mut harness = editor(&store, &note);
    // A field never typed into has its cursor at the start: the first
    // word.
    format(&mut harness, "Bold")?;
    assert_eq!(
        stored(&store, &note)?,
        ("**buy** milk".into(), MARKDOWN.into())
    );
    assert!(field(&harness, "**buy** milk"));
    // The line's format too, on the formatted text.
    format(&mut harness, "Bulleted list")?;
    assert!(field(&harness, "- **buy** milk"));
    Ok(())
}

#[test]
fn ctrl_b_makes_the_word_bold() -> TestResult {
    let (store, note) = with_note("milk", "plain")?;
    let mut harness = editor(&store, &note);
    focus_field(&mut harness, "milk")?;
    harness.key_press_modifiers(Modifiers::COMMAND, Key::B);
    harness.run();
    assert_eq!(stored(&store, &note)?, ("**milk**".into(), MARKDOWN.into()));
    Ok(())
}

#[test]
fn a_shortcut_typed_makes_a_plain_text_formatted() -> TestResult {
    let (store, note) = with_note("Shop", "plain")?;
    let mut harness = editor(&store, &note);
    focus_field(&mut harness, "Shop")?;
    // The cursor at the end of the text.
    harness.event(egui::Event::Text("\n- ".into()));
    harness.run();
    let (_, format) = stored(&store, &note)?;
    assert_eq!(format, MARKDOWN);
    assert!(field(&harness, "Shop\n- "));
    Ok(())
}

#[test]
fn a_paste_with_a_format_makes_a_plain_text_formatted() -> TestResult {
    let (store, note) = with_note("Shop", "plain")?;
    let mut harness = editor(&store, &note);
    focus_field(&mut harness, "Shop")?;
    harness.event(egui::Event::Paste(" **now**".into()));
    harness.run();
    assert_eq!(
        stored(&store, &note)?,
        ("Shop **now**".into(), MARKDOWN.into())
    );
    Ok(())
}

#[test]
fn a_plain_paste_keeps_a_plain_text_plain() -> TestResult {
    let (store, note) = with_note("Shop", "plain")?;
    let mut harness = editor(&store, &note);
    focus_field(&mut harness, "Shop")?;
    harness.event(egui::Event::Paste(" now".into()));
    harness.run();
    harness.state_mut().select(Selection::All);
    harness.run();
    assert_eq!(stored(&store, &note)?, ("Shop now".into(), "plain".into()));
    Ok(())
}

#[test]
fn a_link_gets_the_address_typed() -> TestResult {
    let (store, note) = with_note("docs here", "plain")?;
    let mut harness = editor(&store, &note);
    format(&mut harness, "Link")?;
    // The dialog's field has the keyboard.
    harness.event(egui::Event::Text("example.org".into()));
    harness.run();
    click(&mut harness, "Set link")?;
    assert_eq!(
        stored(&store, &note)?,
        ("[docs](https://example.org) here".into(), MARKDOWN.into())
    );
    Ok(())
}

#[test]
fn add_to_note_has_the_format_bar_without_the_mark() -> TestResult {
    let (store, note) = with_note("Shop", "plain")?;
    let mut harness = editor(&store, &note);
    // ☐ only over the Note's text, the rest over both fields.
    assert_eq!(
        harness
            .query_all_by_role_and_label(Role::Button, "Put in ☐")
            .count(),
        1
    );
    assert_eq!(
        harness
            .query_all_by_role_and_label(Role::Button, "Bulleted list")
            .count(),
        2
    );
    harness
        .get_by(|n| n.placeholder() == Some("What's new?"))
        .click();
    harness.run();
    harness.get_by(|n| n.is_focused()).type_text("milk");
    harness.run();
    amend_format(&mut harness, "Bold")?;
    assert_eq!(amend_field(&harness).as_deref(), Some("**milk**"));
    // The Note's text is left as it is.
    assert_eq!(stored(&store, &note)?, ("Shop".into(), "plain".into()));
    // A link goes on the field's text too.
    amend_format(&mut harness, "Link")?;
    harness.event(egui::Event::Text("example.org".into()));
    harness.run();
    click(&mut harness, "Set link")?;
    let added = amend_field(&harness).ok_or("the Amend field")?;
    assert!(added.contains("milk](https://example.org)"), "{added}");
    click(&mut harness, "Add to Note")?;
    let sections = Notes::new(&store).sections(&note)?;
    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].text, added);
    assert_eq!(sections[0].text_format, MARKDOWN);
    assert_eq!(amend_field(&harness).as_deref(), Some(""));
    Ok(())
}

#[test]
fn a_table_of_add_to_note_is_edited_full_screen_and_goes_back_into_the_field() -> TestResult {
    let (store, note) = with_note("Shop", "plain")?;
    let mut harness = editor(&store, &note);
    amend_format(&mut harness, "Table")?;
    assert!(harness.state().shown_table().is_some());
    harness.event(egui::Event::Text("Item".into()));
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    // Its arrow back, "Close", leaves it.
    harness
        .query_all_by_role_and_label(Role::Button, "Close")
        .next()
        .ok_or("the arrow back")?
        .click_accesskit();
    harness.run();
    assert!(harness.state().shown_table().is_none());
    let added = amend_field(&harness).ok_or("the Amend field")?;
    assert!(added.starts_with("| Item |  |\n| --- | --- |"), "{added}");
    // Nothing of it in the Note until it is added.
    assert_eq!(stored(&store, &note)?, ("Shop".into(), "plain".into()));
    assert!(Notes::new(&store).sections(&note)?.is_empty());
    Ok(())
}

#[test]
fn the_text_fields_have_a_name_once_they_hold_text() -> TestResult {
    let (store, note) = with_note("buy milk", "plain")?;
    let harness = editor(&store, &note);
    let named = |name: &'static str| {
        harness
            .query_all_by_role_and_label(Role::MultilineTextInput, name)
            .filter_map(|n| n.value())
            .collect::<Vec<_>>()
    };
    assert_eq!(named("Note text"), ["buy milk"]);
    assert_eq!(named("Capture"), [""]);
    Ok(())
}
