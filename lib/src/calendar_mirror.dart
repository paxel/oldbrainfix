import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'app_lock.dart';
import 'l10n.dart';

/// A calendar on the Device that events can be written into.
typedef DeviceCalendar = ({String id, String name, String account});

/// What the calendar answered to a plan: the ids the updated and created
/// events have now, in the plan's order (null for one that failed), and
/// whether every delete went through.
typedef MirrorApplied = ({
  List<String?> updated,
  List<String?> created,
  bool deleted,
});

/// The platform seam for the calendar mirror: the Device's calendars and
/// the events the mirror wrote into one of them
/// (android/…/CalendarMirror.kt). Widget tests hand in a fake.
abstract interface class MirrorPlatform {
  /// Whether this Device has calendars to mirror into; the switch only
  /// shows where it has.
  bool get available;

  /// Whether the user allowed reading and writing calendars. Never asks.
  Future<bool> granted();

  /// Asks the user to allow reading and writing calendars; whether it is
  /// allowed now.
  Future<bool> request();

  /// The Device's time zone, which the calendar keeps wall clocks in.
  Future<String> zone();

  /// The calendars events can be written into.
  Future<List<DeviceCalendar>> calendars();

  /// The events the mirror wrote into [calendarId], each id to its UID,
  /// or null in the record when the calendar is gone; null when the
  /// calendar could not tell.
  Future<({Map<String, String>? events})?> marked(String calendarId);

  /// Carries out [plan] in [calendarId]; null when the calendar could
  /// not be asked.
  Future<MirrorApplied?> apply(String calendarId, MirrorPlan plan);
}

/// No calendars: the default where none is handed in.
class NoMirror implements MirrorPlatform {
  const NoMirror();

  @override
  bool get available => false;

  @override
  Future<bool> granted() async => false;

  @override
  Future<bool> request() async => false;

  @override
  Future<String> zone() async => '';

  @override
  Future<List<DeviceCalendar>> calendars() async => const [];

  @override
  Future<({Map<String, String>? events})?> marked(String calendarId) async =>
      null;

  @override
  Future<MirrorApplied?> apply(String calendarId, MirrorPlan plan) async =>
      null;
}

/// Android's calendar provider behind the app's method channel; no
/// Google library.
class AndroidMirror implements MirrorPlatform {
  static const _channel = MethodChannel('io.github.paxel.oldbrainfix/calendar');

  const AndroidMirror();

  @override
  bool get available => true;

  @override
  Future<bool> granted() async =>
      await _call<bool>('mirrorGranted', null) ?? false;

  @override
  Future<bool> request() async =>
      await _call<bool>('mirrorRequest', null) ?? false;

  @override
  Future<String> zone() async => await _call<String>('zone', null) ?? '';

  @override
  Future<List<DeviceCalendar>> calendars() async {
    final rows = await _call<List<Object?>>('calendars', null) ?? const [];
    return [
      for (final row in rows)
        if (row case {
          'id': final String id,
          'name': final String name,
          'account': final String account,
        })
          (id: id, name: name, account: account),
    ];
  }

  @override
  Future<({Map<String, String>? events})?> marked(String calendarId) async {
    final answer = await _call<Map<Object?, Object?>>('marked', {
      'calendar': calendarId,
    });
    return switch (answer) {
      {'gone': true} => (events: null),
      {'events': final Map<Object?, Object?> events} => (
        events: {
          for (final MapEntry(:key, :value) in events.entries)
            if ((key, value) case (final String id, final String uid)) id: uid,
        },
      ),
      _ => null,
    };
  }

  @override
  Future<MirrorApplied?> apply(String calendarId, MirrorPlan plan) async {
    Map<String, Object?> row(CalendarEvent event) => {
      'uid': event.uid,
      'title': event.summary,
      'description': event.description,
      'start': event.start?.key,
      'rule': event.rule,
    };
    final answer = await _call<Map<Object?, Object?>>('apply', {
      'calendar': calendarId,
      'create': [for (final event in plan.create) row(event)],
      'update': [
        for (final (id, event) in plan.update) {...row(event), 'id': id},
      ],
      'delete': plan.delete,
      'minutes': CalendarEvent.length.inMinutes,
    });
    if (answer case {
      'updated': final List<Object?> updated,
      'created': final List<Object?> created,
      'deleted': final bool deleted,
    }) {
      return (
        updated: [for (final id in updated) id is String ? id : null],
        created: [for (final id in created) id is String ? id : null],
        deleted: deleted,
      );
    }
    return null;
  }

  static Future<T?> _call<T>(String method, Object? args) async {
    try {
      return await _channel.invokeMethod<T>(method, args);
    } on PlatformException {
      return null;
    }
  }
}

/// Keeps the Routines and Reminders as events in the calendar the user
/// chose on this Device: on start, a moment after every change to the
/// log, and whenever the app comes back. Passes run one after the
/// other, never side by side, and off the UI thread on the platform's
/// side. Tells its listeners when the mirror was turned on or off,
/// including when its calendar went away.
class CalendarMirror extends ChangeNotifier {
  /// How long the mirror waits after a change for more to come.
  static const defaultDelay = Duration(seconds: 2);

  final Store store;
  final MirrorPlatform platform;
  final Duration delay;

  AppLocalizations? _t;
  Timer? _timer;
  Future<void> _tail = Future.value();
  var _disposed = false;

  CalendarMirror({
    required this.store,
    required this.platform,
    this.delay = defaultDelay,
  }) {
    store.addListener(_changed);
  }

  /// The calendar mirrored into; null while the mirror is off.
  MirrorCalendar? get calendar => store.mirrorCalendar;

  /// The texts the events are titled with; the first ones start the
  /// mirror.
  void setTexts(AppLocalizations t) {
    final first = _t == null;
    final changed = t != _t;
    _t = t;
    if (first) {
      unawaited(sync());
    } else if (changed) {
      _changed();
    }
  }

  /// Brings the calendar in step with the Routines and Reminders now.
  Future<void> sync() => _serial(_sync);

  /// Mirrors into [chosen] from now on: the events go there, and leave
  /// the calendar mirrored into before. Returns false when they could
  /// not be removed from there; the mirror stays as it was then.
  Future<bool> turnOn(MirrorCalendar chosen) => _serial(() async {
    final before = store.mirrorCalendar;
    if (before != null && before.id != chosen.id) {
      if (!await _removeAll(before)) return false;
    }
    if (_disposed) return false;
    if (before?.id != chosen.id) store.mirroredEvents = const {};
    store.mirrorCalendar = chosen;
    notifyListeners();
    await _sync();
    return true;
  });

  /// Removes every event the mirror wrote and turns it off. Returns false
  /// when they could not be removed; the mirror stays on then, to try
  /// again.
  Future<bool> turnOff() => _serial(() async {
    final before = store.mirrorCalendar;
    if (before != null && !await _removeAll(before)) return false;
    if (_disposed) return false;
    store
      ..mirrorCalendar = null
      ..mirroredEvents = const {};
    notifyListeners();
    return true;
  });

  void _changed() {
    if (_disposed || !platform.available || store.mirrorCalendar == null) {
      return;
    }
    _timer?.cancel();
    _timer = Timer(delay, () => unawaited(sync()));
  }

  Future<T> _serial<T>(Future<T> Function() work) {
    final next = _tail.then((_) => work());
    _tail = next.then<void>((_) {}, onError: (Object _) {});
    return next;
  }

  /// The text an event of [routine] is titled with: its Note's, or the
  /// masked text when it comes from a locked Area set to Masked, as in
  /// alarms and on the Agenda.
  String _textOf(AppLocalizations t, Routines routines, Routine routine) {
    if (AreaLocks(store).masks(routine)) return maskedText(t, '', routine);
    final note = routines.note(routine);
    return note == null ? t.noteGone : noteText(t, note);
  }

  Future<void> _sync() async {
    final t = _t;
    final calendar = store.mirrorCalendar;
    if (t == null || calendar == null || _disposed || !platform.available) {
      return;
    }
    if (!await platform.granted()) return;
    final zone = await platform.zone();
    final marked = await platform.marked(calendar.id);
    if (_disposed || marked == null) return;
    final events = marked.events;
    if (events == null) {
      // The calendar went, and its events with it.
      store
        ..mirrorCalendar = null
        ..mirroredEvents = const {};
      notifyListeners();
      return;
    }
    final routines = Routines(store);
    final plan = planMirror(
      desired: mirrorEvents(routines, (r) => _textOf(t, routines, r)),
      remembered: store.mirroredEvents,
      marked: events,
      zone: zone,
    );
    if (plan.isEmpty) {
      store.mirroredEvents = plan.keep;
      return;
    }
    final applied = await platform.apply(calendar.id, plan);
    // Not answered: what was written is marked, and the next pass takes
    // it over.
    if (_disposed || applied == null) return;
    store.mirroredEvents = plan.remembered(
      created: applied.created,
      updated: applied.updated,
      zone: zone,
    );
  }

  /// Removes every event the mirror wrote into [calendar], remembered or
  /// not; whether none is left.
  Future<bool> _removeAll(MirrorCalendar calendar) async {
    if (!platform.available || !await platform.granted()) return false;
    final marked = await platform.marked(calendar.id);
    if (marked == null) return false;
    final events = marked.events;
    if (events == null || events.isEmpty) return true;
    final applied = await platform.apply(
      calendar.id,
      MirrorPlan(delete: events.keys.toList()),
    );
    return applied?.deleted ?? false;
  }

  @override
  void dispose() {
    _disposed = true;
    _timer?.cancel();
    store.removeListener(_changed);
    super.dispose();
  }
}

/// Hands the app's [CalendarMirror] down to every screen.
class CalendarMirrorScope extends InheritedWidget {
  final CalendarMirror mirror;

  const CalendarMirrorScope({
    super.key,
    required this.mirror,
    required super.child,
  });

  /// The app's [CalendarMirror], or null where there is none; read once,
  /// so it does not listen.
  static CalendarMirror? of(BuildContext context) =>
      context.getInheritedWidgetOfExactType<CalendarMirrorScope>()?.mirror;

  @override
  bool updateShouldNotify(CalendarMirrorScope oldWidget) =>
      mirror != oldWidget.mirror;
}

/// Explains that the mirror writes into the calendar and touches only its
/// own events, then asks for calendar access when the user agrees.
/// Returns whether it is allowed now.
Future<bool> explainAndAskMirror(
  BuildContext context,
  MirrorPlatform platform,
) async {
  final agreed = await showDialog<bool>(
    context: context,
    builder: (context) {
      final t = context.t;
      return AlertDialog(
        icon: const Icon(Icons.event_repeat),
        title: Text(t.calendarMirrorExplainTitle),
        content: Text(t.calendarMirrorExplainBody),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: Text(t.notNow),
          ),
          TextButton(
            onPressed: () => Navigator.of(context).pop(true),
            child: Text(t.allowAccess),
          ),
        ],
      );
    },
  );
  if (agreed != true) return false;
  return platform.request();
}
