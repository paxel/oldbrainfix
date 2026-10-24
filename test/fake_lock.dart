import 'package:oldbrainfix/src/lock_platform.dart';

/// A [LockPlatform] the test drives: whether the Device has biometrics
/// or a screen lock, and what the user answers when asked.
class FakeBiometrics implements LockPlatform {
  /// Whether the Device can unlock with biometrics or its screen lock.
  bool can;

  /// Whether the user unlocks when asked.
  bool answer;

  /// The titles the user was asked with, in order.
  final asked = <String>[];

  FakeBiometrics({this.can = true, this.answer = true});

  @override
  Future<bool> available() async => can;

  @override
  Future<bool> authenticate({required String title}) async {
    asked.add(title);
    return answer;
  }
}
