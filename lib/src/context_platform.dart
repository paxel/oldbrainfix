import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'l10n.dart';

/// The parts of the Capture Context the user switches on, each behind
/// its own permission.
enum ContextKind { calendar, bluetooth }

/// The calendar and the Bluetooth devices: the platform seam for those
/// parts of the Capture Context. Widget tests hand in a fake.
abstract class ContextSource {
  /// Whether this Device can tell them at all; the switches only show
  /// where it can.
  bool get available;

  /// Whether the user allowed reading [kind]. Never asks.
  Future<bool> granted(ContextKind kind);

  /// Asks the user for [kind]; whether it is allowed now.
  Future<bool> request(ContextKind kind);

  /// The calendar events running now, or null when that is not allowed
  /// or not known. Never asks.
  Future<List<RunningEvent>?> runningEvents();

  /// The names of the Bluetooth devices connected now, or null when that
  /// is not allowed or not known. Never asks.
  Future<List<String>?> connectedDevices();
}

/// No calendar and no Bluetooth: the default where none is handed in.
class NoContext implements ContextSource {
  const NoContext();

  @override
  bool get available => false;

  @override
  Future<bool> granted(ContextKind kind) async => false;

  @override
  Future<bool> request(ContextKind kind) async => false;

  @override
  Future<List<RunningEvent>?> runningEvents() async => null;

  @override
  Future<List<String>?> connectedDevices() async => null;
}

/// Android's calendar provider and Bluetooth profiles behind the app's
/// method channel; no Google library (android/…/CaptureExtras.kt).
class AndroidContext implements ContextSource {
  static const _channel = MethodChannel('io.github.paxel.oldbrainfix/context');

  /// How long a lookup may take before there is none: a Bluetooth
  /// profile may never answer.
  static const lookupLimit = Duration(seconds: 10);

  const AndroidContext();

  @override
  bool get available => true;

  @override
  Future<bool> granted(ContextKind kind) => _ask('granted', kind);

  @override
  Future<bool> request(ContextKind kind) => _ask('request', kind);

  @override
  Future<List<RunningEvent>?> runningEvents() async {
    final rows = await _lookup('calendar');
    if (rows == null) return null;
    return [
      for (final row in rows)
        if (row case {'title': final String title, 'allDay': final bool allDay})
          (title: title, allDay: allDay),
    ];
  }

  @override
  Future<List<String>?> connectedDevices() async {
    final names = await _lookup('bluetooth');
    return names?.whereType<String>().toList();
  }

  static Future<List<Object?>?> _lookup(String method) async {
    try {
      return await _channel
          .invokeListMethod<Object?>(method)
          .timeout(lookupLimit);
    } on Exception {
      return null;
    }
  }

  static Future<bool> _ask(String method, ContextKind kind) async {
    try {
      return await _channel.invokeMethod<bool>(method, kind.name) ?? false;
    } on Exception {
      return false;
    }
  }
}

/// Hands the app's [ContextSource] down to every screen.
class ContextScope extends InheritedWidget {
  final ContextSource source;

  const ContextScope({super.key, required this.source, required super.child});

  /// The app's [ContextSource]; read once, so it does not listen.
  static ContextSource of(BuildContext context) =>
      context.getInheritedWidgetOfExactType<ContextScope>()?.source ??
      const NoContext();

  @override
  bool updateShouldNotify(ContextScope oldWidget) => source != oldWidget.source;
}

/// The calendar events running and the Bluetooth devices connected, as
/// a Note records them.
typedef ContextExtras = ({
  List<String> calendarEvents,
  List<String> bluetoothDevices,
});

/// Whether [kind] is switched on in [store] for this Device.
bool contextOn(Store store, ContextKind kind) => switch (kind) {
  ContextKind.calendar => store.calendarContextOn,
  ContextKind.bluetooth => store.bluetoothContextOn,
};

/// Looks up what is switched on and allowed now; nothing else, and it
/// never asks.
Future<ContextExtras> currentExtras(Store store, ContextSource source) async {
  Future<bool> on(ContextKind kind) async =>
      contextOn(store, kind) && await source.granted(kind);
  final events = on(ContextKind.calendar)
      .then((on) => on ? source.runningEvents() : null);
  final devices = on(ContextKind.bluetooth)
      .then((on) => on ? source.connectedDevices() : null);
  return (
    calendarEvents: runningEventTitles(await events ?? const []),
    bluetoothDevices: {
      for (final name in await devices ?? const <String>[])
        if (name.trim().isNotEmpty) name.trim(),
    }.toList(),
  );
}

/// Attaches [extras], once they arrive, to the Note or Section [id].
/// Returns whether anything was attached; nothing is when there is
/// nothing to tell or the Note is gone meanwhile.
Future<bool> attachExtras(
  Notes notes,
  String id,
  Future<ContextExtras> extras,
) async {
  final (:calendarEvents, :bluetoothDevices) = await extras;
  if (calendarEvents.isEmpty && bluetoothDevices.isEmpty) return false;
  try {
    notes.attachContext(
      id,
      calendarEvents: calendarEvents,
      bluetoothDevices: bluetoothDevices,
    );
    return true;
  } on ArgumentError {
    return false;
  }
}

/// Explains what Notes record of [kind] and that it stays on the user's
/// Devices, then asks for it when the user agrees. Returns whether it is
/// allowed now.
Future<bool> explainAndAskContext(
  BuildContext context,
  ContextSource source,
  ContextKind kind,
) async {
  final agreed = await showDialog<bool>(
    context: context,
    builder: (context) {
      final t = context.t;
      final (icon, title, body) = switch (kind) {
        ContextKind.calendar => (
          Icons.event_outlined,
          t.calendarExplainTitle,
          t.calendarExplainBody,
        ),
        ContextKind.bluetooth => (
          Icons.bluetooth,
          t.bluetoothExplainTitle,
          t.bluetoothExplainBody,
        ),
      };
      return AlertDialog(
        icon: Icon(icon),
        title: Text(title),
        content: Text(body),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: Text(t.notNow),
          ),
          TextButton(
            onPressed: () => Navigator.of(context).pop(true),
            child: Text(t.allowAccess),
          ),
        ],
      );
    },
  );
  if (agreed != true) return false;
  return source.request(kind);
}
