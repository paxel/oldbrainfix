import 'dart:async';
import 'dart:io';
import 'dart:ui';

import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:path_provider/path_provider.dart';

import 'l10n/app_localizations.dart';
import 'src/about_screen.dart';
import 'src/alarm_platform.dart';
import 'src/alarm_sync.dart';
import 'src/announcements.dart';
import 'src/app.dart';
import 'src/appointments.dart';
import 'src/auto_sync.dart';
import 'src/bundle_platform.dart';
import 'src/calendar_mirror.dart';
import 'src/calendar_platform.dart';
import 'src/capture_requests.dart';
import 'src/context_platform.dart';
import 'src/crash_guard.dart';
import 'src/drafts.dart';
import 'src/folder_platform.dart';
import 'src/location.dart';
import 'src/lock_platform.dart';
import 'src/ocr_platform.dart';
import 'src/pairing_platform.dart';
import 'src/photo_platform.dart';
import 'src/profile_platform.dart';
import 'src/routine_editor_screen.dart';
import 'src/speech_platform.dart';
import 'src/store_needs_update_screen.dart';
import 'src/sync_platform.dart';
import 'src/voice_platform.dart';

/// The store the running app writes to; closed before Restart opens it
/// again.
Store? _store;

/// What the app has to say; one for the whole run, across Restarts.
final _announcements = Announcements();

Future<void> main() async {
  await runZonedGuarded(() async {
    WidgetsFlutterBinding.ensureInitialized();
    registerLicenses();
    final dir = await getApplicationSupportDirectory();
    // Armed before the store opens: a failure while opening it has to
    // reach the user instead of dying in a zone with nothing to catch it.
    // Restart closes what is left and opens it again.
    await initCrashGuard(
      dir,
      restart: () {
        try {
          _store?.close();
        } catch (_) {}
        unawaited(_openAndRun(dir));
      },
    );
    // Read the marker before arming it again: dirty means the last run
    // was killed without going out of sight first.
    if (await previousRunDied()) {
      final locale = basicLocaleListResolution(
        PlatformDispatcher.instance.locales,
        AppLocalizations.supportedLocales,
      );
      offerCrashReport(_announcements, lookupAppLocalizations(locale));
    }
    markRunning();
    // Out of sight is a clean end; back on screen, running again.
    AppLifecycleListener(onHide: markCleanExit, onShow: markRunning);
    await _openAndRun(dir);
  }, reportCrash);
}

/// Opens the store and starts the app. Separate from [main] because the
/// crash screen's Restart runs it again. A database a newer version saved
/// is not opened: one screen says so, with nothing else to do.
Future<void> _openAndRun(Directory dir) async {
  final work = await getTemporaryDirectory();
  final Store store;
  try {
    store = Store.open('${dir.path}/oldbrainfix.db');
  } on StoreNeedsUpdate {
    runApp(const StoreNeedsUpdateApp());
    return;
  }
  _store = store;
  runApp(
    OldBrainFixApp(
      store: store,
      announcements: _announcements,
      drafts: FileDrafts(Directory('${dir.path}/drafts')),
      location: const AndroidLocation(),
      alarms: AndroidAlarms(),
      photos: AndroidPhotos(),
      voice: AndroidVoice(),
      context: const AndroidContext(),
      requests: AndroidCaptureRequests(),
      speech: AndroidSpeech(),
      ocr: const AndroidOcr(),
      calendar: const AndroidCalendar(),
      mirror: const AndroidMirror(),
      appointments: const AndroidAppointments(),
      lock: const AndroidBiometrics(),
      pairing: const AndroidPairing(),
      sync: AndroidSync(),
      pairedHost: startPairedHosting,
      folders: const AndroidFolders(),
      bundles: AndroidBundles(),
      profile: AndroidProfile(),
      workDir: work.path,
    ),
  );
}

/// The background sync the periodic work starts in an engine of its own
/// while the app is closed (android/…/SyncWorker.kt): paired Devices,
/// hosted for and joined, and the shared folder, the same store, the
/// weekly Backup when it is due, the alarms armed with the texts in the
/// Device's language, and the work told when it is over. A database a
/// newer version saved is left alone: nothing syncs.
@pragma('vm:entry-point')
Future<void> backgroundSyncMain() async {
  WidgetsFlutterBinding.ensureInitialized();
  final sync = AndroidSync();
  try {
    final dir = await getApplicationSupportDirectory();
    final Store store;
    try {
      store = Store.open('${dir.path}/oldbrainfix.db');
    } on StoreNeedsUpdate {
      return;
    }
    final alarms = AlarmSync(
      store: store,
      platform: AndroidAlarms(),
      wallClock: deviceWallClock,
    );
    try {
      final locale = basicLocaleListResolution(
        PlatformDispatcher.instance.locales,
        AppLocalizations.supportedLocales,
      );
      alarms.setTexts(lookupAppLocalizations(locale), locale.toString());
      await backgroundSync(
        store: store,
        platform: sync,
        alarms: alarms,
        name: await const AndroidPairing().deviceName(),
        folders: const AndroidFolders(),
        host: startPairedHosting,
        workDir: (await getTemporaryDirectory()).path,
      );
    } finally {
      alarms.dispose();
      store.close();
    }
  } finally {
    await sync.finished();
  }
}
