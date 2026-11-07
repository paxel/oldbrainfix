import XCTest

@testable import Runner

/// The Agenda widget's rules on the iPhone (#242), as
/// android/…/AgendaWidgetPlanTest.kt covers Android's: the snapshot the
/// app hands over, read from the fixture a Dart test writes
/// (test/ios_widget_fixture_test.dart), what shows on which day, what
/// ticks, what fits and how a tick is queued for the app.
class AgendaWidgetPlanTests: XCTestCase {
  private let berlin = TimeZone(identifier: "Europe/Berlin") ?? .current

  private func fixture() throws -> AgendaSnapshot {
    let url = try XCTUnwrap(
      Bundle(for: AgendaWidgetPlanTests.self).url(
        forResource: "agenda_snapshot", withExtension: "json"))
    return try XCTUnwrap(AgendaWidgetPlan.snapshot(try Data(contentsOf: url)))
  }

  private func view(_ snapshot: AgendaSnapshot?, today: String = "2026-10-24",
    pending: [QueuedAction] = [], appointments: [String: [WidgetRow]] = [:]
  ) -> AgendaView {
    AgendaWidgetPlan.view(
      snapshot, today: today, pending: pending, appointments: appointments, openApp: "Open")
  }

  private func queued(_ build: (inout ActionQueue) -> Void) -> [QueuedAction] {
    var queue = ActionQueue()
    build(&queue)
    return queue.pending
  }

  // MARK: The snapshot

  func testTheFixtureOfTheDartTestReadsWithEveryRow() throws {
    let snapshot = try fixture()
    XCTAssertEqual(snapshot.days.keys.sorted(), ["2026-10-24", "2026-10-25", "2026-10-26"])
    let today = try XCTUnwrap(snapshot.days["2026-10-24"])
    XCTAssertEqual(
      today.map(\.kind),
      [
        "occurrence", "shopping", "shopping", "checklist", "checklistLine", "occurrence",
        "occurrence", "line", "line", "session", "pick", "heading", "frog",
      ])
    XCTAssertEqual(today[0].block, WidgetRow.allDay)
    XCTAssertEqual(today[0].routine, "water")
    XCTAssertEqual(today[0].occurrence, "2026-10-24")
    XCTAssertEqual(today[2].line, "milk")
    XCTAssertEqual(today[2].indent, 1)
    XCTAssertTrue(today[4].done)
    XCTAssertEqual(today[5].title, "Something in Private", "a Masked row stays masked")
    XCTAssertEqual(today[5].at, 480)
    XCTAssertEqual(today[8].indent, 2)
    XCTAssertEqual(today[10].note, "pushups")
    XCTAssertEqual(today[12].frog, "bank")
    XCTAssertEqual(snapshot.headings["2026-10-25"], "Sunday, 25 October")
    XCTAssertEqual(snapshot.missed.map(\.kind), ["heading", "text", "missed"])
    XCTAssertEqual(snapshot.unsorted, 3)
    XCTAssertEqual(snapshot.calendars, ["calendar-1"])
    XCTAssertEqual(snapshot.today, "Today")
    XCTAssertEqual(snapshot.tomorrow, "Tomorrow")
    XCTAssertEqual(snapshot.planned, "Nothing planned")
    XCTAssertEqual(snapshot.unsortedText, "3 Unsorted")
    XCTAssertEqual(snapshot.done, "Done")
    XCTAssertEqual(snapshot.since, "from {time}")
    XCTAssertEqual(snapshot.textScale, AgendaWidgetPlan.fatFingers, "Fat fingers (#224)")
  }

  func testOnlyVersionFourReadsAndABrokenRowIsLeftOut() {
    let rows = #"[{"kind": "occurrence", "title": "A"}, {"kind": "later", "title": "B"}, {"kind": "frog"}]"#
    let json = { (v: String) in
      Data(#"{"v": \#(v), "days": {"2026-10-24": \#(rows)}, "labels": {}}"#.utf8)
    }
    XCTAssertEqual(AgendaWidgetPlan.snapshot(json("4"))?.days["2026-10-24"]?.map(\.title), ["A"])
    XCTAssertEqual(AgendaWidgetPlan.snapshot(json("4"))?.textScale, 1)
    XCTAssertNil(AgendaWidgetPlan.snapshot(json("3")))
    XCTAssertNil(AgendaWidgetPlan.snapshot(json("5")))
    XCTAssertNil(AgendaWidgetPlan.snapshot(json("true")))
    XCTAssertNil(AgendaWidgetPlan.snapshot(Data("not json".utf8)))
    XCTAssertNil(AgendaWidgetPlan.snapshot(nil))
  }

  // MARK: What shows

  func testTodayShowsUnderTheMissedSummaryAndTheComingDaysDimmed() throws {
    let rows = view(try fixture()).rows
    XCTAssertEqual(
      rows.prefix(5).map(\.title), ["Missed", "Pills · 2 times", "Fri, 23 Oct", "3 Unsorted", "Today"])
    XCTAssertEqual(rows[3].kind, WidgetRow.unsorted)
    let tomorrow = try XCTUnwrap(rows.firstIndex { $0.title == "Tomorrow" })
    XCTAssertEqual(rows[tomorrow - 1].kind, WidgetRow.frog, "the Frogs end today")
    XCTAssertEqual(
      rows[tomorrow...].map(\.title),
      ["Tomorrow", "Morning", "Picks 2", "Monday, 26 October", "Nothing planned"])
    XCTAssertTrue(rows[(tomorrow + 1)...].filter { !$0.isHeading }.allSatisfy(\.ahead))
    XCTAssertFalse(rows[..<tomorrow].contains(where: \.ahead))
  }

  func testTheNextDayIsTodayAfterMidnightAndAStaleSnapshotAsksToOpenTheApp() throws {
    let snapshot = try fixture()
    let sunday = view(snapshot, today: "2026-10-25").rows
    XCTAssertEqual(sunday.first { $0.kind == WidgetRow.occurrence }?.ahead, false)
    XCTAssertEqual(sunday.filter { $0.kind == WidgetRow.heading }.map(\.title), ["Missed", "Today", "Tomorrow"])

    let stale = view(snapshot, today: "2026-10-28")
    XCTAssertTrue(stale.stale)
    XCTAssertEqual(stale.rows.last?.title, "Open")
    XCTAssertEqual(view(nil).rows.map(\.title), ["Open"])
  }

  func testOnlyTodaysAndTheMissedRowsTick() throws {
    let rows = view(try fixture()).rows
    let row = { (title: String, ahead: Bool) in
      try XCTUnwrap(rows.first { $0.title == title && $0.ahead == ahead })
    }
    XCTAssertEqual(
      try row("Morning", false).onBox,
      .tick(WidgetTap(kind: "done", routine: "morning", occurrence: "2026-10-24T09:00", line: "")))
    XCTAssertEqual(try row("Morning", true).onBox, .open, "a coming day's box opens the app")
    XCTAssertEqual(
      try row("Fri, 23 Oct", false).onBox,
      .tick(WidgetTap(kind: "done", routine: "pills", occurrence: "2026-10-23T08:00", line: "")))
    XCTAssertEqual(
      try row("Floss", false).onBox,
      .tick(
        WidgetTap(kind: "line", routine: "morning", occurrence: "2026-10-24T09:00", line: "floss")))
    XCTAssertEqual(
      try row("Milk", false).onBox,
      .tick(WidgetTap(kind: "shopping", routine: "", occurrence: "", line: "milk")))
    XCTAssertEqual(
      try row("Call the bank", false).onBox,
      .tick(WidgetTap(kind: "frog", routine: "bank", occurrence: "", line: "")))
    XCTAssertEqual(try row("Passport", false).onBox, .open, "a checked box is unchecked in the app")
    XCTAssertEqual(try row("Workout", false).onBox, .nothing, "a Session's box only shows Done")
    XCTAssertEqual(
      try row("Push-ups", false).onRow,
      .link(.pick(session: "workout", occurrence: "2026-10-24T18:00", note: "pushups")))
    XCTAssertEqual(try row("Push-ups", false).onBox, try row("Push-ups", false).onRow)
    XCTAssertEqual(try row("3 Unsorted", false).onRow, .unsorted)
    XCTAssertFalse(try row("Groceries", false).box)
    XCTAssertTrue(try row("Groceries", false).listHeading)
  }

  func testWhatIsQueuedShowsCheckedAndTheLastLineChecksItsOccurrenceOff() throws {
    let pending = queued { queue in
      queue.add(
        kind: "line", routine: "morning", occurrence: "2026-10-24T09:00", title: "", body: "",
        line: "teeth", now: 1)
      queue.add(
        kind: "line", routine: "morning", occurrence: "2026-10-24T09:00", title: "", body: "",
        line: "floss", now: 2)
      queue.add(kind: "done", routine: "pills", occurrence: "2026-10-23T08:00", title: "", body: "", now: 3)
      queue.add(kind: "checklist", routine: "", occurrence: "", title: "", body: "", line: "passport", now: 4)
      queue.add(kind: "shopping", routine: "", occurrence: "", title: "", body: "", line: "milk", now: 5)
    }
    let rows = view(try fixture(), pending: pending).rows.filter { !$0.ahead }
    let done = { (title: String) in rows.first { $0.title == title }?.done }
    XCTAssertEqual(done("Teeth"), true)
    XCTAssertEqual(done("Floss"), true)
    XCTAssertEqual(done("Morning"), true, "every Line checked checks its Occurrence off")
    XCTAssertEqual(done("Fri, 23 Oct"), true)
    XCTAssertEqual(done("Milk"), true)
    XCTAssertEqual(done("Water the plants"), false)
  }

  func testOnceAFrogIsEatenOnlyTheEatenOnesShow() {
    let frog = { (id: String) in
      WidgetRow(kind: WidgetRow.frog, title: id, checks: true, frog: id, block: WidgetRow.frogs)
    }
    let pending = queued {
      $0.add(kind: "frog", routine: "a", occurrence: "", title: "", body: "", now: 1)
    }
    XCTAssertEqual(
      AgendaWidgetPlan.checked([frog("a"), frog("b")], pending).map(\.title), ["a"])
    XCTAssertEqual(AgendaWidgetPlan.checked([frog("a"), frog("b")], []).map(\.title), ["a", "b"])
  }

  // MARK: Appointments

  func testAppointmentsGoAmongTheRowsAsOnTheAgenda() throws {
    let snapshot = try fixture()
    let midnightUtc = Date(timeIntervalSince1970: 1_792_800_000)  // 2026-10-24T00:00Z
    let at = { (hour: Int, minute: Int) in
      AgendaWidgetPlan.midnight(of: midnightUtc.addingTimeInterval(43_200), zone: self.berlin)
        .addingTimeInterval(Double(hour * 3600 + minute * 60))
    }
    let entries = [
      CalendarEntry(
        title: "Holiday", begin: midnightUtc, end: midnightUtc.addingTimeInterval(86_400),
        allDay: true, colour: 0xFF0000),
      CalendarEntry(title: " ", begin: at(8, 30), end: at(9, 0), allDay: false, colour: nil),
      CalendarEntry(
        title: "Night", begin: at(23, 0), end: at(26, 0), allDay: false, colour: nil),
    ]
    let rows = AgendaWidgetPlan.appointmentRows(snapshot, entries, day: "2026-10-24", zone: berlin)
    XCTAssertEqual(rows.map(\.title), ["Holiday", "(No title)", "Night"])
    XCTAssertEqual(rows.map(\.time), ["All day", "08:30–09:00", "from 23:00"])
    XCTAssertEqual(rows.first?.colour, 0xFF0000)
    let next = AgendaWidgetPlan.appointmentRows(snapshot, entries, day: "2026-10-25", zone: berlin)
    XCTAssertEqual(next.map(\.time), ["until 02:00"])

    let shown = view(snapshot, appointments: ["2026-10-24": rows]).rows
    let titles = shown.map(\.title)
    let index = { (title: String) in titles.firstIndex(of: title) ?? -1 }
    XCTAssertEqual(index("Holiday"), index("Water the plants") + 1, "after the all-day Occurrences")
    XCTAssertLessThan(index("Something in Private"), index("(No title)"))
    XCTAssertLessThan(index("(No title)"), index("Morning"))
    XCTAssertLessThan(index("Night"), index("Eat a frog"), "before the Frogs")
    XCTAssertGreaterThan(index("Night"), index("Push-ups"))
  }

  func testTheDayEndsAtLocalMidnight() {
    // 2026-10-24 23:30 in Berlin is 21:30 UTC.
    let late = Date(timeIntervalSince1970: 1_792_877_400)
    XCTAssertEqual(AgendaWidgetPlan.dayKey(late, zone: berlin), "2026-10-24")
    let next = AgendaWidgetPlan.nextMidnight(after: late, zone: berlin)
    XCTAssertEqual(AgendaWidgetPlan.dayKey(next, zone: berlin), "2026-10-25")
    XCTAssertEqual(next.timeIntervalSince(late), 30 * 60)
    // Over the clock change on 25 October the day has 25 hours.
    let after = AgendaWidgetPlan.nextMidnight(after: next, zone: berlin)
    XCTAssertEqual(after.timeIntervalSince(next), 25 * 3600)
  }

  // MARK: What fits

  func testAsManyRowsAsFitWithFatFingersFewer() {
    XCTAssertEqual(AgendaFit.lines(height: 154, scale: 1), 7)
    XCTAssertEqual(AgendaFit.lines(height: 154, scale: 1.3), 5)
    XCTAssertEqual(AgendaFit.lines(height: 10, scale: 1), 1)
  }

  func testRowsThatDoNotFitEndInACountAndNeverAfterAHeading() throws {
    let rows = view(try fixture()).rows
    XCTAssertEqual(AgendaFit.fit(rows, lines: rows.count), rows)
    let six = AgendaFit.fit(rows, lines: 6)
    // The fifth row would be "Today" with nothing under it.
    XCTAssertEqual(
      six.map(\.title),
      ["Missed", "Pills · 2 times", "Fri, 23 Oct", "3 Unsorted", "+\(rows.count - 4)"])
    XCTAssertEqual(six.last?.kind, WidgetRow.more)
    XCTAssertEqual(six.last?.onRow, .open)
    let seven = AgendaFit.fit(rows, lines: 7)
    XCTAssertEqual(seven.count, 7)
    XCTAssertEqual(seven[5].title, "Water the plants")
    XCTAssertEqual(seven.last?.title, "+\(rows.count - 6)")
  }

  // MARK: The queue

  func testATickIsQueuedInTheShapeOfAndroidsQueue() {
    var queue = ActionQueue()
    let tap = WidgetTap(kind: "line", routine: "morning", occurrence: "2026-10-24T09:00", line: "floss")
    let action = tap.queue(in: &queue, title: "Floss", body: "", now: 42)
    XCTAssertEqual(
      Set(action.json.keys),
      ["seq", "kind", "routine", "occurrence", "at", "until", "title", "body", "line"])
    XCTAssertEqual(action.json["line"] as? String, "floss")
    XCTAssertEqual(action.channelMap["line"] as? String, "floss")
    XCTAssertEqual((action.json["at"] as? NSNumber)?.int64Value, 42)
    XCTAssertEqual(queue.pending, [action])

    let done = WidgetTap(kind: "done", routine: "r", occurrence: "o", line: "")
      .queue(in: &queue, title: "t", body: "08:00", now: 43)
    XCTAssertNil(done.json["line"], "as EventQueue.kt, a Done carries no line")
    XCTAssertEqual(done.seq, 2)
    for kind in ["line", "frog", "shopping", "checklist"] {
      XCTAssertTrue(QueuedAction.kinds.contains(kind), kind)
    }
  }

  func testTheQueueFileIsSharedAndChangedInOneStep() throws {
    let directory = FileManager.default.temporaryDirectory
      .appendingPathComponent("WidgetFiles-\(UUID().uuidString)", isDirectory: true)
    defer { try? FileManager.default.removeItem(at: directory) }
    XCTAssertTrue(WidgetFiles.readQueue(in: directory).pending.isEmpty)
    let seq = WidgetFiles.updateQueue(in: directory) {
      WidgetTap(kind: "frog", routine: "bank", occurrence: "", line: "")
        .queue(in: &$0, title: "", body: "", now: 1).seq
    }
    XCTAssertEqual(seq, 1)
    XCTAssertEqual(WidgetFiles.readQueue(in: directory).pending.map(\.kind), ["frog"])
    WidgetFiles.updateQueue(in: directory) { $0.ack(1) }
    XCTAssertTrue(WidgetFiles.readQueue(in: directory).pending.isEmpty)
    XCTAssertEqual(WidgetFiles.readQueue(in: directory).seq, 1)

    let empty: [String: Any] = ["v": 4, "days": [String: Any](), "labels": [String: Any]()]
    XCTAssertTrue(WidgetFiles.writeAgenda(empty, in: directory))
    XCTAssertNotNil(AgendaWidgetPlan.snapshot(WidgetFiles.readAgenda(in: directory)))
    XCTAssertEqual(WidgetFiles.group, ShareInbox.group)
  }
}
