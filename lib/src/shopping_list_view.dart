import 'dart:async';

import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'browser.dart';
import 'checklist_view.dart';
import 'l10n.dart';
import 'line_editing.dart';
import 'note_editor_screen.dart';
import 'routine_editor_screen.dart';

/// What a Note's card knows of its Shopping List (#122): [read] gives it
/// as it shows now, or null for a Note that is none, read when the card
/// is, never while building; [lists] checks its Lines.
typedef NoteShoppingList = ({
  ShoppingList? Function() read,
  ShoppingLists lists,
});

/// Calls [onDay] at the start of the next day by the Device's wall clock
/// while [due] says something changes then, such as a struck Line that
/// comes back: nothing is written when one does (ADR 0030).
mixin _ReadsAtDayStart<T extends StatefulWidget> on State<T> {
  Timer? _dayTimer;

  Store get dayStore;

  void onDay();

  void watchDay(bool due) {
    _dayTimer?.cancel();
    _dayTimer = null;
    if (!due) return;
    final wall = deviceWallClock(dayStore.clock().toUtc());
    final next = DateTime(wall.year, wall.month, wall.day + 1);
    _dayTimer = Timer(next.difference(wall) + const Duration(seconds: 1), () {
      if (mounted) onDay();
    });
  }

  @override
  void dispose() {
    _dayTimer?.cancel();
    super.dispose();
  }
}

/// A Shopping List on a Note's card (#122): every Line with its checkbox
/// on a [full] card, open ones first and struck ones at the end with the
/// day they come back, checkable where this Device may; else how many
/// are open. Nothing for a Note that is none. It reads the list whenever
/// the card is built anew, after a check and when a new day begins.
class CardShoppingList extends StatefulWidget {
  final String noteId;
  final NoteShoppingList shopping;
  final bool full;

  const CardShoppingList({
    super.key,
    required this.noteId,
    required this.shopping,
    required this.full,
  });

  @override
  State<CardShoppingList> createState() => _CardShoppingListState();
}

class _CardShoppingListState extends State<CardShoppingList>
    with _ReadsAtDayStart {
  ShoppingList? _list;
  var _checks = false;

  @override
  Store get dayStore => widget.shopping.lists.store;

  @override
  void initState() {
    super.initState();
    _show(widget.shopping.read());
  }

  @override
  void didUpdateWidget(CardShoppingList old) {
    super.didUpdateWidget(old);
    // A check changes a Line, not the Note, so this reads every time.
    _show(widget.shopping.read());
  }

  /// Reads anew, not from the list of cards, which still holds the old.
  @override
  void onDay() => setState(_readOwn);

  void _readOwn() =>
      _show(widget.shopping.lists.byNote(widget.noteId, deviceWallClock));

  void _show(ShoppingList? list) {
    _list = list;
    _checks = list != null && widget.shopping.lists.mayCheck(list.noteId);
    watchDay(list != null && list.struckCount > 0);
  }

  void _check(String id, bool checked) {
    final lists = widget.shopping.lists;
    checked
        ? lists.check(id, deviceWallClock)
        : lists.uncheck(id, deviceWallClock);
    setState(_readOwn);
  }

  @override
  Widget build(BuildContext context) {
    final list = _list;
    if (list == null) return const SizedBox.shrink();
    if (!widget.full) {
      return Padding(
        padding: const EdgeInsets.only(top: 4),
        child: Text(
          context.t.shoppingOpenCount(list.openCount),
          key: const ValueKey('shopping-count'),
          style: Theme.of(context).textTheme.bodyMedium,
        ),
      );
    }
    return Padding(
      padding: const EdgeInsets.only(top: 8),
      child: ChecklistLines(
        key: const ValueKey('shopping-card'),
        lines: _shown(context, list),
        onCheck: _checks ? _check : null,
      ),
    );
  }
}

/// The Lines of [list] as they are drawn: open ones first, struck ones
/// at the end with the day they come back under them, as a short weekday
/// and date ("back Tue, Nov 12").
List<ShownLine> _shown(BuildContext context, ShoppingList list) {
  final t = context.t;
  final day = DateFormat.MMMEd(Localizations.localeOf(context).toString());
  return [for (final line in list.lines) _shownLine(t, day, line)];
}

/// [line] as it is drawn, a struck one with the [day] it comes back.
ShownLine _shownLine(AppLocalizations t, DateFormat day, ShoppingLine line) => (
  id: line.id,
  text: line.text,
  inGroup: false,
  checked: line.struck,
  below: switch (line.backOn) {
    final back? when line.struck => t.shoppingBack(day.format(back.wall)),
    _ => null,
  },
);

/// What the menu of a Shopping List's Line does besides setting its
/// reappear time.
enum _ShopStep { delete }

/// The Shopping List part of a Note's editor (#122), nothing while the
/// Note is none: its Lines with their boxes, typed in place as one block
/// of text as a Checklist's are (#141, #184, [TypedRows]); a new list
/// starts with one empty Line. A Line stays where it was typed, also the
/// next time; one checked while the editor is open stays in its row until
/// the list is read anew, then it shows below the open ones. A name the
/// list has already brings its Line back instead. ⋮ of a Line this Device may change has its reappear time
/// as radio buttons and Delete. "Back to a plain Note" is in the editor's
/// kind badge (#160). What is typed is stored as the cursor leaves a Line
/// and whenever [typed] says so; a Line left empty goes as the editor is
/// left. It follows the log as it changes and reads again when a new day
/// begins.
class ShoppingSection extends StatefulWidget {
  final Notes notes;
  final String noteId;

  /// Tells it when the editor saves and when it is left.
  final TypedLines? typed;

  /// Whether the cursor starts in the empty Line of a new list (#161).
  final bool startLine;

  const ShoppingSection({
    super.key,
    required this.notes,
    required this.noteId,
    this.typed,
    this.startLine = false,
  });

  @override
  State<ShoppingSection> createState() => _ShoppingSectionState();
}

class _ShoppingSectionState extends State<ShoppingSection>
    with
        ReloadsOnLogChange,
        _ReadsAtDayStart,
        TypedRows,
        AutomaticKeepAliveClientMixin {
  late final _lists = ShoppingLists(widget.notes.store);
  late final _twoRows = widget.notes.store.twoRowEditorOn;
  ShoppingList? _list;
  var _checks = false;
  Set<String> _editable = const {};

  @override
  Store get log => widget.notes.store;

  @override
  Store get dayStore => widget.notes.store;

  @override
  void reloadLog() => setState(_load);

  @override
  void onDay() => setState(_load);

  /// Kept while scrolled away, so nothing typed goes with it.
  @override
  bool get wantKeepAlive => true;

  @override
  void initState() {
    super.initState();
    _load();
    widget.typed?.listen(keepTyped);
    if (widget.startLine) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) focusEmptyRow();
      });
    }
  }

  @override
  void didUpdateWidget(ShoppingSection old) {
    super.didUpdateWidget(old);
    if (old.typed != widget.typed) {
      old.typed?.forget(keepTyped);
      widget.typed?.listen(keepTyped);
    }
  }

  @override
  void dispose() {
    widget.typed?.forget(keepTyped);
    super.dispose();
  }

  /// Whether the list was read once, to tell a list made in the editor.
  var _read = false;

  void _load() {
    final made = _read && _list == null;
    _read = true;
    final list = _list = _lists.byNote(widget.noteId, deviceWallClock);
    _checks = list != null && _lists.mayCheck(widget.noteId);
    _editable = {
      for (final line in list?.lines ?? const <ShoppingLine>[])
        if (_lists.mayEdit(line.id)) line.id,
    };
    rows = rowsInPlace(switch (list) {
      final list? => [
        for (final line in list.lines)
          (id: line.id, text: line.text, inGroup: false),
      ],
      null => null,
    });
    // Made a Shopping List in the editor: the cursor in its first Line.
    if (made && list != null && list.lines.isEmpty && rows.isNotEmpty) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) focusEmptyRow();
      });
    }
    watchDay(list != null && list.struckCount > 0);
  }

  @override
  Store get linesLog => widget.notes.store;

  /// Whether the Note is a Shopping List out of the trash, to write to.
  @override
  bool get linesWritable =>
      !(widget.notes.byId(widget.noteId)?.isTrashed ?? true) &&
      _lists.isShoppingList(widget.noteId);

  @override
  bool get mayAddLine => _checks;

  @override
  bool mayEditLine(String id) => _lists.mayEdit(id);

  @override
  bool showsLine(String id) =>
      _lists.byNote(widget.noteId, deviceWallClock)?.line(id) != null;

  /// A Line checked while the editor is open keeps its row, but the list
  /// places a new one among the open Lines.
  @override
  bool placesAmong(TypedLine row) => switch (row.id) {
    final id? => !(_list?.line(id)?.struck ?? false),
    null => false,
  };

  /// Where it was typed among the open Lines; a name it has already
  /// brings that Line back.
  @override
  String addLine(String text, {required int at, required bool inGroup}) =>
      _lists.add(widget.noteId, text, deviceWallClock, at: at);

  @override
  void renameLine(String id, String text) => _lists.rename(id, text);

  /// Into the trash, as a Checklist's Line goes (ADR 0030).
  @override
  void deleteLine(String id) => _lists.delete(id);

  void _check(String id, bool checked) {
    // A new Line is stored first; an empty one has nothing to check.
    final line = storedId(id);
    if (line == null) return;
    checked
        ? _lists.check(line, deviceWallClock)
        : _lists.uncheck(line, deviceWallClock);
    setState(_load);
  }

  String _reappearLabel(Reappear reappear) => reappear == Reappear.once
      ? context.t.shoppingOnce
      : context.t.shoppingWeeks(reappear.weeks);

  Future<void> _menu(String shown, Offset at) async {
    // A new Line is stored first, and what was typed, so the menu sees
    // the list as shown.
    final id = storedId(shown);
    if (id == null) return;
    keepTyped(leaving: false);
    final line = _lists.byNote(widget.noteId, deviceWallClock)?.line(id);
    if (line == null) return;
    final t = context.t;
    final overlay =
        Overlay.of(context).context.findRenderObject() as RenderBox?;
    if (overlay == null) return;
    final picked = await showMenu<Object>(
      context: context,
      position: RelativeRect.fromRect(
        overlay.globalToLocal(at) & Size.zero,
        Offset.zero & overlay.size,
      ),
      items: [
        // The heading the choices continue: "Reappear time" "1 week".
        PopupMenuItem(enabled: false, child: Text(t.shoppingReappear)),
        for (final reappear in Reappear.values)
          PopupMenuItem(
            key: ValueKey('shopping-reappear-${reappear.code}'),
            value: reappear,
            child: Semantics(
              checked: reappear == line.reappear,
              inMutuallyExclusiveGroup: true,
              child: Row(
                children: [
                  Icon(
                    reappear == line.reappear
                        ? Icons.radio_button_checked
                        : Icons.radio_button_unchecked,
                  ),
                  const SizedBox(width: 12),
                  Flexible(child: Text(_reappearLabel(reappear))),
                ],
              ),
            ),
          ),
        const PopupMenuDivider(),
        PopupMenuItem(
          key: const ValueKey('shopping-delete'),
          value: _ShopStep.delete,
          child: Row(
            children: [
              const Icon(Icons.delete_outline),
              const SizedBox(width: 12),
              Flexible(child: Text(t.delete)),
            ],
          ),
        ),
      ],
    );
    if (picked == null || !mounted) return;
    // A sync while the menu was open may have taken it away.
    if (!linesWritable || !showsLine(id)) return;
    switch (picked) {
      case Reappear reappear:
        // A tap on the one it has changes nothing.
        if (reappear == line.reappear) return;
        _lists.setReappear(id, reappear);
      case _ShopStep.delete:
        if (!await confirmLineDelete(context)) return;
        if (!mounted || !linesWritable || !showsLine(id)) return;
        _lists.delete(id);
    }
    setState(_load);
  }

  @override
  Widget build(BuildContext context) {
    super.build(context);
    final list = _list;
    if (list == null) return const SizedBox.shrink();
    final t = context.t;
    final day = DateFormat.MMMEd(Localizations.localeOf(context).toString());
    ShownLine shownRow(TypedLine row) {
      if (row.id case final id?) {
        if (list.line(id) case final line?) return _shownLine(t, day, line);
      }
      // New, or gone from the list a moment ago and read anew soon.
      return (
        id: TypedRows.shownId(row),
        text: row.kept,
        inGroup: false,
        checked: false,
        below: null,
      );
    }

    // Under the text field, as the Lines are under the text on a card.
    return Padding(
      padding: const EdgeInsets.only(top: 16),
      child: EditorPanel(
        key: const ValueKey('shopping'),
        heading: t.shoppingHeading,
        child: ChecklistLines(
          lines: [for (final row in rows) shownRow(row)],
          onCheck: _checks ? _check : null,
          onMenu: _menu,
          editable: withMenus(_editable),
          field: (shown) => typedField(
            shown,
            prefix: 'shopping',
            editable: _editable,
            twoRows: _twoRows,
          ),
          compact: true,
        ),
      ),
    );
  }
}

/// Makes the Note [noteId] a Shopping List where this Device may; whether
/// it did.
bool makeShoppingList(Notes notes, String noteId) {
  final lists = ShoppingLists(notes.store);
  if (!lists.mayMake(noteId)) return false;
  lists.make(noteId);
  return true;
}

/// Turns the Shopping List of [noteId] back into a plain Note where this
/// Device may; whether it did.
bool shoppingListToPlain(Notes notes, String noteId) {
  final store = notes.store;
  if (!ShoppingLists(store).isShoppingList(noteId) ||
      !store.mayChange(noteId) ||
      !(notes.byId(noteId)?.textEditable ?? false)) {
    return false;
  }
  ShoppingLists(store).toPlain(noteId, deviceWallClock);
  return true;
}
