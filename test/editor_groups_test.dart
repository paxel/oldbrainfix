import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'editor_actions.dart';
import 'fake_files.dart';
import 'fake_photos.dart';
import 'fake_voice.dart';

/// The Note editor's actions in the four groups of the desktop's Note
/// dialog (obf-1073): This Note, its badges under the text; Add, Share
/// and Tidy at the page's end; the bar keeps its editing actions.
void main() {
  late Store store;
  late Notes notes;
  late Directory dir;

  setUp(() {
    store = Store.inMemory(clock: () => DateTime.utc(2026, 11, 24, 8))
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    notes = Notes(store);
    dir = Directory.systemTemp.createTempSync('editor_groups_');
  });
  tearDown(() {
    store.close();
    dir.deleteSync(recursive: true);
  });

  Finder key(String name) => find.byKey(ValueKey(name));

  /// Opens the editor of the Note [id] on a phone 360 wide, where the
  /// Device has a camera, a microphone, a gallery and files.
  Future<void> edit(WidgetTester tester, String id) async {
    tester.view
      ..physicalSize = const Size(1080, 2340)
      ..devicePixelRatio = 3;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      OldBrainFixApp(
        store: store,
        photos: FakePhotos(),
        voice: FakeVoice(),
        files: FakeFiles(dir),
      ),
    );
    await tester.pumpAndSettle();
    final agenda = tester.element(find.byType(AgendaScreen));
    unawaited(openNote(agenda, notes, Places(store), notes.byId(id)!));
    await tester.pumpAndSettle();
    expect(find.byType(NoteEditorScreen), findsOneWidget);
  }

  /// The keys of the buttons and badges in the group [group], in order.
  List<String> members(WidgetTester tester, String group) => [
    for (final element
        in find
            .descendant(
              of: find.descendant(of: key(group), matching: find.byType(Wrap)),
              matching: find.byWidgetPredicate(
                (w) =>
                    w.key is ValueKey<String> &&
                    (w.key! as ValueKey<String>).value.contains('-'),
              ),
            )
            .evaluate())
      (element.widget.key! as ValueKey<String>).value,
  ];

  testWidgets('four named groups in order, each with its own, and the bar '
      'keeps Undo, Redo, Find, History and Revert', (tester) async {
    final id = notes.capture('oil the hinge').id;
    await edit(tester, id);

    expect(
      find.descendant(
        of: key('editor-group-this-note'),
        matching: find.text('This Note'),
      ),
      findsOneWidget,
    );
    expect(
      find.descendant(
        of: key('editor-group-this-note'),
        matching: key('note-badges'),
      ),
      findsOneWidget,
    );
    final bar = find.byType(AppBar);
    for (final action in [
      'editor-undo',
      'editor-redo',
      'find-in-note',
      'note-history',
    ]) {
      expect(
        find.descendant(of: bar, matching: key(action)),
        findsOneWidget,
        reason: action,
      );
    }

    await revealInEditor(tester, key('editor-group-tidy'));
    final groups = [
      ('editor-group-this-note', 'This Note'),
      ('editor-group-add', 'Add'),
      ('editor-group-share', 'Share'),
      ('editor-group-tidy', 'Tidy'),
    ];
    for (final (group, heading) in groups.skip(1)) {
      expect(
        find.descendant(of: key(group), matching: find.text(heading)),
        findsOneWidget,
        reason: group,
      );
    }
    // At the page's end, one after another.
    for (final (i, (group, _)) in groups.indexed.skip(2)) {
      expect(
        tester.getTopLeft(key(group)).dy,
        greaterThanOrEqualTo(tester.getBottomLeft(key(groups[i - 1].$1)).dy),
        reason: group,
      );
    }
    expect(members(tester, 'editor-group-add'), [
      'badge-add',
      'editor-voice',
      'editor-photo',
      'editor-gallery',
      'editor-paste',
      'attach-file',
      'editor-add-part',
    ]);
    expect(members(tester, 'editor-group-share'), [
      'send-note-file',
      'share-note',
      'pack-note',
    ]);
    expect(members(tester, 'editor-group-tidy'), [
      'editor-merge',
      'editor-trash',
    ]);
    // None of them on the bar or in its ⋮ any more.
    for (final action in ['editor-photo', 'editor-merge', 'editor-trash']) {
      expect(
        find.descendant(of: bar, matching: key(action)),
        findsNothing,
        reason: action,
      );
    }
    if (key('bar-more').evaluate().isNotEmpty) {
      await tester.tap(key('bar-more'));
      await tester.pumpAndSettle();
      for (final words in ['Photo', 'Merge into…', 'Move to trash']) {
        expect(
          find.descendant(
            of: find.byType(PopupMenuItem<VoidCallback>),
            matching: find.text(words),
          ),
          findsNothing,
          reason: words,
        );
      }
    }
    expect(tester.takeException(), isNull);
  });

  testWidgets('Move to trash is filled on the error colour, the others '
      'outlined (obf-1099)', (tester) async {
    final id = notes.capture('oil the hinge').id;
    await edit(tester, id);
    await revealInEditor(tester, key('editor-trash'));
    final scheme = Theme.of(tester.element(key('editor-trash'))).colorScheme;
    Material ground(String name) => tester.widget<Material>(
      find.descendant(of: key(name), matching: find.byType(Material)).first,
    );
    expect(ground('editor-trash').color, scheme.error);
    expect(
      find.descendant(
        of: key('editor-trash'),
        matching: find.byWidgetPredicate((w) => w is FilledButton),
      ),
      findsOneWidget,
    );
    expect(tester.widget(key('editor-merge')), isA<OutlinedButton>());
    // The Add group's "+" has the outlined button's shape too (obf-1117).
    expect(tester.widget(key('badge-add')), isA<OutlinedButton>());
    expect(
      find.descendant(of: key('badge-add'), matching: find.byIcon(Icons.add)),
      findsOneWidget,
    );
    expect(ground('editor-merge').color, isNot(scheme.error));
    expect(find.byIcon(Icons.delete_outline), findsOneWidget);
  });

  testWidgets('a screen reader reads each heading on its own', (tester) async {
    final semantics = tester.ensureSemantics();
    final id = notes.capture('oil the hinge').id;
    await edit(tester, id);
    await revealInEditor(tester, key('badge-add'));
    expect(
      tester.getSemantics(key('badge-add')),
      // A button as its group's others, which takes the focus too.
      containsSemantics(
        label: 'Routine, Reminder, Frog or Session',
        isButton: true,
        hasTapAction: true,
      ),
    );
    expect(find.bySemanticsLabel('Add'), findsOneWidget);
    semantics.dispose();
  });
}
