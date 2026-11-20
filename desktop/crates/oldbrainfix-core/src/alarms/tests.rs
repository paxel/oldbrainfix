use super::*;

fn utc_wall(utc: DateTime<Utc>) -> NaiveDateTime {
    utc.naive_utc()
}

fn alarm(at: &str, renag: i64, ends: &str) -> Result<PlannedAlarm> {
    let moment = WallMoment {
        day: Day::new(2026, 1, 1),
        time: crate::settings::DEFAULT_ALL_DAY_FROM,
    };
    Ok(PlannedAlarm {
        kind: AlarmKind::Occurrence,
        routine_id: "routine:a".into(),
        occurrence: Occurrence::new(Day::new(2026, 1, 1), None),
        at: parse_time(at)?,
        wall: Some(moment),
        renag: Duration::minutes(renag),
        escalation: Some(2),
        ends: moment,
        ends_at: parse_time(ends)?,
        quiet: None,
        text: None,
        wordless: Wordless::Photo,
        wordless_name: String::new(),
    })
}

#[test]
fn an_alarm_renags_until_it_counts_as_missed() -> Result<()> {
    let a = alarm("2026-01-01T09:00:00.000Z", 10, "2026-01-01T09:35:00.000Z")?;
    let t = |s: &str| parse_time(s);
    assert_eq!(a.fires_after(t("2026-01-01T08:00:00.000Z")?), Some(a.at));
    assert_eq!(
        a.fires_after(t("2026-01-01T09:00:00.000Z")?),
        Some(t("2026-01-01T09:10:00.000Z")?)
    );
    assert_eq!(a.fires_after(t("2026-01-01T09:30:00.000Z")?), None);
    assert_eq!(a.escalates_at(), Some(t("2026-01-01T09:30:00.000Z")?));
    Ok(())
}

#[test]
fn an_alarm_names_what_its_note_without_words_holds() -> Result<()> {
    let store = crate::store::tests::store();
    let notes = Notes::new(&store);
    let routines = Routines::new(&store);
    let memo = notes.capture_with("", None, None, &[], &[vec![0x52, 0x49, 0x46, 0x46]])?;
    let receipt = notes.capture_with("", None, None, &[vec![0xFF, 0xD8, 0xFF, 0xD9]], &[])?;
    let ten = ClockTime::new(10, 0).into_iter().collect::<Vec<_>>();
    let schedule = crate::schedule::Schedule::new(Day::new(2026, 1, 2), &ten, None)
        .ok_or_else(|| Error::Invalid("a schedule".into()))?;
    let lease = notes.capture_with_files(
        "",
        None,
        None,
        &[],
        &[],
        &[crate::notes::NewFile {
            name: "Lease.pdf",
            media_type: "",
            bytes: &[1],
            pages: None,
        }],
    )?;
    let voice = routines.spawn(&memo.id, &schedule)?.id;
    let photo = routines.spawn(&receipt.id, &schedule)?.id;
    let file = routines.spawn(&lease.id, &schedule)?.id;
    let titled: BTreeMap<String, (Wordless, String)> = Alarms::new(&store)
        .plan(&utc_wall)?
        .into_iter()
        .map(|a| (a.routine_id, (a.wordless, a.wordless_name)))
        .collect();
    assert_eq!(
        titled,
        BTreeMap::from([
            (voice, (Wordless::VoiceRecording, String::new())),
            (photo, (Wordless::Photo, String::new())),
            (file, (Wordless::File, "Lease.pdf".to_string())),
        ])
    );
    Ok(())
}

#[test]
fn an_alarm_names_a_formatted_note_as_it_reads() -> Result<()> {
    let store = crate::store::tests::store();
    let note = Notes::new(&store).capture_as(
        "**pills** ~~now~~",
        crate::text_format::MARKDOWN,
        None,
        None,
    )?;
    let ten = ClockTime::new(10, 0).into_iter().collect::<Vec<_>>();
    let schedule = crate::schedule::Schedule::new(Day::new(2026, 1, 2), &ten, None)
        .ok_or_else(|| Error::Invalid("a schedule".into()))?;
    Routines::new(&store).spawn(&note.id, &schedule)?;
    let texts: Vec<Option<String>> = Alarms::new(&store)
        .plan(&utc_wall)?
        .into_iter()
        .map(|a| a.text)
        .collect();
    assert_eq!(texts.first(), Some(&Some("pills now".to_string())));
    Ok(())
}

#[test]
fn a_skipped_time_fires_when_the_clock_jumps() {
    let day = Day::new(2026, 3, 29);
    let half_past_two = ClockTime {
        hour: 2,
        minute: 30,
    };
    assert_eq!(
        instant_of(day, half_past_two, &utc_wall),
        utc_at(day, half_past_two)
    );
}

/// An all-day Reminder today, 1 January, made at 10:00; its one alarm.
fn all_day_today(store: &Store) -> Result<Vec<PlannedAlarm>> {
    let note = Notes::new(store).capture("bins out", None, None)?;
    let schedule = crate::schedule::Schedule::new(Day::new(2026, 1, 1), &[], None)
        .ok_or_else(|| Error::Invalid("a schedule".into()))?;
    Routines::new(store).spawn(&note.id, &schedule)?;
    Alarms::new(store).plan(&utc_wall)
}

#[test]
fn an_all_day_occurrence_rings_from_15_and_renags_after() -> Result<()> {
    let store = crate::store::tests::store();
    let [alarm] = &all_day_today(&store)?[..] else {
        return Err(Error::Invalid("one alarm".into()));
    };
    let three = parse_time("2026-01-01T15:00:00.000Z")?;
    assert_eq!(alarm.at, three);
    assert_eq!(
        alarm.wall.map(|w| w.time),
        Some(ClockTime {
            hour: 15,
            minute: 0
        })
    );
    assert_eq!(alarm.fires_after(store.now()), Some(three), "none before");
    assert_eq!(
        alarm.fires_after(three),
        Some(three + Duration::minutes(10))
    );
    Ok(())
}

#[test]
fn an_all_day_occurrence_rings_from_the_devices_own_time() -> Result<()> {
    let store = crate::store::tests::store();
    let half_past_eleven = ClockTime {
        hour: 11,
        minute: 30,
    };
    DeviceSettings::new(&store).set_all_day_from(half_past_eleven)?;
    let plan = all_day_today(&store)?;
    assert_eq!(plan.len(), 1);
    assert_eq!(plan[0].at, parse_time("2026-01-01T11:30:00.000Z")?);
    assert_eq!(plan[0].wall.map(|w| w.time), Some(half_past_eleven));
    Ok(())
}

#[test]
fn quiet_hours_win_over_the_all_day_time() -> Result<()> {
    let store = crate::store::tests::store();
    let settings = DeviceSettings::new(&store);
    let end = ClockTime {
        hour: 16,
        minute: 30,
    };
    settings.set_quiet_hours(QuietHours {
        start: ClockTime {
            hour: 14,
            minute: 0,
        },
        end,
    })?;
    let plan = all_day_today(&store)?;
    assert_eq!(plan.len(), 1);
    assert_eq!(plan[0].at, parse_time("2026-01-01T16:30:00.000Z")?);
    assert_eq!(
        plan[0].wall,
        Some(WallMoment {
            day: Day::new(2026, 1, 1),
            time: end
        })
    );
    settings.set_quiet_hours_on(false)?;
    let off = Alarms::new(&store).plan(&utc_wall)?;
    assert_eq!(off[0].at, parse_time("2026-01-01T15:00:00.000Z")?);
    // Lasting past midnight, they leave it to the Agenda.
    settings.set_quiet_hours_on(true)?;
    settings.set_quiet_hours(QuietHours {
        start: ClockTime {
            hour: 14,
            minute: 0,
        },
        end: ClockTime { hour: 8, minute: 0 },
    })?;
    assert!(Alarms::new(&store).plan(&utc_wall)?.is_empty());
    Ok(())
}

#[test]
fn an_alarm_rings_silent_in_the_quiet_hours_unless_it_escalates() -> Result<()> {
    let store = crate::store::tests::store();
    let quiet = QuietHours {
        start: ClockTime {
            hour: 17,
            minute: 0,
        },
        end: ClockTime {
            hour: 18,
            minute: 0,
        },
    };
    DeviceSettings::new(&store).set_quiet_hours(quiet)?;
    let plan = all_day_today(&store)?;
    assert_eq!(plan[0].quiet, Some(quiet));
    let t = |s: &str| parse_time(s);
    assert!(!plan[0].silent_at(t("2026-01-01T15:00:00.000Z")?, &utc_wall));
    assert!(plan[0].silent_at(t("2026-01-01T17:00:00.000Z")?, &utc_wall));
    assert!(!plan[0].silent_at(t("2026-01-01T18:00:00.000Z")?, &utc_wall));
    // Escalation is the only way through.
    let routines = Routines::new(&store);
    routines.set_escalation(&plan[0].routine_id, Some(3))?;
    let escalating = Alarms::new(&store).plan(&utc_wall)?;
    assert_eq!(escalating[0].quiet, None);
    assert!(!escalating[0].silent_at(t("2026-01-01T17:00:00.000Z")?, &utc_wall));
    // Off, nothing is silent.
    routines.set_escalation(&plan[0].routine_id, None)?;
    DeviceSettings::new(&store).set_quiet_hours_on(false)?;
    assert_eq!(Alarms::new(&store).plan(&utc_wall)?[0].quiet, None);
    Ok(())
}

#[test]
fn snoozes_are_kept_on_this_device() -> Result<()> {
    let store = crate::store::tests::store();
    let alarms = Alarms::new(&store);
    let until = parse_time("2026-01-01T11:00:00.000Z")?;
    alarms.apply_snooze("routine:a", "2026-01-01T10:00", until)?;
    alarms.apply_snooze("routine:a", "not a key", until)?;
    assert_eq!(
        alarms.snoozes()?.into_iter().collect::<Vec<_>>(),
        vec![("routine:a@2026-01-01T10:00".to_string(), until)]
    );
    alarms.apply_nudge_snooze(until, until)?;
    assert_eq!(
        alarms.nudge_snooze()?,
        Some(NudgeSnooze { at: until, until })
    );
    Ok(())
}

// ------------------------------------------------------------ Heads-ups

/// Central European time, as the fixtures keep it.
fn europe(utc: DateTime<Utc>) -> NaiveDateTime {
    utc.with_timezone(&chrono_tz::Europe::Berlin).naive_local()
}

fn time(hour: u32, minute: u32) -> ClockTime {
    ClockTime { hour, minute }
}

/// A store whose clock stands at `now`.
fn store_at(now: &str) -> Result<Store> {
    let now = parse_time(now)?;
    Store::in_memory(Box::new(move || now), || {
        Ok(crate::signing::SigningKey::from_seed([1; 32]))
    })
}

/// A Reminder of a new Note `text` on `day`, at `at` or all day.
fn reminder(store: &Store, text: &str, day: Day, at: Option<ClockTime>) -> Result<String> {
    let note = Notes::new(store).capture(text, None, None)?;
    let schedule = crate::schedule::Schedule::new(day, &at.into_iter().collect::<Vec<_>>(), None)
        .ok_or_else(|| Error::Invalid("a schedule".into()))?;
    Ok(Routines::new(store).spawn(&note.id, &schedule)?.id)
}

/// A Routine of a new Note `text` every day from 1 October 2026 at
/// `times`.
fn daily(store: &Store, text: &str, times: &[ClockTime]) -> Result<String> {
    let note = Notes::new(store).capture(text, None, None)?;
    let rule = crate::schedule::Recurrence::parse("FREQ=DAILY")
        .map_err(|e| Error::Invalid(e.to_string()))?;
    let schedule = crate::schedule::Schedule::new(Day::new(2026, 10, 1), times, Some(rule))
        .ok_or_else(|| Error::Invalid("a schedule".into()))?;
    Ok(Routines::new(store).spawn(&note.id, &schedule)?.id)
}

/// The Heads-ups of `store` on the European clock, as "minutes
/// occurrence at wall".
fn heads_ups(store: &Store) -> Result<Vec<String>> {
    Ok(Alarms::new(store)
        .heads_ups(&europe)?
        .iter()
        .map(|h| {
            format!(
                "{} {} {} {}",
                h.minutes,
                h.occurrence.key(),
                dart_iso(h.at),
                h.wall
            )
        })
        .collect())
}

#[test]
fn heads_ups_are_kept_leniently_and_written_canonically() -> Result<()> {
    use crate::routines::heads_up_keys;
    let store = store_at("2026-10-24T07:00:00.000Z")?;
    let routines = Routines::new(&store);
    let id = reminder(&store, "dentist", Day::new(2026, 10, 26), Some(time(10, 0)))?;
    let entity = heads_up_keys::of(&id);
    assert!(routines.heads_ups(&id)?.is_empty());
    // As another version or a modified app might write them.
    store.append(
        &entity,
        heads_up_keys::HEADS_UPS,
        Some("1440,020,+5, 5,abc,0,43201,20,1440,,43200,-3,0x10"),
        None,
    )?;
    assert_eq!(routines.heads_ups(&id)?, [20, 1440, 43200]);
    assert!(heads_up_keys::read(None).is_empty());
    assert!(heads_up_keys::read(Some("")).is_empty());
    let written = |store: &Store| -> Result<Vec<Option<String>>> {
        Ok(store
            .all_entries()?
            .into_iter()
            .filter(|e| e.entity == entity)
            .map(|e| e.value)
            .collect())
    };
    routines.set_heads_ups(&id, &[1440, 20, 20, 0, -5, 50000])?;
    assert_eq!(routines.heads_ups(&id)?, [20, 1440]);
    let count = written(&store)?.len();
    assert_eq!(written(&store)?.last(), Some(&Some("20,1440".to_string())));
    routines.set_heads_ups(&id, &[20, 1440])?;
    assert_eq!(written(&store)?.len(), count, "the same again");
    routines.set_heads_ups(&id, &[])?;
    assert_eq!(written(&store)?.last(), Some(&None));
    assert!(routines.heads_ups(&id)?.is_empty());
    assert!(routines.set_heads_ups("routine:none", &[20]).is_err());
    // A Routine in the trash keeps them and plans none.
    routines.set_heads_ups(&id, &[20])?;
    routines.trash(&id)?;
    assert_eq!(routines.heads_ups(&id)?, [20]);
    assert!(heads_ups(&store)?.is_empty());
    Ok(())
}

#[test]
fn a_heads_up_comes_a_day_back_on_the_calendar_across_the_switch() -> Result<()> {
    let store = store_at("2026-10-24T07:00:00.000Z")?;
    let id = reminder(&store, "dentist", Day::new(2026, 10, 25), Some(time(10, 0)))?;
    Routines::new(&store).set_heads_ups(&id, &[20, 1440])?;
    // 10:00 on Sunday is 09:00 UTC in winter time; 10:00 the day before
    // is 08:00 UTC in summer time, 25 hours earlier.
    assert_eq!(
        heads_ups(&store)?,
        [
            "1440 2026-10-25T10:00 2026-10-24T08:00:00.000Z 2026-10-24T10:00",
            "20 2026-10-25T10:00 2026-10-25T08:40:00.000Z 2026-10-25T09:40",
        ]
    );
    let plan = Alarms::new(&store).heads_ups(&europe)?;
    assert_eq!(plan[0].id(), format!("headsUp:1440:{id}@2026-10-25T10:00"));
    assert_eq!(plan[0].start.time, time(10, 0));
    assert_eq!(plan[0].text.as_deref(), Some("dentist"));
    assert_eq!(
        Alarms::new(&store).plan(&europe)?.len(),
        1,
        "it still rings"
    );
    // Only those still ahead.
    let routines = Routines::new(&store);
    let read = vec![(
        routines
            .by_id(&id)?
            .ok_or_else(|| Error::Invalid("a Routine".into()))?,
        None,
        routines.heads_ups(&id)?,
    )];
    let at = |now: &str| -> Result<Vec<i64>> {
        let plan = heads_up_plan(
            &read,
            &BTreeMap::new(),
            parse_time(now)?,
            &europe,
            crate::settings::DEFAULT_ALL_DAY_FROM,
            None,
        );
        Ok(plan.iter().map(|h| h.minutes).collect())
    };
    assert_eq!(
        at("2026-10-24T08:00:00.000Z")?,
        [20],
        "the one due has rung"
    );
    assert!(at("2026-10-25T08:50:00.000Z")?.is_empty());
    Ok(())
}

#[test]
fn no_heads_up_at_or_after_its_occurrence_starts() -> Result<()> {
    let store = store_at("2026-03-28T12:00:00.000Z")?;
    // On 29 March the clock jumps from 02:00 to 03:00: 02:10 is skipped
    // and fires at 03:00, as the Occurrence itself does.
    let id = reminder(
        &store,
        "night train",
        Day::new(2026, 3, 29),
        Some(time(2, 30)),
    )?;
    Routines::new(&store).set_heads_ups(&id, &[20, 60])?;
    assert_eq!(
        heads_ups(&store)?,
        ["60 2026-03-29T02:30 2026-03-29T00:30:00.000Z 2026-03-29T01:30"]
    );
    Ok(())
}

#[test]
fn an_all_day_heads_up_takes_whole_days_when_its_ring_would_ring() -> Result<()> {
    let store = store_at("2026-10-24T07:00:00.000Z")?;
    let id = reminder(&store, "bins out", Day::new(2026, 10, 27), None)?;
    Routines::new(&store).set_heads_ups(&id, &[30, 1440, 2880])?;
    assert_eq!(
        heads_ups(&store)?,
        [
            "2880 2026-10-27 2026-10-25T14:00:00.000Z 2026-10-25T15:00",
            "1440 2026-10-27 2026-10-26T14:00:00.000Z 2026-10-26T15:00",
        ]
    );
    // From 07:00, in the Quiet Hours: once they end, at 08:00.
    let settings = DeviceSettings::new(&store);
    settings.set_all_day_from(time(7, 0))?;
    assert_eq!(
        heads_ups(&store)?,
        [
            "2880 2026-10-27 2026-10-25T07:00:00.000Z 2026-10-25T08:00",
            "1440 2026-10-27 2026-10-26T07:00:00.000Z 2026-10-26T08:00",
        ]
    );
    settings.set_quiet_hours_on(false)?;
    assert_eq!(
        heads_ups(&store)?,
        [
            "2880 2026-10-27 2026-10-25T06:00:00.000Z 2026-10-25T07:00",
            "1440 2026-10-27 2026-10-26T06:00:00.000Z 2026-10-26T07:00",
        ]
    );
    Ok(())
}

#[test]
fn a_heads_up_shows_silent_in_the_quiet_hours_whether_it_escalates_or_not() -> Result<()> {
    let store = store_at("2026-10-24T07:00:00.000Z")?;
    let id = reminder(&store, "pills", Day::new(2026, 10, 25), Some(time(8, 10)))?;
    let routines = Routines::new(&store);
    routines.set_heads_ups(&id, &[5, 20])?;
    let plan = Alarms::new(&store).heads_ups(&europe)?;
    let [early, late] = &plan[..] else {
        return Err(Error::Invalid("two Heads-ups".into()));
    };
    // 07:50 lies in 22:00-08:00; 08:05 does not.
    assert_eq!(early.minutes, 20);
    assert_eq!(early.quiet, Some(crate::settings::DEFAULT_QUIET_HOURS));
    assert!(early.silent_at(&europe));
    assert!(early.to_string().contains("silent in 22:00-08:00"));
    assert!(!late.silent_at(&europe));
    routines.set_escalation(&id, Some(3))?;
    assert!(Alarms::new(&store).heads_ups(&europe)?[0].silent_at(&europe));
    DeviceSettings::new(&store).set_quiet_hours_on(false)?;
    let off = Alarms::new(&store).heads_ups(&europe)?;
    assert_eq!(off[0].quiet, None);
    assert!(!off[0].silent_at(&europe));
    Ok(())
}

#[test]
fn done_skip_or_a_snooze_of_the_occurrence_cancels_its_heads_ups() -> Result<()> {
    let store = store_at("2026-10-24T07:00:00.000Z")?;
    let id = daily(&store, "feed the cat", &[time(18, 0)])?;
    let routines = Routines::new(&store);
    routines.set_heads_ups(&id, &[20])?;
    let on = |day: i64| Occurrence::new(Day::new(2026, 10, day), Some(time(18, 0)));
    let days = || -> Result<Vec<String>> {
        Ok(Alarms::new(&store)
            .heads_ups(&europe)?
            .iter()
            .take(2)
            .map(|h| h.occurrence.key())
            .collect())
    };
    assert_eq!(days()?, ["2026-10-24T18:00", "2026-10-25T18:00"]);
    routines.mark_done(&id, &on(24), None)?;
    routines.skip(&id, &on(25), None)?;
    Alarms::new(&store).apply_snooze(
        &id,
        &on(26).key(),
        parse_time("2026-10-26T18:00:00.000Z")?,
    )?;
    assert_eq!(days()?, ["2026-10-27T18:00", "2026-10-28T18:00"]);
    Ok(())
}

#[test]
fn a_routine_has_at_most_so_many_heads_ups() -> Result<()> {
    let store = store_at("2026-10-24T07:00:00.000Z")?;
    let times: Vec<ClockTime> = (0..6).map(|i| time(1 + 4 * i, 0)).collect();
    let id = daily(&store, "water", &times)?;
    Routines::new(&store).set_heads_ups(&id, &[10])?;
    assert_eq!(
        Alarms::new(&store).heads_ups(&europe)?.len(),
        ALARMS_PER_ROUTINE
    );
    Ok(())
}
