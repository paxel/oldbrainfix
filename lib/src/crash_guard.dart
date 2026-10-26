import 'dart:async';
import 'dart:io';
import 'dart:ui';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:url_launcher/url_launcher.dart';

import '../l10n/app_localizations.dart';
import 'announcements.dart';
import 'l10n.dart';
import 'look.dart';

/// Global crash guard (adapted from cat(a)log).
///
/// Catches every uncaught Dart and Flutter error and replaces the
/// framework's grey or red death screen with one that says what
/// happened, with a Restart button and a prefilled mail to the
/// developer. A real out-of-memory kill cannot be caught by anything in
/// the process. On Android 11 and newer the system remembers why the
/// process died and is asked at the next start: a crash, a native crash,
/// an ANR or a memory kill while on screen get the offer of a report, a
/// battery saver's or the user's kill gets none. Older systems fall back
/// to the running marker: set at startup, cleared when the app goes out
/// of sight, dirty at the next start means a kill. A crash in the
/// Kotlin code writes the same files (android/…/CrashLog.kt).
const crashMail = 'taum@tuta.io';

Directory? _supportDir;
void Function()? _restart;
String? _appVersion;

File? _file(String name) {
  final dir = _supportDir;
  return dir == null ? null : File('${dir.path}/$name');
}

File? get _crashFile => _file('last_crash.txt');
File? get _marker => _file('running.marker');
File? get _exitSeen => _file('exit_seen.txt');

Future<void> initCrashGuard(
  Directory supportDir, {
  required void Function() restart,
  String? appVersion,
}) async {
  _supportDir = supportDir;
  _restart = restart;
  // A report without a version is a report nobody can act on, so the
  // version is fetched here rather than left to the caller.
  _appVersion = appVersion ?? await _versionFromPackage();

  FlutterError.onError = (details) {
    FlutterError.presentError(details);
    _record(details.exception, details.stack);
  };
  PlatformDispatcher.instance.onError = (error, stack) {
    reportCrash(error, stack);
    return true;
  };
  // In release builds, build errors render as a compact mark instead of
  // a grey void.
  if (kReleaseMode) {
    ErrorWidget.builder = (details) =>
        const Center(child: Icon(Icons.healing_outlined, size: 32));
  }
}

/// Records [error] and shows the crash screen. Works before
/// [initCrashGuard] too, when the start failed before it: then nothing
/// is written and the screen offers no Restart, but the report can
/// still be sent.
void reportCrash(Object error, StackTrace? stack) {
  _record(error, stack);
  _showCrashScreen(error, stack);
}

/// True when the previous run died in a way worth a report. The system's
/// exit record decides where there is one; the marker decides otherwise.
/// Cleared by asking exactly once.
Future<bool> previousRunDied() async {
  final marker = _marker;
  if (marker == null) return false;
  final dirty = marker.existsSync();
  if (dirty) marker.deleteSync();
  final exit = await _lastExit();
  if (exit == null) return dirty;
  // Each record is asked about once; the same one at the next start
  // means the run in between ended without a record, so nothing died.
  final seenFile = _exitSeen!;
  final seen = seenFile.existsSync() ? seenFile.readAsStringSync().trim() : '';
  if (seen == '${exit.timestamp}') return false;
  try {
    seenFile.writeAsStringSync('${exit.timestamp}');
  } catch (_) {}
  if (!exitDeservesReport(exit.reason, foreground: exit.foreground)) {
    return false;
  }
  final crashFile = _crashFile!;
  if (!crashFile.existsSync()) {
    try {
      crashFile.writeAsStringSync(
        '${crashReportHeader()}\n\n'
        'The system ended the app: ${exit.reason}'
        '${exit.foreground ? ' while on screen' : ' in the background'}.\n'
        '${exit.description}',
      );
    } catch (_) {}
  }
  return true;
}

/// Whether an exit the system reports deserves a report: a crash, a
/// native crash, an ANR, or a memory kill while the app was on screen.
/// A memory kill in the background is Android's housekeeping.
bool exitDeservesReport(String reason, {required bool foreground}) =>
    switch (reason) {
      'crash' || 'native' || 'anr' => true,
      'low_memory' => foreground,
      _ => false,
    };

/// What the system remembers of the last exit.
class LastExit {
  final String reason;
  final bool foreground;
  final int timestamp;
  final String description;

  const LastExit(
    this.reason, {
    required this.foreground,
    required this.timestamp,
    required this.description,
  });
}

/// Asks Android 11+ for the newest exit record; null elsewhere, on older
/// systems, without a record, or when the channel is not there.
Future<LastExit?> _lastExit() async {
  if (!Platform.isAndroid) return null;
  try {
    final info = await const MethodChannel('io.github.paxel.oldbrainfix/exit')
        .invokeMethod<Map<Object?, Object?>>('lastExit');
    if (info == null) return null;
    return LastExit(
      info['reason'] as String? ?? 'other',
      foreground: info['foreground'] as bool? ?? false,
      timestamp: (info['timestamp'] as num?)?.toInt() ?? 0,
      description: info['description'] as String? ?? '',
    );
  } catch (_) {
    return null;
  }
}

void markRunning() {
  try {
    _marker?.writeAsStringSync('1');
  } catch (_) {}
}

void markCleanExit() {
  try {
    final marker = _marker;
    if (marker != null && marker.existsSync()) marker.deleteSync();
  } catch (_) {}
}

Future<String?> _versionFromPackage() async {
  try {
    final info = await PackageInfo.fromPlatform();
    return '${info.version}+${info.buildNumber}';
  } catch (_) {
    return null;
  }
}

/// What every report must carry to be worth reading: which build, on
/// which system, in which language, and when.
String crashReportHeader({DateTime? at}) => [
  'Old Brain Fix ${_appVersion ?? 'version unknown'}',
  '${Platform.operatingSystem} ${Platform.operatingSystemVersion}',
  'locale ${Platform.localeName}',
  (at ?? DateTime.now()).toIso8601String(),
].join('\n');

void _record(Object error, StackTrace? stack) {
  try {
    _crashFile?.writeAsStringSync('${crashReportHeader()}\n\n$error\n\n$stack');
  } catch (_) {}
}

String? lastCrashText() {
  final file = _crashFile;
  return file != null && file.existsSync() ? file.readAsStringSync() : null;
}

void clearLastCrash() {
  try {
    final file = _crashFile;
    if (file != null && file.existsSync()) file.deleteSync();
  } catch (_) {}
}

/// How much of the report fits into a `mailto:` body: mail apps cap the
/// URL around 2000 characters.
const mailBodyLimit = 1800;

/// The report as the mail carries it. The trace is thinned first: the
/// `dart:async` and zone frames say nothing about the app and crowd out
/// the frames that do, so they go, and the app's own frames move to the
/// top. Only then is the text cut to [limit].
String mailBody(String body, {int limit = mailBodyLimit}) {
  final lines = body.split('\n');
  final frame = RegExp(r'^#\d+\s');
  final head = <String>[];
  final own = <String>[];
  final other = <String>[];
  for (final line in lines) {
    if (!frame.hasMatch(line)) {
      head.add(line);
      continue;
    }
    if (line.contains('dart:async') || line.contains('zone')) continue;
    (line.contains('package:oldbrainfix/') ||
                line.contains('package:oldbrainfix_core/')
            ? own
            : other)
        .add(line);
  }
  final text = [...head, ...own, ...other].join('\n');
  return text.length > limit ? text.substring(0, limit) : text;
}

Future<void> mailCrashReport(String body) async {
  final uri = Uri(
    scheme: 'mailto',
    path: crashMail,
    query:
        'subject=${Uri.encodeComponent('Old Brain Fix crash report')}'
        '&body=${Uri.encodeComponent(mailBody(body))}',
  );
  try {
    await launchUrl(uri);
  } catch (_) {}
}

/// Offers the report of the run that died at the top of the screen (UI
/// laws, "Announcements"): tapping it opens the mail, which the user
/// still sends or discards. The report is taken now and the file
/// cleared, so the same crash is offered once.
void offerCrashReport(
  Announcements announcements,
  AppLocalizations t, {
  Future<void> Function(String body) mail = mailCrashReport,
}) {
  final body =
      lastCrashText() ??
      '${crashReportHeader()}\n\n'
          'Old Brain Fix was killed without a crash log '
          '(most likely out of memory).';
  clearLastCrash();
  announcements.announce(
    t.crashLastRunBody,
    onTap: () => unawaited(mail(body)),
  );
}

bool _crashScreenUp = false;

void _showCrashScreen(Object error, StackTrace? stack) {
  if (_crashScreenUp) return;
  _crashScreenUp = true;
  runApp(_CrashApp(error: error, stack: stack));
}

class _CrashApp extends StatelessWidget {
  final Object error;
  final StackTrace? stack;

  const _CrashApp({required this.error, this.stack});

  @override
  Widget build(BuildContext context) => MaterialApp(
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    theme: lookTheme(Brightness.light),
    darkTheme: lookTheme(Brightness.dark),
    home: Builder(
      builder: (context) {
        final t = context.t;
        final restart = _restart;
        return Scaffold(
          body: SafeArea(
            child: Padding(
              padding: const EdgeInsets.all(24),
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  const Icon(Icons.healing_outlined, size: 64),
                  const SizedBox(height: 16),
                  Text(
                    t.crashTitle,
                    style: Theme.of(context).textTheme.headlineSmall,
                    textAlign: TextAlign.center,
                  ),
                  const SizedBox(height: 8),
                  Text(t.crashBody, textAlign: TextAlign.center),
                  const SizedBox(height: 16),
                  Container(
                    constraints: const BoxConstraints(maxHeight: 140),
                    padding: const EdgeInsets.all(8),
                    decoration: BoxDecoration(
                      color: Theme.of(context)
                          .colorScheme
                          .surfaceContainerHighest,
                      borderRadius: BorderRadius.circular(8),
                    ),
                    child: SingleChildScrollView(
                      child: SelectableText(
                        '$error',
                        style: const TextStyle(
                          fontFamily: 'monospace',
                          fontSize: 12,
                        ),
                      ),
                    ),
                  ),
                  const SizedBox(height: 20),
                  if (restart != null) ...[
                    FilledButton.icon(
                      icon: const Icon(Icons.refresh),
                      label: Text(t.crashRestart),
                      onPressed: () {
                        _crashScreenUp = false;
                        restart();
                      },
                    ),
                    const SizedBox(height: 8),
                  ],
                  OutlinedButton.icon(
                    icon: const Icon(Icons.mail_outline),
                    label: Text(t.crashSendReport),
                    onPressed: () => mailCrashReport(
                      lastCrashText() ??
                          '${crashReportHeader()}\n\n$error\n\n$stack',
                    ),
                  ),
                ],
              ),
            ),
          ),
        );
      },
    ),
  );
}
