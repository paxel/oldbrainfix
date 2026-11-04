import 'package:flutter/widgets.dart' show WidgetsBinding;
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// The system this Device shows the known units of Tallies in: the one
/// chosen in Settings, else the one of the region the system reports
/// (ADR 0028).
UnitSystem deviceUnitSystem(Store store) => store.unitSystem(
  WidgetsBinding.instance.platformDispatcher.locale.countryCode,
);
