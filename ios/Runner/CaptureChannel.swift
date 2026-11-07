import Flutter
import Foundation

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
