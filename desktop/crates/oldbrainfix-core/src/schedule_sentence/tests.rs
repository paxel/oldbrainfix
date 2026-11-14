use super::*;

fn at(hour: u32, minute: u32) -> ClockTime {
    ClockTime::new(hour, minute).unwrap_or(ClockTime { hour: 0, minute: 0 })
}

const TODAY: Day = Day {
    year: 2026,
    month: 11,
    day: 3,
};

fn unknown(text: &str, locale: &str) -> Vec<String> {
    read(text, locale, TODAY)
        .unknown
        .into_iter()
        .map(|r| text[r].to_string())
        .collect()
}

fn suggested(text: &str, locale: &str) -> Vec<String> {
    suggest(text, text.len(), locale)
        .into_iter()
        .map(|s| s.text)
        .collect()
}

#[test]
fn a_weekday_out_of_range_reads_as_the_nearest() {
    let schedule = SimpleSchedule::new(SimpleRepeat::weekdays([0, 9].into()), TODAY, &[at(8, 0)]);
    assert_eq!(
        sentence(&schedule, "en", TODAY),
        "Weekdays Mon, Sun at 08:00"
    );
}

#[test]
fn the_issues_example_reads_both_ways() {
    let schedule = SimpleSchedule::new(
        SimpleRepeat::weekdays([1, 4].into()).ending(None, Some(Day::new(2026, 12, 31))),
        TODAY,
        &[at(15, 0), at(8, 0)],
    );
    let en = sentence(&schedule, "en", TODAY);
    let de = sentence(&schedule, "de_DE", TODAY);
    assert_eq!(en, "Weekdays Mon, Thu at 08:00, 15:00, until 12/31");
    assert_eq!(de, "Wochentags Mo, Do um 08:00, 15:00, bis 31.12.");
    for (text, locale) in [(en, "en"), (de, "de")] {
        let back = read(&text, locale, TODAY);
        assert_eq!(back.schedule.as_ref(), Some(&schedule), "{text}");
        assert!(back.unknown.is_empty(), "{text}");
    }
}

#[test]
fn the_start_and_the_year_show_only_when_not_today() {
    let later = SimpleSchedule::new(
        SimpleRepeat::every(2, RepeatUnit::Weeks),
        Day::new(2027, 1, 5),
        &[],
    );
    assert_eq!(
        sentence(&later, "en", TODAY),
        "Every 2 weeks, from 1/5/2027"
    );
    assert_eq!(sentence(&later, "de", TODAY), "Alle 2 Wochen, ab 5.1.2027");
    let now = SimpleSchedule::new(SimpleRepeat::every(1, RepeatUnit::Days), TODAY, &[]);
    assert_eq!(sentence(&now, "en", TODAY), "Every day");
    assert_eq!(read("Every day", "en", TODAY).schedule, Some(now));
}

#[test]
fn unknown_parts_are_marked_and_change_nothing() {
    assert_eq!(unknown("Every day at 08:00 please", "en"), ["please"]);
    assert_eq!(unknown("Every 2 wekks", "en"), ["Every", "2", "wekks"]);
    assert_eq!(read("Every 2 wekks", "en", TODAY).schedule, None);
    assert_eq!(
        unknown("Once at 08:00, 09:00, until 12/1", "en"),
        ["09:00", "until", "12/1"]
    );
    assert_eq!(unknown("Jeden Tag ab 31.2.", "de"), ["ab", "31.2."]);
    assert_eq!(unknown("Jeden Tag 🧠", "de"), ["🧠"]);
}

#[test]
fn suggestions_are_the_next_words() {
    assert_eq!(suggested("Every 2 w", "en"), ["weeks"]);
    assert_eq!(suggested("Weekdays Tue, T", "en"), ["Thu"]);
    assert_eq!(suggested("Every day at 8", "en"), ["08:00", "08:30"]);
    assert_eq!(suggested("Einmal ", "de"), ["am", "um"]);
    let mid = suggest("Every 2 we at 08:00", 10, "en");
    assert_eq!(
        mid,
        [Suggestion {
            range: 8..10,
            text: "weeks".into()
        }]
    );
}

/// A Tuesday afternoon; the day starts at 07:30.
const NOW: SentenceNow = SentenceNow {
    today: TODAY,
    time: ClockTime {
        hour: 14,
        minute: 20,
    },
    day_start: ClockTime {
        hour: 7,
        minute: 30,
    },
};

fn read_now(text: &str, locale: &str, now: &SentenceNow) -> Option<SimpleSchedule> {
    read_at(text, locale, now).schedule
}

fn once(day: Day, time: Option<ClockTime>) -> Option<SimpleSchedule> {
    let times: Vec<ClockTime> = time.into_iter().collect();
    Some(SimpleSchedule::new(SimpleRepeat::once(), day, &times))
}

fn unknown_now(text: &str, locale: &str) -> Vec<String> {
    read_at(text, locale, &NOW)
        .unknown
        .into_iter()
        .map(|r| text[r].to_string())
        .collect()
}

#[test]
fn a_relative_day_is_at_the_day_start_at_19_or_at_the_time_typed() {
    let tomorrow = TODAY.add_days(1);
    let cases = [
        ("tomorrow", "en", once(tomorrow, Some(at(7, 30)))),
        ("Tomorrow 7 pm", "en", once(tomorrow, Some(at(19, 0)))),
        ("tomorrow at 7:30pm", "en", once(tomorrow, Some(at(19, 30)))),
        ("tomorrow evening", "en", once(tomorrow, Some(at(19, 0)))),
        ("tonight", "en", once(TODAY, Some(at(19, 0)))),
        ("Tonight 8 PM", "en", once(TODAY, Some(at(20, 0)))),
        ("today", "en", once(TODAY, None)),
        ("today 12 am", "en", once(TODAY, Some(at(0, 0)))),
        ("today 12 pm", "en", once(TODAY, Some(at(12, 0)))),
        ("morgen 19 Uhr", "de", once(tomorrow, Some(at(19, 0)))),
        ("heute Abend", "de", once(TODAY, Some(at(19, 0)))),
        ("heute", "de", once(TODAY, None)),
        (
            "Once tomorrow at 15:00",
            "en",
            once(tomorrow, Some(at(15, 0))),
        ),
        ("Once on tomorrow", "en", once(tomorrow, Some(at(7, 30)))),
        (
            "Einmal am morgen um 15:00",
            "de",
            once(tomorrow, Some(at(15, 0))),
        ),
        ("in 2 hours", "en", once(TODAY, Some(at(16, 20)))),
        ("in an hour", "en", once(TODAY, Some(at(15, 20)))),
        ("in einer Minute", "de", once(TODAY, Some(at(14, 21)))),
        ("in 3 days", "en", once(TODAY.add_days(3), Some(at(7, 30)))),
        (
            "in 2 weeks 9 am",
            "en",
            once(TODAY.add_days(14), Some(at(9, 0))),
        ),
        ("in 3 Tagen", "de", once(TODAY.add_days(3), Some(at(7, 30)))),
    ];
    for (text, locale, schedule) in cases {
        assert_eq!(read_now(text, locale, &NOW), schedule, "{text}");
    }
    let late = SentenceNow {
        today: Day::new(2026, 12, 31),
        time: at(23, 30),
        ..NOW
    };
    assert_eq!(
        read_now("in 90 min", "en", &late),
        once(Day::new(2027, 1, 1), Some(at(1, 0)))
    );
}

#[test]
fn relative_words_are_read_only_at_a_now() {
    assert_eq!(read("tomorrow", "en", TODAY).schedule, None);
    assert_eq!(unknown("in 2 hours", "en"), ["in", "2", "hours"]);
    assert_eq!(
        read("Every day at 7 pm", "en", TODAY).schedule,
        Some(SimpleSchedule::new(
            SimpleRepeat::every(1, RepeatUnit::Days),
            TODAY,
            &[at(19, 0)]
        ))
    );
}

#[test]
fn relative_parts_that_do_not_fit_are_marked() {
    assert_eq!(unknown_now("today 13 pm", "en"), ["13 pm"]);
    assert_eq!(unknown_now("heute 25 Uhr", "de"), ["25 Uhr"]);
    assert_eq!(unknown_now("in 0 minutes", "en"), ["in", "0", "minutes"]);
    assert_eq!(unknown_now("in 2 hours at 08:00", "en"), ["08:00"]);
    assert!(unknown_now("Einmal am 5.11.", "de").is_empty());
}

#[test]
fn relative_words_are_suggested() {
    assert_eq!(suggested("T", "en"), ["Today", "Tonight", "Tomorrow"]);
    assert_eq!(suggested("Tomorrow ", "en"), ["evening", "at"]);
    assert_eq!(
        suggested("In 2 ", "en"),
        ["minutes", "hours", "days", "weeks"]
    );
    assert_eq!(suggested("in a ", "en"), ["minute", "hour", "day", "week"]);
    assert_eq!(suggested("Heute ", "de"), ["Abend", "um"]);
    assert_eq!(suggested("In 3 T", "de"), ["Tagen"]);
    assert!(suggested("Tomorrow 7 ", "en").is_empty());
}

#[test]
fn the_chips_are_their_words_read_at_now() {
    let chip = |chip, day, time| QuickReminder { chip, day, time };
    assert_eq!(
        quick_reminders(&NOW),
        [
            chip(QuickChip::InAnHour, TODAY, at(15, 20)),
            chip(QuickChip::Tonight, TODAY, at(19, 0)),
            chip(QuickChip::Tomorrow, TODAY.add_days(1), at(7, 30)),
            chip(QuickChip::InAWeek, TODAY.add_days(7), at(7, 30)),
        ]
    );
    let word = |chip: QuickChip, german: bool| match (chip, german) {
        (QuickChip::InAnHour, false) => "in 1 hour",
        (QuickChip::Tonight, false) => "tonight",
        (QuickChip::Tomorrow, false) => "tomorrow",
        (QuickChip::InAWeek, false) => "in 1 week",
        (QuickChip::InAnHour, true) => "in 1 Stunde",
        (QuickChip::Tonight, true) => "heute Abend",
        (QuickChip::Tomorrow, true) => "morgen",
        (QuickChip::InAWeek, true) => "in 1 Woche",
    };
    let moments = [
        NOW,
        SentenceNow {
            today: Day::new(2026, 12, 31),
            time: at(23, 30),
            day_start: at(9, 0),
        },
        SentenceNow {
            time: at(0, 0),
            day_start: at(6, 15),
            ..NOW
        },
    ];
    for now in moments {
        for quick in quick_reminders(&now) {
            for (locale, german) in [("en", false), ("de", true)] {
                assert_eq!(
                    read_now(word(quick.chip, german), locale, &now),
                    Some(quick.schedule()),
                    "{now:?} {locale}"
                );
            }
        }
    }
    // "Tonight" while 19:00 is still to come.
    let evening = SentenceNow {
        time: at(19, 0),
        ..NOW
    };
    let chips: Vec<QuickChip> = quick_reminders(&evening).iter().map(|q| q.chip).collect();
    assert_eq!(
        chips,
        [QuickChip::InAnHour, QuickChip::Tomorrow, QuickChip::InAWeek]
    );
}
