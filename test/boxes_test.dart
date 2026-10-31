import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:intl/date_symbol_data_local.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/box_ids.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/text_marks_view.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'open_capture.dart';

Map<String, Object?> wire(Map<String, Object?> json) =>
    jsonDecode(jsonEncode(json)) as Map<String, Object?>;

/// Boxes in a Note's and a Section's text (ADR 0035): a ☐, ✅ or ❌
/// anywhere in it gets an invisible id when the text is saved; a tap
/// opens Check, Fail and Open at the finger, the current state left out,
/// and the choice writes a tick beside the text, never an edit of it; on
/// full and compact cards, the Agenda's just-captured card, in Sections
/// and in the editor, whose row above the keyboard puts ☐ in at the
/// cursor. Show ticks says who ticked each box last; its History lists
/// every change. In the field the ids go with their glyphs.
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
    for (final state in BoxState.values)
      if (key('box-${state.name}').evaluate().isNotEmpty) state.name,
  ];

  Future<void> closeMenu(WidgetTester tester) async {
    await tester.tapAt(Offset.zero);
    await tester.pumpAndSettle();
  }

  /// The boxes of the Note [id] as they show now.
  List<ShownBox> boxesOf(String id, [Store? on]) =>
      Notes(on ?? store).byId(id)!.boxes;

  Finder boxKey(ShownBox box) => key('box-${box.id}');

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

  /// "You, Sat 08:00" for a tick of this Device at [at].
  String yours(DateTime at) =>
      'You, ${DateFormat.E('en_US').add_Hm().format(at.toLocal())}';

  group('on a card', () {
    testWidgets('a tap on a box offers the other two states and writes a '
        'tick, the text unchanged', (tester) async {
      final id = notes.capture('Release\n- ☐ v2 at the customer\n✅ Docs').id;
      final stored = notes.byId(id)!.text;
      final [first, second] = boxesOf(id);
      await startAtNotes(tester);

      await tap(tester, boxKey(first));
      expect(offered(), ['done', 'failed']);
      expect(find.text('Check'), findsOneWidget);
      expect(find.text('Fail'), findsOneWidget);
      // Nothing ticked it yet: no History.
      expect(key('box-history'), findsNothing);
      await tap(tester, key('box-done'));
      expect(boxesOf(id).first.state, BoxState.done);
      expect(notes.byId(id)!.text, stored);
      expect(
        notes.byId(id)!.shownText,
        'Release\n- ✅ v2 at the customer\n✅ Docs',
      );

      await tap(tester, boxKey(first));
      expect(offered(), ['open', 'failed']);
      await tap(tester, key('box-failed'));
      await tap(tester, boxKey(second));
      expect(offered(), ['open', 'failed']);
      await tap(tester, key('box-open'));
      expect(
        notes.byId(id)!.shownText,
        'Release\n- ❌ v2 at the customer\n☐ Docs',
      );

      // Closing the menu changes nothing.
      await tap(tester, boxKey(second));
      await closeMenu(tester);
      expect(
        notes.byId(id)!.shownText,
        'Release\n- ❌ v2 at the customer\n☐ Docs',
      );
      expect(notes.byId(id)!.text, stored);
    });

    testWidgets('a box in the middle of a word is one too, a Section\'s '
        'box is ticked, and a screen reader reads the state', (tester) async {
      final semantics = tester.ensureSemantics();
      final id = notes.capture('Call Ann☐ later').id;
      final section = notes.amend(id, '☐ Later');
      await startAtNotes(tester);

      final word = boxesOf(id).single;
      expect(boxKey(word), findsOneWidget);
      expect(
        tester.getSemantics(boxKey(word)),
        matchesSemantics(
          label: 'Open',
          isButton: true,
          hasTapAction: true,
          onTapHint: 'change the box',
        ),
      );
      final later = notes.sections(id).single.boxes.single;
      final written = notes.sections(id).single.text;
      await tap(tester, boxKey(later));
      expect(offered(), ['done', 'failed']);
      await tap(tester, key('box-done'));
      final after = notes.sections(id).single;
      expect(after.id, section.id);
      expect(after.boxes.single.state, BoxState.done);
      expect(after.shownText, '✅ Later');
      expect(after.text, written);
      expect(
        tester.getSemantics(boxKey(later)),
        matchesSemantics(
          label: 'Checked',
          isButton: true,
          hasTapAction: true,
          onTapHint: 'change the box',
        ),
      );
      semantics.dispose();
    });

    testWidgets('a compact card ticks the box of its first line and does '
        'not unfold', (tester) async {
      final id = notes.capture('☐ Call Ann\nabout the car').id;
      await startAtNotes(tester);
      await tap(tester, find.byTooltip('Show Notes compact'));
      expect(key('note-first-line'), findsOneWidget);

      await tap(tester, boxKey(boxesOf(id).single));
      await tap(tester, key('box-done'));
      expect(notes.byId(id)!.shownText, '✅ Call Ann\nabout the car');
      expect(key('note-first-line'), findsOneWidget);
    });

    testWidgets('the Agenda\'s just-captured card ticks them too', (
      tester,
    ) async {
      await start(tester);
      await openTextCapture(tester);
      await tester.enterText(find.byType(TextField), '☐ Pay rent');
      await tap(tester, find.text('Save'));
      expect(find.text('Just captured'), findsOneWidget);

      final note = notes.all().single;
      await tap(tester, boxKey(note.boxes.single));
      await tap(tester, key('box-failed'));
      expect(notes.all().single.shownText, '❌ Pay rent');
      expect(notes.all().single.text, note.text);
      expect(find.byType(NoteEditorScreen), findsNothing);
    });

    testWidgets('Show ticks in ⋮ says who ticked each box last and when, '
        'for that Note on this Device only', (tester) async {
      final id = notes.capture('☐ Milk ☐ Eggs').id;
      final other = notes.capture('☐ Bread').id;
      notes.capture('No boxes here');
      await startAtNotes(tester);
      Finder menuOf(String text) => find
          .descendant(
            of: find.ancestor(
              of: find.textContaining(text, findRichText: true),
              matching: find.byType(NoteCard),
            ),
            matching: key('note-menu'),
          )
          .first;

      // A Note without boxes has no Show ticks.
      await tap(tester, menuOf('No boxes'));
      expect(key('note-show-ticks'), findsNothing);
      await closeMenu(tester);

      final milk = boxesOf(id).first;
      await tap(tester, boxKey(milk));
      await tap(tester, key('box-done'));
      expect(find.textContaining('You, ', findRichText: true), findsNothing);

      await tap(tester, menuOf('Milk'));
      expect(find.text('Show ticks'), findsOneWidget);
      expect(tester.widget<Switch>(find.byType(Switch)).value, isFalse);
      await tap(tester, key('note-show-ticks'));
      expect(store.localSetting(showTicksKey(id)), 'true');
      // After the ticked box only; the other Note shows none.
      expect(
        find.textContaining(yours(now), findRichText: true),
        findsOneWidget,
      );
      expect(store.localSetting(showTicksKey(other)), isNull);

      await tap(tester, menuOf('Milk'));
      expect(tester.widget<Switch>(find.byType(Switch)).value, isTrue);
      await tap(tester, key('note-show-ticks'));
      expect(find.textContaining('You, ', findRichText: true), findsNothing);
    });

    testWidgets('History lists every change: the state, who and when', (
      tester,
    ) async {
      final id = notes.capture('Plan ☐').id;
      final box = boxesOf(id).single;
      await startAtNotes(tester);
      final first = now;
      await tap(tester, boxKey(box));
      await tap(tester, key('box-done'));
      now = now.add(const Duration(hours: 2));
      await tap(tester, boxKey(box));
      await tap(tester, key('box-failed'));

      await tap(tester, boxKey(box));
      expect(offered(), ['open', 'done']);
      await tap(tester, key('box-history'));
      final dialog = key('box-history-dialog');
      expect(dialog, findsOneWidget);
      final lines = find.descendant(
        of: dialog,
        matching: find.byType(ListTile),
      );
      expect(lines, findsNWidgets(2));
      String said(int i) => [
        for (final text in tester.widgetList<Text>(
          find.descendant(of: lines.at(i), matching: find.byType(Text)),
        ))
          text.data,
      ].join(' / ');
      final full = DateFormat.yMMMEd('en_US').add_Hm();
      expect(said(0), '✅ / Checked / You, ${full.format(first.toLocal())}');
      expect(said(1), '❌ / Failed / You, ${full.format(now.toLocal())}');
      await tap(tester, find.text('Close'));
      expect(dialog, findsNothing);
    });
  });

  group('in the editor', () {
    Future<void> openEditor(WidgetTester tester) async {
      await tester.longPress(find.byType(NoteCard).first);
      await tester.pumpAndSettle();
    }

    TagTextController field(WidgetTester tester) =>
        tester.widget<EditableText>(find.byType(EditableText)).controller
            as TagTextController;

    Future<void> showKeyboard(WidgetTester tester) async {
      await tester.showKeyboard(find.byType(TextField));
      tester.view.viewInsets = const FakeViewPadding(bottom: 900);
      addTearDown(tester.view.resetViewInsets);
      await tester.pumpAndSettle();
    }

    /// The app goes to the background and comes back: the editor keeps
    /// its text.
    Future<void> background(WidgetTester tester) async {
      for (final state in [
        AppLifecycleState.inactive,
        AppLifecycleState.hidden,
        AppLifecycleState.paused,
        AppLifecycleState.hidden,
        AppLifecycleState.inactive,
        AppLifecycleState.resumed,
      ]) {
        tester.binding.handleAppLifecycleStateChanged(state);
      }
      await tester.pumpAndSettle();
    }

    testWidgets('a tap on a box in the field ticks it, the text as it was; '
        'it is drawn in its state', (tester) async {
      final id = notes.capture('Plan\n☐ Book\nTalk ✅ later').id;
      final stored = notes.byId(id)!.text;
      await startAtNotes(tester);
      await openEditor(tester);
      final [book, talk] = TextBoxes.find(field(tester).text);

      await tester.tapAt(charAt(tester, book.start, book.end));
      await tester.pumpAndSettle();
      expect(offered(), ['done', 'failed']);
      await tap(tester, key('box-done'));
      expect(field(tester).text, stored);
      expect(boxesOf(id).first.state, BoxState.done);
      // The field shows it ticked; its id is never seen.
      final shown = tester
          .state<EditableTextState>(find.byType(EditableText))
          .buildTextSpan()
          .toPlainText();
      expect(withoutIds(shown), 'Plan\n✅ Book\nTalk ✅ later');

      await tester.tapAt(charAt(tester, talk.start, talk.end));
      await tester.pumpAndSettle();
      expect(offered(), ['open', 'failed']);
      await closeMenu(tester);

      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(notes.byId(id)!.text, stored);
      expect(notes.byId(id)!.shownText, 'Plan\n✅ Book\nTalk ✅ later');
    });

    testWidgets('the row above the keyboard shows a box in its state too', (
      tester,
    ) async {
      final id = notes.capture('☐ Book').id;
      Boxes(store).tick(id, boxesOf(id).single.id, BoxState.done);
      await startAtNotes(tester);
      await openEditor(tester);
      await showKeyboard(tester);
      field(tester).selection = const TextSelection.collapsed(offset: 0);
      await tester.pumpAndSettle();
      final row = find.descendant(
        of: key('two-row-bar'),
        matching: find.byType(RichText),
      );
      final shown = [
        for (final text in tester.widgetList<RichText>(row))
          text.text.toPlainText(),
      ].join();
      expect(withoutIds(shown), contains('✅ Book'));
      expect(shown, isNot(contains('☐')));
    });

    testWidgets('the row above the keyboard reads a line with a box from '
        'its start, the cursor right after the box and its id', (tester) async {
      notes.capture('Plan\n☐ Book');
      await startAtNotes(tester);
      await openEditor(tester);
      await showKeyboard(tester);
      final controller = field(tester);
      final box = TextBoxes.find(controller.text).single;
      controller.selection = TextSelection.collapsed(offset: box.end);
      await tester.pumpAndSettle();
      double edge(String of, int offset) {
        final paragraph = tester.renderObject<RenderParagraph>(
          find.descendant(of: key(of), matching: find.byType(RichText)),
        );
        final caret = paragraph.getOffsetForCaret(
          TextPosition(offset: offset),
          Rect.zero,
        );
        return paragraph.localToGlobal(caret).dx;
      }

      String shown(String of) => tester
          .widget<RichText>(
            find.descendant(of: key(of), matching: find.byType(RichText)),
          )
          .text
          .toPlainText();
      final cursor = key('two-row-cursor');
      final start = tester.getTopLeft(key('two-row-line')).dx;
      expect(withoutIds(shown('two-row-before')), '☐');
      expect(shown('two-row-after'), ' Book');
      expect(edge('two-row-before', 0), closeTo(start, 0.5));
      final before = shown('two-row-before').length;
      expect(
        edge('two-row-before', before),
        closeTo(tester.getTopLeft(cursor).dx, 0.5),
      );

      // ◀ steps over the box and its id at once; the letters stay.
      final was = tester.getTopLeft(cursor).dx;
      await tester.tap(key('two-row-back'));
      await tester.pumpAndSettle();
      expect(controller.selection.baseOffset, box.start);
      expect(tester.getTopLeft(cursor).dx, closeTo(start, 0.5));
      expect(tester.getTopLeft(cursor).dx, lessThan(was));
      expect(
        edge('two-row-after', 0),
        closeTo(tester.getTopRight(cursor).dx, 0.5),
      );
    });

    testWidgets('the system\'s text field, with two rows off, ticks them '
        'too', (tester) async {
      store.twoRowEditorOn = false;
      final id = notes.capture('Plan\n❌ Book').id;
      await startAtNotes(tester);
      await openEditor(tester);
      final box = TextBoxes.find(field(tester).text).single;

      await tester.tapAt(charAt(tester, box.start, box.end));
      await tester.pumpAndSettle();
      expect(offered(), ['open', 'done']);
      await tap(tester, key('box-open'));
      expect(notes.byId(id)!.shownText, 'Plan\n☐ Book');
    });

    testWidgets('the row above the keyboard puts ☐ in at the cursor, also '
        'in the middle of a word', (tester) async {
      final id = notes.capture('Shopping list').id;
      await startAtNotes(tester);
      await openEditor(tester);
      await showKeyboard(tester);
      final controller = field(tester);
      controller.selection = const TextSelection.collapsed(offset: 4);
      await tester.pump();

      await tap(tester, key('two-row-insert-box'));
      expect(controller.text, 'Shop☐ping list');
      expect(controller.selection, const TextSelection.collapsed(offset: 5));

      await tester.pageBack();
      await tester.pumpAndSettle();
      final note = notes.byId(id)!;
      expect(note.shownText, 'Shop☐ping list');
      expect(note.boxes.single.state, BoxState.open);
    });

    testWidgets('a ✅ typed becomes a box when the text is saved, and the '
        'field takes it back with its id, the cursor where it was', (
      tester,
    ) async {
      final id = notes.capture('Done').id;
      await startAtNotes(tester);
      await openEditor(tester);
      await showKeyboard(tester);
      await tester.enterText(find.byType(TextField), 'Done ✅ today');
      final controller = field(tester);
      controller.selection = const TextSelection.collapsed(offset: 6);
      await tester.pump();

      await background(tester);
      final stored = notes.byId(id)!.text;
      expect(notes.byId(id)!.shownText, 'Done ✅ today');
      final box = TextBoxes.find(stored).single;
      expect(box.state, BoxState.done);
      expect(stored, isNot('Done ✅ today'));
      expect(controller.text, stored);
      // After the box, its id with it.
      expect(controller.selection, TextSelection.collapsed(offset: box.end));

      // Ticked, then typed on: the id stays, and so does its tick.
      await tester.tapAt(charAt(tester, box.start, box.end));
      await tester.pumpAndSettle();
      await tap(tester, key('box-failed'));
      await tester.enterText(
        find.byType(TextField),
        '${stored.substring(0, box.end)}!${stored.substring(box.end)}',
      );
      await tester.pump();
      await tester.pageBack();
      await tester.pumpAndSettle();
      final note = notes.byId(id)!;
      expect(note.boxes.single.id, box.id);
      expect(note.boxes.single.state, BoxState.failed);
      expect(note.shownText, 'Done ❌! today');
    });

    testWidgets('Show ticks in the editor\'s ⋮ says who ticked a box after '
        'it, in the field and in a Section', (tester) async {
      final id = notes.capture('Plan ☐').id;
      notes.amend(id, '☐ Later');
      Boxes(store).tick(id, boxesOf(id).single.id, BoxState.done);
      final section = notes.sections(id).single;
      Boxes(store).tick(section.id, section.boxes.single.id, BoxState.done);
      await startAtNotes(tester);
      await openEditor(tester);
      expect(find.textContaining('You, ', findRichText: true), findsNothing);

      await tap(tester, key('editor-menu'));
      await tap(tester, key('note-show-ticks'));
      expect(showsTicks(store, id), isTrue);
      expect(
        find.textContaining(yours(now), findRichText: true),
        findsNWidgets(2),
      );
      // The field's text is the same.
      expect(field(tester).text, notes.byId(id)!.text);
    });

    testWidgets('a Note without boxes has no ⋮', (tester) async {
      notes.capture('Plan');
      await startAtNotes(tester);
      await openEditor(tester);
      expect(key('editor-menu'), findsNothing);
    });
  });

  group('the ids in the field', () {
    const formatter = BoxIdsFormatter();
    final id = TextBoxes.encodeId('a7k2');
    final text = 'Buy ☐${id}milk';
    const start = 4;
    final end = start + 1 + id.length;

    TextEditingValue value(String text, int at) => TextEditingValue(
      text: text,
      selection: TextSelection.collapsed(offset: at),
    );

    test('a cursor or a selection inside an id goes out of it', () {
      expect(
        snapSelection(text, TextSelection.collapsed(offset: start + 1)),
        TextSelection.collapsed(offset: end),
      );
      expect(
        snapSelection(text, TextSelection.collapsed(offset: start + 3)),
        TextSelection.collapsed(offset: end),
      );
      // Stepping back from after the box goes before it.
      expect(
        snapSelection(
          text,
          TextSelection.collapsed(offset: end - 2),
          before: TextSelection.collapsed(offset: end),
        ),
        const TextSelection.collapsed(offset: start),
      );
      expect(
        snapSelection(text, TextSelection(baseOffset: 0, extentOffset: 6)),
        TextSelection(baseOffset: 0, extentOffset: end),
      );
      // A controller never holds one there.
      final controller = TagTextController(text: text);
      addTearDown(controller.dispose);
      controller.selection = TextSelection.collapsed(offset: start + 2);
      expect(controller.selection.baseOffset, end);
    });

    test('typing before and after a box keeps its id with the glyph', () {
      final typed = formatter.formatEditUpdate(
        value(text, end),
        value('Buy ☐${id}x milk', end + 2),
      );
      expect(typed.text, 'Buy ☐${id}x milk');
      expect(TextBoxes.find(typed.text).single.id, 'a7k2');
      final before = formatter.formatEditUpdate(
        value(text, start),
        value('Buy x☐${id}milk', start + 1),
      );
      expect(TextBoxes.find(before.text).single.id, 'a7k2');
    });

    test('typing inside an id goes after the box', () {
      final inside = formatter.formatEditUpdate(
        value(text, start + 3),
        value(
          '${text.substring(0, start + 3)}x${text.substring(start + 3)}',
          start + 4,
        ),
      );
      expect(inside.text, 'Buy ☐${id}xmilk');
      expect(inside.selection, TextSelection.collapsed(offset: end + 1));
    });

    test('deleting part of a box deletes all of it; deleting the glyph '
        'deletes its id', () {
      // A keyboard that takes one code point: the cancel tag.
      final part = formatter.formatEditUpdate(
        value(text, end),
        value(text.substring(0, end - 2) + text.substring(end), end - 2),
      );
      expect(part.text, 'Buy milk');
      expect(part.selection, const TextSelection.collapsed(offset: start));
      final glyph = formatter.formatEditUpdate(
        value(text, start + 1),
        value(text.replaceFirst('☐', ''), start),
      );
      expect(glyph.text, 'Buy milk');
      // The whole box, as backspace takes it, stays gone.
      final whole = formatter.formatEditUpdate(
        value(text, end),
        value('Buy milk', start),
      );
      expect(whole.text, 'Buy milk');
    });

    test('a flag\'s tag characters are left alone', () {
      const england =
          '\u{1F3F4}\u{E0067}\u{E0062}\u{E0065}\u{E006E}'
          '\u{E0067}\u{E007F}';
      final next = formatter.formatEditUpdate(
        value(text, end),
        value('$text $england', text.length + 1 + england.length),
      );
      expect(next.text, '$text $england');
    });

    test('ids put in move a cursor past them', () {
      const typed = 'a ✅ b';
      final stored = 'a ✅${TextBoxes.encodeId('zz11')} b';
      expect(idFreeOffset(stored, idFreeCount(typed, 3)), 3 + 10);
      expect(idFreeOffset(stored, idFreeCount(typed, 2)), 2);
      expect(withoutIds(stored), typed);
    });
  });

  group('read only', () {
    testWidgets('a text stored with ids, as a Sent Item carries it, shows '
        'each box in its glyph, without its id and without a button', (
      tester,
    ) async {
      final id = notes.capture('☐ Milk ☐ Eggs').id;
      Boxes(store).tick(id, boxesOf(id).first.id, BoxState.done);
      final sent = TextBoxes.baked(
        notes.byId(id)!.text,
        Boxes(store).ticks(id),
      );
      expect(TextBoxes.find(sent), hasLength(2));
      await tester.pumpWidget(
        MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: Scaffold(body: MarkedText(sent)),
        ),
      );
      // Read by a screen reader as the states they show.
      final drawn = tester.widget<RichText>(find.byType(RichText)).text;
      expect(drawn.toPlainText(includeSemanticsLabels: false), '✅ Milk ☐ Eggs');
      expect(drawn.toPlainText(), 'Checked Milk Open Eggs');
      expect(
        find.byWidgetPredicate(
          (w) => w.key is ValueKey<String> && '${w.key}'.contains("'box-"),
        ),
        findsNothing,
      );
    });
  });

  group('in a Shared Area', () {
    late Store ben;
    late String note;

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

    testWidgets('a Member ticks the box of another\'s Note, on the card and '
        'in the editor, whose text they cannot change', (tester) async {
      final stored = Notes(ben).byId(note)!.text;
      final box = boxesOf(note, ben).single;
      await startAtNotes(tester, ben);
      await tap(tester, boxKey(box));
      expect(offered(), ['done', 'failed']);
      await tap(tester, key('box-done'));
      expect(boxesOf(note, ben).single.state, BoxState.done);
      expect(Notes(ben).byId(note)!.text, stored);

      await tester.longPress(
        find.textContaining('Fix the tap', findRichText: true),
      );
      await tester.pumpAndSettle();
      expect(
        tester.widget<EditableText>(find.byType(EditableText)).readOnly,
        isTrue,
      );
      final found = TextBoxes.find(
        tester.widget<EditableText>(find.byType(EditableText)).controller.text,
      ).single;
      await tester.tapAt(charAt(tester, found.start, found.end));
      await tester.pumpAndSettle();
      expect(offered(), ['open', 'failed']);
      // Its History says who: Ben himself on his phone.
      await tap(tester, key('box-history'));
      expect(find.textContaining('You, '), findsOneWidget);
      await tap(tester, find.text('Close'));
      expect(Notes(ben).byId(note)!.text, stored);
    });
  });
}
