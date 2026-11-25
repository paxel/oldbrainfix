import 'dart:io';

/// The trace of the notification taps (obf-1039): what each tap brought,
/// by its kind and ids, where the app took it, whether the app was
/// running, in the background or started by it, and when. It holds no
/// Note's text, only ids, keeps the last [kept] taps in a file of its
/// own beside the crash report, and goes along with Send feedback when
/// ticked there.

/// How the app was when a notification was tapped, as the platform
/// tells: [unknown] where it does not, as on the iPhone.
enum TapFrom { running, background, cold, unknown }

/// A notification tap as the platform told it: its [kind] ("occurrence",
/// "headsUp", "nudge", "changes", "catchUp", "ring"), how the app was
/// then, and when.
typedef TapArrival = ({String kind, TapFrom from, DateTime at});

/// The kinds of an Occurrence's notifications: its ring and its
/// Heads-up.
const occurrenceTapKinds = {'occurrence', 'headsUp'};

/// Where a tap took the app, as the trace names it.
abstract final class TapWent {
  static const agendaToday = 'agenda today, marked';
  static const agendaMissed = 'agenda Missed, marked';
  static const agendaLater = 'agenda coming day, marked';
  static const agendaDone = 'agenda, Already done';
  static const agendaSkipped = 'agenda, Skipped';
  static const agendaNowhere = 'agenda, not listed';
  static const agendaGone = 'agenda, Routine or Note gone';
  static const agenda = 'agenda';
  static const nudge = 'nudge page';
  static const changes = 'send changes';
  static const ring = 'ring screen';
  static const locked = 'waits for the unlock';
}

/// The last [kept] notification taps, oldest first.
class NotificationTaps {
  static const kept = 10;

  File? _file;
  final _lines = <String>[];

  /// Keeps the trace in [dir] from now on, with what an earlier run left
  /// there.
  void keepIn(Directory dir) {
    final file = _file = File('${dir.path}/notification_taps.txt');
    try {
      if (!file.existsSync()) return;
      final earlier = file
          .readAsLinesSync()
          .where((l) => l.trim().isNotEmpty)
          .toList();
      _lines
        ..insertAll(0, earlier)
        ..removeRange(0, _lines.length > kept ? _lines.length - kept : 0);
    } catch (_) {}
  }

  /// Notes the tap [arrival], or one of [kind] the platform did not tell
  /// of, with its [ids] and where it [went].
  void record({
    required String kind,
    TapArrival? arrival,
    Map<String, String> ids = const {},
    required String went,
  }) {
    final at = (arrival?.at ?? DateTime.now()).toUtc().toIso8601String();
    final from = (arrival?.from ?? TapFrom.unknown).name;
    _lines.add(
      [
        at,
        arrival?.kind ?? kind,
        for (final MapEntry(:key, :value) in ids.entries) '$key=$value',
        'from=$from',
        '-> $went',
      ].join(' '),
    );
    if (_lines.length > kept) _lines.removeRange(0, _lines.length - kept);
    try {
      _file?.writeAsStringSync('${_lines.join('\n')}\n');
    } catch (_) {}
  }

  /// How many taps are kept.
  int get count => _lines.length;

  /// The taps, one per line, oldest first; null while there are none.
  String? get text => _lines.isEmpty ? null : _lines.join('\n');

  /// Forgets every tap, for the tests.
  void clear() {
    _lines.clear();
    try {
      final file = _file;
      if (file != null && file.existsSync()) file.deleteSync();
    } catch (_) {}
  }
}

/// The app's one trace of notification taps.
final notificationTaps = NotificationTaps();

/// The taps Send feedback can send along, and how many: null when there
/// are none.
({String text, int count})? keptNotificationTaps() {
  final text = notificationTaps.text;
  return text == null ? null : (text: text, count: notificationTaps.count);
}
