import Flutter
import UIKit

class SceneDelegate: FlutterSceneDelegate {
  // A file of the app's own types opened with it (#241), on a cold
  // start: kept for Dart, as Android's BundleActivity keeps it.
  override func scene(
    _ scene: UIScene, willConnectTo session: UISceneSession,
    options connectionOptions: UIScene.ConnectionOptions
  ) {
    super.scene(scene, willConnectTo: session, options: connectionOptions)
    SharesChannel.open(connectionOptions.urlContexts.map(\.url))
  }

  // ... and while the app runs.
  override func scene(_ scene: UIScene, openURLContexts URLContexts: Set<UIOpenURLContext>) {
    super.scene(scene, openURLContexts: URLContexts)
    SharesChannel.open(URLContexts.map(\.url))
  }
}
