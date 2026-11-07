import XCTest

@testable import Runner

/// The calendar channel's rules on the iPhone (#241), as
/// AppointmentsTest.kt and CaptureExtrasTest.kt check them on Android:
/// EventKit's events as the channel answers them, the mirror's mark,
/// Occurrence keys and the RRULEs the core writes.
class CalendarRulesTests: XCTestCase {
  /// Berlin: UTC+1 in winter, UTC+2 in summer.
  private var berlin: Calendar {
    var calendar = Calendar(identifier: .gregorian)
    calendar.timeZone = TimeZone(identifier: "Europe/Berlin")!
    return calendar
  }

  private func at(_ text: String, in calendar: Calendar) -> Date {
    let formatter = DateFormatter()
    formatter.locale = Locale(identifier: "en_US_POSIX")
    formatter.calendar = calendar
    formatter.timeZone = calendar.timeZone
    formatter.dateFormat = "yyyy-MM-dd HH:mm:ss"
    return formatter.date(from: text)!
  }

  private func utcMillis(_ text: String) -> Int64 {
    var utc = Calendar(identifier: .gregorian)
    utc.timeZone = TimeZone(identifier: "UTC")!
    return CalendarRules.millis(at(text, in: utc))
  }

  private func event(
    _ id: String = "e1", start: String, end: String, allDay: Bool = false, title: String = "Dentist",
    calendar: String = "c1", declined: Bool = false, url: String? = nil
  ) -> EventFacts {
    EventFacts(
      id: id, calendarId: calendar, title: title, start: at(start, in: berlin),
      end: at(end, in: berlin), allDay: allDay, location: "Main St", notes: "Bring the card",
      declined: declined, url: url)
  }

  // MARK: Appointments

  func testATimedEventIsAsItIs() {
    let row = CalendarRules.row(
      event(start: "2026-11-05 14:30:00", end: "2026-11-05 15:30:00"), in: berlin)
    XCTAssertEqual(row["begin"] as? Int64, utcMillis("2026-11-05 13:30:00"))
    XCTAssertEqual(row["end"] as? Int64, utcMillis("2026-11-05 14:30:00"))
    XCTAssertEqual(row["allDay"] as? Bool, false)
    XCTAssertEqual(row["calendar"] as? String, "c1")
    XCTAssertEqual(row["title"] as? String, "Dentist")
    XCTAssertEqual(row["location"] as? String, "Main St")
    XCTAssertEqual(row["description"] as? String, "Bring the card")
  }

  /// EventKit ends a one-day all-day event on that day; Android's
  /// calendar keeps it from midnight UTC to midnight UTC.
  func testAnAllDayEventIsMidnightUtcToMidnightUtcAfterItsLastDay() {
    let oneDay = CalendarRules.span(
      event(start: "2026-11-05 00:00:00", end: "2026-11-05 23:59:59", allDay: true), in: berlin)
    XCTAssertEqual(oneDay.begin, utcMillis("2026-11-05 00:00:00"))
    XCTAssertEqual(oneDay.end, utcMillis("2026-11-06 00:00:00"))

    let atMidnight = CalendarRules.span(
      event(start: "2026-11-05 00:00:00", end: "2026-11-06 00:00:00", allDay: true), in: berlin)
    XCTAssertEqual(atMidnight.end, utcMillis("2026-11-06 00:00:00"))

    let threeDays = CalendarRules.span(
      event(start: "2026-07-01 00:00:00", end: "2026-07-03 23:59:59", allDay: true), in: berlin)
    XCTAssertEqual(threeDays.begin, utcMillis("2026-07-01 00:00:00"))
    XCTAssertEqual(threeDays.end, utcMillis("2026-07-04 00:00:00"))

    // An end before the start is a day, not less.
    let backwards = CalendarRules.span(
      event(start: "2026-11-05 00:00:00", end: "2026-11-04 00:00:00", allDay: true), in: berlin)
    XCTAssertEqual(backwards.end, utcMillis("2026-11-06 00:00:00"))
  }

  func testEachOccurrenceIsARowOfItsOwn() {
    let monday = CalendarRules.row(event(start: "2026-11-02 08:00:00", end: "2026-11-02 09:00:00"), in: berlin)
    let tuesday = CalendarRules.row(event(start: "2026-11-03 08:00:00", end: "2026-11-03 09:00:00"), in: berlin)
    XCTAssertNotEqual(monday["id"] as? String, tuesday["id"] as? String)
    XCTAssertEqual(monday["id"] as? String, "e1@\(utcMillis("2026-11-02 07:00:00"))")
  }

  func testTheAgendaShowsPickedCalendarsWithoutDeclinedOrTheMirrorsOwn() {
    let events = [
      event("a", start: "2026-11-05 08:00:00", end: "2026-11-05 09:00:00"),
      event("b", start: "2026-11-05 08:00:00", end: "2026-11-05 09:00:00", calendar: "other"),
      event("c", start: "2026-11-05 08:00:00", end: "2026-11-05 09:00:00", declined: true),
      event("d", start: "2026-11-05 08:00:00", end: "2026-11-05 09:00:00", url: "oldbrainfix:mirror/r1@oldbrainfix"),
      event("e", start: "2026-11-05 08:00:00", end: "2026-11-05 09:00:00", url: "https://example.org"),
      event("a", start: "2026-11-05 08:00:00", end: "2026-11-05 09:00:00"),
    ]
    let shown = CalendarRules.shown(events, calendarIds: ["c1"], in: berlin)
    XCTAssertEqual(shown.compactMap { ($0["id"] as? String)?.components(separatedBy: "@").first }, ["a", "e"])
  }

  func testAColourIsArgb() {
    XCTAssertEqual(CalendarRules.colour(red: 1, green: 0, blue: 0, alpha: 1), 0xFFFF_0000)
    XCTAssertEqual(CalendarRules.colour(red: 0, green: 0.5, blue: 1, alpha: 1), 0xFF00_80FF)
    XCTAssertEqual(CalendarRules.colour(red: 2, green: -1, blue: 0, alpha: 0), 0x00FF_0000)
  }

  // MARK: Running now

  func testRunningNowHasATitleAndWasNotDeclined() {
    let now = at("2026-11-05 14:45:00", in: berlin)
    let events = [
      event("a", start: "2026-11-05 14:30:00", end: "2026-11-05 15:30:00"),
      event("b", start: "2026-11-05 14:30:00", end: "2026-11-05 15:30:00", declined: true),
      event("c", start: "2026-11-05 14:30:00", end: "2026-11-05 15:30:00", title: "  "),
      event("d", start: "2026-11-05 15:30:00", end: "2026-11-05 16:30:00", title: "Later"),
      event("e", start: "2026-11-05 13:30:00", end: "2026-11-05 14:45:00", title: "Just ended"),
      event("f", start: "2026-11-05 00:00:00", end: "2026-11-05 23:59:59", allDay: true, title: "Holiday"),
      event("g", start: "2026-11-06 00:00:00", end: "2026-11-06 23:59:59", allDay: true, title: "Tomorrow"),
    ]
    let running = CalendarRules.running(events, now: now, in: berlin)
    XCTAssertEqual(running.compactMap { $0["title"] as? String }, ["Dentist", "Holiday"])
    XCTAssertEqual(running.compactMap { $0["allDay"] as? Bool }, [false, true])
  }

  /// Just after midnight Berlin's date is a day ahead of UTC's: the
  /// local date decides, as on Android.
  func testAnAllDayEventRunsByTheLocalDate() {
    let lateEvening = at("2026-11-05 23:30:00", in: berlin)
    let earlyMorning = at("2026-11-05 00:30:00", in: berlin)
    let holiday = [event(start: "2026-11-05 00:00:00", end: "2026-11-05 23:59:59", allDay: true)]
    XCTAssertEqual(CalendarRules.running(holiday, now: lateEvening, in: berlin).count, 1)
    XCTAssertEqual(CalendarRules.running(holiday, now: earlyMorning, in: berlin).count, 1)
    XCTAssertEqual(
      CalendarRules.running(holiday, now: at("2026-11-06 00:30:00", in: berlin), in: berlin).count, 0)
  }

  // MARK: The mirror's mark

  func testTheMarkIsTheUidBehindTheAppsPrefix() {
    XCTAssertEqual(MirrorMark.url("r1-0800@oldbrainfix")?.absoluteString, "oldbrainfix:mirror/r1-0800@oldbrainfix")
    XCTAssertEqual(MirrorMark.uid("oldbrainfix:mirror/r1-0800@oldbrainfix"), "r1-0800@oldbrainfix")
    XCTAssertNil(MirrorMark.uid("oldbrainfix:mirror/"))
    XCTAssertNil(MirrorMark.uid("https://example.org"))
    XCTAssertNil(MirrorMark.uid(nil))
  }

  // MARK: Occurrence keys

  func testAKeyIsAllDayOrAtATimeInTheDevicesZone() throws {
    let allDay = try XCTUnwrap(StartKey.span("2026-10-25", minutes: 15, in: berlin))
    XCTAssertTrue(allDay.allDay)
    XCTAssertEqual(allDay.start, at("2026-10-25 00:00:00", in: berlin))
    XCTAssertEqual(allDay.end, allDay.start)

    let timed = try XCTUnwrap(StartKey.span("2026-10-25T08:00", minutes: 15, in: berlin))
    XCTAssertFalse(timed.allDay)
    // The day summer time ends: 08:00 is UTC+1 already.
    XCTAssertEqual(CalendarRules.millis(timed.start), utcMillis("2026-10-25 07:00:00"))
    XCTAssertEqual(timed.end.timeIntervalSince(timed.start), 15 * 60)
  }

  func testAnHourSkippedBySummerTimeMovesOn() throws {
    let skipped = try XCTUnwrap(StartKey.span("2026-03-29T02:30", minutes: 15, in: berlin))
    XCTAssertEqual(berlin.component(.hour, from: skipped.start), 3)
  }

  func testNoKeyIsNoStart() {
    for key in ["", "2026-10-5", "2026-13-01", "2026-10-25T8:00", "2026-10-25T24:00", "2026-10-25 08:00", "abcd-ef-gh"] {
      XCTAssertNil(StartKey.parse(key), key)
    }
  }

  // MARK: Rules

  func testTheCoresRulesAreRead() {
    XCTAssertEqual(
      RuleParts.parse("FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,WE"),
      RuleParts(frequency: .weekly, interval: 2, byDay: [.init(weekday: 2), .init(weekday: 4)]))
    XCTAssertEqual(
      RuleParts.parse("RRULE:FREQ=MONTHLY;BYDAY=-1FR"),
      RuleParts(frequency: .monthly, byDay: [.init(weekday: 6, ordinal: -1)]))
    XCTAssertEqual(
      RuleParts.parse("FREQ=MONTHLY;COUNT=5;BYMONTHDAY=-1,15"),
      RuleParts(frequency: .monthly, count: 5, byMonthDay: [-1, 15]))
    XCTAssertEqual(
      RuleParts.parse("FREQ=YEARLY;BYMONTH=3;BYDAY=1SU"),
      RuleParts(frequency: .yearly, byDay: [.init(weekday: 1, ordinal: 1)], byMonth: [3]))
    XCTAssertEqual(RuleParts.parse("FREQ=DAILY;WKST=SU"), RuleParts(frequency: .daily))
  }

  func testEveryDayOnSomeWeekdaysIsEveryWeekOnThem() {
    XCTAssertEqual(
      RuleParts.parse("FREQ=DAILY;BYDAY=MO,FR"),
      RuleParts(frequency: .weekly, byDay: [.init(weekday: 2), .init(weekday: 6)]))
  }

  func testUntilIsWallClockOrTheEndOfItsDay() throws {
    let timed = try XCTUnwrap(RuleParts.parse("FREQ=DAILY;UNTIL=20261231T235900"))
    XCTAssertEqual(timed.untilDate(in: berlin), at("2026-12-31 23:59:00", in: berlin))
    let dated = try XCTUnwrap(RuleParts.parse("FREQ=DAILY;UNTIL=20261231"))
    XCTAssertEqual(dated.untilDate(in: berlin), at("2026-12-31 23:59:59", in: berlin))
    let utc = try XCTUnwrap(RuleParts.parse("FREQ=DAILY;UNTIL=20261231T120000Z"))
    XCTAssertEqual(utc.untilDate(in: berlin).map(CalendarRules.millis), utcMillis("2026-12-31 12:00:00"))
    XCTAssertNil(RuleParts(frequency: .daily).untilDate(in: berlin))
  }

  func testWhatEventKitCannotHoldIsNoRule() {
    for rule in [
      "", "INTERVAL=2", "FREQ=HOURLY", "FREQ=DAILY;INTERVAL=0", "FREQ=DAILY;COUNT=x",
      "FREQ=WEEKLY;BYDAY=XX", "FREQ=MONTHLY;BYMONTHDAY=0", "FREQ=YEARLY;BYMONTH=13",
      "FREQ=DAILY;UNTIL=2026", "FREQ=DAILY;NOVALUE",
    ] {
      XCTAssertNil(RuleParts.parse(rule), rule)
    }
  }
}
