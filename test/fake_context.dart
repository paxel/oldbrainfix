import 'dart:async';

import 'package:oldbrainfix/src/context_platform.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// A [ContextSource] the test drives: what is allowed, what asking
/// answers, and when the calendar events and Bluetooth devices arrive.
class FakeContext implements ContextSource {
  /// The kinds allowed now.
  final Set<ContextKind> allowed;

  /// What the user answers when asked.
  bool grantOnAsk;

  /// The kinds the user was asked for, in order.
  final asked = <ContextKind>[];

  /// How often each was looked up.
  int eventLookups = 0;
  int deviceLookups = 0;

  /// What the next lookups get; complete them to let it arrive. Made on
  /// first use, inside the widget test's fake time.
  late Completer<List<RunningEvent>?> events = Completer();
  late Completer<List<String>?> devices = Completer();

  FakeContext({Set<ContextKind>? allowed, this.grantOnAsk = true})
    : allowed = allowed ?? {...ContextKind.values};

  @override
  bool get available => true;

  @override
  Future<bool> granted(ContextKind kind) async => allowed.contains(kind);

  @override
  Future<bool> request(ContextKind kind) async {
    asked.add(kind);
    if (grantOnAsk) allowed.add(kind);
    return grantOnAsk;
  }

  @override
  Future<List<RunningEvent>?> runningEvents() {
    eventLookups++;
    return events.future;
  }

  @override
  Future<List<String>?> connectedDevices() {
    deviceLookups++;
    return devices.future;
  }
}
