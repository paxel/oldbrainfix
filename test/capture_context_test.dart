import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/announcements.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_location.dart';
import 'open_settings.dart';
import 'reveal.dart';

void main() {
  late Store store;
  late Notes notes;
  late NamedPlaces named;
  late FakeLocation location;
  late Announcements announcements;

  final home = Coordinates(48.137154, 11.576124);
  // About 1.1 km north of home.
  final faraway = Coordinates(48.147154, 11.576124);

  setUp(() {
    store = Store.inMemory()..starterSetOffered = true;
    notes = Notes(store);
    named = NamedPlaces(store);
    location = FakeLocation();
    announcements = Announcements();
  });
  tearDown(() {
    announcements.dispose();
    store.close();
  });

  Future<void> start(WidgetTester tester) async {
    await tester.pumpWidget(
      OldBrainFixApp(
        store: store,
        location: location,
        announcements: announcements,
      ),
    );
    await tester.pumpAndSettle();
  }

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  Future<void> capture(WidgetTester tester, String text) async {
    await tap(tester, find.byIcon(Icons.add));
    await tester.enterText(find.byType(TextField), text);
    await tap(tester, find.text('Save'));
  }

  Future<void> arrive(WidgetTester tester, Coordinates? at) async {
    location.fix.complete(at);
    await tester.pumpAndSettle();
  }

  group('Capture', () {
    testWidgets('records the Note at once; the place shows when it arrives', (
      tester,
    ) async {
      store.locationExplained = true;
      named.add('Home', home, radius: 100);
      await start(tester);
      await capture(tester, 'nees mor sauce');

      expect(find.text('nees mor sauce'), findsOneWidget);
      expect(notes.all().single.context.location, isNull);
      expect(find.text('Home'), findsNothing);

      await arrive(tester, home);
      expect(find.text('Home'), findsOneWidget);
      expect(notes.all().single.context.location, home);
      expect(location.asked, 0);
    });

    testWidgets('away from every Named Place the card shows coordinates', (
      tester,
    ) async {
      store.locationExplained = true;
      named.add('Home', home, radius: 100);
      await start(tester);
      await capture(tester, 'nees mor sauce');
      await arrive(tester, faraway);
      expect(find.text('48.1472, 11.5761'), findsOneWidget);
      expect(find.text('Home'), findsNothing);
    });

    testWidgets('with no position the Note stays, without a place', (
      tester,
    ) async {
      store.locationExplained = true;
      await start(tester);
      await capture(tester, 'nees mor sauce');
      await arrive(tester, null);
      expect(find.text('nees mor sauce'), findsOneWidget);
      expect(notes.all().single.context.location, isNull);
    });

    testWidgets('while the location is denied, Capture never asks', (
      tester,
    ) async {
      store.locationExplained = true;
      location.allowed = false;
      await start(tester);
      await capture(tester, 'nees mor sauce');
      expect(find.text('nees mor sauce'), findsOneWidget);
      expect(find.byType(AlertDialog), findsNothing);
      expect(location.asked, 0);
      expect(location.lookups, 0);
    });

    testWidgets('after the first Capture it explains, then asks once', (
      tester,
    ) async {
      location.allowed = false;
      named.add('Home', home, radius: 100);
      await start(tester);
      await tap(tester, find.byIcon(Icons.add));
      expect(find.byType(AlertDialog), findsNothing, reason: 'not in Capture');
      await tester.enterText(find.byType(TextField), 'nees mor sauce');
      await tap(tester, find.text('Save'));

      expect(notes.all().single.text, 'nees mor sauce');
      expect(find.text('Remember where you were?'), findsOneWidget);
      expect(find.text('nees mor sauce'), findsOneWidget, reason: 'listed');
      expect(location.asked, 0, reason: 'the explanation comes first');
      await tap(tester, find.text('Allow'));
      expect(location.asked, 1);
      expect(
        find.text('nees mor sauce'),
        findsOneWidget,
        reason: 'the list does not wait for the position',
      );
      expect(find.text('Home'), findsNothing);
      await arrive(tester, home);
      expect(find.text('Home'), findsOneWidget, reason: 'that Note too');

      location.fix = Completer()..complete(home);
      await capture(tester, 'second');
      expect(find.byType(AlertDialog), findsNothing);
      expect(location.asked, 1);
    });

    testWidgets('"Not now" leaves the Note without a place and never asks', (
      tester,
    ) async {
      location.allowed = false;
      await start(tester);
      await capture(tester, 'nees mor sauce');
      await tap(tester, find.text('Not now'));
      expect(location.asked, 0);
      expect(notes.all().single.context.location, isNull);
      await capture(tester, 'second');
      expect(find.byType(AlertDialog), findsNothing);
      expect(location.asked, 0);
    });

    testWidgets('without a location source nothing is explained', (
      tester,
    ) async {
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();
      await capture(tester, 'nees mor sauce');
      expect(find.byType(AlertDialog), findsNothing);
      expect(store.locationExplained, isFalse);
    });
  });

  group('editor', () {
    testWidgets('shows the Named Place with its coordinates, and an Amend '
        'gets its own place', (tester) async {
      named.add('Home', home, radius: 100);
      final note = notes.capture('nees mor sauce');
      notes.locate(note.id, home);
      await start(tester);
      await tap(tester, find.byIcon(Icons.search));
      await tap(tester, find.byTooltip('All Notes'));
      await tap(tester, find.text('nees mor sauce'));
      expect(find.text('Home (48.1372, 11.5761)'), findsOneWidget);

      await reveal(tester, find.text('Add to Note'));
      await tap(tester, find.text('Add to Note'));
      await tester.enterText(
        find.descendant(
          of: find.byType(AlertDialog),
          matching: find.byType(TextField),
        ),
        'sauce bought',
      );
      await tap(tester, find.text('Add'));
      expect(find.text('sauce bought'), findsOneWidget);
      await arrive(tester, faraway);
      expect(find.text('48.1472, 11.5761'), findsOneWidget);
      expect(notes.sections(note.id).single.context.location, faraway);
    });
  });

  group('Named Places', () {
    Future<void> openPage(WidgetTester tester) async {
      await start(tester);
      await openDeviceSettings(tester);
      await tester.ensureVisible(find.text('Named Places'));
      await tap(tester, find.text('Named Places'));
    }

    Finder inDialog(Finder finder) =>
        find.descendant(of: find.byType(AlertDialog), matching: finder);

    testWidgets('+ names the spot the phone is at, with a radius', (
      tester,
    ) async {
      await openPage(tester);
      expect(find.textContaining('No Named Places yet'), findsOneWidget);
      await tester.tap(find.byTooltip('Name this place'));
      await tester.pump();
      await arrive(tester, home);
      await tester.enterText(inDialog(find.byType(TextField)), 'Home');
      await tap(tester, inDialog(find.text('250 m')));
      await tap(tester, inDialog(find.text('Save')));

      final place = named.all().single;
      expect(place.name, 'Home');
      expect(place.centre, home);
      expect(place.radius, 250);
      expect(find.text('Home'), findsOneWidget);
      expect(find.text('250 m · 48.1372, 11.5761'), findsOneWidget);
    });

    testWidgets('a Named Place is renamed, resized and deleted', (
      tester,
    ) async {
      named.add('Home', home, radius: 100);
      final note = notes.capture('nees mor sauce');
      notes.locate(note.id, home);
      await openPage(tester);

      await tap(tester, find.byIcon(Icons.more_vert));
      await tap(tester, find.text('Rename or resize'));
      await tester.enterText(inDialog(find.byType(TextField)), "Mum's");
      await tap(tester, inDialog(find.text('50 m')));
      await tap(tester, inDialog(find.text('Save')));
      expect(named.all().single.name, "Mum's");
      expect(named.all().single.radius, 50);
      expect(find.text("Mum's"), findsOneWidget);

      await tap(tester, find.byIcon(Icons.more_vert));
      await tap(tester, find.text('Delete'));
      expect(find.textContaining('coordinates again'), findsOneWidget);
      await tap(tester, inDialog(find.text('Delete')));
      expect(named.all(), isEmpty);
      expect(notes.byId(note.id)?.context.location, home);

      // Back on the Agenda, through Settings and the Profile page, and
      // into the browser.
      await tap(tester, find.byType(BackButton));
      await tap(tester, find.byType(BackButton));
      await tap(tester, find.byType(BackButton));
      await tap(tester, find.byIcon(Icons.search));
      await tap(tester, find.byTooltip('All Notes'));
      expect(find.text('48.1372, 11.5761'), findsOneWidget);
    });

    testWidgets('+ explains and asks when the location is not allowed', (
      tester,
    ) async {
      location
        ..allowed = false
        ..grantOnAsk = false;
      await openPage(tester);
      await tap(tester, find.byTooltip('Name this place'));
      expect(find.text('Remember where you were?'), findsOneWidget);
      await tap(tester, find.text('Allow'));
      expect(location.asked, 1);
      expect(announcements.visible, contains('may not use your location'));
      expect(named.all(), isEmpty);
    });

    testWidgets('+ with no position says so at the top', (tester) async {
      await openPage(tester);
      await tester.tap(find.byTooltip('Name this place'));
      await tester.pump();
      await arrive(tester, null);
      expect(announcements.visible, contains('Could not find out where'));
      expect(find.byType(AlertDialog), findsNothing);
    });
  });
}
