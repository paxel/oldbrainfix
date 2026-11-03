import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/alarm_platform.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_alarms.dart';
import 'open_settings.dart';

/// "Move a Note to the Trash when its one-time Reminder is done" (#211).
void main() {
  late Store store;
  late Notes notes;
  late Routines routines;
  // 09:00 on Saturday, 24 October 2026, on the test machine's clock.
  var now = DateTime(2026, 10, 24, 9);
  final today = Day(2026, 10, 24);
  final eight = Occurrence(today, ClockTime(8, 0));

  setUp(() {
    now = DateTime(2026, 10, 24, 7);
    store = Store.inMemory(clock: () => now)
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    notes = Notes(store);
    routines = Routines(store);
  });
  tearDown(() => store.close());

  /// A Note with [text] and a one-time Reminder at 08:00 today, set up
  /// before it was due; the clock then at 09:00.
  Routine reminder(String text) {
    final r = routines.spawn(
      notes.capture(text).id,
      Schedule(start: today, times: [ClockTime(8, 0)]),
    );
    now = DateTime(2026, 10, 24, 9);
    return r;
  }

  bool trashed(Routine r) => notes.byId(r.noteId)?.isTrashed ?? false;

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  Future<void> start(WidgetTester tester) async {
    await tester.pumpWidget(OldBrainFixApp(store: store, alarms: FakeAlarms()));
    await tester.pumpAndSettle();
  }

  Finder agendaBox(Routine r) => find.descendant(
    of: find.byKey(ValueKey('row-${r.id}@${eight.key}')),
    matching: find.byType(Checkbox),
  );

  /// From the Agenda, shows All Notes as cards.
  Future<void> allNotes(WidgetTester tester) async {
    await tap(tester, find.byTooltip('Search and browse'));
    await tap(tester, find.byTooltip('All Notes'));
  }

  testWidgets('Settings has it in the Agenda section, off; switched on it '
      'is the person\'s, kept in the log', (tester) async {
    await start(tester);
    await openDeviceSettings(tester);
    final setting = find.byKey(const ValueKey('trashDoneReminder'));
    await tester.scrollUntilVisible(
      setting,
      200,
      scrollable: find.byType(Scrollable).last,
    );
    await tester.pumpAndSettle();
    expect(
      find.text('Move a Note to the Trash when its one-time Reminder is done'),
      findsOneWidget,
    );
    expect(tester.widget<Switch>(setting).value, isFalse);
    await tap(tester, setting);
    expect(store.trashesDoneReminder, isTrue);
    expect(
      store.allEntries().map((e) => e.entity),
      contains(PersonSettingKeys.entity),
      reason: 'it goes to the own Devices',
    );
    expect(tester.widget<Switch>(setting).value, isTrue);
  });

  testWidgets('off, Done on the Agenda leaves the Note where it is', (
    tester,
  ) async {
    final dentist = reminder('dentist');
    await start(tester);
    await tap(tester, agendaBox(dentist));
    expect(routines.byId(dentist.id)?.doneAt(eight), isNotNull);
    expect(trashed(dentist), isFalse);
  });

  testWidgets('on, Done on the Agenda trashes the Note and its row goes; '
      "a Checklist's Reminder stays", (tester) async {
    final dentist = reminder('dentist');
    now = DateTime(2026, 10, 24, 7);
    final packing = reminder('packing');
    Checklists(store).make(packing.noteId);
    store.trashesDoneReminder = true;
    await start(tester);
    await tap(tester, agendaBox(dentist));
    expect(trashed(dentist), isTrue);
    expect(
      find.byKey(ValueKey('row-${dentist.id}@${eight.key}')),
      findsNothing,
    );
    await tap(tester, agendaBox(packing));
    expect(routines.byId(packing.id)?.doneAt(eight), isNotNull);
    expect(trashed(packing), isFalse);
  });

  testWidgets("on, the editor's Done trashes the Note and closes", (
    tester,
  ) async {
    final dentist = reminder('dentist');
    store.trashesDoneReminder = true;
    await start(tester);
    await allNotes(tester);
    await tester.longPress(find.text('dentist').first);
    await tester.pumpAndSettle();
    expect(find.byType(NoteEditorScreen), findsOneWidget);
    await tap(tester, find.byKey(const ValueKey('open-done')));
    expect(find.byType(NoteEditorScreen), findsNothing);
    expect(trashed(dentist), isTrue);
    expect(routines.byId(dentist.id)?.doneAt(eight), isNotNull);
  });

  testWidgets("on, a card's box trashes the Note and the card goes", (
    tester,
  ) async {
    final dentist = reminder('dentist');
    store.trashesDoneReminder = true;
    await start(tester);
    await allNotes(tester);
    final box = find.descendant(
      of: find.byKey(ValueKey('card-occurrence-${dentist.id}@${eight.key}')),
      matching: find.byType(Checkbox),
    );
    await tap(tester, box);
    expect(trashed(dentist), isTrue);
    expect(find.text('dentist'), findsNothing);
  });

  testWidgets("on, a notification's Done trashes it as it is applied", (
    tester,
  ) async {
    final dentist = reminder('dentist');
    store.trashesDoneReminder = true;
    final alarms = FakeAlarms()
      ..pending.add(DoneTapped(1, dentist.id, eight.key, now.toUtc()));
    await tester.pumpWidget(OldBrainFixApp(store: store, alarms: alarms));
    await tester.pumpAndSettle();
    expect(trashed(dentist), isTrue);
    expect(
      find.byKey(ValueKey('row-${dentist.id}@${eight.key}')),
      findsNothing,
    );
  });
}
