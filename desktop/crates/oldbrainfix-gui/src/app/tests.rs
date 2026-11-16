use super::*;
use chrono::TimeZone;

#[test]
fn area_colours_read_as_the_phone_reads_them() {
    assert_eq!(
        area_colour(Some("6A1B9A")),
        Some(Color32::from_rgb(0x6A, 0x1B, 0x9A))
    );
    assert_eq!(area_colour(Some("nope")), None);
    assert_eq!(area_colour(None), None);
}

#[test]
fn a_bundle_for_a_person_is_named_after_them() {
    let Some(day) = chrono::NaiveDate::from_ymd_opt(2026, 3, 4) else {
        panic!("a valid date")
    };
    assert_eq!(
        changes_file_name(day, "profile:1", "Ann-Marie Öz!"),
        "oldbrainfix-2026-03-04-Ann-Marie-Öz.obfsync"
    );
    assert_eq!(
        changes_file_name(day, "profile:1", " "),
        "oldbrainfix-2026-03-04-profile-1.obfsync"
    );
}

#[test]
fn a_report_mails_the_details_percent_encoded_and_names_no_file() {
    let url = report_url("unreadable: .ogg\nunsupported");
    assert!(
        url.starts_with("mailto:taum@tuta.io?subject=Old%20Brain%20Fix%20problem%20report&body=")
    );
    assert!(
        url.ends_with("%0A%0Aunreadable%3A%20.ogg%0Aunsupported"),
        "{url}"
    );
    assert_eq!(percent_encoded("ä &"), "%C3%A4%20%26");
    let line = problem_line(oldbrainfix_whisper::Problem::Unsupported, "Dr. Ben.OGG");
    assert_eq!(line, "unsupported: .OGG");
    let line = problem_line(oldbrainfix_whisper::Problem::Unreadable, "voice");
    assert_eq!(line, "unreadable");
}

#[test]
fn capture_times_read_in_the_apps_language() {
    let at = Utc.with_ymd_and_hms(2026, 3, 4, 12, 0, 0).single();
    let Some(at) = at else { panic!("a valid date") };
    let local = at.with_timezone(&Local);
    let en = when(&L10n::new("en"), at);
    let de = when(&L10n::new("de"), at);
    assert!(
        en.starts_with(&local.format("%-m/%-d/%Y").to_string()),
        "{en}"
    );
    assert!(
        de.starts_with(&local.format("%-d.%-m.%Y").to_string()),
        "{de}"
    );
}
