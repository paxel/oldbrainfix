import 'dart:io';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'test_host.dart';

DateTime Function() ticking() {
  var now = DateTime.utc(2026, 3, 1, 9);
  return () => now = now.add(const Duration(seconds: 1));
}

Matcher failsWith(AskTrouble trouble) =>
    throwsA(isA<AskException>().having((e) => e.trouble, 'trouble', trouble));

const _anthropic = DesktopProvider(
  protocol: AskProtocol.anthropic,
  model: 'claude-opus-5-5',
  host: 'api.anthropic.com',
);

void main() {
  group('asking the paired desktop', () {
    late Store desk;
    late Store phone;
    late TestHost host;

    /// Pairs the phone with the desk once by code, and the desk
    /// remembers the phone as the desktop does after a session.
    Future<PairedDevice> pair({bool shared = false}) async {
      await lanSync(phone, decodePairCode(host.code())!, name: 'Phone');
      desk.rememberDevice(
        PairedDevice(
          id: phone.deviceId,
          name: 'Phone',
          fingerprint: '',
          host: null,
          port: null,
          lastSync: desk.clock(),
        ),
      );
      final device = phone.pairedDevice(desk.deviceId)!;
      if (!shared) return device;
      final theirs = PairedDevice(
        id: device.id,
        name: device.name,
        fingerprint: device.fingerprint,
        host: device.host,
        port: device.port,
        lastSync: device.lastSync,
        shared: true,
      );
      phone.rememberDevice(theirs);
      return theirs;
    }

    List<PairedAddress> here() => [(host: '127.0.0.1', port: host.port)];

    setUp(() async {
      final clock = ticking();
      desk = Store.inMemory(clock: clock);
      phone = Store.inMemory(clock: clock);
      host = TestHost(desk);
      await host.start();
      host.provider = _anthropic;
    });

    tearDown(() => host.stop());

    test('names the provider and answers the question with its Notes, '
        'whether a source could not be checked, and the review that '
        'waits', () async {
      final device = await pair();
      host.answering = (body) => const AskReply.answered(
        DesktopAnswer(
          text: 'In the [blue folder](oldbrainfix://note/note:1), a scan.',
          notes: ['note:1'],
          unchecked: true,
        ),
        review: true,
      );
      expect(await desktopProvider(phone, device, at: here()), _anthropic);
      final reply = await askDesktop(
        phone,
        device,
        at: here(),
        question: 'And where is it now?',
        earlier: const [AskExchange('Do I have a passport?', 'Yes.')],
      );
      expect(reply.answer?.notes, ['note:1']);
      expect(reply.answer?.text, contains('blue folder'));
      expect(reply.answer?.unchecked, isTrue);
      expect(reply.review, isTrue);
      expect(host.asked.single, {
        'earlier': [
          {'answer': 'Yes.', 'question': 'Do I have a passport?'},
        ],
        'question': 'And where is it now?',
      });
      expect(host.askers, [phone.deviceId, phone.deviceId]);
      expect(host.nonces, hasLength(2), reason: 'a fresh nonce each time');
    });

    test('a failure on the desktop comes back as its kind', () async {
      final device = await pair();
      host.answering = (body) => const AskReply.failed(
        AskFailure.provider,
        status: 529,
        message: 'Overloaded',
      );
      final reply = await askDesktop(phone, device, at: here(), question: 'Q');
      expect(reply.answer, isNull);
      expect(reply.failure, AskFailure.provider);
      expect((reply.status, reply.message), (529, 'Overloaded'));
    });

    test('another person\'s Device, or one not paired, is never '
        'asked', () async {
      final theirs = await pair(shared: true);
      final before = host.requests;
      await expectLater(
        askDesktop(phone, theirs, at: here(), question: 'Q'),
        failsWith(AskTrouble.notOwn),
      );
      await expectLater(
        desktopProvider(phone, theirs, at: here()),
        failsWith(AskTrouble.notOwn),
      );
      final stranger = PairedDevice(
        id: 'device:stranger',
        name: 'Stranger',
        fingerprint: theirs.fingerprint,
        host: '127.0.0.1',
        port: host.port,
        lastSync: phone.clock(),
      );
      await expectLater(
        askDesktop(phone, stranger, at: here(), question: 'Q'),
        failsWith(AskTrouble.notOwn),
      );
      expect(host.requests, before, reason: 'nothing was sent');
    });

    test('a desktop that does not know this Device says no, a phone\'s '
        'host answers no questions', () async {
      final device = await pair();
      host.answering = null;
      await expectLater(
        desktopProvider(phone, device, at: here()),
        failsWith(AskTrouble.notOffered),
      );
      host.answering = (body) =>
          const AskReply.answered(DesktopAnswer(text: 'Here.', notes: []));
      desk.forgetDevice(phone.deviceId);
      await expectLater(
        askDesktop(phone, device, at: here(), question: 'Q'),
        failsWith(AskTrouble.declined),
      );
    });

    test('another certificate than the pinned one hears no question', () async {
      final device = await pair();
      host.answering = (body) =>
          const AskReply.answered(DesktopAnswer(text: 'Here.', notes: []));
      final other = PairedDevice(
        id: device.id,
        name: device.name,
        fingerprint: '00' * 32,
        host: device.host,
        port: device.port,
        lastSync: device.lastSync,
      );
      await expectLater(
        askDesktop(phone, other, at: here(), question: 'Q'),
        failsWith(AskTrouble.wrongHost),
      );
      expect(host.asked, isEmpty);
    });

    test('an address that does not answer is passed for the next, and an '
        'answer not in time is given up', () async {
      final device = await pair();
      host.answering = (body) =>
          const AskReply.answered(DesktopAnswer(text: 'Here.', notes: []));
      final closed = await ServerSocket.bind(InternetAddress.loopbackIPv4, 0);
      final gone = closed.port;
      await closed.close();
      final reply = await askDesktop(
        phone,
        device,
        at: [(host: '127.0.0.1', port: gone), ...here()],
        question: 'Q',
      );
      expect(reply.answer?.text, 'Here.');
      await expectLater(
        askDesktop(
          phone,
          device,
          at: [(host: '127.0.0.1', port: gone)],
          question: 'Q',
        ),
        failsWith(AskTrouble.unreachable),
      );
      // A public address not typed for it is not tried.
      await expectLater(
        desktopProvider(phone, device, at: [(host: '8.8.8.8', port: 443)]),
        failsWith(AskTrouble.unreachable),
      );
      await expectLater(
        askDesktop(
          phone,
          device,
          at: here(),
          question: 'Q',
          wait: Duration.zero,
        ),
        failsWith(AskTrouble.timedOut),
      );
    });
  });

  group('the replies', () {
    test('every failure kind goes both ways, a later one as unknown', () {
      for (final failure in AskFailure.values) {
        final reply = AskReply.failed(
          failure,
          message: 'said',
          status: 500,
          unready: AskUnready.badUrl,
          review: true,
        );
        final back = AskReply.fromJson(reply.toJson());
        expect(back?.failure, failure);
        expect(back?.review, isTrue);
        final messaged = {
          AskFailure.key,
          AskFailure.provider,
          AskFailure.network,
          AskFailure.unreadable,
        }.contains(failure);
        expect(back?.message, messaged ? 'said' : '', reason: '$failure');
        expect(back?.status, failure == AskFailure.provider ? 500 : 0);
        expect(
          back?.unready,
          failure == AskFailure.unready ? AskUnready.badUrl : null,
        );
      }
      expect(
        AskReply.fromJson({'failure': 'quota', 'review': false})?.failure,
        AskFailure.unknown,
      );
      expect(AskReply.fromJson({'what': 1}), isNull);
      expect(AskReply.fromJson({'answer': 'text'}), isNull);
      expect(
        AskReply.fromJson({'failure': 'provider', 'status': 500.0})?.status,
        0,
        reason: 'a status is a whole number',
      );
    });

    test('the mark that a source could not be checked goes both ways, and '
        'an answer without it has none', () {
      for (final unchecked in [true, false]) {
        final reply = AskReply.answered(
          DesktopAnswer(text: 'T', notes: const [], unchecked: unchecked),
        );
        final json = reply.toJson();
        expect((json['answer'] as Map?)?['unchecked'], unchecked);
        expect(AskReply.fromJson(json)?.answer?.unchecked, unchecked);
      }
      for (final later in [
        {'text': 'T'},
        {'text': 'T', 'unchecked': 'yes'},
        {'text': 'T', 'unchecked': 1},
      ]) {
        final reply = AskReply.fromJson({'answer': later, 'review': false});
        expect(reply?.answer?.unchecked, isFalse, reason: '$later');
      }
    });

    test('a provider of a later protocol is none, a reason not known is '
        'no model', () {
      expect(DesktopProvider.fromJson(_anthropic.toJson()), _anthropic);
      expect(
        DesktopProvider.fromJson({'protocol': 'gemini', 'ready': true}),
        isNull,
      );
      expect(
        DesktopProvider.fromJson({
          'protocol': 'openai',
          'ready': false,
          'unready': 'later',
        })?.unready,
        AskUnready.noModel,
      );
    });
  });
}
