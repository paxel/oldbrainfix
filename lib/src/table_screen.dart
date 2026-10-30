import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'l10n.dart';
import 'system_bars.dart';
import 'text_marks_view.dart';
import 'two_row_text_field.dart';

/// How the tables of a text may be changed in their full-screen view:
/// [write] takes the text after each change, its start and end the
/// table's in it; [twoRows] says whether a cell types in two rows.
class TableEditing {
  final void Function(MarkdownChange change) write;
  final bool twoRows;

  const TableEditing({required this.write, required this.twoRows});
}

/// Opens the table of the formatted [text] at [at] full screen, changed
/// through [edit] or, without it, only read; [cell] starts being edited.
/// The search for a tag and the opening of a link work there as where it
/// was opened from.
Future<void> openTable(
  BuildContext context, {
  required String text,
  required int at,
  TableEditing? edit,
  ({int row, int column})? cell,
}) {
  final onSearch = TagSearchScope.of(context);
  final onOpen = LinkOpenScope.of(context);
  return Navigator.of(context).push(
    MaterialPageRoute<void>(
      builder: (_) => LinkOpenScope(
        onOpen: onOpen,
        child: TagSearchScope(
          onSearch: onSearch,
          child: TableScreen(text: text, at: at, edit: edit, cell: cell),
        ),
      ),
    ),
  );
}

/// A table of a formatted text full screen (ADR 0033): its header row
/// stays on screen while the rows scroll up and down, and so does its
/// first column when it holds headings. Sideways the table moves by the
/// bar above it, not by a swipe, which is navigation (UI laws).
///
/// With [edit], a tap on a cell edits it in place, its frame drawn, its
/// words typed as in the Note editor, in two rows when set so, with the
/// bar of formats of a line's parts; the cell is written when another
/// one is tapped, the space around them, ↵, or Back. A row's number and
/// a column's letter open its menu at the finger: put one in before or
/// after it, move it, delete it, and on the first column whether it
/// holds headings. The empty row under the table and the empty column
/// beside it add one when a word is written into them.
class TableScreen extends StatefulWidget {
  /// How wide a column may get at most and at least, and how wide the
  /// empty column that adds one is.
  static const widest = 280.0;
  static const narrowest = 72.0;
  static const adding = 56.0;

  /// How wide the row numbers are, and how high the column letters and
  /// the bar that moves the table sideways.
  static const gutter = 40.0;
  static const letters = 32.0;
  static const bar = 28.0;

  final String text;
  final int at;
  final TableEditing? edit;
  final ({int row, int column})? cell;

  const TableScreen({
    super.key,
    required this.text,
    required this.at,
    this.edit,
    this.cell,
  });

  /// The letter of the column [index]: A to Z, then AA and on.
  static String letter(int index) {
    var name = '';
    var i = index;
    do {
      name = String.fromCharCode(0x41 + i % 26) + name;
      i = i ~/ 26 - 1;
    } while (i >= 0);
    return name;
  }

  @override
  State<TableScreen> createState() => _TableScreenState();
}

/// What a heading's menu does.
enum _Op { before, after, back, forward, delete, headings }

class _TableScreenState extends State<TableScreen> {
  late String _text = widget.text;
  late int _at = widget.at;
  MarkdownTable? _table;
  List<MarkdownMark> _marks = const [];
  List<FoundTag> _tags = const [];
  List<double> _widths = const [];

  /// How far the table is moved sideways, and how far it can be.
  final _offset = ValueNotifier<double>(0);
  double _most = 0;
  double _room = 0;

  /// The cell being edited, the row or column after the last one adding
  /// one, and its words as typed.
  ({int row, int column})? _editing;
  TagTextController? _cell;
  late final AppLifecycleListener _lifecycle;

  bool get _edits => widget.edit != null;

  @override
  void initState() {
    super.initState();
    _read();
    _lifecycle = AppLifecycleListener(onStateChange: _lifecycleChanged);
    if (widget.cell case final cell? when _edits) _startEditing(cell);
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _measure();
  }

  @override
  void dispose() {
    _lifecycle.dispose();
    _cell?.removeListener(_typed);
    _cell?.dispose();
    _offset.dispose();
    super.dispose();
  }

  /// The app may be killed in the background: the cell is written.
  void _lifecycleChanged(AppLifecycleState state) {
    if (state == AppLifecycleState.paused ||
        state == AppLifecycleState.hidden) {
      _commit();
    }
  }

  void _read() {
    _table = MarkdownTables.at(_text, _at);
    _marks = Markdown.marks(_text);
    _tags = Markdown.tags(_text);
  }

  /// Each column as wide as its widest cell's words, within bounds.
  void _measure() {
    final table = _table;
    if (table == null) return;
    final style = DefaultTextStyle.of(context).style
        .merge(Theme.of(context).textTheme.bodyLarge);
    final bold = style.copyWith(fontWeight: FontWeight.bold);
    final scaler = MediaQuery.textScalerOf(context);
    final direction = Directionality.of(context);
    _widths = [
      for (var c = 0; c < table.columns; c++)
        [
                  for (final row in table.rows)
                    (TextPainter(
                      text: TextSpan(text: row.cells[c].visible, style: bold),
                      textDirection: direction,
                      textScaler: scaler,
                      maxLines: 1,
                    )..layout()).width,
                ]
                .fold(0.0, math.max)
                .clamp(TableScreen.narrowest - 24, TableScreen.widest - 24) +
            24,
    ];
  }

  /// Writes [change] when it changed the text, and shows the table it
  /// gives; [gone] when the table went with it, which closes the screen.
  void _apply(MarkdownChange change, {bool gone = false}) {
    if (change.text == _text) return;
    _text = change.text;
    _at = change.start;
    widget.edit?.write(change);
    if (gone) {
      _table = null;
      Navigator.of(context).pop();
      return;
    }
    setState(() {
      _read();
      _measure();
    });
  }

  void _startEditing(({int row, int column}) cell) {
    final table = _table;
    if (table == null) return;
    final controller = TagTextController(
      text: MarkdownTables.cell(_text, _at, cell.row, cell.column) ?? '',
      hideMarks: widget.edit?.twoRows ?? false,
    )..markdown = true;
    controller.addListener(_typed);
    _editing = cell;
    _cell = controller;
    _reveal(cell.column);
  }

  /// A tap on [cell]: the one edited before is written, this one edited.
  void _edit(({int row, int column}) cell) {
    if (_editing == cell) return;
    _commit();
    _dropCell();
    setState(() => _startEditing(cell));
  }

  /// Ends editing, the cell written.
  void _finish() {
    if (_editing == null) return;
    _commit();
    setState(_dropCell);
  }

  /// The field of the cell goes with the next frame; its controller after
  /// it.
  void _dropCell() {
    final old = _cell;
    _editing = null;
    _cell = null;
    if (old == null) return;
    old.removeListener(_typed);
    WidgetsBinding.instance.addPostFrameCallback((_) => old.dispose());
  }

  /// ↵ ends the cell: it holds one line.
  void _typed() {
    final text = _cell?.text ?? '';
    if (text.contains('\n') || text.contains('\r')) _finish();
  }

  /// Writes the cell being edited when its words changed; one in the
  /// empty row or column adds that row or column, but only with words.
  void _commit() {
    final (editing, cell, table) = (_editing, _cell, _table);
    if (editing == null || cell == null || table == null) return;
    final typed = cell.text;
    final (:row, :column) = editing;
    final rows = table.rows.length;
    final columns = table.columns;
    final adds = row >= rows || column >= columns;
    if (adds && typed.trim().isEmpty) return;
    if (!adds && typed == MarkdownTables.cell(_text, _at, row, column)) {
      return;
    }
    var text = _text;
    var at = _at;
    if (row >= rows) {
      final change = MarkdownTables.insertRow(text, at, rows);
      (text, at) = (change.text, change.start);
    }
    if (column >= columns) {
      final change = MarkdownTables.insertColumn(text, at, columns);
      (text, at) = (change.text, change.start);
    }
    // What is typed on goes into the row or column now there.
    _apply(MarkdownTables.setCell(text, at, row, column, typed));
  }

  /// Moves the table sideways so that [column] shows, when it moves.
  void _reveal(int column) {
    final table = _table;
    if (table == null || _room <= 0) return;
    final first = table.headingColumn ? 1 : 0;
    if (column < first) return;
    var start = 0.0;
    for (var c = first; c < column && c < _widths.length; c++) {
      start += _widths[c];
    }
    final width = column < _widths.length
        ? _widths[column]
        : TableScreen.adding;
    final now = _offset.value;
    if (start < now) {
      _offset.value = start;
    } else if (start + width > now + _room) {
      _offset.value = math.min(start + width - _room, math.max(_most, start));
    }
  }

  /// A mark in a cell changed to [to].
  void _swap(int line, TextMark to, {int? column}) {
    _commit();
    final swapped = Markdown.swapMark(_text, line, to, column: column);
    final table = _table;
    if (swapped == null || table == null) return;
    _apply((text: swapped, start: _at, end: _at));
  }

  /// The menu of the row [row] ([column] false) or of the column [row],
  /// at [at] (global).
  Future<void> _menu(int index, Offset at, {required bool column}) async {
    final table = _table;
    if (table == null) return;
    _finish();
    final t = context.t;
    final overlay =
        Overlay.of(context).context.findRenderObject() as RenderBox?;
    if (overlay == null) return;
    final count = column ? table.columns : table.rows.length;
    PopupMenuItem<_Op> item(_Op op, IconData icon, String label) =>
        PopupMenuItem(
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
            ],
          ),
        );
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
                      Switch(
                        value: table.headingColumn,
                        onChanged: (_) =>
                            Navigator.of(context).pop(_Op.headings),
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
    if (holds && !await _confirmDelete(gone: gone, column: column)) return;
    if (!mounted) return;
    _apply(
      column
          ? MarkdownTables.deleteColumn(_text, _at, index)
          : MarkdownTables.deleteRow(_text, _at, index),
      gone: gone,
    );
  }

  Future<bool> _confirmDelete({
    required bool gone,
    required bool column,
  }) async {
    final sure = await showDialog<bool>(
      context: context,
      builder: (context) {
        final t = context.t;
        return AlertDialog(
          title: Text(
            gone
                ? t.tableDeleteTableQuestion
                : column
                ? t.tableDeleteColumnQuestion
                : t.tableDeleteRowQuestion,
          ),
          content: Text(t.tableDeleteBody),
          actions: [
            TextButton(
              onPressed: () => Navigator.of(context).pop(false),
              child: Text(t.cancel),
            ),
            TextButton(
              key: const ValueKey('table-delete-confirm'),
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
    final t = context.t;
    final table = _table;
    return PopScope(
      onPopInvokedWithResult: (didPop, _) {
        if (didPop) _commit();
      },
      child: Scaffold(
        appBar: AppBar(title: Text(t.tableTitle)),
        body: table == null
            ? const SizedBox.shrink()
            : LayoutBuilder(
                builder: (context, constraints) =>
                    _grid(context, table, constraints.maxWidth),
              ),
      ),
    );
  }

  Widget _grid(BuildContext context, MarkdownTable table, double width) {
    final t = context.t;
    final first = table.headingColumn ? 1 : 0;
    final fixed =
        (_edits ? TableScreen.gutter : 0) +
        (table.headingColumn ? _widths[0] : 0);
    var moving = _edits ? TableScreen.adding : 0.0;
    for (var c = first; c < table.columns; c++) {
      moving += _widths[c];
    }
    // Kept for the next tap on a cell, which moves it into view.
    _room = math.max(0, width - fixed);
    _most = math.max(0, moving - _room);
    final padding = clearOfSystemBars(context, EdgeInsets.zero);
    return Padding(
      padding: EdgeInsets.only(left: padding.left, right: padding.right),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (_most > 0)
            _SidewaysBar(
              key: const ValueKey('table-sideways'),
              offset: _offset,
              most: _most,
              room: _room,
              whole: moving,
              label: t.tableSideways,
            ),
          if (_edits) _letters(table),
          _row(context, table, 0),
          Expanded(
            child: GestureDetector(
              // A tap beside the cells ends editing.
              behavior: HitTestBehavior.translucent,
              onTap: _finish,
              child: ListView(
                key: const ValueKey('table-rows'),
                padding: EdgeInsets.only(bottom: padding.bottom + 16),
                children: [
                  for (var r = 1; r < table.rows.length; r++)
                    _row(context, table, r),
                  if (_edits) _row(context, table, table.rows.length),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }

  /// The columns' letters, each opening its column's menu.
  Widget _letters(MarkdownTable table) {
    final t = context.t;
    Widget letter(int c) => _Heading(
      key: ValueKey('column-$c'),
      text: c < table.columns ? TableScreen.letter(c) : '+',
      label: c < table.columns
          ? t.tableColumn(TableScreen.letter(c))
          : t.tableAddColumn,
      onMenu: c < table.columns
          ? (at) => unawaited(_menu(c, at, column: true))
          : null,
      onTap: c < table.columns
          ? null
          : () => _edit((row: 0, column: table.columns)),
    );
    final first = table.headingColumn ? 1 : 0;
    return SizedBox(
      height: TableScreen.letters,
      child: Row(
        children: [
          const SizedBox(width: TableScreen.gutter),
          if (table.headingColumn)
            SizedBox(width: _widths[0], child: letter(0)),
          Expanded(
            child: _Shifted(
              offset: _offset,
              most: _most,
              child: Row(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  for (var c = first; c < table.columns; c++)
                    SizedBox(width: _widths[c], child: letter(c)),
                  SizedBox(
                    width: TableScreen.adding,
                    child: letter(table.columns),
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }

  /// The row [r], the one after the last adding one; its first column
  /// stays when it holds headings.
  Widget _row(BuildContext context, MarkdownTable table, int r) {
    final t = context.t;
    final first = table.headingColumn ? 1 : 0;
    final adds = r >= table.rows.length;
    return IntrinsicHeight(
      key: ValueKey('table-row-$r'),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (_edits)
            SizedBox(
              width: TableScreen.gutter,
              child: _Heading(
                key: ValueKey('row-$r'),
                text: adds ? '+' : '${r + 1}',
                label: adds ? t.tableAddRow : t.tableRow(r + 1),
                onMenu: adds
                    ? null
                    : (at) => unawaited(_menu(r, at, column: false)),
                onTap: adds ? () => _edit((row: r, column: 0)) : null,
              ),
            ),
          if (table.headingColumn)
            SizedBox(width: _widths[0], child: _cellAt(context, table, r, 0)),
          Expanded(
            child: _Shifted(
              offset: _offset,
              most: _most,
              child: Row(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  for (var c = first; c < table.columns; c++)
                    SizedBox(
                      width: _widths[c],
                      child: _cellAt(context, table, r, c),
                    ),
                  if (_edits)
                    SizedBox(
                      width: TableScreen.adding,
                      child: _cellAt(context, table, r, table.columns),
                    ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }

  Widget _cellAt(BuildContext context, MarkdownTable table, int r, int c) {
    final scheme = Theme.of(context).colorScheme;
    final base = DefaultTextStyle.of(context).style
        .merge(Theme.of(context).textTheme.bodyLarge);
    final adds = r >= table.rows.length || c >= table.columns;
    final heading = !adds && (r == 0 || (c == 0 && table.headingColumn));
    final style = heading ? base.copyWith(fontWeight: FontWeight.bold) : base;
    final editing = _editing == (row: r, column: c);
    final cell = _cell;
    final edit = widget.edit;
    final Widget content;
    if (editing && cell != null && edit != null) {
      content = TwoRowTextField(
        controller: cell,
        twoRows: edit.twoRows,
        autofocus: true,
        marks: true,
        formats: true,
        lineFormats: false,
        style: style,
        decoration: const InputDecoration.collapsed(hintText: null),
      );
    } else if (adds) {
      content = const SizedBox.shrink();
    } else {
      content = MarkedCell(
        _text,
        table.rows[r].cells[c],
        marks: _marks,
        tags: _tags,
        style: style,
        textAlign: cellAlign(table.aligns[c]),
        onMark: edit == null ? null : _swap,
      );
    }
    final box = DecoratedBox(
      decoration: BoxDecoration(
        color: heading ? scheme.surfaceContainerHighest : scheme.surface,
        border: editing
            ? Border.all(color: scheme.primary, width: 2)
            : Border.all(
                color: adds ? scheme.outlineVariant : scheme.outline,
                width: 0.5,
              ),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
        child: content,
      ),
    );
    return GestureDetector(
      key: ValueKey('cell-$r-$c'),
      behavior: HitTestBehavior.opaque,
      onTap: edit == null || editing ? null : () => _edit((row: r, column: c)),
      child: box,
    );
  }
}

/// A row's number or a column's letter: a tap or a long press opens its
/// menu at the finger ([onMenu]); the one of the empty row or column
/// adds one with a tap ([onTap]).
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

/// [child] moved sideways by [offset], up to [most], cut to the room it
/// has.
class _Shifted extends StatelessWidget {
  final ValueListenable<double> offset;
  final double most;
  final Widget child;

  const _Shifted({
    required this.offset,
    required this.most,
    required this.child,
  });

  @override
  Widget build(BuildContext context) => ClipRect(
    child: OverflowBox(
      alignment: Alignment.topLeft,
      minWidth: 0,
      maxWidth: double.infinity,
      child: ValueListenableBuilder<double>(
        valueListenable: offset,
        builder: (context, x, child) => Transform.translate(
          offset: Offset(-math.min(x, most), 0),
          child: child,
        ),
        child: child,
      ),
    ),
  );
}

/// The bar that moves the table sideways: its thumb shows the part of
/// the [whole] width that is in the [room] there is, and is dragged; a
/// tap beside it moves the table by a screen. A screen reader scrolls it
/// left and right.
class _SidewaysBar extends StatelessWidget {
  final ValueNotifier<double> offset;
  final double most;
  final double room;
  final double whole;
  final String label;

  const _SidewaysBar({
    super.key,
    required this.offset,
    required this.most,
    required this.room,
    required this.whole,
    required this.label,
  });

  void _moveTo(double x) => offset.value = x.clamp(0, most);

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Semantics(
      label: label,
      onScrollLeft: () => _moveTo(offset.value + room * 0.8),
      onScrollRight: () => _moveTo(offset.value - room * 0.8),
      child: SizedBox(
        height: TableScreen.bar,
        child: LayoutBuilder(
          builder: (context, constraints) {
            final track = constraints.maxWidth;
            final thumb = math.min(track, math.max(48.0, track * room / whole));
            final travel = math.max(1.0, track - thumb);
            return ValueListenableBuilder<double>(
              valueListenable: offset,
              builder: (context, x, _) {
                final left = travel * math.min(x, most) / most;
                return GestureDetector(
                  behavior: HitTestBehavior.opaque,
                  onTapUp: (d) => _moveTo(
                    d.localPosition.dx < left ? x - room * 0.8 : x + room * 0.8,
                  ),
                  onHorizontalDragUpdate: (d) =>
                      _moveTo(offset.value + d.delta.dx * most / travel),
                  child: Stack(
                    children: [
                      Positioned.fill(
                        child: Center(
                          child: Container(
                            height: 4,
                            color: scheme.surfaceContainerHighest,
                          ),
                        ),
                      ),
                      Positioned(
                        left: left,
                        width: thumb,
                        top: 6,
                        bottom: 6,
                        child: DecoratedBox(
                          key: const ValueKey('table-sideways-thumb'),
                          decoration: BoxDecoration(
                            color: scheme.primary,
                            borderRadius: BorderRadius.circular(8),
                          ),
                        ),
                      ),
                    ],
                  ),
                );
              },
            );
          },
        ),
      ),
    );
  }
}
