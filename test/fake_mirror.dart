import 'package:oldbrainfix/src/calendar_mirror.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// One event in a [FakeMirror] calendar; [uid] is the mirror's mark,
/// null for an event someone else wrote.
typedef FakeEvent = ({
  String calendar,
  String? uid,
  String title,
  Occurrence? start,
  String? rule,
});

/// A [MirrorPlatform] the test drives: calendars holding events, the
/// mirror's marked by their UID, and calendar access to allow or refuse.
class FakeMirror implements MirrorPlatform {
  /// Whether calendar access is allowed.
  bool allowed;

  /// What the user answers when asked for calendar access.
  bool allowOnRequest;

  /// How often the user was asked.
  var requests = 0;

  /// The calendars on the Device.
  final List<DeviceCalendar> deviceCalendars;

  /// Every event in every calendar, by id.
  final events = <String, FakeEvent>{};

  var _nextId = 1;

  FakeMirror({
    this.allowed = false,
    this.allowOnRequest = true,
    List<DeviceCalendar>? calendars,
  }) : deviceCalendars =
           calendars ??
           [
             (id: '1', name: 'Personal', account: 'me@example.org'),
             (id: '2', name: 'Family', account: 'me@example.org'),
           ];

  /// Adds an event someone else wrote; returns its id.
  String addForeign(String calendar, String title) {
    final id = 'f${_nextId++}';
    events[id] = (
      calendar: calendar,
      uid: null,
      title: title,
      start: null,
      rule: null,
    );
    return id;
  }

  /// The events the mirror wrote into [calendar], by UID.
  Map<String, FakeEvent> mirroredIn(String calendar) => {
    for (final event in events.values)
      if (event.uid case final uid? when event.calendar == calendar) uid: event,
  };

  /// The id of the mirror's event [uid].
  String? idOf(String uid) => [
    for (final MapEntry(:key, :value) in events.entries)
      if (value.uid == uid) key,
  ].firstOrNull;

  @override
  bool get available => true;

  @override
  Future<bool> granted() async => allowed;

  @override
  Future<bool> request() async {
    requests++;
    allowed = allowOnRequest;
    return allowed;
  }

  @override
  Future<String> zone() async => 'Europe/Berlin';

  @override
  Future<List<DeviceCalendar>> calendars() async =>
      allowed ? deviceCalendars : const [];

  @override
  Future<({Map<String, String>? events})?> marked(String calendarId) async {
    if (!allowed) return null;
    if (!deviceCalendars.any((c) => c.id == calendarId)) return (events: null);
    return (
      events: {
        for (final MapEntry(:key, :value) in events.entries)
          if (value.uid case final uid? when value.calendar == calendarId)
            key: uid,
      },
    );
  }

  @override
  Future<MirrorApplied?> apply(String calendarId, MirrorPlan plan) async {
    if (!allowed) return null;
    bool ours(String id) =>
        events[id]?.calendar == calendarId && events[id]?.uid != null;
    FakeEvent row(CalendarEvent event) => (
      calendar: calendarId,
      uid: event.uid,
      title: event.summary,
      start: event.start,
      rule: event.rule,
    );
    for (final id in plan.delete) {
      if (ours(id)) events.remove(id);
    }
    final updated = <String?>[];
    for (final (id, event) in plan.update) {
      if (!ours(id)) {
        updated.add(null);
        continue;
      }
      events[id] = row(event);
      updated.add(id);
    }
    final created = <String?>[];
    for (final event in plan.create) {
      final id = '${_nextId++}';
      events[id] = row(event);
      created.add(id);
    }
    return (updated: updated, created: created, deleted: true);
  }
}
