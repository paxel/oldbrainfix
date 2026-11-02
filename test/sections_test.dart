import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/text_marks_view.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'reveal.dart';

/// A Section's text, an Amend's or a merged Note's, is edited in the Note
/// editor as the Note's text is, by the same people, and shows when it
/// was edited and by whom; a text that is only read is selected and
/// copied (#183).
void main() {
  late Store store;
  late Notes notes;
  var now = DateTime.utc(2026, 11, 1, 8);

  setUp(() {
    now = DateTime.utc(2026, 11, 1, 8);
    store = Store.inMemory(clock: () => now)
      ..starterSetOffered = true
      ..alarmSetupDone = true
      ..applyProfileDefaults(name: 'Desk')
      ..renameOwnProfile('Ann');
    notes = Notes(store);
  });
  tearDown(() => store.close());

  Finder key(String name) => find.byKey(ValueKey(name));

  /// [utc] as the editor shows it; only valid once the app loaded its
  /// date formats.
  String shown(DateTime utc) =>
      DateFormat.yMd('en').add_Hm().format(utc.toLocal());

  /// The field of the Section [id] in the editor.
  Finder fieldOf(String id) => find.descendant(
    of: key('section-text:$id'),
    matching: find.byType(TextField),
  );

  TextEditingController controllerOf(WidgetTester tester, String id) =>
      tester.widget<TextField>(fieldOf(id)).controller!;

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

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

  /// Leaves the editor, which saves.
  Future<void> leave(WidgetTester tester) async {
    await tester.pageBack();
    await tester.pumpAndSettle();
  }

  testWidgets('an Amend is edited in place and saved on leaving; the log '
      'keeps the old text', (tester) async {
    final note = notes.capture('Dentist on Tuesday');
    now = now.add(const Duration(hours: 1));
    final amend = notes.amend(note.id, 'Moved to Wednesday');
    now = now.add(const Duration(minutes: 1));
    final later = notes.amend(note.id, 'Called, all set');
    await edit(tester, note.id);

    expect(controllerOf(tester, amend.id).text, 'Moved to Wednesday');
    expect(key('section-edited'), findsNothing);
    now = now.add(const Duration(hours: 2));
    await tester.enterText(fieldOf(amend.id), 'Moved to Thursday');
    await leave(tester);

    final section = notes.section(amend.id)!;
    expect(section.text, 'Moved to Thursday');
    expect(section.created, amend.created);
    expect(section.editedBy?.name, 'Ann');
    expect(
      [for (final s in notes.sections(note.id)) s.id],
      [amend.id, later.id],
    );
    expect(
      [
        for (final e in store.history(amend.id))
          if (e.field == SectionKeys.text) StoredText.read(e.value).text,
      ],
      ['Moved to Wednesday', 'Moved to Thursday'],
    );
    // The other Section and the Note's text stay as they were.
    expect(notes.section(later.id)!.modified, isNull);
    expect(
      store.history(note.id).where((e) => e.field == NoteKeys.text),
      hasLength(1),
    );
  });

  testWidgets('an edited Section shows when it was captured, when it was '
      'edited and by whom', (tester) async {
    final note = notes.capture('Dentist');
    final amend = notes.amend(note.id, 'Moved to Wednesday');
    now = now.add(const Duration(days: 1, hours: 3));
    notes.editSection(amend.id, 'Moved to Thursday');
    await edit(tester, note.id);

    // Under its text, its time only: it was added the Note's day (#170).
    final lines = key('captured-line:${amend.id}');
    expect(
      find.descendant(
        of: lines,
        matching: find.text(
          DateFormat.Hm('en').format(DateTime.utc(2026, 11, 1, 8).toLocal()),
        ),
      ),
      findsOneWidget,
    );
    expect(
      find.descendant(
        of: lines,
        matching: find.text(
          'Edited ${shown(DateTime.utc(2026, 11, 2, 11))} by Ann',
        ),
      ),
      findsOneWidget,
    );
  });

  testWidgets('the edit and who made it show on the full card too', (
    tester,
  ) async {
    final note = notes.capture('Dentist');
    final amend = notes.amend(note.id, 'Moved to Wednesday');
    now = now.add(const Duration(hours: 1));
    notes.editSection(amend.id, 'Moved to Thursday');
    await tester.pumpWidget(OldBrainFixApp(store: store));
    await tester.pumpAndSettle();
    await tap(tester, find.byTooltip('Search and browse'));
    await tap(tester, find.byTooltip('All Notes'));

    expect(find.text('Moved to Thursday'), findsOneWidget);
    expect(
      find.text('Edited ${shown(DateTime.utc(2026, 11, 1, 9))} by Ann'),
      findsOneWidget,
    );
  });

  testWidgets('the boxes of an edited Section keep their ids and ticks', (
    tester,
  ) async {
    final note = notes.capture('Trip');
    final amend = notes.amend(note.id, 'Tent ☐');
    final box = notes.section(amend.id)!.boxes.single.id;
    Boxes(store).tick(amend.id, box, BoxState.done);
    await edit(tester, note.id);

    final field = controllerOf(tester, amend.id);
    // The field holds the text as stored, the box's id in it.
    expect(field.text, notes.section(amend.id)!.text);
    final typed = '${field.text} and torch ☐';
    field.value = TextEditingValue(
      text: typed,
      selection: TextSelection.collapsed(offset: typed.length),
    );
    await tester.pump();
    await leave(tester);

    final section = notes.section(amend.id)!;
    expect(section.shownText, 'Tent ✅ and torch ☐');
    expect(section.boxes, hasLength(2));
    expect(section.boxes.first.id, box);
    expect(section.boxes.first.state, BoxState.done);
  });

  testWidgets('a tap on a box in a Section\'s field opens its menu there', (
    tester,
  ) async {
    final note = notes.capture('Trip');
    final amend = notes.amend(note.id, 'Tent ☐');
    final box = notes.section(amend.id)!.boxes.single;
    await edit(tester, note.id);

    final editable = tester
        .state<EditableTextState>(
          find.descendant(
            of: key('section-text:${amend.id}'),
            matching: find.byType(EditableText),
          ),
        )
        .renderEditable;
    final rect = editable
        .getBoxesForSelection(
          TextSelection(
            baseOffset: box.sourceStart,
            extentOffset: box.sourceEnd,
          ),
        )
        .first
        .toRect();
    await tester.tapAt(editable.localToGlobal(rect.center));
    await tester.pumpAndSettle();
    await tap(tester, key('box-done'));
    expect(notes.section(amend.id)!.boxes.single.state, BoxState.done);
    expect(notes.section(amend.id)!.modified, isNull);
  });

  testWidgets('a format given in a Section\'s field is written at once', (
    tester,
  ) async {
    final note = notes.capture('Trip');
    final amend = notes.amend(note.id, 'Tent');
    await edit(tester, note.id);

    await tester.enterText(fieldOf(amend.id), '**Tent**');
    await tester.pumpAndSettle();
    expect(notes.section(amend.id)!.textFormat, TextFormats.markdown);
    expect(notes.section(amend.id)!.text, '**Tent**');
  });

  testWidgets('going to the background saves a Section\'s text too', (
    tester,
  ) async {
    final note = notes.capture('Trip');
    final amend = notes.amend(note.id, 'Tent');
    await edit(tester, note.id);

    await tester.enterText(fieldOf(amend.id), 'Tent and torch');
    for (final state in [
      AppLifecycleState.inactive,
      AppLifecycleState.hidden,
      AppLifecycleState.paused,
    ]) {
      tester.binding.handleAppLifecycleStateChanged(state);
    }
    await tester.pump();
    expect(notes.section(amend.id)!.text, 'Tent and torch');
    expect(controllerOf(tester, amend.id).text, 'Tent and torch');
  });

  testWidgets('a blanked Section keeps its text', (tester) async {
    final note = notes.capture('Trip');
    final amend = notes.amend(note.id, 'Tent');
    await edit(tester, note.id);

    await tester.enterText(fieldOf(amend.id), '   ');
    await leave(tester);
    expect(notes.section(amend.id)!.text, 'Tent');
    expect(notes.section(amend.id)!.modified, isNull);
  });

  testWidgets('a Section of a Note in the trash is only read', (tester) async {
    final note = notes.capture('Trip');
    final amend = notes.amend(note.id, 'Tent');
    notes.trash(note.id);
    await edit(tester, note.id);

    expect(key('section-text:${amend.id}'), findsNothing);
    expect(find.text('Tent'), findsOneWidget);
    expect(
      find.ancestor(of: find.text('Tent'), matching: find.byType(MarkedText)),
      findsOneWidget,
    );
  });

  group('in a Shared Area', () {
    late Store ben;
    late String house;
    late String anns;
    late String annsAmend;

    Map<String, Object?> wire(Map<String, Object?> json) =>
        jsonDecode(jsonEncode(json)) as Map<String, Object?>;

    setUp(() {
      ben = Store.inMemory(clock: () => now)
        ..starterSetOffered = true
        ..alarmSetupDone = true
        ..applyProfileDefaults(name: 'Pixel')
        ..renameOwnProfile('Ben');
      house = Places(store).addArea('Household').id;
      anns = notes.capture('Buy milk', place: house).id;
      annsAmend = notes.amend(anns, 'Two litres').id;
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

    testWidgets('a Member edits their own Amend on another person\'s Note, '
        'not the Note nor another person\'s Amend', (tester) async {
      final own = Notes(ben).amend(anns, 'Oat milk please').id;
      await edit(tester, anns, on: ben);

      expect(key('section-text:$annsAmend'), findsNothing);
      expect(
        find.ancestor(
          of: find.text('Two litres'),
          matching: find.byType(MarkedText),
        ),
        findsOneWidget,
      );
      final note = tester.widget<TextField>(
        find.descendant(of: key('note-text'), matching: find.byType(TextField)),
      );
      expect(note.readOnly, isTrue);

      await reveal(tester, fieldOf(own));
      await tester.enterText(fieldOf(own), 'Oat milk, two litres');
      await leave(tester);
      expect(Notes(ben).section(own)!.text, 'Oat milk, two litres');
      expect(Notes(ben).section(own)!.editedBy?.name, 'Ben');
      expect(Notes(ben).section(annsAmend)!.text, 'Two litres');
    });

    testWidgets('another person\'s Amend is selected and copied in the '
        'editor', (tester) async {
      final copied = <String?>[];
      tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SystemChannels.platform,
        (call) async {
          if (call.method == 'Clipboard.setData') {
            copied.add((call.arguments as Map)['text'] as String?);
          }
          return null;
        },
      );
      addTearDown(
        () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
          SystemChannels.platform,
          null,
        ),
      );
      await edit(tester, anns, on: ben);

      await tester.longPress(find.text('Two litres'));
      await tester.pumpAndSettle();
      await tap(tester, find.text('Select all'));
      await tap(tester, find.text('Copy'));
      expect(copied, ['Two litres']);
    });
  });
}
