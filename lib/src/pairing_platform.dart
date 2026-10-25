import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';

/// The platform seam for Pairing (android/…/Pairing.kt): reading the
/// pair code in a photo of the computer's screen, and the name this
/// Device goes by. Widget tests hand in a fake.
abstract interface class PairingPlatform {
  /// The text of the QR code in [photo] (a JPEG), or null when it shows
  /// none.
  Future<String?> readQr(Uint8List photo);

  /// The name this Device goes by when it syncs, such as its model.
  Future<String> deviceName();
}

/// No QR reading: the default where none is handed in. Typing the code
/// always works.
class NoPairing implements PairingPlatform {
  const NoPairing();

  @override
  Future<String?> readQr(Uint8List photo) async => null;

  @override
  Future<String> deviceName() async => '';
}

/// zxing's QR reader in Kotlin, fed by the camera app's photo, behind the
/// app's method channel; no Google service.
class AndroidPairing implements PairingPlatform {
  static const _channel = MethodChannel('io.github.paxel.oldbrainfix/pairing');

  const AndroidPairing();

  @override
  Future<String?> readQr(Uint8List photo) async {
    try {
      return await _channel.invokeMethod<String>('readQr', photo);
    } on PlatformException {
      return null;
    }
  }

  @override
  Future<String> deviceName() async {
    try {
      return await _channel.invokeMethod<String>('deviceName') ?? '';
    } on PlatformException {
      return '';
    }
  }
}

/// Hands the app's [PairingPlatform] down to every screen.
class PairingScope extends InheritedWidget {
  final PairingPlatform pairing;

  const PairingScope({super.key, required this.pairing, required super.child});

  /// The app's [PairingPlatform]; read once, so it does not listen.
  static PairingPlatform of(BuildContext context) =>
      context.getInheritedWidgetOfExactType<PairingScope>()?.pairing ??
      const NoPairing();

  @override
  bool updateShouldNotify(PairingScope oldWidget) =>
      pairing != oldWidget.pairing;
}
