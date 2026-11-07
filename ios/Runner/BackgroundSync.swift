import BackgroundTasks
import Flutter
import Foundation

/// The background sync on the iPhone (#240), as SyncWork.kt and
/// SyncWorker.kt run it on Android: one app refresh task
/// ([BackgroundPlan.identifier]) asked for while some paired Device
/// syncs on its own, a shared folder is chosen or Backups are written
/// weekly. iOS runs it when it sees fit, for some seconds, or not at
/// all; nothing promises a time. When it runs while the app is in
/// memory, the app syncs itself ([SyncChannel.syncInApp]); otherwise a
/// Flutter engine without any screen runs `backgroundSyncMain`
/// (lib/main.dart), which joins the paired Devices once without hosting,
/// syncs through the shared folder, writes a Backup due and arms the
/// alarms anew, then says it finished.
enum BackgroundSync {
  private static let onKey = "sync.background.on"
  private static let folderKey = "sync.background.folder"
  private static var defaults: UserDefaults { .standard }

  /// The background engine while it runs.
  private static var running: HeadlessSync?

  /// Registers the task; before launching ends, as iOS requires.
  static func register() {
    _ = BGTaskScheduler.shared.register(forTaskWithIdentifier: BackgroundPlan.identifier, using: nil) {
      task in
      DispatchQueue.main.async {
        guard let refresh = task as? BGAppRefreshTask else {
          task.setTaskCompleted(success: false)
          return
        }
        run(refresh)
      }
    }
  }

  /// Turns the work on, keeping a run already asked for, or off; with a
  /// shared [folder] it runs without a local network too.
  static func schedule(on: Bool, folder: Bool) {
    defaults.set(on, forKey: onKey)
    defaults.set(on && folder, forKey: folderKey)
    ask()
  }

  private static func ask() {
    let on = defaults.bool(forKey: onKey)
    guard on else {
      BGTaskScheduler.shared.cancel(taskRequestWithIdentifier: BackgroundPlan.identifier)
      return
    }
    BGTaskScheduler.shared.getPendingTaskRequests { requests in
      let pending = requests.contains { $0.identifier == BackgroundPlan.identifier }
      guard BackgroundPlan.asks(on: on, pending: pending),
        let earliest = BackgroundPlan.next(on: on, now: Date())
      else { return }
      let request = BGAppRefreshTaskRequest(identifier: BackgroundPlan.identifier)
      request.earliestBeginDate = earliest
      // Refused in the simulator and with Background App Refresh off:
      // then the app syncs only while it is open.
      try? BGTaskScheduler.shared.submit(request)
    }
  }

  private static func run(_ task: BGAppRefreshTask) {
    // The next run first: this one is no longer pending.
    ask()
    var over = false
    let finish: (Bool) -> Void = { success in
      DispatchQueue.main.async {
        if over { return }
        over = true
        running?.stop()
        running = nil
        task.setTaskCompleted(success: success)
      }
    }
    task.expirationHandler = { finish(false) }
    LocalNetwork.shared.check { local in
      let folder = defaults.bool(forKey: folderKey)
      guard BackgroundPlan.runs(folder: folder, localNetwork: local) else {
        finish(true)
        return
      }
      if let app = SyncChannel.app {
        app.syncInApp { finish(true) }
        return
      }
      running?.stop()
      running = HeadlessSync.start { finish(true) }
      if running == nil { finish(false) }
    }
  }
}

/// A Flutter engine without any screen that runs `backgroundSyncMain`,
/// with the channels it calls: sync, alarms, folder and pairing, as
/// SyncWorker.kt sets them up. Gone again with [stop].
final class HeadlessSync {
  static let entrypoint = "backgroundSyncMain"

  private let engine: FlutterEngine
  private let sync: SyncChannel

  private init(engine: FlutterEngine, sync: SyncChannel) {
    self.engine = engine
    self.sync = sync
  }

  /// The engine started, or nil when it did not; [finished] hears when
  /// Dart says it is over.
  static func start(finished: @escaping () -> Void) -> HeadlessSync? {
    let engine = FlutterEngine(name: "background-sync", project: nil, allowHeadlessExecution: true)
    guard engine.run(withEntrypoint: entrypoint) else { return nil }
    GeneratedPluginRegistrant.register(with: engine)
    let messenger = engine.binaryMessenger
    let sync = SyncChannel(messenger: messenger, inApp: false)
    sync.onFinished = finished
    AlarmsChannel.register(messenger: messenger, background: true)
    FolderChannel.register(messenger: messenger, picking: false)
    PairingChannel.register(messenger: messenger)
    return HeadlessSync(engine: engine, sync: sync)
  }

  func stop() {
    sync.onFinished = nil
    sync.stopAll()
    engine.destroyContext()
  }
}
