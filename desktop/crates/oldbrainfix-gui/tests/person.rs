//! Another person's Profile in the window (ADR 0017, #52): it opens from
//! their picture on a card, in an Area's people and on a Sent Item;
//! it shows their own name, picture, code and verified state, compares
//! codes, and keeps a name and picture for the user alone, which every
//! view shows; the Profile page lists People.

use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, Duration, Utc};
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::lan::{
    Joining, answer_sync, answer_sync_inviting, apply_answer_as, sync_request, sync_request_as,
};
use oldbrainfix_core::{
    AreaRole, Invite, Notes, OwnProfile, PairedDevice, PersonFilter, Places, SigningKey, Store,
    profile,
};
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// One clock for both Devices, a second further per read.
fn device(
    seed: u8,
    name: &str,
    ticks: &Arc<AtomicI64>,
) -> Result<Store, Box<dyn std::error::Error>> {
    let start = DateTime::parse_from_rfc3339("2026-03-01T09:00:00Z")?.with_timezone(&Utc);
    let ticks = ticks.clone();
    let clock = Box::new(move || start + Duration::seconds(ticks.fetch_add(1, Ordering::SeqCst)));
    let store = Store::in_memory(clock, move || Ok(SigningKey::from_seed([seed; 32])))?;
    let own = OwnProfile::new(&store);
    own.apply_defaults("Device", None)?;
    own.rename(name)?;
    Ok(store)
}

/// Ann's Household shared with Ben as a Member, and Ben's Note in it
/// synced to Ann.
struct Shared {
    ann: Store,
    ben: Store,
    area: String,
    /// Ben's Profile id.
    ben_id: String,
}

fn shared() -> Result<Shared, Box<dyn std::error::Error>> {
    let ticks = Arc::new(AtomicI64::new(0));
    let ann = device(1, "Ann", &ticks)?;
    let ben = device(2, "Ben", &ticks)?;
    let house = Places::new(&ann).add_area("Household")?;
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
        role: AreaRole::Member,
    };
    let (answer, _) =
        answer_sync_inviting(&ann, &request, "Desk", Some(&invite))?.ok_or("an answer")?;
    apply_answer_as(&ben, &answer, true, None)?;
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
    Notes::new(&ben).capture("Ben buys bread", Some(&area), None)?;
    let request = sync_request(&ben, &ann.version_vector()?, "Phone", &ann.device_id()?, "")?;
    let (answer, _) = answer_sync(&ann, &request, "Desk")?.ok_or("an answer")?;
    apply_answer_as(&ben, &answer, true, None)?;
    let ben_id = OwnProfile::new(&ben).get()?.ok_or("a Profile")?.id;
    Ok(Shared {
        ann,
        ben,
        area,
        ben_id,
    })
}

fn window(app: App) -> Harness<'static, App> {
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
    harness
}

fn app(store: Store, selection: Selection) -> App {
    let mut app = App::new(store, "en");
    app.set_sync_host(Some(IpAddr::V4(Ipv4Addr::LOCALHOST)), "Desk");
    app.select(selection);
    app
}

fn shows(harness: &Harness<'_, App>, label: &str) -> bool {
    harness.query_all_by_label_contains(label).next().is_some()
}

/// Clicks the last widget labelled `label`: a dialog's, over the
/// window's.
fn click(harness: &mut Harness<'_, App>, label: &str) {
    harness
        .get_all_by_label(label)
        .last()
        .expect(label)
        .click_accesskit();
    harness.run();
}

/// Types `text` into the field labelled `label` and leaves it.
fn type_into(harness: &mut Harness<'_, App>, label: &str, text: &str) {
    harness
        .get_by_role_and_label(Role::TextInput, label)
        .click();
    harness.run();
    harness
        .get_by_role_and_label(Role::TextInput, label)
        .type_text(text);
    harness.run();
    harness.key_press(egui::Key::Tab);
    harness.run();
}

#[test]
fn a_picture_on_a_card_opens_the_persons_profile_not_the_note() -> TestResult {
    let Shared {
        ann, area, ben_id, ..
    } = shared()?;
    let code = ann.person_code(&ben_id)?.ok_or("a code")?;
    let mut harness = window(app(ann, Selection::Place(area)));
    assert!(shows(&harness, "Written by Ben"));
    harness
        .get_by_role_and_label(Role::Button, "Open the Profile of Ben")
        .click();
    harness.run();
    assert_eq!(harness.state().selected_note(), None);
    assert!(shows(&harness, "Name and picture for me"));
    assert!(shows(&harness, &code));
    assert!(shows(&harness, "Not verified"));

    // A name for Ann alone: the card shows it once the Profile closes.
    type_into(&mut harness, "Name for me", "Mum");
    assert_eq!(harness.state().store().person_shown(&ben_id)?.name, "Mum");
    click(&mut harness, "Close");
    assert!(!shows(&harness, "Name and picture for me"));
    assert!(shows(&harness, "Written by Mum"));
    // Their own name stays theirs.
    assert_eq!(harness.state().store().person_theirs(&ben_id)?.name, "Ben");
    Ok(())
}

#[test]
fn codes_compared_there_mark_the_profile_verified() -> TestResult {
    let Shared {
        ann, area, ben_id, ..
    } = shared()?;
    let mut harness = window(app(ann, Selection::Place(area)));
    harness
        .get_by_role_and_label(Role::Button, "Open the Profile of Ben")
        .click();
    harness.run();
    click(&mut harness, "Compare codes");
    click(&mut harness, "Codes differ");
    assert!(shows(&harness, "The codes differ"));
    assert!(!harness.state().store().person_verified(&ben_id)?);
    click(&mut harness, "Compare codes");
    click(&mut harness, "Codes match");
    assert!(harness.state().store().person_verified(&ben_id)?);
    assert!(shows(&harness, "Verified: code compared in person."));
    assert!(!shows(&harness, "Compare codes"));
    Ok(())
}

#[test]
fn the_picture_for_me_is_a_square_the_user_picks() -> TestResult {
    let Shared {
        ann, area, ben_id, ..
    } = shared()?;
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("mum.png");
    let image = image::RgbImage::from_fn(300, 200, |x, _| image::Rgb([x as u8, 0, 255]));
    image::DynamicImage::ImageRgb8(image).save(&file)?;
    let mut app = app(ann, Selection::Place(area));
    app.set_picture_picker(Box::new(move |_| Some(file.clone())));
    let mut harness = window(app);
    harness
        .get_by_role_and_label(Role::Button, "Open the Profile of Ben")
        .click();
    harness.run();
    click(&mut harness, "Choose a picture");
    assert!(shows(&harness, "Choose the square"));
    click(&mut harness, "Use this part");
    let store = harness.state().store();
    let picture = store
        .person_override(&ben_id)?
        .and_then(|o| o.picture)
        .ok_or("a picture")?;
    assert_eq!(profile::picture_size(&picture), Some((256, 256)));
    assert_eq!(store.person_theirs(&ben_id)?.picture, None);
    assert!(shows(&harness, "Name and picture for me"));
    click(&mut harness, "Use their own picture");
    assert_eq!(harness.state().store().person_override(&ben_id)?, None);
    Ok(())
}

#[test]
fn an_areas_people_open_their_profile() -> TestResult {
    let Shared { ann, ben_id, .. } = shared()?;
    ann.set_override_name(&ben_id, "Mum")?;
    let mut harness = window(app(ann, Selection::All));
    harness
        .get_by_role_and_label(Role::Button, "Household")
        .click_secondary();
    harness.run();
    harness.get_by_label("People").click();
    harness.run();
    assert!(shows(&harness, "Ann (you)"));
    assert!(shows(&harness, "Mum"));
    assert!(harness.query_by_label("Open the Profile of Ann").is_none());
    click(&mut harness, "Open the Profile of Mum");
    assert!(shows(&harness, "Name and picture for me"));
    // Closing it shows the people again.
    click(&mut harness, "Close");
    assert!(shows(&harness, "Ann (you)"));
    Ok(())
}

#[test]
fn people_on_the_profile_page_lists_every_stored_profile() -> TestResult {
    let Shared { ann, ben_id, .. } = shared()?;
    ann.set_override_name(&ben_id, "Mum")?;
    let mut harness = window(app(ann, Selection::Profile));
    assert!(shows(&harness, "People"));
    click(&mut harness, "Open the Profile of Mum");
    assert!(shows(&harness, "Name and picture for me"));
    // Their own name heads it.
    assert!(shows(&harness, "Ben"));

    let ticks = Arc::new(AtomicI64::new(0));
    let alone = window(app(device(5, "Cid", &ticks)?, Selection::Profile));
    assert!(shows(&alone, "Nobody yet."));
    Ok(())
}

#[test]
fn a_sent_items_sender_opens_their_profile_once_kept() -> TestResult {
    let Shared {
        ann, ben, ben_id, ..
    } = shared()?;
    let note = Notes::new(&ben).capture("Dentist", None, None)?;
    let bytes = ben.send_note(&note.id, None, false)?;
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("from-ben.oldbrain");
    std::fs::write(&path, &bytes)?;
    let mut harness = window(app(ann, Selection::All));
    harness.input_mut().dropped_files = vec![egui::DroppedFile {
        path: Some(path),
        ..Default::default()
    }];
    harness.run();
    assert!(shows(&harness, "From Ben"));
    click(&mut harness, "Open the Profile of Ben");
    type_into(&mut harness, "Name for me", "Mum");
    click(&mut harness, "Close");
    // Back on the preview, the sender as Ann sees him now.
    assert!(shows(&harness, "From Mum"));
    assert_eq!(harness.state().store().person_shown(&ben_id)?.name, "Mum");

    // Never met: nothing to open before accepting.
    let ticks = Arc::new(AtomicI64::new(0));
    let mut alone = window(app(device(5, "Cid", &ticks)?, Selection::All));
    let again = dir.path().join("again.oldbrain");
    std::fs::write(&again, &bytes)?;
    alone.input_mut().dropped_files = vec![egui::DroppedFile {
        path: Some(again),
        ..Default::default()
    }];
    alone.run();
    assert!(shows(&alone, "From Ben"));
    assert!(alone.query_by_label("Open the Profile of Ben").is_none());
    Ok(())
}

/// Clicks the widget with the role `role` labelled `label`.
fn press(harness: &mut Harness<'_, App>, role: Role, label: &str) {
    harness.get_by_role_and_label(role, label).click();
    harness.run();
}

/// Ben's Note as Ann's window shows it, and one of Ann's own beside it.
fn two_notes(shared: &Shared) -> Result<(String, String), Box<dyn std::error::Error>> {
    let mine = Notes::new(&shared.ann).capture("Ann buys milk", Some(&shared.area), None)?;
    let theirs = Notes::new(&shared.ann)
        .all()?
        .into_iter()
        .find(|n| n.text == "Ben buys bread")
        .ok_or("Ben's Note")?;
    Ok((mine.id, theirs.id))
}

#[test]
fn their_notes_on_a_profile_shows_all_notes_with_only_theirs() -> TestResult {
    let shared = shared()?;
    let (mine, theirs) = two_notes(&shared)?;
    let Shared {
        ann, area, ben_id, ..
    } = shared;
    let mut harness = window(app(ann, Selection::Place(area)));
    press(&mut harness, Role::Button, "Open the Profile of Ben");
    press(&mut harness, Role::Button, "Their Notes");
    assert!(!shows(&harness, "Name and picture for me"));
    assert_eq!(harness.state().selection(), &Selection::All);
    assert_eq!(
        harness.state().people_filter(),
        &PersonFilter::person(&ben_id)
    );
    assert_eq!(harness.state().shown_notes(), vec![theirs.clone()]);
    assert!(shows(&harness, "Only Notes by"));
    // Taking Ben out shows everybody's again.
    press(&mut harness, Role::Button, "Ben");
    assert!(harness.state().people_filter().is_everyone());
    assert!(!shows(&harness, "Only Notes by"));
    let mut shown = harness.state().shown_notes();
    shown.sort();
    let mut both = vec![mine, theirs];
    both.sort();
    assert_eq!(shown, both);
    Ok(())
}

#[test]
fn the_person_filter_ticks_me_and_people_and_combines_with_the_search() -> TestResult {
    let shared = shared()?;
    let (mine, theirs) = two_notes(&shared)?;
    let unsorted = Notes::new(&shared.ann)
        .capture("Call the bank", None, None)?
        .id;
    let mut harness = window(app(shared.ann, Selection::Place(shared.area.clone())));
    press(&mut harness, Role::Button, "Show Notes by…");
    press(&mut harness, Role::CheckBox, "Me");
    assert_eq!(harness.state().shown_notes(), vec![mine.clone()]);
    // The menu stays open while people are ticked.
    press(&mut harness, Role::CheckBox, "Ben");
    assert_eq!(harness.state().shown_notes().len(), 2);
    press(&mut harness, Role::CheckBox, "Me");
    assert_eq!(harness.state().shown_notes(), vec![theirs.clone()]);
    harness.state_mut().set_query("milk");
    harness.run();
    assert!(harness.state().shown_notes().is_empty());
    harness.state_mut().set_query("bread");
    harness.run();
    assert_eq!(harness.state().shown_notes(), vec![theirs]);
    // Still open; "Everyone" closes it.
    press(&mut harness, Role::Button, "Everyone");
    assert!(harness.query_by_label("Everyone").is_none());
    assert!(harness.state().people_filter().is_everyone());
    // Each view has a filter of its own: Unsorted starts with everyone's.
    press(&mut harness, Role::Button, "Show Notes by…");
    press(&mut harness, Role::CheckBox, "Ben");
    assert!(!harness.state().people_filter().is_everyone());
    harness.state_mut().select(Selection::Unsorted);
    harness.run();
    assert!(harness.state().people_filter().is_everyone());
    assert_eq!(harness.state().shown_notes(), vec![unsorted.clone()]);
    press(&mut harness, Role::Button, "Show Notes by…");
    press(&mut harness, Role::CheckBox, "Ben");
    assert!(harness.state().shown_notes().is_empty());
    press(&mut harness, Role::CheckBox, "Ben");
    press(&mut harness, Role::CheckBox, "Me");
    assert_eq!(harness.state().shown_notes(), vec![unsorted]);
    Ok(())
}
