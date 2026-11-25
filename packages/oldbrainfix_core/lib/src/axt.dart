/// AXT files (obf-1144, ADR 0056): Notes, with their Reminders and
/// Routines, as records of the open standard AXT for small apps to
/// exchange typed records, written and read through the library
/// `axt_protocol`. A Note goes as an AXT `Note`, a Note with a Reminder
/// or Routine as a `Task`; a `Note`, a `Task` and an `Appointment` that
/// come in become Notes, every other type is listed as not taken. The
/// desktop core reads and writes the same records (`axt.rs`); the file's
/// bytes differ, as the library draws its sync marker and the low bits
/// of each version at random.
library;

import 'dart:convert';
import 'dart:math';
import 'dart:typed_data';

import 'package:axt_protocol/axt_protocol.dart';
import 'package:axt_protocol/examples.dart' as ax;
import 'package:crypto/crypto.dart';

import 'axt_types.dart';
import 'entry.dart';
import 'notes.dart';
import 'places.dart';
import 'profiles.dart';
import 'routines.dart';
import 'schedule.dart';
import 'shared_areas.dart';
import 'signing.dart';
import 'store.dart';
import 'text_format.dart';
import 'text_tags.dart';

/// The full names of the AXT types Old Brain Fix writes and takes.
abstract final class AxtTypes {
  static const note = 'axt.example.v1.Note';
  static const task = 'axt.example.v1.Task';
  static const appointment = 'axt.example.v1.Appointment';

  /// The types a file's records are taken from.
  static const taken = {note, task, appointment};
}

abstract final class AxtKeys {
  /// On a Note taken from an AXT file: `{"id":…,"sender":…,"type":…,
  /// "version":…}`, the record's id as its sender wrote it, the sender's
  /// key as hexadecimal text, the record's type and the version taken
  /// ([AxtFrom]). Only what the user's own Devices wrote counts.
  static const from = 'axtFrom';
}

/// A file that is no AXT file this app can read: no Avro container, a
/// signature that does not hold, a header of another format.
class NotAnAxtFile implements Exception {
  const NotAnAxtFile();

  @override
  String toString() => 'NotAnAxtFile';
}

/// Where a Note taken from an AXT file came from: the record [id] as its
/// [sender] wrote it, the sender's key as hexadecimal text, the record's
/// [type] and the [version] taken.
class AxtFrom {
  final String sender;
  final String id;
  final String type;
  final int version;

  const AxtFrom({
    required this.sender,
    required this.id,
    required this.type,
    required this.version,
  });

  /// Keys sorted, no spaces, the same bytes from both cores.
  String encode() => jsonEncode({
    'id': id,
    'sender': sender,
    'type': type,
    'version': version,
  });

  /// What [raw] holds, or null when it is none.
  static AxtFrom? parse(String? raw) {
    if (raw == null) return null;
    try {
      if (jsonDecode(raw)
          case {
            'id': final String id,
            'sender': final String sender,
            'type': final String type,
            'version': final int version,
          }
          when _key.hasMatch(sender)) {
        return AxtFrom(sender: sender, id: id, type: type, version: version);
      }
    } on FormatException {
      return null;
    }
    return null;
  }

  static final _key = RegExp(r'^[0-9a-f]{64}$');
}

/// A Note as a record of an AXT file makes it, before it is taken: its
/// [text] in [format], and for a `Task` with a due date or an
/// `Appointment` its Reminder or Routine, the first Occurrence Done or
/// not, and the Heads-ups of an `Appointment`.
class AxtNoteDraft {
  final String text;
  final String format;
  final Schedule? schedule;

  /// Whether the first Occurrence of [schedule] is Done.
  final bool done;

  /// When it was checked off, or null when not known: it is then taken
  /// as checked off when it is taken.
  final DateTime? doneAt;

  /// Minutes before each Occurrence, ascending ([HeadsUpKeys]).
  final List<int> headsUps;

  const AxtNoteDraft({
    required this.text,
    required this.format,
    this.schedule,
    this.done = false,
    this.doneAt,
    this.headsUps = const [],
  });
}

/// What a record of a file is to the Notes taken from that sender before
/// (obf-1147), by the record id and the sender's key their [AxtFrom]
/// names, as the user's own Devices wrote it.
enum AxtRecordKind {
  /// A record no Note was taken from: a `Note`, `Task` or `Appointment`
  /// that came in whole, to become a new Note.
  newRecord('new'),

  /// A later version of a record a Note was taken from: it rewrites the
  /// Note's text and its Reminder or Routine.
  update('update'),

  /// A later deletion of such a record: it moves the Note
  /// to the trash.
  deletion('deletion'),

  /// A record that may not change the Note taken under its id: one of
  /// another sender, one of another type than the Note was taken as, or
  /// one the library refused.
  refused('refused'),

  /// A version of the record no later than the one taken: nothing to do.
  stale('stale'),

  /// Nothing to take: a record of a type not taken, one the library
  /// refused, or a deletion of a record no Note was taken from.
  none('none');

  /// How the fixtures and the desktop core name it.
  final String code;

  const AxtRecordKind(this.code);
}

/// One record of an AXT file, as its last message the library applied
/// leaves it: its [id] as its sender wrote it, its [version], its [type]
/// when it came in whole, and [result], what the library made of that
/// message (`APPLIED`, or why not, such as `VALUE_INVALID`). A record of a
/// type this app takes has its [draft]; a deletion has none. [kind] says
/// what it is to the Notes taken before, [noteId] which Note it updates,
/// deletes or may not change, and [kept] whether the user changed that
/// Note since it was taken or last updated, so that nothing changes.
class AxtRecord {
  final String id;
  final int version;
  final String? type;
  final String result;
  final AxtNoteDraft? draft;
  final AxtRecordKind kind;
  final String? noteId;
  final bool kept;

  const AxtRecord({
    required this.id,
    required this.version,
    required this.type,
    required this.result,
    this.draft,
    this.kind = AxtRecordKind.none,
    this.noteId,
    this.kept = false,
  });

  /// Whether it becomes a new Note when taken: a `Note`, `Task` or
  /// `Appointment` that came in whole, under an id no Note was taken
  /// from.
  bool get taken => kind == AxtRecordKind.newRecord;

  /// Whether taking it updates or deletes the Note it was taken as,
  /// unless the user changed that Note since ([kept]).
  bool get changes =>
      kind == AxtRecordKind.update || kind == AxtRecordKind.deletion;

  AxtRecord _as(AxtRecordKind kind, {String? noteId, bool kept = false}) =>
      AxtRecord(
        id: id,
        version: version,
        type: type,
        result: result,
        draft: draft,
        kind: kind,
        noteId: noteId,
        kept: kept,
      );
}

/// A Note taken from a record: its id, where it came from, and the entry
/// of the user's own Devices that says so last.
typedef _Taken = ({String noteId, AxtFrom from, Entry entry});

/// What an AXT file holds, checked as a whole and read, nothing taken
/// yet: its [sender], the key that signed it, as hexadecimal text, with
/// its [shortFingerprint] to compare; the [appName] its manifest gives;
/// whether the key is one of the user's own Devices' ([fromOwnDevice]);
/// and each record once, where its last message lies in the file.
class AxtFile {
  final String sender;
  final String shortFingerprint;
  final String? appName;
  final bool fromOwnDevice;
  final List<AxtRecord> records;

  const AxtFile({
    required this.sender,
    required this.shortFingerprint,
    required this.appName,
    required this.fromOwnDevice,
    required this.records,
  });

  /// The records that become new Notes when taken, in the file's order.
  List<AxtRecord> get taken => [
    for (final r in records)
      if (r.taken) r,
  ];

  /// The records that update or delete a Note taken before, in the
  /// file's order, those the user changed since among them ([AxtRecord.kept]).
  List<AxtRecord> get changes => [
    for (final r in records)
      if (r.changes) r,
  ];

  /// The others, in the file's order: of another type, a deletion of
  /// a record never taken, one the library refused, one that may not
  /// change the Note taken under its id, or one no later than it.
  List<AxtRecord> get notTaken => [
    for (final r in records)
      if (!r.taken && !r.changes) r,
  ];
}

/// The three types as the library reads them, made once.
final List<TypeDescription> _types = [
  for (final text in [
    axtNoteDescription,
    axtTaskDescription,
    axtAppointmentDescription,
  ])
    TypeDescription.fromJson(jsonDecode(text)),
];

TypeDescription _type(String name) => _types.firstWhere((t) => t.name == name);

/// The fingerprint of each type's derived schema, by full name, as the
/// 8 bytes a file names it by in hexadecimal digits, which the `axt`
/// fixture pins to the Rust library's.
Map<String, String> get axtTypeFingerprints => {
  for (final t in _types)
    t.name: hex([
      for (var i = 0; i < 8; i++) (t.fingerprint >> (8 * i)) & 0xff,
    ]),
};

Map<String, Object?> _take(String type, String purpose, String de) => {
  'type': type,
  'min_version': 1,
  'max_version': null,
  'purpose': purpose,
  'purpose_translations': {'de': de},
};

/// The manifest Old Brain Fix writes into every AXT file, without its
/// revision: it takes Appointments, Tasks and Notes, and sends Notes and
/// Tasks.
final Map<String, Object?> axtManifest = {
  'app_name': 'Old Brain Fix',
  'app_url': null,
  'formats': [1],
  'takes': [
    _take(
      AxtTypes.appointment,
      'An Appointment becomes a Note with a Reminder at its start.',
      'Ein Termin wird zu einer Notiz mit einer Erinnerung zu seinem Beginn.',
    ),
    _take(
      AxtTypes.task,
      'A Task becomes a Note with a Reminder or Routine.',
      'Eine Aufgabe wird zu einer Notiz mit einer Erinnerung oder Routine.',
    ),
    _take(
      AxtTypes.note,
      'A Note becomes a Note.',
      'Eine Notiz wird zu einer Notiz.',
    ),
  ],
  'any_type': null,
  'sends': [
    for (final type in [AxtTypes.note, AxtTypes.task])
      {'type': type, 'min_version': 1, 'max_version': 1, 'proposals': false},
  ],
  'encryption_key': null,
  'relay': null,
};

/// The manifest a file is read with: every type comes in, so that the
/// records of a type not taken are listed by their names; only the three
/// of [axtManifest] are ever taken.
final Map<String, Object?> _readingManifest = {
  ...axtManifest,
  'any_type': {
    'purpose': 'Any other type is listed as not taken.',
    'purpose_translations': <String, String>{},
  },
};

/// The AXT id of the Note [noteId] as a record of [type]: as a `Note`,
/// its 32 hexadecimal digits as a UUID of version 4, the bits of the
/// version and the variant set, so the same Note is always the same
/// record; an id of another form, and the Note as a record of another
/// type, through SHA-256 first. A record keeps its type for its whole
/// life (AXT `spec/records.md`, section 4), so a Note that gains a
/// Reminder goes as a `Task` of an id of its own.
String axtRecordId(String noteId, String type) {
  final digits = noteId.startsWith('note:') ? noteId.substring(5) : '';
  final seed = type == AxtTypes.note
      ? 'oldbrainfix:axt:$noteId'
      : 'oldbrainfix:axt:$type:$noteId';
  final bytes = type == AxtTypes.note && _hex32.hasMatch(digits)
      ? fromHex(digits)
      : Uint8List.fromList(
          sha256.convert(utf8.encode(seed)).bytes.sublist(0, 16),
        );
  bytes[6] = 0x40 | (bytes[6] & 0x0f);
  bytes[8] = 0x80 | (bytes[8] & 0x3f);
  final h = hex(bytes);
  return '${h.substring(0, 8)}-${h.substring(8, 12)}-${h.substring(12, 16)}-'
      '${h.substring(16, 20)}-${h.substring(20)}';
}

final _hex32 = RegExp(r'^[0-9a-f]{32}$');

/// The results of the messages the library hands on ([AxtNode.apply]).
final _changing = {
  Outcome.applied.code,
  Outcome.merged.code,
  Outcome.withdrawn.code,
};

/// The key-value store the library keeps its state in for one file: in
/// memory only, as nothing of it outlives the file (ADR 0056).
final class _MemoryStore implements AxtStore {
  final _values = <String, Uint8List>{};

  @override
  Future<Uint8List?> read(String key) async => _values[key];

  @override
  Future<void> write(String key, Uint8List value) async {
    _values[key] = Uint8List.fromList(value);
  }

  @override
  Future<void> delete(String key) async {
    _values.remove(key);
  }

  @override
  Future<List<String>> list(String prefix) async => [
    for (final key in _values.keys)
      if (key.startsWith(prefix)) key,
  ];

  @override
  Future<T> transaction<T>(Future<T> Function() body) async {
    final before = Map.of(_values);
    try {
      return await body();
    } catch (_) {
      _values
        ..clear()
        ..addAll(before);
      rethrow;
    }
  }
}

/// AXT files of the Notes of one store (ADR 0056).
class AxtFiles {
  final Store store;

  AxtFiles(this.store);

  /// The AXT file of the Notes [noteIds], in their order, signed with
  /// this Device's key: each as an AXT `Note`, or as a `Task` when it
  /// has a Reminder or Routine ([axtRecordOf]). An id that is no Note,
  /// or one in the trash or merged into another, is left out. A second
  /// file of the same Note is an update of the same record: its id is
  /// [axtRecordId], its version a stamp of the store's now, above every
  /// version a file of it had before.
  Future<Uint8List> write(Iterable<String> noteIds) async {
    final notes = Notes(store);
    final byNote = _routinesByNote();
    final records = <OutgoingRecord>[];
    for (final id in noteIds) {
      final note = notes.byId(id);
      if (note == null || note.isTrashed || note.isMerged) continue;
      final routines = byNote[id] ?? const <Routine>[];
      final changed = _latestChange(note, routines);
      final (type, value) = _record(note, routines, changed);
      records.add(OutgoingRecord(axtRecordId(id, type), _type(type), value));
    }
    final at = store.clock();
    final node = await AxtNode.open(
      store: _MemoryStore(),
      signingSeed: store.signingKey.seed,
      manifest: axtManifest,
      readers: _types,
      clock: () => at,
    );
    return node.writeFile(records);
  }

  /// The AXT type and value the Note [noteId] goes as ([write]); null
  /// when it is no Note in the lists.
  (String, Map<String, Object?>)? axtRecordOf(String noteId) {
    final note = Notes(store).byId(noteId);
    if (note == null || note.isTrashed || note.isMerged) return null;
    final routines = _routinesByNote()[noteId] ?? const <Routine>[];
    return _record(note, routines, _latestChange(note, routines));
  }

  /// The Routines and Reminders of each Note, those of the Notes merged
  /// into it included, read once.
  Map<String, List<Routine>> _routinesByNote() {
    final routines = Routines(store);
    final byNote = <String, List<Routine>>{};
    for (final routine in routines.all()) {
      final note = routines.note(routine);
      if (note == null) continue;
      (byNote[note.id] ??= []).add(routine);
    }
    return byNote;
  }

  /// The date of the latest entry of [note] and of its [routines].
  DateTime _latestChange(Note note, List<Routine> routines) {
    var latest = note.captured;
    for (final id in [note.id, for (final r in routines) r.id]) {
      for (final Entry e in store.loggedHistory(id)) {
        if (e.date.isAfter(latest)) latest = e.date;
      }
    }
    return latest.toUtc();
  }

  (String, Map<String, Object?>) _record(
    Note note,
    List<Routine> routines,
    DateTime changed,
  ) {
    final shown = note.shownText;
    final visible = note.visibleText;
    final title = _firstLine(visible);
    final routine = _chosen(routines);
    if (routine == null) {
      final value = ax.Note(
        title: title,
        text: shown,
        format: note.textFormat == TextFormats.markdown
            ? ax.NoteFormat.markdown
            : ax.NoteFormat.plain,
        created: _instant(note.captured),
        updated: _instant(changed),
        tags: _tagsOf(shown),
        relations: const [],
        extensions: const [],
      );
      return (AxtTypes.note, _map(ax.Note.avroType.toAvro(value)));
    }
    final schedule = routine.schedule;
    final first = schedule.next(schedule.start, 1).firstOrNull;
    final doneAt = first == null ? null : routine.doneAt(first);
    final value = ax.Task(
      title: title,
      notes: _rest(visible),
      due: first == null ? null : _floating(first),
      dueAllDay: first != null && first.time == null,
      repeat: first == null ? null : _repeat(schedule, first),
      done: doneAt != null,
      doneAt: doneAt == null ? null : _instant(doneAt),
      priority: ax.TaskPriority.none,
      relations: const [],
      extensions: const [],
    );
    return (AxtTypes.task, _map(ax.Task.avroType.toAvro(value)));
  }

  /// Reads the AXT file [bytes]: checks it as a whole, signatures and
  /// all, and reads each of its records, applying nothing. Times in a
  /// zone are put on this Device's wall clock by [wallClock]. Throws
  /// [NotAnAxtFile] for a file the library rejects as a whole.
  Future<AxtFile> read(Uint8List bytes, {required WallClock wallClock}) async {
    final random = Random.secure();
    final node = await AxtNode.open(
      store: _MemoryStore(),
      // A key of its own: nothing is signed with it, and no file can
      // pass for one of this reader's own.
      signingSeed: Uint8List.fromList([
        for (var i = 0; i < 32; i++) random.nextInt(256),
      ]),
      manifest: _readingManifest,
      readers: _types,
      clock: store.clock,
    );
    final FilePreview preview;
    try {
      preview = await node.inspect(bytes);
    } on AxtException {
      throw const NotAnAxtFile();
    }
    final changes = <RecordChange>[];
    final result = await node.apply(
      preview,
      acceptSender: true,
      onRecord: (change) async => changes.add(change),
      currentValue: (_, _) async => null,
    );
    // Each record once, as the file leaves it: a later message of the
    // same id that the library applied replaces what came before, and
    // moves to where it lies.
    final read = <String, AxtRecord>{};
    var next = 0;
    for (final r in result.records) {
      final changed = _changing.contains(r.result);
      final change = changed && next < changes.length ? changes[next++] : null;
      if (!changed && read.containsKey(r.id)) continue;
      final value = change?.id == r.id ? change?.value : null;
      final type = value == null ? null : change?.type;
      read.remove(r.id);
      read[r.id] = AxtRecord(
        id: r.id,
        version: r.version,
        type: type,
        result: r.result,
        draft: type == null || value == null
            ? null
            : _draft(type, value, wallClock),
      );
    }
    final key = fromHex(preview.sender);
    return AxtFile(
      sender: preview.sender,
      shortFingerprint: preview.shortFingerprint,
      appName: preview.appName,
      fromOwnDevice: store.writtenByOwn(deviceIdFromKey(key)),
      records: _classified(preview.sender, read.values.toList()),
    );
  }

  /// [records] of a file of [sender], each with what it is to the Notes
  /// taken before ([AxtRecordKind]): the Note taken under its id from the
  /// same sender, by the [AxtFrom] the user's own Devices wrote last,
  /// the one of the highest version where two were, the first by id
  /// where they tie. A record from another sender under an id taken
  /// may change nothing.
  List<AxtRecord> _classified(String sender, List<AxtRecord> records) {
    final own = <String, bool>{};
    bool isOwn(String device) =>
        own.putIfAbsent(device, () => store.writtenByOwn(device));
    final taken = _takenByRecord(isOwn);
    return [
      for (final r in records) _classify(r, sender, taken[r.id] ?? [], isOwn),
    ];
  }

  /// Every Note taken from an AXT file, by the record id it came from,
  /// in the order of the Notes' ids.
  Map<String, List<_Taken>> _takenByRecord(bool Function(String) isOwn) {
    final ids = {
      for (final (entity, _, device) in store.writersOf([AxtKeys.from]))
        if (isOwn(device)) entity,
    }.toList()..sort();
    final histories = store.loggedHistoriesOf(ids);
    final byRecord = <String, List<_Taken>>{};
    for (final id in ids) {
      Entry? held;
      for (final e in histories[id] ?? const <Entry>[]) {
        if (e.field != AxtKeys.from || !isOwn(e.device)) continue;
        if (held == null || Store.latestWins(e, held) > 0) held = e;
      }
      if (held == null) continue;
      if (AxtFrom.parse(held.value) case final from?) {
        (byRecord[from.id] ??= []).add((noteId: id, from: from, entry: held));
      }
    }
    return byRecord;
  }

  AxtRecord _classify(
    AxtRecord r,
    String sender,
    List<_Taken> taken,
    bool Function(String) isOwn,
  ) {
    if (taken.isEmpty) {
      return r._as(
        r.draft == null ? AxtRecordKind.none : AxtRecordKind.newRecord,
      );
    }
    _Taken? target;
    for (final t in taken) {
      if (t.from.sender != sender) continue;
      if (target == null || t.from.version > target.from.version) target = t;
    }
    if (target == null) {
      return r._as(AxtRecordKind.refused, noteId: taken.first.noteId);
    }
    final noteId = target.noteId;
    final deletion =
        r.type == null &&
        (r.result == Outcome.applied.code ||
            r.result == Outcome.withdrawn.code);
    final data = r.draft != null && r.type == target.from.type;
    if (!deletion && !data) return r._as(AxtRecordKind.refused, noteId: noteId);
    if (r.version <= target.from.version) {
      return r._as(AxtRecordKind.stale, noteId: noteId);
    }
    return r._as(
      deletion ? AxtRecordKind.deletion : AxtRecordKind.update,
      noteId: noteId,
      kept: _changedSince(target, isOwn),
    );
  }

  /// Whether the user changed the Note [taken] since it was taken or
  /// last updated: an entry of one of their own Devices after the
  /// [AxtFrom] that says so, on the Note, on a Routine, Reminder or
  /// Section that ever was the Note's, on their Heads-ups, or on a Note
  /// ever merged into it. What another person's Device wrote is no
  /// change of the user's.
  bool _changedSince(_Taken taken, bool Function(String) isOwn) {
    final noteId = taken.noteId;
    final linked = {
      ...store.everWith(RoutineKeys.note, noteId),
      ...store.everWith(NoteKeys.merged, noteId),
    };
    final entities = {
      noteId,
      ...linked,
      for (final id in linked) HeadsUpKeys.of(id),
    };
    for (final MapEntry(key: entity, value: entries)
        in store.loggedHistoriesOf(entities).entries) {
      for (final e in entries) {
        if (entity == noteId && e.field == AxtKeys.from) continue;
        if (!isOwn(e.device)) continue;
        if (Store.latestWins(e, taken.entry) > 0) return true;
      }
    }
    return false;
  }

  /// Takes the records of [file] whose ids are among [ids], in the file's
  /// order, all in one batch, each as it is to the Notes taken before
  /// now, whatever [file] said when it was read ([AxtRecordKind]):
  ///
  /// - a new one becomes a new Note of this Device's, captured now, filed
  ///   into the Area or Topic [place] or left in Unsorted with null, with
  ///   its Reminder or Routine written, not spawned, its first Occurrence
  ///   Done when the record says so, its Heads-ups, and where it came
  ///   from ([AxtFrom]);
  /// - an update rewrites the text of the Note taken from it and its
  ///   Reminder or Routine to the record's, bringing it back from the
  ///   trash a deletion put it in;
  /// - a deletion moves that Note to the trash with its Reminders and
  ///   Routines;
  /// - one whose Note the user changed since ([AxtRecord.kept]), one
  ///   refused and one stale change nothing.
  ///
  /// An update or a deletion keeps the record's version in [AxtFrom],
  /// written last. Returns the new Notes' ids. Throws an [ArgumentError]
  /// for a place that is no Area or Topic and a [StateError] where this
  /// Device may not write there, may not make Reminders and Routines
  /// there, or may not change a Note to update or delete, before
  /// anything is written.
  List<String> take(AxtFile file, Iterable<String> ids, {String? place}) {
    final chosen = ids.toSet();
    final records = [
      for (final r in _classified(file.sender, file.records))
        if (chosen.contains(r.id)) r,
    ];
    if (place != null && !Places(store).isPlace(place)) {
      throw ArgumentError.value(place, 'place', 'not an Area or Topic');
    }
    if (place != null && !store.mayWriteIn(place)) {
      throw StateError('no role in the Shared Area of $place');
    }
    final area = place == null ? null : store.areaOf(place);
    if (records.any((r) => r.taken && r.draft?.schedule != null) &&
        !store.mayManage(area)) {
      throw StateError('not a Manager of the Area of $place');
    }
    for (final r in records) {
      if (!r.changes || r.kept) continue;
      final noteId = r.noteId;
      if (noteId == null) continue;
      if (!store.mayChange(noteId)) {
        throw StateError('$noteId may not be changed here');
      }
      final schedules =
          (r.kind == AxtRecordKind.update && r.draft?.schedule != null) ||
          _routinesOf(noteId).isNotEmpty;
      if (schedules && !store.mayManage(store.areaOf(noteId))) {
        throw StateError('not a Manager of the Area of $noteId');
      }
    }
    final notes = Notes(store);
    return store.batch(() {
      final made = <String>[];
      for (final record in records) {
        final draft = record.draft;
        final type = record.type;
        final noteId = record.noteId;
        switch (record.kind) {
          case AxtRecordKind.newRecord when draft != null && type != null:
            final note = notes.capture(
              draft.text,
              format: draft.format,
              place: place,
            );
            if (draft.schedule case final schedule?) {
              _writeRoutine(note.id, schedule, draft);
            }
            // Last, so that what the record wrote lies before it and is
            // no change of the user's.
            store.append(
              note.id,
              AxtKeys.from,
              AxtFrom(
                sender: file.sender,
                id: record.id,
                type: type,
                version: record.version,
              ).encode(),
            );
            made.add(note.id);
          case AxtRecordKind.update
              when !record.kept && draft != null && noteId != null:
            _update(noteId, draft, record.version);
          case AxtRecordKind.deletion when !record.kept && noteId != null:
            _delete(noteId, record.version);
          default:
            break;
        }
      }
      return made;
    });
  }

  /// The Reminders and Routines of the Note [noteId] not in the trash,
  /// by id, found by what ever named the Note rather than by reading
  /// every Routine.
  List<Routine> _routinesOf(String noteId) {
    final routines = Routines(store);
    return [
      for (final id
          in store.everWith(RoutineKeys.note, noteId).toList()..sort())
        if (routines.byId(id) case final r?
            when r.noteId == noteId && !r.isTrashed)
          r,
    ];
  }

  /// Rewrites the Note [noteId] to [draft], the record of [version]: its
  /// text when it differs, its first Reminder or Routine to the record's
  /// schedule, Heads-ups and first Occurrence Done or not, one written
  /// where it had none, every other one in the trash.
  void _update(String noteId, AxtNoteDraft draft, int version) {
    final notes = Notes(store);
    final routines = Routines(store);
    final note = notes.byId(noteId);
    final from = this.from(noteId);
    if (note == null || from == null) return;
    if (note.isTrashed) routines.restoreNote(noteId);
    final text = draft.text.trim();
    final format = switch (note.textFormat) {
      TextFormats.markdown || TextFormats.code => note.textFormat,
      _ => draft.format,
    };
    if (text.isNotEmpty &&
        (note.shownText.trim() != text || note.textFormat != format)) {
      notes.edit(noteId, text, format: draft.format);
    }
    final live = _routinesOf(noteId);
    final schedule = draft.schedule;
    if (schedule == null) {
      for (final r in live) {
        routines.trash(r.id);
      }
    } else if (live.isEmpty) {
      _writeRoutine(noteId, schedule, draft);
    } else {
      final routine = live.first;
      routines
        ..reschedule(routine.id, schedule)
        ..setHeadsUps(routine.id, draft.headsUps);
      final first = schedule.next(schedule.start, 1).firstOrNull;
      if (first != null) {
        final doneAt = routine.doneAt(first);
        if (draft.done && doneAt == null) {
          store.append(
            routine.id,
            RoutineKeys.done(first),
            (draft.doneAt ?? store.clock()).toUtc().toIso8601String(),
          );
        } else if (!draft.done && doneAt != null) {
          store.append(routine.id, RoutineKeys.done(first), null);
        }
      }
      for (final other in live.skip(1)) {
        routines.trash(other.id);
      }
    }
    _bump(noteId, from, version);
  }

  /// Moves the Note [noteId] to the trash with its Reminders and
  /// Routines, as [Routines.trashNote] does, for the record's deletion of
  /// [version]; one in the trash already stays there.
  void _delete(String noteId, int version) {
    final notes = Notes(store);
    final note = notes.byId(noteId);
    final from = this.from(noteId);
    if (note == null || from == null) return;
    if (!note.isTrashed) {
      final live = _routinesOf(noteId);
      notes.trash(noteId);
      // The same moment as the Note's, so both are purged together.
      final at =
          store.current(noteId, NoteKeys.trashed) ??
          store.clock().toUtc().toIso8601String();
      for (final r in live) {
        store
          ..append(r.id, RoutineKeys.trashed, at)
          ..append(r.id, RoutineKeys.trashedWith, noteId);
      }
    }
    _bump(noteId, from, version);
  }

  /// The [AxtFrom] of the Note [noteId] at [version], written last, so
  /// that what the record changed lies before it.
  void _bump(String noteId, AxtFrom from, int version) => store.append(
    noteId,
    AxtKeys.from,
    AxtFrom(
      sender: from.sender,
      id: from.id,
      type: from.type,
      version: version,
    ).encode(),
  );

  /// A Reminder or Routine of the Note [noteId] on [schedule], written,
  /// not spawned, as a Sent Item's is.
  void _writeRoutine(String noteId, Schedule schedule, AxtNoteDraft draft) {
    final id = 'routine:${Store.newId()}';
    store.append(id, RoutineKeys.type, RoutineKeys.typeRoutine);
    store.append(id, RoutineKeys.note, noteId);
    store.append(id, RoutineKeys.start, schedule.start.toString());
    store.append(id, RoutineKeys.times, schedule.times.join(','));
    if (schedule.rule case final rule?) {
      store.append(id, RoutineKeys.rule, rule.format());
    }
    if (HeadsUpKeys.write(draft.headsUps) case final headsUps?) {
      store.append(HeadsUpKeys.of(id), HeadsUpKeys.headsUps, headsUps);
    }
    final first = schedule.next(schedule.start, 1).firstOrNull;
    if (draft.done && first != null) {
      store.append(
        id,
        RoutineKeys.done(first),
        (draft.doneAt ?? store.clock()).toUtc().toIso8601String(),
      );
    }
  }

  /// Where the Note [noteId] came from over AXT, as the user's own
  /// Devices last wrote it ([currentByOwn]): another person's Device in
  /// a Shared Area can neither set nor change it. Null when it came from
  /// no AXT file.
  AxtFrom? from(String noteId) =>
      AxtFrom.parse(store.currentByOwn(noteId, AxtKeys.from));
}

/// The Routine or Reminder of a Note that goes as its `Task`: the one
/// whose first Occurrence comes first, by id when two start alike; one
/// without any Occurrence last.
Routine? _chosen(List<Routine> routines) {
  Routine? chosen;
  Occurrence? at;
  for (final r in routines) {
    final first = r.schedule.next(r.schedule.start, 1).firstOrNull;
    final better =
        chosen == null ||
        switch ((first, at)) {
          (null, null) => r.id.compareTo(chosen.id) < 0,
          (null, _) => false,
          (_, null) => true,
          (final f?, final a?) =>
            f.compareTo(a) < 0 ||
                (f.compareTo(a) == 0 && r.id.compareTo(chosen.id) < 0),
        };
    if (better) {
      chosen = r;
      at = first;
    }
  }
  return chosen;
}

/// The RRULE of a `Task` due at [first] on [schedule], as the calendar
/// file writes it (`ics.dart`): several times of day go as BYHOUR and
/// BYMINUTE when they are every hour with every minute named; otherwise
/// the Task keeps the first time only and a COUNT becomes the UNTIL of
/// the last Occurrence. An UNTIL takes the type of the due date.
String? _repeat(Schedule schedule, Occurrence first) {
  final rule = schedule.rule;
  if (rule == null) return null;
  var count = rule.count;
  var until = rule.until;
  var byHour = rule.byHour;
  var byMinute = rule.byMinute;
  final times = schedule.dayTimes;
  if (rule.times == null && times.length > 1) {
    final hours = {for (final t in times) t.hour}.toList()..sort();
    final minutes = {for (final t in times) t.minute}.toList()..sort();
    if (hours.length * minutes.length == times.length) {
      byHour = hours;
      byMinute = minutes;
    } else if (count != null) {
      final last = schedule.next(first.day, count).last;
      count = null;
      until = (day: last.day, time: last.time);
    }
  }
  if (until != null) {
    until = first.time == null
        ? (day: until.day, time: null)
        : (day: until.day, time: until.time ?? ClockTime(23, 59));
  }
  return Recurrence(
    frequency: rule.frequency,
    interval: rule.interval,
    byDay: rule.byDay,
    byMonthDay: rule.byMonthDay,
    byMonth: rule.byMonth,
    byHour: byHour,
    byMinute: byMinute,
    count: count,
    until: until,
    weekStart: rule.weekStart,
  ).format();
}

/// The first line of [text], trimmed: a record's title.
String _firstLine(String text) => text.trim().split('\n').first.trim();

/// The lines of [text] after its first, trimmed: a `Task`'s notes.
String _rest(String text) {
  final lines = text.trim().split('\n');
  return lines.skip(1).join('\n').trim();
}

/// The Tags of [text] without their `#`, each once whatever its case, in
/// their order, as written.
List<String> _tagsOf(String text) {
  final seen = <String>{};
  return [
    for (final t in TextTags.find(text))
      if (seen.add(t.tag)) text.substring(t.start + 1, t.end),
  ];
}

ax.AxtDateTime _instant(DateTime at) {
  final u = at.toUtc();
  return ax.AxtDateTime(
    year: u.year,
    month: u.month,
    day: u.day,
    hour: u.hour,
    minute: u.minute,
    second: u.second,
    millisecond: u.millisecond,
    zone: 'Etc/UTC',
    offsetSeconds: 0,
    instantMs: u.millisecondsSinceEpoch,
  );
}

ax.AxtDateTime _floating(Occurrence o) => ax.AxtDateTime(
  year: o.day.year,
  month: o.day.month,
  day: o.day.day,
  hour: o.time?.hour ?? 0,
  minute: o.time?.minute ?? 0,
  second: 0,
  millisecond: 0,
);

Map<String, Object?> _map(Object? value) => switch (value) {
  final Map<String, Object?> map => map,
  _ => throw StateError('a record is no map'),
};

/// The Note a record of [type] with [value] makes.
AxtNoteDraft? _draft(
  String type,
  Map<String, Object?> value,
  WallClock wallClock,
) {
  String text(String field) => switch (value[field]) {
    final String s => s.replaceAll('\r\n', '\n').replaceAll('\r', '\n'),
    _ => '',
  };
  switch (type) {
    case AxtTypes.note:
      final markdown = value['format'] == 'MARKDOWN';
      final format = markdown ? TextFormats.markdown : TextFormats.plain;
      final title = text('title').trim();
      var body = text('text');
      if (title.isNotEmpty &&
          _firstLine(TextFormats.visible(format, body)) != title) {
        body = body.trim().isEmpty ? title : '$title\n$body';
      }
      final tags = _missingTags(value['tags'], body);
      if (tags.isNotEmpty) body = body.isEmpty ? tags : '$body\n$tags';
      return AxtNoteDraft(text: body, format: format);
    case AxtTypes.task:
      final body = [
        text('title').trim(),
        text('notes').trim(),
      ].where((s) => s.isNotEmpty).join('\n');
      final due = value['due'];
      if (due is! Map<String, Object?>) {
        return AxtNoteDraft(text: body, format: TextFormats.plain);
      }
      final allDay = value['due_all_day'] == true;
      final start = _wall(due, wallClock, allDay: allDay);
      final rule = switch (value['repeat']) {
        final String r => Recurrence.tryParse(r),
        _ => null,
      };
      final schedule = Schedule(
        start: start.day,
        times: [?start.time],
        rule: rule,
      );
      final done = value['done'] == true;
      return AxtNoteDraft(
        text: body,
        format: TextFormats.plain,
        schedule: schedule,
        done: done,
        doneAt: done ? _instantOf(value['done_at']) : null,
      );
    case AxtTypes.appointment:
      final body = [
        text('title').trim(),
        text('location').trim(),
        text('notes').trim(),
      ].where((s) => s.isNotEmpty).join('\n');
      final start = value['start'];
      if (start is! Map<String, Object?>) {
        return AxtNoteDraft(text: body, format: TextFormats.plain);
      }
      final allDay = value['all_day'] == true;
      final at = _wall(start, wallClock, allDay: allDay);
      return AxtNoteDraft(
        text: body,
        format: TextFormats.plain,
        schedule: Schedule(start: at.day, times: [?at.time]),
        headsUps: _headsUps(value['reminders'], allDay: allDay),
      );
  }
  return null;
}

/// The tags of [tags] ([_tagOf]) not in [body] yet, each
/// once whatever its case, in their order, apart by spaces.
String _missingTags(Object? tags, String body) {
  if (tags is! List<Object?>) return '';
  final seen = {for (final t in TextTags.find(body)) t.tag};
  return [
    for (final name in tags)
      if (name is String)
        if (_tagOf(name) case final tag? when seen.add(tag.toLowerCase())) tag,
  ].join(' ');
}

/// The tag a record's tag [name] is in a text: each part between `/`
/// as [TextTags.fromLabel] makes it, nested again by `/`; null when
/// nothing is left.
String? _tagOf(String name) {
  final parts = [
    for (final part in name.split('/'))
      if (TextTags.fromLabel(part) case final tag?) tag.substring(1),
  ];
  return parts.isEmpty ? null : '#${parts.join('/')}';
}

/// The Occurrence an AXT date and time [value] is on this Device's wall
/// clock: a zoned one through its instant and [wallClock], a floating
/// one as it reads; [allDay] keeps only its day as written.
Occurrence _wall(
  Map<String, Object?> value,
  WallClock wallClock, {
  required bool allDay,
}) {
  int field(String name) => switch (value[name]) {
    final int n => n,
    _ => 0,
  };
  if (allDay) {
    return Occurrence(Day(field('year'), field('month'), field('day')));
  }
  final instant = value['instant_ms'];
  if (value['zone'] is String && instant is int) {
    final wall = wallClock(
      DateTime.fromMillisecondsSinceEpoch(instant, isUtc: true),
    );
    return Occurrence(
      Day(wall.year, wall.month, wall.day),
      ClockTime(wall.hour, wall.minute),
    );
  }
  return Occurrence(
    Day(field('year'), field('month'), field('day')),
    ClockTime(field('hour'), field('minute')),
  );
}

/// The instant of a zoned AXT date and time [value]; null for a
/// floating one, which names none.
DateTime? _instantOf(Object? value) {
  if (value case {'zone': String _, 'instant_ms': final int ms}) {
    return DateTime.fromMillisecondsSinceEpoch(ms, isUtc: true);
  }
  return null;
}

/// The Heads-ups of an `Appointment`'s [reminders]: each length before
/// its start in whole minutes, an all-day one in whole days; one at or
/// after the start, one in months, one of seconds and one further ahead
/// than [HeadsUpKeys.maxMinutes] are left out.
List<int> _headsUps(Object? reminders, {required bool allDay}) {
  if (reminders is! List<Object?>) return const [];
  final minutes = <int>[];
  for (final r in reminders) {
    if (r
        case {
          'months': final int months,
          'days': final int days,
          'millis': final int millis,
        }
        when months == 0) {
      final total = days * 86400000 + millis;
      if (total >= 0 || total % 60000 != 0) continue;
      final before = -total ~/ 60000;
      if (allDay && before % 1440 != 0) continue;
      minutes.add(before);
    }
  }
  return HeadsUpKeys.read(HeadsUpKeys.write(minutes));
}
