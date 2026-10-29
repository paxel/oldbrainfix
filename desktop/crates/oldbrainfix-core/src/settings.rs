//! Settings of this Device, kept as local settings and never synced (ADR
//! 0008), under the keys and in the forms the Dart core's
//! `device_settings.dart` uses, plus the few only the desktop has.

use chrono::{DateTime, Duration, Utc};

use crate::entry::{dart_iso, parse_time};
use crate::nudges::QuietHours;
use crate::schedule::ClockTime;
use crate::store::Store;
use crate::tally_units::{self, UnitSystem};
use crate::{Error, Result};

const RENAG_MINUTES: &str = "renagMinutes";
const MISSED_SEEN: &str = "missedSeen";
const QUIET_HOURS: &str = "quietHours";
const QUIET_HOURS_OFF: &str = "quietHoursOff";
const NUDGE_REPEAT_MINUTES: &str = "nudgeRepeatMinutes";
const ALERTS_OFF: &str = "alertsOff";
const AUTOSTART_OFF: &str = "autostartOff";
const LAST_FIRED: &str = "lastFired";
const CAPTURE_HOTKEY: &str = "captureHotkey";
const PLAYBACK_SPEED: &str = "playbackSpeed";
const ALARM_SOUND: &str = "alarmSound";
const UNIT_SYSTEM: &str = "unitSystem";

/// The playback speeds of voice recordings offered, 1 being normal speed.
pub const PLAYBACK_SPEEDS: [f64; 5] = [0.5, 0.75, 1.0, 1.5, 2.0];

/// `speed` as the Dart core writes it: "1.0", "1.5", "0.75".
pub(crate) fn speed_text(speed: f64) -> String {
    format!("{speed:?}")
}

/// The playback speed in `text`, if it is one offered.
pub(crate) fn offered_speed(text: &str) -> Option<f64> {
    text.parse::<f64>()
        .ok()
        .filter(|s| PLAYBACK_SPEEDS.contains(s))
}

/// The key chord that opens the desktop's capture popup when nothing
/// else is set.
pub const DEFAULT_CAPTURE_HOTKEY: &str = "Ctrl+Alt+N";

/// The Re-nag interval when nothing else is set.
pub fn default_renag() -> Duration {
    Duration::minutes(10)
}

/// How long an ignored Nudge waits before it comes again, when nothing
/// else is set.
pub fn default_nudge_repeat() -> Duration {
    Duration::hours(1)
}

/// The Quiet Hours when nothing else is set: 22:00 to 08:00.
pub const DEFAULT_QUIET_HOURS: QuietHours = QuietHours {
    start: ClockTime {
        hour: 22,
        minute: 0,
    },
    end: ClockTime { hour: 8, minute: 0 },
};

/// This Device's settings in the store's local settings.
pub struct DeviceSettings<'a> {
    store: &'a Store,
}

impl<'a> DeviceSettings<'a> {
    pub fn new(store: &'a Store) -> DeviceSettings<'a> {
        DeviceSettings { store }
    }

    fn minutes(&self, key: &str, default: Duration) -> Result<Duration> {
        let minutes = self
            .store
            .local_setting(key)?
            .and_then(|m| m.parse::<i64>().ok())
            .filter(|m| *m >= 1);
        Ok(minutes.map_or(default, Duration::minutes))
    }

    fn set_minutes(&self, key: &str, interval: Duration) -> Result<()> {
        let minutes = interval.num_minutes().max(1);
        self.store.set_local_setting(key, &minutes.to_string())
    }

    /// How often an ignored Occurrence fires again, unless its Routine has
    /// its own interval.
    pub fn renag_interval(&self) -> Result<Duration> {
        self.minutes(RENAG_MINUTES, default_renag())
    }

    /// Sets the Re-nag interval; under a minute counts as one.
    pub fn set_renag_interval(&self, interval: Duration) -> Result<()> {
        self.set_minutes(RENAG_MINUTES, interval)
    }

    /// When the user last tapped "seen" on the Missed Summary here.
    pub fn missed_seen(&self) -> Result<Option<DateTime<Utc>>> {
        Ok(self
            .store
            .local_setting(MISSED_SEEN)?
            .and_then(|s| parse_time(&s).ok()))
    }

    pub fn set_missed_seen(&self, seen: Option<DateTime<Utc>>) -> Result<()> {
        let value = seen.map(dart_iso).unwrap_or_default();
        self.store.set_local_setting(MISSED_SEEN, &value)
    }

    /// This Device's Quiet Hours, kept while they are switched off.
    pub fn quiet_hours(&self) -> Result<QuietHours> {
        let text = self.store.local_setting(QUIET_HOURS)?.unwrap_or_default();
        let parts: Vec<&str> = text.split('-').collect();
        let quiet = match parts[..] {
            [from, until] => ClockTime::parse(from)
                .zip(ClockTime::parse(until))
                .map(|(start, end)| QuietHours { start, end }),
            _ => None,
        };
        Ok(quiet.unwrap_or(DEFAULT_QUIET_HOURS))
    }

    pub fn set_quiet_hours(&self, quiet: QuietHours) -> Result<()> {
        self.store
            .set_local_setting(QUIET_HOURS, &quiet.to_string())
    }

    /// Whether Nudges wait out the Quiet Hours; on unless switched off.
    pub fn quiet_hours_on(&self) -> Result<bool> {
        Ok(self.store.local_setting(QUIET_HOURS_OFF)?.as_deref() != Some("true"))
    }

    pub fn set_quiet_hours_on(&self, on: bool) -> Result<()> {
        self.store
            .set_local_setting(QUIET_HOURS_OFF, &(!on).to_string())
    }

    /// How long an ignored Nudge waits before it comes again.
    pub fn nudge_repeat(&self) -> Result<Duration> {
        self.minutes(NUDGE_REPEAT_MINUTES, default_nudge_repeat())
    }

    /// Sets the Nudge's repeat; under a minute counts as one.
    pub fn set_nudge_repeat(&self, interval: Duration) -> Result<()> {
        self.set_minutes(NUDGE_REPEAT_MINUTES, interval)
    }

    /// Whether this Device shows notifications for Occurrences and
    /// Nudges; on unless switched off.
    pub fn alerts_on(&self) -> Result<bool> {
        Ok(self.store.local_setting(ALERTS_OFF)?.as_deref() != Some("true"))
    }

    pub fn set_alerts_on(&self, on: bool) -> Result<()> {
        self.store.set_local_setting(ALERTS_OFF, &(!on).to_string())
    }

    /// Whether the desktop starts at login, into the tray; on unless
    /// switched off.
    pub fn autostart_on(&self) -> Result<bool> {
        Ok(self.store.local_setting(AUTOSTART_OFF)?.as_deref() != Some("true"))
    }

    pub fn set_autostart_on(&self, on: bool) -> Result<()> {
        self.store
            .set_local_setting(AUTOSTART_OFF, &(!on).to_string())
    }

    /// When the desktop last showed what was due; `None` before the first
    /// time.
    pub fn last_fired(&self) -> Result<Option<DateTime<Utc>>> {
        Ok(self
            .store
            .local_setting(LAST_FIRED)?
            .and_then(|s| parse_time(&s).ok()))
    }

    pub fn set_last_fired(&self, at: DateTime<Utc>) -> Result<()> {
        self.store.set_local_setting(LAST_FIRED, &dart_iso(at))
    }

    /// The key chord that opens the desktop's capture popup, as typed,
    /// such as "Ctrl+Alt+N".
    pub fn capture_hotkey(&self) -> Result<String> {
        Ok(self
            .store
            .local_setting(CAPTURE_HOTKEY)?
            .filter(|chord| !chord.trim().is_empty())
            .unwrap_or_else(|| DEFAULT_CAPTURE_HOTKEY.to_string()))
    }

    pub fn set_capture_hotkey(&self, chord: &str) -> Result<()> {
        self.store.set_local_setting(CAPTURE_HOTKEY, chord)
    }

    /// The sound file this Device rings every Occurrence with, as chosen;
    /// `None` for the app's own sound. Escalation keeps the system alarm
    /// sound either way.
    pub fn alarm_sound(&self) -> Result<Option<String>> {
        Ok(self
            .store
            .local_setting(ALARM_SOUND)?
            .filter(|sound| !sound.trim().is_empty()))
    }

    /// `None` or blank goes back to the app's own sound.
    pub fn set_alarm_sound(&self, sound: Option<&str>) -> Result<()> {
        match sound.filter(|sound| !sound.trim().is_empty()) {
            Some(sound) => self.store.set_local_setting(ALARM_SOUND, sound),
            None => self.store.remove_local_setting(ALARM_SOUND),
        }
    }

    /// The system this Device shows the known units of Tallies in, when
    /// one was chosen; `None` follows the region.
    pub fn chosen_unit_system(&self) -> Result<Option<UnitSystem>> {
        Ok(self
            .store
            .local_setting(UNIT_SYSTEM)?
            .and_then(|name| UnitSystem::parse(&name)))
    }

    /// `None` follows the region again.
    pub fn set_chosen_unit_system(&self, system: Option<UnitSystem>) -> Result<()> {
        match system {
            Some(system) => self.store.set_local_setting(UNIT_SYSTEM, system.name()),
            None => self.store.remove_local_setting(UNIT_SYSTEM),
        }
    }

    /// The system this Device shows the known units of Tallies in: the
    /// one chosen, else the one of `region`, the ISO country code the
    /// system reports.
    pub fn unit_system(&self, region: Option<&str>) -> Result<UnitSystem> {
        Ok(self
            .chosen_unit_system()?
            .unwrap_or_else(|| tally_units::system_for_region(region)))
    }

    /// How fast this Device plays voice recordings: one of
    /// [`PLAYBACK_SPEEDS`], normal speed when none was chosen. The one
    /// Device setting a Backup carries (ADR 0016).
    pub fn playback_speed(&self) -> Result<f64> {
        Ok(self.chosen_playback_speed()?.unwrap_or(1.0))
    }

    /// The playback speed chosen on this Device; `None` when none of
    /// [`PLAYBACK_SPEEDS`] was.
    pub fn chosen_playback_speed(&self) -> Result<Option<f64>> {
        Ok(self
            .store
            .local_setting(PLAYBACK_SPEED)?
            .and_then(|s| offered_speed(&s)))
    }

    /// Keeps `speed` as written since 0.1.1: "1.0", "1.5"; one not
    /// offered is refused.
    pub fn set_playback_speed(&self, speed: f64) -> Result<()> {
        if !PLAYBACK_SPEEDS.contains(&speed) {
            return Err(Error::Invalid(format!("{speed} is no speed offered")));
        }
        self.store
            .set_local_setting(PLAYBACK_SPEED, &speed_text(speed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_fall_back_to_their_defaults() -> Result<()> {
        let store = crate::store::tests::store();
        let settings = DeviceSettings::new(&store);
        assert_eq!(settings.renag_interval()?, Duration::minutes(10));
        assert_eq!(settings.nudge_repeat()?, Duration::hours(1));
        assert_eq!(settings.quiet_hours()?, DEFAULT_QUIET_HOURS);
        assert!(settings.quiet_hours_on()?);
        assert!(settings.alerts_on()?);
        assert!(settings.autostart_on()?);
        assert_eq!(settings.capture_hotkey()?, DEFAULT_CAPTURE_HOTKEY);
        assert_eq!(settings.missed_seen()?, None);
        store.set_local_setting(RENAG_MINUTES, "0")?;
        assert_eq!(settings.renag_interval()?, Duration::minutes(10));
        store.set_local_setting(QUIET_HOURS, "23:00")?;
        assert_eq!(settings.quiet_hours()?, DEFAULT_QUIET_HOURS);
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
        settings.set_alerts_on(false)?;
        settings.set_autostart_on(false)?;
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
        settings.set_capture_hotkey("Super+Shift+K")?;
        assert_eq!(settings.capture_hotkey()?, "Super+Shift+K");
        let now = store.now();
        settings.set_missed_seen(Some(now))?;
        assert_eq!(settings.missed_seen()?, Some(now));
        settings.set_missed_seen(None)?;
        assert_eq!(settings.missed_seen()?, None);
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
}
