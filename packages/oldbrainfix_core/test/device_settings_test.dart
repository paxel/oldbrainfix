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

  test('the alarm setup is offered until this Device has walked it', () {
    final store = Store.inMemory();
    addTearDown(store.close);
    expect(store.alarmSetupDone, isFalse);
    store.alarmSetupDone = true;
    expect(store.alarmSetupDone, isTrue);
    expect(store.allEntries(), isEmpty, reason: 'settings are never logged');
  });
}
