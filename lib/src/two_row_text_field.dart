import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/semantics.dart';

import 'l10n.dart';

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
class TwoRowTextField extends StatefulWidget {
  /// The height of the bottom row.
  static const rowHeight = 56.0;

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

  const TwoRowTextField({
    super.key,
    required this.controller,
    required this.twoRows,
    this.readOnly = false,
    this.autofocus = false,
    this.style,
    this.decoration = const InputDecoration(),
  });

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

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    _focus.addListener(_focusChanged);
    widget.controller.addListener(_changed);
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
    }
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
    if (_selecting && widget.controller.text != _selectingText) {
      setState(() => _selecting = false);
    }
  }

  bool get _rowShown =>
      widget.twoRows && !widget.readOnly && _focus.hasFocus && _keyboard > 0;

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
    if (!widget.twoRows) {
      return TextField(
        controller: widget.controller,
        readOnly: widget.readOnly,
        autofocus: widget.autofocus,
        maxLines: null,
        textCapitalization: TextCapitalization.sentences,
        style: widget.style,
        decoration: widget.decoration,
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
      decoration: widget.decoration,
      magnifierConfiguration: TextMagnifierConfiguration.disabled,
      selectionControls: _handles,
      contextMenuBuilder: _menu,
      // The bottom row must not hide the line being typed.
      scrollPadding: const EdgeInsets.fromLTRB(
        20,
        20,
        20,
        20 + TwoRowTextField.rowHeight,
      ),
    );
    // The field's own gestures would let a sideways drag move the cursor:
    // they are switched off, and these, which drag nothing, take their
    // place. Long-press selection, the handles of a selection and the
    // menu work as in the field.
    return OverlayPortal(
      controller: _row,
      overlayChildBuilder: _bottomRow,
      child: TextFieldTapRegion(
        child: _gestures.buildGestureDetector(
          behavior: HitTestBehavior.translucent,
          child: IgnorePointer(child: field),
        ),
      ),
    );
  }

  Widget _bottomRow(BuildContext context) {
    if (!_rowShown) return const SizedBox.shrink();
    final t = context.t;
    return Positioned(
      left: 0,
      right: 0,
      bottom: _keyboard,
      // Part of the field: a tap here keeps the keyboard open.
      child: TextFieldTapRegion(
        child: Material(
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

/// The text around the cursor on one line, the cursor in the middle and
/// a selection highlighted; a long press starts or ends selecting.
class _CursorLine extends StatelessWidget {
  final TextEditingValue value;
  final bool selecting;
  final VoidCallback onLongPress;

  const _CursorLine({
    required this.value,
    required this.selecting,
    required this.onLongPress,
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
