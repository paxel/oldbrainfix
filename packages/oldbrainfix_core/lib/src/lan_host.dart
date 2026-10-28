import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:math';
import 'dart:typed_data';

import 'host_identity.dart';
import 'lan.dart';
import 'pair_code.dart';
import 'pairing.dart';
import 'profiles.dart';
import 'shared_areas.dart';
import 'signing.dart';
import 'store.dart';
import 'sync.dart';

/// The LAN host as every Device runs it (ADR 0003), the twin of the
/// desktop core's `Host`: a small HTTPS server with this Device's
/// [HostIdentity]. Every request that passed the gate goes out on
/// [requests] for whoever answers it with the store ([PairingHost] and
/// [PairedHost] do), and is answered with [HostRequest.reply], perhaps
/// after the user was asked whether to trust the Device.
///
/// The pairing host takes the PIN of its pair code on every request; an
/// address is locked out after [pinFailuresPerAddress] wrong ones,
/// everyone after [pinFailuresTotal]. The paired host takes no PIN: it
/// hands out nonces on `GET /challenge` and lets in only a request
/// carrying one that still holds; whose proof it carries, the store
/// checks ([LanServing.pairedJoiner]).
class LanHost {
  final HttpServer _server;
  final InternetAddress address;

  /// The PIN; empty for the paired host, which takes none.
  final String pin;

  /// The whole fingerprint of the certificate it serves with.
  final List<int> fingerprint;

  final DateTime Function() _clock;
  final _requests = StreamController<HostRequest>();
  final _pending = <HostRequest>{};
  final _failures = <String, int>{};
  var _failuresTotal = 0;
  var _locked = false;
  final _challenges = <(String, DateTime)>[];

  LanHost._(
    this._server,
    this.address,
    this.pin,
    this.fingerprint,
    this._clock,
  ) {
    _server.listen((request) => unawaited(_handle(request)));
  }

  /// The pairing host with [pin], on [bind] or else the LAN address (the
  /// loopback without a network), on a free port.
  static Future<LanHost> start(
    HostIdentity identity, {
    required String pin,
    required DateTime Function() clock,
    InternetAddress? bind,
  }) async {
    final address = bind ?? await lanAddress() ?? InternetAddress.loopbackIPv4;
    final server = await HttpServer.bindSecure(
      address,
      0,
      identity.securityContext(),
    );
    return LanHost._(server, address, pin, identity.fingerprint, clock);
  }

  /// The paired host, without a PIN: on [bind] (every address of this
  /// Device, so it outlives a change of network), on [port] while that
  /// is free, else on a free one.
  static Future<LanHost> startPaired(
    HostIdentity identity, {
    required DateTime Function() clock,
    required InternetAddress bind,
    int port = 0,
  }) async {
    final context = identity.securityContext();
    HttpServer server;
    try {
      server = await HttpServer.bindSecure(bind, port, context);
    } on SocketException {
      if (port == 0) rethrow;
      server = await HttpServer.bindSecure(bind, 0, context);
    }
    return LanHost._(server, bind, '', identity.fingerprint, clock);
  }

  int get port => _server.port;

  /// Whether too many wrong PINs arrived: nothing is answered any more;
  /// hosting must start again with a new PIN.
  bool get lockedOut => _locked;

  /// The requests that passed the gate, each to be answered.
  Stream<HostRequest> get requests => _requests.stream;

  /// The pair code for this host: the whole fingerprint for a QR, its
  /// first bytes for typing.
  String pairCode({required bool typed}) => encodePairCode(
    address.type == InternetAddressType.IPv4 ? address.address : '127.0.0.1',
    port,
    pin,
    fingerprint,
    typed: typed,
  );

  /// Stops answering; requests not answered yet hear 503.
  Future<void> stop() async {
    for (final request in _pending.toList()) {
      request.reply(const HostResponse.status(503));
    }
    // Not waited for: nobody may be listening.
    unawaited(_requests.close());
    await _server.close(force: true);
  }

  Future<void> _handle(HttpRequest req) async {
    final res = req.response..persistentConnection = false;
    HostResponse response;
    try {
      response = await _answer(req);
    } on Object {
      response = HostResponse.status(HttpStatus.internalServerError);
    }
    try {
      res.statusCode = response.status;
      if (response.contentType case final type?) {
        res.headers.set(HttpHeaders.contentTypeHeader, type);
      }
      response.headers.forEach(res.headers.set);
      res.contentLength = response.body.length;
      res.add(response.body);
      await res.close();
    } on Object {
      // The joiner left meanwhile.
    }
  }

  Future<HostResponse> _answer(HttpRequest req) async {
    final path = req.uri.path;
    final limit = path.startsWith('/blob/') ? maxBlobBytes : maxEntriesBytes;
    if (req.contentLength > limit) return HostResponse.status(413);
    final body = BytesBuilder(copy: false);
    await for (final chunk in req) {
      body.add(chunk);
      if (body.length > limit) return HostResponse.status(413);
    }
    final headers = <String, String>{};
    req.headers.forEach((name, values) {
      headers[name.toLowerCase()] = values.join(',');
    });
    final from = req.connectionInfo?.remoteAddress.address ?? '';
    // Nonces only on the paired host: on a pairing host they would let
    // anyone past the PIN. Asking there costs no PIN try.
    if (req.method == 'GET' && path == '/challenge') {
      return pin.isEmpty
          ? HostResponse.json(200, {'nonce': _challenge()})
          : HostResponse.status(404);
    }
    final challenged = pin.isEmpty && _knows(headers[nonceHeader]);
    if (!challenged) {
      final refusal = _check(from, headers[pinHeader]);
      if (refusal != null) return refusal;
    }
    final request = HostRequest(
      method: req.method,
      path: path,
      headers: headers,
      body: body.takeBytes(),
      from: from,
      challenged: challenged,
    );
    if (_requests.isClosed) return const HostResponse.status(503);
    _pending.add(request);
    _requests.add(request);
    try {
      return await request._reply.future;
    } finally {
      _pending.remove(request);
    }
  }

  /// A fresh nonce, kept for [challengeTtl].
  String _challenge() {
    final random = Random.secure();
    final nonce = hex([for (var i = 0; i < 32; i++) random.nextInt(256)]);
    final now = _clock();
    _challenges.removeWhere((c) => !_holds(c.$2, now));
    if (_challenges.length >= maxChallenges) _challenges.removeAt(0);
    _challenges.add((nonce, now));
    return nonce;
  }

  bool _holds(DateTime at, DateTime now) =>
      now.difference(at) < challengeTtl && !now.isBefore(at);

  /// Whether [nonce] was handed out here and still holds.
  bool _knows(String? nonce) {
    if (nonce == null) return false;
    final now = _clock();
    return _challenges.any((c) => c.$1 == nonce && _holds(c.$2, now));
  }

  /// Null when a request from [from] with [given] may pass, else the
  /// answer to send instead. The lock is looked at before the PIN: once
  /// locked, even the right PIN hears `locked`.
  HostResponse? _check(String from, String? given) {
    if (pin.isEmpty) return HostResponse.status(403);
    if (_failuresTotal >= pinFailuresTotal ||
        (_failures[from] ?? 0) >= pinFailuresPerAddress) {
      return HostResponse.text(403, 'locked');
    }
    if (given != pin) {
      _failures[from] = (_failures[from] ?? 0) + 1;
      _failuresTotal++;
      if (_failuresTotal >= pinFailuresTotal) _locked = true;
      return HostResponse.status(403);
    }
    return null;
  }
}

/// How long a nonce from `/challenge` holds, and how many are kept.
const challengeTtl = Duration(seconds: 120);
const maxChallenges = 256;

/// The update notices' source for a joiner that did not name itself,
/// one joining by pair code (ADR 0019).
const lanSource = 'lan';

/// What a host advertises on the network for paired Devices to find it
/// (ADR 0003), as the desktop core's `discovery`: an mDNS/DNS-SD service
/// of this type whose instance name and only TXT entry, [serviceIdKey],
/// are the device id, on the paired host's port. No names: anyone on the
/// network hears the record.
const serviceType = '_oldbrainfix._tcp';
const serviceIdKey = 'id';

/// A six-digit PIN, never starting with zero.
String newPin() => (Random.secure().nextInt(900000) + 100000).toString();

/// This Device's address on its local network, or null without one.
Future<InternetAddress?> lanAddress() async {
  try {
    final interfaces = await NetworkInterface.list(
      type: InternetAddressType.IPv4,
    );
    return pickLanAddress([
      for (final i in interfaces)
        for (final a in i.addresses) (i.name, a),
    ]);
  } on SocketException {
    return null;
  }
}

/// The address to host on among [addresses] by interface name: a
/// private IPv4 one, Wi-Fi or Ethernet before anything else, never a
/// mobile network's (which can be private too) nor the loopback.
InternetAddress? pickLanAddress(List<(String, InternetAddress)> addresses) {
  bool mobile(String name) => _mobile.any(name.toLowerCase().startsWith);
  bool local(String name) => _local.any(name.toLowerCase().startsWith);
  final usable = [
    for (final (name, a) in addresses)
      if (a.type == InternetAddressType.IPv4 &&
          !a.isLoopback &&
          isPrivateHost(a.address) &&
          !mobile(name))
        (name, a),
  ];
  for (final (name, a) in usable) {
    if (local(name)) return a;
  }
  return usable.isEmpty ? null : usable.first.$2;
}

const _mobile = ['rmnet', 'ccmni', 'pdp', 'v4-', 'clat', 'dummy', 'tun'];
const _local = ['wlan', 'wl', 'eth', 'en', 'ap', 'swlan'];

/// One request that passed the gate, waiting for its answer.
class HostRequest {
  final String method;
  final String path;

  /// By lower-case name.
  final Map<String, String> headers;
  final Uint8List body;

  /// The address it came from.
  final String from;

  /// Whether it carries a nonce this host handed out and that still
  /// holds; whose proof it is, [LanServing.pairedJoiner] checks.
  final bool challenged;

  final _reply = Completer<HostResponse>();

  HostRequest({
    required this.method,
    required this.path,
    this.headers = const {},
    required this.body,
    this.from = '',
    this.challenged = false,
  });

  String? header(String name) => headers[name.toLowerCase()];

  /// Answers the joiner; only the first answer counts.
  void reply(HostResponse response) {
    if (!_reply.isCompleted) _reply.complete(response);
  }

  /// The Device asking to sync, for a `POST /sync`: its id and name.
  JoinAsk? get joinAsk {
    if (method != 'POST' || path != '/sync') return null;
    final json = _json();
    final device = json?['deviceId'];
    if (json == null || device is! String) return null;
    final name = json['name'];
    final proof = json['proof'];
    return JoinAsk(
      device: device,
      name: name is String ? name : '',
      proof: proof is String ? proof : null,
    );
  }

  /// The Profile the Device asking to sync sent of its person, for a
  /// `POST /sync` of a joiner invited (ADR 0017), checked; none without
  /// one that holds its key.
  ProfileChoice? joinerProfile(Store store) {
    final ask = joinAsk;
    if (ask == null) return null;
    return store.sentProfileOf(_json()?['profile'], device: ask.device);
  }

  Map<String, Object?>? _json() {
    try {
      final value = jsonDecode(utf8.decode(body));
      return value is Map<String, Object?> ? value : null;
    } on FormatException {
      return null;
    }
  }
}

/// A Device asking to sync: the id and name it gives, and its signature
/// over this host's id and the PIN ([pairingProofBytes]).
class JoinAsk {
  final String device;
  final String name;
  final String? proof;

  const JoinAsk({required this.device, required this.name, this.proof});
}

/// What goes back over the wire.
class HostResponse {
  final int status;
  final String? contentType;
  final Map<String, String> headers;
  final List<int> body;

  const HostResponse(
    this.status, {
    this.contentType,
    this.headers = const {},
    this.body = const [],
  });

  const HostResponse.status(int status) : this(status);

  HostResponse.text(int status, String text)
    : this(status, body: utf8.encode(text));

  HostResponse.json(
    int status,
    Object? value, {
    Map<String, String> headers = const {},
  }) : this(
         status,
         contentType: 'application/json',
         headers: headers,
         body: utf8.encode(jsonEncode(value)),
       );

  const HostResponse.bytes(List<int> bytes)
    : this(200, contentType: 'application/octet-stream', body: bytes);

  /// The refusal a Device the user said no to hears.
  HostResponse.declined() : this.text(403, 'declined');
}

/// The store's side of hosting, the twin of the desktop core's
/// `Store::serve`.
extension LanServing on Store {
  /// Where the paired host's port is kept from run to run.
  static const pairedPortKey = 'lan:port';

  /// Whether [ask] comes from a paired Device: one paired before whose
  /// pinned key signed the proof for this host and [pin]. A device id
  /// alone proves nothing; anyone with the PIN can read it.
  bool trustsJoiner(JoinAsk ask, String pin) {
    if (pairedDevice(ask.device) == null) return false;
    final key = pinnedKey(ask.device);
    if (key == null) return false;
    return verifySignature(
      key.record.publicKey,
      pairingProofBytes(deviceId, pin),
      ask.proof,
    );
  }

  /// The paired Device [request] comes from, when it carries a nonce
  /// this host handed out and that Device's proof over it.
  String? pairedJoiner(HostRequest request) {
    final device = request.header(deviceHeader);
    final nonce = request.header(nonceHeader);
    if (device == null || nonce == null || !request.challenged) return null;
    final trusted = trustsPairedJoiner(
      this,
      device: device,
      nonce: nonce,
      proof: request.header(proofHeader),
    );
    return trusted ? device : null;
  }

  /// Answers one request as the pairing host, going by [name], while it
  /// hands out [inviting] (ADR 0013): `/vector` says so, and only the
  /// invited Area's blobs are handed out. A completed `/sync` comes back
  /// as its session.
  (HostResponse, SyncSession?) serve(
    HostRequest request, {
    required String name,
    Invite? inviting,
  }) => _serveAs(request, name, inviting, null);

  /// Answers one request on the paired host, going by [name]: only a
  /// paired Device that syncs on its own, proven by its key, and its
  /// `/sync` only in its own name. The Device is remembered as synced
  /// after a completed `/sync` ([rememberJoiner]).
  (HostResponse, SyncSession?) servePaired(
    HostRequest request, {
    required String name,
  }) {
    final device = pairedJoiner(request);
    if (device == null) return (HostResponse.declined(), null);
    if (request.joinAsk case final ask? when ask.device != device) {
      return (HostResponse.declined(), null);
    }
    final (response, session) = _serveAs(request, name, null, device);
    if (session != null) rememberJoiner(session);
    return (response, session);
  }

  /// Remembers the joiner of [session] as a paired Device, with the
  /// certificate it proved it hosts with ([SyncSession.fingerprint]),
  /// by invite when the session invited one side.
  void rememberJoiner(SyncSession session) {
    if (session.peer.isEmpty) return;
    rememberDevice(
      PairedDevice(
        id: session.peer,
        name: session.name,
        fingerprint: session.fingerprint ?? '',
        host: null,
        port: null,
        lastSync: clock().toUtc(),
        shared: session.sharedArea != null,
      ),
    );
  }

  /// Answers [request], from the paired Device [joiner] when known: one
  /// paired by invite gets only the blobs of the Areas it shares.
  (HostResponse, SyncSession?) _serveAs(
    HostRequest request,
    String name,
    Invite? inviting,
    String? joiner,
  ) {
    final path = request.path;
    final method = request.method;
    if (method == 'GET' && path == '/vector') {
      // A joiner sharing no format with this build syncs nothing, and is
      // told so (ADR 0019).
      final theirs = parseFormats(request.header(formatHeader));
      if (commonFormat(theirs) == null) {
        final update = updateNeeded(theirs) == SyncFailure.updateThere
            ? UpdateNeeded.there
            : UpdateNeeded.here;
        return (_refuseFormat(request.header(deviceHeader), update), null);
      }
      return (serveVector(inviting: inviting != null), null);
    }
    if (method == 'GET' && path == '/profile') {
      return (HostResponse.json(200, profileAnswer(this)), null);
    }
    if (method == 'POST' && path == '/sync') {
      // A paired Device's proof is over no PIN.
      final pin = request.challenged ? '' : request.header(pinHeader) ?? '';
      return serveSync(request.body, name: name, inviting: inviting, pin: pin);
    }
    if (path.startsWith('/blob/')) {
      final ref = path.substring('/blob/'.length);
      final (kind, hash) = splitBlobRef(ref);
      if (method == 'GET') {
        Set<String>? areas;
        if (inviting != null) (areas ??= {}).add(inviting.areaId);
        if (joiner != null && isSharedPeer(joiner)) {
          (areas ??= {}).addAll(areasSharedWith(joiner));
        }
        final bytes = blob(kind, hash);
        if (bytes == null || (areas != null && !blobIn(ref, areas))) {
          return (HostResponse.status(404), null);
        }
        return (HostResponse.bytes(bytes), null);
      }
      if (method == 'POST') {
        // Only a blob the log mentions: anything else is storage filled
        // by whoever holds the PIN.
        final kept = knowsBlob(ref) && putBlob(kind, hash, request.body);
        return (HostResponse.status(kept ? 200 : 404), null);
      }
    }
    return (HostResponse.status(404), null);
  }

  /// `/vector`: the version vector, with the wire formats and this
  /// Device's id in the headers, and the invite when this host invites.
  HostResponse serveVector({bool inviting = false}) => HostResponse.json(
    200,
    versionVector(),
    headers: {
      formatHeader: encodeFormats(ownFormats),
      deviceHeader: deviceId,
      if (inviting) inviteHeader: '1',
    },
  );

  /// `/sync`: refuses a format this build does not speak with its
  /// reason, declines an invite this Device must refuse or a joiner
  /// ignoring this host's, and answers the rest. [pin] is the one the
  /// joiner's proof is over.
  (HostResponse, SyncSession?) serveSync(
    List<int> body, {
    required String name,
    Invite? inviting,
    String pin = '',
  }) {
    final Map<String, Object?> json;
    try {
      final value = jsonDecode(utf8.decode(body));
      if (value is! Map<String, Object?>) {
        return (HostResponse.status(500), null);
      }
      json = value;
    } on FormatException {
      return (HostResponse.status(500), null);
    }
    final format = formatOf(json);
    final done = answerSync(
      this,
      json,
      name: name,
      inviting: inviting,
      pin: pin,
    );
    if (done != null) {
      updateNoLongerNeeded(done.session.peer);
      return (
        HostResponse.json(
          200,
          done.answer,
          headers: {formatHeader: encodeFormats(ownFormats)},
        ),
        done.session,
      );
    }
    if (speaksFormat(format)) return (HostResponse.declined(), null);
    final update = format > newestSyncFormat
        ? UpdateNeeded.here
        : UpdateNeeded.there;
    final joiner = json['deviceId'];
    return (_refuseFormat(joiner is String ? joiner : null, update), null);
  }

  /// The refusal of a joiner, [joiner] when it named itself, that shares
  /// no wire format with this build: this build's formats in the header
  /// and the body. Which Device needs an update is kept as an update
  /// notice, by the joiner's id, to be announced once; a joiner that did
  /// not name itself, joining by pair code, is news every time.
  HostResponse _refuseFormat(String? joiner, UpdateNeeded update) {
    final source = joiner == null || joiner.isEmpty ? lanSource : joiner;
    if (source == lanSource) updateNoLongerNeeded(lanSource);
    noteUpdateNeeded(source, update);
    final formats = encodeFormats(ownFormats);
    return HostResponse.json(
      403,
      {'refusal': 'format', 'formats': formats},
      headers: {formatHeader: formats},
    );
  }

  /// Starts the paired host with this Device's certificate on [bind], on
  /// the port it had before while that is free, and keeps the port.
  Future<LanHost> startPairedHost(InternetAddress bind) async {
    final port = int.tryParse(localSetting(pairedPortKey) ?? '') ?? 0;
    final host = await LanHost.startPaired(
      hostIdentity(),
      clock: clock,
      bind: bind,
      port: port,
    );
    if (host.port != port) setLocalSetting(pairedPortKey, '${host.port}');
    return host;
  }
}

/// Pairing as any Device hosts it (ADR 0003), the twin of the desktop's
/// Sync view: the pairing host with a fresh PIN, its pair code shown as
/// a QR and typed. A Device not paired yet gets the user's question
/// first ([ask]), its `/sync` held open meanwhile, one at a time: another
/// asking then hears 503 and tries again. While the host invites to a
/// Shared Area ([inviting]), every joiner waits so, with the Profile its
/// person sent, for the user to confirm it (ADR 0017). Every session's
/// joiner is remembered as a paired Device ([LanServing.rememberJoiner])
/// and handed to [onSession].
class PairingHost {
  final Store store;
  final LanHost host;

  /// The name this Device goes by.
  final String name;

  /// The invite to a Shared Area this host hands out (ADR 0013): whoever
  /// joins with its code gets that Area only.
  final Invite? inviting;

  /// Whether to trust the Device asking, with the Profile its person
  /// sent when the host invites.
  final Future<bool> Function(JoinAsk ask, ProfileChoice? profile) ask;

  final void Function(SyncSession session)? onSession;

  HostRequest? _held;
  late final StreamSubscription<HostRequest> _subscription;

  PairingHost._(
    this.store,
    this.host, {
    required this.name,
    required this.inviting,
    required this.ask,
    required this.onSession,
  }) {
    _subscription = host.requests.listen((r) => unawaited(_take(r)));
  }

  /// Starts hosting with a fresh PIN, or [pin], and this Device's
  /// certificate, on [bind] or else the LAN address.
  static Future<PairingHost> start(
    Store store, {
    required String name,
    required Future<bool> Function(JoinAsk ask, ProfileChoice? profile) ask,
    Invite? inviting,
    void Function(SyncSession session)? onSession,
    InternetAddress? bind,
    String? pin,
  }) async {
    final host = await LanHost.start(
      store.hostIdentity(),
      pin: pin ?? newPin(),
      clock: store.clock,
      bind: bind,
    );
    return PairingHost._(
      store,
      host,
      name: name,
      inviting: inviting,
      ask: ask,
      onSession: onSession,
    );
  }

  /// The pair code: the whole fingerprint for the QR, its first bytes
  /// for typing.
  String pairCode({required bool typed}) => host.pairCode(typed: typed);

  bool get lockedOut => host.lockedOut;

  /// Stops hosting; a Device still waiting for an answer is refused.
  Future<void> stop() async {
    _held?.reply(HostResponse.declined());
    _held = null;
    await _subscription.cancel();
    await host.stop();
  }

  Future<void> _take(HostRequest request) async {
    if (request.joinAsk case final asking?) {
      // Invited, the joiner shows its person's Profile, and the user
      // confirms it even for a Device trusted before; one without one is
      // refused by the answer.
      final profile = inviting == null ? null : request.joinerProfile(store);
      final trusted = store.trustsJoiner(asking, host.pin);
      if (!trusted || profile != null) {
        if (_held != null) {
          // One question at a time; the other tries again.
          request.reply(const HostResponse.status(503));
          return;
        }
        _held = request;
        bool yes;
        try {
          yes = await ask(asking, profile);
        } on Object {
          yes = false;
        }
        // Stopped meanwhile: the request was refused already.
        if (!identical(_held, request)) return;
        _held = null;
        if (!yes) {
          request.reply(HostResponse.declined());
          return;
        }
      }
    }
    _serve(request);
  }

  void _serve(HostRequest request) {
    final (HostResponse, SyncSession?) done;
    try {
      done = store.serve(request, name: name, inviting: inviting);
    } on Object {
      request.reply(const HostResponse.status(500));
      return;
    }
    final (response, session) = done;
    request.reply(response);
    if (session == null) return;
    store.rememberJoiner(session);
    onSession?.call(session);
  }
}

/// The paired host as every Device runs it for automatic sync (ADR
/// 0003), the twin of the desktop's: paired Devices that sync with this
/// one on its own join it without the PIN, proven by their keys
/// ([LanServing.servePaired]); on every address of this Device, its port
/// kept from run to run. The phone runs it while the app is open and
/// during its background sync, and advertises it ([serviceType]).
class PairedHost {
  final Store store;
  final LanHost host;

  /// The name this Device goes by.
  final String name;
  final void Function(SyncSession session)? onSession;
  late final StreamSubscription<HostRequest> _subscription;

  PairedHost._(this.store, this.host, {required this.name, this.onSession}) {
    _subscription = host.requests.listen(_serve);
  }

  /// Starts the paired host on [bind], every IPv4 address unless given.
  static Future<PairedHost> start(
    Store store, {
    required String name,
    InternetAddress? bind,
    void Function(SyncSession session)? onSession,
  }) async {
    final host = await store.startPairedHost(bind ?? InternetAddress.anyIPv4);
    return PairedHost._(store, host, name: name, onSession: onSession);
  }

  int get port => host.port;

  Future<void> stop() async {
    await _subscription.cancel();
    await host.stop();
  }

  void _serve(HostRequest request) {
    final (HostResponse, SyncSession?) done;
    try {
      done = store.servePaired(request, name: name);
    } on Object {
      request.reply(const HostResponse.status(500));
      return;
    }
    final (response, session) = done;
    request.reply(response);
    if (session != null) onSession?.call(session);
  }
}
