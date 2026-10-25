import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// A long-lived self-signed test certificate (ECDSA P-256), as the
/// desktop makes one per install.
const testCert = '''
-----BEGIN CERTIFICATE-----
MIIBjDCCATOgAwIBAgIUEj8dc0/kZ2BaNPGw8bjFnPe3eHYwCgYIKoZIzj0EAwIw
GzEZMBcGA1UEAwwQb2xkYnJhaW5maXgtdGVzdDAgFw0yNjEwMjUwMTEzMzBaGA8y
MTI2MTAwMTAxMTMzMFowGzEZMBcGA1UEAwwQb2xkYnJhaW5maXgtdGVzdDBZMBMG
ByqGSM49AgEGCCqGSM49AwEHA0IABIKGeiWsptaFV/Zvu2+FC7PijpHW/zk5/aFJ
wHbjg585hXWjo9dmCo4BLMopxHzewJTqtd21MzLaqM08B9T2DCajUzBRMB0GA1Ud
DgQWBBT/LAbkaYDq/siubHUbWBHSK7Wo0DAfBgNVHSMEGDAWgBT/LAbkaYDq/siu
bHUbWBHSK7Wo0DAPBgNVHRMBAf8EBTADAQH/MAoGCCqGSM49BAMCA0cAMEQCICw8
iY7YYYRlIl1vjPx21T3QEAiNQkTA1T9zEz+Av5jxAiAiYHmlyHKtoEaKoqCG7zPP
Ya0WiHhXLWpBETvaCaoHng==
-----END CERTIFICATE-----
''';

const testKey = '''
-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQg/n4vHI7vg1QTBaYF
D/4EyJiiUYwbLlM660kHW+0hcaahRANCAASChnolrKbWhVf2b7tvhQuz4o6R1v85
Of2hScB244OfOYV1o6PXZgqOASzKKcR83sCU6rXdtTMy2qjNPAfU9gwm
-----END PRIVATE KEY-----
''';

/// The DER bytes of [testCert].
Uint8List get testCertDer => base64.decode(
  testCert
      .replaceAll(RegExp(r'-----[A-Z ]+-----'), '')
      .replaceAll(RegExp(r'\s'), ''),
);

/// A host as the desktop runs one, for the joiner's tests: the PIN, or a
/// paired Device's proof over a nonce from `/challenge`, on every
/// request, `/vector`, `/sync` and the blobs answered by the core.
class TestHost {
  final Store store;
  final String pin;
  final String name;
  late final HttpServer _server;

  /// Every session a joiner completed, and the `/sync` bodies it sent.
  final sessions = <SyncSession>[];
  final bodies = <Map<String, Object?>>[];

  /// Every request that reached the host, and the nonces it handed out.
  var requests = 0;
  final nonces = <String>[];

  TestHost(this.store, {this.pin = '246810', this.name = 'Desk'});

  int get port => _server.port;

  Future<void> start() async {
    final context = SecurityContext()
      ..useCertificateChainBytes(utf8.encode(testCert))
      ..usePrivateKeyBytes(utf8.encode(testKey));
    _server = await HttpServer.bindSecure(
      InternetAddress.loopbackIPv4,
      0,
      context,
    );
    _server.listen(_handle);
  }

  Future<void> stop() => _server.close(force: true);

  /// The code a phone types for this host.
  String code({bool typed = true}) => encodePairCode(
    '127.0.0.1',
    port,
    pin,
    certificateFingerprint(testCertDer),
    typed: typed,
  );

  Future<void> _handle(HttpRequest req) async {
    final res = req.response;
    try {
      final body = await req.fold<List<int>>([], (a, b) => a..addAll(b));
      requests++;
      final path = req.uri.path;
      if (req.method == 'GET' && path == '/challenge') {
        final nonce = 'nonce${nonces.length}';
        nonces.add(nonce);
        res.headers.contentType = ContentType.json;
        res.write(jsonEncode({'nonce': nonce}));
        return;
      }
      final nonce = req.headers.value(nonceHeader);
      final joiner = req.headers.value(deviceHeader);
      final paired =
          nonce != null &&
          joiner != null &&
          nonces.contains(nonce) &&
          trustsPairedJoiner(
            store,
            device: joiner,
            nonce: nonce,
            proof: req.headers.value(proofHeader),
          );
      if (!paired && req.headers.value(pinHeader) != pin) {
        res.statusCode = HttpStatus.forbidden;
        return;
      }
      if (req.method == 'GET' && path == '/vector') {
        res.headers.set(formatHeader, '$syncFormat');
        res.headers.set(deviceHeader, store.deviceId);
        res.headers.contentType = ContentType.json;
        res.write(jsonEncode(store.versionVector()));
      } else if (req.method == 'POST' && path == '/sync') {
        final sent = (jsonDecode(utf8.decode(body)) as Map)
            .cast<String, Object?>();
        bodies.add(sent);
        final done = answerSync(store, sent, name: name);
        if (done == null) {
          res.statusCode = HttpStatus.forbidden;
          res.write(jsonEncode({'refusal': 'format', 'format': syncFormat}));
          return;
        }
        sessions.add(done.session);
        res.headers.contentType = ContentType.json;
        res.write(jsonEncode(done.answer));
      } else if (path.startsWith('/blob/')) {
        final ref = path.substring('/blob/'.length);
        final (kind, hash) = splitBlobRef(ref);
        if (req.method == 'GET') {
          final bytes = store.blob(kind, hash);
          if (bytes == null) {
            res.statusCode = HttpStatus.notFound;
          } else {
            res.add(bytes);
          }
        } else if (!store.knowsBlob(ref) ||
            !store.putBlob(kind, hash, Uint8List.fromList(body))) {
          res.statusCode = HttpStatus.notFound;
        }
      } else {
        res.statusCode = HttpStatus.notFound;
      }
    } finally {
      await res.close();
    }
  }
}
