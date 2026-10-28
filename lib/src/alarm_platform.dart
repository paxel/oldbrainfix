import 'dart:async';

import 'package:flutter/services.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'agenda_widget.dart';

/// The access alarms need (ADR 0004), each granted by the user.
enum AlarmPermission {
  /// Showing notifications at all.
  notifications,

  /// Alarms on the minute; without it they come inexact.
  exactAlarms,

  /// Full-screen alarms, for Escalation.
  fullScreen,
}

/// One planned alarm with the texts its notification shows.
class ArmedAlarm {
  final PlannedAlarm alarm;
  final String title;
  final String body;

  const ArmedAlarm({
    required this.alarm,
    required this.title,
    required this.body,
  });
}

/// The planned Nudge with the texts its notification shows.
class ArmedNudge {
  final PlannedNudge nudge;
  final String title;
  final String body;

  const ArmedNudge({
    required this.nudge,
    required this.title,
    required this.body,
  });
}

/// The planned Changes Nudge (#109) with the texts its notification
/// shows: [title] names the people, [public] does not, for the lock
/// screen.
class ArmedChangesNudge {
  final PlannedChangesNudge nudge;
  final String title;
  final String body;
  final String public;

  const ArmedChangesNudge({
    required this.nudge,
    required this.title,
    required this.body,
    required this.public,
  });
}

/// The texts of the alarm notifications, in the reader's language. The
/// platform keeps them, so it can show alarms while the app is closed.
class AlarmLabels {
  final String channel;
  final String done;
  final String snooze;
  final String snoozeQuestion;
  final String snooze10;
  final String snooze30;
  final String snooze60;

  /// The Catch-up Notification's title, and its text for one and for
  /// several Missed; the latter holds `{count}` for the number.
  final String catchUpTitle;
  final String catchUpOne;
  final String catchUpMany;

  /// The channels of the full-screen alarm and of its high-priority
  /// fallback.
  final String ringChannel;
  final String escalatedChannel;

  /// The channel of the Nudge.
  final String nudgeChannel;

  const AlarmLabels({
    required this.channel,
    required this.done,
    required this.snooze,
    required this.snoozeQuestion,
    required this.snooze10,
    required this.snooze30,
    required this.snooze60,
    required this.catchUpTitle,
    required this.catchUpOne,
    required this.catchUpMany,
    required this.ringChannel,
    required this.escalatedChannel,
    required this.nudgeChannel,
  });

  Map<String, String> toMap() => {
    'channel': channel,
    'done': done,
    'snooze': snooze,
    'snoozeQuestion': snoozeQuestion,
    'snooze10': snooze10,
    'snooze30': snooze30,
    'snooze60': snooze60,
    'catchUpTitle': catchUpTitle,
    'catchUpOne': catchUpOne,
    'catchUpMany': catchUpMany,
    'ringChannel': ringChannel,
    'escalatedChannel': escalatedChannel,
    'nudgeChannel': nudgeChannel,
  };
}

/// Done or a snooze tapped on a notification, waiting to be applied.
sealed class AlarmAction {
  /// Counts up; acknowledging one acknowledges everything before it.
  final int seq;

  /// When it was tapped.
  final DateTime at;

  const AlarmAction(this.seq, this.at);
}

class DoneTapped extends AlarmAction {
  final String routineId;
  final String occurrenceKey;

  const DoneTapped(int seq, this.routineId, this.occurrenceKey, DateTime at)
    : super(seq, at);
}

class SnoozeTapped extends AlarmAction {
  final String routineId;
  final String occurrenceKey;

  /// When the Occurrence rings again.
  final DateTime until;

  const SnoozeTapped(
    int seq,
    this.routineId,
    this.occurrenceKey,
    DateTime at,
    this.until,
  ) : super(seq, at);
}

/// A snooze tapped on the Nudge: the Notes Unsorted then wait until
/// [until].
class NudgeSnoozed extends AlarmAction {
  final DateTime until;

  const NudgeSnoozed(super.seq, super.at, this.until);
}

/// An Occurrence ringing as a full-screen alarm (Escalation), with the
/// texts its notification shows.
class Ringing {
  final String routineId;
  final String occurrenceKey;
  final String title;
  final String body;

  const Ringing({
    required this.routineId,
    required this.occurrenceKey,
    required this.title,
    required this.body,
  });

  @override
  bool operator ==(Object other) =>
      other is Ringing &&
      other.routineId == routineId &&
      other.occurrenceKey == occurrenceKey &&
      other.title == title &&
      other.body == body;

  @override
  int get hashCode => Object.hash(routineId, occurrenceKey, title, body);
}

/// The seam to the platform's alarm layer (android/…/Scheduler.kt).
/// Widget tests hand in a fake.
abstract interface class AlarmPlatform {
  /// Whether this Device has an alarm layer at all; setup and banner
  /// only show where it has.
  bool get available;

  /// Arms exactly [alarms], the [nudge] and the Changes Nudge
  /// [changes], replacing everything armed before, and takes away the
  /// notifications of the Occurrences in [done], given as Routine id and
  /// Occurrence, and the Nudge's or the Changes Nudge's when there is
  /// none.
  Future<void> arm(
    List<ArmedAlarm> alarms,
    AlarmLabels labels, {
    List<(String, Occurrence)> done = const [],
    ArmedNudge? nudge,
    ArmedChangesNudge? changes,
  });

  /// Whether the Nudge's notification was tapped since this was last
  /// asked; asking clears it.
  Future<bool> nudgeOpened();

  /// Whether the Changes Nudge's notification was tapped since this was
  /// last asked; asking clears it.
  Future<bool> changesOpened();

  /// Hands the home-screen Agenda widget what it shows from now on.
  Future<void> showAgenda(AgendaSnapshot snapshot);

  /// Whether the Agenda widget's Unsorted count was tapped since this
  /// was last asked; asking clears it.
  Future<bool> unsortedOpened();

  /// The taps not acknowledged yet, oldest first.
  Future<List<AlarmAction>> pendingActions();

  /// The taps up to [seq] are applied; the platform forgets them.
  Future<void> acknowledge(int seq);

  /// A notification was tapped while the app runs, or a full-screen
  /// alarm started or stopped.
  Stream<void> get actionsArrived;

  /// The Occurrence ringing as a full-screen alarm now, or null.
  Future<Ringing?> ringing();

  /// Done, or a snooze for [snooze], from the full-screen alarm: it stops
  /// ringing, and the answer is queued like a tap on the notification.
  Future<void> answerRing(Ringing ring, {Duration? snooze});

  Future<Map<AlarmPermission, bool>> permissions();

  /// Asks for [permission] or opens its system settings page; the answer
  /// shows in [permissions] once the user is back.
  Future<void> request(AlarmPermission permission);
}

/// No alarm layer: the default where none is handed in.
class NoAlarms implements AlarmPlatform {
  const NoAlarms();

  @override
  bool get available => false;

  @override
  Future<void> arm(
    List<ArmedAlarm> alarms,
    AlarmLabels labels, {
    List<(String, Occurrence)> done = const [],
    ArmedNudge? nudge,
    ArmedChangesNudge? changes,
  }) async {}

  @override
  Future<bool> nudgeOpened() async => false;

  @override
  Future<bool> changesOpened() async => false;

  @override
  Future<void> showAgenda(AgendaSnapshot snapshot) async {}

  @override
  Future<bool> unsortedOpened() async => false;

  @override
  Future<List<AlarmAction>> pendingActions() async => const [];

  @override
  Future<void> acknowledge(int seq) async {}

  @override
  Stream<void> get actionsArrived => const Stream.empty();

  @override
  Future<Ringing?> ringing() async => null;

  @override
  Future<void> answerRing(Ringing ring, {Duration? snooze}) async {}

  @override
  Future<Map<AlarmPermission, bool>> permissions() async => const {};

  @override
  Future<void> request(AlarmPermission permission) async {}
}

/// Chaos Alert's Kotlin alarm layer behind the app's method channel.
class AndroidAlarms implements AlarmPlatform {
  static const _channel = MethodChannel('io.github.paxel.oldbrainfix/alarms');
  final _actions = StreamController<void>.broadcast();

  AndroidAlarms() {
    _channel.setMethodCallHandler((call) async {
      if (call.method == 'actions') _actions.add(null);
    });
  }

  @override
  bool get available => true;

  @override
  Future<void> arm(
    List<ArmedAlarm> alarms,
    AlarmLabels labels, {
    List<(String, Occurrence)> done = const [],
    ArmedNudge? nudge,
    ArmedChangesNudge? changes,
  }) => _channel.invokeMethod('arm', {
    'alarms': [
      for (final ArmedAlarm(:alarm, :title, :body) in alarms)
        {
          'id': alarm.id,
          'routine': alarm.routineId,
          'occurrence': alarm.occurrence.key,
          if (alarm.wall case final wall?)
            // Wall-clock fields, so Android re-arms at the same local
            // time after a time-zone or clock change.
            'local': [
              wall.day.year,
              wall.day.month,
              wall.day.day,
              wall.time.hour,
              wall.time.minute,
            ]
          else
            'at': alarm.at.millisecondsSinceEpoch,
          'renag': alarm.renag.inMinutes,
          'escalate': ?alarm.escalation,
          // On the wall clock as well, like 'local'.
          'end': [
            alarm.ends.day.year,
            alarm.ends.day.month,
            alarm.ends.day.day,
            alarm.ends.time.hour,
            alarm.ends.time.minute,
          ],
          'title': title,
          'body': body,
        },
    ],
    'labels': labels.toMap(),
    'done': [
      for (final (routine, occurrence) in done)
        {'routine': routine, 'occurrence': occurrence.key},
    ],
    if (nudge case ArmedNudge(:final nudge, :final title, :final body))
      'nudge': {
        'anchors': [
          for (final (:due, :captured) in nudge.anchors)
            [due.millisecondsSinceEpoch, captured.millisecondsSinceEpoch],
        ],
        if (nudge.snooze case (:final at, :final until)) ...{
          'snoozeAt': at.millisecondsSinceEpoch,
          'snoozeUntil': until.millisecondsSinceEpoch,
        },
        'repeat': nudge.repeat.inMinutes,
        // On the wall clock, so they hold in whatever zone the phone is.
        if (nudge.quiet case final quiet?)
          'quiet': [
            quiet.start.hour,
            quiet.start.minute,
            quiet.end.hour,
            quiet.end.minute,
          ],
        'since': ?nudge.since?.millisecondsSinceEpoch,
        'title': title,
        'body': body,
      },
    if (changes case ArmedChangesNudge(
      nudge: final changes,
      :final title,
      :final body,
      :final public,
    ))
      // Followed like the Nudge, without a snooze.
      'changes': {
        'anchors': [
          for (final (:due, :captured) in changes.anchors)
            [due.millisecondsSinceEpoch, captured.millisecondsSinceEpoch],
        ],
        'repeat': changes.repeat.inMinutes,
        if (changes.quiet case final quiet?)
          'quiet': [
            quiet.start.hour,
            quiet.start.minute,
            quiet.end.hour,
            quiet.end.minute,
          ],
        'since': ?changes.since?.millisecondsSinceEpoch,
        'title': title,
        'body': body,
        'public': public,
      },
  });

  @override
  Future<bool> nudgeOpened() async =>
      await _channel.invokeMethod<bool>('nudgeOpened') ?? false;

  @override
  Future<bool> changesOpened() async =>
      await _channel.invokeMethod<bool>('changesOpened') ?? false;

  @override
  Future<void> showAgenda(AgendaSnapshot snapshot) =>
      _channel.invokeMethod('agenda', snapshot.toMap());

  @override
  Future<bool> unsortedOpened() async =>
      await _channel.invokeMethod<bool>('unsortedOpened') ?? false;

  @override
  Future<List<AlarmAction>> pendingActions() async {
    final raw = await _channel.invokeListMethod<Map>('pending') ?? const [];
    return [for (final m in raw) ?_action(m)];
  }

  @override
  Future<void> acknowledge(int seq) => _channel.invokeMethod('ack', seq);

  @override
  Stream<void> get actionsArrived => _actions.stream;

  @override
  Future<Ringing?> ringing() async {
    final raw = await _channel.invokeMapMethod<String, Object?>('ringing');
    if (raw == null) return null;
    final (routine, occurrence, title, body) = (
      raw['routine'],
      raw['occurrence'],
      raw['title'],
      raw['body'],
    );
    if (routine is! String || occurrence is! String) return null;
    return Ringing(
      routineId: routine,
      occurrenceKey: occurrence,
      title: title is String ? title : '',
      body: body is String ? body : '',
    );
  }

  @override
  Future<void> answerRing(Ringing ring, {Duration? snooze}) =>
      _channel.invokeMethod('answerRing', {
        'routine': ring.routineId,
        'occurrence': ring.occurrenceKey,
        'title': ring.title,
        'body': ring.body,
        'snooze': ?snooze?.inMinutes,
      });

  @override
  Future<Map<AlarmPermission, bool>> permissions() async {
    final raw =
        await _channel.invokeMapMethod<String, bool>('permissions') ?? const {};
    return {for (final p in AlarmPermission.values) p: raw[p.name] ?? false};
  }

  @override
  Future<void> request(AlarmPermission permission) =>
      _channel.invokeMethod('request', permission.name);

  static AlarmAction? _action(Map<Object?, Object?> m) {
    final seq = m['seq'];
    final routine = m['routine'];
    final occurrence = m['occurrence'];
    final at = m['at'];
    if (seq is! int || routine is! String || occurrence is! String) {
      return null;
    }
    if (at is! int) return null;
    final tapped = DateTime.fromMillisecondsSinceEpoch(at, isUtc: true);
    return switch ((m['kind'], m['until'])) {
      ('nudgeSnooze', final int until) => NudgeSnoozed(
        seq,
        tapped,
        DateTime.fromMillisecondsSinceEpoch(until, isUtc: true),
      ),
      ('done', _) => DoneTapped(seq, routine, occurrence, tapped),
      ('snooze', final int until) => SnoozeTapped(
        seq,
        routine,
        occurrence,
        tapped,
        DateTime.fromMillisecondsSinceEpoch(until, isUtc: true),
      ),
      _ => null,
    };
  }
}
