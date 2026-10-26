import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/conflicts_screen.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'reveal.dart';

/// A text in a format this version does not know, as a later version
/// would store it.
const _future = '{"format":"x-future","text":"# Shopping\\n**milk**"}';

const _newer =
    'This text was written with a newer version of Old Brain Fix. Update '
    'the app to change it.';

void main() {
  late Store store;
  late Notes notes;
  var now = DateTime.utc(2026, 10, 24, 8);

  setUp(() {
    now = DateTime.utc(2026, 10, 24, 8);
    store = Store.inMemory(
      clock: () => now = now.add(const Duration(minutes: 1)),
    )..starterSetOffered = true;
    notes = Notes(store);
  });
  tearDown(() => store.close());

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  Future<void> openEditor(WidgetTester tester, String cardText) async {
    await tester.pumpWidget(OldBrainFixApp(store: store));
    await tester.pumpAndSettle();
    await tap(tester, find.byTooltip('Search and browse'));
    await tap(tester, find.byTooltip('All Notes'));
    await tap(tester, find.text(cardText));
    expect(find.byType(NoteEditorScreen), findsOneWidget);
  }

  testWidgets('a text in an unknown format shows as its source, read-only; '
      'Amend and Merge leave it as it is', (tester) async {
    final target = notes.capture('Target');
    final note = notes.capture('x');
    store.append(note.id, NoteKeys.text, _future);
    await openEditor(tester, '# Shopping\n**milk**');

    expect(find.text(_newer), findsOneWidget);
    final field = tester.widget<TextField>(find.byType(TextField));
    expect(field.readOnly, isTrue);
    expect(field.controller?.text, '# Shopping\n**milk**');

    // Amend adds a Section and leaves the text as it is.
    await tester.scrollUntilVisible(
      find.text('Add to Note'),
      300,
      scrollable: find.byType(Scrollable).first,
    );
    await tap(tester, find.text('Add to Note'));
    await tester.enterText(find.byType(TextField).last, 'Oat milk');
    await tap(tester, find.text('Add'));
    expect(store.current(note.id, NoteKeys.text), _future);
    expect(notes.sections(note.id).single.text, 'Oat milk');

    // Merge points the Note at the one picked, its text as it is.
    await tap(tester, find.byTooltip('Merge into…'));
    await tap(tester, find.text('Target'));
    expect(notes.byId(note.id)?.mergedInto, target.id);
    expect(store.current(note.id, NoteKeys.text), _future);
  });

  testWidgets('a plain text shows no note and stays editable', (tester) async {
    notes.capture('Milk');
    await openEditor(tester, 'Milk');
    expect(find.text(_newer), findsNothing);
    expect(tester.widget<TextField>(find.byType(TextField)).readOnly, isFalse);
  });

  testWidgets('a text in a later structure shows as its source, read-only, '
      'and keeps its value', (tester) async {
    const later = '{"format":"plain","more":1,"text":"Later milk","v":2}';
    final note = notes.capture('x');
    store.append(note.id, NoteKeys.text, later);
    await openEditor(tester, 'Later milk');
    expect(find.text(_newer), findsOneWidget);
    final field = tester.widget<TextField>(find.byType(TextField));
    expect(field.readOnly, isTrue);
    expect(field.controller?.text, 'Later milk');
    await tester.pageBack();
    await tester.pumpAndSettle();
    expect(store.current(note.id, NoteKeys.text), later);
  });

  testWidgets('a later structure without text shows nothing to change', (
    tester,
  ) async {
    final note = notes.capture('Dentist');
    final section = notes.amend(note.id, 'Room 3');
    store.append(section.id, SectionKeys.text, '{"format":"plain","v":2}');
    await openEditor(tester, 'Dentist');
    await reveal(tester, find.byKey(ValueKey('newer:${section.id}')));
    expect(find.byKey(ValueKey('newer:${section.id}')), findsOneWidget);
    expect(find.text('Room 3'), findsNothing);
  });

  testWidgets('version 1 written out stays editable', (tester) async {
    final note = notes.capture('x');
    store.append(
      note.id,
      NoteKeys.text,
      '{"format":"plain","text":"Milk","v":1}',
    );
    await openEditor(tester, 'Milk');
    expect(find.text(_newer), findsNothing);
    expect(tester.widget<TextField>(find.byType(TextField)).readOnly, isFalse);
  });

  testWidgets('a Section in an unknown format says so', (tester) async {
    final note = notes.capture('Dentist');
    final section = notes.amend(note.id, 'Room 3');
    store.append(section.id, SectionKeys.text, _future);
    await openEditor(tester, 'Dentist');
    expect(
      tester.widget<TextField>(find.byType(TextField)).readOnly,
      isFalse,
      reason: "the Note's own text is plain",
    );
    await reveal(tester, find.byKey(ValueKey('newer:${section.id}')));
    expect(find.byKey(ValueKey('newer:${section.id}')), findsOneWidget);
    expect(find.text('# Shopping\n**milk**'), findsOneWidget);
  });

  testWidgets('a Conflict with a version in an unknown format cannot be '
      'picked here', (tester) async {
    final desk = Store.inMemory(
      clock: () => now = now.add(const Duration(minutes: 1)),
    );
    addTearDown(desk.close);
    final note = notes.capture('Milk');
    desk.applyEntries(store.entriesSince(const {}), keys: [store.ownKeyRecord]);
    notes.edit(note.id, 'Oat milk');
    desk.append(note.id, NoteKeys.text, _future);
    store.applyEntries(desk.entriesSince(const {}), keys: [desk.ownKeyRecord]);
    expect(store.conflicts(), hasLength(1));

    await tester.pumpWidget(
      MaterialApp(
        locale: const Locale('en'),
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: ConflictsScreen(store: store),
      ),
    );
    await tester.pumpAndSettle();
    expect(
      find.text(
        'A version was written with a newer version of Old Brain Fix. '
        'Update the app to pick one.',
      ),
      findsOneWidget,
    );
    expect(find.text('# Shopping\n**milk**'), findsOneWidget);
    expect(find.text('Oat milk'), findsOneWidget);
    await tester.tap(find.text('Oat milk'));
    await tester.pumpAndSettle();
    expect(store.conflicts(), hasLength(1));
    expect(store.current(note.id, NoteKeys.text), _future);
  });
}
