import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:crypto/crypto.dart';

import 'conflicts.dart';
import 'entry.dart';
import 'pair_code.dart';
import 'pairing.dart';
import 'signing.dart';
import 'store.dart';
import 'sync.dart';

/// LAN sync (ADR 0003), Catlog's in-person sync copied and adapted: the
/// desktop hosts a small HTTPS server with a self-signed certificate
/// made once per install, the pair code carries the certificate's
/// fingerprint and a PIN, and the phone joins: `GET /vector`, then
/// `POST /sync` with what the host lacks, which answers with what the
/// phone lacks, then the photos and voice recordings both ways under
/// `/blob/<kind>/<hash>`. Every request carries the PIN; the host locks
/// an address out after [pinFailuresPerAddress] wrong ones and stops
/// answering anyone after [pinFailuresTotal].
///
/// The host side here ([answerSync] and the blob answers) is what the
/// fixtures and the tests serve with; the desktop core answers the
/// same, byte for byte.

const pinHeader = 'x-oldbrainfix-pin';
const deviceHeader = 'x-oldbrainfix-device';
const formatHeader = 'x-oldbrainfix-format';

/// The wire format this build speaks; a peer speaking another one is
/// refused with the reason, never half understood. Format 2 carries the
/// entries' causal contexts, which format 1 would refuse as forged.
const syncFormat = 2;

const pinFailuresPerAddress = 5;
const pinFailuresTotal = 20;

/// The most a sync body or a blob may weigh; a peer with the PIN must not
/// be able to fill the memory.
const maxEntriesBytes = 64 << 20;
const maxBlobBytes = 64 << 20;

/// What one side of a session brought in: from whom, what was new here,
/// the Moment before it, what the signatures said, and the Conflicts
/// what came in is part of.
class SyncSession {
  /// The other Device's id and the name it goes by.
  final String peer;
  final String name;
  final List<Entry> applied;
  final Moment? moment;
  final ImportReport report;
  final List<Conflict> conflicts;

  const SyncSession({
    required this.peer,
    required this.name,
    required this.applied,
    required this.moment,
    required this.report,
    this.conflicts = const [],
  });
}

Map<String, int> _vector(Object? json) => {
  if (json is Map)
    for (final MapEntry(:key, :value) in json.entries)
      if (key is String && value is int) key: value,
};

List<Entry> _entries(Object? json) => [
  if (json is List)
    for (final e in json)
      if (e is Map) Entry.fromJson(e.cast<String, dynamic>()),
];

List<KeyRecord> _keys(Object? json) => [
  if (json is List)
    for (final k in json) ?KeyRecord.fromJson(k),
];

/// The bytes a joiner signs to prove it is the Device it names: the
/// host's device id and the session's PIN, so the proof is good for this
/// host and this session only.
Uint8List pairingProofBytes(String hostDevice, String pin) =>
    Uint8List.fromList(utf8.encode(jsonEncode(['pair', hostDevice, pin])));

/// The joiner's `POST /sync` body: what a host holding [hostVector]
/// lacks, with this Device's vector, id, [name] and keys, and its proof
/// over [hostDevice] and [pin] that it holds its own key: a host that
/// paired with it before lets it in without asking only then.
Map<String, Object?> syncRequest(
  Store store,
  Map<String, int> hostVector, {
  required String name,
  String hostDevice = '',
  String pin = '',
}) => {
  'format': syncFormat,
  'deviceId': store.deviceId,
  'name': name,
  'proof': store.signingKey.sign(pairingProofBytes(hostDevice, pin)),
  'vector': store.versionVector(),
  'entries': [for (final e in store.entriesSince(hostVector)) e.toJson()],
  'keys': [for (final k in store.keyRecords()) k.toJson()],
};

/// Takes in a joiner's `POST /sync` [body] (applied as one batch, with
/// a Moment before it when anything was new) and answers with what the
/// joiner lacks, the blobs this Device wants, its keys, id and [name].
/// The joiner's own key is pinned as verified: it came over a session
/// the pair code authenticated. Null when the joiner speaks another
/// format.
({Map<String, Object?> answer, SyncSession session})? answerSync(
  Store store,
  Map<String, Object?> body, {
  required String name,
}) {
  if (body['format'] != syncFormat) return null;
  final peer = body['deviceId'] is String ? body['deviceId'] as String : '';
  final peerName = body['name'] is String ? body['name'] as String : '';
  final report = ImportReport();
  final before = store.currentSeq();
  final applied = store.applyEntries(
    _entries(body['entries']),
    keys: _keys(body['keys']),
    verifiedDevice: peer.isEmpty ? null : peer,
    report: report,
  );
  final moment = store.momentFor(
    before: before,
    changed: applied.isNotEmpty,
    cause: MomentCause.sync,
    label: peerName,
  );
  return (
    answer: {
      'format': syncFormat,
      'deviceId': store.deviceId,
      'name': name,
      'entries': [
        for (final e in store.entriesSince(_vector(body['vector']))) e.toJson(),
      ],
      'wantBlobs': store.missingBlobs(),
      'keys': [for (final k in store.keyRecords()) k.toJson()],
    },
    session: SyncSession(
      peer: peer,
      name: peerName,
      applied: applied,
      moment: moment,
      report: report,
      conflicts: store.conflictsAmong(applied),
    ),
  );
}

/// The joiner takes in the host's `/sync` [answer]; the host's own key
/// is verified, the session was authenticated by the pair code.
SyncSession applyAnswer(Store store, Map<String, Object?> answer) {
  final host = answer['deviceId'] is String ? answer['deviceId'] as String : '';
  final name = answer['name'] is String ? answer['name'] as String : '';
  final report = ImportReport();
  final before = store.currentSeq();
  final applied = store.applyEntries(
    _entries(answer['entries']),
    keys: _keys(answer['keys']),
    verifiedDevice: host.isEmpty ? null : host,
    report: report,
  );
  return SyncSession(
    peer: host,
    name: name,
    applied: applied,
    moment: store.momentFor(
      before: before,
      changed: applied.isNotEmpty,
      cause: MomentCause.sync,
      label: name,
    ),
    report: report,
    conflicts: store.conflictsAmong(applied),
  );
}

/// Why a sync did not happen.
enum SyncFailure {
  /// The code names no address on a local network.
  notPrivate,

  /// Nobody answered at the address.
  unreachable,

  /// The certificate was not the one the code named.
  wrongHost,
  wrongPin,

  /// Too many wrong PINs: the host has to start again.
  locked,

  /// The host said no to this Device.
  declined,

  /// The other side speaks another wire format.
  format,

  /// Anything else went wrong on the way.
  failed,
}

class SyncException implements Exception {
  final SyncFailure failure;
  const SyncException(this.failure);

  @override
  String toString() => 'SyncException(${failure.name})';
}

/// What a sync did.
class SyncResult {
  final SyncSession session;
  final int entriesSent;
  final int blobsSent;
  final int blobsReceived;

  const SyncResult({
    required this.session,
    required this.entriesSent,
    required this.blobsSent,
    required this.blobsReceived,
  });
}

/// The SHA-256 fingerprint of a certificate's DER bytes, what the pair
/// code carries.
List<int> certificateFingerprint(List<int> der) => sha256.convert(der).bytes;

/// Whether a certificate's fingerprint [actual] is the one the code
/// named: all of it from a QR, its first bytes from a typed code.
bool fingerprintMatches(List<int> actual, List<int> expected) {
  if (expected.length < typedFingerprintBytes) return false;
  if (expected.length > actual.length) return false;
  for (var i = 0; i < expected.length; i++) {
    if (actual[i] != expected[i]) return false;
  }
  return true;
}

/// Joins the host the pair code [info] names and syncs both ways:
/// entries, then photos and voice recordings. This Device goes by
/// [name]. After a session the host is remembered as a paired Device,
/// with the whole fingerprint of its certificate. Throws a
/// [SyncException]; an address outside the local network is refused
/// before anything is sent.
Future<SyncResult> lanSync(
  Store store,
  PairInfo info, {
  required String name,
  Duration timeout = const Duration(seconds: 20),
}) async {
  if (!isPrivateHost(info.host)) {
    throw const SyncException(SyncFailure.notPrivate);
  }
  List<int>? seen;
  final client = HttpClient()
    ..connectionTimeout = timeout
    ..badCertificateCallback = (cert, _, _) {
      final actual = certificateFingerprint(cert.der);
      if (!fingerprintMatches(actual, info.fingerprint)) return false;
      seen = actual;
      return true;
    };
  try {
    Future<(HttpHeaders, Uint8List)?> call(
      String method,
      String path, {
      List<int>? body,
      bool json = false,
    }) async {
      final HttpClientResponse res;
      final Uint8List data;
      try {
        final req = await client.openUrl(
          method,
          Uri(scheme: 'https', host: info.host, port: info.port, path: path),
        );
        req.headers.set(pinHeader, info.pin);
        if (json) req.headers.contentType = ContentType.json;
        req.contentLength = body?.length ?? 0;
        if (body != null) req.add(body);
        res = await req.close();
        final bytes = BytesBuilder(copy: false);
        await for (final chunk in res) {
          bytes.add(chunk);
          if (bytes.length > maxEntriesBytes) {
            throw const SyncException(SyncFailure.failed);
          }
        }
        data = bytes.takeBytes();
      } on HandshakeException {
        throw SyncException(
          seen == null ? SyncFailure.wrongHost : SyncFailure.failed,
        );
      } on SocketException {
        throw const SyncException(SyncFailure.unreachable);
      } on IOException {
        throw const SyncException(SyncFailure.failed);
      }
      if (res.statusCode == HttpStatus.forbidden) {
        final text = utf8.decode(data, allowMalformed: true);
        if (text.contains('locked')) {
          throw const SyncException(SyncFailure.locked);
        }
        if (text.contains('declined')) {
          throw const SyncException(SyncFailure.declined);
        }
        if (text.contains('refusal')) {
          throw const SyncException(SyncFailure.format);
        }
        throw const SyncException(SyncFailure.wrongPin);
      }
      // A blob the log mentions but nobody holds: skipped, never failed on.
      if (res.statusCode == HttpStatus.notFound && path.startsWith('/blob/')) {
        return null;
      }
      if (res.statusCode != HttpStatus.ok) {
        throw const SyncException(SyncFailure.failed);
      }
      return (res.headers, data);
    }

    Map<String, Object?> decode(Uint8List data) {
      try {
        final value = jsonDecode(utf8.decode(data));
        if (value is Map<String, Object?>) return value;
      } on FormatException {
        // Falls through to the failure below.
      }
      throw const SyncException(SyncFailure.failed);
    }

    final vectorCall = await call('GET', '/vector');
    if (vectorCall == null) throw const SyncException(SyncFailure.failed);
    final (headers, vectorBytes) = vectorCall;
    if (headers.value(formatHeader) != '$syncFormat') {
      throw const SyncException(SyncFailure.format);
    }
    final hostVector = _vector(decode(vectorBytes));
    final request = syncRequest(
      store,
      hostVector,
      name: name,
      hostDevice: headers.value(deviceHeader) ?? '',
      pin: info.pin,
    );
    final syncCall = await call(
      'POST',
      '/sync',
      body: utf8.encode(jsonEncode(request)),
      json: true,
    );
    if (syncCall == null) throw const SyncException(SyncFailure.failed);
    final answer = decode(syncCall.$2);
    final SyncSession session;
    try {
      session = applyAnswer(store, answer);
    } on Object {
      // Entries that are not entries: nothing of them was kept.
      throw const SyncException(SyncFailure.failed);
    }
    if (session.peer.isNotEmpty) {
      store.rememberDevice(
        PairedDevice(
          id: session.peer,
          name: session.name,
          fingerprint: hex(seen ?? info.fingerprint),
          host: info.host,
          port: info.port,
          lastSync: store.clock().toUtc(),
        ),
      );
    }

    // The entries are in and the Moment recorded; whatever happens to the
    // blobs now, the next sync fetches what is still missing.
    var blobsIn = 0, blobsOut = 0;
    try {
      for (final ref in store.missingBlobs()) {
        final got = await call('GET', '/blob/$ref');
        if (got == null) continue;
        final (kind, hash) = splitBlobRef(ref);
        if (store.putBlob(kind, hash, got.$2)) blobsIn++;
      }
      final wanted = answer['wantBlobs'];
      for (final ref in wanted is List ? wanted : const []) {
        if (ref is! String) continue;
        final (kind, hash) = splitBlobRef(ref);
        final bytes = store.blob(kind, hash);
        if (bytes == null) continue;
        await call('POST', '/blob/$ref', body: bytes);
        blobsOut++;
      }
    } on SyncException {
      // Counted what landed; the rest waits for the next sync.
    }
    return SyncResult(
      session: session,
      entriesSent: (request['entries'] as List).length,
      blobsSent: blobsOut,
      blobsReceived: blobsIn,
    );
  } finally {
    client.close(force: true);
  }
}
