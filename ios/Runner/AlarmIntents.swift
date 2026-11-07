import AlarmKit
import AppIntents
import Foundation

/// The Occurrence an AlarmKit alarm rings for.
struct OccurrenceMetadata: AlarmMetadata {
  var routine: String
  var occurrence: String
}

/// Stop on an escalated Occurrence's AlarmKit alarm: Done, queued for
/// the app like a tap on its notification. It runs in the app's process
/// without opening it.
struct DoneOccurrenceIntent: LiveActivityIntent {
  static let title: LocalizedStringResource = "Done"

  @Parameter(title: "Routine") var routine: String
  @Parameter(title: "Occurrence") var occurrence: String
  @Parameter(title: "Title") var name: String
  @Parameter(title: "Text") var text: String

  init() {}

  init(_ alarm: PlannedAlarm) {
    routine = alarm.routine
    occurrence = alarm.occurrence
    name = alarm.title
    text = alarm.body
  }

  func perform() async throws -> some IntentResult {
    await AlarmScheduler.answer(
      kind: QueuedAction.done, routine: routine, occurrence: occurrence, title: name, body: text,
      snoozeMinutes: nil)
    return .result()
  }
}

/// Snooze on an escalated Occurrence's AlarmKit alarm: it rings again
/// after `AlarmPlan.alarmSnoozeMinutes`, queued for the app like a
/// snooze tapped on its notification.
struct SnoozeOccurrenceIntent: LiveActivityIntent {
  static let title: LocalizedStringResource = "Snooze"

  @Parameter(title: "Routine") var routine: String
  @Parameter(title: "Occurrence") var occurrence: String
  @Parameter(title: "Title") var name: String
  @Parameter(title: "Text") var text: String

  init() {}

  init(_ alarm: PlannedAlarm) {
    routine = alarm.routine
    occurrence = alarm.occurrence
    name = alarm.title
    text = alarm.body
  }

  func perform() async throws -> some IntentResult {
    await AlarmScheduler.answer(
      kind: QueuedAction.snooze, routine: routine, occurrence: occurrence, title: name, body: text,
      snoozeMinutes: AlarmPlan.alarmSnoozeMinutes)
    return .result()
  }
}
