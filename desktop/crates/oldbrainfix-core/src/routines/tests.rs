use super::*;
use crate::schedule::Frequency;

fn daily_at_eight() -> Schedule {
    let eight = ClockTime::new(8, 0).into_iter().collect::<Vec<_>>();
    let rule = Recurrence::every(Frequency::Daily, 1);
    match Schedule::new(Day::new(2026, 1, 1), &eight, Some(rule)) {
        Some(s) => s,
        None => panic!("a schedule"),
    }
}

fn fields(pairs: &[(&str, &str)]) -> Fields {
    Fields::new(
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), Some(v.to_string())))
            .collect(),
    )
}

#[test]
fn a_schedule_reads_as_newer_by_its_version_or_a_part_not_known() {
    let daily = [
        (keys::START, "2026-10-24"),
        (keys::TIMES, "08:00"),
        (keys::RULE, "FREQ=DAILY"),
    ];
    let with = |key: &'static str, value: &'static str| {
        let mut out: Vec<(&str, &str)> = daily.iter().filter(|(k, _)| *k != key).copied().collect();
        out.push((key, value));
        out
    };
    for known in [
        daily.to_vec(),
        with(keys::SCHEDULE_VERSION, "1"),
        with(keys::TIMES, ""),
    ] {
        let f = fields(&known);
        assert!(Routines::schedule_of(&f).is_some(), "{known:?}");
        assert!(!Routines::schedule_is_newer(&f), "{known:?}");
    }
    for newer in [
        with(keys::SCHEDULE_VERSION, "2"),
        with(keys::SCHEDULE_VERSION, "01"),
        with(keys::SCHEDULE_VERSION, " 1"),
        with(keys::SCHEDULE_VERSION, "1.0"),
        with(keys::SCHEDULE_VERSION, ""),
        vec![(keys::SCHEDULE_VERSION, "2")],
        with(keys::TIMES, "08:00,8pm"),
        with(keys::TIMES, "08:00,"),
        with(keys::RULE, "FREQ=DAILY;BYEASTER=1"),
        with(keys::START, "2026-13-01"),
    ] {
        let f = fields(&newer);
        assert!(Routines::schedule_of(&f).is_none(), "{newer:?}");
        assert!(Routines::schedule_is_newer(&f), "{newer:?}");
    }
    for none in [
        vec![],
        vec![(keys::TIMES, "08:00"), (keys::RULE, "FREQ=DAILY")],
        vec![(keys::SCHEDULE_VERSION, "1")],
    ] {
        assert!(!Routines::schedule_is_newer(&fields(&none)), "{none:?}");
    }
}

#[test]
fn a_routine_of_a_later_version_fires_nothing_and_nothing_here_changes_it() -> Result<()> {
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    let routines = Routines::new(&store);
    let pills = notes.capture("pills", None, None)?;
    let id = "routine:later";
    store.batch(|| {
        store.append(id, keys::TYPE, Some(keys::TYPE_ROUTINE), None)?;
        store.append(id, keys::NOTE, Some(&pills.id), None)?;
        store.append(id, keys::START, Some("2026-01-01"), None)?;
        store.append(id, keys::TIMES, Some("08:00"), None)?;
        store.append(id, keys::RULE, Some("FREQ=DAILY"), None)?;
        store.append(id, keys::SCHEDULE_VERSION, Some("2"), None)?;
        Ok(())
    })?;
    assert_eq!(routines.by_id(id)?, None);
    assert!(routines.all()?.is_empty());
    let newer = NewerRoutine {
        id: id.to_string(),
        note_id: pills.id.clone(),
    };
    assert_eq!(routines.newer()?, vec![newer.clone()]);
    assert_eq!(routines.newer_of_note(&pills.id)?, vec![newer]);
    let before = store.history(id)?.len();
    let occurrence = Occurrence::new(Day::new(2026, 1, 1), ClockTime::new(8, 0));
    assert!(routines.reschedule(id, &daily_at_eight()).is_err());
    assert!(routines.set_renag(id, Some(Duration::minutes(5))).is_err());
    assert!(routines.set_escalation(id, Some(2)).is_err());
    assert!(routines.trash(id).is_err());
    assert!(routines.mark_done(id, &occurrence, None).is_err());
    assert!(routines.unmark_done(id, &occurrence).is_err());
    assert_eq!(store.history(id)?.len(), before);
    // Merged into another Note, it shows under that one.
    let bx = notes.capture("pill box", None, None)?;
    notes.merge(&pills.id, &bx.id)?;
    assert_eq!(routines.newer_of_note(&bx.id)?.len(), 1);
    assert!(routines.newer_of_note(&pills.id)?.is_empty());
    // In the trash, by a newer version, it is listed no more.
    store.append(id, keys::TRASHED, Some("2026-01-01T10:00:00.000Z"), None)?;
    assert!(routines.newer()?.is_empty());
    Ok(())
}

#[test]
fn a_spawned_routine_takes_its_note_out_of_unsorted() -> Result<()> {
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    let note = notes.capture("Take the pills", None, None)?;
    assert!(note.is_unsorted());
    let routines = Routines::new(&store);
    let routine = routines.spawn(&note.id, &daily_at_eight())?;
    assert!(!routine.is_reminder());
    assert_eq!(notes.unsorted()?, vec![]);
    assert_eq!(routines.of_note(&note.id)?, vec![routine.clone()]);
    // The same schedule again writes nothing.
    let before = store.all_entries()?.len();
    routines.reschedule(&routine.id, &daily_at_eight())?;
    assert_eq!(store.all_entries()?.len(), before);
    Ok(())
}

#[test]
fn a_card_ticks_the_first_open_occurrence_today_else_the_last_done() -> Result<()> {
    let store = crate::store::tests::store();
    let routines = Routines::new(&store);
    let wall = |utc: DateTime<Utc>| utc.naive_utc();
    let times: Vec<ClockTime> = [ClockTime::new(8, 0), ClockTime::new(20, 0)]
        .into_iter()
        .flatten()
        .collect();
    let rule = Recurrence::every(Frequency::Daily, 1);
    let Some(schedule) = Schedule::new(Day::new(2026, 1, 1), &times, Some(rule)) else {
        panic!("a schedule");
    };
    let routine = routines.create("pills", &schedule)?;
    let note = routine.note_id.clone();
    let ticked = || -> Result<Vec<(String, bool)>> {
        let by_note = routines.today_by_note(&wall)?;
        let rows = by_note.iter().find(|(id, _)| *id == note);
        Ok(rows
            .map(|(_, rows)| rows.as_slice())
            .unwrap_or_default()
            .iter()
            .map(|r| (r.occurrence.key(), r.done().is_some()))
            .collect())
    };
    assert_eq!(ticked()?, [("2026-01-01T08:00".to_string(), false)]);
    let at = |hour| Occurrence::new(Day::new(2026, 1, 1), ClockTime::new(hour, 0));
    routines.mark_done(&routine.id, &at(8), None)?;
    assert_eq!(ticked()?, [("2026-01-01T20:00".to_string(), false)]);
    routines.mark_done(&routine.id, &at(20), None)?;
    assert_eq!(ticked()?, [("2026-01-01T20:00".to_string(), true)]);
    Ok(())
}

/// A one-time Reminder at 08:00 on 2026-01-01 and its Occurrence.
fn once_at_eight() -> (Schedule, Occurrence) {
    let eight = ClockTime::new(8, 0).into_iter().collect::<Vec<_>>();
    let Some(schedule) = Schedule::new(Day::new(2026, 1, 1), &eight, None) else {
        panic!("a schedule");
    };
    (
        schedule,
        Occurrence::new(Day::new(2026, 1, 1), ClockTime::new(8, 0)),
    )
}

fn trashed(store: &Store, routine: &Routine) -> Result<bool> {
    Ok(Notes::new(store)
        .by_id(&routine.note_id)?
        .is_some_and(|n| n.is_trashed()))
}

#[test]
fn done_moves_nothing_by_default() -> Result<()> {
    let store = crate::store::tests::store();
    let routines = Routines::new(&store);
    let (once, eight) = once_at_eight();
    let dentist = routines.create("dentist", &once)?;
    assert!(!PersonSettings::new(&store).trashes_done_reminder()?);
    assert!(!routines.check_off(&dentist.id, &eight, None)?);
    let done = routines.by_id(&dentist.id)?;
    assert!(done.is_some_and(|r| r.done_at(&eight).is_some()));
    assert!(!trashed(&store, &dentist)?);
    Ok(())
}

#[test]
fn done_on_a_notes_only_reminder_trashes_it_when_chosen() -> Result<()> {
    let store = crate::store::tests::store();
    let routines = Routines::new(&store);
    let (once, eight) = once_at_eight();
    let dentist = routines.create("dentist", &once)?;
    PersonSettings::new(&store).set_trashes_done_reminder(true)?;
    assert!(routines.check_off(&dentist.id, &eight, None)?);
    assert!(trashed(&store, &dentist)?);
    let gone = routines
        .by_id(&dentist.id)?
        .ok_or(Error::Invalid("gone".into()))?;
    assert!(gone.done_at(&eight).is_some());
    assert_eq!(gone.trashed_with.as_deref(), Some(dentist.note_id.as_str()));
    // No unchecking brings it back: it comes back from the trash, Done.
    assert!(routines.unmark_done(&dentist.id, &eight).is_err());
    routines.restore_note(&dentist.note_id)?;
    assert!(!trashed(&store, &dentist)?);
    let back = routines.by_id(&dentist.id)?;
    assert!(back.is_some_and(|r| r.done_at(&eight).is_some()));
    routines.unmark_done(&dentist.id, &eight)?;
    assert!(!trashed(&store, &dentist)?);
    // A notification's Done, applied later, trashes too.
    let flyer = routines.create("flyer", &once)?;
    let at = store.now();
    assert!(crate::Alarms::new(&store).apply_done(&flyer.id, &eight.key(), at)?);
    assert!(trashed(&store, &flyer)?);
    Ok(())
}

#[test]
fn a_routine_a_second_schedule_or_a_note_of_a_kind_never_go() -> Result<()> {
    use crate::{Checklists, SessionRule, Sessions, ShoppingLists, Tallies};
    let store = crate::store::tests::store();
    let routines = Routines::new(&store);
    let notes = Notes::new(&store);
    let (once, eight) = once_at_eight();
    PersonSettings::new(&store).set_trashes_done_reminder(true)?;
    let pills = routines.create("pills", &daily_at_eight())?;
    let twice = routines.create("passport", &once)?;
    routines.spawn(&twice.note_id, &daily_at_eight())?;
    let packing = routines.create("packing", &once)?;
    Checklists::new(&store).make(&packing.note_id)?;
    let groceries = routines.create("groceries", &once)?;
    ShoppingLists::new(&store).make(&groceries.note_id)?;
    let gym = routines.create("gym", &once)?;
    Tallies::new(&store).add_field(&gym.note_id, "Push-ups", "times", None, None)?;
    let squats = notes.capture("squats", None, None)?.id;
    Tallies::new(&store).add_field(&squats, "Squats", "times", None, None)?;
    let workout = routines.create("workout", &once)?;
    Sessions::new(&store).make(&workout.note_id, &[squats], 1, SessionRule::Least)?;
    for r in [&pills, &twice, &packing, &groceries, &gym, &workout] {
        assert!(!routines.check_off(&r.id, &eight, None)?, "{}", r.note_id);
        assert!(!trashed(&store, r)?, "{}", r.note_id);
        let done = routines.by_id(&r.id)?;
        assert!(
            done.is_some_and(|d| d.done_at(&eight).is_some()),
            "{}",
            r.note_id
        );
    }
    // Its other schedule in the trash, the Reminder is the only one.
    let last = routines.create("last one", &once)?;
    let other = routines.spawn(&last.note_id, &daily_at_eight())?;
    routines.trash(&other.id)?;
    assert!(routines.check_off(&last.id, &eight, None)?);
    Ok(())
}

#[test]
fn done_and_its_undoing_and_the_trash() -> Result<()> {
    let store = crate::store::tests::store();
    let routines = Routines::new(&store);
    let routine = routines.create("  Water  ", &daily_at_eight())?;
    let note = routines.note(&routine)?;
    assert_eq!(note.map(|n| n.text), Some("Water".to_string()));
    let o = Occurrence::new(Day::new(2026, 1, 2), ClockTime::new(8, 0));
    routines.mark_done(&routine.id, &o, None)?;
    assert!(
        routines
            .by_id(&routine.id)?
            .is_some_and(|r| r.done_at(&o).is_some())
    );
    let not_due = Occurrence::new(Day::new(2026, 1, 2), ClockTime::new(9, 0));
    assert!(routines.mark_done(&routine.id, &not_due, None).is_err());
    routines.unmark_done(&routine.id, &o)?;
    assert!(
        routines
            .by_id(&routine.id)?
            .is_some_and(|r| r.done_at(&o).is_none())
    );
    routines.trash(&routine.id)?;
    assert!(routines.all()?.is_empty());
    assert_eq!(routines.trashed()?.len(), 1);
    routines.restore(&routine.id)?;
    assert_eq!(routines.all()?.len(), 1);
    assert!(
        routines
            .set_renag(&routine.id, Some(Duration::seconds(5)))
            .is_err()
    );
    routines.set_renag(&routine.id, Some(Duration::minutes(5)))?;
    routines.set_escalation(&routine.id, Some(3))?;
    let read = routines.by_id(&routine.id)?;
    assert_eq!(
        read.as_ref().and_then(|r| r.renag),
        Some(Duration::minutes(5))
    );
    assert_eq!(read.and_then(|r| r.escalation), Some(3));
    Ok(())
}

#[test]
fn a_note_trashed_with_its_routines_brings_them_back() -> Result<()> {
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    let routines = Routines::new(&store);
    let note = notes.capture("Walk the dog", None, None)?;
    routines.spawn(&note.id, &daily_at_eight())?;
    routines.trash_note(&note.id)?;
    assert!(routines.all()?.is_empty());
    // Trashed with the Note, they come back with it, not on their own.
    assert!(routines.trashed()?.is_empty());
    routines.restore_note(&note.id)?;
    assert_eq!(routines.all()?.len(), 1);
    // One left behind before #262 stays, linking to the Note in the
    // trash.
    notes.trash(&note.id)?;
    assert_eq!(routines.all()?.len(), 1);
    Ok(())
}

#[test]
fn a_place_sending_its_notes_to_the_trash_takes_their_routines_along() -> Result<()> {
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    let routines = Routines::new(&store);
    let places = crate::places::Places::new(&store);
    let health = places.add_area("Health")?;
    let note = notes.capture("pills", Some(&health.id), None)?;
    routines.spawn(&note.id, &daily_at_eight())?;
    places.delete_area(&health.id, crate::places::NotesGo::Trash)?;
    assert!(routines.all()?.is_empty());
    routines.restore_note(&note.id)?;
    assert_eq!(routines.of_note(&note.id)?.len(), 1);
    Ok(())
}

/// Fridays at 08:00 from 2 January 2026.
fn fridays() -> Result<Schedule> {
    let rule =
        Recurrence::parse("FREQ=WEEKLY;BYDAY=FR").map_err(|e| Error::Invalid(e.to_string()))?;
    let eight = ClockTime::new(8, 0).into_iter().collect::<Vec<_>>();
    Schedule::new(Day::new(2026, 1, 2), &eight, Some(rule))
        .ok_or_else(|| Error::Invalid("a schedule".into()))
}

fn store_at(now: &str) -> Result<Store> {
    let now = parse_time(now)?;
    Store::in_memory(Box::new(move || now), || {
        Ok(crate::signing::SigningKey::from_seed([1; 32]))
    })
}

fn weekly_store() -> Result<(Store, String)> {
    let store = store_at("2026-01-02T06:00:00.000Z")?;
    let id = Routines::new(&store).create("bins", &fridays()?)?.id;
    Ok((store, id))
}

#[test]
fn a_checklists_and_a_sessions_moved_occurrence_is_theirs_to_check() -> Result<()> {
    // Saturday, the day after the one moved.
    let store = store_at("2026-01-03T09:00:00.000Z")?;
    let wall = |at: DateTime<Utc>| at.naive_utc();
    let note = Notes::new(&store)
        .capture("Before the bins\nRinse\nSort", None, None)?
        .id;
    let lists = crate::Checklists::new(&store);
    lists.make(&note)?;
    let bins = Routines::new(&store).spawn(&note, &fridays()?)?.id;
    Routines::new(&store).tomorrow(&bins, &friday(0), None)?;
    let on = lists
        .occurrence_now(&note, &wall)?
        .ok_or_else(|| Error::Invalid("an Occurrence now".into()))?;
    assert_eq!(on.occurrence, friday(1));
    let list = lists
        .by_note(&note)?
        .ok_or_else(|| Error::Invalid("a Checklist".into()))?;
    for line in &list.lines {
        lists.check_for(&line.id, &on)?;
    }
    let read = Routines::new(&store).by_id(&bins)?;
    assert!(read.is_some_and(|r| r.done_at(&friday(1)).is_some()));

    let push_ups = Notes::new(&store).capture("Push-ups", None, None)?.id;
    crate::Tallies::new(&store).add_field(&push_ups, "Push-ups", "times", Some(20.0), None)?;
    let sessions = crate::Sessions::new(&store);
    let session = sessions.create(
        "Sport",
        &fridays()?,
        std::slice::from_ref(&push_ups),
        1,
        crate::SessionRule::Rolling,
    )?;
    assert!(sessions.on(session.id(), &friday(1)).is_err());
    Routines::new(&store).tomorrow(session.id(), &friday(0), None)?;
    assert!(sessions.on(session.id(), &friday(1)).is_ok());
    Ok(())
}

fn friday(days: i64) -> Occurrence {
    Occurrence::new(Day::new(2026, 1, 2).add_days(days), ClockTime::new(8, 0))
}

#[test]
fn tomorrow_adds_the_next_day_and_moves_on_and_nothing_else_is_planted() -> Result<()> {
    let (store, id) = weekly_store()?;
    let routines = Routines::new(&store);
    routines.tomorrow(&id, &friday(0), None)?;
    routines.tomorrow(&id, &friday(1), None)?;
    // As no app writes it: Tomorrow on what is no Occurrence.
    store.append(
        &id,
        &format!("{}2026-01-05T03:00", keys::SKIP_PREFIX),
        Some("2026-01-02T06:00:00.000Z tomorrow"),
        None,
    )?;
    let routine = routines.by_id(&id)?.ok_or(Error::Invalid("gone".into()))?;
    assert_eq!(routine.moved, [friday(1), friday(2)]);
    assert!(routine.skipped_at(&friday(1)).is_some());
    assert_eq!(routine.first_from(Day::new(2026, 1, 5)), Some(friday(7)));
    assert!(
        routine
            .schedule
            .rule
            .as_ref()
            .is_some_and(|r| r.format() == "FREQ=WEEKLY;BYDAY=FR")
    );
    routines.unskip(&id, &friday(0))?;
    let back = routines.by_id(&id)?.ok_or(Error::Invalid("gone".into()))?;
    assert!(back.moved.is_empty(), "the chain goes with its start");
    Ok(())
}

#[test]
fn only_an_occurrence_due_and_not_done_is_skipped() -> Result<()> {
    let (store, id) = weekly_store()?;
    let routines = Routines::new(&store);
    assert!(routines.skip(&id, &friday(1), None).is_err());
    routines.mark_done(&id, &friday(0), None)?;
    assert!(routines.skip(&id, &friday(0), None).is_err());
    assert!(routines.tomorrow(&id, &friday(0), None).is_err());
    let written = store.all_entries()?.len();
    routines.unskip(&id, &friday(7))?;
    assert_eq!(store.all_entries()?.len(), written, "nothing held");
    Ok(())
}

#[test]
fn a_skip_value_reads_as_the_dart_core_reads_it() -> Result<()> {
    let at = parse_time("2026-03-02T06:00:00.000123Z")?;
    assert_eq!(
        skip_of("2026-03-02T06:00:00.000123Z later"),
        Some((at, false))
    );
    assert_eq!(
        skip_of("2026-03-02T06:00:00.000123Z tomorrow"),
        Some((at, true))
    );
    for value in [
        "Rude",
        "2026-02-30T06:00:00.000Z tomorrow",
        "2026-03-02T24:00:00Z",
        "2026-03-02T06:00:60Z",
        "2026-03-02T06:00:00+01:00",
        "2026-03-02 06:00:00Z",
        " tomorrow",
    ] {
        assert_eq!(skip_of(value), None, "{value}");
    }
    Ok(())
}

#[test]
fn over_a_week_the_days_after_tomorrow_cross_a_month_end() -> Result<()> {
    let now = parse_time("2026-10-28T11:00:00.000Z")?;
    let store = Store::in_memory(Box::new(move || now), || {
        Ok(crate::signing::SigningKey::from_seed([1; 32]))
    })?;
    let routines = Routines::new(&store);
    routines.create("pills", &daily_at_eight())?;
    let Some(once) = Schedule::new(Day::new(2026, 11, 2), &[], None) else {
        panic!("a schedule");
    };
    routines.create("dentist", &once)?;
    let wall = |at: DateTime<Utc>| at.naive_utc();
    assert!(routines.agenda(&wall)?.later_rows.is_empty());
    let agenda = routines.agenda_over(&wall, 7)?;
    assert_eq!(agenda.today, Day::new(2026, 10, 28));
    assert_eq!(agenda.later_rows.len(), 5);
    let days: Vec<String> = (0..5).map(|i| agenda.later(i).to_string()).collect();
    assert_eq!(
        days,
        [
            "2026-10-30",
            "2026-10-31",
            "2026-11-01",
            "2026-11-02",
            "2026-11-03"
        ]
    );
    let texts = |rows: &[AgendaRow]| -> Vec<String> {
        rows.iter()
            .map(|r| r.note.as_ref().map_or(String::new(), |n| n.text.clone()))
            .collect()
    };
    assert_eq!(texts(&agenda.later_rows[1]), ["pills"]);
    // All-day first, then by time.
    assert_eq!(texts(&agenda.later_rows[3]), ["dentist", "pills"]);
    Ok(())
}

/// The ids of `names`, given in this order, listed by name.
fn named_order(names: &[(&str, &str)]) -> Vec<String> {
    let given = names.iter().map(|(id, _)| Routine {
        id: id.to_string(),
        note_id: format!("note:{id}"),
        schedule: daily_at_eight(),
        done: BTreeMap::new(),
        renag: None,
        escalation: None,
        scheduled_at: None,
        trashed: None,
        trashed_with: None,
        skipped: BTreeMap::new(),
        tomorrow: Default::default(),
        moved: Vec::new(),
    });
    let name_of = |r: &Routine| {
        names
            .iter()
            .find(|(id, _)| *id == r.id)
            .map(|(_, name)| name.to_string())
            .unwrap_or_default()
    };
    routines_by_name(given, name_of)
        .into_iter()
        .map(|r| r.id)
        .collect()
}

#[test]
fn by_name_alphabetical_whatever_the_case() {
    let order = named_order(&[
        ("routine:1", "Cherry"),
        ("routine:2", "apple"),
        ("routine:3", "Banana"),
    ]);
    assert_eq!(order, ["routine:2", "routine:3", "routine:1"]);
}

#[test]
fn by_name_equal_names_go_by_id() {
    let order = named_order(&[
        ("routine:b", "Pills"),
        ("routine:a", "pills"),
        ("routine:c", "PILLS"),
    ]);
    assert_eq!(order, ["routine:a", "routine:b", "routine:c"]);
}

#[test]
fn by_name_a_name_counts_by_its_first_line_trimmed() {
    let order = named_order(&[
        ("routine:1", "  Zebra"),
        ("routine:2", "Mango\nAardvark"),
        ("routine:3", "\u{feff}\n Kiwi "),
    ]);
    assert_eq!(order, ["routine:3", "routine:2", "routine:1"]);
}

#[test]
fn by_name_lowers_as_dart_does_and_compares_utf16() {
    // Dart lowers ΣΑΣ to σασ, with no final sigma, so σας comes first;
    // U+1F34E comes before U+FF41 in UTF-16, not in code points.
    let order = named_order(&[
        ("routine:1", "ΣΑΣ"),
        ("routine:2", "σας"),
        ("routine:3", "\u{ff21}"),
        ("routine:4", "\u{1f34e}"),
        ("routine:5", "Ärzte"),
        ("routine:6", "zoo"),
    ]);
    assert_eq!(
        order,
        [
            "routine:6",
            "routine:5",
            "routine:2",
            "routine:1",
            "routine:4",
            "routine:3"
        ]
    );
}
