import 'dart:async';

import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'browser.dart';
import 'checklist_view.dart';
import 'l10n.dart';
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
enum _ShopStep { rename, delete }

/// The Shopping List part of a Note's editor (#122), nothing while the
/// Note is none: at the top a field and "Add" for anyone with a role in
/// its Area, then its Lines to check, open first and struck at the end;
/// for who may change a Line, ⋮ and a long press with Rename, its
/// reappear time as radio buttons and "Delete for good"; and, with
/// [onToPlain], "Back to a plain Note". It follows the log as it changes
/// and reads again when a new day begins.
class ShoppingSection extends StatefulWidget {
  final Notes notes;
  final String noteId;

  /// Turns the Note back into a plain Note; no button when null.
  final VoidCallback? onToPlain;

  const ShoppingSection({
    super.key,
    required this.notes,
    required this.noteId,
    this.onToPlain,
  });

  @override
  State<ShoppingSection> createState() => _ShoppingSectionState();
}

class _ShoppingSectionState extends State<ShoppingSection>
    with ReloadsOnLogChange, _ReadsAtDayStart {
  late final _lists = ShoppingLists(widget.notes.store);
  late final _twoRows = widget.notes.store.twoRowEditorOn;
  final _add = TextEditingController();
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

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void dispose() {
    _add.dispose();
    super.dispose();
  }

  void _load() {
    final list = _list = _lists.byNote(widget.noteId, deviceWallClock);
    _checks = list != null && _lists.mayCheck(widget.noteId);
    _editable = {
      for (final line in list?.lines ?? const <ShoppingLine>[])
        if (_lists.mayEdit(line.id)) line.id,
    };
    watchDay(list != null && list.struckCount > 0);
  }

  void _check(String id, bool checked) {
    checked
        ? _lists.check(id, deviceWallClock)
        : _lists.uncheck(id, deviceWallClock);
    setState(_load);
  }

  /// Adds a Line for every line typed that is not blank, the first typed
  /// ending at the top; a name the list has already brings its Line back
  /// instead.
  void _addTyped() {
    final rows = [
      for (final row in splitLines(_add.text))
        if (row.trim().isNotEmpty) row,
    ];
    if (rows.isEmpty) return;
    for (final row in rows.reversed) {
      _lists.add(widget.noteId, row, deviceWallClock);
    }
    _add.clear();
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
    PopupMenuItem<Object> item(_ShopStep step, IconData icon, String label) =>
        PopupMenuItem(
          key: ValueKey('shopping-${step.name}'),
          value: step,
          child: Row(
            children: [
              Icon(icon),
              const SizedBox(width: 12),
              Flexible(child: Text(label)),
            ],
          ),
        );
    final picked = await showMenu<Object>(
      context: context,
      position: RelativeRect.fromRect(
        overlay.globalToLocal(at) & Size.zero,
        Offset.zero & overlay.size,
      ),
      items: [
        item(_ShopStep.rename, Icons.edit_outlined, t.rename),
        const PopupMenuDivider(),
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
        item(
          _ShopStep.delete,
          Icons.delete_forever_outlined,
          t.shoppingDeleteForGood,
        ),
      ],
    );
    if (picked == null || !mounted) return;
    switch (picked) {
      case Reappear reappear:
        // A tap on the one it has changes nothing.
        if (reappear == line.reappear) return;
        _lists.setReappear(id, reappear);
      case _ShopStep.rename:
        final text = await showDialog<String>(
          context: context,
          builder: (_) => LineRenameDialog(text: line.text, twoRows: _twoRows),
        );
        final named = text?.replaceAll(RegExp('[\r\n]+'), ' ').trim();
        if (!mounted || named == null || named.isEmpty) return;
        // A sync while the dialog was open may have taken it away.
        if (!_shows(id)) return;
        _lists.rename(id, named);
      case _ShopStep.delete:
        if (!await confirmLineDelete(context, verb: t.shoppingDeleteForGood)) {
          return;
        }
        if (!mounted || !_shows(id)) return;
        _lists.delete(id);
    }
    setState(_load);
  }

  /// Whether the Line [id] still shows on the list.
  bool _shows(String id) =>
      _lists.byNote(widget.noteId, deviceWallClock)?.line(id) != null;

  @override
  Widget build(BuildContext context) {
    final list = _list;
    if (list == null) return const SizedBox.shrink();
    final t = context.t;
    final plain = widget.onToPlain;
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
            ),
            if (plain != null) ...[
              const SizedBox(height: 8),
              Align(
                alignment: Alignment.centerLeft,
                child: OutlinedButton.icon(
                  key: const ValueKey('shopping-to-plain'),
                  onPressed: plain,
                  icon: const Icon(Icons.notes),
                  label: Text(t.checklistToPlain),
                ),
              ),
            ],
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
