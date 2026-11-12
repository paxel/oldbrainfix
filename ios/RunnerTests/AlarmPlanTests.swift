import XCTest

@testable import Runner

/// The iPhone's alarm rules (#238), free of AlarmKit and
/// UserNotifications: the window of 64 rings, Re-nag and Escalation,
/// and the queue of taps the app applies.
class AlarmPlanTests: XCTestCase {
  private let berlin = TimeZone(identifier: "Europe/Berlin") ?? TimeZone.current
  private let minute: Int64 = 60_000

  /// 08:00 on Saturday, 24 October 2026, in Berlin (UTC+2).
  private var eight: Int64 { AlarmPlan.instantOf([2026, 10, 24, 8, 0], zone: berlin) }

  private func alarm(
    _ routine: String, at local: [Int] = [2026, 10, 24, 8, 0], renag: Int64 = 0,
    end: [Int]? = nil, escalate: Int = 0
  ) -> PlannedAlarm {
    PlannedAlarm(
      id: "occurrence:\(routine)@2026-10-24T08:00", routine: routine,
      occurrence: "2026-10-24T08:00", local: local, at: nil, title: routine, body: "08:00",
      renagMs: renag * minute, end: end, escalate: escalate)
  }

  func testWallClockInBerlin() {
    XCTAssertEqual(eight, 1_792_821_600_000)
  }

  func testATimeTheClockSkipsFiresWhenItJumps() {
    // 29 March 2026, 02:30 does not exist in Berlin: 03:00 CEST.
    let skipped = AlarmPlan.instantOf([2026, 3, 29, 2, 30], zone: berlin)
    XCTAssertEqual(skipped, AlarmPlan.instantOf([2026, 3, 29, 3, 0], zone: berlin))
  }

  func testATimeTheClockShowsTwiceFiresAtTheFirst() {
    // 25 October 2026, 02:30 comes twice in Berlin: first in CEST.
    let twice = AlarmPlan.instantOf([2026, 10, 25, 2, 30], zone: berlin)
    XCTAssertEqual(twice, AlarmPlan.instantOf([2026, 10, 25, 0, 30], zone: berlin) + 2 * 60 * minute)
  }

  func testTheWindowHoldsTheNext64RingsSoonestFirst() {
    // Re-nags every 5 minutes for 10 hours: 120 rings, more than iOS keeps.
    let pills = alarm("pills", renag: 5, end: [2026, 10, 24, 18, 0])
    let water = alarm("water", at: [2026, 10, 24, 9, 0])
    let rings = AlarmPlan.window(
      alarms: [pills, water], nudge: nil, changes: nil, zone: berlin, now: eight - minute)

    XCTAssertEqual(rings.count, AlarmPlan.windowSize)
    XCTAssertEqual(rings.map { $0.at }, rings.map { $0.at }.sorted())
    XCTAssertEqual(rings.first?.at, eight)
    // The water's one ring at 09:00 is in it, after pills' 08:55 ring.
    let nine = eight + 60 * minute
    XCTAssertEqual(rings.filter { $0.alarm?.routine == "water" }.map { $0.at }, [nine])
    XCTAssertEqual(rings.last?.at, eight + 62 * 5 * minute)
  }

  func testTheWindowStartsAfterNowAndStopsAtTheEnd() {
    let pills = alarm("pills", renag: 10, end: [2026, 10, 24, 8, 30])
    let rings = AlarmPlan.window(
      alarms: [pills], nudge: nil, changes: nil, zone: berlin, now: eight + 5 * minute)
    XCTAssertEqual(rings.map { $0.at }, [eight + 10 * minute, eight + 20 * minute])
  }

  func testReNagsEscalateAfterTheEscalationCount() {
    // The first ring and two Re-nags ignored: the third Re-nag escalates.
    let pills = alarm("pills", renag: 10, end: [2026, 10, 24, 9, 0], escalate: 2)
    let rings = AlarmPlan.window(
      alarms: [pills], nudge: nil, changes: nil, zone: berlin, now: eight - minute)
    XCTAssertEqual(
      rings.map { $0.kind }, [.alarm, .alarm, .alarm, .escalated, .escalated, .escalated])
  }

  func testWithoutReNagNothingEscalates() {
    let pills = alarm("pills", escalate: 2)
    let rings = AlarmPlan.window(
      alarms: [pills], nudge: nil, changes: nil, zone: berlin, now: eight - minute)
    XCTAssertEqual(rings.map { $0.kind }, [.alarm])
  }

  func testDoneTakesAnOccurrenceOutAndASnoozeRingsAgainReNagging() {
    let pills = alarm("pills", renag: 10, end: [2026, 10, 24, 9, 0], escalate: 1)
    let water = alarm("water")
    var queue = ActionQueue()
    queue.add(
      kind: QueuedAction.done, routine: "water", occurrence: "2026-10-24T08:00", title: "water",
      body: "", now: eight)
    queue.add(
      kind: QueuedAction.snooze, routine: "pills", occurrence: "2026-10-24T08:00",
      title: "pills", body: "08:00", until: eight + 30 * minute, now: eight)

    let plan = AlarmPlan.effective([pills, water], queued: queue.pending)
    XCTAssertEqual(plan.map { $0.id }, ["snooze:pills@2026-10-24T08:00"])
    XCTAssertEqual(plan.first?.at, eight + 30 * minute)
    XCTAssertEqual(plan.first?.renagMs, 10 * minute)
    XCTAssertEqual(plan.first?.escalate, 1)
    let rings = AlarmPlan.window(
      alarms: plan, nudge: nil, changes: nil, zone: berlin, now: eight + minute)
    XCTAssertEqual(rings.map { $0.at }, [eight + 30 * minute, eight + 40 * minute, eight + 50 * minute])
    XCTAssertEqual(rings.map { $0.kind }, [.alarm, .alarm, .escalated])
  }

  func testTheNudgeSharesTheWindow() {
    let nudge = PlannedNudge(
      anchors: [[eight + 30 * minute, eight - 60 * minute]], snoozeAt: nil, snoozeUntil: nil,
      repeatMs: 60 * minute, quiet: nil, since: nil, title: "3 Unsorted", body: "")
    let rings = AlarmPlan.window(
      alarms: [alarm("pills")], nudge: nudge, changes: nil, zone: berlin, now: eight - minute,
      limit: 3)
    XCTAssertEqual(rings.map { $0.kind }, [.alarm, .nudge, .nudge])
    XCTAssertEqual(rings.map { $0.at }, [eight, eight + 30 * minute, eight + 90 * minute])
  }

  func testTheNudgeWaitsForTheEndOfTheQuietHours() {
    // Due at 23:00, quiet from 22:00 to 07:00: it comes at 07:00.
    let due = AlarmPlan.instantOf([2026, 10, 24, 23, 0], zone: berlin)
    let nudge = PlannedNudge(
      anchors: [[due, eight]], snoozeAt: nil, snoozeUntil: nil, repeatMs: 60 * minute,
      quiet: [22, 0, 7, 0], since: nil, title: "", body: "")
    XCTAssertEqual(
      AlarmPlan.nudgeFiresAfter(nudge, zone: berlin, after: eight),
      AlarmPlan.instantOf([2026, 10, 25, 7, 0], zone: berlin))
  }

  func testTheQueueCountsUpAndAcknowledgesUpToASeq() {
    var queue = ActionQueue()
    let first = queue.add(
      kind: QueuedAction.done, routine: "a", occurrence: "o", title: "", body: "", now: 1)
    let second = queue.add(
      kind: QueuedAction.nudgeSnooze, routine: "", occurrence: "", title: "", body: "",
      until: 99, now: 2)
    XCTAssertEqual([first.seq, second.seq], [1, 2])

    queue.ack(1)
    XCTAssertEqual(queue.pending.map { $0.seq }, [2])
    // Past every seq handed out, even once all are acknowledged.
    queue.ack(2)
    let third = queue.add(
      kind: QueuedAction.snooze, routine: "a", occurrence: "o", title: "", body: "", until: 5,
      now: 3)
    XCTAssertEqual(third.seq, 3)
  }

  func testTheQueueKeepsWhatItCannotRead() {
    var queue = ActionQueue(stored: [["seq": 1, "kind": "later"]], seq: 1)
    XCTAssertTrue(queue.pending.isEmpty)
    queue.add(kind: QueuedAction.done, routine: "a", occurrence: "o", title: "", body: "", now: 4)
    queue.ack(5)
    XCTAssertEqual(queue.stored.count, 1, "an entry of a later version stays queued")
  }

  func testPendingAnswersAsAndroidDoes() {
    var queue = ActionQueue()
    let tap = queue.add(
      kind: QueuedAction.snooze, routine: "r", occurrence: "o", title: "t", body: "b",
      until: 600, now: 7)
    let map = tap.channelMap
    XCTAssertEqual(map["kind"] as? String, "snooze")
    XCTAssertEqual(map["routine"] as? String, "r")
    XCTAssertEqual(map["occurrence"] as? String, "o")
    XCTAssertEqual((map["seq"] as? NSNumber)?.int64Value, 1)
    XCTAssertEqual((map["at"] as? NSNumber)?.int64Value, 7)
    XCTAssertEqual((map["until"] as? NSNumber)?.int64Value, 600)
    XCTAssertEqual(map["line"] as? String, "")
  }

  func testNotificationActionsMapToTheirAnswers() {
    XCTAssertEqual(
      NotificationAnswer.of(category: NotificationAnswer.alarmCategory, action: "obf.done"), .done)
    XCTAssertEqual(
      NotificationAnswer.of(category: NotificationAnswer.alarmCategory, action: "obf.snooze.30"),
      .snooze(minutes: 30))
    XCTAssertEqual(
      NotificationAnswer.of(category: NotificationAnswer.alarmCategory, action: "obf.snooze.7"),
      .open)
    XCTAssertEqual(
      NotificationAnswer.of(
        category: NotificationAnswer.nudgeCategory, action: "obf.nudgeSnooze.60"),
      .nudgeSnooze(minutes: 60))
    XCTAssertEqual(
      NotificationAnswer.of(
        category: NotificationAnswer.nudgeCategory, action: NotificationAnswer.defaultAction),
      .openNudge)
    XCTAssertEqual(
      NotificationAnswer.of(
        category: NotificationAnswer.changesCategory, action: NotificationAnswer.defaultAction),
      .openChanges)
  }

  func testATapOnAnOccurrencesNotificationOpensItsNote() {
    XCTAssertEqual(
      NotificationAnswer.of(
        category: NotificationAnswer.alarmCategory, action: NotificationAnswer.defaultAction),
      .openOccurrence)
    XCTAssertEqual(
      NotificationAnswer.of(
        category: NotificationAnswer.pastAlarmCategory, action: NotificationAnswer.defaultAction),
      .openOccurrence)
  }

  func testNotTodaysActionsMapToTheirAnswers() {
    XCTAssertEqual(
      NotificationAnswer.of(category: NotificationAnswer.alarmCategory, action: "obf.skip"), .skip)
    XCTAssertEqual(
      NotificationAnswer.of(category: NotificationAnswer.alarmCategory, action: "obf.tomorrow"),
      .tomorrow)
    XCTAssertEqual(
      NotificationAnswer.of(category: NotificationAnswer.pastAlarmCategory, action: "obf.skip"),
      .skip)
    XCTAssertEqual(
      NotificationAnswer.of(
        category: NotificationAnswer.pastAlarmCategory, action: "obf.snooze.10"),
      .snooze(minutes: 10))
    // An earlier day's Occurrence has no Tomorrow.
    XCTAssertEqual(
      NotificationAnswer.of(
        category: NotificationAnswer.pastAlarmCategory, action: "obf.tomorrow"),
      .open)
    XCTAssertTrue(AlarmPlan.offersTomorrow("2026-10-24T08:00", today: "2026-10-24"))
    XCTAssertFalse(AlarmPlan.offersTomorrow("2026-10-23T22:00", today: "2026-10-24"))
  }

  func testSkipSilencesAnOccurrenceAndTomorrowRingsItTheDayAfter() {
    let pills = alarm("pills", renag: 10, end: [2026, 10, 24, 14, 0])
    let water = alarm("water")
    var queue = ActionQueue()
    queue.add(
      kind: QueuedAction.skip, routine: "water", occurrence: "2026-10-24T08:00", title: "water",
      body: "", now: eight)
    queue.add(
      kind: QueuedAction.tomorrow, routine: "pills", occurrence: "2026-10-24T08:00",
      title: "pills", body: "08:00", now: eight)

    let plan = AlarmPlan.effective([pills, water], queued: queue.pending)
    XCTAssertEqual(plan.map { $0.id }, ["occurrence:pills@2026-10-25T08:00"])
    XCTAssertEqual(plan.first?.local, [2026, 10, 25, 8, 0])
    XCTAssertEqual(plan.first?.end, [2026, 10, 25, 14, 0])
    XCTAssertEqual(plan.first?.renagMs, 10 * minute)
    // Missed when the next one starts, as before, where that comes later.
    let weekly = alarm("pills", renag: 10, end: [2026, 10, 31, 8, 0])
    XCTAssertEqual(AlarmPlan.moved(weekly)?.end, [2026, 10, 31, 8, 0])
    XCTAssertEqual(AlarmPlan.movedKey("2026-12-31"), "2027-01-01")
    XCTAssertEqual(AlarmPlan.movedKey("2028-02-28T07:00"), "2028-02-29T07:00")
    XCTAssertNil(AlarmPlan.movedKey("someday"))
  }

  func testTheArmedPlanReadsAsTheChannelSendsIt() {
    let alarms = PlanCodec.alarms([
      [
        "id": "occurrence:p@2026-10-24T08:00", "routine": "p", "occurrence": "2026-10-24T08:00",
        "local": [2026, 10, 24, 8, 0], "renag": 15, "escalate": 3, "end": [2026, 10, 24, 20, 0],
        "title": "pills", "body": "08:00",
      ],
      ["id": "broken"],
    ] as [[String: Any]])
    XCTAssertEqual(alarms.count, 1)
    XCTAssertEqual(alarms.first?.renagMs, 15 * minute)
    XCTAssertEqual(alarms.first?.escalate, 3)
    XCTAssertEqual(alarms.first?.end, [2026, 10, 24, 20, 0])
  }

  func testTheAlarmKitUsageTextIsThere() {
    let text = Bundle.main.object(forInfoDictionaryKey: "NSAlarmKitUsageDescription") as? String
    XCTAssertFalse((text ?? "").isEmpty)
  }
}
