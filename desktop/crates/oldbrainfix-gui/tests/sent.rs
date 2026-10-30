//! Sent Items in the window, driven headless: a Note is sent from its
//! editor, encrypted to a person met or signed only with a hint; a Sent
//! Item dropped onto the window or handed over shows its preview, and
//! Accept takes it into Unsorted with its Routines while Decline keeps
//! nothing. A Note sent before goes as an update or a cancellation; an
//! update shows what changes and applies it, a cancellation trashes the
//! Note, and one signed by another key is refused. The preview shows
//! the sender's Profile; accepting keeps it, its picture shows on the
//! Note, and comparing its short code marks it verified (#51). A
//! stranger's item shows only who it is from until "Show", is accepted,
//! declined or blocked before it too, and waits behind the Agenda's
//! "x Sent Items waiting" until decided (#91); declined before it,
//! nothing of its sender's picture or its Note's photos and recordings
//! is decoded or played.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use oldbrainfix_core::profile::{self, SignedProfile};
use oldbrainfix_core::{
    ClockTime, Day, ImportReport, Notes, OwnProfile, PairedDevice, Routines, Schedule, SigningKey,
    Store,
};
use oldbrainfix_gui::media::Player;
use oldbrainfix_gui::schedule_text::schedule_text;
use oldbrainfix_gui::{App, Selection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn store(seed: u8, author: &str) -> Result<Store, Box<dyn std::error::Error>> {
    let store = Store::in_memory(Box::new(chrono::Utc::now), move || {
        Ok(SigningKey::from_seed([seed; 32]))
    })?;
    store.set_local_setting("author", author)?;
    OwnProfile::new(&store).apply_defaults(author, None)?;
    Ok(store)
}

/// A PNG of `width` × `height`, red on the left half, blue on the right.
fn halves(width: u32, height: u32) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let image = image::RgbImage::from_fn(width, height, |x, _| {
        if x < width / 2 {
            image::Rgb([255, 0, 0])
        } else {
            image::Rgb([0, 0, 255])
        }
    });
    let mut out = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(image).write_to(&mut out, image::ImageFormat::Png)?;
    Ok(out.into_inner())
}

/// The short code of `store`'s own Profile.
fn code(store: &Store) -> Result<String, Box<dyn std::error::Error>> {
    Ok(OwnProfile::new(store).code()?.ok_or("code")?)
}

/// `other` is another person's Device paired by invite with `store`, its
/// key met in person.
fn meet(store: &Store, other: &Store, name: &str) -> TestResult {
    let id = other.device_id()?;
    store.remember_device(&PairedDevice {
        id: id.clone(),
        name: name.into(),
        fingerprint: String::new(),
        host: None,
        port: None,
        last_sync: chrono::Utc::now(),
        shared: true,
    })?;
    store.learn_keys(
        &[other.own_key_record()?],
        Some(&id),
        &mut ImportReport::default(),
    )?;
    Ok(())
}

fn window(app: App) -> Harness<'static, App> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1280.0, 900.0))
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
    harness.query_all_by_label_contains(label).next().is_some()
}

fn drop_file(harness: &mut Harness<'_, App>, path: &std::path::Path) {
    harness.input_mut().dropped_files = vec![egui::DroppedFile {
        path: Some(path.to_path_buf()),
        ..Default::default()
    }];
    harness.run();
}

#[test]
fn a_person_without_a_name_shows_as_no_name() -> TestResult {
    let desk = store(1, "Ada")?;
    let ben = store(2, "Ben")?;
    meet(&desk, &ben, "")?;
    let note = Notes::new(&desk).capture("Dentist on Friday", None, None)?;
    let mut app = App::new(desk, "en");
    app.select(Selection::All);
    app.select_note(&note.id);
    let mut harness = window(app);
    harness.get_by_label("Send…").click_accesskit();
    harness.run();
    assert!(shows(&harness, "No name, encrypted"));
    Ok(())
}

#[test]
fn a_note_is_saved_to_send_encrypted_to_a_person_met_or_signed_only() -> TestResult {
    let desk = store(1, "Ada")?;
    let ben = store(2, "Ben")?;
    meet(&desk, &ben, "Ben")?;
    let note = Notes::new(&desk).capture("Dentist on Friday", None, None)?;
    let dir = tempfile::tempdir()?;
    let saved = dir.path().join("for-ben");
    let mut app = App::new(desk, "en");
    app.select(Selection::All);
    app.select_note(&note.id);
    {
        let save = saved.clone();
        app.set_item_saver(Box::new(move |title, name| {
            assert_eq!(title, "Save the Note to send");
            assert!(name.starts_with("oldbrainfix-note-") && name.ends_with(".oldbrain"));
            Some(save.clone())
        }));
    }
    let mut harness = window(app);

    harness.get_by_label("Send…").click_accesskit();
    harness.run();
    assert!(shows(&harness, "Ben, encrypted"));
    assert!(!shows(&harness, "Not encrypted: whoever"));
    harness.get_by_label("Send").click_accesskit();
    harness.run();
    assert!(shows(&harness, "Saved. Send the file"));
    let file = saved.with_extension("oldbrain");
    let bytes = std::fs::read(&file)?;
    assert!(!String::from_utf8_lossy(&bytes).contains("Dentist"));
    let opened = ben.open_sent_item(&bytes)?.map_err(|_| "Ben opens it")?;
    assert!(opened.encrypted);
    assert_eq!(opened.item.text, "Dentist on Friday");

    // To anyone: signed only, and said so before.
    harness.get_by_label("Send…").click_accesskit();
    harness.run();
    harness
        .get_by_label("Anyone, not encrypted")
        .click_accesskit();
    harness.run();
    assert!(shows(&harness, "Not encrypted: whoever"));
    harness.get_by_label("Send").click_accesskit();
    harness.run();
    let bytes = std::fs::read(&file)?;
    assert!(String::from_utf8_lossy(&bytes).contains("Dentist"));
    let again = ben.open_sent_item(&bytes)?.map_err(|_| "signed")?;
    assert_eq!(again.item.id, opened.item.id, "the same Sent Item");
    assert_eq!(again.item.version, 2);
    Ok(())
}

#[test]
fn a_sent_item_dropped_or_handed_over_is_previewed_and_accepted_or_declined() -> TestResult {
    let ann = store(3, "Ann")?;
    let note = Notes::new(&ann).capture("Blood test", None, None)?;
    let schedule = Schedule::new(
        Day::new(2026, 11, 2),
        &[ClockTime::new(8, 15).ok_or("time")?],
        None,
    )
    .ok_or("schedule")?;
    Routines::new(&ann).spawn(&note.id, &schedule)?;
    let dir = tempfile::tempdir()?;
    let file = dir.path().join("from-ann.oldbrain");
    std::fs::write(&file, ann.send_note(&note.id, None, false)?)?;

    let desk = store(1, "Ada")?;
    let mut app = App::new(desk, "en");
    app.select(Selection::Agenda);
    let handed = Arc::new(Mutex::new(Vec::new()));
    app.set_handed_files(handed.clone());
    let mut harness = window(app);

    drop_file(&mut harness, &file);
    assert!(harness.state().previewing());
    assert!(shows(&harness, "From Ann"));
    // From a stranger: nothing of it before "Show".
    assert!(shows(&harness, "An unknown person: not among your People."));
    assert!(!shows(&harness, "Blood test"));
    assert!(!shows(&harness, &code(&ann)?));
    harness.get_by_label("Show").click_accesskit();
    harness.run();
    assert!(shows(
        &harness,
        &format!(
            "Not verified: this Profile is new here. Code {}",
            code(&ann)?
        )
    ));
    assert!(shows(&harness, "came unencrypted"));
    assert!(shows(&harness, "Blood test"));
    assert!(shows(&harness, "08:15"));
    harness.get_by_label("Decline").click_accesskit();
    harness.run();
    assert!(!harness.state().previewing());
    assert!(shows(&harness, "Declined: nothing was kept."));
    assert!(Notes::new(harness.state().store()).all()?.is_empty());
    assert!(harness.state().store().kept_people()?.is_empty());
    assert!(harness.state().store().pending_sets()?.is_empty());

    // Handed over by `oldbrainfix open`: shown at the next frame, and
    // accepted before "Show".
    if let Ok(mut held) = handed.lock() {
        held.push(file.clone());
    }
    harness.run();
    assert!(harness.state().previewing());
    assert!(!shows(&harness, "Blood test"));
    harness.get_by_label("Accept").click_accesskit();
    harness.run();
    assert!(shows(&harness, "Added to Unsorted, with its alarms on."));
    assert!(shows(&harness, "1 Unsorted"), "the window reads it at once");
    let held = harness.state().store();
    let got = Notes::new(held).unsorted()?;
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].text, "Blood test");
    let routines = Routines::new(held).of_note(&got[0].id)?;
    assert_eq!(routines.len(), 1);
    assert_eq!(routines[0].schedule, schedule);
    // Her Profile is kept, and her Note shows her picture.
    let ann_id = OwnProfile::new(&ann).get()?.ok_or("profile")?.id;
    assert_eq!(held.person_shown(&ann_id)?.name, "Ann");
    harness.state_mut().select(Selection::Unsorted);
    harness.run();
    assert!(shows(&harness, "Written by Ann"));

    assert!(harness.state().store().pending_sets()?.is_empty());

    // Opened again: accepted before, nothing new; Ann is known now.
    drop_file(&mut harness, &file);
    assert!(shows(&harness, "Already up to date"));
    assert!(harness.query_by_label("Show").is_none());
    harness.get_by_label("Close").click_accesskit();
    harness.run();
    assert_eq!(Notes::new(harness.state().store()).all()?.len(), 1);

    // For someone else, or damaged.
    let ben = store(2, "Ben")?;
    meet(&ann, &ben, "Ben")?;
    let to = ann.sent_item_recipients()?;
    let other = dir.path().join("for-ben.oldbrain");
    std::fs::write(&other, ann.send_note(&note.id, to.first(), false)?)?;
    drop_file(&mut harness, &other);
    // Announced without why; the click opens why and what to do.
    assert_eq!(harness.state().notice(), Some("The file was not taken."));
    assert!(!shows(&harness, "encrypted for someone else"));
    harness.get_by_label("The file was not taken.").click();
    harness.run();
    assert!(shows(&harness, "File not taken"));
    assert!(shows(&harness, "encrypted for someone else"));
    assert!(shows(&harness, "Ask the sender to send it again."));
    assert!(shows(&harness, "Report"));
    harness.get_by_label("Close").click_accesskit();
    harness.run();
    let damaged = dir.path().join("damaged.oldbrain");
    std::fs::write(&damaged, b"{\"format\":1}")?;
    drop_file(&mut harness, &damaged);
    assert_eq!(harness.state().notice(), Some("The file was not taken."));
    harness.get_by_label("The file was not taken.").click();
    harness.run();
    assert!(shows(&harness, "cannot be read"));
    Ok(())
}

#[test]
fn a_sent_item_of_a_later_format_says_an_update_is_needed() -> TestResult {
    let dir = tempfile::tempdir()?;
    let later = dir.path().join("later.oldbrain");
    std::fs::write(&later, br#"{"format":3,"item":"x","sig":"y"}"#)?;
    let mut app = App::new(store(1, "Ada")?, "en");
    app.select(Selection::Agenda);
    let mut harness = window(app);
    drop_file(&mut harness, &later);
    assert_eq!(harness.state().notice(), Some("The file was not taken."));
    harness.get_by_label("The file was not taken.").click();
    harness.run();
    assert!(shows(&harness, "File not taken"));
    assert!(shows(
        &harness,
        "This file comes from a newer Old Brain Fix."
    ));
    assert!(shows(
        &harness,
        "Update Old Brain Fix on this Device, then open it again."
    ));
    Ok(())
}

#[test]
fn a_note_sent_before_goes_as_an_update_or_a_cancellation() -> TestResult {
    let desk = store(1, "Ada")?;
    let ben = store(2, "Ben")?;
    let note = Notes::new(&desk).capture("Dentist", None, None)?;
    desk.send_note(&note.id, None, false)?;
    let dir = tempfile::tempdir()?;
    let saved = dir.path().join("update.oldbrain");
    let mut app = App::new(desk, "en");
    app.select(Selection::All);
    app.select_note(&note.id);
    {
        let save = saved.clone();
        app.set_item_saver(Box::new(move |_, _| Some(save.clone())));
    }
    let mut harness = window(app);

    harness.get_by_label("Send…").click_accesskit();
    harness.run();
    assert!(shows(&harness, "Send an update"));
    harness.get_by_label("Send").click_accesskit();
    harness.run();
    let update = ben
        .open_sent_item(&std::fs::read(&saved)?)?
        .map_err(|_| "update")?;
    assert_eq!(update.item.version, 2);
    assert!(!update.item.cancel);

    harness.get_by_label("Send…").click_accesskit();
    harness.run();
    harness
        .get_by_label("Send a cancellation instead")
        .click_accesskit();
    harness.run();
    harness.get_by_label("Send").click_accesskit();
    harness.run();
    let cancel = ben
        .open_sent_item(&std::fs::read(&saved)?)?
        .map_err(|_| "cancel")?;
    assert_eq!(cancel.item.version, 3);
    assert!(cancel.item.cancel);
    Ok(())
}

#[test]
fn an_update_shows_what_changes_and_a_cancellation_trashes_the_note() -> TestResult {
    let ann = store(3, "Ann")?;
    let note = Notes::new(&ann).capture("Dentist", None, None)?;
    let tuesday = Schedule::new(
        Day::new(2026, 11, 3),
        &[ClockTime::new(10, 0).ok_or("time")?],
        None,
    )
    .ok_or("schedule")?;
    let routine = Routines::new(&ann).spawn(&note.id, &tuesday)?;
    let desk = store(1, "Ada")?;
    let first = desk
        .open_sent_item(&ann.send_note(&note.id, None, false)?)?
        .map_err(|_| "first")?;
    let got = desk
        .accept_sent_item(&first.item, false)?
        .ok_or("accepted")?;
    Notes::new(&desk).amend(&got, "Ask about the bill")?;

    let wednesday = Schedule::new(
        Day::new(2026, 11, 4),
        &[ClockTime::new(14, 0).ok_or("time")?],
        None,
    )
    .ok_or("schedule")?;
    Routines::new(&ann).reschedule(&routine.id, &wednesday)?;
    Notes::new(&ann).edit(&note.id, "Dentist, room 3")?;
    let dir = tempfile::tempdir()?;
    let update = dir.path().join("update.oldbrain");
    std::fs::write(&update, ann.send_note(&note.id, None, false)?)?;

    let mut app = App::new(desk, "en");
    app.select(Selection::Agenda);
    let mut harness = window(app);
    drop_file(&mut harness, &update);
    assert!(harness.state().previewing());
    assert!(shows(&harness, "An update to a Note you accepted before."));
    assert!(shows(&harness, "Text so far: Dentist"));
    assert!(shows(&harness, "New text: Dentist, room 3"));
    let t = oldbrainfix_gui::l10n::L10n::new("en");
    let moved = format!(
        "{} → {}",
        schedule_text(&t, &tuesday),
        schedule_text(&t, &wednesday)
    );
    assert!(
        moved.contains("10:00") && moved.contains("14:00"),
        "{moved}"
    );
    assert!(shows(&harness, &moved));
    harness.get_by_label("Apply update").click_accesskit();
    harness.run();
    assert!(shows(&harness, "Updated. Your own additions stayed."));
    let held = harness.state().store();
    let now = Notes::new(held).by_id(&got)?.ok_or("note")?;
    assert_eq!(now.text, "Dentist, room 3");
    let sections = Notes::new(held).sections(&got)?;
    assert_eq!(sections.len(), 1, "his Amend stays");
    assert_eq!(Routines::new(held).of_note(&got)?[0].schedule, wednesday);

    // Another key's cancellation is refused.
    let eve = store(9, "Eve")?;
    let forged = oldbrainfix_core::sent::encode(
        &oldbrainfix_core::sent::SentItem {
            version: 9,
            cancel: true,
            from: eve.own_key_record()?,
            profile: eve
                .own_profile_for_others()?
                .as_ref()
                .and_then(SignedProfile::from_json)
                .ok_or("Eve's Profile")?,
            ..first.item.clone()
        },
        &eve.signing_key()?,
        &[],
        &mut |n| Ok(vec![0; n]),
    )?;
    let forged_file = dir.path().join("forged.oldbrain");
    std::fs::write(&forged_file, forged)?;
    drop_file(&mut harness, &forged_file);
    assert!(!harness.state().previewing());
    assert_eq!(harness.state().notice(), Some("The file was not taken."));
    harness.get_by_label("The file was not taken.").click();
    harness.run();
    assert!(shows(&harness, "it is not signed by them"));
    harness.get_by_label("Close").click_accesskit();
    harness.run();

    // Ann's own cancellation trashes it, its alarms with it.
    let cancel = dir.path().join("cancel.oldbrain");
    std::fs::write(&cancel, ann.send_note(&note.id, None, true)?)?;
    drop_file(&mut harness, &cancel);
    assert!(shows(&harness, "Cancelled: Dentist, room 3"));
    harness.get_by_label("Move to the trash").click_accesskit();
    harness.run();
    assert!(shows(&harness, "Moved to the trash"));
    let held = harness.state().store();
    assert!(Notes::new(held).by_id(&got)?.ok_or("note")?.is_trashed());
    assert!(Routines::new(held).all()?.is_empty());

    // Again: nothing more to do.
    drop_file(&mut harness, &cancel);
    assert!(shows(&harness, "Already up to date"));
    Ok(())
}

#[test]
fn comparing_the_short_code_marks_the_senders_profile_verified_on_accept() -> TestResult {
    let ann = store(3, "Ann")?;
    let picture = profile::square_picture(&halves(60, 40)?).ok_or("picture")?;
    OwnProfile::new(&ann).set_picture(Some(picture))?;
    let ann_id = OwnProfile::new(&ann).get()?.ok_or("profile")?.id;
    let dir = tempfile::tempdir()?;
    let first = dir.path().join("first.oldbrain");
    let dentist = Notes::new(&ann).capture("Dentist", None, None)?;
    std::fs::write(&first, ann.send_note(&dentist.id, None, false)?)?;
    let second = dir.path().join("second.oldbrain");
    let party = Notes::new(&ann).capture("Party", None, None)?;
    std::fs::write(&second, ann.send_note(&party.id, None, false)?)?;

    let mut app = App::new(store(1, "Ada")?, "en");
    app.select(Selection::Agenda);
    let mut harness = window(app);
    drop_file(&mut harness, &first);
    assert!(shows(&harness, "From Ann"));
    harness.get_by_label("Show").click_accesskit();
    harness.run();
    let hint = format!(
        "Ask Ann, in person, to read out the code on their Profile page. It must be {}.",
        code(&ann)?
    );
    assert!(!shows(&harness, &hint));

    // Codes that differ say so and verify nothing; declining after a
    // match keeps nothing either.
    harness.get_by_label("Compare codes").click_accesskit();
    harness.run();
    assert!(shows(&harness, &hint));
    harness.get_by_label("Codes differ").click_accesskit();
    harness.run();
    assert!(shows(&harness, "The codes differ"));
    harness.get_by_label("Compare codes").click_accesskit();
    harness.run();
    harness.get_by_label("Codes match").click_accesskit();
    harness.run();
    assert!(shows(&harness, "Verified: code compared in person."));
    assert!(shows(&harness, "Marked as verified when you accept."));
    harness.get_by_label("Decline").click_accesskit();
    harness.run();
    assert!(harness.state().store().kept_people()?.is_empty());

    // Matched and accepted: kept and verified, with her picture. "Show"
    // was not remembered: the item is hidden again.
    drop_file(&mut harness, &first);
    assert!(!shows(&harness, "Not verified: this Profile is new here."));
    harness.get_by_label("Show").click_accesskit();
    harness.run();
    assert!(shows(&harness, "Not verified: this Profile is new here."));
    harness.get_by_label("Compare codes").click_accesskit();
    harness.run();
    harness.get_by_label("Codes match").click_accesskit();
    harness.run();
    harness.get_by_label("Accept").click_accesskit();
    harness.run();
    let held = harness.state().store();
    assert!(held.person_verified(&ann_id)?);
    assert!(held.person_shown(&ann_id)?.picture.is_some());

    // The next one from her says so, with nothing left to compare.
    drop_file(&mut harness, &second);
    assert!(shows(&harness, "Verified: code compared in person."));
    assert!(harness.query_by_label("Compare codes").is_none());
    Ok(())
}

/// A Sent Item from Ann, a stranger here, with a Routine at 08:15,
/// saved in `dir`.
fn from_stranger(
    dir: &tempfile::TempDir,
) -> Result<(Store, std::path::PathBuf), Box<dyn std::error::Error>> {
    let ann = store(3, "Ann")?;
    let note = Notes::new(&ann).capture("Blood test", None, None)?;
    let schedule = Schedule::new(
        Day::new(2026, 11, 2),
        &[ClockTime::new(8, 15).ok_or("time")?],
        None,
    )
    .ok_or("schedule")?;
    Routines::new(&ann).spawn(&note.id, &schedule)?;
    let file = dir.path().join("from-ann.oldbrain");
    std::fs::write(&file, ann.send_note(&note.id, None, false)?)?;
    Ok((ann, file))
}

#[test]
fn a_strangers_sent_item_waits_hidden_behind_the_agenda_line() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (ann, file) = from_stranger(&dir)?;
    let mut app = App::new(store(1, "Ada")?, "en");
    app.select(Selection::Agenda);
    let mut harness = window(app);
    assert!(!shows(&harness, "waiting"));

    // Hidden: only who it is from, no picture, code or content.
    drop_file(&mut harness, &file);
    assert!(shows(&harness, "From Ann"));
    assert!(shows(&harness, "An unknown person: not among your People."));
    assert!(shows(
        &harness,
        "What they sent stays hidden until you tap Show."
    ));
    assert!(!shows(&harness, "Blood test"));
    assert!(!shows(&harness, "08:15"));
    assert!(!shows(&harness, &code(&ann)?));
    assert!(!shows(&harness, "Compare codes"));
    let store = harness.state().store();
    assert_eq!(store.pending_sets()?.len(), 1);
    assert!(Notes::new(store).all()?.is_empty());
    assert!(store.kept_people()?.is_empty());

    // Left undecided, it waits behind the Agenda's line.
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(!harness.state().previewing());
    assert_eq!(harness.state().store().pending_sets()?.len(), 1);
    harness
        .get_by_role_and_label(Role::Button, "1 Sent Item waiting")
        .click_accesskit();
    harness.run();
    assert_eq!(harness.state().selection(), &Selection::SentItemsWaiting);
    assert!(shows(&harness, "Sent Items waiting"));
    harness
        .get_by_role_and_label(Role::Button, "From Ann")
        .click_accesskit();
    harness.run();
    assert!(shows(&harness, "An unknown person"));
    assert!(!shows(&harness, "Blood test"));

    // "Show": today's preview, Compare codes included; still held.
    harness.get_by_label("Show").click_accesskit();
    harness.run();
    assert!(shows(&harness, "Blood test"));
    assert!(shows(&harness, "08:15"));
    assert!(shows(&harness, &code(&ann)?));
    assert!(shows(&harness, "Compare codes"));
    assert_eq!(harness.state().store().pending_sets()?.len(), 1);

    // Not remembered: opened again, it is hidden again.
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(!harness.state().previewing());
    harness
        .get_by_role_and_label(Role::Button, "From Ann")
        .click_accesskit();
    harness.run();
    assert!(harness.state().previewing());
    assert!(!shows(&harness, "Blood test"));

    // Decline before "Show": nothing kept, nothing waiting.
    harness.get_by_label("Decline").click_accesskit();
    harness.run();
    assert!(!harness.state().previewing());
    assert_eq!(
        harness.state().notice(),
        Some("Declined: nothing was kept.")
    );
    let store = harness.state().store();
    assert!(store.pending_sets()?.is_empty());
    assert!(Notes::new(store).all()?.is_empty());
    assert!(store.kept_people()?.is_empty());
    assert!(shows(&harness, "No Sent Items are waiting."));
    harness.key_press_modifiers(egui::Modifiers::ALT, egui::Key::ArrowLeft);
    harness.run();
    assert_eq!(harness.state().selection(), &Selection::Agenda);
    assert!(!shows(&harness, "waiting"));
    Ok(())
}

#[test]
fn a_strangers_sent_item_shown_is_accepted_from_where_it_waits() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (_ann, file) = from_stranger(&dir)?;
    let mut app = App::new(store(1, "Ada")?, "en");
    app.select(Selection::Agenda);
    let mut harness = window(app);
    drop_file(&mut harness, &file);
    // The same file again is the one held, not a second.
    harness.key_press(egui::Key::Escape);
    harness.run();
    drop_file(&mut harness, &file);
    assert_eq!(harness.state().store().pending_sets()?.len(), 1);
    harness.get_by_label("Show").click_accesskit();
    harness.run();
    harness.get_by_label("Accept").click_accesskit();
    harness.run();
    assert_eq!(
        harness.state().notice(),
        Some("Added to Unsorted, with its alarms on.")
    );
    let store = harness.state().store();
    assert!(store.pending_sets()?.is_empty());
    let got = Notes::new(store).unsorted()?;
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].text, "Blood test");
    assert_eq!(Routines::new(store).of_note(&got[0].id)?.len(), 1);
    assert!(!shows(&harness, "waiting"));
    Ok(())
}

#[test]
fn decline_and_block_before_show_blocks_the_stranger() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (ann, file) = from_stranger(&dir)?;
    let ann_id = OwnProfile::new(&ann).get()?.ok_or("profile")?.id;
    let mut app = App::new(store(1, "Ada")?, "en");
    app.select(Selection::Agenda);
    let mut harness = window(app);
    drop_file(&mut harness, &file);
    harness.get_by_label("Decline and block").click_accesskit();
    harness.run();
    assert!(shows(&harness, "Block Ann?"));
    // Cancel goes back to the hidden preview.
    harness.get_by_label("Cancel").click_accesskit();
    harness.run();
    assert!(shows(&harness, "An unknown person"));
    harness.get_by_label("Decline and block").click_accesskit();
    harness.run();
    harness.get_by_label("Block").click_accesskit();
    harness.run();
    assert!(!harness.state().previewing());
    assert_eq!(
        harness.state().notice(),
        Some("Declined and blocked: nothing was kept.")
    );
    let store = harness.state().store();
    assert!(store.is_blocked(&ann_id)?);
    assert!(store.pending_sets()?.is_empty());
    assert!(Notes::new(store).all()?.is_empty());

    // Another of hers shows nothing.
    drop_file(&mut harness, &file);
    assert!(!harness.state().previewing());
    assert_eq!(harness.state().notice(), Some("The file was not taken."));
    Ok(())
}

#[test]
fn decline_and_block_after_show_blocks_the_stranger() -> TestResult {
    let dir = tempfile::tempdir()?;
    let (ann, file) = from_stranger(&dir)?;
    let ann_id = OwnProfile::new(&ann).get()?.ok_or("profile")?.id;
    let mut app = App::new(store(1, "Ada")?, "en");
    app.select(Selection::Agenda);
    let mut harness = window(app);
    drop_file(&mut harness, &file);
    harness.get_by_label("Show").click_accesskit();
    harness.run();
    assert!(shows(&harness, "Blood test"));
    harness.get_by_label("Decline and block").click_accesskit();
    harness.run();
    harness.get_by_label("Block").click_accesskit();
    harness.run();
    assert!(!harness.state().previewing());
    assert_eq!(
        harness.state().notice(),
        Some("Declined and blocked: nothing was kept.")
    );
    let store = harness.state().store();
    assert!(store.is_blocked(&ann_id)?);
    assert!(store.pending_sets()?.is_empty());
    assert!(Notes::new(store).all()?.is_empty());
    Ok(())
}

/// Speakers that only count what they were asked to play.
struct Speakers(Rc<RefCell<u32>>);

impl Player for Speakers {
    fn play(&mut self, _bytes: Vec<u8>) -> bool {
        *self.0.borrow_mut() += 1;
        true
    }

    fn stop(&mut self) {}

    fn playing(&self) -> bool {
        false
    }

    fn set_speed(&mut self, _speed: f64) {}
}

/// The names of the textures made from a photo or from `profile`'s
/// picture.
fn pictures_drawn(harness: &Harness<'_, App>, profile: &str) -> Vec<String> {
    let profile = format!("profile:{profile}:");
    harness
        .ctx
        .tex_manager()
        .read()
        .allocated()
        .map(|(_, meta)| meta.name.clone())
        .filter(|name| name.starts_with("photo") || name.starts_with(&profile))
        .collect()
}

#[test]
fn declining_a_strangers_sent_item_before_show_decodes_no_photo_or_recording() -> TestResult {
    let png = halves(60, 40)?;
    let photo = |notes: &Notes<'_>, id: &str| notes.add_photo(id, &png);
    let recording = |notes: &Notes<'_>, id: &str| notes.add_recording(id, &[1, 2, 3]);
    type Media<'a> = &'a dyn Fn(&Notes<'_>, &str) -> oldbrainfix_core::Result<String>;
    let kinds: [(Media, bool); 2] = [(&photo, true), (&recording, false)];
    for (media, is_photo) in kinds {
        // Ann has a picture; her Note holds a photo or a recording on
        // her side.
        let ann = store(3, "Ann")?;
        let picture = profile::square_picture(&halves(60, 40)?).ok_or("picture")?;
        OwnProfile::new(&ann).set_picture(Some(picture))?;
        let ann_id = OwnProfile::new(&ann).get()?.ok_or("profile")?.id;
        let note = Notes::new(&ann).capture("Dentist on Friday", None, None)?;
        let hash = media(&Notes::new(&ann), &note.id)?;
        let dir = tempfile::tempdir()?;
        let file = dir.path().join("from-ann.oldbrain");
        std::fs::write(&file, ann.send_note(&note.id, None, false)?)?;

        let played = Rc::new(RefCell::new(0));
        let mut app = App::new(store(1, "Ada")?, "en");
        app.set_player(Box::new(Speakers(played.clone())));
        app.select(Selection::Agenda);
        let mut harness = window(app);
        let images = |h: &Harness<'_, App>| h.query_all_by_role(Role::Image).count();
        let before = images(&harness);
        drop_file(&mut harness, &file);
        harness.run_steps(3);
        assert!(shows(&harness, "An unknown person"));
        // Nothing of hers drawn: no picture, no photo, no player.
        assert!(pictures_drawn(&harness, &ann_id).is_empty());
        assert_eq!(images(&harness), before);
        assert!(!shows(&harness, "Play the voice recording"));

        harness.get_by_label("Decline").click_accesskit();
        harness.run_steps(3);
        assert_eq!(
            harness.state().notice(),
            Some("Declined: nothing was kept.")
        );
        assert!(pictures_drawn(&harness, &ann_id).is_empty());
        assert_eq!(*played.borrow(), 0);
        let held = harness.state().store();
        if is_photo {
            assert!(held.photo(&hash).is_none());
        } else {
            assert!(held.audio(&hash).is_none());
        }
        assert!(Notes::new(held).all()?.is_empty());
        assert!(held.kept_people()?.is_empty());
        assert!(held.pending_sets()?.is_empty());
        assert!(held.held_sent_items()?.is_empty());

        // "Show" would have drawn her picture: what was checked above
        // can be seen.
        drop_file(&mut harness, &file);
        harness.get_by_label("Show").click_accesskit();
        harness.run_steps(3);
        assert!(!pictures_drawn(&harness, &ann_id).is_empty());
    }
    Ok(())
}
