import 'dart:async';

import 'package:flutter/services.dart';

/// A paired Device's host as discovery heard of it: its device id from
/// the record, and where it answers.
class FoundHost {
  final String device;
  final String host;
  final int port;

  const FoundHost(this.device, this.host, this.port);

  @override
  bool operator ==(Object other) =>
      other is FoundHost &&
      other.device == device &&
      other.host == host &&
      other.port == port;

  @override
  int get hashCode => Object.hash(device, host, port);
}

/// The platform seam for automatic sync (android/…/Discovery.kt,
/// Advertiser.kt and SyncWorker.kt): finding paired hosts on the network
/// (mDNS/DNS-SD, `_oldbrainfix._tcp`), telling the network about this
/// Device's own paired host, and the periodic work that syncs while the
/// app is closed. Widget tests hand in a fake.
abstract interface class SyncPlatform {
  /// Hosts found while discovery runs.
  Stream<FoundHost> get found;

  /// The periodic work came due while the app is open: sync now.
  Stream<void> get wanted;

  /// Starts and stops looking, while the app shows.
  Future<void> startDiscovery();
  Future<void> stopDiscovery();

  /// Looks for [within] and returns what was found, for one background
  /// sync.
  Future<List<FoundHost>> discover(Duration within);

  /// Turns the periodic background sync on or off; with a shared
  /// [folder] it runs without a local network too.
  Future<void> setBackground(bool on, {bool folder = false});

  /// Whether the Device is on a local network now (Wi-Fi or Ethernet),
  /// where paired Devices can be.
  Future<bool> onLocalNetwork();

  /// Advertises this Device's paired host on [port] (`_oldbrainfix._tcp`,
  /// its id [device] as the name and the only TXT entry), until
  /// [unadvertise].
  Future<void> advertise({required String device, required int port});
  Future<void> unadvertise();

  /// Tells the periodic work that the sync it asked for is over.
  Future<void> finished();
}

/// No discovery and no background work: the default where none is
/// handed in. Paired Devices are still tried at their last address.
class NoSync implements SyncPlatform {
  const NoSync();

  @override
  Stream<FoundHost> get found => const Stream.empty();

  @override
  Stream<void> get wanted => const Stream.empty();

  @override
  Future<void> startDiscovery() async {}

  @override
  Future<void> stopDiscovery() async {}

  @override
  Future<List<FoundHost>> discover(Duration within) async => const [];

  @override
  Future<void> setBackground(bool on, {bool folder = false}) async {}

  @override
  Future<bool> onLocalNetwork() async => true;

  @override
  Future<void> advertise({required String device, required int port}) async {}

  @override
  Future<void> unadvertise() async {}

  @override
  Future<void> finished() async {}
}

/// Android's NsdManager and WorkManager in Kotlin, behind the app's
/// method channel; no Google service.
class AndroidSync implements SyncPlatform {
  static const _channel = MethodChannel('io.github.paxel.oldbrainfix/sync');
  final _found = StreamController<FoundHost>.broadcast();
  final _wanted = StreamController<void>.broadcast();

  AndroidSync() {
    _channel.setMethodCallHandler((call) async {
      switch (call.method) {
        case 'found':
          if (_read(call.arguments) case final host?) _found.add(host);
        case 'wanted':
          _wanted.add(null);
      }
    });
  }

  static FoundHost? _read(Object? json) {
    if (json is! Map) return null;
    final device = json['device'];
    final host = json['host'];
    final port = json['port'];
    if (device is! String || host is! String || port is! int) return null;
    return FoundHost(device, host, port);
  }

  @override
  Stream<FoundHost> get found => _found.stream;

  @override
  Stream<void> get wanted => _wanted.stream;

  @override
  Future<void> startDiscovery() => _call('start');

  @override
  Future<void> stopDiscovery() => _call('stop');

  @override
  Future<List<FoundHost>> discover(Duration within) async {
    try {
      final found = await _channel.invokeListMethod<Object?>(
        'discover',
        within.inMilliseconds,
      );
      return [for (final f in found ?? const []) ?_read(f)];
    } on PlatformException {
      return const [];
    }
  }

  @override
  Future<void> setBackground(bool on, {bool folder = false}) =>
      _call('background', {'on': on, 'folder': folder});

  @override
  Future<bool> onLocalNetwork() async {
    try {
      return await _channel.invokeMethod<bool>('localNetwork') ?? true;
    } on PlatformException {
      return true;
    } on MissingPluginException {
      return true;
    }
  }

  @override
  Future<void> advertise({required String device, required int port}) =>
      _call('advertise', {'device': device, 'port': port});

  @override
  Future<void> unadvertise() => _call('unadvertise');

  @override
  Future<void> finished() => _call('finished');

  Future<void> _call(String method, [Object? arguments]) async {
    try {
      await _channel.invokeMethod<void>(method, arguments);
    } on PlatformException {
      // No discovery here: paired Devices are tried at their last address.
    } on MissingPluginException {
      // Likewise.
    }
  }
}
