import 'package:flutter/services.dart';

/// The platform seam for the user's own Profile (ADR 0017): the phone's
/// name for the default and a picture picked from the phone
/// (android/…/Profiles.kt, MainActivity.kt). Profile card files (#144)
/// go out and come in as bundle files do. Widget tests hand in a fake.
abstract interface class ProfilePlatform {
  /// The name the phone was given in its settings, else its model; empty
  /// when neither is known.
  Future<String> deviceName();

  /// Whether a picture can be picked here.
  bool get canPick;

  /// A picture the user picks with the system's picker, upright and
  /// without metadata; null when none was picked.
  Future<Uint8List?> pickPicture();
}

/// No phone name and no picker: the default where none is handed in.
class NoProfilePlatform implements ProfilePlatform {
  const NoProfilePlatform();

  @override
  Future<String> deviceName() async => '';

  @override
  bool get canPick => false;

  @override
  Future<Uint8List?> pickPicture() async => null;
}

/// Android's own settings and picture picker behind the app's method
/// channel; no permission, no Google library.
class AndroidProfile implements ProfilePlatform {
  static const _channel = MethodChannel('io.github.paxel.oldbrainfix/profile');

  @override
  Future<String> deviceName() async {
    try {
      return await _channel.invokeMethod<String>('deviceName') ?? '';
    } on PlatformException {
      return '';
    }
  }

  @override
  bool get canPick => true;

  @override
  Future<Uint8List?> pickPicture() async {
    try {
      return await _channel.invokeMethod<Uint8List>('pick');
    } on PlatformException {
      return null;
    }
  }
}
