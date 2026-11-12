import AlarmKit
import Flutter
import SwiftUI
import UIKit
import UserNotifications
import WidgetKit

/// The alarm channel on the iPhone (#238), answering the calls of
/// android/…/AlarmChannel.kt and MainActivity.kt with the same
/// arguments and answers: the plan armed, the taps queued meanwhile,
/// the permissions and what was tapped open. Nothing rings as a
/// full-screen alarm here; AlarmKit's own alarm screen takes its place.
/// The Agenda widget's calls (#242) keep its snapshot in the App Group
/// and answer what a tap on it opened. The sound picker is not there;
/// it answers not implemented, which the app takes as none.
enum AlarmsChannel {
  static let name = "io.github.paxel.oldbrainfix/alarms"
  private static var channel: FlutterMethodChannel?

  /// The app's engine's channel, or the background sync's (#240): there
  /// nothing was tapped to open, since there is no screen, and the app's
  /// channel stays the one told of taps.
  @discardableResult
  static func register(messenger: FlutterBinaryMessenger, background: Bool = false)
    -> FlutterMethodChannel
  {
    let channel = FlutterMethodChannel(name: name, binaryMessenger: messenger)
    channel.setMethodCallHandler { call, result in
      if background && ["nudgeOpened", "changesOpened", "unsortedOpened"].contains(call.method) {
        result(false)
        return
      }
      if background && call.method == "pickOpened" {
        result(nil)
        return
      }
      handle(call, result)
    }
    if !background { self.channel = channel }
    return channel
  }

  /// Tells the app a tap was queued, so it applies it at once.
  static func actionsArrived() {
    channel?.invokeMethod("actions", arguments: nil)
  }

  private static func handle(_ call: FlutterMethodCall, _ result: @escaping FlutterResult) {
    let scheduler = AlarmScheduler.shared
    switch call.method {
    case "arm":
      scheduler.arm(call.arguments as? [String: Any] ?? [:])
      result(nil)
    case "pending":
      result(scheduler.queue.pending.map { $0.channelMap })
    case "ack":
      if let seq = PlanCodec.int64(call.arguments) { scheduler.ack(seq) }
      result(nil)
    case "ringing":
      result(nil)
    case "answerRing":
      let args = call.arguments as? [String: Any] ?? [:]
      let minutes = PlanCodec.int64(args["snooze"]).map { Int($0) }
      scheduler.answer(
        minutes == nil ? .done : .snooze(minutes: minutes ?? AlarmPlan.alarmSnoozeMinutes),
        routine: args["routine"] as? String ?? "",
        occurrence: args["occurrence"] as? String ?? "",
        title: args["title"] as? String ?? "",
        body: args["body"] as? String ?? "")
      result(nil)
    case "permissions":
      AlarmPermissions.granted { result($0) }
    case "request":
      AlarmPermissions.request(call.arguments as? String ?? "") { result(nil) }
    case "nudgeOpened":
      result(scheduler.nudgeOpened)
      scheduler.nudgeOpened = false
    case "changesOpened":
      result(scheduler.changesOpened)
      scheduler.changesOpened = false
    case "agenda":
      WidgetFiles.writeAgenda(call.arguments as? [String: Any] ?? [:])
      WidgetCenter.shared.reloadAllTimelines()
      result(nil)
    case "unsortedOpened":
      result(WidgetOpens.takeUnsorted())
    case "pickOpened":
      result(WidgetOpens.takePick())
    default:
      result(FlutterMethodNotImplemented)
    }
  }
}

/// The access alarms need on the iPhone, in the shape the setup reads:
/// notifications, and AlarmKit's alarms for Escalation.
enum AlarmPermissions {
  static func granted(_ done: @escaping ([String: Bool]) -> Void) {
    UNUserNotificationCenter.current().getNotificationSettings { settings in
      let allowed: [UNAuthorizationStatus] = [.authorized, .provisional, .ephemeral]
      let notifications = allowed.contains(settings.authorizationStatus)
      DispatchQueue.main.async {
        done([
          "notifications": notifications,
          "alarms": AlarmManager.shared.authorizationState == .authorized,
        ])
      }
    }
  }

  /// The system dialog while iOS still shows it, else the app's page in
  /// the Settings app; `done` once the dialog is answered or the page
  /// asked for.
  static func request(_ permission: String, _ done: @escaping () -> Void) {
    switch permission {
    case "notifications":
      let center = UNUserNotificationCenter.current()
      center.getNotificationSettings { settings in
        if settings.authorizationStatus == .notDetermined {
          center.requestAuthorization(options: [.alert, .sound, .badge]) { _, _ in
            DispatchQueue.main.async {
              AlarmScheduler.shared.replan()
              done()
            }
          }
        } else {
          DispatchQueue.main.async {
            open(UIApplication.openNotificationSettingsURLString)
            done()
          }
        }
      }
    case "alarms":
      if AlarmManager.shared.authorizationState == .notDetermined {
        Task {
          _ = try? await AlarmManager.shared.requestAuthorization()
          DispatchQueue.main.async {
            AlarmScheduler.shared.replan()
            done()
          }
        }
      } else {
        open(UIApplication.openSettingsURLString)
        done()
      }
    default:
      done()
    }
  }

  private static func open(_ address: String) {
    guard let url = URL(string: address) else { return }
    UIApplication.shared.open(url)
  }
}

/// Taps on the app's notifications, also while it is not running: iOS
/// starts it in the background, and the tap is queued and the plan
/// armed anew without the app's engine.
final class AlarmNotifications: NSObject, UNUserNotificationCenterDelegate {
  static let shared = AlarmNotifications()

  func userNotificationCenter(
    _ center: UNUserNotificationCenter,
    willPresent notification: UNNotification,
    withCompletionHandler completionHandler: @escaping (UNNotificationPresentationOptions) -> Void
  ) {
    completionHandler([.banner, .list, .sound])
  }

  func userNotificationCenter(
    _ center: UNUserNotificationCenter,
    didReceive response: UNNotificationResponse,
    withCompletionHandler completionHandler: @escaping () -> Void
  ) {
    let content = response.notification.request.content
    let action =
      response.actionIdentifier == UNNotificationDefaultActionIdentifier
      ? NotificationAnswer.defaultAction : response.actionIdentifier
    let answer = NotificationAnswer.of(category: content.categoryIdentifier, action: action)
    let info = content.userInfo
    DispatchQueue.main.async {
      let task = AlarmScheduler.shared.answer(
        answer,
        routine: info["routine"] as? String ?? "",
        occurrence: info["occurrence"] as? String ?? "",
        title: info["title"] as? String ?? "",
        body: info["body"] as? String ?? "")
      Task {
        await task.value
        completionHandler()
      }
    }
  }
}

/// Hands the alarm plan to iOS. The whole plan is kept, and the next
/// `AlarmPlan.windowSize` rings of it are handed over: escalated ones as
/// AlarmKit alarms where allowed, the others as time-sensitive
/// notifications, the Nudges as normal ones. Every plan, tap and answer
/// hands them over anew. Used on the main thread only.
final class AlarmScheduler {
  static let shared = AlarmScheduler()

  /// Whether the Nudge's or the Changes Nudge's notification was tapped
  /// since the app last asked.
  var nudgeOpened = false
  var changesOpened = false

  private var tail: Task<Void, Never>?

  private static let planFile = "plan.json"
  /// The AlarmKit alarms handed over, by id, with their Occurrence.
  private static let alarmsFile = "alarmkit.json"

  // MARK: Kept on disk

  /// The taps not applied yet, in the App Group, where the Agenda
  /// widget's boxes queue theirs too (#242).
  var queue: ActionQueue { WidgetFiles.readQueue() }

  /// Keeps the plan "arm" handed over and hands it to iOS. The
  /// notifications of what is `done` go away, and so do the Nudge's
  /// once nothing is Unsorted and the Changes Nudge's once nobody waits.
  func arm(_ args: [String: Any]) {
    AlarmFiles.write(Self.planFile, args)
    removeDelivered(keys: Set(PlanCodec.done(args["done"])))
    if PlanCodec.nudge(args["nudge"]) == nil {
      removeDelivered(category: NotificationAnswer.nudgeCategory)
    }
    if PlanCodec.nudge(args["changes"]) == nil {
      removeDelivered(category: NotificationAnswer.changesCategory)
    }
    replan()
  }

  func ack(_ seq: Int64) {
    WidgetFiles.updateQueue { $0.ack(seq) }
    WidgetCenter.shared.reloadTimelines(ofKind: WidgetFiles.agendaKind)
    replan()
  }

  /// A tap on a notification or an alarm, or an answer from the app:
  /// queued, its notifications and alarm gone, the plan handed over anew.
  @discardableResult
  func answer(
    _ answer: NotificationAnswer, routine: String, occurrence: String, title: String,
    body: String
  ) -> Task<Void, Never> {
    let now = Int64(Date().timeIntervalSince1970 * 1000)
    let key = "\(routine)@\(occurrence)"
    switch answer {
    case .done:
      WidgetFiles.updateQueue {
        $0.add(
          kind: QueuedAction.done, routine: routine, occurrence: occurrence, title: title,
          body: body, now: now)
      }
    case .snooze(let minutes):
      WidgetFiles.updateQueue {
        $0.add(
          kind: QueuedAction.snooze, routine: routine, occurrence: occurrence, title: title,
          body: body, until: now + Int64(minutes) * 60_000, now: now)
      }
    case .skip, .tomorrow:
      WidgetFiles.updateQueue {
        $0.add(
          kind: answer == .skip ? QueuedAction.skip : QueuedAction.tomorrow, routine: routine,
          occurrence: occurrence, title: title, body: body, now: now)
      }
    case .nudgeSnooze(let minutes):
      WidgetFiles.updateQueue {
        $0.add(
          kind: QueuedAction.nudgeSnooze, routine: "", occurrence: "", title: "", body: "",
          until: now + Int64(minutes) * 60_000, now: now)
      }
    case .openNudge:
      nudgeOpened = true
    case .openChanges:
      changesOpened = true
    case .open:
      return replan()
    }
    // A Done shows checked on the Agenda widget at once, as on Android.
    WidgetCenter.shared.reloadTimelines(ofKind: WidgetFiles.agendaKind)
    switch answer {
    case .done, .snooze, .skip, .tomorrow:
      removeDelivered(keys: [key])
      stopAlarms(of: key)
    case .nudgeSnooze:
      removeDelivered(category: NotificationAnswer.nudgeCategory)
    default:
      break
    }
    AlarmsChannel.actionsArrived()
    return replan()
  }

  /// Stop or Snooze on an AlarmKit alarm, from its intent.
  static func answer(
    kind: String, routine: String, occurrence: String, title: String, body: String,
    snoozeMinutes: Int?
  ) async {
    let task = await MainActor.run { () -> Task<Void, Never> in
      AlarmScheduler.shared.answer(
        snoozeMinutes.map { NotificationAnswer.snooze(minutes: $0) } ?? NotificationAnswer.done,
        routine: routine, occurrence: occurrence, title: title, body: body)
    }
    await task.value
  }

  // MARK: Handing over

  /// Hands the plan to iOS after the hand-overs before it, one at a time.
  @discardableResult
  func replan() -> Task<Void, Never> {
    let before = tail
    let task = Task {
      await before?.value
      await self.handOver()
    }
    tail = task
    return task
  }

  private func handOver() async {
    let stored = AlarmFiles.read(Self.planFile) as? [String: Any] ?? [:]
    let queued = queue.pending
    let plan = AlarmPlan.effective(PlanCodec.alarms(stored["alarms"]), queued: queued)
    let nudge = PlanCodec.nudge(stored["nudge"]).map { AlarmPlan.effective($0, queued: queued) }
    let changes = PlanCodec.nudge(stored["changes"])
    let labels = stored["labels"] as? [String: Any] ?? [:]
    let now = Int64(Date().timeIntervalSince1970 * 1000)
    let rings = AlarmPlan.window(
      alarms: plan, nudge: nudge, changes: changes, zone: TimeZone.current, now: now)
    let alarmKit = AlarmManager.shared.authorizationState == .authorized

    let center = UNUserNotificationCenter.current()
    center.setNotificationCategories(categories(labels, changes: changes))
    center.removeAllPendingNotificationRequests()
    for ring in rings where !(alarmKit && ring.kind == .escalated) {
      if let request = request(ring, nudge: nudge, changes: changes) {
        center.add(request, withCompletionHandler: nil)
      }
    }
    await handOverAlarms(
      alarmKit ? rings.filter { $0.kind == .escalated } : [], plan: plan, labels: labels)
  }

  /// Replaces the AlarmKit alarms by those of `rings`; one ringing now
  /// rings on while its Occurrence still escalates and is not answered.
  private func handOverAlarms(_ rings: [Ring], plan: [PlannedAlarm], labels: [String: Any]) async
  {
    let manager = AlarmManager.shared
    let known = AlarmFiles.read(Self.alarmsFile) as? [String: String] ?? [:]
    let escalating = Set(plan.filter { $0.escalate > 0 }.map { $0.key })
    var kept: [String: String] = [:]
    for alarm in (try? manager.alarms) ?? [] {
      let id = alarm.id.uuidString
      if alarm.state == .alerting, let key = known[id], escalating.contains(key) {
        kept[id] = key
        continue
      }
      try? manager.cancel(id: alarm.id)
    }
    for ring in rings {
      guard let alarm = ring.alarm else { continue }
      let id = UUID()
      do {
        _ = try await manager.schedule(
          id: id, configuration: configuration(alarm, at: ring.at, labels: labels))
        kept[id.uuidString] = alarm.key
      } catch {
        continue
      }
    }
    AlarmFiles.write(Self.alarmsFile, kept)
  }

  /// Stops and takes away the AlarmKit alarms of the Occurrence `key`.
  private func stopAlarms(of key: String) {
    let known = AlarmFiles.read(Self.alarmsFile) as? [String: String] ?? [:]
    for (id, owner) in known where owner == key {
      guard let uuid = UUID(uuidString: id) else { continue }
      try? AlarmManager.shared.stop(id: uuid)
      try? AlarmManager.shared.cancel(id: uuid)
    }
  }

  private func configuration(_ alarm: PlannedAlarm, at: Int64, labels: [String: Any])
    -> AlarmManager.AlarmConfiguration<OccurrenceMetadata>
  {
    let title = LocalizedStringResource(stringLiteral: alarm.title)
    let snooze = AlarmButton(
      text: LocalizedStringResource(
        stringLiteral: Self.snoozeTitle(labels, minutes: AlarmPlan.alarmSnoozeMinutes)),
      textColor: .white,
      systemImageName: "zzz")
    let alert: AlarmPresentation.Alert
    if #available(iOS 26.1, *) {
      // The system's own Stop, which answers Done.
      alert = AlarmPresentation.Alert(
        title: title, secondaryButton: snooze, secondaryButtonBehavior: .custom)
    } else {
      alert = AlarmPresentation.Alert(
        title: title,
        stopButton: AlarmButton(
          text: LocalizedStringResource(stringLiteral: Self.label(labels, "done")),
          textColor: .white,
          systemImageName: "checkmark"),
        secondaryButton: snooze,
        secondaryButtonBehavior: .custom)
    }
    let attributes = AlarmAttributes<OccurrenceMetadata>(
      presentation: AlarmPresentation(alert: alert),
      metadata: OccurrenceMetadata(routine: alarm.routine, occurrence: alarm.occurrence),
      tintColor: .orange)
    return .alarm(
      schedule: .fixed(Date(timeIntervalSince1970: Double(at) / 1000)),
      attributes: attributes,
      stopIntent: DoneOccurrenceIntent(alarm),
      secondaryIntent: SnoozeOccurrenceIntent(alarm))
  }

  // MARK: Notifications

  private func categories(_ labels: [String: Any], changes: PlannedNudge?)
    -> Set<UNNotificationCategory>
  {
    let snoozes = AlarmPlan.snoozeMinutes
    // All at once, not Snooze and Not today and then their choices
    // (#269): Done, the three snoozes, Skip and Tomorrow.
    func alarmActions(tomorrow: Bool) -> [UNNotificationAction] {
      [
        UNNotificationAction(
          identifier: NotificationAnswer.doneAction, title: Self.label(labels, "done"),
          options: [])
      ]
        + snoozes.map {
          UNNotificationAction(
            identifier: NotificationAnswer.snoozePrefix + String($0),
            title: Self.snoozeTitle(labels, minutes: $0), options: [])
        }
        + [
          UNNotificationAction(
            identifier: NotificationAnswer.skipAction, title: Self.label(labels, "skip"),
            options: [])
        ]
        + (tomorrow
          ? [
            UNNotificationAction(
              identifier: NotificationAnswer.tomorrowAction,
              title: Self.label(labels, "tomorrow"), options: [])
          ] : [])
    }
    let alarm = UNNotificationCategory(
      identifier: NotificationAnswer.alarmCategory,
      actions: alarmActions(tomorrow: true),
      intentIdentifiers: [],
      options: [])
    // A Re-nag after midnight of an earlier day's Occurrence: no Tomorrow.
    let pastAlarm = UNNotificationCategory(
      identifier: NotificationAnswer.pastAlarmCategory,
      actions: alarmActions(tomorrow: false),
      intentIdentifiers: [],
      options: [])
    let nudge = UNNotificationCategory(
      identifier: NotificationAnswer.nudgeCategory,
      actions: snoozes.map {
        UNNotificationAction(
          identifier: NotificationAnswer.nudgeSnoozePrefix + String($0),
          title: Self.snoozeTitle(labels, minutes: $0), options: [])
      },
      intentIdentifiers: [],
      options: [])
    // The names stay off the lock screen.
    let changesCategory = UNNotificationCategory(
      identifier: NotificationAnswer.changesCategory,
      actions: [],
      intentIdentifiers: [],
      hiddenPreviewsBodyPlaceholder: changes?.publicText ?? "",
      options: [])
    return [alarm, pastAlarm, nudge, changesCategory]
  }

  private func request(_ ring: Ring, nudge: PlannedNudge?, changes: PlannedNudge?)
    -> UNNotificationRequest?
  {
    let content = UNMutableNotificationContent()
    content.sound = .default
    switch ring.kind {
    case .alarm, .escalated:
      guard let alarm = ring.alarm else { return nil }
      content.title = alarm.title
      content.body = alarm.body
      content.categoryIdentifier =
        AlarmPlan.offersTomorrow(alarm.occurrence, today: Self.dayText(ring.at))
        ? NotificationAnswer.alarmCategory : NotificationAnswer.pastAlarmCategory
      content.threadIdentifier = alarm.key
      content.interruptionLevel = .timeSensitive
      content.userInfo = [
        "routine": alarm.routine, "occurrence": alarm.occurrence, "title": alarm.title,
        "body": alarm.body,
      ]
    case .nudge:
      guard let nudge = nudge else { return nil }
      content.title = nudge.title
      content.body = nudge.body
      content.categoryIdentifier = NotificationAnswer.nudgeCategory
      content.threadIdentifier = NotificationAnswer.nudgeCategory
    case .changes:
      guard let changes = changes else { return nil }
      content.title = changes.title
      content.body = changes.body
      content.categoryIdentifier = NotificationAnswer.changesCategory
      content.threadIdentifier = NotificationAnswer.changesCategory
    }
    let date = Date(timeIntervalSince1970: Double(ring.at) / 1000)
    // On the wall clock, so it keeps its local time in a new time zone.
    let parts = Calendar.current.dateComponents(
      [.year, .month, .day, .hour, .minute, .second], from: date)
    return UNNotificationRequest(
      identifier: AlarmPlan.requestId(ring),
      content: content,
      trigger: UNCalendarNotificationTrigger(dateMatching: parts, repeats: false))
  }

  /// The day `at` (epoch millis) falls on, on this phone's clock, as an
  /// Occurrence's key names it ("2026-10-24").
  private static func dayText(_ at: Int64) -> String {
    let parts = Calendar.current.dateComponents(
      [.year, .month, .day], from: Date(timeIntervalSince1970: Double(at) / 1000))
    return String(format: "%04d-%02d-%02d", parts.year ?? 0, parts.month ?? 0, parts.day ?? 0)
  }

  /// Takes away the delivered notifications of the Occurrences `keys`.
  private func removeDelivered(keys: Set<String>) {
    if keys.isEmpty { return }
    let center = UNUserNotificationCenter.current()
    center.getDeliveredNotifications { delivered in
      let ids = delivered.filter { note in
        let info = note.request.content.userInfo
        let key = "\(info["routine"] as? String ?? "")@\(info["occurrence"] as? String ?? "")"
        return keys.contains(key)
      }.map { $0.request.identifier }
      center.removeDeliveredNotifications(withIdentifiers: ids)
    }
  }

  /// Takes away the delivered notifications of `category`.
  private func removeDelivered(category: String) {
    let center = UNUserNotificationCenter.current()
    center.getDeliveredNotifications { delivered in
      let ids = delivered.filter { $0.request.content.categoryIdentifier == category }
        .map { $0.request.identifier }
      center.removeDeliveredNotifications(withIdentifiers: ids)
    }
  }

  /// The text `name` of the labels the app handed over; `name` itself
  /// without one, as on Android.
  static func label(_ labels: [String: Any], _ name: String) -> String {
    labels[name] as? String ?? name
  }

  /// "Snooze 10 min": iOS shows the actions as one list, so each names
  /// what it does.
  static func snoozeTitle(_ labels: [String: Any], minutes: Int) -> String {
    let choice = label(labels, minutes == 10 ? "snooze10" : minutes == 30 ? "snooze30" : "snooze60")
    return "\(label(labels, "snooze")) \(choice)"
  }
}

/// The alarm layer's files in the app's Application Support, readable
/// in the background after the first unlock.
enum AlarmFiles {
  private static var directory: URL {
    let base =
      FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
      ?? FileManager.default.temporaryDirectory
    return base.appendingPathComponent("alarms", isDirectory: true)
  }

  static func read(_ name: String) -> Any? {
    guard let data = try? Data(contentsOf: directory.appendingPathComponent(name)) else {
      return nil
    }
    return try? JSONSerialization.jsonObject(with: data)
  }

  static func write(_ name: String, _ value: Any) {
    guard JSONSerialization.isValidJSONObject(value),
      let data = try? JSONSerialization.data(withJSONObject: value)
    else { return }
    try? FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    try? data.write(
      to: directory.appendingPathComponent(name),
      options: [.atomic, .completeFileProtectionUntilFirstUserAuthentication])
  }
}
