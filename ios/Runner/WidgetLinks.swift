import Foundation

/// Where a tap on a widget or a control opens the app (#242), as a URL
/// of the app's own scheme, which SceneDelegate hands to `WidgetOpens`:
/// the Agenda, the Unsorted list, a Session's pick's entry form (#209)
/// or a capture screen, the same requests Android's widgets and tiles
/// send (`capture_requests`). The app and the Widgets extension both
/// compile this file.
enum WidgetLink: Equatable {
  case agenda
  case unsorted
  case pick(session: String, occurrence: String, note: String)
  case capture(String)

  static let scheme = "oldbrainfix"

  /// The capture modes, as lib/src/capture_requests.dart names them.
  static let modes = ["voice", "text", "photo"]

  /// Every character but these is percent-encoded in a query value, so
  /// an id with `&`, `=` or `+` reads back as it was.
  private static let plain = CharacterSet(
    charactersIn: "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-._~")

  var url: URL {
    var parts = URLComponents()
    parts.scheme = Self.scheme
    switch self {
    case .agenda:
      parts.host = "agenda"
    case .unsorted:
      parts.host = "unsorted"
    case .pick(let session, let occurrence, let note):
      parts.host = "pick"
      parts.percentEncodedQueryItems = [
        ("session", session), ("occurrence", occurrence), ("note", note),
      ].map {
        URLQueryItem(
          name: $0.0, value: $0.1.addingPercentEncoding(withAllowedCharacters: Self.plain))
      }
    case .capture(let mode):
      parts.host = "capture"
      parts.path = "/\(mode)"
    }
    return parts.url ?? URL(fileURLWithPath: "/")
  }

  /// The link `url` is; nil for any other URL, such as a file opened
  /// with the app, or one that does not read.
  init?(url: URL) {
    guard url.scheme?.lowercased() == Self.scheme,
      let parts = URLComponents(url: url, resolvingAgainstBaseURL: false)
    else { return nil }
    switch parts.host {
    case "agenda":
      self = .agenda
    case "unsorted":
      self = .unsorted
    case "pick":
      let items = parts.queryItems ?? []
      let value = { (name: String) in items.first { $0.name == name }?.value ?? "" }
      let (session, occurrence, note) = (value("session"), value("occurrence"), value("note"))
      guard !session.isEmpty, !occurrence.isEmpty, !note.isEmpty else { return nil }
      self = .pick(session: session, occurrence: occurrence, note: note)
    case "capture":
      let mode = String(parts.path.drop { $0 == "/" })
      guard Self.modes.contains(mode) else { return nil }
      self = .capture(mode)
    default:
      return nil
    }
  }
}
