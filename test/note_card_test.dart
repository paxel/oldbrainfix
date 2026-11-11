import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:intl/date_symbol_data_local.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix/src/sent_item_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'bundle_test.dart' show FakeBundles;
import 'fake_calendar.dart';

void main() {
  final now = DateTime.utc(2026, 10, 24, 9);
  late Store store;
  late Notes notes;

  setUpAll(initializeDateFormatting);

  setUp(() {
    store = Store.inMemory(clock: () => now)
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    notes = Notes(store);
  });
  tearDown(() => store.close());

  const ann = Profile(id: 'ann', name: 'Ann', version: 1);

  /// [note] as a card with every action, in [locale] at [width] dp.
  Future<void> card(
    WidgetTester tester,
    Note note, {
    Profile? author,
    Area? area,
    Topic? topic,
    bool showPlace = false,
    String locale = 'en',
    double width = 800,
    bool full = false,
  }) async {
    tester.view.physicalSize = Size(width, 1200);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      MaterialApp(
        locale: Locale(locale),
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(
          body: ListView(
            children: [
              NoteCard(
                note: note,
                when: DateFormat.yMd(locale).add_Hm(),
                author: author,
                area: area,
                topic: topic,
                showPlace: showPlace,
                onFile: () {},
                onMerge: () {},
                onSchedule: (_) {},
                actions: NoteActions(onSend: () {}, onDelete: () {}),
                full: full,
              ),
            ],
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  double top(WidgetTester tester, Finder finder) =>
      tester.getTopLeft(finder).dy;

  group('order', () {
    testWidgets('another person\'s Note, detailed: Profile, separator, Note, '
        'its Capture Context, separator, actions', (tester) async {
      final note = notes.capture('Buy milk');
      await card(tester, note, author: ann, full: true);

      final date = DateFormat.yMd('en')
          .add_Hm()
          .format(note.captured.toLocal());
      final dividers = find.byType(Divider);
      expect(dividers, findsNWidgets(2));
      final order = [
        top(tester, find.byKey(const ValueKey('author'))),
        top(tester, dividers.first),
        top(tester, find.text('Buy milk')),
        top(tester, find.text(date)),
        top(tester, dividers.last),
        top(tester, find.byTooltip('File into…')),
      ];
      expect(find.byKey(const ValueKey('author-name')), findsOneWidget);
      expect(find.text('Ann'), findsOneWidget);
      for (var i = 1; i < order.length; i++) {
        expect(order[i], greaterThan(order[i - 1]), reason: 'step $i');
      }
    });

    testWidgets('another person\'s Note, compact: Profile, separator, Note, '
        'its ⋮ top right, no Capture Context and no buttons at the bottom '
        '(#189)', (tester) async {
      final note = notes.capture('Buy milk');
      await card(tester, note, author: ann);

      final date = DateFormat.yMd('en')
          .add_Hm()
          .format(note.captured.toLocal());
      expect(find.text(date), findsNothing);
      expect(find.byType(Divider), findsOneWidget);
      expect(find.byKey(const ValueKey('note-actions')), findsNothing);
      expect(find.byTooltip('File into…'), findsNothing);
      final order = [
        top(tester, find.byKey(const ValueKey('author'))),
        top(tester, find.byType(Divider)),
        top(tester, find.text('Buy milk')),
      ];
      for (var i = 1; i < order.length; i++) {
        expect(order[i], greaterThan(order[i - 1]), reason: 'step $i');
      }
      final menu = find.byKey(const ValueKey('note-menu'));
      expect(
        tester.getTopLeft(menu).dy,
        lessThan(top(tester, find.byKey(const ValueKey('author')))),
      );
      expect(
        tester.getTopLeft(menu).dx,
        greaterThan(tester.getTopRight(find.text('Ann')).dx),
      );
    });

    testWidgets('one\'s own Note has no Profile line', (tester) async {
      final note = notes.capture('Buy milk');
      await card(tester, note);

      expect(find.byKey(const ValueKey('author')), findsNothing);
      expect(find.byKey(const ValueKey('author-name')), findsNothing);
      // Nothing over the text, so no separator under it.
      expect(find.byType(Divider), findsNothing);
    });

    testWidgets('where it is filed comes after the Profile, when shown', (
      tester,
    ) async {
      final places = Places(store);
      final home = places.addArea('Household');
      final kitchen = places.addTopic(home.id, 'Kitchen');
      final note = notes.capture('Buy milk', place: kitchen.id);
      await card(
        tester,
        note,
        author: ann,
        area: home,
        topic: kitchen,
        showPlace: true,
      );

      final order = [
        top(tester, find.byKey(const ValueKey('author'))),
        top(tester, find.text('Household / Kitchen')),
        top(tester, find.byType(Divider).first),
        top(tester, find.text('Buy milk')),
      ];
      for (var i = 1; i < order.length; i++) {
        expect(order[i], greaterThan(order[i - 1]), reason: 'step $i');
      }
    });

    testWidgets('where it is filed stays out unless the screen asks', (
      tester,
    ) async {
      final places = Places(store);
      final home = places.addArea('Household');
      final kitchen = places.addTopic(home.id, 'Kitchen');
      final filed = notes.capture('Buy milk', place: kitchen.id);
      await card(tester, filed, area: home, topic: kitchen);
      expect(find.text('Household / Kitchen'), findsNothing);

      await card(tester, notes.capture('Buy bread'));
      expect(find.text('Unsorted'), findsNothing);
      await card(tester, notes.capture('Buy eggs'), showPlace: true);
      expect(find.text('Unsorted'), findsOneWidget);
    });

    testWidgets('a card without actions ends with the Note', (tester) async {
      final note = notes.capture('Buy milk');
      await tester.pumpWidget(
        MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: Scaffold(
            body: NoteCard(note: note, when: DateFormat.yMd('en').add_Hm()),
          ),
        ),
      );
      await tester.pumpAndSettle();

      expect(find.byType(Divider), findsNothing);
      expect(find.byKey(const ValueKey('note-actions')), findsNothing);
      expect(find.byKey(const ValueKey('note-menu')), findsNothing);
    });
  });

  for (final locale in ['en', 'de']) {
    testWidgets('the action row stays one line at 360 dp ($locale)', (
      tester,
    ) async {
      final note = notes.capture('Buy milk and bread for the weekend');
      await card(
        tester,
        note,
        author: ann,
        locale: locale,
        width: 360,
        full: true,
      );

      expect(tester.takeException(), isNull);
      final row = find.byKey(const ValueKey('note-actions'));
      final buttons = [
        find.descendant(
          of: row,
          matching: find.byIcon(Icons.drive_file_move_outline),
        ),
        find.descendant(
          of: row,
          matching: find.byIcon(Icons.alarm_add_outlined),
        ),
        find.descendant(of: row, matching: find.byIcon(Icons.call_merge)),
        find.descendant(of: row, matching: find.byIcon(Icons.more_vert)),
      ];
      final centre = tester.getCenter(buttons.first).dy;
      for (final button in buttons) {
        expect(button, findsOneWidget);
        expect(tester.getCenter(button).dy, centre);
      }
      // Words only as tooltips: none of them is drawn in the row.
      expect(
        find.descendant(of: row, matching: find.byType(Text)),
        findsNothing,
      );
      expect(tester.getRect(row).right, lessThanOrEqualTo(360));
    });
  }

  group('the ⋮ menu', () {
    late Directory dir;
    late FakeBundles files;
    late FakeCalendar calendar;

    setUp(() {
      dir = Directory.systemTemp.createTempSync('note_card_');
      files = FakeBundles(dir);
      calendar = FakeCalendar();
    });
    tearDown(() => dir.deleteSync(recursive: true));

    /// All Notes, with the ⋮ of [text]'s card open.
    Future<void> menu(WidgetTester tester, String text) async {
      tester.view.physicalSize = const Size(800, 2400);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.reset);
      await tester.pumpWidget(
        OldBrainFixApp(store: store, bundles: files, calendar: calendar),
      );
      await tester.pumpAndSettle();
      for (final step in [
        find.byTooltip('Search and browse'),
        find.byTooltip('All Notes'),
        find.descendant(
          of: find.ancestor(
            of: find.text(text),
            matching: find.byType(NoteCard),
          ),
          matching: find.byKey(const ValueKey('note-menu')),
        ),
      ]) {
        await tester.tap(step);
        await tester.pumpAndSettle();
      }
    }

    /// Taps the ⋮ item [key] after scrolling it on screen: the menu is
    /// longer than the test surface, and it scrolls.
    Future<void> pick(WidgetTester tester, String key) async {
      await tester.ensureVisible(find.byKey(ValueKey(key)));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(ValueKey(key)));
      await tester.pumpAndSettle();
    }

    testWidgets('offers sending, the calendar, adding and deleting with '
        'words', (tester) async {
      notes.capture('Buy milk');
      await menu(tester, 'Buy milk');

      for (final words in [
        'Send to a person',
        'Add to calendar',
        'Add to Note',
        'Delete',
      ]) {
        expect(find.text(words), findsOneWidget, reason: words);
      }
      // Without Routines there are no events to share as a file.
      expect(find.text('Share as calendar file'), findsNothing);
    });

    testWidgets('has no Importance: its badge in the editor changes it '
        '(#182)', (tester) async {
      notes.capture('Buy milk');
      await menu(tester, 'Buy milk');

      for (final words in [
        'Importance',
        'Important',
        'Normal',
        'Unimportant',
      ]) {
        expect(find.text(words), findsNothing, reason: words);
      }
      for (final value in Importance.values) {
        expect(
          find.byKey(ValueKey('importance-${value.name}')),
          findsNothing,
          reason: value.name,
        );
      }
    });

    testWidgets('Send to a person sends it as a Sent Item', (tester) async {
      notes.capture('Dentist on Friday');
      await menu(tester, 'Dentist on Friday');
      await pick(tester, 'note-send');

      expect(find.byType(SendNoteDialog), findsOneWidget);
      await tester.tap(find.byKey(const ValueKey('send-note')));
      await tester.pumpAndSettle();
      final sent = files.shared.single;
      expect(files.types.single, sentItemMimeType);
      expect(
        openSentItem(store, File(sent).readAsBytesSync()).item.text,
        'Dentist on Friday',
      );
    });

    testWidgets('Share as calendar file shows with Routines and shares '
        'their events', (tester) async {
      final note = notes.capture('Dentist');
      Routines(store).spawn(
        note.id,
        Schedule(start: Day(2026, 11, 3), times: [ClockTime(10, 0)]),
      );
      await menu(tester, 'Dentist');
      await pick(tester, 'note-calendar-file');

      expect(calendar.shared.single.$2, contains('Dentist'));
    });

    testWidgets('Add to Note adds a Section', (tester) async {
      final note = notes.capture('Buy milk');
      await menu(tester, 'Buy milk');
      await pick(tester, 'note-amend');
      await tester.enterText(find.byType(TextField).last, 'and bread');
      await tester.tap(find.text('Add'));
      await tester.pumpAndSettle();

      expect(notes.sections(note.id).single.text, 'and bread');
    });

    testWidgets('Delete moves the Note to the trash without asking; the '
        'announcement at the top brings it back (#268)', (tester) async {
      final note = notes.capture('Buy milk');
      await menu(tester, 'Buy milk');
      await pick(tester, 'note-delete');

      expect(find.byType(AlertDialog), findsNothing);
      expect(notes.byId(note.id)!.isTrashed, isTrue);
      expect(find.text('Buy milk'), findsNothing);
      expect(find.text('Moved to the Trash'), findsOneWidget);

      await tester.tap(find.text('Moved to the Trash'));
      await tester.pumpAndSettle();
      expect(notes.byId(note.id)!.isTrashed, isFalse);
      expect(find.text('Buy milk'), findsOneWidget);
    });

    testWidgets('Delete takes the Note\'s Routines along, and the undo '
        'brings them back', (tester) async {
      final note = notes.capture('Dentist');
      final routines = Routines(store);
      routines.spawn(
        note.id,
        Schedule(start: Day(2026, 11, 3), times: [ClockTime(10, 0)]),
      );
      await menu(tester, 'Dentist');
      await pick(tester, 'note-delete');

      expect(notes.byId(note.id)!.isTrashed, isTrue);
      expect(routines.ofNote(note.id), isEmpty);
      expect(routines.all(), isEmpty);
      await tester.tap(find.text('Moved to the Trash'));
      await tester.pumpAndSettle();
      expect(routines.ofNote(note.id), hasLength(1));
    });

    testWidgets('the undo does nothing once the trash restored it', (
      tester,
    ) async {
      final note = notes.capture('Buy milk');
      await menu(tester, 'Buy milk');
      await pick(tester, 'note-delete');
      // As the Trash's Restore does.
      Routines(store).restoreNote(note.id);
      await tester.tap(find.text('Moved to the Trash'));
      await tester.pumpAndSettle();
      expect(notes.byId(note.id)!.isTrashed, isFalse);
      expect(tester.takeException(), isNull);
    });
  });
}
