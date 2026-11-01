import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/tally_unit_match.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

void main() {
  final languages = AppLocalizations.supportedLocales.map(
    lookupAppLocalizations,
  );
  final metric = unitNames(languages, UnitSystem.metric);
  final imperial = unitNames(languages, UnitSystem.imperial);

  List<String> codes(
    String typed, {
    UnitSystem system = UnitSystem.metric,
    UnitKind? kind,
  }) => [
    for (final known in matchingUnits(
      typed,
      system,
      system == UnitSystem.metric ? metric : imperial,
      kind: kind,
    ))
      known.code,
  ];

  group('matching the units typed', () {
    test('by the start of their symbol, ignoring case and spaces', () {
      expect(codes('k'), ['kg', 'km', 'km/h']);
      expect(codes(' KM '), ['km', 'km/h']);
      expect(codes('min'), ['min']);
      expect(codes('°'), ['°C', '°']);
    });

    test('by the start of an English or a German name', () {
      expect(codes('kilo'), ['kg', 'km', 'km/h']);
      expect(codes('minute'), ['min']);
      expect(codes('Stun'), ['km/h', 'h']);
      expect(codes('sekunden'), ['s']);
      expect(codes('celsius'), ['°C']);
      expect(codes('grad'), ['°C', '°']);
    });

    test('in imperial, by the imperial symbols and names', () {
      final system = UnitSystem.imperial;
      expect(codes('lb', system: system), ['kg']);
      expect(codes('pound', system: system), ['kg']);
      expect(codes('Pfund', system: system), ['kg']);
      expect(codes('Fuß', system: system), ['m']);
      expect(codes('mi', system: system), ['km', 'km/h', 'min']);
      expect(codes('kilo', system: system), isEmpty);
    });

    test('every unit for nothing typed, and none for a text of its own', () {
      expect(codes(''), [for (final known in TallyUnits.known) known.code]);
      expect(codes('pages'), isEmpty);
    });

    test('only units of the kind fixed', () {
      expect(codes('', kind: UnitKind.distance), ['km', 'm', 'cm']);
      expect(codes('m', kind: UnitKind.distance), ['m']);
      expect(codes('', kind: UnitKind.typed), isEmpty);
    });
  });

  group('the unit typed', () {
    test('is a known code as typed, or a known symbol of the system', () {
      expect(unitOfTyped('kg', UnitSystem.metric), 'kg');
      expect(unitOfTyped('kg', UnitSystem.imperial), 'kg');
      expect(unitOfTyped(' lb ', UnitSystem.imperial), 'kg');
      expect(unitOfTyped('°F', UnitSystem.imperial), '°C');
    });

    test('else is the text as typed, a unit of the user\'s own', () {
      expect(unitOfTyped(' pages ', UnitSystem.metric), 'pages');
      expect(unitOfTyped('lb', UnitSystem.metric), 'lb');
      expect(unitOfTyped('KG', UnitSystem.metric), 'KG');
      expect(unitOfTyped('', UnitSystem.metric), '');
    });
  });

  test('the names come from every language', () {
    expect(metric['kg'], containsAll(['kilogram', 'Kilogramm']));
    expect(imperial['kg'], containsAll(['pound', 'Pfund']));
    expect(metric.keys, [for (final known in TallyUnits.known) known.code]);
    expect(
      Locale('de'),
      isIn(AppLocalizations.supportedLocales),
      reason: 'the German names are matched',
    );
  });
}
