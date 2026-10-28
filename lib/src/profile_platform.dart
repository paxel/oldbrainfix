import 'dart:async';

import 'package:flutter/services.dart';

/// The platform seam for the user's own Profile (ADR 0017): the phone's
/// name for the default, a picture picked from the phone, and Profile
/// cards (#107) sent through another app or opened with this one
/// (android/…/Profiles.kt, Cards.kt, MainActivity.kt). Widget tests hand
/// in a fake.
abstract interface class ProfilePlatform {
  /// The name the phone was given in its settings, else its model; empty
  /// when neither is known.
  Future<String> deviceName();

  /// Whether a picture can be picked here.
  bool get canPick;

  /// A picture the user picks with the system's picker, upright and
  /// without metadata; null when none was picked.
  Future<Uint8List?> pickPicture();

  /// Offers [text] to the app the user picks, a messenger say; false when
  /// no app can take it.
  Future<bool> shareText(String text);

  /// The Profile card links tapped in other apps and not opened yet,
  /// oldest first; each is handed out once.
  Future<List<String>> takeCards();

  /// A Profile card link was tapped in another app while this one runs.
  Stream<void> get cardOpened;
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

  @override
  Future<bool> shareText(String text) async => false;

  @override
  Future<List<String>> takeCards() async => const [];

  @override
  Stream<void> get cardOpened => const Stream.empty();
}

/// Android's own settings and picture picker behind the app's method
/// channel; no permission, no Google library.
class AndroidProfile implements ProfilePlatform {
  static const _channel = MethodChannel('io.github.paxel.oldbrainfix/profile');

  final _cardOpened = StreamController<void>.broadcast();

  AndroidProfile() {
    _channel.setMethodCallHandler((call) async {
      if (call.method == 'cardOpened') _cardOpened.add(null);
    });
  }

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

  @override
  Future<bool> shareText(String text) async {
    try {
      return await _channel.invokeMethod<bool>('share', text) ?? false;
    } on PlatformException {
      return false;
    }
  }

  @override
  Future<List<String>> takeCards() async {
    try {
      final links = await _channel.invokeMethod<List<Object?>>('takeCards');
      return [
        for (final link in links ?? const [])
          if (link is String) link,
      ];
    } on PlatformException {
      return const [];
    }
  }

  @override
  Stream<void> get cardOpened => _cardOpened.stream;
}
