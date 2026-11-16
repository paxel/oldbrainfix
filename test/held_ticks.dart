import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/tick_saves.dart';

/// Holds the saves of the ticks tapped from now on back (#338) until
/// [HeldTicks.release], so a test sees the frame after a tap with its
/// save still waiting; the test's tear-down lets go of what still waits.
class HeldTicks {
  final _held = <VoidCallback>[];

  HeldTicks() {
    TickSaves.after = _held.add;
    addTearDown(() {
      TickSaves.after = TickSaves.afterFrame;
      TickSaves.run = (save) => save();
      TickSaves.instance.clear();
    });
  }

  /// How many runs of the waiting saves are held.
  int get held => _held.length;

  /// Makes the saves run from now on throw [error], as a full storage
  /// would.
  void failWith(Object error) => TickSaves.run = (_) => throw error;

  /// Runs the saves held, then draws the frame after them.
  Future<void> release(WidgetTester tester) async {
    final held = List.of(_held);
    _held.clear();
    for (final saves in held) {
      saves();
    }
    await tester.pumpAndSettle();
  }
}
