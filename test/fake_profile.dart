import 'dart:convert';
import 'dart:typed_data';

import 'package:oldbrainfix/src/profile_platform.dart';

/// A 60 × 40 PNG, red on the left half and blue on the right.
final Uint8List halvesPng = base64.decode(
  'iVBORw0KGgoAAAANSUhEUgAAADwAAAAoCAIAAAAt2Q6oAAAAPUlEQVR42u3OwQkAAAjEsBPcf2WdwoeQ'
  'DlDSk6sqV+/Ow6ChoaGhoaGhoaGhoaGhoaGhoaGhoaGhoaGh/7RE4gKgLGzVqQAAAABJRU5ErkJggg==',
);

/// The phone's name and picture picker as a test sets them.
class FakeProfilePlatform implements ProfilePlatform {
  String name;

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
}
