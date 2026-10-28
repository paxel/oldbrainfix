//! An Area shared from afar in the window (#108, ADR 0023): "Share
//! with…" in an Area's Share dialog lists the People it can go to, marks
//! those not verified, asks once how changes travel with them, asks for
//! the shared folder when they travel by one, and saves the offer where
//! the user says. A bundle holding an offer, dropped onto the window,
//! shows "Ann shares Household with you as Member" with Compare codes,
//! Accept, Decline and "Decline and block"; one left undecided waits
//! behind the Agenda's "x shared Areas waiting". Each refusal says its
//! own cause. A person's Profile offers "Send changes to Ann" when
//! changes travel with them as files. A click on the Changes Nudge (#109)
//! asks where to save each waiting person's bundle, one after another;
//! only a bundle saved ends their wait.

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Duration, Utc};
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::bundle::{import_bundle, write_area_offer_now, write_person_bundle_now};
use oldbrainfix_core::offers::{OfferError, Transport};
use oldbrainfix_core::{Alarms, AreaRole, Notes, OwnProfile, Places, SigningKey, Store};
use oldbrainfix_gui::app::FileSaver;
use oldbrainfix_gui::l10n::L10n;
use oldbrainfix_gui::sync_view::offer_refusal;
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A Device with its own Profile named `name`, on a clock of its own.
fn device(seed: u8, name: &str) -> Result<Store, Box<dyn std::error::Error>> {
    let start = DateTime::parse_from_rfc3339("2026-03-01T09:00:00Z")?.with_timezone(&Utc);
    let ticks = Arc::new(AtomicI64::new(0));
    let clock = Box::new(move || start + Duration::seconds(ticks.fetch_add(1, Ordering::SeqCst)));
    let store = Store::in_memory(clock, move || Ok(SigningKey::from_seed([seed; 32])))?;
    let own = OwnProfile::new(&store);
    own.apply_defaults("Device", None)?;
    own.rename(name)?;
    Ok(store)
}

fn id_of(store: &Store) -> Result<String, Box<dyn std::error::Error>> {
    Ok(OwnProfile::new(store).get()?.ok_or("a Profile")?.id)
}

/// Ann, with her Household and a Note in it, and Ben, who keeps her
/// Profile card as she keeps his: the stores, the Area's id and Ben's
/// Profile id.
fn met() -> Result<(Store, Store, String, String), Box<dyn std::error::Error>> {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let house = Places::new(&ann).add_area("Household")?;
    Notes::new(&ann).capture("Buy milk", Some(&house.id), None)?;
    ann.open_profile_card(&ben.profile_card(false)?.ok_or("his card")?, false)?;
    ben.open_profile_card(&ann.profile_card(false)?.ok_or("her card")?, false)?;
    let ben_id = id_of(&ben)?;
    assert!(!ann.offer_devices(&ben_id)?.is_empty());
    Ok((ann, ben, house.id, ben_id))
}

/// The offer of the Area `area` to `person`, shared as `transport`, in
/// `dir`; the id the Area went on under.
fn offered(
    ann: &Store,
    area: &str,
    person: &str,
    transport: Transport,
    dir: &Path,
) -> Result<(PathBuf, String), Box<dyn std::error::Error>> {
    let shared = ann.share_area_with(area, person, AreaRole::Member, transport)?;
    let path = dir.join("household.obfsync");
    write_area_offer_now(ann, &path, &shared, person)?;
    Ok((path, shared))
}

fn window(app: App) -> Harness<'static, App> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 1000.0))
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

fn app(store: Store, selection: Selection, locale: &str) -> App {
    let mut app = App::new(store, locale);
    app.select(selection);
    app
}

fn shows(harness: &Harness<'_, App>, label: &str) -> bool {
    harness.query_all_by_label_contains(label).next().is_some()
}

/// Clicks the last widget labelled `label`: a dialog's, over the
/// window's.
fn click(harness: &mut Harness<'_, App>, label: &str) {
    match harness.get_all_by_label(label).last() {
        Some(node) => node.click_accesskit(),
        None => panic!("no {label}"),
    }
    harness.run();
}

fn drop_file(harness: &mut Harness<'_, App>, path: &Path) {
    harness.input_mut().dropped_files = vec![egui::DroppedFile {
        path: Some(path.to_path_buf()),
        ..Default::default()
    }];
    harness.run();
}

/// A save dialog answering `path` and noting the titles it was asked
/// with.
fn saver(path: PathBuf, titles: Arc<Mutex<Vec<String>>>) -> FileSaver {
    Box::new(move |title, name| {
        assert!(name.ends_with(".obfsync"), "{name}");
        if let Ok(mut titles) = titles.lock() {
            titles.push(title.to_string());
        }
        Some(path.clone())
    })
}

/// Opens the Share dialog of the Area `label` on the left.
fn open_share(harness: &mut Harness<'_, App>, label: &str) {
    harness
        .get_by_role_and_label(Role::Button, label)
        .click_secondary();
    harness.run();
    click(harness, "Share…");
}

#[test]
fn sharing_by_files_saves_an_offer_the_other_window_accepts() -> TestResult {
    let (ann, ben, area, ben_id) = met()?;
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("for-ben.obfsync");
    let titles = Arc::new(Mutex::new(Vec::new()));
    let mut desk = app(ann, Selection::Place(area), "en");
    desk.set_bundle_dialogs(saver(file.clone(), titles.clone()), Box::new(|_| None));
    let mut harness = window(desk);
    open_share(&mut harness, "Household");
    assert!(shows(&harness, "Someone not nearby"));
    click(&mut harness, "Manager");
    click(&mut harness, "Share with…");
    assert!(shows(&harness, "Share Household with…"));
    assert!(shows(&harness, "Not verified"));
    click(&mut harness, "Ben");
    assert!(shows(&harness, "How do changes travel with Ben?"));
    click(&mut harness, "Files through a messenger");
    click(&mut harness, "Share");
    assert_eq!(
        harness.state().notice(),
        Some("Saved. Send the file to them through any messenger or mail.")
    );
    assert_eq!(
        *titles.lock().map_err(|e| e.to_string())?,
        ["Save the shared Area to send"]
    );
    assert!(file.exists());
    assert_eq!(
        harness.state().store().transport_with(&ben_id)?,
        Some(Transport::Bundle)
    );

    // Ben's window shows who shares what, and takes it in on Accept.
    let mut harness = window(app(ben, Selection::Agenda, "en"));
    drop_file(&mut harness, &file);
    assert!(harness.state().previewing());
    assert!(shows(&harness, "Ann shares Household with you as Manager"));
    assert!(shows(
        &harness,
        "Changes travel as files through a messenger."
    ));
    assert!(shows(
        &harness,
        "Nothing of the Area is kept until you accept."
    ));
    click(&mut harness, "Accept");
    assert_eq!(harness.state().notice(), Some("Shared Area accepted."));
    assert!(!harness.state().previewing());
    let store = harness.state().store();
    assert!(store.pending_sets()?.is_empty());
    assert!(
        Places::new(store)
            .areas()?
            .iter()
            .any(|a| a.name == "Household")
    );
    Ok(())
}

#[test]
fn sharing_through_a_folder_picks_it_and_asks_how_only_once() -> TestResult {
    let (ann, ben, area, ben_id) = met()?;
    let second = Places::new(&ann).add_area("Garden shed")?;
    let dir = tempfile::tempdir()?;
    let folder = dir.path().join("shared");
    std::fs::create_dir(&folder)?;
    let file = dir.path().join("for-ben.obfsync");
    let titles = Arc::new(Mutex::new(Vec::new()));
    let mut desk = app(ann, Selection::Place(area), "en");
    desk.set_bundle_dialogs(saver(file.clone(), titles.clone()), Box::new(|_| None));
    {
        let folder = folder.clone();
        desk.set_folder_picker(Box::new(move |title| {
            assert_eq!(title, "Choose the shared folder");
            Some(folder.clone())
        }));
    }
    let mut harness = window(desk);
    open_share(&mut harness, "Household");
    click(&mut harness, "Share with…");
    click(&mut harness, "Ben");
    click(&mut harness, "A shared folder you both reach");
    click(&mut harness, "Share");
    let store = harness.state().store();
    assert_eq!(store.transport_with(&ben_id)?, Some(Transport::Folder));
    assert_eq!(
        store.sync_folder()?,
        Some(folder.to_string_lossy().into_owned())
    );
    assert!(file.exists());

    // The second Area goes the same way without asking again.
    harness.state_mut().select(Selection::Place(second.id));
    harness.run();
    open_share(&mut harness, "Garden shed");
    click(&mut harness, "Share with…");
    click(&mut harness, "Ben");
    assert!(!shows(&harness, "How do changes travel with Ben?"));
    assert_eq!(titles.lock().map_err(|e| e.to_string())?.len(), 2);

    // Ben accepts the second one, saved over the first, and is asked
    // for the folder.
    let other = tempfile::tempdir()?;
    let mut phone = app(ben, Selection::Agenda, "en");
    {
        let folder = other.path().to_path_buf();
        phone.set_folder_picker(Box::new(move |_| Some(folder.clone())));
    }
    let mut harness = window(phone);
    std::fs::rename(&file, dir.path().join("second.obfsync"))?;
    drop_file(&mut harness, &dir.path().join("second.obfsync"));
    assert!(shows(
        &harness,
        "Changes travel through a shared folder you both reach."
    ));
    click(&mut harness, "Accept");
    assert!(shows(&harness, "Choose the shared folder?"));
    click(&mut harness, "Choose folder");
    assert_eq!(
        harness.state().store().sync_folder()?,
        Some(other.path().to_string_lossy().into_owned())
    );
    Ok(())
}

#[test]
fn backing_out_of_the_save_dialog_shares_nothing() -> TestResult {
    let (ann, _ben, area, ben_id) = met()?;
    let mut desk = app(ann, Selection::Place(area.clone()), "en");
    desk.set_bundle_dialogs(Box::new(|_, _| None), Box::new(|_| None));
    let mut harness = window(desk);
    open_share(&mut harness, "Household");
    click(&mut harness, "Share with…");
    click(&mut harness, "Ben");
    click(&mut harness, "Share");
    let store = harness.state().store();
    assert_eq!(store.transport_with(&ben_id)?, None);
    assert!(store.people(&area)?.is_empty());
    assert_eq!(harness.state().notice(), None);
    Ok(())
}

#[test]
fn nobody_to_share_with_from_afar_says_how_to_add_them() -> TestResult {
    let ann = device(1, "Ann")?;
    let house = Places::new(&ann).add_area("Household")?;
    let mut harness = window(app(ann, Selection::Place(house.id), "en"));
    open_share(&mut harness, "Household");
    assert!(shows(&harness, "Nobody in People can get it yet."));
    assert!(!shows(&harness, "Share with…"));
    Ok(())
}

#[test]
fn an_offer_waits_behind_the_agenda_line_until_declined() -> TestResult {
    let (ann, ben, area, ben_id) = met()?;
    let dir = tempfile::tempdir()?;
    let (file, _) = offered(&ann, &area, &ben_id, Transport::Bundle, dir.path())?;
    assert!(import_bundle(&ben, &file)?.offer.is_some());
    let mut harness = window(app(ben, Selection::Agenda, "en"));
    assert!(shows(&harness, "1 shared Area waiting"));
    click(&mut harness, "1 shared Area waiting");
    assert_eq!(harness.state().selection(), &Selection::AreaOffersWaiting);
    assert!(shows(&harness, "Shared Areas waiting"));
    click(&mut harness, "Ann shares Household with you as Member");
    assert!(harness.state().previewing());
    click(&mut harness, "Decline");
    assert_eq!(
        harness.state().notice(),
        Some("Declined: nothing was kept.")
    );
    let store = harness.state().store();
    assert!(store.pending_sets()?.is_empty());
    assert!(
        !Places::new(store)
            .areas()?
            .iter()
            .any(|a| a.name == "Household")
    );
    assert!(shows(&harness, "No shared Areas are waiting."));
    // Back goes to the Agenda, where the line is gone.
    harness.key_press_modifiers(egui::Modifiers::ALT, egui::Key::ArrowLeft);
    harness.run();
    assert_eq!(harness.state().selection(), &Selection::Agenda);
    assert!(!shows(&harness, "shared Area waiting"));
    Ok(())
}

#[test]
fn decline_and_block_blocks_the_sharer_and_keeps_nothing() -> TestResult {
    let (ann, ben, area, ben_id) = met()?;
    let ann_id = id_of(&ann)?;
    let dir = tempfile::tempdir()?;
    let (file, _) = offered(&ann, &area, &ben_id, Transport::Bundle, dir.path())?;
    let mut harness = window(app(ben, Selection::Agenda, "en"));
    drop_file(&mut harness, &file);
    click(&mut harness, "Decline and block");
    assert!(shows(&harness, "Block Ann?"));
    click(&mut harness, "Block");
    assert_eq!(
        harness.state().notice(),
        Some("Declined and blocked: nothing was kept.")
    );
    let store = harness.state().store();
    assert!(store.is_blocked(&ann_id)?);
    assert!(store.pending_sets()?.is_empty());
    Ok(())
}

#[test]
fn comparing_codes_marks_the_sharer_verified_on_accept() -> TestResult {
    let (ann, ben, area, ben_id) = met()?;
    let ann_id = id_of(&ann)?;
    let dir = tempfile::tempdir()?;
    let (file, _) = offered(&ann, &area, &ben_id, Transport::Bundle, dir.path())?;
    let mut harness = window(app(ben, Selection::Agenda, "en"));
    drop_file(&mut harness, &file);
    click(&mut harness, "Compare codes");
    click(&mut harness, "Codes match");
    assert!(shows(&harness, "Verified"));
    click(&mut harness, "Accept");
    assert!(harness.state().store().person_verified(&ann_id)?);
    Ok(())
}

#[test]
fn an_offer_refused_says_why() -> TestResult {
    let (ann, ben, area, ben_id) = met()?;
    let ann_id = id_of(&ann)?;
    let dir = tempfile::tempdir()?;
    let (file, _) = offered(&ann, &area, &ben_id, Transport::Bundle, dir.path())?;

    // From a person blocked: refused, nothing held.
    ben.block_person(&ann_id, false)?;
    let mut harness = window(app(ben, Selection::Agenda, "en"));
    drop_file(&mut harness, &file);
    assert!(!harness.state().previewing());
    assert_eq!(
        harness.state().notice(),
        Some("Refused: this shared Area comes from a person you blocked.")
    );
    assert!(harness.state().store().pending_sets()?.is_empty());

    // Sealed for another person's Devices.
    let carl = device(3, "Carl")?;
    let mut harness = window(app(carl, Selection::Agenda, "en"));
    drop_file(&mut harness, &file);
    assert!(!harness.state().previewing());
    assert_eq!(
        harness.state().notice(),
        Some(
            "This shared Area is meant for another person or another of your Devices. \
             Ask the sender to share it again with you."
        )
    );

    // The others, each in words of its own.
    let t = L10n::new("en");
    assert_eq!(
        offer_refusal(&t, OfferError::Refused),
        "Refused: this Area is your own on this Device already."
    );
    assert_eq!(
        offer_refusal(&t, OfferError::NotAnOffer),
        "This file holds no shared Area this app can read. Ask the sender to share it again."
    );
    Ok(())
}

#[test]
fn send_changes_saves_a_bundle_for_the_person_or_says_nothing_is_new() -> TestResult {
    let (ann, ben, area, ben_id) = met()?;
    let ann_id = id_of(&ann)?;
    let dir = tempfile::tempdir()?;
    let (file, _) = offered(&ann, &area, &ben_id, Transport::Bundle, dir.path())?;
    let changes = dir.path().join("changes.obfsync");
    let titles = Arc::new(Mutex::new(Vec::new()));
    let mut desk = app(ann, Selection::Profile, "en");
    desk.set_bundle_dialogs(saver(changes.clone(), titles.clone()), Box::new(|_| None));
    let mut harness = window(desk);
    click(&mut harness, "Open the Profile of Ben");
    assert!(shows(&harness, "As a file through any messenger."));
    click(&mut harness, "Send changes to Ben");
    assert_eq!(
        harness.state().notice(),
        Some("Saved. Send the file to them through any messenger or mail.")
    );
    assert_eq!(
        *titles.lock().map_err(|e| e.to_string())?,
        ["Save the changes to send"]
    );
    assert!(changes.exists());

    // Once Ben holds all of it, nothing is new for him.
    let held = import_bundle(&ben, &file)?.offer.ok_or("an offer")?;
    ben.accept_area_offer(&held.id, false)?;
    let back = dir.path().join("back.obfsync");
    write_person_bundle_now(&ben, &back, &ann_id)?.ok_or("a bundle back")?;
    import_bundle(harness.state().store(), &back)?;
    click(&mut harness, "Send changes to Ben");
    assert_eq!(
        harness.state().notice(),
        Some("Nothing new to send to Ben.")
    );
    assert_eq!(titles.lock().map_err(|e| e.to_string())?.len(), 1);
    Ok(())
}

#[test]
fn no_send_changes_while_changes_travel_by_a_folder() -> TestResult {
    let (ann, _ben, area, ben_id) = met()?;
    let dir = tempfile::tempdir()?;
    offered(&ann, &area, &ben_id, Transport::Folder, dir.path())?;
    let mut harness = window(app(ann, Selection::Profile, "en"));
    click(&mut harness, "Open the Profile of Ben");
    assert!(shows(&harness, "Name and picture for me"));
    assert!(!shows(&harness, "Send changes to Ben"));
    Ok(())
}

#[test]
fn the_offer_preview_speaks_german() -> TestResult {
    let (ann, ben, area, ben_id) = met()?;
    let dir = tempfile::tempdir()?;
    let (file, _) = offered(&ann, &area, &ben_id, Transport::Bundle, dir.path())?;
    let mut harness = window(app(ben, Selection::Agenda, "de"));
    drop_file(&mut harness, &file);
    assert!(shows(&harness, "Ann teilt Household mit dir als Mitglied"));
    assert!(shows(&harness, "Annehmen"));
    assert!(shows(&harness, "Ablehnen und blockieren"));
    Ok(())
}

#[test]
fn leaving_the_preview_decides_nothing() -> TestResult {
    let (ann, ben, area, ben_id) = met()?;
    let dir = tempfile::tempdir()?;
    let (file, _) = offered(&ann, &area, &ben_id, Transport::Bundle, dir.path())?;
    let mut harness = window(app(ben, Selection::Agenda, "en"));
    drop_file(&mut harness, &file);
    assert!(harness.state().previewing());
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(!harness.state().previewing());
    assert_eq!(harness.state().store().pending_sets()?.len(), 1);
    assert!(shows(&harness, "1 shared Area waiting"));
    Ok(())
}

#[test]
fn blocking_the_sharer_on_their_profile_drops_the_offer() -> TestResult {
    let (ann, ben, area, ben_id) = met()?;
    let dir = tempfile::tempdir()?;
    let (file, _) = offered(&ann, &area, &ben_id, Transport::Bundle, dir.path())?;
    let mut harness = window(app(ben, Selection::Agenda, "en"));
    drop_file(&mut harness, &file);
    click(&mut harness, "Open the Profile of Ann");
    click(&mut harness, "Block");
    assert!(shows(&harness, "Block Ann?"));
    click(&mut harness, "Block");
    click(&mut harness, "Close");
    assert!(!harness.state().previewing());
    assert_eq!(
        harness.state().notice(),
        Some("Refused: this shared Area comes from a person you blocked.")
    );
    assert!(harness.state().store().pending_sets()?.is_empty());
    Ok(())
}

/// A save dialog answering each call with the next of `answers`, `None`
/// being cancelled, and noting the titles and names it was asked with.
fn answering(answers: Vec<Option<PathBuf>>, asked: Arc<Mutex<Vec<(String, String)>>>) -> FileSaver {
    let answers = Mutex::new(VecDeque::from(answers));
    Box::new(move |title, name| {
        if let Ok(mut asked) = asked.lock() {
            asked.push((title.to_string(), name.to_string()));
        }
        answers.lock().ok()?.pop_front().flatten()
    })
}

/// Ann of [`met`], who also shares a Garden with Cat by bundles, and then
/// adds a Note for Ben and one for Cat: both wait, Ben first. The store
/// with Ben's and Cat's Profile ids.
fn two_waiting() -> Result<(Store, String, String), Box<dyn std::error::Error>> {
    let (ann, _ben, area, ben_id) = met()?;
    let cat = device(3, "Cat")?;
    ann.open_profile_card(&cat.profile_card(false)?.ok_or("her card")?, false)?;
    let cat_id = id_of(&cat)?;
    let house = ann.share_area_with(&area, &ben_id, AreaRole::Member, Transport::Bundle)?;
    let garden = Places::new(&ann).add_area("Garden")?;
    let garden = ann.share_area_with(&garden.id, &cat_id, AreaRole::Member, Transport::Bundle)?;
    assert!(ann.waiting_for_changes()?.is_empty());
    Notes::new(&ann).capture("Buy bread", Some(&house), None)?;
    Notes::new(&ann).capture("Water the roses", Some(&garden), None)?;
    let people = Alarms::new(&ann)
        .changes_nudge(&|utc| utc.naive_utc())?
        .ok_or("the Changes Nudge")?
        .people;
    assert_eq!(people, [ben_id.clone(), cat_id.clone()]);
    Ok((ann, ben_id, cat_id))
}

#[test]
fn a_click_on_the_changes_nudge_saves_each_bundle_and_a_cancel_keeps_the_wait() -> TestResult {
    let (ann, ben, cat) = two_waiting()?;
    let dir = tempfile::tempdir()?;
    let for_ben = dir.path().join("for-ben.obfsync");
    let asked = Arc::new(Mutex::new(Vec::new()));
    let wanted = Arc::new(AtomicBool::new(true));
    let mut desk = app(ann, Selection::Agenda, "en");
    desk.set_bundle_dialogs(
        answering(vec![Some(for_ben.clone()), None], asked.clone()),
        Box::new(|_| None),
    );
    desk.set_changes_wanted(wanted.clone());
    let harness = window(desk);
    assert!(!wanted.load(Ordering::SeqCst));
    let asked = asked.lock().map_err(|e| e.to_string())?.clone();
    let titles: Vec<&str> = asked.iter().map(|(t, _)| t.as_str()).collect();
    assert_eq!(
        titles,
        [
            "Save the changes for Ben to send",
            "Save the changes for Cat to send"
        ]
    );
    assert!(asked[0].1.ends_with("-Ben.obfsync"), "{}", asked[0].1);
    assert!(asked[1].1.ends_with("-Cat.obfsync"), "{}", asked[1].1);
    assert!(for_ben.exists());
    assert_eq!(
        harness.state().notice(),
        Some("Saved. Send the file to them through any messenger or mail.")
    );
    // Ben's bundle was saved: he no longer waits. Cat's dialog was
    // cancelled: she keeps the Changes Nudge.
    let waiting = harness.state().store().waiting_for_changes()?;
    assert!(!waiting.contains_key(&ben));
    assert!(waiting.contains_key(&cat));
    Ok(())
}

#[test]
fn a_bundle_that_could_not_be_saved_counts_as_not_sent() -> TestResult {
    let (ann, ben, _cat) = two_waiting()?;
    let dir = tempfile::tempdir()?;
    let nowhere = dir.path().join("gone").join("for-ben.obfsync");
    let asked = Arc::new(Mutex::new(Vec::new()));
    let wanted = Arc::new(AtomicBool::new(true));
    let mut desk = app(ann, Selection::Agenda, "de");
    desk.set_bundle_dialogs(
        answering(vec![Some(nowhere.clone()), None], asked.clone()),
        Box::new(|_| None),
    );
    desk.set_changes_wanted(wanted);
    let harness = window(desk);
    assert_eq!(
        asked.lock().map_err(|e| e.to_string())?[0].0,
        "Änderungen für Ben zum Senden speichern"
    );
    assert!(!nowhere.exists());
    assert!(harness.state().notice().is_some_and(|n| !n.is_empty()));
    let waiting = harness.state().store().waiting_for_changes()?;
    assert!(waiting.contains_key(&ben));
    Ok(())
}

#[test]
fn send_changes_on_the_profile_ends_the_wait_only_once_saved() -> TestResult {
    let (ann, ben, cat) = two_waiting()?;
    let dir = tempfile::tempdir()?;
    let for_ben = dir.path().join("for-ben.obfsync");
    let asked = Arc::new(Mutex::new(Vec::new()));
    let mut desk = app(ann, Selection::Profile, "en");
    desk.set_bundle_dialogs(
        answering(vec![None, Some(for_ben.clone())], asked.clone()),
        Box::new(|_| None),
    );
    let mut harness = window(desk);
    click(&mut harness, "Open the Profile of Ben");
    // Cancelled: nothing saved, he still waits.
    click(&mut harness, "Send changes to Ben");
    assert!(!for_ben.exists());
    assert!(
        harness
            .state()
            .store()
            .waiting_for_changes()?
            .contains_key(&ben)
    );
    // Saved: he no longer waits; Cat still does.
    click(&mut harness, "Send changes to Ben");
    assert!(for_ben.exists());
    let waiting = harness.state().store().waiting_for_changes()?;
    assert!(!waiting.contains_key(&ben));
    assert!(waiting.contains_key(&cat));
    assert_eq!(
        asked.lock().map_err(|e| e.to_string())?[0].0,
        "Save the changes to send"
    );
    Ok(())
}
