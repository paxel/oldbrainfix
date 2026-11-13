import 'dart:async';
import 'dart:io';
import 'dart:ui';

import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:path_provider/path_provider.dart';

import 'l10n/app_localizations.dart';
import 'src/about_screen.dart';
import 'src/agenda_reader.dart';
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
import 'src/file_platform.dart';
import 'src/folder_platform.dart';
import 'src/help_notes.dart';
import 'src/home_shortcuts.dart';
import 'src/location.dart';
import 'src/lock_platform.dart';
import 'src/ocr_platform.dart';
import 'src/pairing_platform.dart';
import 'src/photo_platform.dart';
import 'src/print_platform.dart';
import 'src/profile_platform.dart';
import 'src/routine_editor_screen.dart';
import 'src/speech_platform.dart';
import 'src/store_needs_update_screen.dart';
import 'src/sync_platform.dart';
import 'src/sync_runner.dart';
import 'src/voice_platform.dart';

/// The store the running app writes to; closed before Restart opens it
/// again.
Store? _store;

/// What the app has to say; one for the whole run, across Restarts.
final _announcements = Announcements();

/// The app's root navigator, where the report of a run that died opens.
final _navigator = GlobalKey<NavigatorState>();

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
      offerCrashReport(
        _announcements,
        lookupAppLocalizations(locale),
        navigator: _navigator,
      );
    }
    markRunning();
    watchRunning();
    await _openAndRun(dir);
  }, reportCrash);
}

/// Opens the store and starts the app. Separate from [main] because the
/// crash screen's Restart runs it again. A database a newer version saved
/// is not opened: one screen says so, with nothing else to do.
Future<void> _openAndRun(Directory dir) async {
  final work = await getTemporaryDirectory();
  final Store store;
  final path = '${dir.path}/oldbrainfix.db';
  try {
    store = Store.open(path);
  } on StoreNeedsUpdate {
    runApp(const StoreNeedsUpdateApp());
    return;
  }
  _store = store;
  // The version the help Notes are made for (#180); without it the app
  // starts without them.
  String? version;
  try {
    version = (await PackageInfo.fromPlatform()).version;
  } on Exception {
    version = null;
  }
  runApp(
    OldBrainFixApp(
      store: store,
      announcements: _announcements,
      navigatorKey: _navigator,
      drafts: FileDrafts(Directory('${dir.path}/drafts')),
      location: const AndroidLocation(),
      // The one place the platform picks the alarm layer; its answers,
      // such as the setup's steps, do the rest (#238).
      alarms: Platform.isIOS ? IosAlarms() : AndroidAlarms(),
      photos: AndroidPhotos(),
      files: const AndroidFiles(),
      // Only Android lets an app be a printer (#293).
      printing: Platform.isIOS ? const NoPrinting() : const AndroidPrinting(),
      voice: AndroidVoice(),
      context: const AndroidContext(),
      requests: AndroidCaptureRequests(),
      shortcuts: ChannelHomeShortcuts(),
      speech: AndroidSpeech(),
      ocr: const AndroidOcr(),
      calendar: const AndroidCalendar(),
      mirror: const AndroidMirror(),
      appointments: const AndroidAppointments(),
      lock: const AndroidBiometrics(),
      pairing: const AndroidPairing(),
      // Hosts only while the app is open on the iPhone (#240).
      sync: Platform.isIOS ? IosSync() : AndroidSync(),
      pairedHost: startPairedHosting,
      folders: const AndroidFolders(),
      // The automatic sync in an isolate of its own, so the app keeps
      // drawing while it merges (#225).
      syncRunner: IsolateSyncRunner(
        path: path,
        open: openSafFolder,
        token: RootIsolateToken.instance,
      ),
      // The Agenda's reads and the alarms' plan there too, so the app
      // draws on while they read (#207).
      agendaReader: IsolateAgendaReader(path),
      bundles: AndroidBundles(),
      profile: AndroidProfile(),
      workDir: work.path,
      help: version == null
          ? null
          : HelpSetup(const BundledHelp(), version: version),
    ),
  );
}

/// The background sync the periodic work starts in an engine of its own
/// while the app is closed (android/…/SyncWorker.kt): paired Devices,
/// hosted for and joined, and the shared folder, the same store, the
/// weekly Backup when it is due, the alarms armed with the texts in the
/// Device's language, and the work told when it is over. A database a
/// newer version saved is left alone: nothing syncs. On the iPhone
/// BGTaskScheduler's task starts it the same way
/// (ios/Runner/BackgroundSync.swift), and it hosts nothing (#240).
@pragma('vm:entry-point')
Future<void> backgroundSyncMain() async {
  WidgetsFlutterBinding.ensureInitialized();
  final sync = Platform.isIOS ? IosSync() : AndroidSync();
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
      platform: Platform.isIOS ? IosAlarms() : AndroidAlarms(),
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
