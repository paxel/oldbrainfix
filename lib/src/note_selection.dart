import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'announcements.dart';
import 'file_into_screen.dart';
import 'hidden_from_ai_view.dart';
import 'l10n.dart';
import 'merge_into_screen.dart';
import 'note_card.dart';
import 'page_bar.dart';

/// The selection of a phone Note list (#273): "Select" in its bar shows a
/// box on every card this Device may change, as a long press opens the
/// editor and a tap switches the view; the bar then acts on the Notes
/// ticked. Back, or Stop selecting, drops it.
mixin SelectsNotes<T extends StatefulWidget> on State<T> {
  /// The Notes ticked; null while the list does not select.
  Set<String>? _selected;

  /// Whether the list shows its boxes.
  bool get selecting => _selected != null;

  /// The Notes ticked.
  Set<String> get selected => _selected ?? const {};

  void startSelecting() => setState(() => _selected = <String>{});

  void stopSelecting() {
    if (_selected == null) return;
    setState(() => _selected = null);
  }

  /// Ticks [note] or takes it out of the selection.
  void setSelected(Note note, bool on) {
    final selected = _selected;
    if (selected == null) return;
    setState(() => on ? selected.add(note.id) : selected.remove(note.id));
  }

  /// Keeps ticked only what [shown] still shows and this Device may
  /// change, so nothing unseen is acted on; for the list's reading.
  void keepShown(Iterable<PlacedNote> shown) {
    final selected = _selected;
    if (selected == null) return;
    final kept = {
      for (final placed in shown)
        if (placed.changes) placed.note.id,
    };
    selected.retainWhere(kept.contains);
  }

  /// The ticked Notes of [shown], in its order.
  List<Note> selectedOf(Iterable<PlacedNote> shown) => [
    for (final placed in shown)
      if (placed.changes && selected.contains(placed.note.id)) placed.note,
  ];

  /// [child] under a back that drops the selection before it leaves.
  Widget selectionBack(Widget child) => PopScope(
    canPop: !selecting,
    onPopInvokedWithResult: (didPop, _) {
      if (!didPop) stopSelecting();
    },
    child: child,
  );

  /// Whether any card of [shown] would get a box: only then the bar
  /// offers Select.
  bool mayTick(Iterable<PlacedNote> shown) =>
      shown.any((placed) => placed.changes);

  /// The bar's "Select", shown while the list does not select.
  BarAction selectButton(BuildContext context) => BarAction(
    key: const ValueKey('selectNotes'),
    icon: Icons.library_add_check_outlined,
    label: context.t.selectNotes,
    onPressed: startSelecting,
  );

  /// The bar while the list selects: Stop selecting, then what the
  /// selection offers, each greyed while nothing is ticked; Move to trash
  /// last, or in the Trash Restore alone.
  List<Widget> selectionActions(
    BuildContext context, {
    VoidCallback? onFile,
    VoidCallback? onMerge,
    ValueChanged<Importance>? onImportance,
    VoidCallback? onTrash,
    VoidCallback? onRestore,
  }) {
    final t = context.t;
    final any = selected.isNotEmpty;
    return [
      IconButton(
        key: const ValueKey('stopSelecting'),
        tooltip: t.stopSelecting,
        isSelected: true,
        onPressed: stopSelecting,
        icon: const Icon(Icons.library_add_check_outlined),
        selectedIcon: const Icon(Icons.close),
      ),
      if (onFile != null)
        BarAction(
          key: const ValueKey('selection-file'),
          icon: Icons.drive_file_move_outline,
          label: t.fileInto,
          onPressed: any ? onFile : null,
        ),
      if (onMerge != null)
        BarAction(
          key: const ValueKey('selection-merge'),
          icon: Icons.call_merge,
          label: t.mergeInto,
          onPressed: any ? onMerge : null,
        ),
      if (onImportance != null)
        _ImportanceButton(onPicked: any ? onImportance : null),
      if (onRestore != null)
        BarAction(
          key: const ValueKey('selection-restore'),
          icon: Icons.restore,
          label: t.restore,
          onPressed: any ? onRestore : null,
        ),
      if (onTrash != null)
        BarAction(
          key: const ValueKey('selection-trash'),
          icon: Icons.delete_outline,
          label: t.moveToTrash,
          onPressed: any ? onTrash : null,
        ),
    ];
  }

  /// The bar of a Note list in the lists while it selects: File into…,
  /// Merge into…, Importance and Move to trash on the ticked Notes of
  /// [shown]; [reload] runs after each that changed something.
  List<Widget> noteSelectionActions(
    BuildContext context,
    Notes notes,
    Places places,
    Iterable<PlacedNote> shown,
    VoidCallback reload,
  ) {
    Future<void> then(Future<bool> step) async {
      if (await step && mounted) reload();
    }

    return selectionActions(
      context,
      onFile: () => then(fileSelected(notes, places, shown)),
      onMerge: () => then(mergeSelected(notes, places, shown)),
      onImportance: (importance) {
        setSelectedImportance(notes, shown, importance);
        reload();
      },
      onTrash: () {
        trashSelected(notes, shown);
        reload();
      },
    );
  }

  /// Files the ticked Notes of [shown] into the Area or Topic picked in
  /// File into…, in one batch, and ends the selection; whether they moved.
  Future<bool> fileSelected(
    Notes notes,
    Places places,
    Iterable<PlacedNote> shown,
  ) async {
    final chosen = selectedOf(shown);
    if (chosen.isEmpty) return false;
    final at = {for (final note in chosen) note.place};
    final placeId = await Navigator.of(context).push(
      MaterialPageRoute<String>(
        builder: (_) => FileIntoScreen(
          places: places,
          current: at.length == 1 ? at.first : null,
        ),
      ),
    );
    if (!mounted || placeId == null) return false;
    final store = notes.store;
    final moved = [
      for (final note in chosen)
        if (store.mayChange(note.id)) note.id,
    ];
    // Nothing Hidden from AI goes into a Shared Area (#267): one question
    // for all of them, before any moves.
    final area = store.areaOf(placeId);
    if (area != null && store.isShared(area)) {
      final marks = {
        for (final id in moved) ...store.hiddenFromAiBy(id),
      }.toList()..sort();
      if (!await showToAiFirst(context, store, marks) || !mounted) {
        return false;
      }
    }
    final filed = await unlessHiddenFromAi(context, store, () {
      store.batch(() {
        for (final id in moved) {
          notes.file(id, placeId);
        }
      });
      return true;
    });
    if (!mounted || filed == null) return false;
    stopSelecting();
    return true;
  }

  /// Merges the ticked Notes of [shown] into the Note picked in Merge
  /// into…, in one batch (#273), and ends the selection; whether they
  /// were merged. A ticked Note picked takes the others in.
  Future<bool> mergeSelected(
    Notes notes,
    Places places,
    Iterable<PlacedNote> shown,
  ) async {
    final chosen = selectedOf(shown);
    if (chosen.isEmpty) return false;
    final intoId = await Navigator.of(context).push(
      MaterialPageRoute<String>(
        builder: (_) => MergeIntoScreen(
          notes: notes,
          places: places,
          // A single Note is not offered to itself.
          noteId: chosen.length == 1 ? chosen.single.id : null,
        ),
      ),
    );
    if (!mounted || intoId == null) return false;
    // A sync while picking may have taken one out of the lists.
    final going = [
      for (final note in chosen)
        if (note.id != intoId)
          if (notes.byId(note.id) case final now?
              when !now.isTrashed &&
                  !now.isMerged &&
                  notes.store.mayChange(now.id))
            now.id,
    ];
    final target = notes.byId(intoId);
    final merges =
        going.isNotEmpty &&
        target != null &&
        !target.isTrashed &&
        !target.isMerged;
    if (merges) notes.mergeAll(going, intoId);
    stopSelecting();
    return merges;
  }

  /// Sets the Importance of the ticked Notes of [shown] in one batch and
  /// ends the selection.
  void setSelectedImportance(
    Notes notes,
    Iterable<PlacedNote> shown,
    Importance importance,
  ) {
    final chosen = selectedOf(shown);
    notes.store.batch(() {
      for (final note in chosen) {
        if (note.importance != importance && notes.store.mayChange(note.id)) {
          notes.setImportance(note.id, importance);
        }
      }
    });
    stopSelecting();
  }

  /// Moves the ticked Notes of [shown] to the trash in one batch, each
  /// with its Routines and Reminders, without asking, ends the selection
  /// and offers to undo all of them at once at the top (#268).
  void trashSelected(Notes notes, Iterable<PlacedNote> shown) {
    final store = notes.store;
    final book = Routines(store);
    final going = [
      for (final note in selectedOf(shown))
        if (!note.isTrashed && store.mayChange(note.id)) note.id,
    ];
    stopSelecting();
    if (going.isEmpty) return;
    store.batch(() {
      for (final id in going) {
        book.trashNote(id);
      }
    });
    AnnouncementsScope.of(context)?.offerUndo(context.t.undoNoteTrashed, () {
      // The Trash, a purge or a sync may have got to some first.
      store.batch(() {
        for (final id in going) {
          if (notes.byId(id)?.isTrashed != true) continue;
          if (!store.mayChange(id)) continue;
          book.restoreNote(id);
        }
      });
    });
  }

  /// Brings the ticked Notes of [shown] back from the trash in one batch,
  /// each with the Routines and Reminders that went with it, and ends the
  /// selection.
  void restoreSelected(Notes notes, Iterable<PlacedNote> shown) {
    final store = notes.store;
    final book = Routines(store);
    final chosen = selectedOf(shown);
    store.batch(() {
      for (final note in chosen) {
        if (note.isTrashed && store.mayChange(note.id)) {
          book.restoreNote(note.id);
        }
      }
    });
    stopSelecting();
  }
}

/// Importance for the selection: a menu at the button with the three,
/// whose pick sets it on every ticked Note; greyed while [onPicked] is
/// null. It opens a menu of its own, so it never goes into ⋮.
class _ImportanceButton extends StatelessWidget {
  final ValueChanged<Importance>? onPicked;

  const _ImportanceButton({required this.onPicked});

  Future<void> _open(BuildContext button, ValueChanged<Importance> set) async {
    final t = button.t;
    final box = button.findRenderObject() as RenderBox?;
    final overlay = Overlay.of(button).context.findRenderObject() as RenderBox?;
    if (box == null || overlay == null) return;
    final picked = await showMenu<Importance>(
      context: button,
      position: RelativeRect.fromRect(
        box.localToGlobal(Offset.zero, ancestor: overlay) & box.size,
        Offset.zero & overlay.size,
      ),
      items: [
        // The heading the choices continue: "Importance" "Important".
        PopupMenuItem(enabled: false, child: Text(t.importance)),
        for (final (value, label) in [
          (Importance.important, t.importanceImportant),
          (Importance.normal, t.importanceNormal),
          (Importance.unimportant, t.importanceUnimportant),
        ])
          PopupMenuItem(
            key: ValueKey('selection-importance-${value.name}'),
            value: value,
            child: Text(label),
          ),
      ],
    );
    if (picked != null) set(picked);
  }

  @override
  Widget build(BuildContext context) {
    final set = onPicked;
    return Builder(
      builder: (button) => IconButton(
        key: const ValueKey('selection-importance'),
        tooltip: context.t.importance,
        onPressed: set == null ? null : () => _open(button, set),
        icon: const Icon(Icons.label_important_outline),
      ),
    );
  }
}
