import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:intl/date_symbol_data_local.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/agenda_screen.dart';
import 'package:oldbrainfix/src/agenda_widget.dart';
import 'package:oldbrainfix/src/alarm_platform.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/app_lock.dart';
import 'package:oldbrainfix/src/areas_screen.dart';
import 'package:oldbrainfix/src/calendar_export_screen.dart';
import 'package:oldbrainfix/src/calendar_mirror.dart';
import 'package:oldbrainfix/src/capture_requests.dart';
import 'package:oldbrainfix/src/capture_screen.dart';
import 'package:oldbrainfix/src/file_into_screen.dart';
import 'package:oldbrainfix/src/lock_screen.dart';
import 'package:oldbrainfix/src/merge_into_screen.dart';
import 'package:oldbrainfix/src/missed_summary_card.dart';
import 'package:oldbrainfix/src/notes_screen.dart';
import 'package:oldbrainfix/src/nudge_screen.dart';
import 'package:oldbrainfix/src/photo_platform.dart';
import 'package:oldbrainfix/src/ring_screen.dart';
import 'package:oldbrainfix/src/routine_editor_screen.dart';
import 'package:oldbrainfix/src/trash_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_alarms.dart';
import 'fake_calendar.dart';
import 'fake_capture_requests.dart';
import 'fake_lock.dart';
import 'fake_mirror.dart';
import 'fake_photos.dart';
import 'open_settings.dart';

void main() {
  late Store store;
  late Notes notes;
  late Places places;
  late Routines routines;
  late FakeAlarms alarms;
  late FakeBiometrics biometrics;
  late FakePhotos photos;
  late FakeCaptureRequests requests;
  // 09:00 on Saturday, 24 October 2026, on the test machine's clock.
  var now = DateTime(2026, 10, 24, 9);
  final en = lookupAppLocalizations(const Locale('en'));
  const pin = '2468';

  // The app's localizations load these; the snapshot test runs without.
  setUpAll(initializeDateFormatting);

  setUp(() {
    now = DateTime(2026, 10, 24, 9);
    store = Store.inMemory(clock: () => now)
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    notes = Notes(store);
    places = Places(store);
    routines = Routines(store);
    alarms = FakeAlarms();
    biometrics = FakeBiometrics(can: false);
    photos = FakePhotos();
    requests = FakeCaptureRequests();
    // Few rounds, so the tests stay quick; the hash says how many.
    store.setPin(pin, iterations: 10);
  });
  tearDown(() => store.close());

  Schedule daily(int hour) => Schedule(
    start: Day(2026, 10, 1),
    times: [ClockTime(hour, 0)],
    rule: Recurrence.parse('FREQ=DAILY'),
  );

  /// "Doctor", locked, with "Pills" in it at 10:00 every day.
  ({Area doctor, Note note, Routine routine}) lockedDoctor({
    bool masked = true,
  }) {
    final doctor = places.addArea('Doctor');
    final note = notes.capture('Pills', place: doctor.id);
    final routine = routines.spawn(note.id, daily(10));
    store.setAreaLocked(doctor.id, true);
    store.setAreaMasked(doctor.id, masked);
    return (doctor: doctor, note: note, routine: routine);
  }

  Future<void> start(WidgetTester tester, {MirrorPlatform? mirror}) async {
    await tester.pumpWidget(
      OldBrainFixApp(
        store: store,
        alarms: alarms,
        lock: biometrics,
        photos: photos,
        requests: requests,
        calendar: FakeCalendar(),
        mirror: mirror ?? const NoMirror(),
        mirrorDelay: const Duration(milliseconds: 100),
      ),
    );
    await tester.pumpAndSettle();
  }

  /// Starts on a page tall enough for the Agenda's whole week.
  Future<void> startWeek(WidgetTester tester) async {
    tester.view.physicalSize = const Size(800, 4000);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await start(tester);
  }

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  Future<void> back(WidgetTester tester) async {
    await tester.binding.handlePopRoute();
    await tester.pumpAndSettle();
  }

  /// The app goes to the background for [away] and comes back.
  Future<void> away(WidgetTester tester, Duration away) async {
    final binding = tester.binding;
    binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
    now = now.add(away);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await tester.pumpAndSettle();
  }

  /// Types [pin] into the PIN dialog and unlocks.
  Future<void> enterPin(WidgetTester tester, String pin) async {
    await tester.enterText(find.byKey(const ValueKey('pinField')), pin);
    await tap(tester, find.widgetWithText(TextButton, en.unlock));
  }

  final lockScreen = find.byType(LockScreen);

  group('the App Lock', () {
    setUp(() => store.appLockOn = true);

    testWidgets('stands in front of the Agenda from the start', (tester) async {
      final semantics = tester.ensureSemantics();
      routines.create('Call the vet', daily(10));
      await start(tester);

      expect(lockScreen, findsOneWidget);
      expect(find.text(en.lockedTitle), findsOneWidget);
      expect(find.text('Call the vet'), findsNothing);
      // Not even for a screen reader.
      expect(find.bySemanticsLabel(RegExp('Call the vet')), findsNothing);
      expect(find.bySemanticsLabel(RegExp(en.lockedTitle)), findsWidgets);
      semantics.dispose();
    });

    testWidgets('opens with the PIN, not with a wrong one', (tester) async {
      routines.create('Call the vet', daily(10));
      await start(tester);

      await tester.enterText(find.byKey(const ValueKey('lockPin')), '1357');
      await tap(tester, find.byKey(const ValueKey('unlockPin')));
      expect(find.text(en.pinWrong), findsOneWidget);
      expect(lockScreen, findsOneWidget);

      await tester.enterText(find.byKey(const ValueKey('lockPin')), pin);
      await tap(tester, find.byKey(const ValueKey('unlockPin')));
      expect(lockScreen, findsNothing);
      expect(find.text('Call the vet'), findsWidgets);
    });

    testWidgets('opens with biometrics where they are on', (tester) async {
      biometrics.can = true;
      biometrics.answer = false;
      store.biometricsOn = true;
      await start(tester);

      final button = find.byKey(const ValueKey('unlockBiometrics'));
      await tap(tester, button);
      expect(biometrics.asked, [en.unlockApp]);
      expect(lockScreen, findsOneWidget);

      biometrics.answer = true;
      await tap(tester, button);
      expect(lockScreen, findsNothing);
      expect(find.byType(AgendaScreen), findsOneWidget);
    });

    testWidgets('offers no biometrics while they are off', (tester) async {
      await start(tester);
      expect(find.byKey(const ValueKey('unlockBiometrics')), findsNothing);
    });

    testWidgets('lets "+" capture without unlocking', (tester) async {
      await start(tester);

      await tap(tester, find.byTooltip(en.captureTooltip));
      expect(find.byType(CaptureScreen), findsOneWidget);
      await tester.enterText(find.byType(TextField), 'buy stamps');
      await back(tester);

      expect(notes.all().single.text, 'buy stamps');
      expect(lockScreen, findsOneWidget);
      expect(find.text('buy stamps'), findsNothing);
    });

    testWidgets('lets a share open Capture without unlocking', (tester) async {
      await start(tester);

      photos.share(const SharedItem(text: 'https://example.org/pills'));
      await tester.pumpAndSettle();
      expect(find.byType(CaptureScreen), findsOneWidget);
      await back(tester);

      expect(notes.all().single.text, 'https://example.org/pills');
      expect(lockScreen, findsOneWidget);
    });

    testWidgets('lets a shortcut open Capture at start without unlocking', (
      tester,
    ) async {
      requests.waiting = CaptureMode.text;
      await start(tester);

      expect(find.byType(CaptureScreen), findsOneWidget);
      await tester.enterText(find.byType(TextField), 'water plants');
      await back(tester);

      expect(notes.all().single.text, 'water plants');
      expect(lockScreen, findsOneWidget);
    });

    testWidgets('locks again after the re-lock time away, and the browser '
        'goes', (tester) async {
      notes.capture('secret plan');
      await start(tester);
      await tester.enterText(find.byKey(const ValueKey('lockPin')), pin);
      await tap(tester, find.byKey(const ValueKey('unlockPin')));
      await tap(tester, find.byTooltip(en.browseTooltip));
      await tap(tester, find.byTooltip(en.allNotesTitle));
      expect(find.text('secret plan'), findsOneWidget);

      await away(tester, const Duration(seconds: 30));
      expect(lockScreen, findsNothing, reason: 'the default is 1 min');
      expect(find.text('secret plan'), findsOneWidget);

      await away(tester, const Duration(minutes: 2));
      expect(lockScreen, findsOneWidget);
      expect(find.byType(AllNotesScreen, skipOffstage: false), findsNothing);
      expect(find.byType(AreasScreen, skipOffstage: false), findsNothing);
      expect(find.text('secret plan'), findsNothing);

      await tester.enterText(find.byKey(const ValueKey('lockPin')), pin);
      await tap(tester, find.byKey(const ValueKey('unlockPin')));
      expect(find.byType(AgendaScreen), findsOneWidget);
    });

    testWidgets('locks at once when the Device says so', (tester) async {
      store.relockAfter = Duration.zero;
      await start(tester);
      await tester.enterText(find.byKey(const ValueKey('lockPin')), pin);
      await tap(tester, find.byKey(const ValueKey('unlockPin')));

      await away(tester, Duration.zero);
      expect(lockScreen, findsOneWidget);
    });

    testWidgets('keeps Capture open when it locks again meanwhile', (
      tester,
    ) async {
      await start(tester);
      await tester.enterText(find.byKey(const ValueKey('lockPin')), pin);
      await tap(tester, find.byKey(const ValueKey('unlockPin')));
      await tap(tester, find.byTooltip(en.browseTooltip));
      await tap(tester, find.byTooltip(en.allNotesTitle));
      await back(tester);
      await back(tester);
      await tap(tester, find.byTooltip(en.captureTooltip));
      await tester.enterText(find.byType(TextField), 'long thought');

      await away(tester, const Duration(minutes: 5));
      expect(find.byType(CaptureScreen), findsOneWidget);

      await back(tester);
      expect(lockScreen, findsOneWidget);
      expect(notes.all().single.text, 'long thought');
    });

    testWidgets('keeps a share open in Capture over the browser when it '
        'locks again, and the browser goes', (tester) async {
      notes.capture('secret plan');
      await start(tester);
      await tester.enterText(find.byKey(const ValueKey('lockPin')), pin);
      await tap(tester, find.byKey(const ValueKey('unlockPin')));
      await tap(tester, find.byTooltip(en.browseTooltip));
      await tap(tester, find.byTooltip(en.allNotesTitle));

      photos.share(const SharedItem(text: 'https://example.org/pills'));
      await tester.pumpAndSettle();
      expect(find.byType(CaptureScreen), findsOneWidget);

      await away(tester, const Duration(minutes: 5));
      expect(find.byType(CaptureScreen), findsOneWidget);
      expect(find.byType(AllNotesScreen, skipOffstage: false), findsNothing);
      expect(find.byType(AreasScreen, skipOffstage: false), findsNothing);

      await back(tester);
      expect(lockScreen, findsOneWidget);
      expect(find.text('secret plan'), findsNothing);
      expect(
        notes.all().map((n) => n.text),
        contains('https://example.org/pills'),
      );
    });

    testWidgets('keeps the full-screen alarm over the lock', (tester) async {
      final routine = routines.create('Pills', daily(10));
      await start(tester);

      alarms.ring(
        Ringing(
          routineId: routine.id,
          occurrenceKey: Occurrence(Day(2026, 10, 24), ClockTime(10, 0)).key,
          title: 'Pills',
          body: '10:00',
        ),
      );
      await tester.pumpAndSettle();
      expect(find.byType(RingScreen), findsOneWidget);

      await tap(tester, find.byKey(const ValueKey('ringDone')));
      expect(routines.byId(routine.id)?.done, isNotEmpty);
      expect(lockScreen, findsOneWidget);
    });

    testWidgets('applies Done from a notification while locked', (
      tester,
    ) async {
      final routine = routines.create('Pills', daily(10));
      await start(tester);

      final occurrence = Occurrence(Day(2026, 10, 24), ClockTime(10, 0));
      alarms.tap(DoneTapped(1, routine.id, occurrence.key, now.toUtc()));
      await tester.pumpAndSettle();

      expect(routines.byId(routine.id)?.doneAt(occurrence), isNotNull);
      expect(lockScreen, findsOneWidget);
    });

    testWidgets('opens the Nudge screen only after the unlock', (tester) async {
      notes.capture('nees mor sauce');
      await start(tester);

      alarms.tapNudge();
      await tester.pumpAndSettle();
      expect(find.byType(NudgeScreen), findsNothing);
      expect(lockScreen, findsOneWidget);

      await tester.enterText(find.byKey(const ValueKey('lockPin')), pin);
      await tap(tester, find.byKey(const ValueKey('unlockPin')));
      expect(find.byType(NudgeScreen), findsOneWidget);
    });
  });

  group('an Area Lock', () {
    testWidgets('keeps the Area closed until the PIN is given', (tester) async {
      lockedDoctor();
      await start(tester);
      await tap(tester, find.byTooltip(en.browseTooltip));

      expect(find.byIcon(Icons.lock_outline), findsOneWidget);
      await tap(tester, find.text('Doctor'));
      expect(find.byKey(const ValueKey('pinField')), findsOneWidget);
      await enterPin(tester, '1357');
      expect(find.text(en.pinWrong), findsOneWidget);
      await tap(tester, find.widgetWithText(TextButton, en.cancel));
      expect(find.byType(AreaScreen), findsNothing);

      await tap(tester, find.text('Doctor'));
      await enterPin(tester, pin);
      expect(find.byType(AreaScreen), findsOneWidget);
      expect(find.text('Pills'), findsOneWidget);
    });

    testWidgets('asks for the PIN before its menu does anything', (
      tester,
    ) async {
      lockedDoctor();
      await start(tester);
      await tap(tester, find.byTooltip(en.browseTooltip));

      await tap(tester, find.byIcon(Icons.more_vert));
      await tap(tester, find.text(en.rename));
      expect(find.byKey(const ValueKey('pinField')), findsOneWidget);
      await tap(tester, find.widgetWithText(TextButton, en.cancel));
      expect(find.text(en.rename), findsNothing, reason: 'no name asked');

      await tap(tester, find.byIcon(Icons.more_vert));
      await tap(tester, find.text(en.rename));
      await enterPin(tester, pin);
      expect(find.byKey(const ValueKey('pinField')), findsNothing);
      expect(find.byType(AlertDialog), findsOneWidget, reason: 'the name');
    });

    testWidgets('opens with biometrics where they are on', (tester) async {
      lockedDoctor();
      biometrics.can = true;
      store.biometricsOn = true;
      await start(tester);
      await tap(tester, find.byTooltip(en.browseTooltip));

      await tap(tester, find.text('Doctor'));
      expect(biometrics.asked, [en.unlockArea('Doctor')]);
      expect(find.byType(AreaScreen), findsOneWidget);
    });

    testWidgets('keeps its Notes out of search and All Notes until opened, '
        'and closes back on the Agenda', (tester) async {
      lockedDoctor();
      notes.capture('Pills for the cat');
      await start(tester);
      await tap(tester, find.byTooltip(en.browseTooltip));

      await tester.enterText(find.byType(TextField), 'Pill');
      await tester.pumpAndSettle();
      expect(find.text('Pills for the cat'), findsOneWidget);
      expect(find.text('Pills'), findsNothing);

      await tap(tester, find.byTooltip(en.allNotesTitle));
      expect(find.text('Pills for the cat'), findsOneWidget);
      expect(find.text('Pills'), findsNothing);
      await back(tester);

      await tester.enterText(find.byType(TextField), '');
      await tester.pumpAndSettle();
      await tap(tester, find.text('Doctor'));
      await enterPin(tester, pin);
      await back(tester);
      await tap(tester, find.byTooltip(en.allNotesTitle));
      expect(find.text('Pills'), findsOneWidget, reason: 'open now');
      await back(tester);
      await back(tester);

      // Back on the Agenda: closed again.
      await tap(tester, find.byTooltip(en.browseTooltip));
      await tap(tester, find.byTooltip(en.allNotesTitle));
      expect(find.text('Pills'), findsNothing);
    });

    testWidgets('shows a Masked Occurrence on the Agenda without its title, '
        'checkable without unlocking', (tester) async {
      final (:routine, doctor: _, note: _) = lockedDoctor();
      await startWeek(tester);

      expect(find.text('Pills'), findsNothing);
      expect(
        find.text(en.maskedRoutineAt('10:00')),
        findsNWidgets(AgendaScreen.days),
      );

      await tap(tester, find.byType(Checkbox).first);
      expect(find.byKey(const ValueKey('pinField')), findsNothing);
      final occurrence = Occurrence(Day(2026, 10, 24), ClockTime(10, 0));
      expect(routines.byId(routine.id)?.doneAt(occurrence), isNotNull);
    });

    testWidgets('asks for the PIN before a Masked row opens its Routine', (
      tester,
    ) async {
      lockedDoctor();
      await start(tester);

      await tap(tester, find.text(en.maskedRoutineAt('10:00')).first);
      expect(find.byType(RoutineEditorScreen), findsNothing);
      await enterPin(tester, pin);
      expect(find.byType(RoutineEditorScreen), findsOneWidget);
    });

    testWidgets('shows a Reminder Masked as "Private Reminder"', (
      tester,
    ) async {
      final doctor = places.addArea('Doctor');
      final note = notes.capture('Blood test', place: doctor.id);
      routines.spawn(
        note.id,
        Schedule(start: Day(2026, 10, 24), times: [ClockTime(11, 0)]),
      );
      store.setAreaLocked(doctor.id, true);
      await start(tester);

      expect(find.text('Blood test'), findsNothing);
      expect(find.text(en.maskedReminder), findsOneWidget);
      expect({for (final a in alarms.armed) a.title}, {en.maskedReminder});
    });

    testWidgets('shows its Occurrences in full when the Area says so', (
      tester,
    ) async {
      lockedDoctor(masked: false);
      await startWeek(tester);

      expect(find.text('Pills'), findsNWidgets(AgendaScreen.days));
      expect(alarms.armed.map((a) => a.title), everyElement('Pills'));
    });

    testWidgets('arms Masked alarms, for the notification and the ring '
        'screen', (tester) async {
      lockedDoctor();
      routines.create('Water plants', daily(12));
      await start(tester);

      final titles = {for (final a in alarms.armed) a.title};
      expect(titles, {en.maskedRoutineAt('10:00'), 'Water plants'});
    });

    testWidgets('shows a Masked Missed Occurrence, checkable', (tester) async {
      // Made two days ago: yesterday's 10:00 was Missed.
      now = DateTime(2026, 10, 22, 9);
      final (:routine, doctor: _, note: _) = lockedDoctor();
      now = DateTime(2026, 10, 24, 9);
      await start(tester);

      final card = find.byType(MissedSummaryCard);
      expect(card, findsOneWidget);
      expect(
        find.descendant(of: card, matching: find.textContaining('Pills')),
        findsNothing,
      );
      expect(
        find.descendant(
          of: card,
          matching: find.textContaining(en.maskedRoutine),
        ),
        findsOneWidget,
      );
      await tap(
        tester,
        find.descendant(of: card, matching: find.byType(Checkbox)).first,
      );
      expect(find.byKey(const ValueKey('pinField')), findsNothing);
      expect(routines.byId(routine.id)?.done, isNotEmpty);
    });

    testWidgets('masks the Agenda widget', (tester) async {
      lockedDoctor();
      routines.create('Water plants', daily(12));
      final snapshot = AgendaSnapshot.of(store, deviceWallClock, en, 'en');
      expect(
        [for (final row in snapshot.days['2026-10-24'] ?? []) row.title],
        [en.maskedRoutineAt('10:00'), 'Water plants'],
      );
    });

    testWidgets('masks the calendar mirror', (tester) async {
      final mirror = FakeMirror(allowed: true);
      store.mirrorCalendar = (id: '1', name: 'Personal');
      lockedDoctor();
      routines.create('Water plants', daily(12));
      await start(tester, mirror: mirror);

      final titles = {
        for (final event in mirror.mirroredIn('1').values) event.title,
      };
      expect(titles, {en.maskedRoutine, 'Water plants'});
    });
  });

  group('lists elsewhere leave a closed Area out', () {
    /// [screen] with the app's lock above it, the way the app shows it.
    Future<void> show(WidgetTester tester, Widget screen) async {
      final lock = AppLock(store: store);
      addTearDown(lock.dispose);
      await tester.pumpWidget(
        MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: LockScope(lock: lock, child: screen),
        ),
      );
      await tester.pumpAndSettle();
    }

    testWidgets('the trash, Notes and Routines alike', (tester) async {
      final (:doctor, :note, :routine) = lockedDoctor();
      final other = notes.capture('Blood test', place: doctor.id);
      notes.trash(other.id);
      routines.trash(routine.id);
      notes.trash(notes.capture('Milk').id);
      await show(tester, TrashScreen(notes: notes, places: places));

      expect(find.text('Milk'), findsOneWidget);
      expect(find.text('Blood test'), findsNothing);
      expect(find.textContaining(note.text), findsNothing);
    });

    testWidgets('merging into', (tester) async {
      lockedDoctor();
      notes.capture('Pill box');
      final milk = notes.capture('Milk');
      await show(
        tester,
        MergeIntoScreen(notes: notes, places: places, noteId: milk.id),
      );

      expect(find.text('Pill box'), findsOneWidget);
      expect(find.text('Pills'), findsNothing);
    });

    testWidgets('filing into: the Area takes Notes, its Topics stay hidden', (
      tester,
    ) async {
      final (:doctor, note: _, routine: _) = lockedDoctor();
      places.addTopic(doctor.id, 'Dentist');
      final home = places.addArea('Home');
      places.addTopic(home.id, 'Garden');
      await show(tester, FileIntoScreen(places: places));

      expect(find.text('Doctor'), findsOneWidget);
      expect(find.text('Dentist'), findsNothing);
      expect(find.text('Garden'), findsOneWidget);
    });

    testWidgets('the calendar export, and so its file', (tester) async {
      lockedDoctor();
      routines.create('Water plants', daily(12));
      await show(tester, CalendarExportScreen(notes: notes));

      expect(find.text('Water plants'), findsOneWidget);
      expect(find.textContaining('Pills'), findsNothing);
    });
  });

  group('Settings', () {
    Future<void> openSettings(WidgetTester tester) async {
      await openDeviceSettings(tester);
    }

    testWidgets('turn the App Lock on with a new PIN, and off only with '
        'the PIN', (tester) async {
      // No PIN yet on this Device.
      store.setLocalSetting('lockPin', '');
      await start(tester);
      await openSettings(tester);

      final appLock = find.byKey(const ValueKey('appLock'));
      await tap(tester, appLock);
      await tester.enterText(find.byKey(const ValueKey('newPin')), '1357');
      await tester.enterText(find.byKey(const ValueKey('newPinAgain')), '1358');
      await tap(tester, find.widgetWithText(TextButton, en.save));
      expect(find.text(en.pinMismatch), findsOneWidget);
      await tester.enterText(find.byKey(const ValueKey('newPinAgain')), '1357');
      await tap(tester, find.widgetWithText(TextButton, en.save));
      expect(store.appLockOn, isTrue);
      expect(store.checkPin('1357'), isTrue);
      expect(lockScreen, findsNothing, reason: 'locks next time');

      await tap(tester, appLock);
      await tap(tester, find.widgetWithText(TextButton, en.cancel));
      expect(store.appLockOn, isTrue);
      await tap(tester, appLock);
      await enterPin(tester, '1357');
      expect(store.appLockOn, isFalse);
    });

    testWidgets('lock an Area, and mask or show its alarms', (tester) async {
      final doctor = places.addArea('Doctor');
      routines.spawn(notes.capture('Pills', place: doctor.id).id, daily(10));
      await start(tester);
      expect({for (final a in alarms.armed) a.title}, {'Pills'});
      await openSettings(tester);
      await tap(tester, find.byKey(const ValueKey('areaLocks')));

      await tap(tester, find.byKey(ValueKey('areaLock-${doctor.id}')));
      expect(store.isAreaLocked(doctor.id), isTrue);
      expect(
        {for (final a in alarms.armed) a.title},
        {en.maskedRoutineAt('10:00')},
      );

      // Showing them in full asks for the PIN.
      await tap(tester, find.text(en.areaAlarmsFull));
      await enterPin(tester, pin);
      expect(store.isAreaMasked(doctor.id), isFalse);
      expect({for (final a in alarms.armed) a.title}, {'Pills'});

      await tap(tester, find.byKey(ValueKey('areaLock-${doctor.id}')));
      await enterPin(tester, pin);
      expect(store.isAreaLocked(doctor.id), isFalse);
    });

    testWidgets('offer biometrics only where the Device has them', (
      tester,
    ) async {
      await start(tester);
      await openSettings(tester);
      expect(find.byKey(const ValueKey('biometrics')), findsNothing);
    });

    testWidgets('switch biometrics on with the PIN, once they work', (
      tester,
    ) async {
      biometrics.can = true;
      biometrics.answer = false;
      await start(tester);
      await openSettings(tester);

      final toggle = find.byKey(const ValueKey('biometrics'));
      await tap(tester, toggle);
      await enterPin(tester, pin);
      expect(store.biometricsOn, isFalse, reason: 'they failed');
      biometrics.answer = true;
      await tap(tester, toggle);
      await enterPin(tester, pin);
      expect(store.biometricsOn, isTrue);
    });

    testWidgets('never let the screen lock alone in: switching biometrics '
        'on asks for the PIN first', (tester) async {
      lockedDoctor();
      biometrics.can = true;
      await start(tester);
      await openSettings(tester);

      await tap(tester, find.byKey(const ValueKey('biometrics')));
      expect(find.byKey(const ValueKey('pinField')), findsOneWidget);
      expect(biometrics.asked, isEmpty);
      await tap(tester, find.widgetWithText(TextButton, en.cancel));
      expect(store.biometricsOn, isFalse);
    });
  });
}
