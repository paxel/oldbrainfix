import 'dart:async';

import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'browser.dart';
import 'checklist_view.dart';
import 'l10n.dart';
import 'line_editing.dart';
import 'look.dart';
import 'note_editor_screen.dart';
import 'routine_editor_screen.dart';
import 'setting_badges.dart';

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

/// The setting of this Device that keeps the struck Lines of the Shopping
/// List [noteId] unfolded on its card (#189); nothing kept means folded.
String struckShownSetting(String noteId) => 'shoppingStruckShown:$noteId';

/// A Shopping List on a Note's card (#122): on a [full] card the open
/// Lines with their checkboxes, then the struck ones folded under how many
/// they are, a tap showing them with the day they come back (#189),
/// checkable where this Device may; else how many are open. Nothing for a Note that is none. It reads the list whenever
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

  /// Whether the struck Lines show, unfolded by a tap (#189), kept per
  /// Note in a setting of this Device; folded by default.
  var _struckShown = false;

  String get _struckSetting => struckShownSetting(widget.noteId);

  void _toggleStruck() {
    final store = widget.shopping.lists.store;
    setState(() => _struckShown = !_struckShown);
    _struckShown
        ? store.setLocalSetting(_struckSetting, 'true')
        : store.removeLocalSetting(_struckSetting);
  }

  @override
  Store get dayStore => widget.shopping.lists.store;

  @override
  void initState() {
    super.initState();
    _struckShown =
        widget.shopping.lists.store.localSetting(_struckSetting) == 'true';
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
    final lines = _shown(context, list);
    final struck = [
      for (final line in lines)
        if (line.checked) line,
    ];
    final scheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.only(top: 8),
      child: Column(
        key: const ValueKey('shopping-card'),
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          ChecklistLines(
            lines: [
              for (final line in lines)
                if (!line.checked) line,
            ],
            onCheck: _checks ? _check : null,
          ),
          // The struck ones folded under the open ones (#189).
          if (struck.isNotEmpty) ...[
            Semantics(
              button: true,
              expanded: _struckShown,
              child: InkWell(
                key: const ValueKey('shopping-struck'),
                borderRadius: BorderRadius.circular(8),
                onTap: _toggleStruck,
                child: Padding(
                  padding: const EdgeInsets.symmetric(vertical: 8),
                  child: Row(
                    children: [
                      Icon(
                        _struckShown ? Icons.expand_less : Icons.expand_more,
                        color: scheme.onSurfaceVariant,
                      ),
                      const SizedBox(width: 8),
                      Text(
                        context.t.shoppingStruckCount(struck.length),
                        style: Theme.of(context).textTheme.bodyMedium
                            ?.copyWith(color: scheme.onSurfaceVariant),
                      ),
                    ],
                  ),
                ),
              ),
            ),
            if (_struckShown)
              ChecklistLines(
                key: const ValueKey('shopping-card-struck'),
                lines: struck,
                onCheck: _checks ? _check : null,
              ),
          ],
        ],
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

/// A reappear time in words: "None", "Once", "3 months" (#195).
String reappearWords(AppLocalizations t, Reappear reappear) =>
    switch (reappear.unit) {
      null => reappear.isOnce ? t.shoppingOnce : t.shoppingReappearNone,
      ReappearUnit.day => t.shoppingDays(reappear.count),
      ReappearUnit.week => t.shoppingWeeks(reappear.count),
      ReappearUnit.month => t.shoppingMonths(reappear.count),
      ReappearUnit.year => t.shoppingYears(reappear.count),
    };

/// A reappear time as its badge shows it, short text (#192, #195): "—"
/// for none, "Once", "4w", "3m".
String reappearShort(AppLocalizations t, Reappear reappear) =>
    switch (reappear.unit) {
      null => reappear.isOnce ? t.shoppingOnce : '—',
      ReappearUnit.day => t.shoppingShortDays(reappear.count),
      ReappearUnit.week => t.shoppingShortWeeks(reappear.count),
      ReappearUnit.month => t.shoppingShortMonths(reappear.count),
      ReappearUnit.year => t.shoppingShortYears(reappear.count),
    };

/// The icon of a Line's reappear time, on its badge and its dialog.
const reappearIcon = Icons.autorenew;

/// The badge at the end of a Shopping List's Line (#195): its reappear
/// time as short text, read as "Reappear time: 4 weeks". With [onTap] it
/// opens the reappear time dialog; without it only shows the time.
Widget reappearBadge(
  AppLocalizations t,
  String lineId,
  Reappear reappear, {
  VoidCallback? onTap,
}) => Padding(
  padding: const EdgeInsetsDirectional.only(start: 4),
  child: SettingBadge(
    key: ValueKey('shopping-reappear-$lineId'),
    setting: t.shoppingReappear,
    value: reappearShort(t, reappear),
    valueWords: reappearWords(t, reappear),
    icon: reappearIcon,
    onTap: onTap == null ? null : (_) => onTap(),
  ),
);

/// The choices of the reappear time dialog.
enum _Kind { none, once, after }

/// The reappear time dialog of a Line whose time is [current] (#195): None,
/// Once or After as radio buttons, After with its number from 1 to 99 and
/// Day, Week, Month or Year under it. Each change goes to [pick] at once;
/// Close closes it.
class ReappearDialog extends StatefulWidget {
  final Reappear current;
  final ValueChanged<Reappear> pick;

  const ReappearDialog({super.key, required this.current, required this.pick});

  @override
  State<ReappearDialog> createState() => _ReappearDialogState();
}

class _ReappearDialogState extends State<ReappearDialog> {
  late var _kind = switch (widget.current) {
    final r when r.isOnce => _Kind.once,
    final r when r.isUnset => _Kind.none,
    _ => _Kind.after,
  };
  late var _count = widget.current.unit == null ? 1 : widget.current.count;
  late var _unit = widget.current.unit ?? ReappearUnit.week;

  Reappear get _picked => switch (_kind) {
    _Kind.none => Reappear.unset,
    _Kind.once => Reappear.once,
    _Kind.after => Reappear.after(_count, _unit),
  };

  void _change(void Function() change) {
    setState(change);
    widget.pick(_picked);
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return BadgeDialog(
      setting: t.shoppingReappear,
      icon: reappearIcon,
      content: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        mainAxisSize: MainAxisSize.min,
        children: [
          ChoiceChips<_Kind>(
            key: const ValueKey('reappear-kind'),
            value: _kind,
            // After starts at a week, as on the desktop.
            onChanged: (kind) => _change(() {
              _kind = kind;
              _count = 1;
              _unit = ReappearUnit.week;
            }),
            options: [
              RadioOption(
                key: const ValueKey('reappear-none'),
                value: _Kind.none,
                title: Text(t.shoppingReappearNone),
              ),
              RadioOption(
                key: const ValueKey('reappear-once'),
                value: _Kind.once,
                title: Text(t.shoppingOnce),
              ),
              RadioOption(
                key: const ValueKey('reappear-after'),
                value: _Kind.after,
                title: Text(t.shoppingReappearAfter),
              ),
            ],
          ),
          if (_kind == _Kind.after) ...[
            CountStepper(
              key: const ValueKey('reappear-count'),
              value: _count,
              max: Reappear.maxCount,
              onChanged: (n) => _change(() => _count = n),
            ),
            ChoiceChips<ReappearUnit>(
              key: const ValueKey('reappear-unit'),
              value: _unit,
              onChanged: (unit) => _change(() => _unit = unit),
              options: [
                for (final unit in ReappearUnit.values)
                  RadioOption(
                    key: ValueKey('reappear-unit-${unit.letter}'),
                    value: unit,
                    title: Text(switch (unit) {
                      ReappearUnit.day => t.unitDays(_count),
                      ReappearUnit.week => t.unitWeeks(_count),
                      ReappearUnit.month => t.unitMonths(_count),
                      ReappearUnit.year => t.unitYears(_count),
                    }),
                  ),
              ],
            ),
          ],
        ],
      ),
    );
  }
}

/// What the menu of a Shopping List's Line does.
enum _ShopStep { delete }

/// The Shopping List part of a Note's editor (#122), nothing while the
/// Note is none: its Lines with their boxes, typed in place as one block
/// of text as a Checklist's are (#141, #184, [TypedRows]); a new list
/// starts with one empty Line. A Line stays where it was typed, also the
/// next time; one checked while the editor is open stays in its row until
/// the list is read anew, then it shows below the open ones. A name the
/// list has already brings its Line back instead. Each Line ends in the
/// badge of its reappear time (#195), whose tap opens its dialog where
/// this Device may change the Line; ⋮ of such a Line has Delete. "Back to
/// a plain Note" is in the editor's
/// kind badge (#160). What is typed is stored as the cursor leaves a Line
/// and whenever [typed] says so; a Line left empty goes as the editor is
/// left. It follows the log as it changes and reads again when a new day
/// begins.
class ShoppingSection extends StatefulWidget {
  final Notes notes;
  final String noteId;

  /// Tells it when the editor saves and when it is left.
  final TypedLines? typed;

  /// Whether the cursor starts in a new empty Line after the open ones:
  /// the one of a new list (#161), or one opened from its group on the
  /// Agenda (#276).
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
  List<DeletedLine> _removed = const [];

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
        // Before the struck Lines.
        if (mounted) startRowAt(rows.lastIndexWhere(placesAmong) + 1);
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
    _removed = _lists.deletedLines(widget.noteId);
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

  /// The reappear time dialog of the Line [shown], each pick written at
  /// once.
  Future<void> _reappear(String shown) async {
    // A new Line is stored first, and what was typed, as for the menu.
    final id = storedId(shown);
    if (id == null) return;
    keepTyped(leaving: false);
    final line = _lists.byNote(widget.noteId, deviceWallClock)?.line(id);
    if (line == null) return;
    var current = line.reappear;
    await showDialog<void>(
      context: context,
      builder: (_) => ReappearDialog(
        current: current,
        pick: (picked) {
          // A sync while the dialog was open may have taken it away.
          if (!mounted || !linesWritable || !showsLine(id)) return;
          if (picked == current) return;
          _lists.setReappear(id, picked);
          current = picked;
        },
      ),
    );
    if (mounted) setState(_load);
  }

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
      case _ShopStep.delete:
        _lists.delete(id);
        offerLineUndo(
          context,
          _lists.deletedLines,
          _lists.mayEdit,
          _lists.restore,
          widget.noteId,
          id,
        );
    }
    setState(_load);
  }

  /// Brings the deleted Line [id] back from "Removed Lines".
  void _restore(String id) {
    if (!linesWritable || !_lists.mayEdit(id)) return;
    if (!_lists.deletedLines(widget.noteId).any((l) => l.id == id)) return;
    _lists.restore(id);
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
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            ChecklistLines(
              lines: [for (final row in rows) shownRow(row)],
              onCheck: _checks ? _check : null,
              onMenu: _menu,
              editable: withMenus(_editable),
              trailing: (shown) => switch (list.line(shown.id)) {
                final line? => reappearBadge(
                  t,
                  line.id,
                  line.reappear,
                  onTap: _editable.contains(line.id)
                      ? () => _reappear(shown.id)
                      : null,
                ),
                // A new Line has none yet; a tap stores it first.
                null => reappearBadge(
                  t,
                  shown.id,
                  Reappear.unset,
                  onTap: _checks ? () => _reappear(shown.id) : null,
                ),
              },
              field: (shown) => typedField(
                shown,
                prefix: 'shopping',
                editable: _editable,
                twoRows: _twoRows,
              ),
              compact: true,
            ),
            if (_removed.isNotEmpty)
              RemovedLines(
                store: widget.notes.store,
                lines: _removed,
                onRestore: linesWritable
                    ? (id) => _lists.mayEdit(id) ? () => _restore(id) : null
                    : (_) => null,
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
