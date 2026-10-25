import 'dart:convert';
import 'dart:typed_data';

import 'area_keys.dart';
import 'box_keys.dart';
import 'entry.dart';
import 'notes.dart';
import 'pairing.dart';
import 'profiles.dart';
import 'routines.dart';
import 'schedule.dart';
import 'shared_areas.dart';
import 'signing.dart';
import 'store.dart';
import 'sync.dart';

/// Sent Items (CONTEXT.md, ADR 0009, 0015): one Note with its Routines
/// and Reminders, sent as a file through any messenger to another Old
/// Brain Fix user. The file is signed by the sender's Device key and
/// carries a stable id, so a later file with the same id can be told
/// apart as an update; it is encrypted to the recipient's Devices when
/// their keys are known here, else signed only. The receiver previews it
/// and accepts it into Unsorted, or declines and nothing is kept.
///
/// What travels is the Note's text, the texts of its Sections and the
/// schedules of its Routines and Reminders; never its Capture Context,
/// photos or voice recordings.
const sentItemExtension = 'oldbrain';

/// The type a Sent Item is shared as.
const sentItemMimeType = 'application/vnd.oldbrainfix.item';

/// The format a file says it is in.
const sentItemFormat = 1;

/// A file larger than this is no Sent Item: it holds only texts.
const maxSentItemBytes = 1 << 20;

/// The name a Sent Item written on [day] goes by. It says nothing of
/// what is in it.
String sentItemFileName(DateTime day) {
  String two(int n) => n.toString().padLeft(2, '0');
  return 'oldbrainfix-note-${day.year}-${two(day.month)}-${two(day.day)}'
      '.$sentItemExtension';
}

/// What a Sent Item's parts are bound to: the signature, the sealed
/// file and the content key sealed for each recipient.
abstract final class SentAs {
  static const signed = 'oldbrainfix:sent:';
  static const sealed = 'oldbrainfix:sent';
  static const key = 'oldbrainfix:sent-key';
}

/// The kind a cancellation says it is; an item says none.
const _cancelKind = 'cancel';

/// Field keys a Sent Item leaves in the log.
abstract final class SentKeys {
  /// On the sender's Note, one per sending Device: `sent:<device>` =
  /// `{"id":…,"version":…}`, so sending it again keeps its id.
  static const sentPrefix = 'sent:';

  static String sent(String device) => '$sentPrefix$device';

  /// On an accepted Note: `{"from":…,"id":…,"name":…,"version":…}`, the
  /// sender's key (base64), the Sent Item's id, the sender's name and
  /// the version accepted; on an accepted Routine: the id the sender's
  /// Routine has.
  static const received = 'received';
}

/// A file that is no Sent Item this app can read: no JSON, damaged, too
/// large, or not signed by the key it names.
class NotASentItem implements Exception {
  const NotASentItem();

  @override
  String toString() => 'NotASentItem';
}

/// A Sent Item whose id came here from another key: only its sender may
/// update or cancel it (ADR 0009).
class SentItemNotFromSender implements Exception {
  const SentItemNotFromSender();

  @override
  String toString() => 'SentItemNotFromSender';
}

/// A Sent Item encrypted to other Devices, not to this one.
class SentItemNotForUs implements Exception {
  const SentItemNotForUs();

  @override
  String toString() => 'SentItemNotForUs';
}

/// A Section of a Sent Item: its text and when it was captured.
class SentSection {
  final String text;
  final DateTime captured;

  const SentSection(this.text, this.captured);
}

/// A Routine or Reminder of a Sent Item: the sender's id for it and its
/// schedule.
class SentRoutine {
  final String id;
  final Schedule schedule;

  const SentRoutine(this.id, this.schedule);
}

/// What a Sent Item holds, as signed.
class SentItem {
  /// The Sent Item's id: the same every time the same Device sends the
  /// same Note.
  final String id;

  /// How often it was sent, counting from 1.
  final int version;

  /// Whether it cancels the Sent Item of this id: the receiver may then
  /// move it to the trash. It still carries what the Note holds.
  final bool cancel;

  /// The sender's key record and the name the sender goes by.
  final KeyRecord from;
  final String name;

  final String text;
  final List<SentSection> sections;
  final List<SentRoutine> routines;

  const SentItem({
    required this.id,
    required this.version,
    required this.from,
    required this.name,
    required this.text,
    this.sections = const [],
    this.routines = const [],
    this.cancel = false,
  });

  /// What is signed: keys sorted, no spaces, the same from both cores. A
  /// Reminder has no rule; only a cancellation says its kind.
  String encode() => jsonEncode({
    'from': {'key': from.toJson(), 'name': name},
    'id': id,
    if (cancel) 'kind': _cancelKind,
    'note': {
      'sections': [
        for (final s in sections)
          {'captured': s.captured.toUtc().toIso8601String(), 'text': s.text},
      ],
      'text': text,
    },
    'routines': [
      for (final r in routines)
        {
          'id': r.id,
          if (r.schedule.rule case final rule?) 'rule': rule.format(),
          'start': r.schedule.start.toString(),
          'times': r.schedule.times.join(','),
        },
    ],
    'version': version,
  });

  /// The item [text] holds; null when it is none.
  static SentItem? parse(String text) {
    try {
      final json = jsonDecode(text);
      if (json
          case {
            'from': {'key': final Object key, 'name': final String name},
            'id': final String id,
            'note': {
              'sections': final List<Object?> sections,
              'text': final String body,
            },
            'routines': final List<Object?> routines,
            'version': final int version,
          }
          when id.isNotEmpty && version > 0) {
        final from = KeyRecord.fromJson(key);
        if (from == null) return null;
        final kind = json['kind'];
        if (kind != null && kind != _cancelKind) return null;
        final parts = <SentSection>[];
        for (final s in sections) {
          if (s case {'captured': final String at, 'text': final String text}) {
            final captured = DateTime.tryParse(at);
            if (captured == null) return null;
            parts.add(SentSection(text, captured.toUtc()));
          } else {
            return null;
          }
        }
        final scheduled = <SentRoutine>[];
        for (final r in routines) {
          if (r is! Map) return null;
          if (r case {
            'id': final String rid,
            'start': final String start,
            'times': final String times,
          }) {
            final rule = r['rule'];
            if (rule != null && rule is! String) return null;
            final schedule = Routines.scheduleOf({
              RoutineKeys.start: start,
              RoutineKeys.times: times,
              RoutineKeys.rule: rule is String ? rule : null,
            });
            if (schedule == null) return null;
            scheduled.add(SentRoutine(rid, schedule));
          } else {
            return null;
          }
        }
        return SentItem(
          id: id,
          version: version,
          from: from,
          name: name,
          text: body,
          sections: parts,
          routines: scheduled,
          cancel: kind == _cancelKind,
        );
      }
    } on FormatException {
      return null;
    }
    return null;
  }
}

/// Someone a Sent Item can be encrypted to: another person met through
/// a Shared Area or Pairing by invite, with the X25519 keys of every
/// Device of theirs known here.
class Recipient {
  /// The person's id, or the Device's when no Shared Area names one.
  final String id;
  final String name;
  final List<Uint8List> keys;

  const Recipient({required this.id, required this.name, required this.keys});
}

/// Who a Sent Item can be encrypted to: the people of the Shared Areas
/// and the Devices paired by invite, never this Device's own group, each
/// with the keys of their Devices known here; by name. A person none of
/// whose keys is known is not listed: a file to them is signed only.
List<Recipient> sentItemRecipients(Store store) {
  final self = store.deviceId;
  bool own(String device) => device == self || store.isOwnDevice(device);
  final devices = <String, Set<String>>{};
  final names = <String, String>{};
  for (final area in store.judged.areas.keys) {
    for (final person in store.people(area)) {
      if (person.devices.any(own)) continue;
      devices.putIfAbsent(person.id, () => {}).addAll(person.devices);
      names.putIfAbsent(person.id, () => store.personShown(person.id).name);
    }
  }
  final grouped = {for (final ds in devices.values) ...ds};
  for (final paired in store.pairedDevices()) {
    if (!paired.shared || grouped.contains(paired.id)) continue;
    devices.putIfAbsent(paired.id, () => {}).add(paired.id);
    names.putIfAbsent(paired.id, () => paired.name);
  }
  final recipients = <Recipient>[];
  for (final MapEntry(key: id, value: ds) in devices.entries) {
    final keys = [
      for (final d in ds.toList()..sort()) ?store.pinnedKey(d)?.record.boxKey,
    ];
    if (keys.isEmpty) continue;
    recipients.add(Recipient(id: id, name: names[id] ?? '', keys: keys));
  }
  recipients.sort((a, b) {
    final byName = a.name.compareTo(b.name);
    return byName != 0 ? byName : a.id.compareTo(b.id);
  });
  return recipients;
}

/// The Sent Item the Note [noteId] goes out as now: its text, the texts
/// of its Sections and its Routines and Reminders, under the id it went
/// out with from this Device before and the next version, or a new id.
/// Records the id and version on the Note, so sending it again keeps
/// them; a Note this Device may not change goes out under a new id each
/// time. With [cancel] it goes out as the cancellation of that id.
SentItem sentItemOf(Store store, String noteId, {bool cancel = false}) {
  final notes = Notes(store);
  final note = notes.byId(noteId);
  if (note == null || note.isTrashed || note.isMerged) {
    throw ArgumentError.value(noteId, 'noteId', 'not a Note in the lists');
  }
  final field = SentKeys.sent(store.deviceId);
  var id = '';
  var version = 1;
  try {
    if (jsonDecode(store.current(noteId, field) ?? '')
        case {'id': final String held, 'version': final int was}
        when held.isNotEmpty) {
      id = held;
      version = was + 1;
    }
  } on FormatException {
    // Never sent from here.
  }
  if (id.isEmpty) id = Store.newId();
  if (store.mayChange(noteId)) {
    store.append(noteId, field, jsonEncode({'id': id, 'version': version}));
  }
  final routines = Routines(store).ofNote(noteId)
    ..sort((a, b) => a.id.compareTo(b.id));
  return SentItem(
    id: id,
    version: version,
    cancel: cancel,
    from: store.ownKeyRecord,
    name: store.author,
    text: note.text,
    sections: [
      for (final s in notes.sections(noteId))
        if (s.text.trim().isNotEmpty) SentSection(s.text, s.context.captured),
    ],
    routines: [for (final r in routines) SentRoutine(r.id, r.schedule)],
  );
}

/// The file [item] goes out as, signed by [key]: signed only when [to]
/// is empty, else encrypted to each of the X25519 keys [to]. The content
/// key, the nonces and the ephemeral keys come from [random] in this
/// order: content key, nonce of the content, then per recipient its
/// ephemeral secret and the nonce of its key; the fixture generator
/// hands in a counter.
Uint8List encodeSentItem(
  SentItem item,
  SigningKey key, {
  List<List<int>> to = const [],
  Uint8List Function(int length)? random,
}) {
  final bytesOf = random ?? secretBytes;
  final payload = item.encode();
  final signed = utf8.encode(
    jsonEncode({
      'format': sentItemFormat,
      'item': payload,
      'sig': key.sign(_signedBytes(payload)),
    }),
  );
  if (to.isEmpty) return signed;
  final content = bytesOf(areaKeyBytes);
  final sealed = seal(
    content,
    signed,
    aad: SentAs.sealed,
    nonce: bytesOf(sealNonceBytes),
  );
  final stanzas = <Map<String, Object?>>[];
  for (final recipient in to) {
    final ephemeral = bytesOf(boxKeyBytes);
    final public = boxPublicOf(ephemeral);
    final shared = x25519(ephemeral, recipient);
    if (shared == null) {
      throw ArgumentError.value(recipient, 'to', 'not a key to encrypt to');
    }
    stanzas.add({
      'eph': base64.encode(public),
      'key': base64.encode(
        seal(
          _wrapKey(shared, public, recipient),
          content,
          aad: SentAs.key,
          nonce: bytesOf(sealNonceBytes),
        ),
      ),
    });
  }
  return utf8.encode(
    jsonEncode({
      'format': sentItemFormat,
      'sealed': base64.encode(sealed),
      'to': stanzas,
    }),
  );
}

/// Writes the file the Note [noteId] goes out as ([sentItemOf],
/// [encodeSentItem]): encrypted to the keys of [to], else signed only;
/// with [cancel] as its cancellation.
Uint8List sendNote(
  Store store,
  String noteId, {
  Recipient? to,
  bool cancel = false,
}) => encodeSentItem(
  sentItemOf(store, noteId, cancel: cancel),
  store.signingKey,
  to: to?.keys ?? const [],
);

/// Whether this Device sent the Note [noteId] before, so sending it
/// again sends an update, or a cancellation.
bool wasSent(Store store, String noteId) =>
    store.current(noteId, SentKeys.sent(store.deviceId)) != null;

/// Who a Sent Item opened here comes from, as far as this Device knows.
enum SenderTrust {
  /// Their key was met in person, over a session the pair code
  /// authenticated.
  verified,

  /// Their key came along with a sync, never met in person.
  known,

  /// Never met.
  unknown,
}

/// What accepting a Sent Item opened here comes to.
enum SentItemState {
  /// Never accepted here: accepting makes a new Note in Unsorted.
  fresh,

  /// A newer version of one accepted here: accepting applies its
  /// [SentItemChanges] to that Note, wherever it is filed.
  update,

  /// The cancellation of one accepted here: accepting moves that Note to
  /// the trash with its Routines and Reminders, alarms off.
  cancel,

  /// This version or a newer one was accepted here: nothing to do.
  upToDate,

  /// The cancellation of nothing accepted here, or of a Note already in
  /// the trash: nothing to do.
  nothingToCancel,
}

/// A Routine or Reminder an update gives another schedule: the one
/// here, its schedule now and the one it gets.
class SentReschedule {
  final String routine;
  final Schedule before;
  final Schedule after;

  const SentReschedule(this.routine, this.before, this.after);
}

/// What applying an update changes on the Note accepted before. What the
/// receiver added stays: their Sections, Routines and Reminders, Done
/// marks, Importance and where the Note is filed.
class SentItemChanges {
  /// The Note's text here, when the update replaces it: only when the
  /// sender changed theirs since.
  final String? textBefore;

  /// Whether the text here was changed here since it came, so the
  /// update replaces the receiver's own change.
  final bool textEdited;

  /// Sections the sender added since; none is ever taken away.
  final List<SentSection> sectionsAdded;
  final List<SentReschedule> rescheduled;
  final List<SentRoutine> routinesAdded;

  /// The sender's Routines and Reminders here that they took away: they
  /// go to the trash, restorable.
  final List<Routine> routinesRemoved;

  /// Whether the Note is in the trash here: applying brings it back.
  final bool restores;

  const SentItemChanges({
    this.textBefore,
    this.textEdited = false,
    this.sectionsAdded = const [],
    this.rescheduled = const [],
    this.routinesAdded = const [],
    this.routinesRemoved = const [],
    this.restores = false,
  });

  /// Whether nothing but the version changes.
  bool get isEmpty =>
      textBefore == null &&
      sectionsAdded.isEmpty &&
      rescheduled.isEmpty &&
      routinesAdded.isEmpty &&
      routinesRemoved.isEmpty &&
      !restores;
}

/// A Sent Item opened here, to preview.
class OpenedSentItem {
  final SentItem item;

  /// Whether it came encrypted to this Device.
  final bool encrypted;
  final SenderTrust trust;

  /// The Note it was accepted as before, if it was, in the trash too.
  final String? accepted;

  /// What accepting it comes to.
  final SentItemState state;

  /// What an update changes; null for anything else.
  final SentItemChanges? changes;

  const OpenedSentItem({
    required this.item,
    required this.encrypted,
    required this.trust,
    required this.accepted,
    this.state = SentItemState.fresh,
    this.changes,
  });
}

/// Reads the Sent Item in [bytes]: decrypts it when it is encrypted,
/// checks the signature over it and what accepting it would do here.
/// Throws [NotASentItem] for a file that is none, damaged, or not signed
/// by the key it names, [SentItemNotForUs] for one encrypted to other
/// Devices, and [SentItemNotFromSender] for one whose id came here from
/// another key (ADR 0009).
OpenedSentItem openSentItem(Store store, List<int> bytes) {
  if (bytes.length > maxSentItemBytes) throw const NotASentItem();
  final outer = _json(bytes);
  var encrypted = false;
  var signed = outer;
  if (outer case {
    'format': sentItemFormat,
    'sealed': final String sealed,
    'to': final List<Object?> to,
  }) {
    final inner = _open(store, sealed, to);
    if (inner == null) throw const SentItemNotForUs();
    signed = _json(inner);
    encrypted = true;
  }
  if (signed case {
    'format': sentItemFormat,
    'item': final String payload,
    'sig': final String sig,
  }) {
    final item = SentItem.parse(payload);
    if (item == null ||
        !item.from.valid ||
        !verifySignature(item.from.publicKey, _signedBytes(payload), sig)) {
      throw const NotASentItem();
    }
    final pinned = store.pinnedKey(item.from.device);
    final own = item.from.device == store.deviceId;
    final held = _held(store, item);
    final (state, changes) = _judge(store, item, held);
    return OpenedSentItem(
      item: item,
      encrypted: encrypted,
      trust: own || pinned?.trust == KeyTrust.verified
          ? SenderTrust.verified
          : pinned == null
          ? SenderTrust.unknown
          : SenderTrust.known,
      accepted: held?.note,
      state: state,
      changes: changes,
    );
  }
  throw const NotASentItem();
}

/// Whether [bytes] look like a Sent Item rather than a bundle: a JSON
/// object, where a bundle is a zip.
bool looksLikeSentItem(List<int> bytes) {
  for (final b in bytes) {
    if (b == 0x20 || b == 0x09 || b == 0x0a || b == 0x0d) continue;
    return b == 0x7b; // {
  }
  return false;
}

/// The Note [item] was accepted as here before, from the same key, in
/// the trash or merged too: the newest of them; null when there is none.
/// Throws [SentItemNotFromSender] when its id came here from another key
/// only.
String? acceptedNote(Store store, SentItem item) => _held(store, item)?.note;

/// Accepts [item] as [openSentItem] judged it ([SentItemState]): a new
/// Note of this Device's in Unsorted, captured now, with the Sections and
/// the Routines and Reminders it holds, their alarms on; for an update,
/// its changes applied to the Note accepted before ([SentItemChanges]);
/// for a cancellation, that Note moved to the trash with its Routines and
/// Reminders. The Note and its Routines keep the Sent Item's id, version
/// and sender's key, and the sender's id for each Routine, so a later
/// version can be recognised. Returns the Note it is held as; null when
/// there was nothing to cancel. Throws [SentItemNotFromSender] as
/// [openSentItem] does.
String? acceptSentItem(Store store, SentItem item) {
  final held = _held(store, item);
  final (state, changes) = _judge(store, item, held);
  final now = store.clock().toUtc();
  switch ((state, held, changes)) {
    case (SentItemState.fresh, _, _):
      return _acceptNew(store, item);
    case (SentItemState.update, final held?, final changes?):
      _apply(store, held.note, item, changes, now);
      return held.note;
    case (SentItemState.cancel, final held?, _):
      _cancel(store, held.note, item, now);
      return held.note;
    case (SentItemState.upToDate, final held?, _):
      return held.note;
    default:
      return null;
  }
}

/// The Note a Sent Item was accepted as here and the version it holds.
typedef _Held = ({String note, int version});

/// The Note [item] was accepted as here from the same key, in the trash
/// or merged too, the newest of them, with the version it holds. Throws
/// [SentItemNotFromSender] when only another key's Sent Item has its id.
_Held? _held(Store store, SentItem item) {
  final from = base64.encode(item.from.publicKey);
  final notes = Notes(store);
  _Held? found;
  DateTime? newest;
  var foreign = false;
  for (final id in store.entitiesOfType(NoteKeys.typeNote)) {
    final link = _received(store.current(id, SentKeys.received));
    if (link == null || link.id != item.id) continue;
    if (link.from != from) {
      foreign = true;
      continue;
    }
    final note = notes.byId(id);
    if (note == null) continue;
    if (newest == null || note.captured.isAfter(newest)) {
      newest = note.captured;
      found = (note: id, version: link.version);
    }
  }
  if (found == null && foreign) throw const SentItemNotFromSender();
  return found;
}

/// What accepting [item] comes to, with the Note it was accepted as
/// before [held], and for an update what it changes.
(SentItemState, SentItemChanges?) _judge(
  Store store,
  SentItem item,
  _Held? held,
) {
  if (held == null) {
    return (
      item.cancel ? SentItemState.nothingToCancel : SentItemState.fresh,
      null,
    );
  }
  if (item.version <= held.version) return (SentItemState.upToDate, null);
  final trashed = Notes(store).byId(held.note)?.isTrashed ?? false;
  if (item.cancel) {
    return (
      trashed ? SentItemState.nothingToCancel : SentItemState.cancel,
      null,
    );
  }
  return (SentItemState.update, _changes(store, held.note, item, trashed));
}

/// What the update [item] changes on the Note [noteId]: its text when the
/// sender changed theirs, the Sections not here yet (by when they were
/// captured), and the sender's Routines and Reminders, known by the
/// sender's id for each, rescheduled, added or taken away. Those the
/// receiver put in the trash on their own stay there, untouched; when
/// the Note is in the trash ([restores]) those that went with it count.
SentItemChanges _changes(
  Store store,
  String noteId,
  SentItem item,
  bool restores,
) {
  final notes = Notes(store);
  final text = notes.byId(noteId)?.text ?? '';
  final sent = _lastSentText(store, noteId);
  final changed = item.text != text && (sent == null || item.text != sent);
  final here = {
    for (final s in notes.sections(noteId))
      s.context.captured.toUtc().microsecondsSinceEpoch,
  };
  final routines = Routines(store);
  final theirs = <String, Routine>{};
  for (final id in store.entitiesOfType(RoutineKeys.typeRoutine)..sort()) {
    if (store.current(id, RoutineKeys.note) != noteId) continue;
    final from = store.current(id, SentKeys.received);
    final routine = routines.byId(id);
    if (from == null || routine == null) continue;
    theirs.putIfAbsent(from, () => routine);
  }
  bool live(Routine r) => !r.isTrashed || (restores && r.trashedWith == noteId);
  final rescheduled = <SentReschedule>[];
  final added = <SentRoutine>[];
  for (final r in item.routines) {
    final held = theirs[r.id];
    if (held == null) {
      added.add(r);
    } else if (live(held) && !_sameSchedule(held.schedule, r.schedule)) {
      rescheduled.add(SentReschedule(held.id, held.schedule, r.schedule));
    }
  }
  final kept = {for (final r in item.routines) r.id};
  return SentItemChanges(
    textBefore: changed ? text : null,
    textEdited: changed && sent != null && text != sent,
    sectionsAdded: [
      for (final s in item.sections)
        if (!here.contains(s.captured.toUtc().microsecondsSinceEpoch)) s,
    ],
    rescheduled: rescheduled,
    routinesAdded: added,
    routinesRemoved: [
      for (final MapEntry(key: from, value: r) in theirs.entries)
        if (!kept.contains(from) && live(r)) r,
    ],
    restores: restores,
  );
}

/// The text the sender last sent for the Note [noteId]: the newest text
/// written with its link, as accepting and applying write both at once;
/// null when there is none.
String? _lastSentText(Store store, String noteId) {
  final history = store.history(noteId);
  final linked = {
    for (final e in history)
      if (e.field == SentKeys.received) e.date.microsecondsSinceEpoch,
  };
  Entry? last;
  for (final e in history) {
    if (e.field != NoteKeys.text ||
        !linked.contains(e.date.microsecondsSinceEpoch)) {
      continue;
    }
    if (last == null || !e.date.isBefore(last.date)) last = e;
  }
  return last?.value;
}

bool _sameSchedule(Schedule a, Schedule b) =>
    a.start == b.start &&
    a.times.join(',') == b.times.join(',') &&
    a.rule?.format() == b.rule?.format();

/// The link an accepted Note keeps to [item].
String _link(SentItem item) => jsonEncode({
  'from': base64.encode(item.from.publicKey),
  'id': item.id,
  'name': item.name,
  'version': item.version,
});

/// A new Note of this Device's in Unsorted for [item], captured now.
String _acceptNew(Store store, SentItem item) {
  final id = 'note:${Store.newId()}';
  final now = store.clock().toUtc();
  store.batch(() {
    store.append(id, NoteKeys.type, NoteKeys.typeNote, date: now);
    store.append(id, NoteKeys.captured, now.toIso8601String(), date: now);
    store.append(id, NoteKeys.text, item.text, date: now);
    store.append(id, SentKeys.received, _link(item), date: now);
    for (final s in item.sections) {
      _writeSection(store, id, s);
    }
    // Written as they are, not spawned: the Note stays in Unsorted.
    for (final r in item.routines) {
      _writeRoutine(store, id, r);
    }
  });
  return id;
}

/// Applies the update [item] to the Note [noteId] as [changes] says, at
/// [now], in one go: back from the trash first when it is there, then
/// its text, its link, the Sections added, the Routines and Reminders
/// rescheduled, added and taken away.
void _apply(
  Store store,
  String noteId,
  SentItem item,
  SentItemChanges changes,
  DateTime now,
) {
  final at = now.toIso8601String();
  final routines = Routines(store);
  store.batch(() {
    if (changes.restores) {
      store.append(noteId, NoteKeys.trashed, null);
      for (final id in store.entitiesOfType(RoutineKeys.typeRoutine)..sort()) {
        final routine = routines.byId(id);
        if (routine == null || routine.trashedWith != noteId) continue;
        store.append(id, RoutineKeys.trashed, null);
        store.append(id, RoutineKeys.trashedWith, null);
      }
    }
    if (changes.textBefore != null) {
      store.append(noteId, NoteKeys.text, item.text, date: now);
    }
    store.append(noteId, SentKeys.received, _link(item), date: now);
    for (final s in changes.sectionsAdded) {
      _writeSection(store, noteId, s);
    }
    for (final r in changes.rescheduled) {
      _writeSchedule(store, r.routine, r.after);
    }
    for (final r in changes.routinesAdded) {
      _writeRoutine(store, noteId, r);
    }
    for (final r in changes.routinesRemoved) {
      store.append(r.id, RoutineKeys.trashed, at);
    }
  });
}

/// Moves the Note [noteId] to the trash for the cancellation [item], at
/// [now], with its Routines and Reminders: they come back with it. Those
/// of its own count even when the receiver merged it into another Note.
void _cancel(Store store, String noteId, SentItem item, DateTime now) {
  final at = now.toIso8601String();
  final routines = Routines(store);
  final ids = {
    for (final r in routines.ofNote(noteId)) r.id,
    for (final id in store.entitiesOfType(RoutineKeys.typeRoutine))
      if (routines.byId(id) case final r?
          when r.noteId == noteId && !r.isTrashed)
        id,
  }.toList()..sort();
  store.batch(() {
    store.append(noteId, NoteKeys.trashed, at);
    for (final id in ids) {
      store.append(id, RoutineKeys.trashed, at);
      store.append(id, RoutineKeys.trashedWith, noteId);
    }
    store.append(noteId, SentKeys.received, _link(item), date: now);
  });
}

/// A Section of the Note [noteId] holding [section].
void _writeSection(Store store, String noteId, SentSection section) {
  final id = 'section:${Store.newId()}';
  store.append(id, SectionKeys.type, SectionKeys.typeSection);
  store.append(id, SectionKeys.note, noteId);
  store.append(
    id,
    SectionKeys.captured,
    section.captured.toUtc().toIso8601String(),
  );
  store.append(id, SectionKeys.text, section.text);
}

/// A Routine or Reminder of the Note [noteId] as [routine] is, knowing
/// the sender's id for it; written, not spawned.
void _writeRoutine(Store store, String noteId, SentRoutine routine) {
  final id = 'routine:${Store.newId()}';
  final schedule = routine.schedule;
  store.append(id, RoutineKeys.type, RoutineKeys.typeRoutine);
  store.append(id, RoutineKeys.note, noteId);
  store.append(id, RoutineKeys.start, schedule.start.toString());
  store.append(id, RoutineKeys.times, schedule.times.join(','));
  if (schedule.rule case final rule?) {
    store.append(id, RoutineKeys.rule, rule.format());
  }
  store.append(id, SentKeys.received, routine.id);
}

/// Gives the Routine [id] [schedule], writing only what changes, so what
/// was Done stays Done (as [Routines.reschedule], inside a batch).
void _writeSchedule(Store store, String id, Schedule schedule) {
  final current = store.currentFields(id);
  void write(String field, String? value) {
    if (current.containsKey(field) && current[field] == value) return;
    store.append(id, field, value);
  }

  write(RoutineKeys.start, schedule.start.toString());
  write(RoutineKeys.times, schedule.times.join(','));
  write(RoutineKeys.rule, schedule.rule?.format());
}

/// What a signature over [payload] covers.
Uint8List _signedBytes(String payload) =>
    utf8.encode('${SentAs.signed}$payload');

/// The key the content key is sealed with for one recipient: HKDF over
/// the X25519 secret, bound to both public keys.
Uint8List _wrapKey(List<int> shared, List<int> ephemeral, List<int> to) =>
    hkdfSha256(
      shared,
      salt: [...ephemeral, ...to],
      info: utf8.encode(SentAs.key),
    );

/// The signed file inside [sealed], opened with the first of [to] sealed
/// for this Device's X25519 key; null when none is.
Uint8List? _open(Store store, String sealed, List<Object?> to) {
  final Uint8List body;
  try {
    body = base64.decode(sealed);
  } on FormatException {
    throw const NotASentItem();
  }
  final secret = boxSecretOf(store.signingKey.seed);
  final mine = boxPublicOf(secret);
  for (final stanza in to) {
    if (stanza case {'eph': final String eph, 'key': final String wrapped}) {
      try {
        final public = base64.decode(eph);
        final shared = x25519(secret, public);
        if (shared == null) continue;
        final content = unseal(
          _wrapKey(shared, public, mine),
          base64.decode(wrapped),
          aad: SentAs.key,
        );
        if (content == null || content.length != areaKeyBytes) continue;
        return unseal(content, body, aad: SentAs.sealed);
      } on FormatException {
        continue;
      }
    }
  }
  return null;
}

Object? _json(List<int> bytes) {
  try {
    return jsonDecode(utf8.decode(bytes));
  } on FormatException {
    throw const NotASentItem();
  }
}

/// The link [raw] on an accepted Note holds: the sender's key, the Sent
/// Item's id and the version accepted.
({String from, String id, int version})? _received(String? raw) {
  if (raw == null) return null;
  try {
    if (jsonDecode(raw) case {
      'from': final String from,
      'id': final String id,
      'version': final int version,
    }) {
      return (from: from, id: id, version: version);
    }
  } on FormatException {
    return null;
  }
  return null;
}
