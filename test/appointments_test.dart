import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/appointments.dart';
import 'package:oldbrainfix/src/calendar_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_appointments.dart';

void main() {
  late Store store;
  late Notes notes;
  late Routines routines;
  late FakeAppointments source;
  // 09:00 on Saturday, 24 October 2026, on the test machine's clock.
  var now = DateTime(2026, 10, 24, 9);

  setUp(() {
    now = DateTime(2026, 10, 24, 9);
    store = Store.inMemory(clock: () => now)..starterSetOffered = true;
    notes = Notes(store);
    routines = Routines(store);
    source = FakeAppointments();
  });
  tearDown(() => store.close());

  DateTime local(int day, int hour, [int minute = 0]) =>
      DateTime(2026, 10, day, hour, minute).toUtc();

  Appointment timed(
    String id,
    String title,
    DateTime begin,
    DateTime end, {
    String calendar = '1',
    String location = '',
    String description = '',
  }) => Appointment(
    id: id,
    calendarId: calendar,
    title: title,
    begin: begin,
    end: end,
    allDay: false,
    location: location,
    description: description,
  );

  Appointment allDay(String id, String title, int day, {int days = 1}) =>
      Appointment(
        id: id,
        calendarId: '1',
        title: title,
        begin: DateTime.utc(2026, 10, day),
        end: DateTime.utc(2026, 10, day + days),
        allDay: true,
      );

  Future<void> start(WidgetTester tester) async {
    await tester.pumpWidget(OldBrainFixApp(store: store, appointments: source));
    await tester.pumpAndSettle();
  }

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  /// Shows calendar 1, "Personal", on the Agenda, allowed.
  void showPersonal() {
    source.allowed = true;
    store
      ..agendaCalendarOn = true
      ..agendaCalendars = {'1'};
  }

  final calendarButton = find.byKey(const ValueKey('calendarPage'));
  final agendaSwitch = find.byKey(const ValueKey('agendaCalendar'));
  Finder row(String key) => find.byKey(ValueKey('appointment-$key'));
  Finder menu(String key) => find.byKey(ValueKey('appointment-menu-$key'));
  const copy = 'Copy into a Note';

  group('Calendar page', () {
    testWidgets('the switch asks, then lists the calendars, none picked', (
      tester,
    ) async {
      await start(tester);
      await tap(tester, calendarButton);
      expect(find.text('Read from your calendar'), findsOneWidget);
      expect(tester.widget<Switch>(agendaSwitch).value, isFalse);

      await tap(tester, agendaSwitch);
      expect(find.text('Show your appointments?'), findsOneWidget);
      await tap(tester, find.text('Allow'));

      expect(source.requests, 1);
      expect(find.byType(AgendaCalendarsScreen), findsOneWidget);
      expect(find.text('Personal'), findsOneWidget);
      expect(find.text('Work'), findsOneWidget);
      final boxes = tester.widgetList<CheckboxListTile>(
        find.byType(CheckboxListTile),
      );
      expect(boxes.map((box) => box.value), [false, false]);
      expect(store.agendaCalendarOn, isTrue);
      expect(store.agendaCalendars, isEmpty);

      await tap(tester, find.byKey(const ValueKey('agendaCalendar-2')));
      expect(store.agendaCalendars, {'2'});
      await tap(tester, find.byTooltip('Back'));

      expect(find.byType(CalendarScreen), findsOneWidget);
      expect(tester.widget<Switch>(agendaSwitch).value, isTrue);
      expect(
        find.text('From Work. Tap to pick other calendars.'),
        findsOneWidget,
      );

      // Off keeps the pick; the text opens the list again only while on.
      await tap(tester, agendaSwitch);
      expect(store.agendaCalendarOn, isFalse);
      expect(store.agendaCalendars, {'2'});
    });

    testWidgets('while on, a tap on the text picks the calendars again', (
      tester,
    ) async {
      showPersonal();
      await start(tester);
      await tap(tester, calendarButton);
      expect(
        find.text('From Personal. Tap to pick other calendars.'),
        findsOneWidget,
      );
      await tap(
        tester,
        find.text("Show my calendar's appointments on the Agenda"),
      );
      expect(find.byType(AgendaCalendarsScreen), findsOneWidget);
      expect(
        tester
            .widget<CheckboxListTile>(
              find.byKey(const ValueKey('agendaCalendar-1')),
            )
            .value,
        isTrue,
      );
    });

    testWidgets('a phone without a calendar says what to do behind the tap', (
      tester,
    ) async {
      source = FakeAppointments(allowOnRequest: true, calendars: const []);
      await start(tester);
      await tap(tester, calendarButton);
      await tap(tester, agendaSwitch);
      await tap(tester, find.text('Allow'));
      expect(find.byType(AgendaCalendarsScreen), findsNothing);

      await tap(
        tester,
        find.text('The calendar cannot be shown on the Agenda.'),
      );
      expect(
        find.text(
          'There is no calendar on this device to read from. Add a calendar '
          "account in the phone's settings, then turn this on again.",
        ),
        findsOneWidget,
      );
      expect(find.text('Report'), findsOneWidget);
    });

    testWidgets('refused access leaves it off and says so', (tester) async {
      source.allowOnRequest = false;
      await start(tester);
      await tap(tester, calendarButton);
      await tap(tester, agendaSwitch);
      await tap(tester, find.text('Allow'));

      // Only that it failed; why and what to do behind the tap.
      await tap(
        tester,
        find.text('The calendar cannot be shown on the Agenda.'),
      );
      expect(find.text('Calendar not shown'), findsOneWidget);
      expect(
        find.text(
          'Old Brain Fix may not read your calendar. You can allow it in the '
          'system settings, then turn this on again.',
        ),
        findsOneWidget,
      );
      expect(find.text('Report'), findsOneWidget);
      await tap(tester, find.text('Close'));
      expect(find.byType(AgendaCalendarsScreen), findsNothing);
      expect(tester.widget<Switch>(agendaSwitch).value, isFalse);
      expect(store.agendaCalendarOn, isFalse);

      // "Not now" does not even ask.
      await tap(tester, agendaSwitch);
      await tap(tester, find.text('Not now'));
      expect(source.requests, 1);
      expect(store.agendaCalendarOn, isFalse);
    });

    testWidgets('access taken away later shows nothing on the Agenda', (
      tester,
    ) async {
      showPersonal();
      source
        ..allowed = false
        ..add(timed('a', 'Lunch with Ann', local(24, 12), local(24, 13)));
      await start(tester);
      expect(find.text('Lunch with Ann'), findsNothing);
    });
  });

  group('Agenda', () {
    testWidgets('appointments show among the Occurrences, marked, read only', (
      tester,
    ) async {
      showPersonal();
      routines
        ..create(
          'water the plants',
          Schedule(start: Day(2026, 10, 24), times: const []),
        )
        ..create(
          'pills',
          Schedule(start: Day(2026, 10, 24), times: [ClockTime(8, 0)]),
        )
        ..create(
          'walk',
          Schedule(start: Day(2026, 10, 24), times: [ClockTime(18, 0)]),
        );
      source
        ..add(allDay('h', 'Holiday', 24))
        ..add(
          timed(
            'l',
            'Lunch with Ann',
            local(24, 12),
            local(24, 13, 30),
            location: 'Main Street 5',
            description: 'Bring the voucher',
          ),
        )
        ..add(
          timed('w', 'Stand-up', local(24, 10), local(24, 11), calendar: '2'),
        )
        // The mirror's own copy of "walk".
        ..add(
          timed('m', 'walk', local(24, 18), local(24, 18, 30)),
          mirrored: true,
        );
      await start(tester);

      double y(String text) => tester.getTopLeft(find.text(text).first).dy;
      expect(y('water the plants'), lessThan(y('Holiday')));
      expect(y('Holiday'), lessThan(y('pills')));
      expect(y('pills'), lessThan(y('Lunch with Ann')));
      expect(y('Lunch with Ann'), lessThan(y('walk')));

      final lunch = row('l@2026-10-24');
      expect(
        find.descendant(of: lunch, matching: find.text('12:00–13:30')),
        findsOneWidget,
      );
      expect(
        find.descendant(of: lunch, matching: find.text('Personal')),
        findsOneWidget,
      );
      expect(
        find.descendant(
          of: lunch,
          matching: find.byWidgetPredicate(
            (widget) =>
                widget is Icon &&
                widget.semanticLabel == 'From the calendar “Personal”',
          ),
        ),
        findsOneWidget,
      );
      expect(
        find.descendant(of: lunch, matching: find.byType(Checkbox)),
        findsNothing,
      );
      expect(find.text('Stand-up'), findsNothing, reason: 'not picked');
      expect(find.text('walk'), findsOneWidget, reason: 'mirror left out');

      // A tap shows the place and description, and changes nothing.
      expect(find.text('Place: Main Street 5'), findsNothing);
      await tap(tester, find.text('Lunch with Ann'));
      expect(find.text('Place: Main Street 5'), findsOneWidget);
      expect(find.text('Bring the voucher'), findsOneWidget);
      expect(find.text(copy), findsNothing, reason: 'a tap opens no menu');
      expect(notes.all(), hasLength(3), reason: 'only the Routines\' Notes');
    });

    testWidgets('appointments show on the later days too', (tester) async {
      showPersonal();
      source
        ..add(timed('t', 'Dentist', local(25, 14, 30), local(25, 15)))
        ..add(allDay('v', 'Holidays', 27, days: 2));
      await start(tester);
      Future<void> shows(String key) => tester.scrollUntilVisible(
        row(key),
        200,
        scrollable: find.byType(Scrollable).first,
      );
      await shows('t@2026-10-25');
      await shows('v@2026-10-27');
      await shows('v@2026-10-28');
      await tester.scrollUntilVisible(
        find.text('Nothing planned').last,
        200,
        scrollable: find.byType(Scrollable).first,
      );
      expect(row('v@2026-10-29'), findsNothing);
    });

    testWidgets('"Copy into a Note" makes a Note of every detail', (
      tester,
    ) async {
      showPersonal();
      source.add(
        timed(
          'l',
          'Lunch with Ann',
          local(24, 12),
          local(24, 13, 30),
          location: 'Main Street 5',
          description: 'Bring the voucher',
        ),
      );
      await start(tester);

      await tester.longPress(find.text('Lunch with Ann'));
      await tester.pumpAndSettle();
      await tap(tester, find.text(copy));

      final note = notes.all().single;
      expect(
        note.text,
        'Lunch with Ann\n'
        'Sat, 10/24/2026, 12:00–13:30\n'
        'Place: Main Street 5\n'
        '\n'
        'Bring the voucher',
      );
      expect(note.isUnsorted, isTrue);
      expect(find.text('Just captured'), findsOneWidget);
      expect(source.events, hasLength(1), reason: 'the calendar stays');

      // ⋮ offers the same.
      await tap(tester, menu('l@2026-10-24'));
      await tap(tester, find.text(copy));
      expect(notes.all(), hasLength(2));
    });

    testWidgets('read again on resume', (tester) async {
      showPersonal();
      await start(tester);
      expect(find.text('Lunch with Ann'), findsNothing);

      source.add(timed('l', 'Lunch with Ann', local(24, 12), local(24, 13)));
      for (final state in [
        AppLifecycleState.inactive,
        AppLifecycleState.hidden,
        AppLifecycleState.inactive,
        AppLifecycleState.resumed,
      ]) {
        tester.binding.handleAppLifecycleStateChanged(state);
      }
      await tester.pumpAndSettle();
      expect(find.text('Lunch with Ann'), findsOneWidget);
    });
  });

  group('appointmentsOn', () {
    DateTime wall(DateTime utc) => utc.toLocal();

    test('a timed one over midnight shows on both days', () {
      final night = timed('n', 'Night shift', local(24, 22), local(25, 6));
      final first = appointmentsOn(Day(2026, 10, 24), [night], wall).single;
      expect((first.from, first.until), (ClockTime(22, 0), null));
      final second = appointmentsOn(Day(2026, 10, 25), [night], wall).single;
      expect((second.from, second.until), (null, ClockTime(6, 0)));
      expect(second.at, ClockTime(0, 0));
      expect(appointmentsOn(Day(2026, 10, 26), [night], wall), isEmpty);
    });

    test('an end at midnight closes the day before', () {
      final evening = timed('e', 'Party', local(24, 20), local(25, 0));
      expect(appointmentsOn(Day(2026, 10, 25), [evening], wall), isEmpty);
    });

    test('all-day ones first, then by time and title', () {
      final day = Day(2026, 10, 24);
      final pieces = appointmentsOn(day, [
        timed('b', 'B', local(24, 9), local(24, 10)),
        allDay('z', 'Z', 24),
        timed('a', 'A', local(24, 9), local(24, 10)),
        allDay('y', 'Y', 24),
      ], wall);
      expect(
        [for (final p in pieces) p.appointment.title],
        ['Y', 'Z', 'A', 'B'],
      );
    });
  });
}
