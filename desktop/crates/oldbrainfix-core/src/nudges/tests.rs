use super::*;
use chrono::NaiveDateTime;

fn utc_wall(utc: DateTime<Utc>) -> NaiveDateTime {
    utc.naive_utc()
}

fn at(h: u32, m: u32) -> ClockTime {
    ClockTime { hour: h, minute: m }
}

#[test]
fn quiet_hours_over_midnight_hold_a_nudge_until_they_end() -> Result<()> {
    let quiet = QuietHours {
        start: at(22, 0),
        end: at(8, 0),
    };
    assert!(quiet.contains(at(23, 0)));
    assert!(quiet.contains(at(7, 59)));
    assert!(!quiet.contains(at(8, 0)));
    let late = crate::parse_time("2026-01-01T23:00:00.000Z")?;
    let morning = crate::parse_time("2026-01-02T08:00:00.000Z")?;
    assert_eq!(quiet.defer(late, &utc_wall), morning);
    assert_eq!(quiet.defer(morning, &utc_wall), morning);
    Ok(())
}
