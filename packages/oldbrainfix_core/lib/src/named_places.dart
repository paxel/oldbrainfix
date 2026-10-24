import 'dart:math';

import 'store.dart';

/// Field keys of a Named Place.
abstract final class NamedPlaceKeys {
  static const type = r'$type';
  static const typeNamedPlace = 'namedPlace';
  static const name = 'name';

  /// The centre, as [Coordinates.encode] writes it.
  static const centre = 'centre';

  /// How far around the centre still counts, in metres.
  static const radius = 'radius';
}

/// A point on Earth in degrees, as the platform reports it.
class Coordinates {
  final double latitude;
  final double longitude;

  Coordinates(this.latitude, this.longitude) {
    if (!latitude.isFinite || latitude < -90 || latitude > 90) {
      throw ArgumentError.value(latitude, 'latitude', 'not within ±90°');
    }
    if (!longitude.isFinite || longitude < -180 || longitude > 180) {
      throw ArgumentError.value(longitude, 'longitude', 'not within ±180°');
    }
  }

  /// The stored form, `<latitude>,<longitude>`.
  String encode() => '$latitude,$longitude';

  /// Reads what [encode] wrote; null for anything else.
  static Coordinates? decode(String? value) {
    final parts = value?.split(',');
    if (parts == null || parts.length != 2) return null;
    final latitude = double.tryParse(parts[0]);
    final longitude = double.tryParse(parts[1]);
    if (latitude == null || longitude == null) return null;
    try {
      return Coordinates(latitude, longitude);
    } on ArgumentError {
      return null;
    }
  }

  @override
  bool operator ==(Object other) =>
      other is Coordinates &&
      other.latitude == latitude &&
      other.longitude == longitude;

  @override
  int get hashCode => Object.hash(latitude, longitude);

  @override
  String toString() => 'Coordinates($latitude, $longitude)';
}

/// The great-circle distance between [a] and [b] in metres (haversine,
/// on a sphere of the Earth's mean radius). Good to a few metres at the
/// distances a Named Place spans.
double distanceInMetres(Coordinates a, Coordinates b) {
  const earthRadius = 6371008.8;
  double rad(double degrees) => degrees * pi / 180;
  final dLat = rad(b.latitude - a.latitude);
  final dLon = rad(b.longitude - a.longitude);
  final h =
      pow(sin(dLat / 2), 2) +
      cos(rad(a.latitude)) * cos(rad(b.latitude)) * pow(sin(dLon / 2), 2);
  return 2 * earthRadius * asin(sqrt(min(1.0, h.toDouble())));
}

/// A spot the user gave a name (CONTEXT.md).
class NamedPlace {
  /// Entity id, `namedplace:<id>`.
  final String id;
  final String name;
  final Coordinates centre;

  /// How far around [centre] still counts, in metres.
  final double radius;

  const NamedPlace({
    required this.id,
    required this.name,
    required this.centre,
    required this.radius,
  });

  /// Whether [point] lies within [radius] of [centre].
  bool contains(Coordinates point) => distanceInMetres(centre, point) <= radius;
}

/// The Named Place among [places] that [point] lies in, or null when it
/// lies in none. Where Named Places overlap, the smallest one wins, as
/// the most precise ("Kitchen" inside "Home"); between equal ones, the
/// nearest centre. Works offline: nothing but arithmetic.
NamedPlace? namedPlaceAt(Iterable<NamedPlace> places, Coordinates point) {
  NamedPlace? best;
  double? bestDistance;
  for (final place in places) {
    final distance = distanceInMetres(place.centre, point);
    if (distance > place.radius) continue;
    final better =
        best == null ||
        place.radius < best.radius ||
        (place.radius == best.radius && distance < (bestDistance ?? 0));
    if (better) {
      best = place;
      bestDistance = distance;
    }
  }
  return best;
}

/// Making, changing and finding Named Places.
class NamedPlaces {
  final Store store;

  NamedPlaces(this.store);

  /// Names the spot [centre], counting everything within [radius]
  /// metres of it.
  NamedPlace add(String name, Coordinates centre, {required double radius}) {
    final trimmed = _name(name);
    _requireRadius(radius);
    final id = 'namedplace:${Store.newId()}';
    store.batch(() {
      store.append(id, NamedPlaceKeys.type, NamedPlaceKeys.typeNamedPlace);
      store.append(id, NamedPlaceKeys.name, trimmed);
      store.append(id, NamedPlaceKeys.centre, centre.encode());
      store.append(id, NamedPlaceKeys.radius, '$radius');
    });
    return byId(id)!;
  }

  /// Renames the Named Place [id].
  void rename(String id, String name) {
    final trimmed = _name(name);
    _require(id);
    store.append(id, NamedPlaceKeys.name, trimmed);
  }

  /// Sets how far around the Named Place [id] still counts, in metres.
  void setRadius(String id, double radius) {
    _requireRadius(radius);
    _require(id);
    store.append(id, NamedPlaceKeys.radius, '$radius');
  }

  /// Deletes the Named Place [id]; Notes captured near it show their
  /// coordinates again.
  void delete(String id) {
    _require(id);
    store.append(id, NamedPlaceKeys.type, null);
  }

  /// The Named Place [id], or null when there is none.
  NamedPlace? byId(String id) {
    final fields = store.currentFields(id);
    if (fields[NamedPlaceKeys.type] != NamedPlaceKeys.typeNamedPlace) {
      return null;
    }
    final centre = Coordinates.decode(fields[NamedPlaceKeys.centre]);
    final radius = double.tryParse(fields[NamedPlaceKeys.radius] ?? '');
    if (centre == null || radius == null) return null;
    return NamedPlace(
      id: id,
      name: fields[NamedPlaceKeys.name] ?? '',
      centre: centre,
      radius: radius,
    );
  }

  /// Every Named Place, by name, ignoring case.
  List<NamedPlace> all() {
    final places = [
      for (final id in store.entitiesOfType(NamedPlaceKeys.typeNamedPlace))
        ?byId(id),
    ];
    places.sort((a, b) => a.name.toLowerCase().compareTo(b.name.toLowerCase()));
    return places;
  }

  /// The Named Place [point] lies in, or null (see [namedPlaceAt]).
  NamedPlace? at(Coordinates point) => namedPlaceAt(all(), point);

  void _require(String id) {
    if (byId(id) == null) {
      throw ArgumentError.value(id, 'id', 'not a Named Place');
    }
  }

  static void _requireRadius(double radius) {
    if (!radius.isFinite || radius <= 0) {
      throw ArgumentError.value(radius, 'radius', 'must be above 0 metres');
    }
  }

  static String _name(String name) {
    final trimmed = name.trim();
    if (trimmed.isEmpty) {
      throw ArgumentError.value(name, 'name', 'must not be empty');
    }
    return trimmed;
  }
}
