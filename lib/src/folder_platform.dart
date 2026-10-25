import 'dart:io';

import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// The platform seam for the shared folder (android/…/FolderChannel.kt):
/// picking it, its name for people, and the folder itself; the Backup
/// folder, and the directory Auto Backup takes the newest Backup from
/// (android/…/AutoBackup.kt). Widget tests hand in a fake.
abstract interface class FolderPlatform {
  /// Whether a folder can be picked here.
  bool get available;

  /// Opens the system's folder picker; the folder as it is kept, or null
  /// when the user backed out.
  Future<String?> pick();

  /// The folder's own name, for the screen.
  Future<String> displayName(String folder);

  /// The [folder] kept by [pick], as the sync reads and writes it.
  SyncFolder open(String folder);

  /// The [folder] kept by [pick], as Backups go into it (ADR 0016).
  BackupFolder openBackups(String folder);

  /// The app's own directory the platform's backup (Android's Auto
  /// Backup to the user's Google account, ADR 0010) takes the newest
  /// Backup from, and where one lies after a restore from there; null
  /// where there is none.
  Future<String?> autoBackupDir();

  /// Tells the platform's backup whether the user opted in; until then
  /// it takes nothing at all.
  Future<void> setAutoBackup(bool on);
}

/// The directory of [folders]'s Auto Backup as Backups go into it; null
/// where there is none.
Future<BackupFolder?> openAutoBackup(FolderPlatform folders) async =>
    switch (await folders.autoBackupDir()) {
      final dir? => LocalBackupFolder(dir),
      null => null,
    };

/// No shared folder: the default where none is handed in.
class NoFolders implements FolderPlatform {
  const NoFolders();

  @override
  bool get available => false;

  @override
  Future<String?> pick() async => null;

  @override
  Future<String> displayName(String folder) async => folder;

  @override
  SyncFolder open(String folder) => LocalSyncFolder(folder);

  @override
  BackupFolder openBackups(String folder) => LocalBackupFolder(folder);

  @override
  Future<String?> autoBackupDir() async => null;

  @override
  Future<void> setAutoBackup(bool on) async {}
}

/// Android's folder picker and the document tree it grants (Storage
/// Access Framework), in Kotlin behind the app's method channel: good for
/// shared storage, a USB stick and cloud providers alike, none of which
/// is a path an app may read.
class AndroidFolders implements FolderPlatform {
  static const channel = MethodChannel('io.github.paxel.oldbrainfix/folder');

  const AndroidFolders();

  @override
  bool get available => true;

  @override
  Future<String?> pick() async {
    try {
      return await channel.invokeMethod<String>('pickTree');
    } on PlatformException {
      return null;
    }
  }

  @override
  Future<String> displayName(String folder) async {
    try {
      final name = await channel.invokeMethod<String>('name', {'tree': folder});
      if (name != null && name.isNotEmpty) return name;
    } on PlatformException {
      // Named from the URI below.
    }
    // The last segment reads `primary:Sync`; the part after the colon is
    // the folder.
    final last = Uri.decodeComponent(folder.split('/').last);
    return last.contains(':') ? last.substring(last.indexOf(':') + 1) : last;
  }

  @override
  SyncFolder open(String folder) => SafSyncFolder(folder);

  @override
  BackupFolder openBackups(String folder) => SafBackupFolder(folder);

  @override
  Future<String?> autoBackupDir() async {
    try {
      return await channel.invokeMethod<String>('autoBackupDir');
    } on PlatformException {
      return null;
    } on MissingPluginException {
      return null;
    }
  }

  @override
  Future<void> setAutoBackup(bool on) async {
    try {
      await channel.invokeMethod<void>('autoBackup', {'on': on});
    } on PlatformException {
      // The agent keeps what it had; the switch is tried again at start.
    } on MissingPluginException {
      // No agent here.
    }
  }
}

/// The Backup folder through the document tree the picker granted:
/// Backups lie in the folder itself, and each goes in from the app's
/// own file, streamed by the platform, never through the channel whole.
class SafBackupFolder implements BackupFolder {
  /// The tree URI the picker returned.
  final String tree;

  const SafBackupFolder(this.tree);

  Map<String, Object> _args([String? name]) => {
    'tree': tree,
    'root': '',
    'dir': '',
    'name': ?name,
  };

  Future<T?> _call<T>(String method, Map<String, Object> args) async {
    try {
      return await AndroidFolders.channel.invokeMethod<T>(method, args);
    } on PlatformException catch (e) {
      throw FileSystemException(e.message ?? method, tree);
    } on MissingPluginException {
      throw FileSystemException('No folder access here', tree);
    }
  }

  @override
  Future<List<String>> list() async {
    final names = await _call<List<Object?>>('list', _args());
    return [
      for (final n in names ?? const [])
        if (n is String) n,
    ]..sort();
  }

  @override
  Future<void> put(String name, String path) =>
      _call<void>('put', {..._args(name), 'path': path});

  @override
  Future<void> delete(String name) => _call<void>('delete', _args(name));
}

/// The shared folder through the document tree the picker granted: every
/// call goes to the platform, which lists, reads and writes the documents
/// by name inside `oldbrainfix-sync`. A platform error becomes a
/// [FileSystemException], what a vanished drive raises elsewhere.
class SafSyncFolder implements SyncFolder {
  /// The tree URI the picker returned.
  final String tree;

  const SafSyncFolder(this.tree);

  Map<String, Object> _args(String dir, [String? name]) => {
    'tree': tree,
    'root': syncFolderRoot,
    'dir': dir,
    'name': ?name,
  };

  Future<T?> _call<T>(String method, Map<String, Object> args) async {
    try {
      return await AndroidFolders.channel.invokeMethod<T>(method, args);
    } on PlatformException catch (e) {
      throw FileSystemException(e.message ?? method, tree);
    } on MissingPluginException {
      throw FileSystemException('No folder access here', tree);
    }
  }

  @override
  Future<List<String>> list(String dir) async {
    final names = await _call<List<Object?>>('list', _args(dir));
    return [
      for (final n in names ?? const [])
        if (n is String) n,
    ]..sort();
  }

  @override
  Future<Uint8List?> read(String dir, String name) =>
      _call<Uint8List>('read', _args(dir, name));

  @override
  Future<void> write(String dir, String name, List<int> bytes) => _call<void>(
    'write',
    {..._args(dir, name), 'bytes': Uint8List.fromList(bytes)},
  );

  @override
  Future<void> delete(String dir, String name) =>
      _call<void>('delete', _args(dir, name));

  @override
  Future<void> ensure(String dir) => _call<void>('ensure', _args(dir));
}

/// Hands the app's [FolderPlatform] down to the Sync page.
class FolderScope extends InheritedWidget {
  final FolderPlatform folders;

  const FolderScope({super.key, required this.folders, required super.child});

  /// The app's [FolderPlatform]; read once, so it does not listen.
  static FolderPlatform of(BuildContext context) =>
      context.getInheritedWidgetOfExactType<FolderScope>()?.folders ??
      const NoFolders();

  @override
  bool updateShouldNotify(FolderScope oldWidget) =>
      folders != oldWidget.folders;
}
