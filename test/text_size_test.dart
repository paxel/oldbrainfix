import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/agenda_group.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/look.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/settings_screen.dart';
import 'package:oldbrainfix/src/text_marks_view.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_photos.dart';
import 'fake_voice.dart';
import 'open_settings.dart';

/// The text's own size (#326, obf-1019): normal, larger and largest in
/// Settings ▸ Look, the text of Notes, Lines, the Agenda's rows and Ask's
/// answers grown on top of the system's font size and "Fat fingers",
/// icons, buttons and the labels around them kept, nothing overflowing
/// on a phone 360 wide in German.
void main() {
  late Store store;
  late Notes notes;
  // 09:00 on Saturday, 24 October 2026, on the test machine's clock.
  var now = DateTime(2026, 10, 24, 9);

  setUp(() {
    now = DateTime(2026, 10, 24, 9);
    store = Store.inMemory(clock: () => now)
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    notes = Notes(store);
  });
  tearDown(() => store.close());

  Future<void> start(WidgetTester tester) async {
    await tester.pumpWidget(
      OldBrainFixApp(
        store: store,
        voice: FakeVoice(),
        photos: FakePhotos(nextPhoto: onePixelPng),
      ),
    );
    await tester.pumpAndSettle();
  }

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  /// The system's font size, 1.5 times its usual.
  void bigSystemFont(WidgetTester tester) {
    tester.platformDispatcher.textScaleFactorTestValue = 1.5;
    addTearDown(tester.platformDispatcher.clearTextScaleFactorTestValue);
  }

  /// A phone 360 dp wide, in German.
  void germanPhone(WidgetTester tester) {
    tester.view.physicalSize = const Size(1080, 2340);
    tester.view.devicePixelRatio = 3;
    addTearDown(tester.view.reset);
    tester.platformDispatcher.localesTestValue = const [Locale('de')];
    addTearDown(tester.platformDispatcher.clearLocalesTestValue);
  }

  /// How large text of size 10 shows at [at].
  double textOf(WidgetTester tester, Finder at) =>
      MediaQuery.textScalerOf(tester.element(at)).scale(10);

  double iconOf(WidgetTester tester, Finder at) =>
      IconTheme.of(tester.element(at)).size ?? 24;

  void pills() => Routines(store).create(
    'pills',
    Schedule(
      start: Day(2026, 10, 1),
      times: [ClockTime(10, 0)],
      rule: Recurrence.parse('FREQ=DAILY'),
    ),
  );

  Finder chip(TextSize size) => find.byKey(ValueKey('textSize-${size.name}'));

  test('the steps are 1, 1.15 and 1.3 times', () {
    expect([for (final s in TextSize.values) s.factor], [1.0, 1.15, 1.3]);
  });

  testWidgets("each step in Settings ▸ Look grows the Agenda's rows at once "
      "on top of the system's font size, keeps icons, boxes and the bar, "
      'and is kept on this Device', (tester) async {
    bigSystemFont(tester);
    pills();
    await start(tester);
    final row = find
        .descendant(of: find.byType(AgendaGroup), matching: find.text('pills'))
        .first;
    expect(store.textSize, TextSize.normal);
    expect(textOf(tester, row), 15);

    for (final size in [TextSize.larger, TextSize.largest, TextSize.normal]) {
      await openDeviceSettings(tester);
      await tap(tester, chip(size));
      expect(store.textSize, size);
      expect(
        tester.widget<ChoiceChip>(chip(size)).selected,
        isTrue,
        reason: '$size',
      );
      // The Settings' own words and icons are no content: they keep
      // their size.
      expect(textOf(tester, chip(size)), 15, reason: '$size');
      expect(iconOf(tester, find.byType(SettingsScreen)), 24);
      await tap(tester, find.byType(BackButton));
      await tap(tester, find.byType(BackButton));
      expect(
        textOf(tester, row),
        closeTo(10 * 1.5 * size.factor, 0.001),
        reason: '$size',
      );
      expect(textOf(tester, find.byType(AppBar).first), 15, reason: '$size');
      expect(iconOf(tester, find.byType(AgendaScreen)), 24);
      expect(tester.getSize(find.byType(AgendaBox).first), const Size(40, 40));
    }
  });

  testWidgets('Fat fingers and the system multiply the step, each once', (
    tester,
  ) async {
    bigSystemFont(tester);
    pills();
    store
      ..fatFingers = true
      ..textSize = TextSize.largest;
    await start(tester);
    final row = find
        .descendant(of: find.byType(AgendaGroup), matching: find.text('pills'))
        .first;
    expect(textOf(tester, row), closeTo(10 * 1.5 * 1.3 * 1.3, 0.001));
    // The bar keeps Fat fingers' size alone.
    expect(
      textOf(tester, find.byType(AppBar).first),
      closeTo(10 * 1.5 * 1.3, 0.001),
    );
    expect(iconOf(tester, find.byType(AgendaScreen)), closeTo(24 * 1.3, 0.001));
  });

  testWidgets("a Note's text grows in its card and in the editor, kept "
      'across a new start', (tester) async {
    notes.capture('Soße kaufen');
    final trip = notes.capture('Vor der Reise\nReisepass\nTickets').id;
    Checklists(store).make(trip);
    store.textSize = TextSize.larger;
    await start(tester);
    await tester.tap(find.byTooltip('Search and browse'));
    await tester.pumpAndSettle();
    await tester.tap(find.byTooltip('All Notes'));
    await tester.pumpAndSettle();
    final card = find.text('Soße kaufen');
    expect(textOf(tester, card), closeTo(10 * 1.15, 0.001));
    // A Checklist's Lines on its card grow with it.
    expect(textOf(tester, find.text('Reisepass').first), closeTo(11.5, 0.001));
    await tester.longPress(card);
    await tester.pumpAndSettle();
    expect(find.byType(NoteEditorScreen), findsOneWidget);
    final field = find.descendant(
      of: find.byType(NoteEditorScreen),
      matching: find.byType(EditableText),
    );
    expect(textOf(tester, field.first), closeTo(10 * 1.15, 0.001));
    // The editor's bar keeps its size.
    expect(textOf(tester, find.byType(AppBar).last), 10);
  });

  /// A formatted text in the app's theme at [size].
  Widget formatted(TextSize size, String text) => MaterialApp(
    theme: lookTheme(Brightness.light, textSize: size),
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    home: Scaffold(
      body: Align(
        alignment: Alignment.topLeft,
        child: MarkedText(text, format: TextFormats.markdown),
      ),
    ),
  );

  testWidgets("a formatted text's room grows with the step: it keeps its "
      'look (#327, obf-1020)', (tester) async {
    Future<(double, double, double)> measure(TextSize size) async {
      await tester.pumpWidget(formatted(size, 'one\ntwo\n\n- three'));
      await tester.pumpAndSettle();
      final one = tester.getTopLeft(find.text('one')).dy;
      final two = tester.getTopLeft(find.text('two')).dy;
      final three = tester.getTopLeft(find.text('three'));
      return (two - one, three.dy - two, three.dx);
    }

    final (lines, blank, indent) = await measure(TextSize.normal);
    final (grownLines, grownBlank, grownIndent) = await measure(
      TextSize.largest,
    );
    expect(grownLines, closeTo(lines * 1.3, 0.5));
    expect(grownBlank, closeTo(blank * 1.3, 0.5));
    expect(grownIndent, closeTo(indent * 1.3, 0.5));
  });

  testWidgets('a text inside a grown one grows once', (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        theme: lookTheme(Brightness.light, textSize: TextSize.largest),
        home: const ContentText(child: ContentText(child: Text('inner'))),
      ),
    );
    expect(textOf(tester, find.text('inner')), closeTo(13, 0.001));
  });

  group('at largest, with Fat fingers and a big system font, a phone 360 '
      'wide in German fits', () {
    setUp(
      () => store
        ..fatFingers = true
        ..textSize = TextSize.largest,
    );

    testWidgets("the Agenda's rows of every kind", (tester) async {
      germanPhone(tester);
      bigSystemFont(tester);
      final routines = Routines(store);
      Schedule daily(List<ClockTime> times) => Schedule(
        start: Day(2026, 10, 1),
        times: times,
        rule: Recurrence.parse('FREQ=DAILY'),
      );
      routines.create('Tabletten nehmen', daily([ClockTime(10, 0)]));
      routines.create('Aufräumen', daily([]));
      final trip = notes.capture('Vor der Reise\nReisepass\nTickets').id;
      Checklists(store).make(trip);
      final shopping = notes.capture('Einkauf\nMilch\nEier').id;
      ShoppingLists(store).make(shopping);
      Frogs(store).spawn(notes.capture('Steuererklärung').id);
      await start(tester);
      expect(find.byType(AgendaGroup), findsWidgets);
      expect(tester.takeException(), isNull);
      await tester.drag(find.byType(Scrollable).first, const Offset(0, -3000));
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
    });

    testWidgets('the Note editor with a formatted text', (tester) async {
      germanPhone(tester);
      bigSystemFont(tester);
      final note = notes.capture(
        '# Soße kaufen\n\nFür das **Abendessen** am Samstag\n\n'
        '- Tomaten\n  - reife\n\n```\ncode\n```',
      );
      notes.setFormat(note.id, TextFormats.markdown);
      await start(tester);
      await tester.tap(find.byTooltip('Suchen und stöbern'));
      await tester.pumpAndSettle();
      await tester.tap(find.byTooltip('Alle Notizen'));
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      await tester.longPress(find.text('Soße kaufen').first);
      await tester.pumpAndSettle();
      expect(find.byType(NoteEditorScreen), findsOneWidget);
      expect(tester.takeException(), isNull);
    });

    testWidgets('the Settings, all the way down', (tester) async {
      germanPhone(tester);
      bigSystemFont(tester);
      await start(tester);
      await openDeviceSettings(tester);
      expect(tester.widget<ChoiceChip>(chip(TextSize.largest)).selected, true);
      await tester.drag(find.byType(Scrollable).first, const Offset(0, -6000));
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
    });
  });
}
