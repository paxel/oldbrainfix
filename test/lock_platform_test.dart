import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/lock_platform.dart';

/// The lock channel's wrapper, as Biometrics.kt and, on the iPhone,
/// LockChannel.swift answer it (#243): whether the Device can unlock,
/// and whether the user did; anything else, an error or no channel at
/// all, is a no and leaves the PIN.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  const channel = MethodChannel('io.github.paxel.oldbrainfix/lock');
  final messenger =
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
  tearDown(() => messenger.setMockMethodCallHandler(channel, null));

  void answer(Object? Function(MethodCall call) answer) =>
      messenger.setMockMethodCallHandler(channel, (call) async => answer(call));

  test('the Device says whether it can unlock', () async {
    answer((call) => call.method == 'available');
    expect(await const AndroidBiometrics().available(), isTrue);
    answer((call) => false);
    expect(await const AndroidBiometrics().available(), isFalse);
  });

  test(
    'unlocking hands the title over and answers whether the user did',
    () async {
      final asked = <Object?>[];
      var unlocked = true;
      answer((call) {
        asked.add(call.arguments);
        return call.method == 'authenticate' && unlocked;
      });
      expect(
        await const AndroidBiometrics().authenticate(title: 'Unlock'),
        isTrue,
      );
      unlocked = false;
      expect(
        await const AndroidBiometrics().authenticate(title: 'Unlock'),
        isFalse,
      );
      expect(asked, [
        {'title': 'Unlock'},
        {'title': 'Unlock'},
      ]);
    },
  );

  test('an error or no answer is a no', () async {
    answer((call) => throw PlatformException(code: 'failed'));
    expect(await const AndroidBiometrics().available(), isFalse);
    expect(await const AndroidBiometrics().authenticate(title: 'x'), isFalse);
    answer((call) => null);
    expect(await const AndroidBiometrics().available(), isFalse);
    expect(await const AndroidBiometrics().authenticate(title: 'x'), isFalse);
  });

  test(
    'without the channel, as before #243 on the iPhone, it is a no',
    () async {
      expect(await const AndroidBiometrics().available(), isFalse);
      expect(await const AndroidBiometrics().authenticate(title: 'x'), isFalse);
    },
  );
}
