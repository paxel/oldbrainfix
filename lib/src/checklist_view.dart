import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'browser.dart';
import 'l10n.dart';
import 'line_editing.dart';
import 'note_editor_screen.dart';
import 'routine_editor_screen.dart';
import 'two_row_text_field.dart';

/// A Line as it is drawn: its id, its text, whether it is in the group of
/// the Line above it, whether it shows checked and what shows dimmed under
/// its text, such as the day a Shopping List's Line comes back (#122).
typedef ShownLine = ({
  String id,
  String text,
  bool inGroup,
  bool checked,
  String? below,
});

/// The Lines of [list] as they are drawn.
List<ShownLine> shownLines(Checklist list) => [
  for (final line in list.lines)
    (
      id: line.id,
      text: line.text,
      inGroup: line.inGroup,
      checked: line.checked,
      below: null,
    ),
];

/// Whether this Device may check the Lines of [list] as it shows: for
/// its Occurrence with a role that checks off its Routine (#121), else
/// with one in its Note's Area.
bool mayCheckShown(Checklists lists, Checklist list) => switch (list.on) {
  final on? => lists.store.mayCheckOff(on.routineId),
  null => lists.mayCheck(list.noteId),
};

/// What a Note's card knows of its Checklist (#120): [read] gives it as
/// it shows now (for the Occurrence it is on, #121), or null for a Note
/// that is none, read when the card is, never while building; [lists]
/// checks its Lines.
typedef NoteChecklist = ({Checklist? Function() read, Checklists lists});

/// The Lines of a Checklist, each with its checkbox and its text, struck
/// through in place once checked; the group of a Line is drawn as one
/// frame under it. A checkbox checks with [onCheck] and is disabled while
/// it is null. With [onMenu] a Line whose id [editable] holds has ⋮, and
/// a long press on it opens the same menu where the finger is. A Line
/// [field] gives a field for is typed in that field, in place of its
/// text (#141), and its ⋮ is the only way to its menu: a long press in
/// the field selects.
class ChecklistLines extends StatelessWidget {
  final List<ShownLine> lines;
  final void Function(String id, bool checked)? onCheck;
  final void Function(String id, Offset at)? onMenu;
  final Set<String> editable;
  final Widget? Function(ShownLine line)? field;

  const ChecklistLines({
    super.key,
    required this.lines,
    this.onCheck,
    this.onMenu,
    this.editable = const {},
    this.field,
  });

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final rows = <Widget>[];
    var group = <Widget>[];
    String? parent;
    void closeGroup() {
      if (group.isEmpty) return;
      rows.add(
        Container(
          key: ValueKey('checklist-group-$parent'),
          margin: const EdgeInsets.only(left: 20, bottom: 4),
          decoration: BoxDecoration(
            border: Border.all(color: scheme.outlineVariant),
            borderRadius: BorderRadius.circular(12),
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: group,
          ),
        ),
      );
      group = [];
    }

    for (final line in lines) {
      final row = _LineRow(
        line: line,
        onCheck: onCheck,
        onMenu: editable.contains(line.id) ? onMenu : null,
        field: field?.call(line),
      );
      if (line.inGroup) {
        group.add(row);
      } else {
        closeGroup();
        parent = line.id;
        rows.add(row);
      }
    }
    closeGroup();
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: rows,
    );
  }
}

/// The style of a Line's text: struck through once [checked].
TextStyle? lineStyle(BuildContext context, {required bool checked}) {
  final theme = Theme.of(context);
  final style = theme.textTheme.bodyLarge;
  return checked
      ? style?.copyWith(
          decoration: TextDecoration.lineThrough,
          color: theme.colorScheme.onSurfaceVariant,
        )
      : style;
}

/// One Line: its checkbox, its text or [field] and, with [onMenu], ⋮.
class _LineRow extends StatelessWidget {
  final ShownLine line;
  final void Function(String id, bool checked)? onCheck;
  final void Function(String id, Offset at)? onMenu;
  final Widget? field;

  const _LineRow({required this.line, this.onCheck, this.onMenu, this.field});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final check = onCheck;
    final menu = onMenu;
    final field = this.field;
    final box = Checkbox(
      key: ValueKey('checklist-check-${line.id}'),
      value: line.checked,
      onChanged: check == null ? null : (on) => check(line.id, on ?? false),
    );
    final text = Expanded(
      child: Padding(
        padding: EdgeInsets.symmetric(vertical: field == null ? 8 : 0),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            field ??
                Text(
                  line.text,
                  key: ValueKey('checklist-line-${line.id}'),
                  style: lineStyle(context, checked: line.checked),
                ),
            if (line.below case final below?)
              Text(
                below,
                key: ValueKey('checklist-below-${line.id}'),
                style: theme.textTheme.bodySmall?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
          ],
        ),
      ),
    );
    // The checkbox reads with its text; ⋮ stays a button of its own, and
    // so does a field.
    final row = Row(
      children: [
        Expanded(
          child: field == null
              ? MergeSemantics(child: Row(children: [box, text]))
              : Row(children: [box, text]),
        ),
        if (menu != null)
          Builder(
            builder: (button) => IconButton(
              key: ValueKey('checklist-menu-${line.id}'),
              tooltip: context.t.more,
              icon: const Icon(Icons.more_vert),
              onPressed: () {
                final box = button.findRenderObject() as RenderBox?;
                if (box == null) return;
                menu(line.id, box.localToGlobal(box.size.center(Offset.zero)));
              },
            ),
          ),
      ],
    );
    if (menu == null || field != null) return row;
    return GestureDetector(
      onLongPressStart: (details) => menu(line.id, details.globalPosition),
      child: row,
    );
  }
}

/// A Checklist on a Note's card (#120): every Line with its checkbox on a
/// [full] card, checkable where this Device may, else how many of its
/// Lines are checked. Nothing for a Note that is none. It reads the
/// Checklist whenever the card is built anew, and again after a check.
class CardChecklist extends StatefulWidget {
  final String noteId;
  final NoteChecklist checklist;
  final bool full;

  const CardChecklist({
    super.key,
    required this.noteId,
    required this.checklist,
    required this.full,
  });

  @override
  State<CardChecklist> createState() => _CardChecklistState();
}

class _CardChecklistState extends State<CardChecklist> {
  Checklist? _list;
  var _checks = false;

  @override
  void initState() {
    super.initState();
    _read();
  }

  @override
  void didUpdateWidget(CardChecklist old) {
    super.didUpdateWidget(old);
    // A check changes a Line, not the Note, so this reads every time.
    _read();
  }

  void _read() {
    final list = _list = widget.checklist.read();
    _checks = list != null && mayCheckShown(widget.checklist.lists, list);
  }

  void _check(String id, bool checked) {
    final lists = widget.checklist.lists;
    final on = _list?.on;
    checked ? lists.check(id, on: on) : lists.uncheck(id, on: on);
    // Read anew, not from the list of cards, which still holds the old.
    setState(() {
      final list = _list = lists.shownNow(widget.noteId, deviceWallClock);
      _checks = list != null && mayCheckShown(lists, list);
    });
  }

  @override
  Widget build(BuildContext context) {
    final list = _list;
    if (list == null) return const SizedBox.shrink();
    if (!widget.full) {
      return Padding(
        padding: const EdgeInsets.only(top: 4),
        child: Text(
          context.t.checklistCount(list.checkedCount, list.count),
          key: const ValueKey('checklist-count'),
          style: Theme.of(context).textTheme.bodyMedium,
        ),
      );
    }
    return Padding(
      padding: const EdgeInsets.only(top: 8),
      child: ChecklistLines(
        key: const ValueKey('checklist-card'),
        lines: shownLines(list),
        onCheck: _checks ? _check : null,
      ),
    );
  }
}

/// The Checklist part of a Note's editor (#120), nothing while the Note
/// is none: its Lines with their boxes, each typed in place where this
/// Device may change it (#141). Return splits a Line at the cursor, or at
/// its end starts a new empty one under it; Backspace at its start joins
/// it with the Line above, whose box stays, and an empty one goes; "in"
/// and "out" on the bar above the keyboard put it under the Line above
/// or back at the top. "Add Line" starts an empty one at the end for
/// anyone with a role in its Area. ⋮ of a Line it may change has Move
/// up, Move down, putting it under the Line above or back to the top
/// level, and Delete; "Uncheck all" shows while one is checked. "Back
/// to a plain Note" is in the editor's kind badge (#160). What is typed is
/// stored as the cursor leaves a Line and whenever [typed] says so; a Line left empty
/// goes as the editor is left. It follows the log as it changes.
class ChecklistSection extends StatefulWidget {
  final Notes notes;
  final String noteId;

  /// Tells it when the editor saves and when it is left.
  final TypedLines? typed;

  /// Whether a list without Lines starts one, the cursor in it, as
  /// Add Line does (#161).
  final bool startLine;

  const ChecklistSection({
    super.key,
    required this.notes,
    required this.noteId,
    this.typed,
    this.startLine = false,
  });

  @override
  State<ChecklistSection> createState() => _ChecklistSectionState();
}

/// What the menu of a Line does.
enum _LineStep { up, down, indent, outdent, delete }

class _ChecklistSectionState extends State<ChecklistSection>
    with ReloadsOnLogChange, AutomaticKeepAliveClientMixin {
  late final _lists = Checklists(widget.notes.store);
  late final _twoRows = widget.notes.store.twoRowEditorOn;
  Checklist? _list;
  var _checks = false;
  var _adds = false;
  Set<String> _editable = const {};

  /// The Lines as they are typed, in the order they show, the new ones
  /// not stored yet among them.
  List<TypedLine> _rows = const [];

  /// How many new Lines were started, to tell them apart.
  var _started = 0;

  @override
  Store get log => widget.notes.store;

  @override
  void reloadLog() => setState(_load);

  /// Kept while scrolled away, so nothing typed goes with it.
  @override
  bool get wantKeepAlive => true;

  @override
  void initState() {
    super.initState();
    _load();
    widget.typed?.listen(_keepTyped);
    if (widget.startLine && _adds && _rows.isEmpty) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) _addEmpty();
      });
    }
  }

  @override
  void didUpdateWidget(ChecklistSection old) {
    super.didUpdateWidget(old);
    if (old.typed != widget.typed) {
      old.typed?.forget(_keepTyped);
      widget.typed?.listen(_keepTyped);
    }
  }

  @override
  void dispose() {
    widget.typed?.forget(_keepTyped);
    for (final row in _rows) {
      row.dispose();
    }
    super.dispose();
  }

  void _load() {
    final list = _list = _lists.shownNow(widget.noteId, deviceWallClock);
    _checks = list != null && mayCheckShown(_lists, list);
    _adds = list != null && _lists.mayCheck(widget.noteId);
    _editable = {
      for (final line in list?.lines ?? const <Line>[])
        if (_lists.mayEdit(line.id)) line.id,
    };
    _rows = _arrange(list);
  }

  /// The rows for the Lines of [list]: those there already kept with what
  /// is typed in them, a new one after the Line it was started under, a
  /// row whose Line went while typed in kept as a new one.
  List<TypedLine> _arrange(Checklist? list) {
    final old = _rows;
    final stored = {for (final row in old) ?row.id: row};
    final after = <String?, List<TypedLine>>{};
    String? above;
    for (final row in old) {
      final id = row.id;
      if (id != null && list?.line(id) != null) {
        above = id;
        continue;
      }
      if (id != null) {
        stored.remove(id);
        if (!row.changed || row.typed.isEmpty) {
          _retire(row);
          continue;
        }
        row
          ..id = null
          ..kept = '';
      }
      after.putIfAbsent(above, () => []).add(row);
    }
    if (list == null) {
      for (final row in [...stored.values, ...after.values.expand((r) => r)]) {
        _retire(row);
      }
      return const [];
    }
    final rows = <TypedLine>[...?after.remove(null)];
    for (final line in list.lines) {
      final row = stored.remove(line.id) ?? _row(id: line.id, text: line.text);
      row
        ..inGroup = line.inGroup
        ..take(line.text);
      rows
        ..add(row)
        ..addAll(after.remove(line.id) ?? const []);
    }
    // Started under a Line that went: at the end.
    for (final rest in after.values) {
      rows.addAll(rest);
    }
    for (final row in stored.values) {
      _retire(row);
    }
    return rows;
  }

  /// A row for the Line [id] holding [text], or a new one.
  TypedLine _row({String? id, String text = '', bool inGroup = false}) {
    final row = TypedLine(
      id: id,
      kept: text,
      inGroup: inGroup,
      marked: true,
      started: id == null ? ++_started : 0,
    );
    row.text.addListener(() => _changed(row));
    row.focus.addListener(() {
      if (!row.focus.hasFocus) scheduleMicrotask(() => _left(row));
    });
    return row;
  }

  /// Disposes [row] once its field is gone.
  void _retire(TypedLine row) {
    WidgetsBinding.instance.addPostFrameCallback((_) => row.dispose());
  }

  /// The id [row] shows under: its Line's, or one of its own while new.
  static String _shownId(TypedLine row) => row.id ?? 'new-${row.started}';

  TypedLine? _rowShown(String id) {
    for (final row in _rows) {
      if (_shownId(row) == id) return row;
    }
    return null;
  }

  /// Whether the Note is a Checklist out of the trash, to write to.
  bool get _open =>
      !(widget.notes.byId(widget.noteId)?.isTrashed ?? true) &&
      _lists.isChecklist(widget.noteId);

  /// Whether the Line [id] still shows on the list.
  bool _shows(String id) => _lists.byNote(widget.noteId)?.line(id) != null;

  /// How many stored Lines show before [row]: where it is added.
  int _storedBefore(TypedLine row) {
    var count = 0;
    for (final r in _rows) {
      if (identical(r, row)) break;
      if (r.id != null) count++;
    }
    return count;
  }

  /// Whether [row] is a Line with a group under it.
  bool _hasGroup(TypedLine row) => switch (row.id) {
    final id? => _list?.groupOf(id).isNotEmpty ?? false,
    null => false,
  };

  /// Stores what was typed into [row]: a new Line once it holds text, a
  /// Line renamed once its text changed. An empty one waits.
  void _store(TypedLine row) {
    final typed = row.typed;
    if (typed.isEmpty || !row.changed || !_open) return;
    switch (row.id) {
      case null:
        if (!_lists.mayCheck(widget.noteId)) return;
        row.id = _lists.add(
          widget.noteId,
          typed,
          at: _storedBefore(row),
          indent: row.inGroup,
        );
      case final id:
        if (!_shows(id) || !_lists.mayEdit(id)) return;
        _lists.rename(id, typed);
    }
    row.kept = typed;
  }

  /// The cursor left [row].
  void _left(TypedLine row) {
    if (!mounted || !_rows.contains(row) || !row.changed) return;
    _store(row);
  }

  /// Stores what was typed; [leaving], the Lines left empty go too.
  void _keepTyped({required bool leaving}) {
    if (!mounted) return;
    final typed = [
      for (final row in _rows)
        if (row.changed && row.typed.isNotEmpty) row,
    ];
    if (typed.isNotEmpty) {
      widget.notes.store.batch(() {
        for (final row in typed) {
          _store(row);
        }
      });
    }
    if (!leaving) return;
    final empty = [
      for (final row in _rows)
        if (row.typed.isEmpty) row,
    ];
    if (empty.isEmpty) return;
    final gone = [
      for (final row in empty)
        if (row.id case final id? when _shows(id) && _lists.mayEdit(id)) id,
    ];
    if (gone.isNotEmpty && _open) {
      widget.notes.store.batch(() {
        for (final id in gone) {
          _lists.delete(id);
        }
      });
    }
    setState(() {
      _rows = [
        for (final row in _rows)
          if (!empty.contains(row)) row,
      ];
    });
    empty.forEach(_retire);
  }

  /// Keeps the text of [row] after [lineStart], and the cursor there;
  /// takes a Return as a split and a Backspace at its start as a join.
  void _changed(TypedLine row) {
    if (row.setting) return;
    final value = row.text.value;
    final before = row.last;
    row.last = value;
    final text = value.text;
    if (!text.startsWith(lineStart)) {
      final selection = before.selection;
      if (before.text == '$lineStart$text' &&
          selection.isCollapsed &&
          selection.baseOffset <= 1) {
        scheduleMicrotask(() => _join(row));
        return;
      }
      // Set as a whole, by a paste over all or the system.
      int moved(int offset) => offset < 0 ? offset : offset + 1;
      final now = value.selection;
      row.set(
        TextEditingValue(
          text: '$lineStart$text',
          selection: now.isValid
              ? now.copyWith(
                  baseOffset: moved(now.baseOffset),
                  extentOffset: moved(now.extentOffset),
                )
              : TextSelection.collapsed(offset: text.length + 1),
          composing: value.composing.isValid
              ? TextRange(
                  start: value.composing.start + 1,
                  end: value.composing.end + 1,
                )
              : TextRange.empty,
        ),
      );
    }
    if (text.contains('\n') || text.contains('\r')) {
      scheduleMicrotask(() => _split(row));
      return;
    }
    final selection = value.selection;
    if (text.startsWith(lineStart) &&
        selection.isValid &&
        (selection.baseOffset < 1 || selection.extentOffset < 1)) {
      row.set(
        value.copyWith(
          selection: selection.copyWith(
            baseOffset: math.max(1, selection.baseOffset),
            extentOffset: math.max(1, selection.extentOffset),
          ),
        ),
      );
    }
  }

  /// Splits [row] at the Return typed: the text before stays, the text
  /// after becomes a new Line under it, an empty one at its end; a Line
  /// with a group gets it as the first of its group. At its start, an
  /// empty Line goes in above and the cursor stays. Pasted lines become
  /// a Line each.
  void _split(TypedLine row) {
    if (!mounted || !_rows.contains(row)) return;
    final parts = row.text.text
        .replaceAll(lineStart, '')
        .split(RegExp('\r\n|\r|\n'));
    final filled = [
      for (final part in parts)
        if (part.trim().isNotEmpty) part.trim(),
    ];
    if (filled.isEmpty) {
      // Return in an empty Line: it is the new one already.
      row.set(
        const TextEditingValue(
          text: lineStart,
          selection: TextSelection.collapsed(offset: 1),
        ),
      );
      return;
    }
    final at = _rows.indexOf(row);
    if (parts.length == 2 && parts.first.trim().isEmpty) {
      row.set(
        TextEditingValue(
          text: '$lineStart${parts.last}',
          selection: const TextSelection.collapsed(offset: 1),
        ),
      );
      setState(() {
        _rows = [..._rows]..insert(at, _row(inGroup: row.inGroup));
      });
      return;
    }
    final group = row.inGroup || _hasGroup(row);
    final first = filled.first;
    row.set(
      TextEditingValue(
        text: '$lineStart$first',
        selection: TextSelection.collapsed(offset: 1 + first.length),
      ),
    );
    final added = <TypedLine>[];
    widget.notes.store.batch(() {
      _store(row);
      if (row.id == null) return;
      var index = _storedBefore(row) + 1;
      for (final text in filled.skip(1)) {
        final id = _lists.add(widget.noteId, text, at: index++, indent: group);
        added.add(_row(id: id, text: text, inGroup: group));
      }
    });
    if (row.id == null) return;
    final ends = parts.last.trim().isEmpty;
    if (ends) added.add(_row(inGroup: group));
    if (added.isEmpty) return;
    setState(() {
      _rows = [..._rows]..insertAll(at + 1, added);
    });
    final target = added.last;
    _focusOn(target, parts.length == 2 || ends ? 0 : target.typed.length);
  }

  /// Joins [row] with the Line above at a Backspace at its start: its
  /// text goes to the end of the Line above, which keeps its box and
  /// state, and the Line [row] is deleted, its checks staying in the
  /// log; its group stays a group. An empty Line simply goes.
  void _join(TypedLine row) {
    if (!mounted || !_rows.contains(row)) return;
    final at = _rows.indexOf(row);
    final text = row.typed;
    void restore() => row.set(
      TextEditingValue(
        text: '$lineStart${row.text.text}',
        selection: const TextSelection.collapsed(offset: 1),
      ),
    );
    if (text.isEmpty) {
      _drop(row);
      return;
    }
    final upper = at > 0 ? _rows[at - 1] : null;
    final upperId = upper?.id;
    if (upper == null || (upperId != null && !_editable.contains(upperId))) {
      restore();
      return;
    }
    final head = upper.typed;
    final group = switch (row.id) {
      final id? => [
        for (final line in _list?.groupOf(id) ?? const <Line>[])
          if (_lists.mayEdit(line.id)) line.id,
      ],
      null => const <String>[],
    };
    upper.set(
      TextEditingValue(
        text: '$lineStart$head$text',
        selection: TextSelection.collapsed(offset: 1 + head.length),
      ),
    );
    widget.notes.store.batch(() {
      _store(upper);
      if (row.id case final id? when _open && _shows(id)) {
        _lists.delete(id);
        for (final line in group) {
          _lists.indent(line);
        }
      }
    });
    setState(() {
      _rows = [..._rows]..remove(row);
    });
    _retire(row);
    _focusOn(upper, head.length);
  }

  /// Takes the empty [row] away, deleting its Line; the cursor goes to
  /// the end of the Line above, or to the start of the next.
  void _drop(TypedLine row) {
    final at = _rows.indexOf(row);
    if (row.id case final id? when _open && _shows(id)) {
      if (!_lists.mayEdit(id)) return;
      _lists.delete(id);
    }
    setState(() {
      _rows = [..._rows]..remove(row);
    });
    _retire(row);
    if (at > 0) {
      final above = _rows[at - 1];
      _focusOn(above, above.text.text.length);
    } else if (_rows.isNotEmpty) {
      _focusOn(_rows.first, 0);
    }
  }

  /// Puts the cursor into [row] at [offset] of its text.
  void _focusOn(TypedLine row, int offset) {
    row.cursorAt(offset);
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted && _rows.contains(row)) row.focus.requestFocus();
    });
  }

  /// "Add Line": an empty Line at the end, the cursor in it.
  void _addEmpty() {
    final last = _rows.lastOrNull;
    if (last != null && last.id == null && last.typed.isEmpty) {
      _focusOn(last, 0);
      return;
    }
    final row = _row();
    setState(() => _rows = [..._rows, row]);
    _focusOn(row, 0);
  }

  /// Puts [row] under the Line above, or with [under] false back at the
  /// top.
  void _indent(TypedLine row, {required bool under}) {
    final at = _rows.indexOf(row);
    if (at < 0) return;
    switch (row.id) {
      case null:
        setState(() => row.inGroup = under && at > 0);
      case final id:
        if (!_open || !_shows(id) || !_lists.mayEdit(id)) return;
        if (under && _lists.byNote(widget.noteId)?.lines.first.id == id) {
          return;
        }
        _lists.indent(id, under: under);
        setState(_load);
    }
  }

  void _check(String id, bool checked) {
    var line = id;
    if (_rowShown(id) case final row? when row.id == null) {
      // A new Line is stored first; an empty one has nothing to check.
      _store(row);
      final stored = row.id;
      if (stored == null) return;
      line = stored;
    }
    final on = _list?.on;
    checked ? _lists.check(line, on: on) : _lists.uncheck(line, on: on);
    setState(_load);
  }

  void _uncheckAll() {
    _lists.uncheckAll(widget.noteId, on: _list?.on);
    setState(_load);
  }

  /// The steps the menu of [line] offers, as [list] stands.
  List<_LineStep> _steps(Checklist list, Line line) {
    final lines = list.lines;
    final at = lines.indexWhere((l) => l.id == line.id);
    final (before, after) = _neighbours(list, line);
    return [
      if (before != null) _LineStep.up,
      if (after) _LineStep.down,
      if (!line.inGroup && at > 0) _LineStep.indent,
      if (line.inGroup) _LineStep.outdent,
      _LineStep.delete,
    ];
  }

  /// The Line [line] goes before when it moves up, or null at the top of
  /// its level; and whether it can move down: Lines of a group move
  /// within it, Lines at the top past the next one with its group.
  (String?, bool) _neighbours(Checklist list, Line line) {
    final level = _level(list, line);
    final at = level.indexWhere((l) => l.id == line.id);
    return (at > 0 ? level[at - 1].id : null, at < level.length - 1);
  }

  /// The Lines on the level of [line]: its group, or the top.
  List<Line> _level(Checklist list, Line line) => switch (line.parent) {
    final parent? => list.groupOf(parent),
    null => [
      for (final l in list.lines)
        if (!l.inGroup) l,
    ],
  };

  void _moveDown(Checklist list, Line line) {
    final level = _level(list, line);
    final at = level.indexWhere((l) => l.id == line.id);
    if (at + 2 < level.length) {
      _lists.move(line.id, before: level[at + 2].id);
      return;
    }
    if (!line.inGroup) {
      _lists.move(line.id);
      return;
    }
    // Last of its group: after the group's last Line, under it again.
    final lines = list.lines;
    final last = lines.indexWhere((l) => l.id == level.last.id);
    _lists.move(
      line.id,
      before: last + 1 < lines.length ? lines[last + 1].id : null,
    );
    _lists.indent(line.id);
  }

  Future<void> _menu(String id, Offset at) async {
    // What was typed is stored first, so the menu sees the list as shown.
    _keepTyped(leaving: false);
    final list = _lists.shownNow(widget.noteId, deviceWallClock);
    final line = list?.line(id);
    if (list == null || line == null) return;
    final t = context.t;
    final overlay =
        Overlay.of(context).context.findRenderObject() as RenderBox?;
    if (overlay == null) return;
    final step = await showMenu<_LineStep>(
      context: context,
      position: RelativeRect.fromRect(
        overlay.globalToLocal(at) & Size.zero,
        Offset.zero & overlay.size,
      ),
      items: [
        for (final step in _steps(list, line))
          PopupMenuItem(
            key: ValueKey('checklist-${step.name}'),
            value: step,
            child: Row(
              children: [
                Icon(switch (step) {
                  _LineStep.up => Icons.arrow_upward,
                  _LineStep.down => Icons.arrow_downward,
                  _LineStep.indent => Icons.format_indent_increase,
                  _LineStep.outdent => Icons.format_indent_decrease,
                  _LineStep.delete => Icons.delete_outline,
                }),
                const SizedBox(width: 12),
                Flexible(
                  child: Text(switch (step) {
                    _LineStep.up => t.checklistMoveUp,
                    _LineStep.down => t.checklistMoveDown,
                    _LineStep.indent => t.checklistIndent,
                    _LineStep.outdent => t.checklistOutdent,
                    _LineStep.delete => t.delete,
                  }),
                ),
              ],
            ),
          ),
      ],
    );
    if (step == null || !mounted) return;
    // A sync while the menu was open may have taken it away.
    if (!_open || !_shows(id)) return;
    switch (step) {
      case _LineStep.up:
        if (_neighbours(list, line).$1 case final before?) {
          _lists.move(id, before: before);
        }
      case _LineStep.down:
        _moveDown(list, line);
      case _LineStep.indent:
        _lists.indent(id);
      case _LineStep.outdent:
        _lists.indent(id, under: false);
      case _LineStep.delete:
        if (!await confirmLineDelete(context)) return;
        if (!mounted) return;
        if (!_open || !_shows(id)) return;
        _lists.delete(id);
    }
    setState(_load);
  }

  /// The field of [shown] where it is typed in place: a new Line, or one
  /// this Device may change.
  Widget? _field(ShownLine shown) {
    final row = _rowShown(shown.id);
    if (row == null) return null;
    final id = row.id;
    if (id != null && !_editable.contains(id)) return null;
    final t = context.t;
    final at = _rows.indexOf(row);
    return KeyedSubtree(
      key: ValueKey('checklist-field-${shown.id}'),
      child: TwoRowTextField(
        // The same field while the Line moves, in a group or out.
        key: GlobalObjectKey(row),
        controller: row.text,
        focusNode: row.focus,
        twoRows: _twoRows,
        style: lineStyle(context, checked: shown.checked),
        decoration: const InputDecoration(
          filled: false,
          border: InputBorder.none,
          isDense: true,
          contentPadding: EdgeInsets.symmetric(vertical: 12),
        ),
        actions: [
          (
            key: 'line-indent',
            label: t.checklistIndent,
            icon: Icons.format_indent_increase,
            run: at > 0 && !row.inGroup
                ? () => _indent(row, under: true)
                : null,
          ),
          (
            key: 'line-outdent',
            label: t.checklistOutdent,
            icon: Icons.format_indent_decrease,
            run: row.inGroup ? () => _indent(row, under: false) : null,
          ),
        ],
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    super.build(context);
    final list = _list;
    if (list == null) return const SizedBox.shrink();
    final t = context.t;
    final uncheck = _checks && list.checkedCount > 0;
    // Under the text field, as the Lines are under the text on a card.
    return Padding(
      padding: const EdgeInsets.only(top: 16),
      child: EditorPanel(
        key: const ValueKey('checklist'),
        heading: t.checklistHeading,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            ChecklistLines(
              lines: [
                for (final row in _rows)
                  (
                    id: _shownId(row),
                    text: row.kept,
                    inGroup: row.inGroup,
                    checked: switch (row.id) {
                      final id? => list.line(id)?.checked ?? false,
                      null => false,
                    },
                    below: null,
                  ),
              ],
              onCheck: _checks ? _check : null,
              onMenu: _menu,
              editable: _editable,
              field: _adds ? _field : null,
            ),
            if (_adds) ...[
              const SizedBox(height: 8),
              Align(
                alignment: Alignment.centerLeft,
                child: FilledButton.tonalIcon(
                  key: const ValueKey('checklist-add'),
                  onPressed: _addEmpty,
                  icon: const Icon(Icons.add),
                  label: Text(t.checklistAddLine),
                ),
              ),
            ],
            if (uncheck) ...[
              const SizedBox(height: 8),
              Align(
                alignment: Alignment.centerLeft,
                child: OutlinedButton.icon(
                  key: const ValueKey('checklist-uncheck-all'),
                  onPressed: _uncheckAll,
                  icon: const Icon(Icons.check_box_outline_blank),
                  label: Text(t.checklistUncheckAll),
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }
}

/// Asks once whether a Line goes, with Delete on the button in the
/// error colour.
Future<bool> confirmLineDelete(BuildContext context) async {
  final sure = await showDialog<bool>(
    context: context,
    builder: (context) {
      final t = context.t;
      return AlertDialog(
        content: Text(t.checklistDeleteQuestion),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: Text(t.cancel),
          ),
          TextButton(
            key: const ValueKey('checklist-delete-confirm'),
            style: TextButton.styleFrom(
              foregroundColor: Theme.of(context).colorScheme.error,
            ),
            onPressed: () => Navigator.of(context).pop(true),
            child: Text(t.delete),
          ),
        ],
      );
    },
  );
  return sure ?? false;
}

/// Makes the Note [noteId] a Checklist where this Device may; whether it
/// did.
bool makeChecklist(Notes notes, String noteId) {
  final lists = Checklists(notes.store);
  if (!lists.mayMake(noteId)) return false;
  lists.make(noteId);
  return true;
}

/// Turns the Checklist of [noteId] back into a plain Note where this
/// Device may; whether it did.
bool checklistToPlain(Notes notes, String noteId) {
  final store = notes.store;
  if (!Checklists(store).isChecklist(noteId) ||
      !store.mayChange(noteId) ||
      !(notes.byId(noteId)?.textEditable ?? false)) {
    return false;
  }
  Checklists(store).toPlain(noteId);
  return true;
}
