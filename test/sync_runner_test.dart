import 'dart:async';
import 'dart:io';
import 'dart:isolate';

import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/auto_sync.dart';
import 'package:oldbrainfix/src/sync_runner.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// One LAN session without the network: [joiner] joins [host].
SyncSession _exchange(Store joiner, Store host) {
  final done = answerSync(
    host,
    syncRequest(joiner, host.versionVector(), name: 'Pixel'),
    name: 'Desk',
  )!;
  return applyAnswer(joiner, done.answer);
}

/// A join run in the isolate: the "host" is the path of the desk's
/// database, opened there, as the network would answer.
Future<SyncResult> _deskJoin(
  Store store,
  PairedDevice device, {
  required String host,
  required int port,
  required String name,
}) async {
  final desk = Store.open(host);
  try {
    return SyncResult(
      session: _exchange(store, desk),
      entriesSent: 0,
      blobsSent: 0,
      blobsReceived: 0,
    );
  } finally {
    desk.close();
  }
}

/// A join to a Device that needs a newer app here.
Future<SyncResult> _tooOld(
  Store store,
  PairedDevice device, {
  required String host,
  required int port,
  required String name,
}) async => throw const SyncException(SyncFailure.updateHere);

/// A job that writes a Note there and hands back its id.
String _writes(Store store) => Notes(store).capture('From the isolate').id;

/// A job that writes a Note there, then throws from [_failsHere].
String _writesThenThrows(Store store) {
  Notes(store).capture('Half done');
  return _failsHere();
}

String _failsHere() => throw StateError('broke there');

/// A job that writes a Note there and hands back what cannot leave the
/// isolate, so the isolate itself fails.
Object _writesUnsendable(Store store) {
  Notes(store).capture('Lost on the way');
  return ReceivePort();
}

void main() {
  late Directory dir;
  late Store phone;
  late Store desk;
  late String deskPath;
  late String phonePath;
  var told = 0;

  PairedDevice paired() => PairedDevice(
    id: desk.deviceId,
    name: 'Desk',
    fingerprint: 'ab' * 32,
    host: deskPath,
    port: 1,
    lastSync: DateTime.utc(2026, 10, 24),
  );

  setUp(() {
    dir = Directory.systemTemp.createTempSync('sync_runner_test_');
    Directory('${dir.path}/phone').createSync();
    Directory('${dir.path}/desk').createSync();
    Directory('${dir.path}/folder').createSync();
    phonePath = '${dir.path}/phone/oldbrainfix.db';
    phone = Store.open(phonePath)..setLocalSetting('author', 'Ann');
    deskPath = '${dir.path}/desk/oldbrainfix.db';
    desk = Store.open(deskPath)..setLocalSetting('author', 'Ann');
    // Paired: each knows the other's key.
    _exchange(phone, desk);
    told = 0;
    phone.addListener(() => told++);
  });
  tearDown(() {
    phone.close();
    desk.close();
    dir.deleteSync(recursive: true);
  });

  IsolateSyncRunner runner({PairedJoin joining = _deskJoin}) =>
      IsolateSyncRunner(
        path: phonePath,
        open: LocalSyncFolder.new,
        joining: joining,
      );

  test('a folder round merges in an isolate of its own while this one '
      'keeps running, and the store here hears of it (#225)', () async {
    // The other Device's months of use, waiting in the folder.
    desk.batch(() {
      for (var i = 0; i < 2000; i++) {
        Notes(desk).capture('Desk note $i');
      }
    });
    await folderRound(desk, LocalSyncFolder('${dir.path}/folder'));
    // This isolate's event loop, as the app's frames ride on it.
    var longest = Duration.zero;
    var ticks = 0;
    final gap = Stopwatch()..start();
    final ticker = Timer.periodic(const Duration(milliseconds: 2), (_) {
      if (gap.elapsed > longest) longest = gap.elapsed;
      ticks++;
      gap.reset();
    });
    final took = Stopwatch()..start();
    final result = await runner().folder(phone, '${dir.path}/folder');
    took.stop();
    ticker.cancel();
    if (gap.elapsed > longest) longest = gap.elapsed;
    expect(result?.applied, hasLength(6000));
    expect(told, 1, reason: 'once, as after a sync of its own');
    expect(Notes(phone).all(), hasLength(2000));
    expect(phone.folderStatus?.entriesIn, 6000);
    // Drawing goes on: no stall here near the time the merge took.
    expect(took.elapsed, greaterThan(const Duration(milliseconds: 300)));
    expect(longest, lessThan(took.elapsed ~/ 3));
    expect(ticks, greaterThan(10));
  });

  test('a join runs there too; what it brought, and a refusal, come back '
      'as they would here (#225)', () async {
    Notes(desk).capture('From the desk');
    final result = await runner().join(
      phone,
      paired(),
      host: deskPath,
      port: 1,
      name: 'Pixel',
    );
    expect(result.session.applied.map((e) => e.value), isNotEmpty);
    expect(told, 1);
    expect([for (final n in Notes(phone).all()) n.text], ['From the desk']);
    await expectLater(
      runner(joining: _tooOld)
          .join(phone, paired(), host: deskPath, port: 1, name: 'Pixel'),
      throwsA(
        isA<SyncException>().having(
          (e) => e.failure,
          'failure',
          SyncFailure.updateHere,
        ),
      ),
    );
    expect(told, 1, reason: 'nothing changed');
  });

  group('a job on a connection of its own (#207, #225)', () {
    var profiles = 0;
    setUp(() {
      profiles = 0;
      phone.addProfileListener(() => profiles++);
    });

    test('tells the store here of what it wrote, and nothing when it '
        'wrote nothing', () async {
      final id = await runIsolated(phone, phonePath, _writes);
      expect(told, 1);
      expect(Notes(phone).byId(id)?.text, 'From the isolate');
      final count = await runIsolated(
        phone,
        phonePath,
        (store) => Notes(store).all().length,
      );
      expect(count, 1);
      expect(told, 1, reason: 'a read writes nothing');
      expect(profiles, 0);
    });

    test('tells the store here of what it wrote before it threw, and '
        'throws it with the trace it was thrown at there', () async {
      Object? thrown;
      StackTrace? trace;
      try {
        await runIsolated(phone, phonePath, _writesThenThrows);
      } on StateError catch (e, t) {
        thrown = e;
        trace = t;
      }
      expect(thrown, isA<StateError>());
      expect(trace.toString(), contains('_failsHere'));
      expect(told, 1);
      expect([for (final n in Notes(phone).all()) n.text], ['Half done']);
    });

    test('tells the store here of everything when the isolate itself '
        'failed, as nobody knows what it wrote', () async {
      await expectLater(
        runIsolated(phone, phonePath, _writesUnsendable),
        throwsA(anything),
      );
      expect(told, 1);
      expect(profiles, 1);
      expect([for (final n in Notes(phone).all()) n.text], ['Lost on the way']);
    });
  });
}
