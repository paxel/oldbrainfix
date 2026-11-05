import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'box_ids.dart';
import 'l10n.dart';
import 'look.dart';
import 'table_screen.dart';
import 'text_marks_view.dart';
import 'two_row_text_field.dart';

/// A cell of a table: its row, the header row 0, and its column.
typedef GridCell = ({int row, int column});

/// What a field made of a change of its table: the text it holds then,
/// the new boxes perhaps with their ids (ADR 0035), and where the table
/// starts in it.
typedef GridWritten = ({String text, int at});

/// A table of a formatted text as the grid it is edited in, in place of
/// its lines in the editor's field (#257, ADR 0033).
///
/// A tap on a cell edits it in place, its words typed as in the Note
/// editor in one row, with the bar of a line's parts when [bar] (☐, bold,
/// italic, strike-through, link, code). Tab and Shift+Tab go to the next
/// and the previous cell row by row, ↵ and ↓ to the cell below, ↑ to the
/// one above, ← and → at a cell's ends to its neighbour; past the grid's
/// first or last cell the cursor goes back into the text before or after
/// the table ([onLeave]). "+" after the last row and after the last
/// column adds one. A row's number and a column's letter open its menu
/// at the finger: put one in before or after it, move it, delete it
/// (asking once while it holds words; the last one takes the table), for
/// a column how it lines up its cells, and on the first column whether it
/// holds headings. "Show as lines" in its corner shows the table's lines
/// as text instead ([onShowLines]).
///
/// Every change is made by the table edits of the core
/// ([MarkdownTables]) on [text], the table [table] in it, and given to
/// [onChange], also each letter typed into a cell; while a cell is edited
/// its words as typed are what counts, the spaces at their ends kept. The
/// cells not being edited show their parts, boxes and tags as a card
/// does. The columns are as wide as their words within bounds; a grid
/// wider than the field moves sideways.
///
/// A cell's field takes the focus beside the editor's, under
/// [focusParent], so that the editor's field gives the keyboard up while
/// a cell has it.
class TableGrid extends StatefulWidget {
  /// How wide the row numbers are, how high the column letters, and how
  /// wide the column of "+".
  static const gutter = 40.0;
  static const letters = 32.0;
  static const adding = 40.0;

  final String text;
  final MarkdownTable table;
  final TextStyle? style;
  final BoxTicking? ticking;
  final bool bar;
  final FocusNode? focusParent;
  final GridWritten Function(MarkdownChange change) onChange;
  final void Function({required bool forward}) onLeave;
  final VoidCallback onShowLines;

  const TableGrid({
    super.key,
    required this.text,
    required this.table,
    required this.onChange,
    required this.onLeave,
    required this.onShowLines,
    this.style,
    this.ticking,
    this.bar = true,
    this.focusParent,
  });

  @override
  State<TableGrid> createState() => TableGridState();
}

/// Where a key moves from the cell being edited.
enum _Move { next, previous, above, below }

/// What a heading's menu does.
enum _Op {
  before,
  after,
  back,
  forward,
  delete,
  headings,
  alignLeft,
  alignCenter,
  alignRight,
}

class TableGridState extends State<TableGrid> {
  late String _text = widget.text;
  late int _at = widget.table.start;
  late MarkdownTable? _table = widget.table;
  List<DrawnBox> _boxes = const [];
  List<FoundTag> _tags = const [];
  List<(int, int)> _links = const [];
  List<double> _widths = const [];

  /// The cell being edited and its words as typed: [_typedWritten] those
  /// last written, [_cellWritten] the cell as the text holds them since.
  GridCell? _editing;
  TagTextController? _draft;
  String _typedWritten = '';
  String? _cellWritten;

  final _focus = FocusNode(debugLabel: 'table cell');
  final _editor = GlobalKey();

  /// The bar of the cell's field, shown from here: the field moves from
  /// cell to cell.
  final _bar = TwoRowBarHost();
  final _barShown = OverlayPortalController();
  final _sideways = ScrollController();
  ModalRoute<Object?>? _route;

  /// The editor field's text actions, left to the cell's own field.
  Map<Type, Action<Intent>> _deferred = const {};

  /// Whether a cell is being edited.
  bool get editing => _editing != null;

  @override
  void initState() {
    super.initState();
    _focus.addListener(_focusChanged);
    _read();
    _barShown.show();
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _route = ModalRoute.of(context);
    // A cell's field sits in the editor's: the actions of the editor's
    // field would act for its own (Action.overridable).
    final types = <Type>{};
    context.visitAncestorElements((element) {
      final widget = element.widget;
      if (widget is Actions) types.addAll(widget.actions.keys);
      return widget is! EditableText;
    });
    _deferred = {for (final type in types) type: _Deferred()};
    _measure();
  }

  @override
  void didUpdateWidget(TableGrid oldWidget) {
    super.didUpdateWidget(oldWidget);
    _text = widget.text;
    _at = widget.table.start;
    _table = widget.table;
    _read();
    _draft?.boxes = widget.ticking;
    final (cell, draft) = (_editing, _draft);
    if (cell != null && draft != null) {
      final now = MarkdownTables.cell(_text, _at, cell.row, cell.column);
      if (now == null) {
        _drop();
      } else if (now != _cellWritten) {
        // Changed from elsewhere: kept with the ids of its new boxes, the
        // words stay as typed; else it is no longer the cell edited.
        if (withoutIds(now) == withoutIds(draft.text)) {
          final typed = draft.text;
          int moved(int offset) => offset < 0
              ? offset
              : idFreeOffset(now, idFreeCount(typed, offset));
          final selection = draft.selection;
          _cellWritten = now;
          _typedWritten = now;
          draft.value = TextEditingValue(
            text: now,
            selection: selection.copyWith(
              baseOffset: moved(selection.baseOffset),
              extentOffset: moved(selection.extentOffset),
            ),
          );
        } else {
          _drop();
        }
      }
    }
    _measure();
  }

  @override
  void dispose() {
    _focus.removeListener(_focusChanged);
    _focus.dispose();
    _draft?.removeListener(_typed);
    _draft?.dispose();
    _sideways.dispose();
    _bar.dispose();
    super.dispose();
  }

  void _read() {
    _boxes = storedBoxes(_text, widget.ticking);
    _tags = Markdown.tags(_text);
    _links = Markdown.autolinks(_text);
  }

  TextStyle _base() => DefaultTextStyle.of(context).style.merge(widget.style);

  /// Each column as wide as its widest cell's words, within bounds; the
  /// cell being edited as wide as its words as typed.
  void _measure() {
    final table = _table;
    if (table == null) return;
    final base = _base();
    final bold = base.copyWith(fontWeight: FontWeight.bold);
    final scaler = MediaQuery.textScalerOf(context);
    final direction = Directionality.of(context);
    double width(String words, TextStyle style) {
      final painter = TextPainter(
        text: TextSpan(text: words, style: style),
        textDirection: direction,
        textScaler: scaler,
        maxLines: 1,
      )..layout();
      final width = painter.width;
      painter.dispose();
      return width;
    }

    final editing = _editing;
    final typed = _draft?.text;
    _widths = [
      for (var c = 0; c < table.columns; c++)
        [
                  for (final (r, row) in table.rows.indexed)
                    if (editing == (row: r, column: c) && typed != null)
                      width(typed, r == 0 ? bold : base) + 4
                    else
                      width(row.cells[c].visible, bold),
                ]
                .fold(0.0, math.max)
                .clamp(TableScreen.narrowest - 24, TableScreen.widest - 24) +
            24,
    ];
  }

  double _widthOf(int column) =>
      column < _widths.length ? _widths[column] : TableScreen.narrowest;

  /// Edits [cell], the cursor at the start of its words or with [atEnd]
  /// at their end.
  void edit(GridCell cell, {bool atEnd = false}) {
    final words = MarkdownTables.cell(_text, _at, cell.row, cell.column);
    if (words == null) return;
    final draft = _draft ?? (TagTextController()..addListener(_typed));
    _draft = draft;
    draft
      ..markdown = true
      ..boxes = widget.ticking;
    _typedWritten = words;
    _cellWritten = words;
    draft.value = TextEditingValue(
      text: words,
      selection: TextSelection.collapsed(offset: atEnd ? words.length : 0),
    );
    setState(() {
      _editing = cell;
      _measure();
    });
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted || _editing != cell) return;
      _focus.requestFocus();
      _reveal(cell.column);
    });
  }

  /// Ends editing; the cell is written already.
  void stop() {
    if (_editing == null) return;
    setState(_drop);
  }

  /// The cell's field goes with the next frame, its words after it.
  void _drop() {
    final old = _draft;
    _editing = null;
    _draft = null;
    if (old == null) return;
    old.removeListener(_typed);
    WidgetsBinding.instance.addPostFrameCallback((_) => old.dispose());
    _measure();
  }

  /// The cell's field lost the focus: to the editor's text or another
  /// field the cell ends; to a dialog or a menu it stays, to come back.
  void _focusChanged() {
    if (_focus.hasFocus || _editing == null) return;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted || _focus.hasFocus || _editing == null) return;
      final primary = FocusManager.instance.primaryFocus?.context;
      if (primary != null && ModalRoute.of(primary) != _route) return;
      stop();
    });
  }

  /// Each change of the words typed is written at once; a line break
  /// typed goes to the cell below, one pasted is a space.
  void _typed() {
    final (cell, draft) = (_editing, _draft);
    if (cell == null || draft == null) return;
    final typed = draft.text;
    if (typed == _typedWritten) return;
    final breaks = RegExp(r'\r\n|\r|\n');
    if (breaks.hasMatch(typed)) {
      if (typed.replaceAll(breaks, '') == _typedWritten) {
        final at = typed.indexOf(breaks);
        draft.value = TextEditingValue(
          text: _typedWritten,
          selection: TextSelection.collapsed(offset: at),
        );
        _move(_Move.below);
      } else {
        draft.value = draft.value.copyWith(
          text: typed.replaceAll(breaks, ' '),
          composing: TextRange.empty,
        );
      }
      return;
    }
    _typedWritten = typed;
    _write(MarkdownTables.setCell(_text, _at, cell.row, cell.column, typed));
    _cellWritten = MarkdownTables.cell(_text, _at, cell.row, cell.column);
    setState(_measure);
  }

  /// Gives [change] to the field and keeps the text as it holds it.
  void _write(MarkdownChange change) {
    final written = widget.onChange(change);
    _text = written.text;
    _at = written.at;
    _table = MarkdownTables.at(_text, _at);
    _read();
  }

  /// Makes [change] when it changes the text and draws the table it
  /// gives.
  void _apply(MarkdownChange change) {
    if (change.text == _text) return;
    _write(change);
    setState(_measure);
  }

  /// The keys of a cell being edited: Tab, Shift+Tab, ↵ and the arrows.
  KeyEventResult _key(FocusNode node, KeyEvent event) {
    if (event is! KeyDownEvent && event is! KeyRepeatEvent) {
      return KeyEventResult.ignored;
    }
    final draft = _draft;
    if (_editing == null || draft == null) return KeyEventResult.ignored;
    final keys = HardwareKeyboard.instance;
    if (keys.isControlPressed || keys.isAltPressed || keys.isMetaPressed) {
      return KeyEventResult.ignored;
    }
    final key = event.logicalKey;
    final selection = draft.selection;
    final collapsed = selection.isValid && selection.isCollapsed;
    final shift = keys.isShiftPressed;
    final _Move? move;
    if (key == LogicalKeyboardKey.tab) {
      move = shift ? _Move.previous : _Move.next;
    } else if (shift) {
      move = null;
    } else if (key == LogicalKeyboardKey.enter ||
        key == LogicalKeyboardKey.numpadEnter ||
        key == LogicalKeyboardKey.arrowDown) {
      move = _Move.below;
    } else if (key == LogicalKeyboardKey.arrowUp) {
      move = _Move.above;
    } else if (key == LogicalKeyboardKey.arrowLeft &&
        collapsed &&
        selection.baseOffset == 0) {
      move = _Move.previous;
    } else if (key == LogicalKeyboardKey.arrowRight &&
        collapsed &&
        selection.baseOffset == draft.text.length) {
      move = _Move.next;
    } else {
      move = null;
    }
    if (move == null) return KeyEventResult.ignored;
    _move(move);
    return KeyEventResult.handled;
  }

  /// Goes from the cell being edited as [move] says; past the first or
  /// the last cell into the text before or after the table.
  void _move(_Move move) {
    final (cell, table) = (_editing, _table);
    if (cell == null || table == null) return;
    var (row, column) = (cell.row, cell.column);
    final columns = table.columns;
    switch (move) {
      case _Move.next:
        column++;
        if (column >= columns) (row, column) = (row + 1, 0);
      case _Move.previous:
        column--;
        if (column < 0) (row, column) = (row - 1, columns - 1);
      case _Move.above:
        row--;
      case _Move.below:
        row++;
    }
    if (row < 0 || row >= table.rows.length) {
      stop();
      widget.onLeave(forward: row >= 0);
      return;
    }
    edit((row: row, column: column), atEnd: move != _Move.next);
  }

  /// Moves the grid sideways so that [column] shows, when it moves.
  void _reveal(int column) {
    if (!_sideways.hasClients) return;
    var start = 0.0;
    for (var c = 0; c < column; c++) {
      start += _widthOf(c);
    }
    final end = TableGrid.gutter + start + _widthOf(column);
    final position = _sideways.position;
    if (start < position.pixels) {
      _sideways.jumpTo(start);
    } else if (end > position.pixels + position.viewportDimension) {
      _sideways.jumpTo(
        math.min(position.maxScrollExtent, end - position.viewportDimension),
      );
    }
  }

  void _addRow() {
    final table = _table;
    if (table == null) return;
    stop();
    final rows = table.rows.length;
    _apply(MarkdownTables.insertRow(_text, _at, rows));
    edit((row: rows, column: 0));
  }

  void _addColumn() {
    final table = _table;
    if (table == null) return;
    stop();
    final columns = table.columns;
    _apply(MarkdownTables.insertColumn(_text, _at, columns));
    edit((row: 0, column: columns));
  }

  void _showLines() {
    stop();
    widget.onShowLines();
  }

  /// The menu of the row [index] or, with [column], of the column
  /// [index], at [at] (global).
  Future<void> _menu(int index, Offset at, {required bool column}) async {
    final table = _table;
    if (table == null) return;
    stop();
    final t = context.t;
    final overlay =
        Overlay.of(context).context.findRenderObject() as RenderBox?;
    if (overlay == null) return;
    final count = column ? table.columns : table.rows.length;
    PopupMenuItem<_Op> item(
      _Op op,
      IconData icon,
      String label, {
      bool checked = false,
    }) => PopupMenuItem(
      key: ValueKey('table-${op.name}'),
      value: op,
      enabled: switch (op) {
        _Op.back => index > 0,
        _Op.forward => index + 1 < count,
        _ => true,
      },
      child: Row(
        children: [
          Icon(icon),
          const SizedBox(width: 12),
          Flexible(child: Text(label)),
          if (checked) ...[
            const SizedBox(width: 12),
            const Icon(Icons.check, size: 20),
          ],
        ],
      ),
    );
    final align = column ? table.aligns[index] : MarkdownAlign.none;
    final picked = await showMenu<_Op>(
      context: context,
      position: RelativeRect.fromRect(
        overlay.globalToLocal(at) & Size.zero,
        Offset.zero & overlay.size,
      ),
      items: column
          ? [
              item(_Op.before, Icons.west, t.tableColumnBefore),
              item(_Op.after, Icons.east, t.tableColumnAfter),
              item(_Op.back, Icons.arrow_back, t.tableColumnLeft),
              item(_Op.forward, Icons.arrow_forward, t.tableColumnRight),
              item(
                _Op.alignLeft,
                Icons.format_align_left,
                t.tableAlignLeft,
                checked:
                    align == MarkdownAlign.none || align == MarkdownAlign.left,
              ),
              item(
                _Op.alignCenter,
                Icons.format_align_center,
                t.tableAlignCenter,
                checked: align == MarkdownAlign.center,
              ),
              item(
                _Op.alignRight,
                Icons.format_align_right,
                t.tableAlignRight,
                checked: align == MarkdownAlign.right,
              ),
              item(_Op.delete, Icons.delete_outline, t.tableColumnDelete),
              if (index == 0)
                PopupMenuItem(
                  key: const ValueKey('table-headings'),
                  value: _Op.headings,
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Text(t.tableHeadingColumn),
                      LookGrown(
                        child: Switch(
                          value: table.headingColumn,
                          onChanged: (_) =>
                              Navigator.of(context).pop(_Op.headings),
                        ),
                      ),
                    ],
                  ),
                ),
            ]
          : [
              item(_Op.before, Icons.north, t.tableRowAbove),
              item(_Op.after, Icons.south, t.tableRowBelow),
              item(_Op.back, Icons.arrow_upward, t.tableRowUp),
              item(_Op.forward, Icons.arrow_downward, t.tableRowDown),
              item(_Op.delete, Icons.delete_outline, t.tableRowDelete),
            ],
    );
    if (!mounted || picked == null) return;
    final (text, where) = (_text, _at);
    switch ((picked, column)) {
      case (_Op.before, false):
        _apply(MarkdownTables.insertRow(text, where, index));
      case (_Op.after, false):
        _apply(MarkdownTables.insertRow(text, where, index + 1));
      case (_Op.back, false):
        _apply(MarkdownTables.moveRow(text, where, index, index - 1));
      case (_Op.forward, false):
        _apply(MarkdownTables.moveRow(text, where, index, index + 1));
      case (_Op.before, true):
        _apply(MarkdownTables.insertColumn(text, where, index));
      case (_Op.after, true):
        _apply(MarkdownTables.insertColumn(text, where, index + 1));
      case (_Op.back, true):
        _apply(MarkdownTables.moveColumn(text, where, index, index - 1));
      case (_Op.forward, true):
        _apply(MarkdownTables.moveColumn(text, where, index, index + 1));
      case (_Op.headings, _):
        _apply(MarkdownTables.toggleHeadingColumn(text, where));
      // Left is how a column lines up without saying so.
      case (_Op.alignLeft, _):
        if (align != MarkdownAlign.none) {
          _apply(
            MarkdownTables.setAlign(text, where, index, MarkdownAlign.left),
          );
        }
      case (_Op.alignCenter, _):
        _apply(
          MarkdownTables.setAlign(text, where, index, MarkdownAlign.center),
        );
      case (_Op.alignRight, _):
        _apply(
          MarkdownTables.setAlign(text, where, index, MarkdownAlign.right),
        );
      case (_Op.delete, _):
        await _delete(table, index, column: column);
    }
  }

  /// Deletes the row or column [index] after asking once while it holds
  /// words; the last one takes the table with it.
  Future<void> _delete(
    MarkdownTable table,
    int index, {
    required bool column,
  }) async {
    final gone = column ? table.columns == 1 : table.rows.length == 1;
    final cells = column
        ? [for (final row in table.rows) row.cells[index]]
        : table.rows[index].cells;
    final holds = cells.any((cell) => cell.end > cell.start);
    if (holds &&
        !await confirmTableDelete(context, gone: gone, column: column)) {
      return;
    }
    if (!mounted) return;
    _apply(
      column
          ? MarkdownTables.deleteColumn(_text, _at, index)
          : MarkdownTables.deleteRow(_text, _at, index),
    );
  }

  @override
  Widget build(BuildContext context) {
    final table = _table;
    if (table == null) return const SizedBox.shrink();
    final t = context.t;
    // One table of all the cells, so that the field of the cell being
    // edited moves from one to the next as itself, keeping the keyboard.
    return OverlayPortal(
      controller: _barShown,
      overlayChildBuilder: (context) {
        // Drawn anew as the keyboard opens and closes.
        MediaQuery.viewInsetsOf(context);
        return ListenableBuilder(
          listenable: Listenable.merge([_focus, _bar]),
          builder: (context, _) => _bar.build(context),
        );
      },
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 4),
        child: SingleChildScrollView(
          key: const ValueKey('table-grid-sideways'),
          controller: _sideways,
          scrollDirection: Axis.horizontal,
          child: Table(
            defaultColumnWidth: const FixedColumnWidth(TableGrid.adding),
            columnWidths: {
              0: const FixedColumnWidth(TableGrid.gutter),
              for (var c = 0; c < table.columns; c++)
                c + 1: FixedColumnWidth(_widthOf(c)),
            },
            defaultVerticalAlignment:
                TableCellVerticalAlignment.intrinsicHeight,
            children: [
              TableRow(
                children: [
                  SizedBox(
                    height: TableGrid.letters,
                    child: IconButton(
                      key: const ValueKey('table-show-lines'),
                      tooltip: t.tableShowLines,
                      padding: EdgeInsets.zero,
                      iconSize: 20,
                      onPressed: _showLines,
                      icon: const Icon(Icons.notes),
                    ),
                  ),
                  for (var c = 0; c < table.columns; c++)
                    SizedBox(
                      height: TableGrid.letters,
                      child: _Heading(
                        key: ValueKey('grid-column-$c'),
                        text: TableScreen.letter(c),
                        label: t.tableColumn(TableScreen.letter(c)),
                        onMenu: (at) => unawaited(_menu(c, at, column: true)),
                      ),
                    ),
                  SizedBox(
                    height: TableGrid.letters,
                    child: _Heading(
                      key: const ValueKey('grid-add-column'),
                      text: '+',
                      label: t.tableAddColumn,
                      onTap: _addColumn,
                    ),
                  ),
                ],
              ),
              for (var r = 0; r < table.rows.length; r++)
                TableRow(
                  children: [
                    _Heading(
                      key: ValueKey('grid-row-$r'),
                      text: '${r + 1}',
                      label: t.tableRow(r + 1),
                      onMenu: (at) => unawaited(_menu(r, at, column: false)),
                    ),
                    for (var c = 0; c < table.columns; c++)
                      _cell(context, table, r, c),
                    const SizedBox.shrink(),
                  ],
                ),
              TableRow(
                children: [
                  SizedBox(
                    height: TableGrid.letters,
                    child: _Heading(
                      key: const ValueKey('grid-add-row'),
                      text: '+',
                      label: t.tableAddRow,
                      onTap: _addRow,
                    ),
                  ),
                  for (var c = 0; c <= table.columns; c++)
                    const SizedBox.shrink(),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _cell(BuildContext context, MarkdownTable table, int r, int c) {
    final scheme = Theme.of(context).colorScheme;
    final base = _base();
    final heading = r == 0 || (c == 0 && table.headingColumn);
    final style = heading ? base.copyWith(fontWeight: FontWeight.bold) : base;
    final cell = (row: r, column: c);
    final draft = _draft;
    final editing = _editing == cell && draft != null;
    final Widget content = editing
        ? KeyedSubtree(
            key: _editor,
            child: Actions(
              actions: _deferred,
              child: Focus(
                parentNode: widget.focusParent,
                canRequestFocus: false,
                skipTraversal: true,
                onKeyEvent: _key,
                child: TwoRowTextField(
                  controller: draft,
                  focusNode: _focus,
                  twoRows: false,
                  boxes: true,
                  formats: true,
                  lineFormats: false,
                  bar: widget.bar,
                  style: style,
                  decoration: const InputDecoration.collapsed(hintText: null),
                  onNext: () => _move(_Move.below),
                  barHost: _bar,
                ),
              ),
            ),
          )
        : MarkedCell(
            _text,
            table.rows[r].cells[c],
            boxes: _boxes,
            tags: _tags,
            links: _links,
            style: style,
            textAlign: cellAlign(table.aligns[c]),
            ticking: widget.ticking,
          );
    return Semantics(
      label: context.t.tableCell(r + 1, TableScreen.letter(c)),
      child: GestureDetector(
        key: ValueKey('grid-cell-$r-$c'),
        behavior: HitTestBehavior.opaque,
        onTap: editing ? null : () => edit(cell, atEnd: true),
        child: DecoratedBox(
          decoration: BoxDecoration(
            color: heading ? scheme.surfaceContainerHighest : scheme.surface,
            border: editing
                ? Border.all(color: scheme.primary, width: 2)
                : Border.all(color: scheme.outline, width: 0.5),
          ),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
            child: content,
          ),
        ),
      ),
    );
  }
}

/// An action of the editor's field that leaves the intent to the action
/// it overrides, the cell's own field's.
class _Deferred extends Action<Intent> {
  @override
  bool isEnabled(Intent intent, [BuildContext? context]) =>
      callingAction?.isEnabled(intent) ?? false;

  @override
  bool consumesKey(Intent intent) =>
      callingAction?.consumesKey(intent) ?? false;

  @override
  Object? invoke(Intent intent) => callingAction?.invoke(intent);
}

/// A row's number or a column's letter: a tap or a long press opens its
/// menu at the finger ([onMenu]); "+" adds one with a tap ([onTap]).
class _Heading extends StatelessWidget {
  final String text;
  final String label;
  final void Function(Offset at)? onMenu;
  final VoidCallback? onTap;

  const _Heading({
    super.key,
    required this.text,
    required this.label,
    this.onMenu,
    this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final onMenu = this.onMenu;
    void atMe() {
      // A screen reader has no finger: the menu opens at the heading.
      final box = context.findRenderObject() as RenderBox?;
      if (box == null || onMenu == null) return;
      onMenu(box.localToGlobal(box.size.center(Offset.zero)));
    }

    return Semantics(
      button: true,
      label: label,
      onTap: onMenu == null ? onTap : atMe,
      excludeSemantics: true,
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTapUp: onMenu == null ? null : (d) => onMenu(d.globalPosition),
        onTap: onMenu == null ? onTap : null,
        onLongPressStart: onMenu == null
            ? null
            : (d) => onMenu(d.globalPosition),
        child: ColoredBox(
          color: scheme.surfaceContainer,
          child: Center(
            child: Text(
              text,
              style: TextStyle(
                color: scheme.primary,
                fontWeight: FontWeight.bold,
              ),
            ),
          ),
        ),
      ),
    );
  }
}

/// Lets the grids of a field ([grids]) take the pointers that land on
/// them, ahead of the field's own gestures, which never see them: the
/// field's text takes the rest.
class GridHits extends SingleChildRenderObjectWidget {
  final List<RenderBox> Function() grids;

  const GridHits({super.key, required this.grids, super.child});

  @override
  RenderObject createRenderObject(BuildContext context) =>
      RenderGridHits(grids);

  @override
  void updateRenderObject(BuildContext context, RenderGridHits renderObject) =>
      renderObject.grids = grids;
}

/// The render object of [GridHits].
class RenderGridHits extends RenderProxyBox {
  List<RenderBox> Function() grids;

  RenderGridHits(this.grids);

  @override
  bool hitTest(BoxHitTestResult result, {required Offset position}) {
    if (!size.contains(position)) return false;
    for (final grid in grids()) {
      if (!grid.attached || !grid.hasSize) continue;
      final hit = result.addWithPaintTransform(
        transform: grid.getTransformTo(this),
        position: position,
        hitTest: (result, local) =>
            grid.size.contains(local) && grid.hitTest(result, position: local),
      );
      if (hit) return true;
    }
    return super.hitTest(result, position: position);
  }
}
