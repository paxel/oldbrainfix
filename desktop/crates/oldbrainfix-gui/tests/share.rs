//! Shared Areas in the window (ADR 0013): the shared mark and the people
//! on the left, what a Member may not do kept out of reach, sharing an
//! Area by hosting its invite, and a Manager changing roles.

use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, Duration, Utc};
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::lan::{Joining, answer_sync_inviting, apply_answer_as, sync_request_as};
use oldbrainfix_core::{AreaAccess, AreaRole, Invite, Notes, Places, SigningKey, Store};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A store on a clock that moves a second per read.
fn device(seed: u8) -> Result<Store, Box<dyn std::error::Error>> {
    let start = DateTime::parse_from_rfc3339("2026-03-01T09:00:00Z")?.with_timezone(&Utc);
    let ticks = Arc::new(AtomicI64::new(0));
    let clock = Box::new(move || start + Duration::seconds(ticks.fetch_add(1, Ordering::SeqCst)));
    Ok(Store::in_memory(clock, move || {
        Ok(SigningKey::from_seed([seed; 32]))
    })?)
}

/// Ann's Household with her Note in it, shared with Ben as `role`: both
/// stores and the Area's id.
fn shared(role: AreaRole) -> Result<(Store, Store, String), Box<dyn std::error::Error>> {
    let ann = device(1)?;
    let ben = device(2)?;
    let house = Places::new(&ann).add_area("Household")?;
    Notes::new(&ann).capture("Buy milk", Some(&house.id), None)?;
    let area = ann.share(&house.id, "Ann")?;
    let request = sync_request_as(
        &ben,
        &ann.version_vector()?,
        "Phone",
        &ann.device_id()?,
        "",
        Joining::Invited,
    )?;
    let invite = Invite {
        area: area.clone(),
        role,
        name: "Ben".into(),
    };
    let (answer, _) =
        answer_sync_inviting(&ann, &request, "Desk", Some(&invite))?.ok_or("an answer")?;
    apply_answer_as(&ben, &answer, true, None)?;
    Ok((ann, ben, area))
}

fn harness(store: Store, selection: Selection) -> Harness<'static, App> {
    let mut app = App::new(store, "en");
    app.select(selection);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 800.0))
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

/// Opens the context menu of the left pane's row `label`.
fn open_menu(harness: &mut Harness<'_, App>, label: &str) {
    harness
        .get_by_role_and_label(Role::Button, label)
        .click_secondary();
    harness.run();
}

#[test]
fn a_member_sees_who_shares_and_only_adds_to_someone_elses_note() -> TestResult {
    let (_ann, ben, area) = shared(AreaRole::Member)?;
    let milk = Notes::new(&ben)
        .all()?
        .into_iter()
        .next()
        .ok_or("a Note")?
        .id;
    let mut harness = harness(ben, Selection::Place(area));
    assert!(shows(&harness, "Shared · Ben, Ann"));
    harness.state_mut().select_note(&milk);
    harness.run();
    assert!(shows(&harness, "Add to Note"));
    assert!(shows(
        &harness,
        "Only a Manager of this Area can change this."
    ));
    for hidden in ["Move to trash", "Merge into…", "File into…"] {
        assert!(!shows(&harness, hidden), "{hidden}");
    }
    open_menu(&mut harness, "Household");
    assert!(shows(&harness, "People"));
    for hidden in ["Rename", "Share…", "Delete", "New Topic"] {
        assert!(!shows(&harness, hidden), "{hidden}");
    }
    Ok(())
}

#[test]
fn sharing_an_area_hosts_its_invite() -> TestResult {
    let ann = device(1)?;
    let house = Places::new(&ann).add_area("Household")?;
    let mut harness = harness(ann, Selection::All);
    open_menu(&mut harness, "Household");
    harness.get_by_label("Share…").click();
    harness.run();
    // The own name's field has the keyboard; the other one still shows
    // its hint.
    harness.get_by(|n| n.is_focused()).type_text("Ann");
    harness.run();
    harness
        .get_by(|n| n.placeholder() == Some("Name") && n.value().as_deref() == Some(""))
        .click();
    harness.run();
    harness.get_by(|n| n.is_focused()).type_text("Ben");
    harness.run();
    harness.get_by_label("Manager").click();
    harness.run();
    harness.get_by_label("Show the code").click();
    harness.step();
    harness.step();
    assert_eq!(harness.state().selection(), &Selection::Sync);
    assert!(harness.state().store().is_shared(&house.id)?);
    assert!(shows(
        &harness,
        "Inviting Ben to Household as Manager. The code below is for them."
    ));
    let invite = harness
        .state()
        .sync_host()
        .inviting
        .clone()
        .ok_or("an invite")?;
    assert_eq!(invite.role, AreaRole::Manager);
    harness.get_by_label("Stop inviting").click();
    harness.step();
    assert!(harness.state().sync_host().inviting.is_none());
    assert!(harness.state().sync_host().hosting());
    Ok(())
}

#[test]
fn the_own_name_starts_as_the_own_profiles() -> TestResult {
    let ann = device(1)?;
    oldbrainfix_core::OwnProfile::new(&ann).apply_defaults("Ann Smith", None)?;
    Places::new(&ann).add_area("Household")?;
    let mut harness = harness(ann, Selection::All);
    open_menu(&mut harness, "Household");
    harness.get_by_label("Share…").click();
    harness.run();
    assert!(harness.query_all_by_value("Ann Smith").next().is_some());
    Ok(())
}

#[test]
fn a_manager_changes_roles_and_removes_people() -> TestResult {
    let (ann, _ben, area) = shared(AreaRole::Member)?;
    let mut harness = harness(ann, Selection::All);
    open_menu(&mut harness, "Household");
    harness.get_by_label("People").click();
    harness.run();
    assert!(shows(&harness, "Ann (you)"));
    harness.get_by_label("Make Manager").click();
    harness.run();
    let store = harness.state().store();
    let ben = store.people(&area)?.into_iter().find(|p| p.name == "Ben");
    assert_eq!(ben.map(|p| p.role), Some(AreaRole::Manager));
    harness.get_by_label("Remove").click();
    harness.run();
    assert!(shows(&harness, "Remove Ben from Household?"));
    harness
        .get_all_by_label("Remove")
        .last()
        .ok_or("Remove")?
        .click();
    harness.run();
    let store = harness.state().store();
    let names: Vec<String> = store.people(&area)?.into_iter().map(|p| p.name).collect();
    assert_eq!(names, vec!["Ann".to_string()]);
    assert_eq!(store.access_in(&area, None)?, AreaAccess::Manager);
    Ok(())
}
