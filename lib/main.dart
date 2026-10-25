import 'dart:io';
import 'dart:ui';

import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:path_provider/path_provider.dart';

import 'l10n/app_localizations.dart';
import 'src/about_screen.dart';
import 'src/alarm_platform.dart';
import 'src/alarm_sync.dart';
import 'src/app.dart';
import 'src/auto_sync.dart';
import 'src/calendar_mirror.dart';
import 'src/calendar_platform.dart';
import 'src/capture_requests.dart';
import 'src/context_platform.dart';
import 'src/drafts.dart';
import 'src/folder_platform.dart';
import 'src/location.dart';
import 'src/lock_platform.dart';
import 'src/ocr_platform.dart';
import 'src/pairing_platform.dart';
import 'src/photo_platform.dart';
import 'src/routine_editor_screen.dart';
import 'src/speech_platform.dart';
import 'src/sync_platform.dart';
import 'src/voice_platform.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  registerLicenses();
  final dir = await getApplicationSupportDirectory();
  final store = Store.open('${dir.path}/oldbrainfix.db');
  runApp(
    OldBrainFixApp(
      store: store,
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
      lock: const AndroidBiometrics(),
      pairing: const AndroidPairing(),
      sync: AndroidSync(),
      folders: const AndroidFolders(),
    ),
  );
}

/// The background sync the periodic work starts in an engine of its own
/// while the app is closed (android/…/SyncWorker.kt): paired Devices and
/// the shared folder, the same store,
/// the alarms armed with the texts in the Device's language, and the
/// work told when it is over.
@pragma('vm:entry-point')
Future<void> backgroundSyncMain() async {
  WidgetsFlutterBinding.ensureInitialized();
  final sync = AndroidSync();
  try {
    final dir = await getApplicationSupportDirectory();
    final store = Store.open('${dir.path}/oldbrainfix.db');
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
      );
    } finally {
      alarms.dispose();
      store.close();
    }
  } finally {
    await sync.finished();
  }
}
