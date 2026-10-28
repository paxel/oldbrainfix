//! Frogs in the window (#101, ADR 0027): the Agenda's "Eat a frog"
//! section under today's Occurrences offers two, "Another one" swaps one
//! and checking one off hides the other for the day; its heading opens
//! the Frog view with the folded "Done" section, closed until opened; a
//! right-click deletes a Frog after one question and the trash restores
//! it; the Note editor makes one. Which two Frogs show depends on the
//! rotation, so the tests count them rather than name them.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use egui::accesskit::{Role, Toggled};
use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use oldbrainfix_core::{Frogs, Notes, OwnProfile, SigningKey, Store};
use oldbrainfix_gui::l10n::L10n;
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// The jobs the Frogs are spawned from, oldest first.
const JOBS: [&str; 3] = ["Wash the windows", "Sort the DVDs", "Clean the cupboard"];

fn berlin(utc: DateTime<Utc>) -> NaiveDateTime {
    utc.with_timezone(&chrono_tz::Europe::Berlin).naive_local()
}

/// A store whose clock ticks a second a read from Monday noon, the same
/// day on every wall clock near Europe, with a Frog spawned from each of
/// [`JOBS`].
fn device() -> Result<Arc<Store>, Box<dyn std::error::Error>> {
    let start = DateTime::parse_from_rfc3339("2026-03-02T12:00:00Z")?.with_timezone(&Utc);
    let ticks = Arc::new(AtomicI64::new(0));
    let clock = Box::new(move || start + Duration::seconds(ticks.fetch_add(1, Ordering::SeqCst)));
    let store = Store::in_memory(clock, || Ok(SigningKey::from_seed([5; 32])))?;
    OwnProfile::new(&store).apply_defaults("Device", None)?;
    for job in JOBS {
        let note = Notes::new(&store).capture(job, None, None)?;
        Frogs::new(&store).spawn(&note.id)?;
    }
    Ok(Arc::new(store))
}

/// The window over `store` in `locale`, on the Agenda.
fn window(store: &Arc<Store>, locale: &str) -> Harness<'static, App> {
    let mut app = App::new(store.clone(), locale);
    app.set_wall_clock(Arc::new(berlin));
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

/// The jobs whose Frog row shows.
fn shown(harness: &Harness<'_, App>) -> Vec<&'static str> {
    JOBS.into_iter()
        .filter(|job| {
            harness
                .query_all_by_role_and_label(Role::Button, job)
                .next()
                .is_some()
        })
        .collect()
}

/// The box that checks the Frog of `job` off.
fn checked(harness: &Harness<'_, App>, job: &str) -> bool {
    harness
        .get_by_label(&format!("Done: {job}"))
        .accesskit_node()
        .toggled()
        == Some(Toggled::True)
}

/// Clicks the last widget labelled `label`: a dialog's, over the
/// window.
fn click(harness: &mut Harness<'_, App>, label: &str) -> TestResult {
    harness
        .query_all_by_label(label)
        .last()
        .ok_or(format!("{label:?}"))?
        .click();
    harness.run();
    Ok(())
}

#[test]
fn the_agenda_offers_two_frogs_and_checking_one_hides_the_other() -> TestResult {
    let store = device()?;
    let t = L10n::new("en");
    let mut harness = window(&store, "en");
    assert!(shows(&harness, t.eat_a_frog()));
    let two = shown(&harness);
    assert_eq!(two.len(), 2, "{two:?}");
    assert_eq!(harness.query_all_by_label(t.another_frog()).count(), 2);
    // Under today's Occurrences, before tomorrow.
    let heading = harness
        .get_by_role_and_label(Role::Button, t.eat_a_frog())
        .rect()
        .top();
    assert!(harness.get_by_label(t.agenda_today()).rect().top() < heading);
    assert!(heading < harness.get_by_label(t.agenda_tomorrow()).rect().top());

    harness.get_by_label(&format!("Done: {}", two[0])).click();
    harness.run();
    assert_eq!(shown(&harness), vec![two[0]]);
    assert!(checked(&harness, two[0]));
    assert!(!shows(&harness, t.another_frog()));
    assert!(shows(&harness, t.eat_a_frog()), "the heading stays");

    // Unchecked, the other shows again.
    harness.get_by_label(&format!("Done: {}", two[0])).click();
    harness.run();
    assert_eq!(shown(&harness), two);
    assert!(Frogs::new(&store).done()?.is_empty());
    Ok(())
}

#[test]
fn another_one_swaps_a_frog_for_the_next() -> TestResult {
    let store = device()?;
    let t = L10n::new("en");
    let mut harness = window(&store, "en");
    let before = shown(&harness);
    harness
        .query_all_by_label(t.another_frog())
        .next()
        .ok_or("Another one")?
        .click();
    harness.run();
    let after = shown(&harness);
    assert_eq!(after.len(), 2, "{after:?}");
    assert_ne!(after, before);
    let third = JOBS.into_iter().find(|j| !before.contains(j));
    assert!(after.iter().any(|j| Some(*j) == third), "{after:?}");
    Ok(())
}

#[test]
fn the_section_goes_without_open_frogs() -> TestResult {
    let store = device()?;
    let frogs = Frogs::new(&store);
    for frog in frogs.open()? {
        frogs.trash(&frog.id)?;
    }
    let harness = window(&store, "en");
    assert!(!shows(&harness, L10n::new("en").eat_a_frog()));
    Ok(())
}

#[test]
fn the_heading_opens_the_frog_view_with_the_done_fold_closed() -> TestResult {
    let store = device()?;
    let t = L10n::new("en");
    let frogs = Frogs::new(&store);
    let eaten = frogs.open()?.remove(0);
    frogs.mark_done(&eaten.id, None)?;
    let mut harness = window(&store, "en");
    harness
        .get_by_role_and_label(Role::Button, t.eat_a_frog())
        .click();
    harness.run();
    assert_eq!(harness.state().selection(), &Selection::Frogs);
    assert!(shows(&harness, t.frogs_title()));
    // The open ones, oldest first; the Done one folded away.
    let open: Vec<f32> = JOBS[1..]
        .iter()
        .map(|job| {
            harness
                .get_by_role_and_label(Role::Button, job)
                .rect()
                .top()
        })
        .collect();
    assert!(open[0] < open[1]);
    assert!(!shown(&harness).contains(&JOBS[0]));
    assert!(shows(&harness, t.frogs_done()));

    click(&mut harness, t.frogs_done())?;
    assert!(shown(&harness).contains(&JOBS[0]));
    assert!(checked(&harness, JOBS[0]));
    assert!(
        harness
            .query_all_by_label_contains("Done on")
            .next()
            .is_some()
    );
    assert_eq!(
        store.local_setting("frogsDoneOpen")?.as_deref(),
        Some("true")
    );
    // Its box unchecks it.
    harness.get_by_label(&format!("Done: {}", JOBS[0])).click();
    harness.run();
    assert!(frogs.done()?.is_empty());

    // A click on a row opens its Note; back goes to the Agenda.
    harness.get_by_role_and_label(Role::Button, JOBS[1]).click();
    harness.run();
    let note = frogs.open()?.remove(1).note_id;
    assert_eq!(harness.state().selected_note(), Some(note.as_str()));
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowLeft);
    harness.run();
    assert_eq!(harness.state().selection(), &Selection::Agenda);
    Ok(())
}

#[test]
fn a_right_click_deletes_a_frog_after_one_question_and_the_trash_restores_it() -> TestResult {
    let store = device()?;
    let t = L10n::new("en");
    let mut harness = window(&store, "en");
    let harness = &mut harness;
    harness.state_mut().select(Selection::Frogs);
    harness.run();
    harness
        .get_by_role_and_label(Role::Button, JOBS[0])
        .click_secondary();
    harness.run();
    harness.get_by_label(t.delete()).click();
    harness.run();
    assert!(shows(harness, t.delete_frog_question()));
    click(harness, t.delete())?;
    assert!(!shows(harness, t.delete_frog_question()));
    let frogs = Frogs::new(&store);
    assert_eq!(frogs.open()?.len(), 2);
    assert!(!shown(harness).contains(&JOBS[0]));

    harness.state_mut().select(Selection::Trash);
    harness.run();
    assert!(shows(harness, JOBS[0]));
    click(harness, t.restore())?;
    assert_eq!(frogs.open()?.len(), 3);
    Ok(())
}

#[test]
fn the_note_editor_makes_a_frog() -> TestResult {
    let store = device()?;
    let t = L10n::new("en");
    let note = Notes::new(&store).capture("Defrost the freezer", None, None)?;
    let mut harness = window(&store, "en");
    harness.state_mut().select_note(&note.id);
    harness.run();
    harness
        .get_by_role_and_label(Role::Button, t.frog())
        .click();
    harness.run();
    let frogs = Frogs::new(&store).of_note(&note.id)?;
    assert_eq!(frogs.len(), 1);
    assert!(
        !Notes::new(&store)
            .unsorted()?
            .iter()
            .any(|n| n.id == note.id)
    );
    // One open Frog at a time: the button goes while it is open.
    assert!(
        harness
            .query_all_by_role_and_label(Role::Button, t.frog())
            .next()
            .is_none()
    );
    assert!(shows(&harness, &format!("Done: {}", t.frog())));
    Ok(())
}

#[test]
fn the_section_speaks_german() -> TestResult {
    let store = device()?;
    let harness = window(&store, "de");
    assert!(shows(&harness, "Iss einen Frosch"));
    assert_eq!(harness.query_all_by_label("Einen anderen").count(), 2);
    Ok(())
}
