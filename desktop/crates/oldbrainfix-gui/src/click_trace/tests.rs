use super::*;

fn tap(key: &str, action: &str, second: i64) -> Tap {
    Tap {
        key: key.into(),
        action: action.into(),
        at: DateTime::from_timestamp(1_795_510_800 + second, 0).unwrap_or_default(),
    }
}

#[test]
fn a_click_is_traced_as_it_came_and_where_it_went() {
    let trace = ClickTrace::default();
    trace.set_shown(false);
    trace.arrived(&tap("routine:a@2026-11-24T09:00", OPEN, 0));
    assert_eq!(
        trace.lines(),
        ["2026-11-24 09:00:00Z routine:a@2026-11-24T09:00 click, tray: not placed"]
    );
    trace.went("routine:a@2026-11-24T09:00", "Agenda today");
    trace.set_shown(true);
    trace.arrived(&tap(NUDGE_KEY, "snooze10", 3));
    trace.went(NUDGE_KEY, "snoozed");
    assert_eq!(
        trace.lines(),
        [
            "2026-11-24 09:00:00Z routine:a@2026-11-24T09:00 click, tray: Agenda today",
            "2026-11-24 09:00:03Z Nudge snooze10, window: snoozed",
        ]
    );
}

#[test]
fn the_trace_keeps_the_latest_clicks_only() {
    let trace = ClickTrace::default();
    for second in 0..(KEPT as i64 + 5) {
        trace.arrived(&tap(CATCH_UP_KEY, OPEN, second));
    }
    let clicks = trace.clicks();
    assert_eq!(clicks.len(), KEPT);
    assert_eq!(clicks.first().map(|c| c.at), Some(tap("", "", 5).at));
}

#[test]
fn where_it_went_goes_to_the_latest_click_not_placed() {
    let trace = ClickTrace::default();
    trace.arrived(&tap(CHANGES_KEY, OPEN, 0));
    trace.arrived(&tap(CHANGES_KEY, OPEN, 1));
    trace.went(CHANGES_KEY, "send changes");
    let went: Vec<Option<String>> = trace.clicks().into_iter().map(|c| c.went).collect();
    assert_eq!(went, [None, Some("send changes".to_string())]);
}
