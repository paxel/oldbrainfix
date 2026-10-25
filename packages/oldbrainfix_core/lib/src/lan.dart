import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:crypto/crypto.dart';

import 'area_keys.dart';
import 'backup.dart';
import 'conflicts.dart';
import 'entry.dart';
import 'pair_code.dart';
import 'pairing.dart';
import 'profiles.dart';
import 'shared_areas.dart';
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
/// A Device paired before syncs on its own without the PIN
/// ([pairedSync]): it checks the host's certificate against the whole
/// fingerprint it pinned before anything is sent, fetches a fresh nonce
/// from `GET /challenge`, and signs it with its own key in every request
/// ([pairedProofBytes]); the host answers only a paired Device whose
/// verified key made that signature ([trustsPairedJoiner]).
///
/// The host side here ([answerSync] and the blob answers) is what the
/// fixtures and the tests serve with; the desktop core answers the
/// same, byte for byte.

const pinHeader = 'x-oldbrainfix-pin';
const deviceHeader = 'x-oldbrainfix-device';
const formatHeader = 'x-oldbrainfix-format';

/// On a host's `/vector` answer while it invites the joiner to a Shared
/// Area (ADR 0013): the joiner then sends nothing of its own.
const inviteHeader = 'x-oldbrainfix-invite';

/// A paired Device's nonce from `/challenge` and its signature over it.
const nonceHeader = 'x-oldbrainfix-nonce';
const proofHeader = 'x-oldbrainfix-proof';

/// The wire formats this build speaks, the oldest and the newest (ADR
/// 0019). Both sides state theirs in [formatHeader], `<oldest>-<newest>`,
/// and sync in the newest both speak, which the `/sync` body names as
/// its `format`; a Device stating none speaks format 1. Two Devices
/// sharing none sync nothing, and the one whose newest is the older
/// needs an update ([SyncFailure.updateHere], [SyncFailure.updateThere]).
/// Format 2 carries the entries' causal contexts, which format 1 would
/// refuse as forged; format 3 the invites to Shared Areas, which format
/// 2 would answer with everything it holds; format 4 the Profiles (ADR
/// 0017), the own one between one's own Devices and those of the people
/// of a Shared Area with its sync, which format 3 would leave behind.
const oldestSyncFormat = 4;
const newestSyncFormat = 4;

/// The wire formats a Device speaks.
typedef FormatRange = ({int oldest, int newest});

/// The wire formats this build speaks.
const ownFormats = (oldest: oldestSyncFormat, newest: newestSyncFormat);

/// [range] as [formatHeader] states it.
String encodeFormats(FormatRange range) => '${range.oldest}-${range.newest}';

/// The range [formatHeader] states: `<oldest>-<newest>`, or one format
/// alone. A Device stating none, or nothing readable, speaks format 1.
FormatRange parseFormats(String? text) {
  final parts = (text ?? '').trim().split('-');
  final numbers = [for (final p in parts) int.tryParse(p.trim())];
  return switch (numbers) {
    [final int one] when one >= 1 => (oldest: one, newest: one),
    [final int oldest, final int newest] when oldest >= 1 && oldest <= newest =>
      (oldest: oldest, newest: newest),
    _ => (oldest: 1, newest: 1),
  };
}

/// The newest wire format both [theirs] and [ours] speak; null when they
/// share none.
int? commonFormat(FormatRange theirs, [FormatRange ours = ownFormats]) {
  final newest = theirs.newest < ours.newest ? theirs.newest : ours.newest;
  final oldest = theirs.oldest > ours.oldest ? theirs.oldest : ours.oldest;
  return newest >= oldest ? newest : null;
}

/// Which Device needs an update when [theirs] and [ours] share no
/// format: the one whose newest is older than the other's oldest.
SyncFailure updateNeeded(FormatRange theirs, [FormatRange ours = ownFormats]) =>
    theirs.newest < ours.oldest
    ? SyncFailure.updateThere
    : SyncFailure.updateHere;

/// The wire format a body is written in: its `format`, 1 when missing,
/// 0 when it is no whole number.
int formatOf(Map<String, Object?> body) => switch (body['format']) {
  null when !body.containsKey('format') => 1,
  final int format => format,
  _ => 0,
};

/// Whether this build reads a body written in [format].
bool speaksFormat(int format) =>
    format >= oldestSyncFormat && format <= newestSyncFormat;

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

  /// The Shared Area this session invited one side to, if it did.
  final String? sharedArea;

  /// What became of the own Profile the other Device sent (ADR 0017).
  final ProfileTaken profile;

  /// Whether a Profile of another person came in or changed.
  final bool peopleChanged;

  const SyncSession({
    required this.peer,
    required this.name,
    required this.applied,
    required this.moment,
    required this.report,
    this.conflicts = const [],
    this.sharedArea,
    this.profile = ProfileTaken.unchanged,
    this.peopleChanged = false,
  });
}

/// What the inviting side of a session hands out (ADR 0013): the Area
/// and the role the other Device gets there. The person it is given to
/// is the one whose Profile that Device sent, which the inviter saw
/// first (ADR 0017).
class Invite {
  final String areaId;
  final AreaRole role;

  const Invite(this.areaId, {required this.role});
}

/// The host's answer to `GET /profile`: its own Profile, signed, as it
/// goes to another person, for an inviting joiner to show before it
/// invites (ADR 0017); empty before the Profile exists.
Map<String, Object?> profileAnswer(Store store) => {
  'profile': ?store.ownProfileForOthers,
};

Map<String, int> _vector(Object? json) => {
  if (json is Map)
    for (final MapEntry(:key, :value) in json.entries)
      if (key is String && value is int) key: value,
};

/// The entries of a body; each is taken in its canonical form (ADR
/// 0019).
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

/// The bytes a paired Device signs in every request of an automatic
/// sync: the host's device id and the nonce the host handed out, so the
/// proof is good for this host and this nonce only.
Uint8List pairedProofBytes(String hostDevice, String nonce) =>
    Uint8List.fromList(utf8.encode(jsonEncode(['auto', hostDevice, nonce])));

/// Whether a request naming [device], carrying a [nonce] this host
/// handed out and [proof], comes from a paired Device that syncs
/// automatically: one whose verified key signed [pairedProofBytes] for
/// this host. A device id alone proves nothing.
bool trustsPairedJoiner(
  Store store, {
  required String device,
  required String nonce,
  required String? proof,
}) {
  if (store.pairedDevice(device) == null || !store.autoSyncOn(device)) {
    return false;
  }
  final key = store.pinnedKey(device);
  if (key == null || key.trust != KeyTrust.verified) return false;
  return verifySignature(
    key.record.publicKey,
    pairedProofBytes(store.deviceId, nonce),
    proof,
  );
}

/// The joiner's `POST /sync` body: what a host holding [hostVector]
/// lacks, with this Device's vector, id, [name] and keys, and its proof
/// over [hostDevice] and [pin] that it holds its own key: a host that
/// paired with it before lets it in without asking only then. The
/// Area keys travel along (ADR 0012): only over this session, which the
/// pair code or the pinned certificate authenticated; to a Device of
/// this one's own group the own Profile's backup key (ADR 0016) and the
/// Profile itself, signed, with its binding (ADR 0017) as well.
///
/// To another person's Device (ADR 0013) only what it shares goes: when
/// the host invites this Device ([invited]), nothing but the own Profile
/// for the host to show; when this Device invites the host ([invite]),
/// the grant written first, to the person whose Profile the host sent
/// ([hostProfile], kept here), and the Area's whole history; to a Device
/// paired by invite before, what lies in the Areas it holds a role in.
/// The Profiles of the people of those Areas go along (ADR 0017); to a
/// Device of this one's own group every Profile kept here.
///
/// The body is written in [format], the newest both Devices speak.
Map<String, Object?> syncRequest(
  Store store,
  Map<String, int> hostVector, {
  required String name,
  int format = newestSyncFormat,
  String hostDevice = '',
  String pin = '',
  bool invited = false,
  Invite? invite,
  Object? hostProfile,
}) {
  store.ensureAreaKeys();
  final List<Entry> entries;
  final List<AreaKey> areaKeys;
  // The backup key goes to a Device of its own group only.
  var own = false;
  List<Map<String, Object?>>? profiles;
  if (invited) {
    entries = const [];
    areaKeys = const [];
  } else if (invite != null) {
    final person = _invitedPerson(store, hostProfile, hostDevice);
    if (person == null) throw const SyncException(SyncFailure.failed);
    store.invite(invite.areaId, hostDevice, role: invite.role, profile: person);
    entries = store.entriesFor(
      hostDevice,
      const {},
      whole: true,
      also: {invite.areaId},
    );
    areaKeys = store.areaKeysFor(hostDevice, also: {invite.areaId});
    profiles = store.profilesToSend(
      areas: {...store.areasSharedWith(hostDevice), invite.areaId},
    );
  } else if (store.isSharedPeer(hostDevice)) {
    entries = store.entriesFor(hostDevice, hostVector);
    areaKeys = store.areaKeysFor(hostDevice);
    profiles = store.profilesToSend(areas: store.areasSharedWith(hostDevice));
  } else {
    entries = store.entriesSince(hostVector);
    areaKeys = store.areaKeys();
    profiles = store.profilesToSend();
    own = true;
  }
  return {
    'format': format,
    'deviceId': store.deviceId,
    'name': name,
    'proof': store.signingKey.sign(pairingProofBytes(hostDevice, pin)),
    'vector': store.versionVector(),
    'entries': [for (final e in entries) e.toJson()],
    'keys': [for (final k in store.keyRecords()) k.toJson()],
    'areaKeys': [for (final k in areaKeys) k.toJson()],
    if (own) 'backupKey': ?store.backupKeyToSend,
    if (own) 'profile': ?store.ownProfileJson,
    if (invited) 'profile': ?store.ownProfileForOthers,
    'profiles': ?profiles,
    if (invited) 'invited': true,
    if (invite != null) 'invite': {'area': invite.areaId},
  };
}

/// Keeps the Profile [json] the Device [device] sent of its person for
/// an invite, and returns its id; null when it is none, or its binding
/// does not hold that Device's key.
String? _invitedPerson(Store store, Object? json, String device) {
  final choice = store.sentProfileOf(json, device: device);
  if (choice == null) return null;
  store.takePeople([json]);
  return choice.profile.id;
}

/// The Area a `POST /sync` body or an answer invites to, if any.
String? _invitedTo(Object? json) {
  if (json case {'area': final String area} when area.startsWith('area:')) {
    return area;
  }
  return null;
}

/// The blobs this Device lacks that [peer], another person's Device,
/// may be asked for: those of the Areas it holds a role in.
List<String> _missingFor(Store store, String peer) {
  final areas = store.areasSharedWith(peer);
  return [
    for (final ref in store.missingBlobs())
      if (store.blobIn(ref, areas)) ref,
  ];
}

/// Takes in a joiner's `POST /sync` [body] (applied as one batch, with
/// a Moment before it when anything was new) and answers with what the
/// joiner lacks, the blobs this Device wants, its keys, id and [name].
/// The joiner's own key is pinned as verified: it came over a session
/// the pair code authenticated. Null when the body is in a format this
/// build does not speak ([speaksFormat]), the joiner ignores this
/// host's [inviting], or invites it to an Area it holds as its own. The
/// answer is in the body's format.
///
/// Another person's Device (ADR 0013), [inviting] or inviting this one
/// or paired by invite before ([isShared] says which), gets only what it
/// shares, and only the keys of those Areas are taken from it. A Device
/// of this one's own group gets this one's roles in the Shared Areas it
/// has none in ([SharedAreas.extendRolesTo]) before the answer is made,
/// and its key is bound to the own Profile, whose version it sent is
/// taken in ([OwnProfile.takeOwnProfile]).
({Map<String, Object?> answer, SyncSession session})? answerSync(
  Store store,
  Map<String, Object?> body, {
  required String name,
  Invite? inviting,
  bool Function(String peer)? isShared,
}) {
  final format = formatOf(body);
  if (!speaksFormat(format)) return null;
  final peer = body['deviceId'] is String ? body['deviceId'] as String : '';
  final peerName = body['name'] is String ? body['name'] as String : '';
  final peerVector = _vector(body['vector']);
  final invitedTo = _invitedTo(body['invite']);
  if (inviting != null && (body['invited'] != true || peer.isEmpty)) {
    return null;
  }
  // The person invited is the one whose Profile the joiner sent, and the
  // user saw before the session went on.
  if (inviting != null &&
      store.sentProfileOf(body['profile'], device: peer) == null) {
    return null;
  }
  if (invitedTo != null &&
      (inviting != null || store.refusesInvite(invitedTo))) {
    return null;
  }
  // A Device of this one's own group is no other person: inviting it
  // would cut its sync down to one Area.
  if ((inviting != null || invitedTo != null) && store.isOwnDevice(peer)) {
    return null;
  }
  final shared =
      inviting != null ||
      invitedTo != null ||
      (isShared ?? store.isSharedPeer)(peer);
  final sharedArea = inviting?.areaId ?? invitedTo;
  final report = ImportReport();
  final before = store.currentSeq();
  if (!shared) {
    store.learnAreaKeys(areaKeysOf(body['areaKeys']));
  }
  final applied = store.applyEntries(
    inviting != null ? const [] : _entries(body['entries']),
    keys: _keys(body['keys']),
    verifiedDevice: peer.isEmpty ? null : peer,
    report: report,
  );
  if (shared) {
    final areas = {...store.areasSharedWith(peer), ?invitedTo};
    store.learnAreaKeys([
      for (final k in areaKeysOf(body['areaKeys']))
        if (areas.contains(k.scope)) k,
    ]);
  }
  store.ensureAreaKeys();
  var profile = ProfileTaken.unchanged;
  var people = false;
  if (inviting != null) {
    final person = _invitedPerson(store, body['profile'], peer);
    if (person != null) {
      store.invite(inviting.areaId, peer, role: inviting.role, profile: person);
    }
    people = person != null;
  } else if (!shared) {
    store.extendRolesTo(peer);
    profile = _takeProfile(store, peer, body);
    people = store.takePeople(body['profiles']);
  } else {
    people = store.takePeople(
      body['profiles'],
      allowed: store.peopleFrom(peer),
    );
  }
  final moment = store.momentFor(
    before: before,
    changed: applied.isNotEmpty,
    cause: MomentCause.sync,
    label: peerName,
  );
  // The answer carries everything beyond what the joiner holds: once it
  // took it in, it holds what this Device holds now.
  store.noteHeldBy(peer, peerVector);
  store.noteHeldBy(peer, store.versionVector());
  return (
    answer: {
      'format': format,
      'deviceId': store.deviceId,
      'name': name,
      'entries': [
        for (final e
            in !shared
                ? store.entriesSince(peerVector)
                : store.entriesFor(peer, peerVector, whole: inviting != null))
          e.toJson(),
      ],
      'wantBlobs': shared ? _missingFor(store, peer) : store.missingBlobs(),
      'keys': [for (final k in store.keyRecords()) k.toJson()],
      'areaKeys': [
        for (final k in shared ? store.areaKeysFor(peer) : store.areaKeys())
          k.toJson(),
      ],
      if (!shared) 'backupKey': ?store.backupKeyToSend,
      if (!shared) 'profile': ?store.ownProfileJson,
      'profiles': shared
          ? store.profilesToSend(areas: store.areasSharedWith(peer))
          : store.profilesToSend(),
      if (inviting != null) 'invite': {'area': inviting.areaId},
    },
    session: SyncSession(
      peer: peer,
      name: peerName,
      applied: applied,
      moment: moment,
      report: report,
      conflicts: store.conflictsAmong(applied),
      sharedArea: sharedArea,
      profile: profile,
      peopleChanged: people,
    ),
  );
}

/// Takes in the own Profile [peer] sent in [body], a Device of this
/// one's own group whose key the session verified, then the backup key
/// it sent for that Profile (ADR 0016); nothing without that key.
ProfileTaken _takeProfile(Store store, String peer, Map<String, Object?> body) {
  final key = store.pinnedKey(peer)?.record.publicKey;
  if (key == null || peer.isEmpty) return ProfileTaken.unchanged;
  final taken = store.takeOwnProfile(body['profile'], from: key);
  store.takeBackupKey(body['backupKey']);
  return taken;
}

/// The joiner takes in the host's `/sync` [answer]; the host's own key
/// is verified, the session was authenticated by the pair code. From
/// another person's Device ([shared], ADR 0013) only the keys of the
/// Areas it shares are taken, and an invite to an Area this Device holds
/// as its own is refused before anything is taken, as `declined`. A Device
/// of this one's own group gets this one's roles in the Shared Areas it
/// has none in, to go out with the next sync, and its own Profile is
/// taken in.
SyncSession applyAnswer(
  Store store,
  Map<String, Object?> answer, {
  bool shared = false,
  String? invitedBy,
}) {
  final host = answer['deviceId'] is String ? answer['deviceId'] as String : '';
  final name = answer['name'] is String ? answer['name'] as String : '';
  final invitedTo = _invitedTo(answer['invite']);
  if (invitedTo != null && store.refusesInvite(invitedTo)) {
    throw const SyncException(SyncFailure.declined);
  }
  final report = ImportReport();
  final before = store.currentSeq();
  if (!shared) {
    store.learnAreaKeys(areaKeysOf(answer['areaKeys']));
  }
  final applied = store.applyEntries(
    _entries(answer['entries']),
    keys: _keys(answer['keys']),
    verifiedDevice: host.isEmpty ? null : host,
    report: report,
  );
  var profile = ProfileTaken.unchanged;
  final bool people;
  if (shared) {
    final areas = {...store.areasSharedWith(host), ?invitedTo};
    store.learnAreaKeys([
      for (final k in areaKeysOf(answer['areaKeys']))
        if (areas.contains(k.scope)) k,
    ]);
    people = store.takePeople(
      answer['profiles'],
      allowed: store.peopleFrom(host),
    );
  } else {
    store.extendRolesTo(host);
    profile = _takeProfile(store, host, answer);
    people = store.takePeople(answer['profiles']);
  }
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
    sharedArea: invitedTo ?? invitedBy,
    profile: profile,
    peopleChanged: people,
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

  /// The user did not confirm the person an invite was for.
  cancelled,

  /// The two Devices share no wire format, and this one is the older:
  /// this app needs an update (ADR 0019).
  updateHere,

  /// The two Devices share no wire format, and the other one is the
  /// older: the app there needs an update.
  updateThere,

  /// Anything else went wrong on the way.
  failed,
}

class SyncException implements Exception {
  final SyncFailure failure;
  const SyncException(this.failure);

  /// Whether one of the two Devices needs an update to sync.
  bool get needsUpdate =>
      failure == SyncFailure.updateHere || failure == SyncFailure.updateThere;

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
/// before anything is sent. With [invite], the host is another person's
/// Device, invited to a Shared Area (ADR 0013): its Profile is fetched
/// first and shown through [confirm] (ADR 0017), and nothing is sent
/// unless the user confirms it; a host that invites this Device itself
/// says so before anything is sent.
Future<SyncResult> lanSync(
  Store store,
  PairInfo info, {
  required String name,
  Invite? invite,
  Future<bool> Function(ProfileChoice other)? confirm,
  Duration timeout = const Duration(seconds: 20),
}) async {
  if (!isPrivateHost(info.host)) {
    throw const SyncException(SyncFailure.notPrivate);
  }
  return _sync(
    store,
    host: info.host,
    port: info.port,
    fingerprint: info.fingerprint,
    pin: info.pin,
    name: name,
    invite: invite,
    confirm: confirm,
    timeout: timeout,
  );
}

/// Syncs with the paired [device] hosting at [host] and [port] without
/// the PIN, as automatic sync does: only once the host's certificate is
/// the very one pinned for it, and with this Device's proof over a fresh
/// nonce from the host in every request. Afterwards the Device is
/// remembered at this address. Throws a [SyncException]: `wrongHost`
/// for another certificate or a Device that never hosted, `declined`
/// when the host does not sync automatically with this Device. A Device
/// that needs an update to sync is kept as an update notice
/// ([UpdateNotices]) until a sync with it goes through.
Future<SyncResult> pairedSync(
  Store store,
  PairedDevice device, {
  required String host,
  required int port,
  required String name,
  Duration timeout = const Duration(seconds: 20),
}) async {
  if (!isPrivateHost(host)) {
    throw const SyncException(SyncFailure.notPrivate);
  }
  final pinned = fromHex(device.fingerprint);
  if (pinned.length != certificateFingerprintBytes) {
    throw const SyncException(SyncFailure.wrongHost);
  }
  try {
    return await _sync(
      store,
      host: host,
      port: port,
      fingerprint: pinned,
      paired: device.id,
      name: name,
      timeout: timeout,
    );
  } on SyncException catch (e) {
    // Kept to be announced once, not at every try (ADR 0019).
    if (e.needsUpdate) {
      store.noteUpdateNeeded(
        device.id,
        e.failure == SyncFailure.updateHere
            ? UpdateNeeded.here
            : UpdateNeeded.there,
      );
    }
    rethrow;
  }
}

/// The bytes of a whole certificate fingerprint, SHA-256.
const certificateFingerprintBytes = 32;

/// One session with the host at [host] and [port] whose certificate
/// matches [fingerprint]: with the [pin] of a pair code, or as the
/// [paired] Device's partner with a proof over the host's nonce.
Future<SyncResult> _sync(
  Store store, {
  required String host,
  required int port,
  required List<int> fingerprint,
  String pin = '',
  String? paired,
  required String name,
  Invite? invite,
  Future<bool> Function(ProfileChoice other)? confirm,
  required Duration timeout,
}) async {
  List<int>? seen;
  final client = HttpClient()
    ..connectionTimeout = timeout
    ..badCertificateCallback = (cert, _, _) {
      final actual = certificateFingerprint(cert.der);
      if (!fingerprintMatches(actual, fingerprint)) return false;
      seen = actual;
      return true;
    };
  // The PIN, or once the nonce is in, the paired Device's proof; and the
  // formats this build speaks, on every request.
  final formats = {formatHeader: encodeFormats(ownFormats)};
  var auth = <String, String>{...formats, if (paired == null) pinHeader: pin};
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
          Uri(scheme: 'https', host: host, port: port, path: path),
        );
        auth.forEach(req.headers.set);
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
          throw SyncException(
            updateNeeded(parseFormats(res.headers.value(formatHeader))),
          );
        }
        // No PIN to be wrong for a paired Device: the host said no.
        throw SyncException(
          paired == null ? SyncFailure.wrongPin : SyncFailure.declined,
        );
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

    if (paired != null) {
      final challenge = await call('GET', '/challenge');
      if (challenge == null) throw const SyncException(SyncFailure.failed);
      final nonce = decode(challenge.$2)['nonce'];
      if (nonce is! String || nonce.isEmpty) {
        throw const SyncException(SyncFailure.failed);
      }
      auth = {
        ...formats,
        deviceHeader: store.deviceId,
        nonceHeader: nonce,
        proofHeader: store.signingKey.sign(pairedProofBytes(paired, nonce)),
      };
    }
    final vectorCall = await call('GET', '/vector');
    if (vectorCall == null) throw const SyncException(SyncFailure.failed);
    final (headers, vectorBytes) = vectorCall;
    // The newest format both speak; sharing none, nothing is sent.
    final hostFormats = parseFormats(headers.value(formatHeader));
    final format = commonFormat(hostFormats);
    if (format == null) throw SyncException(updateNeeded(hostFormats));
    final hostDevice = headers.value(deviceHeader) ?? '';
    // The certificate is the pinned one; a host naming another Device
    // all the same is not the one paired.
    if (paired != null && hostDevice != paired) {
      throw const SyncException(SyncFailure.wrongHost);
    }
    final hostVector = _vector(decode(vectorBytes));
    final invited = headers.value(inviteHeader) == '1';
    // Two Devices inviting each other: neither gets what it meant.
    if (invited && invite != null) {
      throw const SyncException(SyncFailure.declined);
    }
    if (invite != null && hostDevice.isEmpty) {
      throw const SyncException(SyncFailure.failed);
    }
    // A Device of this one's own group is no other person to invite.
    if ((invited || invite != null) && store.isOwnDevice(hostDevice)) {
      throw const SyncException(SyncFailure.declined);
    }
    final shared = invited || invite != null || store.isSharedPeer(hostDevice);
    // The person invited is the one whose Profile the host shows, once
    // the user confirmed it.
    Object? hostProfile;
    if (invite != null) {
      final profileCall = await call('GET', '/profile');
      if (profileCall == null) throw const SyncException(SyncFailure.failed);
      hostProfile = decode(profileCall.$2)['profile'];
      final choice = store.sentProfileOf(hostProfile, device: hostDevice);
      if (choice == null) throw const SyncException(SyncFailure.failed);
      if (confirm != null && !await confirm(choice)) {
        throw const SyncException(SyncFailure.cancelled);
      }
    }
    final request = syncRequest(
      store,
      hostVector,
      name: name,
      format: format,
      hostDevice: hostDevice,
      pin: pin,
      invited: invited,
      invite: invite,
      hostProfile: hostProfile,
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
      session = applyAnswer(
        store,
        answer,
        shared: shared,
        invitedBy: invite?.areaId,
      );
    } on SyncException {
      rethrow;
    } on Object {
      // Entries that are not entries: nothing of them was kept.
      throw const SyncException(SyncFailure.failed);
    }
    if (session.peer.isNotEmpty) {
      store.rememberDevice(
        PairedDevice(
          id: session.peer,
          name: session.name,
          fingerprint: hex(seen ?? fingerprint),
          host: host,
          port: port,
          lastSync: store.clock().toUtc(),
          shared: shared,
        ),
      );
      // The host took in everything beyond its vector, as it answered.
      store.noteHeldBy(session.peer, hostVector);
      store.noteHeldBy(session.peer, _vector(request['vector']));
      store.updateNoLongerNeeded(session.peer);
    }

    // The entries are in and the Moment recorded; whatever happens to the
    // blobs now, the next sync fetches what is still missing.
    var blobsIn = 0, blobsOut = 0;
    final sharedAreas = shared ? store.areasSharedWith(session.peer) : null;
    try {
      for (final ref in store.missingBlobs()) {
        if (sharedAreas != null && !store.blobIn(ref, sharedAreas)) continue;
        final got = await call('GET', '/blob/$ref');
        if (got == null) continue;
        final (kind, hash) = splitBlobRef(ref);
        if (store.putBlob(kind, hash, got.$2)) blobsIn++;
      }
      final wanted = answer['wantBlobs'];
      for (final ref in wanted is List ? wanted : const []) {
        if (ref is! String) continue;
        if (sharedAreas != null && !store.blobIn(ref, sharedAreas)) continue;
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
