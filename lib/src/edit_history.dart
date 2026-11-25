import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// What a change typed into a text does, to tell when typing makes a
/// step of its own: letters put in, letters taken out, or both at once,
/// such as a word the keyboard corrects or a table's cell rewritten.
enum TypedChange { insert, delete, replace }

/// The Note editor's Undo and Redo (obf-1051): every change made since
/// it opened, step by step, its text, its table cells, its Lines and its
/// badges. Each step undoes and redoes itself the way the change was
/// made, through the same calls that made it, so an Undo writes the
/// earlier value as a new edit; a step with nothing left to bring back
/// does nothing.
///
/// Typing makes steps of its own ([typed]): what is typed joins the open
/// step until a pause of [typingPause], a word begins after a space or a
/// line break, typing turns into deleting or back, or anything else is
/// recorded. Changes made in one go, such as a Return in a name that
/// moves the rest into the description, or a Line split into two, are
/// one step.
///
/// A change of the whole text that is a step on its own, such as a
/// table's row moved, deleted or its rows sorted, runs through
/// [wholeStep]. Listeners hear when Undo or Redo becomes possible or no
/// longer is, not of every letter.
class EditHistory extends ChangeNotifier {
  /// How long typing rests before what follows is a step of its own.
  static const typingPause = Duration(seconds: 1);

  final _done = <_Step>[];
  final _undone = <_Step>[];

  /// The step typing goes into now, the last of [_done]; null once it
  /// ended.
  _Step? _typing;
  TypedChange? _typingChange;
  Timer? _pause;

  /// The step recorded in the turn running now, which more changes of
  /// the same turn join.
  _Step? _turn;

  /// Whether a step is being undone or redone: what it changes is no
  /// step of its own.
  var _running = false;

  /// Whether the change running now is a step of its own ([wholeStep]).
  var _whole = false;

  /// Whether something is typed that becomes a step once it is stored,
  /// such as a Line's words ([typedElsewhere]).
  var _pending = false;

  bool get canUndo => _done.isNotEmpty || _pending;
  bool get canRedo => _undone.isNotEmpty;

  /// Whether a step is being undone or redone now: the editor takes what
  /// it puts into its fields for no typing.
  bool get running => _running;

  /// Records a change done at once, such as a badge set or a Line
  /// checked: [undo] takes it back, [redo] makes it again.
  void record({required VoidCallback undo, required VoidCallback redo}) {
    if (_running) return;
    _endTyping();
    final step = _inTurn() ?? _push();
    step.parts.add((undo: undo, redo: redo));
  }

  /// Records a change typed into the text: [undo] puts the text back as
  /// it was before it, [redo] as it is after it. It joins the step
  /// typing goes into while that lasts; then only its [redo] counts, the
  /// step's [undo] being the one before its first change. [wordStart]
  /// says that it puts a letter right after a space or a line break.
  void typed({
    required VoidCallback undo,
    required VoidCallback redo,
    required TypedChange change,
    bool wordStart = false,
  }) {
    if (_running) return;
    final open = _typing;
    final joins =
        open != null &&
        identical(open, _done.lastOrNull) &&
        !_whole &&
        (identical(open, _turn) ||
            (change == _typingChange &&
                !(wordStart && change == TypedChange.insert)));
    if (joins) {
      final (undo: first, redo: _) = open.parts.last;
      open.parts.last = (undo: first, redo: redo);
    } else {
      _endTyping();
      final step = _inTurn() ?? _push();
      step.parts.add((undo: undo, redo: redo));
      _typing = step;
    }
    _typingChange = change;
    _pause?.cancel();
    _pause = Timer(typingPause, _endTyping);
    if (_whole) _endTyping();
  }

  /// Tells that something was typed that is stored as a step later, as
  /// a Line's words are once the cursor leaves it: Undo stores it first
  /// and takes it back.
  void typedElsewhere() {
    if (_running || _pending) return;
    final could = canUndo;
    _pending = true;
    if (could != canUndo) notifyListeners();
  }

  /// Runs [change], a change of a field's whole text such as a table's
  /// row deleted or moved, or its rows sorted (obf-1050, obf-1052), as a
  /// step of its own of the history over [context], if there is one:
  /// what was typed before it and what is typed after it are steps
  /// apart.
  static void wholeStep(BuildContext context, VoidCallback change) {
    final history = EditHistoryScope.of(context);
    if (history == null) return change();
    history
      .._endTyping()
      .._turn = null
      .._whole = true;
    try {
      change();
    } finally {
      history._whole = false;
      history._endTyping();
      history._turn = null;
    }
  }

  /// Takes back the last step.
  void undo() {
    _endTyping();
    _pending = false;
    if (_done.isEmpty) return _changed();
    final step = _done.removeLast();
    _run(() {
      for (final part in step.parts.reversed) {
        _safely(part.undo);
      }
    });
    _undone.add(step);
    _changed();
  }

  /// Makes the last step taken back again.
  void redo() {
    _endTyping();
    if (_undone.isEmpty) return;
    final step = _undone.removeLast();
    _run(() {
      for (final part in step.parts) {
        _safely(part.redo);
      }
    });
    _done.add(step);
    _changed();
  }

  /// Takes back every step, the last first: back to what was opened.
  /// Redo makes them again one by one.
  void revertAll() {
    _pending = false;
    while (_done.isNotEmpty) {
      undo();
    }
  }

  @override
  void dispose() {
    _pause?.cancel();
    super.dispose();
  }

  /// Ends the step typing goes into: what is typed next is a step of its
  /// own.
  void _endTyping() {
    _pause?.cancel();
    _pause = null;
    _typing = null;
    _typingChange = null;
  }

  /// The step this turn recorded already, which a change of the same
  /// turn joins.
  _Step? _inTurn() {
    final step = _turn;
    if (step == null || !identical(step, _done.lastOrNull)) {
      return null;
    }
    return step;
  }

  /// A new step on top; what was taken back is no longer made again.
  _Step _push() {
    final could = (canUndo, canRedo);
    _pending = false;
    final step = _Step();
    _done.add(step);
    _undone.clear();
    _turn = step;
    scheduleMicrotask(() {
      if (identical(_turn, step)) _turn = null;
    });
    if (could != (canUndo, canRedo)) notifyListeners();
    return step;
  }

  void _run(VoidCallback body) {
    _running = true;
    try {
      body();
    } finally {
      _running = false;
    }
  }

  /// Runs [part]; one the log refuses now, as a sync took away what it
  /// brings back or this Device may no longer change it, does nothing.
  static void _safely(VoidCallback part) {
    try {
      part();
    } on StateError {
      // Nothing left to bring back.
    } on ArgumentError {
      // Gone meanwhile.
    } on HiddenFromAiRefused {
      // A place it may not go back to now (#267).
    }
  }

  void _changed() => notifyListeners();
}

/// One step of the history: its parts, undone the last first.
class _Step {
  final parts = <({VoidCallback undo, VoidCallback redo})>[];
}

/// Hands the Note editor's [EditHistory] down to what is edited in it:
/// its fields, their tables and the Lines of its list.
class EditHistoryScope extends InheritedWidget {
  final EditHistory history;

  const EditHistoryScope({
    super.key,
    required this.history,
    required super.child,
  });

  /// The history of the editor around [context], if any; read once, so
  /// it does not listen.
  static EditHistory? of(BuildContext context) =>
      context.getInheritedWidgetOfExactType<EditHistoryScope>()?.history;

  @override
  bool updateShouldNotify(EditHistoryScope oldWidget) =>
      history != oldWidget.history;
}

/// What [after] does to [before]: letters put in, taken out or replaced,
/// and whether a letter went in right after a space or a line break, as
/// [EditHistory.typed] takes them; null when the texts are the same.
({TypedChange change, bool wordStart})? typedChange(
  String before,
  String after,
) {
  if (before == after) return null;
  var head = 0;
  final most = before.length < after.length ? before.length : after.length;
  while (head < most && before.codeUnitAt(head) == after.codeUnitAt(head)) {
    head++;
  }
  var tail = 0;
  while (tail < most - head &&
      before.codeUnitAt(before.length - 1 - tail) ==
          after.codeUnitAt(after.length - 1 - tail)) {
    tail++;
  }
  final removed = before.length - head - tail;
  final inserted = after.substring(head, after.length - tail);
  final change = removed == 0
      ? TypedChange.insert
      : inserted.isEmpty
      ? TypedChange.delete
      : TypedChange.replace;
  bool space(String c) => c.trim().isEmpty;
  final wordStart =
      inserted.isNotEmpty &&
      !space(inserted[0]) &&
      head > 0 &&
      space(after[head - 1]);
  return (change: change, wordStart: wordStart);
}
