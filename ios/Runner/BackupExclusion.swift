import Foundation

/// Keeps the app's data out of the iPhone's own backup to iCloud or a
/// computer (#244, ADR 0010): the database, the photos and recordings
/// beside it and the alarm layer's state in Application Support, and the
/// Agenda widget's snapshot and the waiting shares in the App Group
/// container. Old Brain Fix has its own encrypted Backups; as Android's
/// backup holds only the encrypted Backup file, the iPhone's holds none
/// of it. A directory left out of backups leaves out everything in it,
/// also the files SQLite makes beside the database later.
enum BackupExclusion {
  /// Makes `directory` where it is missing and leaves it out of the
  /// device's backups. Throws when either cannot be done.
  static func exclude(_ directory: URL, _ files: FileManager = .default) throws {
    try files.createDirectory(at: directory, withIntermediateDirectories: true)
    var values = URLResourceValues()
    values.isExcludedFromBackup = true
    var url = directory
    try url.setResourceValues(values)
  }

  /// Whether `url` is left out of the device's backups.
  static func isExcluded(_ url: URL) -> Bool {
    var url = url
    url.removeAllCachedResourceValues()
    return (try? url.resourceValues(forKeys: [.isExcludedFromBackupKey]))?
      .isExcludedFromBackup ?? false
  }

  /// The folders that hold the app's data: its Application Support, and
  /// the App Group's folders once the container is there.
  static func appDataDirectories(_ files: FileManager = .default) -> [URL] {
    var directories = files.urls(for: .applicationSupportDirectory, in: .userDomainMask)
    directories.append(WidgetFiles.directory(files))
    if let inbox = ShareInbox.directory(files) { directories.append(inbox) }
    return directories
  }

  /// Leaves every folder of the app's data out of the device's backups,
  /// on every start, before the database opens: iOS keeps the mark, and
  /// setting it again costs nothing. One that fails is left as it is; the
  /// others go on.
  static func excludeAppData(_ files: FileManager = .default) {
    for directory in appDataDirectories(files) {
      try? exclude(directory, files)
    }
  }
}
