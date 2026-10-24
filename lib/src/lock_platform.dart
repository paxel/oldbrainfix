import 'package:flutter/services.dart';

/// The platform seam for unlocking with biometrics or the Device's own
/// screen lock (android/…/Biometrics.kt). The App Lock and the Area
/// Locks always have a PIN besides. Widget tests hand in a fake.
abstract interface class LockPlatform {
  /// Whether this Device can unlock with biometrics or its screen lock.
  Future<bool> available();

  /// Asks the user to unlock, showing [title]; whether they did.
  Future<bool> authenticate({required String title});
}

/// No biometrics: the default where none is handed in; the PIN unlocks.
class NoBiometrics implements LockPlatform {
  const NoBiometrics();

  @override
  Future<bool> available() async => false;

  @override
  Future<bool> authenticate({required String title}) async => false;
}

/// Android's BiometricPrompt, with the screen lock as fallback, behind
/// the app's method channel; no Google library.
class AndroidBiometrics implements LockPlatform {
  static const _channel = MethodChannel('io.github.paxel.oldbrainfix/lock');

  const AndroidBiometrics();

  @override
  Future<bool> available() async => await _call('available', null) ?? false;

  @override
  Future<bool> authenticate({required String title}) async =>
      await _call('authenticate', {'title': title}) ?? false;

  static Future<bool?> _call(String method, Object? args) async {
    try {
      return await _channel.invokeMethod<bool>(method, args);
    } on PlatformException {
      return null;
    }
  }
}
