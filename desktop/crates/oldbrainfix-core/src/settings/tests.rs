use super::*;

#[test]
fn settings_fall_back_to_their_defaults() -> Result<()> {
    let store = crate::store::tests::store();
    let settings = DeviceSettings::new(&store);
    assert_eq!(settings.renag_interval()?, Duration::minutes(10));
    assert_eq!(settings.nudge_repeat()?, Duration::hours(1));
    assert_eq!(settings.quiet_hours()?, DEFAULT_QUIET_HOURS);
    assert!(settings.quiet_hours_on()?);
    assert_eq!(settings.all_day_from()?, DEFAULT_ALL_DAY_FROM);
    assert!(settings.alerts_on()?);
    assert!(settings.autostart_on()?);
    assert!(settings.streaks_on()?);
    assert!(!settings.agenda_hides_checked()?);
    assert!(!settings.fat_fingers()?);
    for control in QuickNoteControl::ALL {
        assert!(!settings.quick_note_shows(control)?, "{control:?}");
    }
    assert!(settings.quick_note_nudge_buttons_on()?);
    assert_eq!(settings.capture_hotkey()?, DEFAULT_CAPTURE_HOTKEY);
    assert_eq!(settings.missed_seen()?, None);
    assert_eq!(settings.theme()?, ThemeChoice::System);
    store.set_local_setting(THEME, "sepia")?;
    assert_eq!(settings.theme()?, ThemeChoice::System);
    store.set_local_setting(RENAG_MINUTES, "0")?;
    assert_eq!(settings.renag_interval()?, Duration::minutes(10));
    store.set_local_setting(QUIET_HOURS, "23:00")?;
    assert_eq!(settings.quiet_hours()?, DEFAULT_QUIET_HOURS);
    store.set_local_setting(ALL_DAY_FROM, "noon")?;
    assert_eq!(settings.all_day_from()?, DEFAULT_ALL_DAY_FROM);
    Ok(())
}

#[test]
fn the_day_starts_when_the_quiet_hours_end_at_nine_without_them() -> Result<()> {
    use chrono::TimeZone;
    let Some(now) = Utc.with_ymd_and_hms(2026, 11, 3, 13, 20, 59).single() else {
        return Err(Error::Invalid("now".into()));
    };
    let store = Store::in_memory(Box::new(move || now), || {
        Ok(crate::signing::SigningKey::from_seed([1; 32]))
    })?;
    let settings = DeviceSettings::new(&store);
    let at = |hour, minute| ClockTime { hour, minute };
    // The default 22:00-08:00.
    assert_eq!(settings.day_start()?, at(8, 0));
    settings.set_quiet_hours(QuietHours {
        start: at(23, 30),
        end: at(6, 45),
    })?;
    assert_eq!(settings.day_start()?, at(6, 45));
    // Now on the wall clock, an hour ahead of UTC here, to the minute.
    let wall = |utc: DateTime<Utc>| (utc + Duration::hours(1)).naive_utc();
    assert_eq!(
        settings.sentence_now(&wall)?,
        SentenceNow {
            today: crate::schedule::Day::new(2026, 11, 3),
            time: at(14, 20),
            day_start: at(6, 45),
        }
    );
    // Empty.
    settings.set_quiet_hours(QuietHours {
        start: at(7, 0),
        end: at(7, 0),
    })?;
    assert_eq!(settings.day_start()?, DEFAULT_DAY_START);
    settings.set_quiet_hours(QuietHours {
        start: at(23, 30),
        end: at(6, 45),
    })?;
    settings.set_quiet_hours_on(false)?;
    assert_eq!(settings.day_start()?, DEFAULT_DAY_START);
    Ok(())
}

#[test]
fn settings_keep_what_they_are_set_to() -> Result<()> {
    let store = crate::store::tests::store();
    let settings = DeviceSettings::new(&store);
    settings.set_renag_interval(Duration::minutes(5))?;
    settings.set_nudge_repeat(Duration::minutes(30))?;
    let quiet = QuietHours {
        start: ClockTime {
            hour: 23,
            minute: 30,
        },
        end: ClockTime { hour: 6, minute: 0 },
    };
    settings.set_quiet_hours(quiet)?;
    settings.set_quiet_hours_on(false)?;
    let half_past_eleven = ClockTime {
        hour: 11,
        minute: 30,
    };
    settings.set_all_day_from(half_past_eleven)?;
    assert_eq!(settings.all_day_from()?, half_past_eleven);
    assert_eq!(store.local_setting(ALL_DAY_FROM)?.as_deref(), Some("11:30"));
    settings.set_alerts_on(false)?;
    settings.set_autostart_on(false)?;
    settings.set_streaks_on(false)?;
    assert_eq!(settings.renag_interval()?, Duration::minutes(5));
    assert_eq!(settings.nudge_repeat()?, Duration::minutes(30));
    assert_eq!(settings.quiet_hours()?, quiet);
    assert_eq!(
        store.local_setting(QUIET_HOURS)?.as_deref(),
        Some("23:30-06:00")
    );
    assert!(!settings.quiet_hours_on()?);
    assert!(!settings.alerts_on()?);
    assert!(!settings.autostart_on()?);
    assert!(!settings.streaks_on()?);
    assert_eq!(store.local_setting(STREAKS_OFF)?.as_deref(), Some("true"));
    settings.set_streaks_on(true)?;
    assert!(settings.streaks_on()?);
    // The Agenda strikes through what is checked until this Device
    // hides it (#217), under the Dart core's key.
    settings.set_agenda_hides_checked(true)?;
    assert!(settings.agenda_hides_checked()?);
    assert_eq!(
        store.local_setting(AGENDA_HIDE_CHECKED)?.as_deref(),
        Some("true")
    );
    settings.set_agenda_hides_checked(false)?;
    assert!(!settings.agenda_hides_checked()?);
    store.set_local_setting(AGENDA_HIDE_CHECKED, "garbled")?;
    assert!(!settings.agenda_hides_checked()?);
    // Text and icons keep their size until this Device makes them
    // bigger (#224), under the Dart core's key.
    settings.set_fat_fingers(true)?;
    assert!(settings.fat_fingers()?);
    assert_eq!(store.local_setting(FAT_FINGERS)?.as_deref(), Some("true"));
    settings.set_fat_fingers(false)?;
    assert!(!settings.fat_fingers()?);
    store.set_local_setting(FAT_FINGERS, "garbled")?;
    assert!(!settings.fat_fingers()?);
    for control in QuickNoteControl::ALL {
        settings.set_quick_note_shows(control, true)?;
        assert!(settings.quick_note_shows(control)?, "{control:?}");
        assert_eq!(store.local_setting(control.key())?.as_deref(), Some("true"));
        settings.set_quick_note_shows(control, false)?;
        assert!(!settings.quick_note_shows(control)?, "{control:?}");
    }
    assert_eq!(
        store.local_setting("quickNoteFormatBar")?.as_deref(),
        Some("false")
    );
    settings.set_quick_note_nudge_buttons_on(false)?;
    assert!(!settings.quick_note_nudge_buttons_on()?);
    assert_eq!(
        store
            .local_setting(QUICK_NOTE_NUDGE_BUTTONS_OFF)?
            .as_deref(),
        Some("true")
    );
    settings.set_quick_note_nudge_buttons_on(true)?;
    assert!(settings.quick_note_nudge_buttons_on()?);
    // The Shopping List a capture went into last (#276).
    assert_eq!(settings.last_shopping_list()?, None);
    settings.set_last_shopping_list(Some("note:weekly"))?;
    assert_eq!(
        settings.last_shopping_list()?.as_deref(),
        Some("note:weekly")
    );
    assert_eq!(
        store.local_setting("lastShoppingList")?.as_deref(),
        Some("note:weekly")
    );
    settings.set_last_shopping_list(None)?;
    assert_eq!(settings.last_shopping_list()?, None);
    store.set_local_setting("lastShoppingList", "")?;
    assert_eq!(settings.last_shopping_list()?, None);
    settings.set_capture_hotkey("Super+Shift+K")?;
    assert_eq!(settings.capture_hotkey()?, "Super+Shift+K");
    let now = store.now();
    settings.set_missed_seen(Some(now))?;
    assert_eq!(settings.missed_seen()?, Some(now));
    settings.set_missed_seen(None)?;
    assert_eq!(settings.missed_seen()?, None);
    for choice in ThemeChoice::ALL {
        settings.set_theme(choice)?;
        assert_eq!(settings.theme()?, choice);
    }
    settings.set_theme(ThemeChoice::Light)?;
    assert_eq!(store.local_setting(THEME)?.as_deref(), Some("light"));
    Ok(())
}

#[test]
fn occurrences_ring_with_the_own_sound_until_one_is_chosen() -> Result<()> {
    let store = crate::store::tests::store();
    let settings = DeviceSettings::new(&store);
    assert_eq!(settings.alarm_sound()?, None);
    let chosen = "/home/someone/Music/bell.ogg";
    settings.set_alarm_sound(Some(chosen))?;
    assert_eq!(settings.alarm_sound()?.as_deref(), Some(chosen));
    assert_eq!(store.local_setting(ALARM_SOUND)?.as_deref(), Some(chosen));
    settings.set_alarm_sound(Some("  "))?;
    assert_eq!(settings.alarm_sound()?, None);
    assert_eq!(store.local_setting(ALARM_SOUND)?, None);
    settings.set_alarm_sound(Some(chosen))?;
    settings.set_alarm_sound(None)?;
    assert_eq!(settings.alarm_sound()?, None);
    store.set_local_setting(ALARM_SOUND, "")?;
    assert_eq!(settings.alarm_sound()?, None);
    Ok(())
}

#[test]
fn the_agenda_showed_a_row_per_shopping_list_with_lines_back_unless_another_way_was_chosen()
-> Result<()> {
    let store = crate::store::tests::store();
    let settings = DeviceSettings::new(&store);
    assert_eq!(settings.shopping_agenda()?, ShoppingAgenda::Back);
    // As a version before #186 wrote it.
    for way in ShoppingAgenda::ALL {
        store.set_local_setting(SHOPPING_AGENDA, way.code())?;
        assert_eq!(settings.shopping_agenda()?, way);
    }
    store.set_local_setting(SHOPPING_AGENDA, "sideways")?;
    assert_eq!(settings.shopping_agenda()?, ShoppingAgenda::Back);
    Ok(())
}

#[test]
fn recordings_play_at_normal_speed_until_this_device_chooses_one() -> Result<()> {
    let store = crate::store::tests::store();
    let settings = DeviceSettings::new(&store);
    assert_eq!(settings.playback_speed()?, 1.0);
    assert_eq!(settings.chosen_playback_speed()?, None);
    settings.set_playback_speed(0.75)?;
    assert_eq!(settings.playback_speed()?, 0.75);
    assert_eq!(
        store.local_setting(PLAYBACK_SPEED)?.as_deref(),
        Some("0.75")
    );
    settings.set_playback_speed(2.0)?;
    assert_eq!(store.local_setting(PLAYBACK_SPEED)?.as_deref(), Some("2.0"));
    assert!(settings.set_playback_speed(1.25).is_err());
    assert_eq!(settings.playback_speed()?, 2.0);
    Ok(())
}

#[test]
fn the_playback_speed_kept_before_carries_over_as_it_is() -> Result<()> {
    let store = crate::store::tests::store();
    let settings = DeviceSettings::new(&store);
    // As the desktop and the phone wrote it in 0.1.1.
    store.set_local_setting(PLAYBACK_SPEED, "1.5")?;
    assert_eq!(settings.playback_speed()?, 1.5);
    for kept in ["7", "fast", "1.25", ""] {
        store.set_local_setting(PLAYBACK_SPEED, kept)?;
        assert_eq!(settings.playback_speed()?, 1.0, "{kept}");
        assert_eq!(settings.chosen_playback_speed()?, None, "{kept}");
    }
    Ok(())
}

#[test]
fn file_into_keeps_the_last_three_places_filed_into_latest_first() -> Result<()> {
    let store = crate::store::tests::store();
    let settings = DeviceSettings::new(&store);
    assert!(settings.recently_filed_into()?.is_empty());
    for id in ["a", "b", "c"] {
        settings.filed_into(id)?;
    }
    assert_eq!(settings.recently_filed_into()?, ["c", "b", "a"]);
    settings.filed_into("a")?;
    assert_eq!(
        settings.recently_filed_into()?,
        ["a", "c", "b"],
        "once, moved up"
    );
    settings.filed_into("d")?;
    assert_eq!(
        settings.recently_filed_into()?,
        ["d", "a", "c"],
        "b forgotten"
    );
    assert_eq!(
        store.local_setting(RECENTLY_FILED_INTO)?.as_deref(),
        Some(r#"["d","a","c"]"#)
    );
    store.set_local_setting(RECENTLY_FILED_INTO, r#"["x",7,"x","y","z","w"]"#)?;
    assert_eq!(settings.recently_filed_into()?, ["x", "y", "z"]);
    store.set_local_setting(RECENTLY_FILED_INTO, "not json")?;
    assert!(settings.recently_filed_into()?.is_empty());
    settings.filed_into("e")?;
    assert_eq!(settings.recently_filed_into()?, ["e"]);
    assert!(store.all_entries()?.is_empty(), "settings are never logged");
    Ok(())
}
