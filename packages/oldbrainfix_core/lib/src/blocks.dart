part of 'profiles.dart';

/// Blocking a person (ADR 0021, #90): once the user blocks someone,
/// nothing that person wrote reaches the user's screen or storage,
/// whichever Device relays it, but their check-offs. The block is the
/// entity `block:<profile id>` in the log, outside every Area like an
/// Override, so it is sealed under the personal key and goes to the
/// user's own Devices only, never to the person, a Shared Area or a Sent
/// Item; only what one's own Devices wrote counts. The person's Devices
/// are those the grants of the Shared Areas name them for, never one of
/// the user's own.
abstract final class BlockKeys {
  /// The entity id's prefix, before the Profile id.
  static const prefix = 'block:';

  /// The name the person showed under when they were blocked, so every
  /// one of the user's Devices lists them by it; null once unblocked.
  static const blocked = 'blocked';
}

/// The most code points the author name of an entry kept from a
/// blocked person may have; the apps write none.
const blockedAuthorMax = 40;

final _blockedId = RegExp(r'^[0-9a-z-]{1,64}$');
final _blockedPlace = RegExp(r'^(area|topic):[0-9a-z-]{1,64}$');
final _blockedInstant = RegExp(
  r'^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d{1,6})?Z$',
);

/// Whether [e], written by a blocked person's Device, is kept (ADR 0021):
/// only what carries no text, picture or sound and keeps the Shared Areas
/// whole. That is a check-off or its undoing (`done@…` on a Routine), a
/// grant (`role:…` on an Area), and a Note's `$type` and filing, which
/// say where the Routines and Sections others hung on it lie; the Note
/// itself shows nowhere without its capture. Each in its exact shape, so
/// nothing else can hide in it: version 1 with no causal context, ids
/// of letters, digits and dashes, values as the apps write them, and an
/// author of at most [blockedAuthorMax] code points.
bool keptFromBlocked(Entry e) {
  if (e.extra.isNotEmpty || e.ctx != null) return false;
  if (e.author.runes.length > blockedAuthorMax) return false;
  final colon = e.entity.indexOf(':');
  if (colon < 0 || !_blockedId.hasMatch(e.entity.substring(colon + 1))) {
    return false;
  }
  final kind = e.entity.substring(0, colon);
  final value = e.value;
  if (kind == 'routine' && e.field.startsWith(RoutineKeys.donePrefix)) {
    final key = e.field.substring(RoutineKeys.donePrefix.length);
    return Occurrence.tryParse(key)?.key == key &&
        (value == null || _blockedInstant.hasMatch(value));
  }
  if (kind == 'area' && e.field.startsWith(ShareKeys.rolePrefix)) {
    final device = e.field.substring(ShareKeys.rolePrefix.length);
    if (!_blockedId.hasMatch(device)) return false;
    if (value == null) return true;
    final grant = Grant.parse(value);
    return grant != null &&
        grant.encode() == value &&
        _blockedId.hasMatch(grant.profile);
  }
  if (kind == 'note' && e.field == NoteKeys.type) {
    return value == null || value == NoteKeys.typeNote;
  }
  if (kind == 'note' && e.field == NoteKeys.place) {
    return value == null || _blockedPlace.hasMatch(value);
  }
  return false;
}

extension Blocks on Store {
  /// The Profile ids blocked now, through their aliases: those whose
  /// block one of the user's own Devices wrote last.
  Set<String> get blockedIds => {
    for (final (id, _) in _blocks()) resolvePerson(id),
  };

  /// Every block that holds now, as (Profile id as written, name then).
  List<(String, String)> _blocks() {
    final own = <String, bool>{};
    return [
      for (final entity in entitiesWithPrefix(BlockKeys.prefix))
        if (currentByOwn(entity, BlockKeys.blocked, own) case final name?)
          (entity.substring(BlockKeys.prefix.length), name),
    ];
  }

  /// Whether the person [id] is blocked; the own Profile never is.
  bool isBlocked(String id) =>
      !isOwnPerson(id) && blockedIds.contains(resolvePerson(id));

  /// The name the person [id] showed under when they were blocked, for
  /// a Device that does not keep their Profile; null when not blocked.
  String? _nameWhenBlocked(String id) {
    for (final (blocked, name) in _blocks()) {
      if (resolvePerson(blocked) == id) return sanitizeProfileName(name);
    }
    return null;
  }

  /// The people blocked now, for the folded "Blocked" list in People:
  /// as the user sees them, without their own picture (ADR 0021), by
  /// name.
  List<Profile> get blockedPeople {
    final shown = [for (final id in blockedIds) personShown(id)];
    shown.sort((a, b) {
      final byName = _compareUtf8(a.name, b.name);
      return byName != 0 ? byName : _compareUtf8(a.id, b.id);
    });
    return shown;
  }

  /// The Shared Areas the user has to leave to block the person [id]:
  /// those this Device holds a role in where a Device of theirs is a
  /// Manager now, sorted. Their Routines, Reminders and names there would
  /// otherwise go half missing.
  List<String> areasToLeaveForBlocking(String id) {
    final person = resolvePerson(id);
    final self = deviceId;
    return [
      for (final MapEntry(key: area, value: grants) in judged.areas.entries)
        if (grants.grantOf(self) != null &&
            grants.devices.any((device) {
              final grant = grants.grantOf(device);
              return grant != null &&
                  grant.role == AreaRole.manager &&
                  device != self &&
                  !isOwnPerson(grant.profile) &&
                  resolvePerson(grant.profile) == person &&
                  !writtenByOwn(device);
            }))
          area,
    ]..sort();
  }

  /// Blocks the person [id] on all of the user's Devices (#90): every
  /// entry of theirs held here goes but their check-offs
  /// ([enforceBlocks]), and nothing more of theirs is taken in. When they
  /// manage a Shared Area this Device holds a role in
  /// ([areasToLeaveForBlocking]), blocking leaves it, and only with
  /// [leaveAreas]: otherwise a [StateError], and nothing changes. Throws
  /// an [ArgumentError] for the own Profile. Returns the Areas left.
  List<String> blockPerson(String id, {bool leaveAreas = false}) {
    if (isOwnPerson(id)) {
      throw ArgumentError.value(id, 'id', 'is the own Profile');
    }
    final areas = areasToLeaveForBlocking(id);
    if (areas.isNotEmpty && !leaveAreas) {
      throw StateError('blocking $id leaves the Areas they manage');
    }
    // Left first: an Area left without the block is the safer half.
    for (final area in areas) {
      leaveArea(area);
    }
    final person = resolvePerson(id);
    if (!isBlocked(person)) {
      append(
        '${BlockKeys.prefix}$person',
        BlockKeys.blocked,
        personShown(person).name,
      );
    }
    enforceBlocks();
    profileChanged();
    return areas;
  }

  /// Unblocks the person [id]: what was refused of theirs is fetched
  /// again from the Devices that still hold it, and their Profile is
  /// taken in again with the next sync that carries it.
  void unblockPerson(String id) {
    final person = resolvePerson(id);
    final own = <String, bool>{};
    final entities = [
      for (final entity in entitiesWithPrefix(BlockKeys.prefix))
        if (resolvePerson(entity.substring(BlockKeys.prefix.length)) ==
                person &&
            currentByOwn(entity, BlockKeys.blocked, own) != null)
          entity,
    ];
    if (entities.isEmpty) return;
    batch(() {
      for (final entity in entities) {
        append(entity, BlockKeys.blocked, null);
      }
    });
    enforceBlocks();
    profileChanged();
  }

  /// The Devices of the people blocked now: those a grant in a Shared
  /// Area names one of them for (its last grant that named anyone),
  /// never this Device nor one of the user's own. A Profile's binding
  /// does not count, since any key bound to it can bind another.
  Set<String> blockedDevices() {
    final blocked = blockedIds;
    if (blocked.isEmpty) return const {};
    final self = deviceId;
    final out = <String>{};
    for (final grants in judged.areas.values) {
      for (final device in grants.devices) {
        if (device == self || out.contains(device)) continue;
        final person = grants.personOf(device);
        if (person == null || isOwnPerson(person)) continue;
        if (blocked.contains(resolvePerson(person)) && !writtenByOwn(device)) {
          out.add(device);
        }
      }
    }
    return out;
  }

  /// Makes the log what the blocks say (ADR 0021), after a block, an
  /// unblock and every sync that brings anything: every entry a blocked
  /// person's Device wrote that is not [keptFromBlocked] is deleted and
  /// its dseq noted as refused with the scope its entity lay in
  /// ([Store.noteRefused]), and the bytes of
  /// their photos and recordings nothing else refers to go; their
  /// Profile's picture is taken out. A Device no longer blocked has its
  /// refusals released, so what was refused is asked for again, the
  /// shared folder read whole again too. The shared folder itself is not
  /// touched. Returns whether anything was deleted or let go.
  bool enforceBlocks() {
    final blocked = blockedIds;
    final marks = refusalMarks();
    if (blocked.isEmpty && marks.isEmpty) return false;
    final devices = blockedDevices();
    final refs = <String>{};
    var changed = false;
    batch(() {
      for (final device in devices.toList()..sort()) {
        final refused = [
          for (final e in entriesOf(device))
            if (!keptFromBlocked(e)) e,
        ];
        // Where each lay is read before anything goes.
        final scopes = {for (final e in refused) e.entity: scopeOf(e.entity)};
        for (final e in refused) {
          for (final (kind, prefix) in Sync.blobKinds) {
            if (e.field.startsWith(prefix)) {
              refs.add('$kind/${e.field.substring(prefix.length)}');
            }
          }
          noteRefused(device, e.dseq, scopes[e.entity] ?? personalScope);
        }
        if (refused.isNotEmpty) {
          deleteEntries(device, [for (final e in refused) e.dseq]);
          changed = true;
        }
      }
      var released = false;
      for (final (device, _) in marks) {
        if (devices.contains(device)) {
          // Blocked again: the mark refuses again.
          holdRefusals(device);
        } else if (releaseRefusals(device)) {
          released = true;
        }
      }
      if (released) forgetFolderReads();
      dropFilledRefusals();
      for (final id in blocked) {
        if (_stripPicture(id)) changed = true;
      }
    });
    for (final ref in refs.toList()..sort()) {
      if (knowsBlob(ref)) continue;
      final (kind, hash) = splitBlobRef(ref);
      if (dropBlob(kind, hash)) changed = true;
    }
    return changed;
  }

  /// Takes the picture out of the Profile of the blocked person [id]
  /// kept here; returns whether there was one.
  bool _stripPicture(String id) {
    final held = _people.byId[id];
    if (held == null) return false;
    final record = Profile.parse(held.sent.record);
    if (record == null) return false;
    if (record.picture == null && held.profile.picture == null) return false;
    _keepPerson(
      SignedProfile(
        record: Profile(
          id: record.id,
          name: record.name,
          version: record.version,
        ).encode(),
        signer: held.sent.signer,
        sig: held.sent.sig,
        first: held.sent.first,
        keys: held.sent.keys,
        aliases: held.sent.aliases,
      ),
      null,
      held.seen,
      verified: held.verified,
      stripped: true,
    );
    return true;
  }
}
