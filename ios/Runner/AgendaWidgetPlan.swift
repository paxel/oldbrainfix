import Foundation

/// One row of the Agenda widget (#242), in the words the app handed over
/// (lib/src/agenda_widget.dart), or one the widget makes itself: the
/// Unsorted count, a day's heading, an appointment of the calendar, a
/// message or the count of rows that do not fit. A group of the Agenda
/// is its heading's row with its entries' rows after it, indented
/// (#209). The rules are android/…/AgendaWidgetPlan.kt's; the app and the
/// Widgets extension both compile this file.
struct WidgetRow: Equatable {
  static let occurrence = "occurrence"
  static let session = "session"
  static let pick = "pick"
  static let line = "line"
  static let shopping = "shopping"
  static let checklist = "checklist"
  static let checklistLine = "checklistLine"
  static let frog = "frog"
  static let missed = "missed"
  static let heading = "heading"
  static let text = "text"

  /// The rows the widget makes itself.
  static let unsorted = "unsorted"
  static let appointment = "appointment"
  static let message = "message"
  /// "+3": how many rows did not fit (#242).
  static let more = "more"

  /// The kinds of row a snapshot of version 4 holds.
  static let handed: Set<String> = [
    occurrence, session, pick, line, shopping, checklist, checklistLine, frog, missed, heading,
    text,
  ]

  static let allDay = "allDay"
  static let lists = "lists"
  static let timed = "timed"
  static let frogs = "frogs"

  var kind: String
  var title: String
  /// Its time, or "All day"; empty for a row without one.
  var time = ""
  var done = false
  /// Whether this Device may check it here, where it has a box.
  var checks = false
  /// How far it is indented: 1 under its heading, 2 for a Line under
  /// another Line.
  var indent = 0
  /// The Routine and Occurrence an Occurrence, a Missed one or a Line is
  /// checked for.
  var routine = ""
  var occurrence = ""
  /// The Line of a Checklist or a Shopping List its box checks.
  var line = ""
  /// The Frog its box checks off.
  var frog = ""
  /// The Tally whose entry form a pick opens.
  var note = ""
  /// Where it sorts among the appointments.
  var block = ""
  /// The minute of the day of a timed row.
  var at: Int?
  /// An appointment's calendar colour, 0xRRGGBB; nil for none.
  var colour: Int?
  /// Whether it is on a day after the one the widget shows as today
  /// (#194): its box checks nothing and opens the app.
  var ahead = false

  /// Whether it shows a box.
  var box: Bool {
    switch kind {
    case Self.occurrence, Self.session, Self.pick, Self.missed, Self.line, Self.checklistLine,
      Self.frog:
      return true
    case Self.shopping:
      return !line.isEmpty
    default:
      return false
    }
  }

  /// Whether it is a Shopping List's or a Checklist's heading, which has
  /// no box: its words start where those of a heading with a box do.
  var listHeading: Bool { kind == Self.checklist || (kind == Self.shopping && line.isEmpty) }

  /// Whether a heading over other rows: "Today", a coming day's,
  /// "Missed" or "Eat a frog"; never the last row the widget shows.
  var isHeading: Bool { kind == Self.heading }

  /// What its box queues for the app, as the Agenda's box does; nil when
  /// it has none, is checked already, this Device may not check it or
  /// it is on a coming day. A Session's and a pick's box queue nothing.
  var tap: WidgetTap? {
    if !box || done || !checks || ahead { return nil }
    switch kind {
    case Self.occurrence, Self.missed:
      return WidgetTap(kind: QueuedAction.done, routine: routine, occurrence: occurrence, line: "")
    case Self.line:
      return WidgetTap(kind: QueuedAction.line, routine: routine, occurrence: occurrence, line: line)
    case Self.checklistLine:
      return WidgetTap(kind: QueuedAction.checklist, routine: "", occurrence: "", line: line)
    case Self.frog:
      return WidgetTap(kind: QueuedAction.frog, routine: frog, occurrence: "", line: "")
    case Self.shopping:
      return WidgetTap(kind: QueuedAction.shopping, routine: "", occurrence: "", line: line)
    default:
      return nil
    }
  }

  /// What a tap on its box does (#208, #209): a Session's does nothing,
  /// a pick's opens its entry form as its words do, any other queues its
  /// tap; where it has none, the app opens.
  var onBox: WidgetAction {
    switch kind {
    case Self.session: return .nothing
    case Self.pick: return onRow
    default: return tap.map { WidgetAction.tick($0) } ?? .open
    }
  }

  /// What a tap on the rest of it does: a pick of today this Device may
  /// record for opens its entry form, the Unsorted count the Unsorted
  /// list, any other row the app.
  var onRow: WidgetAction {
    if kind == Self.unsorted { return .unsorted }
    if kind == Self.pick && checks && !ahead && !routine.isEmpty && !occurrence.isEmpty
      && !note.isEmpty
    {
      return .link(.pick(session: routine, occurrence: occurrence, note: note))
    }
    return .open
  }
}

/// What a box checked in the widget queues: its kind, Routine or Frog,
/// Occurrence and Line, as android/…/EventQueue.kt's entries hold them.
struct WidgetTap: Equatable {
  var kind: String
  var routine: String
  var occurrence: String
  var line: String

  /// Queues it in `queue` at `now`, with the row's words as a
  /// notification's Done carries them.
  @discardableResult
  func queue(in actions: inout ActionQueue, title: String, body: String, now: Int64)
    -> QueuedAction
  {
    actions.add(
      kind: kind, routine: routine, occurrence: occurrence, title: title, body: body, line: line,
      now: now)
  }
}

/// What a tap on the widget does.
enum WidgetAction: Equatable {
  /// The box is checked: queued for the app, without opening it.
  case tick(WidgetTap)
  /// The app opens at `link`.
  case link(WidgetLink)
  /// The app opens on the Unsorted list.
  case unsorted
  /// The app opens on the Agenda.
  case open
  /// Nothing happens: a Session's box only shows Done.
  case nothing
}

/// What the app handed over for the Agenda widget, version 4 (#194):
/// the Missed Summary, the rows of the days the Agenda shows by
/// `YYYY-MM-DD` with each day's heading, the Unsorted count, the
/// calendars whose appointments show, and the texts in the reader's
/// language.
struct AgendaSnapshot: Equatable {
  var days: [String: [WidgetRow]]
  var headings: [String: String]
  var missed: [WidgetRow]
  var unsorted: Int
  var calendars: [String]
  var today: String
  var tomorrow: String
  var empty: String
  var planned: String
  var unsortedText: String
  var done: String
  var allDay: String
  var noTitle: String
  /// "from {time}" and "until {time}".
  var since: String
  var until: String
  /// How much bigger the text shows: `AgendaWidgetPlan.fatFingers` with
  /// "Fat fingers" on (#224), else 1.
  var textScale: Double
}

/// An appointment of a calendar, read by the widget as it shows and
/// never kept: an all-day one runs from midnight UTC to midnight UTC.
struct CalendarEntry: Equatable {
  var title: String
  var begin: Date
  var end: Date
  var allDay: Bool
  var colour: Int?
}

/// What the widget shows now: `rows` in Agenda order, `stale` when the
/// snapshot does not reach today and the widget asks to open the app.
struct AgendaView: Equatable {
  var rows: [WidgetRow]
  var stale: Bool
}

/// The Agenda widget's rules, free of WidgetKit, so XCTest covers them:
/// which day of the snapshot shows, what was checked in the widget or on
/// a notification and not yet applied, where the calendar's
/// appointments go, when the day ends and how many rows fit.
enum AgendaWidgetPlan {
  /// The only version the iPhone app ever wrote (#194).
  static let version = 4

  /// How much bigger "Fat fingers" shows the text (#224), as in the app.
  static let fatFingers = 1.3

  /// The snapshot in `data`; nil when there is none, it does not read or
  /// it is of another version, which makes the widget ask to open the
  /// app. A row that does not read, or of a kind not known, is left out,
  /// and so is a heading that is no text.
  static func snapshot(_ data: Data?) -> AgendaSnapshot? {
    guard let data = data,
      let json = (try? JSONSerialization.jsonObject(with: data)) as? [String: Any],
      isVersion(json["v"]),
      let labels = json["labels"] as? [String: Any],
      let days = json["days"] as? [String: Any]
    else { return nil }
    let label = { (name: String) in labels[name] as? String ?? "" }
    return AgendaSnapshot(
      days: days.mapValues { rows(of: $0) },
      headings: (json["headings"] as? [String: Any] ?? [:]).compactMapValues { $0 as? String },
      missed: rows(of: json["missed"]),
      unsorted: (json["unsorted"] as? NSNumber)?.intValue ?? 0,
      calendars: (json["calendars"] as? [Any] ?? []).compactMap { $0 as? String },
      today: label("today"),
      tomorrow: label("tomorrow"),
      empty: label("empty"),
      planned: label("planned"),
      unsortedText: label("unsorted"),
      done: label("done"),
      allDay: label("allDay"),
      noTitle: label("noTitle"),
      since: label("since"),
      until: label("until"),
      textScale: (json["fatFingers"] as? Bool) == true ? fatFingers : 1)
  }

  /// Whether `v` is the whole number 4, not true or 4.5.
  private static func isVersion(_ v: Any?) -> Bool {
    guard let number = v as? NSNumber, CFGetTypeID(number) != CFBooleanGetTypeID() else {
      return false
    }
    return number.doubleValue == Double(version)
  }

  private static func rows(of value: Any?) -> [WidgetRow] {
    (value as? [Any] ?? []).compactMap { row($0 as? [String: Any]) }
  }

  private static func row(_ json: [String: Any]?) -> WidgetRow? {
    guard let json = json, let kind = json["kind"] as? String, WidgetRow.handed.contains(kind),
      let title = json["title"] as? String
    else { return nil }
    let text = { (name: String) in json[name] as? String ?? "" }
    let flag = { (name: String) in (json[name] as? Bool) ?? false }
    return WidgetRow(
      kind: kind,
      title: title,
      time: text("time"),
      done: flag("done"),
      checks: flag("checks"),
      indent: (json["indent"] as? NSNumber)?.intValue ?? 0,
      routine: text("routine"),
      occurrence: text("occurrence"),
      line: text("line"),
      frog: text("frog"),
      note: text("note"),
      block: text("block"),
      at: (json["at"] as? NSNumber)?.intValue)
  }

  // MARK: Days

  private static func calendar(_ zone: TimeZone) -> Calendar {
    var calendar = Calendar(identifier: .gregorian)
    calendar.timeZone = zone
    return calendar
  }

  /// The day `date` shows on `zone`'s clock, `YYYY-MM-DD`.
  static func dayKey(_ date: Date, zone: TimeZone) -> String {
    let parts = calendar(zone).dateComponents([.year, .month, .day], from: date)
    return String(format: "%04d-%02d-%02d", parts.year ?? 0, parts.month ?? 0, parts.day ?? 0)
  }

  /// The local midnight that starts the day of `date` on `zone`'s clock.
  static func midnight(of date: Date, zone: TimeZone) -> Date {
    calendar(zone).startOfDay(for: date)
  }

  /// The next local midnight after `date` on `zone`'s clock.
  static func nextMidnight(after date: Date, zone: TimeZone) -> Date {
    let calendar = calendar(zone)
    let start = calendar.startOfDay(for: date)
    return calendar.date(byAdding: .day, value: 1, to: start) ?? start.addingTimeInterval(86_400)
  }

  private static func minute(of date: Date, zone: TimeZone) -> Int {
    let parts = calendar(zone).dateComponents([.hour, .minute], from: date)
    return (parts.hour ?? 0) * 60 + (parts.minute ?? 0)
  }

  private static func clock(_ minute: Int) -> String {
    String(format: "%02d:%02d", minute / 60, minute % 60)
  }

  // MARK: Appointments

  /// The rows of `entries` on `day` as the Agenda shows them (#116):
  /// all-day ones first, then by time, each by title after that; read
  /// only, marked with their calendar's colour.
  static func appointmentRows(
    _ snapshot: AgendaSnapshot, _ entries: [CalendarEntry], day: String, zone: TimeZone
  ) -> [WidgetRow] {
    let utc = TimeZone(identifier: "UTC") ?? zone
    struct Piece {
      var entry: CalendarEntry
      var from: Int?
      var until: Int?
      var isAllDay: Bool { from == nil && until == nil }
      var at: Int? { isAllDay ? nil : from ?? 0 }
    }
    let pieces = entries.compactMap { entry -> Piece? in
      if entry.allDay {
        // Midnight UTC to midnight UTC, the end not counted.
        let first = dayKey(entry.begin, zone: utc)
        let end = dayKey(entry.end, zone: utc)
        let last = end > first ? dayKey(entry.end.addingTimeInterval(-86_400), zone: utc) : first
        return day < first || day > last ? nil : Piece(entry: entry, from: nil, until: nil)
      }
      let first = dayKey(entry.begin, zone: zone)
      let endDay = dayKey(entry.end, zone: zone)
      let endMinute = minute(of: entry.end, zone: zone)
      // An end at midnight closes the day before.
      let last =
        endMinute == 0 && endDay > first
        ? dayKey(entry.end.addingTimeInterval(-60), zone: zone) : endDay
      if day < first || day > last { return nil }
      return Piece(
        entry: entry,
        from: day == first ? minute(of: entry.begin, zone: zone) : nil,
        until: day == endDay ? endMinute : nil)
    }.sorted { a, b in
      let (x, y) = (a.at ?? -1, b.at ?? -1)
      return x != y ? x < y : a.entry.title < b.entry.title
    }
    return pieces.map { piece in
      let time: String
      switch (piece.from, piece.until) {
      case (nil, nil): time = snapshot.allDay
      case let (from?, until?) where from == until: time = clock(from)
      case let (from?, until?): time = "\(clock(from))–\(clock(until))"
      case let (from?, nil): time = snapshot.since.replacingOccurrences(of: "{time}", with: clock(from))
      case let (nil, until?): time = snapshot.until.replacingOccurrences(of: "{time}", with: clock(until))
      }
      let title = piece.entry.title.trimmingCharacters(in: .whitespacesAndNewlines)
      return WidgetRow(
        kind: WidgetRow.appointment,
        title: title.isEmpty ? snapshot.noTitle : title,
        time: time,
        block: piece.isAllDay ? WidgetRow.allDay : WidgetRow.timed,
        at: piece.at,
        colour: piece.entry.colour)
    }
  }

  /// The day's `rows` with the `appointments` put in as the Agenda puts
  /// them: the all-day ones after the all-day Occurrences, the timed ones
  /// among the timed Occurrences by time, an Occurrence, with what shows
  /// under it, before an appointment at the same time.
  static func merged(_ rows: [WidgetRow], _ appointments: [WidgetRow]) -> [WidgetRow] {
    var out = rows
    let afterAllDay = out.firstIndex { $0.block != WidgetRow.allDay } ?? out.count
    out.insert(contentsOf: appointments.filter { $0.block == WidgetRow.allDay }, at: afterAllDay)
    for piece in appointments where piece.block == WidgetRow.timed {
      let at = piece.at ?? 0
      let before = out.firstIndex {
        $0.block == WidgetRow.frogs || ($0.block == WidgetRow.timed && ($0.at ?? 0) > at)
      }
      out.insert(piece, at: before ?? out.count)
    }
    return out
  }

  // MARK: The view

  /// The days the widget shows on `today` from `snapshot`, in order:
  /// today, then each later day of it with a heading; none when it does
  /// not reach today.
  static func shownDays(_ snapshot: AgendaSnapshot, today: String) -> [String] {
    guard snapshot.days[today] != nil else { return [] }
    return [today]
      + snapshot.days.keys.filter { $0 > today && snapshot.headings[$0] != nil }.sorted()
  }

  /// What the widget shows on `today` from `snapshot`, in Agenda order:
  /// the Missed Summary, the Unsorted count, then the day under its
  /// heading with its appointments, "Nothing due today." where nothing
  /// is, then the Frogs; then each coming day under its heading,
  /// "Tomorrow" first, "Nothing planned" where nothing is (#194). Only
  /// today's and the Missed rows tick: a coming day's row is `ahead`.
  /// What is still `pending` for the app shows checked. Without a
  /// snapshot that reaches today it shows `openApp`.
  static func view(
    _ snapshot: AgendaSnapshot?, today: String, pending: [QueuedAction],
    appointments: [String: [WidgetRow]], openApp: String
  ) -> AgendaView {
    guard let snapshot = snapshot else {
      return AgendaView(rows: [message(openApp)], stale: true)
    }
    let top =
      checked(snapshot.missed, pending)
      + (snapshot.unsorted > 0
        ? [WidgetRow(kind: WidgetRow.unsorted, title: snapshot.unsortedText)] : [])
    guard let day = snapshot.days[today] else {
      return AgendaView(rows: top + [message(openApp)], stale: true)
    }
    let rows = merged(checked(day, pending), appointments[today] ?? [])
    let due = rows.filter { $0.block != WidgetRow.frogs }
    let frogs = rows.filter { $0.block == WidgetRow.frogs }
    var coming: [WidgetRow] = []
    for (i, key) in shownDays(snapshot, today: today).dropFirst().enumerated() {
      let later = merged(snapshot.days[key] ?? [], appointments[key] ?? []).map { row -> WidgetRow in
        var row = row
        row.ahead = true
        return row
      }
      coming.append(
        WidgetRow(
          kind: WidgetRow.heading, title: i == 0 ? snapshot.tomorrow : snapshot.headings[key] ?? ""))
      if later.isEmpty {
        var nothing = message(snapshot.planned)
        nothing.ahead = true
        coming.append(nothing)
      } else {
        coming += later
      }
    }
    return AgendaView(
      rows: top + [WidgetRow(kind: WidgetRow.heading, title: snapshot.today)]
        + (due.isEmpty ? [message(snapshot.empty)] : due) + frogs + coming,
      stale: false)
  }

  private static func message(_ text: String) -> WidgetRow {
    WidgetRow(kind: WidgetRow.message, title: text)
  }

  /// `rows` with what is still `pending` for the app shown checked: the
  /// last Line of an Occurrence checks it off, and a Frog checked off
  /// leaves the other one out.
  static func checked(_ rows: [WidgetRow], _ pending: [QueuedAction]) -> [WidgetRow] {
    let of = { (kind: String) in pending.filter { $0.kind == kind } }
    let done = Set(of(QueuedAction.done).map { "\($0.routine)@\($0.occurrence)" })
    let lines = Set(of(QueuedAction.line).map { "\($0.routine)@\($0.occurrence)#\($0.line)" })
    let frogs = Set(of(QueuedAction.frog).map { $0.routine })
    let shopping = Set(of(QueuedAction.shopping).map { $0.line })
    let checklist = Set(of(QueuedAction.checklist).map { $0.line })
    let tapped = rows.map { row -> WidgetRow in
      let now: Bool
      switch row.kind {
      case WidgetRow.occurrence, WidgetRow.session, WidgetRow.missed:
        now = done.contains("\(row.routine)@\(row.occurrence)")
      case WidgetRow.line: now = lines.contains("\(row.routine)@\(row.occurrence)#\(row.line)")
      case WidgetRow.frog: now = frogs.contains(row.frog)
      case WidgetRow.shopping: now = !row.line.isEmpty && shopping.contains(row.line)
      case WidgetRow.checklistLine: now = !row.line.isEmpty && checklist.contains(row.line)
      default: now = false
      }
      var shown = row
      if now { shown.done = true }
      return shown
    }
    // Every Line checked checks its Occurrence off.
    let key = { (row: WidgetRow) in "\(row.routine)@\(row.occurrence)" }
    let lineRows = tapped.filter { $0.kind == WidgetRow.line }
    let complete = Set(lineRows.map(key)).subtracting(lineRows.filter { !$0.done }.map(key))
    let eaten = tapped.contains { $0.kind == WidgetRow.frog && $0.done }
    return tapped.compactMap { row in
      if row.kind == WidgetRow.occurrence && !row.done && complete.contains(key(row)) {
        var shown = row
        shown.done = true
        return shown
      }
      // Once a Frog is eaten, only the eaten ones show.
      if row.kind == WidgetRow.frog && eaten && !row.done { return nil }
      return row
    }
  }
}

/// How many of the widget's rows fit (#242). A widget does not scroll as
/// Android's list does, so it shows the rows from the top as far as its
/// height allows, never a heading without a row under it, and "+n" for
/// the rows left out; a tap there opens the app.
enum AgendaFit {
  /// One row's height in points at text scale 1: 14 pt text and the
  /// widget's 4 pt between rows, as on Android.
  static let rowHeight = 22.0

  /// How many rows fit into `height` points with text `scale` times as
  /// big; at least one.
  static func lines(height: Double, scale: Double) -> Int {
    max(1, Int((height / (rowHeight * max(1, scale))).rounded(.down)))
  }

  /// The rows of `rows` shown in `lines` rows.
  static func fit(_ rows: [WidgetRow], lines: Int) -> [WidgetRow] {
    if rows.count <= lines { return rows }
    var shown = Array(rows.prefix(max(0, lines - 1)))
    while let last = shown.last, last.isHeading { shown.removeLast() }
    shown.append(WidgetRow(kind: WidgetRow.more, title: "+\(rows.count - shown.count)"))
    return shown
  }
}
