use super::*;

#[test]
fn importance_reads_its_stored_name_and_defaults_to_normal() {
    for i in [
        Importance::Unimportant,
        Importance::Normal,
        Importance::Important,
    ] {
        assert_eq!(Importance::from_name(Some(i.name())), i);
    }
    assert_eq!(Importance::from_name(None), Importance::Normal);
    assert_eq!(Importance::from_name(Some("huge")), Importance::Normal);
}

#[test]
fn a_note_without_words_is_titled_by_what_it_holds() -> Result<()> {
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    let wordless =
        |id: &str| -> Result<Option<Wordless>> { Ok(notes.by_id(id)?.map(|n| n.wordless())) };
    let untranscribed = notes.capture_with("", None, None, &[], &[voice(1)])?;
    let silent = notes.capture_with("", None, None, &[], &[voice(2)])?;
    notes.set_transcript(&silent.id, &silent.recordings[0], "")?;
    let receipt = notes.capture_with("", None, None, &[photo(1)], &[])?;
    let both = notes.capture_with("", None, None, &[photo(2)], &[voice(3)])?;
    assert_eq!(wordless(&untranscribed.id)?, Some(Wordless::VoiceRecording));
    assert_eq!(wordless(&silent.id)?, Some(Wordless::VoiceRecording));
    assert_eq!(wordless(&receipt.id)?, Some(Wordless::Photo));
    assert_eq!(wordless(&both.id)?, Some(Wordless::Photo));
    // A file gone while its text stays still counts.
    let memo = notes.capture_with("", None, None, &[], &[voice(4)])?;
    notes.set_transcript(&memo.id, &memo.recordings[0], "call the bank")?;
    notes.remove_recording(&memo.id, &memo.recordings[0])?;
    notes.set_photo_text(&both.id, &both.photos[0], "RECEIPT")?;
    notes.remove_photo(&both.id, &both.photos[0])?;
    assert_eq!(notes.by_id(&memo.id)?.map(|n| n.recordings.len()), Some(0));
    assert_eq!(wordless(&memo.id)?, Some(Wordless::VoiceRecording));
    assert_eq!(notes.by_id(&both.id)?.map(|n| n.photos.len()), Some(0));
    assert_eq!(wordless(&both.id)?, Some(Wordless::Photo));
    Ok(())
}

#[test]
fn a_note_of_files_without_words_is_titled_by_its_first_files_name() -> Result<()> {
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    let titled = |id: &str| -> Result<Option<(Wordless, String)>> {
        Ok(notes.by_id(id)?.map(|n| (n.wordless(), n.wordless_name())))
    };
    let file = |name: &'static str, byte: &'static [u8]| NewFile {
        name,
        media_type: "",
        bytes: byte,
        pages: None,
    };
    let lease = notes.capture_with_files(
        "",
        None,
        None,
        &[],
        &[],
        &[file("Lease 2026.pdf", &[1]), file("Plan.zip", &[2])],
    )?;
    let nameless = notes.capture_with_files("", None, None, &[], &[], &[file("", &[3])])?;
    let with_photo =
        notes.capture_with_files("", None, None, &[photo(1)], &[], &[file("a.pdf", &[4])])?;
    let with_voice =
        notes.capture_with_files("", None, None, &[], &[voice(1)], &[file("b.pdf", &[5])])?;
    let file_of = |name: &str| Some((Wordless::File, name.to_string()));
    assert_eq!(titled(&lease.id)?, file_of("Lease 2026.pdf"));
    assert_eq!(titled(&nameless.id)?, file_of(""));
    assert_eq!(
        titled(&with_photo.id)?,
        Some((Wordless::Photo, String::new()))
    );
    assert_eq!(
        titled(&with_voice.id)?,
        Some((Wordless::VoiceRecording, String::new()))
    );
    // The file went, its text stayed: still titled by it.
    let rent = notes.capture_with_files("", None, None, &[], &[], &[file("Rent.txt", &[6])])?;
    let hash = rent.attachments[0].hash.clone();
    notes.set_attachment_text(&rent.id, &hash, "500 EUR")?;
    notes.remove_attachment(&rent.id, &hash)?;
    assert_eq!(notes.by_id(&rent.id)?.map(|n| n.attachments.len()), Some(0));
    assert_eq!(titled(&rent.id)?, file_of("Rent.txt"));
    let words = notes.capture("Words", None, None)?;
    assert_eq!(titled(&words.id)?, Some((Wordless::Photo, String::new())));
    Ok(())
}

#[test]
fn writes_check_what_they_write_to() -> Result<()> {
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    assert!(notes.capture("x", Some("area:none"), None).is_err());
    let a = notes.capture("  kept as typed ", None, None)?;
    assert_eq!(a.text, "  kept as typed ");
    assert!(notes.edit(&a.id, "   ").is_err());
    assert!(notes.edit("note:none", "x").is_err());
    assert!(notes.amend(&a.id, "").is_err());
    assert!(notes.file(&a.id, "topic:none").is_err());
    assert!(notes.merge(&a.id, &a.id).is_err());
    assert!(notes.merge(&a.id, "note:none").is_err());
    notes.edit(&a.id, " fixed ")?;
    assert_eq!(notes.by_id(&a.id)?.map(|n| n.text), Some("fixed".into()));
    let b = notes.capture("other", None, Some(Duration::minutes(10)))?;
    notes.trash(&b.id)?;
    assert!(notes.merge(&a.id, &b.id).is_err());
    notes.restore(&b.id)?;
    notes.merge(&a.id, &b.id)?;
    assert_eq!(notes.sections(&b.id)?.len(), 1);
    assert_eq!(
        store.current(&b.id, keys::NUDGE)?.as_deref(),
        Some("2026-01-01T10:10:06.000Z")
    );
    Ok(())
}

#[test]
fn the_sections_of_many_notes_read_at_once_are_those_of_each() -> Result<()> {
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    let plain = notes.capture("plain", None, None)?;
    let amended = notes.capture("amended", None, None)?;
    notes.amend(&amended.id, "amend")?;
    // A chain a → b → c, with Amends on a and b, read for all three.
    let a = notes.capture("a", None, None)?;
    notes.amend(&a.id, "a amend")?;
    let b = notes.capture("b", None, None)?;
    notes.amend(&b.id, "b amend")?;
    let c = notes.capture("c", None, None)?;
    notes.merge(&a.id, &b.id)?;
    notes.merge(&b.id, &c.id)?;
    // Two Notes merged into each other, and a Merge into a Note gone.
    let x = notes.capture("x", None, None)?;
    let y = notes.capture("y", None, None)?;
    notes.amend(&y.id, "y amend")?;
    store.append(&x.id, keys::MERGED, Some(&y.id), None)?;
    store.append(&y.id, keys::MERGED, Some(&x.id), None)?;
    let lost = notes.capture("lost", None, None)?;
    store.append(&lost.id, keys::MERGED, Some("note:never-synced"), None)?;
    notes.amend(&lost.id, "lost amend")?;
    // A Section whose Note never arrived.
    let orphan = "section:orphan";
    store.append(orphan, keys::TYPE, Some(keys::TYPE_SECTION), None)?;
    store.append(orphan, keys::NOTE, Some("note:never-synced"), None)?;
    store.append(orphan, keys::TEXT, Some("orphan"), None)?;
    let at = dart_iso(store.now());
    store.append(orphan, keys::CAPTURED, Some(&at), None)?;

    let mut ids: Vec<String> = [&plain, &amended, &a, &b, &c, &x, &y, &lost]
        .iter()
        .map(|n| n.id.clone())
        .collect();
    ids.push("note:never-synced".into());
    ids.push("note:unknown".into());
    let read = notes.sections_of(&ids)?;
    assert_eq!(read.len(), ids.len());
    for id in &ids {
        assert_eq!(read.get(id), Some(&notes.sections(id)?), "{id}");
        let alone = notes.sections_of(std::slice::from_ref(id))?;
        assert_eq!(alone.get(id), read.get(id), "{id}");
    }
    let texts = |id: &str| -> Vec<String> {
        read.get(id)
            .map(|list| list.iter().map(|s| s.text.clone()).collect())
            .unwrap_or_default()
    };
    assert_eq!(texts(&b.id), ["b amend"]);
    assert_eq!(texts(&c.id), ["a", "a amend", "b", "b amend"]);
    assert!(
        read.get(&c.id)
            .into_iter()
            .flatten()
            .all(|s| s.note_id == c.id)
    );
    assert_eq!(texts("note:never-synced"), ["orphan"]);
    assert_eq!(read.get("note:unknown"), Some(&Vec::new()));
    assert!(notes.sections_of(&[])?.is_empty());
    Ok(())
}

#[test]
fn a_plain_text_becomes_formatted_at_its_first_format_and_stays_so() -> Result<()> {
    use crate::text_format::{MARKDOWN, PLAIN};
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    let shop = notes.capture_as("# Shop", MARKDOWN, None, None)?;
    assert_eq!(
        store.current(&shop.id, keys::TEXT)?.as_deref(),
        Some(r##"{"format":"markdown","text":"# Shop"}"##)
    );
    let section = notes.amend_as(&shop.id, "~~old~~ new", MARKDOWN)?;
    assert_eq!(section.visible_text(), "old new");
    assert!(notes.capture_as("x", "x-future", None, None).is_err());
    let note = notes.capture("Call mum", None, None)?;
    notes.edit(&note.id, "Call mum later")?;
    let format =
        |id: &str| -> Result<Option<String>> { Ok(notes.by_id(id)?.map(|n| n.text_format)) };
    assert_eq!(format(&note.id)?.as_deref(), Some(PLAIN));
    notes.edit_as(&note.id, "Call **mum**", MARKDOWN)?;
    // An edit without a format, or asking for plain, keeps it formatted.
    notes.edit(&note.id, "Call **mum** now")?;
    notes.edit_as(&note.id, "Call mum", PLAIN)?;
    assert_eq!(
        store.current(&note.id, keys::TEXT)?.as_deref(),
        Some(r#"{"format":"markdown","text":"Call mum"}"#)
    );
    assert!(notes.edit_as(&note.id, "x", "x-future").is_err());
    // set_format makes a plain text formatted, never back.
    let list = notes.capture("- milk", None, None)?;
    notes.set_format(&list.id, MARKDOWN)?;
    assert_eq!(format(&list.id)?.as_deref(), Some(MARKDOWN));
    let entries = store.history(&list.id)?.len();
    notes.set_format(&list.id, PLAIN)?;
    notes.set_format(&list.id, MARKDOWN)?;
    assert_eq!(store.history(&list.id)?.len(), entries);
    Ok(())
}

#[test]
fn a_code_text_keeps_its_glyphs_its_format_and_has_no_boxes()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    use crate::text_format::{CODE, MARKDOWN, PLAIN};
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    let stored = |id: &str| store.current(id, keys::TEXT);
    let note = notes.capture_as("☐ echo", CODE, None, None)?;
    assert_eq!(
        stored(&note.id)?.as_deref(),
        Some(r#"{"format":"code","text":"☐ echo"}"#)
    );
    let read = notes.by_id(&note.id)?.ok_or("the Note")?;
    assert!(read.boxes.is_empty());
    assert_eq!(read.visible_text(), "☐ echo");
    // An edit keeps it code, whatever format it asks.
    notes.edit(&note.id, "ls -l")?;
    notes.edit_as(&note.id, "**ls**", MARKDOWN)?;
    notes.edit_as(&note.id, "ls -la", PLAIN)?;
    assert_eq!(
        stored(&note.id)?.as_deref(),
        Some(r#"{"format":"code","text":"ls -la"}"#)
    );
    // A box with its id that came in a code text is no box.
    let boxed = format!("☐{} x", text_boxes::encode_id("ab12"));
    let came = notes.capture("x", None, None)?;
    let value = StoredText::of(CODE, &boxed, text_format::VERSION).encode();
    store.append(&came.id, keys::TEXT, Some(&value), None)?;
    let came = notes.by_id(&came.id)?.ok_or("the Note")?;
    assert!(came.boxes.is_empty());
    assert_eq!(came.visible_text(), "☐ x");
    // Its words are found.
    let grep = notes.capture_as("grep -r **needle** .", CODE, None, None)?;
    let found = crate::search::Search::new(&store).notes("**needle**", None)?;
    assert_eq!(found.iter().map(|n| &n.id).collect::<Vec<_>>(), [&grep.id]);
    Ok(())
}

#[test]
fn set_format_puts_a_plain_or_formatted_text_into_code_and_back()
-> std::result::Result<(), Box<dyn std::error::Error>> {
    use crate::text_format::{CODE, MARKDOWN, PLAIN};
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    let stored = |id: &str| store.current(id, keys::TEXT);
    let format =
        |id: &str| -> Result<Option<String>> { Ok(notes.by_id(id)?.map(|n| n.text_format)) };
    let note = notes.capture("# comment\necho hi", None, None)?;
    notes.set_format(&note.id, MARKDOWN)?;
    notes.set_format(&note.id, CODE)?;
    assert_eq!(
        stored(&note.id)?.as_deref(),
        Some(r##"{"format":"code","text":"# comment\necho hi"}"##)
    );
    let entries = store.history(&note.id)?.len();
    notes.set_format(&note.id, CODE)?;
    assert_eq!(store.history(&note.id)?.len(), entries);
    notes.set_format(&note.id, PLAIN)?;
    assert_eq!(
        stored(&note.id)?.as_deref(),
        Some(r##"{"format":"plain","text":"# comment\necho hi"}"##)
    );
    notes.set_format(&note.id, CODE)?;
    notes.set_format(&note.id, MARKDOWN)?;
    assert_eq!(format(&note.id)?.as_deref(), Some(MARKDOWN));
    // A formatted text still never goes back to plain.
    notes.set_format(&note.id, PLAIN)?;
    assert_eq!(format(&note.id)?.as_deref(), Some(MARKDOWN));
    // Into code a text goes as it shows; back, its glyphs are boxes.
    let list = notes.capture("☐ milk ☐ eggs", None, None)?;
    Boxes::new(&store).tick(
        &list.id,
        &list.boxes[0].id,
        crate::text_boxes::BoxState::Done,
    )?;
    notes.set_format(&list.id, CODE)?;
    assert_eq!(
        stored(&list.id)?.as_deref(),
        Some(r#"{"format":"code","text":"✅ milk ☐ eggs"}"#)
    );
    notes.set_format(&list.id, PLAIN)?;
    let back = notes.by_id(&list.id)?.ok_or("the Note")?;
    assert_eq!(back.boxes.len(), 2);
    assert_eq!(back.visible_text(), "✅ milk ☐ eggs");
    // A code Note becomes no Checklist or Shopping List.
    let script = notes.capture_as("Script\nls\npwd", CODE, None, None)?;
    let lists = crate::checklists::Checklists::new(&store);
    let shopping = crate::shopping_lists::ShoppingLists::new(&store);
    assert!(!lists.may_make(&script.id)?);
    assert!(!shopping.may_make(&script.id)?);
    notes.set_format(&script.id, PLAIN)?;
    assert!(lists.may_make(&script.id)?);
    assert!(shopping.may_make(&script.id)?);
    Ok(())
}

#[test]
fn a_transcript_taken_over_into_a_formatted_text_reads_as_itself() -> Result<()> {
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    let note = notes.capture_as("**Todo**", crate::text_format::MARKDOWN, None, None)?;
    let written = notes.take_over(&note.id, "2*3*4 and #tag_x\n- not a list ", true)?;
    assert_eq!(written, "**Todo**\n2\\*3\\*4 and #tag_x\n\\- not a list");
    let got = notes
        .by_id(&note.id)?
        .ok_or(Error::Invalid("gone".into()))?;
    assert_eq!(got.text, written);
    assert_eq!(got.visible_text(), "Todo\n2*3*4 and #tag_x\n- not a list");
    assert_eq!(notes.take_over(&note.id, "*x*", false)?, "\\*x\\*");
    // Into a plain text it goes as it is.
    let plain = notes.capture("Plain", None, None)?;
    assert_eq!(notes.take_over(&plain.id, "*x*", true)?, "Plain\n*x*");
    Ok(())
}

#[test]
fn a_checklist_or_shopping_list_keeps_the_text_formatted() -> Result<()> {
    use crate::text_format::MARKDOWN;
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    let format =
        |id: &str| -> Result<Option<String>> { Ok(notes.by_id(id)?.map(|n| n.text_format)) };
    let packing = notes.capture_as("**Packing**\n- socks", MARKDOWN, None, None)?;
    let checklists = crate::Checklists::new(&store);
    checklists.make(&packing.id)?;
    assert_eq!(
        store.current(&packing.id, keys::TEXT)?.as_deref(),
        Some(r#"{"format":"markdown","text":"**Packing**"}"#)
    );
    checklists.to_plain(&packing.id)?;
    assert_eq!(format(&packing.id)?.as_deref(), Some(MARKDOWN));
    let weekly = notes.capture_as("# Weekly\nmilk", MARKDOWN, None, None)?;
    let shopping = crate::ShoppingLists::new(&store);
    shopping.make(&weekly.id)?;
    assert_eq!(format(&weekly.id)?.as_deref(), Some(MARKDOWN));
    shopping.to_plain(&weekly.id, &|utc| utc.naive_utc())?;
    assert_eq!(format(&weekly.id)?.as_deref(), Some(MARKDOWN));
    Ok(())
}

#[test]
fn a_text_in_an_unknown_format_is_never_overwritten_here() -> Result<()> {
    const FUTURE: &str = r##"{"format":"x-future","text":"# Heading"}"##;
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    let a = notes.capture("x", None, None)?;
    assert_eq!(
        store.current(&a.id, keys::TEXT)?.as_deref(),
        Some(r#"{"format":"plain","text":"x"}"#)
    );
    store.append(&a.id, keys::TEXT, Some("Old milk"), None)?;
    let old = notes.by_id(&a.id)?.ok_or(Error::Invalid("note".into()))?;
    assert_eq!((old.text.as_str(), old.text_editable), ("Old milk", true));
    store.append(&a.id, keys::TEXT, Some(FUTURE), None)?;
    let other = notes.capture("Other", None, None)?;
    let note = notes.by_id(&a.id)?.ok_or(Error::Invalid("note".into()))?;
    assert_eq!(note.text, "# Heading");
    assert_eq!(note.text_format, "x-future");
    assert!(!note.text_editable);
    assert!(notes.edit(&a.id, "Plain now").is_err());
    // Amend adds a Section; Merge points at the other Note.
    let section = notes.amend(&a.id, "Amended")?;
    assert!(section.text_editable);
    notes.merge(&a.id, &other.id)?;
    assert_eq!(store.current(&a.id, keys::TEXT)?.as_deref(), Some(FUTURE));
    store.append(&section.id, keys::TEXT, Some(FUTURE), None)?;
    let shown = notes.sections(&other.id)?;
    assert_eq!(shown.len(), 2);
    assert!(
        shown
            .iter()
            .all(|s| !s.text_editable && s.text == "# Heading")
    );
    Ok(())
}

#[test]
fn a_text_in_a_later_structure_is_never_overwritten_here() -> Result<()> {
    const LATER: &str = r#"{"format":"plain","more":1,"text":"Later milk","v":2}"#;
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    let a = notes.capture("x", None, None)?;
    store.append(&a.id, keys::TEXT, Some(LATER), None)?;
    let note = notes.by_id(&a.id)?.ok_or(Error::Invalid("note".into()))?;
    assert_eq!(note.text, "Later milk");
    assert_eq!(note.text_version, 2);
    assert!(!note.text_editable);
    assert!(notes.edit(&a.id, "Plain now").is_err());
    let section = notes.amend(&a.id, "Amended")?;
    store.append(&section.id, keys::TEXT, Some(LATER), None)?;
    let shown = notes.sections(&a.id)?;
    assert_eq!(shown.len(), 1);
    assert_eq!(shown[0].text_version, 2);
    assert!(!shown[0].text_editable);
    assert_eq!(store.current(&a.id, keys::TEXT)?.as_deref(), Some(LATER));
    Ok(())
}

// ------------------------------------------ removing media (ADR 0020)

use crate::lan::{Joining, answer_sync_inviting, apply_answer_as, sync_request_as};
use crate::shared::{AreaRole, Invite};
use crate::signing::SigningKey;
use crate::store::Clock;
use crate::sync::split_blob_ref;
use crate::{PairedDevice, Search};
use chrono::TimeZone;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

/// Devices sharing one clock, a second per read, so they write in a
/// known order; each with a Profile of its own.
struct Devices(Arc<AtomicI64>);

impl Devices {
    fn new() -> Devices {
        Devices(Arc::new(AtomicI64::new(0)))
    }

    fn device(&self, seed: u8, name: &str) -> Result<Store> {
        let ticks = Arc::clone(&self.0);
        let clock: Clock = Box::new(move || {
            let n = ticks.fetch_add(1, Ordering::SeqCst) + 1;
            let start = Utc.with_ymd_and_hms(2026, 3, 1, 9, 0, 0).single();
            start.unwrap_or_default() + Duration::seconds(n)
        });
        let store = Store::in_memory(clock, move || Ok(SigningKey::from_seed([seed; 32])))?;
        crate::profile::OwnProfile::new(&store).apply_defaults(name, None)?;
        Ok(store)
    }
}

#[test]
fn a_discarded_note_is_gone_at_once_without_the_trash() -> Result<()> {
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    let note = notes.capture_with("milk", None, None, &[], &[voice(1)])?;
    let kept = notes.capture("kept", None, None)?;
    notes.discard(&note.id)?;
    assert!(notes.by_id(&note.id)?.is_none());
    let ids: Vec<String> = notes.all()?.into_iter().map(|n| n.id).collect();
    assert_eq!(ids, [kept.id]);
    assert!(notes.trashed()?.is_empty());
    assert!(notes.untranscribed()?.is_empty());
    assert!(notes.discard(&note.id).is_err());
    let frog = notes.capture("frog", None, None)?;
    crate::Frogs::new(&store).spawn(&frog.id)?;
    assert!(notes.discard(&frog.id).is_err(), "it spawned a Frog");
    assert!(notes.by_id(&frog.id)?.is_some());
    Ok(())
}

/// Stand-ins: a JPEG start, so the store keeps the photo as it is.
fn photo(n: u8) -> Vec<u8> {
    vec![0xFF, 0xD8, 0xFF, 0xD9, n]
}

fn voice(n: u8) -> Vec<u8> {
    vec![0x52, 0x49, 0x46, 0x46, n]
}

/// A Note with a photo and a recording, both with their text.
fn with_texts(store: &Store, place: Option<&str>, n: u8) -> Result<(String, String, String)> {
    let notes = Notes::new(store);
    let note = notes.capture_with("Dentist", place, None, &[photo(n)], &[voice(n)])?;
    let (p, r) = (note.photos[0].clone(), note.recordings[0].clone());
    notes.set_photo_text(&note.id, &p, "Dr. Weber, Mon 9-12")?;
    notes.set_transcript(&note.id, &r, "Bring the old X-ray")?;
    Ok((note.id, p, r))
}

/// The blobs `to` lacks, carried over from `from`.
fn carry(from: &Store, to: &Store) -> Result<()> {
    for reference in to.missing_blobs()? {
        let (kind, hash) = split_blob_ref(&reference);
        if let Some(bytes) = from.blob(kind, hash) {
            to.put_blob(kind, hash, &bytes)?;
        }
    }
    Ok(())
}

/// One LAN session without the network, the blobs after it both
/// ways: `joiner` joins `host`, which invites it when `inviting`.
/// Both remember each other, as the apps do.
fn exchange(joiner: &Store, host: &Store, inviting: Option<&Invite>) -> TestResult {
    let host_id = host.device_id()?;
    let joiner_id = joiner.device_id()?;
    let shared = inviting.is_some() || joiner.is_shared_peer(&host_id)?;
    let joining = match inviting {
        Some(_) => Joining::Invited,
        None => Joining::Plain,
    };
    let vector = host.version_vector()?;
    let request = sync_request_as(joiner, &vector, "Phone", &host_id, "", joining)?;
    let (answer, _) = answer_sync_inviting(host, &request, "Desk", inviting)?.ok_or("an answer")?;
    apply_answer_as(joiner, &answer, shared, None)?;
    for (store, id) in [(host, &joiner_id), (joiner, &host_id)] {
        store.remember_device(&PairedDevice {
            id: id.clone(),
            name: "Other".into(),
            fingerprint: String::new(),
            host: None,
            port: None,
            last_sync: store.now(),
            shared,
        })?;
    }
    carry(host, joiner)?;
    carry(joiner, host)?;
    Ok(())
}

fn found(store: &Store, query: &str) -> Result<Vec<String>> {
    Ok(Search::new(store)
        .notes(query, None)?
        .into_iter()
        .map(|n| n.id)
        .collect())
}

fn note(store: &Store, id: &str) -> Result<Note> {
    Notes::new(store)
        .by_id(id)?
        .ok_or_else(|| Error::Invalid(format!("no Note {id}")))
}

/// Ann's Household shared with Ben as a Member, and her private
/// Health beside it: (area, private).
fn household(
    ann: &Store,
    ben: &Store,
) -> std::result::Result<(String, String), Box<dyn std::error::Error>> {
    let places = Places::new(ann);
    let area = ann.share(&places.add_area("Household")?.id)?;
    let private = places.add_area("Health")?.id;
    let invite = Invite {
        area: area.clone(),
        role: AreaRole::Member,
    };
    exchange(ben, ann, Some(&invite))?;
    Ok((area, private))
}

#[test]
fn a_removed_file_goes_and_its_text_stays_and_is_found() -> TestResult {
    let ann = Devices::new().device(1, "Ann")?;
    let notes = Notes::new(&ann);
    let (id, p, r) = with_texts(&ann, None, 1)?;
    assert!(notes.may_remove_photo(&id, &p)?);
    assert!(notes.may_remove_recording(&id, &r)?);
    notes.remove_photo(&id, &p)?;
    notes.remove_recording(&id, &r)?;
    let read = note(&ann, &id)?;
    assert!(read.photos.is_empty() && read.recordings.is_empty());
    assert_eq!(read.removed_photos, vec![p.clone()]);
    assert_eq!(read.removed_recordings, vec![r.clone()]);
    assert_eq!(
        read.photo_texts.get(&p).map(String::as_str),
        Some("Dr. Weber, Mon 9-12")
    );
    assert_eq!(
        read.transcripts.get(&r).map(String::as_str),
        Some("Bring the old X-ray")
    );
    assert_eq!(ann.photo(&p), None);
    assert_eq!(ann.audio(&r), None);
    assert!(ann.missing_blobs()?.is_empty(), "not wanted back");
    assert!(!ann.knows_blob(&format!("photo/{p}"))?);
    assert_eq!(found(&ann, "weber")?, vec![id.clone()]);
    assert_eq!(found(&ann, "x-ray")?, vec![id.clone()]);
    assert!(!notes.may_remove_photo(&id, &p)?);
    assert_eq!(ann.current(&id, &keys::photo(&p))?, None);
    Ok(())
}

#[test]
fn a_file_without_a_text_or_with_an_empty_one_is_not_removed() -> TestResult {
    let ann = Devices::new().device(1, "Ann")?;
    let notes = Notes::new(&ann);
    let n = notes.capture_with("", None, None, &[photo(1), photo(2)], &[voice(1)])?;
    let (p1, p2, r) = (&n.photos[0], &n.photos[1], &n.recordings[0]);
    notes.set_photo_text(&n.id, p2, "   ")?;
    for hash in [p1, p2] {
        assert!(!notes.may_remove_photo(&n.id, hash)?);
        assert!(notes.remove_photo(&n.id, hash).is_err());
    }
    assert!(!notes.may_remove_recording(&n.id, r)?);
    assert!(notes.remove_recording(&n.id, r).is_err());
    assert!(
        notes.remove_photo(&n.id, r).is_err(),
        "a recording is no photo"
    );
    assert!(!notes.may_remove_photo("note:none", p1)?);
    assert_eq!(note(&ann, &n.id)?.photos, n.photos);
    assert!(ann.photo(p1).is_some());
    Ok(())
}

#[test]
fn the_bytes_stay_while_another_note_shows_the_same_photo() -> TestResult {
    let ann = Devices::new().device(1, "Ann")?;
    let notes = Notes::new(&ann);
    let (id, p, _) = with_texts(&ann, None, 1)?;
    let other = notes.capture_with("Same card", None, None, &[photo(1)], &[])?;
    assert_eq!(other.photos, vec![p.clone()]);
    notes.remove_photo(&id, &p)?;
    assert_eq!(ann.photo(&p), Some(photo(1)));
    assert!(ann.removed_blobs()?.is_empty());
    let target = notes.capture("Health", None, None)?;
    notes.merge(&other.id, &target.id)?;
    notes.set_photo_text(&other.id, &p, "Same card")?;
    notes.remove_photo(&other.id, &p)?;
    let section = notes.sections(&target.id)?;
    assert_eq!(section[0].removed_photos, vec![p.clone()]);
    assert_eq!(ann.photo(&p), None, "gone with the last Note showing it");
    Ok(())
}

#[test]
fn the_removal_reaches_another_own_device() -> TestResult {
    let devices = Devices::new();
    let ann = devices.device(1, "Ann")?;
    let desk = devices.device(3, "Ann")?;
    let (id, p, r) = with_texts(&ann, None, 1)?;
    exchange(&desk, &ann, None)?;
    assert!(desk.photo(&p).is_some() && desk.audio(&r).is_some());
    Notes::new(&ann).remove_photo(&id, &p)?;
    Notes::new(&desk).remove_recording(&id, &r)?;
    exchange(&desk, &ann, None)?;
    for store in [&ann, &desk] {
        let read = note(store, &id)?;
        assert!(read.photos.is_empty() && read.recordings.is_empty());
        assert_eq!(read.photo_texts.len() + read.transcripts.len(), 2);
        assert_eq!(store.photo(&p), None);
        assert_eq!(store.audio(&r), None);
        assert!(store.missing_blobs()?.is_empty());
        assert_eq!(found(store, "weber")?, vec![id.clone()]);
    }
    Ok(())
}

#[test]
fn a_removal_in_a_shared_area_reaches_the_member_and_the_manager() -> TestResult {
    let devices = Devices::new();
    let ann = devices.device(1, "Ann")?;
    let ben = devices.device(2, "Ben")?;
    let (area, _) = household(&ann, &ben)?;
    let (id, p, _) = with_texts(&ann, Some(&area), 1)?;
    let (own, own_p, own_r) = with_texts(&ben, Some(&area), 2)?;
    exchange(&ben, &ann, None)?;
    assert!(ben.photo(&p).is_some() && ann.photo(&own_p).is_some());
    assert!(!Notes::new(&ben).may_remove_photo(&id, &p)?);
    Notes::new(&ann).remove_photo(&id, &p)?;
    // A Member removes from their own Note, a Manager from anyone's.
    Notes::new(&ben).remove_photo(&own, &own_p)?;
    Notes::new(&ann).remove_recording(&own, &own_r)?;
    exchange(&ben, &ann, None)?;
    for store in [&ann, &ben] {
        assert_eq!(note(store, &id)?.removed_photos, vec![p.clone()]);
        assert_eq!(note(store, &own)?.removed_photos, vec![own_p.clone()]);
        assert_eq!(note(store, &own)?.removed_recordings, vec![own_r.clone()]);
        assert_eq!(store.photo(&p), None);
        assert_eq!(store.photo(&own_p), None);
        assert_eq!(store.audio(&own_r), None);
        assert_eq!(found(store, "weber")?.len(), 2);
    }
    Ok(())
}

#[test]
fn a_members_removal_on_anothers_note_is_refused_and_ignored() -> TestResult {
    let devices = Devices::new();
    let ann = devices.device(1, "Ann")?;
    let ben = devices.device(2, "Ben")?;
    let (area, _) = household(&ann, &ben)?;
    let (id, p, _) = with_texts(&ann, Some(&area), 1)?;
    exchange(&ben, &ann, None)?;
    assert!(Notes::new(&ben).remove_photo(&id, &p).is_err());
    // As a modified app would write it.
    ben.append(&id, &keys::photo(&p), None, None)?;
    ben.drop_removed_blobs()?;
    exchange(&ben, &ann, None)?;
    for store in [&ann, &ben] {
        assert_eq!(note(store, &id)?.photos, vec![p.clone()]);
        assert!(store.photo(&p).is_some());
    }
    Ok(())
}

#[test]
fn another_persons_removal_outside_a_shared_area_keeps_the_bytes() -> TestResult {
    let devices = Devices::new();
    let ann = devices.device(1, "Ann")?;
    let ben = devices.device(2, "Ben")?;
    let (area, private) = household(&ann, &ben)?;
    let (id, p, _) = with_texts(&ann, Some(&area), 1)?;
    exchange(&ben, &ann, None)?;
    // Ann takes the Note out of the Shared Area; Ben has not seen it
    // and writes a removal as a modified app would.
    Notes::new(&ann).file(&id, &private)?;
    ben.append(&id, &keys::photo(&p), None, None)?;
    exchange(&ben, &ann, None)?;
    assert_eq!(ann.current(&id, &keys::photo(&p))?, None);
    assert!(ann.removed_blobs()?.is_empty());
    assert_eq!(ann.photo(&p), Some(photo(1)));
    Ok(())
}

#[test]
fn a_removed_text_goes_its_file_stays_and_it_is_not_read_again() -> TestResult {
    let ann = Devices::new().device(1, "Ann")?;
    let notes = Notes::new(&ann);
    let (id, p, r) = with_texts(&ann, None, 1)?;
    assert!(notes.may_remove_photo_text(&id, &p)?);
    assert!(notes.may_remove_transcript(&id, &r)?);
    notes.remove_photo_text(&id, &p)?;
    notes.remove_transcript(&id, &r)?;
    let read = note(&ann, &id)?;
    assert_eq!(read.photos, vec![p.clone()]);
    assert_eq!(read.recordings, vec![r.clone()]);
    assert!(read.photo_texts.is_empty() && read.transcripts.is_empty());
    assert_eq!(read.removed_photo_texts, vec![p.clone()]);
    assert_eq!(read.removed_transcripts, vec![r.clone()]);
    assert!(read.removed_photos.is_empty() && read.removed_recordings.is_empty());
    assert!(ann.photo(&p).is_some() && ann.audio(&r).is_some());
    assert!(found(&ann, "weber")?.is_empty());
    assert!(found(&ann, "x-ray")?.is_empty());
    assert!(notes.unread_photos()?.is_empty());
    assert!(notes.untranscribed()?.is_empty());
    // Without a text the file may not go either.
    assert!(!notes.may_remove_photo(&id, &p)?);
    assert!(!notes.may_remove_photo_text(&id, &p)?);
    assert_eq!(ann.current(&id, &keys::transcript(&r))?, None);
    // Transcribing or reading again brings a text back.
    notes.set_photo_text(&id, &p, "Dr. Weber, Tue 9-12")?;
    notes.set_transcript(&id, &r, "Bring the new X-ray")?;
    let read = note(&ann, &id)?;
    assert!(read.removed_photo_texts.is_empty() && read.removed_transcripts.is_empty());
    assert_eq!(found(&ann, "tue")?, vec![id.clone()]);
    assert_eq!(found(&ann, "new x-ray")?, vec![id.clone()]);
    Ok(())
}

#[test]
fn a_missing_or_empty_text_or_one_without_its_file_is_not_removed() -> TestResult {
    let ann = Devices::new().device(1, "Ann")?;
    let notes = Notes::new(&ann);
    let n = notes.capture_with("", None, None, &[photo(1), photo(2)], &[voice(1)])?;
    let (p1, p2, r) = (&n.photos[0], &n.photos[1], &n.recordings[0]);
    notes.set_photo_text(&n.id, p2, "")?;
    for hash in [p1, p2] {
        assert!(!notes.may_remove_photo_text(&n.id, hash)?);
        assert!(notes.remove_photo_text(&n.id, hash).is_err());
    }
    assert!(!notes.may_remove_transcript(&n.id, r)?);
    assert!(notes.remove_transcript(&n.id, r).is_err());
    assert!(
        notes.remove_photo_text(&n.id, r).is_err(),
        "a recording is no photo"
    );
    assert!(!notes.may_remove_transcript("note:none", r)?);
    let (id, p, _) = with_texts(&ann, None, 3)?;
    notes.remove_photo(&id, &p)?;
    assert!(!notes.may_remove_photo_text(&id, &p)?);
    assert!(notes.remove_photo_text(&id, &p).is_err());
    Ok(())
}

#[test]
fn a_removed_text_shows_in_the_section_and_reaches_another_own_device() -> TestResult {
    let devices = Devices::new();
    let ann = devices.device(1, "Ann")?;
    let desk = devices.device(3, "Ann")?;
    let notes = Notes::new(&ann);
    let (id, p, r) = with_texts(&ann, None, 1)?;
    let target = notes.capture("Health", None, None)?;
    notes.merge(&id, &target.id)?;
    exchange(&desk, &ann, None)?;
    notes.remove_transcript(&id, &r)?;
    Notes::new(&desk).remove_photo_text(&id, &p)?;
    exchange(&desk, &ann, None)?;
    for store in [&ann, &desk] {
        let section = Notes::new(store).sections(&target.id)?;
        assert_eq!(section[0].removed_transcripts, vec![r.clone()]);
        assert_eq!(section[0].removed_photo_texts, vec![p.clone()]);
        assert!(store.photo(&p).is_some() && store.audio(&r).is_some());
        assert!(Notes::new(store).untranscribed()?.is_empty());
        assert!(Notes::new(store).unread_photos()?.is_empty());
        assert!(found(store, "weber")?.is_empty());
    }
    Ok(())
}

#[test]
fn a_members_text_removal_on_anothers_note_is_refused_and_ignored() -> TestResult {
    let devices = Devices::new();
    let ann = devices.device(1, "Ann")?;
    let ben = devices.device(2, "Ben")?;
    let (area, _) = household(&ann, &ben)?;
    let (id, p, r) = with_texts(&ann, Some(&area), 1)?;
    let (own, own_p, own_r) = with_texts(&ben, Some(&area), 2)?;
    exchange(&ben, &ann, None)?;
    assert!(!Notes::new(&ben).may_remove_transcript(&id, &r)?);
    assert!(Notes::new(&ben).remove_transcript(&id, &r).is_err());
    // As a modified app would write them.
    ben.append(&id, &keys::transcript(&r), None, None)?;
    ben.append(&id, &keys::ocr(&p), None, None)?;
    // A Member removes from their own Note, a Manager from anyone's.
    Notes::new(&ben).remove_photo_text(&own, &own_p)?;
    Notes::new(&ann).remove_transcript(&own, &own_r)?;
    exchange(&ben, &ann, None)?;
    for store in [&ann, &ben] {
        let read = note(store, &id)?;
        assert_eq!(
            read.transcripts.get(&r).map(String::as_str),
            Some("Bring the old X-ray")
        );
        assert_eq!(
            read.photo_texts.get(&p).map(String::as_str),
            Some("Dr. Weber, Mon 9-12")
        );
        assert!(read.removed_transcripts.is_empty());
        let own = note(store, &own)?;
        assert_eq!(own.removed_photo_texts, vec![own_p.clone()]);
        assert_eq!(own.removed_transcripts, vec![own_r.clone()]);
    }
    Ok(())
}

#[test]
fn a_members_machine_still_gives_anothers_note_its_texts() -> TestResult {
    let devices = Devices::new();
    let ann = devices.device(1, "Ann")?;
    let ben = devices.device(2, "Ben")?;
    let (area, _) = household(&ann, &ben)?;
    let shed =
        Notes::new(&ann).capture_with("Shed", Some(&area), None, &[photo(4)], &[voice(4)])?;
    exchange(&ben, &ann, None)?;
    Notes::new(&ben).set_photo_text(&shed.id, &shed.photos[0], "Spare key")?;
    Notes::new(&ben).set_transcript(&shed.id, &shed.recordings[0], "Oil the door")?;
    exchange(&ben, &ann, None)?;
    let read = note(&ann, &shed.id)?;
    assert_eq!(
        read.photo_texts.get(&shed.photos[0]).map(String::as_str),
        Some("Spare key")
    );
    assert_eq!(
        read.transcripts
            .get(&shed.recordings[0])
            .map(String::as_str),
        Some("Oil the door")
    );
    Ok(())
}

#[test]
fn a_removed_photo_leaves_the_shared_folder() -> TestResult {
    use crate::folder::{MemoryFolder, folder_sync, random_nonce};
    let devices = Devices::new();
    let ann = devices.device(1, "Ann")?;
    let desk = devices.device(3, "Ann")?;
    exchange(&desk, &ann, None)?;
    let (id, p, r) = with_texts(&ann, None, 1)?;
    let mut folder = MemoryFolder::default();
    let blobs = |folder: &MemoryFolder| folder.dirs.get("blobs").map_or(0, |b| b.len());
    folder_sync(&ann, &mut folder, &mut random_nonce)?;
    assert_eq!(blobs(&folder), 2);
    folder_sync(&desk, &mut folder, &mut random_nonce)?;
    assert!(desk.photo(&p).is_some());
    Notes::new(&ann).remove_photo(&id, &p)?;
    folder_sync(&ann, &mut folder, &mut random_nonce)?;
    assert_eq!(blobs(&folder), 1);
    folder_sync(&desk, &mut folder, &mut random_nonce)?;
    assert_eq!(desk.photo(&p), None);
    assert!(desk.audio(&r).is_some());
    assert_eq!(note(&desk, &id)?.removed_photos, vec![p]);
    Ok(())
}

#[test]
fn a_note_without_a_capture_time_is_no_note() -> Result<()> {
    let store = crate::store::tests::store();
    store.append("note:a", keys::TYPE, Some(keys::TYPE_NOTE), None)?;
    assert_eq!(Notes::new(&store).by_id("note:a")?, None);
    store.append(
        "note:a",
        keys::CAPTURED,
        Some("2026-01-01T10:00:00.000Z"),
        None,
    )?;
    let note = Notes::new(&store).by_id("note:a")?;
    assert!(note.is_some_and(|n| n.is_unsorted() && n.text.is_empty()));
    Ok(())
}

#[test]
fn names_read_only_the_strings_of_a_json_list() {
    let read = |json: Option<&str>| names(json);
    assert_eq!(read(None), Vec::<String>::new());
    assert_eq!(read(Some(r#"["Standup","Lunch"]"#)), ["Standup", "Lunch"]);
    // As the Dart core's `whereType<String>`: in their order, empty
    // and repeated ones kept.
    assert_eq!(
        read(Some(r#" ["Dentist",3,null,"",["x"],{"a":"b"},"Dentist"] "#)),
        ["Dentist", "", "Dentist"]
    );
    for garbled in [
        "",
        "not json",
        r#""Car kit""#,
        r#"{"name":"Car kit"}"#,
        "[]",
        r#"["a",]"#,
        "[\"a\"] x",
    ] {
        assert_eq!(read(Some(garbled)), Vec::<String>::new(), "{garbled:?}");
    }
}

#[test]
fn a_source_app_is_named_by_its_label_else_its_package() {
    let app = |label: Option<&str>| SourceApp {
        package: "com.whatsapp".into(),
        label: label.map(str::to_string),
    };
    assert_eq!(app(Some("WhatsApp")).name(), "WhatsApp");
    assert_eq!(app(Some(" WhatsApp\u{a0}")).name(), "WhatsApp");
    for blank in [None, Some(""), Some("  "), Some("\u{feff}\n")] {
        assert_eq!(app(blank).name(), "com.whatsapp", "{blank:?}");
    }
}

/// Writes `values` on `id`, as another version or Device might have.
fn put(store: &Store, id: &str, values: &[(&str, Option<&str>)]) -> Result<()> {
    for (field, value) in values {
        store.append(id, field, *value, None)?;
    }
    Ok(())
}

#[test]
fn notes_and_sections_read_their_capture_context() -> Result<()> {
    use crate::named_places::keys as place_keys;
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    for (id, name, centre, radius) in [
        ("namedplace:home", "Home", "52.52,13.405", "300.0"),
        ("namedplace:kitchen", "Kitchen", "52.521,13.405", "30.0"),
    ] {
        put(
            &store,
            id,
            &[
                (place_keys::TYPE, Some(place_keys::TYPE_NAMED_PLACE)),
                (place_keys::NAME, Some(name)),
                (place_keys::CENTRE, Some(centre)),
                (place_keys::RADIUS, Some(radius)),
            ],
        )?;
    }
    let shared = notes.capture("Shared in", None, None)?;
    assert_eq!(shared.context, CaptureContext::default());
    assert!(!shared.context.has_more());
    put(
        &store,
        &shared.id,
        &[
            (keys::LOCATION, Some("52.521,13.405")),
            (keys::SOURCE_PACKAGE, Some("com.whatsapp")),
            (keys::CALENDAR, Some(r#"["Standup"]"#)),
            (keys::BLUETOOTH, Some(r#"["Car kit"]"#)),
        ],
    )?;
    let context = note(&store, &shared.id)?.context;
    assert_eq!(context.location, Coordinates::new(52.521, 13.405));
    // The smaller of the two holding it.
    assert_eq!(
        context.named_place.map(|p| p.id).as_deref(),
        Some("namedplace:kitchen")
    );
    let app = context.source_app.ok_or(Error::Invalid("no app".into()))?;
    assert_eq!((app.package.as_str(), app.label), ("com.whatsapp", None));
    assert_eq!(context.calendar_events, ["Standup"]);
    assert_eq!(context.bluetooth_devices, ["Car kit"]);
    // An Amend keeps its own, under the same keys; garbled values
    // read as nothing, and an empty package as no app.
    let host = notes.capture("Host", None, None)?;
    let amend = notes.amend(&host.id, "Later")?;
    put(
        &store,
        &amend.id,
        &[
            (keys::LOCATION, Some(" 52.52 , 13.405 ")),
            (keys::SOURCE_PACKAGE, Some("")),
            (keys::SOURCE_LABEL, Some("Ghost")),
            (keys::CALENDAR, Some("not json")),
            (keys::BLUETOOTH, Some(r#"["Ear buds"]"#)),
        ],
    )?;
    put(&store, &host.id, &[(keys::LOCATION, Some("91,0"))])?;
    notes.merge(&shared.id, &host.id)?;
    let host_context = note(&store, &host.id)?.context;
    assert_eq!(host_context.location, None);
    assert_eq!(host_context.named_place, None);
    let sections = notes.sections(&host.id)?;
    let at_once = notes.sections_of(std::slice::from_ref(&host.id))?;
    assert_eq!(at_once.get(&host.id), Some(&sections));
    let by_id = |id: &str| sections.iter().find(|s| s.id == id).map(|s| &s.context);
    // The merged Note brings its own.
    assert_eq!(by_id(&shared.id), Some(&note(&store, &shared.id)?.context));
    let amended = by_id(&amend.id).cloned().unwrap_or_default();
    assert_eq!(
        amended.named_place.as_ref().map(|p| p.name.as_str()),
        Some("Home")
    );
    assert_eq!(amended.source_app, None);
    assert!(amended.calendar_events.is_empty());
    assert_eq!(amended.bluetooth_devices, ["Ear buds"]);
    assert!(amended.has_more());
    // A Named Place renamed or deleted shows on what was captured
    // before.
    put(&store, "namedplace:kitchen", &[(place_keys::TYPE, None)])?;
    let moved = note(&store, &shared.id)?.context.named_place;
    assert_eq!(moved.map(|p| p.name).as_deref(), Some("Home"));
    put(
        &store,
        "namedplace:home",
        &[(place_keys::NAME, Some("Flat"))],
    )?;
    let renamed = note(&store, &shared.id)?.context.named_place;
    assert_eq!(renamed.map(|p| p.name).as_deref(), Some("Flat"));
    // A cleared field reads as unknown again.
    put(
        &store,
        &shared.id,
        &[(keys::CALENDAR, None), (keys::LOCATION, None)],
    )?;
    let cleared = note(&store, &shared.id)?.context;
    assert!(cleared.calendar_events.is_empty() && cleared.location.is_none());
    Ok(())
}

// ------------------------------- removing a captured detail (ADR 0037)

/// What the phone records on `id` when capturing at Work, shared in
/// from a chat, during a meeting and with a headset connected.
fn with_context(store: &Store, id: &str) -> Result<()> {
    put(
        store,
        id,
        &[
            (keys::SOURCE_PACKAGE, Some("org.example.chat")),
            (keys::SOURCE_LABEL, Some("Chat")),
            (keys::LOCATION, Some("52.52,13.405")),
            (keys::CALENDAR, Some(r#"["Katzenklo"]"#)),
            (keys::BLUETOOTH, Some(r#"["Headset"]"#)),
        ],
    )
}

#[test]
fn each_captured_detail_goes_on_its_own_and_the_time_stays() -> TestResult {
    use crate::named_places::keys as place_keys;
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    put(
        &store,
        "namedplace:work",
        &[
            (place_keys::TYPE, Some(place_keys::TYPE_NAMED_PLACE)),
            (place_keys::NAME, Some("Work")),
            (place_keys::CENTRE, Some("52.52,13.405")),
            (place_keys::RADIUS, Some("100.0")),
        ],
    )?;
    let id = notes.capture("Order toner", None, None)?.id;
    with_context(&store, &id)?;
    let at = note(&store, &id)?.captured;
    for detail in CapturedDetail::ALL {
        assert!(notes.may_remove_captured(&id, detail)?, "{detail:?}");
        assert_eq!(CapturedDetail::parse(detail.name()), Some(detail));
    }
    assert_eq!(CapturedDetail::parse("time"), None);

    notes.remove_captured(&id, CapturedDetail::Event)?;
    let context = note(&store, &id)?.context;
    assert!(context.calendar_events.is_empty());
    assert_eq!(context.bluetooth_devices, ["Headset"]);
    assert_eq!(context.named_place.map(|p| p.name).as_deref(), Some("Work"));
    notes.remove_captured(&id, CapturedDetail::Place)?;
    let context = note(&store, &id)?.context;
    assert!(context.location.is_none() && context.named_place.is_none());
    assert_eq!(NamedPlaces::new(&store).all()?.len(), 1);
    notes.remove_captured(&id, CapturedDetail::Device)?;
    notes.remove_captured(&id, CapturedDetail::App)?;
    let read = note(&store, &id)?;
    assert_eq!(read.context, CaptureContext::default());
    assert_eq!(read.captured, at);
    let last: Vec<(String, Option<String>)> = store
        .history(&id)?
        .into_iter()
        .rev()
        .take(5)
        .map(|e| (e.field, e.value))
        .collect();
    let nulls: Vec<(String, Option<String>)> = [
        keys::SOURCE_LABEL,
        keys::SOURCE_PACKAGE,
        keys::BLUETOOTH,
        keys::LOCATION,
        keys::CALENDAR,
    ]
    .iter()
    .map(|f| (f.to_string(), None))
    .collect();
    assert_eq!(last, nulls);

    // What does not show is not removed: gone once, garbled, typed.
    let typed = notes.capture("Typed", None, None)?.id;
    put(
        &store,
        &typed,
        &[
            (keys::LOCATION, Some("somewhere")),
            (keys::CALENDAR, Some("[]")),
        ],
    )?;
    let mark = store.current_seq()?;
    for (on, detail) in CapturedDetail::ALL
        .iter()
        .flat_map(|d| [(&id, *d), (&typed, *d)])
    {
        assert!(!notes.may_remove_captured(on, detail)?);
        assert!(notes.remove_captured(on, detail).is_err());
    }
    assert_eq!(store.current_seq()?, mark);
    assert!(
        notes
            .remove_captured("note:none", CapturedDetail::Place)
            .is_err()
    );
    let area = Places::new(&store).add_area("Work")?.id;
    assert!(!notes.may_remove_captured(&area, CapturedDetail::Place)?);
    Ok(())
}

#[test]
fn an_amend_and_a_merged_note_lose_theirs_as_sections() -> TestResult {
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    let host = notes.capture("Toner", None, None)?;
    let amend = notes.amend(&host.id, "Ordered")?;
    with_context(&store, &amend.id)?;
    let merged = notes.capture("Order toner", None, None)?.id;
    with_context(&store, &merged)?;
    notes.merge(&merged, &host.id)?;
    assert!(notes.may_remove_captured(&amend.id, CapturedDetail::Event)?);
    notes.remove_captured(&amend.id, CapturedDetail::Event)?;
    notes.remove_captured(&merged, CapturedDetail::Device)?;
    let sections = notes.sections(&host.id)?;
    let of = |id: &str| {
        sections
            .iter()
            .find(|s| s.id == id)
            .map(|s| s.context.clone())
            .unwrap_or_default()
    };
    assert!(of(&amend.id).calendar_events.is_empty());
    assert!(of(&amend.id).location.is_some());
    assert!(of(&merged).bluetooth_devices.is_empty());
    assert_eq!(of(&merged).calendar_events, ["Katzenklo"]);
    Ok(())
}

#[test]
fn a_members_removal_of_a_captured_detail_is_refused_and_ignored() -> TestResult {
    let devices = Devices::new();
    let ann = devices.device(1, "Ann")?;
    let ben = devices.device(2, "Ben")?;
    let (area, _) = household(&ann, &ben)?;
    let id = Notes::new(&ann)
        .capture("Order toner", Some(&area), None)?
        .id;
    with_context(&ann, &id)?;
    let own = Notes::new(&ben)
        .capture("Ben's toner", Some(&area), None)?
        .id;
    with_context(&ben, &own)?;
    exchange(&ben, &ann, None)?;
    let mark = ben.current_seq()?;
    for detail in CapturedDetail::ALL {
        assert!(!Notes::new(&ben).may_remove_captured(&id, detail)?);
        assert!(Notes::new(&ben).remove_captured(&id, detail).is_err());
    }
    assert_eq!(ben.current_seq()?, mark);
    // As a modified app would write them.
    put(
        &ben,
        &id,
        &[
            (keys::LOCATION, None),
            (keys::CALENDAR, None),
            (keys::BLUETOOTH, None),
            (keys::SOURCE_PACKAGE, None),
        ],
    )?;
    // A Member removes from their own Note, a Manager from anyone's.
    Notes::new(&ben).remove_captured(&own, CapturedDetail::Place)?;
    Notes::new(&ann).remove_captured(&own, CapturedDetail::Event)?;
    exchange(&ben, &ann, None)?;
    for store in [&ann, &ben] {
        let context = note(store, &id)?.context;
        assert!(context.location.is_some());
        assert_eq!(context.calendar_events, ["Katzenklo"]);
        assert_eq!(context.bluetooth_devices, ["Headset"]);
        assert_eq!(
            context.source_app.map(|a| a.package).as_deref(),
            Some("org.example.chat")
        );
        let theirs = note(store, &own)?.context;
        assert!(theirs.location.is_none() && theirs.calendar_events.is_empty());
        assert_eq!(theirs.bluetooth_devices, ["Headset"]);
    }
    Ok(())
}

// ------------------------------------------ editing a Section (#183)

fn section_of(store: &Store, id: &str) -> Result<Section> {
    Notes::new(store)
        .section_by_id(id)?
        .ok_or_else(|| Error::Invalid(format!("no Section {id}")))
}

#[test]
fn an_amend_is_fixed_in_place_and_shows_when_and_by_whom() -> TestResult {
    let ann = Devices::new().device(1, "Ann")?;
    let notes = Notes::new(&ann);
    let note = notes.capture("Dentist on Tuesday", None, None)?;
    let amend = notes.amend(&note.id, "Moved to Wednesday")?;
    let later = notes.amend(&note.id, "Called, all set")?;
    assert!(amend.modified.is_none() && amend.edited_by.is_none());
    assert!(notes.may_edit_section(&amend.id)?);
    notes.edit_section(&amend.id, "  Moved to Thursday  ")?;
    let edited = section_of(&ann, &amend.id)?;
    assert_eq!(edited.text, "Moved to Thursday");
    assert_eq!(edited.created(), amend.created());
    assert!(edited.modified.is_some_and(|at| at > edited.created()));
    assert_eq!(edited.edited_by.map(|p| p.name).as_deref(), Some("Ann"));
    let order: Vec<String> = notes
        .sections(&note.id)?
        .into_iter()
        .map(|s| s.id)
        .collect();
    assert_eq!(order, [amend.id.clone(), later.id.clone()]);
    assert!(section_of(&ann, &later.id)?.modified.is_none());
    let texts: Vec<String> = ann
        .history(&amend.id)?
        .into_iter()
        .filter(|e| e.field == keys::TEXT)
        .map(|e| StoredText::read(e.value.as_deref()).text)
        .collect();
    assert_eq!(texts, ["Moved to Wednesday", "Moved to Thursday"]);
    Ok(())
}

#[test]
fn an_unchanged_section_text_writes_nothing_and_a_blank_one_is_refused() -> TestResult {
    let ann = Devices::new().device(1, "Ann")?;
    let notes = Notes::new(&ann);
    let note = notes.capture("Dentist", None, None)?;
    let amend = notes.amend(&note.id, "Bring ☐ the X-ray")?;
    let mark = ann.current_seq()?;
    let stored = section_of(&ann, &amend.id)?.text;
    notes.edit_section(&amend.id, &stored)?;
    notes.edit_section(&amend.id, &format!(" {stored}\n"))?;
    assert_eq!(ann.current_seq()?, mark);
    assert!(section_of(&ann, &amend.id)?.modified.is_none());
    assert!(notes.edit_section(&amend.id, "  ").is_err());
    assert!(notes.edit_section("section:none", "Text").is_err());
    assert!(notes.edit_section(&note.id, "Text").is_err());
    assert!(notes.edit_section_as(&amend.id, "Text", "x-later").is_err());
    assert_eq!(ann.current_seq()?, mark);
    Ok(())
}

#[test]
fn a_section_edit_keeps_the_format_and_the_boxes_as_a_note_edit_does() -> TestResult {
    let ann = Devices::new().device(1, "Ann")?;
    let notes = Notes::new(&ann);
    let note = notes.capture("Trip", None, None)?;
    let amend = notes.amend(&note.id, "Tent")?;
    notes.edit_section_as(&amend.id, "**Tent** ☐", text_format::MARKDOWN)?;
    let section = section_of(&ann, &amend.id)?;
    assert_eq!(section.text_format, text_format::MARKDOWN);
    assert_eq!(section.boxes.len(), 1);
    assert_eq!(section.shown_text(), "**Tent** ☐");
    let id = section.boxes[0].id.clone();
    notes.edit_section_as(
        &amend.id,
        &format!("{} torch", section.text),
        text_format::PLAIN,
    )?;
    let section = section_of(&ann, &amend.id)?;
    assert_eq!(section.text_format, text_format::MARKDOWN);
    assert_eq!(section.boxes[0].id, id);
    assert_eq!(section.shown_text(), "**Tent** ☐ torch");
    // A text in a format not known here is never overwritten.
    ann.append(
        &amend.id,
        keys::TEXT,
        Some(r##"{"format":"x-future","text":"# Tent"}"##),
        None,
    )?;
    assert!(!notes.may_edit_section(&amend.id)?);
    let mark = ann.current_seq()?;
    assert!(notes.edit_section(&amend.id, "Tent").is_err());
    assert_eq!(ann.current_seq()?, mark);
    Ok(())
}

#[test]
fn a_merged_note_is_edited_as_a_section_of_its_target() -> TestResult {
    let ann = Devices::new().device(1, "Ann")?;
    let notes = Notes::new(&ann);
    let host = notes.capture("Dentist number", None, None)?;
    let merged = notes.capture("Ask about the crown", None, None)?;
    notes.merge(&merged.id, &host.id)?;
    assert!(notes.may_edit_section(&merged.id)?);
    notes.edit_section(&merged.id, "Ask about the bridge")?;
    let sections = notes.sections(&host.id)?;
    assert_eq!(sections.len(), 1);
    assert_eq!(sections[0].id, merged.id);
    assert_eq!(sections[0].text, "Ask about the bridge");
    assert!(sections[0].modified.is_some());
    assert_eq!(
        sections[0].edited_by.as_ref().map(|p| p.name.as_str()),
        Some("Ann")
    );
    assert_eq!(note(&ann, &merged.id)?.text, "Ask about the bridge");
    Ok(())
}

#[test]
fn a_member_edits_their_own_amend_and_a_manager_anyones() -> TestResult {
    let devices = Devices::new();
    let ann = devices.device(1, "Ann")?;
    let ben = devices.device(2, "Ben")?;
    let (area, _) = household(&ann, &ben)?;
    let note = Notes::new(&ann).capture("Buy milk", Some(&area), None)?;
    exchange(&ben, &ann, None)?;
    let own = Notes::new(&ben).amend(&note.id, "Oat milk please")?;
    assert!(Notes::new(&ben).may_edit_section(&own.id)?);
    Notes::new(&ben).edit_section(&own.id, "Oat milk, two litres")?;
    exchange(&ben, &ann, None)?;
    for store in [&ann, &ben] {
        let section = section_of(store, &own.id)?;
        assert_eq!(section.text, "Oat milk, two litres");
        assert_eq!(section.edited_by.map(|p| p.name).as_deref(), Some("Ben"));
    }
    assert!(Notes::new(&ann).may_edit_section(&own.id)?);
    Notes::new(&ann).edit_section(&own.id, "Oat milk, three litres")?;
    exchange(&ben, &ann, None)?;
    for store in [&ann, &ben] {
        let section = section_of(store, &own.id)?;
        assert_eq!(section.text, "Oat milk, three litres");
        assert_eq!(section.created(), own.created());
        assert_eq!(section.edited_by.map(|p| p.name).as_deref(), Some("Ann"));
    }
    assert_eq!(
        section_of(&ann, &own.id)?.modified,
        section_of(&ben, &own.id)?.modified
    );
    Ok(())
}

#[test]
fn a_members_edit_of_anothers_amend_is_refused_and_ignored() -> TestResult {
    let devices = Devices::new();
    let ann = devices.device(1, "Ann")?;
    let ben = devices.device(2, "Ben")?;
    let (area, _) = household(&ann, &ben)?;
    let note = Notes::new(&ann).capture("Buy milk", Some(&area), None)?;
    let theirs = Notes::new(&ann).amend(&note.id, "Two litres")?;
    exchange(&ben, &ann, None)?;
    let mark = ben.current_seq()?;
    assert!(!Notes::new(&ben).may_edit_section(&theirs.id)?);
    assert!(
        Notes::new(&ben)
            .edit_section(&theirs.id, "Ten litres")
            .is_err()
    );
    assert_eq!(ben.current_seq()?, mark);
    // As a modified app would write it.
    let forged = StoredText::of(text_format::PLAIN, "Ten litres", text_format::VERSION).encode();
    ben.append(&theirs.id, keys::TEXT, Some(&forged), None)?;
    exchange(&ben, &ann, None)?;
    for store in [&ann, &ben] {
        let section = section_of(store, &theirs.id)?;
        assert_eq!(section.text, "Two litres");
        assert!(section.modified.is_none() && section.edited_by.is_none());
    }
    Ok(())
}

fn ids(notes: Vec<Note>) -> Vec<String> {
    notes.into_iter().map(|n| n.id).collect()
}

fn merged_into(store: &Store, id: &str) -> Result<Option<String>> {
    Ok(Notes::new(store).by_id(id)?.and_then(|n| n.merged_into))
}

#[test]
fn an_undone_merge_brings_the_note_back_where_it_was_with_all_it_holds() -> TestResult {
    let ann = Devices::new().device(1, "Ann")?;
    let notes = Notes::new(&ann);
    let home = Places::new(&ann).add_area("Home")?.id;
    let keys = notes.capture("Spare keys at the neighbours", Some(&home), None)?;
    let amend = notes.amend(&keys.id, "Back by Friday")?;
    use crate::schedule::{ClockTime, Day, Frequency, Recurrence, Schedule};
    let eight = ClockTime::new(8, 0).into_iter().collect::<Vec<_>>();
    let rule = Recurrence::every(Frequency::Daily, 1);
    let daily = Schedule::new(Day::new(2026, 3, 1), &eight, Some(rule)).ok_or("a Schedule")?;
    let routine = crate::Routines::new(&ann).spawn(&keys.id, &daily)?;
    let frog = crate::Frogs::new(&ann).spawn(&keys.id)?;
    let loose = notes.capture("Ask about the plants", None, None)?;
    let into = notes.capture("Neighbours on holiday", None, None)?;
    notes.merge(&keys.id, &into.id)?;
    notes.merge(&loose.id, &into.id)?;
    assert_eq!(ids(notes.all()?), std::slice::from_ref(&into.id));
    let routines_of = |id: &str| -> Result<Vec<String>> {
        Ok(crate::Routines::new(&ann)
            .of_note(id)?
            .into_iter()
            .map(|r| r.id)
            .collect())
    };
    let frogs_of = |id: &str| -> Result<Vec<String>> {
        Ok(crate::Frogs::new(&ann)
            .of_note(id)?
            .into_iter()
            .map(|f| f.id)
            .collect())
    };
    assert_eq!(routines_of(&into.id)?, std::slice::from_ref(&routine.id));
    assert_eq!(frogs_of(&into.id)?, std::slice::from_ref(&frog.id));

    let mark = ann.current_seq()?;
    assert!(notes.may_undo_merge(&keys.id)?);
    notes.undo_merge(&keys.id)?;
    notes.undo_merge(&loose.id)?;
    let written: Vec<_> = ann
        .entries_after(mark)?
        .into_iter()
        .map(|e| (e.entity, e.field, e.value))
        .collect();
    assert_eq!(
        written,
        [
            (keys.id.clone(), keys::MERGED.to_string(), None),
            (loose.id.clone(), keys::MERGED.to_string(), None),
        ]
    );
    assert_eq!(
        ids(notes.all()?),
        [into.id.clone(), loose.id.clone(), keys.id.clone()]
    );
    assert_eq!(ids(notes.inside(&home)?), std::slice::from_ref(&keys.id));
    assert_eq!(ids(notes.unsorted()?), [loose.id.clone(), into.id.clone()]);
    let back = note(&ann, &keys.id)?;
    assert_eq!(back.merged_into, None);
    assert_eq!(back.place.as_deref(), Some(home.as_str()));
    assert!(notes.sections(&into.id)?.is_empty());
    let sections: Vec<String> = notes
        .sections(&keys.id)?
        .into_iter()
        .map(|s| s.id)
        .collect();
    assert_eq!(sections, [amend.id]);
    assert_eq!(routines_of(&keys.id)?, [routine.id]);
    assert!(routines_of(&into.id)?.is_empty());
    assert_eq!(frogs_of(&keys.id)?, [frog.id]);
    assert!(frogs_of(&into.id)?.is_empty());
    // Nothing is left to undo.
    assert!(!notes.may_undo_merge(&keys.id)?);
    assert!(notes.undo_merge(&keys.id).is_err());
    Ok(())
}

#[test]
fn undoing_a_merge_is_refused_where_nothing_shows_merged_and_writes_nothing() -> TestResult {
    let ann = Devices::new().device(1, "Ann")?;
    let notes = Notes::new(&ann);
    let alone = notes.capture("alone", None, None)?;
    let stranger = notes.capture("merged into a stranger", None, None)?;
    ann.append(&stranger.id, keys::MERGED, Some("note:never-synced"), None)?;
    let mark = ann.current_seq()?;
    for id in [alone.id.as_str(), &stranger.id, "note:none", "section:x"] {
        assert!(!notes.may_undo_merge(id)?, "{id}");
        assert!(notes.undo_merge(id).is_err(), "{id}");
    }
    assert_eq!(ann.current_seq()?, mark);
    Ok(())
}

#[test]
fn undoing_a_merge_of_a_chain_takes_out_only_the_one_note_and_what_is_in_it() -> TestResult {
    let ann = Devices::new().device(1, "Ann")?;
    let notes = Notes::new(&ann);
    let a = notes.capture("a", None, None)?;
    let b = notes.capture("b", None, None)?;
    let c = notes.capture("c", None, None)?;
    notes.merge(&a.id, &b.id)?;
    notes.merge(&b.id, &c.id)?;
    notes.undo_merge(&b.id)?;
    assert_eq!(ids(notes.all()?), [c.id.clone(), b.id.clone()]);
    assert_eq!(merged_into(&ann, &a.id)?, Some(b.id.clone()));
    let sections: Vec<String> = notes.sections(&b.id)?.into_iter().map(|s| s.id).collect();
    assert_eq!(sections, [a.id]);
    assert!(notes.sections(&c.id)?.is_empty());
    Ok(())
}

#[test]
fn undoing_a_merge_of_two_notes_merged_into_each_other_leaves_the_other() -> TestResult {
    let ann = Devices::new().device(1, "Ann")?;
    let notes = Notes::new(&ann);
    let x = notes.capture("x", None, None)?;
    let y = notes.capture("y", None, None)?;
    // Both merges arrive, as after a sync of two Devices.
    ann.append(&x.id, keys::MERGED, Some(&y.id), None)?;
    ann.append(&y.id, keys::MERGED, Some(&x.id), None)?;
    let (low, high) = if x.id < y.id { (x, y) } else { (y, x) };
    assert!(!notes.may_undo_merge(&low.id)?);
    notes.undo_merge(&high.id)?;
    assert_eq!(ids(notes.all()?), std::slice::from_ref(&high.id));
    assert_eq!(merged_into(&ann, &low.id)?, Some(high.id));
    Ok(())
}

#[test]
fn an_undone_merge_reaches_the_users_other_device_and_a_later_merge_wins() -> TestResult {
    let devices = Devices::new();
    let ann = devices.device(1, "Ann")?;
    let desk = devices.device(3, "Ann")?;
    let notes = Notes::new(&ann);
    let a = notes.capture("a", None, None)?;
    let b = notes.capture("b", None, None)?;
    notes.merge(&a.id, &b.id)?;
    exchange(&desk, &ann, None)?;
    Notes::new(&desk).undo_merge(&a.id)?;
    exchange(&desk, &ann, None)?;
    for store in [&ann, &desk] {
        assert_eq!(merged_into(store, &a.id)?, None);
    }
    notes.merge(&a.id, &b.id)?;
    exchange(&desk, &ann, None)?;
    for store in [&ann, &desk] {
        assert_eq!(merged_into(store, &a.id)?, Some(b.id.clone()));
    }
    Ok(())
}

#[test]
fn a_member_undoes_the_merge_of_their_own_note_a_manager_anyones() -> TestResult {
    let devices = Devices::new();
    let ann = devices.device(1, "Ann")?;
    let ben = devices.device(2, "Ben")?;
    let (area, _) = household(&ann, &ben)?;
    let into = Notes::new(&ann).capture("Toner", Some(&area), None)?;
    let own = Notes::new(&ben).capture("Ben's toner", Some(&area), None)?;
    exchange(&ben, &ann, None)?;
    Notes::new(&ben).merge(&own.id, &into.id)?;
    assert!(Notes::new(&ben).may_undo_merge(&own.id)?);
    Notes::new(&ben).undo_merge(&own.id)?;
    exchange(&ben, &ann, None)?;
    for store in [&ann, &ben] {
        assert_eq!(merged_into(store, &own.id)?, None);
    }
    Notes::new(&ben).merge(&own.id, &into.id)?;
    exchange(&ben, &ann, None)?;
    assert!(Notes::new(&ann).may_undo_merge(&own.id)?);
    Notes::new(&ann).undo_merge(&own.id)?;
    exchange(&ben, &ann, None)?;
    for store in [&ann, &ben] {
        assert_eq!(merged_into(store, &own.id)?, None);
    }
    Ok(())
}

#[test]
fn a_members_undo_of_anothers_merge_is_refused_and_ignored() -> TestResult {
    let devices = Devices::new();
    let ann = devices.device(1, "Ann")?;
    let ben = devices.device(2, "Ben")?;
    let (area, _) = household(&ann, &ben)?;
    let into = Notes::new(&ann).capture("Toner", Some(&area), None)?;
    let merged = Notes::new(&ann).capture("Order more", Some(&area), None)?;
    Notes::new(&ann).merge(&merged.id, &into.id)?;
    exchange(&ben, &ann, None)?;
    let mark = ben.current_seq()?;
    assert!(!Notes::new(&ben).may_undo_merge(&merged.id)?);
    assert!(Notes::new(&ben).undo_merge(&merged.id).is_err());
    assert_eq!(ben.current_seq()?, mark);
    // As a modified app would write it.
    ben.append(&merged.id, keys::MERGED, None, None)?;
    exchange(&ben, &ann, None)?;
    for store in [&ann, &ben] {
        assert_eq!(merged_into(store, &merged.id)?, Some(into.id.clone()));
    }
    Ok(())
}

#[test]
fn several_notes_merge_into_one_in_one_batch_and_come_back_together() -> TestResult {
    let ann = Devices::new().device(1, "Ann")?;
    let notes = Notes::new(&ann);
    let home = Places::new(&ann).add_area("Home")?.id;
    let keys = notes.capture("Spare keys at the neighbours", Some(&home), None)?;
    let plants = notes.capture("Water the plants", None, None)?;
    let post = notes.capture("Empty the letter box", None, None)?;
    let into = notes.capture("Neighbours on holiday", None, None)?;
    let mark = ann.current_seq()?;
    let merged = notes.merge_all([&keys.id, &plants.id, &plants.id, &post.id], &into.id)?;
    assert_eq!(
        merged,
        [keys.id.clone(), plants.id.clone(), post.id.clone()]
    );
    let written: Vec<_> = ann
        .entries_after(mark)?
        .into_iter()
        .map(|e| (e.entity, e.field, e.value))
        .collect();
    let pointer = |id: &str| {
        (
            id.to_string(),
            keys::MERGED.to_string(),
            Some(into.id.clone()),
        )
    };
    assert_eq!(
        written,
        [pointer(&keys.id), pointer(&plants.id), pointer(&post.id)]
    );
    assert_eq!(ids(notes.all()?), std::slice::from_ref(&into.id));
    let sections: Vec<String> = notes
        .sections(&into.id)?
        .into_iter()
        .map(|s| s.id)
        .collect();
    assert_eq!(sections, merged);

    // Another step got to one of them first.
    notes.undo_merge(&plants.id)?;
    let mark = ann.current_seq()?;
    let undone = notes.undo_merge_all(merged.iter().chain([&"note:none".to_string()]))?;
    assert_eq!(undone, [keys.id.clone(), post.id.clone()]);
    let written: Vec<_> = ann
        .entries_after(mark)?
        .into_iter()
        .map(|e| (e.entity, e.field, e.value))
        .collect();
    assert_eq!(
        written,
        [
            (keys.id.clone(), keys::MERGED.to_string(), None),
            (post.id.clone(), keys::MERGED.to_string(), None),
        ]
    );
    assert_eq!(place_of(&ann, &keys.id)?.as_deref(), Some(home.as_str()));
    assert!(notes.sections(&into.id)?.is_empty());
    Ok(())
}

#[test]
fn merging_several_writes_nothing_when_one_note_or_the_target_is_refused() -> TestResult {
    let ann = Devices::new().device(1, "Ann")?;
    let notes = Notes::new(&ann);
    let a = notes.capture("a", None, None)?;
    let b = notes.capture("b", None, None)?;
    let into = notes.capture("into", None, None)?;
    let trashed = notes.capture("trashed", None, None)?;
    notes.trash(&trashed.id)?;
    let merged = notes.capture("merged", None, None)?;
    notes.merge(&merged.id, &into.id)?;
    let mark = ann.current_seq()?;
    let cases: [([&str; 2], &str); 6] = [
        ([&a.id, &trashed.id], &into.id),
        ([&a.id, &merged.id], &into.id),
        ([&a.id, "note:none"], &into.id),
        ([&a.id, &b.id], &trashed.id),
        ([&a.id, &b.id], &merged.id),
        ([&a.id, &into.id], &into.id),
    ];
    for (from, target) in cases {
        assert!(
            notes.merge_all(from, target).is_err(),
            "{from:?} into {target}"
        );
    }
    assert_eq!(ann.current_seq()?, mark);
    assert!(notes.merge_all(Vec::<String>::new(), &into.id)?.is_empty());
    assert_eq!(ann.current_seq()?, mark);
    Ok(())
}

#[test]
fn a_member_merges_several_of_their_own_notes_and_none_of_anothers() -> TestResult {
    let devices = Devices::new();
    let ann = devices.device(1, "Ann")?;
    let ben = devices.device(2, "Ben")?;
    let (area, _) = household(&ann, &ben)?;
    let into = Notes::new(&ann).capture("Toner", Some(&area), None)?;
    let anns = Notes::new(&ann).capture("Order more", Some(&area), None)?;
    let bens = Notes::new(&ben);
    let own = bens.capture("Ben's toner", Some(&area), None)?;
    let also = bens.capture("Ben's paper", Some(&area), None)?;
    exchange(&ben, &ann, None)?;
    let mark = ben.current_seq()?;
    assert!(bens.merge_all([&own.id, &anns.id], &into.id).is_err());
    assert_eq!(ben.current_seq()?, mark);
    let merged = bens.merge_all([&own.id, &also.id], &into.id)?;
    assert_eq!(merged, [own.id.clone(), also.id.clone()]);
    exchange(&ben, &ann, None)?;
    for id in [&own.id, &also.id] {
        assert_eq!(merged_into(&ann, id)?, Some(into.id.clone()));
    }
    assert_eq!(bens.undo_merge_all(&merged)?, merged);
    Ok(())
}

fn place_of(store: &Store, id: &str) -> Result<Option<String>> {
    Ok(Notes::new(store).by_id(id)?.and_then(|n| n.place))
}

#[test]
fn taking_a_note_out_of_its_place_sends_it_back_to_unsorted_with_one_entry() -> TestResult {
    let ann = Devices::new().device(1, "Ann")?;
    let notes = Notes::new(&ann);
    let home = Places::new(&ann).add_area("Home")?.id;
    let keys = notes.capture("Spare keys at the neighbours", None, None)?;
    notes.file(&keys.id, &home)?;
    assert!(notes.unsorted()?.is_empty());

    let mark = ann.current_seq()?;
    assert!(notes.may_unfile(&keys.id)?);
    notes.unfile(&keys.id)?;
    let written: Vec<_> = ann
        .entries_after(mark)?
        .into_iter()
        .map(|e| (e.entity, e.field, e.value))
        .collect();
    assert_eq!(written, [(keys.id.clone(), keys::PLACE.to_string(), None)]);
    assert_eq!(place_of(&ann, &keys.id)?, None);
    assert_eq!(ids(notes.unsorted()?), std::slice::from_ref(&keys.id));
    assert!(notes.inside(&home)?.is_empty());
    // Nothing is left to take out.
    assert!(!notes.may_unfile(&keys.id)?);
    assert!(notes.unfile(&keys.id).is_err());
    Ok(())
}

#[test]
fn taking_a_note_out_of_its_place_leaves_one_that_spawned_a_routine_out_of_unsorted() -> TestResult
{
    use crate::schedule::{ClockTime, Day, Frequency, Recurrence, Schedule};
    let ann = Devices::new().device(1, "Ann")?;
    let notes = Notes::new(&ann);
    let home = Places::new(&ann).add_area("Home")?.id;
    let bins = notes.capture("Bins out", Some(&home), None)?;
    let eight = ClockTime::new(8, 0).into_iter().collect::<Vec<_>>();
    let rule = Recurrence::every(Frequency::Daily, 1);
    let daily = Schedule::new(Day::new(2026, 3, 1), &eight, Some(rule)).ok_or("a Schedule")?;
    crate::Routines::new(&ann).spawn(&bins.id, &daily)?;
    let mark = ann.current_seq()?;
    notes.unfile(&bins.id)?;
    let fields: Vec<_> = ann
        .entries_after(mark)?
        .into_iter()
        .map(|e| e.field)
        .collect();
    assert_eq!(fields, [keys::PLACE]);
    let back = note(&ann, &bins.id)?;
    assert_eq!(back.place, None);
    assert!(back.spawned);
    assert!(notes.unsorted()?.is_empty());
    assert_eq!(ids(notes.all()?), std::slice::from_ref(&bins.id));
    Ok(())
}

#[test]
fn taking_a_note_out_is_refused_where_nothing_shows_filed_and_writes_nothing() -> TestResult {
    let ann = Devices::new().device(1, "Ann")?;
    let notes = Notes::new(&ann);
    let places = Places::new(&ann);
    let loose = notes.capture("never filed", None, None)?;
    let gone = places.add_area("Gone")?.id;
    let stranded = notes.capture("filed where nothing is", Some(&gone), None)?;
    // The place deleted elsewhere, its Notes not moved here yet.
    ann.append(&gone, crate::places::keys::TYPE, None, None)?;
    let mark = ann.current_seq()?;
    for id in [loose.id.as_str(), &stranded.id, "note:none", &gone] {
        assert!(!notes.may_unfile(id)?, "{id}");
        assert!(notes.unfile(id).is_err(), "{id}");
    }
    assert_eq!(ann.current_seq()?, mark);
    Ok(())
}

#[test]
fn taking_a_note_out_reaches_the_users_other_device_and_a_later_filing_wins() -> TestResult {
    let devices = Devices::new();
    let ann = devices.device(1, "Ann")?;
    let desk = devices.device(3, "Ann")?;
    let notes = Notes::new(&ann);
    let home = Places::new(&ann).add_area("Home")?.id;
    let keys = notes.capture("Spare keys", Some(&home), None)?;
    exchange(&desk, &ann, None)?;
    Notes::new(&desk).unfile(&keys.id)?;
    exchange(&desk, &ann, None)?;
    for store in [&ann, &desk] {
        assert_eq!(
            ids(Notes::new(store).unsorted()?),
            std::slice::from_ref(&keys.id)
        );
    }
    notes.file(&keys.id, &home)?;
    exchange(&desk, &ann, None)?;
    for store in [&ann, &desk] {
        assert_eq!(place_of(store, &keys.id)?, Some(home.clone()));
    }
    Ok(())
}

#[test]
fn a_member_takes_out_their_own_note_a_manager_anyones() -> TestResult {
    let devices = Devices::new();
    let ann = devices.device(1, "Ann")?;
    let ben = devices.device(2, "Ben")?;
    let (area, _) = household(&ann, &ben)?;
    let own = Notes::new(&ben).capture("Ben's toner", Some(&area), None)?;
    let more = Notes::new(&ben).capture("Ben's stapler", Some(&area), None)?;
    exchange(&ben, &ann, None)?;
    assert!(Notes::new(&ben).may_unfile(&own.id)?);
    Notes::new(&ben).unfile(&own.id)?;
    assert_eq!(
        ids(Notes::new(&ben).unsorted()?),
        std::slice::from_ref(&own.id)
    );
    assert!(Notes::new(&ann).may_unfile(&more.id)?);
    Notes::new(&ann).unfile(&more.id)?;
    assert_eq!(
        ids(Notes::new(&ann).unsorted()?),
        std::slice::from_ref(&more.id)
    );
    // Out of the Area, neither leaves for the other person: each
    // stays filed there for them (ADR 0013).
    exchange(&ben, &ann, None)?;
    assert_eq!(place_of(&ann, &own.id)?, Some(area.clone()));
    assert_eq!(place_of(&ben, &more.id)?, Some(area));
    Ok(())
}

#[test]
fn a_members_try_on_anothers_note_is_refused_and_ignored() -> TestResult {
    let devices = Devices::new();
    let ann = devices.device(1, "Ann")?;
    let ben = devices.device(2, "Ben")?;
    let (area, _) = household(&ann, &ben)?;
    let theirs = Notes::new(&ann).capture("Order paper", Some(&area), None)?;
    exchange(&ben, &ann, None)?;
    let mark = ben.current_seq()?;
    assert!(!Notes::new(&ben).may_unfile(&theirs.id)?);
    assert!(Notes::new(&ben).unfile(&theirs.id).is_err());
    assert_eq!(ben.current_seq()?, mark);
    // As a modified app would write it.
    ben.append(&theirs.id, keys::PLACE, None, None)?;
    exchange(&ben, &ann, None)?;
    for store in [&ann, &ben] {
        assert_eq!(place_of(store, &theirs.id)?, Some(area.clone()));
        assert!(Notes::new(store).unsorted()?.is_empty());
    }
    Ok(())
}

// ------------------------------------------ attached files (ADR 0047)

fn new_file<'b>(name: &'b str, bytes: &'b [u8], pages: Option<u64>) -> NewFile<'b> {
    NewFile {
        name,
        media_type: "",
        bytes,
        pages,
    }
}

#[test]
fn a_file_is_attached_as_it_is_and_waits_to_be_read_where_its_kind_allows() -> TestResult {
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    let big = vec![b'a'; (file_text::PHONE_BYTES + 1) as usize];
    let note = notes.capture_with_files(
        "Papers",
        None,
        None,
        &[],
        &[],
        &[
            new_file("Lease.pdf", b"%PDF-1.4 lease", Some(2)),
            new_file("Scan.pdf", b"%PDF-1.4 scan", Some(40)),
            new_file("notes.md", b"# Notes", None),
            new_file("big.txt", &big, None),
            new_file("plan.dwg", b"dwg", None),
        ],
    )?;
    let names: Vec<&str> = note.attachments.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(
        names,
        ["Lease.pdf", "Scan.pdf", "notes.md", "big.txt", "plan.dwg"]
    );
    let computer: Vec<bool> = note.attachments.iter().map(|a| a.computer).collect();
    assert_eq!(computer, [false, true, false, true, false]);
    let lease = &note.attachments[0];
    assert_eq!(lease.media_type, "application/pdf");
    assert_eq!((lease.size, lease.pages), (14, Some(2)));
    assert_eq!(
        store
            .current(&note.id, &keys::file(&lease.hash))?
            .as_deref(),
        Some(r#"{"name":"Lease.pdf","pages":2,"size":14,"type":"application/pdf"}"#)
    );
    assert_eq!(
        notes.file_bytes(&lease.hash).as_deref(),
        Some(&b"%PDF-1.4 lease"[..])
    );
    assert!(notes.has_file(&lease.hash) && !notes.has_photo(&lease.hash));
    let hashes = |blobs: Vec<Blob>| -> Vec<String> { blobs.into_iter().map(|b| b.hash).collect() };
    let h = |i: usize| note.attachments[i].hash.clone();
    assert_eq!(hashes(notes.unread_attachments(false)?), [h(0), h(2)]);
    assert_eq!(
        hashes(notes.unread_attachments(true)?),
        [h(0), h(1), h(2), h(3)]
    );
    notes.set_attachment_text(&note.id, &h(0), "  Rent \r\n\r\n\r\n due ")?;
    notes.set_attachment_text(&note.id, &h(2), "")?;
    let read = notes.by_id(&note.id)?.ok_or("note")?;
    assert_eq!(
        read.attachment_texts.get(&h(0)).map(String::as_str),
        Some("Rent\n\n due")
    );
    assert_eq!(hashes(notes.unread_attachments(true)?), [h(1), h(3)]);
    assert!(
        notes
            .set_attachment_text(&note.id, &"0".repeat(64), "x")
            .is_err()
    );
    assert!(
        notes
            .attach_file("note:none", &new_file("a.txt", b"a", None))
            .is_err()
    );
    assert_eq!(read.alone_file(), None, "it has words and five files");
    // Neither waits in the trash.
    notes.trash(&note.id)?;
    assert!(notes.unread_attachments(true)?.is_empty());
    Ok(())
}

#[test]
fn a_note_made_from_a_file_alone_shows_the_file() -> TestResult {
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    let letter = notes.capture_with_files(
        "",
        None,
        None,
        &[],
        &[],
        &[new_file("Letter.odt", b"PK", None)],
    )?;
    assert_eq!(
        letter.alone_file().map(|a| a.name.as_str()),
        Some("Letter.odt")
    );
    let hash = notes.attach_file(&letter.id, &new_file("Other.ods", b"PK2", None))?;
    assert_eq!(notes.by_id(&letter.id)?.ok_or("note")?.alone_file(), None);
    let with_photo = notes.capture_with_files(
        "",
        None,
        None,
        &[photo(1)],
        &[],
        &[new_file("a.zip", b"z", None)],
    )?;
    assert_eq!(with_photo.alone_file(), None);
    let worded = notes.capture_with_files(
        "About it",
        None,
        None,
        &[],
        &[],
        &[new_file("a.zip", b"z", None)],
    )?;
    assert_eq!(worded.alone_file(), None);
    assert_eq!(
        notes.by_id(&letter.id)?.ok_or("note")?.attachments[1].hash,
        hash
    );
    Ok(())
}

#[test]
fn removing_a_files_text_keeps_the_file_and_removing_a_file_keeps_its_text() -> TestResult {
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    let note = notes.capture_with_files(
        "",
        None,
        None,
        &[],
        &[],
        &[
            new_file("Lease.txt", b"lease", None),
            new_file("Bill.txt", b"bill", None),
            new_file("x.zip", b"zip", None),
        ],
    )?;
    let [lease, bill, zip] = [0, 1, 2].map(|i| note.attachments[i].hash.clone());
    notes.set_attachment_text(&note.id, &lease, "Rent due")?;
    notes.set_attachment_text(&note.id, &bill, "Power bill")?;
    // Nothing would be left of a file without a text (ADR 0020).
    assert!(!notes.may_remove_attachment(&note.id, &zip)?);
    assert!(notes.remove_attachment(&note.id, &zip).is_err());
    notes.remove_attachment_text(&note.id, &bill)?;
    let read = notes.by_id(&note.id)?.ok_or("note")?;
    assert_eq!(read.removed_attachment_texts, vec![bill.clone()]);
    assert!(read.attachments.iter().any(|a| a.hash == bill));
    assert!(notes.has_file(&bill));
    assert!(
        !notes
            .unread_attachments(true)?
            .iter()
            .any(|b| b.hash == bill)
    );
    assert!(!notes.may_remove_attachment_text(&note.id, &bill)?);
    notes.remove_attachment(&note.id, &lease)?;
    let read = notes.by_id(&note.id)?.ok_or("note")?;
    assert!(!read.attachments.iter().any(|a| a.hash == lease));
    let removed: Vec<&str> = read
        .removed_attachments
        .iter()
        .map(|a| a.name.as_str())
        .collect();
    assert_eq!(removed, ["Lease.txt"]);
    assert_eq!(
        read.attachment_texts.get(&lease).map(String::as_str),
        Some("Rent due")
    );
    assert_eq!(notes.file_bytes(&lease), None, "its bytes go");
    assert!(!store.knows_blob(&format!("file/{lease}"))?);
    assert!(store.missing_blobs()?.is_empty());
    // Reading the text again ends its removal.
    notes.set_attachment_text(&note.id, &bill, "Power bill, March")?;
    let read = notes.by_id(&note.id)?.ok_or("note")?;
    assert!(read.removed_attachment_texts.is_empty());
    Ok(())
}

#[test]
fn what_another_device_wrote_of_a_file_is_read_as_data_only() {
    let hash = "a".repeat(64);
    let read = |value: &str| Attachment::read(&hash, value);
    let plain = read(r#"{"name":"../../.bashrc","size":5,"type":"Text/Plain; charset=utf-8"}"#);
    assert_eq!(
        (plain.name.as_str(), plain.media_type.as_str()),
        (".bashrc", "text/plain")
    );
    assert_eq!((plain.size, plain.pages, plain.computer), (5, None, false));
    let crafted = read(
        r#"{"computer":"yes","name":"inv\u202Efdp.exe\u0007","pages":1.0,"size":-3,"type":"<script>"}"#,
    );
    assert_eq!(crafted.name, "invfdp.exe");
    assert_eq!(crafted.media_type, UNKNOWN_TYPE);
    assert_eq!(
        (crafted.size, crafted.pages, crafted.computer),
        (0, None, false)
    );
    let huge = read(r#"{"name":"..","pages":9007199254740993,"size":9007199254740992}"#);
    assert_eq!(
        (huge.name.as_str(), huge.pages, huge.size),
        ("", None, 1 << 53)
    );
    for garbled in ["", "null", "[1]", "\"name\"", "{\"name\":3}"] {
        let a = read(garbled);
        assert_eq!(
            (a.name.as_str(), a.media_type.as_str(), a.size),
            ("", UNKNOWN_TYPE, 0)
        );
    }
    assert_eq!(clean_name("C:\\Users\\ann\\ Brief.pdf "), "Brief.pdf");
    assert_eq!(clean_name(&"ä".repeat(300)).chars().count(), 255);
    assert_eq!(
        clean_type("application/vnd.oasis.opendocument.text"),
        "application/vnd.oasis.opendocument.text"
    );
    assert_eq!(clean_type("text"), UNKNOWN_TYPE);
    assert_eq!(clean_type("tëxt/plain"), UNKNOWN_TYPE);
}

#[test]
fn a_file_goes_only_to_the_people_of_its_area() -> TestResult {
    let devices = Devices::new();
    let ann = devices.device(1, "Ann")?;
    let ben = devices.device(2, "Ben")?;
    let (area, private) = household(&ann, &ben)?;
    let notes = Notes::new(&ann);
    let inside = notes.capture("In", Some(&area), None)?;
    let outside = notes.capture("Out", Some(&private), None)?;
    let lease = notes.attach_file(&inside.id, &new_file("Lease.pdf", b"lease", None))?;
    let bill = notes.attach_file(&outside.id, &new_file("Bill.pdf", b"bill", None))?;
    let areas = ann.areas_shared_with(&ben.device_id()?)?;
    assert!(ann.blob_in(&format!("file/{lease}"), &areas)?);
    assert!(!ann.blob_in(&format!("file/{bill}"), &areas)?);
    Ok(())
}

#[test]
fn the_local_network_carries_a_file_as_large_as_a_bundle_does() {
    assert_eq!(
        crate::lan::MAX_BLOB_BYTES as u64,
        crate::bundle::MAX_BLOB_BYTES
    );
    assert_eq!(crate::lan::MAX_BLOB_BYTES as u64, MAX_FILE_BYTES);
}

#[test]
fn a_file_and_its_text_travel_to_the_other_device() -> TestResult {
    let devices = Devices::new();
    let phone = devices.device(1, "Ann")?;
    let desk = devices.device(3, "Ann")?;
    exchange(&desk, &phone, None)?;
    let scan = Notes::new(&phone).capture_with_files(
        "",
        None,
        None,
        &[],
        &[],
        &[new_file("Scan.pdf", b"%PDF-1.4 thick", Some(30))],
    )?;
    let hash = scan.attachments[0].hash.clone();
    assert!(
        Notes::new(&phone).unread_attachments(false)?.is_empty(),
        "left to the computer"
    );
    exchange(&desk, &phone, None)?;
    assert_eq!(desk.missing_blobs()?, Vec::<String>::new());
    assert_eq!(
        Notes::new(&desk).file_bytes(&hash).as_deref(),
        Some(&b"%PDF-1.4 thick"[..])
    );
    let waiting = Notes::new(&desk).unread_attachments(true)?;
    let waiting: Vec<&str> = waiting.iter().map(|b| b.hash.as_str()).collect();
    assert_eq!(waiting, [hash.as_str()]);
    Notes::new(&desk).set_attachment_text(&scan.id, &hash, "Page one\n\nPage two")?;
    exchange(&desk, &phone, None)?;
    let read = note(&phone, &scan.id)?;
    assert_eq!(
        read.attachment_texts.get(&hash).map(String::as_str),
        Some("Page one\n\nPage two")
    );
    Ok(())
}

#[test]
fn a_members_machine_gives_anothers_file_its_text_and_only_the_owner_removes_it() -> TestResult {
    let devices = Devices::new();
    let ann = devices.device(1, "Ann")?;
    let ben = devices.device(2, "Ben")?;
    let (area, _) = household(&ann, &ben)?;
    let lease = Notes::new(&ann).capture_with_files(
        "Lease",
        Some(&area),
        None,
        &[],
        &[],
        &[
            new_file("Lease.docx", b"PK lease", None),
            new_file("Bill.txt", b"bill", None),
        ],
    )?;
    let [docx, bill] = [0, 1].map(|i| lease.attachments[i].hash.clone());
    Notes::new(&ann).set_attachment_text(&lease.id, &bill, "Power bill")?;
    exchange(&ben, &ann, None)?;
    // Ben's machine reads the file for Ann's Note.
    Notes::new(&ben).set_attachment_text(&lease.id, &docx, "Rent due")?;
    assert!(!Notes::new(&ben).may_remove_attachment_text(&lease.id, &bill)?);
    // As a modified app would write them: a removal and a file of his.
    ben.append(&lease.id, &keys::file_text(&bill), None, None)?;
    ben.append(
        &lease.id,
        &keys::file(&"b".repeat(64)),
        Some(r#"{"name":"x.exe","size":1,"type":"application/x"}"#),
        None,
    )?;
    exchange(&ben, &ann, None)?;
    for store in [&ann, &ben] {
        let read = note(store, &lease.id)?;
        assert_eq!(
            read.attachment_texts.get(&docx).map(String::as_str),
            Some("Rent due")
        );
        assert_eq!(
            read.attachment_texts.get(&bill).map(String::as_str),
            Some("Power bill")
        );
        assert!(read.removed_attachment_texts.is_empty());
        assert_eq!(read.attachments.len(), 2, "his file on her Note is ignored");
    }
    Ok(())
}
