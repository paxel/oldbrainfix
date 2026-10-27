part of 'profiles.dart';

/// Overrides (ADR 0017, #52): a name and a picture the user gives
/// another person's Profile for themselves alone, such as "Mum". Kept in
/// the log as the entity `override:<profile id>`, outside every Area, so
/// it is sealed under the personal key and goes only to the user's own
/// Devices. Only what one's own Devices wrote counts: an entry another
/// person's Device slipped into the log changes nothing.
abstract final class OverrideKeys {
  /// The entity id's prefix, before the Profile id.
  static const prefix = 'override:';

  /// The name the user gave the person, sanitised like a Profile name;
  /// null or empty for none.
  static const name = 'overrideName';

  /// The picture the user gave the person, a Profile picture (JPEG) in
  /// base64; null for none.
  static const picture = 'overridePicture';
}

/// The name and picture the user gave another person; null where they
/// gave none.
typedef PersonOverride = ({String? name, Uint8List? picture});

extension Overrides on Store {
  /// The Override of the person [id], through their aliases; null when
  /// the user gave them neither name nor picture, or [id] is the own
  /// Profile.
  PersonOverride? personOverride(String id) {
    if (isOwnPerson(id)) return null;
    Entry? name, picture;
    final own = <String, bool>{};
    for (final e in history('${OverrideKeys.prefix}${resolvePerson(id)}')) {
      final isName = e.field == OverrideKeys.name;
      if (!isName && e.field != OverrideKeys.picture) continue;
      if (!own.putIfAbsent(e.device, () => writtenByOwn(e.device))) continue;
      final held = isName ? name : picture;
      if (held != null && Store.latestWins(e, held) <= 0) continue;
      if (isName) {
        name = e;
      } else {
        picture = e;
      }
    }
    final shownName = sanitizeProfileName(name?.value ?? '');
    final shownPicture = _overridePicture(picture?.value);
    if (shownName.isEmpty && shownPicture == null) return null;
    return (name: shownName.isEmpty ? null : shownName, picture: shownPicture);
  }

  /// Gives the person [id] the [name] for the user alone, sanitised; an
  /// empty name takes it away, so their own shows again. Nothing for the
  /// own Profile.
  void setOverrideName(String id, String name) {
    if (isOwnPerson(id)) return;
    final clean = sanitizeProfileName(name);
    final value = clean.isEmpty ? null : clean;
    if (personOverride(id)?.name == value) return;
    append(_overrideEntity(id), OverrideKeys.name, value);
    profileChanged();
  }

  /// Gives the person [id] the [picture] (a Profile picture) for the user
  /// alone; null takes it away, so their own shows again. Nothing for
  /// the own Profile.
  void setOverridePicture(String id, Uint8List? picture) {
    if (isOwnPerson(id)) return;
    if (_same(personOverride(id)?.picture, picture)) return;
    append(
      _overrideEntity(id),
      OverrideKeys.picture,
      picture == null ? null : base64.encode(picture),
    );
    profileChanged();
  }

  String _overrideEntity(String id) =>
      '${OverrideKeys.prefix}${resolvePerson(id)}';

  /// [profile] as the user sees it: the name and picture of its
  /// Override, where they gave one.
  Profile _withOverride(Profile profile) {
    final o = personOverride(profile.id);
    if (o == null) return profile;
    return Profile(
      id: profile.id,
      name: o.name ?? profile.name,
      picture: o.picture ?? profile.picture,
      version: profile.version,
    );
  }

  /// The newest value of [field] on [entity] that one of the user's own
  /// Devices wrote ([writtenByOwn]); null when none did. In a Shared
  /// Area another person's Device can write any field of a Note, so a
  /// link one's own Devices keep is read this way. [own] remembers, across
  /// calls, which Devices are one's own. Read from the log alone: a
  /// pending change (ADR 0022) is trusted for nothing.
  @internal
  String? currentByOwn(String entity, String field, [Map<String, bool>? own]) {
    own ??= {};
    Entry? held;
    for (final e in loggedHistory(entity)) {
      if (e.field != field) continue;
      if (!own.putIfAbsent(e.device, () => writtenByOwn(e.device))) continue;
      if (held == null || Store.latestWins(e, held) > 0) held = e;
    }
    return held?.value;
  }

  /// Whether the Device [device] is one of the user's own: this one, one
  /// paired as its own, or one whose key is bound to the own Profile.
  @internal
  bool writtenByOwn(String device) {
    if (device == deviceId || isOwnDevice(device)) return true;
    final key = pinnedKey(device)?.record.publicKey;
    return key != null && People(this)._own.bound.contains(base64.encode(key));
  }

  /// People (#52): every Profile of another person kept here, as the
  /// user sees them, Overrides applied, by name; a Profile another one
  /// kept here replaced shows only as that one, and a blocked person
  /// only among [Blocks.blockedPeople].
  List<Profile> get peopleShown {
    final ids = _people.byId.keys.toList()..sort(_compareUtf8);
    final blocked = blockedIds;
    final shown = [
      for (final id in ids)
        if (resolvePerson(id) == id && !blocked.contains(id)) personShown(id),
    ];
    shown.sort((a, b) {
      final byName = _compareUtf8(a.name, b.name);
      return byName != 0 ? byName : _compareUtf8(a.id, b.id);
    });
    return shown;
  }
}

/// The picture an Override keeps in [value]; null for none, and for
/// anything that is no standard base64 of a JPEG within
/// [profilePictureMaxBytes].
Uint8List? _overridePicture(String? value) {
  if (value == null || value.isEmpty) return null;
  try {
    final bytes = base64.decode(value);
    if (base64.encode(bytes) != value) return null;
    if (bytes.length > profilePictureMaxBytes || bytes.length < 3) return null;
    if (bytes[0] != 0xFF || bytes[1] != 0xD8 || bytes[2] != 0xFF) return null;
    return bytes;
  } on FormatException {
    return null;
  }
}
