import 'dart:io';
import 'dart:math';
import 'dart:typed_data';

import 'package:crypto/crypto.dart';
import 'package:sqlite3/sqlite3.dart';

import 'blobs.dart';
import 'conflicts.dart';
import 'entry.dart';
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
class Store {
  final Database _db;
  final Blobs _photos;
  final Blobs _audio;

  /// Where the store reads "now" for [Entry.recorded] and default dates.
  final DateTime Function() clock;

  Store._(this._db, this.clock, this._photos, this._audio);

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
  );

  static Store _init(
    Database db,
    DateTime Function() clock,
    SigningKey Function() newKey,
    Blobs photos,
    Blobs audio,
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
    final store = Store._(db, clock, photos, audio);
    if (store.localSetting('device') == null) {
      final key = newKey();
      store.setLocalSetting('device', deviceIdFromKey(key.publicKey));
      store.setLocalSetting('signseed', key.seedHex);
    }
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
    final vector = versionVector();
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
  /// the unit here; null when nobody wrote it yet.
  String? _context(
    String entity,
    String unit,
    String device,
    Map<String, int> vector,
  ) {
    final fields = conflictUnitFields(unit);
    final writers = {
      for (final r in _db.select(
        'SELECT DISTINCT device FROM entries WHERE $_applied AND entity = ? '
        'AND field IN '
        '(${List.filled(fields.length, '?').join(', ')})',
        [entity, ...fields],
      ))
        r['device'] as String,
    };
    if (writers.isEmpty) return null;
    return encodeContext({
      for (final d in writers)
        if (d != device) d: vector[d] ?? 0,
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
  List<(String, String)> fieldsWithPrefix(String prefix) => [
    for (final r in _db.select(
      'SELECT DISTINCT entity, field FROM entries '
      'WHERE $_applied AND substr(field, 1, ?) = ? '
      'ORDER BY entity, field',
      [prefix.length, prefix],
    ))
      (r['entity'] as String, r['field'] as String),
  ];

  /// Which Device wrote which of [fields] on which entity, as (entity,
  /// field, device), each once.
  List<(String, String, String)> writersOf(List<String> fields) => [
    for (final r in _db.select(
      'SELECT DISTINCT entity, field, device FROM entries '
      'WHERE $_applied AND field IN '
      '(${List.filled(fields.length, '?').join(', ')})',
      fields,
    ))
      (r['entity'] as String, r['field'] as String, r['device'] as String),
  ];

  /// Whether any entry ever wrote [field].
  bool mentions(String field) => _db.select(
    'SELECT 1 FROM entries WHERE $_applied AND field = ? LIMIT 1',
    [field],
  ).isNotEmpty;

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

  /// The newest dseq held from each Device.
  Map<String, int> versionVector() => {
    for (final row in _db.select(
      'SELECT device, MAX(dseq) AS m FROM entries GROUP BY device',
    ))
      row['device'] as String: row['m'] as int,
  };

  /// Every entry of [entity] that counts, in log order: what no role in
  /// a Shared Area allows is left out ([judged]).
  List<Entry> history(String entity) {
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
  /// Area was ever shared.
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
    final ids = _db
        .select(
          'SELECT DISTINCT entity FROM entries WHERE $_applied '
          r"AND field = '$type' AND value = ?",
          [type],
        )
        .map((r) => r['entity'] as String);
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
