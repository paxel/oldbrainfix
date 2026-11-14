//! Settings of this Device, kept as local settings and never synced (ADR
//! 0008), under the keys and in the forms the Dart core's
//! `device_settings.dart` uses, plus the few only the desktop has.

use chrono::{DateTime, Duration, Utc};

use crate::entry::{dart_iso, parse_time};
use crate::nudges::QuietHours;
use crate::schedule::{ClockTime, WallClock};
use crate::schedule_sentence::SentenceNow;
use crate::shopping_lists::ShoppingAgenda;
use crate::store::Store;
use crate::tally_units::{self, UnitSystem};
use crate::{Error, Result};

const RENAG_MINUTES: &str = "renagMinutes";
const MISSED_SEEN: &str = "missedSeen";
const QUIET_HOURS: &str = "quietHours";
const QUIET_HOURS_OFF: &str = "quietHoursOff";
const ALL_DAY_FROM: &str = "allDayFrom";
const NUDGE_REPEAT_MINUTES: &str = "nudgeRepeatMinutes";
const ALERTS_OFF: &str = "alertsOff";
const AUTOSTART_OFF: &str = "autostartOff";
const LAST_FIRED: &str = "lastFired";
const CAPTURE_HOTKEY: &str = "captureHotkey";
const PLAYBACK_SPEED: &str = "playbackSpeed";
const ALARM_SOUND: &str = "alarmSound";
const UNIT_SYSTEM: &str = "unitSystem";
const SHOPPING_AGENDA: &str = "shoppingAgenda";
const STREAKS_OFF: &str = "streaksOff";
const AGENDA_HIDE_CHECKED: &str = "agendaHideChecked";
const FAT_FINGERS: &str = "fatFingers";
const QUICK_NOTE_NUDGE_BUTTONS_OFF: &str = "quickNoteNudgeButtonsOff";
const LAST_SHOPPING_LIST: &str = "lastShoppingList";
const THEME: &str = "theme";
const RECENTLY_FILED_INTO: &str = "recentlyFiledInto";

/// How many places filed into File into… shows at its top (#272).
pub const RECENT_PLACES_KEPT: usize = 3;

/// Which theme this Device shows: the system's, or a fixed one, as the
/// Dart core's `ThemeChoice`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum ThemeChoice {
    #[default]
    System,
    Light,
    Dark,
}

impl ThemeChoice {
    /// All of them, in the menu's order.
    pub const ALL: [ThemeChoice; 3] = [ThemeChoice::System, ThemeChoice::Light, ThemeChoice::Dark];

    /// How it is stored, as the Dart core's enum names it.
    pub fn name(self) -> &'static str {
        match self {
            ThemeChoice::System => "system",
            ThemeChoice::Light => "light",
            ThemeChoice::Dark => "dark",
        }
    }

    /// The choice `name` names; anything unknown follows the system.
    pub fn read(name: Option<&str>) -> ThemeChoice {
        ThemeChoice::ALL
            .into_iter()
            .find(|c| Some(c.name()) == name)
            .unwrap_or_default()
    }
}

/// What the quick note (the capture field and popup on the desktop)
/// shows beyond its text, its buttons that add a recording, a picture or
/// a file, its Nudge buttons, "To full editor" and the alarm clock for a
/// Routine or Reminder (#223), each once this Device switched it on
/// (#142), as the Dart core's `QuickNoteControl`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum QuickNoteControl {
    /// "File into…".
    FileInto,
    /// Frog and Session in the alarm clock's menu, and "Make it a Tally"
    /// (Q481); the clock offers a Routine or Reminder without it (#223).
    Schedule,
    /// "Add to a Shopping List".
    Shopping,
    /// The Importance.
    Importance,
    /// The format bar (ADR 0033).
    FormatBar,
}

impl QuickNoteControl {
    /// All of them, in the Settings' order.
    pub const ALL: [QuickNoteControl; 5] = [
        QuickNoteControl::FileInto,
        QuickNoteControl::Schedule,
        QuickNoteControl::Shopping,
        QuickNoteControl::Importance,
        QuickNoteControl::FormatBar,
    ];

    /// The local setting that holds "true" while it is switched on.
    pub fn key(self) -> &'static str {
        match self {
            QuickNoteControl::FileInto => "quickNoteFileInto",
            QuickNoteControl::Schedule => "quickNoteSchedule",
            QuickNoteControl::Shopping => "quickNoteShopping",
            QuickNoteControl::Importance => "quickNoteImportance",
            QuickNoteControl::FormatBar => "quickNoteFormatBar",
        }
    }
}

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

/// When all-day Occurrences start to ring when nothing else is set
/// (#201).
pub const DEFAULT_ALL_DAY_FROM: ClockTime = ClockTime {
    hour: 15,
    minute: 0,
};

/// When the day starts without Quiet Hours (#271).
pub const DEFAULT_DAY_START: ClockTime = ClockTime { hour: 9, minute: 0 };

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

    /// When this Device's day starts, at which a Reminder for "tomorrow"
    /// rings (#271): when its Quiet Hours end, or 09:00 while they are
    /// switched off or empty.
    pub fn day_start(&self) -> Result<ClockTime> {
        let quiet = self.quiet_hours()?;
        Ok(if self.quiet_hours_on()? && quiet.start != quiet.end {
            quiet.end
        } else {
            DEFAULT_DAY_START
        })
    }

    /// Now by the store's clock on `wall`, with this Device's
    /// [`day_start`](Self::day_start): what the schedule sentence's
    /// relative words and the quick Reminder chips are read at (#271).
    pub fn sentence_now(&self, wall: &WallClock) -> Result<SentenceNow> {
        Ok(SentenceNow::at(wall(self.store.now()), self.day_start()?))
    }

    /// From when on this Device an all-day Occurrence rings, on the
    /// wall clock; before that it only shows on the Agenda (#201).
    pub fn all_day_from(&self) -> Result<ClockTime> {
        let text = self.store.local_setting(ALL_DAY_FROM)?.unwrap_or_default();
        Ok(ClockTime::parse(&text).unwrap_or(DEFAULT_ALL_DAY_FROM))
    }

    pub fn set_all_day_from(&self, time: ClockTime) -> Result<()> {
        self.store
            .set_local_setting(ALL_DAY_FROM, &time.to_string())
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

    /// Whether this Device shows Streaks; on unless switched off. Adherence
    /// shows either way.
    pub fn streaks_on(&self) -> Result<bool> {
        Ok(self.store.local_setting(STREAKS_OFF)?.as_deref() != Some("true"))
    }

    pub fn set_streaks_on(&self, on: bool) -> Result<()> {
        self.store
            .set_local_setting(STREAKS_OFF, &(!on).to_string())
    }

    /// Whether this Device's Agenda hides what is checked instead of
    /// striking it through (#217): a Done Occurrence, a checked Line, a
    /// pick with its entry, and a group with nothing left; off until
    /// switched on.
    pub fn agenda_hides_checked(&self) -> Result<bool> {
        Ok(self.store.local_setting(AGENDA_HIDE_CHECKED)?.as_deref() == Some("true"))
    }

    pub fn set_agenda_hides_checked(&self, on: bool) -> Result<()> {
        self.store
            .set_local_setting(AGENDA_HIDE_CHECKED, &on.to_string())
    }

    /// Whether this Device shows text, icons and tap targets 1.3 times
    /// their size (#224): "Bigger text and icons" on the desktop, egui's
    /// zoom, as "Fat fingers" on the phone; off until switched on.
    pub fn fat_fingers(&self) -> Result<bool> {
        Ok(self.store.local_setting(FAT_FINGERS)?.as_deref() == Some("true"))
    }

    pub fn set_fat_fingers(&self, on: bool) -> Result<()> {
        self.store.set_local_setting(FAT_FINGERS, &on.to_string())
    }

    /// Whether the quick note on this Device shows `control`; off until
    /// switched on.
    pub fn quick_note_shows(&self, control: QuickNoteControl) -> Result<bool> {
        Ok(self.store.local_setting(control.key())?.as_deref() == Some("true"))
    }

    pub fn set_quick_note_shows(&self, control: QuickNoteControl, on: bool) -> Result<()> {
        self.store.set_local_setting(control.key(), &on.to_string())
    }

    /// Whether the quick note on this Device shows the three Nudge
    /// buttons; on unless switched off, which gives a single Save in
    /// their place.
    pub fn quick_note_nudge_buttons_on(&self) -> Result<bool> {
        Ok(self
            .store
            .local_setting(QUICK_NOTE_NUDGE_BUTTONS_OFF)?
            .as_deref()
            != Some("true"))
    }

    pub fn set_quick_note_nudge_buttons_on(&self, on: bool) -> Result<()> {
        self.store
            .set_local_setting(QUICK_NOTE_NUDGE_BUTTONS_OFF, &(!on).to_string())
    }

    /// The Note id of the Shopping List a capture on this Device went into
    /// last, at the top of the lists to pick from (#276); `None` for none.
    /// It may name a list gone since.
    pub fn last_shopping_list(&self) -> Result<Option<String>> {
        Ok(self
            .store
            .local_setting(LAST_SHOPPING_LIST)?
            .filter(|id| !id.is_empty()))
    }

    pub fn set_last_shopping_list(&self, note_id: Option<&str>) -> Result<()> {
        match note_id {
            Some(id) => self.store.set_local_setting(LAST_SHOPPING_LIST, id),
            None => self.store.remove_local_setting(LAST_SHOPPING_LIST),
        }
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

    /// The theme this Device shows; the system's unless one was chosen.
    pub fn theme(&self) -> Result<ThemeChoice> {
        Ok(ThemeChoice::read(
            self.store.local_setting(THEME)?.as_deref(),
        ))
    }

    pub fn set_theme(&self, choice: ThemeChoice) -> Result<()> {
        self.store.set_local_setting(THEME, choice.name())
    }

    /// How this Device's Agenda showed Shopping Lists before each list had
    /// its own choice (#186); one row per list on a day when Lines came
    /// back unless another way was chosen. Read only for the default of a
    /// list nobody chose for ([`crate::ShoppingOnAgenda::from_device`]); no
    /// app offers it any more.
    pub fn shopping_agenda(&self) -> Result<ShoppingAgenda> {
        Ok(ShoppingAgenda::read(
            self.store.local_setting(SHOPPING_AGENDA)?.as_deref(),
        ))
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

    /// The ids of the Areas and Topics this Device last filed into, the
    /// latest first, at most [`RECENT_PLACES_KEPT`] (#272). They may name
    /// a place deleted since; File into… shows only those it lists.
    pub fn recently_filed_into(&self) -> Result<Vec<String>> {
        let kept = self.store.local_setting(RECENTLY_FILED_INTO)?;
        let ids = match kept.as_deref().map(serde_json::from_str) {
            Some(Ok(serde_json::Value::Array(ids))) => ids,
            _ => return Ok(Vec::new()),
        };
        let mut recent: Vec<String> = Vec::new();
        for id in ids.iter().filter_map(serde_json::Value::as_str) {
            if recent.len() < RECENT_PLACES_KEPT && !recent.iter().any(|r| r == id) {
                recent.push(id.to_owned());
            }
        }
        Ok(recent)
    }

    /// Puts `place_id` first among [`Self::recently_filed_into`], once,
    /// and forgets the oldest beyond [`RECENT_PLACES_KEPT`].
    pub fn filed_into(&self, place_id: &str) -> Result<()> {
        let mut recent = vec![place_id.to_owned()];
        recent.extend(
            self.recently_filed_into()?
                .into_iter()
                .filter(|id| id != place_id),
        );
        recent.truncate(RECENT_PLACES_KEPT);
        let json = serde_json::to_string(&recent).map_err(|e| Error::Invalid(e.to_string()))?;
        self.store.set_local_setting(RECENTLY_FILED_INTO, &json)
    }
}

#[cfg(test)]
mod tests;
