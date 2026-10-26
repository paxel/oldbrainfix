import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:intl/date_symbol_data_local.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/schedule_text.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

void main() {
  final en = lookupAppLocalizations(const Locale('en'));
  final de = lookupAppLocalizations(const Locale('de'));
  final start = Day(2026, 10, 24);

  setUpAll(initializeDateFormatting);

  /// How [rule] repeats from [start], in English and German.
  (String, String) words(String rule) {
    final schedule = Schedule(start: start, rule: Recurrence.parse(rule));
    return (repeatText(en, 'en', schedule), repeatText(de, 'de', schedule));
  }

  test('a day of the month', () {
    expect(words('FREQ=MONTHLY;BYMONTHDAY=15'), (
      'Every month on day 15',
      'Jeden Monat am 15.',
    ));
    expect(words('FREQ=MONTHLY;INTERVAL=3;BYMONTHDAY=1'), (
      'Every 3 months on day 1',
      'Alle 3 Monate am 1.',
    ));
  });

  test('the Nth or the last weekday of the month', () {
    expect(words('FREQ=MONTHLY;BYDAY=2TU'), (
      'Every month on the second Tuesday',
      'Jeden Monat am zweiten Dienstag',
    ));
    expect(words('FREQ=MONTHLY;INTERVAL=2;BYDAY=-1FR'), (
      'Every 2 months on the last Friday',
      'Alle 2 Monate am letzten Freitag',
    ));
  });

  test('an end on a day or after a number of times', () {
    expect(words('FREQ=DAILY;UNTIL=20261231'), (
      'Every day, until Thu, 12/31/2026',
      'Jeden Tag, bis Do., 31.12.2026',
    ));
    expect(words('FREQ=MONTHLY;COUNT=10;BYDAY=1MO'), (
      'Every month on the first Monday, 10 times',
      'Jeden Monat am ersten Montag, 10-mal',
    ));
    expect(words('FREQ=WEEKLY;COUNT=1;BYDAY=MO'), (
      'Every Mon, once',
      'Immer Mo, einmal',
    ));
  });

  test('a rule beyond the simple editor shows as it is', () {
    expect(words('FREQ=MONTHLY;BYDAY=1MO,-1FR'), (
      'Rule: FREQ=MONTHLY;BYDAY=1MO,-1FR',
      'Regel: FREQ=MONTHLY;BYDAY=1MO,-1FR',
    ));
  });
}
