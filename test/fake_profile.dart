import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:oldbrainfix/src/profile_platform.dart';

/// A 60 × 40 PNG, red on the left half and blue on the right.
final Uint8List halvesPng = base64.decode(
  'iVBORw0KGgoAAAANSUhEUgAAADwAAAAoCAIAAAAt2Q6oAAAAPUlEQVR42u3OwQkAAAjEsBPcf2WdwoeQ'
  'DlDSk6sqV+/Ow6ChoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGh/7RE4gKgLGzVqQAAAABJRU5ErkJggg==',
);

/// The phone's name and picture picker as a test sets them; what was
/// shared is kept, and card links are tapped with [tapCard].
class FakeProfilePlatform implements ProfilePlatform {
  String name;

  /// The texts handed to the share chooser.
  final shared = <String>[];

  /// Whether an app takes what is shared.
  var shares = true;

  final _cards = <String>[];
  final _cardOpened = StreamController<void>.broadcast();

  /// What the next pick hands back; null as if nothing was picked.
  Uint8List? picked;
  var picks = 0;

  FakeProfilePlatform({this.name = '', this.picked});

  @override
  Future<String> deviceName() async => name;

  @override
  bool get canPick => true;

  @override
  Future<Uint8List?> pickPicture() async {
    picks++;
    return picked;
  }

  @override
  Future<bool> shareText(String text) async {
    shared.add(text);
    return shares;
  }

  @override
  Future<List<String>> takeCards() async {
    final taken = [..._cards];
    _cards.clear();
    return taken;
  }

  @override
  Stream<void> get cardOpened => _cardOpened.stream;

  /// A card [link] is tapped in another app while this one runs.
  void tapCard(String link) {
    _cards.add(link);
    _cardOpened.add(null);
  }

  /// A card [link] was tapped before the app started.
  void tappedBefore(String link) => _cards.add(link);
}
