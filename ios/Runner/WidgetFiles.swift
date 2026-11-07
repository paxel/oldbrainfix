import Foundation

/// What the app and its Widgets extension (#242) share in the App Group
/// container, in both targets: the queue of taps not applied yet, which
/// the widget's boxes add to and the app reads, and the Agenda widget's
/// snapshot, which the app writes and the widget reads. No Flutter and
/// no UIApplication here: the extension compiles this file too.
///
/// Both processes change the queue, so every read and write of it goes
/// through an NSFileCoordinator on its file. Without the container, as
/// in an unsigned build or a test, both files live in the process's own
/// Application Support, where only the app reaches them.
enum WidgetFiles {
  /// The App Group the app and its extensions are entitled to; the same
  /// as `ShareInbox.group`.
  static let group = "group.io.github.paxel.oldbrainfix"

  /// The Agenda widget's kind, which the app asks WidgetKit to reload.
  static let agendaKind = "io.github.paxel.oldbrainfix.agenda"

  /// The folder both files live in.
  static func directory(_ files: FileManager = .default) -> URL {
    if let shared = files.containerURL(forSecurityApplicationGroupIdentifier: group) {
      return shared.appendingPathComponent("alarms", isDirectory: true)
    }
    return ownDirectory(files)
  }

  /// Where the app kept the queue before #242, in its own Application
  /// Support.
  static func ownDirectory(_ files: FileManager = .default) -> URL {
    let base =
      files.urls(for: .applicationSupportDirectory, in: .userDomainMask).first
      ?? files.temporaryDirectory
    return base.appendingPathComponent("alarms", isDirectory: true)
  }

  static let queueName = "queue.json"
  static let agendaName = "agenda.json"

  // MARK: The queue

  /// The taps not applied yet, oldest first.
  static func readQueue(in directory: URL? = nil) -> ActionQueue {
    let url = queueURL(directory)
    var queue = ActionQueue()
    var error: NSError?
    NSFileCoordinator().coordinate(readingItemAt: url, options: [], error: &error) { url in
      queue = decodeQueue((try? Data(contentsOf: url)) ?? Data())
    }
    return queue
  }

  /// Changes the queue as `change` does, read and written as one step,
  /// so a tap in the widget and an acknowledgement in the app never
  /// write over each other. Returns what `change` returned.
  @discardableResult
  static func updateQueue<T>(
    in directory: URL? = nil, _ change: (inout ActionQueue) -> T
  ) -> T? {
    let url = queueURL(directory)
    var answer: T?
    var error: NSError?
    NSFileCoordinator().coordinate(writingItemAt: url, options: .forMerging, error: &error) {
      url in
      var queue = decodeQueue((try? Data(contentsOf: url)) ?? Data())
      answer = change(&queue)
      if let data = encodeQueue(queue) { write(data, to: url) }
    }
    return answer
  }

  /// The queue as stored: `{"actions": [...], "seq": n}`; empty for
  /// nothing or what does not read.
  static func decodeQueue(_ data: Data) -> ActionQueue {
    guard let stored = (try? JSONSerialization.jsonObject(with: data)) as? [String: Any] else {
      return ActionQueue()
    }
    return ActionQueue(
      stored: stored["actions"] as? [[String: Any]] ?? [],
      seq: PlanCodec.int64(stored["seq"]) ?? 0)
  }

  static func encodeQueue(_ queue: ActionQueue) -> Data? {
    let value: [String: Any] = ["actions": queue.stored, "seq": NSNumber(value: queue.seq)]
    guard JSONSerialization.isValidJSONObject(value) else { return nil }
    return try? JSONSerialization.data(withJSONObject: value)
  }

  /// The queue's file in `directory`, or in the shared one by default,
  /// where the queue the app kept in its own Application Support before
  /// #242 moves once, so no tap is lost.
  private static func queueURL(_ directory: URL?) -> URL {
    let files = FileManager.default
    let folder = directory ?? WidgetFiles.directory()
    try? files.createDirectory(at: folder, withIntermediateDirectories: true)
    let url = folder.appendingPathComponent(queueName)
    if directory == nil {
      let own = ownDirectory().appendingPathComponent(queueName)
      if own != url, files.fileExists(atPath: own.path), !files.fileExists(atPath: url.path) {
        try? files.moveItem(at: own, to: url)
      }
    }
    return url
  }

  // MARK: The Agenda widget's snapshot

  /// Keeps the snapshot the app handed over (lib/src/agenda_widget.dart).
  @discardableResult
  static func writeAgenda(_ snapshot: Any, in directory: URL = WidgetFiles.directory()) -> Bool {
    guard JSONSerialization.isValidJSONObject(snapshot),
      let data = try? JSONSerialization.data(withJSONObject: snapshot)
    else { return false }
    try? FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    let url = directory.appendingPathComponent(agendaName)
    var error: NSError?
    var written = false
    NSFileCoordinator().coordinate(writingItemAt: url, options: .forReplacing, error: &error) {
      url in written = write(data, to: url)
    }
    return written
  }

  /// The snapshot the app handed over last, or nil before the first.
  static func readAgenda(in directory: URL = WidgetFiles.directory()) -> Data? {
    let url = directory.appendingPathComponent(agendaName)
    var data: Data?
    var error: NSError?
    NSFileCoordinator().coordinate(readingItemAt: url, options: [], error: &error) { url in
      data = try? Data(contentsOf: url)
    }
    return data
  }

  /// Readable in the background after the first unlock, as the widget
  /// and the app's notification taps need.
  @discardableResult
  private static func write(_ data: Data, to url: URL) -> Bool {
    (try? data.write(to: url, options: [.atomic, .completeFileProtectionUntilFirstUserAuthentication]))
      != nil
  }
}
