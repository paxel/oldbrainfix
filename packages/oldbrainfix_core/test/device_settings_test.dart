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
}
