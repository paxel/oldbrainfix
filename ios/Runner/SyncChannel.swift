import Flutter
import Foundation
import Network
import UIKit

/// The sync channel on the iPhone (#240), answering the calls of
/// MainActivity.kt and SyncWorker.kt with the same arguments and
/// answers: Bonjour for NsdManager, finding paired hosts (`start`,
/// `stop`, `discover`) and telling the network about this Device's own
/// (`advertise`, `unadvertise`); whether the iPhone is on Wi-Fi or
/// Ethernet; and BGTaskScheduler for WorkManager ([BackgroundSync]).
/// The iPhone hosts only while the app is open (#236): when it goes to
/// the background, the record leaves the network at once and the app
/// gets a few seconds to stop its host and send what changed.
final class SyncChannel {
  static let name = "io.github.paxel.oldbrainfix/sync"

  /// The app's own engine's, while it runs: a background task that comes
  /// due asks it to sync instead of starting an engine of its own.
  private(set) static var app: SyncChannel?

  /// How long the app keeps running once out of sight, at most, to stop
  /// its host and send what changed.
  static let leavingSeconds: TimeInterval = 10

  private let channel: FlutterMethodChannel
  private let inApp: Bool
  private var discovery: Discovery?
  private var looks: [Discovery] = []
  private let advertiser = Advertiser()
  private var leaving = UIBackgroundTaskIdentifier.invalid

  /// Who hears that the sync asked for is over: the background task
  /// waiting for the app, or the background engine's.
  var onFinished: (() -> Void)?

  static func registerApp(messenger: FlutterBinaryMessenger) {
    app = SyncChannel(messenger: messenger, inApp: true)
  }

  init(messenger: FlutterBinaryMessenger, inApp: Bool) {
    channel = FlutterMethodChannel(name: SyncChannel.name, binaryMessenger: messenger)
    self.inApp = inApp
    channel.setMethodCallHandler { [weak self] call, result in
      guard let self else {
        result(FlutterMethodNotImplemented)
        return
      }
      self.handle(call, result)
    }
    if inApp {
      _ = NotificationCenter.default.addObserver(
        forName: UIApplication.didEnterBackgroundNotification, object: nil, queue: .main
      ) { [weak self] _ in self?.left() }
    }
  }

  private func handle(_ call: FlutterMethodCall, _ result: @escaping FlutterResult) {
    let args = call.arguments as? [String: Any] ?? [:]
    switch call.method {
    case "start":
      if inApp { lookingAround().start() }
      result(nil)
    case "stop":
      discovery?.stop()
      result(nil)
    case "discover":
      let millis = (call.arguments as? NSNumber)?.intValue ?? 8_000
      discover(for: millis, result)
    case "background":
      BackgroundSync.schedule(on: args["on"] as? Bool == true, folder: args["folder"] as? Bool == true)
      result(nil)
    case "localNetwork":
      LocalNetwork.shared.check { result($0) }
    case "advertise":
      if let device = args["device"] as? String, let port = (args["port"] as? NSNumber)?.intValue {
        advertiser.start(device: device, port: port)
      }
      result(nil)
    case "unadvertise":
      advertiser.stop()
      result(nil)
    case "finished":
      result(nil)
      finished()
    default:
      result(FlutterMethodNotImplemented)
    }
  }

  private func lookingAround() -> Discovery {
    if let discovery { return discovery }
    let made = Discovery { [weak self] found in
      self?.channel.invokeMethod("found", arguments: found.channelMap)
    }
    discovery = made
    return made
  }

  /// Looks for paired hosts for [millis] and answers with what was found.
  private func discover(for millis: Int, _ result: @escaping FlutterResult) {
    var found: [String: FoundHost] = [:]
    let looking = Discovery { found[$0.device] = $0 }
    looks.append(looking)
    looking.start()
    DispatchQueue.main.asyncAfter(deadline: .now() + .milliseconds(millis)) { [weak self] in
      looking.stop()
      self?.looks.removeAll { $0 === looking }
      result(found.values.map(\.channelMap))
    }
  }

  /// The periodic work came due while the app runs: it syncs itself, with
  /// its own store, and [done] hears when it is over.
  func syncInApp(done: @escaping () -> Void) {
    // One at a time: an earlier one still waiting is over.
    finished()
    onFinished = done
    channel.invokeMethod("wanted", arguments: nil) { [weak self] answer in
      // No app listening yet: nothing syncs this time.
      if answer is FlutterError || (answer as AnyObject) === FlutterMethodNotImplemented {
        self?.finished()
      }
    }
  }

  private func finished() {
    let done = onFinished
    onFinished = nil
    done?()
  }

  /// The app went out of sight: no host is advertised from now on, and
  /// the app keeps running a little to stop it.
  private func left() {
    discovery?.stop()
    advertiser.stop()
    let application = UIApplication.shared
    if leaving == .invalid {
      leaving = application.beginBackgroundTask(withName: "Stop hosting") { [weak self] in
        self?.endLeaving()
      }
    }
    DispatchQueue.main.asyncAfter(deadline: .now() + SyncChannel.leavingSeconds) { [weak self] in
      self?.endLeaving()
    }
  }

  private func endLeaving() {
    guard leaving != .invalid else { return }
    UIApplication.shared.endBackgroundTask(leaving)
    leaving = .invalid
  }

  /// The background engine is going: nothing looks or is advertised.
  func stopAll() {
    discovery?.stop()
    for look in looks { look.stop() }
    looks = []
    advertiser.stop()
    channel.setMethodCallHandler(nil)
  }
}

/// Looks for paired hosts with Bonjour, as Discovery.kt does with
/// NsdManager: each record found is resolved, one at a time, and what it
/// names is handed to [onFound] on the main queue. The phone checks the
/// host's certificate against the one it pinned before it sends
/// anything, so a record only says where to try. NetService is the API
/// that resolves a record's addresses and TXT entries without opening a
/// connection; the Info.plist names the service type, without which
/// iOS finds nothing.
final class Discovery: NSObject, NetServiceBrowserDelegate, NetServiceDelegate {
  private let onFound: (FoundHost) -> Void
  private var browser: NetServiceBrowser?
  private var waiting: [NetService] = []
  private var resolving: NetService?

  init(onFound: @escaping (FoundHost) -> Void) {
    self.onFound = onFound
  }

  /// Starts looking, unless it already does.
  func start() {
    guard browser == nil else { return }
    let started = NetServiceBrowser()
    started.delegate = self
    browser = started
    started.searchForServices(ofType: "\(SyncRecords.serviceType).", inDomain: "local.")
  }

  /// Stops looking.
  func stop() {
    browser?.stop()
    browser?.delegate = nil
    browser = nil
    waiting = []
    resolving?.stop()
    resolving?.delegate = nil
    resolving = nil
  }

  func netServiceBrowser(
    _ browser: NetServiceBrowser, didFind service: NetService, moreComing: Bool
  ) {
    guard browser === self.browser, SyncRecords.isOurs(service.type) else { return }
    waiting.append(service)
    next()
  }

  func netServiceBrowser(_ browser: NetServiceBrowser, didNotSearch errorDict: [String: NSNumber]) {
    if browser === self.browser { self.browser = nil }
  }

  private func next() {
    guard resolving == nil, browser != nil, !waiting.isEmpty else { return }
    let service = waiting.removeFirst()
    resolving = service
    service.delegate = self
    service.resolve(withTimeout: 5)
  }

  func netServiceDidResolveAddress(_ sender: NetService) {
    guard sender === resolving else { return }
    let entries = sender.txtRecordData().map(SyncRecords.txtEntries) ?? [:]
    // An IPv6 address may come first; the IPv4 one is waited for.
    for address in sender.addresses ?? [] {
      if let host = SyncRecords.ipv4(sockaddr: address),
        let found = SyncRecords.found(entries, host: host, port: sender.port)
      {
        onFound(found)
        done(sender)
        return
      }
    }
  }

  func netService(_ sender: NetService, didNotResolve errorDict: [String: NSNumber]) {
    done(sender)
  }

  func netServiceDidStop(_ sender: NetService) {
    done(sender)
  }

  private func done(_ service: NetService) {
    guard service === resolving else { return }
    service.delegate = nil
    service.stop()
    resolving = nil
    next()
  }
}

/// Tells the network about this Device's paired host (ADR 0003), as
/// Advertiser.kt does: the service type `_oldbrainfix._tcp`, the device
/// id as the instance name and as the only TXT entry, on the port the
/// app's host listens on.
final class Advertiser: NSObject, NetServiceDelegate {
  private var service: NetService?
  private var device: String?
  private var port: Int?

  /// Advertises [device]'s host on [port], in place of what was before.
  func start(device: String, port: Int) {
    if service != nil, self.device == device, self.port == port { return }
    stop()
    guard let txt = SyncRecords.own(device: device, port: port) else { return }
    let record = NetService(
      domain: "local.", type: "\(SyncRecords.serviceType).", name: device, port: Int32(port))
    record.delegate = self
    _ = record.setTXTRecord(SyncRecords.txtRecord(txt))
    service = record
    self.device = device
    self.port = port
    record.publish()
  }

  /// Takes the record off the network.
  func stop() {
    service?.delegate = nil
    service?.stop()
    service = nil
    device = nil
    port = nil
  }

  func netService(_ sender: NetService, didNotPublish errorDict: [String: NSNumber]) {
    if sender === service { stop() }
  }
}

/// Whether the iPhone is on a local network now, Wi-Fi or Ethernet,
/// where paired Devices can be, as SyncWork.kt asks it; from one path
/// monitor for the whole run. Used on the main queue.
final class LocalNetwork {
  static let shared = LocalNetwork()

  private let monitor = NWPathMonitor()
  private var local: Bool?
  private var waiting: [(Bool) -> Void] = []

  private init() {
    monitor.pathUpdateHandler = { [weak self] path in
      let local =
        path.status == .satisfied
        && (path.usesInterfaceType(.wifi) || path.usesInterfaceType(.wiredEthernet))
      DispatchQueue.main.async { self?.update(local) }
    }
    monitor.start(queue: DispatchQueue(label: "io.github.paxel.oldbrainfix.network"))
  }

  /// [done] hears it; before the monitor's first answer, once it comes,
  /// or as off after two seconds without one.
  func check(_ done: @escaping (Bool) -> Void) {
    if let local {
      done(local)
      return
    }
    waiting.append(done)
    DispatchQueue.main.asyncAfter(deadline: .now() + 2) { [weak self] in self?.flush(false) }
  }

  private func update(_ local: Bool) {
    self.local = local
    flush(local)
  }

  private func flush(_ local: Bool) {
    let told = waiting
    waiting = []
    for done in told { done(local) }
  }
}
