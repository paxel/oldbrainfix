import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/file_into_screen.dart';
import 'package:oldbrainfix/src/line_editing.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/routine_editor_screen.dart';
import 'package:oldbrainfix/src/tally_field_screen.dart';
import 'package:oldbrainfix/src/setting_badges.dart';
import 'package:oldbrainfix/src/text_marks_view.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// The Note editor's row of badges (#160): each setting as its icon and
/// value, a tap opening its small editor, "+" spawning from the Note.
void main() {
  late Store store;
  late Notes notes;
  late Places places;
  var now = DateTime.utc(2026, 10, 24, 8);

  setUp(() {
    now = DateTime.utc(2026, 10, 24, 8);
    store = Store.inMemory(clock: () => now)..starterSetOffered = true;
    notes = Notes(store);
    places = Places(store);
  });
  tearDown(() => store.close());

  Finder key(String name) => find.byKey(ValueKey(name));

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  /// The value [badge] shows.
  String value(WidgetTester tester, String badge) =>
      tester.widget<SettingBadge>(key(badge)).value ?? '';

  /// Opens the editor of the Note [id] from the Agenda, on a phone-tall
  /// page, on this Device or [on].
  Future<void> edit(WidgetTester tester, String id, {Store? on}) async {
    tester.view
      ..physicalSize = const Size(1080, 2340)
      ..devicePixelRatio = 3;
    addTearDown(tester.view.reset);
    final device = on ?? store;
    final book = Notes(device);
    await tester.pumpWidget(OldBrainFixApp(store: device));
    await tester.pumpAndSettle();
    final agenda = tester.element(find.byType(AgendaScreen));
    unawaited(openNote(agenda, book, Places(device), book.byId(id)!));
    await tester.pumpAndSettle();
    expect(find.byType(NoteEditorScreen), findsOneWidget);
  }

  final radios = find.byWidgetPredicate((w) => w is RadioGroup<NoteKind>);

  Finder inDialog(Finder finder) =>
      find.descendant(of: find.byType(AlertDialog), matching: finder);

  /// The title of the open badge editor, in words.
  Finder dialogTitle(String text) => inDialog(find.text(text));

  group('a plain Note', () {
    testWidgets('shows Importance, Area and Topic and Captured, its kind and '
        '"+"; no Show on the Agenda or Show ticks', (tester) async {
      final home = places.addArea('Household');
      final kitchen = places.addTopic(home.id, 'Kitchen');
      final id = notes.capture('sauce', place: kitchen.id).id;
      await edit(tester, id);
      expect(value(tester, 'badge-importance'), 'Normal');
      expect(value(tester, 'badge-place'), 'Household / Kitchen');
      expect(
        value(tester, 'badge-captured'),
        DateFormat.yMd('en_US').add_Hm().format(now.toLocal()),
      );
      expect(key('badge-add'), findsOneWidget);
      // Every Note has a kind (#188).
      expect(value(tester, 'badge-kind'), 'Plain Note');
      for (final none in ['badge-agenda', 'badge-ticks']) {
        expect(key(none), findsNothing, reason: none);
      }
      // One row under the text, before anything else of the page.
      final text = tester.getBottomLeft(find.byType(TextField).first);
      expect(tester.getTopLeft(key('note-badges')).dy, greaterThan(text.dy));
      // The bar has no ⋮ of its own any more.
      expect(key('editor-menu'), findsNothing);
    });

    testWidgets('a screen reader reads each badge as "setting: value"', (
      tester,
    ) async {
      final semantics = tester.ensureSemantics();
      final id = notes.capture('sauce').id;
      await edit(tester, id);
      expect(find.bySemanticsLabel('Importance: Normal'), findsOneWidget);
      expect(find.bySemanticsLabel('Area and Topic: Unsorted'), findsOneWidget);
      expect(find.bySemanticsLabel(RegExp(r'^Captured: ')), findsOneWidget);
      expect(find.bySemanticsLabel('Kind: Plain Note'), findsOneWidget);
      expect(
        find.bySemanticsLabel('Routine, Reminder, Frog or Session'),
        findsOneWidget,
      );
      expect(
        tester.getSemantics(key('badge-importance')),
        matchesSemantics(
          label: 'Importance: Normal',
          isButton: true,
          hasTapAction: true,
        ),
      );
      semantics.dispose();
    });

    testWidgets('Importance opens radio buttons; a pick changes it and the '
        'badge', (tester) async {
      final id = notes.capture('pills').id;
      await edit(tester, id);
      await tap(tester, key('badge-importance'));
      expect(dialogTitle('Importance'), findsOneWidget);
      expect(
        inDialog(find.byIcon(Icons.label_important_outline)),
        findsOneWidget,
      );
      expect(
        inDialog(find.byType(RadioListTile<Importance>)),
        findsNWidgets(3),
      );
      await tap(tester, key('importance-important'));
      expect(find.byType(AlertDialog), findsNothing);
      expect(notes.byId(id)!.importance, Importance.important);
      expect(value(tester, 'badge-importance'), 'Important');
    });

    testWidgets('Area and Topic opens File into…; the badge shows the place '
        'picked', (tester) async {
      places.addArea('Household');
      final id = notes.capture('sauce').id;
      await edit(tester, id);
      expect(value(tester, 'badge-place'), 'Unsorted');
      await tap(tester, key('badge-place'));
      expect(find.byType(FileIntoScreen), findsOneWidget);
      await tap(tester, find.text('Household'));
      expect(find.byType(NoteEditorScreen), findsOneWidget);
      expect(value(tester, 'badge-place'), 'Household');
      expect(notes.byId(id)!.isUnsorted, isFalse);
    });

    testWidgets('Captured shows the time, the calendar event and the '
        'Bluetooth device, each detail with a Remove', (tester) async {
      final id = notes.capture('meeting notes').id;
      notes.attachContext(
        id,
        calendarEvents: ['Dentist'],
        bluetoothDevices: ['Car'],
      );
      await edit(tester, id);
      await tap(tester, key('badge-captured'));
      expect(dialogTitle('Captured'), findsOneWidget);
      expect(inDialog(key('captured-details')), findsOneWidget);
      expect(
        inDialog(find.text(value(tester, 'badge-captured'))),
        findsOneWidget,
      );
      expect(inDialog(find.text('During: Dentist')), findsOneWidget);
      expect(inDialog(find.textContaining('Car')), findsOneWidget);
      // Close, and a Remove for the event and the device; none for the time.
      expect(inDialog(find.byType(TextButton)), findsNWidgets(3));
      await tap(tester, key('badge-close'));
      expect(find.byType(AlertDialog), findsNothing);
    });

    testWidgets('"+" offers Routine, Reminder, Frog and Session at the '
        'finger; a Routine shows as a badge with its schedule, which opens '
        'its editor', (tester) async {
      final id = notes.capture('dentist').id;
      await edit(tester, id);
      await tap(tester, key('badge-add'));
      expect(find.byType(BottomSheet), findsNothing);
      for (final spawn in ['routine', 'reminder', 'frog', 'session']) {
        expect(key('spawn-$spawn'), findsOneWidget, reason: spawn);
      }
      for (final words in ['Routine', 'Reminder', 'Frog', 'Session']) {
        expect(find.text(words), findsOneWidget, reason: words);
      }
      await tap(tester, key('spawn-routine'));
      expect(find.byType(RoutineEditorScreen), findsOneWidget);
      await tap(tester, find.text('Save'));

      final routine = Routines(store).ofNote(id).single;
      final badge = 'badge-routine-${routine.id}';
      expect(value(tester, badge), startsWith('Every day · '));
      await tap(tester, key(badge));
      expect(find.byType(RoutineEditorScreen), findsOneWidget);
      expect(find.text('From the Note'), findsOneWidget);
    });
  });

  group('each badge is the setting\'s icon and its value (#192)', () {
    /// The icons and the texts the badge [badge] draws.
    List<Object> drawn(WidgetTester tester, String badge) => [
      for (final icon in tester.widgetList<Icon>(
        find.descendant(of: key(badge), matching: find.byType(Icon)),
      ))
        icon.icon!,
      for (final text in tester.widgetList<Text>(
        find.descendant(of: key(badge), matching: find.byType(Text)),
      ))
        text.data!,
    ];

    testWidgets('a fixed set of values shows as an icon, never with its '
        'word; other values as short text', (tester) async {
      final home = places.addArea('Household');
      final id = notes.capture('Trip\nPassport\nTickets').id;
      notes.file(id, home.id);
      Checklists(store).make(id);
      notes.setImportance(id, Importance.important);
      await edit(tester, id);
      expect(drawn(tester, 'badge-kind'), [
        Icons.category_outlined,
        Icons.checklist,
      ]);
      expect(drawn(tester, 'badge-importance'), [
        Icons.label_important_outline,
        Icons.priority_high,
      ]);
      expect(drawn(tester, 'badge-agenda'), [
        Icons.today_outlined,
        Icons.format_list_bulleted,
      ]);
      // The same folder for every Area, its name as text.
      expect(drawn(tester, 'badge-place'), [
        Icons.folder_outlined,
        'Household',
      ]);
      // No badge shows a value both as an icon and in words.
      for (final badge in tester.widgetList<SettingBadge>(
        find.byType(SettingBadge),
      )) {
        if (badge.valueIcon == null) continue;
        expect(
          find.descendant(
            of: find.byWidget(badge),
            matching: find.byType(Text),
          ),
          findsNothing,
          reason: badge.setting,
        );
      }
    });

    testWidgets('a screen reader still reads the value in words', (
      tester,
    ) async {
      final semantics = tester.ensureSemantics();
      final id = notes.capture('Groceries\nMilk').id;
      ShoppingLists(store).make(id);
      notes.setImportance(id, Importance.unimportant);
      await edit(tester, id);
      expect(find.bySemanticsLabel('Kind: Shopping List'), findsOneWidget);
      expect(find.bySemanticsLabel('Importance: Unimportant'), findsOneWidget);
      expect(
        drawn(tester, 'badge-agenda').last,
        isA<IconData>(),
        reason: 'drawn as an icon',
      );
      expect(
        find.bySemanticsLabel(RegExp(r'^Show on the Agenda: .+')),
        findsOneWidget,
      );
      semantics.dispose();
    });
  });

  group('a Checklist', () {
    testWidgets('shows its kind, Show on the Agenda and Show ticks; Plain '
        'Note in the kind\'s editor turns it back', (tester) async {
      final id = notes.capture('Trip\nPassport\nTickets').id;
      Checklists(store).make(id);
      await edit(tester, id);
      expect(value(tester, 'badge-kind'), 'Checklist');
      expect(value(tester, 'badge-agenda'), 'Each open Line on its own row');
      expect(key('checklist-to-plain'), findsNothing, reason: 'in the badge');

      await tap(tester, key('badge-kind'));
      expect(dialogTitle('Kind'), findsOneWidget);
      expect(inDialog(find.byIcon(Icons.category_outlined)), findsOneWidget);
      expect(
        tester.widget<RadioGroup<NoteKind>>(inDialog(radios)).groupValue,
        NoteKind.checklist,
      );
      await tap(tester, key('kind-plain'));
      expect(find.byType(AlertDialog), findsNothing);
      expect(Checklists(store).isChecklist(id), isFalse);
      expect(value(tester, 'badge-kind'), 'Plain Note');
      expect(key('badge-agenda'), findsNothing);
      expect(
        tester.widget<TextField>(find.byType(TextField).first).controller!.text,
        'Trip\nPassport\nTickets',
      );
    });

    testWidgets('Show on the Agenda offers the three ways as radio '
        'buttons and writes at once', (tester) async {
      final id = notes.capture('Trip\nPassport').id;
      final lists = Checklists(store)..make(id);
      await edit(tester, id);
      expect(value(tester, 'badge-agenda'), 'Each open Line on its own row');
      await tap(tester, key('badge-agenda'));
      expect(dialogTitle('Show on the Agenda'), findsOneWidget);
      expect(key('badge-switch'), findsNothing, reason: 'no switch');
      expect(
        tester
            .widget<RadioGroup<ListOnAgenda>>(
              inDialog(
                find.byWidgetPredicate((w) => w is RadioGroup<ListOnAgenda>),
              ),
            )
            .groupValue,
        ListOnAgenda.lines,
      );
      await tap(tester, key('shopping-agenda-off'));
      expect(lists.onAgenda(id), ListOnAgenda.off);
      expect(find.byType(AlertDialog), findsNothing, reason: 'it closes');
      expect(value(tester, 'badge-agenda'), 'Off');
      await tap(tester, key('badge-agenda'));
      await tap(tester, key('shopping-agenda-count'));
      expect(lists.onAgenda(id), ListOnAgenda.count);
      expect(
        value(tester, 'badge-agenda'),
        'One row with the number of open Lines',
      );
    });

    testWidgets('with a Routine it has no Show on the Agenda', (tester) async {
      final id = notes.capture('Trip\nPassport').id;
      Checklists(store).make(id);
      Routines(store).spawn(
        id,
        Schedule(
          start: Day(2026, 10, 24),
          times: const [],
          rule: Recurrence.parse('FREQ=DAILY'),
        ),
      );
      await edit(tester, id);
      expect(key('badge-kind'), findsOneWidget);
      expect(key('badge-agenda'), findsNothing);
    });
  });

  group('the kind badge changes the kind (#188)', () {
    /// The kinds the open kind editor offers, by their keys.
    List<String> offered(WidgetTester tester) => [
      for (final kind in NoteKind.values)
        if (inDialog(key('kind-${kind.name}')).evaluate().isNotEmpty) kind.name,
    ];

    /// The fields of the Lines that hold [text].
    Finder lineFields(String text) => find.byWidgetPredicate(
      (w) =>
          w is TextField &&
          w.controller?.text.replaceAll(lineStart, '') == text,
    );

    List<String> shoppingLines(String id) => [
      for (final line
          in ShoppingLists(store).byNote(id, deviceWallClock)?.lines ??
              const <ShoppingLine>[])
        line.text,
    ];

    testWidgets('in Simple a plain Note offers no Tally nor Session, and '
        '"+" only Routine and Reminder', (tester) async {
      final id = notes.capture('Trip\nPassport').id;
      store.advanced = false;
      await edit(tester, id);
      await tap(tester, key('badge-kind'));
      expect(offered(tester), ['plain', 'checklist', 'shoppingList']);
      await tester.tapAt(Offset.zero);
      await tester.pumpAndSettle();
      await tap(tester, key('badge-add'));
      expect(key('spawn-routine'), findsOneWidget);
      expect(key('spawn-reminder'), findsOneWidget);
      expect(key('spawn-frog'), findsNothing);
      expect(key('spawn-session'), findsNothing);
    });

    testWidgets('a plain Note offers every kind as radio buttons, its own '
        'marked; Checklist makes it one at once, its first line the name, '
        'the rest its Lines', (tester) async {
      final id = notes.capture('Trip\nPassport\nTickets').id;
      await edit(tester, id);
      // The badge replaces the buttons at the bottom.
      expect(key('checklist-make'), findsNothing);
      expect(key('shopping-make'), findsNothing);
      await tap(tester, key('badge-kind'));
      expect(dialogTitle('Kind'), findsOneWidget);
      expect(inDialog(find.byIcon(Icons.category_outlined)), findsOneWidget);
      expect(offered(tester), [
        'plain',
        'checklist',
        'shoppingList',
        'tally',
        'session',
      ]);
      expect(
        tester.widget<RadioGroup<NoteKind>>(inDialog(radios)).groupValue,
        NoteKind.plain,
      );
      await tap(tester, key('kind-checklist'));
      expect(find.byType(AlertDialog), findsNothing);
      expect(Checklists(store).isChecklist(id), isTrue);
      expect(value(tester, 'badge-kind'), 'Checklist');
      expect(key('note-text'), findsNothing);
      expect(
        tester.widget<TextField>(find.byType(TextField).first).controller!.text,
        'Trip',
      );
      expect(
        [for (final line in Checklists(store).byNote(id)!.lines) line.text],
        ['Passport', 'Tickets'],
      );
      expect(lineFields('Passport'), findsOneWidget);
    });

    testWidgets('a Checklist becomes a Shopping List with its Lines, the '
        'checked one checked, and back; its text stays', (tester) async {
      final id = notes.capture('Trip\nPassport\nTickets').id;
      final lists = Checklists(store)..make(id);
      lists.check(lists.byNote(id)!.lines.first.id);
      final text = notes.byId(id)!.text;
      await edit(tester, id);
      await tap(tester, key('badge-kind'));
      expect(offered(tester), ['plain', 'checklist', 'shoppingList']);
      await tap(tester, key('kind-shoppingList'));
      expect(ShoppingLists(store).isShoppingList(id), isTrue);
      expect(lists.isChecklist(id), isFalse);
      expect(value(tester, 'badge-kind'), 'Shopping List');
      expect(shoppingLines(id), unorderedEquals(['Passport', 'Tickets']));
      final passport = ShoppingLists(store)
          .byNote(id, deviceWallClock)!
          .lines
          .firstWhere((l) => l.text == 'Passport');
      expect(passport.struck, isTrue);
      // Each Line once, in the Shopping List only.
      expect(key('shopping'), findsOneWidget);
      expect(key('checklist'), findsNothing);
      expect(lineFields('Tickets'), findsOneWidget);
      expect(notes.byId(id)!.text, text);

      await tap(tester, key('badge-kind'));
      expect(offered(tester), ['plain', 'checklist', 'shoppingList']);
      await tap(tester, key('kind-checklist'));
      expect(lists.isChecklist(id), isTrue);
      expect(value(tester, 'badge-kind'), 'Checklist');
      expect([
        for (final line in lists.byNote(id)!.lines) (line.text, line.checked),
      ], unorderedEquals([('Passport', true), ('Tickets', false)]));
      expect(key('checklist'), findsOneWidget);
      expect(key('shopping'), findsNothing);
      expect(notes.byId(id)!.text, text);
    });

    testWidgets('a Shopping List turns back into a plain Note, its Lines '
        'the lines of its text', (tester) async {
      final id = notes.capture('Groceries\nMilk\nBread').id;
      ShoppingLists(store).make(id);
      await edit(tester, id);
      await tap(tester, key('badge-kind'));
      await tap(tester, key('kind-plain'));
      expect(ShoppingLists(store).isShoppingList(id), isFalse);
      expect(value(tester, 'badge-kind'), 'Plain Note');
      expect(key('note-name'), findsNothing);
      final text = tester
          .widget<TextField>(
            find.descendant(
              of: key('note-text'),
              matching: find.byType(TextField),
            ),
          )
          .controller!
          .text;
      expect(text.split('\n').first, 'Groceries');
      expect(text, contains('Milk'));
      expect(text, contains('Bread'));
    });

    testWidgets('Tally opens the page of its first field', (tester) async {
      final id = notes.capture('Reading').id;
      await edit(tester, id);
      await tap(tester, key('badge-kind'));
      await tap(tester, key('kind-tally'));
      expect(find.byType(TallyFieldScreen), findsOneWidget);
      // Left without Save it stays a plain Note.
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(Tallies(store).isTally(id), isFalse);
      expect(value(tester, 'badge-kind'), 'Plain Note');
    });
  });

  testWidgets('Show ticks shows while the text has a box; its switch writes '
      'it for this Device', (tester) async {
    final id = notes.capture('☐ milk').id;
    await edit(tester, id);
    expect(value(tester, 'badge-ticks'), 'Off');
    await tap(tester, key('badge-ticks'));
    expect(dialogTitle('Show ticks'), findsOneWidget);
    await tap(tester, key('badge-switch'));
    await tap(tester, key('badge-close'));
    expect(showsTicks(store, id), isTrue);
    expect(value(tester, 'badge-ticks'), 'On');
    // Its icon, then the switch's as its value (#192).
    for (final icon in [Icons.history_toggle_off, Icons.toggle_on]) {
      expect(
        find.descendant(of: key('badge-ticks'), matching: find.byIcon(icon)),
        findsOneWidget,
      );
    }
  });

  testWidgets('a Shopping List and a Tally show their kind', (tester) async {
    final shop = notes.capture('Groceries\nMilk').id;
    ShoppingLists(store).make(shop);
    await edit(tester, shop);
    expect(value(tester, 'badge-kind'), 'Shopping List');
    await tap(tester, key('badge-kind'));
    expect(inDialog(key('kind-plain')), findsOneWidget);
    expect(inDialog(key('kind-checklist')), findsOneWidget);
    expect(inDialog(key('kind-tally')), findsNothing);
    await tap(tester, key('badge-close'));
    await tester.pageBack();
    await tester.pumpAndSettle();

    final reading = notes.capture('Reading').id;
    Tallies(store).addField(reading, name: 'Pages', unit: 'pages', target: 20);
    final agenda = tester.element(find.byType(AgendaScreen));
    unawaited(openNote(agenda, notes, places, notes.byId(reading)!));
    await tester.pumpAndSettle();
    // Under the Tally's content (#188).
    await tester.scrollUntilVisible(
      key('badge-kind'),
      200,
      scrollable: find
          .descendant(
            of: find.byType(NoteEditorScreen),
            matching: find.byType(Scrollable),
          )
          .first,
    );
    expect(value(tester, 'badge-kind'), 'Tally');
    // A Tally stays one: nothing to change there.
    expect(tester.widget<SettingBadge>(key('badge-kind')).onTap, isNull);
  });

  testWidgets('the badges fit a phone in German', (tester) async {
    tester.platformDispatcher.localesTestValue = const [Locale('de')];
    addTearDown(tester.platformDispatcher.clearLocalesTestValue);
    final id = notes.capture('Reise\nPass\nTickets ☐').id;
    Checklists(store).make(id);
    await edit(tester, id);
    // A Checklist leaves Unsorted.
    expect(value(tester, 'badge-place'), 'Nicht abgelegt');
    expect(
      value(tester, 'badge-agenda'),
      'Jede offene Zeile in einer eigenen Reihe',
    );
    final semantics = tester.ensureSemantics();
    expect(find.bySemanticsLabel('Wichtigkeit: Normal'), findsOneWidget);
    expect(find.bySemanticsLabel('Art: Checkliste'), findsOneWidget);
    semantics.dispose();
    await tap(tester, key('badge-add'));
    for (final words in ['Routine', 'Erinnerung', 'Frosch', 'Einheit']) {
      expect(find.text(words), findsOneWidget, reason: words);
    }
    expect(tester.takeException(), isNull);
  });

  group('removing a captured detail (ADR 0037)', () {
    /// A Note captured at a place, shared in from a chat, during a
    /// meeting and with a headset connected.
    String captured(Store on, {String? place}) {
      final book = Notes(on);
      final id = book
          .capture(
            'Order toner',
            place: place,
            sourceApp: const SourceApp('org.example.chat', label: 'Chat'),
          )
          .id;
      book.locate(id, Coordinates(52.52, 13.405));
      book.attachContext(
        id,
        calendarEvents: ['Katzenklo'],
        bluetoothDevices: ['Headset'],
      );
      return id;
    }

    Finder remove(CapturedDetail detail) =>
        key('remove-captured-${detail.name}');

    testWidgets('each detail goes on its own after asking once, the dialog '
        'follows and the time stays', (tester) async {
      final id = captured(store);
      final at = notes.byId(id)!.captured;
      await edit(tester, id);
      final time = value(tester, 'badge-captured');
      await tap(tester, key('badge-captured'));
      for (final detail in CapturedDetail.values) {
        expect(inDialog(remove(detail)), findsOneWidget, reason: '$detail');
      }
      final semantics = tester.ensureSemantics();
      expect(find.bySemanticsLabel('Remove the place'), findsOneWidget);
      semantics.dispose();

      final lines = {
        CapturedDetail.place: find.byIcon(Icons.place_outlined),
        CapturedDetail.app: find.text('Shared from Chat'),
        CapturedDetail.event: find.text('During: Katzenklo'),
        CapturedDetail.device: find.text('Connected: Headset'),
      };
      for (final MapEntry(key: detail, value: line) in lines.entries) {
        expect(inDialog(line), findsOneWidget, reason: '$detail');
        await tap(tester, remove(detail));
        if (detail == CapturedDetail.place) {
          expect(find.text('Remove the place from this Note?'), findsOneWidget);
        }
        await tap(tester, key('remove-media-confirm'));
        // The badge's dialog stays open and shows what is left.
        expect(dialogTitle('Captured'), findsOneWidget);
        expect(inDialog(line), findsNothing, reason: '$detail');
        expect(remove(detail), findsNothing, reason: '$detail');
        expect(
          notes.byId(id)!.context.holds(detail),
          isFalse,
          reason: '$detail',
        );
      }
      final note = notes.byId(id)!;
      expect(note.context.hasMore, isFalse);
      expect(note.captured, at);
      expect(note.text, 'Order toner');
      expect(inDialog(find.text(time)), findsOneWidget);
      // Only Close is left: the time has no Remove.
      expect(inDialog(find.byType(TextButton)), findsOneWidget);
      await tap(tester, key('badge-close'));
      expect(value(tester, 'badge-captured'), time);
    });

    testWidgets('Cancel keeps the detail', (tester) async {
      final id = captured(store);
      await edit(tester, id);
      await tap(tester, key('badge-captured'));
      await tap(tester, remove(CapturedDetail.event));
      await tap(tester, find.text('Cancel'));
      expect(inDialog(find.text('During: Katzenklo')), findsOneWidget);
      expect(inDialog(remove(CapturedDetail.event)), findsOneWidget);
      expect(notes.byId(id)!.context.calendarEvents, ['Katzenklo']);
    });

    testWidgets('a Section removes its own in the dialog its line under it '
        'opens; the Note keeps its', (tester) async {
      final id = captured(store);
      final section = notes.amend(id, 'Ordered').id;
      notes.attachContext(section, calendarEvents: ['Standup']);
      await edit(tester, id);
      // One weak line in the editor, its details in the dialog (#170).
      final line = key('captured-line:$section');
      expect(
        find.descendant(of: line, matching: find.textContaining('Standup')),
        findsOneWidget,
      );
      expect(remove(CapturedDetail.event), findsNothing);
      // Read once, as a badge is.
      final semantics = tester.ensureSemantics();
      expect(
        find.bySemanticsLabel(RegExp(r'^Captured: .* · Standup$')),
        findsOneWidget,
      );
      semantics.dispose();
      await tester.ensureVisible(line);
      await tester.pumpAndSettle();
      await tap(tester, line);
      expect(dialogTitle('Captured'), findsOneWidget);
      final lines = key('captured-details:$section');
      final button = find.descendant(
        of: lines,
        matching: remove(CapturedDetail.event),
      );
      expect(
        find.descendant(of: lines, matching: remove(CapturedDetail.place)),
        findsNothing,
      );
      await tap(tester, button);
      expect(
        find.text('Remove the calendar events from this Section?'),
        findsOneWidget,
      );
      await tap(tester, key('remove-media-confirm'));
      expect(
        find.descendant(of: lines, matching: find.text('During: Standup')),
        findsNothing,
      );
      expect(button, findsNothing);
      expect(notes.sections(id).single.context.calendarEvents, isEmpty);
      expect(notes.byId(id)!.context.calendarEvents, ['Katzenklo']);
      await tap(tester, key('badge-close'));
      expect(
        find.descendant(of: line, matching: find.textContaining('Standup')),
        findsNothing,
      );
    });

    group('in a Shared Area', () {
      late Store ben;
      late String house;
      late String anns;

      Map<String, Object?> wire(Map<String, Object?> json) =>
          jsonDecode(jsonEncode(json)) as Map<String, Object?>;

      setUp(() {
        store
          ..applyProfileDefaults(name: 'Desk')
          ..renameOwnProfile('Ann');
        ben = Store.inMemory(clock: () => now)
          ..starterSetOffered = true
          ..alarmSetupDone = true
          ..applyProfileDefaults(name: 'Pixel')
          ..renameOwnProfile('Ben');
        house = places.addArea('Household').id;
        anns = captured(store, place: house);
        final area = store.share(house);
        // Ann's computer invites Ben's phone as a Member.
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
      });
      tearDown(() => ben.close());

      testWidgets("a Member sees no Remove on another person's Note", (
        tester,
      ) async {
        await edit(tester, anns, on: ben);
        await tap(tester, key('badge-captured'));
        expect(inDialog(find.text('During: Katzenklo')), findsOneWidget);
        for (final detail in CapturedDetail.values) {
          expect(remove(detail), findsNothing, reason: '$detail');
        }
      });

      testWidgets("a Member sees another person's place by its coordinates, "
          'their Named Place staying theirs', (tester) async {
        NamedPlaces(store).add('Home', Coordinates(52.52, 13.405), radius: 100);
        expect(Notes(store).byId(anns)!.context.namedPlace?.name, 'Home');
        await edit(tester, anns, on: ben);
        await tap(tester, key('badge-captured'));
        expect(inDialog(find.text('52.5200, 13.4050')), findsOneWidget);
        expect(find.textContaining('Home'), findsNothing);
      });

      testWidgets('a Member removes a detail of their own Note', (
        tester,
      ) async {
        final own = captured(ben, place: house);
        await edit(tester, own, on: ben);
        await tap(tester, key('badge-captured'));
        await tap(tester, remove(CapturedDetail.device));
        await tap(tester, key('remove-media-confirm'));
        expect(Notes(ben).byId(own)!.context.bluetoothDevices, isEmpty);
        expect(
          find.text('A captured detail could not be removed.'),
          findsNothing,
        );
      });
    });
  });
}
