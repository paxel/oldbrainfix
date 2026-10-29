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

  test('the Agenda shows no calendar until switched on and picked', () {
    final store = Store.inMemory();
    addTearDown(store.close);
    expect(store.agendaCalendarOn, isFalse);
    expect(store.agendaCalendars, isEmpty);
    store
      ..agendaCalendarOn = true
      ..agendaCalendars = {'7', '3'};
    expect(store.agendaCalendarOn, isTrue);
    expect(store.agendaCalendars, {'3', '7'});
    store.agendaCalendarOn = false;
    expect(store.agendaCalendars, {'3', '7'}, reason: 'kept while off');
    store.setLocalSetting('agendaCalendars', 'not json');
    expect(store.agendaCalendars, isEmpty);
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

  test("Occurrences ring with the app's own sound until one is chosen", () {
    final store = Store.inMemory();
    addTearDown(store.close);
    expect(store.alarmSound, isNull);
    const chosen = 'content://media/internal/audio/media/42';
    store.alarmSound = chosen;
    expect(store.alarmSound, chosen);
    expect(store.localSetting('alarmSound'), chosen);
    store.alarmSound = '  ';
    expect(store.alarmSound, isNull);
    expect(store.localSetting('alarmSound'), isNull);
    store.alarmSound = chosen;
    store.alarmSound = null;
    expect(store.alarmSound, isNull);
    store.setLocalSetting('alarmSound', '');
    expect(store.alarmSound, isNull);
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

  test('recordings play at normal speed until this Device chooses one', () {
    final store = Store.inMemory();
    addTearDown(store.close);
    expect(store.playbackSpeed, 1);
    expect(store.chosenPlaybackSpeed, isNull);
    store.playbackSpeed = 0.75;
    expect(store.playbackSpeed, 0.75);
    expect(store.localSetting('playbackSpeed'), '0.75');
    store.playbackSpeed = 2;
    expect(store.localSetting('playbackSpeed'), '2.0');
    expect(() => store.playbackSpeed = 1.25, throwsArgumentError);
    expect(store.playbackSpeed, 2);
    expect(store.allEntries(), isEmpty, reason: 'settings are never logged');
  });

  test('the playback speed the phone kept before carries over as it is', () {
    final store = Store.inMemory();
    addTearDown(store.close);
    // As the phone's app wrote it in 0.1.1.
    store.setLocalSetting('playbackSpeed', '1.5');
    expect(store.playbackSpeed, 1.5);
    expect(store.chosenPlaybackSpeed, 1.5);
    for (final kept in ['7', 'fast', '1.25', '']) {
      store.setLocalSetting('playbackSpeed', kept);
      expect(store.playbackSpeed, 1, reason: kept);
      expect(store.chosenPlaybackSpeed, isNull, reason: kept);
    }
  });
}
