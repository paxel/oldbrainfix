import Flutter
import UIKit
import UserNotifications

@main
@objc class AppDelegate: FlutterAppDelegate, FlutterImplicitEngineDelegate {
  override func application(
    _ application: UIApplication,
    didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?
  ) -> Bool {
    // Before the database opens: the app's data stays out of the
    // iPhone's own backups (#244).
    BackupExclusion.excludeAppData()
    // Before launching ends, so a tap that started the app in the
    // background reaches it (#238).
    UNUserNotificationCenter.current().delegate = AlarmNotifications.shared
    // Registered before launching ends, as BGTaskScheduler requires (#240).
    BackgroundSync.register()
    return super.application(application, didFinishLaunchingWithOptions: launchOptions)
  }

  func didInitializeImplicitFlutterEngine(_ engineBridge: FlutterImplicitEngineBridge) {
    GeneratedPluginRegistrant.register(with: engineBridge.pluginRegistry)
    let messenger = engineBridge.applicationRegistrar.messenger()
    AlarmsChannel.register(messenger: messenger)
    OcrChannel.register(messenger: messenger)
    SpeechChannel.register(messenger: messenger)
    VoiceChannel.register(messenger: messenger)
    FolderChannel.register(messenger: messenger, picking: true)
    PairingChannel.register(messenger: messenger)
    SyncChannel.registerApp(messenger: messenger)
    ContextChannel.register(messenger: messenger)
    CalendarChannel.register(messenger: messenger)
    SharesChannel.register(messenger: messenger)
    CaptureChannel.register(messenger: messenger)
    LockChannel.register(messenger: messenger)
    DeviceChannel.register(messenger: messenger)
  }
}
