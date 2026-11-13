import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'lan.dart';
import 'pairing.dart';
import 'shared_areas.dart';
import 'signing.dart';
import 'store.dart';

/// The phone's question through the desktop (#267, ADR 0003, ADR 0051):
/// the phone runs no model and talks to no provider, so it asks the
/// user's own desktop over the paired LAN connection, and the desktop
/// runs the question as its Ask panel does, with its own provider and
/// key, and sends back the answer with the Notes it links, or why there
/// is none. Any change the model makes waits as an AI Task for the review
/// on the desktop; the answer says when one does.
///
/// Two requests on the desktop's paired host, each with this Device's
/// proof over a fresh nonce, as automatic sync makes them, and only once
/// the host's certificate is the one pinned for it: `GET /ask/provider`
/// names the provider the desktop uses, never its key, which the phone
/// shows before its first question (App Store guideline 5.1.2(i)), and
/// `POST /ask` carries the question and the earlier exchanges of the same
/// conversation as text. Only the user's own Devices ask; a Device of
/// another person is never asked, as the question would go to their
/// provider. The desktop core's `lan_ask.rs` writes and reads the same
/// bodies.

/// Where the phone asks its question.
const askPath = '/ask';

/// Where the phone learns which provider the desktop asks.
const providerPath = '/ask/provider';

/// How the desktop talks to its provider: Anthropic's Messages API, or
/// the OpenAI-compatible Chat Completions API at an address.
enum AskProtocol { anthropic, openai }

/// Why the desktop's provider cannot be asked yet: no model is named,
/// Anthropic has no key, or the address is not one it may ask.
enum AskUnready { noModel, noKey, badUrl }

/// The provider the desktop asks, as the phone may learn it: the
/// protocol, the model, and the host of its address with its port; never
/// its key.
class DesktopProvider {
  final AskProtocol protocol;
  final String model;
  final String host;

  /// Why it cannot be asked yet; null when it can.
  final AskUnready? unready;

  const DesktopProvider({
    required this.protocol,
    required this.model,
    required this.host,
    this.unready,
  });

  bool get ready => unready == null;

  /// The body of `GET /ask/provider`, keys in order as the desktop core
  /// writes them.
  Map<String, Object?> toJson() => {
    'host': host,
    'model': model,
    'protocol': protocol.name,
    'ready': ready,
    if (unready case final why?) 'unready': why.name,
  };

  /// The provider [json] names; null for a protocol this build does not
  /// know.
  static DesktopProvider? fromJson(Object? json) {
    if (json is! Map) return null;
    final protocol = _byName(AskProtocol.values, json['protocol']);
    if (protocol == null) return null;
    final model = json['model'];
    final host = json['host'];
    return DesktopProvider(
      protocol: protocol,
      model: model is String ? model : '',
      host: host is String ? host : '',
      // Not ready for a reason this build does not know: no model.
      unready: json['ready'] == true
          ? null
          : _byName(AskUnready.values, json['unready']) ?? AskUnready.noModel,
    );
  }

  @override
  bool operator ==(Object other) =>
      other is DesktopProvider &&
      other.protocol == protocol &&
      other.model == model &&
      other.host == host &&
      other.unready == unready;

  @override
  int get hashCode => Object.hash(protocol, model, host, unready);

  @override
  String toString() => 'DesktopProvider(${jsonEncode(toJson())})';
}

/// An earlier question of the same conversation and its answer, which a
/// follow-up is asked after.
class AskExchange {
  final String question;
  final String answer;

  const AskExchange(this.question, this.answer);
}

/// The body of `POST /ask`: [question] after the [earlier] exchanges of
/// the same conversation, keys in order as the desktop core writes them.
Map<String, Object?> askBody(String question, List<AskExchange> earlier) => {
  'earlier': [
    for (final e in earlier) {'answer': e.answer, 'question': e.question},
  ],
  'question': question,
};

/// The answer as the phone gets it: its Markdown text, the ids of the
/// Notes it links as app links in the order first linked (ADR 0046),
/// which open on the phone, and whether it was cut off at its length.
class DesktopAnswer {
  final String text;
  final List<String> notes;
  final bool cut;

  const DesktopAnswer({
    required this.text,
    required this.notes,
    this.cut = false,
  });
}

/// Why the desktop gave no answer to the question.
enum AskFailure {
  /// No provider is set up on the desktop yet ([AskReply.unready]).
  unready,

  /// The provider refused the desktop's API key ([AskReply.message]).
  key,

  /// The provider answered with another error ([AskReply.status] and
  /// [AskReply.message]).
  provider,

  /// The desktop got no reply from the provider ([AskReply.message]).
  network,

  /// The provider's reply could not be read ([AskReply.message]).
  unreadable,

  /// The model called its tools too often.
  rounds,

  /// The model declined to answer.
  refused,

  /// The desktop stopped waiting.
  stopped,

  /// The desktop answers another question now.
  busy,

  /// A kind a later desktop gives, which this build does not know.
  unknown,
}

/// The desktop's reply to the question: the [answer], or the [failure]
/// with what the provider said; and whether changes of an AI Task now
/// wait for the review on the desktop ([review]).
class AskReply {
  final DesktopAnswer? answer;
  final AskFailure? failure;

  /// The provider's message of a failure, in its own words; empty for
  /// none.
  final String message;

  /// The HTTP status of a [AskFailure.provider]; 0 for none.
  final int status;

  /// Why no provider is set up, for [AskFailure.unready], when the
  /// desktop said a reason this build knows.
  final AskUnready? unready;
  final bool review;

  const AskReply.answered(DesktopAnswer this.answer, {this.review = false})
    : failure = null,
      message = '',
      status = 0,
      unready = null;

  const AskReply.failed(
    AskFailure this.failure, {
    this.message = '',
    this.status = 0,
    this.unready,
    this.review = false,
  }) : answer = null;

  /// The body of the answer to `POST /ask`, keys in order as the desktop
  /// core writes them; a message, a status and a reason only where the
  /// kind has one.
  Map<String, Object?> toJson() {
    if (answer case final answer?) {
      return {
        'answer': {
          'cut': answer.cut,
          'notes': answer.notes,
          'text': answer.text,
        },
        'review': review,
      };
    }
    final kind = failure ?? AskFailure.unknown;
    return {
      'failure': kind.name,
      if (_messaged.contains(kind)) 'message': message,
      'review': review,
      if (kind == AskFailure.provider) 'status': status,
      if (kind == AskFailure.unready)
        if (unready case final why?) 'unready': why.name,
    };
  }

  static const _messaged = {
    AskFailure.key,
    AskFailure.provider,
    AskFailure.network,
    AskFailure.unreadable,
  };

  /// The reply [json] holds; null when it is neither an answer nor a
  /// failure. A failure of a kind this build does not know is
  /// [AskFailure.unknown].
  static AskReply? fromJson(Object? json) {
    if (json is! Map) return null;
    final review = json['review'] == true;
    final answered = json['answer'];
    if (answered != null) {
      if (answered is! Map) return null;
      final text = answered['text'];
      if (text is! String) return null;
      final notes = answered['notes'];
      return AskReply.answered(
        DesktopAnswer(
          text: text,
          notes: [
            if (notes is List)
              for (final id in notes)
                if (id is String) id,
          ],
          cut: answered['cut'] == true,
        ),
        review: review,
      );
    }
    final kind = json['failure'];
    if (kind is! String) return null;
    final failure = _byName(AskFailure.values, kind) ?? AskFailure.unknown;
    final message = json['message'];
    final status = json['status'];
    return AskReply.failed(
      failure,
      message: _messaged.contains(failure) && message is String ? message : '',
      status:
          failure == AskFailure.provider &&
              status is int &&
              status >= 0 &&
              status <= 65535
          ? status
          : 0,
      unready: failure == AskFailure.unready
          ? _byName(AskUnready.values, json['unready'])
          : null,
      review: review,
    );
  }
}

/// What kept the question or the provider query from the desktop.
enum AskTrouble {
  /// The Device is no own Device of the user's, or not paired: it is
  /// never asked.
  notOwn,

  /// No address of it answered: it is off, or not on this network nor
  /// through a VPN at its address by hand.
  unreachable,

  /// Another certificate answered than the one pinned, or the Device
  /// never hosted.
  wrongHost,

  /// It does not let this Device ask: it does not know it, or its
  /// automatic sync with it is switched off.
  declined,

  /// It answers no questions: a phone, or a desktop of an older version.
  notOffered,

  /// No answer came within the time allowed.
  timedOut,

  /// Anything else went wrong on the way.
  failed,
}

class AskException implements Exception {
  final AskTrouble trouble;
  const AskException(this.trouble);

  @override
  String toString() => 'AskException(${trouble.name})';
}

/// The provider the paired desktop [device] asks, from the first of [at]
/// that answers ([Pairing.pairedAddresses]), for the disclosure before the
/// first question. Throws an [AskException].
Future<DesktopProvider> desktopProvider(
  Store store,
  PairedDevice device, {
  required List<PairedAddress> at,
  Duration timeout = const Duration(seconds: 20),
}) async {
  final data = await _askCall(
    store,
    device,
    at,
    'GET',
    providerPath,
    timeout: timeout,
    wait: timeout,
  );
  return DesktopProvider.fromJson(_decode(data)) ??
      (throw const AskException(AskTrouble.failed));
}

/// Asks the paired desktop [device] [question], after the [earlier]
/// exchanges of the same conversation, at the first of [at] that answers
/// ([Pairing.pairedAddresses]), waiting up to [wait] for the answer, as
/// the model may search and read for minutes. A question once sent is
/// not sent again elsewhere. Throws an [AskException]; a question the
/// desktop could not answer is a reply with its [AskReply.failure].
Future<AskReply> askDesktop(
  Store store,
  PairedDevice device, {
  required List<PairedAddress> at,
  required String question,
  List<AskExchange> earlier = const [],
  Duration timeout = const Duration(seconds: 20),
  Duration wait = const Duration(minutes: 10),
}) async {
  final data = await _askCall(
    store,
    device,
    at,
    'POST',
    askPath,
    body: utf8.encode(jsonEncode(askBody(question, earlier))),
    timeout: timeout,
    wait: wait,
  );
  return AskReply.fromJson(_decode(data)) ??
      (throw const AskException(AskTrouble.failed));
}

/// One request to [path] on the paired [device], at the first of [at]
/// whose host shows the pinned certificate and hands out a nonce, with
/// this Device's proof over it; the body of its answer.
Future<Uint8List> _askCall(
  Store store,
  PairedDevice device,
  List<PairedAddress> at,
  String method,
  String path, {
  List<int>? body,
  required Duration timeout,
  required Duration wait,
}) async {
  // Paired as one's own, it is one's own, as [isOthersDevice] says too.
  if (!store.isOwnDevice(device.id)) {
    throw const AskException(AskTrouble.notOwn);
  }
  final pinned = fromHex(device.fingerprint);
  if (pinned.length != certificateFingerprintBytes) {
    throw const AskException(AskTrouble.wrongHost);
  }
  var trouble = AskTrouble.unreachable;
  for (final address in at) {
    if (!store.mayReachPaired(device, address.host)) continue;
    var seen = false;
    // No root is trusted, so every certificate meets the pin, also at a
    // name typed by hand that a public certificate covers.
    final client = HttpClient(context: SecurityContext(withTrustedRoots: false))
      ..connectionTimeout = timeout
      ..badCertificateCallback = (cert, _, _) {
        seen = fingerprintMatches(certificateFingerprint(cert.der), pinned);
        return seen;
      };
    try {
      final String nonce;
      try {
        final challenge = await _send(
          client,
          address,
          'GET',
          '/challenge',
          const {},
          null,
          timeout,
          seen: () => seen,
        );
        final value = _decode(challenge)['nonce'];
        if (value is! String || value.isEmpty) {
          throw const AskException(AskTrouble.failed);
        }
        // Nothing goes to a host that did not show the pinned certificate.
        if (!seen) throw const AskException(AskTrouble.wrongHost);
        nonce = value;
      } on AskException catch (e) {
        // Not there, or another Device there: the next address.
        if (e.trouble != AskTrouble.unreachable &&
            e.trouble != AskTrouble.wrongHost &&
            e.trouble != AskTrouble.timedOut) {
          rethrow;
        }
        trouble = e.trouble;
        continue;
      }
      return await _send(
        client,
        address,
        method,
        path,
        {
          deviceHeader: store.deviceId,
          nonceHeader: nonce,
          proofHeader: store.signingKey.sign(
            pairedProofBytes(device.id, nonce),
          ),
        },
        body,
        wait,
        seen: () => seen,
      );
    } finally {
      client.close(force: true);
    }
  }
  throw AskException(trouble);
}

/// Sends one request and reads its answer within [wait].
Future<Uint8List> _send(
  HttpClient client,
  PairedAddress address,
  String method,
  String path,
  Map<String, String> headers,
  List<int>? body,
  Duration wait, {
  required bool Function() seen,
}) async {
  try {
    Future<(HttpClientResponse, Uint8List)> exchange() async {
      final req = await client.openUrl(
        method,
        Uri(
          scheme: 'https',
          host: address.host,
          port: address.port,
          path: path,
        ),
      );
      req.headers.set(formatHeader, encodeFormats(ownFormats));
      headers.forEach(req.headers.set);
      if (body != null) req.headers.contentType = ContentType.json;
      req.contentLength = body?.length ?? 0;
      if (body != null) req.add(body);
      final res = await req.close();
      final bytes = BytesBuilder(copy: false);
      await for (final chunk in res) {
        bytes.add(chunk);
        if (bytes.length > maxEntriesBytes) {
          throw const AskException(AskTrouble.failed);
        }
      }
      return (res, bytes.takeBytes());
    }

    final (res, data) = await exchange().timeout(wait);
    return switch (res.statusCode) {
      HttpStatus.ok => data,
      HttpStatus.forbidden => throw const AskException(AskTrouble.declined),
      HttpStatus.notFound => throw const AskException(AskTrouble.notOffered),
      _ => throw const AskException(AskTrouble.failed),
    };
  } on TimeoutException {
    throw const AskException(AskTrouble.timedOut);
  } on HandshakeException {
    throw AskException(seen() ? AskTrouble.failed : AskTrouble.wrongHost);
  } on SocketException {
    throw const AskException(AskTrouble.unreachable);
  } on IOException {
    throw const AskException(AskTrouble.failed);
  }
}

Map<String, Object?> _decode(Uint8List data) {
  try {
    final value = jsonDecode(utf8.decode(data));
    if (value is Map<String, Object?>) return value;
  } on FormatException {
    // Falls through to the failure below.
  }
  throw const AskException(AskTrouble.failed);
}

/// The value of [values] named [name], or null.
T? _byName<T extends Enum>(List<T> values, Object? name) {
  for (final value in values) {
    if (value.name == name) return value;
  }
  return null;
}
