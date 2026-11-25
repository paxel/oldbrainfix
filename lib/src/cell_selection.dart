import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'photo_platform.dart';
import 'selection_toolbar.dart';
import 'text_marks_view.dart';

/// A rectangle of the cells of a table as they show (obf-1054): its rows
/// from [top] to [bottom] in the order shown, the header row 0, and its
/// columns from [left] to [right].
typedef CellRect = ({int top, int left, int bottom, int right});

/// The cells of the rows [rows], by their index in the table and in that
/// order, and of the columns [left] to [right] of the table at [at] of
/// [text], as stored, as a table of their own (obf-1054): each cell as
/// written, its column lining up as it does, the first row its header
/// row, its first column headings while the table's first column is one
/// of them and holds headings. Null where there is no such table or no
/// cell.
String? tableCells(String text, int at, List<int> rows, int left, int right) {
  final table = MarkdownTables.at(text, at);
  if (table == null) return null;
  final picked = [
    for (final r in rows)
      if (r >= 0 && r < table.rows.length) r,
  ];
  final from = math.max(0, left);
  final to = math.min(table.columns - 1, right);
  if (picked.isEmpty || from > to) return null;
  String line(Iterable<String> cells) => '| ${cells.join(' | ')} |';
  String row(int r) => line([
    for (var c = from; c <= to; c++)
      text.substring(table.rows[r].cells[c].start, table.rows[r].cells[c].end),
  ]);
  return [
    if (table.headingColumn && from == 0) MarkdownTables.headingColumnLine,
    row(picked.first),
    line([
      for (var c = from; c <= to; c++)
        switch (table.aligns[c]) {
          MarkdownAlign.none => '---',
          MarkdownAlign.left => ':--',
          MarkdownAlign.center => ':-:',
          MarkdownAlign.right => '--:',
        },
    ]),
    for (final r in picked.skip(1)) row(r),
  ].join('\n');
}

/// Puts the cells of [rows] and the columns [left] to [right] of the
/// table at [at] of [text], as stored, on the clipboard (obf-1054): as
/// Copy puts a whole table there (#324), an HTML table and, as the plain
/// text, Markdown, each box in the state [ticking] shows. Copied says
/// nothing (UI laws).
Future<void> copyCells(
  BuildContext context, {
  required String text,
  required int at,
  required List<int> rows,
  required int left,
  required int right,
  BoxTicking? ticking,
}) async {
  final cells = tableCells(text, at, rows, left, right);
  if (cells == null) return;
  final copy = MarkdownTables.copy(cells, 0, {
    for (final b in storedBoxes(text, ticking)) b.id: b.state,
  });
  if (copy == null) return;
  if (await PhotoScope.of(context).copyHtml(copy.markdown, copy.html)) {
    return;
  }
  await Clipboard.setData(ClipboardData(text: copy.markdown));
}

/// Where a table's cells select as a rectangle (obf-1054): a long press
/// on a cell ([SelectableCell]) selects it, a drag on without lifting the
/// finger selects every cell from it to the one under the finger, and on
/// lifting it the toolbar above the selection offers Copy ([onCopy]) and,
/// for one cell, Search with its words. A tap elsewhere, a scroll of the
/// page or of the cells or Copy ends it; [CellSelectionState.clear] does
/// too, which the table's own taps and its moving sideways call first.
class CellSelection extends StatefulWidget {
  /// Puts [cells] on the clipboard.
  final Future<void> Function(CellRect cells) onCopy;

  /// The words [cells] show, '' for none: nothing to copy.
  final String Function(CellRect cells) words;
  final Widget child;

  const CellSelection({
    super.key,
    required this.onCopy,
    required this.words,
    required this.child,
  });

  @override
  State<CellSelection> createState() => CellSelectionState();
}

class CellSelectionState extends State<CellSelection> {
  final _menu = ContextMenuController();
  final _selected = ValueNotifier<CellRect?>(null);

  /// The cell the long press started on and the one under the finger,
  /// each with where it lies on the screen.
  ({int row, int column})? _from;
  ({int row, int column})? _to;
  Rect _fromRect = Rect.zero;
  Rect _toRect = Rect.zero;
  List<ScrollPosition> _scrolls = const [];

  /// The cells selected, or null.
  CellRect? get selected => _selected.value;

  @override
  void dispose() {
    _menu.remove();
    _watch(const []);
    _selected.dispose();
    super.dispose();
  }

  /// Ends the selection; whether there was one.
  bool clear() {
    if (_selected.value == null) return false;
    _menu.remove();
    _watch(const []);
    _from = null;
    _to = null;
    _selected.value = null;
    return true;
  }

  void _start(({int row, int column}) cell, RenderObject? box) {
    _menu.remove();
    _watch(const []);
    unawaited(Feedback.forLongPress(context));
    _from = cell;
    _to = cell;
    _fromRect = _toRect = _rectOf(box);
    _select();
  }

  void _extend(Offset global) {
    final box = context.findRenderObject();
    if (_from == null || box is! RenderBox || !box.hasSize) return;
    final result = BoxHitTestResult();
    box.hitTest(result, position: box.globalToLocal(global));
    for (final entry in result.path) {
      final target = entry.target;
      if (target is RenderMetaData && target.metaData is _CellMark) {
        final cell = (target.metaData as _CellMark).cell;
        if (cell == _to) return;
        _to = cell;
        _toRect = _rectOf(target);
        _select();
        return;
      }
    }
  }

  void _select() {
    final (from, to) = (_from, _to);
    if (from == null || to == null) return;
    _selected.value = (
      top: math.min(from.row, to.row),
      left: math.min(from.column, to.column),
      bottom: math.max(from.row, to.row),
      right: math.max(from.column, to.column),
    );
  }

  /// The toolbar above the cells selected, below them where there is no
  /// room.
  void _show() {
    final cells = _selected.value;
    if (cells == null) return;
    final words = widget.words(cells);
    final one = cells.top == cells.bottom && cells.left == cells.right;
    final onSearch = one ? SelectionSearchScope.of(context) : null;
    final around = _fromRect.expandToInclude(_toRect);
    _menu.show(
      context: context,
      contextMenuBuilder: (context) => TapRegion(
        groupId: this,
        child: selectionToolbar(
          context,
          anchors: TextSelectionToolbarAnchors(
            primaryAnchor: around.topCenter,
            secondaryAnchor: around.bottomCenter,
          ),
          system: [
            ContextMenuButtonItem(
              type: ContextMenuButtonType.copy,
              onPressed: _copy,
            ),
          ],
          words: words,
          hide: clear,
          onSearch: onSearch,
        ),
      ),
    );
    _watch([
      for (
        var scrollable = Scrollable.maybeOf(context);
        scrollable != null;
        scrollable = Scrollable.maybeOf(scrollable.context)
      )
        scrollable.position,
    ]);
  }

  void _copy() {
    final cells = _selected.value;
    clear();
    if (cells != null) unawaited(widget.onCopy(cells));
  }

  /// Ends the selection when one of [positions], the page's around the
  /// table, scrolls: the toolbar would stay behind.
  void _watch(List<ScrollPosition> positions) {
    for (final position in _scrolls) {
      position.removeListener(_scrolled);
    }
    _scrolls = positions;
    for (final position in positions) {
      position.addListener(_scrolled);
    }
  }

  void _scrolled() => clear();

  static Rect _rectOf(RenderObject? box) {
    if (box is! RenderBox || !box.hasSize) return Rect.zero;
    return box.localToGlobal(Offset.zero) & box.size;
  }

  @override
  Widget build(BuildContext context) => _CellScope(
    state: this,
    child: TapRegion(
      groupId: this,
      onTapOutside: (_) => clear(),
      // The cells scrolling under it, as full screen, end it too.
      child: NotificationListener<ScrollUpdateNotification>(
        onNotification: (_) {
          clear();
          return false;
        },
        child: widget.child,
      ),
    ),
  );
}

class _CellScope extends InheritedWidget {
  final CellSelectionState state;

  const _CellScope({required this.state, required super.child});

  @override
  bool updateShouldNotify(_CellScope oldWidget) => state != oldWidget.state;
}

/// What marks a cell for the hit test of a drag.
class _CellMark {
  final ({int row, int column}) cell;

  const _CellMark(this.cell);
}

/// The cell [cell] of a table, its row as shown, under a [CellSelection]:
/// tinted while selected, and a long press on it starts selecting unless
/// [presses] is false, as on a cell being edited, whose field takes its
/// own. Without a [CellSelection] above it, [child] alone.
class SelectableCell extends StatelessWidget {
  final ({int row, int column}) cell;
  final bool presses;
  final Widget child;

  const SelectableCell({
    super.key,
    required this.cell,
    this.presses = true,
    required this.child,
  });

  @override
  Widget build(BuildContext context) {
    final state = context.getInheritedWidgetOfExactType<_CellScope>()?.state;
    if (state == null) return child;
    final tint =
        DefaultSelectionStyle.of(context).selectionColor ??
        Theme.of(context).colorScheme.primary.withValues(alpha: 0.4);
    return MetaData(
      metaData: _CellMark(cell),
      behavior: HitTestBehavior.translucent,
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onLongPressStart: presses
            ? (_) => state._start(cell, context.findRenderObject())
            : null,
        onLongPressMoveUpdate: presses
            ? (d) => state._extend(d.globalPosition)
            : null,
        onLongPressEnd: presses ? (_) => state._show() : null,
        child: ValueListenableBuilder<CellRect?>(
          valueListenable: state._selected,
          child: child,
          builder: (context, cells, child) => DecoratedBox(
            position: DecorationPosition.foreground,
            decoration: BoxDecoration(
              color: cells != null && _holds(cells, cell) ? tint : null,
            ),
            child: child,
          ),
        ),
      ),
    );
  }

  static bool _holds(CellRect cells, ({int row, int column}) cell) =>
      cell.row >= cells.top &&
      cell.row <= cells.bottom &&
      cell.column >= cells.left &&
      cell.column <= cells.right;
}
