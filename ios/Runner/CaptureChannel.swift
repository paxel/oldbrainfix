import Flutter
import Foundation
import UIKit

/// What a widget's or a control's tap opened (#242), kept until Dart
/// asks, as MainActivity.kt keeps it: the Unsorted list and a pick on
/// the `alarms` channel, a capture screen on the `capture` channel. A
/// tap while the app runs tells Dart at once; on a cold start Dart asks
/// as it starts. Used on the main thread only.
enum WidgetOpens {
  private static var unsorted = false
  private static var pick: [String: String]?

  /// Keeps what each link among `urls` opens; other URLs are left alone.
  static func open(_ urls: [URL]) {
    for url in urls {
      guard let link = WidgetLink(url: url) else { continue }
      switch link {
      case .agenda:
        break
      case .unsorted:
        unsorted = true
        AlarmsChannel.actionsArrived()
      case .pick(let session, let occurrence, let note):
        pick = ["session": session, "occurrence": occurrence, "note": note]
        AlarmsChannel.actionsArrived()
      case .capture(let mode):
        CaptureChannel.request(mode)
      }
    }
  }

  /// Whether the Unsorted count was tapped since Dart last asked.
  static func takeUnsorted() -> Bool {
    defer { unsorted = false }
    return unsorted
  }

  /// The pick tapped since Dart last asked, or nil.
  static func takePick() -> [String: String]? {
    defer { pick = nil }
    return pick
  }
}

/// The `capture` channel on the iPhone (#242), answering
/// lib/src/capture_requests.dart as MainActivity.kt does: `take` hands
/// over the capture mode a widget or control asked for and clears it,
/// and `requested` tells Dart one arrived while the app runs.
enum CaptureChannel {
  static let name = "io.github.paxel.oldbrainfix/capture"
  private static var channel: FlutterMethodChannel?
  private static var requested: String?

  static func register(messenger: FlutterBinaryMessenger) {
    let channel = FlutterMethodChannel(name: name, binaryMessenger: messenger)
    channel.setMethodCallHandler { call, result in
      switch call.method {
      case "take":
        result(requested)
        requested = nil
      default:
        result(FlutterMethodNotImplemented)
      }
    }
    self.channel = channel
  }

  /// A capture screen of `mode` was asked for.
  static func request(_ mode: String) {
    requested = mode
    channel?.invokeMethod("requested", arguments: nil)
  }
}

/// The `shortcuts` channel on the iPhone (#284), answering
/// lib/src/home_shortcuts.dart as MainActivity.kt does: `pinTally` puts
/// a Tally among the quick actions of the app icon, `take` hands over
/// the Tally whose quick action was tapped and clears it, `requested`
/// tells Dart one was while the app runs. `leave` does nothing: iOS lets
/// no app send itself to the Home Screen.
enum ShortcutsChannel {
  static let name = "io.github.paxel.oldbrainfix/shortcuts"
  private static var channel: FlutterMethodChannel?
  private static var requested: String?

  static func register(messenger: FlutterBinaryMessenger) {
    let channel = FlutterMethodChannel(name: name, binaryMessenger: messenger)
    channel.setMethodCallHandler { call, result in
      switch call.method {
      case "pinTally":
        let args = call.arguments as? [String: Any]
        guard let note = args?["note"] as? String, !note.isEmpty else {
          result(false)
          return
        }
        TallyQuickActions.add(note: note, label: args?["label"] as? String ?? "")
        result(true)
      case "take":
        result(requested)
        requested = nil
      case "leave":
        result(nil)
      default:
        result(FlutterMethodNotImplemented)
      }
    }
    self.channel = channel
  }

  /// The entry page of the Tally of `note` was asked for.
  static func request(_ note: String) {
    requested = note
    channel?.invokeMethod("requested", arguments: nil)
  }
}

/// A Tally's shortcut on the iPhone (#284): iOS lets no app put an icon
/// of its own on the Home Screen, so it is a quick action of the app
/// icon, its long press listing the Tallies added there, the newest
/// first, as many as iOS shows. A tap opens the Tally's entry page.
/// Used on the main thread only.
enum TallyQuickActions {
  static let type = "io.github.paxel.oldbrainfix.tally"

  /// How many iOS shows in the app icon's menu.
  static let limit = 4

  /// The Tallies of the quick actions once `note`, named `label`, is
  /// added to `tallies`: it first and once, then the others, at most
  /// `limit`.
  static func merged(
    _ tallies: [(note: String, label: String)], adding note: String, label: String
  ) -> [(note: String, label: String)] {
    let others = tallies.filter { $0.note != note }
    return Array(([(note: note, label: label)] + others).prefix(limit))
  }

  /// Puts the Tally of `note`, named `label`, into the app icon's menu.
  static func add(note: String, label: String) {
    let app = UIApplication.shared
    let items = app.shortcutItems ?? []
    let tallies = items.compactMap { item -> (note: String, label: String)? in
      guard item.type == type, let note = item.userInfo?["note"] as? String else { return nil }
      return (note: note, label: item.localizedTitle)
    }
    app.shortcutItems =
      merged(tallies, adding: note, label: label).map {
        UIApplicationShortcutItem(
          type: type, localizedTitle: $0.label, localizedSubtitle: nil,
          icon: UIApplicationShortcutIcon(systemImageName: "chart.bar"),
          userInfo: ["note": $0.note as NSString])
      } + items.filter { $0.type != type }
  }

  /// Asks Dart for the entry page of the Tally of `item`; false for a
  /// quick action of another kind.
  @discardableResult
  static func open(_ item: UIApplicationShortcutItem) -> Bool {
    guard item.type == type, let note = item.userInfo?["note"] as? String, !note.isEmpty
    else { return false }
    ShortcutsChannel.request(note)
    return true
  }
}
