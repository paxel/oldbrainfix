import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'announcements.dart';
import 'browser.dart';
import 'l10n.dart';
import 'line_editing.dart';
import 'note_editor_screen.dart';
import 'routine_editor_screen.dart';

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
/// through in place once checked; a Line with a group is drawn in one
/// frame with its group under it. A checkbox checks with [onCheck] and is
/// disabled while it is null. With [onMenu] a Line whose id [editable]
/// holds has ⋮, and a long press on it opens the same menu where the
/// finger is. A Line [field] gives a field for is typed in that field, in
/// place of its text (#141), and its ⋮ is the only way to its menu: a
/// long press in the field selects. [compact] draws the rows close
/// together, as one block of text in the editor (#184). What [trailing]
/// gives for a Line sits at its end, before ⋮, such as a Shopping List
/// Line's reappear time badge (#195).
class ChecklistLines extends StatelessWidget {
  final List<ShownLine> lines;
  final void Function(String id, bool checked)? onCheck;
  final void Function(String id, Offset at)? onMenu;
  final Set<String> editable;
  final Widget? Function(ShownLine line)? field;
  final Widget? Function(ShownLine line)? trailing;
  final bool compact;

  const ChecklistLines({
    super.key,
    required this.lines,
    this.onCheck,
    this.onMenu,
    this.editable = const {},
    this.field,
    this.trailing,
    this.compact = false,
  });

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final rows = <Widget>[];
    Widget? head;
    String? parent;
    var group = <Widget>[];
    // The frame holds the Line of the group and the Lines under it.
    void closeGroup() {
      final top = head;
      if (top == null) return;
      if (group.isEmpty) {
        rows.add(top);
      } else {
        rows.add(
          Container(
            key: ValueKey('checklist-group-$parent'),
            margin: EdgeInsets.symmetric(vertical: compact ? 2 : 4),
            decoration: BoxDecoration(
              border: Border.all(color: scheme.outlineVariant),
              borderRadius: BorderRadius.circular(12),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                top,
                Padding(
                  padding: const EdgeInsets.only(left: 20),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: group,
                  ),
                ),
              ],
            ),
          ),
        );
      }
      head = null;
      group = [];
    }

    for (final line in lines) {
      final row = _LineRow(
        line: line,
        onCheck: onCheck,
        onMenu: editable.contains(line.id) ? onMenu : null,
        field: field?.call(line),
        trailing: trailing?.call(line),
        compact: compact,
      );
      if (line.inGroup && head != null) {
        group.add(row);
      } else {
        closeGroup();
        parent = line.id;
        head = row;
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

/// One Line: its checkbox, its text or [field], [trailing] and, with
/// [onMenu], ⋮.
class _LineRow extends StatelessWidget {
  final ShownLine line;
  final void Function(String id, bool checked)? onCheck;
  final void Function(String id, Offset at)? onMenu;
  final Widget? field;
  final Widget? trailing;
  final bool compact;

  const _LineRow({
    required this.line,
    this.onCheck,
    this.onMenu,
    this.field,
    this.trailing,
    this.compact = false,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final check = onCheck;
    final menu = onMenu;
    final field = this.field;
    final box = Checkbox(
      key: ValueKey('checklist-check-${line.id}'),
      value: line.checked,
      // 40 by 40 in the editor, not 48, so the rows read as one block.
      materialTapTargetSize: compact ? MaterialTapTargetSize.shrinkWrap : null,
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
        ?trailing,
        if (menu != null)
          Builder(
            builder: (button) => IconButton(
              key: ValueKey('checklist-menu-${line.id}'),
              tooltip: context.t.more,
              visualDensity: compact ? VisualDensity.compact : null,
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
/// Lines are open (#189). Nothing for a Note that is none. It reads the
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
          context.t.checklistOpenCount(list.openCount, list.count),
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
/// is none: its Lines with their boxes, typed in place as one block of
/// text where this Device may change them (#141, #184, [TypedRows]); a
/// new list starts with one empty Line. "in" and "out" on the bar above
/// the keyboard put a Line under the Line above or back at the top. ⋮ of
/// a Line it may change has Move up, Move down, putting it under the Line
/// above or back to the top level, and Delete; "Uncheck all" shows while
/// one is checked. "Back to a plain Note" is in the editor's kind badge
/// (#160). What is typed is stored as the cursor leaves a Line and
/// whenever [typed] says so; a Line left empty goes as the editor is
/// left. It follows the log as it changes.
class ChecklistSection extends StatefulWidget {
  final Notes notes;
  final String noteId;

  /// Tells it when the editor saves and when it is left.
  final TypedLines? typed;

  /// Whether the cursor starts in the empty Line of a new list (#161).
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
    with ReloadsOnLogChange, TypedRows, AutomaticKeepAliveClientMixin {
  late final _lists = Checklists(widget.notes.store);
  late final _twoRows = widget.notes.store.twoRowEditorOn;
  Checklist? _list;
  var _checks = false;
  var _adds = false;
  Set<String> _editable = const {};
  List<DeletedLine> _removed = const [];

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
    widget.typed?.listen(keepTyped);
    if (widget.startLine) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) focusEmptyRow();
      });
    }
  }

  @override
  void didUpdateWidget(ChecklistSection old) {
    super.didUpdateWidget(old);
    if (old.typed != widget.typed) {
      old.typed?.forget(keepTyped);
      widget.typed?.listen(keepTyped);
    }
  }

  @override
  void dispose() {
    widget.typed?.forget(keepTyped);
    super.dispose();
  }

  /// Whether the list was read once, to tell a list made in the editor.
  var _read = false;

  void _load() {
    final made = _read && _list == null;
    _read = true;
    final list = _list = _lists.shownNow(widget.noteId, deviceWallClock);
    _checks = list != null && mayCheckShown(_lists, list);
    _adds = list != null && _lists.mayCheck(widget.noteId);
    _editable = {
      for (final line in list?.lines ?? const <Line>[])
        if (_lists.mayEdit(line.id)) line.id,
    };
    _removed = _lists.deletedLines(widget.noteId);
    rows = rowsInOrder(switch (list) {
      final list? => [
        for (final line in list.lines)
          (id: line.id, text: line.text, inGroup: line.inGroup),
      ],
      null => null,
    });
    // Made a Checklist in the editor: the cursor in its first Line.
    if (made && list != null && list.lines.isEmpty && rows.isNotEmpty) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) focusEmptyRow();
      });
    }
  }

  @override
  Store get linesLog => widget.notes.store;

  /// Whether the Note is a Checklist out of the trash, to write to.
  @override
  bool get linesWritable =>
      !(widget.notes.byId(widget.noteId)?.isTrashed ?? true) &&
      _lists.isChecklist(widget.noteId);

  @override
  bool get mayAddLine => _adds;

  @override
  bool mayEditLine(String id) => _lists.mayEdit(id);

  @override
  bool showsLine(String id) => _lists.byNote(widget.noteId)?.line(id) != null;

  @override
  String addLine(String text, {required int at, required bool inGroup}) =>
      _lists.add(widget.noteId, text, at: at, indent: inGroup);

  @override
  void renameLine(String id, String text) => _lists.rename(id, text);

  @override
  void deleteLine(String id) => _lists.delete(id);

  /// Its group stays a group, under the Line above now.
  @override
  void deleteJoined(String id) {
    final group = [
      for (final line in _list?.groupOf(id) ?? const <Line>[])
        if (_lists.mayEdit(line.id)) line.id,
    ];
    _lists.delete(id);
    for (final line in group) {
      _lists.indent(line);
    }
  }

  @override
  bool hasGroup(String id) => _list?.groupOf(id).isNotEmpty ?? false;

  /// Puts [row] under the Line above, or with [under] false back at the
  /// top.
  void _indent(TypedLine row, {required bool under}) {
    final at = rows.indexOf(row);
    if (at < 0) return;
    switch (row.id) {
      case null:
        setState(() => row.inGroup = under && at > 0);
      case final id:
        if (!linesWritable || !showsLine(id) || !_lists.mayEdit(id)) return;
        if (under && _lists.byNote(widget.noteId)?.lines.first.id == id) {
          return;
        }
        _lists.indent(id, under: under);
        setState(_load);
    }
  }

  void _check(String id, bool checked) {
    // A new Line is stored first; an empty one has nothing to check.
    final line = storedId(id);
    if (line == null) return;
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

  Future<void> _menu(String shown, Offset at) async {
    // A new Line is stored first, and what was typed, so the menu sees
    // the list as shown.
    final id = storedId(shown);
    if (id == null) return;
    keepTyped(leaving: false);
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
    if (!linesWritable || !showsLine(id)) return;
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
        _lists.delete(id);
        offerLineUndo(
          context,
          _lists.deletedLines,
          _lists.mayEdit,
          _lists.restore,
          widget.noteId,
          id,
        );
    }
    setState(_load);
  }

  /// Brings the deleted Line [id] back from "Removed Lines".
  void _restore(String id) {
    if (!linesWritable || !_lists.mayEdit(id)) return;
    if (!_lists.deletedLines(widget.noteId).any((l) => l.id == id)) return;
    _lists.restore(id);
    setState(_load);
  }

  /// The field of [shown] where it is typed in place, with "in" and
  /// "out" on the bar above the keyboard.
  Widget? _field(ShownLine shown) {
    final row = rowShown(shown.id);
    if (row == null) return null;
    final t = context.t;
    final at = rows.indexOf(row);
    return typedField(
      shown,
      prefix: 'checklist',
      editable: _editable,
      twoRows: _twoRows,
      actions: [
        (
          key: 'line-indent',
          label: t.checklistIndent,
          icon: Icons.format_indent_increase,
          run: at > 0 && !row.inGroup ? () => _indent(row, under: true) : null,
        ),
        (
          key: 'line-outdent',
          label: t.checklistOutdent,
          icon: Icons.format_indent_decrease,
          run: row.inGroup ? () => _indent(row, under: false) : null,
        ),
      ],
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
                for (final row in rows)
                  (
                    id: TypedRows.shownId(row),
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
              editable: withMenus(_editable),
              field: _adds ? _field : null,
              compact: true,
            ),
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
            if (_removed.isNotEmpty)
              RemovedLines(
                store: widget.notes.store,
                lines: _removed,
                onRestore: linesWritable
                    ? (id) => _lists.mayEdit(id) ? () => _restore(id) : null
                    : (_) => null,
              ),
          ],
        ),
      ),
    );
  }
}

/// Offers at the top to undo the deletion of the Line [lineId] of the
/// list of [noteId] (#268): the undo [restore]s it while [deleted] still
/// lists it and this Device [mayEdit] it, as "Removed Lines" would.
void offerLineUndo(
  BuildContext context,
  List<DeletedLine> Function(String noteId) deleted,
  bool Function(String lineId) mayEdit,
  void Function(String lineId) restore,
  String noteId,
  String lineId,
) => AnnouncementsScope.of(context)?.offerUndo(context.t.undoLineDeleted, () {
  if (!deleted(noteId).any((l) => l.id == lineId) || !mayEdit(lineId)) return;
  restore(lineId);
});

/// The folded "Removed Lines" under a list's Lines in the Note editor
/// (#268): the deleted Lines, the last deleted first, each with Restore
/// below it where [onRestore] gives one; open or folded as last left on
/// this Device, as a Tally's "Removed entries".
class RemovedLines extends StatefulWidget {
  final Store store;
  final List<DeletedLine> lines;

  /// What Restore runs for the Line of an id, or null where it may not.
  final VoidCallback? Function(String id) onRestore;

  const RemovedLines({
    super.key,
    required this.store,
    required this.lines,
    required this.onRestore,
  });

  @override
  State<RemovedLines> createState() => _RemovedLinesState();
}

class _RemovedLinesState extends State<RemovedLines> {
  /// The local setting that keeps "Removed Lines" open.
  static const _openKey = 'linesRemovedOpen';

  late final _open = widget.store.localSetting(_openKey) == 'true';

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final weak = Theme.of(context).colorScheme.onSurfaceVariant;
    return ExpansionTile(
      key: const ValueKey('lines-removed'),
      shape: const Border(),
      collapsedShape: const Border(),
      tilePadding: EdgeInsets.zero,
      childrenPadding: EdgeInsets.zero,
      expandedCrossAxisAlignment: CrossAxisAlignment.start,
      leading: const Icon(Icons.delete_outline),
      title: Text(t.linesRemoved),
      initiallyExpanded: _open,
      onExpansionChanged: (open) =>
          widget.store.setLocalSetting(_openKey, '$open'),
      children: [
        for (final line in widget.lines)
          Padding(
            key: ValueKey('line-removed-${line.id}'),
            padding: const EdgeInsets.only(top: 8),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(line.text, style: TextStyle(color: weak)),
                // The control goes below its text.
                if (widget.onRestore(line.id) case final restore?)
                  TextButton.icon(
                    key: ValueKey('line-restore-${line.id}'),
                    onPressed: restore,
                    icon: const Icon(Icons.restore),
                    label: Text(t.restore),
                  ),
              ],
            ),
          ),
      ],
    );
  }
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
