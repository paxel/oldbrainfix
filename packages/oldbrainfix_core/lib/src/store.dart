import 'dart:convert';
import 'dart:io';
import 'dart:math';
import 'dart:typed_data';

import 'package:crypto/crypto.dart';
import 'package:meta/meta.dart';
import 'package:sqlite3/sqlite3.dart';

import 'blobs.dart';
import 'conflicts.dart';
import 'entry.dart';
import 'pending.dart';
import 'photos.dart';
import 'shared_areas.dart';
import 'signing.dart';

/// The log: an append-only entry log over SQLite with a latest-wins
/// projection (ADR 0002). All reads and writes are synchronous.
///
/// The store never reads the wall clock on its own: [clock] is handed in,
/// so tests and the fixture generator get a reproducible log.
///
/// Photo and voice recording bytes live outside the log, content-addressed
/// by SHA-256: `photos` and `audio` directories next to the database file,
/// or memory for [Store.inMemory].
///
/// Pending Changes (ADR 0022) lie beside it, in a `pending` directory:
/// every read of what the log holds now ([history], [currentFields] and
/// the lists of entities and fields) sees the shown sets on top of it;
/// what is held, passed on, numbered or trusted reads the log alone.
class Store {
  final Database _db;
  final Blobs _photos;
  final Blobs _audio;
  final PendingFiles _pendingFiles;

  /// Where the store reads "now" for [Entry.recorded] and default dates.
  final DateTime Function() clock;

  Store._(this._db, this.clock, this._photos, this._audio, this._pendingFiles);

  final _listeners = <void Function()>[];
  var _inBatch = false;

  /// Calls [listener] after every change to the log: after each append,
  /// or once a [batch] is committed. Local settings do not count.
  void addListener(void Function() listener) => _listeners.add(listener);

  void removeListener(void Function() listener) => _listeners.remove(listener);

  final _profileListeners = <void Function()>[];

  /// Calls [listener] after the own Profile changed, or a question about
  /// it came or went (ADR 0017): from the Profile page, or from another
  /// of the user's Devices in a sync.
  void addProfileListener(void Function() listener) =>
      _profileListeners.add(listener);

  void removeProfileListener(void Function() listener) =>
      _profileListeners.remove(listener);

  /// Tells the [addProfileListener] listeners that the own Profile
  /// changed.
  void profileChanged() {
    for (final listener in List.of(_profileListeners)) {
      listener();
    }
  }

  void _changed() {
    _judged = null;
    _overlay = null;
    _shownJudged = null;
    if (_inBatch) return;
    for (final listener in List.of(_listeners)) {
      listener();
    }
  }

  /// Opens an on-disk store, creating it if needed. [newKey] makes the
  /// Device's signing key on first open; fixtures pass a fixed one.
  factory Store.open(
    String path, {
    DateTime Function() clock = DateTime.now,
    SigningKey Function() newKey = SigningKey.generate,
  }) => _init(
    sqlite3.open(path),
    clock,
    newKey,
    FileBlobs('${File(path).parent.path}/photos'),
    FileBlobs('${File(path).parent.path}/audio'),
    FilePendingFiles('${File(path).parent.path}/pending'),
  );

  /// Opens a throwaway in-memory store (tests).
  factory Store.inMemory({
    DateTime Function() clock = DateTime.now,
    SigningKey Function() newKey = SigningKey.generate,
  }) => _init(
    sqlite3.openInMemory(),
    clock,
    newKey,
    MemoryBlobs(),
    MemoryBlobs(),
    MemoryPendingFiles(),
  );

  static Store _init(
    Database db,
    DateTime Function() clock,
    SigningKey Function() newKey,
    Blobs photos,
    Blobs audio,
    PendingFiles pending,
  ) {
    // The background sync may hold the file a moment while the app opens:
    // a write waits for it instead of failing.
    db.execute('''
      PRAGMA busy_timeout = 5000;
      PRAGMA journal_mode = WAL;
      CREATE TABLE IF NOT EXISTS entries (
        seq      INTEGER PRIMARY KEY AUTOINCREMENT,
        device   TEXT NOT NULL,
        dseq     INTEGER NOT NULL,
        entity   TEXT NOT NULL,
        field    TEXT NOT NULL,
        value    TEXT,
        date     TEXT NOT NULL,
        author   TEXT NOT NULL,
        recorded TEXT NOT NULL,
        sig      TEXT,
        ctx      TEXT,
        raw      TEXT,
        v        INTEGER
      );
      CREATE TABLE IF NOT EXISTS local_settings (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
      );
      CREATE TABLE IF NOT EXISTS moments (
        id    INTEGER PRIMARY KEY AUTOINCREMENT,
        seq   INTEGER NOT NULL,
        cause TEXT NOT NULL,
        label TEXT,
        at    TEXT NOT NULL
      );
      CREATE UNIQUE INDEX IF NOT EXISTS idx_entries_device_dseq
        ON entries (device, dseq);
      CREATE INDEX IF NOT EXISTS idx_entries_entity_field
        ON entries (entity, field);
    ''');
    // A log from before causal contexts were kept gains the column; its
    // entries keep none.
    final columns = db.select('PRAGMA table_info(entries)');
    if (!columns.any((c) => c['name'] == 'ctx')) {
      db.execute('ALTER TABLE entries ADD COLUMN ctx TEXT');
    }
    // And the entries kept as they came, of a later structure among them
    // (ADR 0019); none in a log from before.
    if (!columns.any((c) => c['name'] == 'raw')) {
      db.execute('ALTER TABLE entries ADD COLUMN raw TEXT');
      db.execute('ALTER TABLE entries ADD COLUMN v INTEGER');
    }
    final store = Store._(db, clock, photos, audio, pending);
    if (store.localSetting('device') == null) {
      final key = newKey();
      store.setLocalSetting('device', deviceIdFromKey(key.publicKey));
      store.setLocalSetting('signseed', key.seedHex);
    }
    store._loadPending();
    return store;
  }

  void close() => _db.close();

  /// This installation's stable device id; created once.
  String get deviceId => localSetting('device')!;

  SigningKey? _signingKey;

  /// This Device's signing key.
  SigningKey get signingKey =>
      _signingKey ??= SigningKey.fromSeed(fromHex(localSetting('signseed')!));

  /// Author name stamped on own entries.
  String get author => localSetting('author') ?? '';

  // ------------------------------------------------------ local settings

  /// A setting of this Device. Never synced (ADR 0008).
  String? localSetting(String key) {
    final rows = _db.select('SELECT value FROM local_settings WHERE key = ?', [
      key,
    ]);
    return rows.isEmpty ? null : rows.first['value'] as String;
  }

  void setLocalSetting(String key, String value) => _db.execute(
    'INSERT OR REPLACE INTO local_settings (key, value) VALUES (?, ?)',
    [key, value],
  );

  void removeLocalSetting(String key) =>
      _db.execute('DELETE FROM local_settings WHERE key = ?', [key]);

  /// Every setting of this Device whose key starts with [prefix], as
  /// (key, value), by key.
  List<(String, String)> localSettingsWithPrefix(String prefix) => [
    for (final r in _db.select(
      'SELECT key, value FROM local_settings WHERE substr(key, 1, ?) = ? '
      'ORDER BY key',
      [prefix.length, prefix],
    ))
      (r['key'] as String, r['value'] as String),
  ];

  // -------------------------------------------------------------- photos

  /// Keeps the photo [bytes] without their metadata ([stripJpegMetadata])
  /// and returns the hash it is kept under. The same photo twice is kept
  /// once.
  String putPhoto(Uint8List bytes) {
    final clean = stripJpegMetadata(bytes);
    final hash = sha256.convert(clean).toString();
    _photos.put(hash, clean);
    return hash;
  }

  /// The bytes of the photo [hash], or null when they are not here.
  Uint8List? photo(String hash) => _photos.get(hash);

  // --------------------------------------------------------------- audio

  /// Keeps the voice recording [bytes] as they are and returns the hash
  /// it is kept under. The same recording twice is kept once.
  String putAudio(Uint8List bytes) {
    final hash = sha256.convert(bytes).toString();
    _audio.put(hash, bytes);
    return hash;
  }

  /// The bytes of the voice recording [hash], or null when they are not
  /// here.
  Uint8List? audio(String hash) => _audio.get(hash);

  /// The bytes of the [kind] (`photo` or `audio`) blob [hash], or null.
  Uint8List? blob(String kind, String hash) => switch (kind) {
    'photo' => _photos.get(hash),
    'audio' => _audio.get(hash),
    _ => null,
  };

  /// Lets go of the bytes of the [kind] blob [hash]; returns whether
  /// they were here. Only the log says when that is right (ADR 0020).
  bool dropBlob(String kind, String hash) => switch (kind) {
    'photo' => _photos.remove(hash),
    'audio' => _audio.remove(hash),
    _ => false,
  };

  /// Keeps [bytes] another Device holds under [hash], as they are: they
  /// were made the way this Device makes them over there. Bytes whose
  /// SHA-256 is not [hash], or of an unknown [kind], are not kept;
  /// returns whether they were.
  bool putBlob(String kind, String hash, Uint8List bytes) {
    if (sha256.convert(bytes).toString() != hash) return false;
    switch (kind) {
      case 'photo':
        _photos.put(hash, bytes);
      case 'audio':
        _audio.put(hash, bytes);
      default:
        return false;
    }
    return true;
  }

  // ----------------------------------------------------------------- log

  /// Appends one fact under this Device, signed. [date] defaults to now.
  /// An edit to a field concurrent edits are a Conflict on carries its
  /// causal context ([Entry.ctx]).
  Entry append(String entity, String field, String? value, {DateTime? date}) {
    final now = clock().toUtc();
    final device = deviceId;
    final vector = _heldVector();
    final dseq = (vector[device] ?? 0) + 1;
    final unit = conflictUnit(field);
    final ctx = unit == null ? null : _context(entity, unit, device, vector);
    final dateIso = (date ?? now).toUtc().toIso8601String();
    final recordedIso = now.toIso8601String();
    final sig = signingKey.sign(
      entryBytes(
        device: device,
        dseq: dseq,
        entity: entity,
        field: field,
        value: value,
        dateIso: dateIso,
        author: author,
        recordedIso: recordedIso,
        ctx: ctx,
      ),
    );
    _db.execute(
      'INSERT INTO entries (device, dseq, entity, field, value, date, author, recorded, sig, ctx) '
      'VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)',
      [
        device,
        dseq,
        entity,
        field,
        value,
        dateIso,
        author,
        recordedIso,
        sig,
        ctx,
      ],
    );
    _changed();
    return _entries('SELECT * FROM entries WHERE device = ? AND dseq = ?', [
      device,
      dseq,
    ]).single;
  }

  /// The causal context of a write to [unit] on [entity] by [device]
  /// holding [vector]: how far it has seen every other Device that wrote
  /// the unit here; null when nobody wrote it yet. What the shown pending
  /// sets hold of other Devices counts as seen (ADR 0022): the user saw
  /// it when they wrote.
  String? _context(
    String entity,
    String unit,
    String device,
    Map<String, int> vector,
  ) {
    final fields = conflictUnitFields(unit);
    final seen = Map.of(vector);
    final received = [
      for (final e in _shown)
        if (e.device != device) e,
    ];
    for (final e in received) {
      if (e.dseq > (seen[e.device] ?? 0)) seen[e.device] = e.dseq;
    }
    final writers = {
      for (final r in _db.select(
        'SELECT DISTINCT device FROM entries WHERE $_applied AND entity = ? '
        'AND field IN '
        '(${List.filled(fields.length, '?').join(', ')})',
        [entity, ...fields],
      ))
        r['device'] as String,
      for (final e in received)
        if (e.entity == entity && fields.contains(e.field)) e.device,
    };
    if (writers.isEmpty) return null;
    return encodeContext({
      for (final d in writers)
        if (d != device) d: seen[d] ?? 0,
    });
  }

  /// Takes in an entry another Device wrote, as it is; one already held
  /// by (device, dseq) is skipped. Returns whether it was new. The caller
  /// checks its signature first. What it came as is kept when this
  /// version would write it otherwise ([Entry.raw]), and its version
  /// when it is not 1.
  bool insert(Entry e) {
    _db.execute(
      'INSERT OR IGNORE INTO entries (device, dseq, entity, field, value, date, author, recorded, sig, ctx, raw, v) '
      'VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)',
      [
        e.device,
        e.dseq,
        e.entity,
        e.field,
        e.value,
        e.date.toUtc().toIso8601String(),
        e.author,
        e.recorded.toUtc().toIso8601String(),
        e.sig,
        e.ctx,
        e.raw,
        e.v == 1 ? null : e.v,
      ],
    );
    final added = _db.updatedRows > 0;
    if (added) _changed();
    return added;
  }

  /// Whether the entry (device, dseq) is held.
  bool holds(String device, int dseq) => _db.select(
    'SELECT 1 FROM entries WHERE device = ? AND dseq = ?',
    [device, dseq],
  ).isNotEmpty;

  /// Every entry a Device holding [vector] lacks, by (device, dseq).
  List<Entry> entriesSince(Map<String, int> vector) => [
    for (final e in _entries('SELECT * FROM entries ORDER BY device, dseq'))
      if (e.dseq > (vector[e.device] ?? 0)) e,
  ];

  /// The (entity, field) pairs whose field starts with [prefix], by
  /// entity and field.
  List<(String, String)> fieldsWithPrefix(String prefix) {
    final logged = [
      for (final r in _db.select(
        'SELECT DISTINCT entity, field FROM entries '
        'WHERE $_applied AND substr(field, 1, ?) = ? '
        'ORDER BY entity, field',
        [prefix.length, prefix],
      ))
        (r['entity'] as String, r['field'] as String),
    ];
    final seen = logged.toSet();
    final pending = [
      for (final e in _shown)
        if (e.field.startsWith(prefix) && seen.add((e.entity, e.field)))
          (e.entity, e.field),
    ];
    if (pending.isEmpty) return logged;
    return [...logged, ...pending]..sort((a, b) {
      final byEntity = _compareBytes(a.$1, b.$1);
      return byEntity != 0 ? byEntity : _compareBytes(a.$2, b.$2);
    });
  }

  /// Which Device wrote which of [fields] on which entity, as (entity,
  /// field, device), each once.
  List<(String, String, String)> writersOf(List<String> fields) {
    final logged = [
      for (final r in _db.select(
        'SELECT DISTINCT entity, field, device FROM entries '
        'WHERE $_applied AND field IN '
        '(${List.filled(fields.length, '?').join(', ')})',
        fields,
      ))
        (r['entity'] as String, r['field'] as String, r['device'] as String),
    ];
    final seen = logged.toSet();
    return [
      ...logged,
      for (final e in _shown)
        if (fields.contains(e.field) && seen.add((e.entity, e.field, e.device)))
          (e.entity, e.field, e.device),
    ];
  }

  /// Whether any entry ever wrote [field].
  bool mentions(String field) =>
      _db.select(
        'SELECT 1 FROM entries WHERE $_applied AND field = ? LIMIT 1',
        [field],
      ).isNotEmpty ||
      _shown.any((e) => e.field == field);

  /// The log's high-water mark: the local handle of the newest entry, 0
  /// for an empty log. `seq` is AUTOINCREMENT, so a mark keeps meaning
  /// the same moment.
  int currentSeq() =>
      _db.select('SELECT COALESCE(MAX(seq), 0) AS n FROM entries').first['n']
          as int;

  // ------------------------------------------------------------ moments

  /// Records a Moment at the mark [seq], at the store's now unless [at]
  /// says when (a restored one): why ([cause]) and with whom or what
  /// ([label]). Returns its id.
  int addMoment({
    required int seq,
    required String cause,
    String? label,
    DateTime? at,
  }) {
    _db.execute(
      'INSERT INTO moments (seq, cause, label, at) VALUES (?, ?, ?, ?)',
      [seq, cause, label, (at ?? clock()).toUtc().toIso8601String()],
    );
    return _db.lastInsertRowId;
  }

  /// Every Moment as stored, newest first.
  List<({int id, int seq, String cause, String? label, DateTime at})>
  momentRows() => [
    for (final r in _db.select(
      'SELECT id, seq, cause, label, at FROM moments ORDER BY seq DESC, id DESC',
    ))
      (
        id: r['id'] as int,
        seq: r['seq'] as int,
        cause: r['cause'] as String,
        label: r['label'] as String?,
        at: DateTime.parse(r['at'] as String),
      ),
  ];

  /// Every entry written after the mark [seq], in log order: what a
  /// Moment at [seq] is the state before.
  List<Entry> entriesAfter(int seq) =>
      _entries('SELECT * FROM entries WHERE seq > ? ORDER BY seq', [seq]);

  /// Runs [body] in one transaction.
  T batch<T>(T Function() body) {
    _db.execute('BEGIN');
    _inBatch = true;
    final T result;
    try {
      result = body();
      _db.execute('COMMIT');
    } catch (_) {
      _db.execute('ROLLBACK');
      _judged = null;
      rethrow;
    } finally {
      _inBatch = false;
    }
    _changed();
    return result;
  }

  /// How far this Device holds each Device's entries, as it tells other
  /// Devices: the newest dseq held, raised to the newest a blocked
  /// person's Device had refused, so nobody sends them again; after an
  /// unblock, below the first refused one that is not back yet and lay in
  /// a Shared Area this Device still holds a role in, so they are sent
  /// again (ADR 0021).
  Map<String, int> versionVector() {
    final vector = _heldVector();
    for (final (device, mark) in refusalMarks()) {
      final held = vector[device] ?? 0;
      if (!mark.released) {
        if (mark.high > held) vector[device] = mark.high;
        continue;
      }
      final missing = _firstWanted(device, mark);
      if (missing == null || missing > held) continue;
      if (missing - 1 > 0) {
        vector[device] = missing - 1;
      } else {
        vector.remove(device);
      }
    }
    return vector;
  }

  /// The first dseq of [mark]'s runs not held here whose scope this
  /// Device holds a role in; null when there is none.
  int? _firstWanted(String device, RefusalMark mark) {
    for (final run in mark.runs) {
      if (!_wants(run)) continue;
      final missing = _firstMissing(device, run.from, run.to);
      if (missing != null) return missing;
    }
    return null;
  }

  /// Whether what was refused in [run] can come back: it lay in a Shared
  /// Area this Device holds a role in now.
  bool _wants(RefusedRun run) =>
      judged.areas[run.scope]?.grantOf(deviceId) != null;

  /// The newest dseq held from each Device.
  Map<String, int> _heldVector() => {
    for (final row in _db.select(
      'SELECT device, MAX(dseq) AS m FROM entries GROUP BY device',
    ))
      row['device'] as String: row['m'] as int,
  };

  /// The lowest dseq of [device] from [low] to [high] not held; null when
  /// all of them are.
  int? _firstMissing(String device, int low, int high) {
    var next = low;
    for (final r in _db.select(
      'SELECT dseq FROM entries WHERE device = ? AND dseq BETWEEN ? AND ? '
      'ORDER BY dseq',
      [device, low, high],
    )) {
      if (r['dseq'] as int != next) return next;
      next++;
    }
    return next > high ? null : next;
  }

  // ------------------------------------------------------------- pending

  /// The local setting that says a pending set was accepted, written with
  /// what Accept wrote: a restart before its file is gone deletes it.
  static const acceptedPendingPrefix = 'pending:';

  final _pending = <String, PendingSet>{};
  List<Entry>? _overlay;
  JudgedLog? _shownJudged;

  /// The set [id] as its file holds it now, and so as this store shows
  /// it from now on: another store open on the same file, such as the
  /// phone's background sync, may have changed or ended it. Null when it
  /// is gone, or accepted and its file not deleted yet.
  @internal
  PendingSet? freshPendingSet(String id) {
    final text = localSetting('$acceptedPendingPrefix$id') == null
        ? _pendingFiles.read(id)
        : null;
    final set = text == null ? null : PendingSet.parse(text);
    final fresh = set != null && set.id == id ? set : null;
    _cachePending(id, fresh);
    return fresh;
  }

  /// Every pending set as the files hold them now, the oldest first.
  @internal
  List<PendingSet> freshPendingSets() {
    final files = _pendingFiles.readAll();
    for (final id in _pending.keys.toList()) {
      if (!files.containsKey(id)) _cachePending(id, null);
    }
    for (final MapEntry(key: id, value: text) in files.entries) {
      final set = localSetting('$acceptedPendingPrefix$id') == null
          ? PendingSet.parse(text)
          : null;
      _cachePending(id, set != null && set.id == id ? set : null);
    }
    return sortedPendingSets();
  }

  /// Shows [set] as the set [id] from now on, none for null. What the
  /// reads worked out goes; nobody is told, since this comes from a read.
  void _cachePending(String id, PendingSet? set) {
    final held = _pending[id];
    if (set == null) {
      if (held == null) return;
      _pending.remove(id);
    } else {
      if (held != null && held.encode() == set.encode()) return;
      _pending[id] = set;
    }
    _overlay = null;
    _shownJudged = null;
  }

  /// The pending sets, the oldest first.
  @internal
  List<PendingSet> sortedPendingSets() =>
      _pending.values.toList()..sort((a, b) {
        final byMade = a.made.compareTo(b.made);
        return byMade != 0 ? byMade : a.id.compareTo(b.id);
      });

  /// Keeps [set] in its file and shows it where it is shown.
  @internal
  void keepPendingSet(PendingSet set) {
    _pendingFiles.write(set.id, set.encode());
    _pending[set.id] = set;
    _changed();
  }

  /// Deletes the set [id] and its file, and the mark of its Accept.
  @internal
  void forgetPendingSet(String id) {
    _pendingFiles.remove(id);
    removeLocalSetting('$acceptedPendingPrefix$id');
    if (_pending.remove(id) != null) _changed();
  }

  /// Takes in the sets a restart left. One accepted before its file was
  /// gone is gone now; a file that is no set this version reads stays on
  /// disk, unread.
  void _loadPending() {
    final files = _pendingFiles.readAll();
    for (final MapEntry(key: id, value: text) in files.entries) {
      if (localSetting('$acceptedPendingPrefix$id') != null) {
        forgetPendingSet(id);
        continue;
      }
      final set = PendingSet.parse(text);
      if (set != null && set.id == id) _pending[id] = set;
    }
    for (final (key, _) in localSettingsWithPrefix(acceptedPendingPrefix)) {
      final id = key.substring(acceptedPendingPrefix.length);
      if (!files.containsKey(id)) removeLocalSetting(key);
    }
  }

  /// The changes of the shown pending sets as entries, oldest set first,
  /// each set in the order its changes came: a draft as this Device's,
  /// numbered below zero, so it is numbered nowhere; a received entry
  /// as it came, unless the log holds it by now. Only what this version
  /// applies, and nothing an edit of the user's own made since gives way
  /// to at Accept ([PendingChanges.givesWay]): the reads show what Accept
  /// will keep.
  List<Entry> get _shown => _overlay ??= () {
    if (_pending.isEmpty) return const <Entry>[];
    final self = deviceId;
    final name = author;
    final own = <String, bool>{};
    final histories = <String, List<Entry>>{};
    var drafted = 0;
    final out = <Entry>[];
    for (final set in sortedPendingSets()) {
      if (!set.shown) continue;
      final ids = {for (final r in set.received) r.entry.id};
      bool givesWay(String entity, String field, int mark) => PendingChanges(
        this,
      ).givesWay(entity, field, mark, ids: ids, own: own, histories: histories);
      for (final change in set.changes) {
        switch (change) {
          case PendingDraft():
            if (givesWay(change.entity, change.field, change.mark)) continue;
            drafted++;
            out.add(
              Entry(
                seq: 0,
                device: self,
                dseq: -drafted,
                entity: change.entity,
                field: change.field,
                value: change.value,
                date: change.effectiveDate,
                author: name,
                recorded: change.made,
              ),
            );
          case PendingEntry(:final entry):
            if (entry.applies &&
                !holds(entry.device, entry.dseq) &&
                !givesWay(entry.entity, entry.field, change.mark)) {
              out.add(entry);
            }
        }
      }
    }
    return out;
  }();

  /// What the roles allow with the shown pending sets on top of the log:
  /// what the reads leave out, nothing else.
  JudgedLog get _displayJudged {
    final shown = _shown;
    if (shown.isEmpty) return judged;
    return _shownJudged ??=
        !shown.any((e) => e.field.startsWith('role:')) &&
            identical(judged, JudgedLog.none)
        ? JudgedLog.none
        : judgeLog([
            ..._entries('SELECT * FROM entries WHERE $_applied ORDER BY seq'),
            ...shown,
          ]);
  }

  /// [a] against [b] by their UTF-8 bytes, as SQLite orders text.
  static int _compareBytes(String a, String b) {
    final x = utf8.encode(a);
    final y = utf8.encode(b);
    for (var i = 0; i < x.length && i < y.length; i++) {
      if (x[i] != y[i]) return x[i] - y[i];
    }
    return x.length - y.length;
  }

  // ------------------------------------------------------------ refusals

  static const _refusedPrefix = 'refused:';

  /// Which entries of each of a blocked person's Devices were refused
  /// (ADR 0021), by device: a local setting of this Device, since each of
  /// the user's Devices refuses on its own. [RefusalMark.released] once
  /// the person was unblocked, until what was refused is back.
  List<(String, RefusalMark)> refusalMarks() => [
    for (final (key, value) in localSettingsWithPrefix(_refusedPrefix))
      if (RefusalMark.parse(value) case final mark?)
        (key.substring(_refusedPrefix.length), mark),
  ];

  RefusalMark? _refusalMark(String device) =>
      RefusalMark.parse(localSetting('$_refusedPrefix$device') ?? '');

  void _keepRefusalMark(String device, RefusalMark mark) {
    if (mark.runs.isEmpty) {
      removeLocalSetting('$_refusedPrefix$device');
    } else {
      setLocalSetting('$_refusedPrefix$device', mark.encode());
    }
  }

  /// Notes that the entry ([device], [dseq]) was refused, its entity
  /// lying in [scope] (an Area, or [personalScope]'s name): the mark of
  /// [device] takes it in, and counts as refusing again.
  void noteRefused(String device, int dseq, String scope) {
    final held = _refusalMark(device);
    final mark = (held ?? const RefusalMark(runs: [])).adding(dseq, scope);
    if (held != null && held.encode() == mark.encode()) return;
    _keepRefusalMark(device, mark);
  }

  /// Lets [device]'s mark go: what it refused is wanted again, until it
  /// is all back ([dropFilledRefusals]). Returns whether it was refusing.
  bool releaseRefusals(String device) {
    final held = _refusalMark(device);
    if (held == null || held.released) return false;
    _keepRefusalMark(device, RefusalMark(runs: held.runs, released: true));
    return true;
  }

  /// Makes [device]'s released mark refuse again.
  void holdRefusals(String device) {
    final held = _refusalMark(device);
    if (held == null || !held.released) return;
    _keepRefusalMark(device, RefusalMark(runs: held.runs));
  }

  /// Forgets, of every released mark, the runs that are back and those
  /// that cannot come back: lying in no Shared Area this Device holds a
  /// role in now. A mark with nothing left goes.
  void dropFilledRefusals() {
    for (final (device, mark) in refusalMarks()) {
      if (!mark.released) continue;
      final left = [
        for (final run in mark.runs)
          if (_wants(run) && _firstMissing(device, run.from, run.to) != null)
            run,
      ];
      if (left.length == mark.runs.length) continue;
      _keepRefusalMark(device, RefusalMark(runs: left, released: true));
    }
  }

  /// Every entry [device] wrote that is held here, by dseq.
  List<Entry> entriesOf(String device) => _entries(
    'SELECT * FROM entries WHERE device = ? ORDER BY dseq',
    [device],
  );

  /// Deletes the entries [dseqs] of [device] from the log: only a block
  /// does (ADR 0021), for what a blocked person wrote. Nothing else ever
  /// leaves the log.
  void deleteEntries(String device, Iterable<int> dseqs) {
    var deleted = 0;
    for (final dseq in dseqs) {
      _db.execute('DELETE FROM entries WHERE device = ? AND dseq = ?', [
        device,
        dseq,
      ]);
      deleted += _db.updatedRows;
    }
    if (deleted > 0) _changed();
  }

  /// The entities whose id starts with [prefix], sorted.
  List<String> entitiesWithPrefix(String prefix) {
    final logged = [
      for (final r in _db.select(
        'SELECT DISTINCT entity FROM entries '
        'WHERE $_applied AND substr(entity, 1, ?) = ? ORDER BY entity',
        [prefix.length, prefix],
      ))
        r['entity'] as String,
    ];
    final seen = logged.toSet();
    final pending = [
      for (final e in _shown)
        if (e.entity.startsWith(prefix) && seen.add(e.entity)) e.entity,
    ];
    if (pending.isEmpty) return logged;
    return [...logged, ...pending]..sort(_compareBytes);
  }

  /// Every entry of [entity] that counts, in log order, then those of
  /// the shown pending sets: what no role in a Shared Area allows is left
  /// out, judged with those sets too.
  List<Entry> history(String entity) {
    final shown = _shown;
    if (shown.isEmpty) return loggedHistory(entity);
    final all = [
      ..._entries(
        'SELECT * FROM entries WHERE $_applied AND entity = ? ORDER BY seq',
        [entity],
      ),
      for (final e in shown)
        if (e.entity == entity) e,
    ];
    final ignored = _displayJudged.ignored;
    if (ignored.isEmpty) return all;
    return [
      for (final e in all)
        if (!ignored.contains(e.id)) e,
    ];
  }

  /// Every entry of [entity] in the log that counts, in log order, no
  /// pending change among them: what decides where an entity is sealed,
  /// and what the user's own Devices wrote.
  List<Entry> loggedHistory(String entity) {
    final all = _entries(
      'SELECT * FROM entries WHERE $_applied AND entity = ? ORDER BY seq',
      [entity],
    );
    final ignored = judged.ignored;
    if (ignored.isEmpty) return all;
    return [
      for (final e in all)
        if (!ignored.contains(e.id)) e,
    ];
  }

  /// The value of [field] on [entity] in the log alone, latest wins.
  String? logged(String entity, String field) {
    Entry? held;
    for (final e in loggedHistory(entity)) {
      if (e.field == field && (held == null || _newer(e, held))) held = e;
    }
    return held?.value;
  }

  /// Every entry of [entities] as it is, none left out, by (device,
  /// dseq): what is relayed to another Device, which judges for itself.
  List<Entry> rawHistoryOf(Iterable<String> entities) {
    final ids = entities.toSet();
    return [
      for (final e in _entries('SELECT * FROM entries ORDER BY device, dseq'))
        if (ids.contains(e.entity)) e,
    ];
  }

  JudgedLog? _judged;

  /// What the roles of the Shared Areas allow (ADR 0013), worked out
  /// from the whole log after every change; nothing to work out while no
  /// Area was ever shared. Pending Changes never count here: this is
  /// what decides who may do what, what is refused and what is deleted.
  JudgedLog get judged => _judged ??=
      _db
          .select(
            "SELECT 1 FROM entries WHERE $_applied "
            "AND substr(field, 1, 5) = 'role:' LIMIT 1",
          )
          .isEmpty
      ? JudgedLog.none
      : judgeLog(
          _entries('SELECT * FROM entries WHERE $_applied ORDER BY seq'),
        );

  /// Every entry, in log order, those of a later structure too: what a
  /// Backup carries.
  List<Entry> allEntries() => _entries('SELECT * FROM entries ORDER BY seq');

  /// The current value of [field] on [entity] — latest wins.
  String? current(String entity, String field) => currentFields(entity)[field];

  /// The current value of every field on [entity] — latest wins.
  Map<String, String?> currentFields(String entity) {
    final latest = <String, Entry>{};
    for (final e in history(entity)) {
      final held = latest[e.field];
      if (held == null || _newer(e, held)) latest[e.field] = e;
    }
    return {
      for (final MapEntry(:key, :value) in latest.entries) key: value.value,
    };
  }

  /// Entities whose current `$type` is [type].
  List<String> entitiesOfType(String type) {
    final ids = {
      for (final r in _db.select(
        'SELECT DISTINCT entity FROM entries WHERE $_applied '
        r"AND field = '$type' AND value = ?",
        [type],
      ))
        r['entity'] as String,
      for (final e in _shown)
        if (e.field == r'$type' && e.value == type) e.entity,
    };
    return [
      for (final id in ids)
        if (current(id, r'$type') == type) id,
    ];
  }

  /// Latest-wins order: date, then recording time, then device, then
  /// dseq. Sorting a field's entries by it puts the current value last.
  static int latestWins(Entry a, Entry b) {
    final byDate = a.date.compareTo(b.date);
    if (byDate != 0) return byDate;
    final byRecorded = a.recorded.compareTo(b.recorded);
    if (byRecorded != 0) return byRecorded;
    final byDevice = a.device.compareTo(b.device);
    if (byDevice != 0) return byDevice;
    return a.dseq.compareTo(b.dseq);
  }

  static bool _newer(Entry a, Entry b) => latestWins(a, b) > 0;

  List<Entry> _entries(String sql, [List<Object?> args = const []]) => [
    for (final r in _db.select(sql, args))
      Entry(
        seq: r['seq'] as int,
        device: r['device'] as String,
        dseq: r['dseq'] as int,
        entity: r['entity'] as String,
        field: r['field'] as String,
        value: r['value'] as String?,
        date: DateTime.parse(r['date'] as String),
        author: r['author'] as String,
        recorded: DateTime.parse(r['recorded'] as String),
        sig: r['sig'] as String?,
        ctx: r['ctx'] as String?,
      ).withRaw(r['raw'] as String?),
  ];

  /// The entries this version applies (ADR 0019): those of a later
  /// structure are only held and passed on, so they count for what is
  /// held (`holds`, the version vector, the deltas) and nothing else.
  static const _applied = '(v IS NULL OR v <= $entryVersion)';

  static final _random = Random.secure();

  static String _randomId() =>
      hex(List.generate(16, (_) => _random.nextInt(256)));

  /// Makes the ids of new entities: random, unless the fixture generator
  /// hands in a counter so its logs come out the same on every run.
  static String Function() idMaker = _randomId;

  /// A fresh id for a new entity.
  static String newId() => idMaker();
}

/// The dseqs [from] to [to] of one Device, all refused, whose entities
/// lay in [scope]: an Area, or `personal`.
class RefusedRun {
  final int from;
  final int to;
  final String scope;

  const RefusedRun(this.from, this.to, this.scope);
}

/// Which entries of a blocked person's Device were refused here (ADR
/// 0021): the [runs] of dseqs, by the scope their entities lay in, and
/// whether the person was unblocked since, so those entries are wanted
/// again.
class RefusalMark {
  final List<RefusedRun> runs;
  final bool released;

  const RefusalMark({required this.runs, this.released = false});

  /// The newest dseq refused; 0 for none.
  int get high => runs.fold(0, (m, r) => r.to > m ? r.to : m);

  /// This mark refusing again, with [dseq] of [scope] among its runs.
  RefusalMark adding(int dseq, String scope) {
    final scopes = <int, String>{
      for (final run in runs)
        for (var n = run.from; n <= run.to; n++) n: run.scope,
    };
    scopes[dseq] = scope;
    final sorted = scopes.entries.toList()
      ..sort((a, b) => a.key.compareTo(b.key));
    final next = <RefusedRun>[];
    for (final MapEntry(key: n, value: scope) in sorted) {
      final last = next.isEmpty ? null : next.last;
      if (last != null && last.to == n - 1 && last.scope == scope) {
        next[next.length - 1] = RefusedRun(last.from, n, scope);
      } else {
        next.add(RefusedRun(n, n, scope));
      }
    }
    return RefusalMark(runs: next);
  }

  /// Keys sorted, no spaces, as both cores keep it.
  String encode() => jsonEncode({
    if (released) 'released': true,
    'runs': [
      for (final r in runs) {'from': r.from, 'scope': r.scope, 'to': r.to},
    ],
  });

  /// The mark [text] holds; null when it is none.
  static RefusalMark? parse(String text) {
    try {
      final json = jsonDecode(text);
      if (json case {'runs': final List list}) {
        final runs = <RefusedRun>[];
        for (final r in list) {
          if (r
              case {
                'from': final int from,
                'scope': final String scope,
                'to': final int to,
              }
              when from >= 1 && to >= from) {
            runs.add(RefusedRun(from, to, scope));
          } else {
            return null;
          }
        }
        return RefusalMark(runs: runs, released: json['released'] == true);
      }
    } on FormatException {
      return null;
    }
    return null;
  }
}
