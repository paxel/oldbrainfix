import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'area_keys.dart';
import 'box_keys.dart';
import 'changes_nudges.dart';
import 'entry.dart';
import 'folder_sync.dart';
import 'frogs.dart';
import 'notes.dart';
import 'pending.dart';
import 'places.dart';
import 'profiles.dart';
import 'routines.dart';
import 'sent_items.dart' show SenderTrust;
import 'shared_areas.dart';
import 'signing.dart';
import 'store.dart';
import 'tallies.dart';
import 'sync.dart';

/// Sharing an Area with a person from afar (#108, ADR 0023): someone in
/// People who is not on the same network, added by a Profile card or met
/// before. The sharer grants that person's Devices a role as an invite
/// does (ADR 0013), then sends an offer: a bundle (ADR 0003) of the
/// Area's whole history sealed with the Area's key, with a file `offer`
/// beside it that carries the key, signed by the sharer's Device and
/// sealed to the person's X25519 keys as a Sent Item is (ADR 0015).
///
/// The receiver holds the offer as Pending Changes (ADR 0022), never
/// shown, until the user accepts or declines it; nothing of the Area is
/// kept before. Afterwards the person's changes travel by the shared
/// folder or by bundles, as the sharer chose when sharing.

/// The kind of pending set an offered Area is held in (ADR 0022).
const areaOfferPendingKind = 'area-offer';

/// The name of the offer inside a bundle.
const areaOfferFile = 'offer';

/// What the signature over an offer and its sealing are bound to: never
/// a Sent Item's, so neither stands in for the other.
abstract final class OfferAs {
  static const signed = 'oldbrainfix:area-offer:';
  static const sealed = 'oldbrainfix:area-offer';
  static const key = 'oldbrainfix:area-offer-key';
}

/// How the changes of a Shared Area travel with a person met from afar,
/// asked once when sharing with them.
enum Transport {
  /// A shared folder both reach (Nextcloud, Syncthing, a USB folder):
  /// runs on its own.
  folder,

  /// Bundles sent through any messenger.
  bundle,
}

/// A file that holds no offer this app can read: damaged, not signed by
/// the key it names, with a Profile not bound to that key, or without a
/// grant of the Area to the one who sent it.
class NotAnAreaOffer implements Exception {
  const NotAnAreaOffer();

  @override
  String toString() => 'NotAnAreaOffer';
}

/// An offer sealed for other Devices, or one that grants this Device
/// nothing: another person's, or another Device of the user's.
class AreaOfferNotForUs implements Exception {
  const AreaOfferNotForUs();

  @override
  String toString() => 'AreaOfferNotForUs';
}

/// An offer from a person the user blocked (ADR 0021): nothing of it is
/// shown or kept.
class AreaOfferFromBlocked implements Exception {
  const AreaOfferFromBlocked();

  @override
  String toString() => 'AreaOfferFromBlocked';
}

/// An offer of an Area this Device holds as its own, which nobody shared
/// with it (ADR 0013): it is refused as an invite to it is.
class AreaOfferRefused implements Exception {
  const AreaOfferRefused();

  @override
  String toString() => 'AreaOfferRefused';
}

/// What an offer carries beside the Area's history, signed by the
/// sharer's Device: the Area, its keys, the key record of the Device
/// that signed ([from]) with its Profile, the signing keys the history
/// is checked with, and how changes travel.
class AreaOffer {
  final String area;
  final List<AreaKey> areaKeys;
  final KeyRecord from;
  final List<KeyRecord> keys;
  final SignedProfile profile;
  final Transport transport;

  const AreaOffer({
    required this.area,
    required this.areaKeys,
    required this.from,
    required this.keys,
    required this.profile,
    required this.transport,
  });

  /// What is signed: keys sorted, no spaces, as both cores write it.
  String encode() => jsonEncode({
    'area': area,
    'areaKeys': [for (final k in areaKeys) k.toJson()],
    'from': from.toJson(),
    'keys': [for (final k in keys) k.toJson()],
    'profile': profile.toJson(),
    'transport': transport.name,
  });

  /// The offer [text] holds; null when it is none: an Area key of
  /// another scope, or none at all, makes none either.
  static AreaOffer? parse(String text) {
    try {
      final json = jsonDecode(text);
      if (json
          case {
            'area': final String area,
            'areaKeys': final List areaKeys,
            'from': final Object from,
            'keys': final List keys,
            'profile': final Object profile,
            'transport': final String transport,
          }
          when area.startsWith('area:')) {
        final record = KeyRecord.fromJson(from);
        final signed = SignedProfile.fromJson(profile);
        final how = Transport.values.asNameMap()[transport];
        final scoped = areaKeysOf(areaKeys);
        if (record == null ||
            signed == null ||
            how == null ||
            scoped.isEmpty ||
            scoped.length != areaKeys.length ||
            scoped.any((k) => k.scope != area)) {
          return null;
        }
        return AreaOffer(
          area: area,
          areaKeys: scoped,
          from: record,
          keys: [for (final k in keys) ?KeyRecord.fromJson(k)],
          profile: signed,
          transport: how,
        );
      }
    } on FormatException {
      return null;
    }
    return null;
  }
}

/// The offer the Shared Area [areaId] goes out as now, from this Device.
/// It carries the Area's keys, the key records of this Device and of
/// every Device that wrote in the Area or that its grants name, held
/// here, and the own Profile as it goes to other people; how changes
/// travel is what the user chose for [person], bundles if nothing.
AreaOffer areaOfferFor(Store store, String areaId, String person) {
  final profile = SignedProfile.fromJson(store.ownProfileForOthers);
  if (profile == null) throw StateError('no own Profile to offer with');
  store.ensureAreaKeys();
  final self = store.deviceId;
  final devices = <String>{
    for (final e in store.areaHistory(areaId)) e.device,
    ...?store.grantsOf(areaId)?.devices,
  };
  final keys = [
    store.ownKeyRecord,
    for (final d in devices)
      if (d != self) ?store.pinnedKey(d)?.record,
  ]..sort((a, b) => a.device.compareTo(b.device));
  return AreaOffer(
    area: areaId,
    areaKeys: [
      for (final k in store.areaKeys())
        if (k.scope == areaId) k,
    ]..sort((a, b) => a.id.compareTo(b.id)),
    from: store.ownKeyRecord,
    keys: keys,
    profile: profile,
    transport: store.transportWith(person) ?? Transport.bundle,
  );
}

/// The file [offer] goes out as, signed by [key] and sealed to each of
/// the X25519 keys [to], as a Sent Item is sealed (ADR 0015) but bound
/// to an offer. The content key, the nonces and the ephemeral keys come
/// from [random] in this order: content key, nonce of the content, then
/// per recipient its ephemeral secret and the nonce of its key; the
/// fixture generator hands in a counter.
Uint8List encodeAreaOffer(
  AreaOffer offer,
  SigningKey key, {
  required List<List<int>> to,
  Uint8List Function(int length)? random,
}) {
  if (to.isEmpty) throw ArgumentError.value(to, 'to', 'nobody to seal for');
  final bytesOf = random ?? secretBytes;
  final payload = offer.encode();
  final signed = utf8.encode(
    jsonEncode({'offer': payload, 'sig': key.sign(_signedBytes(payload))}),
  );
  final content = bytesOf(areaKeyBytes);
  final sealed = seal(
    content,
    signed,
    aad: OfferAs.sealed,
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
          aad: OfferAs.key,
          nonce: bytesOf(sealNonceBytes),
        ),
      ),
    });
  }
  return utf8.encode(
    jsonEncode({'sealed': base64.encode(sealed), 'to': stanzas}),
  );
}

/// The Devices of [person] an offer can be sealed for: pinned here,
/// bound to their Profile, with an X25519 key, never one of the user's
/// own; sorted.
List<String> offerDevices(Store store, String person) => [
  for (final d in store.pinnedDevicesOf(store.resolvePerson(person)))
    if (!store.writtenByOwn(d) && store.pinnedKey(d)?.record.boxKey != null) d,
];

/// "Share with…" (#108): shares the Area [areaId] as [SharedAreas.share]
/// does, grants [person] [role] on every Device of theirs an offer can be
/// sealed for ([offerDevices]), and keeps [transport] as how changes
/// travel with them; the history counts as sent to them (ADR 0024).
/// Returns the id the Area goes on under; the offer itself is written by
/// `writeAreaOffer`. Throws a [StateError], changing nothing, for the own
/// Profile, a blocked person, or one none of whose Devices' keys is known
/// here.
String shareAreaWith(
  Store store,
  String areaId,
  String person, {
  required AreaRole role,
  required Transport transport,
}) {
  final id = store.resolvePerson(person);
  if (store.isOwnPerson(id) || store.isBlocked(id)) {
    throw StateError('nobody to share with: $person');
  }
  final devices = offerDevices(store, id);
  if (devices.isEmpty) throw StateError('no key of $person to offer to');
  final area = store.share(areaId);
  store.batch(() {
    for (final device in devices) {
      store.invite(area, device, role: role, profile: id);
    }
  });
  store.setTransportWith(id, transport);
  // The offer carries the Area's history: none of it waits for a bundle.
  store.noteAreaSent(id, area, store.currentSeq());
  return area;
}

/// The X25519 keys of the Devices of [person] that the grants of
/// [areaId] give a role now, by device: those an offer is sealed for.
List<Uint8List> offerKeys(Store store, String areaId, String person) {
  final grants = store.grantsOf(areaId);
  if (grants == null) return const [];
  final target = store.resolvePerson(person);
  return [
    for (final d in grants.devices)
      if (grants.grantOf(d) case final grant?
          when store.resolvePerson(grant.profile) == target &&
              !store.writtenByOwn(d))
        ?store.pinnedKey(d)?.record.boxKey,
  ];
}

/// Changes travel with each person as the user chose when sharing with
/// them (#108): a local setting of this Device by Profile id (ADR 0008).
extension Transports on Store {
  /// The local setting's key before the Profile id.
  static const prefix = 'transport:';

  /// How changes travel with [person], or null when the user was never
  /// asked.
  Transport? transportWith(String person) => Transport.values
      .asNameMap()[localSetting('$prefix${resolvePerson(person)}')];

  void setTransportWith(String person, Transport transport) =>
      setLocalSetting('$prefix${resolvePerson(person)}', transport.name);
}

// ------------------------------------------------------------ receiving

/// An offered Area held here until the user accepts or declines it: who
/// offers it, the Area as the offer has it, and the role this Device is
/// given there. Nothing of it shows anywhere else.
class HeldAreaOffer {
  /// The pending set it is held in.
  final String id;
  final String area;

  /// The Area's name as stored, and its Starter Set slug, if it has one.
  final String areaName;
  final String? areaStarter;
  final AreaRole role;

  /// The sharer's Profile as the offer carries it, the name sanitised and
  /// the picture made again, and its short code.
  final Profile sender;
  final String code;
  final SenderTrust trust;
  final Transport transport;

  /// When it came here.
  final DateTime held;

  const HeldAreaOffer({
    required this.id,
    required this.area,
    required this.areaName,
    required this.areaStarter,
    required this.role,
    required this.sender,
    required this.code,
    required this.trust,
    required this.transport,
    required this.held,
  });
}

/// What a held set keeps for Accept beside the entries: the offer as it
/// came, the Profiles of the Area's people, and the photos and
/// recordings kept for it.
class _Kept {
  final AreaOffer offer;
  final List<Object?> profiles;
  final List<String> blobs;

  const _Kept(this.offer, this.profiles, this.blobs);

  String encode() => jsonEncode({
    'blobs': blobs,
    'offer': offer.encode(),
    'profiles': profiles,
  });

  static _Kept? parse(String? text) {
    if (text == null) return null;
    try {
      if (jsonDecode(text) case {
        'blobs': final List blobs,
        'offer': final String offer,
        'profiles': final List profiles,
      }) {
        final read = AreaOffer.parse(offer);
        if (read == null) return null;
        return _Kept(read, profiles, [
          for (final b in blobs)
            if (b is String) b,
        ]);
      }
    } on FormatException {
      return null;
    }
    return null;
  }
}

/// Takes in the offer [bytes] that came inside the bundle [folder], as
/// `importBundle` hands it over: opens and checks it, reads the Area's
/// history and Profiles from the bundle with the offered key, and holds
/// them as a pending set never shown (ADR 0022), the photos and
/// recordings among the blobs this Device lacked. Returns the offer as
/// held; the one held already when the same offer comes again; null
/// when this Device holds a role in the Area already, so the bundle is
/// read as any other. Throws [AreaOfferNotForUs] for an offer sealed for
/// other Devices or granting this one nothing, [AreaOfferFromBlocked]
/// for a blocked person's, before anything is read, [AreaOfferRefused]
/// for an Area this Device holds as its own and [NotAnAreaOffer] for
/// anything else that is wrong.
Future<HeldAreaOffer?> receiveAreaOffer(
  Store store,
  SyncFolder folder,
  List<int> bytes,
) async {
  final (offer, sender) = _read(store, bytes);
  final area = offer.area;
  final self = store.deviceId;
  if (store.grantsOf(area)?.grantOf(self) != null &&
      offer.areaKeys.any((k) => store.areaKey(k.id) != null)) {
    return null;
  }
  if (store.refusesInvite(area)) throw const AreaOfferRefused();
  for (final set in _heldSets(store)) {
    final kept = _Kept.parse(set.payload);
    if (kept != null &&
        kept.offer.area == area &&
        kept.offer.from.device == offer.from.device) {
      return _heldOf(store, set) ?? (throw const NotAnAreaOffer());
    }
  }
  final List<Entry>? read;
  try {
    read = await entriesSealedWith(store, folder, offer.areaKeys);
  } on FileSystemException {
    throw const NotAnAreaOffer();
  }
  if (read == null) throw const NotAnAreaOffer();
  final entries = _inArea(store, area, read);
  if (_judge(store, offer, entries) == null) throw const AreaOfferNotForUs();
  final profiles = await _profiles(store, folder, offer.areaKeys);
  final blobs = await _keepBlobs(store, folder, offer.areaKeys, entries);
  final id = store.openPending(
    areaOfferPendingKind,
    shown: false,
    payload: _Kept(offer, profiles, blobs).encode(),
  );
  store.receivePending(id, entries, keys: offer.keys);
  final set = store.pendingSet(id) ?? (throw StateError('no pending set $id'));
  // What the grants said holds only for entries whose signatures hold.
  final held = _offerOf(store, set, offer, sender);
  if (held == null) {
    declineAreaOffer(store, id);
    throw const NotAnAreaOffer();
  }
  return held;
}

/// The offered Areas held here, the oldest first, those a restart left
/// too; one that no longer reads, as from a person blocked since, is
/// left out.
List<HeldAreaOffer> heldAreaOffers(Store store) => [
  for (final set in _heldSets(store)) ?_heldOf(store, set),
];

/// The offered Area held as the pending set [id]; null when it is gone,
/// accepted or declined, or no longer reads.
HeldAreaOffer? heldAreaOffer(Store store, String id) {
  final set = _heldSet(store, id);
  return set == null ? null : _heldOf(store, set);
}

/// Accepts the offered Area held as [id]: the Area's key is kept, then
/// its history taken in as any import is ([PendingChanges.acceptPending]),
/// its keys pinned, with a Moment before it; then the Profiles its
/// grants name and how changes travel with the sharer are kept, and the
/// history counts as sent to the sharer (ADR 0024). With [verified] the
/// sharer's Profile is marked verified, as for a Sent Item (#51). Returns
/// the Area's id. Throws a [StateError] when the set is gone,
/// [AreaOfferFromBlocked] for one from a person blocked since, which is
/// declined, and [AreaOfferRefused] likewise for an Area this Device made
/// its own meanwhile.
String acceptAreaOffer(Store store, String id, {bool verified = false}) {
  final set = _heldSet(store, id) ?? (throw StateError('no offer $id'));
  final kept = _Kept.parse(set.payload);
  if (kept == null) throw StateError('no offer $id');
  final offer = kept.offer;
  final sender = store.sentItemProfile(
    offer.profile.toJson(),
    offer.from.publicKey,
  );
  if (sender == null) throw StateError('no offer $id');
  if (_fromBlocked(store, offer, sender.profile.id)) {
    declineAreaOffer(store, id);
    throw const AreaOfferFromBlocked();
  }
  if (store.refusesInvite(offer.area)) {
    declineAreaOffer(store, id);
    throw const AreaOfferRefused();
  }
  final before = store.currentSeq();
  // The key first: a history held without it could never be read on.
  store.learnAreaKeys([
    for (final k in offer.areaKeys)
      if (k.scope == offer.area) k,
  ]);
  final accepted = store.acceptPending(id);
  store.takePeople([
    ...kept.profiles,
    offer.profile.toJson(),
  ], allowed: store.peopleOf(offer.area));
  if (verified) store.verifyPerson(sender.profile.id, sender.code);
  store.setTransportWith(sender.profile.id, offer.transport);
  // The sharer holds the history taken in: none of it waits for a bundle.
  store.noteAreaSent(sender.profile.id, offer.area, store.currentSeq());
  store.momentFor(
    before: before,
    changed: accepted.taken.isNotEmpty,
    cause: MomentCause.bundle,
    label: sender.profile.name.isEmpty ? null : sender.profile.name,
  );
  return offer.area;
}

/// Declines the offered Area held as [id]: the set is deleted, and the
/// photos and recordings kept for it that the log does not refer to go
/// with it. Nothing of the Area stays, no key and no Profile either.
/// Returns whether there was one.
bool declineAreaOffer(Store store, String id) {
  final set = _heldSet(store, id);
  if (set == null) return false;
  final kept = _Kept.parse(set.payload);
  store.declinePending(id);
  for (final ref in kept?.blobs ?? const <String>[]) {
    if (store.knowsBlob(ref)) continue;
    final (kind, hash) = splitBlobRef(ref);
    store.dropBlob(kind, hash);
  }
  return true;
}

/// "Decline and block" on the offered Area held as [id] (#90): declines
/// it and blocks its sharer as a Sent Item's sender is blocked, keeping
/// their Profile without its picture so the "Blocked" list names them.
/// With [leaveAreas] as [Blocks.blockPerson] has it, and a [StateError]
/// before anything changes when blocking leaves Areas not to be left.
/// Nobody is blocked for the own Profile's id, or for an id kept here
/// under another first key. Returns whether anybody was blocked; the
/// offer is declined either way.
bool blockAreaOfferSender(Store store, String id, {bool leaveAreas = false}) {
  final set = _heldSet(store, id);
  if (set == null) return false;
  final offer = _Kept.parse(set.payload)?.offer;
  final sender = offer == null
      ? null
      : store.sentItemProfile(offer.profile.toJson(), offer.from.publicKey);
  var blocked = false;
  if (offer != null && sender != null) {
    final person = sender.profile.id;
    final kept = store.personCode(person);
    if (!store.isOwnPerson(person) && (kept == null || kept == sender.code)) {
      if (!leaveAreas && store.areasToLeaveForBlocking(person).isNotEmpty) {
        throw StateError('blocking $person leaves the Areas they manage');
      }
      store.takePeople([offer.profile.toJson()]);
      store.blockPerson(person, leaveAreas: leaveAreas);
      blocked = true;
    }
  }
  declineAreaOffer(store, id);
  return blocked;
}

/// Declines every offered Area held here from a person the user blocked
/// (ADR 0021): their Profile's id is blocked, or one of their Devices
/// signed it. Returns whether any went.
bool dropOffersFromBlocked(Store store) {
  if (store.blockedIds.isEmpty) return false;
  var dropped = false;
  for (final set in _heldSets(store)) {
    final kept = _Kept.parse(set.payload);
    if (kept == null) continue;
    final id = Profile.parse(kept.offer.profile.record)?.id;
    if (_fromBlocked(store, kept.offer, id) &&
        declineAreaOffer(store, set.id)) {
      dropped = true;
    }
  }
  return dropped;
}

/// The offer in [bytes], opened with this Device's X25519 key and
/// checked: its signature under the key record it names, and the
/// Profile it carries bound to that key, as a Sent Item's (ADR 0015).
(AreaOffer, SentProfile) _read(Store store, List<int> bytes) {
  final outer = _json(bytes);
  if (outer case {'sealed': final String sealed, 'to': final List<Object?> to}
      when _inFormat(outer)) {
    final inner = _open(store, sealed, to);
    if (inner == null) throw const AreaOfferNotForUs();
    final signed = _json(inner);
    if (signed case {'offer': final String payload, 'sig': final String sig}
        when _inFormat(signed)) {
      final offer = AreaOffer.parse(payload);
      if (offer == null ||
          !offer.from.valid ||
          !verifySignature(offer.from.publicKey, _signedBytes(payload), sig)) {
        throw const NotAnAreaOffer();
      }
      final sender = store.sentItemProfile(
        offer.profile.toJson(),
        offer.from.publicKey,
      );
      if (sender == null) throw const NotAnAreaOffer();
      if (_fromBlocked(store, offer, sender.profile.id)) {
        throw const AreaOfferFromBlocked();
      }
      return (offer, sender);
    }
  }
  throw const NotAnAreaOffer();
}

/// Whether [offer] comes from a person the user blocked: the Profile id
/// [profile] it carries is, or a blocked person's Device signed it.
bool _fromBlocked(Store store, AreaOffer offer, String? profile) {
  if (profile != null && store.isBlocked(profile)) return true;
  return store.blockedDevices().contains(offer.from.device);
}

/// The entries of [entries] whose entity lies in [area] by what the
/// entries themselves say: the Area, its Topics, the Notes filed into
/// either and their Sections and Routines. Whatever this Device holds
/// already in another scope stays out, so an offer never moves the
/// user's own Notes or places into somebody else's Area.
List<Entry> _inArea(Store store, String area, List<Entry> entries) {
  final topics = <String>{
    for (final e in entries)
      if (e.entity.startsWith('topic:') &&
          e.field == PlaceKeys.area &&
          e.value == area)
        e.entity,
  };
  final notes = <String>{
    for (final e in entries)
      if (e.entity.startsWith('note:') &&
          e.field == NoteKeys.place &&
          (e.value == area || topics.contains(e.value)))
        e.entity,
  };
  final inside = <String>{
    area,
    ...topics,
    ...notes,
    for (final e in entries)
      if ((e.entity.startsWith('section:') && e.field == SectionKeys.note ||
              e.entity.startsWith('routine:') && e.field == RoutineKeys.note ||
              e.entity.startsWith(FrogKeys.prefix) &&
                  e.field == FrogKeys.note ||
              e.entity.startsWith(TallyKeys.entryPrefix) &&
                  e.field == TallyKeys.note) &&
          notes.contains(e.value))
        e.entity,
  };
  final foreign = <String, bool>{};
  return [
    for (final e in entries)
      if (inside.contains(e.entity) &&
          !foreign.putIfAbsent(
            e.entity,
            () =>
                store.loggedHistory(e.entity).isNotEmpty &&
                store.scopeOf(e.entity) != area,
          ))
        e,
  ];
}

/// The role [entries], judged with the log, give this Device in the
/// offered Area, under the user's own Profile, when the Device that
/// signed the offer is a Manager there under the Profile it carries,
/// with what the roles leave out; null otherwise.
({AreaRole role, Set<String> ignored})? _judge(
  Store store,
  AreaOffer offer,
  List<Entry> entries,
) {
  final judged = judgeLog([
    for (final e in store.allEntries())
      if (e.applies) e,
    for (final e in entries)
      if (e.applies) e,
  ]);
  final grants = judged.areas[offer.area];
  final mine = grants?.grantOf(store.deviceId);
  final theirs = grants?.grantOf(offer.from.device);
  final sharer = Profile.parse(offer.profile.record)?.id;
  if (mine == null || !store.isOwnPerson(mine.profile)) return null;
  if (theirs == null ||
      theirs.role != AreaRole.manager ||
      sharer == null ||
      store.resolvePerson(theirs.profile) != store.resolvePerson(sharer)) {
    return null;
  }
  return (role: mine.role, ignored: judged.ignored);
}

/// The Profiles the bundle [folder] holds sealed with one of [keys], in
/// the order they lie there.
Future<List<Object?>> _profiles(
  Store store,
  SyncFolder folder,
  List<AreaKey> keys,
) async {
  final self = store.deviceId;
  final out = <Object?>[];
  final byId = {for (final k in keys) k.id: k};
  for (final name in await folder.list('keys')) {
    if (!name.endsWith('.profiles')) continue;
    final parts = name.split('.');
    if (parts.length != 3 || parts[0].isEmpty || parts[0] == self) continue;
    final key = byId[parts[1]];
    if (key == null) continue;
    final bytes = await folder.read('keys', name);
    final plain = bytes == null
        ? null
        : unseal(key.key, bytes, aad: SealedAs.profiles(parts[0]));
    if (plain == null) continue;
    try {
      if (jsonDecode(utf8.decode(plain)) case {'profiles': final List list}) {
        out.addAll(list);
      }
    } on FormatException {
      continue;
    }
  }
  return out;
}

/// Keeps the photos and recordings [entries] name that this Device lacks
/// and the bundle [folder] holds sealed with one of [keys]; returns
/// those kept, sorted, so a Decline lets them go again.
Future<List<String>> _keepBlobs(
  Store store,
  SyncFolder folder,
  List<AreaKey> keys,
  List<Entry> entries,
) async {
  final refs = <String>{
    for (final e in entries)
      for (final (kind, prefix) in Sync.blobKinds)
        if (e.field.startsWith(prefix))
          '$kind/${e.field.substring(prefix.length)}',
  }.toList()..sort();
  final names = (await folder.list('blobs')).toSet();
  final kept = <String>[];
  for (final ref in refs) {
    final (kind, hash) = splitBlobRef(ref);
    if (store.blob(kind, hash) != null) continue;
    for (final key in keys) {
      final name = blobName(key.key, ref);
      if (!names.contains(name)) continue;
      final bytes = await folder.read('blobs', name);
      final plain = bytes == null
          ? null
          : unseal(key.key, bytes, aad: SealedAs.blob(ref));
      // Bytes that are not the blob they are named for stay out.
      if (plain != null && store.putBlob(kind, hash, plain)) {
        kept.add(ref);
        break;
      }
    }
  }
  return kept;
}

Iterable<PendingSet> _heldSets(Store store) =>
    store.pendingSets.where((s) => s.kind == areaOfferPendingKind);

PendingSet? _heldSet(Store store, String id) => switch (store.pendingSet(id)) {
  final set? when set.kind == areaOfferPendingKind => set,
  _ => null,
};

/// The held [set] as an offer, checked again against what this Device
/// knows now; null when it no longer reads.
HeldAreaOffer? _heldOf(Store store, PendingSet set) {
  final kept = _Kept.parse(set.payload);
  if (kept == null) return null;
  final offer = kept.offer;
  final sender = store.sentItemProfile(
    offer.profile.toJson(),
    offer.from.publicKey,
  );
  if (sender == null || _fromBlocked(store, offer, sender.profile.id)) {
    return null;
  }
  return _offerOf(store, set, offer, sender);
}

HeldAreaOffer? _offerOf(
  Store store,
  PendingSet set,
  AreaOffer offer,
  SentProfile sender,
) {
  final entries = [for (final r in set.received) r.entry];
  final judged = _judge(store, offer, entries);
  if (judged == null) return null;
  // The name and slug as the roles have them: a Member's rename of the
  // Area is relayed, but counts nowhere.
  String? latest(String field) {
    Entry? held;
    for (final e in entries) {
      if (e.entity != offer.area || e.field != field || !e.applies) continue;
      if (judged.ignored.contains(e.id)) continue;
      if (held == null || Store.latestWins(e, held) > 0) held = e;
    }
    return held?.value;
  }

  return HeldAreaOffer(
    id: set.id,
    area: offer.area,
    areaName: latest(PlaceKeys.name) ?? '',
    areaStarter: latest(PlaceKeys.starter),
    role: judged.role,
    sender: sender.profile,
    code: sender.code,
    trust: sender.verified
        ? SenderTrust.verified
        : sender.kept
        ? SenderTrust.known
        : SenderTrust.unknown,
    transport: offer.transport,
    held: set.made,
  );
}

/// Whether [json] is in the format this version reads: `format`, 1 when
/// missing, and 1 is never written (ADR 0019).
bool _inFormat(Object? json) =>
    json is Map && (json.containsKey('format') ? json['format'] : 1) == 1;

Object? _json(List<int> bytes) {
  try {
    return jsonDecode(utf8.decode(bytes));
  } on FormatException {
    throw const NotAnAreaOffer();
  }
}

/// What a signature over [payload] covers.
Uint8List _signedBytes(String payload) =>
    utf8.encode('${OfferAs.signed}$payload');

/// The key the content key is sealed with for one recipient: HKDF over
/// the X25519 secret, bound to both public keys and to an offer.
Uint8List _wrapKey(List<int> shared, List<int> ephemeral, List<int> to) =>
    hkdfSha256(
      shared,
      salt: [...ephemeral, ...to],
      info: utf8.encode(OfferAs.key),
    );

/// The signed offer inside [sealed], opened with the first of [to]
/// sealed for this Device's X25519 key; null when none is.
Uint8List? _open(Store store, String sealed, List<Object?> to) {
  final Uint8List body;
  try {
    body = base64.decode(sealed);
  } on FormatException {
    throw const NotAnAreaOffer();
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
          aad: OfferAs.key,
        );
        if (content == null || content.length != areaKeyBytes) continue;
        return unseal(content, body, aad: OfferAs.sealed);
      } on FormatException {
        continue;
      }
    }
  }
  return null;
}
