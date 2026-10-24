import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/drafts.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

void main() {
  late Store store;
  late Notes notes;
  late MemoryDrafts drafts;
  var now = DateTime.utc(2026, 10, 24, 8);

  setUp(() {
    now = DateTime.utc(2026, 10, 24, 8);
    store = Store.inMemory(clock: () => now)..starterSetOffered = true;
    notes = Notes(store);
    drafts = MemoryDrafts();
  });
  tearDown(() => store.close());

  void tick([Duration by = const Duration(minutes: 1)]) => now = now.add(by);

  Future<void> start(WidgetTester tester) async {
    await tester.pumpWidget(OldBrainFixApp(store: store, drafts: drafts));
    await tester.pumpAndSettle();
  }

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  String field(WidgetTester tester) =>
      tester.widget<TextField>(find.byType(TextField)).controller?.text ?? '';

  group('editor drafts', () {
    testWidgets('typing keeps a draft after a pause; leaving drops it', (
      tester,
    ) async {
      final note = notes.capture('milk');
      await start(tester);
      await tap(tester, find.text('milk'));
      tick();
      await tester.enterText(find.byType(TextField), 'oat milk');
      await tester.pump(const Duration(milliseconds: 500));
      expect(await drafts.read(note.id), isNull);
      await tester.pump(NoteEditorScreen.draftPause);
      expect(await drafts.read(note.id), (
        text: 'oat milk',
        saved: DateTime.utc(2026, 10, 24, 8, 1),
      ));
      // The log is not touched while typing.
      expect(notes.byId(note.id)?.text, 'milk');

      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(notes.byId(note.id)?.text, 'oat milk');
      expect(await drafts.read(note.id), isNull);
    });

    testWidgets('a draft left by a killed app comes back and is saved', (
      tester,
    ) async {
      final note = notes.capture('milk');
      await drafts.write(note.id, (
        text: 'oat milk',
        saved: DateTime.utc(2026, 10, 24, 8, 5),
      ));
      tick(const Duration(hours: 1));
      await start(tester);
      await tap(tester, find.text('milk'));
      expect(field(tester), 'oat milk');
      expect(notes.byId(note.id)?.text, 'milk');

      await tester.pageBack();
      await tester.pumpAndSettle();
      expect(notes.byId(note.id)?.text, 'oat milk');
      expect(await drafts.read(note.id), isNull);
    });

    testWidgets('a draft older than the Note\'s text is dropped', (
      tester,
    ) async {
      final note = notes.capture('milk');
      await drafts.write(note.id, (
        text: 'oat milk',
        saved: DateTime.utc(2026, 10, 24, 8, 5),
      ));
      tick(const Duration(hours: 1));
      // Fixed later, e.g. on another Device.
      notes.edit(note.id, 'soy milk');
      await start(tester);
      await tap(tester, find.text('soy milk'));
      expect(field(tester), 'soy milk');
      expect(await drafts.read(note.id), isNull);
    });

    testWidgets('going to the background saves the text to the log', (
      tester,
    ) async {
      final note = notes.capture('milk');
      await start(tester);
      await tap(tester, find.text('milk'));
      tick();
      await tester.enterText(find.byType(TextField), 'oat milk');
      await tester.pump(NoteEditorScreen.draftPause);
      for (final state in [
        AppLifecycleState.inactive,
        AppLifecycleState.hidden,
        AppLifecycleState.paused,
      ]) {
        tester.binding.handleAppLifecycleStateChanged(state);
      }
      await tester.pump();
      expect(notes.byId(note.id)?.text, 'oat milk');
      expect(await drafts.read(note.id), isNull);
      expect(find.byType(NoteEditorScreen), findsOneWidget);
      expect(field(tester), 'oat milk');
    });
  });

  group('file drafts', () {
    late Directory dir;

    setUp(() => dir = Directory.systemTemp.createTempSync('obf-drafts'));
    tearDown(() => dir.deleteSync(recursive: true));

    test('a draft is written, read back and deleted', () async {
      final drafts = FileDrafts(Directory('${dir.path}/drafts'));
      expect(await drafts.read('note:a'), isNull);
      final draft = (text: 'oat milk', saved: DateTime.utc(2026, 10, 24, 8));
      await drafts.write('note:a', draft);
      expect(
        await FileDrafts(Directory('${dir.path}/drafts')).read('note:a'),
        draft,
      );
      expect(await drafts.read('note:b'), isNull);
      await drafts.delete('note:a');
      expect(await drafts.read('note:a'), isNull);
    });

    test('a delete right after a write wins', () async {
      final drafts = FileDrafts(dir);
      final writing = drafts.write('note:a', (
        text: 'x',
        saved: DateTime.utc(2026, 10, 24, 8),
      ));
      await drafts.delete('note:a');
      await writing;
      expect(await drafts.read('note:a'), isNull);
    });

    test('a broken draft file is no draft', () async {
      File('${dir.path}/note_a.json').writeAsStringSync('{"text": "x"');
      expect(await FileDrafts(dir).read('note:a'), isNull);
    });
  });
}
