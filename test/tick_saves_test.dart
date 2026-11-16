import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/tick_saves.dart';

/// Ticks shown before they are saved (#338): what waits shows, the saves
/// run after the frame in the order tapped, the last tap of a box wins,
/// and a failed save no longer waits.
void main() {
  final ticks = TickSaves.instance;
  final held = <VoidCallback>[];

  setUp(() => TickSaves.after = held.add);
  tearDown(() {
    held.clear();
    ticks.clear();
    TickSaves.after = TickSaves.afterFrame;
    TickSaves.run = (save) => save();
  });

  void release() {
    final saves = List.of(held);
    held.clear();
    for (final run in saves) {
      run();
    }
  }

  test('a tick shows at once and is saved only once the frame lets it', () {
    final saved = <String>[];
    ticks.tick('a', true, save: () => saved.add('a'), failed: (_) {});
    expect(ticks.waiting<bool>('a'), isTrue);
    expect(saved, isEmpty);
    release();
    expect(saved, ['a']);
    expect(ticks.waiting<bool>('a'), isNull);
  });

  test('two taps on a box save the last; another box keeps its place in '
      'the order; the saves are asked for once', () {
    final saved = <String>[];
    ticks
      ..tick('a', true, save: () => saved.add('a on'), failed: (_) {})
      ..tick('b', true, save: () => saved.add('b on'), failed: (_) {})
      ..tick('a', false, save: () => saved.add('a off'), failed: (_) {});
    expect(ticks.waiting<bool>('a'), isFalse);
    expect(held, hasLength(1));
    release();
    expect(saved, ['a off', 'b on']);
  });

  test('a tick made while the saves run is saved in the same run', () {
    final saved = <String>[];
    ticks.tick(
      'a',
      true,
      save: () {
        saved.add('a');
        ticks.tick('b', true, save: () => saved.add('b'), failed: (_) {});
      },
      failed: (_) {},
    );
    release();
    expect(saved, ['a', 'b']);
    expect(held, isEmpty);
  });

  test('a failed save no longer waits and tells what went wrong; the next '
      'one still runs', () {
    final failed = <Object>[];
    final saved = <String>[];
    ticks
      ..tick(
        'a',
        true,
        save: () => throw StateError('no role'),
        failed: failed.add,
      )
      ..tick('b', true, save: () => saved.add('b'), failed: failed.add);
    release();
    expect(failed.single, isA<StateError>());
    expect(ticks.waiting<bool>('a'), isNull);
    expect(saved, ['b']);
  });

  testWidgets('the app saves once the frame is drawn, not before it', (
    tester,
  ) async {
    await tester.pumpWidget(const SizedBox());
    var ran = false;
    TickSaves.afterFrame(() => ran = true);
    await Future<void>.value();
    expect(ran, isFalse);
    await tester.pump();
    expect(ran, isTrue);
  });

  test('a box drawn in two places has one key', () {
    expect(lineTickKey('l1', null), tickKey(line: 'l1'));
    expect(tickKey(frog: 'f'), isNot(tickKey(line: 'f')));
  });
}
