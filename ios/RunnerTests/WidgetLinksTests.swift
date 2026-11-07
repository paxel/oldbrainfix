import XCTest

@testable import Runner

/// The links a widget or a control opens the app with (#242): each reads
/// back as it was made, ids of any characters included, and nothing
/// else is taken for one.
class WidgetLinksTests: XCTestCase {
  func testEveryLinkReadsBackAsItWasMade() {
    let links: [WidgetLink] = [
      .agenda, .unsorted,
      .pick(session: "s-1", occurrence: "2026-10-24T18:00", note: "n-2"),
      .capture("voice"), .capture("text"), .capture("photo"),
    ]
    for link in links {
      XCTAssertEqual(WidgetLink(url: link.url), link, link.url.absoluteString)
      XCTAssertEqual(link.url.scheme, "oldbrainfix")
    }
    XCTAssertEqual(WidgetLink.capture("voice").url.absoluteString, "oldbrainfix://capture/voice")
    XCTAssertEqual(WidgetLink.agenda.url.absoluteString, "oldbrainfix://agenda")
  }

  func testIdsWithAnyCharacterSurvive() {
    let pick = WidgetLink.pick(session: "a&b=c", occurrence: "2026-10-24T08:00+x", note: "ä ö/?#")
    XCTAssertEqual(WidgetLink(url: pick.url), pick)
  }

  func testNothingElseIsALink() throws {
    let urls = [
      "https://example.org/agenda", "file:///tmp/a.obfpack", "oldbrainfix://capture/video",
      "oldbrainfix://capture", "oldbrainfix://pick?session=s&occurrence=o",
      "oldbrainfix://settings",
    ]
    for text in urls {
      XCTAssertNil(WidgetLink(url: try XCTUnwrap(URL(string: text))), text)
    }
    XCTAssertEqual(
      WidgetLink(url: try XCTUnwrap(URL(string: "OldBrainFix://unsorted"))), .unsorted)
  }

  /// iOS opens the app with the links only for a scheme it declares,
  /// and Flutter must not push them as routes.
  func testTheAppDeclaresItsSchemeAndKeepsFlutterFromRouting() {
    let types = Bundle.main.object(forInfoDictionaryKey: "CFBundleURLTypes") as? [[String: Any]] ?? []
    XCTAssertTrue(
      types.contains { ($0["CFBundleURLSchemes"] as? [String] ?? []).contains(WidgetLink.scheme) })
    XCTAssertEqual(Bundle.main.object(forInfoDictionaryKey: "FlutterDeepLinkingEnabled") as? Bool, false)
  }

  /// The Widgets extension rides inside the app with the app's version.
  func testTheWidgetsExtensionIsEmbedded() throws {
    let plugIns = try XCTUnwrap(Bundle.main.builtInPlugInsURL)
    let widgets = try XCTUnwrap(Bundle(url: plugIns.appendingPathComponent("Widgets.appex")))
    XCTAssertEqual(widgets.bundleIdentifier, "io.github.paxel.oldbrainfix.Widgets")
    XCTAssertEqual(
      widgets.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String,
      Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String)
    let point = (widgets.object(forInfoDictionaryKey: "NSExtension") as? [String: Any])?[
      "NSExtensionPointIdentifier"] as? String
    XCTAssertEqual(point, "com.apple.widgetkit-extension")
  }

  func testTheCaptureModesAreThoseOfCaptureRequests() {
    // lib/src/capture_requests.dart's CaptureMode names.
    XCTAssertEqual(Set(WidgetLink.modes), ["text", "voice", "photo"])
  }
}
