import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'area_keys.dart';
import 'backup.dart';
import 'conflicts.dart';
import 'entry.dart';
import 'profiles.dart';
import 'shared_areas.dart';
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

  /// Devices whose manifest is of a newer format than this version
  /// reads: nothing of theirs was read, this app needs an update.
  final int newer;
  final Moment? moment;
  final ImportReport report;
  final List<Conflict> conflicts;

  /// Whether the own Profile changed, or "Which one is you?" came up,
  /// from what another of the user's Devices published (ADR 0017).
  final bool profileChanged;

  const FolderSyncResult({
    required this.applied,
    required this.entriesOut,
    required this.blobsIn,
    required this.blobsOut,
    required this.unreadable,
    this.newer = 0,
    required this.moment,
    required this.report,
    required this.conflicts,
    this.profileChanged = false,
  });
}

/// A Device's history in the folder is a run of segments per key
/// (Catlog's ADR 0010, ADR 0012): `<device>.<key id>.<n>.seg`, one
/// sealed entry per line, appended to until one passes [segmentCap] and
/// the next is opened; a full segment never changes again. The manifest
/// beside them, `<device>.manifest`, says what the writer knows and
/// which segments exist; it is not sealed.
const segmentCap = 64 * 1024;

/// The manifest's format, `format` in it: 1 when missing, and 1 is never
/// written (ADR 0019). A reader reads no data of a higher one and says
/// so: the Device that wrote it runs a newer version.
const manifestFormat = 1;

String manifestName(String device) => '$device.manifest';
String segmentName(String device, String key, int n) => '$device.$key.$n.seg';

/// The keys a Device publishes, sealed with one of the personal keys:
/// `keys/<device>.<key id>.keys`.
String keysFileName(String device, String key) => '$device.$key.keys';

/// The Profiles of a Shared Area's people a Device publishes, sealed
/// with the Area's key (ADR 0017): `keys/<device>.<key id>.profiles`.
String profilesFileName(String device, String key) => '$device.$key.profiles';

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

  /// Its bytes, the same from both cores: keys sorted, no spaces; the
  /// format only above 1.
  Uint8List encode() => utf8.encode(
    jsonEncode({
      if (format != 1) 'format': format,
      'generation': generation,
      'segments': [
        for (final s in segments)
          {'key': s.key, 'lines': s.lines, 'name': s.name},
      ],
      'vector': {for (final d in vector.keys.toList()..sort()) d: vector[d]},
    }),
  );

  /// The manifest [bytes] hold; null when they hold none this version
  /// reads: half-written, damaged, or of a newer format ([isNewer]).
  static FolderManifest? parse(List<int>? bytes) {
    if (bytes == null) return null;
    try {
      final m = jsonDecode(utf8.decode(bytes));
      if (m is! Map) return null;
      final format = m.containsKey('format') ? m['format'] : 1;
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

  /// Whether [bytes] hold a manifest of a format newer than this version
  /// reads: its writer runs a newer version of the app.
  static bool isNewer(List<int>? bytes) {
    if (bytes == null) return false;
    try {
      final m = jsonDecode(utf8.decode(bytes));
      return m is Map && m['format'] is int && m['format'] > manifestFormat;
    } on FormatException {
      return false;
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
  final profileChanged = await _learnKeys(store, folder, report);
  final before = store.currentSeq();
  final (applied, unreadable, newer) = await _readForeign(
    store,
    folder,
    report,
  );
  final moment = store.momentFor(
    before: before,
    changed: applied.isNotEmpty,
    cause: MomentCause.folder,
  );
  final people = await _learnProfiles(store, folder);
  store.ensureAreaKeys();
  await _publishKeys(store, folder, nextNonce);
  await _publishProfiles(store, folder, nextNonce);
  final entriesOut = await _publishEntries(store, folder, nextNonce);
  final names = (await folder.list('blobs')).toSet();
  final blobsIn = await _fetchBlobs(store, folder, names);
  final blobsOut = await _publishBlobs(store, folder, names, nextNonce);
  await _deleteRemovedBlobs(store, folder, names);
  return FolderSyncResult(
    applied: applied,
    entriesOut: entriesOut,
    blobsIn: blobsIn,
    blobsOut: blobsOut,
    unreadable: unreadable,
    newer: newer,
    moment: moment,
    report: report,
    conflicts: store.conflictsAmong(applied),
    profileChanged: profileChanged || people,
  );
}

/// Reads the sealed layout of [folder] as a folder sync reads the shared
/// folder, for a bundle (bundle.dart): learns the keys, takes in every
/// other Device's segments of a key held here from their first line,
/// remembering nothing of how far it read, with a Moment of [cause]
/// before what was new, and the blobs the log lacks. Writes nothing.
Future<FolderSyncResult> readSealed(
  Store store,
  SyncFolder folder, {
  required String cause,
  String? label,
}) async {
  final report = ImportReport();
  final profileChanged = await _learnKeys(store, folder, report);
  final before = store.currentSeq();
  final (applied, unreadable, newer) = await _readForeign(
    store,
    folder,
    report,
    remember: false,
  );
  final moment = store.momentFor(
    before: before,
    changed: applied.isNotEmpty,
    cause: cause,
    label: label,
  );
  final people = await _learnProfiles(store, folder);
  final names = (await folder.list('blobs')).toSet();
  final blobsIn = await _fetchBlobs(store, folder, names);
  return FolderSyncResult(
    applied: applied,
    entriesOut: 0,
    blobsIn: blobsIn,
    blobsOut: 0,
    unreadable: unreadable,
    newer: newer,
    moment: moment,
    report: report,
    conflicts: store.conflictsAmong(applied),
    profileChanged: profileChanged || people,
  );
}

/// Writes [entries] into the empty [folder] as a folder sync writes this
/// Device's part of the shared folder, for a bundle (bundle.dart): the
/// keys files, the segments under generation 1 with the manifest, and
/// the blobs of [blobs], each sealed as there and with nonces from
/// [nonce] in the same order. Returns how many entries and blobs went
/// out.
Future<({int entries, int blobs})> writeSealed(
  Store store,
  SyncFolder folder,
  List<Entry> entries, {
  required Set<String> blobs,
  required Uint8List Function() nonce,
}) async {
  store.ensureAreaKeys();
  await _publishKeys(store, folder, nonce);
  await _publishProfiles(store, folder, nonce);
  final out = await _writeSegments(
    store,
    folder,
    nonce,
    entries,
    segments: [],
    generation: 1,
  );
  final blobsOut = await _publishBlobs(
    store,
    folder,
    <String>{},
    nonce,
    only: blobs,
  );
  return (entries: out, blobs: blobsOut);
}

/// The (device, key id) a file name `<device>.<key id>.<rest>` names.
(String, String)? _deviceAndKey(String name) {
  final parts = name.split('.');
  if (parts.length < 3 || parts[0].isEmpty || parts[1].isEmpty) return null;
  return (parts[0], parts[1]);
}

/// Opens every other Device's keys file sealed with a personal key held
/// here and takes in what it carries, until nothing new comes: a key
/// learned from one file may open another. The own Profile a writer
/// published is taken in, its writer's key bound to it (ADR 0017): the
/// personal key shows the writer is one of the user's own Devices.
/// Returns whether the own Profile changed or a question came up.
Future<bool> _learnKeys(
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
  var profileChanged = false;
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
      if (store.pinnedKey(device)?.record.publicKey case final key?) {
        final taken = store.takeOwnProfile(json['profile'], from: key);
        if (taken != ProfileTaken.unchanged && taken != ProfileTaken.refused) {
          profileChanged = true;
        }
        // The Profile's backup key, once its Profile was taken in.
        store.takeBackupKey(json['backupKey']);
      }
      if (store.learnAreaKeys(areaKeysOf(json['areaKeys'])) > 0) {
        learned = true;
      }
    }
  }
  return profileChanged;
}

/// Opens every other Device's Profiles file sealed with the key of a
/// Shared Area held here and takes in the Profiles of the people that
/// Area's grants name (ADR 0017); the rest stay out. Read after the
/// entries, so the grants that came with them count. Returns whether a
/// Profile came in or changed.
Future<bool> _learnProfiles(Store store, SyncFolder folder) async {
  final self = store.deviceId;
  var changed = false;
  for (final name in await folder.list('keys')) {
    if (!name.endsWith('.profiles')) continue;
    final (device, id) = _deviceAndKey(name) ?? ('', '');
    if (device.isEmpty || device == self) continue;
    final key = store.areaKey(id);
    if (key == null || !store.isShared(key.scope)) continue;
    final Uint8List? plain;
    try {
      final bytes = await folder.read('keys', name);
      plain = bytes == null
          ? null
          : unseal(key.key, bytes, aad: SealedAs.profiles(device));
    } on FileSystemException {
      continue;
    }
    if (plain == null) continue;
    final Object? json;
    try {
      json = jsonDecode(utf8.decode(plain));
    } on FormatException {
      continue;
    }
    if (json is! Map) continue;
    if (store.takePeople(
      json['profiles'],
      allowed: store.peopleOf(key.scope),
    )) {
      changed = true;
    }
  }
  return changed;
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
/// and only the store knows what it holds. A manifest of a newer format
/// is counted, and nothing of its writer read. A blocked person's
/// Devices' segments are never opened (ADR 0021).
Future<(List<Entry>, int, int)> _readForeign(
  Store store,
  SyncFolder folder,
  ImportReport report, {
  bool remember = true,
}) async {
  final self = store.deviceId;
  final blocked = store.blockedDevices();
  final fresh = <Entry>[];
  final states = <String, Map<String, Object>>{};
  var unreadable = 0;
  var newer = 0;
  for (final name in await folder.list('')) {
    if (!name.endsWith('.manifest')) continue;
    final device = name.substring(0, name.length - '.manifest'.length);
    if (device == self || device.isEmpty || blocked.contains(device)) continue;
    final FolderManifest? manifest;
    final List<int>? bytes;
    try {
      bytes = await folder.read('', name);
      manifest = FolderManifest.parse(bytes);
    } on FileSystemException {
      continue;
    }
    if (manifest == null) {
      if (FolderManifest.isNewer(bytes)) newer++;
      continue; // half-written, or a newer format
    }
    var state = remember
        ? _readState(store, device)
        : (generation: -1, lines: const <String, int>{});
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
  if (remember) {
    for (final MapEntry(:key, :value) in states.entries) {
      store.setLocalSetting(_readStateKey(key), jsonEncode(value));
    }
  }
  return (applied, unreadable, newer);
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
      final text = utf8.decode(plain);
      final json = jsonDecode(text);
      if (json is! Map) return null;
      entries.add(Entry.fromJson(json.cast<String, dynamic>(), text: text));
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
/// one it pinned, the own Profile's backup key (ADR 0016) and Profile,
/// signed, with its binding (ADR 0017), sealed once with each personal
/// key it holds, so a Device holding any of them learns the rest. A
/// file that already says the same stays untouched.
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
      'backupKey': ?store.backupKeyToSend,
      'profile': ?store.ownProfileJson,
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

/// Publishes, for every Shared Area whose key this Device holds, the
/// Profiles of the people its grants name, the own one among them,
/// sealed with that Area's key, so only the people of the Area read them
/// (ADR 0017). By key id; a file that already says the same stays
/// untouched.
Future<void> _publishProfiles(
  Store store,
  SyncFolder folder,
  Uint8List Function() nonce,
) async {
  final self = store.deviceId;
  final keys = store.areaKeys()..sort((a, b) => a.id.compareTo(b.id));
  for (final key in keys) {
    if (key.scope == personalScope || !store.isShared(key.scope)) continue;
    final profiles = store.profilesToSend(areas: {key.scope});
    if (profiles.isEmpty) continue;
    final plain = utf8.encode(jsonEncode({'profiles': profiles}));
    final name = profilesFileName(self, key.id);
    final held = await folder.read('keys', name);
    final said = held == null
        ? null
        : unseal(key.key, held, aad: SealedAs.profiles(self));
    if (said != null && _same(said, plain)) continue;
    await folder.write(
      'keys',
      name,
      seal(key.key, plain, aad: SealedAs.profiles(self), nonce: nonce()),
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
  // A Note filed into a Shared Area goes out whole under its key: what
  // it held before was sealed with another (ADR 0013).
  final entries = store.withFiled(
    store.entriesSince(afresh ? const {} : previous.vector),
    store.isShared,
  );
  if (!afresh && entries.isEmpty) return 0;

  if (afresh) {
    for (final name in present) {
      if (name.startsWith('$self.') && name.endsWith('.seg')) {
        await folder.delete('', name);
      }
    }
  }
  return _writeSegments(
    store,
    folder,
    nonce,
    entries,
    segments: afresh ? [] : [...previous.segments],
    generation: afresh ? (previous?.generation ?? 0) + 1 : previous.generation,
  );
}

/// Seals [entries] in log order, each with the key of its Area as it is
/// now, appends them to the last of [segments] of that key, or a fresh
/// one once the last passed [segmentCap], and writes the manifest of
/// [generation] after. Returns how many entries went out.
Future<int> _writeSegments(
  Store store,
  SyncFolder folder,
  Uint8List Function() nonce,
  List<Entry> entries, {
  required List<Segment> segments,
  required int generation,
}) async {
  final self = store.deviceId;
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
      utf8.encode(e.wire),
      aad: SealedAs.entry,
      nonce: nonce(),
    );
    lines.putIfAbsent(key.id, () => []).add(base64.encode(sealed));
    out++;
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
      generation: generation,
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
/// Areas hold it), or only those of [only]. Returns how many went out.
Future<int> _publishBlobs(
  Store store,
  SyncFolder folder,
  Set<String> names,
  Uint8List Function() nonce, {
  Set<String>? only,
}) async {
  final wanted = <(String, String), AreaKey>{};
  for (final (kind, prefix) in Sync.blobKinds) {
    for (final (entity, field) in store.fieldsWithPrefix(prefix)) {
      final ref = '$kind/${field.substring(prefix.length)}';
      if (only != null && !only.contains(ref)) continue;
      if (store.current(entity, field) == null) continue;
      final key = store.sealingKey(entity);
      if (key == null) continue;
      wanted[(ref, key.id)] = key;
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

/// Deletes the sealed copies of the blobs the log lets go of
/// ([Sync.removedBlobs]) from [folder], under every key held here: a
/// removed photo or recording leaves the shared folder too (ADR 0020).
Future<void> _deleteRemovedBlobs(
  Store store,
  SyncFolder folder,
  Set<String> names,
) async {
  final removed = store.removedBlobs();
  if (removed.isEmpty) return;
  final keys = store.areaKeys();
  for (final ref in removed) {
    for (final key in keys) {
      final name = blobName(key.key, ref);
      if (!names.remove(name)) continue;
      try {
        await folder.delete('blobs', name);
      } on FileSystemException {
        // The folder went meanwhile: the next round tries again.
      }
    }
  }
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

  /// Devices writing a newer format into the folder: this app needs an
  /// update to read what they write.
  final int newer;

  const FolderStatus({
    required this.at,
    required this.entriesIn,
    required this.unreadable,
    required this.failed,
    this.newer = 0,
  });

  Map<String, Object?> toJson() => {
    'at': at.toUtc().toIso8601String(),
    'entriesIn': entriesIn,
    'unreadable': unreadable,
    'failed': failed,
    if (newer > 0) 'newer': newer,
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
        newer: switch (json) {
          {'newer': final int n} => n,
          _ => 0,
        },
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
    forgetFolderReads();
  }

  /// Forgets how far every other Device's segments were read, so the
  /// next folder sync reads them whole; the store skips what it holds.
  /// After an unblock that brings back what was refused (ADR 0021).
  void forgetFolderReads() {
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
        newer: result.newer,
      ),
    );
    store.noteFolderNewer(newer: result.newer > 0);
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
