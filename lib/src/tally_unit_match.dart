import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';

/// The known units a Tally field's unit [typed] so far may mean (#159),
/// in the order of [TallyUnits.known]: those whose symbol in [system],
/// or one of whose [names] in it, starts with it, ignoring case and the
/// spaces around it; every one for nothing typed. Only units of [kind]
/// when given, so none for [UnitKind.typed].
List<KnownUnit> matchingUnits(
  String typed,
  UnitSystem system,
  Map<String, List<String>> names, {
  UnitKind? kind,
}) {
  final start = typed.trim().toLowerCase();
  return [
    for (final known in TallyUnits.known)
      if (kind == null || known.kind == kind)
        if ([
          TallyUnits.shownUnit(known.code, system),
          ...?names[known.code],
        ].any((name) => name.toLowerCase().startsWith(start)))
          known,
  ];
}

/// The unit a Tally field's unit typed as [typed] is: a known code as
/// typed (ADR 0028), the known unit whose symbol in [system] it is, else
/// the text itself, a unit of the user's own; nothing for no unit.
String unitOfTyped(String typed, UnitSystem system) {
  final text = typed.trim();
  if (TallyUnits.byCode(text) != null) return text;
  for (final known in TallyUnits.known) {
    if (TallyUnits.shownUnit(known.code, system) == text) return known.code;
  }
  return text;
}

/// The names of each known unit in [system] that [matchingUnits] goes
/// by, in every language of [languages], by its code.
Map<String, List<String>> unitNames(
  Iterable<AppLocalizations> languages,
  UnitSystem system,
) => {
  for (final known in TallyUnits.known)
    known.code: [
      for (final t in languages)
        for (final name in _names(t, known.code, system).split(','))
          if (name.trim().isNotEmpty) name.trim(),
    ],
};

/// The names of the known unit [code] in [system], separated by commas.
String _names(AppLocalizations t, String code, UnitSystem system) {
  final imperial = system == UnitSystem.imperial;
  return switch (code) {
    'kg' => imperial ? t.unitNamesPound : t.unitNamesKg,
    'g' => imperial ? t.unitNamesOunce : t.unitNamesG,
    'km' => imperial ? t.unitNamesMile : t.unitNamesKm,
    'm' => imperial ? t.unitNamesFoot : t.unitNamesM,
    'cm' => imperial ? t.unitNamesInch : t.unitNamesCm,
    'km/h' => imperial ? t.unitNamesMph : t.unitNamesKmh,
    '°C' => imperial ? t.unitNamesFahrenheit : t.unitNamesCelsius,
    '°' => t.unitNamesDegree,
    'h' => t.unitNamesHour,
    'min' => t.unitNamesMinute,
    's' => t.unitNamesSecond,
    _ => '',
  };
}
