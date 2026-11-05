import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:intl/date_symbol_data_local.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/app_links.dart';
import 'package:oldbrainfix/src/backup_screen.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/settings_screen.dart';
import 'package:oldbrainfix/src/text_marks_view.dart';
import 'package:oldbrainfix/src/two_row_text_field.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'folder_test.dart' show FakeFolders;

/// App links (#261, ADR 0046): a Markdown link with the app's scheme
/// opens its Note, Area or Topic, the Agenda, Unsorted or one of the
/// app's pages; one whose target is not here says so; a file link opens
/// only a file of the shared folder, after one question; "Link to…" in
/// the format bar puts one in from a picker.
void main() {
  late Store store;
  late Notes notes;

  setUpAll(initializeDateFormatting);

  setUp(() {
    store = Store.inMemory(clock: () => DateTime.utc(2026, 11, 5, 8))
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    notes = Notes(store);
  });
  tearDown(() => store.close());

  Finder key(String value) => find.byKey(ValueKey(value));

  /// The app on All Notes, a tall screen so every card shows.
  Future<void> pumpAllNotes(
    WidgetTester tester, {
    FakeFolders? folders,
    List<Uri>? opened,
  }) async {
    tester.view.physicalSize = const Size(800, 2400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      LinkOpenScope(
        onOpen: (address) async => opened?.add(address),
        child: OldBrainFixApp(store: store, folders: folders ?? FakeFolders()),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.byTooltip('Search and browse'));
    await tester.pumpAndSettle();
    await tester.tap(find.byTooltip('All Notes'));
    await tester.pumpAndSettle();
  }

  Future<void> tapLink(WidgetTester tester, String label) async {
    await tester.tapOnText(find.textRange.ofSubstring(label));
    await tester.pumpAndSettle();
  }

  testWidgets('a link opens its Note and a page; a Note not here says so', (
    tester,
  ) async {
    final dentist = notes.capture('Dentist on Monday');
    final gone = notes.capture('Old one');
    notes.trash(gone.id);
    notes.capture(
      'Links\n[the appointment](${AppLink(AppLinkKind.note, dentist.id).url})'
      '\n[the old one](${AppLink(AppLinkKind.note, gone.id).url})'
      '\n[my backups](oldbrainfix://settings/backups)'
      '\n[nowhere](oldbrainfix://nowhere)',
      format: TextFormats.markdown,
    );
    await pumpAllNotes(tester);

    await tapLink(tester, 'the appointment');
    expect(find.byType(NoteEditorScreen), findsOneWidget);
    expect(
      tester.widget<NoteEditorScreen>(find.byType(NoteEditorScreen)).noteId,
      dentist.id,
    );
    await tester.pageBack();
    await tester.pumpAndSettle();

    await tapLink(tester, 'the old one');
    expect(find.text('Not available here'), findsOneWidget);
    expect(find.byType(NoteEditorScreen), findsNothing);

    await tapLink(tester, 'my backups');
    expect(find.byType(BackupScreen), findsOneWidget);
    await tester.pageBack();
    await tester.pumpAndSettle();

    // An address of the scheme it does not know is no link at all.
    await tapLink(tester, 'nowhere');
    expect(find.byType(BackupScreen), findsNothing);
  });

  testWidgets('a link to a Settings section opens the Settings there, and '
      'one to the guide opens the website in the language', (tester) async {
    final opened = <Uri>[];
    notes.capture(
      '[history settings](oldbrainfix://settings/history)\n'
      '[the guide](oldbrainfix://guide/getting-started#set-up)',
      format: TextFormats.markdown,
    );
    await pumpAllNotes(tester, opened: opened);
    tester.view.physicalSize = const Size(800, 900);
    await tester.pumpAndSettle();
    await tapLink(tester, 'the guide');
    expect(opened, [
      Uri.parse(
        'https://paxel.github.io/oldbrainfix/en/guide/getting-started.html#set-up',
      ),
    ]);

    await tapLink(tester, 'history settings');
    final settings = tester.widget<SettingsScreen>(find.byType(SettingsScreen));
    expect(settings.section, 'history');
    final heading = find.widgetWithText(SettingsHeading, 'History');
    expect(heading, findsOneWidget);
    // Scrolled to: on the screen, below the bar.
    expect(tester.getTopLeft(heading).dy, lessThan(900));
    expect(tester.getTopLeft(heading).dy, greaterThan(0));
  });

  testWidgets('a link to the Agenda goes back to it from any page', (
    tester,
  ) async {
    notes.capture(
      'Back\n[to the agenda](oldbrainfix://agenda)',
      format: TextFormats.markdown,
    );
    await pumpAllNotes(tester);
    await tapLink(tester, 'to the agenda');
    expect(find.byTooltip('All Notes'), findsNothing);
    expect(find.byTooltip('Search and browse'), findsOneWidget);
  });

  testWidgets('a file link opens a file of the shared folder after one '
      'question naming it and its app; any other says so', (tester) async {
    final folders = FakeFolders()
      ..fileApps['Scans/tax 2026.pdf'] = 'PDF Viewer';
    store.chooseSyncFolder('content://tree/primary%3ASync');
    notes.capture(
      '[the tax](oldbrainfix://file/Scans/tax%202026.pdf)\n'
      '[a missing one](oldbrainfix://file/nope.pdf)',
      format: TextFormats.markdown,
    );
    await pumpAllNotes(tester, folders: folders);

    await tapLink(tester, 'the tax');
    expect(
      find.text(
        'Open "Scans/tax 2026.pdf" from the shared folder with PDF Viewer?',
      ),
      findsOneWidget,
    );
    await tester.tap(find.text('Cancel'));
    await tester.pumpAndSettle();
    expect(folders.opened, isEmpty);

    await tapLink(tester, 'the tax');
    await tester.tap(key('fileLinkOpen'));
    await tester.pumpAndSettle();
    expect(folders.opened, ['Scans/tax 2026.pdf']);

    await tapLink(tester, 'a missing one');
    expect(find.text('Not available here'), findsOneWidget);
    expect(folders.opened, hasLength(1));
  });

  testWidgets('without a shared folder a file link says not available', (
    tester,
  ) async {
    final folders = FakeFolders()..fileApps['a.pdf'] = 'PDF Viewer';
    notes.capture(
      '[the file](oldbrainfix://file/a.pdf)',
      format: TextFormats.markdown,
    );
    await pumpAllNotes(tester, folders: folders);
    await tapLink(tester, 'the file');
    expect(find.text('Not available here'), findsOneWidget);
    expect(find.byType(AlertDialog), findsNothing);
  });

  testWidgets('the picker offers the pages, Areas and Notes and finds them', (
    tester,
  ) async {
    final places = Places(store);
    final area = places.addArea('Household');
    places.addTopic(area.id, 'Kitchen');
    final note = notes.capture('Dentist on Monday');
    LinkTarget? picked;
    await tester.pumpWidget(
      MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Builder(
          builder: (context) => TextButton(
            onPressed: () async =>
                picked = await pickLinkTarget(context, notes, places),
            child: const Text('pick'),
          ),
        ),
      ),
    );
    await tester.tap(find.text('pick'));
    await tester.pumpAndSettle();
    expect(find.text('Pages'), findsOneWidget);
    await tester.enterText(key('linkToSearch'), 'kitchen');
    await tester.pumpAndSettle();
    expect(find.text('Areas and Topics'), findsOneWidget);
    expect(find.text('Household ▸ Kitchen'), findsOneWidget);
    expect(find.text('Pages'), findsNothing);
    await tester.enterText(key('linkToSearch'), 'dentist');
    await tester.pumpAndSettle();
    expect(find.text('Household'), findsNothing);
    await tester.tap(key('linkTo-${AppLink(AppLinkKind.note, note.id).url}'));
    await tester.pumpAndSettle();
    expect(picked?.link, AppLink(AppLinkKind.note, note.id));
    expect(picked?.label, 'Dentist on Monday');

    await tester.tap(find.text('pick'));
    await tester.pumpAndSettle();
    await tester.enterText(key('linkToSearch'), 'zzz');
    await tester.pumpAndSettle();
    expect(find.text('Nothing found.'), findsOneWidget);
  });

  testWidgets('Link to… puts the picked link in with its name at the '
      'cursor, and on the word there', (tester) async {
    tester.view.physicalSize = const Size(3000, 2000);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    final controller = TagTextController(text: 'see ', hideMarks: true);
    addTearDown(controller.dispose);
    await tester.pumpWidget(
      MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(
          body: AppLinkScope(
            onOpen: (_, _) async {},
            onPick: (_) async =>
                (link: const AppLink(AppLinkKind.backups), label: 'Backups'),
            child: ListView(
              children: [
                TwoRowTextField(
                  controller: controller,
                  twoRows: true,
                  boxes: true,
                  formats: true,
                ),
              ],
            ),
          ),
        ),
      ),
    );
    await tester.showKeyboard(find.byType(TextField));
    tester.view.viewInsets = const FakeViewPadding(bottom: 900);
    addTearDown(tester.view.resetViewInsets);
    await tester.pumpAndSettle();
    controller.selection = const TextSelection.collapsed(offset: 4);
    await tester.pump();
    await tester.tap(key('format-link-to'));
    await tester.pumpAndSettle();
    expect(controller.text, 'see [Backups](oldbrainfix://settings/backups)');
    expect(controller.markdown, isTrue);
  });
}
