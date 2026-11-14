use super::*;
use oldbrainfix_core::alarms::{AlarmKind, WallMoment};
use oldbrainfix_core::{ClockTime, Day, Wordless, parse_time};

fn alarm(at: &str, ends: &str, escalation: Option<i64>) -> oldbrainfix_core::Result<PlannedAlarm> {
    let day = Day::new(2026, 1, 1);
    let moment = WallMoment {
        day,
        time: ClockTime { hour: 9, minute: 0 },
    };
    Ok(PlannedAlarm {
        kind: AlarmKind::Occurrence,
        routine_id: "routine:a".into(),
        occurrence: Occurrence::new(day, ClockTime::new(9, 0)),
        at: parse_time(at)?,
        wall: Some(moment),
        renag: Duration::minutes(10),
        escalation,
        ends: moment,
        ends_at: parse_time(ends)?,
        text: Some("Pills".into()),
        wordless: Wordless::Photo,
        wordless_name: String::new(),
    })
}

#[test]
fn an_occurrence_shows_once_per_tick_however_many_renags_passed() -> oldbrainfix_core::Result<()> {
    let a = alarm(
        "2026-01-01T09:00:00.000Z",
        "2026-01-01T12:00:00.000Z",
        Some(2),
    )?;
    let plan = vec![a.clone()];
    let t = |s: &str| parse_time(s);
    assert_eq!(
        due(
            &plan,
            t("2026-01-01T08:00:00.000Z")?,
            t("2026-01-01T08:59:00.000Z")?
        )
        .len(),
        0
    );
    assert_eq!(
        due(
            &plan,
            t("2026-01-01T08:00:00.000Z")?,
            t("2026-01-01T09:45:00.000Z")?
        )
        .len(),
        1
    );
    assert_eq!(
        next(&plan, t("2026-01-01T09:00:00.000Z")?),
        Some(t("2026-01-01T09:10:00.000Z")?)
    );
    assert!(!escalates(&a, t("2026-01-01T09:20:00.000Z")?));
    assert!(escalates(&a, t("2026-01-01T09:30:00.000Z")?));
    assert!(!escalates(&a, t("2026-01-01T12:00:00.000Z")?));
    // Missed while not running: told once, not rung.
    assert_eq!(
        catch_up(
            &plan,
            t("2026-01-01T08:00:00.000Z")?,
            t("2026-01-01T13:00:00.000Z")?
        ),
        1
    );
    assert_eq!(
        catch_up(
            &plan,
            t("2026-01-01T08:00:00.000Z")?,
            t("2026-01-01T11:00:00.000Z")?
        ),
        0
    );
    assert_eq!(
        catch_up(
            &plan,
            t("2026-01-01T14:00:00.000Z")?,
            t("2026-01-01T13:00:00.000Z")?
        ),
        0
    );
    Ok(())
}

#[test]
fn an_alarm_of_a_note_without_words_names_what_it_holds() -> oldbrainfix_core::Result<()> {
    let notifier = crate::notify::RecordingNotifier::default();
    let changed = Arc::new(AtomicU64::new(0));
    let mut alerts = Alerts::new(Box::new(notifier.clone()), L10n::new("de"), changed);
    let now = parse_time("2026-01-01T09:00:00.000Z")?;
    let voice = PlannedAlarm {
        text: Some(" \n".into()),
        wordless: Wordless::VoiceRecording,
        ..alarm("2026-01-01T09:00:00.000Z", "2026-01-01T12:00:00.000Z", None)?
    };
    let photo = PlannedAlarm {
        wordless: Wordless::Photo,
        ..voice.clone()
    };
    let file = PlannedAlarm {
        wordless: Wordless::File,
        wordless_name: "Mietvertrag.pdf".into(),
        ..voice.clone()
    };
    assert!(alerts.show_alarm(&voice, now));
    assert!(alerts.show_alarm(&photo, now));
    assert!(alerts.show_alarm(&file, now));
    let titles: Vec<String> = notifier
        .take()
        .into_iter()
        .filter_map(|r| match r {
            crate::notify::Recorded::Shown(notice) => Some(notice.title),
            crate::notify::Recorded::Closed(_) => None,
        })
        .collect();
    assert_eq!(titles, ["Sprachaufnahme", "Foto", "Mietvertrag.pdf"]);
    Ok(())
}

#[test]
fn an_escalated_alarm_leaves_its_sound_to_the_system() -> oldbrainfix_core::Result<()> {
    let notifier = crate::notify::RecordingNotifier::default();
    let changed = Arc::new(AtomicU64::new(0));
    let mut alerts = Alerts::new(Box::new(notifier.clone()), L10n::new("en"), changed);
    let a = alarm(
        "2026-01-01T09:00:00.000Z",
        "2026-01-01T12:00:00.000Z",
        Some(2),
    )?;
    assert!(alerts.show_alarm(&a, parse_time("2026-01-01T09:00:00.000Z")?));
    assert!(!alerts.show_alarm(&a, parse_time("2026-01-01T09:30:00.000Z")?));
    let sounds: Vec<(bool, bool)> = notifier
        .take()
        .into_iter()
        .filter_map(|r| match r {
            crate::notify::Recorded::Shown(notice) => Some((notice.urgent, notice.silent)),
            crate::notify::Recorded::Closed(_) => None,
        })
        .collect();
    // Silent while the app rings; urgent with the system's sound.
    assert_eq!(sounds, [(false, true), (true, false)]);
    Ok(())
}

#[test]
fn not_today_offers_tomorrow_only_from_today_on() {
    let t = L10n::new("en");
    let monday = Day::new(2026, 10, 26);
    let eight = Occurrence::new(monday, ClockTime::new(8, 0));
    let ids = |today: Day| -> Vec<String> {
        choices(&t, NOT_TODAY, &eight, today)
            .map(|(_, actions)| actions.into_iter().map(|(id, _)| id).collect())
            .unwrap_or_default()
    };
    assert_eq!(ids(monday), [SKIP, TOMORROW, OPEN]);
    // A Re-nag after midnight of the day before's.
    assert_eq!(ids(monday.add_days(1)), [SKIP, OPEN]);
    assert!(choices(&t, DONE, &eight, monday).is_none());
    let first: Vec<String> = occurrence_buttons(&t)
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    // Three buttons and Open, the click on the notification itself.
    assert_eq!(first, [DONE, SNOOZE, NOT_TODAY, OPEN]);
}

#[test]
fn snooze_buttons_wait_what_they_say() {
    assert_eq!(snooze_of(SNOOZE_10), Some(Duration::minutes(10)));
    assert_eq!(snooze_of(SNOOZE_60), Some(Duration::hours(1)));
    assert_eq!(snooze_of(DONE), None);
    assert_eq!(
        split_key("routine:a@2026-01-01T09:00"),
        Some(("routine:a", "2026-01-01T09:00"))
    );
    assert_eq!(first_line(Some("  \n")), None);
    assert_eq!(first_line(Some(" Pills \n and more")), Some("Pills".into()));
}
