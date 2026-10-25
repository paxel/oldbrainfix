//! Shared Areas in the window (ADR 0013): the shared mark and the people
//! on the left, what a Member may not do kept out of reach, sharing an
//! Area by hosting its invite, the invited person's Profile shown to
//! confirm (ADR 0017), other people's pictures on the cards, and a
//! Manager changing roles.

use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::thread::JoinHandle;

use chrono::{DateTime, Duration, Utc};
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::lan::{
    DEVICE_HEADER, FORMAT_HEADER, Joining, OWN_FORMATS, PIN_HEADER, answer_sync,
    answer_sync_inviting, apply_answer_as, sync_request, sync_request_as,
};
use oldbrainfix_core::{
    AreaAccess, AreaRole, Invite, Notes, OwnProfile, PairedDevice, Places, SigningKey, Store,
};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A store on a clock that moves a second per read, with its own
/// Profile named `name`.
fn device(seed: u8, name: &str) -> Result<Store, Box<dyn std::error::Error>> {
    let start = DateTime::parse_from_rfc3339("2026-03-01T09:00:00Z")?.with_timezone(&Utc);
    let ticks = Arc::new(AtomicI64::new(0));
    let clock = Box::new(move || start + Duration::seconds(ticks.fetch_add(1, Ordering::SeqCst)));
    let store = Store::in_memory(clock, move || Ok(SigningKey::from_seed([seed; 32])))?;
    OwnProfile::new(&store).apply_defaults(name, None)?;
    Ok(store)
}

/// Ann's Household with her Note in it, shared with Ben as `role`: both
/// stores and the Area's id.
fn shared(role: AreaRole) -> Result<(Store, Store, String), Box<dyn std::error::Error>> {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let house = Places::new(&ann).add_area("Household")?;
    Notes::new(&ann).capture("Buy milk", Some(&house.id), None)?;
    let area = ann.share(&house.id)?;
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
    };
    let (answer, _) =
        answer_sync_inviting(&ann, &request, "Desk", Some(&invite))?.ok_or("an answer")?;
    apply_answer_as(&ben, &answer, true, None)?;
    // Each remembers the other as another person's Device, as the apps do.
    for (store, peer) in [(&ann, &ben), (&ben, &ann)] {
        store.remember_device(&PairedDevice {
            id: peer.device_id()?,
            name: String::new(),
            fingerprint: String::new(),
            host: None,
            port: None,
            last_sync: store.now(),
            shared: true,
        })?;
    }
    Ok((ann, ben, area))
}

fn harness(store: Store, selection: Selection) -> Harness<'static, App> {
    let mut app = App::new(store, "en");
    app.set_sync_host(Some(IpAddr::V4(Ipv4Addr::LOCALHOST)), "Desk");
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

/// A phone with its own Profile named Ben, invited by the host on
/// `port` with `pin`: the Notes it holds afterwards and the names of the
/// Profiles of other people it keeps, or the refusal as its status and
/// text.
type Joined = Result<(Vec<String>, Vec<String>), String>;

fn invited_phone(port: u16, pin: String) -> JoinHandle<Joined> {
    std::thread::spawn(move || {
        let fail = |e: &dyn std::fmt::Display| e.to_string();
        let phone = device(2, "Ben").map_err(|e| fail(&e))?;
        let agent = ureq::Agent::new_with_config(
            ureq::Agent::config_builder()
                .tls_config(
                    ureq::tls::TlsConfig::builder()
                        .disable_verification(true)
                        .build(),
                )
                .http_status_as_error(false)
                .build(),
        );
        let base = format!("https://127.0.0.1:{port}");
        let mut res = agent
            .get(format!("{base}/vector"))
            .header(PIN_HEADER, &pin)
            .header(FORMAT_HEADER, OWN_FORMATS.encode())
            .call()
            .map_err(|e| fail(&e))?;
        let host_device = res
            .headers()
            .get(DEVICE_HEADER)
            .and_then(|v| v.to_str().ok())
            .unwrap_or_default()
            .to_string();
        let vector: HashMap<String, i64> =
            serde_json::from_str(&res.body_mut().read_to_string().map_err(|e| fail(&e))?)
                .map_err(|e| fail(&e))?;
        let body = sync_request_as(
            &phone,
            &vector,
            "Pixel",
            &host_device,
            &pin,
            Joining::Invited,
        )
        .map_err(|e| fail(&e))?;
        let mut res = agent
            .post(format!("{base}/sync"))
            .header(PIN_HEADER, &pin)
            .content_type("application/json")
            .send(body.to_string().as_bytes())
            .map_err(|e| fail(&e))?;
        let text = res.body_mut().read_to_string().unwrap_or_default();
        if res.status().as_u16() != 200 {
            return Err(format!("{} {text}", res.status().as_u16()));
        }
        let answer = serde_json::from_str(&text).map_err(|e| fail(&e))?;
        apply_answer_as(&phone, &answer, true, None).map_err(|e| fail(&e))?;
        let notes = Notes::new(&phone).all().map_err(|e| fail(&e))?;
        let people = phone.kept_people().map_err(|e| fail(&e))?;
        Ok((
            notes.into_iter().map(|n| n.text).collect(),
            people.into_iter().map(|p| p.name).collect(),
        ))
    })
}

/// Steps the window until `done` holds, at most a few seconds.
fn step_until(harness: &mut Harness<'_, App>, done: impl Fn(&App) -> bool) -> TestResult {
    for _ in 0..400 {
        if done(harness.state()) {
            return Ok(());
        }
        harness.step();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    Err("the window never got there".into())
}

/// Opens the Share dialog of the Household, picks Manager and shows the
/// code: the host's port and PIN.
fn start_inviting(
    harness: &mut Harness<'_, App>,
) -> Result<(u16, String), Box<dyn std::error::Error>> {
    open_menu(harness, "Household");
    harness.get_by_label("Share…").click();
    harness.run();
    harness.get_by_label("Manager").click();
    harness.run();
    harness.get_by_label("Show the code").click();
    step_until(harness, |app| app.sync_host().hosting())?;
    harness.step();
    let host = harness.state().sync_host().host().ok_or("hosting")?;
    Ok((host.port(), host.pin().to_string()))
}

#[test]
fn sharing_an_area_hosts_its_invite() -> TestResult {
    let ann = device(1, "Ann")?;
    let house = Places::new(&ann).add_area("Household")?;
    let mut harness = harness(ann, Selection::All);
    open_menu(&mut harness, "Household");
    harness.get_by_label("Share…").click();
    harness.run();
    // Nobody types a name: the other person's Profile shows later.
    assert!(!shows(&harness, "Your name"));
    assert!(!shows(&harness, "Their name"));
    harness.get_by_label("Manager").click();
    harness.run();
    harness.get_by_label("Show the code").click();
    harness.step();
    harness.step();
    assert_eq!(harness.state().selection(), &Selection::Sync);
    assert!(harness.state().store().is_shared(&house.id)?);
    assert!(shows(
        &harness,
        "Inviting to Household as Manager. The code below is for the other person; their \
         Profile shows here to confirm before anything is shared."
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
fn the_invited_persons_profile_shows_before_anything_is_shared() -> TestResult {
    let ann = device(1, "Ann")?;
    let house = Places::new(&ann).add_area("Household")?;
    Notes::new(&ann).capture("Buy milk", Some(&house.id), None)?;
    let mut harness = harness(ann, Selection::All);
    let (port, pin) = start_inviting(&mut harness)?;

    // Refused, the phone hears so and gets nothing.
    let refused = invited_phone(port, pin.clone());
    step_until(&mut harness, |app| app.sync_host().asking().is_some())?;
    harness.step();
    assert!(shows(&harness, "Share Household with this person?"));
    assert!(shows(&harness, "Ben"));
    let code = harness
        .state()
        .sync_host()
        .asking_profile()
        .ok_or("a Profile")?
        .code
        .clone();
    assert!(shows(&harness, &format!("Their code: {code}")));
    harness.get_by_label("Refuse").click_accesskit();
    harness.step();
    assert_eq!(
        refused.join().map_err(|_| "joiner")?,
        Err("403 declined".to_string())
    );
    let store = harness.state().store();
    let area = store.areas_shared_with(&store.device_id()?)?;
    let area = area.iter().next().ok_or("the shared Area")?.clone();
    assert_eq!(store.people(&area)?.len(), 1);

    // Confirmed, the Area goes, and each keeps the other's Profile.
    let shared = invited_phone(port, pin);
    step_until(&mut harness, |app| app.sync_host().asking().is_some())?;
    harness.step();
    harness.get_by_label("Share").click_accesskit();
    harness.step();
    let (notes, people) = shared.join().map_err(|_| "joiner")??;
    assert_eq!(notes, vec!["Buy milk".to_string()]);
    assert_eq!(people, vec!["Ann".to_string()]);
    let store = harness.state().store();
    let mut names = Vec::new();
    for person in store.people(&area)? {
        names.push(store.person_shown(&person.id)?.name);
    }
    assert_eq!(names, ["Ann", "Ben"]);
    assert_eq!(
        store.people(&area)?.last().map(|p| p.role),
        Some(AreaRole::Manager)
    );
    Ok(())
}

#[test]
fn another_persons_note_shows_their_picture_and_ones_own_none() -> TestResult {
    let (ann, ben, area) = shared(AreaRole::Member)?;
    let bread = Notes::new(&ben).capture("Ben buys bread", Some(&area), None)?;
    let request = sync_request(&ben, &ann.version_vector()?, "Phone", &ann.device_id()?, "")?;
    let (answer, _) = answer_sync(&ann, &request, "Desk")?.ok_or("an answer")?;
    apply_answer_as(&ben, &answer, true, None)?;
    assert_eq!(
        ann.author_of(&bread.id)?.map(|p| p.name),
        Some("Ben".to_string())
    );
    let harness = harness(ann, Selection::Place(area.clone()));
    assert!(shows(&harness, "Ben buys bread"));
    assert!(shows(&harness, "Written by Ben"));
    assert!(!shows(&harness, "Written by Ann"));
    let harness = self::harness(ben, Selection::Place(area));
    assert!(shows(&harness, "Written by Ann"));
    assert!(!shows(&harness, "Written by Ben"));
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
    let mut ben = None;
    for p in store.people(&area)? {
        if store.person_shown(&p.id)?.name == "Ben" {
            ben = Some(p);
        }
    }
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
    let mut names = Vec::new();
    for p in store.people(&area)? {
        names.push(store.person_shown(&p.id)?.name);
    }
    assert_eq!(names, vec!["Ann".to_string()]);
    assert_eq!(store.access_in(&area, None)?, AreaAccess::Manager);
    Ok(())
}
