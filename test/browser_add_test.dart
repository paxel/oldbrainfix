import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/browser_add.dart';
import 'package:oldbrainfix/src/line_editing.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/routine_editor_screen.dart';
import 'package:oldbrainfix/src/session_form_screen.dart';
import 'package:oldbrainfix/src/tally_form_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

Map<String, Object?> wire(Map<String, Object?> json) =>
    jsonDecode(jsonEncode(json)) as Map<String, Object?>;

/// A browser page with a "+": the Area Household, its Topic Kitchen or
/// All Notes.
enum _Page { area, topic, all }

void main() {
  late Store store;
  late Places places;
  late Notes notes;
  late Area home;
  late Topic kitchen;

  setUp(() {
    // A moment later on each reading, as a real clock: the fields of a
    // Tally keep the order they were added in.
    var now = DateTime.utc(2026, 10, 27, 8);
    store = Store.inMemory(
      clock: () => now = now.add(const Duration(microseconds: 1)),
    )..starterSetOffered = true;
    places = Places(store);
    notes = Notes(store);
    home = places.addArea('Household');
    kitchen = places.addTopic(home.id, 'Kitchen');
  });
  tearDown(() => store.close());

  Finder key(String k) => find.byKey(ValueKey(k));

  Future<void> tap(WidgetTester tester, Finder target) async {
    await tester.ensureVisible(target);
    await tester.pumpAndSettle();
    await tester.tap(target);
    await tester.pumpAndSettle();
  }

  /// The page's place: the Area, the Topic, none for All Notes.
  String? placeOf(_Page page) => switch (page) {
    _Page.area => home.id,
    _Page.topic => kitchen.id,
    _Page.all => null,
  };

  /// Opens [page] on a tall phone, so a whole form shows.
  Future<void> openPage(WidgetTester tester, _Page page) async {
    tester.view.physicalSize = const Size(2400, 3600);
    tester.view.devicePixelRatio = 3;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(OldBrainFixApp(store: store));
    await tap(tester, find.byTooltip('Search and browse'));
    switch (page) {
      case _Page.area:
        await tap(tester, find.text('Household'));
      case _Page.topic:
        await tap(tester, find.text('Household'));
        await tap(tester, find.text('Kitchen'));
      case _Page.all:
        await tap(tester, find.byTooltip('All Notes'));
    }
  }

  /// Opens the fan of "+" and picks [kind].
  Future<void> pick(WidgetTester tester, NewKind kind) async {
    await tap(tester, find.byTooltip('Add here'));
    await tap(tester, key('addFan-${kind.name}'));
  }

  /// Names the new thing in the name dialog and creates it.
  Future<void> name(WidgetTester tester, String text) async {
    await tester.enterText(
      find.descendant(
        of: find.byType(AlertDialog),
        matching: find.byType(TextField),
      ),
      text,
    );
    await tap(tester, find.text('Create'));
  }

  /// The only Note there is, which must exist.
  Note only() => notes.all().single;

  /// Fills in a New Tally page: [title], then each field of [fields] as
  /// its name, unit typed and target, then the known unit [chip] picked
  /// from the suggestions when given; an empty name leaves the row empty.
  Future<void> fillTally(
    WidgetTester tester,
    String title,
    List<({String name, String unit, String target, String? chip})> fields,
  ) async {
    await tester.enterText(key('tally-form-name'), title);
    for (final (i, field) in fields.indexed) {
      if (i > 0) await tap(tester, key('tally-form-add-field'));
      await tester.enterText(key('tally-form-field-name-$i'), field.name);
      if (field.unit.isNotEmpty) {
        await tap(tester, key('tally-form-unit-$i'));
        await tester.enterText(key('tally-unit-text'), field.unit);
        await tester.pumpAndSettle();
        if (field.chip case final chip?) {
          await tap(tester, key('tally-unit-$chip'));
        }
        await tap(tester, key('badge-close'));
      }
      if (field.target.isNotEmpty) {
        await tap(tester, key('tally-form-target-$i'));
        await tester.enterText(key('tally-field-target'), field.target);
        await tap(tester, key('badge-close'));
      }
    }
  }

  group('the fan of "+"', () {
    testWidgets('fans out the six kinds in an Area, with Topic, above the '
        'button; Close makes nothing', (tester) async {
      await openPage(tester, _Page.area);
      final plus = tester.getRect(find.byTooltip('Add here'));
      await tap(tester, find.byTooltip('Add here'));
      for (final kind in NewKind.values) {
        final entry = key('addFan-${kind.name}');
        expect(entry, findsOneWidget);
        // Upward from the button, not from the bottom edge.
        expect(tester.getRect(entry).bottom, lessThanOrEqualTo(plus.top));
      }
      expect(find.byType(BottomSheet), findsNothing);
      // Note nearest the button.
      expect(
        tester.getCenter(key('addFan-note')).dy,
        greaterThan(tester.getCenter(key('addFan-frog')).dy),
      );
      await tap(tester, key('addFanClose'));
      expect(key('addFan-note'), findsNothing);
      expect(notes.all(), isEmpty);
    });

    for (final page in [_Page.topic, _Page.all]) {
      testWidgets('fans out the six kinds on ${page.name}, without Topic', (
        tester,
      ) async {
        await openPage(tester, page);
        await tap(tester, find.byTooltip('Add here'));
        for (final kind in NewKind.values) {
          expect(
            key('addFan-${kind.name}'),
            kind == NewKind.topic ? findsNothing : findsOneWidget,
          );
        }
      });
    }

    testWidgets('a Topic still comes from the Area\'s fan', (tester) async {
      await openPage(tester, _Page.area);
      await pick(tester, NewKind.topic);
      await name(tester, 'Cellar');
      expect(places.topics(home.id).map((t) => t.name), contains('Cellar'));
    });
  });

  for (final page in _Page.values) {
    group('from ${page.name}', () {
      testWidgets('Note captures one filed there', (tester) async {
        await openPage(tester, page);
        await pick(tester, NewKind.note);
        await tester.enterText(find.byType(TextField), 'nees mor sauce');
        // Leaving saves, with a Nudge where it stays Unsorted.
        await tester.pageBack();
        await tester.pumpAndSettle();
        expect(only().text, 'nees mor sauce');
        expect(only().place, placeOf(page));
      });

      testWidgets('Checklist takes its name, then its Lines in place', (
        tester,
      ) async {
        await openPage(tester, page);
        await pick(tester, NewKind.checklist);
        await name(tester, 'Before the trip');
        // Its name on top, the kind's content under it (#188).
        expect(
          tester
              .widget<TextField>(
                find.descendant(
                  of: key('note-name'),
                  matching: find.byType(TextField),
                ),
              )
              .controller!
              .text,
          'Before the trip',
        );
        expect(find.byType(NoteEditorScreen), findsOneWidget);
        // The cursor waits in its first Line.
        final line = find.descendant(
          of: key('checklist-field-new-1'),
          matching: find.byType(TextField),
        );
        expect(
          tester.widget<TextField>(line).focusNode?.hasFocus ?? false,
          isTrue,
        );
        tester.testTextInput.updateEditingValue(
          const TextEditingValue(
            text: '${lineStart}Socks',
            selection: TextSelection.collapsed(offset: 6),
          ),
        );
        await tester.pumpAndSettle();
        await tester.pageBack();
        await tester.pumpAndSettle();

        final note = only();
        expect(note.text, 'Before the trip');
        expect(note.place, placeOf(page));
        expect(note.isUnsorted, isFalse);
        expect(Checklists(store).byNote(note.id)!.lines.map((l) => l.text), [
          'Socks',
        ]);
        expect(find.text('Before the trip'), findsOneWidget);
      });

      testWidgets('Shopping List takes its name, then its Lines in place', (
        tester,
      ) async {
        await openPage(tester, page);
        await pick(tester, NewKind.shopping);
        await name(tester, 'Groceries');
        // Its name on top, the kind's content under it (#188).
        expect(
          tester
              .widget<TextField>(
                find.descendant(
                  of: key('note-name'),
                  matching: find.byType(TextField),
                ),
              )
              .controller!
              .text,
          'Groceries',
        );
        // The cursor waits in its first Line, as a Checklist's (#184).
        expect(key('shopping-add-text'), findsNothing);
        final line = find.descendant(
          of: key('shopping-field-new-1'),
          matching: find.byType(TextField),
        );
        expect(
          tester.widget<TextField>(line).focusNode?.hasFocus ?? false,
          isTrue,
        );
        tester.testTextInput.updateEditingValue(
          const TextEditingValue(
            text: '${lineStart}Milk',
            selection: TextSelection.collapsed(offset: 5),
          ),
        );
        await tester.pumpAndSettle();
        await tester.pageBack();
        await tester.pumpAndSettle();

        final note = only();
        expect(note.text, 'Groceries');
        expect(note.place, placeOf(page));
        expect(
          ShoppingLists(store)
              .byNote(note.id, deviceWallClock)!
              .lines
              .map((l) => l.text),
          ['Milk'],
        );
      });

      testWidgets('Frog takes its name only', (tester) async {
        await openPage(tester, page);
        await pick(tester, NewKind.frog);
        await name(tester, 'Clear the cellar');
        final note = only();
        expect(note.text, 'Clear the cellar');
        expect(note.place, placeOf(page));
        expect(Frogs(store).ofNote(note.id), hasLength(1));
        expect(find.text('Clear the cellar'), findsOneWidget);
      });

      testWidgets('Tally takes one page', (tester) async {
        await openPage(tester, page);
        await pick(tester, NewKind.tally);
        expect(find.byType(TallyFormScreen), findsOneWidget);
        await fillTally(tester, 'Reading', [
          (name: 'Pages', unit: 'pages', target: '', chip: null),
        ]);
        await tap(tester, key('tally-form-save'));
        expect(find.byType(TallyFormScreen), findsNothing);
        final note = only();
        expect(note.text, 'Reading');
        expect(note.place, placeOf(page));
        final field = Tallies(store).byNote(note.id)!.fields.single;
        expect((field.name, field.unit), ('Pages', 'pages'));
      });

      testWidgets('Session takes one page', (tester) async {
        final tally = notes.capture('Push-ups', place: placeOf(page)).id;
        Tallies(store).addField(tally, name: 'Times');
        await openPage(tester, page);
        await pick(tester, NewKind.session);
        await tester.enterText(key('session-form-name'), 'Morning');
        await tap(tester, key('session-pool-$tally'));
        await tap(tester, key('session-form-save'));
        expect(find.byType(SessionFormScreen), findsNothing);
        final session = Sessions(store).all().single;
        final note = Routines(store).note(session.routine)!;
        expect(note.text, 'Morning');
        expect(note.place, placeOf(page));
        expect(session.pool, [tally]);
      });
    });
  }

  group('the New Tally page', () {
    testWidgets('a name and two fields with their settings take one save', (
      tester,
    ) async {
      await openPage(tester, _Page.area);
      await pick(tester, NewKind.tally);
      await fillTally(tester, 'Workout', [
        (name: 'Plank', unit: 'min', target: '2', chip: null),
        (name: 'Tilt', unit: 'deg', target: '', chip: '°'),
      ]);
      // The rule follows the unit until it is picked: an angle counts the
      // largest, a time the sum.
      Finder badge(String badge, String text) =>
          find.descendant(of: key(badge), matching: find.text(text));
      // The rule as its icon (#192).
      Finder rule(String badge, IconData icon) =>
          find.descendant(of: key(badge), matching: find.byIcon(icon));
      expect(rule('tally-form-rule-0', Icons.add_circle_outline), findsOne);
      expect(rule('tally-form-rule-1', Icons.vertical_align_top), findsOne);
      expect(badge('tally-form-unit-1', '°'), findsOneWidget);
      expect(badge('tally-form-target-0', '2 min'), findsOneWidget);
      await tap(tester, key('tally-form-rule-0'));
      await tap(tester, key('tally-rule-max'));
      expect(rule('tally-form-rule-0', Icons.vertical_align_top), findsOne);

      await tap(tester, key('tally-form-save'));
      final fields = Tallies(store).byNote(only().id)!.fields;
      expect(
        [for (final f in fields) (f.name, f.unit, f.target, f.rule)],
        [
          ('Plank', 'min', 120.0, TallyRule.max),
          ('Tilt', '°', null, TallyRule.max),
        ],
      );
    });

    testWidgets('drops the rows left without a name, and needs one', (
      tester,
    ) async {
      await openPage(tester, _Page.topic);
      await pick(tester, NewKind.tally);
      await tester.enterText(key('tally-form-name'), 'Reading');
      await tap(tester, key('tally-form-save'));
      // Nothing yet: every row is empty.
      expect(find.text('Give at least one field a name.'), findsOneWidget);
      expect(notes.all(), isEmpty);

      await fillTally(tester, 'Reading', [
        (name: '', unit: '', target: '', chip: null),
        (name: 'Pages', unit: '', target: '', chip: null),
        (name: '  ', unit: '', target: '', chip: null),
      ]);
      await tap(tester, key('tally-form-save'));
      final fields = Tallies(store).byNote(only().id)!.fields;
      expect(fields.map((f) => f.name), ['Pages']);
    });

    testWidgets('needs a name and a target that is a number', (tester) async {
      await openPage(tester, _Page.area);
      await pick(tester, NewKind.tally);
      await fillTally(tester, '', [
        (name: 'Pages', unit: '', target: 'many', chip: null),
      ]);
      await tap(tester, key('tally-form-save'));
      expect(find.text('Give it a name.'), findsOneWidget);
      expect(notes.all(), isEmpty);

      await tester.enterText(key('tally-form-name'), 'Reading');
      await tap(tester, key('tally-form-save'));
      expect(key('tally-form-target-error-0'), findsOneWidget);
      expect(notes.all(), isEmpty);
    });
  });

  group('the New Session page', () {
    testWidgets('makes a new Tally inside its pool, ticked, on one page', (
      tester,
    ) async {
      await openPage(tester, _Page.area);
      await pick(tester, NewKind.session);
      await tester.enterText(key('session-form-name'), 'Morning');
      // Without a Tally it says what is missing.
      await tap(tester, key('session-form-save'));
      expect(key('session-pool-error'), findsOneWidget);

      await tap(tester, key('session-new-tally'));
      expect(find.byType(TallyFormScreen), findsOneWidget);
      await fillTally(tester, 'Push-ups', [
        (name: 'Times', unit: '', target: '20', chip: null),
      ]);
      await tap(tester, key('tally-form-save'));
      expect(find.byType(SessionFormScreen), findsOneWidget);
      final tally = Tallies(store).all().single.noteId;
      expect(
        tester.widget<CheckboxListTile>(key('session-pool-$tally')).value,
        isTrue,
      );
      expect(notes.byId(tally)!.place, home.id);

      // Two picks each time, Random.
      await tap(tester, key('session-form-count'));
      await tap(
        tester,
        find.descendant(
          of: key('session-count'),
          matching: find.byIcon(Icons.add),
        ),
      );
      await tap(tester, key('badge-close'));
      expect(find.text('Picks 2 Tallies'), findsOneWidget);
      await tap(tester, key('session-form-rule'));
      await tap(tester, key('session-rule-random'));

      await tap(tester, key('session-form-save'));
      final session = Sessions(store).all().single;
      expect(session.pool, [tally]);
      expect((session.count, session.rule), (2, SessionRule.random));
      // Every day, all day (#163).
      final schedule = session.routine.schedule;
      expect(schedule.times, isEmpty);
      expect(schedule.rule?.format(), 'FREQ=DAILY');
      expect(Routines(store).note(session.routine)!.place, home.id);
    });

    testWidgets('its schedule badge opens the Routine editor at All day', (
      tester,
    ) async {
      await openPage(tester, _Page.area);
      await pick(tester, NewKind.session);
      expect(
        find.descendant(
          of: key('session-form-schedule'),
          matching: find.textContaining('All day'),
        ),
        findsOneWidget,
      );
      await tap(tester, key('session-form-schedule'));
      expect(find.byType(RoutineEditorScreen), findsOneWidget);
      expect(find.text('Schedule'), findsWidgets);
      final allDay = find.widgetWithText(Chip, 'All day');
      await tester.scrollUntilVisible(allDay, 200);
      expect(allDay, findsOneWidget);
      await tap(tester, find.text('Save'));
      expect(find.byType(SessionFormScreen), findsOneWidget);
      expect(Routines(store).all(), isEmpty);
    });
  });

  testWidgets('a Member\'s fan offers what a Member may make: no Session, '
      'Frog or Topic', (tester) async {
    store
      ..alarmSetupDone = true
      ..applyProfileDefaults(name: 'Desk')
      ..renameOwnProfile('Ann');
    final ben = Store.inMemory()
      ..starterSetOffered = true
      ..alarmSetupDone = true
      ..applyProfileDefaults(name: 'Pixel')
      ..renameOwnProfile('Ben');
    addTearDown(ben.close);
    final area = store.share(home.id);
    // Ann's computer invites Ben's phone as a Member, as a LAN session
    // would.
    final done = answerSync(
      store,
      wire(
        syncRequest(
          ben,
          store.versionVector(),
          name: 'Phone',
          hostDevice: store.deviceId,
          invited: true,
        ),
      ),
      name: 'Desk',
      inviting: Invite(area, role: AreaRole.member),
    )!;
    applyAnswer(ben, wire(done.answer), shared: true);
    expect(ben.mayWriteIn(home.id), isTrue);
    expect(ben.mayManage(home.id), isFalse);

    await tester.pumpWidget(OldBrainFixApp(store: ben));
    await tap(tester, find.byTooltip('Search and browse'));
    await tap(tester, find.text('Household'));
    await tap(tester, find.byTooltip('Add here'));
    for (final kind in NewKind.values) {
      expect(key('addFan-${kind.name}'), switch (kind) {
        NewKind.session || NewKind.frog || NewKind.topic => findsNothing,
        _ => findsOneWidget,
      });
    }
  });

  group('leaving without saving makes nothing', () {
    testWidgets('the name dialogs, the Tally and the Session', (tester) async {
      await openPage(tester, _Page.area);
      for (final kind in [NewKind.checklist, NewKind.shopping, NewKind.frog]) {
        await pick(tester, kind);
        await tap(tester, find.text('Cancel'));
      }
      await pick(tester, NewKind.tally);
      await tester.enterText(key('tally-form-name'), 'Reading');
      await tester.enterText(key('tally-form-field-name-0'), 'Pages');
      await tester.pageBack();
      await tester.pumpAndSettle();
      await pick(tester, NewKind.session);
      await tester.enterText(key('session-form-name'), 'Morning');
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(notes.all(), isEmpty);
      expect(Routines(store).all(), isEmpty);
      expect(Frogs(store).open(), isEmpty);
    });
  });
}
