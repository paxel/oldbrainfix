//! App links in the window (#261, ADR 0046): a Markdown link with the
//! app's scheme opens its Note, Area or Topic, the Agenda, Unsorted or one
//! of the app's pages; one whose target is not here says "Not available
//! here"; a guide page opens in the browser in the app's language; a file
//! link asks once, naming the file and its app, and only for a file of
//! the synced folder; "Link to…" in the format bar puts one in from a
//! picker.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, Duration, Utc};
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::app_link::AppLink;
use oldbrainfix_core::text_format::MARKDOWN;
use oldbrainfix_core::{Notes, OwnProfile, Places, SigningKey, Store};
use oldbrainfix_gui::{App, Page, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A store on a clock that moves a second per read, with its own
/// Profile.
fn device() -> Result<Store, Box<dyn std::error::Error>> {
    let start = DateTime::parse_from_rfc3339("2026-11-05T09:00:00Z")?.with_timezone(&Utc);
    let ticks = Arc::new(AtomicI64::new(0));
    let clock = Box::new(move || start + Duration::seconds(ticks.fetch_add(1, Ordering::SeqCst)));
    let store = Store::in_memory(clock, move || Ok(SigningKey::from_seed([3; 32])))?;
    store.set_local_setting("author", "Desk")?;
    OwnProfile::new(&store).apply_defaults("Desk", None)?;
    Ok(store)
}

/// The window over `store` in English with the Note `note` open.
fn window(store: &Arc<Store>, note: &str) -> Harness<'static, App> {
    let mut app = App::new(store.clone(), "en");
    app.select(Selection::All);
    app.select_note(note);
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

/// Clicks the link `label`, the last of that name.
fn follow(harness: &mut Harness<'_, App>, label: &str) -> TestResult {
    harness
        .query_all_by_role_and_label(Role::Link, label)
        .last()
        .ok_or(format!("the link {label:?}"))?
        .click_accesskit();
    harness.run();
    Ok(())
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

#[test]
fn a_link_opens_its_note_place_and_page_and_one_not_here_says_so() -> TestResult {
    let store = Arc::new(device()?);
    let notes = Notes::new(&store);
    let work = Places::new(&store).add_area("Work")?;
    let dentist = notes.capture_as("Dentist on Monday", "plain", None, None)?;
    let gone = notes.capture_as("Old one", "plain", None, None)?;
    notes.trash(&gone.id)?;
    let text = format!(
        "[the appointment]({})\n[the old one]({})\n[my work]({})\n\
         [my backups](oldbrainfix://settings/backups)\n[the unsorted](oldbrainfix://unsorted)\n\
         [alarm settings](oldbrainfix://settings/alarms)\n\
         [nowhere](oldbrainfix://nowhere)",
        AppLink::Note(dentist.id.clone()).url(),
        AppLink::Note(gone.id.clone()).url(),
        AppLink::Place(work.id.clone()).url(),
    );
    let links = notes.capture_as(&text, MARKDOWN, Some(&work.id), None)?;
    let mut harness = window(&store, &links.id);
    // The scheme's unknown address is no link.
    assert_eq!(
        harness
            .query_all_by_role_and_label(Role::Link, "nowhere")
            .count(),
        0
    );

    let reopen = |harness: &mut Harness<'_, App>| {
        harness.state_mut().select_note(&links.id);
        harness.run();
    };
    follow(&mut harness, "the unsorted")?;
    assert_eq!(harness.state().selection(), &Selection::Unsorted);
    reopen(&mut harness);
    follow(&mut harness, "my work")?;
    assert_eq!(
        harness.state().selection(),
        &Selection::Place(work.id.clone())
    );
    reopen(&mut harness);
    follow(&mut harness, "my backups")?;
    assert_eq!(harness.state().page(), Some(Page::Backups));
    reopen(&mut harness);
    // Scrolled to its section over the frames the dialog takes.
    harness
        .query_all_by_role_and_label(Role::Link, "alarm settings")
        .last()
        .ok_or("the link")?
        .click_accesskit();
    harness.run_steps(8);
    assert_eq!(harness.state().page(), Some(Page::Settings));
    harness.run();
    reopen(&mut harness);

    follow(&mut harness, "the old one")?;
    assert_eq!(harness.state().notice(), Some("Not available here"));
    assert_eq!(harness.state().selected_note(), Some(links.id.as_str()));
    follow(&mut harness, "the appointment")?;
    assert_eq!(harness.state().selected_note(), Some(dentist.id.as_str()));
    Ok(())
}

#[test]
fn a_guide_link_opens_the_website_in_the_language() -> TestResult {
    let store = Arc::new(device()?);
    let note = Notes::new(&store).capture_as(
        "[the guide](oldbrainfix://guide/getting-started#set-up)",
        MARKDOWN,
        None,
        None,
    )?;
    let mut harness = window(&store, &note.id);
    let opened = Rc::new(RefCell::new(Vec::new()));
    let kept = opened.clone();
    harness
        .state_mut()
        .set_url_opener(Box::new(move |url| kept.borrow_mut().push(url.to_string())));
    follow(&mut harness, "the guide")?;
    assert_eq!(
        *opened.borrow(),
        ["https://paxel.github.io/oldbrainfix/en/guide/getting-started.html#set-up"]
    );
    Ok(())
}

#[test]
fn a_file_link_asks_once_and_only_for_a_file_of_the_synced_folder() -> TestResult {
    let base = std::env::temp_dir().join(format!("obf-file-links-{}", std::process::id()));
    let folder = base.join("Sync");
    std::fs::create_dir_all(folder.join("Scans"))?;
    std::fs::write(folder.join("Scans/tax 2026.pdf"), b"%PDF")?;
    std::fs::write(base.join("secret.txt"), b"x")?;
    let store = Arc::new(device()?);
    store.choose_sync_folder(folder.to_str().ok_or("a path")?)?;
    let note = Notes::new(&store).capture_as(
        "[the tax](oldbrainfix://file/Scans/tax%202026.pdf)\n\
         [a missing one](oldbrainfix://file/nope.pdf)",
        MARKDOWN,
        None,
        None,
    )?;
    let mut harness = window(&store, &note.id);
    follow(&mut harness, "the tax")?;
    let asked = harness
        .query_all_by_label_contains("Open \"Scans/tax 2026.pdf\" from the shared folder with")
        .count();
    // Nothing opened before the answer; Cancel leaves it.
    click(&mut harness, "Cancel")?;
    let after = harness
        .query_all_by_label_contains("from the shared folder with")
        .count();
    follow(&mut harness, "a missing one")?;
    let notice = harness.state().notice().map(str::to_string);
    drop(std::fs::remove_dir_all(&base));
    assert_eq!(asked, 1);
    assert_eq!(after, 0);
    assert_eq!(notice.as_deref(), Some("Not available here"));
    Ok(())
}

#[test]
fn link_to_puts_the_picked_target_in_with_its_name() -> TestResult {
    let store = Arc::new(device()?);
    let note = Notes::new(&store).capture_as("see ", "plain", None, None)?;
    let mut harness = window(&store, &note.id);
    harness
        .query_all_by_role_and_label(Role::Button, "Link to…")
        .next()
        .ok_or("Link to…")?
        .click_accesskit();
    harness.run();
    // The search has the keyboard.
    harness.event(egui::Event::Text("backups".into()));
    harness.run();
    assert_eq!(
        harness.query_all_by_label("Settings ▸ Alarms").count(),
        0,
        "the search leaves out what it does not find"
    );
    click(&mut harness, "Backups")?;
    let stored = Notes::new(&store).by_id(&note.id)?.ok_or("the Note")?;
    assert_eq!(stored.text_format, MARKDOWN);
    assert!(
        stored.text.contains("](oldbrainfix://settings/backups)"),
        "{}",
        stored.text
    );
    Ok(())
}
