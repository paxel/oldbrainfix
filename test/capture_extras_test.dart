import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/announcements.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/context_platform.dart';
import 'package:oldbrainfix/src/photo_platform.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_context.dart';
import 'fake_location.dart';
import 'fake_photos.dart';
import 'open_settings.dart';
import 'reveal.dart';
import 'open_capture.dart';

void main() {
  late Store store;
  late Notes notes;
  late FakeContext source;
  late FakePhotos photos;
  late Announcements announcements;

  setUp(() {
    store = Store.inMemory()..starterSetOffered = true;
    notes = Notes(store);
    source = FakeContext();
    photos = FakePhotos();
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
        context: source,
        photos: photos,
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
    await openTextCapture(tester);
    await tester.enterText(find.byType(TextField), text);
    await tap(tester, find.text('Save'));
  }

  Future<void> arrive(
    WidgetTester tester, {
    List<RunningEvent>? events,
    List<String>? devices,
  }) async {
    source.events.complete(events);
    source.devices.complete(devices);
    await tester.pumpAndSettle();
  }

  Future<void> openEditor(WidgetTester tester, String text) async {
    await tap(tester, find.byIcon(Icons.search));
    await tap(tester, find.byTooltip('All Notes'));
    await tester.longPress(find.text(text));
    await tester.pumpAndSettle();
  }

  final moreIcon = find.byIcon(Icons.more_horiz);

  group('Capture', () {
    testWidgets('records the running event and the connected device once '
        'they arrive; the card gets its Capture Context icon', (tester) async {
      store
        ..calendarContextOn = true
        ..bluetoothContextOn = true;
      await start(tester);
      await capture(tester, 'nees mor sauce');

      expect(find.text('nees mor sauce'), findsOneWidget, reason: 'at once');
      expect(notes.all().single.context.hasMore, isFalse);
      expect(moreIcon, findsNothing);

      await arrive(
        tester,
        events: [
          (title: 'Birthday Anna', allDay: true),
          (title: 'Dentist', allDay: false),
        ],
        devices: ['My Car', 'My Car', ' '],
      );
      final context = notes.all().single.context;
      expect(context.calendarEvents, ['Dentist']);
      expect(context.bluetoothDevices, ['My Car']);
      expect(moreIcon, findsOneWidget);
      expect(source.asked, isEmpty, reason: 'Capture never asks');

      await openEditor(tester, 'nees mor sauce');
      expect(find.text('During: Dentist'), findsOneWidget);
      expect(find.text('Connected: My Car'), findsOneWidget);
    });

    testWidgets('the place shows without waiting for the device', (
      tester,
    ) async {
      final home = Coordinates(48.137154, 11.576124);
      NamedPlaces(store).add('Home', home, radius: 100);
      store
        ..locationExplained = true
        ..bluetoothContextOn = true;
      final location = FakeLocation();
      await tester.pumpWidget(
        OldBrainFixApp(store: store, location: location, context: source),
      );
      await tester.pumpAndSettle();
      await capture(tester, 'nees mor sauce');

      location.fix.complete(home);
      await tester.pumpAndSettle();
      expect(find.text('Home'), findsOneWidget);
      expect(moreIcon, findsNothing);

      source.devices.complete(['My Car']);
      await tester.pumpAndSettle();
      expect(moreIcon, findsOneWidget);
    });

    testWidgets('looks nothing up while switched off', (tester) async {
      await start(tester);
      await capture(tester, 'nees mor sauce');
      expect(source.eventLookups, 0);
      expect(source.deviceLookups, 0);
      expect(moreIcon, findsNothing);
      expect(notes.all().single.context.hasMore, isFalse);
    });

    testWidgets('a permission taken back is not looked up and never asked', (
      tester,
    ) async {
      store
        ..calendarContextOn = true
        ..bluetoothContextOn = true;
      source.allowed.clear();
      await start(tester);
      await capture(tester, 'nees mor sauce');
      expect(find.byType(AlertDialog), findsNothing);
      expect(source.asked, isEmpty);
      expect(source.eventLookups, 0);
      expect(source.deviceLookups, 0);
    });

    testWidgets('with nothing running and nothing connected nothing more '
        'shows', (tester) async {
      store
        ..calendarContextOn = true
        ..bluetoothContextOn = true;
      await start(tester);
      await capture(tester, 'nees mor sauce');
      await arrive(tester, events: const [], devices: null);
      expect(notes.all().single.context.hasMore, isFalse);
      expect(moreIcon, findsNothing);
    });

    testWidgets('an Amend records its own', (tester) async {
      store.bluetoothContextOn = true;
      source.devices.complete(['Headphones']);
      final note = notes.capture('nees mor sauce');
      await start(tester);
      await openEditor(tester, 'nees mor sauce');
      expect(find.text('Connected: Headphones'), findsNothing);

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
      expect(find.text('Connected: Headphones'), findsOneWidget);
      expect(notes.sections(note.id).single.context.bluetoothDevices, [
        'Headphones',
      ]);
      expect(notes.byId(note.id)?.context.hasMore, isFalse);
    });
  });

  group('sharing in', () {
    testWidgets('records the app it came from', (tester) async {
      photos.waiting.add(
        const SharedItem(
          text: 'https://example.org/sauce',
          sourceApp: SourceApp('com.whatsapp', label: 'WhatsApp'),
        ),
      );
      await start(tester);
      await tap(tester, find.text('Save'));

      final note = notes.all().single;
      expect(note.context.sourceApp?.name, 'WhatsApp');
      expect(note.context.sourceApp?.package, 'com.whatsapp');
      expect(moreIcon, findsOneWidget);

      await openEditor(tester, 'https://example.org/sauce');
      expect(find.text('Shared from WhatsApp'), findsOneWidget);
    });

    testWidgets('a share without a source records none', (tester) async {
      photos.waiting.add(const SharedItem(text: 'https://example.org/pills'));
      await start(tester);
      await tap(tester, find.text('Save'));
      expect(notes.all().single.context.sourceApp, isNull);
      expect(moreIcon, findsNothing);
    });
  });

  group('the switches', () {
    final calendarSwitch = find.byKey(const ValueKey('context-calendar'));
    final bluetoothSwitch = find.byKey(const ValueKey('context-bluetooth'));

    /// The Capture Context page, from Settings: the Bluetooth device.
    Future<void> openPage(WidgetTester tester) async {
      await openDeviceSettings(tester);
      await tester.ensureVisible(find.text('Capture Context'));
      await tap(tester, find.text('Capture Context'));
    }

    /// The Calendar page, from the Agenda's bar: the running appointment.
    Future<void> openCalendar(WidgetTester tester) async {
      await tap(tester, find.byKey(const ValueKey('calendarPage')));
      expect(find.text('Read from your calendar'), findsOneWidget);
    }

    bool isOn(WidgetTester tester, Finder finder) =>
        tester.widget<Switch>(finder).value;

    testWidgets('are off until switched on', (tester) async {
      await start(tester);
      await openCalendar(tester);
      expect(isOn(tester, calendarSwitch), isFalse);
      await tester.pageBack();
      await tester.pumpAndSettle();
      await openPage(tester);
      expect(isOn(tester, bluetoothSwitch), isFalse);
      expect(calendarSwitch, findsNothing, reason: 'on the Calendar page');
    });

    testWidgets('switching on explains first, then asks', (tester) async {
      source.allowed.clear();
      await start(tester);
      await openCalendar(tester);
      await tap(tester, calendarSwitch);
      expect(find.text('Remember your appointment?'), findsOneWidget);
      expect(source.asked, isEmpty, reason: 'the explanation comes first');
      await tap(tester, find.text('Allow'));
      expect(source.asked, [ContextKind.calendar]);
      expect(store.calendarContextOn, isTrue);
      expect(isOn(tester, calendarSwitch), isTrue);
      await tester.pageBack();
      await tester.pumpAndSettle();

      await openPage(tester);
      await tap(tester, bluetoothSwitch);
      expect(find.text('Remember what was connected?'), findsOneWidget);
      await tap(tester, find.text('Allow'));
      expect(source.asked, [ContextKind.calendar, ContextKind.bluetooth]);
      expect(store.bluetoothContextOn, isTrue);
    });

    testWidgets('"Not now" leaves it off and never asks', (tester) async {
      source.allowed.clear();
      await start(tester);
      await openPage(tester);
      await tap(tester, bluetoothSwitch);
      await tap(tester, find.text('Not now'));
      expect(source.asked, isEmpty);
      expect(store.bluetoothContextOn, isFalse);
      expect(isOn(tester, bluetoothSwitch), isFalse);
    });

    testWidgets('a refusal leaves it off and says so at the top', (
      tester,
    ) async {
      source
        ..allowed.clear()
        ..grantOnAsk = false;
      await start(tester);
      await openCalendar(tester);
      await tap(tester, calendarSwitch);
      await tap(tester, find.text('Allow'));
      expect(store.calendarContextOn, isFalse);
      expect(isOn(tester, calendarSwitch), isFalse);
      expect(announcements.visible, contains('may not read your calendar'));
    });

    testWidgets('switches off again; a taken-back permission shows off', (
      tester,
    ) async {
      store
        ..calendarContextOn = true
        ..bluetoothContextOn = true;
      source.allowed.remove(ContextKind.bluetooth);
      await start(tester);
      await openCalendar(tester);
      expect(isOn(tester, calendarSwitch), isTrue);
      await tap(tester, calendarSwitch);
      expect(store.calendarContextOn, isFalse);
      expect(isOn(tester, calendarSwitch), isFalse);
      expect(source.asked, isEmpty);
      await tester.pageBack();
      await tester.pumpAndSettle();
      await openPage(tester);
      expect(isOn(tester, bluetoothSwitch), isFalse);
    });

    testWidgets('the Agenda bar keeps its order', (tester) async {
      await start(tester);
      final xs = [
        for (final key in ['allRoutines', 'calendarPage', 'profile'])
          tester.getTopLeft(find.byKey(ValueKey(key))).dx,
      ];
      final newRoutine = tester.getTopLeft(find.byIcon(Icons.add_alarm)).dx;
      expect(
        [xs[0], xs[1], newRoutine, xs[2]],
        [
          ...[xs[0], xs[1], newRoutine, xs[2]]..sort(),
        ],
      );
    });

    testWidgets('are not offered without a source', (tester) async {
      await tester.pumpWidget(OldBrainFixApp(store: store));
      await tester.pumpAndSettle();
      expect(find.byKey(const ValueKey('calendarPage')), findsNothing);
      await openDeviceSettings(tester);
      expect(find.text('Capture Context'), findsNothing);
      expect(find.byKey(const ValueKey('calendarPage')), findsNothing);
    });
  });

  test('the lookups never ask and skip what is off', () async {
    store.calendarContextOn = true;
    source.events.complete([(title: 'Dentist', allDay: false)]);
    final extras = await currentExtras(store, source);
    expect(extras.calendarEvents, ['Dentist']);
    expect(extras.bluetoothDevices, isEmpty);
    expect(source.deviceLookups, 0);
    expect(source.asked, isEmpty);
  });
}
