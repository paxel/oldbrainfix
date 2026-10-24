import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

void main() {
  test('the theme follows the system until the Device overrides it', () {
    final store = Store.inMemory();
    addTearDown(store.close);
    expect(store.themeChoice, ThemeChoice.system);
  });

  test('a theme override is kept per Device and survives', () {
    final store = Store.inMemory();
    addTearDown(store.close);
    store.themeChoice = ThemeChoice.dark;
    expect(store.themeChoice, ThemeChoice.dark);
    expect(store.localSetting('theme'), 'dark');
    store.themeChoice = ThemeChoice.light;
    expect(store.themeChoice, ThemeChoice.light);
    expect(store.allEntries(), isEmpty, reason: 'settings are never logged');
  });

  test('an unknown stored theme reads as following the system', () {
    final store = Store.inMemory();
    addTearDown(store.close);
    store.setLocalSetting('theme', 'purple');
    expect(store.themeChoice, ThemeChoice.system);
  });

  test('the Starter Set is offered until this Device has offered it', () {
    final store = Store.inMemory();
    addTearDown(store.close);
    expect(store.starterSetOffered, isFalse);
    store.starterSetOffered = true;
    expect(store.starterSetOffered, isTrue);
    expect(store.allEntries(), isEmpty, reason: 'settings are never logged');
  });

  test('the location is explained until this Device has explained it', () {
    final store = Store.inMemory();
    addTearDown(store.close);
    expect(store.locationExplained, isFalse);
    store.locationExplained = true;
    expect(store.locationExplained, isTrue);
    expect(store.allEntries(), isEmpty, reason: 'settings are never logged');
  });

  test('calendar and Bluetooth context are off until switched on', () {
    final store = Store.inMemory();
    addTearDown(store.close);
    expect(store.calendarContextOn, isFalse);
    expect(store.bluetoothContextOn, isFalse);
    store
      ..calendarContextOn = true
      ..bluetoothContextOn = true;
    expect(store.calendarContextOn, isTrue);
    expect(store.bluetoothContextOn, isTrue);
    store.calendarContextOn = false;
    expect(store.calendarContextOn, isFalse);
    expect(store.allEntries(), isEmpty, reason: 'settings are never logged');
  });

  test('the alarm setup is offered until this Device has walked it', () {
    final store = Store.inMemory();
    addTearDown(store.close);
    expect(store.alarmSetupDone, isFalse);
    store.alarmSetupDone = true;
    expect(store.alarmSetupDone, isTrue);
    expect(store.allEntries(), isEmpty, reason: 'settings are never logged');
  });

  test('Re-nag every 10 min until this Device sets its own interval', () {
    final store = Store.inMemory();
    addTearDown(store.close);
    expect(store.renagInterval, const Duration(minutes: 10));
    store.renagInterval = const Duration(minutes: 30);
    expect(store.renagInterval, const Duration(minutes: 30));
    expect(() => store.renagInterval = Duration.zero, throwsArgumentError);
    store.setLocalSetting('renagMinutes', 'often');
    expect(store.renagInterval, const Duration(minutes: 10));
    expect(store.allEntries(), isEmpty, reason: 'settings are never logged');
  });

  test('"seen" is kept per Device', () {
    final store = Store.inMemory();
    addTearDown(store.close);
    expect(store.missedSeen, isNull);
    final seen = DateTime.utc(2026, 10, 24, 7);
    store.missedSeen = seen;
    expect(store.missedSeen, seen);
    expect(store.allEntries(), isEmpty, reason: 'settings are never logged');
  });

  test('Streaks show until this Device switches them off', () {
    final store = Store.inMemory();
    addTearDown(store.close);
    expect(store.streaksOn, isTrue);
    store.streaksOn = false;
    expect(store.streaksOn, isFalse);
    store.streaksOn = true;
    expect(store.streaksOn, isTrue);
    expect(store.allEntries(), isEmpty, reason: 'settings are never logged');
  });
}
