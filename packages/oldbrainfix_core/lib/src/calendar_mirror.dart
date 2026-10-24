/// The calendar mirror: Routines and Reminders kept as events in a
/// device calendar the user chose, created, updated and removed as they
/// change. This is the pure part: which events the calendar should hold,
/// what to do to get there, and what this Device remembers of the events
/// it wrote. The calendar itself is the app's.
library;

import 'dart:convert';

import 'ics.dart';
import 'routines.dart';
import 'store.dart';

/// An event the mirror wrote, as this Device remembers it.
class MirroredEvent {
  /// The calendar's own id of the event.
  final String eventId;

  /// What the event was written from ([mirrorPrint]); a different one
  /// means it needs writing again.
  final String print;

  const MirroredEvent(this.eventId, this.print);

  @override
  bool operator ==(Object other) =>
      other is MirroredEvent &&
      other.eventId == eventId &&
      other.print == print;

  @override
  int get hashCode => Object.hash(eventId, print);

  @override
  String toString() => 'MirroredEvent($eventId, $print)';
}

/// What [event] is written from in the Device's time [zone]: the zone
/// counts, as the calendar keeps the instants the wall clock meant there.
String mirrorPrint(CalendarEvent event, String zone) => jsonEncode([
  event.summary,
  event.description,
  event.start?.key,
  event.rule,
  zone,
]);

/// What one pass of the mirror does to the calendar.
class MirrorPlan {
  /// New events to write.
  final List<CalendarEvent> create;

  /// Events of the mirror to write again, by their calendar id.
  final List<(String, CalendarEvent)> update;

  /// Events of the mirror to remove, by their calendar id.
  final List<String> delete;

  /// What is remembered of the events that stay as they are, by UID.
  final Map<String, MirroredEvent> keep;

  const MirrorPlan({
    this.create = const [],
    this.update = const [],
    this.delete = const [],
    this.keep = const {},
  });

  /// Whether the calendar holds what it should already.
  bool get isEmpty => create.isEmpty && update.isEmpty && delete.isEmpty;

  /// What to remember once the plan was carried out in [zone]: the kept
  /// events, and those [created] and [updated] under the calendar ids
  /// they got, in the order of [create] and [update]. An event that
  /// could not be written (null) is not remembered; the next pass tries
  /// again.
  Map<String, MirroredEvent> remembered({
    required List<String?> created,
    required List<String?> updated,
    required String zone,
  }) => {
    ...keep,
    for (final (i, event) in create.indexed)
      if (created.elementAtOrNull(i) case final id?)
        event.uid: MirroredEvent(id, mirrorPrint(event, zone)),
    for (final (i, (_, event)) in update.indexed)
      if (updated.elementAtOrNull(i) case final id?)
        event.uid: MirroredEvent(id, mirrorPrint(event, zone)),
  };
}

/// The plan that turns the mirror's events in the calendar, [marked]
/// (the calendar id of each event the mirror wrote, to its UID), into
/// [desired], with what this Device [remembered] by UID, in the Device's
/// time [zone].
///
/// Only [marked] events are ever updated or removed: an event the mirror
/// did not write is never touched, and a remembered one no longer marked
/// is gone. An event with a time is written again when it changed; a
/// marked one nothing remembers (a pass cut short) is taken over instead
/// of doubled; a second one for the same UID goes, as do those of UIDs
/// no longer desired. Events without a start have no place in the
/// calendar.
MirrorPlan planMirror({
  required Iterable<CalendarEvent> desired,
  required Map<String, MirroredEvent> remembered,
  required Map<String, String> marked,
  required String zone,
}) {
  final byUid = <String, List<String>>{};
  for (final MapEntry(key: id, value: uid) in marked.entries) {
    (byUid[uid] ??= []).add(id);
  }
  final create = <CalendarEvent>[];
  final update = <(String, CalendarEvent)>[];
  final delete = <String>[];
  final keep = <String, MirroredEvent>{};
  for (final event in desired) {
    if (event.start == null) continue;
    final ids = byUid.remove(event.uid) ?? const [];
    final known = remembered[event.uid];
    final id = known != null && ids.contains(known.eventId)
        ? known.eventId
        : ids.firstOrNull;
    if (id == null) {
      create.add(event);
      continue;
    }
    delete.addAll(ids.where((other) => other != id));
    if (known != null &&
        known.eventId == id &&
        known.print == mirrorPrint(event, zone)) {
      keep[event.uid] = known;
    } else {
      update.add((id, event));
    }
  }
  for (final ids in byUid.values) {
    delete.addAll(ids);
  }
  return MirrorPlan(create: create, update: update, delete: delete, keep: keep);
}

/// The events the calendar should hold: those of every Routine and
/// Reminder not in the trash, each titled by [textOf].
List<CalendarEvent> mirrorEvents(
  Routines routines,
  String Function(Routine routine) textOf,
) => [
  for (final routine in routines.all())
    ...routineEvents(routine, textOf(routine)),
];

/// The device calendar the mirror writes into.
typedef MirrorCalendar = ({String id, String name});

/// The mirror's settings and memory on this Device, never synced (ADR
/// 0008): each Device mirrors into a calendar of its own choosing.
extension CalendarMirrorSettings on Store {
  static const _calendar = 'calendarMirror';
  static const _events = 'calendarMirror:events';

  /// The calendar Routines and Reminders are mirrored into; null while
  /// the mirror is off, as it is until the user turns it on.
  MirrorCalendar? get mirrorCalendar {
    final raw = localSetting(_calendar);
    if (raw == null || raw.isEmpty) return null;
    try {
      if (jsonDecode(raw) case {
        'id': final String id,
        'name': final String name,
      }) {
        return (id: id, name: name);
      }
    } on FormatException {
      // A broken setting is no calendar.
    }
    return null;
  }

  set mirrorCalendar(MirrorCalendar? calendar) => setLocalSetting(
    _calendar,
    calendar == null
        ? ''
        : jsonEncode({'id': calendar.id, 'name': calendar.name}),
  );

  /// The events the mirror wrote into [mirrorCalendar], by UID.
  Map<String, MirroredEvent> get mirroredEvents {
    final raw = localSetting(_events);
    if (raw == null || raw.isEmpty) return {};
    try {
      final decoded = jsonDecode(raw);
      if (decoded is! Map) return {};
      return {
        for (final MapEntry(:key, :value) in decoded.entries)
          if (value case [final String id, final String print])
            '$key': MirroredEvent(id, print),
      };
    } on FormatException {
      return {};
    }
  }

  set mirroredEvents(Map<String, MirroredEvent> events) => setLocalSetting(
    _events,
    jsonEncode({
      for (final MapEntry(:key, :value) in events.entries)
        key: [value.eventId, value.print],
    }),
  );
}
