import AppIntents
import EventKit
import SwiftUI
import UIKit
import WidgetKit

/// The Agenda widget on the iPhone (#242), medium and large: what
/// android/…/AgendaWidget.kt shows, from the same snapshot, which the app
/// writes into the App Group (`WidgetFiles`). A widget does not scroll,
/// so it shows the rows from the top as far as they fit (`AgendaFit`). A
/// box of today or of the Missed Summary ticks through `TickIntent`
/// without opening the app; a pick opens its entry form, the Unsorted
/// count the Unsorted list, and every other tap the app at the Agenda.
struct AgendaWidget: Widget {
  var body: some WidgetConfiguration {
    StaticConfiguration(kind: WidgetFiles.agendaKind, provider: AgendaProvider()) { entry in
      AgendaWidgetView(entry: entry)
    }
    .configurationDisplayName("agenda.name")
    .description("agenda.description")
    .supportedFamilies([.systemMedium, .systemLarge])
  }
}

struct AgendaEntry: TimelineEntry {
  let date: Date
  let rows: [WidgetRow]
  let scale: Double
  /// What a box says to VoiceOver.
  let done: String
}

/// The widget's days: one entry now and one at each of the next two
/// local midnights, so it moves on to the new day without the app, as
/// Android's midnight alarm does. Every tick and every snapshot reloads
/// it.
struct AgendaProvider: TimelineProvider {
  func placeholder(in context: Context) -> AgendaEntry {
    AgendaEntry(date: Date(), rows: [], scale: 1, done: "")
  }

  func getSnapshot(in context: Context, completion: @escaping (AgendaEntry) -> Void) {
    completion(Self.entry(at: Date()))
  }

  func getTimeline(in context: Context, completion: @escaping (Timeline<AgendaEntry>) -> Void) {
    let zone = TimeZone.current
    let now = Date()
    let first = AgendaWidgetPlan.nextMidnight(after: now, zone: zone)
    let second = AgendaWidgetPlan.nextMidnight(after: first, zone: zone)
    completion(
      Timeline(
        entries: [now, first, second].map { Self.entry(at: $0) }, policy: .after(second)))
  }

  /// What the widget shows at `date`, with what is still queued for the
  /// app shown checked.
  static func entry(at date: Date) -> AgendaEntry {
    let zone = TimeZone.current
    let snapshot = AgendaWidgetPlan.snapshot(WidgetFiles.readAgenda())
    let today = AgendaWidgetPlan.dayKey(date, zone: zone)
    let appointments =
      snapshot.map { WidgetAppointments.read($0, today: today, at: date, zone: zone) } ?? [:]
    let view = AgendaWidgetPlan.view(
      snapshot, today: today, pending: WidgetFiles.readQueue().pending,
      appointments: appointments, openApp: String(localized: "agenda.openApp"))
    return AgendaEntry(
      date: date, rows: view.rows, scale: snapshot?.textScale ?? 1, done: snapshot?.done ?? "")
  }
}

/// The appointments of the calendars the Agenda shows on each day the
/// widget shows, read as it shows and kept nowhere, as on Android; none
/// while that is off or the app has no full access to the calendars.
enum WidgetAppointments {
  static func read(_ snapshot: AgendaSnapshot, today: String, at date: Date, zone: TimeZone)
    -> [String: [WidgetRow]]
  {
    let days = AgendaWidgetPlan.shownDays(snapshot, today: today)
    guard !snapshot.calendars.isEmpty, !days.isEmpty,
      EKEventStore.authorizationStatus(for: .event) == .fullAccess
    else { return [:] }
    let store = EKEventStore()
    let ids = Set(snapshot.calendars)
    let picked = store.calendars(for: .event).filter { ids.contains($0.calendarIdentifier) }
    guard !picked.isEmpty else { return [:] }
    let colours = Dictionary(
      picked.map { ($0.calendarIdentifier, rgb($0.cgColor)) }, uniquingKeysWith: { a, _ in a })
    let start = AgendaWidgetPlan.midnight(of: date, zone: zone)
    // A day on either side, for the clock changes between.
    let predicate = store.predicateForEvents(
      withStart: start.addingTimeInterval(-86_400),
      end: start.addingTimeInterval(Double(days.count + 1) * 86_400), calendars: picked)
    let facts = store.events(matching: predicate).map { event in
      EventFacts(
        id: event.calendarItemIdentifier,
        calendarId: event.calendar?.calendarIdentifier ?? "",
        title: event.title ?? "",
        start: event.startDate,
        end: event.endDate ?? event.startDate,
        allDay: event.isAllDay,
        declined: event.attendees?.first(where: \.isCurrentUser)?.participantStatus == .declined,
        url: event.url?.absoluteString)
    }
    var calendar = Calendar(identifier: .gregorian)
    calendar.timeZone = zone
    // The Agenda's own rule: the mirror's events and declined ones left
    // out, all-day ones from midnight UTC (CalendarRules).
    let entries = CalendarRules.shown(facts, calendarIds: ids, in: calendar).compactMap {
      row -> CalendarEntry? in
      guard let begin = (row["begin"] as? Int64), let end = (row["end"] as? Int64) else {
        return nil
      }
      return CalendarEntry(
        title: row["title"] as? String ?? "",
        begin: Date(timeIntervalSince1970: Double(begin) / 1000),
        end: Date(timeIntervalSince1970: Double(end) / 1000),
        allDay: row["allDay"] as? Bool ?? false,
        colour: colours[row["calendar"] as? String ?? ""] ?? nil)
    }
    var rows: [String: [WidgetRow]] = [:]
    for day in days {
      rows[day] = AgendaWidgetPlan.appointmentRows(snapshot, entries, day: day, zone: zone)
    }
    return rows
  }

  /// A calendar's colour as 0xRRGGBB.
  private static func rgb(_ colour: CGColor?) -> Int? {
    guard let colour else { return nil }
    var (red, green, blue, alpha): (CGFloat, CGFloat, CGFloat, CGFloat) = (0, 0, 0, 0)
    guard UIColor(cgColor: colour).getRed(&red, green: &green, blue: &blue, alpha: &alpha) else {
      return nil
    }
    return CalendarRules.colour(
      red: Double(red), green: Double(green), blue: Double(blue), alpha: 1) & 0xFF_FFFF
  }
}

struct AgendaWidgetView: View {
  let entry: AgendaEntry

  var body: some View {
    GeometryReader { space in
      let lines = AgendaFit.lines(height: Double(space.size.height), scale: entry.scale)
      VStack(alignment: .leading, spacing: 0) {
        ForEach(Array(AgendaFit.fit(entry.rows, lines: lines).enumerated()), id: \.offset) {
          _, row in
          AgendaRowView(row: row, scale: entry.scale, done: entry.done)
        }
        Spacer(minLength: 0)
      }
    }
    .containerBackground(for: .widget) { Color(uiColor: .systemBackground) }
    .widgetURL(WidgetLink.agenda.url)
  }
}

/// One row, as android/…/AgendaWidget.kt's `rowViews` draws it: its box,
/// its time, its words, struck through when done; a coming day's dimmed.
struct AgendaRowView: View {
  let row: WidgetRow
  let scale: Double
  let done: String

  /// A box's side, and how far an entry moves in under its heading, so
  /// its box stands where the heading's words begin (#208).
  private var boxSide: CGFloat { CGFloat(AgendaFit.rowHeight * scale) }
  private var indent: CGFloat {
    row.indent <= 0 ? 0 : boxSide + CGFloat(14 * scale) * CGFloat(row.indent - 1)
  }

  var body: some View {
    let words = HStack(spacing: CGFloat(4 * scale)) {
      if row.kind == WidgetRow.appointment {
        RoundedRectangle(cornerRadius: 1)
          .fill(row.colour.map(Self.color) ?? Color.accentColor)
          .frame(width: 3)
          .padding(.vertical, 2)
      }
      if !row.time.isEmpty {
        Text(verbatim: row.time)
          .font(.system(size: CGFloat(13 * scale)))
          .foregroundStyle(.secondary)
      }
      Text(verbatim: row.title)
        .font(
          .system(
            size: CGFloat(14 * scale),
            weight: row.kind == WidgetRow.heading ? .semibold : .regular)
        )
        .strikethrough(row.done)
        .foregroundStyle(dim ? AnyShapeStyle(.secondary) : AnyShapeStyle(.primary))
        .lineLimit(1)
      Spacer(minLength: 0)
    }
    HStack(spacing: 0) {
      Spacer().frame(width: indent)
      if row.box {
        box
      } else if row.listHeading {
        // A list's heading has no box; its words start where a boxed
        // heading's do (#209).
        Spacer().frame(width: boxSide)
      }
      tappable(words, row.onRow)
    }
    .frame(height: boxSide)
  }

  private var dim: Bool {
    row.done || row.ahead || [WidgetRow.text, WidgetRow.message, WidgetRow.more].contains(row.kind)
  }

  private var box: some View {
    let image = Image(systemName: row.done ? "checkmark.square.fill" : "square")
      .font(.system(size: CGFloat(15 * scale)))
      .foregroundStyle(row.done ? AnyShapeStyle(.secondary) : AnyShapeStyle(.primary))
      .frame(width: boxSide, height: boxSide)
      .opacity(row.ahead ? 0.55 : 1)
      .accessibilityLabel(Text(verbatim: done))
      .accessibilityAddTraits(row.done ? [.isSelected] : [])
    return tappable(image, row.onBox)
  }

  /// `content` doing `action`: a tick or nothing through an intent, a
  /// link where it opens somewhere, else the widget's own link to the
  /// Agenda.
  @ViewBuilder
  private func tappable<Content: View>(_ content: Content, _ action: WidgetAction) -> some View {
    switch action {
    case .tick(let tap):
      Button(intent: TickIntent(tap, title: row.title, body: row.time)) { content }
        .buttonStyle(.plain)
    case .nothing:
      Button(intent: NothingIntent()) { content }.buttonStyle(.plain)
    case .link(let link):
      Link(destination: link.url) { content }
    case .unsorted:
      Link(destination: WidgetLink.unsorted.url) { content }
    case .open:
      content
    }
  }

  private static func color(_ rgb: Int) -> Color {
    Color(
      red: Double((rgb >> 16) & 0xFF) / 255, green: Double((rgb >> 8) & 0xFF) / 255,
      blue: Double(rgb & 0xFF) / 255)
  }
}

/// A box checked on the widget (#242): queued in the App Group as
/// android/…/AgendaWidget.kt's `check` queues it, for the app to apply
/// the next time it refreshes; WidgetKit then shows the widget anew,
/// the row checked. It runs in the extension and never opens the app.
struct TickIntent: AppIntent {
  static let title: LocalizedStringResource = "intent.tick"
  static let isDiscoverable = false

  @Parameter(title: "Kind") var kind: String
  @Parameter(title: "Routine") var routine: String
  @Parameter(title: "Occurrence") var occurrence: String
  @Parameter(title: "Line") var line: String
  @Parameter(title: "Title") var name: String
  @Parameter(title: "Text") var text: String

  init() {}

  init(_ tap: WidgetTap, title: String, body: String) {
    kind = tap.kind
    routine = tap.routine
    occurrence = tap.occurrence
    line = tap.line
    name = title
    text = body
  }

  func perform() async throws -> some IntentResult {
    let tap = WidgetTap(kind: kind, routine: routine, occurrence: occurrence, line: line)
    let now = Int64(Date().timeIntervalSince1970 * 1000)
    WidgetFiles.updateQueue { tap.queue(in: &$0, title: name, body: text, now: now) }
    return .result()
  }
}

/// A Session's box: it only shows Done, and a tap on it does nothing
/// (#209), rather than open the app.
struct NothingIntent: AppIntent {
  static let title: LocalizedStringResource = "intent.nothing"
  static let isDiscoverable = false

  func perform() async throws -> some IntentResult { .result() }
}
