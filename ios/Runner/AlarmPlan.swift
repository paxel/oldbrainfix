import Foundation

/// One alarm the app handed over, or a snooze tapped on a notification,
/// as android/…/AlarmPlan.kt keeps it. An Occurrence alarm keeps its
/// wall-clock moment in `local`, a snooze its instant in `at`. While
/// ignored it Re-nags every `renagMs` until `end`, when its Occurrence
/// counts as Missed; with `escalate` set, it rings as an alarm once the
/// first ring and that many Re-nags went ignored.
struct PlannedAlarm: Equatable {
  var id: String
  var routine: String
  var occurrence: String
  /// [year, month, day, hour, minute] on the wall clock, or nil.
  var local: [Int]?
  /// Epoch millis, or nil.
  var at: Int64?
  var title: String
  var body: String
  /// The Re-nag interval in millis; 0 rings once.
  var renagMs: Int64 = 0
  /// [year, month, day, hour, minute] on the wall clock when it stops, or nil.
  var end: [Int]? = nil
  /// After how many ignored Re-nags it escalates; 0 never does.
  var escalate: Int = 0

  /// The Occurrence it rings for, as notifications and the queue name it.
  var key: String { "\(routine)@\(occurrence)" }
}

/// The one Nudge over all Unsorted Notes, or the Changes Nudge, as the
/// app handed it over (android/…/NudgePlan.kt).
struct PlannedNudge: Equatable {
  /// (own Nudge time, capture) of each Unsorted Note, epoch millis.
  var anchors: [[Int64]]
  var snoozeAt: Int64?
  var snoozeUntil: Int64?
  var repeatMs: Int64
  /// [startHour, startMinute, endHour, endMinute], or nil while off.
  var quiet: [Int]?
  var since: Int64?
  var title: String
  var body: String
  /// The Changes Nudge's text for the lock screen; empty for the Nudge.
  var publicText: String = ""
}

/// What one ring of the window is.
enum RingKind: Equatable {
  /// An Occurrence, as a time-sensitive notification.
  case alarm
  /// An Occurrence past its Escalation: an AlarmKit alarm where allowed.
  case escalated
  case nudge
  case changes
}

/// One ring handed to iOS: when, what, and the Occurrence it rings for.
struct Ring: Equatable {
  var at: Int64
  var kind: RingKind
  var alarm: PlannedAlarm?
}

/// The scheduling rules, free of iOS, so XCTest covers them: the same
/// rules as android/…/AlarmPlan.kt and NudgePlan.kt, and the window of
/// rings iOS keeps pending, since it fires nothing the app did not hand
/// it beforehand.
enum AlarmPlan {
  /// iOS keeps at most this many notification requests pending.
  static let windowSize = 64

  /// The minutes the snooze on an AlarmKit alarm waits, the first of
  /// the notification's three choices: its alert has room for one.
  static let alarmSnoozeMinutes = 10

  /// The snooze choices of a notification, in minutes, as on Android.
  static let snoozeMinutes = [10, 30, 60]

  /// The instant `local` shows on `zone`'s clock. A time the clock shows
  /// twice fires once, at the first; a time it skips fires at the next
  /// valid instant. The same rule as the core's `instantOf`.
  static func instantOf(_ local: [Int], zone: TimeZone) -> Int64 {
    let target = utcMillis(local)
    let day: Int64 = 24 * 60 * 60 * 1000
    let before = offset(zone, target - day)
    let after = offset(zone, target + day)
    let hits = Set([target - before, target - after]).filter { $0 + offset(zone, $0) == target }
    if let first = hits.min() { return first }
    // Skipped: find where the clock jumps.
    var lo = target - after
    var hi = target - before
    if lo >= hi { return hi }
    while hi - lo > 1 {
      let mid = lo + (hi - lo) / 2
      if mid + offset(zone, mid) < target { lo = mid } else { hi = mid }
    }
    return hi
  }

  /// When `alarm` fires first in `zone`.
  static func instantOf(_ alarm: PlannedAlarm, zone: TimeZone) -> Int64 {
    if let local = alarm.local { return instantOf(local, zone: zone) }
    return alarm.at ?? Int64.max
  }

  /// When `alarm` stops in `zone`: its Occurrence counts as Missed.
  static func endOf(_ alarm: PlannedAlarm, zone: TimeZone) -> Int64 {
    guard let end = alarm.end else { return Int64.max }
    return instantOf(end, zone: zone)
  }

  /// When `alarm` fires first after `after`: at its instant or at a
  /// Re-nag, never at or past its end; nil once it has stopped.
  static func firesAfter(_ alarm: PlannedAlarm, zone: TimeZone, after: Int64) -> Int64? {
    let first = instantOf(alarm, zone: zone)
    let end = endOf(alarm, zone: zone)
    let next: Int64
    if first > after {
      next = first
    } else if alarm.renagMs <= 0 {
      return nil
    } else {
      next = first + ((after - first) / alarm.renagMs + 1) * alarm.renagMs
    }
    return next < end ? next : nil
  }

  /// Whether `alarm`, firing at `now`, has escalated: its first ring and
  /// `escalate` Re-nags after it went ignored, and it has not stopped.
  static func escalates(_ alarm: PlannedAlarm, zone: TimeZone, now: Int64) -> Bool {
    if alarm.escalate <= 0 || alarm.renagMs <= 0 { return false }
    let first = instantOf(alarm, zone: zone)
    if now < first || now >= endOf(alarm, zone: zone) { return false }
    return (now - first) / alarm.renagMs >= Int64(alarm.escalate) + 1
  }

  /// The plan with the taps the app has not applied yet: what was tapped
  /// Done, Skip or Tomorrow is gone, snoozed Occurrences ring again at
  /// their snooze and Re-nag from there, as they did, and an Occurrence
  /// Tomorrow moved rings the day after at the same time (`moved`).
  static func effective(_ plan: [PlannedAlarm], queued: [QueuedAction]) -> [PlannedAlarm] {
    let answered: Set<String> = [QueuedAction.done, QueuedAction.skip, QueuedAction.tomorrow]
    let done = Set(queued.filter { answered.contains($0.kind) }.map { $0.key })
    var snoozes: [String: QueuedAction] = [:]
    var order: [String] = []
    for tap in queued where tap.kind == QueuedAction.snooze {
      if snoozes[tap.key] == nil { order.append(tap.key) }
      snoozes[tap.key] = tap
    }
    let kept = plan.filter { !done.contains($0.key) && snoozes[$0.key] == nil }
    let added: [PlannedAlarm] = order.compactMap { key in
      guard let tap = snoozes[key], !done.contains(key) else { return nil }
      let was = plan.first { $0.key == key }
      return PlannedAlarm(
        id: "snooze:\(key)",
        routine: tap.routine,
        occurrence: tap.occurrence,
        local: nil,
        at: tap.until,
        title: tap.title,
        body: tap.body,
        renagMs: was?.renagMs ?? 0,
        end: was?.end,
        escalate: was?.escalate ?? 0
      )
    }
    var planned = Set((kept + added).map { $0.key })
    var tomorrow: [PlannedAlarm] = []
    for tap in queued where tap.kind == QueuedAction.tomorrow {
      guard let was = plan.first(where: { $0.key == tap.key }),
        let alarm = moved(was), !planned.contains(alarm.key)
      else { continue }
      planned.insert(alarm.key)
      tomorrow.append(alarm)
    }
    return kept + added + tomorrow
  }

  /// The alarm of the one-time Occurrence Tomorrow adds for `alarm`'s
  /// (#269): the day after at the same time, Re-nagging and escalating
  /// as it did, and Missed when it was, or a day later where that came
  /// before the moved one; nil when its Occurrence
  /// or wall-clock moment does not read. A snooze's moment is the
  /// Occurrence's own time, an all-day one's the time it rang at.
  static func moved(_ alarm: PlannedAlarm) -> PlannedAlarm? {
    guard let occurrence = movedKey(alarm.occurrence) else { return nil }
    let parts = alarm.occurrence.split(separator: "T", omittingEmptySubsequences: false)
    let local: [Int]
    if parts.count == 2 {
      let time = parts[1].split(separator: ":").compactMap { Int($0) }
      guard time.count == 2, let day = dayOf(occurrence) else { return nil }
      local = day + time
    } else {
      guard let rang = alarm.local, rang.count == 5 else { return nil }
      local = nextDay(rang)
    }
    var moved = alarm
    moved.id = "occurrence:\(alarm.routine)@\(occurrence)"
    moved.occurrence = occurrence
    moved.local = local
    moved.at = nil
    // Missed when the Occurrence after it starts, as before; a day later
    // only where that came before it, as a Reminder's midnight.
    moved.end = alarm.end.flatMap { end in
      end.count == 5 ? (end.lexicographicallyPrecedes(local) || end == local ? nextDay(end) : end) : nil
    }
    return moved
  }

  /// The key of the Occurrence a day after `occurrence`, at the same
  /// time; nil when it does not read.
  static func movedKey(_ occurrence: String) -> String? {
    let parts = occurrence.split(separator: "T", omittingEmptySubsequences: false)
    guard parts.count <= 2, let day = dayOf(String(parts[0])) else { return nil }
    let next = nextDay(day + [0, 0])
    let text = String(format: "%04d-%02d-%02d", next[0], next[1], next[2])
    return parts.count == 2 ? "\(text)T\(parts[1])" : text
  }

  /// Whether Not today offers Tomorrow for `occurrence` on `today`
  /// ("2026-10-24"): only for an Occurrence of today or later.
  static func offersTomorrow(_ occurrence: String, today: String) -> Bool {
    String(occurrence.prefix(10)) >= today
  }

  /// [year, month, day] of `key`'s day, or nil.
  private static func dayOf(_ key: String) -> [Int]? {
    let parts = key.prefix(while: { $0 != "T" }).split(separator: "-")
    guard parts.count == 3, parts[0].count == 4 else { return nil }
    let numbers = parts.compactMap { Int($0) }
    return numbers.count == 3 ? numbers : nil
  }

  /// `local` a calendar day later, at the same wall-clock time.
  private static func nextDay(_ local: [Int]) -> [Int] {
    var calendar = Calendar(identifier: .gregorian)
    calendar.timeZone = TimeZone(identifier: "UTC")!
    let date = calendar.date(
      from: DateComponents(year: local[0], month: local[1], day: local[2], hour: 12))!
    let next = calendar.date(byAdding: .day, value: 1, to: date)!
    let parts = calendar.dateComponents([.year, .month, .day], from: next)
    return [parts.year!, parts.month!, parts.day!, local[3], local[4]]
  }


  /// The Nudge with the latest snooze tapped on its notification and not
  /// applied by the app yet.
  static func effective(_ nudge: PlannedNudge, queued: [QueuedAction]) -> PlannedNudge {
    guard let tap = queued.filter({ $0.kind == QueuedAction.nudgeSnooze }).max(by: { $0.seq < $1.seq })
    else { return nudge }
    var snoozed = nudge
    snoozed.snoozeAt = tap.at
    snoozed.snoozeUntil = tap.until
    return snoozed
  }

  /// The rings to hand iOS at `now`: every ring of `alarms`, the `nudge`
  /// and the Changes Nudge `changes` after now, soonest first, at most
  /// `limit` of them. An Occurrence's ring past its Escalation is
  /// `.escalated`. Rings past the window are handed over by the next plan.
  static func window(
    alarms: [PlannedAlarm],
    nudge: PlannedNudge?,
    changes: PlannedNudge?,
    zone: TimeZone,
    now: Int64,
    limit: Int = windowSize
  ) -> [Ring] {
    var rings: [Ring] = []
    for alarm in alarms {
      var after = now
      var count = 0
      while count < limit, let at = firesAfter(alarm, zone: zone, after: after) {
        let kind: RingKind = escalates(alarm, zone: zone, now: at) ? .escalated : .alarm
        rings.append(Ring(at: at, kind: kind, alarm: alarm))
        after = at
        count += 1
      }
    }
    for (planned, kind) in [(nudge, RingKind.nudge), (changes, RingKind.changes)] {
      guard let planned = planned else { continue }
      var after = now
      for _ in 0..<limit {
        let at = nudgeFiresAfter(planned, zone: zone, after: after)
        rings.append(Ring(at: at, kind: kind, alarm: nil))
        after = at
      }
    }
    // Stable, so rings at the same moment keep the plan's order.
    let sorted = rings.enumerated().sorted { a, b in
      a.element.at != b.element.at ? a.element.at < b.element.at : a.offset < b.offset
    }
    return sorted.prefix(limit).map { $0.element }
  }

  /// The notification request of `ring`: one per ring, so each Re-nag is
  /// pending on its own until it fires.
  static func requestId(_ ring: Ring) -> String {
    switch ring.kind {
    case .alarm, .escalated: return "obf.alarm.\(ring.alarm?.key ?? "").\(ring.at)"
    case .nudge: return "obf.nudge.\(ring.at)"
    case .changes: return "obf.changes.\(ring.at)"
    }
  }

  // MARK: The Nudge's rule, the same as NudgePlan.kt.

  /// When a Nudge due at `at` fires: then, or at the end of the Quiet
  /// Hours `quiet` it lies in.
  static func deferred(_ quiet: [Int], zone: TimeZone, at: Int64) -> Int64 {
    let start = quiet[0] * 60 + quiet[1]
    let end = quiet[2] * 60 + quiet[3]
    var utc = Calendar(identifier: .gregorian)
    utc.timeZone = TimeZone(identifier: "UTC") ?? zone
    var wall = utc.dateComponents(
      [.year, .month, .day, .hour, .minute],
      from: Date(timeIntervalSince1970: Double(at + offset(zone, at)) / 1000)
    )
    let minute = (wall.hour ?? 0) * 60 + (wall.minute ?? 0)
    let inside = start <= end ? (minute >= start && minute < end) : (minute >= start || minute < end)
    if !inside { return at }
    // In a window over midnight, before midnight it ends tomorrow.
    if start > end && minute >= start {
      let day = utc.date(from: wall).flatMap { utc.date(byAdding: .day, value: 1, to: $0) }
      if let day = day { wall = utc.dateComponents([.year, .month, .day], from: day) }
    }
    let ends = instantOf([wall.year ?? 1970, wall.month ?? 1, wall.day ?? 1, quiet[2], quiet[3]], zone: zone)
    return max(ends, at)
  }

  /// When it fires next after it last fired at `fired` (nil: never).
  static func nextFiring(_ nudge: PlannedNudge, zone: TimeZone, fired: Int64?) -> Int64? {
    func ownAfter(_ from: Int64?) -> Int64? {
      nudge.anchors
        .filter { anchor in
          anchor.count == 2 && (from == nil || anchor[1] > (from ?? 0))
            && (fired == nil || anchor[0] > (fired ?? 0))
        }
        .map { $0[0] }
        .min()
    }
    var raw = [fired.map { $0 + nudge.repeatMs }, ownAfter(fired)].compactMap { $0 }.min()
    if let tapped = nudge.snoozeAt, let until = nudge.snoozeUntil {
      if fired == nil || (fired ?? 0) < tapped {
        // From the tap to its end only Notes captured after it fire.
        if let r = raw, r >= tapped { raw = [until, ownAfter(tapped)].compactMap { $0 }.min() }
      } else if let f = fired, f < until {
        raw = [raw, until].compactMap { $0 }.min()
      }
    }
    guard let fires = raw else { return nil }
    guard let quiet = nudge.quiet, quiet.count == 4 else { return fires }
    return deferred(quiet, zone: zone, at: fires)
  }

  /// When it fires first after `after`.
  static func nudgeFiresAfter(_ nudge: PlannedNudge, zone: TimeZone, after: Int64) -> Int64 {
    var last = nudge.since.flatMap { after >= $0 ? $0 : nil }
    while true {
      let next = nextFiring(nudge, zone: zone, fired: last) ?? (after + nudge.repeatMs)
      if next > after { return next }
      last = next
    }
  }

  private static func offset(_ zone: TimeZone, _ millis: Int64) -> Int64 {
    Int64(zone.secondsFromGMT(for: Date(timeIntervalSince1970: Double(millis) / 1000))) * 1000
  }

  private static func utcMillis(_ local: [Int]) -> Int64 {
    var calendar = Calendar(identifier: .gregorian)
    calendar.timeZone = TimeZone(identifier: "UTC") ?? TimeZone.current
    let parts = local + Array(repeating: 0, count: max(0, 5 - local.count))
    let date = calendar.date(
      from: DateComponents(year: parts[0], month: parts[1], day: parts[2], hour: parts[3], minute: parts[4])
    )
    return Int64(((date?.timeIntervalSince1970 ?? 0) * 1000).rounded())
  }
}
