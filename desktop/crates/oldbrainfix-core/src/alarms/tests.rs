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
