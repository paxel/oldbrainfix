import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

void main() {
  test('an address by hand is read as the desktop core reads it', () {
    expect(
      HandAddress.parse(' Desk.Tail1234.ts.net '),
      const HandAddress('desk.tail1234.ts.net'),
    );
    expect(
      HandAddress.parse('100.101.102.103:4711'),
      const HandAddress('100.101.102.103', 4711),
    );
    expect(HandAddress.parse('[fd7a::1]:80'), const HandAddress('fd7a::1', 80));
    expect(HandAddress.parse('fd7a::1'), const HandAddress('fd7a::1'));
    expect(
      HandAddress.parse('https://desk.local:4711/x'),
      const HandAddress('desk.local', 4711),
    );
    for (final refused in [
      '',
      'desk:0',
      'desk:+80',
      'desk:0x50',
      'desk:65536',
      'a b',
      'ftp://desk',
      '[desk]:1',
      'me@desk',
      'desk:',
      // The Kelvin sign lowers to k in Unicode, not in ASCII.
      'desK',
    ]) {
      expect(HandAddress.parse(refused), isNull, reason: refused);
    }
    expect(const HandAddress('fd7a::1', 80).text, '[fd7a::1]:80');
  });

  test('it is kept on this Device only and forgotten with the paired '
      'Device', () {
    final store = Store.inMemory();
    expect(store.setHandAddress('desk', 'Desk.ts.net:4711'), isTrue);
    expect(store.handAddress('desk'), const HandAddress('desk.ts.net', 4711));
    expect(store.setHandAddress('desk', 'desk:99999'), isFalse);
    expect(
      store.handAddress('desk'),
      const HandAddress('desk.ts.net', 4711),
      reason: 'kept',
    );
    expect(store.allEntries(), isEmpty, reason: 'never synced');
    store.forgetDevice('desk');
    expect(store.handAddress('desk'), isNull);
    expect(store.setHandAddress('desk', 'desk.ts.net'), isTrue);
    expect(store.setHandAddress('desk', '  '), isTrue);
    expect(store.handAddress('desk'), isNull, reason: 'emptied');
  });

  test('a paired Device is looked for where mDNS found it, then at its '
      'address by hand, then where it was last', () {
    final store = Store.inMemory();
    final desk = PairedDevice(
      id: 'desk',
      name: 'Desk',
      fingerprint: 'ab' * 32,
      host: '192.168.1.5',
      port: 4000,
      lastSync: DateTime.utc(2026),
    );
    expect(store.pairedAddresses(desk), [(host: '192.168.1.5', port: 4000)]);
    store.setHandAddress('desk', 'desk.ts.net');
    expect(store.pairedAddresses(desk), [
      (host: 'desk.ts.net', port: 4000),
      (host: '192.168.1.5', port: 4000),
    ], reason: 'mDNS found nothing: the address by hand, at the kept port');
    store.setHandAddress('desk', '100.64.0.7:4711');
    expect(
      store.pairedAddresses(desk, found: (host: '192.168.1.5', port: 4000)),
      [(host: '192.168.1.5', port: 4000), (host: '100.64.0.7', port: 4711)],
    );
    // Not tried outside the local network but at the address by hand.
    expect(store.mayReachPaired(desk, '100.64.0.7'), isTrue);
    expect(store.mayReachPaired(desk, '100.64.0.8'), isFalse);
    expect(store.mayReachPaired(desk, '192.168.1.9'), isTrue);
  });
}
