import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'l10n.dart';

/// Where the Device is: the platform seam for the Capture Context's
/// location. Widget tests hand in a fake.
abstract class LocationSource {
  /// Whether this Device can tell where it is at all; the explanation is
  /// only offered where it can.
  bool get available;

  /// Whether the user allowed the location. Never asks.
  Future<bool> granted();

  /// Asks the user for the location; whether it is allowed now.
  Future<bool> request();

  /// Where the Device is now, or null when that is not allowed or not
  /// known. Never asks.
  Future<Coordinates?> current();
}

/// No location at all: the default where none is handed in.
class NoLocation implements LocationSource {
  const NoLocation();

  @override
  bool get available => false;

  @override
  Future<bool> granted() async => false;

  @override
  Future<bool> request() async => false;

  @override
  Future<Coordinates?> current() async => null;
}

/// Android's own LocationManager behind the app's method channel; no
/// Google library (android/…/Location.kt).
class AndroidLocation implements LocationSource {
  static const _channel = MethodChannel('io.github.paxel.oldbrainfix/location');

  /// How long a fix may take before there is none: indoors GPS may never
  /// answer.
  static const fixLimit = Duration(seconds: 30);

  const AndroidLocation();

  @override
  bool get available => true;

  @override
  Future<bool> granted() => _ask('granted');

  @override
  Future<bool> request() => _ask('request');

  @override
  Future<Coordinates?> current() async {
    try {
      final fix = await _channel
          .invokeListMethod<double>('current')
          .timeout(fixLimit);
      if (fix == null || fix.length != 2) return null;
      return Coordinates(fix[0], fix[1]);
    } on Exception {
      return null;
    } on ArgumentError {
      return null;
    }
  }

  static Future<bool> _ask(String method) async {
    try {
      return await _channel.invokeMethod<bool>(method) ?? false;
    } on Exception {
      return false;
    }
  }
}

/// Hands the app's [LocationSource] down to every screen.
class LocationScope extends InheritedWidget {
  final LocationSource location;

  const LocationScope({
    super.key,
    required this.location,
    required super.child,
  });

  /// The app's [LocationSource]; read once, so it does not listen.
  static LocationSource of(BuildContext context) =>
      context.getInheritedWidgetOfExactType<LocationScope>()?.location ??
      const NoLocation();

  @override
  bool updateShouldNotify(LocationScope oldWidget) =>
      location != oldWidget.location;
}

/// Attaches [where], once it arrives, to the Note or Section [id] as the
/// place it was captured. Returns whether it did; nothing is attached
/// when there is no position or the Note is gone meanwhile.
Future<bool> attachLocation(
  Notes notes,
  String id,
  Future<Coordinates?> where,
) async {
  final at = await where;
  if (at == null) return false;
  try {
    notes.locate(id, at);
    return true;
  } on ArgumentError {
    return false;
  }
}

/// Where the Device is now when the location is allowed; null without
/// asking otherwise.
Future<Coordinates?> currentIfGranted(LocationSource location) async =>
    await location.granted() ? location.current() : null;

/// Explains why Notes record where they were captured, then asks for the
/// location when the user agrees. Returns whether it is allowed now.
Future<bool> explainAndAskLocation(
  BuildContext context,
  LocationSource location,
) async {
  final agreed = await showDialog<bool>(
    context: context,
    builder: (context) {
      final t = context.t;
      return AlertDialog(
        icon: const Icon(Icons.place_outlined),
        title: Text(t.locationExplainTitle),
        content: Text(t.locationExplainBody),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: Text(t.notNow),
          ),
          TextButton(
            onPressed: () => Navigator.of(context).pop(true),
            child: Text(t.allowLocation),
          ),
        ],
      );
    },
  );
  if (agreed != true) return false;
  return location.request();
}
