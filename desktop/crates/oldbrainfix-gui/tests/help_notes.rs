//! The help Notes on the desktop (#180), driven headless: the first start
//! answered with "New to Old Brain Fix" files Quick start and What the
//! icons mean into the Area "Old Brain Fix", and no What's new; an
//! ordinary start makes what was never made, in the computer's language,
//! once; an App not handed the version makes none, as in every other
//! test.

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::help_notes::{self, VERSION_SETTING, parse_whats_new};
use oldbrainfix_core::{Checklists, Notes, OwnProfile, Places, SigningKey, Store};
use oldbrainfix_gui::App;
use oldbrainfix_gui::help_notes::{
    GROUPS, WHATS_NEW, icons_text, quick_start_description, quick_start_lines,
};
use oldbrainfix_gui::l10n::L10n;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn store() -> Result<Store, Box<dyn std::error::Error>> {
    Ok(Store::in_memory(Box::new(chrono::Utc::now), || {
        Ok(SigningKey::from_seed([1; 32]))
    })?)
}

fn harness(app: App) -> Harness<'static, App> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 1400.0))
        .build_ui_state(
            |ui, app: &mut App| {
                App::install(ui.ctx());
                app.show(ui);
            },
            app,
        );
    harness.step();
    harness
}

fn click(harness: &mut Harness<'_, App>, label: &str) {
    harness.get_by_label(label).click_accesskit();
    harness.step();
    harness.step();
}

/// The texts of the Notes in the Area "Old Brain Fix", sorted.
fn help_texts(store: &Store) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let area = Places::new(store)
        .areas()?
        .into_iter()
        .find(|a| a.name == "Old Brain Fix")
        .ok_or("the Area")?;
    assert_eq!(area.icon.as_deref(), Some("brain"));
    let mut texts: Vec<String> = Notes::new(store)
        .inside(&area.id)?
        .into_iter()
        .map(|n| n.text)
        .collect();
    texts.sort();
    Ok(texts)
}

#[test]
fn the_first_start_files_quick_start_and_the_icons_and_no_whats_new() -> TestResult {
    let mut app = App::new(store()?, "en");
    app.set_profile_defaults(Box::new(|store| {
        OwnProfile::new(store).apply_defaults("Desk", None)
    }));
    app.offer_restore();
    app.set_help("0.4.0")?;
    // Nothing before the first start is answered.
    assert!(Places::new(app.store()).areas()?.is_empty());
    let mut harness = harness(app);
    click(&mut harness, "New to Old Brain Fix");
    click(&mut harness, "Start");
    let desk = harness.state().store();
    let t = L10n::new("en");
    // Named "Quick start", the desktop's way folded under it (#260).
    assert_eq!(
        help_texts(desk)?,
        [
            format!("Quick start\n{}", quick_start_description(&t)),
            icons_text(&t)
        ]
    );
    assert!(Notes::new(desk).unsorted()?.is_empty());
    let person = OwnProfile::new(desk).get()?.ok_or("a Profile")?.id;
    let list = Checklists::new(desk)
        .by_note(&help_notes::quick_start_id(&person))?
        .ok_or("the Checklist")?;
    let lines: Vec<String> = list.lines.into_iter().map(|l| l.text).collect();
    assert_eq!(lines, quick_start_lines(&t));
    let icons = Notes::new(desk)
        .by_id(&help_notes::icons_id(&person))?
        .ok_or("the icons")?;
    assert_eq!(icons.photos.len(), GROUPS.len());
    // The Agenda shows the Quick start as one row with its count, not a
    // row per Line (Q549).
    harness.step();
    assert!(
        harness
            .query_all_by_label_contains("Quick start: 8 of 8 open")
            .next()
            .is_some()
    );
    for line in quick_start_lines(&t) {
        assert!(
            harness.query_all_by_label_contains(&line).next().is_none(),
            "{line}"
        );
    }
    Ok(())
}

#[test]
fn an_ordinary_start_makes_what_was_never_made_in_its_language_once() -> TestResult {
    let desk = store()?;
    OwnProfile::new(&desk).apply_defaults("Desk", None)?;
    Places::new(&desk).add_area("Doctor")?;
    let mut app = App::new(desk, "de");
    app.offer_restore();
    assert!(!app.welcoming());
    app.set_help("0.4.0")?;
    let t = L10n::new("de");
    // An update from before the help Notes also gets the bundled What's
    // new of this version, in the computer's language.
    let bundled = parse_whats_new(WHATS_NEW);
    let notes = bundled
        .get("0.4.0")
        .and_then(|n| n.get("de"))
        .ok_or("the bundled German What's new of 0.4.0")?;
    let mut expected = vec![
        format!("{}\n\n{notes}", t.help_whats_new_title("0.4.0")),
        format!("Schnellstart\n{}", quick_start_description(&t)),
        icons_text(&t),
    ];
    expected.sort();
    assert_eq!(help_texts(app.store())?, expected);
    assert_eq!(
        app.store().local_setting(VERSION_SETTING)?.as_deref(),
        Some("0.4.0")
    );
    // The window opened again: nothing more.
    let written = app.store().all_entries()?.len();
    app.set_help("0.4.0")?;
    assert_eq!(app.store().all_entries()?.len(), written);
    Ok(())
}

#[test]
fn an_app_not_handed_the_version_makes_none() -> TestResult {
    let desk = store()?;
    OwnProfile::new(&desk).apply_defaults("Desk", None)?;
    let mut app = App::new(desk, "en");
    app.offer_restore();
    let harness = harness(app);
    assert!(Places::new(harness.state().store()).areas()?.is_empty());
    Ok(())
}

#[test]
fn the_agenda_group_names_new_session_after_new_routine_as_the_phone() -> TestResult {
    let agenda = GROUPS
        .iter()
        .find(|g| g.slug == "agenda")
        .ok_or("the Agenda group")?;
    let t = L10n::new("en");
    let names: Vec<&str> = agenda.icons.iter().map(|(name, _)| name(&t)).collect();
    assert_eq!(
        names,
        [
            "Search and browse",
            "All Routines and Reminders",
            "New Routine or Reminder",
            "New Session",
            "Unsorted",
        ]
    );
    assert!(
        icons_text(&t).contains("\nNew Session: makes a Session with a new Note, in Advanced.")
    );
    assert!(
        icons_text(&L10n::new("de"))
            .contains("\nNeue Einheit: legt in Erweitert eine Einheit mit einer neuen Notiz an.")
    );
    Ok(())
}
