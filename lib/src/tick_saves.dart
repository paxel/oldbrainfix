import 'package:flutter/scheduler.dart';
import 'package:flutter/widgets.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'failure_dialog.dart';
import 'l10n.dart';

/// Ticks shown before they are saved (#338, Q752): a tap ticks its box,
/// and strikes its Line, in the frame the tap lands in; the save follows
/// once that frame is drawn, so the page waits on neither the write nor
/// the pages that read the log again after it. The saves run in the
/// order the boxes were tapped, each with the last tap on its box: two
/// quick taps on one box end as the second left it, and a box tapped
/// while another waits is saved too. A save that throws runs its
/// `failed`: the box shows what is stored again, and the page says so
/// ([announceTickFailed]).
final class TickSaves {
  TickSaves._();

  /// The one of the app: a box drawn in two places shows its tap in both.
  static final instance = TickSaves._();

  /// Hands the waiting saves to run once the frame of the tap is drawn.
  /// A test holds them back to see that frame with the save still
  /// waiting.
  @visibleForTesting
  static void Function(VoidCallback saves) after = afterFrame;

  /// Runs [saves] after the next frame, and asks for that frame.
  static void afterFrame(VoidCallback saves) {
    SchedulerBinding.instance
      ..addPostFrameCallback((_) => saves())
      ..ensureVisualUpdate();
  }

  /// Runs one save. A test makes it throw, as a full storage would.
  @visibleForTesting
  static void Function(VoidCallback save) run = _run;

  static void _run(VoidCallback save) => save();

  final _waiting = <String, _Waiting>{};
  var _queued = false;

  /// What the last tap on [key] ticked it to while its save waits; null
  /// when nothing waits there.
  T? waiting<T extends Object>(String key) => _waiting[key]?.to as T?;

  /// Ticks [key] to [to] at once, as [waiting] says, and runs [save]
  /// after the frame; a tap on a box that waits keeps its place in the
  /// order and replaces its save. When [save] throws, [failed] runs
  /// with what went wrong, once [key] no longer waits.
  void tick(
    String key,
    Object to, {
    required VoidCallback save,
    required void Function(Object error) failed,
  }) {
    _waiting[key] = _Waiting(to, save, failed);
    if (_queued) return;
    _queued = true;
    after(_saveAll);
  }

  /// Saves what waits, also what is ticked as it saves.
  void _saveAll() {
    try {
      while (_waiting.isNotEmpty) {
        final key = _waiting.keys.first;
        final waiting = _waiting.remove(key);
        if (waiting == null) continue;
        try {
          run(waiting.save);
        } on Object catch (error) {
          waiting.failed(error);
        }
      }
    } finally {
      _queued = false;
    }
  }

  /// Forgets what waits, unsaved: for a test's tear-down.
  @visibleForTesting
  void clear() {
    _waiting.clear();
    _queued = false;
  }
}

class _Waiting {
  final Object to;
  final VoidCallback save;
  final void Function(Object error) failed;

  _Waiting(this.to, this.save, this.failed);
}

/// The key a box is ticked under in [TickSaves], the same wherever it is
/// drawn: a Routine's Occurrence, a Line (on an Occurrence), a Frog, or a
/// box in the text of a Note or Section.
String tickKey({
  String? routine,
  Occurrence? occurrence,
  String? line,
  String? frog,
  String? entity,
  String? box,
}) => [routine, occurrence?.key, line, frog, entity, box].join('|');

/// The key of the Line [line] ticked on the Occurrence [on], or on none.
String lineTickKey(String line, RoutineOccurrence? on) =>
    tickKey(routine: on?.routineId, occurrence: on?.occurrence, line: line);

/// A page whose boxes tick at once (#338): [tickAtOnce] draws the tap in
/// the next frame and saves it after that frame; a save that fails draws
/// what is stored again and says so at the top.
mixin TicksAtOnce<T extends StatefulWidget> on State<T> {
  /// Ticks [key] to [to] at once and runs [save] after the frame; when it
  /// throws, [failed] runs first, then the page draws again and says so.
  void tickAtOnce(
    String key,
    Object to,
    VoidCallback save, {
    VoidCallback? failed,
  }) {
    TickSaves.instance.tick(
      key,
      to,
      save: save,
      failed: (error) {
        failed?.call();
        if (!mounted) return;
        setState(() {});
        announceTickFailed(context, error);
      },
    );
    setState(() {});
  }
}

/// Says at the top that a tick could not be saved; its tap opens the
/// cause and the fix: refused, when a sync took the box or the role away
/// while it waited, else the storage.
void announceTickFailed(BuildContext context, Object error) {
  final t = context.t;
  final refused = error is ArgumentError || error is StateError;
  announceFailure(
    context,
    t.tickFailed,
    title: t.tickFailedTitle,
    cause: refused ? t.tickRefusedCause : t.tickStorageCause,
    fix: refused ? t.tickRefusedFix : t.tickStorageFix,
    details: '$error',
  );
}
