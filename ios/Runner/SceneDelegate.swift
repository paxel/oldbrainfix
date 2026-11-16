import Flutter
import UIKit

class SceneDelegate: FlutterSceneDelegate {
  // A file of the app's own types opened with it (#241), or a widget's
  // or control's link (#242), on a cold start: kept for Dart, as
  // Android's BundleActivity and MainActivity keep them.
  override func scene(
    _ scene: UIScene, willConnectTo session: UISceneSession,
    options connectionOptions: UIScene.ConnectionOptions
  ) {
    super.scene(scene, willConnectTo: session, options: connectionOptions)
    SharesChannel.open(connectionOptions.urlContexts.map(\.url))
    WidgetOpens.open(connectionOptions.urlContexts.map(\.url))
    // A Tally's quick action on the app icon that started the app (#284).
    if let item = connectionOptions.shortcutItem { TallyQuickActions.open(item) }
  }

  // ... and while the app runs.
  override func scene(_ scene: UIScene, openURLContexts URLContexts: Set<UIOpenURLContext>) {
    super.scene(scene, openURLContexts: URLContexts)
    SharesChannel.open(URLContexts.map(\.url))
    WidgetOpens.open(URLContexts.map(\.url))
  }

  // A Tally's quick action on the app icon while the app runs (#284).
  override func windowScene(
    _ windowScene: UIWindowScene, performActionFor shortcutItem: UIApplicationShortcutItem,
    completionHandler: @escaping (Bool) -> Void
  ) {
    if TallyQuickActions.open(shortcutItem) {
      completionHandler(true)
      return
    }
    super.windowScene(
      windowScene, performActionFor: shortcutItem, completionHandler: completionHandler)
  }
}
