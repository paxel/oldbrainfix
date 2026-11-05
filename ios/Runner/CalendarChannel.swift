import EventKit
import EventKitUI
import Flutter
import Foundation
import UIKit

/// The calendar channel on the iPhone (#241), answering MainActivity.kt's
/// calls with EventKit: the calendars and appointments the Agenda shows,
/// the calendar mirror's calendars, its own events and writing them, the
/// calendar's own insert screen and a calendar file to the share sheet.
/// Reading and the mirror need full access (iOS 17 and newer), the same
/// one the Capture Context's running event asks for; the insert screen
/// needs none.
enum CalendarChannel {
  static let name = "io.github.paxel.oldbrainfix/calendar"

  static func register(messenger: FlutterBinaryMessenger) {
    let channel = FlutterMethodChannel(name: name, binaryMessenger: messenger)
    channel.setMethodCallHandler { call, result in
      let args = call.arguments as? [String: Any] ?? [:]
      let calendars = Calendars.shared
      switch call.method {
      case "mirrorGranted":
        result(Calendars.granted)
      case "mirrorRequest":
        calendars.request { result($0) }
      case "zone":
        result(TimeZone.current.identifier)
      case "calendars":
        calendars.run(result) { $0.writable() }
      case "readCalendars":
        calendars.run(result) { $0.readable() }
      case "appointments":
        let from = (args["from"] as? NSNumber)?.doubleValue ?? 0
        let until = (args["until"] as? NSNumber)?.doubleValue ?? 0
        let ids = Set(args["calendars"] as? [String] ?? [])
        calendars.run(result) {
          $0.appointments(
            from: Date(timeIntervalSince1970: from / 1000),
            until: Date(timeIntervalSince1970: until / 1000), calendarIds: ids)
        }
      case "marked":
        let id = args["calendar"] as? String ?? ""
        calendars.run(result) { $0.marked(id) }
      case "apply":
        let id = args["calendar"] as? String ?? ""
        let minutes = (args["minutes"] as? NSNumber)?.intValue ?? 15
        let creates = args["create"] as? [[String: Any]] ?? []
        let updates = args["update"] as? [[String: Any]] ?? []
        let deletes = args["delete"] as? [String] ?? []
        calendars.run(result) {
          $0.apply(id, creates: creates, updates: updates, deletes: deletes, minutes: minutes)
        }
      case "insert":
        result(CalendarScreens.insert(args))
      case "share":
        result(CalendarScreens.share(args))
      default:
        result(FlutterMethodNotImplemented)
      }
    }
  }
}

/// EventKit for the calendar and context channels: one store, used on
/// one queue, made anew once access is granted, as a store made before
/// sees no calendars.
final class Calendars {
  static let shared = Calendars()

  private let queue = DispatchQueue(label: "io.github.paxel.oldbrainfix.calendar")
  private var store = EKEventStore()

  /// Whether the app may read and write the calendars. Never asks.
  static var granted: Bool { EKEventStore.authorizationStatus(for: .event) == .fullAccess }

  /// Asks for full access where iOS still asks, and answers on the main
  /// thread whether it is allowed now. Write-only access asks again.
  func request(_ done: @escaping (Bool) -> Void) {
    switch EKEventStore.authorizationStatus(for: .event) {
    case .fullAccess:
      done(true)
      return
    case .denied, .restricted:
      done(false)
      return
    default:
      break
    }
    queue.async {
      self.store.requestFullAccessToEvents { allowed, _ in
        self.queue.async {
          if allowed { self.store = EKEventStore() }
          DispatchQueue.main.async { done(allowed) }
        }
      }
    }
  }

  /// Runs [work] on the calendar queue and answers its value on the
  /// main thread.
  func run(_ result: @escaping FlutterResult, _ work: @escaping (Calendars) -> Any?) {
    queue.async {
      let answer = work(self)
      DispatchQueue.main.async { result(answer) }
    }
  }

  /// The events running now that the user did not decline, as the
  /// context channel's `calendar` answers them; nil without access.
  func running() -> [[String: Any]]? {
    guard Calendars.granted else { return nil }
    let now = Date()
    let day: TimeInterval = 24 * 60 * 60
    let predicate = store.predicateForEvents(
      withStart: now.addingTimeInterval(-day), end: now.addingTimeInterval(day), calendars: nil)
    let facts = store.events(matching: predicate).map(Self.facts)
    return CalendarRules.running(facts, now: now, in: .current)
  }

  /// Every calendar the Calendar app shows, as `readCalendars` answers.
  func readable() -> [[String: Any]] {
    guard Calendars.granted else { return [] }
    return store.calendars(for: .event).map { calendar in
      [
        "id": calendar.calendarIdentifier,
        "name": calendar.title,
        "account": calendar.source?.title ?? "",
        "colour": Self.colour(calendar.cgColor),
      ]
    }
  }

  /// The calendars events can be written into, as `calendars` answers.
  func writable() -> [[String: Any]] {
    guard Calendars.granted else { return [] }
    return store.calendars(for: .event).filter(\.allowsContentModifications).map { calendar in
      [
        "id": calendar.calendarIdentifier,
        "name": calendar.title,
        "account": calendar.source?.title ?? "",
      ]
    }
  }

  /// The appointments of [calendarIds] overlapping [from] until [until],
  /// without the mirror's own; nil without access.
  func appointments(from: Date, until: Date, calendarIds: Set<String>) -> [[String: Any]]? {
    guard Calendars.granted else { return nil }
    let picked = store.calendars(for: .event).filter { calendarIds.contains($0.calendarIdentifier) }
    guard !picked.isEmpty, from < until else { return [] }
    let predicate = store.predicateForEvents(withStart: from, end: until, calendars: picked)
    let facts = store.events(matching: predicate).map(Self.facts)
    return CalendarRules.shown(facts, calendarIds: calendarIds, in: .current)
  }

  /// The events the mirror wrote into [calendarId] as `events`, each id
  /// to its UID, or `gone` when that calendar cannot be written into any
  /// more; nil without access. EventKit finds events only by time, so
  /// the ids the mirror wrote are looked up as well as two years either
  /// side of now.
  func marked(_ calendarId: String) -> [String: Any]? {
    guard Calendars.granted else { return nil }
    guard let calendar = store.calendar(withIdentifier: calendarId),
      calendar.allowsContentModifications
    else { return ["gone": true] }
    var events: [String: String] = [:]
    for id in MirrorIds.read(calendarId) {
      if let event = mirrorEvent(id, in: calendarId), let uid = MirrorMark.uid(event.url?.absoluteString) {
        events[event.calendarItemIdentifier] = uid
      }
    }
    let now = Date()
    let years: TimeInterval = 2 * 365 * 24 * 60 * 60
    let predicate = store.predicateForEvents(
      withStart: now.addingTimeInterval(-years), end: now.addingTimeInterval(years),
      calendars: [calendar])
    for event in store.events(matching: predicate) {
      if let uid = MirrorMark.uid(event.url?.absoluteString) {
        events[event.calendarItemIdentifier] = uid
      }
    }
    MirrorIds.write(calendarId, Set(events.keys))
    return ["events": events]
  }

  /// Carries out a plan from the app as CalendarMirror.kt does: deletes
  /// first, then updates and creates. Answers the ids the updated and
  /// created events have now, in their order, NSNull for one that
  /// failed, and whether every delete went through; nil without access.
  func apply(
    _ calendarId: String, creates: [[String: Any]], updates: [[String: Any]], deletes: [String],
    minutes: Int
  ) -> [String: Any]? {
    guard Calendars.granted, let calendar = store.calendar(withIdentifier: calendarId) else {
      return nil
    }
    var ids = MirrorIds.read(calendarId)
    var deleted = true
    for id in deletes {
      if delete(id, in: calendarId) { ids.remove(id) } else { deleted = false }
    }
    let updated: [Any] = updates.map { row -> Any in
      let old = row["id"] as? String ?? ""
      guard let id = update(old, in: calendar, row: row, minutes: minutes) else { return NSNull() }
      ids.remove(old)
      ids.insert(id)
      return id
    }
    let created: [Any] = creates.map { row -> Any in
      guard let id = create(in: calendar, row: row, minutes: minutes) else { return NSNull() }
      ids.insert(id)
      return id
    }
    MirrorIds.write(calendarId, ids)
    return ["updated": updated, "created": created, "deleted": deleted]
  }

  /// The mirror's event [id] in [calendarId]; nil for one gone or not
  /// the mirror's.
  private func mirrorEvent(_ id: String, in calendarId: String) -> EKEvent? {
    guard let event = store.calendarItem(withIdentifier: id) as? EKEvent,
      event.calendar?.calendarIdentifier == calendarId,
      MirrorMark.uid(event.url?.absoluteString) != nil
    else { return nil }
    return event
  }

  /// Removes the mirror's event [id], every repeat of it; whether it is
  /// gone now. Never touches an event it did not write.
  private func delete(_ id: String, in calendarId: String) -> Bool {
    guard let event = mirrorEvent(id, in: calendarId) else { return true }
    do {
      try store.remove(event, span: .futureEvents, commit: true)
      return true
    } catch {
      store.reset()
      return false
    }
  }

  private func create(in calendar: EKCalendar, row: [String: Any], minutes: Int) -> String? {
    let event = EKEvent(eventStore: store)
    event.calendar = calendar
    guard Self.fill(event, row: row, minutes: minutes, marked: true) else { return nil }
    do {
      try store.save(event, span: .thisEvent, commit: true)
      return event.calendarItemIdentifier
    } catch {
      store.reset()
      return nil
    }
  }

  /// Writes [row] over the mirror's event [id]; its id, a new one when
  /// EventKit would only take it as a new event, or nil when that failed
  /// or the event is no longer the mirror's.
  private func update(_ id: String, in calendar: EKCalendar, row: [String: Any], minutes: Int)
    -> String?
  {
    guard let event = mirrorEvent(id, in: calendar.calendarIdentifier) else { return nil }
    guard Self.fill(event, row: row, minutes: minutes, marked: true) else { return nil }
    do {
      try store.save(event, span: .futureEvents, commit: true)
      return event.calendarItemIdentifier
    } catch {
      store.reset()
      guard delete(id, in: calendar.calendarIdentifier) else { return nil }
      return create(in: calendar, row: row, minutes: minutes)
    }
  }

  /// Fills [event] from a channel row of `uid`, `title`, `description`,
  /// `start` and `rule`: at the wall clock of the Device's zone, free
  /// time, as the app rings its own alarms, and with the mirror's mark
  /// when [marked]. False for a start or rule EventKit cannot hold.
  static func fill(_ event: EKEvent, row: [String: Any], minutes: Int, marked: Bool) -> Bool {
    let calendar = Calendar.current
    guard let start = row["start"] as? String,
      let span = StartKey.span(start, minutes: minutes, in: calendar)
    else { return false }
    event.title = row["title"] as? String ?? ""
    let notes = row["description"] as? String ?? ""
    event.notes = notes.isEmpty ? nil : notes
    event.isAllDay = span.allDay
    event.startDate = span.start
    event.endDate = span.end
    event.timeZone = span.allDay ? nil : calendar.timeZone
    event.availability = .free
    if marked, let uid = row["uid"] as? String { event.url = MirrorMark.url(uid) }
    if let text = row["rule"] as? String {
      guard let rule = RuleParts.parse(text) else { return false }
      event.recurrenceRules = [recurrence(rule, in: calendar)]
    } else {
      event.recurrenceRules = nil
    }
    return true
  }

  /// [rule] as EventKit's recurrence rule.
  static func recurrence(_ rule: RuleParts, in calendar: Calendar) -> EKRecurrenceRule {
    let frequency: EKRecurrenceFrequency =
      switch rule.frequency {
      case .daily: .daily
      case .weekly: .weekly
      case .monthly: .monthly
      case .yearly: .yearly
      }
    let days: [EKRecurrenceDayOfWeek]? =
      rule.byDay.isEmpty
      ? nil
      : rule.byDay.compactMap { day in
        EKWeekday(rawValue: day.weekday).map {
          day.ordinal == 0 ? EKRecurrenceDayOfWeek($0) : EKRecurrenceDayOfWeek($0, weekNumber: day.ordinal)
        }
      }
    let end: EKRecurrenceEnd? =
      rule.count.map { EKRecurrenceEnd(occurrenceCount: $0) }
      ?? rule.untilDate(in: calendar).map { EKRecurrenceEnd(end: $0) }
    return EKRecurrenceRule(
      recurrenceWith: frequency, interval: rule.interval, daysOfTheWeek: days,
      daysOfTheMonth: rule.byMonthDay.isEmpty ? nil : rule.byMonthDay.map { NSNumber(value: $0) },
      monthsOfTheYear: rule.byMonth.isEmpty ? nil : rule.byMonth.map { NSNumber(value: $0) },
      weeksOfTheYear: nil, daysOfTheYear: nil, setPositions: nil, end: end)
  }

  /// [event] as plain values for CalendarRules.
  static func facts(_ event: EKEvent) -> EventFacts {
    EventFacts(
      id: event.calendarItemIdentifier,
      calendarId: event.calendar?.calendarIdentifier ?? "",
      title: event.title ?? "",
      start: event.startDate,
      end: event.endDate ?? event.startDate,
      allDay: event.isAllDay,
      location: event.location ?? "",
      notes: event.notes ?? "",
      declined: event.attendees?.first(where: \.isCurrentUser)?.participantStatus == .declined,
      url: event.url?.absoluteString)
  }

  private static func colour(_ colour: CGColor?) -> Int {
    guard let colour else { return 0 }
    var red: CGFloat = 0
    var green: CGFloat = 0
    var blue: CGFloat = 0
    var alpha: CGFloat = 0
    guard UIColor(cgColor: colour).getRed(&red, green: &green, blue: &blue, alpha: &alpha) else {
      return 0
    }
    return CalendarRules.colour(
      red: Double(red), green: Double(green), blue: Double(blue), alpha: Double(alpha))
  }
}

/// The ids of the events the mirror wrote, per calendar, in Application
/// Support: EventKit looks events up only by time, and a Reminder long
/// past would otherwise be written again.
enum MirrorIds {
  private static var file: URL {
    let base =
      FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
      ?? FileManager.default.temporaryDirectory
    return base.appendingPathComponent("calendar", isDirectory: true)
      .appendingPathComponent("mirror.json")
  }

  private static func all() -> [String: [String]] {
    guard let data = try? Data(contentsOf: file),
      let ids = try? JSONDecoder().decode([String: [String]].self, from: data)
    else { return [:] }
    return ids
  }

  static func read(_ calendarId: String) -> Set<String> { Set(all()[calendarId] ?? []) }

  static func write(_ calendarId: String, _ ids: Set<String>) {
    var everything = all()
    everything[calendarId] = ids.isEmpty ? nil : ids.sorted()
    guard let data = try? JSONEncoder().encode(everything) else { return }
    try? FileManager.default.createDirectory(
      at: file.deletingLastPathComponent(), withIntermediateDirectories: true)
    try? data.write(to: file, options: [.atomic, .completeFileProtectionUntilFirstUserAuthentication])
  }
}

/// The calendar's screens the calendar channel opens: its own insert
/// screen, filled in and saved by the user there, which needs no access,
/// and the share sheet for a calendar file.
enum CalendarScreens {
  private static let closer = EditorCloser()

  /// Opens the insert screen for a channel row of `title`,
  /// `description`, `start`, `rule` and `minutes`; false when there is
  /// no screen to show it over. A rule EventKit cannot hold is left out,
  /// as Android's calendar apps leave out what they do not support.
  static func insert(_ args: [String: Any]) -> Bool {
    guard let top = FolderPicker.top() else { return false }
    let store = EKEventStore()
    let event = EKEvent(eventStore: store)
    let minutes = (args["minutes"] as? NSNumber)?.intValue ?? 15
    if !Calendars.fill(event, row: args, minutes: minutes, marked: false) {
      var single = args
      single["rule"] = nil
      if !Calendars.fill(event, row: single, minutes: minutes, marked: false) {
        event.title = args["title"] as? String ?? ""
        let notes = args["description"] as? String ?? ""
        event.notes = notes.isEmpty ? nil : notes
      }
    }
    let editor = EKEventEditViewController()
    editor.eventStore = store
    editor.event = event
    editor.editViewDelegate = closer
    top.present(editor, animated: true)
    return true
  }

  /// Writes `bytes` as the file `name` and offers it on the share sheet;
  /// false when there is no screen to show it over or no file.
  static func share(_ args: [String: Any]) -> Bool {
    guard let top = FolderPicker.top(),
      let bytes = (args["bytes"] as? FlutterStandardTypedData)?.data
    else { return false }
    let raw = (args["name"] as? String) ?? ""
    let name = (raw as NSString).lastPathComponent.isEmpty ? "export" : (raw as NSString).lastPathComponent
    let dir = FileManager.default.temporaryDirectory.appendingPathComponent("exports", isDirectory: true)
    let file = dir.appendingPathComponent(name)
    do {
      try FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
      try bytes.write(to: file, options: .atomic)
    } catch {
      return false
    }
    let sheet = UIActivityViewController(activityItems: [file], applicationActivities: nil)
    if let popover = sheet.popoverPresentationController {
      popover.sourceView = top.view
      popover.sourceRect = CGRect(x: top.view.bounds.midX, y: top.view.bounds.midY, width: 0, height: 0)
      popover.permittedArrowDirections = []
    }
    top.present(sheet, animated: true)
    return true
  }
}

/// Closes the insert screen however it was left.
private final class EditorCloser: NSObject, EKEventEditViewDelegate {
  func eventEditViewController(
    _ controller: EKEventEditViewController, didCompleteWith action: EKEventEditViewAction
  ) {
    controller.dismiss(animated: true)
  }
}
