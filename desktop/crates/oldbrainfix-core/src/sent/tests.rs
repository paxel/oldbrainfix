use super::*;
use crate::OwnProfile;
use crate::pairing::PairedDevice;
use crate::store::tests::ticking;
use crate::{ClockTime, Day, Recurrence};

/// A Device of its own person, with the Profile the app makes at its
/// first start.
fn device(seed: u8) -> Store {
    let name = ["Ann", "Ben", "Eve", "Cid", "Dan", "Fay"][usize::from(seed - 1) % 6];
    match Store::in_memory(ticking(), move || Ok(SigningKey::from_seed([seed; 32])))
        .and_then(|s| OwnProfile::new(&s).apply_defaults(name, None).map(|_| s))
    {
        Ok(s) => s,
        Err(e) => panic!("store: {e}"),
    }
}

/// The own Profile of `store` as a Sent Item carries it.
fn profile_of(store: &Store) -> std::result::Result<SignedProfile, Box<dyn std::error::Error>> {
    Ok(store
        .own_profile_for_others()?
        .as_ref()
        .and_then(SignedProfile::from_json)
        .ok_or("profile")?)
}

/// The `person:` settings of `store`.
fn people(store: &Store) -> Result<Vec<(String, String)>> {
    store.local_settings_with_prefix("person:")
}

fn own_id(store: &Store) -> std::result::Result<String, Box<dyn std::error::Error>> {
    Ok(OwnProfile::new(store).get()?.ok_or("profile")?.id)
}

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

fn invite(store: &Store, other: &Store, name: &str) -> TestResult {
    store.remember_device(&PairedDevice {
        id: other.device_id()?,
        name: name.into(),
        fingerprint: String::new(),
        host: None,
        port: None,
        last_sync: parse_time("2026-01-01T00:00:00.000Z")?,
        shared: true,
    })?;
    let mut report = crate::ImportReport::default();
    let id = other.device_id()?;
    store.learn_keys(&[other.own_key_record()?], Some(&id), &mut report)?;
    Ok(())
}

#[test]
fn a_note_goes_out_signed_under_one_id_and_lands_in_unsorted() -> TestResult {
    let ann = device(1);
    let ben = device(2);
    let note = Notes::new(&ann).capture("Dentist", None, None)?;
    Notes::new(&ann).amend(&note.id, "Room 3")?;
    let schedule = Schedule::new(
        Day::parse("2026-03-02").ok_or("day")?,
        &[ClockTime::parse("08:00").ok_or("time")?],
        Some(Recurrence::parse("FREQ=DAILY").map_err(|_| "rule")?),
    )
    .ok_or("schedule")?;
    let sent = Routines::new(&ann).spawn(&note.id, &schedule)?;

    let file = ann.send_note(&note.id, None, false)?;
    assert!(looks_like_sent_item(&file));
    let opened = ben.open_sent_item(&file)?.map_err(|_| "opened")?;
    assert!(!opened.encrypted);
    assert_eq!(opened.trust, SenderTrust::Unknown);
    assert_eq!(opened.sender.name, "Ann");
    assert_eq!(Some(opened.code.clone()), OwnProfile::new(&ann).code()?);
    assert_eq!(opened.item.version, 1);
    let again = ben
        .open_sent_item(&ann.send_note(&note.id, None, false)?)?
        .map_err(|_| "again")?;
    assert_eq!(again.item.id, opened.item.id);
    assert_eq!(again.item.version, 2);

    let id = ben
        .accept_sent_item(&opened.item, false)?
        .ok_or("accepted")?;
    let got = Notes::new(&ben).by_id(&id)?.ok_or("note")?;
    assert!(got.is_unsorted());
    assert_eq!(got.text, "Dentist");
    let routines = Routines::new(&ben).of_note(&id)?;
    assert_eq!(routines.len(), 1);
    assert_eq!(routines[0].schedule, schedule);
    assert_eq!(
        ben.current(&routines[0].id, keys::RECEIVED)?.as_deref(),
        Some(sent.id.as_str())
    );
    assert_eq!(
        ben.open_sent_item(&file)?.map(|o| o.state),
        Ok(SentItemState::UpToDate)
    );
    // Sent again unchanged: only the version moves on.
    let resent = ben
        .open_sent_item(&ann.send_note(&note.id, None, false)?)?
        .map_err(|_| "resent")?;
    assert_eq!(resent.state, SentItemState::Update);
    assert!(
        resent
            .changes
            .as_ref()
            .is_some_and(SentItemChanges::is_empty)
    );
    assert_eq!(
        ben.accept_sent_item(&resent.item, false)?.as_deref(),
        Some(id.as_str())
    );

    let mut changed = String::from_utf8(file)?;
    changed = changed.replacen("Dentist", "Dentlst", 1);
    assert_eq!(
        ben.open_sent_item(changed.as_bytes())?.err(),
        Some(OpenError::NotASentItem)
    );
    assert_eq!(
        ben.open_sent_item(b"PK\x03\x04")?.err(),
        Some(OpenError::NotASentItem)
    );
    Ok(())
}

#[test]
fn a_session_goes_as_a_plain_note_its_routines_with_it() -> TestResult {
    use crate::note_kinds::NoteKind;
    use crate::sessions::{SessionRule, Sessions};
    let ann = device(1);
    let ben = device(2);
    let push = Notes::new(&ann).capture("Push-ups", None, None)?.id;
    Tallies::new(&ann).add_field(&push, "Times", "times", None, None)?;
    let note = Notes::new(&ann).capture("Sport", None, None)?.id;
    Sessions::new(&ann).make(&note, std::slice::from_ref(&push), 1, SessionRule::Rolling)?;
    for at in [7, 18] {
        let schedule = Schedule::new(
            Day::parse("2026-03-06").ok_or("day")?,
            &[ClockTime::new(at, 0).ok_or("time")?],
            None,
        )
        .ok_or("schedule")?;
        Routines::new(&ann).spawn(&note, &schedule)?;
    }
    let opened = ben
        .open_sent_item(&ann.send_note(&note, None, false)?)?
        .map_err(|_| "opened")?;
    let id = ben
        .accept_sent_item(&opened.item, false)?
        .ok_or("accepted")?;
    // The pool names Tallies Ben does not hold (ADR 0028, 0039).
    assert_eq!(NoteKind::of(&ben, &id)?, NoteKind::Plain);
    assert_eq!(Routines::new(&ben).of_note(&id)?.len(), 2);
    assert!(Sessions::new(&ben).all()?.is_empty());
    Ok(())
}

#[test]
fn a_formatted_text_goes_out_and_arrives_editable() -> TestResult {
    let ann = device(1);
    let ben = device(2);
    let notes = Notes::new(&ann);
    let note = notes.capture_as("**Milk**", text_format::MARKDOWN, None, None)?;
    notes.amend_as(&note.id, "- oat", text_format::MARKDOWN)?;
    let opened = ben
        .open_sent_item(&ann.send_note(&note.id, None, false)?)?
        .map_err(|_| "opened")?;
    let payload = opened.item.encode();
    assert!(payload.contains(r#""note":{"format":"markdown","sections":["#));
    assert_eq!(opened.item.sections[0].format, text_format::MARKDOWN);
    let got = ben
        .accept_sent_item(&opened.item, false)?
        .ok_or("accepted")?;
    let received = Notes::new(&ben).by_id(&got)?.ok_or("note")?;
    assert_eq!(received.text, "**Milk**");
    assert_eq!(received.text_format, text_format::MARKDOWN);
    assert!(received.text_editable);
    let sections = Notes::new(&ben).sections(&got)?;
    assert_eq!(sections[0].text_format, text_format::MARKDOWN);
    Ok(())
}

#[test]
fn every_text_travels_in_its_format_and_one_made_here_is_never_overwritten() -> TestResult {
    const FUTURE: &str = r##"{"format":"x-future","text":"# Heading"}"##;
    let ann = device(1);
    let ben = device(2);
    let note = Notes::new(&ann).capture("x", None, None)?;
    ann.append(&note.id, note_keys::TEXT, Some(FUTURE), None)?;
    Notes::new(&ann).amend(&note.id, "Room 3")?;
    let opened = ben
        .open_sent_item(&ann.send_note(&note.id, None, false)?)?
        .map_err(|_| "opened")?;
    assert_eq!(opened.item.text, "# Heading");
    assert_eq!(opened.item.text_format, "x-future");
    assert_eq!(opened.item.sections[0].format, text_format::PLAIN);
    let payload = opened.item.encode();
    assert!(payload.contains(r#""note":{"format":"x-future","sections":["#));
    assert!(payload.contains(r#","format":"plain","text":"Room 3"}"#));
    // Version 1, the default, is not written.
    assert!(!payload.contains(r#""v":"#));
    let got = ben
        .accept_sent_item(&opened.item, false)?
        .ok_or("accepted")?;
    assert_eq!(ben.current(&got, note_keys::TEXT)?.as_deref(), Some(FUTURE));
    // Without formats, a file reads as plain, version 1.
    let bare = SentItem::parse(&payload.replace(r#""format":"x-future","#, "")).ok_or("bare")?;
    assert_eq!(bare.text_format, text_format::PLAIN);
    assert_eq!(bare.text_version, 1);
    assert_eq!(bare.sections[0].version, 1);
    assert!(SentItem::parse(&payload.replace(r#""format":"plain""#, r#""format":1"#)).is_none());
    let section = |v: &str| {
        payload.replacen(
            r#""text":"Room 3"}"#,
            &format!(r#""text":"Room 3","v":{v}}}"#),
            1,
        )
    };
    let in_note = |v: &str| {
        payload.replacen(
            r##""text":"# Heading"}"##,
            &format!(r##""text":"# Heading","v":{v}}}"##),
            1,
        )
    };
    // Version 1 said all the same reads as the default.
    let said = SentItem::parse(&section("1")).ok_or("section 1")?;
    assert_eq!(said.sections[0].version, 1);
    assert_eq!(
        SentItem::parse(&in_note("1")).ok_or("note 1")?.text_version,
        1
    );
    assert_eq!(
        SentItem::parse(&in_note("2")).ok_or("note 2")?.text_version,
        2
    );
    // A version that is no whole number from 1 makes it unreadable.
    for v in [r#""1""#, "1.5", "0", "null"] {
        assert!(SentItem::parse(&section(v)).is_none(), "section {v}");
        assert!(SentItem::parse(&in_note(v)).is_none(), "note {v}");
    }

    // The sender's own formatted text follows her update.
    let moved = r##"{"format":"x-future","text":"# Moved"}"##;
    ann.append(&note.id, note_keys::TEXT, Some(moved), None)?;
    let update = ben
        .open_sent_item(&ann.send_note(&note.id, None, false)?)?
        .map_err(|_| "update")?;
    let changes = update.changes.clone().ok_or("changes")?;
    assert_eq!(changes.text_before.as_deref(), Some("# Heading"));
    ben.accept_sent_item(&update.item, false)?;
    assert_eq!(ben.current(&got, note_keys::TEXT)?.as_deref(), Some(moved));

    // One a later version made here stays.
    ben.append(&got, note_keys::TEXT, Some(FUTURE), None)?;
    ann.append(
        &note.id,
        note_keys::TEXT,
        Some(r#"{"format":"plain","text":"Plain again"}"#),
        None,
    )?;
    let next = ben
        .open_sent_item(&ann.send_note(&note.id, None, false)?)?
        .map_err(|_| "next")?;
    assert_eq!(next.changes.clone().ok_or("changes")?.text_before, None);
    ben.accept_sent_item(&next.item, false)?;
    assert_eq!(ben.current(&got, note_keys::TEXT)?.as_deref(), Some(FUTURE));
    Ok(())
}

#[test]
fn a_text_in_a_later_structure_goes_out_and_arrives_in_it() -> TestResult {
    const LATER: &str = r#"{"format":"plain","more":1,"text":"Later milk","v":2}"#;
    let ann = device(1);
    let ben = device(2);
    let note = Notes::new(&ann).capture("x", None, None)?;
    ann.append(&note.id, note_keys::TEXT, Some(LATER), None)?;
    let section = Notes::new(&ann).amend(&note.id, "Room 3")?;
    ann.append(&section.id, note_keys::TEXT, Some(LATER), None)?;
    let opened = ben
        .open_sent_item(&ann.send_note(&note.id, None, false)?)?
        .map_err(|_| "opened")?;
    assert_eq!(opened.item.text_version, 2);
    assert_eq!(opened.item.sections[0].version, 2);
    let got = ben
        .accept_sent_item(&opened.item, false)?
        .ok_or("accepted")?;
    let arrived = r#"{"format":"plain","text":"Later milk","v":2}"#;
    assert_eq!(
        ben.current(&got, note_keys::TEXT)?.as_deref(),
        Some(arrived)
    );
    let notes = Notes::new(&ben);
    assert!(!notes.by_id(&got)?.ok_or("note")?.text_editable);
    let there = notes.sections(&got)?.remove(0);
    assert_eq!(
        ben.current(&there.id, note_keys::TEXT)?.as_deref(),
        Some(arrived)
    );
    assert!(!there.text_editable);
    Ok(())
}

#[test]
fn an_update_keeps_the_receivers_text_and_brings_a_trashed_note_back() -> TestResult {
    let ann = device(1);
    let ben = device(2);
    let notes = Notes::new(&ann);
    let note = notes.capture("Dentist", None, None)?;
    let schedule = Schedule::new(
        Day::parse("2026-03-03").ok_or("day")?,
        &[ClockTime::parse("10:00").ok_or("time")?],
        None,
    )
    .ok_or("schedule")?;
    Routines::new(&ann).spawn(&note.id, &schedule)?;
    let first = ben
        .open_sent_item(&ann.send_note(&note.id, None, false)?)?
        .map_err(|_| "first")?;
    let got = ben
        .accept_sent_item(&first.item, false)?
        .ok_or("accepted")?;
    assert!(ann.was_sent(&note.id)?);

    // His own words stay while hers do not change.
    Notes::new(&ben).edit(&got, "Dentist (mine)")?;
    let same = ben
        .open_sent_item(&ann.send_note(&note.id, None, false)?)?
        .map_err(|_| "same")?;
    let changes = same.changes.clone().ok_or("changes")?;
    assert_eq!(changes.text_before, None);
    ben.accept_sent_item(&same.item, false)?;
    assert_eq!(
        Notes::new(&ben).by_id(&got)?.ok_or("note")?.text,
        "Dentist (mine)"
    );

    // In his trash: the cancellation finds nothing, an update brings
    // it back, its Reminder with it.
    Routines::new(&ben).trash_note(&got)?;
    let cancel = ben
        .open_sent_item(&ann.send_note(&note.id, None, true)?)?
        .map_err(|_| "cancel")?;
    assert_eq!(cancel.state, SentItemState::NothingToCancel);
    assert_eq!(ben.accept_sent_item(&cancel.item, false)?, None);
    notes.edit(&note.id, "Dentist moved")?;
    let update = ben
        .open_sent_item(&ann.send_note(&note.id, None, false)?)?
        .map_err(|_| "update")?;
    let changes = update.changes.clone().ok_or("changes")?;
    assert!(changes.restores);
    assert!(changes.text_edited);
    assert_eq!(changes.text_before.as_deref(), Some("Dentist (mine)"));
    ben.accept_sent_item(&update.item, false)?;
    let back = Notes::new(&ben).by_id(&got)?.ok_or("note")?;
    assert!(!back.is_trashed());
    assert_eq!(back.text, "Dentist moved");
    assert_eq!(Routines::new(&ben).of_note(&got)?.len(), 1);

    // Cancelled for real: in the trash, alarms off.
    let cancel = ben
        .open_sent_item(&ann.send_note(&note.id, None, true)?)?
        .map_err(|_| "cancel")?;
    assert_eq!(cancel.state, SentItemState::Cancel);
    ben.accept_sent_item(&cancel.item, false)?;
    assert!(Notes::new(&ben).by_id(&got)?.ok_or("note")?.is_trashed());
    assert!(Routines::new(&ben).all()?.is_empty());
    Ok(())
}

#[test]
fn a_routine_the_receivers_later_version_rescheduled_is_neither_changed_nor_added_again()
-> TestResult {
    let ann = device(1);
    let ben = device(2);
    let note = Notes::new(&ann).capture("Dentist", None, None)?;
    let day = |d: &str| Day::parse(d).ok_or("day");
    let time = |t: &str| ClockTime::parse(t).ok_or("time");
    let reminder = Routines::new(&ann).spawn(
        &note.id,
        &Schedule::new(day("2026-03-03")?, &[time("10:00")?], None).ok_or("schedule")?,
    )?;
    let daily = Routines::new(&ann).spawn(
        &note.id,
        &Schedule::new(
            day("2026-03-01")?,
            &[time("08:00")?],
            Some(Recurrence::parse("FREQ=DAILY")?),
        )
        .ok_or("schedule")?,
    )?;
    let first = ben
        .open_sent_item(&ann.send_note(&note.id, None, false)?)?
        .map_err(|_| "first")?;
    let got = ben
        .accept_sent_item(&first.item, false)?
        .ok_or("accepted")?;
    let held = Routines::new(&ben).of_note(&got)?;
    let theirs = held
        .iter()
        .find(|r| !r.is_reminder())
        .map(|r| r.id.clone())
        .ok_or("daily")?;
    // As Ben's later version writes it.
    ben.append(&theirs, routine_keys::SCHEDULE_VERSION, Some("2"), None)?;
    Routines::new(&ann).reschedule(
        &daily.id,
        &Schedule::new(day("2026-03-01")?, &[time("09:00")?], None).ok_or("schedule")?,
    )?;
    Routines::new(&ann).reschedule(
        &reminder.id,
        &Schedule::new(day("2026-03-04")?, &[time("14:00")?], None).ok_or("schedule")?,
    )?;
    let update = ben
        .open_sent_item(&ann.send_note(&note.id, None, false)?)?
        .map_err(|_| "update")?;
    let changes = update.changes.clone().ok_or("changes")?;
    assert!(changes.routines_added.is_empty());
    assert!(changes.routines_removed.is_empty());
    let known = Routines::new(&ben).of_note(&got)?;
    assert_eq!(
        changes
            .rescheduled
            .iter()
            .map(|r| r.routine.clone())
            .collect::<Vec<_>>(),
        known.iter().map(|r| r.id.clone()).collect::<Vec<_>>()
    );
    let before = ben.history(&theirs)?.len();
    ben.accept_sent_item(&update.item, false)?;
    assert_eq!(ben.history(&theirs)?.len(), before);
    let newer: Vec<String> = Routines::new(&ben)
        .newer_of_note(&got)?
        .into_iter()
        .map(|r| r.id)
        .collect();
    assert_eq!(newer, vec![theirs]);
    assert_eq!(Routines::new(&ben).of_note(&got)?.len(), 1);
    Ok(())
}

/// `signed`, a signed Sent Item, encrypted to the X25519 key `to` as
/// [`encode_sent_item`] does, with fixed bytes.
fn sealed_to(signed: &[u8], to: &[u8; BOX_KEY_BYTES]) -> Result<Vec<u8>> {
    let content = [7u8; AREA_KEY_BYTES];
    let ephemeral = [9u8; BOX_KEY_BYTES];
    let public = box_public_of(&ephemeral);
    let shared = x25519(&ephemeral, to).ok_or(Error::Invalid("no key".into()))?;
    let nonce = [1u8; SEAL_NONCE_BYTES];
    let sealed = seal(&content, signed, SEALED_AAD, &nonce)?;
    let wrapped = seal(
        &wrap_key(&shared, &public, to),
        &content,
        SENT_KEY_AAD,
        &nonce,
    )?;
    Ok(json!({
        "sealed": BASE64.encode(sealed),
        "to": [{"eph": BASE64.encode(public), "key": BASE64.encode(wrapped)}],
    })
    .to_string()
    .into_bytes())
}

#[test]
fn this_device_remembers_whom_it_last_sent_a_note_to() -> TestResult {
    let ann = device(1);
    let ben = device(2);
    invite(&ann, &ben, "Ben")?;
    let to = ann.sent_item_recipients()?;
    let to = to.first().ok_or("a recipient")?;
    let note = Notes::new(&ann).capture("Secret plan", None, None)?;
    let ben_id = ben.device_id()?;
    assert_eq!(ann.last_sent_to(&note.id)?, None);
    ann.send_note(&note.id, Some(to), false)?;
    // A local setting, the same key and value as the Dart core's.
    let key = format!("sentTo:{}", note.id);
    assert_eq!(ann.local_setting(&key)?, Some(ben_id.clone()));
    assert_eq!(ann.last_sent_to(&note.id)?, Some(ben_id.clone()));
    // The log does not say whom it went to.
    let link = ann
        .current(&note.id, &format!("sent:{}", ann.device_id()?))?
        .ok_or("link")?;
    let link: Value = serde_json::from_str(&link)?;
    let fields: Vec<&String> = link.as_object().ok_or("object")?.keys().collect();
    assert_eq!(fields, ["id", "version"]);
    assert!(
        ann.all_entries()?
            .iter()
            .all(|e| !e.value.as_deref().unwrap_or("").contains(&ben_id))
    );
    // A cancellation to the person keeps them.
    ann.send_note(&note.id, Some(to), true)?;
    assert_eq!(ann.last_sent_to(&note.id)?, Some(ben_id.clone()));
    // To anyone: nobody.
    ann.send_note(&note.id, None, false)?;
    assert_eq!(ann.local_setting(&key)?, None);
    assert_eq!(ann.last_sent_to(&note.id)?, None);
    // A Note this Device did not send before starts at nobody.
    let other = Notes::new(&ann).capture("Other", None, None)?;
    ann.set_local_setting(&format!("sentTo:{}", other.id), &ben_id)?;
    assert_eq!(ann.last_sent_to(&other.id)?, None);
    Ok(())
}

#[test]
fn one_of_a_later_format_needs_an_update_and_says_so() -> TestResult {
    let ann = device(1);
    let ben = device(2);
    let note = Notes::new(&ann).capture("Buy milk", None, None)?;
    let signed = String::from_utf8(ann.send_note(&note.id, None, false)?)?;
    assert!(signed.starts_with(r#"{"item":"#));
    for format in ["3", r#""1""#, "1.0", "null", "0"] {
        let later = format!(r#"{{"format":{format},{}"#, &signed[1..]);
        assert_eq!(
            ben.open_sent_item(later.as_bytes())?.err(),
            Some(OpenError::NeedsUpdate),
            "{format}"
        );
    }
    // Whatever else a later format holds.
    assert_eq!(
        ben.open_sent_item(br#"{"format":3,"parts":[]}"#)?.err(),
        Some(OpenError::NeedsUpdate)
    );
    // Encrypted, the signed file inside says it.
    invite(&ann, &ben, "Ben")?;
    let to = ann.sent_item_recipients()?;
    let key = to.first().and_then(|r| r.keys.first()).ok_or("a key")?;
    let inner = format!(r#"{{"format":3,{}"#, &signed[1..]);
    assert_eq!(
        ben.open_sent_item(&sealed_to(inner.as_bytes(), key)?)?
            .err(),
        Some(OpenError::NeedsUpdate)
    );
    // Of the formats known, it opens as before.
    let opened = ben.open_sent_item(&sealed_to(signed.as_bytes(), key)?)?;
    assert!(opened.map_err(|_| "opened")?.encrypted);
    let one = format!(r#"{{"format":1,{}"#, &signed[1..]);
    assert!(ben.open_sent_item(one.as_bytes())?.is_ok());
    Ok(())
}

#[test]
fn a_person_met_gets_it_encrypted_and_nobody_else_reads_it() -> TestResult {
    let ann = device(1);
    let ben = device(2);
    let eve = device(3);
    invite(&ann, &ben, "Ben")?;
    let to = ann.sent_item_recipients()?;
    assert_eq!(to.len(), 1);
    assert_eq!(to[0].name, "Ben");
    let note = Notes::new(&ann).capture("Secret plan", None, None)?;
    let file = ann.send_note(&note.id, Some(&to[0]), false)?;
    assert!(!String::from_utf8_lossy(&file).contains("Secret"));
    let opened = ben.open_sent_item(&file)?.map_err(|_| "opened")?;
    assert!(opened.encrypted);
    assert_eq!(opened.item.text, "Secret plan");
    assert_eq!(eve.open_sent_item(&file)?.err(), Some(OpenError::NotForUs));

    // A key pinned without its X25519 key is not offered, until a
    // record with it comes along.
    let cid = device(4);
    let record = cid.own_key_record()?.bare();
    let mut report = crate::ImportReport::default();
    let other = device(5);
    other.learn_keys(&[record], None, &mut report)?;
    invite(&other, &cid, "Cid")?;
    assert_eq!(other.sent_item_recipients()?.len(), 1, "taken on learning");
    Ok(())
}
#[test]
fn the_preview_shows_the_senders_profile_and_accepting_keeps_it() -> TestResult {
    let ann = device(1);
    let ben = device(2);
    let cid = device(4);
    let ann_id = own_id(&ann)?;
    let note = Notes::new(&ann).capture("Dentist", None, None)?;
    let file = ann.send_note(&note.id, None, false)?;

    let opened = ben.open_sent_item(&file)?.map_err(|_| "opened")?;
    assert_eq!(opened.sender.id, ann_id);
    assert_eq!(opened.sender.name, "Ann");
    assert_eq!(opened.trust, SenderTrust::Unknown);
    // Declining is not accepting: nothing is kept.
    assert!(people(&ben)?.is_empty());

    let got = ben
        .accept_sent_item(&opened.item, false)?
        .ok_or("accepted")?;
    assert_eq!(ben.person_shown(&ann_id)?.name, "Ann");
    assert_eq!(ben.person_code(&ann_id)?, OwnProfile::new(&ann).code()?);
    assert!(!ben.person_verified(&ann_id)?);
    // The accepted Note shows its sender, as other people's Notes do.
    assert_eq!(ben.author_of(&got)?.map(|p| p.id), Some(ann_id.clone()));
    let mine = Notes::new(&ben).capture("Mine", None, None)?;
    assert_eq!(ben.author_of(&mine.id)?, None);
    // Kept now: the next one is known, and opening changes nothing.
    let kept = people(&ben)?;
    let later = Notes::new(&ann).capture("Later", None, None)?;
    let again = ben
        .open_sent_item(&ann.send_note(&later.id, None, false)?)?
        .map_err(|_| "again")?;
    assert_eq!(again.trust, SenderTrust::Known);
    assert_eq!(people(&ben)?, kept);
    assert_eq!(
        cid.open_sent_item(&file)?.map(|o| o.trust),
        Ok(SenderTrust::Unknown)
    );
    // Sent to oneself: one's own Profile, verified, never kept.
    let own = ann.open_sent_item(&file)?.map_err(|_| "own")?;
    assert_eq!(own.trust, SenderTrust::Verified);
    ann.accept_sent_item(&own.item, false)?;
    assert!(people(&ann)?.is_empty());
    Ok(())
}

#[test]
fn a_profile_not_bound_to_the_key_that_signed_is_refused() -> TestResult {
    let ann = device(1);
    let ben = device(2);
    let eve = device(3);
    let item = SentItem {
        id: "x".into(),
        version: 1,
        cancel: false,
        from: eve.own_key_record()?,
        profile: profile_of(&ann)?,
        text: "Hello from Ann".into(),
        text_format: text_format::PLAIN.into(),
        text_version: text_format::VERSION,
        sections: Vec::new(),
        routines: Vec::new(),
        lines: None,
        shopping: false,
    };
    let file = encode(&item, &eve.signing_key()?, &[], &mut |n| Ok(vec![0; n]))?;
    assert_eq!(
        ben.open_sent_item(&file)?.err(),
        Some(OpenError::NotASentItem)
    );
    let mut json: Value = serde_json::from_str(&item.encode())?;
    if let Some(from) = json.get_mut("from").and_then(Value::as_object_mut) {
        from.remove("profile");
    }
    assert_eq!(SentItem::parse(&json.to_string()), None);
    assert!(people(&ben)?.is_empty());
    Ok(())
}

#[test]
fn comparing_the_short_code_verifies_the_profile_on_accept_only() -> TestResult {
    let ann = device(1);
    let ben = device(2);
    let ann_id = own_id(&ann)?;
    let note = Notes::new(&ann).capture("Dentist", None, None)?;
    let first = ben
        .open_sent_item(&ann.send_note(&note.id, None, false)?)?
        .map_err(|_| "first")?;
    assert!(!ben.verify_person(&ann_id, &first.code)?, "not kept");
    ben.accept_sent_item(&first.item, true)?;
    assert!(ben.person_verified(&ann_id)?);
    let update = ben
        .open_sent_item(&ann.send_note(&note.id, None, false)?)?
        .map_err(|_| "update")?;
    assert_eq!(update.trust, SenderTrust::Verified);

    // Eve copies Ann's id under a first key of her own: another code,
    // and nothing of Ann's changes, verified or not.
    let eve = device(3);
    let eve_key = BASE64.encode(eve.signing_key()?.public_key());
    let record = Profile {
        id: ann_id.clone(),
        name: "Ann".into(),
        picture: None,
        version: 9,
    }
    .encode();
    let copy = SignedProfile {
        sig: eve
            .signing_key()?
            .sign(&crate::profile::record_bytes(&record)),
        record,
        signer: eve_key.clone(),
        first: eve_key,
        keys: Vec::new(),
        aliases: Vec::new(),
        fresh: false,
    };
    let dan = device(5);
    let hi = Notes::new(&dan).capture("Hi", None, None)?;
    let from_dan = ben
        .open_sent_item(&dan.send_note(&hi.id, None, false)?)?
        .map_err(|_| "dan")?;
    ben.accept_sent_item(&from_dan.item, false)?;
    assert!(
        !ben.verify_person(&own_id(&dan)?, &first.code)?,
        "wrong code"
    );
    let kept = people(&ben)?;
    let item = SentItem {
        id: "fake".into(),
        version: 1,
        cancel: false,
        from: eve.own_key_record()?,
        profile: copy,
        text: "Hello".into(),
        text_format: text_format::PLAIN.into(),
        text_version: text_format::VERSION,
        sections: Vec::new(),
        routines: Vec::new(),
        lines: None,
        shopping: false,
    };
    let file = encode(&item, &eve.signing_key()?, &[], &mut |n| Ok(vec![0; n]))?;
    let fake = ben.open_sent_item(&file)?.map_err(|_| "fake")?;
    assert_eq!(fake.trust, SenderTrust::Unknown);
    assert_ne!(fake.code, first.code);
    ben.accept_sent_item(&fake.item, true)?;
    assert_eq!(people(&ben)?, kept);
    assert!(ben.person_verified(&ann_id)?);
    assert_eq!(ben.person_shown(&ann_id)?.version, 1);
    Ok(())
}

#[test]
fn an_update_or_cancellation_brings_a_newer_profile_kept_verified() -> TestResult {
    let ann = device(1);
    let ben = device(2);
    let ann_id = own_id(&ann)?;
    let note = Notes::new(&ann).capture("Dentist", None, None)?;
    let first = ben
        .open_sent_item(&ann.send_note(&note.id, None, false)?)?
        .map_err(|_| "first")?;
    let got = ben.accept_sent_item(&first.item, true)?.ok_or("accepted")?;

    OwnProfile::new(&ann).rename("Ann Lee")?;
    let update = ben
        .open_sent_item(&ann.send_note(&note.id, None, false)?)?
        .map_err(|_| "update")?;
    assert_eq!(update.sender.name, "Ann Lee");
    assert_eq!(update.trust, SenderTrust::Verified);
    assert_eq!(ben.person_shown(&ann_id)?.name, "Ann", "not accepted yet");
    ben.accept_sent_item(&update.item, false)?;
    assert_eq!(ben.person_shown(&ann_id)?.name, "Ann Lee");
    assert!(ben.person_verified(&ann_id)?);
    assert_eq!(
        ben.author_of(&got)?.map(|p| p.name),
        Some("Ann Lee".to_string())
    );

    OwnProfile::new(&ann).rename("Ann L.")?;
    let cancel = ben
        .open_sent_item(&ann.send_note(&note.id, None, true)?)?
        .map_err(|_| "cancel")?;
    assert_eq!(cancel.state, SentItemState::Cancel);
    ben.accept_sent_item(&cancel.item, false)?;
    assert_eq!(ben.person_shown(&ann_id)?.name, "Ann L.");
    assert!(ben.person_verified(&ann_id)?);
    Ok(())
}

/// The stranger's Sent Item `received` holds.
fn held(received: std::result::Result<Received, OpenError>) -> Made<HeldSentItem> {
    match received {
        Ok(Received::Held(held)) => Ok(held),
        other => Err(format!("not held: {other:?}").into()),
    }
}

type Made<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[test]
fn a_strangers_one_is_held_unseen_until_show_and_accepting_writes_it_then() -> TestResult {
    let ann = device(1);
    let ben = device(2);
    OwnProfile::new(&ann).set_picture(Some(
        include_bytes!("../../../../fixtures/shared/override.jpg").to_vec(),
    ))?;
    let note = Notes::new(&ann).capture("Dentist", None, None)?;
    Notes::new(&ann).amend(&note.id, "Room 3")?;
    let schedule = Schedule::new(
        Day::parse("2026-03-06").ok_or("day")?,
        &[ClockTime::parse("09:30").ok_or("time")?],
        None,
    )
    .ok_or("schedule")?;
    Routines::new(&ann).spawn(&note.id, &schedule)?;
    let file = ann.send_note(&note.id, None, false)?;
    let before = ben.all_entries()?.len();

    let got = held(ben.receive_sent_item(&file)?)?;
    // Only who it is from: no picture, nothing of what it holds.
    assert_eq!(got.sender.id, own_id(&ann)?);
    assert_eq!(got.sender.name, "Ann");
    assert_eq!(got.sender.picture, None);
    let set = ben.pending_set(&got.id)?.ok_or("the set")?;
    assert_eq!(set.kind, PENDING_KIND);
    assert!(!set.shown);
    assert!(set.changes.is_empty());
    assert_eq!(
        BASE64.decode(set.payload.as_deref().ok_or("payload")?)?,
        file
    );
    // Nothing of it is in the log, the lists or People.
    assert_eq!(ben.all_entries()?.len(), before);
    assert!(Notes::new(&ben).all()?.is_empty());
    assert!(people(&ben)?.is_empty());
    let ids: Vec<String> = ben.held_sent_items()?.into_iter().map(|h| h.id).collect();
    assert_eq!(ids, std::slice::from_ref(&got.id));
    assert_eq!(
        ben.held_sent_item(&got.id)?
            .map(|h| h.sender.name)
            .as_deref(),
        Some("Ann")
    );
    // The same item again is the one held.
    assert_eq!(held(ben.receive_sent_item(&file)?)?.id, got.id);
    assert_eq!(ben.pending_sets()?.len(), 1);

    let shown = ben
        .show_held_sent_item(&got.id)?
        .ok_or("held")?
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(shown.item.text, "Dentist");
    assert_eq!(shown.trust, SenderTrust::Unknown);
    assert_eq!(shown.state, SentItemState::Fresh);
    assert_eq!(ben.all_entries()?.len(), before, "still held");

    let accepted = ben
        .accept_held_sent_item(&got.id, false)?
        .ok_or("accepted")?;
    let note = Notes::new(&ben).by_id(&accepted)?.ok_or("the Note")?;
    assert_eq!(note.text, "Dentist");
    assert!(note.is_unsorted());
    let sections: Vec<String> = Notes::new(&ben)
        .sections(&accepted)?
        .into_iter()
        .map(|s| s.text)
        .collect();
    assert_eq!(sections, ["Room 3"]);
    assert_eq!(Routines::new(&ben).of_note(&accepted)?.len(), 1);
    // Captured when accepted, not when it came.
    assert!(note.captured > got.held);
    assert_eq!(ben.person_shown(&own_id(&ann)?)?.name, "Ann");
    assert!(ben.pending_sets()?.is_empty());
    assert!(ben.held_sent_items()?.is_empty());
    assert!(
        ben.local_settings_with_prefix(crate::store::ACCEPTED_PENDING_PREFIX)?
            .is_empty()
    );
    assert_eq!(ben.show_held_sent_item(&got.id)?, None);
    assert!(ben.accept_held_sent_item(&got.id, false).is_err());

    // Known now: the next one from her previews as before.
    let later = Notes::new(&ann).capture("Later", None, None)?;
    match ben.receive_sent_item(&ann.send_note(&later.id, None, false)?)? {
        Ok(Received::Opened(opened)) => assert_eq!(opened.trust, SenderTrust::Known),
        other => return Err(format!("not a preview: {other:?}").into()),
    }
    assert!(ben.pending_sets()?.is_empty());
    Ok(())
}

#[test]
fn decline_deletes_a_held_one_and_accept_works_before_show() -> TestResult {
    let ann = device(1);
    let ben = device(2);
    let log = ben.all_entries()?.len();
    let one = Notes::new(&ann).capture("One", None, None)?;
    let two = Notes::new(&ann).capture("Two", None, None)?;
    let first = held(ben.receive_sent_item(&ann.send_note(&one.id, None, false)?)?)?;
    let second = held(ben.receive_sent_item(&ann.send_note(&two.id, None, false)?)?)?;
    let ids: Vec<String> = ben.held_sent_items()?.into_iter().map(|h| h.id).collect();
    assert_eq!(ids, [first.id.clone(), second.id.clone()]);

    assert!(ben.decline_pending(&first.id)?);
    assert_eq!(ben.held_sent_item(&first.id)?, None);
    assert_eq!(ben.all_entries()?.len(), log);
    assert!(people(&ben)?.is_empty());
    assert_eq!(ben.pending_sets()?.len(), 1);

    let note = ben
        .accept_held_sent_item(&second.id, false)?
        .ok_or("accepted")?;
    assert_eq!(
        Notes::new(&ben).by_id(&note)?.map(|n| n.text).as_deref(),
        Some("Two")
    );
    assert!(ben.pending_sets()?.is_empty());
    Ok(())
}

#[test]
fn accept_judges_a_held_one_against_the_log_as_it_is_then() -> TestResult {
    let ann = device(1);
    let ben = device(2);
    let note = Notes::new(&ann).capture("Dentist", None, None)?;
    let file = ann.send_note(&note.id, None, false)?;
    let got = held(ben.receive_sent_item(&file)?)?;
    // Accepted meanwhile, as another of Ben's Devices may have.
    let opened = ben.open_sent_item(&file)?.map_err(|_| "opened")?;
    let accepted = ben.accept_sent_item(&opened.item, false)?;
    let count = ben.all_entries()?.len();
    let shown = ben
        .show_held_sent_item(&got.id)?
        .ok_or("held")?
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(shown.state, SentItemState::UpToDate);
    assert_eq!(ben.accept_held_sent_item(&got.id, false)?, accepted);
    assert_eq!(ben.all_entries()?.len(), count, "no second Note");
    assert!(ben.pending_sets()?.is_empty());
    Ok(())
}

#[test]
fn one_from_oneself_or_a_known_person_is_not_held() -> TestResult {
    let ann = device(1);
    let note = Notes::new(&ann).capture("Mine", None, None)?;
    let file = ann.send_note(&note.id, None, false)?;
    assert!(matches!(
        ann.receive_sent_item(&file)?,
        Ok(Received::Opened(_))
    ));
    assert!(ann.pending_sets()?.is_empty());
    Ok(())
}
