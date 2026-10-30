import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/semantics.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'l10n.dart';
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
/// boundary, and, while the keyboard is open, one line of the text around
/// the cursor directly above it, with ◀ and ▶ stepping the cursor one
/// character at a time (UI laws, "Text editor"). [twoRows] false gives the
/// plain system text field.
///
/// Both rows share [controller]; the top row stays the one with focus, so
/// the keyboard's suggestions, autocorrect, swipe typing and voice input
/// work as in any text field.
///
/// With [marks], in either form, a tap on a ☐, ✅ or ❌ at the start of a
/// line opens Check, Fail and Open at the finger and changes it in the
/// text, and the bar above the keyboard has a button that puts ☐ at the
/// start of the cursor's line (ADR 0031); a [readOnly] field has neither.
///
/// With [formats], for a [TagTextController], the same bar formats the
/// selection or the word at the cursor, or the lines they touch (ADR
/// 0033), each button telling whether the cursor is in its format. The
/// first format makes a plain text formatted for good, and so do a
/// format typed as Markdown (`Markdown.shortcutAt`) and a pasted text
/// that uses one; [onFormatted] runs then. In two rows, the bottom row
/// of a formatted text shows the source of the cursor's line only.
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

  /// How many characters the bottom row shows on each side of the cursor.
  static const reach = 13;

  final TextEditingController controller;
  final bool twoRows;
  final bool readOnly;
  final bool autofocus;
  final TextStyle? style;
  final InputDecoration decoration;
  final bool marks;
  final bool formats;
  final VoidCallback? onFormatted;

  const TwoRowTextField({
    super.key,
    required this.controller,
    required this.twoRows,
    this.readOnly = false,
    this.autofocus = false,
    this.style,
    this.decoration = const InputDecoration(),
    this.marks = false,
    this.formats = false,
    this.onFormatted,
  });

  /// The height of what rides on the keyboard above a field that types
  /// in [twoRows], with the bar of formats or status marks when [bar].
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

  /// Whether the field has the bar of formats or status marks.
  bool get _hasBar => _formatting != null || _marks;

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

  bool get _marks => widget.marks && !widget.readOnly;

  /// Opens the menu of the status mark at [at] (global), if there is one
  /// at the start of a line there; whether there was.
  bool _tappedMark(Offset at) {
    if (!_marks) return false;
    final editable = _editable()?.renderEditable;
    if (editable == null) return false;
    final local = editable.globalToLocal(at);
    for (final mark in TextMarks.find(widget.controller.text)) {
      final boxes = editable.getBoxesForSelection(
        TextSelection(baseOffset: mark.start, extentOffset: mark.end),
      );
      if (boxes.any((box) => box.toRect().inflate(4).contains(local))) {
        unawaited(_changeMark(mark, at));
        return true;
      }
    }
    return false;
  }

  /// Changes [mark] to the state picked in its menu at [at]; the cursor
  /// keeps its place in the text around it.
  Future<void> _changeMark(MarkedLine mark, Offset at) async {
    final to = await pickTextMark(context, at, mark.mark);
    if (to == null || !mounted) return;
    final value = widget.controller.value;
    final text = value.text;
    final swapped = TextMarks.swap(text, mark.line, to);
    if (swapped == null) return;
    // Typing while the menu was open cannot happen; a sync can.
    final now = TextMarks.find(text).where((m) => m.line == mark.line);
    final end = now.isEmpty ? mark.end : now.first.end;
    _replace(value, swapped, end);
  }

  /// Puts ☐ at the start of the cursor's line, after its spaces, tabs
  /// and bullet; the cursor stays on what it was at.
  void _insertMark() {
    final value = widget.controller.value;
    final text = value.text;
    final at = value.selection.isValid
        ? value.selection.baseOffset
        : text.length;
    final next = TextMarks.insertOpen(text, TextMarks.lineAt(text, at));
    if (next == null) return;
    var from = 0;
    while (from < text.length && text[from] == next[from]) {
      from++;
    }
    _replace(value, next, from);
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
        scrollPadding: scrollPadding,
        onTap: _marks
            ? () {
                if (_down case final at?) _tappedMark(at);
              }
            : null,
      );
      final Widget tapped = !_marks
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
    final at = _FormatsAt.of(value, markdown: controller?.markdown ?? false);
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
      if (_marks)
        _Format(
          key: 'two-row-insert-mark',
          label: t.textMarkInsert,
          icon: (_) => const Icon(Icons.check_box_outline_blank),
          run: _insertMark,
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
        _Format(
          key: 'format-indent',
          label: t.formatIndent,
          icon: (_) => const Icon(Icons.format_indent_increase),
          run: () =>
              _format((text, start, end) => Markdown.indent(text, start, end)),
        ),
        _Format(
          key: 'format-outdent',
          label: t.formatOutdent,
          icon: (_) => const Icon(Icons.format_indent_decrease),
          run: () => _format(
            (text, start, end) => Markdown.indent(text, start, end, out: true),
          ),
        ),
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
                  // The source of the formatted line being typed.
                  lineOnly: _formatting?.markdown ?? false,
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
    // A status mark opens its menu; the cursor stays where it was.
    if (_field._tappedMark(details.globalPosition)) return;
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

  static _FormatsAt of(TextEditingValue value, {required bool markdown}) {
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
    final blocks = Markdown.parse(text);
    final line = TextMarks.lineAt(text, start);
    if (line >= blocks.length) return none;
    final block = blocks[line];
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

/// The text around the cursor on one line, the cursor in the middle and
/// a selection highlighted; a long press starts or ends selecting.
class _CursorLine extends StatelessWidget {
  final TextEditingValue value;
  final bool selecting;
  final VoidCallback onLongPress;

  /// Whether it shows only the cursor's line, not across line breaks.
  final bool lineOnly;

  const _CursorLine({
    required this.value,
    required this.selecting,
    required this.onLongPress,
    this.lineOnly = false,
  });

  /// [text] from [from] to [to] as one line, the selection highlighted.
  static List<TextSpan> _spans(
    String text,
    int from,
    int to,
    TextSelection selection,
    TextStyle marked,
  ) {
    String line(int a, int b) => text
        .substring(a, b)
        .replaceAll('\r\n', '↵')
        .replaceAll('\n', '↵')
        .replaceAll('\t', ' ');
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
    final text = value.text;
    final selection = value.selection.isValid
        ? value.selection
        : TextSelection.collapsed(offset: text.length);
    final cursor = selection.extentOffset;
    var from = cursor;
    var to = cursor;
    for (var i = 0; i < TwoRowTextField.reach; i++) {
      from = graphemeBefore(text, from);
      to = graphemeAfter(text, to);
    }
    if (lineOnly) {
      bool breaks(int i) => text[i] == '\n' || text[i] == '\r';
      var start = math.min(cursor, text.length);
      while (start > from && !breaks(start - 1)) {
        start--;
      }
      var end = math.min(cursor, text.length);
      while (end < to && !breaks(end)) {
        end++;
      }
      from = start;
      to = end;
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
        child: Row(
          children: [
            Expanded(
              child: ClipRect(
                child: OverflowBox(
                  alignment: AlignmentDirectional.centerEnd,
                  maxWidth: double.infinity,
                  child: Text.rich(
                    TextSpan(children: before),
                    key: const ValueKey('two-row-before'),
                    style: style,
                    maxLines: 1,
                    softWrap: false,
                  ),
                ),
              ),
            ),
            Container(
              key: const ValueKey('two-row-cursor'),
              width: 2,
              height: 28,
              color: scheme.primary,
            ),
            Expanded(
              child: ClipRect(
                child: OverflowBox(
                  alignment: AlignmentDirectional.centerStart,
                  maxWidth: double.infinity,
                  child: Text.rich(
                    TextSpan(children: after),
                    key: const ValueKey('two-row-after'),
                    style: style,
                    maxLines: 1,
                    softWrap: false,
                  ),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
