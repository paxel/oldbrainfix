import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/frogs_screen.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/trash_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'open_capture.dart';

void main() {
  late Store store;
  late Notes notes;
  late Places places;
  late Frogs frogs;
  // 09:00 on Saturday, 24 October 2026, on the test machine's clock.
  var now = DateTime(2026, 10, 24, 9);

  setUp(() {
    now = DateTime(2026, 10, 24, 9);
    store = Store.inMemory(clock: () => now)..starterSetOffered = true;
    notes = Notes(store);
    places = Places(store);
    frogs = Frogs(store);
  });
  tearDown(() => store.close());

  /// A Frog spawned from a new Note [text], a minute after the last one,
  /// so they are oldest first in the order made.
  Frog frog(String text, {String? place}) {
    now = now.add(const Duration(minutes: 1));
    return frogs.spawn(notes.capture(text, place: place).id);
  }

  Future<void> start(WidgetTester tester) async {
    await tester.pumpWidget(OldBrainFixApp(store: store));
    await tester.pumpAndSettle();
  }

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  Finder row(Frog frog) => find.byKey(ValueKey('frog-${frog.id}'));

  Finder box(Frog frog) =>
      find.descendant(of: row(frog), matching: find.byType(Checkbox));

  /// The Frog rows shown, top to bottom, as the Notes' texts.
  List<String> shown(WidgetTester tester, List<Frog> all) {
    final rows =
        [
          for (final f in all)
            if (row(f).evaluate().isNotEmpty) f,
        ]..sort(
          (a, b) => tester
              .getTopLeft(row(a))
              .dy
              .compareTo(tester.getTopLeft(row(b)).dy),
        );
    return [for (final f in rows) frogs.note(f)?.text ?? ''];
  }

  group('making a Frog', () {
    testWidgets('a captured Note\'s card offers "Routine, Reminder, Frog or '
        'Session"; '
        'the Frog takes it out of Unsorted and shows under today', (
      tester,
    ) async {
      await start(tester);
      await openTextCapture(tester);
      await tester.enterText(find.byType(TextField), 'wash the windows');
      await tap(tester, find.text('Save'));
      expect(notes.unsorted(), hasLength(1));

      await tap(tester, find.byTooltip('Routine, Reminder, Frog or Session'));
      expect(find.byType(BottomSheet), findsNothing);
      expect(find.text('Routine'), findsOneWidget);
      expect(find.text('Reminder'), findsOneWidget);
      await tap(tester, find.text('Frog'));

      final made = frogs.open().single;
      expect(frogs.note(made)?.text, 'wash the windows');
      expect(notes.unsorted(), isEmpty);
      expect(find.byType(AgendaScreen), findsOneWidget, reason: 'opens none');
      expect(find.text('Eat a frog'), findsOneWidget);
      expect(
        find.descendant(of: row(made), matching: find.text('wash the windows')),
        findsOneWidget,
      );
    });
  });

  testWidgets('"Frog" in the Note editor spawns one, as the card\'s menu '
      'does', (tester) async {
    final id = notes.capture('wash the windows').id;
    await start(tester);
    final agenda = tester.element(find.byType(AgendaScreen));
    unawaited(openNote(agenda, notes, places, notes.byId(id)!));
    await tester.pumpAndSettle();
    final button = find.byKey(const ValueKey('spawn-frog'));
    await tester.ensureVisible(button);
    await tester.pumpAndSettle();
    await tap(tester, button);

    expect(frogs.note(frogs.open().single)?.id, id);
    expect(notes.unsorted(), isEmpty);
    expect(find.byType(NoteEditorScreen), findsOneWidget, reason: 'opens none');
  });

  group('the "Eat a frog" section', () {
    testWidgets('shows two Frogs under today\'s timed Occurrences, before '
        'tomorrow, and keeps the pick for the own Devices', (tester) async {
      final note = notes.capture('call Mum');
      Routines(store).spawn(
        note.id,
        Schedule(start: Day(2026, 10, 24), times: [ClockTime(12, 0)]),
      );
      final all = [frog('cupboard'), frog('windows'), frog('old DVDs')];
      await start(tester);

      expect(shown(tester, all), ['cupboard', 'windows']);
      final occurrence = tester.getTopLeft(find.text('call Mum')).dy;
      final heading = tester.getTopLeft(find.text('Eat a frog')).dy;
      final tomorrow = tester.getTopLeft(find.text('Tomorrow')).dy;
      expect(occurrence, lessThan(heading));
      expect(heading, lessThan(tester.getTopLeft(row(all[0])).dy));
      expect(tester.getTopLeft(row(all[1])).dy, lessThan(tomorrow));
      expect(
        store.currentFields(FrogKeys.pickEntity)[FrogKeys.pick],
        '2026-10-24,${all[0].id},${all[1].id}',
      );
    });

    testWidgets('checking one off hides the other; it stays struck through '
        'and its box unchecks it', (tester) async {
      final all = [frog('cupboard'), frog('windows')];
      await start(tester);

      await tap(tester, box(all[0]));
      expect(frogs.byId(all[0].id)?.isDone, isTrue);
      expect(shown(tester, all), ['cupboard']);
      expect(tester.widget<Checkbox>(box(all[0])).value, isTrue);
      final title = tester.widget<Text>(
        find.descendant(of: row(all[0]), matching: find.text('cupboard')),
      );
      expect(title.style?.decoration, TextDecoration.lineThrough);
      expect(find.text('Another one'), findsNothing);
      expect(find.text('Eat a frog'), findsOneWidget, reason: 'heading stays');

      await tap(tester, box(all[0]));
      expect(frogs.byId(all[0].id)?.isDone, isFalse);
      expect(shown(tester, all), ['cupboard', 'windows']);
    });

    testWidgets('"Another one" swaps a Frog for the next in its place', (
      tester,
    ) async {
      final all = [frog('cupboard'), frog('windows'), frog('old DVDs')];
      await start(tester);
      expect(find.text('Another one'), findsNWidgets(2));

      await tap(
        tester,
        find.descendant(of: row(all[0]), matching: find.text('Another one')),
      );
      expect(shown(tester, all), ['old DVDs', 'windows']);
    });

    testWidgets('has no "Another one" without a Frog to swap in', (
      tester,
    ) async {
      final all = [frog('cupboard'), frog('windows')];
      await start(tester);
      expect(shown(tester, all), ['cupboard', 'windows']);
      expect(find.text('Another one'), findsNothing);
    });

    testWidgets('is gone without open Frogs', (tester) async {
      final eaten = frog('cupboard');
      now = DateTime(2026, 10, 23, 18);
      frogs.markDone(eaten.id);
      now = DateTime(2026, 10, 24, 9);
      await start(tester);
      expect(find.text('Eat a frog'), findsNothing);
      expect(row(eaten), findsNothing);
    });

    testWidgets('shows a Frog of a Masked Area as "Private Frog"', (
      tester,
    ) async {
      store.setPin('2468', iterations: 10);
      final doctor = places.addArea('Doctor');
      final secret = frog('book the scan', place: doctor.id);
      store.setAreaLocked(doctor.id, true);
      store.setAreaMasked(doctor.id, true);
      await start(tester);
      expect(
        find.descendant(of: row(secret), matching: find.text('Private Frog')),
        findsOneWidget,
      );
      expect(find.text('book the scan'), findsNothing);
    });

    testWidgets('a long press deletes a Frog after asking; the trash '
        'restores it', (tester) async {
      final mistake = frog('cupboard');
      await start(tester);

      await tester.longPress(row(mistake));
      await tester.pumpAndSettle();
      await tap(tester, find.text('Delete'));
      expect(
        find.text('Delete this Frog? It stays in the trash. Its Note stays.'),
        findsOneWidget,
      );
      await tap(tester, find.text('Delete').last);
      expect(frogs.byId(mistake.id)?.isTrashed, isTrue);
      expect(notes.byId(mistake.noteId)?.isTrashed, isFalse);
      expect(find.text('Eat a frog'), findsNothing);

      final navigator = Navigator.of(tester.element(find.byType(AgendaScreen)));
      unawaitedPush(navigator, TrashScreen(notes: notes, places: places));
      await tester.pumpAndSettle();
      expect(find.text('cupboard'), findsOneWidget);
      await tap(tester, find.text('Restore'));
      expect(frogs.byId(mistake.id)?.isTrashed, isFalse);
      navigator.pop();
      await tester.pumpAndSettle();
      expect(row(mistake), findsOneWidget);
    });
  });

  group('the Frog view', () {
    testWidgets('opens from "Frogs" on the Areas list\'s bar, beside All '
        'Notes, eaten Frogs too', (tester) async {
      final all = [frog('cupboard'), frog('old DVDs')];
      frogs.markDone(all[1].id);
      await start(tester);
      await tap(tester, find.byTooltip('Search and browse'));
      final bar = find.byType(AppBar);
      final button = find.descendant(
        of: bar,
        matching: find.byTooltip('Frogs'),
      );
      expect(button, findsOneWidget);
      expect(
        tester.getTopLeft(button).dx,
        greaterThan(tester.getTopLeft(find.byTooltip('All Notes')).dx),
      );
      await tap(tester, button);
      expect(find.byType(FrogsScreen), findsOneWidget);
      expect(shown(tester, all), ['cupboard']);
      await tap(tester, find.text('Done'));
      expect(shown(tester, all), ['cupboard', 'old DVDs']);
    });

    testWidgets('opens from the heading: the open Frogs oldest first, the '
        'Done ones folded below and unchecked there', (tester) async {
      final all = [frog('cupboard'), frog('windows'), frog('old DVDs')];
      frogs.markDone(all[2].id);
      await start(tester);
      await tap(tester, find.text('Eat a frog'));
      expect(find.byType(FrogsScreen), findsOneWidget);
      expect(find.text('Frogs'), findsOneWidget);

      expect(shown(tester, all), ['cupboard', 'windows']);
      expect(find.text('Done'), findsOneWidget);
      expect(row(all[2]), findsNothing, reason: 'folded by default');

      await tap(tester, find.text('Done'));
      expect(row(all[2]), findsOneWidget);
      expect(find.text('Done on 10/24/2026'), findsOneWidget);
      expect(store.localSetting('frogsDoneOpen'), 'true');

      await tap(tester, box(all[2]));
      expect(frogs.byId(all[2].id)?.isDone, isFalse);
      expect(shown(tester, all), ['cupboard', 'windows', 'old DVDs']);
      expect(find.text('Done'), findsNothing);
    });

    testWidgets('checks a Frog off and opens its Note on a tap', (
      tester,
    ) async {
      final all = [frog('cupboard'), frog('windows'), frog('old DVDs')];
      await start(tester);
      await tap(tester, find.byKey(const ValueKey('frogsHeading')));

      await tap(tester, box(all[2]));
      expect(frogs.byId(all[2].id)?.isDone, isTrue);
      expect(shown(tester, all), ['cupboard', 'windows']);

      await tap(tester, find.text('cupboard'));
      expect(find.byType(NoteEditorScreen), findsOneWidget);
    });

    testWidgets('says how to make one when none is open', (tester) async {
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();
      final navigator = Navigator.of(tester.element(find.byType(AgendaScreen)));
      unawaitedPush(navigator, FrogsScreen(notes: notes, places: places));
      await tester.pumpAndSettle();
      expect(
        find.text(
          'No open Frogs. "Routine, Reminder, Frog or Session" on a Note\'s '
          'card '
          'makes one.',
        ),
        findsOneWidget,
      );
    });
  });
}

/// Pushes [screen] without waiting for it to close.
void unawaitedPush(NavigatorState navigator, Widget screen) =>
    unawaited(navigator.push(MaterialPageRoute<void>(builder: (_) => screen)));
