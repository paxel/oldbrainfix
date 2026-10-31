import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/look.dart';
import 'package:oldbrainfix/src/page_bar.dart';

/// The narrowest phone width Android supports.
const _narrowest = 320.0;

/// A phone's width and a wide screen's.
const _phone = 432.0;
const _wide = 1000.0;

/// Roboto, the font a phone draws the titles in: the test font's square
/// glyphs are far wider and would measure nothing a phone shows.
Future<void> _loadRoboto() async {
  final runner = Platform.resolvedExecutable;
  final cut = runner.indexOf(
    '${Platform.pathSeparator}bin${Platform.pathSeparator}cache'
    '${Platform.pathSeparator}',
  );
  final root = cut < 0
      ? Platform.environment['FLUTTER_ROOT']!
      : runner.substring(0, cut);
  final dir = Directory(
    [root, 'bin', 'cache', 'artifacts', 'material_fonts'].join('/'),
  );
  final roboto = FontLoader('Roboto');
  for (final file in dir.listSync().whereType<File>()) {
    final name = file.uri.pathSegments.last;
    if (name.startsWith('Roboto-') && name.endsWith('.ttf')) {
      roboto.addFont(file.readAsBytes().then((b) => ByteData.sublistView(b)));
    }
  }
  await roboto.load();
}

/// Every fixed title a phone page shows. Titles made of the user's own
/// words (an Area's, a Topic's or a person's name) end in "…" when
/// longer than the line, as any long text there does.
List<String> _titles(AppLocalizations t) => [
  t.aboutTitle,
  t.addToShoppingList,
  t.agendaCalendarsTitle,
  t.allNotesTitle,
  t.allRoutinesTitle,
  t.appTitle,
  t.areaLocksTitle,
  t.areaOfferTitle,
  t.areaOffersWaitingTitle,
  t.areasTitle,
  t.backupsTitle,
  t.calendarPageTitle,
  t.captureContextTitle,
  t.conflictsTitle,
  t.fileInto,
  t.firstStartDevice,
  t.firstStartTitle,
  t.frogsTitle,
  t.historyTitle,
  t.hostTitle,
  t.mergeInto,
  t.namedPlacesTitle,
  t.newReminder,
  t.newRoutine,
  t.newSession,
  t.noteTitle,
  t.nudgeScreenTitle,
  t.peopleTitle,
  t.personAdd,
  t.photo,
  t.profileCropTitle,
  t.profileQrTitle,
  t.profileTitle,
  t.reminder,
  t.restoreFoundTitle,
  t.restoreOffer,
  t.routine,
  t.sentItemTitle,
  t.sentItemsWaitingTitle,
  t.session,
  t.settingsTitle,
  t.setupTitle,
  t.speechModelTitle,
  t.starterSetTitle,
  t.syncTitle,
  t.tableTitle,
  t.tallyEntry,
  t.tallyField,
  t.tallyNewEntry,
  t.tallyNewField,
  t.trashTitle,
  t.unsorted,
];

/// Opens a page with [title] and [actions] on a screen [width] wide, as a
/// route over another, so it has a way back.
Future<void> _open(
  WidgetTester tester,
  double width, {
  required String title,
  List<Widget> actions = const [],
  Locale locale = const Locale('en'),
}) async {
  tester.view
    ..physicalSize = Size(width * 2, 1600)
    ..devicePixelRatio = 2;
  addTearDown(tester.view.reset);
  final navigator = GlobalKey<NavigatorState>();
  // A fresh app for each page, not one more route on the last.
  await tester.pumpWidget(const SizedBox());
  await tester.pumpWidget(
    MaterialApp(
      navigatorKey: navigator,
      theme: lookTheme(Brightness.light),
      locale: locale,
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      home: const Scaffold(),
    ),
  );
  navigator.currentState!.push(
    MaterialPageRoute<void>(
      builder: (context) => Scaffold(
        appBar: pageBar(context, title: title, actions: actions),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

void main() {
  setUpAll(_loadRoboto);

  final trash = IconButton(
    tooltip: 'Trash',
    onPressed: () {},
    icon: const Icon(Icons.delete_outline),
  );

  testWidgets('on a phone the title is on the line under the buttons', (
    tester,
  ) async {
    await _open(tester, _phone, title: 'Note', actions: [trash]);
    final title = tester.getRect(find.text('Note'));
    final back = tester.getRect(find.byTooltip('Back'));
    final button = tester.getRect(find.byTooltip('Trash'));
    expect(title.top, greaterThanOrEqualTo(back.bottom));
    expect(title.top, greaterThanOrEqualTo(button.bottom));
    expect(title.left, lessThan(button.left), reason: 'start-aligned');
    expect(
      find.descendant(of: find.byType(AppBar), matching: find.text('Note')),
      findsOneWidget,
    );
  });

  testWidgets('on a wide screen the title is beside the buttons', (
    tester,
  ) async {
    await _open(tester, _wide, title: 'Note', actions: [trash]);
    final title = tester.getRect(find.text('Note'));
    final back = tester.getRect(find.byTooltip('Back'));
    final button = tester.getRect(find.byTooltip('Trash'));
    expect(title.center.dy, closeTo(back.center.dy, 1));
    expect(title.center.dy, closeTo(button.center.dy, 1));
    expect(title.left, greaterThan(back.right));
  });

  testWidgets('a page without any button keeps its title on its one row', (
    tester,
  ) async {
    tester.view
      ..physicalSize = const Size(_phone * 2, 1600)
      ..devicePixelRatio = 2;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      MaterialApp(
        theme: lookTheme(Brightness.light),
        home: Builder(
          builder: (context) =>
              Scaffold(appBar: pageBar(context, title: 'Lock')),
        ),
      ),
    );
    expect(tester.getSize(find.byType(AppBar)).height, kToolbarHeight);
  });

  for (final language in ['en', 'de']) {
    testWidgets('no fixed title is cut on the narrowest phone ($language)', (
      tester,
    ) async {
      final locale = Locale(language);
      final t = lookupAppLocalizations(locale);
      for (final title in _titles(t)) {
        await _open(
          tester,
          _narrowest,
          title: title,
          actions: [trash],
          locale: locale,
        );
        final text = tester.renderObject<RenderParagraph>(find.text(title));
        expect(text.didExceedMaxLines, isFalse, reason: title);
        // A title on two lines makes the bar taller, not cut at its edge.
        final bar = tester.getRect(find.byType(AppBar));
        expect(
          tester.getRect(find.text(title)).bottom,
          lessThanOrEqualTo(bar.bottom),
          reason: title,
        );
      }
    });
  }
}
