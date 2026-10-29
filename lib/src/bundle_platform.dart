import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'announcements.dart';
import 'app_lock.dart';
import 'area_offer_screen.dart';
import 'conflicts_screen.dart';
import 'l10n.dart';
import 'profile.dart';

/// What became of a file offered to the app the user picks.
enum ShareOutcome {
  /// The user picked an app on the share sheet, which took the file.
  taken,

  /// The share sheet was left without picking an app.
  dismissed,

  /// No app here can take the file.
  noApp,
}

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
  /// to the app the user picks; answers once the user is back, with
  /// whether an app was picked.
  Future<ShareOutcome> share(String path, {String type = bundleMimeType});

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
  Future<ShareOutcome> share(
    String path, {
    String type = bundleMimeType,
  }) async => ShareOutcome.noApp;

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
  Future<ShareOutcome> share(
    String path, {
    String type = bundleMimeType,
  }) async =>
      switch (await _ask<String>('share', {'path': path, 'type': type})) {
        'taken' => ShareOutcome.taken,
        'dismissed' => ShareOutcome.dismissed,
        _ => ShareOutcome.noApp,
      };

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
/// announce: behind the App Lock they wait for the unlock. A bundle that
/// offers an Area from afar (#108) opens its preview instead; behind the
/// App Lock it is only said to wait, and waits on the Agenda.
Future<int> importBundleFile(
  BuildContext context,
  Store store,
  String path,
) async {
  final t = context.t;
  final announcements = AnnouncementsScope.of(context);
  final lock = LockScope.of(context);
  final navigator = Navigator.of(context);
  BundleResult? result;
  String? failure;
  Exception? offerRefused;
  try {
    result = await importBundle(store, path);
  } on NotABundle {
    failure = t.bundleNotABundle;
  } on BundleNeedsUpdate {
    failure = t.bundleNeedsUpdate;
  } on Exception catch (e) {
    // An Area offer refused says why behind the tap; whatever else broke
    // on the way leaves nothing half-taken.
    if (areaOfferRefusal(t, e) == null) {
      failure = t.bundleFailed;
    } else {
      offerRefused = e;
    }
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
  if (offerRefused != null) {
    announceAreaOfferRefused(t, announcements, navigator, offerRefused);
    return 0;
  }
  if (failure != null || result == null) {
    announcements?.announce(failure ?? t.bundleFailed);
    return 0;
  }
  if (result.offer case final offer?) {
    if (lock?.locked ?? false) {
      announcements?.announce(t.areaOfferWaits);
    } else {
      await navigator.push(
        MaterialPageRoute<void>(
          builder: (_) => AreaOfferScreen(store: store, offer: offer),
        ),
      );
    }
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

/// The name a bundle for [person], whose own Profile name is [name],
/// goes out as on [day]: the day's bundle name with theirs in it, so the
/// bundles for several people, sent one after another, never overwrite
/// one an app may still be reading.
String changesFileName(DateTime day, String person, String name) {
  String part(String text) => text
      .replaceAll(RegExp(r'[^\p{L}\p{N}]+', unicode: true), '-')
      .replaceAll(RegExp(r'^-+|-+$'), '');
  final who = part(name).isNotEmpty ? part(name) : part(person);
  final base = bundleFileName(day);
  final dot = base.lastIndexOf('.');
  final short = who.length > 32 ? who.substring(0, 32) : who;
  return '${base.substring(0, dot)}-$short${base.substring(dot)}';
}

/// Sends [person] a bundle of what is new for them in the Areas shared
/// with them (#108), offered to the app the user picks. Only once an app
/// was picked do the changes up to the log's mark, read before the
/// bundle was written, count as sent (#109), and [onSent] is told: a
/// share sheet left without an app keeps the Changes Nudge and says
/// nothing. Nothing new, no app and a failure are announced at the top.
Future<ShareOutcome?> sendChanges({
  required Store store,
  required BundlePlatform bundles,
  required AppLocalizations t,
  required Announcements? announcements,
  required String person,
  VoidCallback? onSent,
}) async {
  if (store.entriesForPerson(person).isEmpty) {
    announcements?.announce(
      t.bundleNothingNew(profileDisplayName(t, store.personShown(person).name)),
    );
    return null;
  }
  try {
    final path = await bundles.exportPath(
      changesFileName(
        store.clock().toLocal(),
        person,
        store.personTheirs(person).name,
      ),
    );
    if (path == null) {
      announcements?.announce(t.bundleFailed);
      return null;
    }
    final seq = store.currentSeq();
    await writeBundle(store, path, forPerson: person);
    final shared = await bundles.share(path);
    switch (shared) {
      case ShareOutcome.taken:
        store.noteChangesSent(person, seq);
        onSent?.call();
      case ShareOutcome.noApp:
        announcements?.announce(t.bundleNoApp);
      case ShareOutcome.dismissed:
        break;
    }
    return shared;
  } on Object {
    announcements?.announce(t.bundleFailed);
    return null;
  }
}
