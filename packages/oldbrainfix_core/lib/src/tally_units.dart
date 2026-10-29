/// The units of a Tally's fields and the numbers they hold (ADR 0028).
///
/// A known unit is stored in metric, in the base unit of its kind
/// (kilograms, metres, km/h, °C, degrees, seconds), and shown in the
/// Device's system; a typed unit is kept and shown as typed. Every number
/// in the log is a whole count of millionths of its unit, written as a
/// plain decimal, so both cores store, sum and compare the same numbers
/// without a floating-point text in between.
library;

/// What a unit measures; [typed] for a unit the user typed.
enum UnitKind { mass, distance, speed, temperature, angle, time, typed }

/// How a Device shows known units (ADR 0008).
enum UnitSystem { metric, imperial }

/// How a field counts a day with several entries: their sum, or the
/// largest of them.
enum TallyRule { sum, max }

/// A unit of the known list.
class KnownUnit {
  /// The unit as stored and shown in metric, e.g. `kg`.
  final String code;
  final UnitKind kind;

  /// How many base units of [kind] one of it is.
  final double factor;

  /// The unit shown in imperial, e.g. `lb`; [code] when both systems
  /// share it.
  final String imperial;

  /// How many base units one [imperial] is.
  final double imperialFactor;

  const KnownUnit(
    this.code,
    this.kind,
    this.factor, [
    String? imperial,
    double? imperialFactor,
  ]) : imperial = imperial ?? code,
       imperialFactor = imperialFactor ?? factor;
}

/// Converting, showing and reading the numbers of a Tally's fields.
abstract final class TallyUnits {
  /// The known units, in the order the field page offers them.
  static const known = [
    KnownUnit('kg', UnitKind.mass, 1, 'lb', 0.45359237),
    KnownUnit('g', UnitKind.mass, 0.001, 'oz', 0.028349523125),
    KnownUnit('km', UnitKind.distance, 1000, 'mi', 1609.344),
    KnownUnit('m', UnitKind.distance, 1, 'ft', 0.3048),
    KnownUnit('cm', UnitKind.distance, 0.01, 'in', 0.0254),
    KnownUnit('km/h', UnitKind.speed, 1, 'mph', 1.609344),
    KnownUnit('°C', UnitKind.temperature, 1, '°F', 1),
    KnownUnit('°', UnitKind.angle, 1),
    KnownUnit('h', UnitKind.time, 3600),
    KnownUnit('min', UnitKind.time, 60),
    KnownUnit('s', UnitKind.time, 1),
  ];

  /// The regions whose Devices show imperial units unless set otherwise.
  static const imperialRegions = {'US', 'LR', 'MM'};

  /// The largest number a field holds, in its base unit, either way.
  static const maxValue = 1e9;

  /// The known unit [unit] is, or null for a typed one.
  static KnownUnit? byCode(String unit) {
    for (final candidate in known) {
      if (candidate.code == unit) return candidate;
    }
    return null;
  }

  /// What [unit] measures.
  static UnitKind kindOf(String unit) => byCode(unit)?.kind ?? UnitKind.typed;

  /// The rule a new field with a unit of [kind] gets: the largest for
  /// what is lifted, run or measured at once, the sum for the rest.
  static TallyRule defaultRule(UnitKind kind) => switch (kind) {
    UnitKind.mass ||
    UnitKind.speed ||
    UnitKind.temperature ||
    UnitKind.angle => TallyRule.max,
    UnitKind.distance || UnitKind.time || UnitKind.typed => TallyRule.sum,
  };

  /// The system a Device in [region] (an ISO country code) shows unless
  /// it was set otherwise.
  static UnitSystem systemForRegion(String? region) =>
      imperialRegions.contains(region?.toUpperCase())
      ? UnitSystem.imperial
      : UnitSystem.metric;

  /// The unit [unit] is shown as in [system].
  static String shownUnit(String unit, UnitSystem system) {
    final known = byCode(unit);
    if (known == null || system == UnitSystem.metric) return unit;
    return known.imperial;
  }

  /// [shown], as entered in [unit] shown in [system], in the base unit it
  /// is stored in.
  static double toBase(double shown, String unit, UnitSystem system) {
    final known = byCode(unit);
    if (known == null) return shown;
    if (system == UnitSystem.metric) return shown * known.factor;
    if (known.kind == UnitKind.temperature) return (shown - 32) * 5 / 9;
    return shown * known.imperialFactor;
  }

  /// [base], stored in the base unit of [unit], as shown in [unit] in
  /// [system].
  static double fromBase(double base, String unit, UnitSystem system) {
    final known = byCode(unit);
    if (known == null) return base;
    if (system == UnitSystem.metric) return base / known.factor;
    if (known.kind == UnitKind.temperature) return base * 9 / 5 + 32;
    return base / known.imperialFactor;
  }

  /// [base] as a field of [unit] shows it in [system], without the unit:
  /// a time as `m:ss` or `h:mm:ss`, anything else with up to two
  /// decimals, [decimal] between them.
  static String formatShown(
    double base,
    String unit,
    UnitSystem system, {
    String decimal = '.',
  }) {
    if (!base.isFinite) return '';
    if (kindOf(unit) == UnitKind.time) {
      final seconds = base.round();
      final sign = seconds < 0 ? '-' : '';
      final s = seconds.abs();
      final h = s ~/ 3600;
      final m = s % 3600 ~/ 60;
      final ss = '${s % 60}'.padLeft(2, '0');
      return h > 0 ? '$sign$h:${'$m'.padLeft(2, '0')}:$ss' : '$sign$m:$ss';
    }
    final shown = fromBase(base, unit, system);
    if (!shown.isFinite || shown.abs() > _largestShown) return '';
    return _decimal((shown * 100).round(), 2).replaceFirst('.', decimal);
  }

  /// The number [text], typed into a field of [unit] shown in [system],
  /// in its base unit; null when it is no number or out of range. A
  /// decimal comma counts as a point; a time also reads as `m:ss` or
  /// `h:mm:ss`.
  static double? parseShown(String text, String unit, UnitSystem system) {
    final typed = text.trim();
    if (kindOf(unit) == UnitKind.time && typed.contains(':')) {
      final match = _clock.firstMatch(typed);
      if (match == null) return null;
      final parts = [
        for (final part in typed.substring(match[1]!.length).split(':'))
          int.parse(part),
      ];
      if (parts.skip(1).any((p) => p > 59)) return null;
      final seconds = parts.fold(0, (s, p) => s * 60 + p);
      final base = (match[1]!.isEmpty ? seconds : -seconds).toDouble();
      return base.abs() > maxValue ? null : base;
    }
    if (!_number.hasMatch(typed)) return null;
    final shown = double.parse(typed.replaceFirst(',', '.'));
    final base = toBase(shown, unit, system);
    return base.isFinite && base.abs() <= maxValue ? base : null;
  }

  /// Beyond any number a field holds in any unit: what is shown as
  /// nothing.
  static const _largestShown = 1e13;

  /// `-` or nothing, then `m:ss` or `h:mm:ss`.
  static final _clock = RegExp(r'^(-?)\d{1,6}(:\d{2}){1,2}$');

  /// A number as typed: digits, a point or a comma and more digits.
  static final _number = RegExp(r'^-?\d+([.,]\d+)?$');
}

/// [value] as the log keeps it: millionths, rounded half away from zero,
/// written as a plain decimal (`8046.72`, `-3.5`, `20`). Throws for a
/// number that is not finite or beyond [TallyUnits.maxValue].
String encodeTallyNumber(double value) {
  final micro = tallyMicro(value);
  if (micro == null) {
    throw ArgumentError.value(value, 'value', 'not a number a field holds');
  }
  return _decimal(micro, 6);
}

/// The number [text] holds in millionths, as [encodeTallyNumber] wrote
/// it; null for anything else.
int? decodeTallyMicro(String? text) {
  if (text == null) return null;
  final match = _stored.firstMatch(text);
  if (match == null) return null;
  final whole = int.parse(match[2]!);
  final fraction = int.parse((match[3] ?? '').padRight(6, '0'));
  final micro = whole * 1000000 + fraction;
  if (micro > _maxMicro) return null;
  return match[1]!.isEmpty ? micro : -micro;
}

/// The number [text] holds, as [encodeTallyNumber] wrote it; null for
/// anything else.
double? decodeTallyNumber(String? text) {
  final micro = decodeTallyMicro(text);
  return micro == null ? null : micro / 1000000;
}

/// [value] in millionths, rounded half away from zero; null when it is
/// not finite or beyond [TallyUnits.maxValue].
int? tallyMicro(double value) {
  if (!value.isFinite || value.abs() > TallyUnits.maxValue) return null;
  return (value * 1000000).round();
}

/// [TallyUnits.maxValue] in millionths.
const _maxMicro = 1000000000000000;

/// A number written by [encodeTallyNumber]: a sign, at most ten whole
/// digits without leading zeros, and up to six decimals not ending in 0.
final _stored = RegExp(r'^(-?)(0|[1-9]\d{0,9})(?:\.(\d{0,5}[1-9]))?$');

/// [scaled], a count of 10^-[places], as a plain decimal without
/// trailing zeros.
String _decimal(int scaled, int places) {
  final sign = scaled < 0 ? '-' : '';
  final digits = '${scaled.abs()}'.padLeft(places + 1, '0');
  final whole = digits.substring(0, digits.length - places);
  final fraction = digits
      .substring(digits.length - places)
      .replaceFirst(RegExp(r'0+$'), '');
  final text = fraction.isEmpty ? whole : '$whole.$fraction';
  return text == '0' ? '0' : '$sign$text';
}
