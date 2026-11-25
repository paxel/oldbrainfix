use super::*;
use crate::settings::{DeviceSettings, SpeechModelChoice, SpeechModelSource};
use serde_json::json;

fn pairs(list: &[(&str, &str)]) -> Vec<(String, String)> {
    list.iter()
        .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
        .collect()
}

#[test]
fn the_record_holds_the_settings_carried_sorted_for_either_app() -> Result<()> {
    let store = crate::store::tests::store();
    for (key, value) in [
        ("unitSystem", "imperial"),
        ("theme", "dark"),
        ("alertsOff", "true"),
        ("twoRowEditorOff", "true"),
        ("quietHours", "21:30-07:00"),
        ("allDayFrom", "09:05"),
        ("renagMinutes", "15"),
        ("recordingPauseSeconds", "5"),
        // Bound to this Device, never carried.
        ("captureHotkey", "Ctrl+Alt+M"),
        ("autostartOff", "true"),
        ("alarmSound", "/home/ada/bell.ogg"),
        ("ask:on", "true"),
        ("appLock", "true"),
        ("backup:folder", "/somewhere"),
    ] {
        store.set_local_setting(key, value)?;
    }
    DeviceSettings::new(&store).set_speech_model(Some(&SpeechModelChoice {
        name: "small".into(),
        source: SpeechModelSource::File,
    }))?;
    let carried = carried_settings(&store)?;
    let line = json!({ "settings": carried }).to_string();
    assert_eq!(
        line,
        r#"{"settings":{"alertsOff":"true","allDayFrom":"09:05","quietHours":"21:30-07:00","recordingPauseSeconds":"5","renagMinutes":"15","speechModel":"small:file","theme":"dark","twoRowEditorOff":"true","unitSystem":"imperial"}}"#
    );
    Ok(())
}

#[test]
fn each_app_takes_only_those_it_has_and_the_speech_model_as_an_offer() {
    let record = json!({
        "alertsOff": "true",
        "twoRowEditorOff": "true",
        "speechModel": "base:download",
    });
    assert_eq!(
        taken_settings(&record, SettingsApp::Desktop),
        pairs(&[("alertsOff", "true"), ("speechModelOffer", "base:download")])
    );
    assert_eq!(
        taken_settings(&record, SettingsApp::Phone),
        pairs(&[
            ("speechModelOffer", "base:download"),
            ("twoRowEditorOff", "true")
        ])
    );
}

#[test]
fn a_value_not_offered_written_otherwise_or_not_as_text_is_passed_over() -> Result<()> {
    let store = crate::store::tests::store();
    for (key, value) in [
        ("theme", "Dark"),
        ("textSize", "huge"),
        ("renagMinutes", "0"),
        ("nudgeRepeatMinutes", "010"),
        ("quietHours", "22:00-8:00"),
        ("allDayFrom", "24:00"),
        ("recordingPauseSeconds", "4"),
        ("fatFingers", "yes"),
        ("speechModel", "large:download"),
        ("shoppingAgenda", "all"),
        ("unitSystem", "nautical"),
    ] {
        store.set_local_setting(key, value)?;
    }
    assert!(carried_settings(&store)?.is_empty());
    let record = json!({
        "theme": "Dark",
        "renagMinutes": "+5",
        "nudgeRepeatMinutes": "1441",
        "allDayFrom": "7:00",
        "fatFingers": true,
        "textSize": 1,
        "speechModel": "base",
        "streaksOff": "true",
    });
    assert_eq!(
        taken_settings(&record, SettingsApp::Desktop),
        pairs(&[("streaksOff", "true")])
    );
    assert!(taken_settings(&json!("theme"), SettingsApp::Desktop).is_empty());
    Ok(())
}

#[test]
fn a_crafted_record_names_nothing_but_the_settings_carried() {
    let record = json!({
        "signseed": "00".repeat(32),
        "device": "a".repeat(32),
        "author": "Mallory",
        "lockPin": "pbkdf2-sha256:1:aa:bb",
        "appLock": "true",
        "lockedAreas": "area1",
        "backup:key": "AAAA",
        "backup:folder": "/elsewhere",
        "backup:auto": "on",
        "folder": "/elsewhere",
        "paired:x": "{}",
        "key:x": "{}",
        "ask:on": "true",
        "ask:openai:url": "https://example.org/v1",
        "ai:token": "t",
        "speechModelOffer": "small:file",
        "alarmSound": "/elsewhere/bell.ogg",
        "theme": "light",
    });
    for app in [SettingsApp::Phone, SettingsApp::Desktop] {
        assert_eq!(taken_settings(&record, app), pairs(&[("theme", "light")]));
    }
}

#[test]
fn a_speech_model_still_offered_goes_into_the_next_backup() -> Result<()> {
    let store = crate::store::tests::store();
    store.set_local_setting("speechModelOffer", "base:download")?;
    let settings = DeviceSettings::new(&store);
    assert_eq!(
        settings.speech_model_offer()?,
        Some(SpeechModelChoice {
            name: "base".into(),
            source: SpeechModelSource::Download,
        })
    );
    assert_eq!(settings.speech_model()?, None);
    assert_eq!(
        carried_settings(&store)?
            .get("speechModel")
            .map(String::as_str),
        Some("base:download")
    );
    settings.drop_speech_model_offer()?;
    assert_eq!(settings.speech_model_offer()?, None);
    assert!(carried_settings(&store)?.is_empty());
    Ok(())
}

#[test]
fn the_speech_model_is_kept_with_how_it_came_and_no_other_is_taken() -> Result<()> {
    let store = crate::store::tests::store();
    let settings = DeviceSettings::new(&store);
    assert_eq!(settings.speech_model()?, None);
    let base = SpeechModelChoice {
        name: "base".into(),
        source: SpeechModelSource::Download,
    };
    settings.set_speech_model(Some(&base))?;
    assert_eq!(
        store.local_setting("speechModel")?.as_deref(),
        Some("base:download")
    );
    assert_eq!(settings.speech_model()?, Some(base));
    let large = SpeechModelChoice {
        name: "large".into(),
        source: SpeechModelSource::File,
    };
    assert!(settings.set_speech_model(Some(&large)).is_err());
    settings.set_speech_model(None)?;
    assert_eq!(store.local_setting("speechModel")?, None);
    store.set_local_setting("speechModel", "base:cloud")?;
    assert_eq!(settings.speech_model()?, None);
    Ok(())
}
