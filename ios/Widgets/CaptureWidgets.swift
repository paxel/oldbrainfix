import AppIntents
import SwiftUI
import WidgetKit

/// The quick entry points into Capture on the iPhone (#242), as
/// android/…/CaptureEntry.kt's widgets and tiles: a small home-screen
/// widget with one button, the same widget on the lock screen, and a
/// Control Center control each for voice, text and photo. Each opens
/// the app straight into its capture screen through the app's own link
/// (`WidgetLink.capture`), as Android's ask with `capture_requests`.
enum CaptureMode: String, AppEnum {
  case voice
  case text
  case photo

  static let typeDisplayRepresentation: TypeDisplayRepresentation = "capture.mode"
  static let caseDisplayRepresentations: [CaptureMode: DisplayRepresentation] = [
    .voice: "capture.voice",
    .text: "capture.text",
    .photo: "capture.photo",
  ]

  var link: WidgetLink { .capture(rawValue) }

  var symbol: String {
    switch self {
    case .voice: return "mic.fill"
    case .text: return "square.and.pencil"
    case .photo: return "camera.fill"
    }
  }

  var name: LocalizedStringKey {
    switch self {
    case .voice: return "capture.voice"
    case .text: return "capture.text"
    case .photo: return "capture.photo"
    }
  }

  /// `name` where iOS takes a resource, as the controls' names.
  var resource: LocalizedStringResource {
    switch self {
    case .voice: return "capture.voice"
    case .text: return "capture.text"
    case .photo: return "capture.photo"
    }
  }

  var shortName: LocalizedStringKey {
    switch self {
    case .voice: return "capture.voiceShort"
    case .text: return "capture.textShort"
    case .photo: return "capture.photoShort"
    }
  }
}

/// Which capture screen a Capture widget opens, picked when it is edited;
/// a text note until then.
struct CaptureConfiguration: WidgetConfigurationIntent {
  static let title: LocalizedStringResource = "capture.name"
  static let description = IntentDescription("capture.description")

  @Parameter(title: "capture.mode", default: .text) var mode: CaptureMode

  init() {}

  init(mode: CaptureMode) { self.mode = mode }
}

struct CaptureEntry: TimelineEntry {
  let date: Date
  let mode: CaptureMode
}

/// Nothing on the widget changes but its mode.
struct CaptureProvider: AppIntentTimelineProvider {
  func placeholder(in context: Context) -> CaptureEntry { CaptureEntry(date: Date(), mode: .text) }

  func snapshot(for configuration: CaptureConfiguration, in context: Context) async -> CaptureEntry
  {
    CaptureEntry(date: Date(), mode: configuration.mode)
  }

  func timeline(for configuration: CaptureConfiguration, in context: Context) async -> Timeline<
    CaptureEntry
  > {
    Timeline(entries: [CaptureEntry(date: Date(), mode: configuration.mode)], policy: .never)
  }
}

struct CaptureWidget: Widget {
  static let kind = "io.github.paxel.oldbrainfix.capture"

  var body: some WidgetConfiguration {
    AppIntentConfiguration(
      kind: Self.kind, intent: CaptureConfiguration.self, provider: CaptureProvider()
    ) { entry in
      CaptureWidgetView(entry: entry)
    }
    .configurationDisplayName("capture.name")
    .description("capture.description")
    .supportedFamilies([.systemSmall, .accessoryCircular, .accessoryRectangular, .accessoryInline])
  }
}

struct CaptureWidgetView: View {
  let entry: CaptureEntry
  @Environment(\.widgetFamily) private var family

  var body: some View {
    content
      .containerBackground(for: .widget) { Color(uiColor: .systemBackground) }
      .widgetURL(entry.mode.link.url)
  }

  @ViewBuilder
  private var content: some View {
    switch family {
    case .accessoryCircular:
      ZStack {
        AccessoryWidgetBackground()
        Image(systemName: entry.mode.symbol).font(.title2)
      }
      .accessibilityLabel(Text(entry.mode.name))
    case .accessoryRectangular:
      Label(entry.mode.name, systemImage: entry.mode.symbol).font(.headline)
    case .accessoryInline:
      Label(entry.mode.name, systemImage: entry.mode.symbol)
    default:
      VStack(spacing: 8) {
        Image(systemName: entry.mode.symbol).font(.system(size: 44))
        Text(entry.mode.shortName).font(.headline)
      }
      .accessibilityElement(children: .ignore)
      .accessibilityLabel(Text(entry.mode.name))
    }
  }
}

/// Opens the app on a capture screen: the control's action, which iOS
/// runs and then opens the app at the link.
struct OpenCaptureIntent: AppIntent {
  static let title: LocalizedStringResource = "intent.capture"
  static let isDiscoverable = false

  @Parameter(title: "capture.mode") var mode: CaptureMode

  init() {}

  init(_ mode: CaptureMode) { self.mode = mode }

  func perform() async throws -> some IntentResult & OpensIntent {
    .result(opensIntent: OpenURLIntent(mode.link.url))
  }
}

/// A Control Center and lock-screen control for one capture mode.
struct CaptureControl {
  static func configuration(_ mode: CaptureMode) -> some ControlWidgetConfiguration {
    StaticControlConfiguration(kind: "io.github.paxel.oldbrainfix.capture.\(mode.rawValue)") {
      ControlWidgetButton(action: OpenCaptureIntent(mode)) {
        Label(mode.name, systemImage: mode.symbol)
      }
    }
    .displayName(mode.resource)
    .description("capture.description")
  }
}

struct VoiceCaptureControl: ControlWidget {
  var body: some ControlWidgetConfiguration { CaptureControl.configuration(.voice) }
}

struct TextCaptureControl: ControlWidget {
  var body: some ControlWidgetConfiguration { CaptureControl.configuration(.text) }
}

struct PhotoCaptureControl: ControlWidget {
  var body: some ControlWidgetConfiguration { CaptureControl.configuration(.photo) }
}
