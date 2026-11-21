use super::*;
use crate::alerts::{OPEN, Tap};

fn at() -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::from_timestamp(1_790_000_000, 0).unwrap_or_default()
}

#[test]
fn a_click_on_a_macos_notification_is_open() {
    // obf-1039: the click is the way in on macOS, with no buttons.
    assert_eq!(
        clicked_tap("routine@2026-11-20T09:00".into(), true, at()),
        Some(Tap {
            key: "routine@2026-11-20T09:00".into(),
            action: OPEN.into(),
            at: at(),
        })
    );
}

#[test]
fn a_macos_notification_gone_unclicked_taps_nothing() {
    assert_eq!(clicked_tap("nudge".into(), false, at()), None);
}
