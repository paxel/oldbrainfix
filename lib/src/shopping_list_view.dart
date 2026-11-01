import 'dart:async';

import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'browser.dart';
import 'checklist_view.dart';
import 'l10n.dart';
import 'line_editing.dart';
import 'note_editor_screen.dart';
import 'routine_editor_screen.dart';
import 'two_row_text_field.dart';

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
  return [
    for (final line in list.lines)
      (
        id: line.id,
        text: line.text,
        inGroup: false,
        checked: line.struck,
        below: switch (line.backOn) {
          final back? when line.struck => t.shoppingBack(day.format(back.wall)),
          _ => null,
        },
      ),
  ];
}

/// What the menu of a Shopping List's Line does besides setting its
/// reappear time.
enum _ShopStep { delete }

/// The Shopping List part of a Note's editor (#122), nothing while the
/// Note is none: at the top a field for anyone with a role in its Area,
/// whose Return adds what was typed and leaves it ready for the next,
/// and "Add"; then its Lines to check, open first and struck at the end,
/// each renamed in place where this Device may change it (#141); for
/// those, ⋮ with its reappear time as radio buttons and Delete. "Back
/// to a plain Note" is in the editor's kind badge (#160). What is typed
/// is stored as the cursor leaves a Line and whenever [typed] says so; as the editor
/// is left, what is left in the field is added and a Line left empty
/// goes. It
/// follows the log as it changes and reads again when a new day begins.
class ShoppingSection extends StatefulWidget {
  final Notes notes;
  final String noteId;

  /// Tells it when the editor saves and when it is left.
  final TypedLines? typed;

  const ShoppingSection({
    super.key,
    required this.notes,
    required this.noteId,
    this.typed,
  });

  @override
  State<ShoppingSection> createState() => _ShoppingSectionState();
}

class _ShoppingSectionState extends State<ShoppingSection>
    with ReloadsOnLogChange, _ReadsAtDayStart, AutomaticKeepAliveClientMixin {
  late final _lists = ShoppingLists(widget.notes.store);
  late final _twoRows = widget.notes.store.twoRowEditorOn;
  final _add = TextEditingController();
  ShoppingList? _list;
  var _checks = false;
  Set<String> _editable = const {};

  /// The Lines as they are typed, by id.
  Map<String, TypedLine> _rows = const {};

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
    _add.addListener(_addChanged);
    widget.typed?.listen(_keepTyped);
  }

  @override
  void didUpdateWidget(ShoppingSection old) {
    super.didUpdateWidget(old);
    if (old.typed != widget.typed) {
      old.typed?.forget(_keepTyped);
      widget.typed?.listen(_keepTyped);
    }
  }

  @override
  void dispose() {
    widget.typed?.forget(_keepTyped);
    _add.dispose();
    for (final row in _rows.values) {
      row.dispose();
    }
    super.dispose();
  }

  void _load() {
    final list = _list = _lists.byNote(widget.noteId, deviceWallClock);
    _checks = list != null && _lists.mayCheck(widget.noteId);
    _editable = {
      for (final line in list?.lines ?? const <ShoppingLine>[])
        if (_lists.mayEdit(line.id)) line.id,
    };
    final old = _rows;
    _rows = {
      for (final id in _editable)
        if (list?.line(id) case final line?)
          id: (old[id] ?? _row(id, line.text))..take(line.text),
    };
    for (final MapEntry(:key, :value) in old.entries) {
      if (!_rows.containsKey(key)) {
        WidgetsBinding.instance.addPostFrameCallback((_) => value.dispose());
      }
    }
    watchDay(list != null && list.struckCount > 0);
  }

  TypedLine _row(String id, String text) {
    final row = TypedLine(id: id, kept: text);
    row.text.addListener(() {
      if (row.setting) return;
      final typed = row.text.text;
      // Return ends the rename; there is no new Line under it.
      if (typed.contains('\n') || typed.contains('\r')) {
        scheduleMicrotask(() => _returned(row));
      }
    });
    row.focus.addListener(() {
      if (!row.focus.hasFocus) scheduleMicrotask(() => _store(row));
    });
    return row;
  }

  bool get _open =>
      !(widget.notes.byId(widget.noteId)?.isTrashed ?? true) &&
      _lists.isShoppingList(widget.noteId);

  /// Renames the Line of [row] to what was typed, unless it is empty.
  void _store(TypedLine row) {
    final id = row.id;
    final typed = row.typed;
    if (!mounted || id == null || !identical(_rows[id], row)) return;
    if (typed.isEmpty || !row.changed || !_open) return;
    if (!_shows(id) || !_lists.mayEdit(id)) return;
    _lists.rename(id, typed);
    row.kept = typed;
  }

  void _returned(TypedLine row) {
    if (!mounted || !identical(_rows[row.id], row)) return;
    final typed = row.typed;
    row.set(
      TextEditingValue(
        text: typed,
        selection: TextSelection.collapsed(offset: typed.length),
      ),
    );
    _store(row);
    row.focus.unfocus();
  }

  /// A Return in the field adds what was typed before it; the field
  /// keeps the rest, ready for the next.
  void _addChanged() {
    final text = _add.text;
    if (text.contains('\n') || text.contains('\r')) {
      scheduleMicrotask(_addReturned);
    }
  }

  void _addReturned() {
    if (!mounted) return;
    final text = _add.text;
    final end = text.lastIndexOf(RegExp('[\r\n]'));
    if (end < 0) return;
    final rest = text.substring(end + 1);
    _add.value = TextEditingValue(
      text: rest,
      selection: TextSelection.collapsed(offset: rest.length),
    );
    _addText(text.substring(0, end));
  }

  /// Adds a Line for every line of [text] that is not blank, the first
  /// ending at the top; a name the list has already brings its Line
  /// back instead.
  void _addText(String text) {
    if (text.trim().isEmpty || !_open || !_lists.mayCheck(widget.noteId)) {
      return;
    }
    _lists.addLines(widget.noteId, text, deviceWallClock);
    setState(_load);
  }

  void _addTyped() {
    final text = _add.text;
    _add.clear();
    _addText(text);
  }

  /// Stores what was typed; [leaving], what is left in the field is
  /// added and the Lines left empty go.
  void _keepTyped({required bool leaving}) {
    if (!mounted) return;
    final typed = [
      for (final row in _rows.values)
        if (row.changed && row.typed.isNotEmpty) row,
    ];
    if (typed.isNotEmpty) {
      widget.notes.store.batch(() {
        for (final row in typed) {
          _store(row);
        }
      });
    }
    if (!leaving) return;
    // What is left in the field is added as the editor is left only, so
    // a save on the way leaves it there to finish.
    if (_add.text.trim().isNotEmpty) _addTyped();
    final gone = [
      for (final MapEntry(:key, :value) in _rows.entries)
        if (value.typed.isEmpty && _shows(key)) key,
    ];
    if (gone.isEmpty || !_open) return;
    widget.notes.store.batch(() {
      for (final id in gone) {
        _lists.delete(id);
      }
    });
    setState(_load);
  }

  void _check(String id, bool checked) {
    checked
        ? _lists.check(id, deviceWallClock)
        : _lists.uncheck(id, deviceWallClock);
    setState(_load);
  }

  String _reappearLabel(Reappear reappear) => reappear == Reappear.once
      ? context.t.shoppingOnce
      : context.t.shoppingWeeks(reappear.weeks);

  Future<void> _menu(String id, Offset at) async {
    final line = _list?.line(id);
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
    switch (picked) {
      case Reappear reappear:
        // A tap on the one it has changes nothing.
        if (reappear == line.reappear) return;
        _lists.setReappear(id, reappear);
      case _ShopStep.delete:
        // Into the trash, as a Checklist's Line goes (ADR 0030).
        if (!await confirmLineDelete(context)) return;
        if (!mounted || !_shows(id)) return;
        _lists.delete(id);
    }
    setState(_load);
  }

  /// Whether the Line [id] still shows on the list.
  bool _shows(String id) =>
      _lists.byNote(widget.noteId, deviceWallClock)?.line(id) != null;

  /// The field of [shown] where this Device may rename it in place.
  Widget? _field(ShownLine shown) {
    final row = _rows[shown.id];
    if (row == null) return null;
    return KeyedSubtree(
      key: ValueKey('shopping-field-${shown.id}'),
      child: TwoRowTextField(
        // The same field while the Line moves, struck or back.
        key: GlobalObjectKey(row),
        controller: row.text,
        focusNode: row.focus,
        twoRows: _twoRows,
        style: lineStyle(context, checked: shown.checked),
        decoration: const InputDecoration(
          filled: false,
          border: InputBorder.none,
          isDense: true,
          contentPadding: EdgeInsets.symmetric(vertical: 12),
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    super.build(context);
    final list = _list;
    if (list == null) return const SizedBox.shrink();
    final t = context.t;
    // Under the text field, as the Lines are under the text on a card.
    return Padding(
      padding: const EdgeInsets.only(top: 16),
      child: EditorPanel(
        key: const ValueKey('shopping'),
        heading: t.shoppingHeading,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            // Adding sits at the top, where a new Line shows.
            if (_checks) ...[
              TwoRowTextField(
                key: const ValueKey('shopping-add-text'),
                controller: _add,
                twoRows: _twoRows,
                decoration: InputDecoration(hintText: t.checklistNewLine),
              ),
              const SizedBox(height: 8),
              Align(
                alignment: Alignment.centerLeft,
                child: FilledButton.tonalIcon(
                  key: const ValueKey('shopping-add'),
                  onPressed: _addTyped,
                  icon: const Icon(Icons.add),
                  label: Text(t.add),
                ),
              ),
              const SizedBox(height: 8),
            ],
            ChecklistLines(
              lines: _shown(context, list),
              onCheck: _checks ? _check : null,
              onMenu: _menu,
              editable: _editable,
              field: _field,
            ),
          ],
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
