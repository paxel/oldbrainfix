import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/app_lock.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/note_history_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// A Note's History on the phone (#193): opened from the Note editor's
/// bar, every change newest first with who, when and what, and "Use
/// this version", which brings an old text back as a new change.
void main() {
  late Store store;
  late Places places;
  late Notes notes;
  var now = DateTime.utc(2026, 10, 24, 8);

  setUp(() {
    now = DateTime.utc(2026, 10, 24, 8);
    store =
        Store.inMemory(clock: () => now = now.add(const Duration(minutes: 1)))
          ..starterSetOffered = true
          ..applyProfileDefaults(name: 'Phone')
          ..renameOwnProfile('Ann');
    places = Places(store);
    notes = Notes(store);
  });
  tearDown(() => store.close());

  /// A screen tall enough to build every change of the list.
  void tall(WidgetTester tester) {
    tester.view.physicalSize = const Size(1080, 6000);
    tester.view.devicePixelRatio = 2.5;
    addTearDown(tester.view.reset);
  }

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  /// Opens the editor of the card showing [text] from every Note's list.
  Future<void> openEditor(WidgetTester tester, String text) async {
    await tester.pumpWidget(OldBrainFixApp(store: store));
    await tester.pumpAndSettle();
    await tap(tester, find.byTooltip('Search and browse'));
    await tap(tester, find.byTooltip('All Notes'));
    await tester.longPress(find.text(text));
    await tester.pumpAndSettle();
    expect(find.byType(NoteEditorScreen), findsOneWidget);
  }

  String row(WidgetTester tester, int i) => [
    for (final text in tester.widgetList<Text>(
      find.descendant(
        of: find.byKey(ValueKey('note-change-$i')),
        matching: find.byType(Text),
      ),
    ))
      text.data ?? '',
  ].join(' | ');

  testWidgets('History lists every change, newest first, with who, when '
      'and what, and Use this version brings an old text back', (tester) async {
    tall(tester);
    final note = notes.capture('Dentist on Tuesday');
    notes.edit(note.id, 'Dentist on Wednesday');
    final amend = notes.amend(note.id, 'Bring the X-ray');
    notes.editSection(amend.id, 'Bring nothing');
    notes.setImportance(note.id, Importance.important);
    await openEditor(tester, 'Dentist on Wednesday');

    await tap(tester, find.byKey(const ValueKey('note-history')));
    expect(find.byType(NoteHistoryScreen), findsOneWidget);
    expect(find.text('History'), findsOneWidget);
    expect(row(tester, 0), startsWith('Importance | Ann, '));
    expect(row(tester, 0), endsWith('Before | Normal | After | Important'));
    expect(
      row(tester, 1),
      endsWith('Before | Bring the X-ray | After | Bring nothing'),
    );
    expect(row(tester, 1), startsWith('Amend changed | Ann, '));
    expect(row(tester, 2), startsWith('Amended | Ann, '));
    expect(row(tester, 2), endsWith('Bring the X-ray | Use this version'));
    expect(row(tester, 3), startsWith('Text changed'));
    expect(
      row(tester, 3),
      endsWith('Before | Dentist on Tuesday | After | Dentist on Wednesday'),
    );
    expect(row(tester, 4), startsWith('Captured'));
    expect(row(tester, 4), endsWith('Dentist on Tuesday | Use this version'));
    // The text showing now is no version to go back to.
    expect(find.byKey(const ValueKey('use-version-1')), findsNothing);
    expect(find.byKey(const ValueKey('use-version-3')), findsNothing);

    final written = store.currentSeq();
    await tester.ensureVisible(find.byKey(const ValueKey('use-version-4')));
    await tap(tester, find.byKey(const ValueKey('use-version-4')));
    expect(
      find.text('The old text is back. The History keeps every version.'),
      findsOneWidget,
    );
    expect(store.entriesAfter(written), hasLength(1));
    expect(notes.byId(note.id)!.text, 'Dentist on Tuesday');
    expect(row(tester, 0), startsWith('Text changed'));
    expect(
      row(tester, 0),
      endsWith('Before | Dentist on Wednesday | After | Dentist on Tuesday'),
    );
    expect(find.byKey(const ValueKey('note-change-5')), findsOneWidget);
    expect(find.byKey(const ValueKey('note-change-6')), findsNothing);

    await tester.pageBack();
    await tester.pumpAndSettle();
    expect(find.byType(NoteEditorScreen), findsOneWidget);
    expect(
      tester
          .widget<TextField>(
            find.descendant(
              of: find.byKey(const ValueKey('note-text')),
              matching: find.byType(TextField),
            ),
          )
          .controller!
          .text,
      'Dentist on Tuesday',
    );
  });

  testWidgets('Lines, the kind and filing; a locked Area that is not open '
      'shows no name', (tester) async {
    tall(tester);
    final doctor = places.addArea('Doctor');
    final home = places.addArea('Household');
    final note = notes.capture('Trip');
    notes.file(note.id, doctor.id);
    notes.file(note.id, home.id);
    final lists = Checklists(store);
    lists.make(note.id);
    final tent = lists.add(note.id, 'Tent');
    lists.check(tent);
    lists.delete(tent);
    store
      ..setPin('2468', iterations: 10)
      ..setAreaLocked(doctor.id, true);
    final lock = AppLock(store: store);
    await tester.pumpWidget(
      MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: LockScope(
          lock: lock,
          child: NoteHistoryScreen(
            notes: notes,
            places: places,
            noteId: note.id,
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(row(tester, 0), startsWith('Line deleted'));
    expect(row(tester, 0), endsWith('Tent'));
    expect(row(tester, 1), startsWith('Line checked'));
    expect(row(tester, 2), startsWith('Line added'));
    expect(row(tester, 3), startsWith('Kind'));
    expect(row(tester, 3), endsWith('Before | Plain Note | After | Checklist'));
    expect(
      row(tester, 4),
      endsWith('Before | A locked Area | After | Household'),
    );
    expect(
      row(tester, 5),
      endsWith('Before | Unsorted | After | A locked Area'),
    );
    expect(find.text('Doctor'), findsNothing);

    lock.openArea(doctor.id);
    await tester.pumpWidget(
      MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: LockScope(
          lock: lock,
          child: NoteHistoryScreen(
            key: const ValueKey('again'),
            notes: notes,
            places: places,
            noteId: note.id,
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(row(tester, 5), endsWith('Before | Unsorted | After | Doctor'));
  });
}
