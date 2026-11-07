import Foundation

/// One occurrence of a calendar event as EventKit lists it, as plain
/// values, so the rules below are tested without a calendar (#241).
struct EventFacts: Equatable {
  /// The event's `calendarItemIdentifier`, the same for each occurrence.
  var id: String
  var calendarId: String
  var title: String
  /// As EventKit has them: an all-day event from local midnight, its end
  /// on its last day.
  var start: Date
  var end: Date
  var allDay: Bool
  var location: String = ""
  var notes: String = ""
  /// Whether the user declined it as an attendee.
  var declined: Bool = false
  /// The event's URL, where the calendar mirror marks its own.
  var url: String?
}

/// The calendar channel's rules on the iPhone (#241), as Appointments.kt
/// and CaptureExtras.kt apply them on Android: what the Agenda shows and
/// what runs now for the Capture Context, in the channel's answers.
enum CalendarRules {
  private static let day: TimeInterval = 24 * 60 * 60

  private static var utc: Calendar {
    var calendar = Calendar(identifier: .gregorian)
    calendar.timeZone = TimeZone(identifier: "UTC") ?? .current
    return calendar
  }

  /// Midnight UTC of the day [date] falls on in [calendar]'s zone.
  static func utcMidnight(of date: Date, in calendar: Calendar) -> Date {
    let parts = calendar.dateComponents([.year, .month, .day], from: date)
    return utc.date(from: parts) ?? date
  }

  /// When [event] runs as the channel says it, in epoch milliseconds: a
  /// timed one as it is, an all-day one from midnight UTC of its first
  /// day to midnight UTC after its last, as Android's calendar keeps
  /// them and the Agenda reads them.
  static func span(_ event: EventFacts, in calendar: Calendar) -> (begin: Int64, end: Int64) {
    guard event.allDay else { return (millis(event.start), millis(max(event.start, event.end))) }
    let first = utcMidnight(of: event.start, in: calendar)
    // EventKit ends an all-day event on its last day, at its end or at
    // the next midnight: a moment before the end is always the last day.
    let lastMoment = max(event.start, event.end.addingTimeInterval(-0.001))
    let last = utcMidnight(of: lastMoment, in: calendar)
    return (millis(first), millis(last.addingTimeInterval(day)))
  }

  /// [event] as the channel's `appointments` hands it over; its id is
  /// the occurrence's, as each repeat of an event is a row of its own.
  static func row(_ event: EventFacts, in calendar: Calendar) -> [String: Any] {
    let (begin, end) = span(event, in: calendar)
    return [
      "id": "\(event.id)@\(begin)",
      "calendar": event.calendarId,
      "title": event.title,
      "begin": begin,
      "end": end,
      "allDay": event.allDay,
      "location": event.location,
      "description": event.notes,
    ]
  }

  /// The [events] the Agenda shows: those of [calendarIds] the user did
  /// not decline, without the mirror's own, which the Agenda shows as
  /// Occurrences already; each occurrence once.
  static func shown(_ events: [EventFacts], calendarIds: Set<String>, in calendar: Calendar)
    -> [[String: Any]]
  {
    var seen = Set<String>()
    return events.filter {
      calendarIds.contains($0.calendarId) && !$0.declined && MirrorMark.uid($0.url) == nil
    }
    .map { row($0, in: calendar) }
    .filter { seen.insert($0["id"] as? String ?? "").inserted }
  }

  /// The [events] running at [now] that the user did not decline and
  /// that have a title, as `calendar` answers them: an all-day one runs
  /// while the local date lies inside it.
  static func running(_ events: [EventFacts], now: Date, in calendar: Calendar)
    -> [[String: Any]]
  {
    let offset = Int64(calendar.timeZone.secondsFromGMT(for: now)) * 1000
    var seen = Set<String>()
    return events.filter { event in
      let (begin, end) = span(event, in: calendar)
      let at = millis(now) + (event.allDay ? offset : 0)
      return !event.declined
        && !event.title.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
        && begin <= at && at < end
        && seen.insert("\(event.id)@\(begin)").inserted
    }
    .map { ["title": $0.title, "allDay": $0.allDay] }
  }

  /// A calendar's colour as ARGB, as Android's provider gives it.
  static func colour(red: Double, green: Double, blue: Double, alpha: Double) -> Int {
    func byte(_ value: Double) -> Int { Int((min(1, max(0, value)) * 255).rounded()) }
    return byte(alpha) << 24 | byte(red) << 16 | byte(green) << 8 | byte(blue)
  }

  static func millis(_ date: Date) -> Int64 { Int64((date.timeIntervalSince1970 * 1000).rounded()) }
}

/// The mark of an event the calendar mirror wrote, as CalendarMirror.kt
/// keeps it in Android's custom app URI: the event's URL. Only marked
/// events are ever changed or removed.
enum MirrorMark {
  static let prefix = "oldbrainfix:mirror/"

  static func url(_ uid: String) -> URL? { URL(string: prefix + uid) }

  /// The UID in [url]; nil for anything the mirror did not write.
  static func uid(_ url: String?) -> String? {
    guard let url, url.hasPrefix(prefix) else { return nil }
    let uid = String(url.dropFirst(prefix.count))
    return uid.isEmpty ? nil : uid
  }
}

/// An Occurrence key as the core names a start, `2026-10-25` all day or
/// `2026-10-25T08:00` at a time, as a date in the Device's zone.
enum StartKey {
  /// The key's date parts; nil for a key that is none.
  static func parse(_ key: String) -> (parts: DateComponents, allDay: Bool)? {
    let chars = Array(key)
    func number(_ from: Int, _ count: Int) -> Int? {
      guard from + count <= chars.count else { return nil }
      let text = String(chars[from..<from + count])
      guard text.allSatisfy(\.isASCII), text.allSatisfy(\.isNumber) else { return nil }
      return Int(text)
    }
    guard chars.count == 10 || chars.count == 16,
      chars.count > 7, chars[4] == "-", chars[7] == "-",
      let year = number(0, 4), let month = number(5, 2), let day = number(8, 2),
      (1...12).contains(month), (1...31).contains(day)
    else { return nil }
    var parts = DateComponents(year: year, month: month, day: day)
    if chars.count == 10 { return (parts, true) }
    guard chars[10] == "T", chars[13] == ":", let hour = number(11, 2), let minute = number(14, 2),
      hour < 24, minute < 60
    else { return nil }
    parts.hour = hour
    parts.minute = minute
    return (parts, false)
  }

  /// When an event at [key] starts and ends in [calendar]'s zone: a
  /// timed one lasts [minutes], an hour skipped by daylight saving moved
  /// on past it; an all-day one starts and ends on its day, as EventKit
  /// keeps a one-day all-day event.
  static func span(_ key: String, minutes: Int, in calendar: Calendar)
    -> (start: Date, end: Date, allDay: Bool)?
  {
    guard case let (parts, allDay)? = parse(key) else { return nil }
    var local = parts
    local.timeZone = calendar.timeZone
    guard let start = calendar.date(from: local) else { return nil }
    if allDay { return (start, start, true) }
    return (start, start.addingTimeInterval(TimeInterval(minutes * 60)), false)
  }
}

/// An RRULE value as the core writes it for a calendar event (ics.dart):
/// FREQ, INTERVAL, COUNT, UNTIL as wall clock, BYMONTH, BYMONTHDAY and
/// BYDAY with ordinals; the parts EventKit's recurrence rule takes.
/// WKST has no place there and is left out.
struct RuleParts: Equatable {
  enum Frequency: String { case daily = "DAILY", weekly = "WEEKLY", monthly = "MONTHLY", yearly = "YEARLY" }

  /// One BYDAY item: a weekday as EventKit counts them, 1 for Sunday to
  /// 7 for Saturday, with its ordinal, 0 for every such weekday.
  struct Day: Equatable {
    var weekday: Int
    var ordinal: Int = 0
  }

  var frequency: Frequency
  var interval = 1
  var count: Int?
  /// UNTIL's date and, when it has one, its time.
  var until: DateComponents?
  /// Whether UNTIL ends in `Z`, a time in UTC.
  var untilUtc = false
  var byDay: [Day] = []
  var byMonthDay: [Int] = []
  var byMonth: [Int] = []

  private static let codes = ["SU", "MO", "TU", "WE", "TH", "FR", "SA"]

  /// The rule in [text], with or without `RRULE:`; nil for one EventKit
  /// cannot hold.
  static func parse(_ text: String) -> RuleParts? {
    var body = text.trimmingCharacters(in: .whitespaces).uppercased()
    if body.hasPrefix("RRULE:") { body.removeFirst(6) }
    var parts: [String: String] = [:]
    for part in body.split(separator: ";") where !part.isEmpty {
      let pair = part.split(separator: "=", maxSplits: 1).map(String.init)
      guard pair.count == 2 else { return nil }
      parts[pair[0]] = pair[1]
    }
    guard let frequency = parts["FREQ"].flatMap(Frequency.init(rawValue:)) else { return nil }
    var rule = RuleParts(frequency: frequency)
    if let interval = parts["INTERVAL"] {
      guard let n = Int(interval), n >= 1 else { return nil }
      rule.interval = n
    }
    if let count = parts["COUNT"] {
      guard let n = Int(count), n >= 1 else { return nil }
      rule.count = n
    }
    if let until = parts["UNTIL"] {
      guard case let (date, utc)? = untilParts(until) else { return nil }
      rule.until = date
      rule.untilUtc = utc
    }
    if let days = parts["BYDAY"] {
      for item in days.split(separator: ",") {
        guard let day = weekday(String(item)) else { return nil }
        rule.byDay.append(day)
      }
    }
    if let days = parts["BYMONTHDAY"] {
      for item in days.split(separator: ",") {
        guard let n = Int(item), n != 0, (-31...31).contains(n) else { return nil }
        rule.byMonthDay.append(n)
      }
    }
    if let months = parts["BYMONTH"] {
      for item in months.split(separator: ",") {
        guard let n = Int(item), (1...12).contains(n) else { return nil }
        rule.byMonth.append(n)
      }
    }
    // EventKit takes weekdays only from WEEKLY up: every day on some
    // weekdays is the same as every week on them.
    if rule.frequency == .daily, !rule.byDay.isEmpty, rule.interval == 1,
      rule.byDay.allSatisfy({ $0.ordinal == 0 })
    {
      rule.frequency = .weekly
    }
    return rule
  }

  /// When the rule ends, inclusive, in [calendar]'s zone: UNTIL's time,
  /// or the end of its day when it has none; nil without UNTIL.
  func untilDate(in calendar: Calendar) -> Date? {
    guard var parts = until else { return nil }
    var zoned = calendar
    if untilUtc { zoned.timeZone = TimeZone(identifier: "UTC") ?? calendar.timeZone }
    parts.timeZone = zoned.timeZone
    if parts.hour == nil {
      parts.hour = 23
      parts.minute = 59
      parts.second = 59
    }
    return zoned.date(from: parts)
  }

  private static func untilParts(_ text: String) -> (DateComponents, Bool)? {
    var body = text
    let utc = body.hasSuffix("Z")
    if utc { body.removeLast() }
    let chars = Array(body)
    func number(_ from: Int, _ count: Int) -> Int? {
      guard from + count <= chars.count else { return nil }
      let digits = String(chars[from..<from + count])
      guard digits.allSatisfy(\.isASCII), digits.allSatisfy(\.isNumber) else { return nil }
      return Int(digits)
    }
    guard chars.count == 8 || chars.count == 15,
      let year = number(0, 4), let month = number(4, 2), let day = number(6, 2)
    else { return nil }
    var parts = DateComponents(year: year, month: month, day: day)
    if chars.count == 15 {
      guard chars[8] == "T", let hour = number(9, 2), let minute = number(11, 2),
        let second = number(13, 2)
      else { return nil }
      parts.hour = hour
      parts.minute = minute
      parts.second = second
    }
    return (parts, utc)
  }

  private static func weekday(_ item: String) -> Day? {
    guard item.count >= 2 else { return nil }
    let code = String(item.suffix(2))
    guard let index = codes.firstIndex(of: code) else { return nil }
    let ordinalText = item.dropLast(2)
    if ordinalText.isEmpty { return Day(weekday: index + 1) }
    guard let ordinal = Int(ordinalText.hasPrefix("+") ? String(ordinalText.dropFirst()) : String(ordinalText)),
      ordinal != 0, (-53...53).contains(ordinal)
    else { return nil }
    return Day(weekday: index + 1, ordinal: ordinal)
  }
}
