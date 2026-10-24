/// Writes the fixture corpus the desktop core is tested against.
///
///     dart run tool/gen_fixtures.dart ../../desktop/fixtures
///
/// Every scenario fills an in-memory store with a fixed signing key, a
/// fixed clock and counted ids, and writes its log (`log.jsonl`, one
/// entry per line in log order) and what the core reads from it
/// (`expected.json`). The desktop core loads each log into its own store
/// and must read the same. The Routine scenario adds what the alarm plan,
/// the Missed Summary, the Nudge and the Agenda come to at a few held
/// moments (`alarms.json`), on a European wall clock. Next to them: the
/// store's schema, a signed entry with awkward text, the question-mark
/// scatter, and RRULEs with their Occurrences and the instants of
/// daylight-saving days (`schedules.json`).
library;

import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:sqlite3/sqlite3.dart';

/// The writer's fixed key seed: bytes 1 to 32.
final _seed = Uint8List.fromList(List<int>.generate(32, (i) => 1 + i));

/// A clock that starts at a fixed moment and moves one second per read,
/// unless it is held still for reading the results.
late DateTime _tick;
DateTime? _held;
DateTime _clock() => _held ?? (_tick = _tick.add(const Duration(seconds: 1)));

typedef Scenario = ({String name, String about, void Function(Store) build});

/// The wall clock of Central Europe, which Europe/Berlin keeps since
/// 1996: summer time (+2) from the last Sunday of March 01:00 UTC to the
/// last Sunday of October 01:00 UTC, winter time (+1) around it.
DateTime _europe(DateTime utc) {
  DateTime lastSunday(int year, int month) {
    final last = DateTime.utc(year, month + 1, 0);
    return DateTime.utc(year, month, last.day - last.weekday % 7, 1);
  }

  final u = utc.toUtc();
  final summer =
      !u.isBefore(lastSunday(u.year, 3)) && u.isBefore(lastSunday(u.year, 10));
  final wall = u.add(Duration(hours: summer ? 2 : 1));
  return DateTime.utc(
    wall.year,
    wall.month,
    wall.day,
    wall.hour,
    wall.minute,
    wall.second,
    wall.millisecond,
  );
}

ClockTime _at(int hour, [int minute = 0]) => ClockTime(hour, minute);

final _scenarios = <Scenario>[
  (
    name: 'notes',
    about:
        'Notes of every Importance; a text fixed in place; an edit dated '
        'back that loses to the text before it; two texts half a '
        'millisecond apart written in reverse order; text with quotes, '
        'backslashes, control characters, umlauts and an emoji.',
    build: (store) {
      final notes = Notes(store);
      final a = notes.capture('Buy more sauce');
      notes.setImportance(a.id, Importance.important);
      final b = notes.capture('nees mor sauce');
      notes.edit(b.id, 'Needs more sauce');
      final c = notes.capture('Maybe paint the shed');
      notes.setImportance(c.id, Importance.unimportant);
      notes.setImportance(c.id, Importance.normal);
      notes.setImportance(c.id, Importance.unimportant);
      final d = notes.capture('First text');
      store.append(
        d.id,
        NoteKeys.text,
        'Dated back',
        date: DateTime.utc(2025, 12, 31),
      );
      final e = notes.capture('Sub-millisecond');
      store.append(
        e.id,
        NoteKeys.text,
        'Half a millisecond later',
        date: DateTime.utc(2026, 1, 2, 9, 0, 0, 0, 500),
      );
      store.append(
        e.id,
        NoteKeys.text,
        'Earlier, written later',
        date: DateTime.utc(2026, 1, 2, 9),
      );
      notes.capture(_awkward);
    },
  ),
  (
    name: 'sections',
    about:
        'Amends; a chain of Merges where the merged Note had an Amend of '
        'its own; two Notes merged into each other, kept under the lower '
        'id; a Merge into a Note this Device does not hold.',
    build: (store) {
      final notes = Notes(store);
      final a = notes.capture('Dentist on Tuesday');
      notes.amend(a.id, 'Moved to Wednesday');
      final b = notes.capture('Ask about the crown');
      notes.amend(b.id, 'Bring the old X-ray');
      final c = notes.capture('Dentist number');
      notes.merge(b.id, a.id);
      notes.merge(a.id, c.id);
      notes.amend(c.id, 'Called, all set');
      final x = notes.capture('Loop one');
      final y = notes.capture('Loop two');
      store.append(y.id, NoteKeys.merged, x.id);
      store.append(x.id, NoteKeys.merged, y.id);
      final z = notes.capture('Merged into a stranger');
      store.append(z.id, NoteKeys.merged, 'note:elsewhere');
    },
  ),
  (
    name: 'places',
    about:
        'The whole Starter Set planted, one starter Area renamed; an Area '
        'of the user with a colour and an icon and a Topic; Notes filed '
        'into Areas and Topics; a Note whose Topic vanished falls back to '
        'Unsorted; a deleted Area takes its Topics with it.',
    build: (store) {
      final places = Places(store);
      final notes = Notes(store);
      places.plantStarterSet({
        for (final area in starterSet) ...{
          area.slug,
          for (final topic in area.topics) topic.slug,
        },
      });
      final car = places.areas().firstWhere((a) => a.starter == 'car');
      places.rename(car.id, 'Bike');
      final kids = places.addArea('Kids');
      places.setColour(kids.id, '6A1B9A');
      places.setIcon(kids.id, 'kids');
      final school = places.addTopic(kids.id, 'School');
      final household = places.areas().firstWhere(
        (a) => a.starter == 'household',
      );
      final kitchen = places
          .topics(household.id)
          .firstWhere((t) => t.starter == 'kitchen');
      notes.capture('Descale the kettle', place: kitchen.id);
      notes.capture('Sort the cellar', place: household.id);
      notes.capture('Parents evening', place: school.id);
      notes.capture('New tyres', place: car.id);
      final lost = notes.capture('Where did this go?', place: school.id);
      store.append(school.id, PlaceKeys.type, null);
      notes.capture('Still Unsorted');
      final gone = places.addArea('Gone');
      places.addTopic(gone.id, 'Gone too');
      places.deleteArea(gone.id, NotesGo.unsorted);
      notes.setImportance(lost.id, Importance.important);
    },
  ),
  (
    name: 'trash',
    about:
        'A Note in the trash; one trashed and restored; one trashed long '
        'enough ago to be past restoring; a merged Note whose target is '
        'in the trash.',
    build: (store) {
      final notes = Notes(store);
      final a = notes.capture('Old shopping list');
      notes.trash(a.id);
      final b = notes.capture('Not so old after all');
      notes.trash(b.id);
      notes.restore(b.id);
      final c = notes.capture('Long gone');
      store.append(
        c.id,
        NoteKeys.trashed,
        DateTime.utc(2025, 11, 1).toIso8601String(),
      );
      final d = notes.capture('Into the bin with the other');
      final e = notes.capture('The other');
      notes.merge(d.id, e.id);
      notes.trash(e.id);
    },
  ),
  (
    name: 'edits',
    about:
        'What the desktop writes beyond the other scenarios: Areas and '
        'Topics made and renamed, an Area coloured and given an icon, '
        'another one with both cleared again; Notes captured with each '
        'Nudge time, some into a place; Notes filed and filed again; Topics '
        'deleted with their Notes sent back to Unsorted (one that spawned a '
        'Routine included), into the Area and into the trash, and an empty '
        'one; Areas deleted with their Topics, their Notes sent into the '
        'trash and back to Unsorted.',
    build: (store) {
      final notes = Notes(store);
      final places = Places(store);
      final home = places.addArea('Hom');
      places.rename(home.id, 'Home');
      places.setColour(home.id, '00838F');
      places.setIcon(home.id, 'home');
      final post = places.addTopic(home.id, 'Post');
      final bills = places.addTopic(home.id, 'Bils');
      places.rename(bills.id, 'Bills');
      final junk = places.addTopic(home.id, 'Junk');
      final empty = places.addTopic(home.id, 'Empty');
      final errands = places.addArea('Errands');
      places.setColour(errands.id, 'C62828');
      places.setIcon(errands.id, 'shopping');
      places.setColour(errands.id, null);
      places.setIcon(errands.id, null);
      final shop = places.addTopic(errands.id, 'Shop');
      final garage = places.addArea('Garage');
      final tools = places.addTopic(garage.id, 'Tools');
      final plumber = notes.capture(
        'Call the plumber',
        nudgeIn: const Duration(minutes: 10),
      );
      final plants = notes.capture(
        'Water the plants',
        nudgeIn: const Duration(minutes: 30),
      );
      final passport = notes.capture(
        'Renew the passport',
        nudgeIn: Notes.defaultNudge,
      );
      final hour = Notes.defaultNudge;
      notes.capture('Pay the rent', place: bills.id, nudgeIn: hour);
      notes.capture('Old flyer', place: junk.id, nudgeIn: hour);
      notes.capture('Buy stamps', place: shop.id, nudgeIn: hour);
      notes.capture('Return the drill', place: errands.id, nudgeIn: hour);
      notes.capture('Oil the saw', place: tools.id, nudgeIn: hour);
      notes.capture('Sweep the floor', place: garage.id, nudgeIn: hour);
      notes.file(plumber.id, post.id);
      notes.file(plumber.id, bills.id);
      notes.file(plants.id, post.id);
      notes.file(passport.id, post.id);
      // A Routine was spawned from it on another Device.
      store.append(
        passport.id,
        NoteKeys.spawned,
        DateTime.utc(2026, 1, 1, 12).toIso8601String(),
      );
      places.deleteTopic(post.id, NotesGo.unsorted);
      places.deleteTopic(bills.id, NotesGo.area);
      places.deleteTopic(junk.id, NotesGo.trash);
      places.deleteTopic(empty.id, NotesGo.unsorted);
      places.deleteArea(errands.id, NotesGo.trash);
      places.deleteArea(garage.id, NotesGo.unsorted);
    },
  ),
  (
    name: 'routines',
    about:
        'Routines and Reminders spawned from Notes and made with their '
        'Note behind the scenes: several times a day with its own Re-nag '
        'and Escalation, all-day weekends, Reminders on the hour the clock '
        'skips and the one it shows twice, every other day, the last of '
        'the month rescheduled to two times, a Re-nag set and cleared, an '
        'unchanged reschedule that writes nothing; Occurrences Done, Done '
        'at a given time and unchecked; a Routine trashed, another trashed '
        'and restored; Notes trashed with their Routines and restored, '
        'trashed without them, and trashed with them for good; Unsorted '
        'Notes for the Nudge.',
    build: (store) {
      final notes = Notes(store);
      final routines = Routines(store);
      final pills = notes.capture('Take the pills');
      final medication = routines.spawn(
        pills.id,
        Schedule(
          start: Day(2026, 1, 1),
          times: [_at(20), _at(8), _at(14)],
          rule: Recurrence(frequency: Frequency.daily),
        ),
      );
      routines.setRenag(medication.id, const Duration(minutes: 5));
      routines.setEscalation(medication.id, 3);
      routines.spawn(
        pills.id,
        Schedule(
          start: Day(2026, 3, 1),
          rule: Recurrence.parse('FREQ=MONTHLY;BYDAY=1SU'),
        ),
      );
      final plants = notes.capture(
        'Water the plants',
        nudgeIn: const Duration(minutes: 30),
      );
      final weekends = Schedule(
        start: Day(2026, 1, 3),
        rule: Recurrence.parse('FREQ=WEEKLY;BYDAY=SA,SU'),
      );
      final weekend = routines.spawn(plants.id, weekends);
      routines.reschedule(weekend.id, weekends);
      final dentist = notes.capture('Dentist');
      routines.spawn(
        dentist.id,
        Schedule(start: Day(2026, 3, 29), times: [_at(2, 30)]),
      );
      routines.spawn(
        dentist.id,
        Schedule(start: Day(2026, 10, 25), times: [_at(2, 30)]),
      );
      final night = routines.create(
        '  Night meds  ',
        Schedule(
          start: Day(2026, 1, 1),
          times: [_at(23)],
          rule: Recurrence.parse('FREQ=DAILY;INTERVAL=2'),
        ),
      );
      routines.setRenag(night.id, const Duration(minutes: 15));
      routines.setRenag(night.id, null);
      final monthEnd = Recurrence.parse('FREQ=MONTHLY;BYMONTHDAY=-1');
      final rent = routines.create(
        'Pay the rent',
        Schedule(start: Day(2026, 1, 31), times: [_at(9)], rule: monthEnd),
      );
      routines.reschedule(
        rent.id,
        Schedule(
          start: Day(2026, 1, 31),
          times: [_at(9), _at(18)],
          rule: monthEnd,
        ),
      );
      routines.markDone(medication.id, Occurrence(Day(2026, 3, 28), _at(8)));
      routines.markDone(
        medication.id,
        Occurrence(Day(2026, 3, 28), _at(14)),
        at: DateTime.utc(2026, 3, 28, 15),
      );
      routines.markDone(medication.id, Occurrence(Day(2026, 3, 29), _at(8)));
      routines.unmarkDone(medication.id, Occurrence(Day(2026, 3, 29), _at(8)));
      routines.markDone(weekend.id, Occurrence(Day(2026, 10, 24)));
      final habit = routines.create(
        'Old habit',
        Schedule(
          start: Day(2026, 1, 1),
          rule: Recurrence(frequency: Frequency.daily),
        ),
      );
      routines.trash(habit.id);
      final stretch = routines.create(
        'Stretch',
        Schedule(
          start: Day(2026, 1, 2),
          times: [_at(7, 30)],
          rule: Recurrence.parse('FREQ=WEEKLY;INTERVAL=2;BYDAY=MO,TH;WKST=SU'),
        ),
      );
      routines.trash(stretch.id);
      routines.restore(stretch.id);
      final walk = notes.capture('Walk the dog');
      routines.spawn(
        walk.id,
        Schedule(
          start: Day(2026, 1, 1),
          times: [_at(7)],
          rule: Recurrence(frequency: Frequency.daily),
        ),
      );
      routines.trashNote(walk.id, withRoutines: true);
      routines.restoreNote(walk.id);
      final flyer = notes.capture('Flyer for the fair');
      routines.spawn(
        flyer.id,
        Schedule(start: Day(2026, 4, 1), times: [_at(10)]),
      );
      routines.trashNote(flyer.id, withRoutines: false);
      final car = notes.capture('Car inspection');
      routines.spawn(car.id, Schedule(start: Day(2026, 5, 1)));
      routines.trashNote(car.id, withRoutines: true);
      notes.capture('Unsorted thought', nudgeIn: const Duration(minutes: 10));
      notes.capture('Another thought');
    },
  ),
];

/// A value that tests how the signed bytes escape text.
const _awkward =
    'Quote " backslash \\ slash / newline \n tab \t bell \u0001 '
    'umlaut ü emoji 🧠';

Map<String, Object?> _note(Note note) => {
  'id': note.id,
  'text': note.text,
  'captured': note.captured.toUtc().toIso8601String(),
  'place': note.place,
  'importance': note.importance.name,
  'trashed': note.trashed?.toUtc().toIso8601String(),
  'mergedInto': note.mergedInto,
  'spawned': note.spawned,
};

Map<String, Object?> _section(Section section) => {
  'id': section.id,
  'noteId': section.noteId,
  'text': section.text,
  'captured': section.context.captured.toUtc().toIso8601String(),
};

/// What the core reads from [store]: the desktop core must read the same.
Map<String, Object?> _expected(Store store) {
  final notes = Notes(store);
  final places = Places(store);
  final noteIds = store.entitiesOfType(NoteKeys.typeNote)..sort();
  final all = notes.all();
  final trashed = notes.trashed();
  final placeIds = [
    for (final area in places.areas()) ...[
      area.id,
      for (final topic in places.topics(area.id)) topic.id,
    ],
  ];
  return {
    'now': store.clock().toIso8601String(),
    'device': store.deviceId,
    'areas': [
      for (final area in places.areas())
        {
          'id': area.id,
          'name': area.name,
          'starter': area.starter,
          'colour': area.colour,
          'icon': area.icon,
          'topics': [
            for (final topic in places.topics(area.id))
              {
                'id': topic.id,
                'areaId': topic.areaId,
                'name': topic.name,
                'starter': topic.starter,
              },
          ],
        },
    ],
    'notes': [
      for (final id in noteIds)
        if (notes.byId(id) case final note?) _note(note),
    ],
    'all': [for (final note in all) note.id],
    'unsorted': [for (final note in notes.unsorted()) note.id],
    'trashed': [for (final note in trashed) note.id],
    'inside': {
      for (final id in placeIds) id: [for (final n in notes.inside(id)) n.id],
    },
    'sections': {
      for (final note in [...all, ...trashed])
        note.id: [for (final s in notes.sections(note.id)) _section(s)],
    },
  };
}

Map<String, Object?> _routine(Routine routine) => {
  'id': routine.id,
  'noteId': routine.noteId,
  'start': '${routine.schedule.start}',
  'times': [for (final t in routine.schedule.times) '$t'],
  'rule': routine.schedule.rule?.format(),
  'done': {
    for (final key in routine.done.keys.toList()..sort())
      key: routine.done[key]?.toUtc().toIso8601String(),
  },
  'renag': routine.renag?.inMinutes,
  'escalation': routine.escalation,
  'scheduledAt': routine.scheduledAt?.toUtc().toIso8601String(),
  'trashed': routine.trashed?.toUtc().toIso8601String(),
  'trashedWith': routine.trashedWith,
  'reminder': routine.isReminder,
};

/// How many alarms of each plan the fixture spells out, the soonest
/// first; the count covers them all.
const _alarmsShown = 40;

String _wall(WallMoment m) => '${m.day}T${m.time}';

/// What the Routines of [store] come to: the Routines themselves, then at
/// each held moment with this Device's settings as given, the Agenda, the
/// alarm plan, the Missed Summary and the Nudge, on [_europe].
Map<String, Object?> _alarms(Store store) {
  final routines = Routines(store);
  final ids = store.entitiesOfType(RoutineKeys.typeRoutine)..sort();
  String find(String text, {bool reminder = false}) => routines
      .all()
      .firstWhere(
        (r) => routines.note(r)?.text == text && r.isReminder == reminder,
      )
      .id;
  final medication = find('Take the pills');
  final weekend = find('Water the plants');
  final defaults = {
    'snoozes': '{}',
    'nudgeSnooze': '',
    'missedSeen': '',
    'renagMinutes': '',
    'quietHours': '',
    'quietHoursOff': '',
    'nudgeRepeatMinutes': '',
  };
  final checks = <(DateTime, Map<String, String>)>[
    // Saturday 22:30 in winter time: the Nudge waits for the morning
    // after the switch.
    (
      DateTime.utc(2026, 3, 28, 21, 30),
      {
        ...defaults,
        'missedSeen': DateTime.utc(2026, 3, 26).toIso8601String(),
        'snoozes': jsonEncode({
          '$medication@2026-03-28T20:00': DateTime.utc(
            2026,
            3,
            28,
            21,
            45,
          ).toIso8601String(),
        }),
      },
    ),
    // Sunday 09:00, just after the clock jumped; never seen, Re-nag and
    // repeat of this Device set.
    (
      DateTime.utc(2026, 3, 29, 7),
      {...defaults, 'renagMinutes': '20', 'nudgeRepeatMinutes': '30'},
    ),
    // In the hour the clock shows twice, the first time round; the Nudge
    // snoozed, Quiet Hours of its own.
    (
      DateTime.utc(2026, 10, 25, 0, 45),
      {
        ...defaults,
        'missedSeen': DateTime.utc(2026, 10, 23, 12).toIso8601String(),
        'quietHours': '23:30-06:15',
        'nudgeSnooze':
            '${DateTime.utc(2026, 10, 24, 20).toIso8601String()} '
            '${DateTime.utc(2026, 10, 25, 2).toIso8601String()}',
      },
    ),
    // Sunday noon after the switch, Quiet Hours off, a snooze past when
    // its Occurrence is Missed.
    (
      DateTime.utc(2026, 10, 25, 12),
      {
        ...defaults,
        'missedSeen': DateTime.utc(2026, 10, 24, 12).toIso8601String(),
        'quietHoursOff': 'true',
        'snoozes': jsonEncode({
          '$weekend@2026-10-25': DateTime.utc(
            2026,
            10,
            26,
            3,
          ).toIso8601String(),
          '$medication@2026-10-25T14:00': DateTime.utc(
            2026,
            10,
            25,
            13,
            20,
          ).toIso8601String(),
        }),
      },
    ),
    // Late on the last of the month, in winter time again.
    (
      DateTime.utc(2026, 10, 31, 22, 10),
      {...defaults, 'missedSeen': DateTime.utc(2026, 10, 30).toIso8601String()},
    ),
  ];
  List<Object?> rows(List<AgendaRow> rows) => [
    for (final row in rows)
      [
        row.routine.id,
        row.occurrence.key,
        row.note?.id,
        row.done?.toUtc().toIso8601String(),
      ],
  ];
  return {
    'routines': [
      for (final id in ids)
        if (routines.byId(id) case final routine?) _routine(routine),
    ],
    'all': [for (final r in routines.all()) r.id],
    'trashed': [for (final r in routines.trashed()) r.id],
    'ofNote': {
      for (final note in Notes(store).all())
        note.id: [for (final r in routines.ofNote(note.id)) r.id],
    },
    'checks': [
      for (final (now, local) in checks)
        () {
          _held = now;
          for (final MapEntry(:key, :value) in local.entries) {
            store.setLocalSetting(key, value);
          }
          final alarms = Alarms(store);
          final agenda = routines.agenda(_europe);
          final plan = alarms.plan(_europe);
          final missed = alarms.missed(_europe);
          final nudge = alarms.nudge(_europe);
          final later = now.add(const Duration(hours: 3));
          return {
            'now': now.toIso8601String(),
            'local': local,
            'today': '${agenda.today}',
            'agenda': {
              'today': rows(agenda.todayRows),
              'tomorrow': rows(agenda.tomorrowRows),
            },
            'alarmCount': plan.length,
            'alarms': [
              for (final a in plan.take(_alarmsShown))
                {
                  'id': a.id,
                  'kind': a.kind.name,
                  'routine': a.routineId,
                  'occurrence': a.occurrence.key,
                  'at': a.at.toUtc().toIso8601String(),
                  'wall': a.wall == null ? null : _wall(a.wall!),
                  'renag': a.renag.inMinutes,
                  'escalation': a.escalation,
                  'escalatesAt': a.escalatesAt?.toUtc().toIso8601String(),
                  'ends': _wall(a.ends),
                  'endsAt': a.endsAt.toUtc().toIso8601String(),
                  'text': a.text,
                  'firesAfterNow': a.firesAfter(now)?.toUtc().toIso8601String(),
                },
            ],
            'missed': {
              'count': missed.count,
              'nextChange': missed.nextChange?.toUtc().toIso8601String(),
              'routines': [
                for (final m in missed.routines)
                  {
                    'routine': m.routine.id,
                    'note': m.note?.id,
                    'missed': [
                      for (final o in m.missed)
                        [
                          o.occurrence.key,
                          o.missedAt.toUtc().toIso8601String(),
                        ],
                    ],
                  },
              ],
            },
            'nudge': nudge == null
                ? null
                : {
                    'notes': [for (final n in nudge.notes) n.id],
                    'at': nudge.at.toUtc().toIso8601String(),
                    'since': nudge.since?.toUtc().toIso8601String(),
                    'firesAfterLater': nudge
                        .firesAfter(later, _europe)
                        .toUtc()
                        .toIso8601String(),
                  },
            'recentlyDone': [
              for (final (id, occurrence) in alarms.recentlyDone(_europe))
                [id, occurrence.key],
            ],
          };
        }(),
    ],
  };
}

/// RRULEs as read and written back, refused with their reason, or as the
/// simple editor shows them; schedules with their Occurrences; days
/// normalised; and the instants a European clock shows wall-clock times
/// at around its daylight-saving switches.
Map<String, Object?> _schedules() {
  const rules = [
    'FREQ=DAILY',
    'rrule:freq=weekly;byday=mo,we,fr',
    'FREQ=WEEKLY;BYDAY=SA,MO',
    'FREQ=MONTHLY;BYDAY=1MO,-1FR',
    'FREQ=MONTHLY;BYDAY=+2TU',
    'FREQ=YEARLY;BYMONTH=2;BYMONTHDAY=29',
    'FREQ=MONTHLY;BYMONTHDAY=31',
    'FREQ=MONTHLY;INTERVAL=3;BYMONTHDAY=1,15,-1',
    'FREQ=WEEKLY;INTERVAL=2;BYDAY=TU,SA;WKST=SU',
    'FREQ=DAILY;COUNT=5',
    'FREQ=DAILY;UNTIL=20260110T120000Z',
    'FREQ=DAILY;UNTIL=20260105',
    'FREQ=DAILY;BYHOUR=20,8;BYMINUTE=30,0',
    'FREQ=DAILY;BYMINUTE=15',
    'FREQ=YEARLY;BYDAY=1MO',
    'FREQ=YEARLY;BYDAY=-1SU;BYMONTH=10,3',
    'FREQ=DAILY;BYDAY=SA,SU;BYMONTH=6,7,8',
    'FREQ=YEARLY',
    'FREQ=MONTHLY',
    'FREQ=WEEKLY;BYDAY=SU;BYMONTH=12',
    'FREQ=DAILY;INTERVAL=+3',
    'FREQ=DAILY;BYSECOND=0',
    'FREQ=DAILY;;INTERVAL=2;',
    '',
    'RRULE:',
    'FREQ',
    'FREQ=DAILY;FREQ=WEEKLY',
    'FREQ=DAILY;BYSETPOS=1',
    'INTERVAL=2',
    'FREQ=HOURLY',
    'FREQ=WEEKLY;BYDAY=1MO',
    'FREQ=MONTHLY;BYDAY=0MO',
    'FREQ=MONTHLY;BYDAY=54MO',
    'FREQ=MONTHLY;BYDAY=XX',
    'FREQ=MONTHLY;BYDAY=+MO',
    'FREQ=MONTHLY;BYDAY=',
    'FREQ=MONTHLY;BYMONTHDAY=0',
    'FREQ=MONTHLY;BYMONTHDAY=-0',
    'FREQ=MONTHLY;BYMONTHDAY=32',
    'FREQ=WEEKLY;BYMONTHDAY=3',
    'FREQ=DAILY;BYSECOND=5',
    'FREQ=DAILY;COUNT=2;UNTIL=20260101',
    'FREQ=DAILY;WKST=XY',
    'FREQ=DAILY;INTERVAL=0',
    'FREQ=DAILY;BYMONTH=13',
    'FREQ=DAILY;BYHOUR=24',
    'FREQ=DAILY;BYMINUTE=60',
    'FREQ=DAILY;COUNT=0',
    'FREQ=DAILY;UNTIL=20260230',
    'FREQ=DAILY;UNTIL=20260101T250000',
    'FREQ=DAILY;UNTIL=2026',
    'FREQ=DAILY;INTERVAL=two',
  ];
  Map<String, Object?>? simple(SimpleRepeat? simple) => simple == null
      ? null
      : {
          'kind': simple.kind.name,
          'every': simple.every,
          'unit': simple.unit.name,
          'weekdays': simple.weekdays.toList()..sort(),
          'rule': simple.toRule()?.format(),
        };
  final schedules =
      <(Day start, List<ClockTime> times, String? rule, Day from, Day to)>[
        (
          Day(2026, 1, 1),
          [_at(8), _at(20)],
          'FREQ=DAILY',
          Day(2026, 1, 1),
          Day(2026, 1, 4),
        ),
        (
          Day(2026, 1, 1),
          [],
          'FREQ=DAILY;INTERVAL=3',
          Day(2026, 1, 5),
          Day(2026, 1, 20),
        ),
        (
          Day(2026, 1, 31),
          [_at(9)],
          'FREQ=MONTHLY',
          Day(2026, 1, 1),
          Day(2026, 12, 31),
        ),
        (
          Day(2026, 1, 31),
          [_at(9)],
          'FREQ=MONTHLY;BYMONTHDAY=-1',
          Day(2026, 1, 1),
          Day(2026, 6, 30),
        ),
        (
          Day(2024, 2, 29),
          [],
          'FREQ=YEARLY',
          Day(2024, 1, 1),
          Day(2033, 12, 31),
        ),
        (
          Day(2026, 1, 1),
          [_at(7, 30)],
          'FREQ=WEEKLY;INTERVAL=2;BYDAY=TU,SA;WKST=SU',
          Day(2026, 1, 1),
          Day(2026, 2, 28),
        ),
        (
          Day(2026, 1, 1),
          [_at(7, 30)],
          'FREQ=WEEKLY;INTERVAL=2;BYDAY=TU,SA',
          Day(2026, 1, 1),
          Day(2026, 2, 28),
        ),
        (
          Day(2026, 1, 1),
          [],
          'FREQ=MONTHLY;BYDAY=1MO,-1FR',
          Day(2026, 1, 1),
          Day(2026, 5, 31),
        ),
        (
          Day(2026, 1, 1),
          [],
          'FREQ=YEARLY;BYDAY=-1SU;BYMONTH=10,3',
          Day(2026, 1, 1),
          Day(2028, 12, 31),
        ),
        (
          Day(2026, 1, 1),
          [],
          'FREQ=YEARLY;BYDAY=1MO',
          Day(2025, 1, 1),
          Day(2029, 12, 31),
        ),
        (
          Day(2026, 1, 1),
          [_at(8), _at(12)],
          'FREQ=DAILY;COUNT=5',
          Day(2026, 1, 2),
          Day(2026, 1, 31),
        ),
        (
          Day(2026, 1, 1),
          [_at(8), _at(12)],
          'FREQ=DAILY;UNTIL=20260103T100000',
          Day(2026, 1, 1),
          Day(2026, 1, 31),
        ),
        (
          Day(2026, 1, 1),
          [_at(8)],
          'FREQ=DAILY;UNTIL=20260105',
          Day(2025, 12, 1),
          Day(2026, 1, 31),
        ),
        (
          Day(2026, 1, 1),
          [_at(6)],
          'FREQ=DAILY;BYHOUR=20,8;BYMINUTE=30,0',
          Day(2026, 1, 1),
          Day(2026, 1, 2),
        ),
        (
          Day(2026, 1, 1),
          [],
          'FREQ=DAILY;BYDAY=SA,SU;BYMONTH=6,7',
          Day(2026, 1, 1),
          Day(2026, 7, 31),
        ),
        (
          Day(2026, 1, 1),
          [],
          'FREQ=MONTHLY;INTERVAL=3;BYMONTHDAY=1,15,-1',
          Day(2026, 1, 1),
          Day(2026, 12, 31),
        ),
        (
          Day(2026, 3, 15),
          [],
          'FREQ=YEARLY;BYMONTHDAY=15',
          Day(2026, 1, 1),
          Day(2026, 12, 31),
        ),
        (
          Day(2026, 1, 1),
          [],
          'FREQ=WEEKLY;BYDAY=SU;BYMONTH=12',
          Day(2026, 1, 1),
          Day(2027, 12, 31),
        ),
        (
          Day(2026, 10, 25),
          [_at(2, 30)],
          null,
          Day(2026, 10, 1),
          Day(2026, 10, 31),
        ),
        (Day(2026, 10, 25), [], null, Day(2026, 10, 26), Day(2026, 10, 31)),
        (
          Day(2026, 1, 1),
          [_at(8)],
          'FREQ=DAILY',
          Day(2026, 3, 10),
          Day(2026, 3, 12),
        ),
      ];
  final dstDays = [
    Day(2026, 3, 28),
    Day(2026, 3, 29),
    Day(2026, 3, 30),
    Day(2026, 10, 24),
    Day(2026, 10, 25),
    Day(2026, 10, 26),
    Day(2027, 3, 28),
    Day(2027, 10, 31),
  ];
  final dstTimes = [
    _at(0),
    _at(0, 30),
    _at(1, 59),
    _at(2),
    _at(2, 30),
    _at(2, 59),
    _at(3),
    _at(3, 30),
    _at(8),
    _at(23, 59),
  ];
  final quiet = [
    (QuietHours(_at(22), _at(8)), DateTime.utc(2026, 3, 28, 21, 30)),
    (QuietHours(_at(22), _at(8)), DateTime.utc(2026, 3, 28, 23, 30)),
    (QuietHours(_at(22), _at(8)), DateTime.utc(2026, 3, 29, 6, 30)),
    (QuietHours(_at(22), _at(8)), DateTime.utc(2026, 10, 25, 1, 30)),
    (QuietHours(_at(1), _at(3)), DateTime.utc(2026, 3, 29, 0, 30)),
    (QuietHours(_at(1), _at(3)), DateTime.utc(2026, 10, 25, 0, 30)),
    (QuietHours(_at(13), _at(14)), DateTime.utc(2026, 6, 1, 11, 30)),
    (QuietHours(_at(8), _at(8)), DateTime.utc(2026, 6, 1, 6)),
  ];
  return {
    'rules': [
      for (final text in rules)
        () {
          try {
            final rule = Recurrence.parse(text);
            return {
              'text': text,
              'format': rule.format(),
              'times': rule.times?.map((t) => '$t').toList(),
              'simple': simple(SimpleRepeat.of(rule, Day(2026, 1, 1))),
            };
          } on FormatException catch (e) {
            return {'text': text, 'error': e.message, 'source': '${e.source}'};
          }
        }(),
    ],
    'simpleOnce': simple(SimpleRepeat.of(null, Day(2026, 1, 1))),
    'schedules': [
      for (final (start, times, rule, from, to) in schedules)
        () {
          final schedule = Schedule(
            start: start,
            times: times,
            rule: rule == null ? null : Recurrence.parse(rule),
          );
          return {
            'start': '$start',
            'times': [for (final t in times) '$t'],
            'rule': rule,
            'from': '$from',
            'to': '$to',
            'dayTimes': [for (final t in schedule.dayTimes) '$t'],
            'between': [for (final o in schedule.between(from, to)) o.key],
            'next': [for (final o in schedule.next(from, 8)) o.key],
          };
        }(),
    ],
    'days': [
      for (final [y, m, d] in [
        [2026, 1, 32],
        [2026, 13, 1],
        [2026, 0, 0],
        [2024, 2, 30],
        [2026, -1, 15],
        [2025, 24, 31],
        [2026, 2, 29],
        [2000, 2, 29],
        [1900, 2, 29],
        [2026, 12, 31],
      ])
        {
          'in': [y, m, d],
          'day': '${Day(y, m, d)}',
          'weekday': Day(y, m, d).weekday,
        },
    ],
    'instants': [
      for (final day in dstDays)
        for (final time in dstTimes)
          [
            '$day',
            '$time',
            instantOf(day, time, _europe).toUtc().toIso8601String(),
          ],
    ],
    'quiet': [
      for (final (hours, instant) in quiet)
        [
          '$hours',
          instant.toIso8601String(),
          hours.defer(instant, _europe).toUtc().toIso8601String(),
        ],
    ],
  };
}

Store _store() {
  _tick = DateTime.utc(2026, 1, 1, 10);
  _held = null;
  var ids = 0;
  Store.idMaker = () => (++ids).toRadixString(16).padLeft(32, '0');
  final store = Store.inMemory(
    clock: _clock,
    newKey: () => SigningKey.fromSeed(_seed),
  );
  store.setLocalSetting('author', 'Ada');
  return store;
}

const _json = JsonEncoder.withIndent('  ');

void _write(File file, String text) {
  file.parent.createSync(recursive: true);
  file.writeAsStringSync(text);
}

/// The store's schema, as SQLite keeps it, in a stable order.
String _schema(Directory work) {
  final path = '${work.path}/schema.db';
  Store.open(path, newKey: () => SigningKey.fromSeed(_seed)).close();
  final db = sqlite3.open(path);
  try {
    final rows = db.select(
      "SELECT sql FROM sqlite_master WHERE sql IS NOT NULL AND name NOT LIKE 'sqlite_%' ORDER BY name",
    );
    return '${[for (final row in rows) '${row['sql']};'].join('\n')}\n';
  } finally {
    db.close();
  }
}

/// One entry with awkward text, signed by the writer's key, with the
/// exact bytes the signature covers.
Map<String, Object?> _signing() {
  final key = SigningKey.fromSeed(_seed);
  const date = '2026-01-01T10:00:00.000Z';
  const recorded = '2026-01-01T10:00:00.123456Z';
  final device = deviceIdFromKey(key.publicKey);
  final bytes = entryBytes(
    device: device,
    dseq: 7,
    entity: 'note:0001',
    field: 'text',
    value: _awkward,
    dateIso: date,
    author: 'Ada "the writer"',
    recordedIso: recorded,
  );
  final nullBytes = entryBytes(
    device: device,
    dseq: 8,
    entity: 'note:0001',
    field: 'trashed',
    value: null,
    dateIso: date,
    author: '',
    recordedIso: recorded,
  );
  return {
    'seed': key.seedHex,
    'publicKey': key.publicBase64,
    'device': device,
    'code': key.code,
    'entries': [
      {
        'device': device,
        'dseq': 7,
        'entity': 'note:0001',
        'field': 'text',
        'value': _awkward,
        'date': date,
        'author': 'Ada "the writer"',
        'recorded': recorded,
        'bytes': base64.encode(bytes),
        'sig': key.sign(bytes),
      },
      {
        'device': device,
        'dseq': 8,
        'entity': 'note:0001',
        'field': 'trashed',
        'value': null,
        'date': date,
        'author': '',
        'recorded': recorded,
        'bytes': base64.encode(nullBytes),
        'sig': key.sign(nullBytes),
      },
    ],
  };
}

/// The marks over a region around the origin, negative cells included.
Map<String, Object?> _questionMarks() => {
  'shapes': [
    for (final shape in questionMarkShapes)
      {'strokes': shape.strokes, 'dot': shape.dot, 'weight': shape.weight},
  ],
  'seed': questionMarkSeed,
  'region': [-400, -300, 700, 500],
  'marks': [
    for (final m in questionMarksIn(-400, -300, 700, 500))
      {
        'shape': m.shape,
        'x': m.x,
        'y': m.y,
        'size': m.size,
        'rotation': m.rotation,
      },
  ],
  'otherSeed': [
    for (final m in questionMarksIn(0, 0, 300, 300, seed: 1))
      {
        'shape': m.shape,
        'x': m.x,
        'y': m.y,
        'size': m.size,
        'rotation': m.rotation,
      },
  ],
};

void main(List<String> args) {
  if (args.length != 1) {
    stderr.writeln('usage: dart run tool/gen_fixtures.dart <fixtures dir>');
    exit(2);
  }
  final root = Directory(args[0])..createSync(recursive: true);
  // A scenario dropped from the list leaves no folder behind.
  for (final old in root.listSync().whereType<Directory>()) {
    old.deleteSync(recursive: true);
  }
  final index = StringBuffer(
    '# Fixture corpus\n\n'
    'Generated by `packages/oldbrainfix_core/tool/gen_fixtures.dart` from '
    'the Dart core; the desktop core loads every `log.jsonl` and must read '
    'the same `expected.json`. Do not edit by hand.\n\n'
    '- `schema.sql`: the tables and indexes of the store.\n'
    '- `signing.json`: entries signed by the writer key, with the bytes '
    'the signatures cover.\n'
    '- `question_marks.json`: the background\'s shapes and their scatter '
    'over a region.\n'
    '- `schedules.json`: RRULEs read, written back or refused, schedules '
    'with their Occurrences, days normalised, and the instants of '
    'wall-clock times around daylight-saving switches in Central Europe.\n'
    '- `routines/alarms.json`: the Agenda, alarm plan, Missed Summary and '
    'Nudge of the `routines` log at held moments with this Device\'s '
    'settings, on the same clock.\n',
  );
  final work = Directory.systemTemp.createTempSync('oldbrainfix_fixture_');
  try {
    _write(File('${root.path}/schema.sql'), _schema(work));
  } finally {
    work.deleteSync(recursive: true);
  }
  _write(File('${root.path}/signing.json'), '${_json.convert(_signing())}\n');
  _write(
    File('${root.path}/question_marks.json'),
    '${_json.convert(_questionMarks())}\n',
  );
  _write(
    File('${root.path}/schedules.json'),
    '${_json.convert(_schedules())}\n',
  );
  for (final scenario in _scenarios) {
    final store = _store();
    try {
      scenario.build(store);
      final out = Directory('${root.path}/${scenario.name}');
      final log = StringBuffer();
      for (final entry in store.allEntries()) {
        log.writeln(jsonEncode(entry.toJson()));
      }
      _write(File('${out.path}/log.jsonl'), log.toString());
      _held = _tick.add(const Duration(days: 1));
      _write(
        File('${out.path}/expected.json'),
        '${_json.convert(_expected(store))}\n',
      );
      if (store.entitiesOfType(RoutineKeys.typeRoutine).isNotEmpty) {
        _write(
          File('${out.path}/alarms.json'),
          '${_json.convert(_alarms(store))}\n',
        );
      }
      index.writeln('- `${scenario.name}`: ${scenario.about}');
    } finally {
      store.close();
    }
  }
  _write(File('${root.path}/README.md'), index.toString());
  stdout.writeln('Wrote ${_scenarios.length} scenarios to ${root.path}');
}
