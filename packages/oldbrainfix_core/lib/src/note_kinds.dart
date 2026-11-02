import 'checklists.dart';
import 'shopping_lists.dart';
import 'store.dart';
import 'tallies.dart';

/// The kind of a Note (#188): the page it is. A Note has exactly one.
/// A Frog is no kind but a mark any Note can carry.
enum NoteKind {
  /// A Note of text alone.
  plain,
  checklist,
  shoppingList,
  tally,
  // A Session becomes a kind with #191; until then a Note whose Routine
  // is a Session reads as the kind its own fields say, plain at most.
}

/// Reads the kind of Notes, and splits the text of a Note of a kind into
/// its name and its description.
class NoteKinds {
  final Store store;

  NoteKinds(this.store);

  /// The kind of the Note [noteId]: a Tally while it has a field, which
  /// wins over a list (ADR 0030), else the list its `kind` names, else a
  /// plain Note.
  NoteKind of(String noteId) {
    if (Tallies(store).isTally(noteId)) return NoteKind.tally;
    if (Checklists(store).isChecklist(noteId)) return NoteKind.checklist;
    if (ShoppingLists(store).isShoppingList(noteId)) {
      return NoteKind.shoppingList;
    }
    return NoteKind.plain;
  }

  /// The name of a Note of a kind with the text [text]: its first line,
  /// as stored, without a line break. Shown, nothing is written.
  static String name(String text) {
    final at = _lineEnd(text);
    return at < 0 ? text : text.substring(0, at);
  }

  /// The description of a Note of a kind with the text [text]: all after
  /// its first line, as stored; empty when it has one line.
  static String description(String text) {
    final at = _lineEnd(text);
    if (at < 0) return '';
    final skip = text.startsWith('\r\n', at) ? 2 : 1;
    return text.substring(at + skip);
  }

  /// The text of a Note of the name [name] and the description
  /// [description]: the name alone while the description is empty, else
  /// both, a line break between them. The text of a name and a
  /// description read from a text with "\n" between them is that text.
  static String join(String name, String description) =>
      description.isEmpty ? name : '$name\n$description';

  /// Where the first line of [text] ends, at `\n` or `\r`, or -1.
  static int _lineEnd(String text) {
    for (var i = 0; i < text.length; i++) {
      final c = text.codeUnitAt(i);
      if (c == 0x0A || c == 0x0D) return i;
    }
    return -1;
  }
}
