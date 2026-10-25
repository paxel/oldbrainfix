import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'announcements.dart';
import 'app_lock.dart';
import 'conflicts_screen.dart';
import 'l10n.dart';

/// The platform seam for bundle files (android/…/Bundles.kt): where a
/// bundle is written before it is shared, handing it to the app the user
/// picks, picking one to import, and the bundles other apps opened with
/// this one. Every bundle to import is a copy of the app's own, removed
/// once imported. Sent Items go in and out the same way, told apart by
/// what is in them, and so do Backups to restore and the Recovery Sheet
/// to share or print. Widget tests hand in a fake.
abstract interface class BundlePlatform {
  /// Whether bundles can be shared and imported here.
  bool get available;

  /// Where a bundle named [name] is written to be shared.
  Future<String?> exportPath(String name);

  /// Offers the file at [path], a bundle unless [type] says otherwise,
  /// to the app the user picks; false when no app can take it.
  Future<bool> share(String path, {String type = bundleMimeType});

  /// Opens the system's file picker; the copy of the file picked, or
  /// null when the user backed out.
  Future<String?> pick();

  /// Opens the system's print dialog for the PDF at [path], written
  /// where [exportPath] said, titled [name]; false when there is none.
  Future<bool> print(String path, String name);

  /// The copies of the bundles other apps opened with this one that are
  /// not imported yet, oldest first; each is handed out once.
  Future<List<String>> takeOpened();

  /// Another app opened a bundle with this one while it runs.
  Stream<void> get opened;
}

/// No bundles: the default where none is handed in.
class NoBundles implements BundlePlatform {
  const NoBundles();

  @override
  bool get available => false;

  @override
  Future<String?> exportPath(String name) async => null;

  @override
  Future<bool> share(String path, {String type = bundleMimeType}) async =>
      false;

  @override
  Future<String?> pick() async => null;

  @override
  Future<bool> print(String path, String name) async => false;

  @override
  Future<List<String>> takeOpened() async => const [];

  @override
  Stream<void> get opened => const Stream.empty();
}

/// Android's share chooser, file picker and "open with" behind the app's
/// method channel; no storage permission.
class AndroidBundles implements BundlePlatform {
  static const _channel = MethodChannel('io.github.paxel.oldbrainfix/bundle');

  final _opened = StreamController<void>.broadcast();

  AndroidBundles() {
    _channel.setMethodCallHandler((call) async {
      if (call.method == 'opened') _opened.add(null);
    });
  }

  @override
  bool get available => true;

  @override
  Future<String?> exportPath(String name) =>
      _ask<String>('exportPath', {'name': name});

  @override
  Future<bool> share(String path, {String type = bundleMimeType}) async =>
      await _ask<bool>('share', {'path': path, 'type': type}) ?? false;

  @override
  Future<String?> pick() => _ask<String>('pick', null);

  @override
  Future<bool> print(String path, String name) async =>
      await _ask<bool>('print', {'path': path, 'name': name}) ?? false;

  @override
  Future<List<String>> takeOpened() async {
    final paths = await _ask<List<Object?>>('take', null);
    return [
      for (final p in paths ?? const [])
        if (p is String) p,
    ];
  }

  @override
  Stream<void> get opened => _opened.stream;

  static Future<T?> _ask<T>(String method, Object? args) async {
    try {
      return await _channel.invokeMethod<T>(method, args);
    } on PlatformException {
      return null;
    }
  }
}

/// Hands the app's [BundlePlatform] down to every screen.
class BundleScope extends InheritedWidget {
  final BundlePlatform bundles;

  const BundleScope({super.key, required this.bundles, required super.child});

  /// The app's [BundlePlatform]; read once, so it does not listen.
  static BundlePlatform of(BuildContext context) =>
      context.getInheritedWidgetOfExactType<BundleScope>()?.bundles ??
      const NoBundles();

  @override
  bool updateShouldNotify(BundleScope oldWidget) =>
      bundles != oldWidget.bundles;
}

/// Imports the bundle copy at [path] into [store], removes the copy and
/// announces at the top what came in, or why nothing did. Returns how
/// many of the Conflicts it brought may be shown, for the caller to
/// announce: behind the App Lock they wait for the unlock.
Future<int> importBundleFile(
  BuildContext context,
  Store store,
  String path,
) async {
  final t = context.t;
  final announcements = AnnouncementsScope.of(context);
  final lock = LockScope.of(context);
  BundleResult? result;
  String? failure;
  try {
    result = await importBundle(store, path);
  } on NotABundle {
    failure = t.bundleNotABundle;
  } on BundleNeedsUpdate {
    failure = t.bundleNeedsUpdate;
  } on Object {
    // Whatever else broke on the way: nothing half-taken stays.
    failure = t.bundleFailed;
  } finally {
    try {
      File(path).deleteSync();
    } on FileSystemException {
      // Gone already.
    }
  }
  if (failure != null || result == null) {
    announcements?.announce(failure ?? t.bundleFailed);
    return 0;
  }
  if (result.needsPairing) {
    announcements?.announce(t.bundlePairFirst);
    return 0;
  }
  // How much came in tells nothing of what: fine behind the lock too.
  announcements?.announce(t.bundleCameIn(result.applied.length));
  final refused = result.report.refusedCount;
  if (refused > 0) announcements?.announce(t.syncRefused(refused));
  return visibleConflicts(store, lock, result.conflicts).length;
}
