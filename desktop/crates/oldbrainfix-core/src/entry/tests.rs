use super::*;
use chrono::TimeZone;

#[test]
fn instants_print_as_dart_prints_them() {
    let at = Utc.with_ymd_and_hms(2026, 1, 2, 9, 0, 0).single();
    let Some(at) = at else { panic!("a valid date") };
    assert_eq!(dart_iso(at), "2026-01-02T09:00:00.000Z");
    let later = at + chrono::Duration::microseconds(500);
    assert_eq!(dart_iso(later), "2026-01-02T09:00:00.000500Z");
    let millis = at + chrono::Duration::milliseconds(123);
    assert_eq!(dart_iso(millis), "2026-01-02T09:00:00.123Z");
    assert_eq!(parse_time(&dart_iso(later)).ok(), Some(later));
    assert!(parse_time("yesterday").is_err());
}
