import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/all_routines_screen.dart';
import 'package:oldbrainfix/src/app_lock.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/share_area_screen.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// Values a newer version of the app saved in a form this one does not
/// know show so, where the value would show, and nothing here changes
/// them (ADR 0034).
void main() {
  late Store store;
  late Notes notes;
  late Places places;
  final en = lookupAppLocalizations(const Locale('en'));

  setUp(() {
    store = Store.inMemory(clock: () => DateTime(2026, 10, 24, 9))
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    notes = Notes(store);
    places = Places(store);
  });
  tearDown(() => store.close());

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

  /// A Routine of [noteId] whose schedule a later version wrote.
  String later(String noteId) {
    const id = 'routine:later';
    store.batch(() {
      store.append(id, RoutineKeys.type, RoutineKeys.typeRoutine);
      store.append(id, RoutineKeys.note, noteId);
      store.append(id, RoutineKeys.start, '2026-10-24');
      store.append(id, RoutineKeys.times, '08:00');
      store.append(id, RoutineKeys.scheduleVersion, '2');
    });
    return id;
  }

  testWidgets('a Routine of a later version is listed saying so, and its '
      'card opens its Note', (tester) async {
    final pills = notes.capture('Take the pills');
    final id = later(pills.id);
    Routines(store).create('Water plants', Schedule(start: Day(2026, 10, 24)));
    await show(tester, AllRoutinesScreen(notes: notes, places: places));

    final card = find.byKey(ValueKey('routine-$id'));
    expect(card, findsOneWidget);
    expect(
      find.descendant(of: card, matching: find.text('Take the pills')),
      findsOneWidget,
    );
    expect(
      find.descendant(of: card, matching: find.text(en.newerVersionNeeded)),
      findsOneWidget,
    );
    expect(find.text('Water plants'), findsOneWidget);
    await tester.tap(card);
    await tester.pumpAndSettle();
    expect(find.byType(NoteEditorScreen), findsOneWidget);
    expect(find.byKey(ValueKey('newer-$id')), findsOneWidget);
  });

  testWidgets('a person whose role a later version gave shows saying so, '
      'with nothing to change', (tester) async {
    final area = store.share(places.addArea('Household').id);
    store.append(
      area,
      ShareKeys.role('ben'),
      '{"profile":"ben","role":"viewer","v":2}',
    );
    store.append(
      area,
      ShareKeys.role('cid'),
      Grant(AreaRole.member, 'cid').encode(),
    );
    await show(tester, PeopleScreen(store: store, areaId: area));

    final ben = find.byKey(const ValueKey('person-ben'));
    expect(
      find.descendant(of: ben, matching: find.text(en.newerVersionNeeded)),
      findsOneWidget,
    );
    expect(find.byKey(const ValueKey('person-menu-ben')), findsNothing);
    expect(find.byKey(const ValueKey('person-menu-cid')), findsOneWidget);
  });
}
