import AVFoundation
import CoreLocation
import Flutter
import Foundation

/// The Capture Context on the iPhone (#241), answering MainActivity.kt's
/// `context` and `location` calls with what iOS lets an app know: the
/// calendar events running now (EventKit, full access) and the
/// Bluetooth audio device the sound goes to as the connected device;
/// where the phone is while the app is in use. No source app: iOS never
/// says which app shared. The time is Dart's own.
enum ContextChannel {
  static let contextName = "io.github.paxel.oldbrainfix/context"
  static let locationName = "io.github.paxel.oldbrainfix/location"

  static func register(messenger: FlutterBinaryMessenger) {
    let context = FlutterMethodChannel(name: contextName, binaryMessenger: messenger)
    context.setMethodCallHandler { call, result in
      let kind = call.arguments as? String
      switch call.method {
      case "granted":
        result(kind == "calendar" ? Calendars.granted : kind == "bluetooth")
      case "request":
        switch kind ?? "" {
        case "calendar": Calendars.shared.request { result($0) }
        // The audio route needs no permission.
        case "bluetooth": result(true)
        default: result(false)
        }
      case "calendar":
        Calendars.shared.run(result) { $0.running() }
      case "bluetooth":
        let route = AVAudioSession.sharedInstance().currentRoute
        let ports = (route.outputs + route.inputs).map { (type: $0.portType, name: $0.portName) }
        result(RouteNames.pick(ports))
      default:
        result(FlutterMethodNotImplemented)
      }
    }
    let location = FlutterMethodChannel(name: locationName, binaryMessenger: messenger)
    location.setMethodCallHandler { call, result in
      let whereabouts = Whereabouts.shared
      switch call.method {
      case "granted": result(whereabouts.granted)
      case "request": whereabouts.request { result($0) }
      case "current": whereabouts.current { result($0) }
      default: result(FlutterMethodNotImplemented)
      }
    }
  }
}

/// The names of the devices in the audio route that count as connected
/// devices, as Android lists its Bluetooth audio devices: Bluetooth
/// headphones, speakers and car kits, and a car's CarPlay; trimmed, not
/// blank, each once. The phone's own speaker, a wired headset or AirPlay
/// are not.
enum RouteNames {
  static let connected: Set<AVAudioSession.Port> = [
    .bluetoothA2DP, .bluetoothHFP, .bluetoothLE, .carAudio,
  ]

  static func pick(_ ports: [(type: AVAudioSession.Port, name: String)]) -> [String] {
    var names: [String] = []
    for port in ports where connected.contains(port.type) {
      let name = port.name.trimmingCharacters(in: .whitespacesAndNewlines)
      if !name.isEmpty && !names.contains(name) { names.append(name) }
    }
    return names
  }
}

/// Where the phone is, from Core Location while the app is in use, as
/// Location.kt answers on Android: a fix younger than two minutes is
/// taken as it is, else one is asked for. Lives on the main thread,
/// where Core Location calls back.
final class Whereabouts: NSObject, CLLocationManagerDelegate {
  static let shared = Whereabouts()

  /// A fix younger than this is taken as the current one.
  private static let fresh: TimeInterval = 2 * 60

  private let manager = CLLocationManager()
  private var asking: [(Bool) -> Void] = []
  private var locating: [([Double]?) -> Void] = []

  override init() {
    super.init()
    manager.delegate = self
    manager.desiredAccuracy = kCLLocationAccuracyNearestTenMeters
  }

  var granted: Bool {
    switch manager.authorizationStatus {
    case .authorizedWhenInUse, .authorizedAlways: true
    default: false
    }
  }

  /// Asks while iOS still asks; answers whether it is allowed now.
  func request(_ done: @escaping (Bool) -> Void) {
    guard manager.authorizationStatus == .notDetermined else {
      done(granted)
      return
    }
    asking.append(done)
    manager.requestWhenInUseAuthorization()
  }

  /// [latitude, longitude], or nil when that is not allowed or not
  /// known. Never asks.
  func current(_ done: @escaping ([Double]?) -> Void) {
    guard granted else {
      done(nil)
      return
    }
    if let fix = manager.location, -fix.timestamp.timeIntervalSinceNow < Self.fresh {
      done([fix.coordinate.latitude, fix.coordinate.longitude])
      return
    }
    locating.append(done)
    if locating.count == 1 { manager.requestLocation() }
  }

  // Also called once as the manager is made: only a decided status
  // answers the ones asking.
  func locationManagerDidChangeAuthorization(_ manager: CLLocationManager) {
    guard manager.authorizationStatus != .notDetermined, !asking.isEmpty else { return }
    let waiting = asking
    asking = []
    for done in waiting { done(granted) }
  }

  func locationManager(_ manager: CLLocationManager, didUpdateLocations locations: [CLLocation]) {
    answer(locations.last.map { [$0.coordinate.latitude, $0.coordinate.longitude] })
  }

  func locationManager(_ manager: CLLocationManager, didFailWithError error: Error) {
    answer(nil)
  }

  private func answer(_ fix: [Double]?) {
    let waiting = locating
    locating = []
    for done in waiting { done(fix) }
  }
}
