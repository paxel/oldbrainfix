import AVFoundation
import Flutter
import Foundation
import Network
import UIKit
import UniformTypeIdentifiers

/// The speech channel on the iPhone (#239), answering the calls of
/// MainActivity.kt's with the same arguments and answers: the model's
/// SHA-256, its download from the URL the app hands in (the release
/// asset WhisperModel names) with its progress, a model picked from a
/// file, keeping or dropping it, and the Transcript of a recording.
enum SpeechChannel {
  static let name = "io.github.paxel.oldbrainfix/speech"
  private static var channel: FlutterMethodChannel?

  /// The file picker shown for "pick", until it answers.
  private static var picker: ModelPicker?

  static func register(messenger: FlutterBinaryMessenger) {
    let channel = FlutterMethodChannel(name: name, binaryMessenger: messenger)
    channel.setMethodCallHandler { call, result in handle(call, result) }
    self.channel = channel
    // Watching the network from the start, so onMobileData knows.
    _ = Speech.shared
  }

  private static func handle(_ call: FlutterMethodCall, _ result: @escaping FlutterResult) {
    let speech = Speech.shared
    switch call.method {
    case "supported":
      result(true)
    case "modelHash":
      inBackground(result) { speech.modelHash() }
    case "onMobileData":
      result(speech.onMobileData())
    case "download":
      guard let url = call.arguments as? String else {
        result(nil)
        return
      }
      speech.download(
        url,
        progress: { received, total in
          DispatchQueue.main.async {
            channel?.invokeMethod("progress", arguments: ["received": received, "total": total])
          }
        },
        done: { hash in DispatchQueue.main.async { result(hash) } })
    case "pick":
      pick(result)
    case "cancel":
      speech.cancel()
      result(nil)
    case "install":
      inBackground(result) { speech.install() }
    case "discard":
      inBackground(result) {
        speech.discard()
        return true
      }
    case "remove":
      inBackground(result) {
        speech.remove()
        return true
      }
    case "transcribe":
      let args = call.arguments as? [String: Any] ?? [:]
      guard let bytes = (args["bytes"] as? FlutterStandardTypedData)?.data else {
        result(nil)
        return
      }
      speech.transcribe(
        bytes, english: args["english"] as? String ?? "", german: args["german"] as? String ?? ""
      ) { text in DispatchQueue.main.async { result(text) } }
    case "idle":
      speech.idle()
      result(nil)
    default:
      result(FlutterMethodNotImplemented)
    }
  }

  /// Runs [work] off the main thread and answers with what it gives.
  private static func inBackground(_ result: @escaping FlutterResult, _ work: @escaping () -> Any?)
  {
    DispatchQueue.global(qos: .userInitiated).async {
      let answer = work()
      DispatchQueue.main.async { result(answer) }
    }
  }

  /// Lets the user pick a speech model file of their own with the
  /// system's file picker and copies it to the side; answers its
  /// SHA-256, or nil when none was picked.
  private static func pick(_ result: @escaping FlutterResult) {
    picker?.answer(nil)
    guard let top = topViewController() else {
      result(nil)
      return
    }
    let picker = ModelPicker { url in
      SpeechChannel.picker = nil
      guard let url else {
        result(nil)
        return
      }
      inBackground(result) {
        let hash = Speech.shared.takeIn(url)
        try? FileManager.default.removeItem(at: url)
        return hash
      }
    }
    self.picker = picker
    top.present(picker.controller, animated: true)
  }

  private static func topViewController() -> UIViewController? {
    let scenes = UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }
    var top = scenes.lazy.compactMap { $0.keyWindow?.rootViewController }.first
    while let presented = top?.presentedViewController { top = presented }
    return top
  }
}

/// The system's file picker for a model file, copied out of where it
/// lies, so no access needs keeping; [answer] hears the copy or nil.
private final class ModelPicker: NSObject, UIDocumentPickerDelegate {
  let controller: UIDocumentPickerViewController
  private var done: ((URL?) -> Void)?

  init(done: @escaping (URL?) -> Void) {
    controller = UIDocumentPickerViewController(forOpeningContentTypes: [.item], asCopy: true)
    self.done = done
    super.init()
    controller.delegate = self
    controller.allowsMultipleSelection = false
  }

  /// Answers once; later answers are dropped.
  func answer(_ url: URL?) {
    let done = self.done
    self.done = nil
    done?(url)
  }

  func documentPicker(_ controller: UIDocumentPickerViewController, didPickDocumentsAt urls: [URL])
  {
    answer(urls.first)
  }

  func documentPickerWasCancelled(_ controller: UIDocumentPickerViewController) {
    answer(nil)
  }
}

/// Transcripts for voice Notes (ADR 0007) on the iPhone: whisper.cpp
/// (third_party/whisper.cpp with the desktop's C bridge, linked in) with
/// a model the user fetched on request, from Old Brain Fix's own GitHub
/// release or from a file of their own. The twin of Speech.kt: it only
/// moves the bytes and says what their SHA-256 is; the app's Dart side
/// checks it against the checksums it pins.
///
/// The model lives in Application Support/whisper, left out of the
/// phone's backups: it has no place there, and it can always be fetched
/// again. Its SHA-256 is recorded with its size and modification time
/// when it is kept, so a start reads the whole file again only when it
/// changed.
final class Speech {
  static let shared = Speech()

  /// The one queue transcribing: recordings go one at a time.
  private let worker = DispatchQueue(label: "io.github.paxel.oldbrainfix.speech")

  /// The model loaded on [worker], until [idle]; nil while none is.
  private var loadedModel: OpaquePointer?

  private let lock = NSLock()
  private var cancelledNow = false
  /// The SHA-256 of the model on the side, until it is kept or dropped.
  private var incomingHashNow: String?
  private var downloading: ModelDownload?

  private let monitor = NWPathMonitor()
  private var pathNow: NWPath?

  let dir: URL

  init(dir: URL? = nil) {
    self.dir =
      dir
      ?? (FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
        ?? FileManager.default.temporaryDirectory).appendingPathComponent("whisper")
    monitor.pathUpdateHandler = { [weak self] path in self?.locked { self?.pathNow = path } }
    monitor.start(queue: DispatchQueue(label: "io.github.paxel.oldbrainfix.speech.network"))
  }

  /// The folder of the model, made and left out of backups.
  private var folder: URL {
    var folder = dir
    try? FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
    var values = URLResourceValues()
    values.isExcludedFromBackup = true
    try? folder.setResourceValues(values)
    return folder
  }

  /// The model in use, once it was checked and kept: base or small,
  /// whichever was chosen last. A new one replaces it in one rename.
  var modelFile: URL { folder.appendingPathComponent(SpeechFiles.modelName) }

  /// A model on its way in: downloaded or picked, not yet checked.
  var incomingFile: URL { folder.appendingPathComponent(SpeechFiles.modelName + ".part") }

  /// The [modelFile]'s SHA-256 with its size and modification time then.
  var recordFile: URL { folder.appendingPathComponent(SpeechFiles.modelName + ".checked") }

  @discardableResult
  private func locked<T>(_ body: () -> T) -> T {
    lock.lock()
    defer { lock.unlock() }
    return body()
  }

  private var cancelled: Bool {
    get { locked { cancelledNow } }
    set { locked { cancelledNow = newValue } }
  }

  private var incomingHash: String? {
    get { locked { incomingHashNow } }
    set { locked { incomingHashNow = newValue } }
  }

  /// Whether the phone is online through mobile data now.
  func onMobileData() -> Bool {
    guard let path = locked({ pathNow }) else { return false }
    return path.status == .satisfied && path.usesInterfaceType(.cellular)
  }

  /// The SHA-256 of the model in use, or nil when there is none: as
  /// recorded, unless the file changed since.
  func modelHash() -> String? {
    SpeechFiles.checkedHash(model: modelFile, record: recordFile)
  }

  /// Downloads [url] to the side; [done] hears its SHA-256, or nil when
  /// it failed or was cancelled. Only HTTPS, and only a 200 answer.
  /// [progress] hears the bytes so far and the whole size, -1 while
  /// unknown.
  func download(
    _ url: String, progress: @escaping (Int64, Int64) -> Void,
    done: @escaping (String?) -> Void
  ) {
    cancelled = false
    let target = incomingFile
    try? FileManager.default.removeItem(at: target)
    guard let remote = URL(string: url), remote.scheme == "https" else {
      done(nil)
      return
    }
    let download = ModelDownload(
      target: target, stop: { [weak self] in self?.cancelled ?? true }, progress: progress
    ) { [weak self] hash in
      guard let self else { return done(nil) }
      self.locked {
        self.incomingHashNow = hash
        self.downloading = nil
      }
      if hash == nil { try? FileManager.default.removeItem(at: target) }
      done(hash)
    }
    locked { downloading = download }
    download.start(remote)
  }

  /// Copies a model the user picked, at [source], to the side; its
  /// SHA-256, or nil.
  func takeIn(_ source: URL) -> String? {
    cancelled = false
    let target = incomingFile
    try? FileManager.default.removeItem(at: target)
    let hash: String? = {
      guard let input = try? FileHandle(forReadingFrom: source),
        FileManager.default.createFile(atPath: target.path, contents: nil),
        let output = try? FileHandle(forWritingTo: target)
      else { return nil }
      defer {
        try? input.close()
        try? output.close()
      }
      return SpeechFiles.copyHashing(input, to: output, stop: { self.cancelled })
    }()
    incomingHash = hash
    if hash == nil { try? FileManager.default.removeItem(at: target) }
    return hash
  }

  /// Stops a download or copy running.
  func cancel() {
    let download: ModelDownload? = locked {
      cancelledNow = true
      return downloading
    }
    download?.cancel()
  }

  /// Keeps the model waiting on the side as the one in use, and records
  /// the SHA-256 it was fetched with.
  func install() -> Bool {
    let incoming = incomingFile
    guard FileManager.default.fileExists(atPath: incoming.path) else { return false }
    worker.sync { closeModel() }
    let hash = incomingHash
    incomingHash = nil
    return SpeechFiles.keep(incoming: incoming, model: modelFile, record: recordFile, hash: hash)
  }

  /// Drops the model waiting on the side.
  func discard() {
    incomingHash = nil
    try? FileManager.default.removeItem(at: incomingFile)
  }

  /// Drops the model in use.
  func remove() {
    worker.sync { closeModel() }
    try? FileManager.default.removeItem(at: recordFile)
    try? FileManager.default.removeItem(at: modelFile)
  }

  /// The Transcript of the voice recording [bytes], on the transcribing
  /// queue, Whisper led in with the [english] or [german] prompt: [done]
  /// hears its text, or nil when it could not be read or there is no
  /// model.
  func transcribe(
    _ bytes: Data, english: String, german: String, done: @escaping (String?) -> Void
  ) {
    worker.async { done(self.transcribeNow(bytes, english: english, german: german)) }
  }

  /// Nothing more to transcribe for now: the model's memory goes.
  func idle() {
    worker.async { self.closeModel() }
  }

  private func transcribeNow(_ bytes: Data, english: String, german: String) -> String? {
    let model = modelFile
    guard FileManager.default.fileExists(atPath: model.path) else { return nil }
    let file = FileManager.default.temporaryDirectory.appendingPathComponent(
      "transcribing.\(VoiceFiles.fileExtension(bytes))")
    guard (try? bytes.write(to: file)) != nil else { return nil }
    let samples = Speech.decode(file)
    try? FileManager.default.removeItem(at: file)
    guard let samples, !samples.isEmpty, samples.count <= Int(Int32.max) else { return nil }
    if loadedModel == nil { loadedModel = obf_whisper_open(model.path) }
    guard let context = loadedModel else { return nil }
    let threads = Int32(SpeechFiles.threads(cores: ProcessInfo.processInfo.activeProcessorCount))
    let text: String? = samples.withUnsafeBufferPointer { pcm in
      english.withCString { english in
        german.withCString { german in
          guard
            let out = obf_whisper_transcribe(
              context, pcm.baseAddress, Int32(pcm.count), threads, english, german)
          else { return nil }
          defer { obf_whisper_free_text(out) }
          return String(cString: out)
        }
      }
    }
    return text?.trimmingCharacters(in: .whitespacesAndNewlines)
  }

  private func closeModel() {
    if let loadedModel { obf_whisper_close(loadedModel) }
    loadedModel = nil
  }

  /// The recording in [url] as Whisper hears it: 16 kHz mono samples
  /// from -1 to 1, decoded by the phone's own AVFoundation; nil when it
  /// cannot be read (an Ogg file, which AVFoundation does not read, is
  /// one).
  static func decode(_ url: URL) -> [Float]? {
    guard let file = try? AVAudioFile(forReading: url) else { return nil }
    let format = file.processingFormat
    guard file.length > 0, file.length <= Int64(UInt32.max),
      let buffer = AVAudioPCMBuffer(
        pcmFormat: format, frameCapacity: AVAudioFrameCount(file.length))
    else { return nil }
    do {
      try file.read(into: buffer)
    } catch {
      return nil
    }
    guard let data = buffer.floatChannelData else { return nil }
    let frames = Int(buffer.frameLength)
    let channels = (0..<Int(format.channelCount)).map {
      Array(UnsafeBufferPointer(start: data[$0], count: frames))
    }
    return SpeechFiles.toWhisper(channels, rate: Int(format.sampleRate))
  }
}

/// One model download with URLSession: to the system's temporary file,
/// then moved to [target] and hashed. Kept by its session until it ends.
private final class ModelDownload: NSObject, URLSessionDownloadDelegate {
  private let target: URL
  private let stop: () -> Bool
  private let progress: (Int64, Int64) -> Void
  private var done: ((String?) -> Void)?
  private var told: Int64 = 0
  private var task: URLSessionDownloadTask?

  init(
    target: URL, stop: @escaping () -> Bool, progress: @escaping (Int64, Int64) -> Void,
    done: @escaping (String?) -> Void
  ) {
    self.target = target
    self.stop = stop
    self.progress = progress
    self.done = done
  }

  func start(_ url: URL) {
    let configuration = URLSessionConfiguration.ephemeral
    configuration.timeoutIntervalForRequest = 30
    let queue = OperationQueue()
    queue.maxConcurrentOperationCount = 1
    let session = URLSession(configuration: configuration, delegate: self, delegateQueue: queue)
    let task = session.downloadTask(with: url)
    self.task = task
    task.resume()
    session.finishTasksAndInvalidate()
  }

  func cancel() {
    task?.cancel()
  }

  /// Answers once.
  private func finish(_ hash: String?) {
    let done = self.done
    self.done = nil
    done?(hash)
  }

  func urlSession(
    _ session: URLSession, downloadTask: URLSessionDownloadTask, didWriteData bytesWritten: Int64,
    totalBytesWritten: Int64, totalBytesExpectedToWrite: Int64
  ) {
    if stop() {
      downloadTask.cancel()
      return
    }
    // About every megabyte, as on Android.
    if totalBytesWritten - told >= 1 << 20 {
      told = totalBytesWritten
      progress(totalBytesWritten, totalBytesExpectedToWrite > 0 ? totalBytesExpectedToWrite : -1)
    }
  }

  func urlSession(
    _ session: URLSession, downloadTask: URLSessionDownloadTask,
    didFinishDownloadingTo location: URL
  ) {
    guard (downloadTask.response as? HTTPURLResponse)?.statusCode == 200, !stop() else {
      return finish(nil)
    }
    // The file at [location] is gone once this returns.
    try? FileManager.default.removeItem(at: target)
    guard (try? FileManager.default.moveItem(at: location, to: target)) != nil else {
      return finish(nil)
    }
    let expected = downloadTask.countOfBytesExpectedToReceive
    let hash = SpeechFiles.hash(target, stop: stop)
    let size = SpeechFiles.stamp(target)?.size ?? 0
    progress(size, expected > 0 ? expected : -1)
    finish(hash)
  }

  func urlSession(_ session: URLSession, task: URLSessionTask, didCompleteWithError error: Error?) {
    // After didFinishDownloadingTo when it worked, which answered then.
    finish(nil)
  }
}
