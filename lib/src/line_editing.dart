import 'package:flutter/widgets.dart';

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
