import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'add_a_part.dart';
import 'editor_actions.dart';
import 'open_capture.dart';

/// Amend only when asked (obf-1072): the Note editor never shows the
/// Amend box on its own. A Note being made has no "Add a part"; an
/// existing Note has it among its actions, and it opens the box.
void main() {
  late Store store;
  late Notes notes;
  late Places places;

  setUp(() {
    store = Store.inMemory(clock: () => DateTime.utc(2026, 11, 24, 8))
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    notes = Notes(store);
    places = Places(store);
  });
  tearDown(() => store.close());

  Finder key(String name) => find.byKey(ValueKey(name));

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  /// Starts the app on a phone 360 wide.
  Future<void> start(WidgetTester tester) async {
    tester.view
      ..physicalSize = const Size(1080, 2340)
      ..devicePixelRatio = 3;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(OldBrainFixApp(store: store));
    await tester.pumpAndSettle();
  }

  /// Opens the editor of the Note [id] from the Agenda.
  Future<void> edit(WidgetTester tester, String id) async {
    await start(tester);
    final agenda = tester.element(find.byType(AgendaScreen));
    unawaited(openNote(agenda, notes, places, notes.byId(id)!));
    await tester.pumpAndSettle();
    expect(find.byType(NoteEditorScreen), findsOneWidget);
  }

  /// No Amend box on the page, and no "Add a part" in its groups at the
  /// page's end (obf-1073).
  Future<void> expectNoAmend(WidgetTester tester) async {
    expect(find.text("What's new?"), findsNothing);
    expect(find.text('Add to Note'), findsNothing);
    await revealInEditor(tester, key('editor-group-share'));
    expect(addAPartAction, findsNothing);
    expect(find.text('Add a part'), findsNothing);
  }

  Future<void> addPart(WidgetTester tester, String text) async {
    await addAPart(tester);
    await tester.enterText(
      find.descendant(
        of: find.byType(AlertDialog),
        matching: find.byType(TextField),
      ),
      text,
    );
    await tap(
      tester,
      find.descendant(of: find.byType(AlertDialog), matching: find.text('Add')),
    );
  }

  testWidgets('a Note being made from the quick note has no Add a part', (
    tester,
  ) async {
    await start(tester);
    await openTextCapture(tester);
    await tester.enterText(find.byType(TextField), 'descale the kettle');
    await tester.ensureVisible(key('captureToEditor'));
    await tester.pumpAndSettle();
    await tap(tester, key('captureToEditor'));
    expect(find.byType(NoteEditorScreen), findsOneWidget);
    await expectNoAmend(tester);
  });

  testWidgets('a Checklist being made by "+" has no Add a part', (
    tester,
  ) async {
    await start(tester);
    await tap(tester, key('browse'));
    await tap(tester, find.byTooltip('All Notes'));
    await tap(tester, find.byTooltip('Add here'));
    await tap(tester, key('addFan-checklist'));
    await tester.enterText(
      find.descendant(
        of: find.byType(AlertDialog),
        matching: find.byType(TextField),
      ),
      'Packing',
    );
    await tap(tester, find.text('Create'));
    expect(find.byType(NoteEditorScreen), findsOneWidget);
    await expectNoAmend(tester);
  });

  testWidgets('an existing Note shows no Amend box until Add a part opens '
      'it', (tester) async {
    final id = notes.capture('oil the hinge').id;
    await edit(tester, id);
    expect(find.text("What's new?"), findsNothing);
    expect(find.text('Add to Note'), findsNothing);

    await addAPart(tester);
    expect(find.byType(AlertDialog), findsOneWidget);
    expect(find.text("What's new?"), findsOneWidget);
    expect(find.text('Add to Note'), findsOneWidget);
    // Cancel adds nothing.
    await tap(tester, find.text('Cancel'));
    expect(notes.sections(id), isEmpty);

    await addPart(tester, 'hinge oiled');
    expect(notes.sections(id).single.text, 'hinge oiled');
    expect(find.text("What's new?"), findsNothing);
  });

  testWidgets('Undo and the find bar still work around a part added', (
    tester,
  ) async {
    final id = notes.capture('buy milk').id;
    await edit(tester, id);
    final field = find.descendant(
      of: key('note-text'),
      matching: find.byType(TextField),
    );
    await tester.enterText(field, 'buy oat milk');
    await tester.pump(const Duration(seconds: 2));

    await tap(tester, key('find-in-note'));
    await tester.enterText(key('find-field'), 'bread');
    await tester.pumpAndSettle();
    await addPart(tester, 'and bread');
    expect(notes.sections(id).single.text, 'and bread');
    // The find bar finds in the part just added.
    expect(tester.widget<Text>(key('find-count')).data, '1 of 1');

    await tap(tester, key('editor-undo'));
    // Back up from the groups at the page's end to the text (obf-1073).
    await tester.scrollUntilVisible(
      field,
      -100,
      scrollable: find
          .descendant(
            of: find.byType(ListView),
            matching: find.byType(Scrollable),
          )
          .first,
    );
    expect(tester.widget<TextField>(field).controller?.text, 'buy milk');
  });
}
