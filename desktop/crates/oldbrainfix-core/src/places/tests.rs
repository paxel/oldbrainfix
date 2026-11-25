use super::*;

#[test]
fn place_writes_check_what_they_write_to() -> Result<()> {
    let store = crate::store::tests::store();
    let places = Places::new(&store);
    assert!(places.add_area("  ").is_err());
    let area = places.add_area(" Home ")?;
    assert_eq!(area.name, "Home");
    assert!(places.add_topic("area:none", "x").is_err());
    let topic = places.add_topic(&area.id, "Post")?;
    assert!(places.add_topic(&topic.id, "Third level").is_err());
    assert!(places.rename("area:none", "x").is_err());
    assert!(places.set_icon(&area.id, Some("rocket")).is_err());
    assert!(places.set_colour(&topic.id, Some("C62828")).is_err());
    assert!(places.delete_area(&area.id, NotesGo::Area).is_err());
    assert!(places.delete_topic(&area.id, NotesGo::Unsorted).is_err());
    places.set_icon(&area.id, Some("home"))?;
    assert_eq!(
        places.area_by_id(&area.id)?.and_then(|a| a.icon),
        Some("home".into())
    );
    places.plant_starter_set(&["work", "meetings", "doctor"])?;
    let names: Vec<_> = places.areas()?.into_iter().map(|a| a.name).collect();
    assert_eq!(names, ["Home", "Work"]);
    places.delete_area(&area.id, NotesGo::Unsorted)?;
    assert!(places.topic_by_id(&topic.id)?.is_none());
    Ok(())
}

#[test]
fn starter_names_are_found_by_slug() {
    assert_eq!(starter_name("to-sort-out"), Some("To sort out"));
    assert_eq!(starter_name("health"), Some("Health"));
    assert_eq!(starter_name("nope"), None);
}

/// Deleting an Area in one pass beside the app (obf-1093), as the Dart
/// core's `area_trash_test.dart`: the same log as trashing each Note the
/// way it was done before, read once for all its Notes, counted from
/// the fields.
mod area_trash {
    use std::path::Path;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicI64, Ordering};

    use chrono::{TimeZone, Utc};

    use super::*;
    use crate::SigningKey;
    use crate::schedule::{ClockTime, Day, Frequency, Recurrence, Schedule};
    use crate::store::Clock;

    /// A clock that moves one second per read from `start`, as the
    /// fixtures' does: a read more or less moves every later date.
    fn ticking(start: (u32, u32)) -> Clock {
        let ticks = Arc::new(AtomicI64::new(0));
        let (day, hour) = start;
        Box::new(move || {
            let n = ticks.fetch_add(1, Ordering::SeqCst) + 1;
            let from = Utc.with_ymd_and_hms(2026, 10, day, hour, 0, 0).single();
            from.unwrap_or_default() + chrono::Duration::seconds(n)
        })
    }

    fn open(path: &Path, start: (u32, u32)) -> Result<Store> {
        Store::open(path, ticking(start), || Ok(SigningKey::from_seed([5; 32])))
    }

    fn daily(hour: u32) -> Result<Schedule> {
        let times: Vec<ClockTime> = ClockTime::new(hour, 0).into_iter().collect();
        let rule = Recurrence::every(Frequency::Daily, 1);
        Schedule::new(Day::new(2026, 10, 24), &times, Some(rule))
            .ok_or_else(|| Error::Invalid("a schedule".into()))
    }

    /// Deleting the Area `area_id` as it was done before obf-1093: one
    /// batch, [`Routines::trash_note`] for each Note, which read every
    /// Routine each time.
    fn delete_area_as_before(store: &Store, area_id: &str, notes_go: NotesGo) -> Result<()> {
        let places = Places::new(store);
        let routines = Routines::new(store);
        store.batch(|| {
            for note in Notes::new(store).inside(area_id)? {
                match notes_go {
                    NotesGo::Unsorted => {
                        store.append(&note.id, note_keys::PLACE, None, None)?;
                        if note.spawned {
                            store.append(&note.id, note_keys::SPAWNED, None, None)?;
                        }
                    }
                    NotesGo::Area => return Err(Error::Invalid("an Area has no Area".into())),
                    NotesGo::Trash => routines.trash_note(&note.id)?,
                }
            }
            for topic in places.topics(area_id)? {
                store.append(&topic.id, keys::TYPE, None, None)?;
            }
            store.append(area_id, keys::TYPE, None, None).map(drop)
        })
    }

    /// Every entry of `store` with all it holds but its local order.
    fn log(store: &Store) -> Result<Vec<String>> {
        Ok(store
            .all_entries()?
            .into_iter()
            .map(|e| {
                [
                    e.device,
                    e.dseq.to_string(),
                    e.entity,
                    e.field,
                    format!("{:?}", e.value),
                    e.date,
                    e.author,
                    e.recorded,
                    format!("{:?}", e.sig),
                    format!("{:?}", e.ctx),
                ]
                .join("|")
            })
            .collect())
    }

    /// An Area "Home" with a Topic, Notes in both, Routines on some of
    /// them, one on a Note merged into one of them, one in the trash on
    /// its own, and a Note and a Routine elsewhere; returns the Area's id.
    fn plant(store: &Store) -> Result<String> {
        let places = Places::new(store);
        let notes = Notes::new(store);
        let routines = Routines::new(store);
        let home = places.add_area("Home")?;
        let kitchen = places.add_topic(&home.id, "Kitchen")?;
        let work = places.add_area("Work")?;
        let pills = notes.capture("pills", Some(&home.id), None)?;
        routines.spawn(&pills.id, &daily(8)?)?;
        routines.spawn(&pills.id, &daily(20)?)?;
        let sauce = notes.capture("sauce", Some(&kitchen.id), None)?;
        routines.spawn(&sauce.id, &daily(18)?)?;
        let vitamins = notes.capture("vitamins", Some(&home.id), None)?;
        routines.spawn(&vitamins.id, &daily(9)?)?;
        notes.merge(&vitamins.id, &pills.id)?;
        let plants = notes.capture("plants", Some(&home.id), None)?;
        routines.trash(&routines.spawn(&plants.id, &daily(10)?)?.id)?;
        notes.capture("no routine", Some(&kitchen.id), None)?;
        let report = notes.capture("report", Some(&work.id), None)?;
        routines.spawn(&report.id, &daily(11)?)?;
        Ok(home.id)
    }

    /// The log after deleting the planted Area the old way and the new
    /// way, each on a copy of the same database.
    fn both(notes_go: NotesGo) -> Result<(Vec<String>, Vec<String>)> {
        let dir = tempfile::tempdir()?;
        let (a, b) = (dir.path().join("a.db"), dir.path().join("b.db"));
        let area_id = {
            let first = open(&a, (24, 7))?;
            plant(&first)?
        };
        for end in ["", "-wal", "-shm"] {
            let from = dir.path().join(format!("a.db{end}"));
            if from.exists() {
                std::fs::copy(&from, dir.path().join(format!("b.db{end}")))?;
            }
        }
        let before = open(&a, (25, 7))?;
        let now = open(&b, (25, 7))?;
        delete_area_as_before(&before, &area_id, notes_go)?;
        Places::new(&now).delete_area(&area_id, notes_go)?;
        Ok((log(&before)?, log(&now)?))
    }

    #[test]
    fn an_area_trashed_in_one_pass_writes_the_log_trashing_each_note_wrote() -> Result<()> {
        let (before, now) = both(NotesGo::Trash)?;
        assert!(before.len() > 20);
        assert_eq!(now, before);
        Ok(())
    }

    #[test]
    fn an_area_whose_notes_go_back_to_unsorted_writes_the_log_it_wrote_before() -> Result<()> {
        let (before, now) = both(NotesGo::Unsorted)?;
        assert_eq!(now, before);
        Ok(())
    }

    #[test]
    fn trashing_an_area_takes_every_routine_of_its_notes_along_and_leaves_the_others() -> Result<()>
    {
        let store = crate::store::tests::store();
        let area_id = plant(&store)?;
        let routines = Routines::new(&store);
        Places::new(&store).delete_area(&area_id, NotesGo::Trash)?;
        let left: Vec<String> = routines
            .all()?
            .iter()
            .map(|r| Ok(routines.note(r)?.map(|n| n.text).unwrap_or_default()))
            .collect::<Result<_>>()?;
        assert_eq!(left, ["report"]);
        let pills = Notes::new(&store)
            .trashed()?
            .into_iter()
            .find(|n| n.text.starts_with("pills"))
            .ok_or_else(|| Error::Invalid("pills".into()))?;
        routines.restore_note(&pills.id)?;
        assert_eq!(routines.of_note(&pills.id)?.len(), 3);
        Ok(())
    }

    /// What deleting the planted Area tells as it goes.
    fn told(notes_go: NotesGo) -> Result<Vec<(usize, usize)>> {
        let store = crate::store::tests::store();
        let area_id = plant(&store)?;
        let told = std::cell::RefCell::new(Vec::new());
        Places::new(&store).delete_area_reporting(&area_id, notes_go, &|n, of| {
            told.borrow_mut().push((n, of));
        })?;
        Ok(told.into_inner())
    }

    #[test]
    fn trashing_an_area_tells_how_many_of_its_notes_are_in_the_trash_as_it_goes() -> Result<()> {
        assert_eq!(
            told(NotesGo::Trash)?,
            [(0, 4), (1, 4), (2, 4), (3, 4), (4, 4)]
        );
        assert_eq!(
            told(NotesGo::Unsorted)?,
            [(0, 4), (1, 4), (2, 4), (3, 4), (4, 4)]
        );
        Ok(())
    }

    #[test]
    fn deleting_an_area_stopped_keeps_the_area_and_the_notes_not_gone_yet() -> Result<()> {
        for notes_go in [NotesGo::Trash, NotesGo::Unsorted] {
            let store = crate::store::tests::store();
            let area_id = plant(&store)?;
            let places = Places::new(&store);
            let gone = std::cell::Cell::new(0);
            let deleted = places.delete_area_stopping(
                &area_id,
                notes_go,
                &|n, _| gone.set(n),
                // Stopped once two Notes have gone.
                &|| gone.get() >= 2,
            )?;
            assert!(!deleted);
            assert!(places.area_by_id(&area_id)?.is_some(), "the Area stays");
            let kitchen = places.topics(&area_id)?;
            assert_eq!(kitchen.len(), 1, "its Topic too");
            let notes = Notes::new(&store);
            let left = notes.inside(&area_id)?.len();
            assert_eq!(left, 2, "{notes_go:?}: two of four gone, two stay");
            // Asked again, the rest goes, and the Area.
            assert!(places.delete_area_stopping(&area_id, notes_go, &|_, _| (), &|| false)?);
            assert!(places.area_by_id(&area_id)?.is_none());
        }
        Ok(())
    }

    /// The reads of deleting an Area of `count` Notes into the trash,
    /// beside the same Routines.
    fn reads_with(count: usize) -> Result<usize> {
        let store = crate::store::tests::store();
        let notes = Notes::new(&store);
        let routines = Routines::new(&store);
        let places = Places::new(&store);
        let home = places.add_area("Home")?;
        let work = places.add_area("Work")?;
        for i in 0..3 {
            let elsewhere = notes.capture(&format!("elsewhere {i}"), Some(&work.id), None)?;
            routines.spawn(&elsewhere.id, &daily(8)?)?;
        }
        let pills = notes.capture("pills", Some(&home.id), None)?;
        routines.spawn(&pills.id, &daily(9)?)?;
        for i in 1..count {
            notes.capture(&format!("note {i}"), Some(&home.id), None)?;
        }
        // What a first read of anything reads once.
        places.notes_inside(&home.id)?;
        let reads = || {
            store.history_reads.load(Ordering::SeqCst) + store.logged_reads.load(Ordering::SeqCst)
        };
        let from = reads();
        places.delete_area(&home.id, NotesGo::Trash)?;
        let read = reads() - from;
        assert!(notes.inside(&home.id)?.is_empty());
        Ok(read)
    }

    #[test]
    fn trashing_an_area_reads_as_much_for_many_notes_as_for_a_few() -> Result<()> {
        assert_eq!(reads_with(30)?, reads_with(3)?);
        Ok(())
    }

    #[test]
    fn the_notes_inside_a_place_are_counted_as_inside_lists_them() -> Result<()> {
        let store = crate::store::tests::store();
        let places = Places::new(&store);
        let notes = Notes::new(&store);
        let home = places.add_area("Home")?;
        let kitchen = places.add_topic(&home.id, "Kitchen")?;
        let work = places.add_area("Work")?;
        notes.capture("in the Area", Some(&home.id), None)?;
        notes.capture("in its Topic", Some(&kitchen.id), None)?;
        notes.capture("elsewhere", Some(&work.id), None)?;
        let trashed = notes.capture("trashed", Some(&home.id), None)?;
        notes.trash(&trashed.id)?;
        let kept = notes.capture("kept", Some(&home.id), None)?;
        let merged = notes.capture("merged", Some(&kitchen.id), None)?;
        notes.merge(&merged.id, &kept.id)?;
        let gone = notes.capture("gone Topic", None, None)?;
        store.append(&gone.id, note_keys::PLACE, Some("topic:gone"), None)?;
        for id in [&home.id, &kitchen.id, &work.id, "topic:gone"] {
            assert_eq!(places.notes_inside(id)?, notes.inside(id)?.len(), "{id}");
        }
        assert_eq!(places.notes_inside(&home.id)?, 3);
        assert_eq!(places.notes_inside(&kitchen.id)?, 1);
        Ok(())
    }
}
