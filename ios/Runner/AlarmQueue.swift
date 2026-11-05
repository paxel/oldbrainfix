import Foundation

/// Done or a snooze tapped on a notification or an AlarmKit alarm, kept
/// until the app applied it to the log (android/…/EventQueue.kt).
struct QueuedAction: Equatable {
  static let done = "done"
  static let snooze = "snooze"
  static let nudgeSnooze = "nudgeSnooze"
  /// A Line of a Checklist Routine's Occurrence checked in the Agenda
  /// widget (#242).
  static let line = "line"
  /// A Frog checked off in the Agenda widget; its id travels as the
  /// routine.
  static let frog = "frog"
  /// A Shopping List's Line checked in the Agenda widget.
  static let shopping = "shopping"
  /// A Line of a Checklist without a Routine checked in the Agenda widget.
  static let checklist = "checklist"

  /// The kinds the iPhone queues: a notification's or an alarm's, and
  /// the Agenda widget's boxes (#242), as android/…/EventQueue.kt.
  static let kinds: Set<String> = [done, snooze, nudgeSnooze, line, frog, shopping, checklist]

  /// Counts up; the app acknowledges everything up to one it applied.
  var seq: Int64
  var kind: String
  var routine: String
  var occurrence: String
  /// When it was tapped, epoch millis.
  var at: Int64
  /// When a snooze rings again, epoch millis; 0 for Done.
  var until: Int64
  var title: String
  var body: String
  /// The Line a widget's box checked; empty for every other kind.
  var line: String = ""

  var key: String { "\(routine)@\(occurrence)" }

  /// What the channel's "pending" answers for it, as Android's does.
  var channelMap: [String: Any] {
    [
      "seq": NSNumber(value: seq),
      "kind": kind,
      "routine": routine,
      "occurrence": occurrence,
      "at": NSNumber(value: at),
      "until": NSNumber(value: until),
      "line": line,
    ]
  }

  /// As it is stored, in the shape of android/…/EventQueue.kt's
  /// entries: `line` only where a Line was checked.
  var json: [String: Any] {
    var map = channelMap
    map["title"] = title
    map["body"] = body
    if line.isEmpty { map.removeValue(forKey: "line") }
    return map
  }

  /// The action stored as `json`; nil for one that does not read or of a
  /// kind this version does not apply.
  init?(json: [String: Any]) {
    guard let seq = PlanCodec.int64(json["seq"]),
      let kind = json["kind"] as? String, QueuedAction.kinds.contains(kind),
      let routine = json["routine"] as? String,
      let occurrence = json["occurrence"] as? String,
      let at = PlanCodec.int64(json["at"])
    else { return nil }
    self.init(
      seq: seq, kind: kind, routine: routine, occurrence: occurrence, at: at,
      until: PlanCodec.int64(json["until"]) ?? 0,
      title: json["title"] as? String ?? "",
      body: json["body"] as? String ?? "",
      line: json["line"] as? String ?? "")
  }

  init(
    seq: Int64, kind: String, routine: String, occurrence: String, at: Int64, until: Int64,
    title: String, body: String, line: String = ""
  ) {
    self.seq = seq
    self.kind = kind
    self.routine = routine
    self.occurrence = occurrence
    self.at = at
    self.until = until
    self.title = title
    self.body = body
    self.line = line
  }
}

/// The taps not applied by the app yet, oldest first. The app reads,
/// applies, then acknowledges, so a crash in between loses nothing; an
/// entry that does not read is kept as it was, never dropped unapplied.
struct ActionQueue {
  /// Every entry as stored, those that do not read too.
  private(set) var stored: [[String: Any]]
  /// The highest seq handed out.
  private(set) var seq: Int64

  init(stored: [[String: Any]] = [], seq: Int64 = 0) {
    self.stored = stored
    self.seq = seq
  }

  /// The actions that read, oldest first.
  var pending: [QueuedAction] { stored.compactMap { QueuedAction(json: $0) } }

  /// Queues a tap at `now`, past every seq queued so far, so an
  /// acknowledgement never takes a new tap along with older ones.
  @discardableResult
  mutating func add(
    kind: String, routine: String, occurrence: String, title: String, body: String,
    until: Int64 = 0, line: String = "", now: Int64
  ) -> QueuedAction {
    let last = stored.compactMap { PlanCodec.int64($0["seq"]) }.max() ?? 0
    seq = max(seq, last) + 1
    let action = QueuedAction(
      seq: seq, kind: kind, routine: routine, occurrence: occurrence, at: now, until: until,
      title: title, body: body, line: line)
    stored.append(action.json)
    return action
  }

  /// Drops the actions up to `seq`: the app has applied them.
  mutating func ack(_ seq: Int64) {
    stored.removeAll { entry in QueuedAction(json: entry).map { $0.seq <= seq } ?? false }
  }
}

/// A tap on a notification of the app, by its category and action.
enum NotificationAnswer: Equatable {
  case done
  case snooze(minutes: Int)
  case nudgeSnooze(minutes: Int)
  case openNudge
  case openChanges
  /// Opens the app and changes nothing.
  case open

  static let alarmCategory = "obf.alarm"
  static let nudgeCategory = "obf.nudge"
  static let changesCategory = "obf.changes"
  static let doneAction = "obf.done"
  static let snoozePrefix = "obf.snooze."
  static let nudgeSnoozePrefix = "obf.nudgeSnooze."
  /// UNNotificationDefaultActionIdentifier's value: the notification
  /// itself was tapped.
  static let defaultAction = "com.apple.UNNotificationDefaultActionIdentifier"

  /// What the action `action` on a notification of `category` means.
  static func of(category: String, action: String) -> NotificationAnswer {
    switch category {
    case alarmCategory:
      if action == doneAction { return .done }
      if let minutes = snoozeMinutes(in: action, after: snoozePrefix) { return .snooze(minutes: minutes) }
    case nudgeCategory:
      if action == defaultAction { return .openNudge }
      if let minutes = snoozeMinutes(in: action, after: nudgeSnoozePrefix) {
        return .nudgeSnooze(minutes: minutes)
      }
    case changesCategory:
      if action == defaultAction { return .openChanges }
    default:
      break
    }
    return .open
  }

  private static func snoozeMinutes(in action: String, after prefix: String) -> Int? {
    guard action.hasPrefix(prefix), let minutes = Int(action.dropFirst(prefix.count)),
      AlarmPlan.snoozeMinutes.contains(minutes)
    else { return nil }
    return minutes
  }
}

/// The alarm channel's arguments, read the way android/…/Scheduler.kt
/// reads them; what does not read is left out, never a crash.
enum PlanCodec {
  static func int64(_ value: Any?) -> Int64? {
    if let number = value as? NSNumber { return number.int64Value }
    return nil
  }

  static func ints(_ value: Any?) -> [Int]? {
    guard let list = value as? [Any] else { return nil }
    let ints = list.compactMap { ($0 as? NSNumber)?.intValue }
    return ints.count == list.count ? ints : nil
  }

  /// The alarms of "arm"'s `alarms`, in order.
  static func alarms(_ value: Any?) -> [PlannedAlarm] {
    guard let list = value as? [Any] else { return [] }
    return list.compactMap { alarm($0 as? [String: Any]) }
  }

  static func alarm(_ map: [String: Any]?) -> PlannedAlarm? {
    guard let map = map, let id = map["id"] as? String, let routine = map["routine"] as? String,
      let occurrence = map["occurrence"] as? String
    else { return nil }
    let local = ints(map["local"])
    let at = int64(map["at"])
    if local == nil && at == nil { return nil }
    return PlannedAlarm(
      id: id,
      routine: routine,
      occurrence: occurrence,
      local: local,
      at: at,
      title: map["title"] as? String ?? "",
      body: map["body"] as? String ?? "",
      renagMs: (int64(map["renag"]) ?? 0) * 60_000,
      end: ints(map["end"]),
      escalate: Int(int64(map["escalate"]) ?? 0)
    )
  }

  /// The Nudge or the Changes Nudge of "arm", or nil.
  static func nudge(_ value: Any?) -> PlannedNudge? {
    guard let map = value as? [String: Any], let anchors = map["anchors"] as? [Any] else {
      return nil
    }
    return PlannedNudge(
      anchors: anchors.compactMap { anchor in
        guard let pair = anchor as? [Any], pair.count == 2,
          let due = int64(pair[0]), let captured = int64(pair[1])
        else { return nil }
        return [due, captured]
      },
      snoozeAt: int64(map["snoozeAt"]),
      snoozeUntil: int64(map["snoozeUntil"]),
      repeatMs: max(1, int64(map["repeat"]) ?? 60) * 60_000,
      quiet: ints(map["quiet"]).flatMap { $0.count == 4 ? $0 : nil },
      since: int64(map["since"]),
      title: map["title"] as? String ?? "",
      body: map["body"] as? String ?? "",
      publicText: map["public"] as? String ?? ""
    )
  }

  /// The Occurrences of "arm"'s `done`, as `routine@occurrence`.
  static func done(_ value: Any?) -> [String] {
    guard let list = value as? [Any] else { return [] }
    return list.compactMap { entry in
      guard let map = entry as? [String: Any], let routine = map["routine"] as? String,
        let occurrence = map["occurrence"] as? String
      else { return nil }
      return "\(routine)@\(occurrence)"
    }
  }
}
