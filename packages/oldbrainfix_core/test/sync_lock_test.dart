import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:isolate';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

/// A sync keeps out of the app's way (#339): one that brings nothing
/// tells nobody, and one that brings much holds the write lock only to
/// write, its signatures checked before, so a tap's write on the app's
/// thread does not wait on them, nor Android call the app stuck.
void main() {
  Map<String, Object?> json(Object? o) =>
      jsonDecode(jsonEncode(o)) as Map<String, Object?>;

  /// The phone's entries the host lacks, as the phone asks with them.
  Map<String, Object?> request(Store phone, Store host) =>
      json(syncRequest(phone, host.versionVector(), name: 'Phone'));

  /// Syncs [phone] into [host] and back, as the LAN session does.
  void exchange(Store phone, Store host) {
    final done = answerSync(host, request(phone, host), name: 'Desk')!;
    applyAnswer(phone, json(done.answer));
  }

  test('a batch that changes neither the log nor a pending set tells no '
      'listener; one that does tells once', () {
    final store = Store.inMemory();
    addTearDown(store.close);
    var heard = 0;
    store.addListener(() => heard++);

    store.batch(() => store.setLocalSetting('seen', 'yes'));
    expect(heard, 0);

    store.batch(() {
      Notes(store).capture('one');
      Notes(store).capture('two');
    });
    expect(heard, 1);
  });

  test('a sync that brings nothing tells neither side', () {
    final phone = Store.inMemory()..setLocalSetting('author', 'Ann');
    final host = Store.inMemory()..setLocalSetting('author', 'Ann');
    addTearDown(phone.close);
    addTearDown(host.close);
    Notes(phone).capture('milk');
    exchange(phone, host);
    expect(Notes(host).all().map((n) => n.text), ['milk']);

    var heard = 0;
    phone.addListener(() => heard++);
    host.addListener(() => heard++);
    exchange(phone, host);
    expect(heard, 0);
  });

  test("a write on the app's thread does not wait while a sync of its own "
      'checks what it brings', () async {
    final dir = Directory.systemTemp.createTempSync('sync-lock');
    addTearDown(() => dir.deleteSync(recursive: true));
    final path = '${dir.path}/host.db';
    final host = Store.open(path)..setLocalSetting('author', 'Ann');
    final phone = Store.inMemory()..setLocalSetting('author', 'Ann');
    addTearDown(phone.close);
    addTearDown(host.close);
    exchange(phone, host);
    phone.batch(() {
      for (var i = 0; i < 1000; i++) {
        Notes(phone).capture('From the phone $i\nand a second line');
      }
    });
    final asked = request(phone, host);

    var longest = Duration.zero;
    var running = true;
    Future<void> tapping() async {
      while (running) {
        final w = Stopwatch()..start();
        Notes(host).capture('A tap');
        if (w.elapsed > longest) longest = w.elapsed;
        await Future<void>.delayed(const Duration(milliseconds: 5));
      }
    }

    final taps = tapping();
    final answered = await _answerAside(path, asked);
    running = false;
    await taps;

    expect(Notes(host).all().length, greaterThan(1000));
    // Before #339 a write waited for nearly all of it.
    expect(longest, lessThan(answered ~/ 3));
  });
}

/// Answers [asked] in an isolate of its own on a store of its own on
/// [path], as the app's sync does; how long the answer took there.
Future<Duration> _answerAside(String path, Map<String, Object?> asked) =>
    Isolate.run(() {
      final own = Store.open(path);
      final w = Stopwatch()..start();
      answerSync(own, asked, name: 'Desk');
      own.close();
      return w.elapsed;
    });
