import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'area_keys.dart';
import 'conflicts.dart';
import 'entry.dart';
import 'signing.dart';
import 'store.dart';
import 'sync.dart';

/// The shared folder as the sync sees it: an `oldbrainfix-sync` root
/// with `keys` and `blobs` inside. Plain files on the desktop; on
/// Android the document tree the folder picker granted, where nothing is
/// a path. Every call may throw a [FileSystemException] when the folder
/// is gone. Copied from Catlog and narrowed.
abstract interface class SyncFolder {
  /// The file names directly in [dir] (`''` for the root, `keys`,
  /// `blobs`), sorted; an absent directory lists as empty.
  Future<List<String>> list(String dir);

  /// The file's bytes, or null when it is not there.
  Future<Uint8List?> read(String dir, String name);

  /// Writes [bytes] as [name] in [dir], replacing what was there.
  Future<void> write(String dir, String name, List<int> bytes);

  Future<void> delete(String dir, String name);

  /// Makes sure [dir] exists (`''` makes the root).
  Future<void> ensure(String dir);
}

/// The name of the folder everything lives in, inside the one picked.
const syncFolderRoot = 'oldbrainfix-sync';

/// The folder as plain files under `<path>/oldbrainfix-sync`.
class LocalSyncFolder implements SyncFolder {
  final String root;

  LocalSyncFolder(String path) : root = '$path/$syncFolderRoot';

  Directory _dir(String dir) => Directory(dir.isEmpty ? root : '$root/$dir');

  /// The file [name] in [dir]; a name from the folder ("../x", read from
  /// a manifest) never leaves it.
  File _file(String dir, String name) {
    if (name.isEmpty ||
        name == '.' ||
        name == '..' ||
        name.contains('/') ||
        name.contains(r'\')) {
      throw FileSystemException('Not a folder file', name);
    }
    return File('${_dir(dir).path}/$name');
  }

  @override
  Future<List<String>> list(String dir) async {
    final d = _dir(dir);
    if (!d.existsSync()) return const [];
    // Sorted: a directory's own order is the file system's, and both
    // cores must read one folder the same way.
    return [
      for (final f in d.listSync().whereType<File>()) f.uri.pathSegments.last,
    ]..sort();
  }

  @override
  Future<Uint8List?> read(String dir, String name) async {
    final f = _file(dir, name);
    return f.existsSync() ? f.readAsBytesSync() : null;
  }

  @override
  Future<void> write(String dir, String name, List<int> bytes) async {
    // Beside, then renamed: a cloud client must never upload half a
    // file as the whole.
    final target = _file(dir, name);
    final tmp = File('${target.path}.tmp');
    tmp.writeAsBytesSync(bytes, flush: true);
    tmp.renameSync(target.path);
  }

  @override
  Future<void> delete(String dir, String name) async {
    final f = _file(dir, name);
    if (f.existsSync()) f.deleteSync();
  }

  /// Makes [dir], inside the picked folder only: a folder that was moved
  /// or deleted is gone, not made again where nobody syncs it.
  @override
  Future<void> ensure(String dir) async {
    final picked = Directory(root).parent;
    if (!picked.existsSync()) {
      throw FileSystemException('The folder is gone', picked.path);
    }
    _dir(dir).createSync(recursive: true);
  }
}

/// A folder in memory: tests, and the fixture generator.
class MemorySyncFolder implements SyncFolder {
  final Map<String, Map<String, Uint8List>> dirs = {};

  @override
  Future<List<String>> list(String dir) async =>
      (dirs[dir]?.keys.toList() ?? <String>[])..sort();

  @override
  Future<Uint8List?> read(String dir, String name) async => dirs[dir]?[name];

  @override
  Future<void> write(String dir, String name, List<int> bytes) async =>
      dirs.putIfAbsent(dir, () => {})[name] = Uint8List.fromList(bytes);

  @override
  Future<void> delete(String dir, String name) async => dirs[dir]?.remove(name);

  @override
  Future<void> ensure(String dir) async => dirs.putIfAbsent(dir, () => {});
}

/// What one folder sync did.
class FolderSyncResult {
  /// The entries that were new here.
  final List<Entry> applied;
  final int entriesOut;
  final int blobsIn;
  final int blobsOut;

  /// Segments of other Devices sealed with a key this Device does not
  /// hold: it was not paired with them, or their keys are on the way.
  final int unreadable;
  final Moment? moment;
  final ImportReport report;
  final List<Conflict> conflicts;

  const FolderSyncResult({
    required this.applied,
    required this.entriesOut,
    required this.blobsIn,
    required this.blobsOut,
    required this.unreadable,
    required this.moment,
    required this.report,
    required this.conflicts,
  });
}

/// A Device's history in the folder is a run of segments per key
/// (Catlog's ADR 0010, ADR 0012): `<device>.<key id>.<n>.seg`, one
/// sealed entry per line, appended to until one passes [segmentCap] and
/// the next is opened; a full segment never changes again. The manifest
/// beside them, `<device>.manifest`, says what the writer knows and
/// which segments exist; it is not sealed.
const segmentCap = 64 * 1024;

/// The manifest's format; a reader skips a higher one.
const manifestFormat = 1;

String manifestName(String device) => '$device.manifest';
String segmentName(String device, String key, int n) => '$device.$key.$n.seg';

/// The keys a Device publishes, sealed with one of the personal keys:
/// `keys/<device>.<key id>.keys`.
String keysFileName(String device, String key) => '$device.$key.keys';

/// One segment as the manifest names it.
typedef Segment = ({String name, String key, int lines});

/// What a Device says about its segments.
class FolderManifest {
  final int format;

  /// Moves on whenever the segments were written afresh from the start;
  /// a reader then forgets how far it read.
  final int generation;

  /// The writer's version vector.
  final Map<String, int> vector;

  /// The segments with the lines each held, by key and number.
  final List<Segment> segments;

  const FolderManifest({
    this.format = manifestFormat,
    required this.generation,
    required this.vector,
    required this.segments,
  });

  /// Its bytes, the same from both cores: keys sorted, no spaces.
  Uint8List encode() => utf8.encode(
    jsonEncode({
      'format': format,
      'generation': generation,
      'segments': [
        for (final s in segments)
          {'key': s.key, 'lines': s.lines, 'name': s.name},
      ],
      'vector': {for (final d in vector.keys.toList()..sort()) d: vector[d]},
    }),
  );

  static FolderManifest? parse(List<int>? bytes) {
    if (bytes == null) return null;
    try {
      final m = jsonDecode(utf8.decode(bytes));
      if (m is! Map) return null;
      final format = m['format'];
      final generation = m['generation'];
      final vector = m['vector'];
      final segments = m['segments'];
      if (format is! int || format > manifestFormat) return null;
      if (generation is! int || vector is! Map || segments is! List) {
        return null;
      }
      return FolderManifest(
        format: format,
        generation: generation,
        vector: {
          for (final MapEntry(:key, :value) in vector.entries)
            if (key is String && value is int) key: value,
        },
        segments: [
          for (final s in segments)
            if (s case {
              'name': final String name,
              'key': final String key,
              'lines': final int lines,
            })
              (name: name, key: key, lines: lines),
        ],
      );
    } on FormatException {
      return null;
    }
  }
}

/// Syncs through the shared folder (ADR 0003, 0012): learns the keys
/// the other Devices published, takes in what is new in their segments
/// (checked against their signatures and applied as one batch, as a LAN
/// import, with a Moment before it), then publishes this Device's own
/// keys, entries and blobs, and fetches the blobs the log lacks.
///
/// Everything in the folder but the manifests is sealed: the entries
/// with the key of their Area, the keys under the personal key, the
/// blobs under their Note's key. A Device holding no key reads nothing.
/// Every nonce comes from [nonce], in this order: the keys files by key
/// id, the entry lines in log order, the blobs by reference; the fixture
/// generator hands in a counter.
Future<FolderSyncResult> folderSync(
  Store store,
  SyncFolder folder, {
  Uint8List Function()? nonce,
}) async {
  final nextNonce = nonce ?? () => secretBytes(sealNonceBytes);
  await folder.ensure('');
  await folder.ensure('keys');
  await folder.ensure('blobs');
  if (await folder.read('', '.nomedia') == null) {
    // Keeps the photos out of the phone's gallery.
    await folder.write('', '.nomedia', const []);
  }
  final report = ImportReport();
  await _learnKeys(store, folder, report);
  final before = store.currentSeq();
  final (applied, unreadable) = await _readForeign(store, folder, report);
  final moment = store.momentFor(
    before: before,
    changed: applied.isNotEmpty,
    cause: MomentCause.folder,
  );
  store.ensureAreaKeys();
  await _publishKeys(store, folder, nextNonce);
  final entriesOut = await _publishEntries(store, folder, nextNonce);
  final names = (await folder.list('blobs')).toSet();
  final blobsIn = await _fetchBlobs(store, folder, names);
  final blobsOut = await _publishBlobs(store, folder, names, nextNonce);
  return FolderSyncResult(
    applied: applied,
    entriesOut: entriesOut,
    blobsIn: blobsIn,
    blobsOut: blobsOut,
    unreadable: unreadable,
    moment: moment,
    report: report,
    conflicts: store.conflictsAmong(applied),
  );
}

/// The (device, key id) a file name `<device>.<key id>.<rest>` names.
(String, String)? _deviceAndKey(String name) {
  final parts = name.split('.');
  if (parts.length < 3 || parts[0].isEmpty || parts[1].isEmpty) return null;
  return (parts[0], parts[1]);
}

/// Opens every other Device's keys file sealed with a personal key held
/// here and takes in what it carries, until nothing new comes: a key
/// learned from one file may open another.
Future<void> _learnKeys(
  Store store,
  SyncFolder folder,
  ImportReport report,
) async {
  final self = store.deviceId;
  final files = [
    for (final name in await folder.list('keys'))
      if (name.endsWith('.keys'))
        if (_deviceAndKey(name) case (final device, final key)
            when device != self)
          (name, device, key),
  ];
  final opened = <String>{};
  var learned = true;
  while (learned) {
    learned = false;
    for (final (name, device, id) in files) {
      if (opened.contains(name)) continue;
      final key = store.areaKey(id);
      if (key == null || key.scope != personalScope) continue;
      final Uint8List? plain;
      try {
        final bytes = await folder.read('keys', name);
        plain = bytes == null
            ? null
            : unseal(key.key, bytes, aad: SealedAs.keys(device));
      } on FileSystemException {
        continue;
      }
      if (plain == null) continue; // still on its way, or not this key
      opened.add(name);
      final Object? json;
      try {
        json = jsonDecode(utf8.decode(plain));
      } on FormatException {
        continue;
      }
      if (json is! Map) continue;
      store.learnKeys([
        for (final k in json['signing'] is List ? json['signing'] : const [])
          ?KeyRecord.fromJson(k),
      ], report: report);
      if (store.learnAreaKeys(areaKeysOf(json['areaKeys'])) > 0) {
        learned = true;
      }
    }
  }
}

/// What this Device remembers of a writer's segments: the generation it
/// read and how many lines of each segment it took. A local setting,
/// never synced.
String _readStateKey(String device) => 'folderRead:$device';

({int generation, Map<String, int> lines}) _readState(
  Store store,
  String device,
) {
  final raw = store.localSetting(_readStateKey(device));
  if (raw != null) {
    try {
      final m = jsonDecode(raw);
      if (m case {
        'generation': final int generation,
        'lines': final Map lines,
      }) {
        return (
          generation: generation,
          lines: {
            for (final MapEntry(:key, :value) in lines.entries)
              if (key is String && value is int) key: value,
          },
        );
      }
    } on FormatException {
      // A damaged memory reads as none: the segments are read whole.
    }
  }
  return (generation: -1, lines: const {});
}

/// Reads every other Device's segments sealed with a key held here,
/// from where this Device left off, and applies what is new. A segment
/// the manifest names that is not there yet, shorter than announced or
/// damaged is a delivery in progress: that Device waits a round. A
/// segment of a key not held is counted as unreadable and left unread,
/// to be read once the key is here. Entries are never filtered by the
/// version vector: a segment of a key learned late fills a gap below it,
/// and only the store knows what it holds.
Future<(List<Entry>, int)> _readForeign(
  Store store,
  SyncFolder folder,
  ImportReport report,
) async {
  final self = store.deviceId;
  final fresh = <Entry>[];
  final states = <String, Map<String, Object>>{};
  var unreadable = 0;
  for (final name in await folder.list('')) {
    if (!name.endsWith('.manifest')) continue;
    final device = name.substring(0, name.length - '.manifest'.length);
    if (device == self || device.isEmpty) continue;
    final FolderManifest? manifest;
    try {
      manifest = FolderManifest.parse(await folder.read('', name));
    } on FileSystemException {
      continue;
    }
    if (manifest == null) continue; // half-written, or a newer format
    var state = _readState(store, device);
    if (state.generation != manifest.generation) {
      state = (generation: manifest.generation, lines: const {});
    }
    final lines = Map.of(state.lines);
    final entries = <Entry>[];
    var complete = true;
    for (final segment in manifest.segments) {
      final have = state.lines[segment.name] ?? 0;
      if (segment.lines <= have) continue;
      final key = store.areaKey(segment.key);
      if (key == null) {
        unreadable++;
        continue;
      }
      final got = await _segmentEntries(folder, segment, have, key.key);
      if (got == null) {
        complete = false;
        break;
      }
      entries.addAll(got);
      lines[segment.name] = segment.lines;
    }
    if (!complete) continue;
    fresh.addAll(entries);
    states[device] = {'generation': manifest.generation, 'lines': lines};
  }
  // Remembered after: a round cut short reads the lines again, and the
  // store skips what it holds.
  final applied = store.applyEntries(fresh, report: report);
  for (final MapEntry(:key, :value) in states.entries) {
    store.setLocalSetting(_readStateKey(key), jsonEncode(value));
  }
  return (applied, unreadable);
}

/// The entries of [segment] from line [have] on, or null when it is not
/// all there yet or does not open.
Future<List<Entry>?> _segmentEntries(
  SyncFolder folder,
  Segment segment,
  int have,
  List<int> key,
) async {
  try {
    final bytes = await folder.read('', segment.name);
    if (bytes == null) return null;
    final lines = const LineSplitter().convert(utf8.decode(bytes));
    if (lines.length < segment.lines) return null;
    final entries = <Entry>[];
    for (final line in lines.sublist(have, segment.lines)) {
      final plain = unseal(key, base64.decode(line), aad: SealedAs.entry);
      if (plain == null) return null;
      final json = jsonDecode(utf8.decode(plain));
      if (json is! Map) return null;
      entries.add(Entry.fromJson(json.cast<String, dynamic>()));
    }
    return entries;
  } on FormatException {
    return null;
  } on FileSystemException {
    return null;
  } on TypeError {
    // A line that opens but is no entry.
    return null;
  }
}

/// Publishes the keys this Device holds, its own signing key and every
/// one it pinned, sealed once with each personal key it holds, so a
/// Device holding any of them learns the rest. A file that already says
/// the same stays untouched.
Future<void> _publishKeys(
  Store store,
  SyncFolder folder,
  Uint8List Function() nonce,
) async {
  final self = store.deviceId;
  final keys = store.areaKeys();
  final plain = utf8.encode(
    jsonEncode({
      'areaKeys': [for (final k in keys) k.toJson()],
      'signing': [for (final k in store.keyRecords()) k.toJson()],
    }),
  );
  for (final key in keys) {
    if (key.scope != personalScope) continue;
    final name = keysFileName(self, key.id);
    final held = await folder.read('keys', name);
    final said = held == null
        ? null
        : unseal(key.key, held, aad: SealedAs.keys(self));
    if (said != null && _same(said, plain)) continue;
    await folder.write(
      'keys',
      name,
      seal(key.key, plain, aad: SealedAs.keys(self), nonce: nonce()),
    );
  }
}

bool _same(List<int> a, List<int> b) {
  if (a.length != b.length) return false;
  for (var i = 0; i < a.length; i++) {
    if (a[i] != b[i]) return false;
  }
  return true;
}

/// Writes what is new in this Device's log to its segments: each entry
/// sealed with the key of its Area as it is now, appended to that key's
/// last segment, or a fresh one once the last passed [segmentCap]; the
/// manifest follows. Every Device's segments carry all it knows, so two
/// Devices sharing the folder converge without meeting. Without a
/// manifest of its own that it can read, it writes everything afresh
/// under the next generation. Returns how many entries went out.
Future<int> _publishEntries(
  Store store,
  SyncFolder folder,
  Uint8List Function() nonce,
) async {
  final self = store.deviceId;
  final previous = FolderManifest.parse(
    await folder.read('', manifestName(self)),
  );
  final present = (await folder.list('')).toSet();
  // A segment of its own that went missing is written again with all
  // the rest: readers would wait for it forever.
  final afresh =
      previous == null ||
      previous.format != manifestFormat ||
      !previous.segments.every((s) => present.contains(s.name));
  final entries = store.entriesSince(afresh ? const {} : previous.vector);
  if (!afresh && entries.isEmpty) return 0;

  // Sealed in log order, kept by key.
  final keyOf = <String, AreaKey?>{};
  final lines = <String, List<String>>{};
  final keys = <String, AreaKey>{};
  var out = 0;
  for (final e in entries) {
    final key = keyOf.putIfAbsent(e.entity, () => store.sealingKey(e.entity));
    if (key == null) continue;
    keys[key.id] = key;
    final sealed = seal(
      key.key,
      utf8.encode(jsonEncode(e.toJson())),
      aad: SealedAs.entry,
      nonce: nonce(),
    );
    lines.putIfAbsent(key.id, () => []).add(base64.encode(sealed));
    out++;
  }

  final segments = <Segment>[];
  if (afresh) {
    for (final name in present) {
      if (name.startsWith('$self.') && name.endsWith('.seg')) {
        await folder.delete('', name);
      }
    }
  } else {
    segments.addAll(previous.segments);
  }
  for (final id in lines.keys.toList()..sort()) {
    final fresh = lines[id]!;
    final run = [
      for (final s in segments)
        if (s.key == id) s,
    ];
    var n = run.length;
    var held = <String>[];
    if (run.isNotEmpty) {
      final tail = run.last;
      final bytes = await folder.read('', tail.name);
      if (bytes != null && bytes.length < segmentCap) {
        held = const LineSplitter().convert(utf8.decode(bytes));
        if (held.length > tail.lines) held = held.sublist(0, tail.lines);
        segments.remove(tail);
        n--;
      }
    }
    // Filled to the cap, then the next one.
    var buffer = held;
    var size = buffer.fold(0, (sum, l) => sum + l.length + 1);
    Future<void> flush() async {
      if (buffer.isEmpty) return;
      n++;
      final name = segmentName(self, id, n);
      await folder.write('', name, utf8.encode(buffer.join('\n')));
      segments.add((name: name, key: id, lines: buffer.length));
      buffer = [];
      size = 0;
    }

    for (final line in fresh) {
      buffer.add(line);
      size += line.length + 1;
      if (size >= segmentCap) await flush();
    }
    await flush();
  }
  segments.sort(_bySegment);
  await folder.write(
    '',
    manifestName(self),
    FolderManifest(
      generation: afresh
          ? (previous?.generation ?? 0) + 1
          : previous.generation,
      vector: store.versionVector(),
      segments: segments,
    ).encode(),
  );
  return out;
}

/// Segments by key id, then number.
int _bySegment(Segment a, Segment b) {
  final byKey = a.key.compareTo(b.key);
  if (byKey != 0) return byKey;
  return _number(a.name).compareTo(_number(b.name));
}

int _number(String name) {
  final parts = name.split('.');
  return parts.length < 4 ? 0 : int.tryParse(parts[parts.length - 2]) ?? 0;
}

/// Fetches the blobs the log lacks, as far as the folder has them under
/// a key held here. Returns how many came in.
Future<int> _fetchBlobs(
  Store store,
  SyncFolder folder,
  Set<String> names,
) async {
  final missing = store.missingBlobs();
  if (missing.isEmpty) return 0;
  final keys = store.areaKeys();
  var fetched = 0;
  for (final ref in missing) {
    final (kind, hash) = splitBlobRef(ref);
    for (final key in keys) {
      final name = blobName(key.key, ref);
      if (!names.contains(name)) continue;
      try {
        final bytes = await folder.read('blobs', name);
        final plain = bytes == null
            ? null
            : unseal(key.key, bytes, aad: SealedAs.blob(ref));
        // Bytes that are not the blob they are named for stay out.
        if (plain != null && store.putBlob(kind, hash, plain)) {
          fetched++;
          break;
        }
      } on FileSystemException {
        // Gone meanwhile: next round.
      }
    }
  }
  return fetched;
}

/// Publishes every blob held here that the folder lacks, sealed with the
/// key of the Note it is on (once per key when two Notes in different
/// Areas hold it). Returns how many went out.
Future<int> _publishBlobs(
  Store store,
  SyncFolder folder,
  Set<String> names,
  Uint8List Function() nonce,
) async {
  final wanted = <(String, String), AreaKey>{};
  for (final (kind, prefix) in Sync.blobKinds) {
    for (final (entity, field) in store.fieldsWithPrefix(prefix)) {
      if (store.current(entity, field) == null) continue;
      final key = store.sealingKey(entity);
      if (key == null) continue;
      wanted[('$kind/${field.substring(prefix.length)}', key.id)] = key;
    }
  }
  final order = wanted.keys.toList()
    ..sort((a, b) {
      final byRef = a.$1.compareTo(b.$1);
      return byRef != 0 ? byRef : a.$2.compareTo(b.$2);
    });
  var out = 0;
  for (final pair in order) {
    final (ref, _) = pair;
    final key = wanted[pair]!;
    final name = blobName(key.key, ref);
    if (names.contains(name)) continue;
    final (kind, hash) = splitBlobRef(ref);
    final bytes = store.blob(kind, hash);
    if (bytes == null) continue;
    await folder.write(
      'blobs',
      name,
      seal(key.key, bytes, aad: SealedAs.blob(ref), nonce: nonce()),
    );
    names.add(name);
    out++;
  }
  return out;
}

/// How the last folder sync on this Device went, for the status line;
/// kept as a local setting, so the app shows what the background sync
/// did.
class FolderStatus {
  final DateTime at;

  /// The entries that came in.
  final int entriesIn;

  /// Segments sealed with keys this Device does not hold.
  final int unreadable;

  /// The folder could not be read or written.
  final bool failed;

  const FolderStatus({
    required this.at,
    required this.entriesIn,
    required this.unreadable,
    required this.failed,
  });

  Map<String, Object?> toJson() => {
    'at': at.toUtc().toIso8601String(),
    'entriesIn': entriesIn,
    'unreadable': unreadable,
    'failed': failed,
  };

  static FolderStatus? fromJson(Object? json) {
    if (json case {
      'at': final String at,
      'entriesIn': final int entriesIn,
      'unreadable': final int unreadable,
      'failed': final bool failed,
    }) {
      final when = DateTime.tryParse(at);
      if (when == null) return null;
      return FolderStatus(
        at: when,
        entriesIn: entriesIn,
        unreadable: unreadable,
        failed: failed,
      );
    }
    return null;
  }
}

/// The shared folder of this Device, never synced (ADR 0008): a path on
/// the desktop, the tree URI the picker granted on Android.
extension SyncFolderSettings on Store {
  static const _folderKey = 'folder';
  static const _statusKey = 'folder:last';
  static const _readPrefix = 'folderRead:';

  /// The shared folder, if one was chosen.
  String? get syncFolder => localSetting(_folderKey);

  /// Chooses [folder]; whatever was read from another folder is
  /// forgotten, so the new one is read whole.
  void chooseSyncFolder(String folder) {
    if (syncFolder == folder) return;
    forgetSyncFolder();
    setLocalSetting(_folderKey, folder);
  }

  /// Forgets the shared folder; nothing in it is touched.
  void forgetSyncFolder() {
    removeLocalSetting(_folderKey);
    removeLocalSetting(_statusKey);
    for (final (key, _) in localSettingsWithPrefix(_readPrefix)) {
      removeLocalSetting(key);
    }
  }

  /// How the last folder sync went, or null before the first.
  FolderStatus? get folderStatus {
    final raw = localSetting(_statusKey);
    if (raw == null) return null;
    try {
      return FolderStatus.fromJson(jsonDecode(raw));
    } on FormatException {
      return null;
    }
  }

  void _setFolderStatus(FolderStatus status) =>
      setLocalSetting(_statusKey, jsonEncode(status.toJson()));
}

/// One folder sync of [store] through [folder] as the apps run it, its
/// outcome kept as the [FolderStatus]: null when the folder could not be
/// reached, which is kept too and tried again next time.
Future<FolderSyncResult?> folderRound(
  Store store,
  SyncFolder folder, {
  Uint8List Function()? nonce,
}) async {
  try {
    final result = await folderSync(store, folder, nonce: nonce);
    store._setFolderStatus(
      FolderStatus(
        at: store.clock().toUtc(),
        entriesIn: result.applied.length,
        unreadable: result.unreadable,
        failed: false,
      ),
    );
    return result;
  } on FileSystemException {
    store._setFolderStatus(
      FolderStatus(
        at: store.clock().toUtc(),
        entriesIn: 0,
        unreadable: 0,
        failed: true,
      ),
    );
    return null;
  }
}
