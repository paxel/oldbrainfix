import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:intl/date_symbol_data_local.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'open_capture.dart';

Map<String, Object?> wire(Map<String, Object?> json) =>
    jsonDecode(jsonEncode(json)) as Map<String, Object?>;

/// Status marks at the start of a line of a Note's text (#125, ADR 0031):
/// a tap opens Check, Fail and Open at the finger, the current state left
/// out, and the choice edits the text; on full and compact cards, the
/// Agenda's just-captured card and in the editor, whose row above the
/// keyboard puts ☐ at the start of the cursor's line.
void main() {
  late Store store;
  late Notes notes;
  var now = DateTime.utc(2026, 10, 24, 8);

  setUpAll(initializeDateFormatting);

  setUp(() {
    now = DateTime.utc(2026, 10, 24, 8);
    store = Store.inMemory(clock: () => now)
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    notes = Notes(store);
  });
  tearDown(() => store.close());

  Finder key(String value) => find.byKey(ValueKey(value));

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  Future<void> start(WidgetTester tester, [Store? on]) async {
    tester.view.physicalSize = const Size(800, 2400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(OldBrainFixApp(store: on ?? store));
    await tester.pumpAndSettle();
  }

  Future<void> startAtNotes(WidgetTester tester, [Store? on]) async {
    await start(tester, on);
    await tap(tester, find.byTooltip('Search and browse'));
    await tap(tester, find.byTooltip('All Notes'));
  }

  /// The states the open menu offers, by name.
  List<String> offered() => [
    for (final mark in TextMark.values)
      if (key('mark-${mark.name}').evaluate().isNotEmpty) mark.name,
  ];

  Future<void> closeMenu(WidgetTester tester) async {
    await tester.tapAt(Offset.zero);
    await tester.pumpAndSettle();
  }

  /// The middle of the characters [start] to [end] in the top row.
  Offset charAt(WidgetTester tester, int start, int end) {
    final editable = tester
        .state<EditableTextState>(find.byType(EditableText))
        .renderEditable;
    final box = editable
        .getBoxesForSelection(
          TextSelection(baseOffset: start, extentOffset: end),
        )
        .first;
    return editable.localToGlobal(box.toRect().center);
  }

  group('on a card', () {
    testWidgets('a tap on a mark at the start of a line offers the other '
        'two states and edits the text', (tester) async {
      final id = notes.capture('Release\n- ☐ v2 at the customer\n✅ Docs').id;
      await startAtNotes(tester);

      await tap(tester, key('mark-1'));
      expect(offered(), ['checked', 'failed']);
      expect(find.text('Check'), findsOneWidget);
      expect(find.text('Fail'), findsOneWidget);
      await tap(tester, key('mark-checked'));
      expect(notes.byId(id)!.text, 'Release\n- ✅ v2 at the customer\n✅ Docs');

      await tap(tester, key('mark-1'));
      expect(offered(), ['open', 'failed']);
      await tap(tester, key('mark-failed'));
      expect(notes.byId(id)!.text, 'Release\n- ❌ v2 at the customer\n✅ Docs');

      await tap(tester, key('mark-2'));
      expect(offered(), ['open', 'failed']);
      await tap(tester, key('mark-open'));
      expect(notes.byId(id)!.text, 'Release\n- ❌ v2 at the customer\n☐ Docs');

      // Closing the menu changes nothing.
      await tap(tester, key('mark-2'));
      await closeMenu(tester);
      expect(notes.byId(id)!.text, 'Release\n- ❌ v2 at the customer\n☐ Docs');
    });

    testWidgets('a mark in the middle of a line, or in a Section, is plain '
        'text; a screen reader reads the state', (tester) async {
      final semantics = tester.ensureSemantics();
      final id = notes.capture('v2 supported ✅\n-☐ no bullet\n☐ Call').id;
      notes.amend(id, '☐ Later');
      await startAtNotes(tester);

      expect(key('mark-0'), findsNothing);
      expect(key('mark-1'), findsNothing);
      expect(key('mark-2'), findsOneWidget);
      // The Section's line counts from 0 too, and has no button.
      expect(find.textContaining('Later', findRichText: true), findsOneWidget);
      await tester.tap(find.textContaining('supported'));
      await tester.pumpAndSettle();
      expect(offered(), isEmpty);
      expect(
        tester.getSemantics(key('mark-2')),
        matchesSemantics(
          label: 'Open',
          isButton: true,
          hasTapAction: true,
          onTapHint: 'change the mark',
        ),
      );
      semantics.dispose();
    });

    testWidgets('a compact card changes the mark of its first line and does '
        'not unfold', (tester) async {
      final id = notes.capture('☐ Call Ann\nabout the car').id;
      await startAtNotes(tester);
      await tap(tester, find.byTooltip('Show Notes compact'));
      expect(key('note-first-line'), findsOneWidget);

      await tap(tester, key('mark-0'));
      await tap(tester, key('mark-checked'));
      expect(notes.byId(id)!.text, '✅ Call Ann\nabout the car');
      expect(key('note-first-line'), findsOneWidget);
    });

    testWidgets('the Agenda\'s just-captured card changes them too', (
      tester,
    ) async {
      await start(tester);
      await openTextCapture(tester);
      await tester.enterText(find.byType(TextField), '☐ Pay rent');
      await tap(tester, find.text('Save'));
      expect(find.text('Just captured'), findsOneWidget);

      await tap(tester, key('mark-0'));
      await tap(tester, key('mark-failed'));
      expect(notes.all().single.text, '❌ Pay rent');
      expect(find.byType(NoteEditorScreen), findsNothing);
    });
  });

  group('in the editor', () {
    Future<void> openEditor(WidgetTester tester) async {
      await tester.longPress(find.byType(NoteCard).first);
      await tester.pumpAndSettle();
    }

    TextEditingController field(WidgetTester tester) =>
        tester.widget<EditableText>(find.byType(EditableText)).controller;

    testWidgets('a tap on a mark at the start of a line changes it in the '
        'text; one in the middle of a line opens nothing', (tester) async {
      final id = notes.capture('Plan\n☐ Book\nTalk ✅ later').id;
      await startAtNotes(tester);
      await openEditor(tester);

      await tester.tapAt(charAt(tester, 5, 6));
      await tester.pumpAndSettle();
      expect(offered(), ['checked', 'failed']);
      await tap(tester, key('mark-checked'));
      expect(field(tester).text, 'Plan\n✅ Book\nTalk ✅ later');

      final middle = 'Plan\n✅ Book\nTalk '.length;
      await tester.tapAt(charAt(tester, middle, middle + 1));
      await tester.pumpAndSettle();
      expect(offered(), isEmpty);

      // Leaving saves it, as any text typed.
      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(notes.byId(id)!.text, 'Plan\n✅ Book\nTalk ✅ later');
    });

    testWidgets('the system\'s text field, with two rows off, changes '
        'them too', (tester) async {
      store.twoRowEditorOn = false;
      notes.capture('Plan\n❌ Book');
      await startAtNotes(tester);
      await openEditor(tester);

      await tester.tapAt(charAt(tester, 5, 6));
      await tester.pumpAndSettle();
      expect(offered(), ['open', 'checked']);
      await tap(tester, key('mark-open'));
      expect(field(tester).text, 'Plan\n☐ Book');
    });

    testWidgets('the row above the keyboard puts ☐ at the start of the '
        'cursor\'s line', (tester) async {
      final id = notes.capture('Release\n  - v2 at the customer').id;
      await startAtNotes(tester);
      await openEditor(tester);
      await tester.showKeyboard(find.byType(TextField));
      tester.view.viewInsets = const FakeViewPadding(bottom: 900);
      addTearDown(tester.view.resetViewInsets);
      await tester.pumpAndSettle();
      final controller = field(tester);
      controller.selection = const TextSelection.collapsed(offset: 16);
      await tester.pump();

      await tap(tester, key('two-row-insert-mark'));
      expect(controller.text, 'Release\n  - ☐ v2 at the customer');
      expect(controller.selection, const TextSelection.collapsed(offset: 18));
      // A line with a mark already gets no second one.
      await tap(tester, key('two-row-insert-mark'));
      expect(controller.text, 'Release\n  - ☐ v2 at the customer');

      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(notes.byId(id)!.text, 'Release\n  - ☐ v2 at the customer');
    });
  });

  group('in a Shared Area', () {
    late Store ben;
    late String note;
    late String houseId;

    setUp(() {
      store
        ..applyProfileDefaults(name: 'Desk')
        ..renameOwnProfile('Ann');
      ben = Store.inMemory()
        ..starterSetOffered = true
        ..alarmSetupDone = true
        ..applyProfileDefaults(name: 'Pixel')
        ..renameOwnProfile('Ben');
      final house = Places(store).addArea('Household');
      houseId = house.id;
      note = notes.capture('☐ Fix the tap', place: house.id).id;
      final area = store.share(house.id);
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

    testWidgets('a Member cannot change the marks of another\'s Note, on '
        'the card or in the editor; those of their own they can', (
      tester,
    ) async {
      final own = Notes(ben).capture('☐ Buy a washer', place: houseId).id;
      await startAtNotes(tester, ben);
      expect(key('mark-0'), findsOneWidget);
      await tap(tester, key('mark-0'));
      await tap(tester, key('mark-checked'));
      expect(Notes(ben).byId(own)!.text, '✅ Buy a washer');
      final anns = find.ancestor(
        of: find.textContaining('Fix the tap', findRichText: true),
        matching: find.byType(NoteCard),
      );
      expect(find.descendant(of: anns, matching: key('mark-0')), findsNothing);
      await tester.tap(find.textContaining('Fix the tap', findRichText: true));
      await tester.pumpAndSettle();
      expect(offered(), isEmpty);

      await tester.longPress(
        find.textContaining('Fix the tap', findRichText: true),
      );
      await tester.pumpAndSettle();
      await tester.tapAt(charAt(tester, 0, 1));
      await tester.pumpAndSettle();
      expect(offered(), isEmpty);
      expect(Notes(ben).byId(note)!.text, '☐ Fix the tap');
    });
  });
}
