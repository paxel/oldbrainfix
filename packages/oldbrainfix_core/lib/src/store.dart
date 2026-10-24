import 'dart:io';
import 'dart:math';
import 'dart:typed_data';

import 'package:crypto/crypto.dart';
import 'package:sqlite3/sqlite3.dart';

import 'entry.dart';
import 'photos.dart';
import 'signing.dart';

/// The log: an append-only entry log over SQLite with a latest-wins
/// projection (ADR 0002). All reads and writes are synchronous.
///
/// The store never reads the wall clock on its own: [clock] is handed in,
/// so tests and the fixture generator get a reproducible log.
///
/// Photo bytes live outside the log, content-addressed by SHA-256: a
/// `photos` directory next to the database file, or memory for
/// [Store.inMemory].
class Store {
  final Database _db;
  final PhotoBlobs _photos;

  /// Where the store reads "now" for [Entry.recorded] and default dates.
  final DateTime Function() clock;

  Store._(this._db, this.clock, this._photos);

  final _listeners = <void Function()>[];
  var _inBatch = false;

  /// Calls [listener] after every change to the log: after each append,
  /// or once a [batch] is committed. Local settings do not count.
  void addListener(void Function() listener) => _listeners.add(listener);

  void removeListener(void Function() listener) => _listeners.remove(listener);

  void _changed() {
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
    FilePhotoBlobs('${File(path).parent.path}/photos'),
  );

  /// Opens a throwaway in-memory store (tests).
  factory Store.inMemory({
    DateTime Function() clock = DateTime.now,
    SigningKey Function() newKey = SigningKey.generate,
  }) => _init(sqlite3.openInMemory(), clock, newKey, MemoryPhotoBlobs());

  static Store _init(
    Database db,
    DateTime Function() clock,
    SigningKey Function() newKey,
    PhotoBlobs photos,
  ) {
    db.execute('''
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
        sig      TEXT
      );
      CREATE TABLE IF NOT EXISTS local_settings (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
      );
      CREATE UNIQUE INDEX IF NOT EXISTS idx_entries_device_dseq
        ON entries (device, dseq);
      CREATE INDEX IF NOT EXISTS idx_entries_entity_field
        ON entries (entity, field);
    ''');
    final store = Store._(db, clock, photos);
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

  // ----------------------------------------------------------------- log

  /// Appends one fact under this Device, signed. [date] defaults to now.
  Entry append(String entity, String field, String? value, {DateTime? date}) {
    final now = clock().toUtc();
    final device = deviceId;
    final dseq = (versionVector()[device] ?? 0) + 1;
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
      ),
    );
    _db.execute(
      'INSERT INTO entries (device, dseq, entity, field, value, date, author, recorded, sig) '
      'VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)',
      [device, dseq, entity, field, value, dateIso, author, recordedIso, sig],
    );
    _changed();
    return _entries('SELECT * FROM entries WHERE device = ? AND dseq = ?', [
      device,
      dseq,
    ]).single;
  }

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

  /// Every entry of [entity], in log order.
  List<Entry> history(String entity) =>
      _entries('SELECT * FROM entries WHERE entity = ? ORDER BY seq', [entity]);

  /// Every entry, in log order.
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
          r"SELECT DISTINCT entity FROM entries WHERE field = '$type' AND value = ?",
          [type],
        )
        .map((r) => r['entity'] as String);
    return [
      for (final id in ids)
        if (current(id, r'$type') == type) id,
    ];
  }

  /// Latest-wins order: date, then recording time, then device, then dseq.
  static bool _newer(Entry a, Entry b) {
    final byDate = a.date.compareTo(b.date);
    if (byDate != 0) return byDate > 0;
    final byRecorded = a.recorded.compareTo(b.recorded);
    if (byRecorded != 0) return byRecorded > 0;
    final byDevice = a.device.compareTo(b.device);
    if (byDevice != 0) return byDevice > 0;
    return a.dseq > b.dseq;
  }

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
      ),
  ];

  static final _random = Random.secure();

  /// A fresh random id for a new entity.
  static String newId() => hex(List.generate(16, (_) => _random.nextInt(256)));
}
