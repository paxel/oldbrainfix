use super::*;
use crate::entry::Entry;
use crate::notes::{Importance, NewFile};
use crate::schedule::{ClockTime, Day, Occurrence, Recurrence, Schedule};
use crate::sessions::SessionRule;
use crate::signing::SigningKey;
use chrono::TimeZone;
use std::collections::BTreeMap;
use std::sync::Arc;

type Made<T> = std::result::Result<T, Box<dyn std::error::Error>>;
type TestResult = Made<()>;

fn at(d: u32, h: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, d, h, 0, 0)
        .single()
        .unwrap_or_default()
}

/// A store whose clock `clock` sets, keyed by `seed`.
fn device(clock: &Arc<parking_lot::Mutex<DateTime<Utc>>>, seed: u8) -> Result<Store> {
    let held = clock.clone();
    Store::in_memory(Box::new(move || *held.lock()), move || {
        Ok(SigningKey::from_seed([seed; 32]))
    })
}

fn clock(start: DateTime<Utc>) -> Arc<parking_lot::Mutex<DateTime<Utc>>> {
    Arc::new(parking_lot::Mutex::new(start))
}

fn utc(t: DateTime<Utc>) -> chrono::NaiveDateTime {
    t.naive_utc()
}

fn names() -> (impl Fn(&Area) -> String, impl Fn(&Topic) -> String) {
    (|a: &Area| a.name.clone(), |t: &Topic| t.name.clone())
}

/// Exports `store` into `parent`, Areas and Topics by their stored
/// names, dates in UTC.
fn export_of(store: &Store, parent: &Path) -> Result<Exported> {
    let (area, topic) = names();
    let names = VaultNames {
        area: &area,
        topic: &topic,
        unsorted: "Unsorted",
        untitled: "Note",
    };
    export(store, parent, &names, &utc, &|_, _| {})
}

fn bytes(seed: usize, length: usize) -> Vec<u8> {
    (0..length).map(|i| ((seed + i) % 251) as u8).collect()
}

fn daily(day: u32, time: Option<(u32, u32)>, rule: Option<&str>) -> Made<Schedule> {
    let times: Vec<ClockTime> = time
        .and_then(|(h, m)| ClockTime::new(h, m))
        .into_iter()
        .collect();
    let rule = rule
        .map(Recurrence::parse)
        .transpose()
        .map_err(|e| format!("{e:?}"))?;
    Ok(Schedule::new(Day::new(2026, 11, i64::from(day)), &times, rule).ok_or("a schedule")?)
}

/// A store holding one of every kind with what a rebuild must bring
/// back, written at moments a day apart.
fn full(clock: &Arc<parking_lot::Mutex<DateTime<Utc>>>) -> Made<Store> {
    let store = device(clock, 1)?;
    let set = |d: u32, h: u32| *clock.lock() = at(d, h);
    let places = Places::new(&store);
    let notes = Notes::new(&store);
    set(1, 8);
    let health = places.add_area("Health")?;
    places.set_colour(&health.id, Some("00796B"))?;
    let back = places.add_topic(&health.id, "Back / neck")?;
    let home = places.add_area("Home")?;
    let doctor = notes.capture("Dentist\nTuesday 9:00 ☐ bring card", Some(&health.id), None)?;
    notes.set_importance(&doctor.id, Importance::Important)?;
    set(2, 9);
    let link = format!("See [the dentist](oldbrainfix://note/{}) first", doctor.id);
    let exercises = notes.capture_as(
        &format!("---\nnot front matter\n\n{link}\r\nlast line without a break"),
        text_format::MARKDOWN,
        Some(&back.id),
        None,
    )?;
    set(3, 10);
    notes.amend(&exercises.id, "Stretch twice a day\n\n")?;
    let merged = notes.capture_with("Scan of the plan", None, None, &[bytes(1, 300)], &[])?;
    let held = notes.by_id(&merged.id)?.ok_or("the merged Note")?;
    let photo = held.photos.first().ok_or("its photo")?.clone();
    notes.set_photo_text(&merged.id, &photo, "Plan: 3 x 10")?;
    // A merged Note's Reminder shows under the Note it went into.
    Routines::new(&store).spawn(&merged.id, &daily(25, Some((9, 0)), None)?)?;
    set(4, 11);
    notes.merge(&merged.id, &exercises.id)?;
    let letter = notes.capture("Insurance letter", Some(&home.id), None)?;
    notes.attach_file(
        &letter.id,
        &NewFile {
            name: "letter.pdf",
            media_type: "application/pdf",
            bytes: &bytes(2, 500),
            pages: Some(2),
        },
    )?;
    let voice = notes.capture_with("Shopping idea", None, None, &[], &[bytes(3, 400)])?;
    let voiced = notes.by_id(&voice.id)?.ok_or("the voice Note")?;
    let recording = voiced.recordings.first().ok_or("its recording")?.clone();
    notes.set_transcript(&voice.id, &recording, "Buy oat milk")?;
    notes.remove_recording(&voice.id, &recording)?;
    set(5, 12);
    let bag = notes.capture("Bag\nShoes\nBottle", Some(&health.id), None)?;
    let checklists = Checklists::new(&store);
    checklists.make(&bag.id)?;
    let lines = checklists.by_note(&bag.id)?.ok_or("the Checklist")?.lines;
    let shoes = lines.first().ok_or("a Line")?.id.clone();
    checklists.add(&bag.id, "Socks", Some(1), true)?;
    checklists.check(&shoes)?;
    checklists.set_on_agenda(&bag.id, ListOnAgenda::Off)?;
    let shop = notes.capture("Groceries", Some(&home.id), None)?;
    let shopping = ShoppingLists::new(&store);
    shopping.make(&shop.id)?;
    shopping.add(&shop.id, "Bread", &utc)?;
    let milk = shopping.add(&shop.id, "Milk", &utc)?;
    shopping.check(&milk, &utc)?;
    set(6, 7);
    let weight = notes.capture("Weight", Some(&health.id), None)?;
    let tallies = Tallies::new(&store);
    let kg = tallies.add_field(&weight.id, "Weight", "kg", Some(70.0), None)?;
    let kg_again = kg.clone();
    let done = tallies.add_check(&weight.id, "Walked", None)?;
    tallies.add(
        &weight.id,
        &BTreeMap::from([(kg.clone(), 71.25), (done.clone(), 1.0)]),
        "after breakfast",
        Some(at(6, 7)),
    )?;
    let removed = tallies.add(
        &weight.id,
        &BTreeMap::from([(kg, 70.5)]),
        "",
        Some(at(6, 8)),
    )?;
    tallies.trash_entry(&removed.id)?;
    let training = notes.capture("Training", Some(&health.id), None)?;
    Sessions::new(&store).make(
        &training.id,
        std::slice::from_ref(&weight.id),
        1,
        SessionRule::Rolling,
    )?;
    set(7, 8);
    let routines = Routines::new(&store);
    let pills = routines.spawn(&doctor.id, &daily(1, Some((8, 0)), Some("FREQ=DAILY"))?)?;
    let first = Occurrence::new(Day::new(2026, 11, 2), ClockTime::new(8, 0));
    let second = Occurrence::new(Day::new(2026, 11, 3), ClockTime::new(8, 0));
    routines.mark_done(&pills.id, &first, Some(at(2, 8)))?;
    routines.skip(&pills.id, &second, Some(at(3, 9)))?;
    routines.spawn(&letter.id, &daily(20, Some((10, 30)), None)?)?;
    // A Line checked for an Occurrence of the Checklist's Routine, and
    // a Session's run with its pick and an entry recorded from it.
    let packing = routines.spawn(&bag.id, &daily(1, None, Some("FREQ=DAILY"))?)?;
    let on = crate::checklists::RoutineOccurrence {
        routine_id: packing.id.clone(),
        occurrence: Occurrence::new(Day::new(2026, 11, 4), None),
    };
    Checklists::new(&store).check_for(&shoes, &on)?;
    let run = routines.spawn(&training.id, &daily(1, Some((7, 0)), Some("FREQ=DAILY"))?)?;
    let ran = Occurrence::new(Day::new(2026, 11, 5), ClockTime::new(7, 0));
    Sessions::new(&store).record(
        &run.id,
        &ran,
        &weight.id,
        &BTreeMap::from([(kg_again, 70.75)]),
        "after the run",
    )?;
    Frogs::new(&store).spawn(&letter.id)?;
    let filed_nowhere = notes.capture("Call Max", None, None)?;
    store.set_hidden_from_ai(&filed_nowhere.id, true)?;
    store.set_hidden_from_ai(&home.id, true)?;
    set(8, 12);
    Ok(store)
}

/// `text` with each app link to a Note as that Note's first line.
fn named_links(store: &Store, text: &str) -> String {
    relinked(text, &|url| match app_link::parse(url) {
        Some(AppLink::Note(id)) => Notes::new(store)
            .by_id(&id)
            .ok()
            .flatten()
            .map(|n| format!("note:{}", title(&n, "?"))),
        _ => None,
    })
}

fn place_name(store: &Store, place: Option<&str>) -> Result<String> {
    let places = Places::new(store);
    let Some(place) = place else {
        return Ok("-".into());
    };
    if let Some(topic) = places.topic_by_id(place)? {
        let area = places.area_by_id(&topic.area_id)?.map(|a| a.name);
        return Ok(format!("{}/{}", area.unwrap_or_default(), topic.name));
    }
    Ok(places
        .area_by_id(place)?
        .map(|a| a.name)
        .unwrap_or_default())
}

/// Everything of `store` a rebuild must bring back, without an id:
/// each Note by its capture time with its text, links by their
/// target's title, place, Importance, mark, Sections, files, kind,
/// Routines and Reminders with their check-offs, and Frogs.
fn digest(store: &Store) -> Made<Vec<String>> {
    let notes = Notes::new(store);
    let mut all = notes.all()?;
    all.sort_by_key(|n| (n.captured, n.shown_text()));
    let mut out = Vec::new();
    for n in &all {
        out.push(format!(
            "{} {} {:?} [{}] {:?} hidden={}",
            dart_iso(n.captured),
            n.text_format,
            named_links(store, &n.shown_text()),
            place_name(store, n.place.as_deref())?,
            n.importance,
            store.is_marked_hidden_from_ai(&n.id)?,
        ));
        let area = store.area_of(&n.id)?;
        if let Some(area) = area {
            let a = Places::new(store).area_by_id(&area)?.ok_or("the Area")?;
            out.push(format!(
                "  area {} {:?} hidden={}",
                a.name,
                a.colour,
                store.is_marked_hidden_from_ai(&a.id)?
            ));
        }
        for s in notes.sections(&n.id)? {
            out.push(format!(
                "  section {} merged={} {:?} photos={:?} texts={:?}",
                dart_iso(s.captured),
                s.id.starts_with("note:"),
                s.shown_text(),
                s.photos.len(),
                s.photo_texts.values().collect::<Vec<_>>(),
            ));
        }
        out.push(format!(
            "  photos={} texts={:?} recordings={} removed={} transcripts={:?}",
            n.photos.len(),
            n.photo_texts.values().collect::<Vec<_>>(),
            n.recordings.len(),
            n.removed_recordings.len(),
            n.transcripts.values().collect::<Vec<_>>(),
        ));
        for a in &n.attachments {
            let bytes = store.file(&a.hash).map(|b| b.len());
            out.push(format!(
                "  file {} {} {} {:?} {:?}",
                a.name, a.media_type, a.size, a.pages, bytes
            ));
        }
        out.push(format!("  kind {:?}", NoteKind::of(store, &n.id)?));
        if Checklists::new(store).by_note(&n.id)?.is_some() {
            let shows = Checklists::new(store).on_agenda(&n.id)?;
            out.push(format!("  agenda {}", shows.code()));
        }
        if ShoppingLists::new(store).by_note(&n.id, &utc)?.is_some() {
            let shows = ShoppingLists::new(store).on_agenda(&n.id)?;
            out.push(format!("  agenda {}", shows.code()));
        }
        if let Some(c) = Checklists::new(store).by_note(&n.id)? {
            for l in c.lines {
                out.push(format!(
                    "  line {} group={} checked={} {:?}",
                    l.text,
                    l.in_group(),
                    l.checked,
                    l.checked_at.map(dart_iso)
                ));
            }
        }
        if let Some(s) = ShoppingLists::new(store).by_note(&n.id, &utc)? {
            for l in s.lines {
                out.push(format!(
                    "  shopping {} {} struck={} {:?}",
                    l.text,
                    l.reappear.code(),
                    l.struck,
                    l.checked_at.map(dart_iso)
                ));
            }
        }
        let tallies = Tallies::new(store);
        if let Some(t) = tallies.by_note(&n.id)? {
            let name: HashMap<String, String> = t
                .fields
                .iter()
                .map(|f| (f.id.clone(), f.name.clone()))
                .collect();
            for f in &t.fields {
                out.push(format!(
                    "  field {} {} {:?} {} check={}",
                    f.name,
                    f.unit,
                    f.target,
                    f.rule.name(),
                    f.check
                ));
            }
            let mut entries = tallies.entries(&n.id, &PersonFilter::default())?;
            entries.extend(tallies.removed_entries(&n.id)?);
            entries.sort_by_key(|e| e.at);
            for e in entries {
                let values: BTreeMap<String, f64> = e
                    .values
                    .iter()
                    .map(|(k, v)| (name.get(k).cloned().unwrap_or_default(), *v))
                    .collect();
                out.push(format!(
                    "  entry {} {values:?} {:?} trashed={}",
                    dart_iso(e.at),
                    e.line,
                    e.is_trashed()
                ));
            }
        }
        let sessions = Sessions::new(store);
        if let Some(s) = sessions.setup_of(&n.id)? {
            let pool: Vec<String> = sessions
                .pool(&s)?
                .iter()
                .filter_map(|t| notes.by_id(t).ok().flatten())
                .map(|t| title(&t, "?"))
                .collect();
            out.push(format!("  session {pool:?} {} {}", s.count, s.rule.name()));
        }
        let mut routines = Routines::new(store).of_note(&n.id)?;
        routines.sort_by_key(|r| r.schedule.start);
        for r in routines {
            out.push(format!(
                "  routine {} {:?} {:?} done={:?} skipped={:?} scheduled={:?}",
                r.schedule.start,
                r.schedule.times,
                r.schedule.rule.as_ref().map(Recurrence::format),
                r.done
                    .iter()
                    .map(|(k, v)| (k, dart_iso(*v)))
                    .collect::<Vec<_>>(),
                r.skipped
                    .iter()
                    .map(|(k, v)| (k, dart_iso(*v)))
                    .collect::<Vec<_>>(),
                r.scheduled_at.map(dart_iso),
            ));
        }
        for f in Frogs::new(store).of_note(&n.id)? {
            out.push(format!("  frog done={:?}", f.done.map(dart_iso)));
        }
        out.extend(routine_marks(store, &n.id)?);
    }
    Ok(out)
}

/// The Line checks, picks and recorded entries of the Routines of the
/// Note `note_id`, each by what it names: a Line by its text, a pick
/// by its Tally's title, an entry by its moment and Tally.
fn routine_marks(store: &Store, note_id: &str) -> Made<Vec<String>> {
    let notes = Notes::new(store);
    let named = |id: &str| -> String {
        notes
            .by_id(id)
            .ok()
            .flatten()
            .map_or_else(|| "?".into(), |n| title(&n, "?"))
    };
    let line = |id: &str| -> String {
        store
            .current_fields(id)
            .ok()
            .and_then(|f| f.get(checklist_keys::TEXT).map(str::to_string))
            .unwrap_or_else(|| "?".into())
    };
    let mut out = Vec::new();
    let mut routines = Routines::new(store).of_note(note_id)?;
    routines.sort_by_key(|r| r.schedule.start);
    for r in routines {
        for (key, value) in store.current_fields(&r.id)?.iter() {
            let Some(value) = value else {
                continue;
            };
            if let Some(rest) = key.strip_prefix(checklist_keys::CHECK_PREFIX) {
                let (occurrence, id) = rest.split_once('@').ok_or("a check")?;
                out.push(format!("  check {occurrence} {} {value}", line(id)));
            } else if let Some(occurrence) = key.strip_prefix(session_keys::PICK_PREFIX) {
                let picked: Vec<String> = value.split(',').map(named).collect();
                out.push(format!("  pick {occurrence} {picked:?}"));
            } else if let Some(rest) = key.strip_prefix(session_keys::ENTRY_PREFIX) {
                let (occurrence, id) = rest.split_once('@').ok_or("an entry")?;
                let entry = Tallies::new(store).entry_by_id(id)?.ok_or("the entry")?;
                out.push(format!(
                    "  entry {occurrence} {} {:?} {}",
                    dart_iso(entry.at),
                    entry.line,
                    named(value)
                ));
            }
        }
    }
    out.sort();
    Ok(out)
}

/// The vault folder written into `parent`, its only folder.
fn rebuilt(vault: &Path, seed: u8) -> Made<Store> {
    let store = device(&clock(at(9, 12)), seed)?;
    rebuild(&store, vault, "From the vault", &|_, _| {})?;
    Ok(store)
}

#[test]
fn a_rebuild_brings_back_every_note_as_it_was_under_new_ids() -> TestResult {
    let time = clock(at(1, 8));
    let old = full(&time)?;
    let dir = tempfile::tempdir()?;
    let exported = export_of(&old, dir.path())?;
    assert_eq!(exported.notes, 9);
    assert_eq!(exported.missing, 0);
    let new = rebuilt(&exported.folder, 2)?;
    assert_eq!(digest(&new)?, digest(&old)?);
    // Every entity is new, and nothing new names an old one (Q630).
    let mut old_ids: HashSet<String> = old.all_entries()?.into_iter().map(|e| e.entity).collect();
    for t in Tallies::new(&old).all()? {
        old_ids.extend(t.fields.into_iter().map(|f| f.id));
    }
    let device = new.device_id()?;
    let entries: Vec<Entry> = new.all_entries()?;
    assert!(!entries.is_empty());
    for e in &entries {
        assert_eq!(e.device, device, "{e:?}");
        for id in &old_ids {
            assert_ne!(&e.entity, id);
            assert!(!e.field.contains(id.as_str()), "{} names {id}", e.field);
            let value = e.value.as_deref().unwrap_or_default();
            assert!(!value.contains(id.as_str()), "{value} names {id}");
        }
    }
    Ok(())
}

#[test]
fn what_is_hidden_from_the_ai_stays_hidden_after_a_rebuild() -> TestResult {
    let time = clock(at(1, 8));
    let old = full(&time)?;
    let dir = tempfile::tempdir()?;
    let new = rebuilt(&export_of(&old, dir.path())?.folder, 2)?;
    let seen: Vec<String> = new
        .for_ai(|| Notes::new(&new).all())?
        .iter()
        .map(|n| title(n, "?"))
        .collect();
    assert!(seen.contains(&"Dentist".to_string()), "{seen:?}");
    for hidden in ["Call Max", "Insurance letter", "Groceries"] {
        assert!(!seen.contains(&hidden.to_string()), "{hidden} in {seen:?}");
    }
    Ok(())
}

#[test]
fn a_note_is_a_file_of_its_area_and_topic_with_its_files_beside_it() -> TestResult {
    let time = clock(at(1, 8));
    let old = full(&time)?;
    let dir = tempfile::tempdir()?;
    let folder = export_of(&old, dir.path())?.folder;
    assert_eq!(
        folder.file_name().and_then(|n| n.to_str()),
        Some("oldbrainfix-vault-2026-11-08")
    );
    let dentist = std::fs::read_to_string(folder.join("Health/Dentist 2026-11-01.md"))?;
    assert!(
        dentist.starts_with("---\noldbrainfix: 1\nid: \"note:"),
        "{dentist}"
    );
    assert!(dentist.contains("\narea: \"Health\"\n"), "{dentist}");
    assert!(
        dentist.contains("\nimportance: \"important\"\n"),
        "{dentist}"
    );
    assert!(dentist.contains("\"rule\": \"FREQ=DAILY\""), "{dentist}");
    assert!(dentist.contains("\"done\": {\"2026-11-02T08:00\": \"2026-11-02T08:00:00.000Z\"}"));
    assert!(
        dentist.ends_with("---\nDentist\nTuesday 9:00 ☐ bring card\n"),
        "{dentist}"
    );
    // The Topic's folder, the link to the other Note as a relative
    // Markdown link, the merged Note's photo in the attachments.
    let plan = folder.join("Health/Back neck/not front matter 2026-11-02.md");
    let plan = std::fs::read_to_string(plan)?;
    assert!(
        plan.contains("[the dentist](../Dentist%202026-11-01.md)"),
        "{plan}"
    );
    assert!(
        plan.contains("\n*2026-11-03 10:00*\n\nStretch twice a day\n\n"),
        "{plan}"
    );
    assert!(plan.contains(SHOWN), "{plan}");
    let photos: Vec<_> = std::fs::read_dir(folder.join("Health/Back neck/attachments"))?
        .collect::<std::io::Result<_>>()?;
    assert_eq!(photos.len(), 1);
    assert!(folder.join("Home/attachments/letter.pdf").is_file());
    assert!(folder.join("Unsorted/Call Max 2026-11-07.md").is_file());
    // The removed recording keeps its Transcript in the properties.
    let idea = std::fs::read_to_string(folder.join("Unsorted/Shopping idea 2026-11-04.md"))?;
    assert!(idea.contains("\"removed\": true"), "{idea}");
    assert!(idea.contains("\"text\": \"Buy oat milk\""), "{idea}");
    Ok(())
}

#[test]
fn an_export_never_touches_what_the_folder_holds() -> TestResult {
    let time = clock(at(1, 8));
    let old = full(&time)?;
    let dir = tempfile::tempdir()?;
    std::fs::create_dir(dir.path().join("oldbrainfix-vault-2026-11-08"))?;
    std::fs::write(
        dir.path().join("oldbrainfix-vault-2026-11-08/mine.txt"),
        "kept",
    )?;
    let second = export_of(&old, dir.path())?.folder;
    assert!(second.ends_with("oldbrainfix-vault-2026-11-08-2"));
    assert_eq!(
        std::fs::read_to_string(dir.path().join("oldbrainfix-vault-2026-11-08/mine.txt"))?,
        "kept"
    );
    Ok(())
}

#[test]
fn a_title_never_names_a_file_outside_its_folder() -> TestResult {
    let store = device(&clock(at(1, 8)), 1)?;
    let notes = Notes::new(&store);
    for text in [
        "../../escaped",
        "/etc/passwd",
        "CON",
        "a\\b:c*d?e\"f<g>h|i",
        ". . .",
        ".hidden",
    ] {
        notes.capture(text, None, None)?;
    }
    let dir = tempfile::tempdir()?;
    let folder = export_of(&store, dir.path())?.folder;
    let mut written: Vec<String> = std::fs::read_dir(folder.join("Unsorted"))?
        .map(|e| e.map(|e| e.file_name().to_string_lossy().into_owned()))
        .collect::<std::io::Result<_>>()?;
    written.sort();
    assert_eq!(
        written,
        vec![
            "CON_ 2026-11-01.md",
            "Note 2026-11-01.md",
            "a b c d e f g h i 2026-11-01.md",
            "escaped 2026-11-01.md",
            "etc passwd 2026-11-01.md",
            "hidden 2026-11-01.md",
        ]
    );
    let outside: Vec<_> = std::fs::read_dir(dir.path())?.collect::<std::io::Result<_>>()?;
    assert_eq!(outside.len(), 1);
    Ok(())
}

#[test]
fn names_are_safe_on_every_file_system() {
    assert_eq!(safe_name("CON", "x"), "CON_");
    assert_eq!(safe_name("com3.txt", "x"), "com3.txt_");
    assert_eq!(safe_name("  a\u{202e}b  ", "x"), "a b");
    assert_eq!(safe_name("#tag [x] ^y", "x"), "tag x y");
    assert_eq!(safe_name("...", "Note"), "Note");
    let long = "ä".repeat(200);
    assert_eq!(safe_name(&long, "x").chars().count(), NAME_CHARS);
    assert!(safe_name(&"😀".repeat(100), "x").len() <= NAME_BYTES);
    let mut taken = Taken::reserving(&[ATTACHMENTS]);
    assert_eq!(taken.take("Attachments", ""), "Attachments (2)");
    assert_eq!(taken.take("Back", ".md"), "Back.md");
    assert_eq!(taken.take("back", ".md"), "back (2).md");
}

#[test]
fn a_rebuild_goes_only_into_an_empty_store() -> TestResult {
    let time = clock(at(1, 8));
    let old = full(&time)?;
    let dir = tempfile::tempdir()?;
    let folder = export_of(&old, dir.path())?.folder;
    let store = device(&clock(at(9, 12)), 2)?;
    Notes::new(&store).capture("Already here", None, None)?;
    let seq = store.current_seq()?;
    let refused = rebuild(&store, &folder, "From the vault", &|_, _| {});
    assert!(matches!(refused, Err(Error::Vault(VaultProblem::NotEmpty))));
    assert_eq!(store.current_seq()?, seq);
    let empty = tempfile::tempdir()?;
    std::fs::create_dir(empty.path().join("attachments"))?;
    std::fs::write(empty.path().join("attachments/a.md"), "not a Note")?;
    let none = rebuild(
        &device(&clock(at(9, 12)), 3)?,
        empty.path(),
        "x",
        &|_, _| {},
    );
    assert!(matches!(none, Err(Error::Vault(VaultProblem::NotAVault))));
    Ok(())
}

#[test]
fn a_vault_of_a_later_format_needs_an_update() -> TestResult {
    let dir = tempfile::tempdir()?;
    std::fs::write(
        dir.path().join("a.md"),
        "---\noldbrainfix: 2\nid: \"note:x\"\n---\nText\n",
    )?;
    let store = device(&clock(at(9, 12)), 2)?;
    let refused = rebuild(&store, dir.path(), "x", &|_, _| {});
    assert!(matches!(
        refused,
        Err(Error::Vault(VaultProblem::NeedsUpdate))
    ));
    assert_eq!(store.current_seq()?, 0);
    Ok(())
}

#[test]
fn a_file_made_elsewhere_is_filed_by_its_folders_with_its_tags_and_date() -> TestResult {
    let dir = tempfile::tempdir()?;
    let topic = dir.path().join("Health/Back/deeper");
    std::fs::create_dir_all(&topic)?;
    std::fs::create_dir_all(dir.path().join(".obsidian"))?;
    std::fs::write(dir.path().join(".obsidian/app.md"), "settings")?;
    let made = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_790_000_000);
    let file = topic.join("Exercises.md");
    std::fs::write(
        &file,
        "---\ntags: [life/health, back pain]\n---\nSee [the plan](../../Plan.md)\n",
    )?;
    std::fs::File::options()
        .write(true)
        .open(&file)?
        .set_modified(made)?;
    std::fs::write(
        dir.path().join("Health/Plan.md"),
        "# Plan\nTwice a day #back\n",
    )?;
    std::fs::write(dir.path().join("Top.txt"), "At the top")?;
    // Properties another tool broke: the file is read as made there.
    std::fs::write(
        dir.path().join("Health/Broken.md"),
        "---\noldbrainfix: 1\nid: \"note:x\"\nroutines:\n  - start: 2026-11-01\n---\nKept\n",
    )?;
    let store = device(&clock(at(9, 12)), 2)?;
    let summary = rebuild(&store, dir.path(), "From the vault", &|_, _| {})?;
    assert_eq!(summary.notes, 4);
    assert_eq!(summary.elsewhere, 4);
    let notes = Notes::new(&store);
    let all = notes.all()?;
    let find = |first: &str| {
        all.iter()
            .find(|n| title(n, "?") == first)
            .ok_or(first.to_string())
    };
    let exercises = find("Exercises")?;
    assert_eq!(
        place_name(&store, exercises.place.as_deref())?,
        "Health/Back"
    );
    assert_eq!(exercises.captured.timestamp(), 1_790_000_000);
    let plan = find("Plan")?;
    assert_eq!(
        exercises.text,
        format!(
            "# Exercises\n---\ntags: [life/health, back pain]\n---\nSee [the plan]({})\n#life/health #back-pain",
            AppLink::Note(plan.id.clone()).url()
        )
    );
    assert_eq!(place_name(&store, plan.place.as_deref())?, "Health");
    let top = find("Top")?;
    assert_eq!(place_name(&store, top.place.as_deref())?, "From the vault");
    assert!(find("Broken")?.text.contains("oldbrainfix: 1"));
    assert!(find("settings").is_err());
    Ok(())
}

#[test]
fn a_file_only_comes_back_from_inside_the_vault() -> TestResult {
    let dir = tempfile::tempdir()?;
    let vault = dir.path().join("vault");
    std::fs::create_dir_all(vault.join("Home/attachments"))?;
    std::fs::write(dir.path().join("secret.pdf"), "outside")?;
    #[cfg(unix)]
    std::os::unix::fs::symlink(
        dir.path().join("secret.pdf"),
        vault.join("Home/attachments/link.pdf"),
    )?;
    let note = |path: &str| {
        format!(
            "---\noldbrainfix: 1\nid: \"note:a\"\ncaptured: \"2026-11-01T08:00:00.000Z\"\nformat: \"plain\"\narea: \"Home\"\nattachments: [{{\"kind\": \"file\", \"path\": \"{path}\", \"hash\": \"ab\", \"name\": \"x.pdf\", \"type\": \"application/pdf\"}}]\n---\nLetter\n"
        )
    };
    std::fs::write(vault.join("Home/Link.md"), note("attachments/link.pdf"))?;
    std::fs::write(vault.join("Home/Up.md"), note("../../secret.pdf"))?;
    // Nor does a Note: no link to a folder or a file outside is read.
    std::fs::create_dir_all(dir.path().join("outside"))?;
    std::fs::write(dir.path().join("outside/Far.md"), "Far away")?;
    #[cfg(unix)]
    {
        let far = dir.path().join("outside");
        std::os::unix::fs::symlink(&far, vault.join("Linked"))?;
        std::os::unix::fs::symlink(far.join("Far.md"), vault.join("Home/Far.md"))?;
    }
    let store = device(&clock(at(9, 12)), 2)?;
    let summary = rebuild(&store, &vault, "x", &|_, _| {})?;
    assert_eq!(summary.notes, 2);
    assert_eq!((summary.files, summary.missing), (0, 2));
    for n in Notes::new(&store).all()? {
        assert!(n.attachments.is_empty(), "{n:?}");
    }
    Ok(())
}

#[test]
fn texts_come_back_byte_for_byte() -> TestResult {
    let store = device(&clock(at(1, 8)), 1)?;
    let notes = Notes::new(&store);
    let texts = [
        "---\nlooks like front matter\n---",
        "ends in blank lines\n\n\n",
        "crlf\r\ninside\r\n",
        &format!("holds a marker\n\n{SECTION}section:x{MARKER_END}\n*date*\n\nstill the text"),
        &format!("holds the other\n\n{SHOWN}\n\nstill the text"),
        "",
    ];
    for text in texts {
        let n = notes.capture_with(text, None, None, &[bytes(9, 50)], &[])?;
        notes.amend(&n.id, "an Amend\n")?;
    }
    let dir = tempfile::tempdir()?;
    let new = rebuilt(&export_of(&store, dir.path())?.folder, 2)?;
    assert_eq!(digest(&new)?, digest(&store)?);
    Ok(())
}
