import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:meta/meta.dart';

import 'conflicts.dart';
import 'entry.dart';
import 'profiles.dart' show Blocks, Overrides, keptFromBlocked;
import 'shared_areas.dart';
import 'signing.dart';
import 'store.dart';
import 'sync.dart';

/// Pending Changes (CONTEXT.md, ADR 0022, #111): changes the user sees
/// as if they were stored, which reach the log only when accepted.
///
/// A pending set lies in a file of its own next to the store
/// (`pending/<id>.json`), so it survives a restart and the decision
/// waits on the next start. It holds two kinds of change:
///
/// - drafts this Device makes ([PendingDraft]): no dseq, no signature;
///   Accept writes them as this Device's entries, numbered and signed
///   only then, so its sequence never has holes;
/// - entries other Devices wrote and signed ([PendingEntry]), checked
///   when they come in and taken in at Accept as any import is.
///
/// A [PendingSet.shown] set is read through: every read of the store
/// sees it on top of the log, but nothing that is passed on, numbered,
/// trusted or deleted does. Decline deletes the file; the log never saw
/// anything.

/// A change in a pending set.
sealed class PendingChange {
  /// The log's mark ([Store.currentSeq]) when the change came: an edit
  /// one of the user's own Devices wrote after it wins at Accept.
  final int mark;

  const PendingChange(this.mark);

  Map<String, Object?> toJson();
}

/// A change this Device makes, unsigned until Accept.
class PendingDraft extends PendingChange {
  final String entity;
  final String field;
  final String? value;

  /// The date the entry is written with; null for [made].
  final DateTime? date;

  /// When the draft was made.
  final DateTime made;

  const PendingDraft({
    required this.entity,
    required this.field,
    required this.value,
    required this.date,
    required this.made,
    required int mark,
  }) : super(mark);

  /// The date the entry carries, in the reads and at Accept.
  DateTime get effectiveDate => date ?? made;

  @override
  Map<String, Object?> toJson() => {
    'date': date?.toUtc().toIso8601String(),
    'entity': entity,
    'field': field,
    'made': made.toUtc().toIso8601String(),
    'mark': mark,
    'value': value,
  };
}

/// An entry another Device wrote and signed, kept as it came.
class PendingEntry extends PendingChange {
  final Entry entry;

  const PendingEntry(this.entry, {required int mark}) : super(mark);

  @override
  Map<String, Object?> toJson() => {'entry': entry.wire, 'mark': mark};
}

/// One pending set: what it is for ([kind]), whether the reads show it
/// ([shown]), the changes in the order they came, the keys of the Devices
/// that signed its entries, and what the ticket that made it applies
/// itself at Accept ([payload], opaque here).
class PendingSet {
  final String id;
  final String kind;
  final bool shown;
  final DateTime made;
  final String? payload;
  final List<KeyRecord> keys;
  final List<PendingChange> changes;

  const PendingSet({
    required this.id,
    required this.kind,
    required this.shown,
    required this.made,
    required this.payload,
    required this.keys,
    required this.changes,
  });

  /// The drafts, in the order they were made.
  List<PendingDraft> get drafts => [
    for (final c in changes)
      if (c is PendingDraft) c,
  ];

  /// The entries other Devices wrote, in the order they came.
  List<PendingEntry> get received => [
    for (final c in changes)
      if (c is PendingEntry) c,
  ];

  PendingSet _with({List<KeyRecord>? keys, List<PendingChange>? changes}) =>
      PendingSet(
        id: id,
        kind: kind,
        shown: shown,
        made: made,
        payload: payload,
        keys: keys ?? this.keys,
        changes: changes ?? this.changes,
      );

  /// The file's text: keys sorted, no spaces, as both cores write it.
  String encode() => jsonEncode({
    'changes': [for (final c in changes) c.toJson()],
    'id': id,
    'keys': [for (final k in keys) k.toJson()],
    'kind': kind,
    'made': made.toUtc().toIso8601String(),
    'payload': payload,
    'shown': shown,
  });

  /// The set [text] holds; null when it is none, or of a later format
  /// (`v` above 1; a missing `v` is 1, ADR 0019).
  static PendingSet? parse(String text) {
    try {
      final json = jsonDecode(text);
      if (json case {
        'changes': final List changes,
        'id': final String id,
        'keys': final List keys,
        'kind': final String kind,
        'made': final String made,
        'shown': final bool shown,
      }) {
        if (json['v'] case final v? when v != 1) return null;
        final payload = json['payload'];
        if (payload != null && payload is! String) return null;
        final records = <KeyRecord>[];
        for (final k in keys) {
          final record = KeyRecord.fromJson(k);
          if (record == null) return null;
          records.add(record);
        }
        final read = <PendingChange>[];
        for (final c in changes) {
          final change = _change(c);
          if (change == null) return null;
          read.add(change);
        }
        return PendingSet(
          id: id,
          kind: kind,
          shown: shown,
          made: DateTime.parse(made).toUtc(),
          payload: payload as String?,
          keys: records,
          changes: read,
        );
      }
    } on FormatException {
      return null;
    } on TypeError {
      return null;
    }
    return null;
  }

  static PendingChange? _change(Object? json) {
    switch (json) {
      case {'entry': final String line, 'mark': final int mark}:
        final decoded = jsonDecode(line);
        if (decoded is! Map) return null;
        return PendingEntry(
          Entry.fromJson(decoded.cast<String, dynamic>(), text: line),
          mark: mark,
        );
      case {
        'entity': final String entity,
        'field': final String field,
        'made': final String made,
        'mark': final int mark,
      }:
        final value = json['value'];
        final date = json['date'];
        if (value != null && value is! String) return null;
        if (date != null && date is! String) return null;
        return PendingDraft(
          entity: entity,
          field: field,
          value: value as String?,
          date: date == null ? null : DateTime.parse(date as String).toUtc(),
          made: DateTime.parse(made).toUtc(),
          mark: mark,
        );
    }
    return null;
  }
}

/// A field where the user's own edit, made after the pending change,
/// was kept at Accept: [pending] is what the set held, [kept] the value
/// that stays.
class KeptEdit {
  final String entity;
  final String field;
  final String? pending;
  final String? kept;

  const KeptEdit(this.entity, this.field, {this.pending, this.kept});

  @override
  bool operator ==(Object other) =>
      other is KeptEdit &&
      other.entity == entity &&
      other.field == field &&
      other.pending == pending &&
      other.kept == kept;

  @override
  int get hashCode => Object.hash(entity, field, pending, kept);

  @override
  String toString() => 'KeptEdit($entity $field: $pending -> $kept)';
}

/// What Accept wrote: the received entries taken in ([taken]), the
/// entries this Device wrote ([written]: the drafts, and the user's own
/// values written again over what was received), and the fields where
/// the user's own edit stayed ([kept]).
class AcceptedChanges {
  final List<Entry> taken;
  final List<Entry> written;
  final List<KeptEdit> kept;

  const AcceptedChanges({
    required this.taken,
    required this.written,
    required this.kept,
  });
}

/// Where the pending sets' files lie: a directory next to the database,
/// or memory for in-memory stores.
abstract interface class PendingFiles {
  /// Every file's text, by set id.
  Map<String, String> readAll();

  /// The text of the file of set [id]; null when there is none.
  String? read(String id);

  /// Writes the file of set [id]; a kill midway leaves the old one.
  void write(String id, String text);

  /// Deletes the file of set [id], if there is one.
  void remove(String id);
}

/// A set id, which names its file: nothing that could leave the
/// directory.
final _setId = RegExp(r'^[0-9a-z-]{1,64}$');

/// [PendingFiles] as `<id>.json` files in [dir], made when the first set
/// is.
class FilePendingFiles implements PendingFiles {
  final Directory dir;

  FilePendingFiles(String path) : dir = Directory(path);

  @override
  Map<String, String> readAll() {
    if (!dir.existsSync()) return {};
    final out = <String, String>{};
    for (final file in dir.listSync().whereType<File>()) {
      final name = file.uri.pathSegments.last;
      if (!name.endsWith('.json')) continue;
      final id = name.substring(0, name.length - '.json'.length);
      if (!_setId.hasMatch(id)) continue;
      try {
        out[id] = file.readAsStringSync();
      } on FileSystemException {
        continue;
      }
    }
    return out;
  }

  @override
  String? read(String id) {
    if (!_setId.hasMatch(id)) return null;
    final file = File('${dir.path}/$id.json');
    try {
      return file.existsSync() ? file.readAsStringSync() : null;
    } on FileSystemException {
      return null;
    }
  }

  @override
  void write(String id, String text) {
    dir.createSync(recursive: true);
    final file = File('${dir.path}/$id.json');
    // Written beside, then renamed, as a blob is.
    final tmp = File('${file.path}.tmp');
    tmp.writeAsStringSync(text, flush: true);
    tmp.renameSync(file.path);
  }

  @override
  void remove(String id) {
    final file = File('${dir.path}/$id.json');
    if (file.existsSync()) file.deleteSync();
  }
}

/// [PendingFiles] in memory: gone with the store. For tests.
class MemoryPendingFiles implements PendingFiles {
  final _held = <String, String>{};

  @override
  Map<String, String> readAll() => Map.of(_held);

  @override
  String? read(String id) => _held[id];

  @override
  void write(String id, String text) => _held[id] = text;

  @override
  void remove(String id) => _held.remove(id);
}

/// The fields [field] stands or falls with: its whole schedule for a
/// Routine's, else itself.
List<String> _unitFields(String field) => switch (conflictUnit(field)) {
  final unit? => conflictUnitFields(unit),
  null => [field],
};

/// Opening, filling, accepting and declining pending sets.
extension PendingChanges on Store {
  /// Opens an empty pending set for [kind] (what it is for, such as
  /// `ai-task` or `area-offer`) and returns its id. A [shown] set is read
  /// through at once; any other only through [pendingSet]. [payload] is
  /// kept as it is for the caller to apply at Accept.
  String openPending(String kind, {required bool shown, String? payload}) {
    final id = Store.newId();
    if (!_setId.hasMatch(id)) {
      throw StateError('a set id names a file: $id');
    }
    keepPendingSet(
      PendingSet(
        id: id,
        kind: kind,
        shown: shown,
        made: clock().toUtc(),
        payload: payload,
        keys: const [],
        changes: const [],
      ),
    );
    return id;
  }

  /// The pending set [id] as its file holds it now; null when there is
  /// none.
  PendingSet? pendingSet(String id) => freshPendingSet(id);

  /// Every pending set as the files hold them now, the oldest first:
  /// those a restart left too.
  List<PendingSet> get pendingSets => freshPendingSets();

  /// The set [id] as its file holds it now: another store open on the
  /// same file, the phone's background sync, may have changed or ended
  /// it.
  PendingSet _set(String id) =>
      freshPendingSet(id) ?? (throw StateError('no pending set $id'));

  /// Adds the change of [field] on [entity] to [value] to the set [id],
  /// as this Device would write it: dated [date], or when it is made.
  void draftPending(
    String id,
    String entity,
    String field,
    String? value, {
    DateTime? date,
  }) {
    final set = _set(id);
    keepPendingSet(
      set._with(
        changes: [
          ...set.changes,
          PendingDraft(
            entity: entity,
            field: field,
            value: value,
            date: date?.toUtc(),
            made: clock().toUtc(),
            mark: currentSeq(),
          ),
        ],
      ),
    );
  }

  /// Adds the entries other Devices wrote to the set [id], with the
  /// [keys] they came with, and returns those taken. Each is checked as
  /// an import checks it, but nothing is pinned or stored: one already
  /// held or in the set is skipped; one whose signature does not verify
  /// under a key held here or among the set's, or which claims this
  /// Device, is refused and counted in [report]; one a blocked person's
  /// Device wrote that an import would not keep is refused unseen
  /// (ADR 0021), her Devices judged by the log and the set together.
  List<Entry> receivePending(
    String id,
    List<Entry> entries, {
    Iterable<KeyRecord> keys = const [],
    ImportReport? report,
  }) {
    final set = _set(id);
    final self = deviceId;
    final records = {for (final k in set.keys) k.device: k};
    for (final k in keys) {
      if (k.device != self && k.valid) records.putIfAbsent(k.device, () => k);
    }
    final inSet = {for (final r in set.received) r.entry.id};
    final checked = <Entry>[];
    for (final e in entries) {
      if (inSet.contains(e.id) || holds(e.device, e.dseq)) continue;
      final key = e.device == self
          ? null
          : pinnedKey(e.device)?.record.publicKey ??
                records[e.device]?.publicKey;
      if (key == null || !verifySignature(key, _bytes(e), e.sig)) {
        report?.refuse(e.author, e.device);
        continue;
      }
      inSet.add(e.id);
      checked.add(e);
    }
    final blocked = _blockedFor([
      for (final r in set.received) r.entry,
      ...checked,
    ]);
    final taken = [
      for (final e in checked)
        if (!blocked.contains(e.device) || keptFromBlocked(e)) e,
    ];
    final used = {for (final e in taken) e.device};
    final mark = currentSeq();
    keepPendingSet(
      set._with(
        keys: [
          for (final k in records.values)
            if (used.contains(k.device) ||
                set.keys.any((h) => h.device == k.device))
              k,
        ]..sort((a, b) => a.device.compareTo(b.device)),
        changes: [
          ...set.changes,
          for (final e in taken) PendingEntry(e, mark: mark),
        ],
      ),
    );
    return taken;
  }

  /// The Devices of blocked people, by the grants of the log and of
  /// [entries] together: a set may bring the grant that names her.
  Set<String> _blockedFor(List<Entry> entries) {
    if (blockedIds.isEmpty) return const {};
    if (!entries.any((e) => e.field.startsWith(ShareKeys.rolePrefix))) {
      return blockedDevices();
    }
    return blockedDevices(
      judgeLog([
        for (final e in allEntries())
          if (e.applies) e,
        for (final e in entries)
          if (e.applies) e,
      ]),
    );
  }

  /// Takes out of every pending set what a blocked person's Device wrote
  /// that an import would not keep (ADR 0021); returns whether anything
  /// went.
  bool dropBlockedFromPending() {
    if (blockedIds.isEmpty) return false;
    var dropped = false;
    for (final set in freshPendingSets()) {
      final received = [for (final r in set.received) r.entry];
      if (received.isEmpty) continue;
      final blocked = _blockedFor(received);
      final left = [
        for (final c in set.changes)
          if (c is! PendingEntry ||
              !blocked.contains(c.entry.device) ||
              keptFromBlocked(c.entry))
            c,
      ];
      if (left.length == set.changes.length) continue;
      keepPendingSet(set._with(changes: left));
      dropped = true;
    }
    return dropped;
  }

  /// Accepts the set [id]: its received entries are taken in as any
  /// import is ([Sync.applyEntries]), then its drafts are written as
  /// this Device's entries, numbered and signed now, each dated as it was
  /// drafted. Where one of the user's own Devices edited a field after
  /// the change came (its whole schedule, for a Routine's), the user's
  /// edit stays: the draft is not written, and a value received over it
  /// is written over again with the user's, which ends the Conflict too.
  /// The set is gone after; what the ticket that opened it keeps in its
  /// [PendingSet.payload] is its own to apply. Throws a [StateError] for
  /// a set that is not here.
  AcceptedChanges acceptPending(String id) {
    final set = _set(id);
    final upTo = currentSeq();
    final received = set.received;
    final ids = {for (final r in received) r.entry.id};
    final taken = received.isEmpty
        ? <Entry>[]
        : applyEntries([for (final r in received) r.entry], keys: set.keys);
    final own = <String, bool>{};
    bool editedByOwn(String entity, String field, int mark) =>
        givesWay(entity, field, mark, upTo: upTo, ids: ids, own: own);
    final kept = <(String, String), KeptEdit>{};
    final written = <Entry>[];
    batch(() {
      // What was received gives way to the user's own edits.
      final restored = <(String, String)>{};
      for (final r in received) {
        final e = r.entry;
        if (!e.applies || !holds(e.device, e.dseq)) continue;
        final fields = _unitFields(e.field);
        if (!editedByOwn(e.entity, e.field, r.mark)) continue;
        final history = loggedHistory(e.entity);
        for (final field in fields) {
          if (!restored.add((e.entity, field))) continue;
          final before = _latest(history, field, (x) => !ids.contains(x.id));
          final theirs = _latest(history, field, (x) => ids.contains(x.id));
          if (theirs == null || theirs.value == before?.value) continue;
          kept[(e.entity, field)] = KeptEdit(
            e.entity,
            field,
            pending: theirs.value,
            kept: before?.value,
          );
          // Written again where theirs would show, or stand as a Conflict
          // with the user's edit, which did not see it: the set was not
          // shown then.
          if (logged(e.entity, field) == before?.value &&
              conflictsAmong([theirs]).isEmpty) {
            continue;
          }
          written.add(append(e.entity, field, before?.value));
        }
      }
      // Then the drafts, numbered and signed now.
      for (final d in set.drafts) {
        if (editedByOwn(d.entity, d.field, d.mark)) {
          kept[(d.entity, d.field)] = KeptEdit(
            d.entity,
            d.field,
            pending: d.value,
            kept: logged(d.entity, d.field),
          );
          continue;
        }
        written.add(append(d.entity, d.field, d.value, date: d.effectiveDate));
      }
      // In the same transaction: a restart before the file is gone
      // finds the set accepted, not waiting.
      setLocalSetting('${Store.acceptedPendingPrefix}$id', 'accepted');
    });
    forgetPendingSet(id);
    return AcceptedChanges(
      taken: taken,
      written: written,
      kept: kept.values.toList(),
    );
  }

  /// Declines the set [id]: its file is deleted and the log never saw
  /// anything of it. Returns whether there was one.
  bool declinePending(String id) {
    if (freshPendingSet(id) == null) return false;
    forgetPendingSet(id);
    return true;
  }

  /// Whether one of the user's own Devices wrote [field] on [entity]
  /// after [mark], up to [upTo], with no entry of [ids]: an edit of the
  /// user's, made after the pending change, that the change gives way to
  /// in the reads and at Accept. A Routine's schedule counts as one
  /// field. [own] and [histories] remember across calls.
  @internal
  bool givesWay(
    String entity,
    String field,
    int mark, {
    int? upTo,
    Set<String> ids = const {},
    Map<String, bool>? own,
    Map<String, List<Entry>>? histories,
  }) {
    final fields = _unitFields(field);
    final known = own ?? <String, bool>{};
    final history = histories == null
        ? loggedHistory(entity)
        : histories.putIfAbsent(entity, () => loggedHistory(entity));
    return history.any(
      (e) =>
          e.seq > mark &&
          (upTo == null || e.seq <= upTo) &&
          fields.contains(e.field) &&
          !ids.contains(e.id) &&
          known.putIfAbsent(e.device, () => writtenByOwn(e.device)),
    );
  }

  /// The entry of [field] in [history] that wins among those [where]
  /// takes.
  static Entry? _latest(
    List<Entry> history,
    String field,
    bool Function(Entry) where,
  ) {
    Entry? held;
    for (final e in history) {
      if (e.field != field || !where(e)) continue;
      if (held == null || Store.latestWins(e, held) > 0) held = e;
    }
    return held;
  }

  static Uint8List _bytes(Entry e) => entryBytes(
    device: e.device,
    dseq: e.dseq,
    entity: e.entity,
    field: e.field,
    value: e.value,
    dateIso: e.date.toUtc().toIso8601String(),
    author: e.author,
    recordedIso: e.recorded.toUtc().toIso8601String(),
    ctx: e.ctx,
    extra: e.extra,
  );
}
