import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';

/// Whether Old Brain Fix is on as Android's print service (#293), as far
/// as Android tells.
enum PrintServiceState { on, off, unknown }

/// The platform seam for Print to Old Brain Fix (#293): the app as
/// Android's print service (android/…/PrintIn.kt), whether the user
/// turned it on, and Android's print settings, where that is done. What
/// is printed comes in as a share does (`PhotoPlatform.takeShared`).
/// Widget tests hand in a fake.
abstract interface class PrintPlatform {
  /// Whether this Device can print to the app; Settings shows the row
  /// only where it can, so never on the iPhone.
  bool get available;

  /// Whether the print service is on now.
  Future<PrintServiceState> serviceState();

  /// Opens Android's print settings; whether they opened.
  Future<bool> openSettings();
}

/// No print service: the default where none is handed in, and the
/// iPhone's.
class NoPrinting implements PrintPlatform {
  const NoPrinting();

  @override
  bool get available => false;

  @override
  Future<PrintServiceState> serviceState() async => PrintServiceState.unknown;

  @override
  Future<bool> openSettings() async => false;
}

/// The app's own channel to its print service on Android.
class AndroidPrinting implements PrintPlatform {
  static const _channel = MethodChannel('io.github.paxel.oldbrainfix/print');

  const AndroidPrinting();

  @override
  bool get available => true;

  @override
  Future<PrintServiceState> serviceState() async {
    final String? state;
    try {
      state = await _channel.invokeMethod<String>('state');
    } on PlatformException {
      return PrintServiceState.unknown;
    } on MissingPluginException {
      return PrintServiceState.unknown;
    }
    return switch (state) {
      'on' => PrintServiceState.on,
      'off' => PrintServiceState.off,
      _ => PrintServiceState.unknown,
    };
  }

  @override
  Future<bool> openSettings() async {
    try {
      return await _channel.invokeMethod<bool>('openSettings') ?? false;
    } on PlatformException {
      return false;
    } on MissingPluginException {
      return false;
    }
  }
}

/// Hands the app's [PrintPlatform] down to Settings.
class PrintScope extends InheritedWidget {
  final PrintPlatform printing;

  const PrintScope({super.key, required this.printing, required super.child});

  /// The app's [PrintPlatform]; read once, so it does not listen.
  static PrintPlatform of(BuildContext context) =>
      context.getInheritedWidgetOfExactType<PrintScope>()?.printing ??
      const NoPrinting();

  @override
  bool updateShouldNotify(PrintScope oldWidget) =>
      printing != oldWidget.printing;
}
