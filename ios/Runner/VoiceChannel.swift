import AVFoundation
import Flutter
import Foundation

/// The voice channel on the iPhone (#239), answering the calls of
/// MainActivity.kt's with the same arguments and answers: the
/// microphone's permission, recording, and playing a recording back.
enum VoiceChannel {
  static let name = "io.github.paxel.oldbrainfix/voice"
  private static var channel: FlutterMethodChannel?

  static func register(messenger: FlutterBinaryMessenger) {
    let channel = FlutterMethodChannel(name: name, binaryMessenger: messenger)
    channel.setMethodCallHandler { call, result in handle(call, result) }
    self.channel = channel
  }

  private static func handle(_ call: FlutterMethodCall, _ result: @escaping FlutterResult) {
    let voice = Voice.shared
    switch call.method {
    case "granted":
      result(Voice.granted())
    case "request":
      Voice.request { result($0) }
    case "start":
      result(voice.start())
    case "stop":
      guard let file = voice.stop() else {
        result(nil)
        return
      }
      DispatchQueue.global(qos: .userInitiated).async {
        let bytes = try? Data(contentsOf: file)
        try? FileManager.default.removeItem(at: file)
        DispatchQueue.main.async {
          result(bytes.map { FlutterStandardTypedData(bytes: $0) })
        }
      }
    case "play":
      guard let bytes = (call.arguments as? FlutterStandardTypedData)?.data else {
        result(false)
        return
      }
      result(voice.play(bytes) { channel?.invokeMethod("played", arguments: nil) })
    case "stopPlaying":
      voice.stopPlaying()
      result(true)
    case "speed":
      voice.setSpeed((call.arguments as? NSNumber)?.floatValue ?? 1)
      result(true)
    default:
      result(FlutterMethodNotImplemented)
    }
  }
}

/// Voice Notes with the phone's own AVAudioRecorder and AVAudioPlayer,
/// the twin of Voice.kt. Nothing here asks the user but [request].
///
/// A recording is AAC in an MPEG-4 (.m4a) file, mono, at 16 kHz and about
/// 64 kbit/s, as Android records: half a megabyte a minute to keep and
/// sync, and already the sample rate and channel count Whisper listens
/// at. The desktop's WAV recordings play and transcribe here too; an Ogg
/// voice message shared in does neither, AVFoundation does not read Ogg.
final class Voice: NSObject, AVAudioPlayerDelegate {
  static let shared = Voice()

  private var recorder: AVAudioRecorder?
  private var player: AVAudioPlayer?
  private var ended: (() -> Void)?

  /// How fast recordings play: 1 is normal speed.
  private var speed: Float = 1

  /// Whisper's own rate: transcribing never resamples.
  static let sampleRate = 16_000
  static let channels = 1
  static let bitRate = 64_000

  private var folder: URL {
    let folder = FileManager.default.temporaryDirectory.appendingPathComponent("voice")
    try? FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
    return folder
  }

  /// Where the recording running is written until it is handed over.
  private var recordingFile: URL { folder.appendingPathComponent("recording.m4a") }

  /// Whether the user allowed the microphone. Never asks.
  static func granted() -> Bool {
    AVAudioApplication.shared.recordPermission == .granted
  }

  /// Asks the user for the microphone, once; [done] hears on the main
  /// thread whether it is allowed now.
  static func request(_ done: @escaping (Bool) -> Void) {
    if granted() { return done(true) }
    AVAudioApplication.requestRecordPermission { allowed in
      DispatchQueue.main.async { done(allowed) }
    }
  }

  /// The recorder's settings, as Voice.kt's; then the same without the
  /// bit rate, should the phone's encoder refuse it at 16 kHz.
  static let settings: [[String: Any]] = {
    let aac: [String: Any] = [
      AVFormatIDKey: kAudioFormatMPEG4AAC,
      AVSampleRateKey: Voice.sampleRate,
      AVNumberOfChannelsKey: Voice.channels,
    ]
    return [aac.merging([AVEncoderBitRateKey: Voice.bitRate]) { $1 }, aac]
  }()

  /// Starts recording from the microphone; whether it did.
  func start() -> Bool {
    stopPlaying()
    discard()
    guard Voice.granted() else { return false }
    let file = recordingFile
    try? FileManager.default.removeItem(at: file)
    let session = AVAudioSession.sharedInstance()
    do {
      try session.setCategory(.playAndRecord, mode: .default, options: [.defaultToSpeaker])
      try session.setActive(true)
    } catch {
      return false
    }
    for settings in Voice.settings {
      guard let recorder = try? AVAudioRecorder(url: file, settings: settings),
        recorder.prepareToRecord(), recorder.record()
      else { continue }
      self.recorder = recorder
      return true
    }
    // The microphone is busy elsewhere.
    try? FileManager.default.removeItem(at: file)
    deactivate()
    return false
  }

  /// Ends the recording; the file it was written to, or nil when none
  /// ran or nothing was written. AVAudioRecorder always ends a file
  /// cleanly, as MediaRecorder does when its stop works. The caller
  /// reads the file and deletes it.
  func stop() -> URL? {
    guard let recorder else { return nil }
    self.recorder = nil
    recorder.stop()
    deactivate()
    let file = recordingFile
    if let size = SpeechFiles.stamp(file)?.size, size > 0 { return file }
    try? FileManager.default.removeItem(at: file)
    return nil
  }

  /// Ends a recording without keeping it.
  private func discard() {
    if let file = stop() { try? FileManager.default.removeItem(at: file) }
  }

  /// Plays the recording [bytes], stopping whatever played before;
  /// whether it plays. [ended] runs on the main thread when it played to
  /// its end.
  func play(_ bytes: Data, ended: @escaping () -> Void) -> Bool {
    stopPlaying()
    do {
      let session = AVAudioSession.sharedInstance()
      try session.setCategory(.playback, mode: .spokenAudio)
      try session.setActive(true)
      let player = try AVAudioPlayer(data: bytes)
      player.delegate = self
      player.enableRate = true
      player.rate = speed
      guard player.prepareToPlay(), player.play() else { return false }
      self.player = player
      self.ended = ended
      return true
    } catch {
      return false
    }
  }

  /// Plays recordings at [speed], 1 being normal speed: the one playing
  /// at once, and every one after it.
  func setSpeed(_ speed: Float) {
    self.speed = speed
    player?.rate = speed
  }

  /// Stops playing, if anything plays.
  func stopPlaying() {
    player?.stop()
    player = nil
    ended = nil
  }

  func audioPlayerDidFinishPlaying(_ player: AVAudioPlayer, successfully flag: Bool) {
    guard self.player === player else { return }
    let ended = self.ended
    stopPlaying()
    deactivate()
    ended?()
  }

  /// Lets other apps' sound go on.
  private func deactivate() {
    try? AVAudioSession.sharedInstance().setActive(false, options: .notifyOthersOnDeactivation)
  }
}

/// The rule of Voice.kt's companion on the iPhone (#239), free of
/// AVFoundation: what kind of sound file a recording is.
enum VoiceFiles {
  /// The file extension of the recording [bytes]: `wav` for a WAV file
  /// (`RIFF`…`WAVE`), as the desktop records; `ogg` for an Ogg file
  /// (`OggS`); `mp3` for an MP3 file (an `ID3` tag or an MPEG audio
  /// frame); else `m4a`, as the phones record. Readers read the bytes,
  /// the name only keeps them from guessing wrong.
  static func fileExtension(_ bytes: Data) -> String {
    let bytes = [UInt8](bytes)
    func tag(_ at: Int) -> String {
      bytes.count < at + 4 ? "" : String(decoding: bytes[at..<at + 4], as: UTF8.self)
    }
    func byte(_ at: Int) -> Int { bytes.count <= at ? 0 : Int(bytes[at]) }
    if tag(0) == "RIFF" && tag(8) == "WAVE" { return "wav" }
    if tag(0) == "OggS" { return "ogg" }
    if tag(0).hasPrefix("ID3") { return "mp3" }
    // An MPEG audio frame's sync bits, with a layer set: ADTS AAC has the
    // same sync but no layer.
    if byte(0) == 0xFF && (byte(1) & 0xE0) == 0xE0 && (byte(1) & 0x06) != 0 { return "mp3" }
    return "m4a"
  }
}
