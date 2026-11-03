import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';

import 'app_lock.dart';
import 'l10n.dart';
import 'note_editor_screen.dart';
import 'page_bar.dart';
import 'place_names.dart';
import 'system_bars.dart';

/// Opens the History of the Note [noteId] (#193); whether a text was
/// brought back there, so the editor shows it.
Future<bool> openNoteHistory(
  BuildContext context,
  Notes notes,
  Places places,
  String noteId,
) async =>
    await Navigator.of(context).push<bool>(
      MaterialPageRoute<bool>(
        builder: (_) =>
            NoteHistoryScreen(notes: notes, places: places, noteId: noteId),
      ),
    ) ??
    false;

/// A Note's History (#193): every change of the Note, its Amends, the
/// Notes merged into it and its Lines, the newest first, each with what
/// it changed, who made it and when. An old text of the Note or of an
/// Amend comes back with "Use this version", as a new change; nothing is
/// deleted. Only what this Device reads now shows: an Area or Topic in
/// a locked Area that is not open says so instead of its name.
class NoteHistoryScreen extends StatefulWidget {
  final Notes notes;
  final Places places;
  final String noteId;

  const NoteHistoryScreen({
    super.key,
    required this.notes,
    required this.places,
    required this.noteId,
  });

  @override
  State<NoteHistoryScreen> createState() => _NoteHistoryScreenState();
}

class _NoteHistoryScreenState extends State<NoteHistoryScreen> {
  late final _history = NoteHistory(widget.notes.store);
  late List<NoteChange> _changes = _history.of(widget.noteId);

  /// Whether a text was brought back here.
  var _used = false;

  void _use(NoteChange change) {
    // A sync since the page opened may have changed it.
    if (!_history.mayUse(widget.noteId, change)) return;
    _history.use(widget.noteId, change);
    setState(() {
      _used = true;
      _changes = _history.of(widget.noteId);
    });
    ScaffoldMessenger.of(context)
        .showSnackBar(SnackBar(content: Text(context.t.useThisVersionDone)));
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final locale = Localizations.localeOf(context).toString();
    final when = DateFormat.yMMMEd(locale).add_Hm();
    return PopScope(
      canPop: false,
      onPopInvokedWithResult: (didPop, _) {
        if (!didPop) Navigator.of(context).pop(_used);
      },
      child: Scaffold(
        appBar: pageBar(context, title: t.historyTitle),
        body: ListView(
          key: const ValueKey('note-history'),
          padding: clearOfSystemBars(context, const EdgeInsets.all(16)),
          children: [
            Text(t.noteHistoryHint),
            const SizedBox(height: 8),
            for (final (i, change) in _changes.indexed)
              _row(t, when, i, change),
          ],
        ),
      ),
    );
  }

  Widget _row(AppLocalizations t, DateFormat when, int i, NoteChange change) {
    final theme = Theme.of(context);
    final weak = theme.textTheme.bodySmall?.copyWith(
      color: theme.colorScheme.onSurfaceVariant,
    );
    final at = when.format(change.at.toLocal());
    final name = change.by?.name.trim() ?? '';
    final said = name.isEmpty ? at : t.boxTickBy(name, at);
    final (title, before, after) = _said(t, change);
    return Padding(
      key: ValueKey('note-change-$i'),
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(title, style: theme.textTheme.titleSmall),
          Text(said, style: weak),
          if (before != null) ...[
            const SizedBox(height: 4),
            Text(t.noteHistoryBefore, style: weak),
            Text(before),
          ],
          if (after != null) ...[
            const SizedBox(height: 4),
            if (before != null) Text(t.noteHistoryAfter, style: weak),
            Text(after),
          ],
          if (_history.mayUse(widget.noteId, change))
            TextButton(
              key: ValueKey('use-version-$i'),
              onPressed: () => _use(change),
              child: Text(t.useThisVersion),
            ),
          const Divider(),
        ],
      ),
    );
  }

  /// What [change] says: its title and the values before and after it,
  /// in words; null where it has none to show.
  (String, String?, String?) _said(AppLocalizations t, NoteChange change) {
    final (:before, :after) = (before: change.before, after: change.after);
    return switch (change.kind) {
      NoteChangeKind.captured => (t.noteHistoryCaptured, null, after),
      NoteChangeKind.text => (t.noteHistoryText, before, after),
      NoteChangeKind.format => (
        t.badgeTextFormat,
        textFormatName(t, before ?? TextFormats.plain),
        textFormatName(t, after ?? TextFormats.plain),
      ),
      NoteChangeKind.amended => (t.noteHistoryAmended, null, after),
      NoteChangeKind.amendEdited => (t.noteHistoryAmendEdited, before, after),
      NoteChangeKind.mergedIn => (t.noteHistoryMergedIn, null, after),
      NoteChangeKind.lineAdded => (t.noteHistoryLineAdded, null, after),
      NoteChangeKind.lineRenamed => (t.noteHistoryLineRenamed, before, after),
      NoteChangeKind.lineChecked => (t.noteHistoryLineChecked, null, after),
      NoteChangeKind.lineUnchecked => (t.noteHistoryLineUnchecked, null, after),
      NoteChangeKind.lineRemoved => (t.noteHistoryLineRemoved, null, after),
      NoteChangeKind.lineRestored => (t.noteHistoryLineRestored, null, after),
      NoteChangeKind.importance => (
        t.importance,
        _importance(t, before),
        _importance(t, after),
      ),
      NoteChangeKind.filed => (
        t.badgePlace,
        _place(t, before),
        _place(t, after),
      ),
      NoteChangeKind.kind => (
        t.badgeKind,
        kindName(t, _kind(before)),
        kindName(t, _kind(after)),
      ),
      NoteChangeKind.trashed => (t.noteHistoryTrashed, null, null),
      NoteChangeKind.restored => (t.noteHistoryRestored, null, null),
      NoteChangeKind.photoAdded => (t.noteHistoryPhotoAdded, null, null),
      NoteChangeKind.photoRemoved => (t.noteHistoryPhotoRemoved, null, null),
      NoteChangeKind.recordingAdded => (
        t.noteHistoryRecordingAdded,
        null,
        null,
      ),
      NoteChangeKind.recordingRemoved => (
        t.noteHistoryRecordingRemoved,
        null,
        null,
      ),
    };
  }

  static String _importance(AppLocalizations t, String? name) => switch (name) {
    'important' => t.importanceImportant,
    'unimportant' => t.importanceUnimportant,
    _ => t.importanceNormal,
  };

  static NoteKind _kind(String? list) => switch (list) {
    ChecklistKeys.checklist => NoteKind.checklist,
    ShoppingKeys.shopping => NoteKind.shoppingList,
    _ => NoteKind.plain,
  };

  /// The Area or Topic [id] as it is named now: Unsorted for none, and
  /// neither the name of one in a locked Area that is not open nor of
  /// one gone.
  String _place(AppLocalizations t, String? id) {
    if (id == null) return t.unsorted;
    final places = widget.places;
    final topic = places.topicById(id);
    final area = places.areaById(topic?.areaId ?? id);
    if (area == null) return t.noteHistoryPlaceGone;
    if (LockScope.of(context)?.isAreaClosed(area.id) ?? false) {
      return t.noteHistoryLocked;
    }
    return [
      areaName(t, area),
      if (topic != null) topicName(t, topic),
    ].join(' / ');
  }
}
