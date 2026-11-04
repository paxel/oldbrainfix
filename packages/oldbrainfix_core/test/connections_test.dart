import 'dart:io';
import 'dart:isolate';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

/// How long the first connection holds still where the second one may
/// get in between: long enough for the second to start, far below the
/// store's busy timeout it waits for a lock with.
const _hold = Duration(milliseconds: 500);

/// How long any wait for the other isolate lasts before the test fails
/// instead of hanging.
const _patience = Duration(seconds: 20);

/// The wall clock as the store's: the day as UTC shows it.
DateTime _wall(DateTime utc) => utc;

/// What the second connection does once told to go, in an isolate of its
/// own, as the Agenda's read beside the app's own store (#207).
enum _Job { append, frogs, sessions }

/// Opens a second connection to the database at [path] in an isolate of
/// its own; [go] runs its job there, and its answer is null or what the
/// job threw.
Future<({void Function() go, Future<String?> done})> _second(
  String path,
  _Job job,
) async {
  final answers = ReceivePort();
  await Isolate.spawn(_secondMain, (path, job, answers.sendPort));
  final events = answers.asBroadcastStream();
  final orders = await events.first.timeout(_patience) as SendPort;
  final done = events.first.timeout(_patience).then((answer) {
    answers.close();
    return answer as String?;
  });
  return (go: () => orders.send(true), done: done);
}

Future<void> _secondMain((String, _Job, SendPort) args) async {
  final (path, job, answers) = args;
  final store = Store.open(path);
  final orders = ReceivePort();
  answers.send(orders.sendPort);
  String? failed;
  try {
    await orders.first.timeout(_patience);
    switch (job) {
      case _Job.append:
        store.append('note:second', 'text', 'second');
      case _Job.frogs:
        Frogs(store).pickToday(_wall);
      case _Job.sessions:
        Sessions(store).pickToday(_wall);
    }
  } on Object catch (e) {
    failed = '$e';
  }
  store.close();
  answers.send(failed);
}

void main() {
  late Directory dir;
  late String path;

  setUp(() {
    dir = Directory.systemTemp.createTempSync('connections_test_');
    path = '${dir.path}/oldbrainfix.db';
  });
  tearDown(() => dir.deleteSync(recursive: true));

  /// A store on [path] whose clock, once [armed], lets [between] in the
  /// first time an append asks it for now: after what the append was
  /// worked out from was read, before it is written.
  ({
    Store store,
    void Function(void Function() between) arm,
    bool Function() letIn,
  })
  first() {
    void Function()? waiting;
    final store = Store.open(
      path,
      clock: () {
        final between = waiting;
        if (between != null &&
            StackTrace.current.toString().contains('Store.append')) {
          waiting = null;
          between();
        }
        return DateTime.now();
      },
    );
    return (
      store: store,
      arm: (between) => waiting = between,
      letIn: () => waiting == null,
    );
  }

  /// Tells the second connection to go, then holds still for [_hold].
  void Function() letIn(void Function() go) => () {
    go();
    sleep(_hold);
  };

  test('a batch holds the write lock from its start: an append on another '
      'connection waits for it, then takes the next number', () async {
    final a = Store.open(path);
    addTearDown(a.close);
    a.append('note:first', 'text', 'before');
    final b = await _second(path, _Job.append);
    a.batch(() {
      // Read in the batch, as a check before a write is.
      a.currentSeq();
      letIn(b.go)();
      a.append('note:first', 'text', 'after');
    });
    expect(await b.done, isNull);
    final numbers = [for (final e in a.allEntries()) e.dseq]..sort();
    expect(numbers, [1, 2, 3]);
    expect(a.current('note:first', 'text'), 'after');
    expect(a.current('note:second', 'text'), 'second');
  });

  test(
    'two connections picking today\'s Frogs at once write one pick',
    () async {
      final setup = Store.open(path);
      for (final text in ['Call mum', 'Tax return']) {
        Frogs(setup).spawn(Notes(setup).capture(text).id);
      }
      setup.close();
      final a = first();
      addTearDown(a.store.close);
      final b = await _second(path, _Job.frogs);
      a.arm(letIn(b.go));
      final picked = Frogs(a.store).pickToday(_wall);
      expect(a.letIn(), isTrue, reason: 'the second connection was let in');
      expect(await b.done, isNull);
      final picks = [
        for (final e in a.store.allEntries())
          if (e.field == FrogKeys.pick) e.value,
      ];
      expect(picks, hasLength(1));
      expect(picks.single, endsWith(picked.shown.map((f) => f.id).join(',')));
    },
  );

  test(
    'two connections picking today\'s Session at once write one pick',
    () async {
      final setup = Store.open(path);
      final notes = Notes(setup);
      final pool = [
        for (final text in ['Squats', 'Push-ups', 'Plank'])
          () {
            final id = notes.capture(text).id;
            Tallies(setup).addField(id, name: 'Times', unit: 'times');
            return id;
          }(),
      ];
      final now = DateTime.now().toUtc();
      Sessions(setup).spawn(
        notes.capture('Training').id,
        Schedule(
          start: Day(now.year, now.month, now.day),
          times: [ClockTime(23, 59)],
          rule: Recurrence(frequency: Frequency.daily),
        ),
        pool: pool,
        count: 2,
        rule: SessionRule.rolling,
      );
      setup.close();
      final a = first();
      addTearDown(a.store.close);
      final b = await _second(path, _Job.sessions);
      a.arm(letIn(b.go));
      final shown = Sessions(a.store).pickToday(_wall);
      expect(a.letIn(), isTrue, reason: 'the second connection was let in');
      expect(await b.done, isNull);
      expect(shown, hasLength(1));
      final picks = [
        for (final e in a.store.allEntries())
          if (e.field.startsWith(SessionKeys.pickPrefix)) e.value,
      ];
      expect(picks, hasLength(1));
    },
  );
}
