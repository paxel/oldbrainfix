import Flutter
import UIKit
import UserNotifications

@main
@objc class AppDelegate: FlutterAppDelegate, FlutterImplicitEngineDelegate {
  override func application(
    _ application: UIApplication,
    didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?
  ) -> Bool {
    // Before launching ends, so a tap that started the app in the
    // background reaches it (#238).
    UNUserNotificationCenter.current().delegate = AlarmNotifications.shared
    return super.application(application, didFinishLaunchingWithOptions: launchOptions)
  }

  func didInitializeImplicitFlutterEngine(_ engineBridge: FlutterImplicitEngineBridge) {
    GeneratedPluginRegistrant.register(with: engineBridge.pluginRegistry)
    let messenger = engineBridge.applicationRegistrar.messenger()
    AlarmsChannel.register(messenger: messenger)
    OcrChannel.register(messenger: messenger)
    SpeechChannel.register(messenger: messenger)
    VoiceChannel.register(messenger: messenger)
  }
}
