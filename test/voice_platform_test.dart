import 'dart:typed_data';

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/voice_platform.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  const channel = MethodChannel('io.github.paxel.oldbrainfix/voice');
  final messenger =
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;

  tearDown(() => messenger.setMockMethodCallHandler(channel, null));

  /// Answers every call with [answer] and records what was asked.
  List<MethodCall> answering(Object? Function(MethodCall call) answer) {
    final calls = <MethodCall>[];
    messenger.setMockMethodCallHandler(channel, (call) async {
      calls.add(call);
      return answer(call);
    });
    return calls;
  }

  group('the Android voice channel (#368)', () {
    test(
      'start asks the platform with no arguments and gives its answer',
      () async {
        final calls = answering((_) => true);
        expect(await AndroidVoice().start(), isTrue);
        expect(calls.single.method, 'start');
        expect(calls.single.arguments, isNull);
      },
    );

    test('a refused or failing start is no recording', () async {
      answering((_) => null);
      expect(await AndroidVoice().start(), isFalse);
      answering((_) => throw PlatformException(code: 'busy'));
      expect(await AndroidVoice().start(), isFalse);
    });

    test('stop gives the recording, and nothing when it fails', () async {
      final bytes = Uint8List.fromList([1, 2, 3]);
      answering((_) => bytes);
      expect(await AndroidVoice().stop(), bytes);
      answering((_) => throw PlatformException(code: 'none'));
      expect(await AndroidVoice().stop(), isNull);
    });

    test('the permission and playback calls pass their arguments', () async {
      final calls = answering((_) => true);
      final voice = AndroidVoice();
      final bytes = Uint8List.fromList([9]);
      expect(await voice.granted(), isTrue);
      expect(await voice.request(), isTrue);
      expect(await voice.play(bytes), isTrue);
      await voice.setSpeed(1.5);
      await voice.stopPlaying();
      expect(calls.map((c) => c.method).toList(), [
        'granted',
        'request',
        'play',
        'speed',
        'stopPlaying',
      ]);
      expect(calls[2].arguments, bytes);
      expect(calls[3].arguments, 1.5);
    });

    test('a call the platform does not know is no answer', () async {
      expect(await AndroidVoice().start(), isFalse);
      expect(await AndroidVoice().stop(), isNull);
    });
  });
}
