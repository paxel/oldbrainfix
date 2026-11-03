//! Send feedback in the window (#214): Settings ▸ "Send feedback" opens
//! the page with the version, the system and the install method ticked
//! and Notes as a Pack not; the preview shows exactly the text Mail's
//! link carries and "Save as file…" writes; Mail waits for words and for
//! a text that fits a mail link; Notes ticked open the Pack's tree,
//! nothing ticked, and "Save the Pack…" saves the Notes ticked to be
//! attached. Nothing leaves without a click.

// A test may unwrap, expect, panic and index, its helpers too (#229).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use egui::accesskit::{Role, Toggled};
use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::{Harness, Node};
use oldbrainfix_core::{Notes, Places, SigningKey, Store};
use oldbrainfix_gui::feedback_view::{self, FeedbackFacts, MAIL_BODY_LIMIT};
use oldbrainfix_gui::{App, Page};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// What was asked of a saver or the browser.
type Seen = Arc<Mutex<Vec<String>>>;

fn facts() -> FeedbackFacts {
    FeedbackFacts {
        version: "0.4.0".into(),
        system: "Debian GNU/Linux 13 (trixie), x86_64".into(),
        install: "Flatpak".into(),
    }
}

struct Opened {
    harness: Harness<'static, App>,
    urls: Seen,
    saved: Seen,
}

/// The window with the Settings open, saving into `dir`, and the page
/// opened from them.
fn open(store: Store, dir: &Path) -> Opened {
    let mut app = App::new(store, "en");
    app.set_feedback_facts(facts());
    let (urls, saved) = (Seen::default(), Seen::default());
    let seen = urls.clone();
    app.set_url_opener(Box::new(move |url| {
        if let Ok(mut urls) = seen.lock() {
            urls.push(url.to_string());
        }
    }));
    for pack in [false, true] {
        let (seen, dir) = (saved.clone(), dir.to_path_buf());
        let saver: oldbrainfix_gui::app::FileSaver = Box::new(move |_, name| {
            if let Ok(mut saved) = seen.lock() {
                saved.push(name.to_string());
            }
            Some(dir.join(name))
        });
        if pack {
            app.set_pack_saver(saver);
        } else {
            app.set_feedback_saver(saver);
        }
    }
    app.open(Page::Settings);
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
    click(&mut harness, "Send feedback");
    Opened {
        harness,
        urls,
        saved,
    }
}

fn store() -> Result<Store, Box<dyn std::error::Error>> {
    Ok(Store::in_memory(Box::new(chrono::Utc::now), || {
        Ok(SigningKey::from_seed([5; 32]))
    })?)
}

fn last<'a>(harness: &'a Harness<'_, App>, label: &'a str) -> Node<'a> {
    match harness.get_all_by_label(label).last() {
        Some(node) => node,
        None => panic!("no {label}"),
    }
}

fn click(harness: &mut Harness<'_, App>, label: &str) {
    last(harness, label).click_accesskit();
    harness.run();
}

fn shows(harness: &Harness<'_, App>, label: &str) -> bool {
    harness.query_all_by_label_contains(label).next().is_some()
}

fn ticked(harness: &Harness<'_, App>, label: &str) -> bool {
    harness
        .get_by_role_and_label(Role::CheckBox, label)
        .accesskit_node()
        .toggled()
        == Some(Toggled::True)
}

fn disabled(harness: &Harness<'_, App>, label: &str) -> bool {
    last(harness, label).accesskit_node().is_disabled()
}

fn type_text(harness: &mut Harness<'_, App>, text: &str) {
    harness
        .get_by_label("What happened / what could be better")
        .click();
    harness.run();
    harness.get_by(|n| n.is_focused()).type_text(text);
    harness.run();
}

fn seen(list: &Seen) -> Vec<String> {
    list.lock().map(|l| l.clone()).unwrap_or_default()
}

#[test]
fn the_ticks_build_the_preview_and_mail_and_save_send_exactly_it() -> TestResult {
    let dir = tempfile::tempdir()?;
    let Opened {
        mut harness,
        urls,
        saved,
    } = open(store()?, dir.path());
    assert_eq!(harness.state().page(), Some(Page::Feedback));
    for tick in ["Version", "OS and device", "Install method"] {
        assert!(ticked(&harness, tick), "{tick}");
    }
    assert!(!ticked(&harness, "Notes as a Pack"));
    // The desktop keeps no crash report, so it offers none.
    assert!(!shows(&harness, "crash report"));
    assert!(shows(&harness, "Write what happened first."));
    assert!(disabled(&harness, "Mail"));
    assert!(disabled(&harness, "Save as file…"));

    type_text(&mut harness, "It counts wrong.");
    harness
        .get_by_role_and_label(Role::CheckBox, "OS and device")
        .click();
    harness.run();
    let body = "It counts wrong.\n\nVersion: 0.4.0\nInstalled from: Flatpak";
    assert!(shows(&harness, body));
    assert!(seen(&urls).is_empty(), "nothing leaves without a click");

    click(&mut harness, "Mail");
    assert_eq!(seen(&urls), [feedback_view::mail_url(body)]);
    click(&mut harness, "Save as file…");
    let names = seen(&saved);
    assert_eq!(names.len(), 1);
    assert!(names[0].starts_with("oldbrainfix-feedback-"), "{names:?}");
    assert_eq!(std::fs::read_to_string(dir.path().join(&names[0]))?, body);
    Ok(())
}

#[test]
fn a_text_too_long_for_a_mail_link_is_only_saved() -> TestResult {
    let dir = tempfile::tempdir()?;
    let Opened { mut harness, .. } = open(store()?, dir.path());
    type_text(&mut harness, &"x".repeat(MAIL_BODY_LIMIT + 1));
    assert!(disabled(&harness, "Mail"));
    assert!(!disabled(&harness, "Save as file…"));
    assert!(shows(&harness, "Too long for a mail link."));
    Ok(())
}

#[test]
fn notes_picked_in_the_packs_tree_are_saved_as_a_pack_to_attach() -> TestResult {
    let desk = store()?;
    let health = Places::new(&desk).add_area("Health")?;
    Notes::new(&desk).capture("Drink water", Some(&health.id), None)?;
    Notes::new(&desk).capture("Loose thought", None, None)?;
    let dir = tempfile::tempdir()?;
    let Opened {
        mut harness, saved, ..
    } = open(desk, dir.path());
    type_text(&mut harness, "Look at these.");
    harness
        .get_by_role_and_label(Role::CheckBox, "Notes as a Pack")
        .click();
    harness.run();
    assert!(shows(&harness, "Health"));
    assert!(!ticked(&harness, "Drink water"), "nothing ticked at first");
    assert!(!shows(&harness, "Save the Pack…"));
    harness
        .get_by_role_and_label(Role::CheckBox, "Drink water")
        .click();
    harness.run();
    assert!(shows(&harness, "A mail link cannot carry the Pack"));
    assert!(shows(&harness, "With it, as a file: a Pack of 1 Note"));

    click(&mut harness, "Save the Pack…");
    let names = seen(&saved);
    let pack = names.last().ok_or("a Pack saved")?;
    let bytes = std::fs::read(PathBuf::from(dir.path()).join(pack))?;
    let opened = store()?.open_pack(&bytes)?.map_err(|e| e.name())?;
    let texts: Vec<_> = opened.pack.notes.iter().map(|n| n.text.as_str()).collect();
    assert_eq!(texts, ["Drink water"]);
    // Unticked, the Notes go no more.
    harness
        .get_by_role_and_label(Role::CheckBox, "Notes as a Pack")
        .click();
    harness.run();
    assert!(!shows(&harness, "Save the Pack…"));
    Ok(())
}
