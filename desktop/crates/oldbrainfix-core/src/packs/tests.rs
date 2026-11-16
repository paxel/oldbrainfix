use super::*;
use crate::people::PersonFilter;
use crate::store::tests::ticking;
use crate::{ClockTime, OwnProfile, Recurrence};

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;
type Made<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn wall(at: DateTime<Utc>) -> chrono::NaiveDateTime {
    at.naive_utc()
}

/// A Device of its own person named `name`.
fn device(seed: u8, name: &str) -> Made<Store> {
    let store = Store::in_memory(ticking(), move || Ok(SigningKey::from_seed([seed; 32])))?;
    OwnProfile::new(&store).apply_defaults(name, None)?;
    OwnProfile::new(&store).rename(name)?;
    Ok(store)
}

fn own_id(store: &Store) -> Made<String> {
    Ok(OwnProfile::new(store).get()?.ok_or("a Profile")?.id)
}

fn bytes(seed: usize, length: usize) -> Vec<u8> {
    (0..length).map(|i| ((seed + i) % 251) as u8).collect()
}

/// The payload of the Pack file `bytes`, as a text.
fn payload_of(bytes: &[u8]) -> Made<String> {
    let json: Value = serde_json::from_slice(bytes)?;
    Ok(json["pack"].as_str().ok_or("a payload")?.to_string())
}

/// Ann's Health: a Note of the Area itself, and in its Topic Running
/// a Tally, a Session picking it and a Tally of her Garden, a
/// Checklist and a Note with a photo, a recording and their texts.
struct Health {
    area: String,
    running: String,
    plain: String,
    tally: String,
    session: String,
    checklist: String,
    media: String,
}

fn health(ann: &Store) -> Made<Health> {
    let places = Places::new(ann);
    let notes = Notes::new(ann);
    let area = places.add_area("Health")?;
    let running = places.add_topic(&area.id, "Running")?;
    let garden_area = places.add_area("Garden")?;
    let plain = notes.capture("Drink water", Some(&area.id), None)?;
    let tally = notes.capture("Laps", Some(&running.id), None)?;
    Tallies::new(ann).add_field(&tally.id, "Laps", "", Some(10.0), None)?;
    let garden = notes.capture("Digging", Some(&garden_area.id), None)?;
    Tallies::new(ann).add_field(&garden.id, "Minutes", "min", None, None)?;
    let session = notes.capture("Training", Some(&running.id), None)?;
    Sessions::new(ann).make(
        &session.id,
        &[tally.id.clone(), garden.id.clone()],
        1,
        SessionRule::Rolling,
    )?;
    let checklist = notes.capture("Bag\nShoes\nBottle", Some(&running.id), None)?;
    Checklists::new(ann).make(&checklist.id)?;
    let media = notes.capture_with(
        "Route",
        Some(&running.id),
        None,
        &[bytes(1, 300)],
        &[bytes(2, 600)],
    )?;
    let m = notes.by_id(&media.id)?.ok_or("the Note")?;
    notes.set_photo_text(&media.id, &m.photos[0], "Map text")?;
    notes.set_transcript(&media.id, &m.recordings[0], "Go left")?;
    Ok(Health {
        area: area.id,
        running: running.id,
        plain: plain.id,
        tally: tally.id,
        session: session.id,
        checklist: checklist.id,
        media: media.id,
    })
}

/// The Note of `pack` whose text starts with `text`.
fn index_of(pack: &Pack, text: &str) -> Made<usize> {
    Ok(pack
        .notes
        .iter()
        .position(|n| n.text.starts_with(text))
        .ok_or("the Note")?)
}

fn opened(store: &Store, bytes: &[u8]) -> Made<OpenedPack> {
    Ok(store
        .open_pack(bytes)?
        .map_err(|e| format!("not opened: {}", e.name()))?)
}

fn add(store: &Store, opened: &OpenedPack, claim: bool) -> Result<PackAdded> {
    store.add_pack(
        opened,
        &opened.pack.all_keys(),
        claim,
        &wall,
        "Pack",
        "From a Pack",
    )
}

fn refused(store: &Store, bytes: &[u8]) -> Made<Option<PackError>> {
    Ok(store.open_pack(bytes)?.err())
}

fn texts(store: &Store) -> Made<Vec<Note>> {
    Ok(Notes::new(store).all()?)
}

#[test]
fn the_tree_of_an_area_holds_its_notes_then_its_topics_and_a_tally_from_outside_under_its_session()
-> TestResult {
    let ann = device(1, "Ann")?;
    let h = health(&ann)?;
    let pack = ann.pack_of(&h.area, &HashSet::new())?;
    let tree = pack.tree();
    assert_eq!(tree.len(), 1);
    let area = &tree[0];
    assert_eq!(area.kind, PackItemKind::Area);
    assert_eq!(pack.areas[0].name, "Health");
    let kinds: Vec<_> = area.children.iter().map(|i| i.kind).collect();
    assert_eq!(kinds, [PackItemKind::Note, PackItemKind::Topic]);
    let text = |i: &PackItem| i.note.map(|n| pack.notes[n].text.clone());
    assert_eq!(text(&area.children[0]).as_deref(), Some("Drink water"));
    let topic = &area.children[1];
    let in_topic: Vec<_> = topic.children.iter().filter_map(text).collect();
    for t in ["Laps", "Training", "Route"] {
        assert!(in_topic.iter().any(|n| n.starts_with(t)), "{t}");
    }
    assert!(!in_topic.iter().any(|n| n == "Digging"));
    let session = topic
        .children
        .iter()
        .find(|i| text(i).as_deref() == Some("Training"))
        .ok_or("the Session")?;
    let under: Vec<_> = session.children.iter().filter_map(text).collect();
    assert_eq!(under, ["Digging"]);
    let garden = &pack.notes[index_of(&pack, "Digging")?];
    assert!(garden.outside);
    assert_eq!(pack.areas[garden.area.ok_or("an Area")?].name, "Garden");

    // From a Topic only its Notes; from a Note that Note and the
    // Tallies its Session picks from.
    let of_topic = ann.pack_of(&h.running, &HashSet::new())?;
    assert!(!of_topic.notes.iter().any(|n| n.text == "Drink water"));
    let of_note = ann.pack_of(&h.session, &HashSet::new())?;
    let all: Vec<_> = of_note.notes.iter().map(|n| n.text.as_str()).collect();
    assert_eq!(all, ["Training", "Laps", "Digging"]);
    assert!(of_note.notes[1..].iter().all(|n| n.outside));
    Ok(())
}

#[test]
fn the_pack_of_everything_follows_the_persons_order_and_the_name_shown() -> TestResult {
    let ann = device(1, "Ann")?;
    let h = health(&ann)?;
    PersonSettings::new(&ann).move_area(&h.area, 0)?;
    let places = Places::new(&ann);
    let notes = Notes::new(&ann);
    notes.capture("Boxes", Some(&places.add_area("attic")?.id), None)?;
    notes.capture("Tyres", Some(&places.add_area("Bike")?.id), None)?;
    let names =
        |pack: &Pack| -> Vec<String> { pack.areas.iter().map(|a| a.name.clone()).collect() };
    let pack = ann.pack_of_all(&HashSet::new())?;
    assert_eq!(names(&pack), ["Health", "Garden", "attic", "Bike"]);
    let shown = ann.pack_of_all_by(&HashSet::new(), |a| {
        if a.name == "attic" {
            "Loft".to_string()
        } else {
            a.name.clone()
        }
    })?;
    assert_eq!(names(&shown), ["Health", "Garden", "Bike", "attic"]);
    Ok(())
}

#[test]
fn the_pack_of_everything_holds_every_open_area_by_name_then_the_notes_without_one() -> TestResult {
    let ann = device(1, "Ann")?;
    let h = health(&ann)?;
    let notes = Notes::new(&ann);
    notes.capture("Loose thought", None, None)?;
    // Filed by another Device into a place deleted here meanwhile.
    let stray = notes.capture("Stray", None, None)?;
    ann.append(&stray.id, note_keys::PLACE, Some("gone"), None)?;
    let trashed = notes.capture("Gone", Some(&h.area), None)?;
    notes.trash(&trashed.id)?;
    let pack = ann.pack_of_all(&HashSet::new())?;
    let areas: Vec<_> = pack.areas.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(areas, ["Garden", "Health"]);
    let tree = pack.tree();
    let kinds: Vec<_> = tree.iter().map(|i| i.kind).collect();
    assert_eq!(
        kinds,
        [
            PackItemKind::Area,
            PackItemKind::Area,
            PackItemKind::Note,
            PackItemKind::Note
        ]
    );
    let text = |i: &PackItem| i.note.map(|n| pack.notes[n].text.clone());
    let loose: Vec<_> = tree[2..].iter().filter_map(text).collect();
    assert_eq!(loose, ["Stray", "Loose thought"]);
    assert!(!pack.notes.iter().any(|n| n.text == "Gone"));
    assert!(!pack.notes.iter().any(|n| n.outside));
    assert_eq!(pack.notes[0].text, "Digging");
    assert_eq!(pack.notes.len(), 8);

    let garden = Places::new(&ann)
        .areas()?
        .into_iter()
        .find(|a| a.name == "Garden")
        .ok_or("Garden")?;
    let closed = ann.pack_of_all(&HashSet::from([garden.id]))?;
    let areas: Vec<_> = closed.areas.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(areas, ["Health"]);
    assert!(!closed.notes.iter().any(|n| n.text == "Digging"));
    let session = &closed.notes[index_of(&closed, "Training")?];
    let pool = session.session.as_ref().map(|s| s.pool.clone());
    assert_eq!(pool, Some(vec![index_of(&closed, "Laps")?]));
    Ok(())
}

#[test]
fn a_tally_of_a_closed_area_never_goes_along_with_a_session() -> TestResult {
    let ann = device(1, "Ann")?;
    let h = health(&ann)?;
    let garden = Places::new(&ann)
        .areas()?
        .into_iter()
        .find(|a| a.name == "Garden")
        .ok_or("Garden")?;
    let pack = ann.pack_of(&h.session, &HashSet::from([garden.id]))?;
    let all: Vec<_> = pack.notes.iter().map(|n| n.text.as_str()).collect();
    assert_eq!(all, ["Training", "Laps"]);
    let pool = pack.notes[0].session.as_ref().map(|s| s.pool.clone());
    assert_eq!(pool, Some(vec![1]));
    let areas: Vec<_> = pack.areas.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(areas, ["Health"]);
    Ok(())
}

#[test]
fn everything_is_ticked_but_reminders_and_files_and_the_size_follows_the_ticks() -> TestResult {
    let ann = device(1, "Ann")?;
    let h = health(&ann)?;
    let routines = Routines::new(&ann);
    let eight = ClockTime::new(8, 0).ok_or("a time")?;
    let daily = Recurrence::parse("FREQ=DAILY")?;
    let schedule = Schedule::new(Day::new(2026, 3, 2), &[eight], Some(daily));
    routines.spawn(&h.plain, &schedule.ok_or("a schedule")?)?;
    let once = Schedule::new(Day::new(2026, 3, 9), &[eight], None);
    routines.spawn(&h.plain, &once.ok_or("a schedule")?)?;
    let pack = ann.pack_of(&h.area, &HashSet::new())?;
    let ticks = pack.export_ticks();
    let p = index_of(&pack, "Drink water")?;
    let m = index_of(&pack, "Route")?;
    let schedules = &pack.notes[p].routines;
    let daily = schedules.iter().position(|r| !r.once()).ok_or("daily")?;
    let once = schedules.iter().position(Schedule::once).ok_or("once")?;
    assert!(ticks.contains(&item_keys::routine(p, daily)));
    assert!(!ticks.contains(&item_keys::routine(p, once)));
    assert!(!ticks.contains(&item_keys::photo(m, 0)));
    assert!(!ticks.contains(&item_keys::recording(m, 0)));
    assert!(ticks.contains(&item_keys::photo_text(m, 0)));
    assert!(ticks.contains(&item_keys::transcript(m, 0)));
    let before = pack.size(&ticks);
    let mut with_photo = ticks.clone();
    with_photo.insert(item_keys::photo(m, 0));
    assert_eq!(pack.size(&with_photo) - before, 400);
    assert!(pack.size(&pack.all_keys()) > pack.size(&with_photo));
    assert_eq!(pack.size(&PackTicks::new()), 0);
    // A file of a Note not ticked counts for nothing.
    let without = pack.tick(&ticks, &item_keys::note(m), false);
    let mut photo_alone = without.clone();
    photo_alone.insert(item_keys::photo(m, 0));
    assert_eq!(pack.size(&photo_alone), pack.size(&without));
    Ok(())
}

#[test]
fn ticking_a_session_ticks_its_pool_and_unticking_its_last_tally_unticks_it() -> TestResult {
    let ann = device(1, "Ann")?;
    let h = health(&ann)?;
    let pack = ann.pack_of(&h.session, &HashSet::new())?;
    let (s, laps, digging) = (item_keys::note(0), item_keys::note(1), item_keys::note(2));
    let mut ticks = pack.tick(&PackTicks::new(), &s, true);
    assert!(ticks.contains(&laps) && ticks.contains(&digging));
    ticks = pack.tick(&ticks, &laps, false);
    assert!(ticks.contains(&s));
    let picked = pack.picked(&ticks);
    assert_eq!(
        picked.notes[0].session.as_ref().map(|s| s.pool.clone()),
        Some(vec![1])
    );
    ticks = pack.tick(&ticks, &digging, false);
    assert!(!ticks.contains(&s));
    // A Session ticked without any Tally of its pool still stays out.
    let forced = PackTicks::from([s.clone()]);
    assert!(pack.picked(&forced).notes.is_empty());

    // The same in the tree of a Pack opened: nothing is added.
    let ben = device(2, "Ben")?;
    let open = opened(&ben, &ann.write_pack(&pack, &pack.all_keys(), false)?)?;
    let mut ticks = open.pack.all_keys();
    ticks = open.pack.tick(&ticks, &laps, false);
    ticks = open.pack.tick(&ticks, &digging, false);
    assert!(!ticks.contains(&s));
    ticks.insert(s);
    let added = ben.add_pack(&open, &ticks, true, &wall, "Pack", "From a Pack")?;
    assert_eq!(added.notes, 0);
    assert!(texts(&ben)?.is_empty());
    Ok(())
}

#[test]
fn a_note_brings_no_entry_check_off_capture_context_or_eaten_frog() -> TestResult {
    let ann = device(1, "Ann")?;
    let h = health(&ann)?;
    let tallies = Tallies::new(&ann);
    let field = tallies.by_note(&h.tally)?.ok_or("a Tally")?.fields[0]
        .id
        .clone();
    tallies.add(
        &h.tally,
        &BTreeMap::from([(field, 7777.0)]),
        "secret entry line",
        None,
    )?;
    let checklists = Checklists::new(&ann);
    let first = checklists.by_note(&h.checklist)?.ok_or("a list")?.lines[0]
        .id
        .clone();
    checklists.check(&first)?;
    ann.append(&h.plain, note_keys::LOCATION, Some("52.52,13.405"), None)?;
    let notes = Notes::new(&ann);
    let eaten = notes.capture("Eaten frog", Some(&h.area), None)?;
    let frog = Frogs::new(&ann).spawn(&eaten.id)?;
    Frogs::new(&ann).mark_done(&frog.id, None)?;
    let open = notes.capture("Open frog", Some(&h.area), None)?;
    Frogs::new(&ann).spawn(&open.id)?;
    let pack = ann.pack_of(&h.area, &HashSet::new())?;
    let payload = payload_of(&ann.write_pack(&pack, &pack.all_keys(), false)?)?;
    for leak in ["7777", "secret entry line", "52.52", "\"checked\""] {
        assert!(!payload.contains(leak), "{leak}");
    }
    assert!(payload.contains("\"text\":\"Shoes\""));
    let json: Value = serde_json::from_str(&payload)?;
    let frogs: Vec<_> = json["notes"]
        .as_array()
        .ok_or("notes")?
        .iter()
        .filter(|n| n["frog"] == json!(true))
        .map(|n| n["text"].clone())
        .collect();
    assert_eq!(frogs, [json!("Open frog")]);
    Ok(())
}

#[test]
fn a_photo_and_a_recording_go_only_when_ticked_and_their_texts_without_them() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let h = health(&ann)?;
    let pack = ann.pack_of(&h.media, &HashSet::new())?;
    let ticks = pack.export_ticks();
    let open = opened(&ben, &ann.write_pack(&pack, &ticks, false)?)?;
    let note = &open.pack.notes[0];
    assert!(!note.photos[0].has_file);
    assert_eq!(note.photos[0].text.as_deref(), Some("Map text"));
    assert!(!note.recordings[0].has_file);
    assert_eq!(note.recordings[0].text.as_deref(), Some("Go left"));
    add(&ben, &open, true)?;
    let added = texts(&ben)?.into_iter().next().ok_or("the Note")?;
    assert!(added.photos.is_empty());
    assert_eq!(added.removed_photos.len(), 1);
    assert_eq!(added.photo_texts.values().collect::<Vec<_>>(), ["Map text"]);
    assert_eq!(added.transcripts.values().collect::<Vec<_>>(), ["Go left"]);

    let mut whole_ticks = ticks.clone();
    whole_ticks.insert(item_keys::photo(0, 0));
    whole_ticks.insert(item_keys::recording(0, 0));
    whole_ticks.remove(&item_keys::transcript(0, 0));
    let whole = opened(&ben, &ann.write_pack(&pack, &whole_ticks, false)?)?;
    assert!(whole.pack.notes[0].photos[0].bytes.is_some());
    assert_eq!(whole.pack.notes[0].recordings[0].text, None);
    add(&ben, &whole, true)?;
    let second = texts(&ben)?
        .into_iter()
        .find(|n| !n.photos.is_empty())
        .ok_or("the Note with its photo")?;
    assert!(ben.photo(&second.photos[0]).is_some());
    assert_eq!(ben.audio(&second.recordings[0]), Some(bytes(2, 600)));
    assert!(second.transcripts.is_empty());
    Ok(())
}

#[test]
fn an_attached_file_goes_with_its_name_only_when_ticked_and_its_text_without_it() -> TestResult {
    use crate::notes::NewFile;
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let papers = Places::new(&ann).add_area("Papers")?.id;
    let notes = Notes::new(&ann);
    let lease = notes.capture_with_files(
        "Lease",
        Some(&papers),
        None,
        &[],
        &[],
        &[
            NewFile {
                name: "Lease.pdf",
                media_type: "application/pdf",
                bytes: b"%PDF-1.4 lease",
                pages: Some(12),
            },
            NewFile {
                name: "plan.dwg",
                media_type: "",
                bytes: b"dwg",
                pages: None,
            },
        ],
    )?;
    let pdf = lease.attachments[0].hash.clone();
    notes.set_attachment_text(&lease.id, &pdf, "Rent due")?;
    let pack = ann.pack_of(&lease.id, &HashSet::new())?;
    let kinds: Vec<PackItemKind> = pack.tree()[0].children[0]
        .children
        .iter()
        .map(|c| c.kind)
        .collect();
    assert_eq!(
        kinds,
        [
            PackItemKind::File,
            PackItemKind::FileText,
            PackItemKind::File
        ]
    );
    let ticks = pack.export_ticks();
    assert!(ticks.contains(&item_keys::file_text(0, 0)));
    assert!(!ticks.contains(&item_keys::file(0, 0)));
    let open = opened(&ben, &ann.write_pack(&pack, &ticks, false)?)?;
    let files = &open.pack.notes[0].attachments;
    assert_eq!(files.len(), 1, "the file without a text stays out");
    assert!(!files[0].has_file);
    assert_eq!(files[0].text.as_deref(), Some("Rent due"));
    add(&ben, &open, true)?;
    let added = texts(&ben)?.into_iter().next().ok_or("the Note")?;
    assert!(added.attachments.is_empty());
    assert_eq!(added.removed_attachments.len(), 1);
    assert_eq!(
        added.attachment_texts.values().collect::<Vec<_>>(),
        ["Rent due"]
    );

    let whole = opened(&ben, &ann.write_pack(&pack, &pack.all_keys(), false)?)?;
    add(&ben, &whole, true)?;
    let second = texts(&ben)?
        .into_iter()
        .find(|n| !n.attachments.is_empty())
        .ok_or("the Note with its files")?;
    let names: Vec<&str> = second.attachments.iter().map(|a| a.name.as_str()).collect();
    assert_eq!(names, ["Lease.pdf", "plan.dwg"]);
    assert_eq!(second.attachments[0].pages, Some(12));
    assert!(second.attachments[0].computer, "more than ten pages");
    assert_eq!(second.attachments[1].media_type, "application/octet-stream");
    assert_eq!(ben.file(&pdf).as_deref(), Some(&b"%PDF-1.4 lease"[..]));
    assert_eq!(
        second.attachment_texts.get(&pdf).map(String::as_str),
        Some("Rent due")
    );
    Ok(())
}

#[test]
fn a_checklist_arrives_unticked_a_tally_without_entries_and_a_session_with_its_pool() -> TestResult
{
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let h = health(&ann)?;
    let checklists = Checklists::new(&ann);
    let first = checklists.by_note(&h.checklist)?.ok_or("a list")?.lines[0]
        .id
        .clone();
    checklists.check(&first)?;
    let pack = ann.pack_of(&h.running, &HashSet::new())?;
    add(
        &ben,
        &opened(&ben, &ann.write_pack(&pack, &pack.all_keys(), false)?)?,
        true,
    )?;
    let notes = texts(&ben)?;
    let id = |text: &str| -> Made<String> {
        Ok(notes
            .iter()
            .find(|n| n.text.starts_with(text))
            .ok_or(format!("no {text}"))?
            .id
            .clone())
    };
    let list = Checklists::new(&ben)
        .by_note(&id("Bag")?)?
        .ok_or("a list")?;
    let lines: Vec<_> = list.lines.iter().map(|l| l.text.as_str()).collect();
    assert_eq!(lines, ["Shoes", "Bottle"]);
    assert!(!list.lines.iter().any(|l| l.checked));
    let tally = Tallies::new(&ben).by_note(&id("Laps")?)?.ok_or("a Tally")?;
    assert_eq!(tally.fields[0].name, "Laps");
    assert_eq!(tally.fields[0].target, Some(10.0));
    let entries = Tallies::new(&ben).entries(&id("Laps")?, &PersonFilter::default())?;
    assert!(entries.is_empty());
    let setup = Sessions::new(&ben)
        .setup_of(&id("Training")?)?
        .ok_or("a Session")?;
    assert_eq!(setup.pool, [id("Laps")?, id("Digging")?]);
    assert_eq!(setup.rule, SessionRule::Rolling);
    Ok(())
}

#[test]
fn a_pack_lands_in_new_areas_with_a_suffix_where_a_name_is_taken() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let h = health(&ann)?;
    Places::new(&ben).add_area("Health")?;
    Places::new(&ben).add_area("Health (Pack)")?;
    let pack = ann.pack_of(&h.running, &HashSet::new())?;
    let open = opened(&ben, &ann.write_pack(&pack, &pack.all_keys(), false)?)?;
    let added = add(&ben, &open, true)?;
    let places = Places::new(&ben);
    let name = |id: &String| -> Made<String> { Ok(places.area_by_id(id)?.ok_or("an Area")?.name) };
    let names = added.areas.iter().map(name).collect::<Made<Vec<_>>>()?;
    assert_eq!(names, ["Health (Pack 2)", "Garden"]);
    let topics = places.topics(&added.areas[0])?;
    assert_eq!(topics.len(), 1);
    assert_eq!(topics[0].name, "Running");
    let notes = texts(&ben)?;
    let place = |text: &str| {
        notes
            .iter()
            .find(|n| n.text == text)
            .and_then(|n| n.place.clone())
    };
    assert_eq!(place("Laps"), Some(topics[0].id.clone()));
    assert_eq!(place("Digging"), added.areas.last().cloned());
    assert!(Notes::new(&ben).unsorted()?.is_empty());
    // The same Pack again adds the copies again.
    let again = add(&ben, &open, true)?;
    let names = again.areas.iter().map(name).collect::<Made<Vec<_>>>()?;
    assert_eq!(names, ["Health (Pack 3)", "Garden (Pack)"]);
    Ok(())
}

#[test]
fn routines_start_on_the_day_of_add_and_a_past_reminder_is_left_out() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let plain = Notes::new(&ann).capture("Pills", None, None)?;
    let routines = Routines::new(&ann);
    let eight = ClockTime::new(8, 0).ok_or("a time")?;
    let nine = ClockTime::new(9, 0).ok_or("a time")?;
    let thrice = Recurrence::parse("FREQ=DAILY;COUNT=3")?;
    for schedule in [
        Schedule::new(Day::new(2025, 1, 6), &[eight], Some(thrice)),
        Schedule::new(Day::new(2026, 3, 9), &[nine], None),
        // Ben's clock reads 2026-01-01: a Reminder of the day before.
        Schedule::new(Day::new(2025, 12, 31), &[eight], None),
    ] {
        routines.spawn(&plain.id, &schedule.ok_or("a schedule")?)?;
    }
    let pack = ann.pack_of(&plain.id, &HashSet::new())?;
    let open = opened(&ben, &ann.write_pack(&pack, &pack.all_keys(), false)?)?;
    assert_eq!(open.pack.notes[0].routines.len(), 3);
    let added = add(&ben, &open, true)?;
    let area = Places::new(&ben)
        .area_by_id(&added.areas[0])?
        .ok_or("an Area")?;
    assert_eq!(area.name, "From a Pack");
    let note = texts(&ben)?.into_iter().next().ok_or("the Note")?;
    let kept = Routines::new(&ben).of_note(&note.id)?;
    assert_eq!(kept.len(), 2);
    let daily = kept.iter().find(|r| !r.schedule.once()).ok_or("daily")?;
    assert_eq!(daily.schedule.start, Day::new(2026, 1, 1));
    let rule = daily.schedule.rule.as_ref().map(Recurrence::format);
    assert_eq!(rule.as_deref(), Some("FREQ=DAILY;COUNT=3"));
    assert_eq!(daily.schedule.times, [eight]);
    let once = kept.iter().find(|r| r.schedule.once()).ok_or("once")?;
    assert_eq!(once.schedule.start, Day::new(2026, 3, 9));
    Ok(())
}

#[test]
fn a_reminder_today_is_past_only_before_this_minute() -> TestResult {
    let today = Day::new(2026, 1, 1);
    let at = |h: u32, m: u32| -> Made<Schedule> {
        let time = ClockTime::new(h, m).ok_or("a time")?;
        Ok(Schedule::new(today, &[time], None).ok_or("a schedule")?)
    };
    let minute = 10 * 60;
    assert!(past(&at(9, 59)?, today, minute));
    assert!(!past(&at(10, 0)?, today, minute));
    assert!(!past(
        &Schedule::new(today, &[], None).ok_or("a schedule")?,
        today,
        minute
    ));
    assert!(!past(&at(8, 0)?, Day::new(2025, 12, 31), minute));
    Ok(())
}

#[test]
fn without_claim_each_note_has_a_from_badge_the_receiver_can_remove() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let h = health(&ann)?;
    let pack = ann.pack_of(&h.plain, &HashSet::new())?;
    let open = opened(&ben, &ann.write_pack(&pack, &pack.all_keys(), true)?)?;
    let from = open.from.as_ref().ok_or("from Ann")?;
    assert_eq!(from.name, "Ann");
    assert_eq!(from.id, own_id(&ann)?);
    add(&ben, &open, false)?;
    let note = texts(&ben)?.into_iter().next().ok_or("the Note")?;
    let badge = PackFrom {
        id: own_id(&ann)?,
        name: "Ann".into(),
    };
    assert_eq!(ben.pack_from(&note.id)?, Some(badge.clone()));
    ben.remove_pack_from(&note.id)?;
    assert_eq!(ben.pack_from(&note.id)?, None);
    // Undone (#268): the badge comes back as it was, in one entry.
    let before = ben.all_entries()?.len();
    ben.restore_pack_from(&note.id)?;
    assert_eq!(ben.pack_from(&note.id)?, Some(badge.clone()));
    assert_eq!(ben.all_entries()?.len(), before + 1);
    ben.restore_pack_from(&note.id)?;
    assert_eq!(ben.all_entries()?.len(), before + 1);
    ben.remove_pack_from(&note.id)?;
    add(&ben, &open, true)?;
    for n in texts(&ben)? {
        assert_eq!(ben.pack_from(&n.id)?, None);
    }
    Ok(())
}

#[test]
fn what_another_persons_device_wrote_neither_sets_changes_nor_hides_the_badge() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let h = health(&ann)?;
    let pack = ann.pack_of(&h.plain, &HashSet::new())?;
    let open = opened(&ben, &ann.write_pack(&pack, &pack.all_keys(), true)?)?;
    add(&ben, &open, false)?;
    let badged = texts(&ben)?.into_iter().next().ok_or("the Note")?.id;
    let own = Notes::new(&ben).capture("Milk", None, None)?.id;
    let badge = PackFrom {
        id: own_id(&ann)?,
        name: "Ann".into(),
    };
    // As a modified app in a Shared Area would: Cy's Device writes the
    // field of Ben's Notes, later than anything Ben wrote.
    let cy = device(3, "Cy")?;
    let from = json!({"id": "cy", "name": "Cy"}).to_string();
    let later = |days: i64| Some(ben.now() + chrono::Duration::days(days));
    for note in [&badged, &own] {
        ben.insert(&cy.append(note, keys::FROM, Some(&from), later(1))?.wire())?;
    }
    assert_eq!(ben.pack_from(&badged)?, Some(badge.clone()));
    assert_eq!(ben.pack_from(&own)?, None);
    ben.insert(&cy.append(&badged, keys::FROM, None, later(2))?.wire())?;
    assert_eq!(ben.pack_from(&badged)?, Some(badge.clone()));
    ben.remove_pack_from(&badged)?;
    assert_eq!(ben.pack_from(&badged)?, None);
    ben.remove_pack_from(&own)?;
    assert_eq!(ben.current(&own, keys::FROM)?, Some(from));
    // Undone, it is Ann's again, never Cy's; a Note that never had
    // one gets none.
    ben.restore_pack_from(&badged)?;
    assert_eq!(ben.pack_from(&badged)?, Some(badge));
    let before = ben.all_entries()?.len();
    ben.restore_pack_from(&own)?;
    assert_eq!(ben.pack_from(&own)?, None);
    assert_eq!(ben.all_entries()?.len(), before);
    Ok(())
}

#[test]
fn add_my_profile_signs_the_pack_and_a_changed_one_is_no_pack() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let h = health(&ann)?;
    let pack = ann.pack_of(&h.plain, &HashSet::new())?;
    let signed = ann.write_pack(&pack, &pack.all_keys(), true)?;
    let json: Value = serde_json::from_slice(&signed)?;
    let keys: Vec<_> = json.as_object().ok_or("a file")?.keys().cloned().collect();
    assert_eq!(keys, ["pack", "sig", "type"]);
    let open = opened(&ben, &signed)?;
    assert_eq!(open.from.map(|f| f.name).as_deref(), Some("Ann"));
    let changed = String::from_utf8(signed.clone())?.replacen("Drink water", "Drink wine", 1);
    assert_eq!(
        refused(&ben, changed.as_bytes())?,
        Some(PackError::NotAPack)
    );
    let mut unsigned = json.as_object().ok_or("a file")?.clone();
    unsigned.remove("sig");
    let unsigned = Value::Object(unsigned).to_string();
    assert_eq!(
        refused(&ben, unsigned.as_bytes())?,
        Some(PackError::NotAPack)
    );
    let plain = ann.write_pack(&pack, &pack.all_keys(), false)?;
    let plain_json: Value = serde_json::from_slice(&plain)?;
    assert!(plain_json.get("sig").is_none());
    assert_eq!(opened(&ben, &plain)?.from, None);
    Ok(())
}

#[test]
fn a_blocked_senders_pack_is_refused_and_a_later_format_needs_an_update() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let h = health(&ann)?;
    let pack = ann.pack_of(&h.plain, &HashSet::new())?;
    let signed = ann.write_pack(&pack, &pack.all_keys(), true)?;
    let card = ann.profile_card(false)?.ok_or("a card")?;
    ben.open_profile_card(&card, false)?;
    ben.block_person(&own_id(&ann)?, false)?;
    assert_eq!(refused(&ben, &signed)?, Some(PackError::FromBlocked));
    let json: Value = serde_json::from_slice(&signed)?;
    for later in [json!(2), json!("1"), json!(1.5), json!(1.0), Value::Null] {
        let mut file = json.as_object().ok_or("a file")?.clone();
        file.insert("format".into(), later.clone());
        let file = Value::Object(file).to_string();
        assert_eq!(
            refused(&ben, file.as_bytes())?,
            Some(PackError::NeedsUpdate),
            "{later}"
        );
    }
    let mut one = json.as_object().ok_or("a file")?.clone();
    one.insert("format".into(), json!(1));
    let one = Value::Object(one).to_string();
    assert_eq!(refused(&ben, one.as_bytes())?, Some(PackError::FromBlocked));
    assert!(looks_like_pack(&signed));
    let card_file = ann.profile_card_file(false)?.ok_or("a card file")?;
    assert!(!looks_like_pack(&card_file));
    assert!(!crate::cards::looks_like_profile_card(&signed));
    assert_eq!(refused(&ben, b"{}")?, Some(PackError::NotAPack));
    assert_eq!(refused(&ben, b"hello")?, Some(PackError::NotAPack));
    Ok(())
}

#[test]
fn the_card_and_the_signature_are_checked_before_the_areas_and_notes_are_read() -> TestResult {
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let card = ann.profile_card_text()?.ok_or("a card")?;
    // A payload with a Note that is none: it does not parse.
    let payload = json!({"areas": [], "notes": [{"bad": true}], "profile": card}).to_string();
    let file = |sig: String| {
        json!({"pack": payload, "sig": sig, "type": PACK_TYPE})
            .to_string()
            .into_bytes()
    };
    let key = ann.signing_key()?;
    let signed = file(key.sign(&pack_bytes(&payload)));
    let bad_sig = file(key.sign(&pack_bytes(&format!("{payload} "))));
    assert_eq!(refused(&ben, &signed)?, Some(PackError::NotAPack));
    assert_eq!(refused(&ben, &bad_sig)?, Some(PackError::NotAPack));
    ben.open_profile_card(&ann.profile_card(false)?.ok_or("a card")?, false)?;
    ben.block_person(&own_id(&ann)?, false)?;
    // Only the order tells the two apart: the sender is refused before
    // the Notes fail.
    assert_eq!(refused(&ben, &signed)?, Some(PackError::FromBlocked));
    assert_eq!(refused(&ben, &bad_sig)?, Some(PackError::NotAPack));
    Ok(())
}

#[test]
fn a_payload_whose_indices_point_at_nothing_is_no_pack() {
    let note = |extra: &str| {
        format!(r#"{{"areas":[{{"name":"A","topics":["T"]}}],"notes":[{{"text":"x"{extra}}}]}}"#)
    };
    assert!(parse(&note("")).is_some());
    assert!(parse(&note(r#","area":0,"topic":0"#)).is_some());
    assert!(parse(&note(r#","area":0,"v":null,"format":null"#)).is_some());
    for bad in [
        r#","area":1"#,
        r#","topic":0"#,
        r#","area":0,"topic":1"#,
        r#","area":-1"#,
        r#","area":0.0"#,
        r#","v":0"#,
        r#","format":1"#,
        r#","session":{"count":1,"pool":[0],"rule":"least"}"#,
        r#","list":"checklist""#,
        r#","lines":[],"list":"other""#,
    ] {
        assert!(parse(&note(bad)).is_none(), "{bad}");
    }
    // A Shopping List's Line is under none, so its indent is not read.
    let shop = note(r#","lines":[{"indent":"x","text":"Milk"}],"list":"shopping""#);
    assert!(parse(&shop).is_some());
    let list = note(r#","lines":[{"indent":"x","text":"Milk"}],"list":"checklist""#);
    assert!(parse(&list).is_none());
}

#[test]
fn weekday_counts_and_file_bytes_read_as_the_dart_core_reads_them() {
    for (text, n) in [
        ("3", Some(3)),
        (" 3 ", Some(3)),
        ("+3", Some(3)),
        ("03", Some(3)),
        ("0x3", Some(3)),
        ("\u{feff}3", Some(3)),
        ("-0", Some(0)),
        ("3.0", None),
        ("", None),
        ("\u{663}", None),
    ] {
        assert_eq!(dart_int(text), n, "{text:?}");
    }
    for (text, bytes) in [
        ("-_8=", Some(vec![251, 255])),
        ("AQ==", Some(vec![1])),
        ("AQ%3D%3D", Some(vec![1])),
        ("", Some(vec![])),
        ("AQ", None),
        ("AR==", None),
        ("A Q==", None),
    ] {
        assert_eq!(dart_base64(text), bytes, "{text:?}");
    }
}

#[test]
fn a_file_is_named_for_its_day() {
    assert_eq!(
        file_name(Day::new(2026, 3, 9)),
        "oldbrainfix-pack-2026-03-09.obfpack"
    );
}
