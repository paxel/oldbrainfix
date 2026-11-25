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
/// each ticked, with its type and its Reminder or Routine, "Tick all"
/// and "Untick all" on the bar, the records not taken greyed under
/// them, where they go, and "Take it", which takes the ticked ones there
/// and closes the page.
class AxtOpenScreen extends StatefulWidget {
  final Store store;
  final AxtFile file;

  const AxtOpenScreen({super.key, required this.store, required this.file});

  @override
  State<AxtOpenScreen> createState() => _AxtOpenScreenState();
}

class _AxtOpenScreenState extends State<AxtOpenScreen> {
  late final List<AxtRecord> _records = widget.file.taken;
  late final List<AxtRecord> _notTaken = widget.file.notTaken;
  late Set<String> _ticks = {for (final r in _records) r.id};

  /// The Area or Topic picked; null for Unsorted.
  String? _place;

  /// What the button shows for [_place], worked out as it is picked.
  String? _placeName;

  /// The records ticked, in the file's order.
  List<String> get _ticked => [
    for (final r in _records)
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
      (r) => _ticks.contains(r.id) && r.draft?.schedule != null,
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
    void refused(String details) => announceFailureWith(
      announcements,
      root,
      t.axtNotTaken,
      title: t.axtNotTakenTitle,
      cause: t.axtPlaceRefused,
      fix: t.axtPickAnotherFix,
      details: details,
    );
    final List<String> made;
    try {
      made = AxtFiles(widget.store).take(widget.file, _ticked, place: _place);
    } on Exception catch (e) {
      refused('$e');
      return;
    } on Error catch (e) {
      // The core refuses a place with an ArgumentError or a StateError,
      // before anything is written.
      refused('$e');
      return;
    }
    announcements?.announce(t.axtTaken(made.length));
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
        actions: _records.isEmpty
            ? const []
            : packTickActions(
                t,
                onAll: () =>
                    setState(() => _ticks = {for (final r in _records) r.id}),
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
              value: _ticks.contains(record.id),
              onChanged: (on) => _tick(record.id, on ?? false),
              title: Text(
                _title(record),
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
              ),
          ],
          if (_records.isNotEmpty) ...[
            const Divider(height: 32),
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

  /// The first line of [record]'s text as it reads.
  static String _title(AxtRecord record) {
    final draft = record.draft;
    if (draft == null) return record.id;
    final shown = TextFormats.visible(draft.format, draft.text);
    for (final line in shown.split('\n')) {
      if (line.trim().isNotEmpty) return line.trim();
    }
    return shown.trim();
  }

  /// [record]'s type and, for a Task or Appointment with a date, its
  /// Reminder or Routine.
  String _detail(AppLocalizations t, String locale, AxtRecord record) {
    final type = _typeName(record.type);
    final schedule = record.draft?.schedule;
    if (schedule == null) return type;
    final when = scheduleText(t, locale, schedule);
    return '$type · ${schedule.once ? t.packReminder(when) : t.packRoutine(when)}';
  }
}
