import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'box_ids.dart';
import 'l10n.dart';
import 'look.dart';
import 'page_bar.dart';
import 'photo_platform.dart';
import 'system_bars.dart';
import 'text_marks_view.dart';
import 'two_row_text_field.dart';

/// How the tables of a text may be changed in their full-screen view:
/// [write] takes the text after each change, its start and end the
/// table's in it, and gives the text as it is kept then, its new boxes
/// with their ids (ADR 0035), or null when it kept nothing; [twoRows]
/// says whether a cell types in two rows.
class TableEditing {
  final String? Function(MarkdownChange change) write;
  final bool twoRows;

  const TableEditing({required this.write, required this.twoRows});
}

/// Opens the table of the formatted [text] at [at] full screen, changed
/// through [edit] or, without it, only read; [cell] starts being edited.
/// [text] is the text as stored, with the ids of its boxes, which
/// [ticking] ticks. The search for a tag and the opening of a link work
/// there as where it was opened from.
Future<void> openTable(
  BuildContext context, {
  required String text,
  required int at,
  TableEditing? edit,
  BoxTicking? ticking,
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
          child: TableScreen(
            text: text,
            at: at,
            edit: edit,
            ticking: ticking,
            cell: cell,
          ),
        ),
      ),
    ),
  );
}

/// Asks once whether to delete the words of a row, of a [column] or,
/// when it is [gone], of the whole table; whether that was confirmed.
Future<bool> confirmTableDelete(
  BuildContext context, {
  bool gone = true,
  bool column = false,
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

/// Puts the table at [at] of [text], as stored, on the clipboard as an
/// HTML table and, as the plain text, as Markdown (#324), each box in
/// the state [ticking] shows. Where the platform takes no HTML, the
/// Markdown alone. Copied says nothing (UI laws).
Future<void> copyTable(
  BuildContext context, {
  required String text,
  required int at,
  BoxTicking? ticking,
}) async {
  final copy = MarkdownTables.copy(text, at, {
    for (final b in storedBoxes(text, ticking)) b.id: b.state,
  });
  if (copy == null) return;
  if (await PhotoScope.of(context).copyHtml(copy.markdown, copy.html)) {
    return;
  }
  await Clipboard.setData(ClipboardData(text: copy.markdown));
}

/// How a table being read shows its rows (obf-1052): sorted by its
/// column [column], ascending or [descending], in the view only.
typedef TableSort = ({int column, bool descending});

/// The sort a tap on the heading of [column] gives after [sort]:
/// ascending, then descending, then as written.
TableSort? nextTableSort(TableSort? sort, int column) => switch (sort) {
  (column: final c, descending: false) when c == column => (
    column: column,
    descending: true,
  ),
  (column: final c, descending: true) when c == column => null,
  _ => (column: column, descending: false),
};

/// The rows of [table] in the order [sort] shows them, each by its index
/// in [MarkdownTable.rows], the header row first; as written without a
/// sort or with one by a column the table no longer has.
List<int> tableRowOrder(MarkdownTable table, TableSort? sort) =>
    sort == null || sort.column >= table.columns
    ? [for (var r = 0; r < table.rows.length; r++) r]
    : [
        0,
        ...MarkdownTables.sortedRows(
          table,
          sort.column,
          descending: sort.descending,
        ),
      ];

/// How much wider the heading of the column a table is sorted by gets
/// for its arrow, at the text size [size] scaled by [scaler].
double tableSortRoom(TextScaler scaler, double size) => scaler.scale(size) + 4;

/// The words of a column's heading, [words], with the arrow of the
/// direction the rows are sorted by it, up for ascending, down for
/// [descending], at the text size [size].
class TableSortedHeading extends StatelessWidget {
  final Widget words;
  final bool descending;
  final double size;

  const TableSortedHeading({
    super.key,
    required this.words,
    required this.descending,
    required this.size,
  });

  @override
  Widget build(BuildContext context) => Row(
    crossAxisAlignment: CrossAxisAlignment.start,
    children: [
      Expanded(child: words),
      const SizedBox(width: 4),
      Icon(
        descending ? Icons.arrow_downward : Icons.arrow_upward,
        key: const ValueKey('table-sort-arrow'),
        size: MediaQuery.textScalerOf(context).scale(size),
      ),
    ],
  );
}

/// A column's heading where a tap sorts the rows shown by it
/// ([onSort], obf-1052), taking the tap before the table's or the card's
/// under it; a screen reader hears which way the rows are sorted by it:
/// [descending] true or false, null while they are not.
class TableSortTap extends StatelessWidget {
  final VoidCallback onSort;
  final bool? descending;
  final Widget child;

  const TableSortTap({
    super.key,
    required this.onSort,
    required this.descending,
    required this.child,
  });

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return Semantics(
      button: true,
      onTapHint: t.tableSort,
      value: switch (descending) {
        true => t.tableSortedDown,
        false => t.tableSortedUp,
        null => null,
      },
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTap: onSort,
        child: child,
      ),
    );
  }
}

/// What a table's ⋮ does (obf-1050).
enum TableAction { copy, up, down, delete }

/// A table's ⋮ at its top right corner (obf-1050): it takes its own tap,
/// never the card's or the table's under it, and tells [onMenu] where
/// the finger was (global), a screen reader's press at its middle.
class TableMenuButton extends StatefulWidget {
  final void Function(Offset at) onMenu;

  const TableMenuButton({super.key, required this.onMenu});

  @override
  State<TableMenuButton> createState() => _TableMenuButtonState();
}

class _TableMenuButtonState extends State<TableMenuButton> {
  Offset? _finger;

  @override
  Widget build(BuildContext context) => Listener(
    onPointerDown: (event) => _finger = event.position,
    child: IconButton(
      tooltip: context.t.tableMore,
      visualDensity: VisualDensity.compact,
      icon: const Icon(Icons.more_vert),
      onPressed: () {
        final finger = _finger;
        _finger = null;
        if (finger != null) return widget.onMenu(finger);
        final box = context.findRenderObject() as RenderBox?;
        if (box == null) return;
        widget.onMenu(box.localToGlobal(box.size.center(Offset.zero)));
      },
    ),
  );
}

/// Opens a table's ⋮ at [at] (global): Copy, and with [edits] Move up
/// while [up], Move down while [down], and Delete; what was picked, or
/// null.
Future<TableAction?> showTableMenu(
  BuildContext context,
  Offset at, {
  bool edits = false,
  bool up = false,
  bool down = false,
}) {
  final t = context.t;
  final overlay = Overlay.of(context).context.findRenderObject() as RenderBox?;
  if (overlay == null) return Future.value();
  PopupMenuItem<TableAction> item(
    TableAction action,
    IconData icon,
    String label, {
    bool enabled = true,
  }) => PopupMenuItem(
    key: ValueKey('table-menu-${action.name}'),
    value: action,
    enabled: enabled,
    child: Row(
      children: [
        Icon(icon),
        const SizedBox(width: 12),
        Flexible(child: Text(label)),
      ],
    ),
  );
  return showMenu<TableAction>(
    context: context,
    position: RelativeRect.fromRect(
      overlay.globalToLocal(at) & Size.zero,
      Offset.zero & overlay.size,
    ),
    items: [
      item(TableAction.copy, Icons.copy, t.tableCopy),
      if (edits) ...[
        item(TableAction.up, Icons.arrow_upward, t.tableUp, enabled: up),
        item(
          TableAction.down,
          Icons.arrow_downward,
          t.tableDown,
          enabled: down,
        ),
        item(TableAction.delete, Icons.delete_outline, t.tableDelete),
      ],
    ],
  );
}

/// A table of a formatted text full screen (ADR 0033): its header row
/// stays on screen while the rows scroll up and down, and so does its
/// first column when it holds headings. Sideways the table moves by the
/// bar above it and by a drag on its cells, the one place where a
/// horizontal swipe does not navigate (UI laws).
///
/// With [edit], a tap on a cell edits it in place, its frame drawn, its
/// words typed as in the Note editor, in two rows when set so, with the
/// bar of formats of a line's parts; the cell is written when another
/// one is tapped, the space around them, ↵, or Back. A row's number and
/// a column's letter open its menu at the finger: put one in before or
/// after it, move it, delete it, and on the first column whether it
/// holds headings. The empty row under the table and the empty column
/// beside it add one when a word is written into them.
///
/// A tap on a box of a cell opens its menu at the finger while there is
/// [ticking], also where the table is only read (ADR 0035).
///
/// The rows show sorted by a column in the view only (obf-1052), the
/// Note's text as written: where the table is only read a tap on a
/// column's heading sorts them ascending, then descending, then shows
/// them as written; with [edit], whose heading's tap edits it, its
/// letter's menu does. The screen forgets it when it closes.
///
/// Copy in the bar, and in the ⋮ at the table's top right corner
/// (obf-1050), puts the table on the clipboard as an HTML table and, as
/// the plain text, as Markdown (#324).
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
  final BoxTicking? ticking;
  final ({int row, int column})? cell;

  const TableScreen({
    super.key,
    required this.text,
    required this.at,
    this.edit,
    this.ticking,
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
enum _Op {
  before,
  after,
  back,
  forward,
  delete,
  headings,
  sortUp,
  sortDown,
  asWritten,
}

class _TableScreenState extends State<TableScreen> {
  late String _text = widget.text;
  late int _at = widget.at;
  MarkdownTable? _table;

  /// How the rows show sorted, in the view only (obf-1052), and the rows
  /// in that order by their index in the table, the header row first.
  TableSort? _sort;
  List<int> _order = const [];

  /// Read as each frame draws, so a tap shows while its save waits
  /// (#338).
  List<DrawnBox> get _boxes => storedBoxes(_text, _ticking);
  List<FoundTag> _tags = const [];
  List<(int, int)> _links = const [];

  /// The ticks of the boxes, read again after each tick here.
  late BoxTicking? _ticking = _watched(widget.ticking);
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
    final table = _table = MarkdownTables.at(_text, _at);
    _order = table == null ? const [] : tableRowOrder(table, _sort);
    _tags = Markdown.tags(_text);
    _links = Markdown.autolinks(_text);
  }

  /// [ticking] telling this screen of each tick too.
  BoxTicking? _watched(BoxTicking? ticking) => ticking == null
      ? null
      : BoxTicking(
          notes: ticking.notes,
          entity: ticking.entity,
          ticks: ticking.ticks,
          mayTick: ticking.mayTick,
          showTicks: ticking.showTicks,
          changed: () {
            ticking.changed?.call();
            if (!mounted) return;
            setState(() {
              _ticking = _ticking?.again();
              _cell?.boxes = _ticking;
              _read();
            });
          },
        );

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
            24 +
            (_sort?.column == c
                ? tableSortRoom(scaler, style.fontSize ?? 14)
                : 0),
    ];
  }

  /// Shows the rows sorted as [sort] says, in the view only.
  void _sortBy(TableSort? sort) => setState(() {
    _sort = sort;
    _read();
    _measure();
  });

  /// Writes [change] when it changed the text, and shows the table it
  /// gives; [gone] when the table went with it, which closes the screen.
  void _apply(MarkdownChange change, {bool gone = false}) {
    if (change.text == _text) return;
    // As kept: its new boxes have their ids, the table moved by them.
    final kept = widget.edit?.write(change) ?? change.text;
    _text = kept;
    _at = kept == change.text
        ? change.start
        : idFreeOffset(kept, idFreeCount(change.text, change.start));
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
    final controller =
        TagTextController(
            text: MarkdownTables.cell(_text, _at, cell.row, cell.column) ?? '',
            hideMarks: widget.edit?.twoRows ?? false,
          )
          ..markdown = true
          ..boxes = _ticking;
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
              if (_sort != (column: index, descending: false))
                item(_Op.sortUp, Icons.arrow_upward, t.tableShowSortedUp),
              if (_sort != (column: index, descending: true))
                item(_Op.sortDown, Icons.arrow_downward, t.tableShowSortedDown),
              if (_sort != null)
                item(
                  _Op.asWritten,
                  Icons.format_list_numbered,
                  t.tableShowAsWritten,
                ),
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
        _reshape(MarkdownTables.insertRow(text, where, index));
      case (_Op.after, false):
        _reshape(MarkdownTables.insertRow(text, where, index + 1));
      case (_Op.back, false):
        _reshape(MarkdownTables.moveRow(text, where, index, index - 1));
      case (_Op.forward, false):
        _reshape(MarkdownTables.moveRow(text, where, index, index + 1));
      case (_Op.before, true):
        _reshape(MarkdownTables.insertColumn(text, where, index));
      case (_Op.after, true):
        _reshape(MarkdownTables.insertColumn(text, where, index + 1));
      case (_Op.back, true):
        _reshape(MarkdownTables.moveColumn(text, where, index, index - 1));
      case (_Op.forward, true):
        _reshape(MarkdownTables.moveColumn(text, where, index, index + 1));
      case (_Op.headings, _):
        _apply(MarkdownTables.toggleHeadingColumn(text, where));
      case (_Op.sortUp, _):
        _sortBy((column: index, descending: false));
      case (_Op.sortDown, _):
        _sortBy((column: index, descending: true));
      case (_Op.asWritten, _):
        _sortBy(null);
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
    _reshape(
      column
          ? MarkdownTables.deleteColumn(_text, _at, index)
          : MarkdownTables.deleteRow(_text, _at, index),
      gone: gone,
    );
  }

  /// Makes [change], which puts in, moves or deletes a row or column:
  /// the rows show as written again, the sort no longer fitting them.
  void _reshape(MarkdownChange change, {bool gone = false}) {
    if (change.text != _text) _sort = null;
    _apply(change, gone: gone);
  }

  /// Puts the table on the clipboard as an HTML table and, as the plain
  /// text, as Markdown (#324), each box in the state it shows; the cell
  /// edited is written first. Where the platform takes no HTML, the
  /// Markdown alone. Copied says nothing (UI laws).
  Future<void> _copy() async {
    _commit();
    await copyTable(context, text: _text, at: _at, ticking: _ticking);
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
        appBar: pageBar(
          context,
          title: t.tableTitle,
          actions: [
            if (table != null)
              IconButton(
                key: const ValueKey('table-copy'),
                tooltip: t.tableCopy,
                onPressed: _copy,
                icon: const Icon(Icons.copy),
              ),
          ],
        ),
        body: table == null
            ? const SizedBox.shrink()
            : LayoutBuilder(
                builder: (context, constraints) =>
                    _grid(context, table, constraints.maxWidth),
              ),
      ),
    );
  }

  /// The table's ⋮ at [at] (global): Copy.
  Future<void> _tableMenu(Offset at) async {
    final picked = await showTableMenu(context, at);
    if (!mounted || picked != TableAction.copy) return;
    await _copy();
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
      // A drag on the cells moves the table under the finger; the bar
      // keeps its own.
      child: GestureDetector(
        onHorizontalDragUpdate: _most > 0
            ? (d) =>
                  _offset.value = (_offset.value - d.delta.dx).clamp(0, _most)
            : null,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            // The ⋮ at the top right corner, offering Copy (obf-1050),
            // beside the bar that moves the table sideways.
            Row(
              children: [
                Expanded(
                  child: _most > 0
                      ? TableSidewaysBar(
                          key: const ValueKey('table-sideways'),
                          offset: _offset,
                          most: _most,
                          room: _room,
                          whole: moving,
                          label: t.tableSideways,
                        )
                      : const SizedBox.shrink(),
                ),
                TableMenuButton(
                  key: const ValueKey('table-screen-menu'),
                  onMenu: (at) => unawaited(_tableMenu(at)),
                ),
              ],
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
                    // In the order shown, each by its index in the table.
                    for (final r in _order.skip(1)) _row(context, table, r),
                    if (_edits) _row(context, table, table.rows.length),
                  ],
                ),
              ),
            ),
          ],
        ),
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
            child: TableShifted(
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
            child: TableShifted(
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
        boxes: true,
        formats: true,
        lineFormats: false,
        style: style,
        decoration: const InputDecoration.collapsed(hintText: null),
      );
    } else if (adds) {
      content = const SizedBox.shrink();
    } else {
      final words = MarkedCell(
        _text,
        table.rows[r].cells[c],
        boxes: _boxes,
        tags: _tags,
        links: _links,
        style: style,
        textAlign: cellAlign(table.aligns[c]),
        ticking: _ticking,
      );
      content = switch (_sort) {
        (column: final by, :final descending) when r == 0 && by == c =>
          TableSortedHeading(
            words: words,
            descending: descending,
            size: style.fontSize ?? 14,
          ),
        _ => words,
      };
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
    // Where it is only read, a heading's tap sorts the rows by its
    // column (obf-1052).
    if (edit == null && r == 0 && !adds) {
      return TableSortTap(
        key: ValueKey('cell-$r-$c'),
        onSort: () => _sortBy(nextTableSort(_sort, c)),
        descending: _sort?.column == c ? _sort?.descending : null,
        child: box,
      );
    }
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
/// has; full screen and where a table is read in a text (obf-1048).
class TableShifted extends StatelessWidget {
  final ValueListenable<double> offset;
  final double most;
  final Widget child;

  const TableShifted({
    super.key,
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
/// left and right. Full screen and where a table is read in a text
/// (obf-1048).
class TableSidewaysBar extends StatelessWidget {
  final ValueNotifier<double> offset;
  final double most;
  final double room;
  final double whole;
  final String label;

  const TableSidewaysBar({
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
