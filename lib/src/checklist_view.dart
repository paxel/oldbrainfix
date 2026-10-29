import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'browser.dart';
import 'l10n.dart';
import 'note_editor_screen.dart';
import 'two_row_text_field.dart';

/// A Line as it is drawn: its id, its text, whether it is in the group of
/// the Line above it and whether it shows checked.
typedef ShownLine = ({String id, String text, bool inGroup, bool checked});

/// The Lines of [list] as they are drawn.
List<ShownLine> shownLines(Checklist list) => [
  for (final line in list.lines)
    (
      id: line.id,
      text: line.text,
      inGroup: line.inGroup,
      checked: line.checked,
    ),
];

/// What a Note's card knows of its Checklist (#120): [read] gives it, or
/// null for a Note that is none, read when the card is, never while
/// building; [lists] checks its Lines.
typedef NoteChecklist = ({Checklist? Function() read, Checklists lists});

/// The Lines of a Checklist, each with its checkbox and its text, struck
/// through in place once checked; the group of a Line is drawn as one
/// frame under it. A checkbox checks with [onCheck] and is disabled while
/// it is null. With [onMenu] a Line whose id [editable] holds has ⋮, and
/// a long press on it opens the same menu where the finger is.
class ChecklistLines extends StatelessWidget {
  final List<ShownLine> lines;
  final void Function(String id, bool checked)? onCheck;
  final void Function(String id, Offset at)? onMenu;
  final Set<String> editable;

  const ChecklistLines({
    super.key,
    required this.lines,
    this.onCheck,
    this.onMenu,
    this.editable = const {},
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

/// One Line: its checkbox, its text and, with [onMenu], ⋮.
class _LineRow extends StatelessWidget {
  final ShownLine line;
  final void Function(String id, bool checked)? onCheck;
  final void Function(String id, Offset at)? onMenu;

  const _LineRow({required this.line, this.onCheck, this.onMenu});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final style = theme.textTheme.bodyLarge;
    final check = onCheck;
    final menu = onMenu;
    // The checkbox reads with its text; ⋮ stays a button of its own.
    final row = Row(
      children: [
        Expanded(
          child: MergeSemantics(
            child: Row(
              children: [
                Checkbox(
                  key: ValueKey('checklist-check-${line.id}'),
                  value: line.checked,
                  onChanged: check == null
                      ? null
                      : (on) => check(line.id, on ?? false),
                ),
                Expanded(
                  child: Padding(
                    padding: const EdgeInsets.symmetric(vertical: 8),
                    child: Text(
                      line.text,
                      key: ValueKey('checklist-line-${line.id}'),
                      style: line.checked
                          ? style?.copyWith(
                              decoration: TextDecoration.lineThrough,
                              color: theme.colorScheme.onSurfaceVariant,
                            )
                          : style,
                    ),
                  ),
                ),
              ],
            ),
          ),
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
    if (menu == null) return row;
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
    _list = widget.checklist.read();
    _checks = _list != null && widget.checklist.lists.mayCheck(widget.noteId);
  }

  void _check(String id, bool checked) {
    final lists = widget.checklist.lists;
    checked ? lists.check(id) : lists.uncheck(id);
    setState(() => _list = lists.byNote(widget.noteId));
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
/// is none: its Lines to check; for who may change a Line, ⋮ and a long
/// press with Rename, Move up, Move down, putting it under the Line above
/// or back to the top level, and Delete; a field and "Add Line" for
/// anyone with a role in its Area; "Uncheck all" while one is checked;
/// and, with [onToPlain], "Back to a plain Note". It follows the log as it
/// changes.
class ChecklistSection extends StatefulWidget {
  final Notes notes;
  final String noteId;

  /// Turns the Note back into a plain Note; no button when null.
  final VoidCallback? onToPlain;

  const ChecklistSection({
    super.key,
    required this.notes,
    required this.noteId,
    this.onToPlain,
  });

  @override
  State<ChecklistSection> createState() => _ChecklistSectionState();
}

/// What the menu of a Line does.
enum _LineStep { rename, up, down, indent, outdent, delete }

class _ChecklistSectionState extends State<ChecklistSection>
    with ReloadsOnLogChange {
  late final _lists = Checklists(widget.notes.store);
  late final _twoRows = widget.notes.store.twoRowEditorOn;
  final _add = TextEditingController();
  Checklist? _list;
  var _checks = false;
  Set<String> _editable = const {};

  @override
  Store get log => widget.notes.store;

  @override
  void reloadLog() => setState(_load);

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void dispose() {
    _add.dispose();
    super.dispose();
  }

  void _load() {
    final list = _list = _lists.byNote(widget.noteId);
    _checks = list != null && _lists.mayCheck(widget.noteId);
    _editable = {
      for (final line in list?.lines ?? const <Line>[])
        if (_lists.mayEdit(line.id)) line.id,
    };
  }

  void _check(String id, bool checked) {
    checked ? _lists.check(id) : _lists.uncheck(id);
    setState(_load);
  }

  void _uncheckAll() {
    _lists.uncheckAll(widget.noteId);
    setState(_load);
  }

  /// Adds a Line for every line typed that is not blank, one that starts
  /// with a space or a tab under the Line above.
  void _addTyped() {
    final rows = [
      for (final row in splitLines(_add.text))
        if (row.trim().isNotEmpty) row,
    ];
    if (rows.isEmpty) return;
    for (final row in rows) {
      _lists.add(
        widget.noteId,
        row,
        indent: row.startsWith(' ') || row.startsWith('\t'),
      );
    }
    _add.clear();
    setState(_load);
  }

  /// The steps the menu of [line] offers, as [list] stands.
  List<_LineStep> _steps(Checklist list, Line line) {
    final lines = list.lines;
    final at = lines.indexWhere((l) => l.id == line.id);
    final (before, after) = _neighbours(list, line);
    return [
      _LineStep.rename,
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
    final list = _list;
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
                  _LineStep.rename => Icons.edit_outlined,
                  _LineStep.up => Icons.arrow_upward,
                  _LineStep.down => Icons.arrow_downward,
                  _LineStep.indent => Icons.format_indent_increase,
                  _LineStep.outdent => Icons.format_indent_decrease,
                  _LineStep.delete => Icons.delete_outline,
                }),
                const SizedBox(width: 12),
                Flexible(
                  child: Text(switch (step) {
                    _LineStep.rename => t.rename,
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
    switch (step) {
      case _LineStep.rename:
        final text = await showDialog<String>(
          context: context,
          builder: (_) => _RenameDialog(text: line.text, twoRows: _twoRows),
        );
        final named = text?.replaceAll(RegExp('[\r\n]+'), ' ').trim();
        if (!mounted || named == null || named.isEmpty) return;
        // A sync while the dialog was open may have taken it away.
        if (_lists.byNote(widget.noteId)?.line(id) == null) return;
        _lists.rename(id, named);
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
        if (!await _deleteLine()) return;
        if (!mounted) return;
        if (_lists.byNote(widget.noteId)?.line(id) == null) return;
        _lists.delete(id);
    }
    setState(_load);
  }

  /// Asks once whether a Line goes, with the verb on the button in the
  /// error colour.
  Future<bool> _deleteLine() async {
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

  @override
  Widget build(BuildContext context) {
    final list = _list;
    if (list == null) return const SizedBox.shrink();
    final t = context.t;
    final plain = widget.onToPlain;
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
              lines: shownLines(list),
              onCheck: _checks ? _check : null,
              onMenu: _menu,
              editable: _editable,
            ),
            if (_checks) ...[
              const SizedBox(height: 8),
              TwoRowTextField(
                key: const ValueKey('checklist-add-text'),
                controller: _add,
                twoRows: _twoRows,
                decoration: InputDecoration(hintText: t.checklistNewLine),
              ),
              const SizedBox(height: 8),
              Align(
                alignment: Alignment.centerLeft,
                child: FilledButton.tonalIcon(
                  key: const ValueKey('checklist-add'),
                  onPressed: _addTyped,
                  icon: const Icon(Icons.add),
                  label: Text(t.checklistAddLine),
                ),
              ),
            ],
            if (uncheck || plain != null) ...[
              const SizedBox(height: 8),
              Wrap(
                spacing: 8,
                runSpacing: 8,
                children: [
                  if (uncheck)
                    OutlinedButton.icon(
                      key: const ValueKey('checklist-uncheck-all'),
                      onPressed: _uncheckAll,
                      icon: const Icon(Icons.check_box_outline_blank),
                      label: Text(t.checklistUncheckAll),
                    ),
                  if (plain != null)
                    OutlinedButton.icon(
                      key: const ValueKey('checklist-to-plain'),
                      onPressed: plain,
                      icon: const Icon(Icons.notes),
                      label: Text(t.checklistToPlain),
                    ),
                ],
              ),
            ],
          ],
        ),
      ),
    );
  }
}

/// Asks for a Line's new text, starting from [text].
class _RenameDialog extends StatefulWidget {
  final String text;

  /// Whether the text is typed in two rows (UI laws, "Text editor").
  final bool twoRows;

  const _RenameDialog({required this.text, required this.twoRows});

  @override
  State<_RenameDialog> createState() => _RenameDialogState();
}

class _RenameDialogState extends State<_RenameDialog> {
  late final _text = TextEditingController(text: widget.text);

  @override
  void dispose() {
    _text.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final media = MediaQuery.of(context);
    final keyboard = media.viewInsets.bottom;
    // The dialog stays clear of the bottom row above the keyboard.
    final below = widget.twoRows && keyboard > 0
        ? keyboard + TwoRowTextField.rowHeight
        : keyboard;
    return MediaQuery(
      data: media.copyWith(
        viewInsets: media.viewInsets.copyWith(bottom: below),
      ),
      child: AlertDialog(
        title: Text(t.checklistRenameTitle),
        content: TwoRowTextField(
          key: const ValueKey('checklist-rename-text'),
          controller: _text,
          twoRows: widget.twoRows,
          autofocus: true,
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: Text(t.cancel),
          ),
          TextButton(
            key: const ValueKey('checklist-rename-save'),
            onPressed: () => Navigator.of(context).pop(_text.text),
            child: Text(t.save),
          ),
        ],
      ),
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
