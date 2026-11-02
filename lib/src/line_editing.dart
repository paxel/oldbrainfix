import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'checklist_view.dart';
import 'two_row_text_field.dart';

/// What the Note editor tells the list in it (#141): [keep] stores what
/// was typed into its Lines, as the editor saves its text; [leave] does
/// too and drops the Lines left empty, as the editor is left.
class TypedLines {
  final _lists = <void Function({required bool leaving})>[];

  void keep() => _tell(leaving: false);

  void leave() => _tell(leaving: true);

  void _tell({required bool leaving}) {
    for (final list in [..._lists]) {
      list(leaving: leaving);
    }
  }

  void listen(void Function({required bool leaving}) list) => _lists.add(list);

  void forget(void Function({required bool leaving}) list) =>
      _lists.remove(list);
}

/// Stands first in the text of a Line typed in place, unseen: a
/// Backspace at the start of the Line takes it, which tells the list to
/// join the Line with the one above. A keyboard on the phone sends
/// nothing for a Backspace with no text before the cursor.
const lineStart = '​';

/// A Line as it is typed in place: its text and focus, and its id once
/// it is stored, null while it is new and only typed.
class TypedLine {
  String? id;

  /// Whether it is in the group of the Line above; for a stored Line as
  /// the list says.
  bool inGroup;

  /// The text as it was last stored or read, to tell what was typed.
  String kept;

  /// The value before the latest change, to tell a Backspace at the
  /// start from any other change.
  TextEditingValue last;

  /// Whether its text starts with [lineStart].
  final bool marked;

  /// Whether it was last drawn new with text, its ⋮ showing (#202).
  bool menued = false;

  /// Which new Line it was started as, counted by its list; 0 for one
  /// read from the list.
  final int started;

  final TextEditingController text;
  final focus = FocusNode();

  TypedLine({
    this.id,
    required this.kept,
    this.inGroup = false,
    this.marked = false,
    this.started = 0,
  }) : text = TextEditingController(text: '${marked ? lineStart : ''}$kept'),
       last = TextEditingValue.empty {
    last = text.value;
  }

  /// The text typed, on one line and trimmed, as a Line holds it; empty
  /// for none.
  String get typed => text.text
      .replaceAll(lineStart, '')
      .replaceAll(RegExp('[\r\n]+'), ' ')
      .trim();

  /// Whether something was typed that is not stored yet.
  bool get changed => typed != kept;

  /// Puts [stored] into the field, unless something typed is not stored
  /// yet; the cursor stays where it was, as far as the text goes.
  void take(String stored) {
    if (stored == kept) return;
    if (changed) {
      kept = stored;
      return;
    }
    kept = stored;
    final next = '${marked ? lineStart : ''}$stored';
    final selection = text.selection;
    int clamp(int offset) => offset.clamp(marked ? 1 : 0, next.length);
    set(
      TextEditingValue(
        text: next,
        selection: selection.isValid
            ? TextSelection(
                baseOffset: clamp(selection.baseOffset),
                extentOffset: clamp(selection.extentOffset),
              )
            : TextSelection.collapsed(offset: next.length),
      ),
    );
  }

  /// Sets the field to [value] without taking it for typing.
  void set(TextEditingValue value) {
    last = value;
    _setting = true;
    try {
      text.value = value;
    } finally {
      _setting = false;
    }
  }

  var _setting = false;

  /// Whether the field is being set by [set], not typed into.
  bool get setting => _setting;

  /// Puts the cursor at [offset] of the text typed, after [lineStart].
  void cursorAt(int offset) {
    final at = (offset + (marked ? 1 : 0)).clamp(0, text.text.length);
    set(text.value.copyWith(selection: TextSelection.collapsed(offset: at)));
  }

  void dispose() {
    text.dispose();
    focus.dispose();
  }
}

/// A Line of a list as the rows of [TypedRows] take it: its id, its text
/// and whether it is in the group of the Line above.
typedef RowLine = ({String id, String text, bool inGroup});

/// The rows of a list whose Lines are typed in place in the Note editor,
/// the same for a Checklist and a Shopping List (#141, #184): one block
/// of text, every Line a row beside its box. Return splits a Line at the
/// cursor, or at its end starts a new empty one under it; Backspace at
/// its start joins it with the Line above, whose box stays, and an empty
/// one goes. A list this Device may add to always has a row to type in,
/// an empty one at its end while it has none. What is typed is stored as the cursor
/// leaves a Line and whenever [keepTyped] is told so; the list says
/// through the members below how a Line is stored.
mixin TypedRows<T extends StatefulWidget> on State<T> {
  /// The Lines as they are typed, in the order they show, the new ones
  /// not stored yet among them.
  List<TypedLine> rows = const [];

  /// How many new Lines were started, to tell them apart.
  var _started = 0;

  /// The log the Lines are written to.
  Store get linesLog;

  /// Whether the Note is the list, out of the trash, to write to.
  bool get linesWritable;

  /// Whether this Device may add Lines to the list.
  bool get mayAddLine;

  /// Whether this Device may rename and delete the Line [id].
  bool mayEditLine(String id);

  /// Whether the Line [id] still shows on the list.
  bool showsLine(String id);

  /// Stores [text] as a new Line, [at] stored Lines from the top, in the
  /// group of the Line above with [inGroup]; its id. A Shopping List
  /// gives the id of a Line of that name it has already.
  String addLine(String text, {required int at, required bool inGroup});

  void renameLine(String id, String text);

  void deleteLine(String id);

  /// Deletes the Line [id] as it is joined with the Line above; its
  /// group, if any, stays a group, under that Line now.
  void deleteJoined(String id) => deleteLine(id);

  /// Whether the Line [id] has a group under it.
  bool hasGroup(String id) => false;

  @override
  void dispose() {
    for (final row in rows) {
      row.dispose();
    }
    super.dispose();
  }

  /// The rows for [lines] in their order: those there already kept with
  /// what is typed in them, a new one after the Line it was started
  /// under, a row whose Line went while typed in kept as a new one.
  List<TypedLine> rowsInOrder(List<RowLine>? lines) {
    final old = rows;
    final stored = {for (final row in old) ?row.id: row};
    final after = <String?, List<TypedLine>>{};
    final ids = {for (final line in lines ?? const <RowLine>[]) line.id};
    String? above;
    for (final row in old) {
      final id = row.id;
      if (id != null && ids.contains(id)) {
        above = id;
        continue;
      }
      if (id != null) {
        stored.remove(id);
        if (!_keptAsNew(row)) continue;
      }
      after.putIfAbsent(above, () => []).add(row);
    }
    if (lines == null) {
      for (final row in [...stored.values, ...after.values.expand((r) => r)]) {
        retire(row);
      }
      return const [];
    }
    final next = <TypedLine>[...?after.remove(null)];
    for (final line in lines) {
      final row =
          stored.remove(line.id) ?? newRow(id: line.id, text: line.text);
      row
        ..inGroup = line.inGroup
        ..take(line.text);
      next
        ..add(row)
        ..addAll(after.remove(line.id) ?? const []);
    }
    // Started under a Line that went: at the end.
    for (final rest in after.values) {
      next.addAll(rest);
    }
    for (final row in stored.values) {
      retire(row);
    }
    return _withRow(next);
  }

  /// The rows for [lines] with the rows there already where they are, so
  /// a Line stays where it was, checked or not, while the editor is open;
  /// a Line new to the list from elsewhere comes in at the end. A row
  /// whose Line went while typed in is kept as a new one.
  List<TypedLine> rowsInPlace(List<RowLine>? lines) {
    if (lines == null) {
      rows.forEach(retire);
      return const [];
    }
    final byId = {for (final line in lines) line.id: line};
    final known = {for (final row in rows) ?row.id};
    final next = <TypedLine>[];
    for (final row in rows) {
      final id = row.id;
      if (id == null) {
        next.add(row);
      } else if (byId[id] case final line?) {
        next.add(
          row
            ..inGroup = line.inGroup
            ..take(line.text),
        );
      } else if (_keptAsNew(row)) {
        next.add(row);
      }
    }
    for (final line in lines) {
      if (!known.contains(line.id)) {
        next.add(newRow(id: line.id, text: line.text, inGroup: line.inGroup));
      }
    }
    return _withRow(next);
  }

  /// Whether [row], whose Line went, stays as a new one: something typed
  /// in it is not stored. Else it is retired.
  bool _keptAsNew(TypedLine row) {
    if (!row.changed || row.typed.isEmpty) {
      retire(row);
      return false;
    }
    row
      ..id = null
      ..kept = '';
    return true;
  }

  /// [next] with an empty row at its end where Lines may be added and
  /// none of [next] can be typed in: a list without Lines, or one of
  /// Lines others wrote.
  List<TypedLine> _withRow(List<TypedLine> next) =>
      mayAddLine && !next.any(_typable) ? [...next, newRow()] : next;

  /// Whether [row] is typed in: a new Line, or one this Device may change.
  bool _typable(TypedLine row) => switch (row.id) {
    final id? => mayEditLine(id),
    null => true,
  };

  /// A row for the Line [id] holding [text], or a new one.
  TypedLine newRow({String? id, String text = '', bool inGroup = false}) {
    final row = TypedLine(
      id: id,
      kept: text,
      inGroup: inGroup,
      marked: true,
      started: id == null ? ++_started : 0,
    );
    row.text.addListener(() {
      _changed(row);
      _followMenu(row);
    });
    row.focus.addListener(() {
      if (!row.focus.hasFocus) scheduleMicrotask(() => _left(row));
    });
    return row;
  }

  /// Draws the list anew as the new [row] gets text or loses it: its ⋮
  /// comes and goes with it (#202).
  void _followMenu(TypedLine row) {
    final menued = row.id == null && row.typed.isNotEmpty;
    if (menued == row.menued) return;
    row.menued = menued;
    if (mounted) setState(() {});
  }

  /// The Lines with ⋮: [editable], and the new ones that hold text, stored
  /// as an entry of their ⋮ is used (#202). An empty new one has none.
  Set<String> withMenus(Set<String> editable) => {
    ...editable,
    if (mayAddLine && linesWritable)
      for (final row in rows)
        if (row.id == null && row.typed.isNotEmpty) shownId(row),
  };

  /// Disposes [row] once its field is gone.
  void retire(TypedLine row) {
    WidgetsBinding.instance.addPostFrameCallback((_) => row.dispose());
  }

  /// The id [row] shows under: its Line's, or one of its own while new.
  static String shownId(TypedLine row) => row.id ?? 'new-${row.started}';

  TypedLine? rowShown(String id) {
    for (final row in rows) {
      if (shownId(row) == id) return row;
    }
    return null;
  }

  TypedLine? _rowOf(String id) {
    for (final row in rows) {
      if (row.id == id) return row;
    }
    return null;
  }

  /// How many stored Lines show before [row]: where it is added.
  int storedBefore(TypedLine row) {
    var count = 0;
    for (final r in rows) {
      if (identical(r, row)) break;
      if (placesAmong(r)) count++;
    }
    return count;
  }

  /// Whether the Line of [row] counts for the place a new Line is added
  /// at: every stored one; on a Shopping List the open ones alone.
  bool placesAmong(TypedLine row) => row.id != null;

  /// Stores what was typed into [row]: a new Line once it holds text, a
  /// Line renamed once its text changed. An empty one waits. Gives the
  /// row that holds the Line now: [row], or for a name a Shopping List
  /// has already the row of that Line, [row] going.
  TypedLine storeRow(TypedLine row) {
    final typed = row.typed;
    if (typed.isEmpty || !row.changed || !linesWritable) return row;
    switch (row.id) {
      case null:
        if (!mayAddLine) return row;
        final id = addLine(typed, at: storedBefore(row), inGroup: row.inGroup);
        if (_rowOf(id) case final other? when !identical(other, row)) {
          _mergeInto(row, other);
          return other;
        }
        row.id = id;
      case final id:
        if (!showsLine(id) || !mayEditLine(id)) return row;
        renameLine(id, typed);
    }
    row.kept = typed;
    return row;
  }

  /// The new [row] was stored as the Line of [other], a name the list
  /// has already: it goes, and the cursor to the end of [other].
  void _mergeInto(TypedLine row, TypedLine other) {
    final focused = row.focus.hasFocus;
    setState(() => rows = [...rows]..remove(row));
    retire(row);
    if (focused) focusOn(other, other.typed.length);
  }

  /// The id of the Line [shown] shows, a new one stored first; null for
  /// an empty new one.
  String? storedId(String shown) {
    final row = rowShown(shown);
    if (row == null) return shown;
    if (row.id case final id?) return id;
    return storeRow(row).id;
  }

  /// The cursor left [row].
  void _left(TypedLine row) {
    if (!mounted || !rows.contains(row) || !row.changed) return;
    storeRow(row);
  }

  /// Stores what was typed; [leaving], the Lines left empty go too.
  void keepTyped({required bool leaving}) {
    if (!mounted) return;
    final typed = [
      for (final row in rows)
        if (row.changed && row.typed.isNotEmpty) row,
    ];
    if (typed.isNotEmpty) {
      linesLog.batch(() {
        for (final row in typed) {
          if (rows.contains(row)) storeRow(row);
        }
      });
    }
    if (!leaving) return;
    final empty = [
      for (final row in rows)
        if (row.typed.isEmpty) row,
    ];
    if (empty.isEmpty) return;
    final gone = [
      for (final row in empty)
        if (row.id case final id? when showsLine(id) && mayEditLine(id)) id,
    ];
    if (gone.isNotEmpty && linesWritable) {
      linesLog.batch(() {
        for (final id in gone) {
          deleteLine(id);
        }
      });
    }
    setState(() {
      rows = [
        for (final row in rows)
          if (!empty.contains(row)) row,
      ];
    });
    empty.forEach(retire);
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
    if (!mounted || !rows.contains(row)) return;
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
    final at = rows.indexOf(row);
    if (parts.length == 2 && parts.first.trim().isEmpty) {
      row.set(
        TextEditingValue(
          text: '$lineStart${parts.last}',
          selection: const TextSelection.collapsed(offset: 1),
        ),
      );
      setState(() {
        rows = [...rows]..insert(at, newRow(inGroup: row.inGroup));
      });
      return;
    }
    final group = row.inGroup || (row.id != null && hasGroup(row.id!));
    final first = filled.first;
    row.set(
      TextEditingValue(
        text: '$lineStart$first',
        selection: TextSelection.collapsed(offset: 1 + first.length),
      ),
    );
    final added = <TypedLine>[];
    var holder = row;
    linesLog.batch(() {
      holder = storeRow(row);
      if (holder.id == null) return;
      var index = storedBefore(holder) + (placesAmong(holder) ? 1 : 0);
      for (final text in filled.skip(1)) {
        final id = addLine(text, at: index++, inGroup: group);
        // A name a Shopping List has already keeps its row.
        if (_rowOf(id) != null || added.any((r) => r.id == id)) continue;
        added.add(newRow(id: id, text: text, inGroup: group));
      }
    });
    if (holder.id == null) return;
    final ends = parts.last.trim().isEmpty;
    if (ends) added.add(newRow(inGroup: group));
    if (added.isEmpty) return;
    final into = identical(holder, row) ? rows.indexOf(row) + 1 : at;
    setState(() {
      rows = [...rows]..insertAll(into, added);
    });
    final target = added.last;
    focusOn(target, parts.length == 2 || ends ? 0 : target.typed.length);
  }

  /// Joins [row] with the Line above at a Backspace at its start: its
  /// text goes to the end of the Line above, which keeps its box and
  /// state, and the Line [row] is deleted, its checks staying in the
  /// log. An empty Line simply goes.
  void _join(TypedLine row) {
    if (!mounted || !rows.contains(row)) return;
    final at = rows.indexOf(row);
    final text = row.typed;
    if (text.isEmpty) {
      _drop(row);
      return;
    }
    final upper = at > 0 ? rows[at - 1] : null;
    final upperId = upper?.id;
    if (upper == null || (upperId != null && !mayEditLine(upperId))) {
      row.set(
        TextEditingValue(
          text: '$lineStart${row.text.text}',
          selection: const TextSelection.collapsed(offset: 1),
        ),
      );
      return;
    }
    final head = upper.typed;
    upper.set(
      TextEditingValue(
        text: '$lineStart$head$text',
        selection: TextSelection.collapsed(offset: 1 + head.length),
      ),
    );
    var holder = upper;
    linesLog.batch(() {
      holder = storeRow(upper);
      if (row.id case final id? when linesWritable && showsLine(id)) {
        deleteJoined(id);
      }
    });
    setState(() {
      rows = [...rows]..remove(row);
    });
    retire(row);
    focusOn(holder, head.length);
  }

  /// Takes the empty [row] away, deleting its Line; the cursor goes to
  /// the end of the Line above, or to the start of the next. The only
  /// row to type in stays, empty and new.
  void _drop(TypedLine row) {
    final at = rows.indexOf(row);
    if (row.id case final id? when linesWritable && showsLine(id)) {
      if (!mayEditLine(id)) return;
      deleteLine(id);
    }
    if (mayAddLine &&
        !rows.any((other) => !identical(other, row) && _typable(other))) {
      row
        ..id = null
        ..kept = ''
        ..set(
          const TextEditingValue(
            text: lineStart,
            selection: TextSelection.collapsed(offset: 1),
          ),
        );
      return;
    }
    setState(() {
      rows = [...rows]..remove(row);
    });
    retire(row);
    if (at > 0) {
      final above = rows[at - 1];
      focusOn(above, above.text.text.length);
    } else if (rows.isNotEmpty) {
      focusOn(rows.first, 0);
    }
  }

  /// Puts the cursor into [row] at [offset] of its text.
  void focusOn(TypedLine row, int offset) {
    row.cursorAt(offset);
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted && rows.contains(row)) row.focus.requestFocus();
    });
  }

  /// Puts the cursor into the empty row of a list without Lines, as a
  /// new list starts.
  void focusEmptyRow() {
    if (rows case [final row] when row.id == null && row.typed.isEmpty) {
      focusOn(row, 0);
    }
  }

  /// The field of [shown] where it is typed in place: a new Line, or one
  /// of [editable], the Lines this Device may change; keyed [prefix] and
  /// its id.
  Widget? typedField(
    ShownLine shown, {
    required String prefix,
    required Set<String> editable,
    required bool twoRows,
    List<FieldAction> actions = const [],
  }) {
    final row = rowShown(shown.id);
    if (row == null) return null;
    if (row.id case final id? when !editable.contains(id)) return null;
    return KeyedSubtree(
      key: ValueKey('$prefix-field-${shown.id}'),
      child: TwoRowTextField(
        // The same field while the Line moves, in a group or out.
        key: GlobalObjectKey(row),
        controller: row.text,
        focusNode: row.focus,
        twoRows: twoRows,
        style: lineStyle(context, checked: shown.checked),
        decoration: const InputDecoration(
          filled: false,
          border: InputBorder.none,
          isDense: true,
          // One line of text beside its box, the rows close together.
          contentPadding: EdgeInsets.symmetric(vertical: 8),
        ),
        actions: actions,
      ),
    );
  }
}
