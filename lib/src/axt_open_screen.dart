import 'dart:async';
import 'dart:io';
import 'dart:isolate';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';

import 'announcements.dart';
import 'app_lock.dart' show LockScope;
import 'failure_dialog.dart';
import 'folder_platform.dart';
import 'import_run.dart';
import 'l10n.dart';
import 'look.dart' show ControlBelow;
import 'pack_screen.dart' show packTickActions;
import 'page_bar.dart';
import 'place_names.dart';
import 'routine_editor_screen.dart' show deviceWallClock;
import 'schedule_text.dart';
import 'system_bars.dart';

/// AXT files opened (obf-1146, ADR 0056): a file another app wrote,
/// opened with the app, shared to it or picked, shows its records on a
/// page as an opened Pack does, all ticked, the records of a type not
/// taken named and greyed, who it is from, and where they go, Unsorted
/// at first; "Take it" makes the ticked ones Notes there.

/// Whether [head], a file's first bytes, opens an Avro object container,
/// as every AXT file does (AXT `spec/envelope.md`, section 9): a file is
/// told by what it holds, not by its name or type.
bool startsLikeAxt(List<int> head) =>
    head.length >= 4 &&
    head[0] == 0x4F &&
    head[1] == 0x62 &&
    head[2] == 0x6A &&
    head[3] == 0x01;

/// Whether the file at [path] starts as an AXT file does, read only as
/// far as its first four bytes.
bool isAxtFile(String path) {
  try {
    final file = File(path).openSync();
    try {
      return startsLikeAxt(file.readSync(4));
    } finally {
      file.closeSync();
    }
  } on FileSystemException {
    return false;
  }
}

/// The bytes of the file at [path] when it starts as an AXT file does,
/// read whole; null for any other file. A file no larger than a Sent
/// Item is read here, as a Pack's is; a larger one off the main isolate.
Future<Uint8List?> axtFileBytes(String path) async {
  if (!isAxtFile(path)) return null;
  try {
    if (File(path).lengthSync() > maxSentItemBytes) {
      return await Isolate.run(() => _readAll(path));
    }
  } on FileSystemException {
    return null;
  }
  return _readAll(path);
}

Uint8List? _readAll(String path) {
  try {
    return File(path).readAsBytesSync();
  } on FileSystemException {
    return null;
  }
}

/// The job that reads the AXT file [bytes] on the store it is given. At
/// the top level, so the closure that crosses into the job's isolate
/// holds only the bytes.
@visibleForTesting
LongWork<AxtFile> axtReadJob(Uint8List bytes) =>
    (store, _, _, _) => AxtFiles(store).read(bytes, wallClock: deviceWallClock);

/// Opens the AXT file [bytes]: read as a long job is ([JobRunner]), in an
/// isolate of its own on the phone, then its page; why it did not open
/// behind the tap at the top.
Future<void> openAxtBytes(
  BuildContext context,
  Store store,
  Uint8List bytes,
) async {
  final t = context.t;
  final announcements = AnnouncementsScope.of(context);
  final navigator = Navigator.of(context);
  final root = Navigator.of(context, rootNavigator: true);
  final runner = ImportJobsScope.of(context)?.runner;
  void notOpened(String details) => announceFailureWith(
    announcements,
    root,
    t.axtNotOpened,
    title: t.axtNotOpenedTitle,
    cause: t.axtUnreadable,
    fix: t.axtSendAgainFix,
    details: details,
  );
  final job = axtReadJob(bytes);
  final stop = JobStop();
  final AxtFile file;
  try {
    file = runner == null
        ? await job(store, const NoFolders(), (_, _) {}, () => false)
        : await runner.run(store, job, (_, _) {}, stop);
  } on Exception catch (e) {
    notOpened('$e');
    return;
  } on Error catch (e) {
    // What the AXT library refuses as an Error is told as any failure.
    notOpened('$e');
    return;
  } finally {
    stop.clear();
  }
  await navigator.push(
    MaterialPageRoute<void>(
      builder: (_) => AxtOpenScreen(store: store, file: file),
    ),
  );
}

/// The page of an AXT file opened: who it is from, the records to take,
/// each ticked, with its type and its Reminder or Routine, and the
/// updates and deletions of Notes taken from the same sender before
/// (obf-1147), each with the Note it changes, ticked, greyed with "Kept
/// your change" where the user changed that Note since; "Tick all" and
/// "Untick all" on the bar, the records not taken greyed under them,
/// refused and stale ones with the Note they leave alone, where new ones
/// go, and "Take it", which takes the ticked ones and closes the page.
class AxtOpenScreen extends StatefulWidget {
  final Store store;
  final AxtFile file;

  const AxtOpenScreen({super.key, required this.store, required this.file});

  @override
  State<AxtOpenScreen> createState() => _AxtOpenScreenState();
}

class _AxtOpenScreenState extends State<AxtOpenScreen> {
  /// The new records and the changes, in the file's order.
  late final List<AxtRecord> _records;
  late final List<AxtRecord> _notTaken;

  /// What can be ticked: all but the changes the user's own change keeps.
  late final List<AxtRecord> _tickable;
  late Set<String> _ticks;

  /// Whether the file holds a new record, which goes to the place.
  late final bool _anyNew;

  /// The first line of each Note a record changes or leaves alone, read
  /// once as the page opens.
  late final Map<String, String> _noteNames;

  @override
  void initState() {
    super.initState();
    final file = widget.file;
    _records = [
      for (final r in file.records)
        if (r.taken || r.changes) r,
    ];
    _notTaken = file.notTaken;
    _tickable = [
      for (final r in _records)
        if (!r.kept) r,
    ];
    _ticks = {for (final r in _tickable) r.id};
    _anyNew = _records.any((r) => r.taken);
    final notes = Notes(widget.store);
    _noteNames = {
      for (final r in file.records)
        if (r.noteId case final id?)
          if (notes.byId(id) case final note?) id: _firstLine(note.visibleText),
    };
  }

  /// The Area or Topic picked; null for Unsorted.
  String? _place;

  /// What the button shows for [_place], worked out as it is picked.
  String? _placeName;

  /// The records ticked, in the file's order.
  List<String> get _ticked => [
    for (final r in _tickable)
      if (_ticks.contains(r.id)) r.id,
  ];

  void _tick(String id, bool on) => setState(() {
    _ticks = {..._ticks};
    on ? _ticks.add(id) : _ticks.remove(id);
  });

  /// The places the ticked records may go, as File into… lists them
  /// (#272): Unsorted first, then the Areas in the person's own order,
  /// each with its Topics by name, where this Device may write; a closed
  /// locked Area without its Topics. While a ticked record brings a
  /// Reminder or Routine, only where this Device may make them.
  List<PopupMenuEntry<String>> _places(Map<String, String> labels) {
    final t = context.t;
    final store = widget.store;
    final places = Places(store);
    final lock = LockScope.of(context);
    final schedules = _records.any(
      (r) => r.taken && _ticks.contains(r.id) && r.draft?.schedule != null,
    );
    PopupMenuEntry<String> entry(
      String id,
      String name, {
      int depth = 0,
      String? label,
    }) {
      labels[id] = label ?? name;
      return CheckedPopupMenuItem<String>(
        key: ValueKey('axt-place-$id'),
        value: id,
        checked: (_place ?? '') == id,
        child: Padding(
          padding: EdgeInsets.only(left: 16.0 * depth),
          child: Text(name),
        ),
      );
    }

    return [
      entry('', t.unsorted),
      for (final area in store.areasInOrder(shown: (a) => areaName(t, a)))
        if (store.mayWriteIn(area.id) &&
            (!schedules || store.mayManage(area.id))) ...[
          entry(area.id, areaName(t, area)),
          if (!(lock?.isAreaClosed(area.id) ?? false))
            for (final topic
                in places.topics(area.id)..sort(
                  (a, b) => topicName(
                    t,
                    a,
                  ).toLowerCase().compareTo(topicName(t, b).toLowerCase()),
                ))
              entry(
                topic.id,
                topicName(t, topic),
                depth: 1,
                label: '${areaName(t, area)} ▸ ${topicName(t, topic)}',
              ),
        ],
    ];
  }

  /// Opens the places at the button [button] (UI laws: a list to choose
  /// from is a menu at the finger).
  Future<void> _pick(BuildContext button) async {
    final box = button.findRenderObject() as RenderBox?;
    final overlay = Overlay.of(button).context.findRenderObject() as RenderBox?;
    if (box == null || overlay == null) return;
    final labels = <String, String>{};
    final items = _places(labels);
    final picked = await showMenu<String>(
      context: button,
      position: RelativeRect.fromRect(
        box.localToGlobal(Offset.zero, ancestor: overlay) & box.size,
        Offset.zero & overlay.size,
      ),
      items: items,
    );
    if (!mounted || picked == null) return;
    setState(() {
      _place = picked.isEmpty ? null : picked;
      _placeName = _place == null ? null : labels[picked];
    });
  }

  void _take() {
    final t = context.t;
    final announcements = AnnouncementsScope.of(context);
    final root = Navigator.of(context, rootNavigator: true);
    final ticked = _ticked;
    final changes = _records.any((r) => r.changes && ticked.contains(r.id));
    void refused(String details) => announceFailureWith(
      announcements,
      root,
      t.axtNotTaken,
      title: t.axtNotTakenTitle,
      cause: changes ? t.axtChangeRefused : t.axtPlaceRefused,
      fix: t.axtPickAnotherFix,
      details: details,
    );
    // Taking judges each record again by the store as it is now (ADR
    // 0056): the same file taken twice writes nothing the second time.
    final store = widget.store;
    var wrote = false;
    void heard() => wrote = true;
    store.addListener(heard);
    final List<String> made;
    try {
      made = AxtFiles(store).take(widget.file, ticked, place: _place);
    } on Exception catch (e) {
      refused('$e');
      return;
    } on Error catch (e) {
      // The core refuses a place, or a Note this Device may not change,
      // with an ArgumentError or a StateError, before anything is written.
      refused('$e');
      return;
    } finally {
      store.removeListener(heard);
    }
    announcements?.announce(
      !wrote
          ? t.axtNothingChanged
          : changes
          ? t.axtRecordsTaken(ticked.length)
          : t.axtTaken(made.length),
    );
    Navigator.of(context).pop();
  }

  String _typeName(String? type) {
    final t = context.t;
    return switch (type) {
      AxtTypes.note => t.axtTypeNote,
      AxtTypes.task => t.axtTypeTask,
      AxtTypes.appointment => t.axtTypeAppointment,
      final other? => other,
      null => t.axtNoType,
    };
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final locale = Localizations.localeOf(context).toString();
    final text = Theme.of(context).textTheme;
    final file = widget.file;
    final any = _ticks.isNotEmpty;
    final greyed = Theme.of(context).disabledColor;
    return Scaffold(
      appBar: pageBar(
        context,
        title: t.axtTitle,
        actions: _tickable.isEmpty
            ? const []
            : packTickActions(
                t,
                onAll: () =>
                    setState(() => _ticks = {for (final r in _tickable) r.id}),
                onNone: () => setState(() => _ticks = {}),
              ),
      ),
      body: ListView(
        padding: clearOfSystemBars(context, const EdgeInsets.all(16)),
        children: [
          Text(
            file.fromOwnDevice
                ? t.axtFromOwnDevice
                : switch (file.appName) {
                    final name? when name.trim().isNotEmpty => t.axtFromApp(
                      name.trim(),
                    ),
                    _ => t.axtFromNoName,
                  },
            key: const ValueKey('axt-from'),
            style: text.titleMedium,
          ),
          const SizedBox(height: 4),
          Text(t.axtKey(file.shortFingerprint), key: const ValueKey('axt-key')),
          const Divider(height: 32),
          if (_records.isEmpty) Text(t.axtNothingToTake),
          for (final record in _records)
            CheckboxListTile(
              key: ValueKey('axt-record-${record.id}'),
              contentPadding: EdgeInsets.zero,
              controlAffinity: ListTileControlAffinity.leading,
              value: !record.kept && _ticks.contains(record.id),
              enabled: !record.kept,
              onChanged: record.kept
                  ? null
                  : (on) => _tick(record.id, on ?? false),
              title: Text(
                _title(t, record),
                maxLines: 2,
                overflow: TextOverflow.ellipsis,
              ),
              subtitle: Text(_detail(t, locale, record)),
            ),
          if (_notTaken.isNotEmpty) ...[
            const SizedBox(height: 16),
            Text(
              t.axtNotTakenHeading,
              style: text.titleSmall?.copyWith(color: greyed),
            ),
            for (final record in _notTaken)
              CheckboxListTile(
                key: ValueKey('axt-not-taken-${record.id}'),
                contentPadding: EdgeInsets.zero,
                controlAffinity: ListTileControlAffinity.leading,
                enabled: false,
                value: false,
                onChanged: null,
                title: Text(_typeName(record.type)),
                subtitle: switch (_leftAlone(t, record)) {
                  final why? => Text(why),
                  null => null,
                },
              ),
          ],
          if (_records.isNotEmpty) const Divider(height: 32),
          if (_anyNew) ...[
            ControlBelow(
              padding: EdgeInsets.zero,
              title: Text(t.axtInto),
              control: Builder(
                builder: (button) => OutlinedButton.icon(
                  key: const ValueKey('axt-place'),
                  onPressed: () => unawaited(_pick(button)),
                  icon: const Icon(Icons.arrow_drop_down),
                  label: Text(_placeName ?? t.unsorted),
                ),
              ),
            ),
            const SizedBox(height: 16),
          ],
          if (_tickable.isNotEmpty) ...[
            FilledButton(
              key: const ValueKey('axt-take'),
              onPressed: any ? _take : null,
              child: Text(t.axtTake),
            ),
            if (!any) ...[const SizedBox(height: 8), Text(t.packNothingTicked)],
          ],
        ],
      ),
    );
  }

  /// The first line of [record]'s text as it reads; of a deletion, the
  /// Note's it moves to the trash.
  String _title(AppLocalizations t, AxtRecord record) {
    final draft = record.draft;
    if (draft == null) return _noteName(t, record);
    return _firstLine(TextFormats.visible(draft.format, draft.text));
  }

  /// The first line of the Note [record] changes or leaves alone, or its
  /// type's name where it has none.
  String _noteName(AppLocalizations t, AxtRecord record) =>
      switch (_noteNames[record.noteId]) {
        final name? when name.isNotEmpty => name,
        _ => _typeName(record.type),
      };

  /// Why a record not taken leaves its Note alone: refused, or no newer
  /// than what the Note was taken from (obf-1147); null for the others.
  String? _leftAlone(AppLocalizations t, AxtRecord record) =>
      record.noteId == null
      ? null
      : switch (record.kind) {
          AxtRecordKind.refused => t.axtRefusedRecord(_noteName(t, record)),
          AxtRecordKind.stale => t.axtStaleRecord(_noteName(t, record)),
          _ => null,
        };

  /// The first line of [text] with words in it.
  static String _firstLine(String text) {
    for (final line in text.split('\n')) {
      if (line.trim().isNotEmpty) return line.trim();
    }
    return text.trim();
  }

  /// [record]'s type and, for a Task or Appointment with a date, its
  /// Reminder or Routine.
  String _detail(AppLocalizations t, String locale, AxtRecord record) {
    final change = switch (record.kind) {
      AxtRecordKind.update => t.axtUpdates(_noteName(t, record)),
      AxtRecordKind.deletion => t.axtDeletes(_noteName(t, record)),
      _ => null,
    };
    if (change != null) {
      return record.kept ? '$change · ${t.axtKeptChange}' : change;
    }
    final type = _typeName(record.type);
    final schedule = record.draft?.schedule;
    if (schedule == null) return type;
    final when = scheduleText(t, locale, schedule);
    return '$type · ${schedule.once ? t.packReminder(when) : t.packRoutine(when)}';
  }
}
