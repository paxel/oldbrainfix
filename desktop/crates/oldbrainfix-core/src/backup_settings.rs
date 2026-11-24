//! The Device settings a Backup carries (ADR 0016, obf-1098), as the
//! Dart core's `backup_settings.dart`: one record after the Profile,
//! `{"settings":{…}}`, keys sorted, each value as the setting keeps it.
//! Only the settings of [`CARRIED`] go into it and come out of it, each
//! only with a value the Device offers, so a crafted file cannot plant a
//! signing seed, a PIN, the backup key or a folder.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::Result;
use crate::settings::{SPEECH_MODEL, SPEECH_MODEL_OFFER};
use crate::store::Store;

/// The app a Device runs: a Backup's settings are taken only by the app
/// that has them, so a phone's Backup restored on the desktop brings
/// what both have.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SettingsApp {
    Phone,
    Desktop,
}

/// One setting a Backup carries: its key, the values a Device offers
/// for it, and which apps have it.
struct Carried {
    key: &'static str,
    offered: fn(&str) -> bool,
    phone: bool,
    desktop: bool,
}

fn switch(value: &str) -> bool {
    matches!(value, "true" | "false")
}

fn digits(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit())
}

/// A whole number of minutes from 1 to a day, as the apps write it: an
/// interval set elsewhere stays offered beside the choices.
fn minutes(value: &str) -> bool {
    value.len() <= 4
        && digits(value)
        && !value.starts_with('0')
        && value.parse::<u32>().is_ok_and(|m| m <= 1440)
}

/// A time of day as the cores write it: `08:00`.
fn clock(value: &str) -> bool {
    let Some((hour, minute)) = value.split_once(':') else {
        return false;
    };
    hour.len() == 2
        && minute.len() == 2
        && digits(hour)
        && digits(minute)
        && hour.parse::<u32>().is_ok_and(|h| h < 24)
        && minute.parse::<u32>().is_ok_and(|m| m < 60)
}

/// Quiet Hours as the cores write them: `22:00-08:00`.
fn quiet_hours(value: &str) -> bool {
    value.len() == 11
        && value
            .split_once('-')
            .is_some_and(|(from, until)| clock(from) && clock(until))
}

fn recording_pause(value: &str) -> bool {
    matches!(value, "2" | "3" | "5" | "10")
}

fn shopping_agenda(value: &str) -> bool {
    matches!(value, "lines" | "back" | "open" | "off")
}

/// A speech model and how it came: `base:download`, `small:file`.
fn speech_model(value: &str) -> bool {
    matches!(
        value,
        "base:download" | "base:file" | "small:download" | "small:file"
    )
}

fn text_size(value: &str) -> bool {
    matches!(value, "normal" | "larger" | "largest")
}

fn theme(value: &str) -> bool {
    matches!(value, "system" | "light" | "dark")
}

fn unit_system(value: &str) -> bool {
    matches!(value, "metric" | "imperial")
}

const fn both(key: &'static str, offered: fn(&str) -> bool) -> Carried {
    Carried {
        key,
        offered,
        phone: true,
        desktop: true,
    }
}

/// Every setting a Backup carries, by key, as the Dart core's list.
/// Left behind, and asked for again: the App Lock with its PIN,
/// biometrics and relock time, the Area Locks, the paired Devices, the
/// shared folder and the Backup folder (with Google's copy), the alarm
/// sound and the calendars picked (named as only that Device names
/// them), the switches that need a permission granted there, the AI
/// with its provider, keys and the questions allowed, the autostart and
/// the capture key chord, the window and the folds, and what the Device
/// only notes for itself.
const CARRIED: [Carried; 18] = [
    both("agendaHideChecked", switch),
    Carried {
        key: "alertsOff",
        offered: switch,
        phone: false,
        desktop: true,
    },
    both("allDayFrom", clock),
    both("fatFingers", switch),
    both("nudgeRepeatMinutes", minutes),
    both("quickNoteNudgeButtonsOff", switch),
    both("quietHours", quiet_hours),
    both("quietHoursOff", switch),
    both("recordingPauseSeconds", recording_pause),
    both("renagMinutes", minutes),
    both("shoppingAgenda", shopping_agenda),
    both(SPEECH_MODEL, speech_model),
    both("streaksOff", switch),
    both("textSize", text_size),
    both("theme", theme),
    both("tipsOff", switch),
    Carried {
        key: "twoRowEditorOff",
        offered: switch,
        phone: true,
        desktop: false,
    },
    both("unitSystem", unit_system),
];

/// The settings record of a Backup of `store`, keys sorted: each setting
/// of [`CARRIED`] it holds with a value offered, for either app; the
/// speech model in use, else the one still offered after a restore, so
/// an offer not yet answered is not lost. Empty when it holds none.
pub fn carried_settings(store: &Store) -> Result<BTreeMap<String, String>> {
    let mut carried = BTreeMap::new();
    for setting in &CARRIED {
        let keys: &[&str] = if setting.key == SPEECH_MODEL {
            &[SPEECH_MODEL, SPEECH_MODEL_OFFER]
        } else {
            &[setting.key]
        };
        for key in keys {
            if let Some(value) = store.local_setting(key)?
                && (setting.offered)(&value)
            {
                carried.insert(setting.key.to_owned(), value);
                break;
            }
        }
    }
    Ok(carried)
}

/// What `app` takes of a Backup's settings `record`, as the local
/// settings to keep, sorted by key: each setting of [`CARRIED`] that
/// `app` has, written as text with a value offered. Anything else is
/// passed over: a key not carried, a value not offered or not text, and
/// a setting the other app has alone. The speech model comes as the
/// offer of it ([`crate::DeviceSettings::speech_model_offer`]).
pub fn taken_settings(record: &Value, app: SettingsApp) -> Vec<(String, String)> {
    let Some(record) = record.as_object() else {
        return Vec::new();
    };
    let mut taken: Vec<(String, String)> = CARRIED
        .iter()
        .filter(|setting| match app {
            SettingsApp::Phone => setting.phone,
            SettingsApp::Desktop => setting.desktop,
        })
        .filter_map(|setting| {
            let value = record.get(setting.key)?.as_str()?;
            if !(setting.offered)(value) {
                return None;
            }
            let key = if setting.key == SPEECH_MODEL {
                SPEECH_MODEL_OFFER
            } else {
                setting.key
            };
            Some((key.to_owned(), value.to_owned()))
        })
        .collect();
    taken.sort();
    taken
}

#[cfg(test)]
mod tests;
