import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';

/// Display-time translation of the Starter Set (Catlog's ADR 0005): a
/// planted Area shows in the device language while its stored name is
/// still the canonical English one. Once the user renamed it, or when the
/// user made it, the typed name shows, untranslated.
String areaName(AppLocalizations t, Area area) =>
    _shown(t, area.starter, area.name);

/// Like [areaName], for a Topic.
String topicName(AppLocalizations t, Topic topic) =>
    _shown(t, topic.starter, topic.name);

/// [areaName] for a slug and name of the Starter Set itself.
String starterName(AppLocalizations t, StarterPlace place) =>
    _shown(t, place.slug, place.name);

String _shown(AppLocalizations t, String? slug, String name) {
  if (slug == null || name != _canonical[slug]) return name;
  return _translated(t, slug) ?? name;
}

final _canonical = {
  for (final area in starterSet) ...{
    area.slug: area.name,
    for (final topic in area.topics) topic.slug: topic.name,
  },
};

String? _translated(AppLocalizations t, String slug) => switch (slug) {
  'household' => t.starterHousehold,
  'kitchen' => t.starterKitchen,
  'garden' => t.starterGarden,
  'cleaning' => t.starterCleaning,
  'work' => t.starterWork,
  'meetings' => t.starterMeetings,
  'to-sort-out' => t.starterToSortOut,
  'car' => t.starterCar,
  'service' => t.starterService,
  'repairs' => t.starterRepairs,
  'health' => t.starterHealth,
  'doctor' => t.starterDoctor,
  'medication' => t.starterMedication,
  _ => null,
};
