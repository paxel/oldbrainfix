import 'dart:async';

import 'package:oldbrainfix/src/home_shortcuts.dart';

/// A [HomeShortcuts] the test drives: a Tally's shortcut pinned, tapped
/// on the home screen, and the app sent back to it.
class FakeHomeShortcuts implements HomeShortcuts {
  @override
  final bool available;

  /// Whether the home screen takes a shortcut.
  final bool takes;

  /// Each shortcut asked for: the Tally's Note and its name.
  final pinned = <(String, String)>[];

  /// The Tally a shortcut asked for and not taken yet.
  String? waiting;

  /// How often the app went back to where the user was.
  var left = 0;

  final _arrived = StreamController<void>.broadcast();

  FakeHomeShortcuts({this.available = true, this.takes = true, this.waiting});

  /// The shortcut of the Tally of [noteId] is tapped while the app runs.
  void tap(String noteId) {
    waiting = noteId;
    _arrived.add(null);
  }

  @override
  Future<bool> pin(String noteId, String label) async {
    pinned.add((noteId, label));
    return takes;
  }

  @override
  Future<String?> take() async {
    final noteId = waiting;
    waiting = null;
    return noteId;
  }

  @override
  Stream<void> get arrived => _arrived.stream;

  @override
  Future<void> leave() async => left++;
}
