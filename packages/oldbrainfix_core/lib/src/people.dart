part of 'profiles.dart';

/// Other people's Profiles (ADR 0017, #50): the Profiles of the people
/// this Device shares an Area with, as they arrived with the Area's sync
/// or a bundle. Each is kept as a local setting `person:<id>`, as
/// received with its signature, so it can be passed on unchanged:
/// `{"profile":…,"seen":…,"shown":…}`, the signed Profile, when it was
/// first seen here, and its picture made again for the screen. On first
/// sight an id is bound to the keys that came with it; later a record
/// counts only when one of them signed it. `"verified":true` says the
/// user compared its short code in person (#51); it stays on this
/// Device and is kept through every newer record.

/// What became of another person's Profile that came in.
enum PersonTaken {
  /// Nothing new.
  unchanged,

  /// Its signatures did not hold, it was signed by a key not bound to
  /// the Profile kept here, a copied id, or not wanted from that sender.
  refused,

  /// Seen for the first time and kept, bound to the keys it came with.
  added,

  /// A newer record, keys or aliases of a Profile kept here.
  updated,
}

/// A Profile of another person as this Device keeps it.
class _Held {
  final SignedProfile sent;

  /// The record read, with the picture made again.
  final Profile profile;
  final Set<String> bound;
  final String seen;

  /// Whether the user compared its short code in person.
  final bool verified;

  const _Held(this.sent, this.profile, this.bound, this.seen, this.verified);

  String get id => profile.id;
}

/// Every kept Profile by id, and the ids they replaced.
class _PeopleIndex {
  final Map<String, _Held> byId;
  final Map<String, String> aliasTo;

  const _PeopleIndex(this.byId, this.aliasTo);
}

/// Who wrote a Note: the user, another person, or nobody known.
sealed class _Writer {
  const _Writer();
}

class _Own extends _Writer {
  const _Own();
}

class _Theirs extends _Writer {
  final Profile author;

  const _Theirs(this.author);
}

class _Unknown extends _Writer {
  const _Unknown();
}

/// Which people's Notes the notes browser shows (#53): the user's own
/// with [me], and those of [people], by Profile id; everybody's when
/// neither is set.
class PersonFilter {
  final bool me;
  final Set<String> people;

  const PersonFilter({this.me = false, this.people = const {}});

  /// Every Note, whoever wrote it.
  static const everyone = PersonFilter();

  bool get isEveryone => !me && people.isEmpty;

  /// The same filter with the user's own Notes in, or out again.
  PersonFilter toggledMe() => PersonFilter(me: !me, people: people);

  /// The same filter with the person [id] in, or out again.
  PersonFilter toggled(String id) => PersonFilter(
    me: me,
    people: people.contains(id) ? ({...people}..remove(id)) : {...people, id},
  );

  @override
  bool operator ==(Object other) =>
      other is PersonFilter &&
      other.me == me &&
      other.people.length == people.length &&
      other.people.containsAll(people);

  @override
  int get hashCode => Object.hash(me, Object.hashAllUnordered(people));
}

/// The own Profile's ids and keys, with what they were read from.
typedef _OwnKeys = ({String stamp, Set<String> ids, Set<String> bound});

final _peopleIndex = Expando<_PeopleIndex>('people');
final _ownKeysIndex = Expando<_OwnKeys>('ownKeys');

extension People on Store {
  static const _prefix = 'person:';

  _PeopleIndex get _people => _peopleIndex[this] ??= _readPeople();

  _PeopleIndex _readPeople() {
    final byId = <String, _Held>{};
    for (final (key, raw) in localSettingsWithPrefix(_prefix)) {
      final held = _readHeld(raw);
      if (held != null && key == '$_prefix${held.id}') byId[held.id] = held;
    }
    // An id counts as replaced only when its own Profile is kept here
    // and a key bound to it made the alias: one Profile cannot claim to
    // replace somebody else's.
    final aliasTo = <String, String>{};
    for (final id in byId.keys.toList()..sort()) {
      for (final a in byId[id]!.sent.aliases) {
        if (a.alias != id && _replaced(byId, a)) {
          aliasTo.putIfAbsent(a.alias, () => id);
        }
      }
    }
    return _PeopleIndex(byId, aliasTo);
  }

  /// Whether [a] gives up a Profile kept in [byId], made by a key bound
  /// to it.
  static bool _replaced(Map<String, _Held> byId, ProfileAlias a) =>
      byId[a.alias]?.bound.contains(a.by) ?? false;

  static _Held? _readHeld(String raw) {
    try {
      final json = jsonDecode(raw);
      if (json case {'profile': final Object sent, 'seen': final String seen}) {
        final signed = SignedProfile.fromJson(sent);
        final profile = signed == null ? null : Profile.parse(signed.record);
        if (signed == null || profile == null) return null;
        final shown = json['shown'];
        return _Held(
          signed,
          Profile(
            id: profile.id,
            name: profile.name,
            picture: shown is String ? base64.decode(shown) : null,
            version: profile.version,
          ),
          _chain(profile.id, signed.first, signed.keys).$1,
          seen,
          json['verified'] == true,
        );
      }
    } on FormatException {
      return null;
    }
    return null;
  }

  /// The ids of the own Profile and the keys bound to it.
  _OwnKeys get _own {
    final record = localSetting(OwnProfile._record) ?? '';
    final first = localSetting(OwnProfile._first) ?? '';
    final keys = localSetting(OwnProfile._keys) ?? '';
    final aliases = localSetting(OwnProfile._aliases) ?? '';
    final stamp = '$first\n$keys\n$aliases\n$record';
    final held = _ownKeysIndex[this];
    if (held != null && held.stamp == stamp) return held;
    final profile = Profile.parse(record);
    final made = (
      stamp: stamp,
      ids: {?profile?.id, ...ownProfileAliases},
      bound: profile == null || !_isKey(first)
          ? <String>{}
          : _chain(profile.id, first, _heldKeys).$1,
    );
    _ownKeysIndex[this] = made;
    return made;
  }

  /// Whether the Profile id [id] is the own Profile's, or one it
  /// replaced.
  bool isOwnPerson(String id) => _own.ids.contains(id);

  /// The id [id] stands for now: the Profile kept here that replaced it,
  /// through every alias, else [id] itself.
  String resolvePerson(String id) {
    final aliasTo = _people.aliasTo;
    var at = id;
    for (var i = 0; i <= aliasTo.length; i++) {
      final next = aliasTo[at];
      if (next == null) break;
      at = next;
    }
    return at;
  }

  /// The person [id] as every screen shows them: the own Profile for
  /// one's own ids, a Profile kept here through its aliases with its
  /// picture made again, else the placeholder with no name, until their
  /// Profile arrives; with the name and picture of the user's Override,
  /// where they gave one (#52).
  Profile personShown(String id) {
    final theirs = personTheirs(id);
    return isOwnPerson(id) ? theirs : _withOverride(theirs);
  }

  /// The person [id] with their own name and picture, as [personShown]
  /// but without the user's Override: what their Profile view shows them
  /// as.
  Profile personTheirs(String id) {
    if (isOwnPerson(id)) {
      if (ownProfile case final own?) return own;
    }
    final resolved = resolvePerson(id);
    return _people.byId[resolved]?.profile ??
        Profile(id: resolved, name: '', version: 1);
  }

  /// The short code of the person [id]'s Profile, when it is kept here
  /// or is the own one.
  String? personCode(String id) {
    if (isOwnPerson(id)) return ownProfileCode;
    final held = _people.byId[resolvePerson(id)];
    return held == null
        ? null
        : profileShortCode(held.id, base64.decode(held.sent.first));
  }

  /// Every Profile of another person kept here, by id.
  List<Profile> get keptPeople {
    final ids = _people.byId.keys.toList()..sort();
    return [for (final id in ids) _people.byId[id]!.profile];
  }

  /// The Profile [json] that the Device [device] sent of its person, to
  /// show before an invite: its signatures hold, its binding holds the
  /// key [device] is named for, and its picture is made again. Null when
  /// any of it fails.
  ProfileChoice? sentProfileOf(Object? json, {required String device}) {
    final sent = SignedProfile.fromJson(json);
    final checked = sent == null ? null : _check(sent);
    if (sent == null || checked == null) return null;
    final p = checked.profile;
    final bound = _chain(p.id, sent.first, sent.keys).$1;
    if (!bound.any((k) => deviceIdFromKey(base64.decode(k)) == device)) {
      return null;
    }
    return (
      profile: Profile(
        id: p.id,
        name: p.name,
        picture: checked.shown,
        version: p.version,
      ),
      code: profileShortCode(p.id, base64.decode(sent.first)),
    );
  }

  /// Takes in every Profile of [list] (signed Profiles as JSON) that
  /// [allowed] lets in by id, or by the id of a Profile kept here that it
  /// replaced; all of them without [allowed]. Returns whether any was added or changed.
  bool takePeople(Object? list, {bool Function(String id)? allowed}) {
    var changed = false;
    for (final json in list is List ? list : const []) {
      final taken = takePerson(json, allowed: allowed);
      if (taken == PersonTaken.added || taken == PersonTaken.updated) {
        changed = true;
      }
    }
    if (changed) profileChanged();
    return changed;
  }

  /// Takes in another person's Profile [json]: kept on first sight,
  /// bound to the keys it came with; later a newer record replaces the
  /// one kept only when a key bound to it signed it, keys join only when
  /// a bound key signed them, and aliases only when a bound key made
  /// them. Its name is sanitised and its picture made again, or it is
  /// refused. The own Profile is left to [OwnProfile.takeOwnProfile].
  PersonTaken takePerson(Object? json, {bool Function(String id)? allowed}) {
    final theirs = SignedProfile.fromJson(json);
    final checked = theirs == null ? null : _check(theirs);
    if (theirs == null || checked == null) return PersonTaken.refused;
    final id = checked.profile.id;
    if (isOwnPerson(id)) return PersonTaken.unchanged;
    if (allowed != null &&
        !allowed(id) &&
        !theirs.aliases.any(
          (a) => allowed(a.alias) && _replaced(_people.byId, a) && a.signed,
        )) {
      return PersonTaken.refused;
    }
    final held = _people.byId[id];
    if (held == null) {
      final bound = _chain(id, theirs.first, theirs.keys).$1;
      _keepPerson(
        SignedProfile(
          record: theirs.record,
          signer: theirs.signer,
          sig: theirs.sig,
          first: theirs.first,
          keys: checked.keys,
          aliases: _sortedAliases([
            for (final a in theirs.aliases)
              if (a.alias != id && bound.contains(a.by) && a.signed) a,
          ]),
        ),
        checked.shown,
        clock().toUtc().toIso8601String(),
      );
      return PersonTaken.added;
    }
    // A copied id with another first key is somebody else.
    if (theirs.first != held.sent.first) return PersonTaken.refused;
    final (bound, keys) = _chain(
      id,
      held.sent.first,
      theirs.keys,
      held: held.sent.keys,
    );
    if (!bound.contains(theirs.signer)) return PersonTaken.refused;
    final aliases = {for (final a in held.sent.aliases) a.alias: a};
    for (final a in theirs.aliases) {
      if (a.alias == id || aliases.containsKey(a.alias)) continue;
      if (bound.contains(a.by) && a.signed) aliases[a.alias] = a;
    }
    final kept = Profile.parse(held.sent.record);
    final newer =
        kept == null ||
        _replaces(theirs.record, checked.profile, held.sent.record, kept);
    final next = SignedProfile(
      record: newer ? theirs.record : held.sent.record,
      signer: newer ? theirs.signer : held.sent.signer,
      sig: newer ? theirs.sig : held.sent.sig,
      first: held.sent.first,
      keys: keys,
      aliases: _sortedAliases(aliases.values),
    );
    if (jsonEncode(next.toJson()) == jsonEncode(held.sent.toJson())) {
      return PersonTaken.unchanged;
    }
    _keepPerson(
      next,
      newer ? checked.shown : held.profile.picture,
      held.seen,
      verified: held.verified,
    );
    return PersonTaken.updated;
  }

  static List<ProfileAlias> _sortedAliases(Iterable<ProfileAlias> aliases) =>
      aliases.toList()..sort((a, b) => _compareUtf8(a.alias, b.alias));

  void _keepPerson(
    SignedProfile sent,
    Uint8List? shown,
    String seen, {
    bool verified = false,
  }) {
    final id = Profile.parse(sent.record)?.id;
    if (id == null) return;
    setLocalSetting(
      '$_prefix$id',
      jsonEncode({
        'profile': sent.toJson(),
        'seen': seen,
        if (shown != null) 'shown': base64.encode(shown),
        if (verified) 'verified': true,
      }),
    );
    _peopleIndex[this] = null;
  }

  /// Whether the user compared the short code of the person [id]'s
  /// Profile in person; the own Profile counts as verified.
  bool personVerified(String id) =>
      isOwnPerson(id) || (_people.byId[resolvePerson(id)]?.verified ?? false);

  /// Marks the Profile of the person [id] kept here as verified, when its
  /// short code is [code], the one the user compared in person: a copied
  /// id with another first key has another code and stays as it is.
  /// Returns whether it is verified now.
  bool verifyPerson(String id, String code) {
    if (isOwnPerson(id)) return true;
    final held = _people.byId[resolvePerson(id)];
    if (held == null || personCode(held.id) != code) return false;
    if (held.verified) return true;
    _keepPerson(held.sent, held.profile.picture, held.seen, verified: true);
    profileChanged();
    return true;
  }

  /// The Profile [json] a Sent Item carries, signed with [key], as its
  /// preview shows it (#51): its signatures hold and its binding holds
  /// [key], its name sanitised and its picture made again, with its
  /// short code; whether it is kept here already, bound to [key], and
  /// verified. Null when any of it fails: the file is then no Sent Item.
  SentProfile? sentItemProfile(Object? json, List<int> key) {
    final sent = SignedProfile.fromJson(json);
    final checked = sent == null ? null : _check(sent);
    if (sent == null || checked == null) return null;
    final p = checked.profile;
    final signer = base64.encode(key);
    if (!_chain(p.id, sent.first, sent.keys).$1.contains(signer)) return null;
    final code = profileShortCode(p.id, base64.decode(sent.first));
    var kept = false;
    var verified = false;
    if (isOwnPerson(p.id)) {
      kept = verified = ownProfileCode == code;
    } else if (_people.byId[p.id] case final held?
        when held.sent.first == sent.first) {
      kept = _chain(
        p.id,
        held.sent.first,
        sent.keys,
        held: held.sent.keys,
      ).$1.contains(signer);
      verified = kept && held.verified;
    }
    final shown = Profile(
      id: p.id,
      name: p.name,
      picture: checked.shown,
      version: p.version,
    );
    return (
      // The user's Override shows only for the Profile kept here, bound
      // to the key that signed: a copied id gets none.
      profile: kept && !isOwnPerson(p.id) ? _withOverride(shown) : shown,
      code: code,
      kept: kept,
      verified: verified,
    );
  }

  /// The signed Profiles that go out with a sync, as JSON, sorted by id.
  /// To one of the user's own Devices ([areas] null) every Profile of
  /// another person kept here; to another person's Device, or sealed
  /// under an Area's key, the own Profile and the Profiles of the people
  /// the grants of [areas] name now, so only the people of an Area get
  /// those of its people.
  List<Map<String, Object?>> profilesToSend({Set<String>? areas}) {
    final out = <String, Map<String, Object?>>{};
    if (areas == null) {
      for (final held in _people.byId.values) {
        out[held.id] = held.sent.toJson();
      }
    } else {
      if (ownSignedProfile case final own?) {
        final id = Profile.parse(own.record)?.id;
        if (id != null) {
          out[id] = SignedProfile(
            record: own.record,
            signer: own.signer,
            sig: own.sig,
            first: own.first,
            keys: own.keys,
            aliases: own.aliases,
          ).toJson();
        }
      }
      for (final area in areas) {
        for (final named in grantsOf(area)?.profiles ?? const <String>[]) {
          if (isOwnPerson(named)) continue;
          final held = _people.byId[resolvePerson(named)];
          if (held != null) out[held.id] = held.sent.toJson();
        }
      }
    }
    final ids = out.keys.toList()..sort(_compareUtf8);
    return [for (final id in ids) out[id]!];
  }

  /// The own Profile as it goes to another person: signed, with its
  /// binding and aliases, without what only one's own Devices are told.
  Map<String, Object?>? get ownProfileForOthers {
    final own = ownSignedProfile;
    if (own == null) return null;
    return SignedProfile(
      record: own.record,
      signer: own.signer,
      sig: own.sig,
      first: own.first,
      keys: own.keys,
      aliases: own.aliases,
    ).toJson();
  }

  /// Which Profile ids may come from the Device [peer] of another
  /// person: those the grants of an Area shared with it name.
  bool Function(String id) peopleFrom(String peer) {
    final areas = areasSharedWith(peer);
    final named = <String>{
      for (final area in areas) ...?grantsOf(area)?.profiles,
    };
    return named.contains;
  }

  /// Which Profile ids may come sealed under the key of the Area
  /// [area]: those its grants name.
  bool Function(String id) peopleOf(String area) =>
      (grantsOf(area)?.profiles ?? const <String>[]).toSet().contains;

  /// The person who wrote the Note or Section [entity], as their Profile
  /// shows: through the key of the Device that wrote its first `$type`,
  /// the Profile bound to it, through its aliases. In a Shared Area the
  /// grant of that Device names whose Profile it must be, so a Profile
  /// that claims somebody else's key shows nobody's Notes; until that
  /// Profile arrives, its placeholder. Null for the user's own: written
  /// on this Device, one of its own group, or a Device bound to the own
  /// Profile; and for a Note whose writer is not known. A Note accepted
  /// from a Sent Item shows its sender, through the key and the Profile
  /// id it keeps (#51).
  Profile? authorOf(String entity) {
    if (_writerOf(entity) case _Theirs(:final author)) {
      return _withOverride(author);
    }
    return null;
  }

  /// Whether the person filter of the browser keeps the Note or Section
  /// [entity] (#53): with [filter]'s `me`, the user's own, written on any
  /// of their Devices or under any of their Profile's ids; with its
  /// people, those of each of them, matched through the aliases from
  /// either side. A Note whose writer is not known is nobody's; with an
  /// empty filter every Note is kept.
  bool wroteBy(String entity, PersonFilter filter) {
    if (filter.isEveryone) return true;
    switch (_writerOf(entity)) {
      case _Own():
        return filter.me || filter.people.any(isOwnPerson);
      case _Theirs(:final author):
        final writer = resolvePerson(author.id);
        return filter.people.any(
          (id) => !isOwnPerson(id) && resolvePerson(id) == writer,
        );
      case _Unknown():
        return false;
    }
  }

  /// The Notes of [notes] that [filter] keeps, in their order
  /// ([wroteBy]).
  List<Note> notesBy(Iterable<Note> notes, PersonFilter filter) => [
    for (final note in notes)
      if (wroteBy(note.id, filter)) note,
  ];

  /// Who wrote the Note or Section [entity]: the user, another person
  /// as their Profile shows without the Override, or nobody known.
  _Writer _writerOf(String entity) {
    if (_receivedFrom(entity) case (:final key, :final person)) {
      if (_own.bound.contains(key) || isOwnPerson(person)) return const _Own();
      return _theirs(_shownFor(key, person));
    }
    String? device;
    for (final e in history(entity)) {
      if (e.field == r'$type') {
        device = e.device;
        break;
      }
    }
    if (device == null) return const _Unknown();
    if (device == deviceId || isOwnDevice(device)) return const _Own();
    final keyBytes = pinnedKey(device)?.record.publicKey;
    final key = keyBytes == null ? null : base64.encode(keyBytes);
    if (key != null && _own.bound.contains(key)) return const _Own();
    final area = areaOf(entity);
    final named = area == null ? null : grantsOf(area)?.personOf(device);
    if (named != null && isOwnPerson(named)) return const _Own();
    return _theirs(_shownFor(key, named));
  }

  static _Writer _theirs(Profile? author) =>
      author == null ? const _Unknown() : _Theirs(author);

  /// The sender's key and Profile id the Note [entity] keeps from the
  /// Sent Item it was accepted from; null for any other.
  ({String key, String person})? _receivedFrom(String entity) {
    final raw = current(entity, 'received');
    if (raw == null) return null;
    try {
      if (jsonDecode(raw) case {
        'from': final String key,
        'profile': final String person,
      }) {
        return (key: key, person: person);
      }
    } on FormatException {
      return null;
    }
    return null;
  }

  /// The Profile shown for a writer with the key [key]: the one [named]
  /// stands for among those bound to it, or its placeholder; without
  /// [named], the Profile first seen with the key.
  Profile? _shownFor(String? key, String? named) {
    final candidates =
        [
          if (key != null)
            for (final held in _people.byId.values)
              if (held.bound.contains(key)) held,
        ]..sort((a, b) {
          final bySeen = a.seen.compareTo(b.seen);
          return bySeen != 0 ? bySeen : _compareUtf8(a.id, b.id);
        });
    final byId = _people.byId;
    if (named == null) {
      final first = candidates.firstOrNull;
      return first == null
          ? null
          : (byId[resolvePerson(first.id)] ?? first).profile;
    }
    final target = resolvePerson(named);
    for (final held in candidates) {
      if (resolvePerson(held.id) == target) {
        return (byId[target] ?? held).profile;
      }
    }
    return Profile(id: target, name: '', version: 1);
  }
}
