import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'box_ids.dart';
import 'l10n.dart';
import 'table_screen.dart';
import 'text_marks_view.dart';

/// The offset of the grapheme boundary after [offset] in [text]; the end
/// when there is none. An emoji or a letter with its accents is one step.
int graphemeAfter(String text, int offset) {
  var at = 0;
  for (final grapheme in text.characters) {
    at += grapheme.length;
    if (at > offset) return at;
  }
  return text.length;
}

/// The offset of the grapheme boundary before [offset] in [text]; zero
/// when there is none.
int graphemeBefore(String text, int offset) {
  var before = 0;
  var at = 0;
  for (final grapheme in text.characters) {
    if (at >= offset) break;
    before = at;
    at += grapheme.length;
  }
  return math.min(before, offset);
}

/// The line of [text] that [offset] is on, counted from 0; `\r\n` is
/// one break.
int _lineAt(String text, int offset) {
  final stop = offset.clamp(0, text.length);
  var line = 0;
  for (var i = 0; i < stop; i++) {
    final c = text.codeUnitAt(i);
    if (c == 0x0D && i + 1 < text.length && text.codeUnitAt(i + 1) == 0x0A) {
      if (i + 1 < stop) line++;
      i++;
    } else if (c == 0x0A || c == 0x0D) {
      line++;
    }
  }
  return line;
}

final _wordCharacter = RegExp(r'^[\p{L}\p{N}\p{M}]', unicode: true);

/// The word boundary in [text] nearest to [offset]: the start or end of a
/// word, the start or end of the text. A tie goes to the earlier one.
int wordEdgeNear(String text, int offset) {
  if (offset <= 0 || offset >= text.length) return offset.clamp(0, text.length);
  var before = 0;
  var at = 0;
  bool? previousWord;
  for (final grapheme in text.characters) {
    final word = _wordCharacter.hasMatch(grapheme);
    if (previousWord != null && word != previousWord) {
      if (at == offset) return offset;
      if (at > offset) {
        return offset - before <= at - offset ? before : at;
      }
      before = at;
    }
    previousWord = word;
    at += grapheme.length;
  }
  return offset - before <= text.length - offset ? before : text.length;
}

/// The text field of the Note editor and of Amend on the phone: the whole
/// text on top, as a system text field without magnifier and without
/// dragging the cursor, whose tap puts the cursor at the nearest word
/// boundary, and, while the keyboard is open, the cursor's line directly
/// above it, as a text field shows it, with ◀ and ▶ stepping the cursor one
/// character at a time (UI laws, "Text editor"). [twoRows] false gives the
/// plain system text field.
///
/// Both rows share [controller]; the top row stays the one with focus, so
/// the keyboard's suggestions, autocorrect, swipe typing and voice input
/// work as in any text field.
///
/// With [boxes], in either form, the bar above the keyboard has a button
/// that puts ☐ in at the cursor, also in the middle of a word; a
/// [readOnly] field has none. A tap on a box of the text (ADR 0035), one
/// with its id, goes to [onBox] with where the box starts and where the
/// finger is, also in a [readOnly] field; the cursor stays where it was.
/// The ids of the boxes are never typed into, nor taken in part
/// ([BoxIdsFormatter]).
///
/// With [formats], for a [TagTextController], the same bar formats the
/// selection or the word at the cursor, or the lines they touch (ADR
/// 0033), each button telling whether the cursor is in its format. The
/// first format makes a plain text formatted for good, and so do a
/// format typed as Markdown (`Markdown.shortcutAt`) and a pasted text
/// that uses one; [onFormatted] runs then. In two rows, the bottom row
/// of a formatted text shows the source of the cursor's line.
///
/// With [lineFormats], the bar also has the formats of whole lines and
/// the table button, which puts a new table at the cursor or opens the
/// table the cursor is in full screen ([openTable]); a tap on a table of
/// a formatted text opens it too. A table's cell is edited without them.
class TwoRowTextField extends StatefulWidget {
  /// The height of the bottom row.
  static const rowHeight = 56.0;

  /// The height of the bar of formats above it.
  static const barHeight = 48.0;

  /// How long ◀ or ▶ is held before it repeats.
  static const holdDelay = Duration(milliseconds: 400);

  /// The pause between the first repeats; each next one is shorter, down
  /// to [fastestRepeat].
  static const firstRepeat = Duration(milliseconds: 160);
  static const fastestRepeat = Duration(milliseconds: 30);

  final TextEditingController controller;
  final bool twoRows;
  final bool readOnly;
  final bool autofocus;
  final TextStyle? style;
  final InputDecoration decoration;
  final bool boxes;
  final void Function(int start, Offset at)? onBox;
  final bool formats;
  final bool lineFormats;
  final VoidCallback? onFormatted;

  const TwoRowTextField({
    super.key,
    required this.controller,
    required this.twoRows,
    this.readOnly = false,
    this.autofocus = false,
    this.style,
    this.decoration = const InputDecoration(),
    this.boxes = false,
    this.onBox,
    this.formats = false,
    this.lineFormats = true,
    this.onFormatted,
  });

  /// The height of what rides on the keyboard above a field that types
  /// in [twoRows], with the bar of formats or ☐ when [bar].
  static double above({required bool twoRows, required bool bar}) =>
      (twoRows ? rowHeight : 0) + (bar ? barHeight : 0);

  @override
  State<TwoRowTextField> createState() => _TwoRowTextFieldState();
}

class _TwoRowTextFieldState extends State<TwoRowTextField>
    with WidgetsBindingObserver
    implements TextSelectionGestureDetectorBuilderDelegate {
  @override
  GlobalKey<EditableTextState> get editableTextKey => _editableKey;

  @override
  bool get forcePressEnabled => false;

  @override
  bool get selectionEnabled => true;

  final _focus = FocusNode();
  final _field = GlobalKey();
  final _row = OverlayPortalController();
  late final _handles = _NoCaretHandle(widget.controller);
  late final _gestures = _TopRowGestures(this);
  late final _editableKey = _FoundEditable(_editable);

  /// How high the keyboard covers the screen now, in logical pixels.
  double _keyboard = 0;

  /// Whether the bottom row selects: the anchor stays where it was when
  /// the long press started, ◀ ▶ move only the moving end.
  bool _selecting = false;
  int _anchor = 0;
  String _selectingText = '';

  Timer? _repeat;
  Duration _repeatPause = TwoRowTextField.firstRepeat;

  /// Where the finger last went down on the plain field, for its tap.
  Offset? _down;

  /// The text before its last change, and now, to tell what was typed
  /// or pasted.
  String _before = '';
  late String _now = widget.controller.text;

  /// Whether the text was formatted when last drawn.
  bool _markdown = false;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    _focus.addListener(_focusChanged);
    widget.controller.addListener(_changed);
    _markdown = _formatting?.markdown ?? false;
    _row.show();
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _keyboard = _readKeyboard();
  }

  @override
  void didUpdateWidget(TwoRowTextField oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.controller != widget.controller) {
      oldWidget.controller.removeListener(_changed);
      widget.controller.addListener(_changed);
      _before = '';
      _now = widget.controller.text;
    }
    _markdown = _formatting?.markdown ?? false;
  }

  @override
  void dispose() {
    _repeat?.cancel();
    WidgetsBinding.instance.removeObserver(this);
    widget.controller.removeListener(_changed);
    _focus.dispose();
    super.dispose();
  }

  /// The keyboard's height; the Scaffold hides it from the field's own
  /// media query.
  double _readKeyboard() {
    final view = View.of(context);
    return view.viewInsets.bottom / view.devicePixelRatio;
  }

  @override
  void didChangeMetrics() {
    if (!mounted) return;
    final keyboard = _readKeyboard();
    if (keyboard == _keyboard) return;
    if (keyboard <= 0) _stopRepeat();
    setState(() => _keyboard = keyboard);
  }

  void _focusChanged() {
    if (!_focus.hasFocus) {
      _stopRepeat();
      _selecting = false;
    }
    setState(() {});
  }

  /// Typing ends selecting: the typed text replaced the selection.
  void _changed() {
    final text = widget.controller.text;
    if (text != _now) {
      _before = _now;
      _now = text;
    }
    if (_selecting && text != _selectingText) {
      setState(() => _selecting = false);
    }
    final markdown = _formatting?.markdown ?? false;
    if (markdown != _markdown) setState(() => _markdown = markdown);
  }

  bool get _keyboardOpen =>
      !widget.readOnly && _focus.hasFocus && _keyboard > 0;

  bool get _rowShown => widget.twoRows && _keyboardOpen;

  /// Whether the field has the bar of formats or ☐.
  bool get _hasBar => _formatting != null || _inserts;

  bool get _barShown => _hasBar && _keyboardOpen;

  /// The controller, when this field formats its text.
  TagTextController? get _formatting => switch (widget.controller) {
    final TagTextController controller
        when widget.formats && !widget.readOnly =>
      controller,
    _ => null,
  };

  /// A change the user made, typed or pasted: a plain text becomes
  /// formatted with a format typed as Markdown or pasted (ADR 0033).
  void _userChanged(String text) {
    final controller = _formatting;
    if (controller == null || controller.markdown) return;
    final before = _before;
    final most = math.min(before.length, text.length);
    var head = 0;
    while (head < most && before.codeUnitAt(head) == text.codeUnitAt(head)) {
      head++;
    }
    var tail = 0;
    while (tail < most - head &&
        before.codeUnitAt(before.length - 1 - tail) ==
            text.codeUnitAt(text.length - 1 - tail)) {
      tail++;
    }
    final inserted = text.substring(head, text.length - tail);
    if (inserted.isEmpty) return;
    final selection = controller.selection;
    final cursor = selection.isValid ? selection.extentOffset : text.length;
    if (Markdown.shortcutAt(text, cursor) ||
        (inserted.length > 1 && Markdown.uses(inserted))) {
      _becomeFormatted(controller);
    }
  }

  void _becomeFormatted(TagTextController controller) {
    controller.markdown = true;
    widget.onFormatted?.call();
  }

  /// Applies [edit] of the bar to the selection, or to the cursor, and
  /// selects what it gives; a change makes the text formatted.
  void _format(
    MarkdownChange Function(String text, int start, int end) edit, {
    TextSelection? at,
  }) {
    final controller = _formatting;
    if (controller == null) return;
    final value = controller.value;
    final text = value.text;
    final selection = at ?? value.selection;
    final start = selection.isValid ? selection.start : text.length;
    final end = selection.isValid ? selection.end : text.length;
    final change = edit(
      text,
      start.clamp(0, text.length),
      end.clamp(0, text.length),
    );
    if (change.text == text && change.start == start && change.end == end) {
      return;
    }
    if (_selecting) setState(() => _selecting = false);
    final next = TextEditingValue(
      text: change.text,
      selection: TextSelection(
        baseOffset: change.start,
        extentOffset: change.end,
      ),
    );
    final editable = _editable();
    if (editable == null) {
      controller.value = next;
    } else {
      editable.userUpdateTextEditingValue(next, SelectionChangedCause.toolbar);
    }
    if (change.text != text && !controller.markdown) {
      _becomeFormatted(controller);
    }
  }

  /// Asks for the address of a link on the selection or the word at the
  /// cursor, the one there already offered for change.
  Future<void> _link(String? current) async {
    final selection = widget.controller.selection;
    final address = await askLinkAddress(context, current);
    if (!mounted || address == null) return;
    _focus.requestFocus();
    _format(
      (text, start, end) => Markdown.link(text, start, end, address),
      at: selection,
    );
  }

  /// The bar's buttons not shown for want of room, in a menu at [at].
  Future<void> _more(Offset at, List<_Format> formats) async {
    final overlay =
        Overlay.of(context).context.findRenderObject() as RenderBox?;
    if (overlay == null) return;
    final picked = await showMenu<_Format>(
      context: context,
      position: RelativeRect.fromRect(
        overlay.globalToLocal(at) & Size.zero,
        Offset.zero & overlay.size,
      ),
      items: [
        for (final format in formats)
          PopupMenuItem(
            key: ValueKey('${format.key}-more'),
            value: format,
            child: Row(
              children: [
                SizedBox(width: 32, child: format.icon(context)),
                const SizedBox(width: 12),
                Flexible(child: Text(format.label)),
                if (format.active) ...[
                  const SizedBox(width: 12),
                  const Icon(Icons.check, size: 20),
                ],
              ],
            ),
          ),
      ],
    );
    if (!mounted || picked == null) return;
    _focus.requestFocus();
    picked.run();
  }

  /// The field's own editable text, which tells the keyboard of a moved
  /// cursor as a key press would.
  EditableTextState? _editable() {
    EditableTextState? found;
    void visit(Element element) {
      if (found != null) return;
      if (element is StatefulElement && element.state is EditableTextState) {
        found = element.state as EditableTextState;
        return;
      }
      element.visitChildren(visit);
    }

    _field.currentContext?.visitChildElements(visit);
    return found;
  }

  /// Moves the selection to [next]. A composing word the selection leaves
  /// is finished, as the controller does when its selection is set.
  void _select(TextSelection next) {
    final value = widget.controller.value;
    final composing = value.composing;
    final keeps =
        composing.isValid &&
        next.start >= composing.start &&
        next.end <= composing.end;
    final moved = value.copyWith(
      selection: next,
      composing: keeps ? composing : TextRange.empty,
    );
    final editable = _editable();
    if (editable == null) {
      widget.controller.value = moved;
      return;
    }
    editable.userUpdateTextEditingValue(moved, SelectionChangedCause.keyboard);
    if (_selecting && !next.isCollapsed) {
      editable.showToolbar();
    } else {
      editable.hideToolbar();
    }
  }

  /// One step of ◀ ([forward] false) or ▶; whether the cursor moved.
  bool _step({required bool forward}) {
    final value = widget.controller.value;
    final text = value.text;
    final selection = value.selection;
    if (!selection.isValid) return false;
    final TextSelection next;
    final int passed;
    if (_selecting) {
      final extent = selection.extentOffset;
      final to = forward
          ? graphemeAfter(text, extent)
          : graphemeBefore(text, extent);
      if (to == extent) return false;
      next = TextSelection(baseOffset: _anchor, extentOffset: to);
      passed = forward ? extent : to;
    } else if (!selection.isCollapsed) {
      next = TextSelection.collapsed(
        offset: forward ? selection.end : selection.start,
      );
      passed = -1;
    } else {
      final at = selection.baseOffset;
      final to = forward ? graphemeAfter(text, at) : graphemeBefore(text, at);
      if (to == at) return false;
      next = TextSelection.collapsed(offset: to);
      passed = forward ? at : to;
    }
    _select(next);
    if (passed >= 0) _announce(text, passed);
    return true;
  }

  /// Tells a screen reader the character the cursor passed.
  void _announce(String text, int at) {
    if (!MediaQuery.accessibleNavigationOf(context)) return;
    final t = context.t;
    final grapheme = text.substring(at, graphemeAfter(text, at));
    final spoken = switch (grapheme) {
      '\n' => t.cursorNewLine,
      ' ' || '\t' => t.cursorSpace,
      _ => grapheme,
    };
    unawaited(
      SemanticsService.sendAnnouncement(
        View.of(context),
        spoken,
        Directionality.of(context),
      ),
    );
  }

  void _press({required bool forward}) {
    _stopRepeat();
    _step(forward: forward);
    _repeatPause = TwoRowTextField.firstRepeat;
    _repeat = Timer(
      TwoRowTextField.holdDelay,
      () => _repeating(forward: forward),
    );
  }

  /// Each repeat comes a little sooner than the one before.
  void _repeating({required bool forward}) {
    if (!mounted || !_step(forward: forward)) return;
    final pause = _repeatPause;
    _repeatPause = Duration(
      microseconds: math.max(
        TwoRowTextField.fastestRepeat.inMicroseconds,
        (pause.inMicroseconds * 0.8).round(),
      ),
    );
    _repeat = Timer(pause, () => _repeating(forward: forward));
  }

  void _stopRepeat() {
    _repeat?.cancel();
    _repeat = null;
  }

  /// A long press on the bottom row starts selecting where the cursor is,
  /// or ends it with the cursor at the moving end; it never moves the
  /// cursor by itself.
  void _toggleSelecting() {
    final value = widget.controller.value;
    if (!value.selection.isValid) return;
    if (_selecting) {
      setState(() => _selecting = false);
      _select(TextSelection.collapsed(offset: value.selection.extentOffset));
    } else {
      setState(() {
        _selecting = true;
        _anchor = value.selection.baseOffset;
        _selectingText = value.text;
      });
    }
  }

  /// A tap in the top row: the cursor went where the finger was; it goes
  /// on to the nearest word boundary.
  void _tapped() {
    if (_selecting) setState(() => _selecting = false);
    final value = widget.controller.value;
    final selection = value.selection;
    if (!selection.isValid || !selection.isCollapsed) return;
    final edge = wordEdgeNear(value.text, selection.baseOffset);
    if (edge != selection.baseOffset) {
      _select(TextSelection.collapsed(offset: edge));
    }
  }

  bool get _inserts => widget.boxes && !widget.readOnly;

  String? _blocksText;
  List<MarkdownBlock> _blocksOf = const [];

  /// The Markdown blocks of [text], parsed once for each text: a move of
  /// the cursor alone reads the ones it had.
  List<MarkdownBlock> _blocks(String text) {
    if (text != _blocksText) {
      _blocksOf = Markdown.parse(text);
      _blocksText = text;
    }
    return _blocksOf;
  }

  /// Tells [TwoRowTextField.onBox] of the box at [at] (global), if
  /// there is one there; whether there was.
  bool _tappedBox(Offset at) {
    final onBox = widget.onBox;
    if (onBox == null) return false;
    final editable = _editable()?.renderEditable;
    if (editable == null) return false;
    final local = editable.globalToLocal(at);
    for (final box in TextBoxes.find(widget.controller.text)) {
      final rects = editable.getBoxesForSelection(
        TextSelection(baseOffset: box.start, extentOffset: box.end),
      );
      if (rects.any((rect) => rect.toRect().inflate(4).contains(local))) {
        onBox(box.start, at);
        return true;
      }
    }
    return false;
  }

  /// Opens the table of a formatted text at [at] (global) full screen,
  /// if there is one there; whether there was.
  bool _tappedTable(Offset at) {
    final controller = _formatting;
    if (controller == null || !controller.markdown || !widget.lineFormats) {
      return false;
    }
    final editable = _editable()?.renderEditable;
    if (editable == null) return false;
    final local = editable.globalToLocal(at);
    for (final table in MarkdownTables.all(controller.text)) {
      final boxes = editable.getBoxesForSelection(
        TextSelection(baseOffset: table.start, extentOffset: table.end),
      );
      if (boxes.any((box) => box.toRect().contains(local))) {
        unawaited(_openTable(table.start));
        return true;
      }
    }
    return false;
  }

  /// The table button: the table the cursor is in opens full screen; else
  /// a new one goes in at the cursor and opens with its first cell being
  /// edited.
  void _table() {
    final controller = _formatting;
    if (controller == null) return;
    final value = controller.value;
    final text = value.text;
    final cursor = (value.selection.isValid ? value.selection.start : 0).clamp(
      0,
      text.length,
    );
    if (MarkdownTables.at(text, cursor) case final table?) {
      unawaited(_openTable(table.start));
      return;
    }
    final change = MarkdownTables.insert(text, cursor);
    _format((_, _, _) => change);
    // The first format keeps the text at once, its new boxes with their
    // ids (ADR 0035), which moves the table: it is found again as the
    // same one of the tables of the text now.
    final index = MarkdownTables.all(change.text)
        .indexWhere((table) => table.start == change.start);
    final now = MarkdownTables.all(controller.text);
    final at = index >= 0 && index < now.length
        ? now[index].start
        : change.start;
    unawaited(_openTable(at, cell: (row: 0, column: 0)));
  }

  /// Opens the table at [at] full screen, [cell] being edited; what it
  /// changes comes back into the field, the cursor after the table.
  Future<void> _openTable(int at, {({int row, int column})? cell}) async {
    final controller = _formatting;
    if (controller == null) return;
    if (_selecting) setState(() => _selecting = false);
    await openTable(
      context,
      text: controller.text,
      at: at,
      cell: cell,
      ticking: controller.boxes,
      edit: TableEditing(
        twoRows: widget.twoRows,
        write: (change) {
          final end =
              MarkdownTables.at(change.text, change.start)?.end ?? change.start;
          controller.value = TextEditingValue(
            text: change.text,
            selection: TextSelection.collapsed(offset: end),
          );
          return change.text;
        },
      ),
    );
  }

  /// Puts ☐ in at the cursor, also in the middle of a word, or before
  /// the selection; the cursor or the selection stays after it.
  void _insertBox() {
    final value = widget.controller.value;
    final text = value.text;
    final at = (value.selection.isValid ? value.selection.start : text.length)
        .clamp(0, text.length);
    final next = text.replaceRange(at, at, BoxState.open.glyph);
    _replace(value, next, at);
  }

  /// Writes [next] into the field as typing would, the selection's ends
  /// at or after [from] moved by what the text grew.
  void _replace(TextEditingValue value, String next, int from) {
    final grew = next.length - value.text.length;
    final selection = value.selection.isValid
        ? value.selection
        : TextSelection.collapsed(offset: value.text.length);
    int moved(int offset) =>
        (offset >= from ? offset + grew : offset).clamp(0, next.length);
    final changed = TextEditingValue(
      text: next,
      selection: selection.copyWith(
        baseOffset: moved(selection.baseOffset),
        extentOffset: moved(selection.extentOffset),
      ),
    );
    final editable = _editable();
    if (editable == null) {
      widget.controller.value = changed;
      return;
    }
    editable.userUpdateTextEditingValue(changed, SelectionChangedCause.toolbar);
  }

  /// The system's menu, with Delete added for a selection.
  Widget _menu(BuildContext context, EditableTextState editable) {
    final value = editable.textEditingValue;
    final selection = value.selection;
    return AdaptiveTextSelectionToolbar.buttonItems(
      anchors: editable.contextMenuAnchors,
      buttonItems: [
        ...editable.contextMenuButtonItems,
        if (!widget.readOnly && selection.isValid && !selection.isCollapsed)
          ContextMenuButtonItem(
            label: context.t.delete,
            onPressed: () {
              if (_selecting) setState(() => _selecting = false);
              editable.userUpdateTextEditingValue(
                value.replaced(selection, ''),
                SelectionChangedCause.toolbar,
              );
              editable.hideToolbar();
            },
          ),
      ],
    );
  }

  @override
  Widget build(BuildContext context) {
    // Hidden or small marks must not make a line lower than the text,
    // also where it is only read.
    final controller = widget.controller;
    final strut = controller is TagTextController && controller.markdown
        ? StrutStyle.fromTextStyle(
            widget.style ??
                Theme.of(context).textTheme.bodyLarge ??
                const TextStyle(),
          )
        : null;
    // What rides on the keyboard must not hide the line being typed.
    final scrollPadding = EdgeInsets.fromLTRB(
      20,
      20,
      20,
      20 + TwoRowTextField.above(twoRows: widget.twoRows, bar: _hasBar),
    );
    if (!widget.twoRows) {
      final field = TextField(
        key: _field,
        controller: widget.controller,
        focusNode: _focus,
        readOnly: widget.readOnly,
        autofocus: widget.autofocus,
        maxLines: null,
        textCapitalization: TextCapitalization.sentences,
        style: widget.style,
        strutStyle: strut,
        decoration: widget.decoration,
        onChanged: _userChanged,
        inputFormatters: const [BoxIdsFormatter()],
        scrollPadding: scrollPadding,
        onTap: widget.onBox != null || _formatting != null
            ? () {
                if (_down case final at?) {
                  if (!_tappedBox(at)) _tappedTable(at);
                }
              }
            : null,
      );
      final Widget tapped = widget.onBox == null && _formatting == null
          ? field
          // The field tells its tap, not where it was.
          : Listener(
              onPointerDown: (event) => _down = event.position,
              child: field,
            );
      if (!_hasBar) return tapped;
      return OverlayPortal(
        controller: _row,
        overlayChildBuilder: _aboveKeyboard,
        child: tapped,
      );
    }
    final field = TextField(
      key: _field,
      controller: widget.controller,
      focusNode: _focus,
      readOnly: widget.readOnly,
      autofocus: widget.autofocus,
      maxLines: null,
      textCapitalization: TextCapitalization.sentences,
      style: widget.style,
      strutStyle: strut,
      decoration: widget.decoration,
      onChanged: _userChanged,
      inputFormatters: const [BoxIdsFormatter()],
      magnifierConfiguration: TextMagnifierConfiguration.disabled,
      selectionControls: _handles,
      contextMenuBuilder: _menu,
      scrollPadding: scrollPadding,
    );
    // The field's own gestures would let a sideways drag move the cursor:
    // they are switched off, and these, which drag nothing, take their
    // place. Long-press selection, the handles of a selection and the
    // menu work as in the field.
    return OverlayPortal(
      controller: _row,
      overlayChildBuilder: _aboveKeyboard,
      child: TextFieldTapRegion(
        child: _gestures.buildGestureDetector(
          behavior: HitTestBehavior.translucent,
          child: IgnorePointer(child: field),
        ),
      ),
    );
  }

  /// The bar of formats and, in two rows, the bottom row, on the
  /// keyboard while it is open.
  Widget _aboveKeyboard(BuildContext context) {
    final bar = _barShown;
    final row = _rowShown;
    if (!bar && !row) return const SizedBox.shrink();
    return Positioned(
      left: 0,
      right: 0,
      bottom: _keyboard,
      // Part of the field: a tap here keeps the keyboard open.
      child: TextFieldTapRegion(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            if (bar)
              ListenableBuilder(
                listenable: widget.controller,
                builder: (context, _) => _formatBar(context),
              ),
            if (row) _bottomRow(context),
          ],
        ),
      ),
    );
  }

  /// The bar of formats: ☐ first, then each format; those that find no
  /// room go into ⋮ at its end.
  Widget _formatBar(BuildContext context) {
    final t = context.t;
    final controller = _formatting;
    final value = widget.controller.value;
    final at = _FormatsAt.of(
      value,
      markdown: controller?.markdown ?? false,
      blocks: _blocks,
    );
    void toggle(MarkdownSpanKind kind) =>
        _format((text, start, end) => Markdown.toggle(text, start, end, kind));
    void line(MarkdownBlockKind kind) => _format(
      (text, start, end) => Markdown.toggleLine(text, start, end, kind),
    );
    Widget glyph(String text) => Text(
      text,
      style: const TextStyle(fontSize: 18, fontWeight: FontWeight.bold),
    );
    final formats = [
      if (_inserts)
        _Format(
          key: 'two-row-insert-box',
          label: t.boxInsert,
          icon: (_) => const Icon(Icons.check_box_outline_blank),
          run: _insertBox,
        ),
      if (controller != null) ...[
        _Format(
          key: 'format-bold',
          label: t.formatBold,
          icon: (_) => const Icon(Icons.format_bold),
          toggles: true,
          active: at.spans.contains(MarkdownSpanKind.bold),
          run: () => toggle(MarkdownSpanKind.bold),
        ),
        _Format(
          key: 'format-italic',
          label: t.formatItalic,
          icon: (_) => const Icon(Icons.format_italic),
          toggles: true,
          active: at.spans.contains(MarkdownSpanKind.italic),
          run: () => toggle(MarkdownSpanKind.italic),
        ),
        _Format(
          key: 'format-strike',
          label: t.formatStrike,
          icon: (_) => const Icon(Icons.format_strikethrough),
          toggles: true,
          active: at.spans.contains(MarkdownSpanKind.strike),
          run: () => toggle(MarkdownSpanKind.strike),
        ),
        if (widget.lineFormats) ...[
          _Format(
            key: 'format-heading1',
            label: t.formatHeading1,
            icon: (_) => glyph('H1'),
            toggles: true,
            active: at.line == MarkdownBlockKind.heading1,
            run: () => line(MarkdownBlockKind.heading1),
          ),
          _Format(
            key: 'format-heading2',
            label: t.formatHeading2,
            icon: (_) => glyph('H2'),
            toggles: true,
            active: at.line == MarkdownBlockKind.heading2,
            run: () => line(MarkdownBlockKind.heading2),
          ),
          _Format(
            key: 'format-bullet',
            label: t.formatBullets,
            icon: (_) => const Icon(Icons.format_list_bulleted),
            toggles: true,
            active: at.line == MarkdownBlockKind.bullet,
            run: () => line(MarkdownBlockKind.bullet),
          ),
          _Format(
            key: 'format-numbered',
            label: t.formatNumbers,
            icon: (_) => const Icon(Icons.format_list_numbered),
            toggles: true,
            active: at.line == MarkdownBlockKind.numbered,
            run: () => line(MarkdownBlockKind.numbered),
          ),
          _Format(
            key: 'format-quote',
            label: t.formatQuote,
            icon: (_) => const Icon(Icons.format_quote),
            toggles: true,
            active: at.line == MarkdownBlockKind.quote,
            run: () => line(MarkdownBlockKind.quote),
          ),
        ],
        _Format(
          key: 'format-link',
          label: t.formatLink,
          icon: (_) => const Icon(Icons.link),
          toggles: true,
          active: at.url != null,
          run: () => unawaited(_link(at.url)),
        ),
        _Format(
          key: 'format-code',
          label: t.formatCode,
          icon: (_) => const Icon(Icons.code),
          toggles: true,
          active: at.spans.contains(MarkdownSpanKind.code),
          run: () => toggle(MarkdownSpanKind.code),
        ),
        if (widget.lineFormats) ...[
          _Format(
            key: 'format-indent',
            label: t.formatIndent,
            icon: (_) => const Icon(Icons.format_indent_increase),
            run: () => _format(
              (text, start, end) => Markdown.indent(text, start, end),
            ),
          ),
          _Format(
            key: 'format-outdent',
            label: t.formatOutdent,
            icon: (_) => const Icon(Icons.format_indent_decrease),
            run: () => _format(
              (text, start, end) =>
                  Markdown.indent(text, start, end, out: true),
            ),
          ),
          _Format(
            key: 'format-table',
            label: t.formatTable,
            icon: (_) => const Icon(Icons.table_chart_outlined),
            toggles: true,
            active: at.line == MarkdownBlockKind.table,
            run: _table,
          ),
        ],
      ],
    ];
    return Material(
      key: const ValueKey('format-bar'),
      elevation: 3,
      color: Theme.of(context).colorScheme.surfaceContainer,
      child: SizedBox(
        height: TwoRowTextField.barHeight,
        child: LayoutBuilder(
          builder: (context, constraints) {
            final room = (constraints.maxWidth / _FormatButton.width).floor();
            final fits = formats.length <= room;
            final shown = fits ? formats : formats.take(room - 1).toList();
            final rest = fits ? const <_Format>[] : formats.skip(room - 1);
            return Row(
              children: [
                for (final format in shown) _FormatButton(format: format),
                if (rest.isNotEmpty)
                  _MoreButton(
                    key: const ValueKey('format-more'),
                    label: t.moreFormats,
                    onTap: (at) => unawaited(_more(at, rest.toList())),
                  ),
              ],
            );
          },
        ),
      ),
    );
  }

  Widget _bottomRow(BuildContext context) {
    final t = context.t;
    return Material(
      key: const ValueKey('two-row-bar'),
      elevation: 3,
      color: Theme.of(context).colorScheme.surfaceContainerHigh,
      child: SizedBox(
        height: TwoRowTextField.rowHeight,
        child: Row(
          children: [
            _StepButton(
              key: const ValueKey('two-row-back'),
              icon: Icons.arrow_left,
              label: _selecting ? t.selectionBack : t.cursorBack,
              inverted: _selecting,
              onDown: () => _press(forward: false),
              onUp: _stopRepeat,
              onActivate: () => _step(forward: false),
            ),
            Expanded(
              child: ListenableBuilder(
                listenable: widget.controller,
                builder: (context, _) => _CursorLine(
                  value: widget.controller.value,
                  selecting: _selecting,
                  onLongPress: _toggleSelecting,
                  boxes: switch (widget.controller) {
                    final TagTextController controller => controller.boxes,
                    _ => null,
                  },
                ),
              ),
            ),
            _StepButton(
              key: const ValueKey('two-row-forward'),
              icon: Icons.arrow_right,
              label: _selecting ? t.selectionForward : t.cursorForward,
              inverted: _selecting,
              onDown: () => _press(forward: true),
              onUp: _stopRepeat,
              onActivate: () => _step(forward: true),
            ),
          ],
        ),
      ),
    );
  }
}

/// The top row's gestures: those of a text field, but a drag moves no
/// cursor, and a tap goes on to the nearest word boundary.
class _TopRowGestures extends TextSelectionGestureDetectorBuilder {
  final _TwoRowTextFieldState _field;

  _TopRowGestures(this._field) : super(delegate: _field);

  @override
  void onSingleTapUp(TapDragUpDetails details) {
    // A box opens its menu; the cursor stays where it was.
    if (_field._tappedBox(details.globalPosition)) return;
    // A table opens full screen; the cursor stays where it was.
    if (_field._tappedTable(details.globalPosition)) return;
    super.onSingleTapUp(details);
    _field._tapped();
  }

  @override
  void onDragSelectionStart(TapDragStartDetails details) {}

  @override
  void onDragSelectionUpdate(TapDragUpdateDetails details) {}

  @override
  void onDragSelectionEnd(TapDragEndDetails details) {}
}

/// The key of the editable text inside the top row's field, found rather
/// than given: the field keeps its own.
class _FoundEditable extends GlobalKey<EditableTextState> {
  final EditableTextState? Function() _find;

  const _FoundEditable(this._find) : super.constructor();

  @override
  EditableTextState? get currentState => _find();

  @override
  BuildContext? get currentContext => _find()?.context;

  @override
  Widget? get currentWidget => _find()?.widget;
}

/// Material handles without the one under a lone cursor, so the cursor
/// cannot be dragged; a selection's two handles stay.
class _NoCaretHandle extends MaterialTextSelectionControls
    with TextSelectionHandleControls {
  final TextEditingController controller;

  _NoCaretHandle(this.controller);

  @override
  Size getHandleSize(double textLineHeight) => controller.selection.isCollapsed
      ? Size.zero
      : super.getHandleSize(textLineHeight);

  @override
  Widget buildHandle(
    BuildContext context,
    TextSelectionHandleType type,
    double textHeight, [
    VoidCallback? onTap,
  ]) => type == TextSelectionHandleType.collapsed
      ? const SizedBox.shrink()
      : super.buildHandle(context, type, textHeight, onTap);
}

/// ◀ or ▶: a step on touch, repeating while held; a screen reader
/// activates it as a button. It never takes the focus.
class _StepButton extends StatelessWidget {
  final IconData icon;
  final String label;
  final bool inverted;
  final VoidCallback onDown;
  final VoidCallback onUp;
  final VoidCallback onActivate;

  const _StepButton({
    super.key,
    required this.icon,
    required this.label,
    required this.inverted,
    required this.onDown,
    required this.onUp,
    required this.onActivate,
  });

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Semantics(
      button: true,
      label: label,
      onTap: onActivate,
      excludeSemantics: true,
      child: Listener(
        behavior: HitTestBehavior.opaque,
        onPointerDown: (_) => onDown(),
        onPointerUp: (_) => onUp(),
        onPointerCancel: (_) => onUp(),
        child: Padding(
          padding: const EdgeInsets.all(4),
          child: DecoratedBox(
            decoration: BoxDecoration(
              color: inverted ? scheme.primary : null,
              borderRadius: BorderRadius.circular(8),
            ),
            child: SizedBox(
              width: 56,
              height: double.infinity,
              child: Icon(
                icon,
                size: 36,
                color: inverted ? scheme.onPrimary : scheme.primary,
              ),
            ),
          ),
        ),
      ),
    );
  }
}

/// A button of the bar of formats: what it shows and says, whether the
/// cursor is in its format, and what it does.
class _Format {
  final String key;
  final String label;
  final Widget Function(BuildContext context) icon;

  /// Whether it is a format the cursor can be in, [active] or not.
  final bool toggles;
  final bool active;
  final VoidCallback run;

  const _Format({
    required this.key,
    required this.label,
    required this.icon,
    required this.run,
    this.toggles = false,
    this.active = false,
  });
}

/// The formats at the cursor or the selection's start of a formatted
/// text: its line's, the parts it is in, and the address of the link it
/// is in.
class _FormatsAt {
  final MarkdownBlockKind line;
  final Set<MarkdownSpanKind> spans;
  final String? url;

  const _FormatsAt(this.line, this.spans, this.url);

  static const none = _FormatsAt(MarkdownBlockKind.paragraph, {}, null);

  static _FormatsAt of(
    TextEditingValue value, {
    required bool markdown,
    required List<MarkdownBlock> Function(String text) blocks,
  }) {
    final text = value.text;
    if (!markdown) return none;
    final selection = value.selection;
    final start = (selection.isValid ? selection.start : text.length).clamp(
      0,
      text.length,
    );
    final end = (selection.isValid ? selection.end : text.length).clamp(
      start,
      text.length,
    );
    final parsed = blocks(text);
    final line = _lineAt(text, start);
    if (line >= parsed.length) return none;
    final block = parsed[line];
    final spans = <MarkdownSpanKind>{};
    String? url;
    void walk(List<MarkdownSpan> parts) {
      for (final span in parts) {
        if (span.kind == MarkdownSpanKind.text) continue;
        if (span.contentStart <= start && end <= span.contentEnd) {
          spans.add(span.kind);
          if (span.kind == MarkdownSpanKind.link) url = span.url;
          walk(span.children);
        }
      }
    }

    walk(block.spans);
    return _FormatsAt(block.kind, spans, url);
  }
}

/// A button of the bar of formats; it never takes the focus. A screen
/// reader reads its name and, for a format, whether the cursor is in it.
class _FormatButton extends StatelessWidget {
  static const width = 44.0;

  final _Format format;

  const _FormatButton({required this.format});

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final active = format.active;
    return Semantics(
      button: true,
      label: format.label,
      toggled: format.toggles ? active : null,
      onTap: format.run,
      excludeSemantics: true,
      child: GestureDetector(
        key: ValueKey(format.key),
        behavior: HitTestBehavior.opaque,
        onTap: format.run,
        child: Padding(
          padding: const EdgeInsets.all(4),
          child: DecoratedBox(
            decoration: BoxDecoration(
              color: active ? scheme.primaryContainer : null,
              borderRadius: BorderRadius.circular(8),
            ),
            child: SizedBox(
              width: width - 8,
              height: double.infinity,
              child: Center(
                child: IconTheme.merge(
                  data: IconThemeData(
                    size: 24,
                    color: active ? scheme.onPrimaryContainer : scheme.primary,
                  ),
                  child: DefaultTextStyle.merge(
                    style: TextStyle(
                      color: active
                          ? scheme.onPrimaryContainer
                          : scheme.primary,
                    ),
                    child: format.icon(context),
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

/// ⋮ at the end of the bar of formats, with the buttons that found no
/// room; it never takes the focus.
class _MoreButton extends StatelessWidget {
  final String label;
  final void Function(Offset at) onTap;

  const _MoreButton({super.key, required this.label, required this.onTap});

  @override
  Widget build(BuildContext context) => Semantics(
    button: true,
    label: label,
    onTap: () {
      // A screen reader has no finger: the menu opens at the button.
      final box = context.findRenderObject() as RenderBox?;
      if (box == null) return;
      onTap(box.localToGlobal(box.size.center(Offset.zero)));
    },
    excludeSemantics: true,
    child: GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTapUp: (details) => onTap(details.globalPosition),
      child: SizedBox(
        width: _FormatButton.width,
        height: double.infinity,
        child: Icon(
          Icons.more_vert,
          color: Theme.of(context).colorScheme.primary,
        ),
      ),
    ),
  );
}

/// Asks for the address of a link, [current] filled in when there is
/// one; an empty answer removes it. Gives null when the dialog was
/// cancelled. An address written without its kind is taken as one of the
/// web, or of e-mail when it holds an "@", so that it opens.
Future<String?> askLinkAddress(BuildContext context, String? current) async {
  final address = await showDialog<String>(
    context: context,
    builder: (_) => _LinkDialog(current: current),
  );
  if (address == null) return null;
  final trimmed = address.trim();
  if (trimmed.isEmpty ||
      RegExp(r'^[A-Za-z][A-Za-z0-9+.-]*:').hasMatch(trimmed)) {
    return trimmed;
  }
  return trimmed.contains('@') && !trimmed.contains('/')
      ? 'mailto:$trimmed'
      : 'https://$trimmed';
}

class _LinkDialog extends StatefulWidget {
  final String? current;

  const _LinkDialog({required this.current});

  @override
  State<_LinkDialog> createState() => _LinkDialogState();
}

class _LinkDialogState extends State<_LinkDialog> {
  late final _address = TextEditingController(text: widget.current ?? '');

  @override
  void dispose() {
    _address.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return AlertDialog(
      title: Text(t.formatLink),
      content: TextField(
        key: const ValueKey('link-address'),
        controller: _address,
        autofocus: true,
        keyboardType: TextInputType.url,
        autocorrect: false,
        decoration: InputDecoration(
          labelText: t.linkAddress,
          helperText: widget.current == null ? null : t.linkRemoveHint,
          helperMaxLines: 2,
        ),
        onSubmitted: (text) => Navigator.of(context).pop(text),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(t.cancel),
        ),
        TextButton(
          key: const ValueKey('link-done'),
          onPressed: () => Navigator.of(context).pop(_address.text),
          child: Text(t.linkSet),
        ),
      ],
    );
  }
}

/// The cursor's line as a text field shows it: from its start, the
/// cursor right after the text before it, a selection highlighted; a long
/// press starts or ends selecting. Of a formatted text it is the source.
class _CursorLine extends StatelessWidget {
  final TextEditingValue value;
  final bool selecting;
  final VoidCallback onLongPress;

  /// The ticks of the text's boxes, each drawn in its state.
  final BoxTicking? boxes;

  const _CursorLine({
    required this.value,
    required this.selecting,
    required this.onLongPress,
    this.boxes,
  });

  /// [text] from [from] to [to], the selection highlighted.
  static List<TextSpan> _spans(
    String text,
    int from,
    int to,
    TextSelection selection,
    TextStyle marked,
  ) {
    String line(int a, int b) => text.substring(a, b).replaceAll('\t', ' ');
    final start = selection.start.clamp(from, to);
    final end = selection.end.clamp(from, to);
    return [
      if (start > from) TextSpan(text: line(from, start)),
      if (end > start) TextSpan(text: line(start, end), style: marked),
      if (to > end) TextSpan(text: line(end, to)),
    ];
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    final boxes = this.boxes;
    // Each glyph is one code unit: the offsets stay.
    final text = boxes == null ? value.text : tickedGlyphs(value.text, boxes);
    final selection = value.selection.isValid
        ? value.selection
        : TextSelection.collapsed(offset: text.length);
    final cursor = math.min(selection.extentOffset, text.length);
    bool breaks(int i) => text[i] == '\n' || text[i] == '\r';
    var from = cursor;
    while (from > 0 && !breaks(from - 1)) {
      from--;
    }
    var to = cursor;
    while (to < text.length && !breaks(to)) {
      to++;
    }
    final style = theme.textTheme.titleMedium;
    final marked = TextStyle(
      backgroundColor: scheme.primaryContainer,
      color: scheme.onPrimaryContainer,
    );
    final before = _spans(text, from, cursor, selection, marked);
    final after = _spans(text, cursor, to, selection, marked);
    String plain(List<TextSpan> spans) => spans.map((s) => s.text).join();
    return Semantics(
      container: true,
      label: t.cursorLine,
      value: '${plain(before)} | ${plain(after)}',
      onLongPress: onLongPress,
      onLongPressHint: selecting
          ? t.cursorLineStopSelecting
          : t.cursorLineSelect,
      excludeSemantics: true,
      child: GestureDetector(
        key: const ValueKey('two-row-line'),
        behavior: HitTestBehavior.opaque,
        onLongPress: onLongPress,
        child: _CursorStrip(
          children: [
            Text.rich(
              TextSpan(children: before),
              key: const ValueKey('two-row-before'),
              style: style,
              maxLines: 1,
              softWrap: false,
            ),
            Container(
              key: const ValueKey('two-row-cursor'),
              width: 2,
              height: 28,
              color: scheme.primary,
            ),
            Text.rich(
              TextSpan(children: after),
              key: const ValueKey('two-row-after'),
              style: style,
              maxLines: 1,
              softWrap: false,
            ),
          ],
        ),
      ),
    );
  }
}

/// The text before the cursor, the cursor and the text after it, side by
/// side from the row's start. A line wider than the row scrolls sideways,
/// only as far as keeps the cursor in view, so the letters stay where
/// they are while the cursor moves among them.
class _CursorStrip extends MultiChildRenderObjectWidget {
  const _CursorStrip({required super.children});

  @override
  RenderObject createRenderObject(BuildContext context) => _RenderCursorStrip();
}

class _StripParentData extends ContainerBoxParentData<RenderBox> {}

class _RenderCursorStrip extends RenderBox
    with
        ContainerRenderObjectMixin<RenderBox, _StripParentData>,
        RenderBoxContainerDefaultsMixin<RenderBox, _StripParentData> {
  /// How far the line is scrolled; kept from one layout to the next, as
  /// a text field keeps it.
  double _scroll = 0;

  final _clip = LayerHandle<ClipRectLayer>();

  @override
  void setupParentData(RenderBox child) {
    if (child.parentData is! _StripParentData) {
      child.parentData = _StripParentData();
    }
  }

  @override
  void performLayout() {
    final children = getChildrenAsList();
    final loose = BoxConstraints(maxHeight: constraints.maxHeight);
    var width = 0.0;
    var height = 0.0;
    for (final child in children) {
      child.layout(loose, parentUsesSize: true);
      width += child.size.width;
      height = math.max(height, child.size.height);
    }
    size = constraints.constrain(Size(double.infinity, height));
    if (width <= size.width) {
      _scroll = 0;
    } else {
      // The cursor is the second child: its edges stay inside the row.
      final left = children.first.size.width;
      final right = left + (children.length > 1 ? children[1].size.width : 0);
      if (left < _scroll) _scroll = left;
      if (right > _scroll + size.width) _scroll = right - size.width;
      _scroll = _scroll.clamp(0, width - size.width);
    }
    var x = -_scroll;
    for (final child in children) {
      (child.parentData! as _StripParentData).offset = Offset(
        x,
        (size.height - child.size.height) / 2,
      );
      x += child.size.width;
    }
  }

  @override
  bool hitTestChildren(BoxHitTestResult result, {required Offset position}) =>
      defaultHitTestChildren(result, position: position);

  @override
  void paint(PaintingContext context, Offset offset) {
    _clip.layer = context.pushClipRect(
      needsCompositing,
      offset,
      Offset.zero & size,
      defaultPaint,
      oldLayer: _clip.layer,
    );
  }

  @override
  void dispose() {
    _clip.layer = null;
    super.dispose();
  }
}
