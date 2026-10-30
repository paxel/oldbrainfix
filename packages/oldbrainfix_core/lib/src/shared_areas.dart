import 'dart:convert';

import 'area_keys.dart';
import 'checklists.dart';
import 'entry.dart';
import 'frogs.dart';
import 'notes.dart';
import 'pairing.dart';
import 'places.dart';
import 'profiles.dart';
import 'routines.dart';
import 'sessions.dart';
import 'signing.dart';
import 'store.dart';
import 'tallies.dart';
import 'sync.dart';

/// Shared Areas (CONTEXT.md, ADR 0013): an Area other people sync with.
/// Who may do what in it is signed into the Area itself, one grant per
/// Device as `role:<device>` on the Area entity, so the grants travel
/// with the Area under its key. Every Device works out from the raw log
/// which entries a role does not allow ([judgeLog]) and leaves them out
/// of everything it shows; nothing is refused at import.

/// What a Device may do in a Shared Area.
enum AreaRole {
  /// Everything.
  manager,

  /// Add Notes and change their own, Amend any Note and check off
  /// Occurrences.
  member,
}

/// What this Device, or another, may do in an Area right now.
enum AreaAccess {
  /// The Area is not shared: its owner's Devices do everything.
  owner,
  manager,
  member,

  /// Shared, and no role here, or removed; or a grant this version
  /// cannot read ([Grant.newer]).
  none,
}

/// Field keys of the grants on an Area.
abstract final class ShareKeys {
  static const rolePrefix = 'role:';

  /// The grant of [device]: [Grant.encode], or null once removed.
  static String role(String device) => '$rolePrefix$device';

  /// On an Area moved to fresh ids before it was shared: the Area it
  /// went on as.
  static const movedTo = 'movedTo';
}

/// One grant as it is signed: the role, and the Profile of the person
/// the Device belongs to (ADR 0017).
class Grant {
  /// The role; null for a grant this version cannot read ([newer]).
  final AreaRole? role;

  /// The id of the person's Profile, as the grant named it.
  final String profile;

  const Grant(this.role, this.profile);

  /// Whether the grant needs a newer version of the app (ADR 0034): of
  /// a later structure, or with a role this version does not know. It
  /// holds, so the person keeps getting the Area, but allows nothing
  /// here, and nothing here changes or ends it.
  bool get newer => role == null;

  /// The value of the grant field: keys sorted, no spaces, the same from
  /// both cores. Never for a [newer] grant, which is not this version's
  /// to write.
  String encode() => switch (role) {
    final role? => jsonEncode({'profile': profile, 'role': role.name}),
    null => throw StateError('a grant of $profile not known here'),
  };

  /// The keys version 1 of a grant holds.
  static const _keys = {'profile', 'role', 'v'};

  /// The grant in [value]; null when it is none. A grant names a
  /// person; one of a structure other than version 1 (`v` absent or 1),
  /// with a key version 1 does not hold, or with a role this version
  /// does not know is [newer].
  static Grant? parse(String value) {
    try {
      final json = jsonDecode(value);
      if (json case {'profile': final String profile}
          when profile.isNotEmpty && json is Map<String, Object?>) {
        final v = json['v'];
        final known =
            (!json.containsKey('v') || (v is int && v == 1)) &&
            json.keys.every(_keys.contains);
        final role = json['role'];
        return Grant(
          known && role is String ? AreaRole.values.asNameMap()[role] : null,
          profile,
        );
      }
    } on FormatException {
      return null;
    }
    return null;
  }
}

/// Someone a Shared Area is shared with, as its grants say now: the id
/// of their Profile as the grants name it.
class Person {
  final String id;

  /// Their role; null when a grant of one of their Devices needs a newer
  /// version of the app ([Grant.newer]): then nothing here changes them.
  final AreaRole? role;

  /// The ids of their Devices that hold the role, sorted.
  final List<String> devices;

  const Person({required this.id, required this.role, required this.devices});

  /// Whether a grant of theirs needs a newer version of the app.
  bool get newer => role == null;
}

/// A grant that counts, with the entry that signed it; [grant] is null
/// for a removal.
typedef _Granted = ({Entry entry, Grant? grant});

/// The grants of one Shared Area that count, in latest-wins order.
class AreaGrants {
  /// The first grant that counts: the first Manager's to itself. What
  /// is dated before it was the owner's.
  final Entry first;
  final Map<String, List<_Granted>> _byDevice;

  AreaGrants._(this.first, this._byDevice);

  /// Every Device that ever held a grant here, sorted.
  List<String> get devices => _byDevice.keys.toList()..sort();

  /// The grant [device] holds now; null without one, or once removed.
  Grant? grantOf(String device) => _byDevice[device]?.last.grant;

  /// Whether [device] was ever granted anything here.
  bool everGranted(String device) => _byDevice.containsKey(device);

  /// The person [device] belongs to, the id of their Profile: of its
  /// last grant that named one.
  String? personOf(String device) {
    for (final g in (_byDevice[device] ?? const <_Granted>[]).reversed) {
      if (g.grant case final grant?) return grant.profile;
    }
    return null;
  }

  /// The Profile ids the grants that hold now name, sorted.
  List<String> get profiles => {
    for (final g in _byDevice.values)
      if (g.last.grant case final grant?) grant.profile,
  }.toList()..sort();

  /// What [device] may do at the entry [at]: everything before the
  /// first grant; else as its last grant before it says, or its first
  /// grant when it had none yet.
  AreaAccess accessAt(String device, Entry at) {
    if (Store.latestWins(at, first) < 0) return AreaAccess.owner;
    final granted = _byDevice[device];
    if (granted == null) return AreaAccess.none;
    var pick = granted.first;
    for (final g in granted) {
      if (Store.latestWins(g.entry, at) < 0) pick = g;
    }
    return _access(pick.grant);
  }

  static AreaAccess _access(Grant? grant) => switch (grant?.role) {
    AreaRole.manager => AreaAccess.manager,
    AreaRole.member => AreaAccess.member,
    null => AreaAccess.none,
  };
}

/// What [judgeLog] found: the entries no role allows, and the grants of
/// every Shared Area.
class JudgedLog {
  /// The ids (`<device>:<dseq>`) of the entries left out.
  final Set<String> ignored;

  /// The Shared Areas by id.
  final Map<String, AreaGrants> areas;

  const JudgedLog(this.ignored, this.areas);

  static const none = JudgedLog({}, {});
}

/// Which of [entries], the whole raw log, no role allows. Layered, so
/// nothing depends on what is still to be judged: the grants of every
/// Area by latest wins; the Topics by their Area; the Notes by their
/// Area or Topic; the Sections, Routines, Frogs, Tally entries and
/// Checklist Lines by their Note. Each entry is
/// judged in the Area its entity was in at that point, by the filings
/// that count: the last one before it, or the first one for what came
/// before any.
JudgedLog judgeLog(Iterable<Entry> entries) {
  final byEntity = <String, List<Entry>>{};
  for (final e in entries) {
    byEntity.putIfAbsent(e.entity, () => []).add(e);
  }
  for (final list in byEntity.values) {
    list.sort(Store.latestWins);
  }
  final ignored = <String>{};
  final areas = <String, AreaGrants>{};

  // The grants, Area by Area.
  for (final MapEntry(key: entity, value: list) in byEntity.entries) {
    if (!entity.startsWith('area:')) continue;
    Entry? first;
    final byDevice = <String, List<_Granted>>{};
    for (final e in list) {
      if (!e.field.startsWith(ShareKeys.rolePrefix)) continue;
      final target = e.field.substring(ShareKeys.rolePrefix.length);
      final value = e.value;
      final grant = value == null ? null : Grant.parse(value);
      bool counts() {
        if (target.isEmpty || (value != null && grant == null)) return false;
        if (first == null) {
          return target == e.device && grant?.role == AreaRole.manager;
        }
        final signer = byDevice[e.device]?.last.grant;
        if (signer == null) return false;
        if (signer.role == AreaRole.manager) return true;
        // A person ends the grants of their own Devices: they leave.
        if (grant == null) {
          return byDevice[target]?.last.grant?.profile == signer.profile;
        }
        // A grant not known here allows nothing more (ADR 0034).
        return signer.role == AreaRole.member &&
            grant.role == AreaRole.member &&
            grant.profile == signer.profile &&
            !byDevice.containsKey(target);
      }

      if (!counts()) {
        ignored.add(e.id);
        continue;
      }
      first ??= e;
      byDevice.putIfAbsent(target, () => []).add((entry: e, grant: grant));
    }
    if (first != null) areas[entity] = AreaGrants._(first, byDevice);
  }

  AreaAccess access(String? area, Entry e) =>
      areas[area]?.accessAt(e.device, e) ?? AreaAccess.owner;

  // The person who wrote the first `$type` of [list], in [area].
  String? ownerIn(String? area, List<Entry> list) {
    for (final e in list) {
      if (e.field == r'$type') return areas[area]?.personOf(e.device);
    }
    return null;
  }

  // Judges every entry of [entity]: the filings ([placing]) first, each
  // in the Area it leaves and the one it enters, then the rest in the
  // Area of the filing before them. Returns the filings that count, as
  // (entry, Area).
  List<(Entry, String?)> judge(
    String entity, {
    required String placing,
    required String? Function(String? value) areaOf,
    required bool Function(AreaAccess access, Entry e, String? area) may,
  }) {
    final list = byEntity[entity] ?? const <Entry>[];
    final filings = <(Entry, String?)>[];
    for (final e in list) {
      if (e.field != placing) continue;
      final target = areaOf(e.value);
      final from = filings.isEmpty ? null : filings.last.$2;
      final ok =
          (filings.isEmpty || may(access(from, e), e, from)) &&
          may(access(target, e), e, target);
      if (ok) {
        filings.add((e, target));
      } else {
        ignored.add(e.id);
      }
    }
    for (final e in list) {
      if (e.field == placing) continue;
      final area = areaAt(filings, e);
      if (!may(access(area, e), e, area)) ignored.add(e.id);
    }
    return filings;
  }

  bool full(AreaAccess a) => a == AreaAccess.owner || a == AreaAccess.manager;

  final ids = byEntity.keys.toList()..sort();
  final topicArea = <String, String?>{};
  for (final id in ids) {
    if (!id.startsWith('topic:')) continue;
    final filings = judge(
      id,
      placing: PlaceKeys.area,
      areaOf: (value) =>
          value != null && value.startsWith('area:') ? value : null,
      may: (a, e, area) => full(a),
    );
    topicArea[id] = filings.isEmpty ? null : filings.last.$2;
  }

  final noteFilings = <String, List<(Entry, String?)>>{};
  for (final id in ids) {
    if (!id.startsWith('note:')) continue;
    final list = byEntity[id] ?? const <Entry>[];
    noteFilings[id] = judge(
      id,
      placing: NoteKeys.place,
      areaOf: (value) => value == null
          ? null
          : value.startsWith('topic:')
          ? topicArea[value]
          : value.startsWith('area:')
          ? value
          : null,
      may: (a, e, area) {
        if (full(a)) return true;
        if (a != AreaAccess.member) return false;
        // Any Note gets its texts from a Member's machine; removing one
        // is the owner's (ADR 0025).
        if (e.value != null &&
            (e.field.startsWith(NoteKeys.transcriptPrefix) ||
                e.field.startsWith(NoteKeys.ocrPrefix))) {
          return true;
        }
        final owner = ownerIn(area, list);
        return owner != null && owner == areas[area]?.personOf(e.device);
      },
    );
  }

  for (final id in ids) {
    // A Tally's entry is its author's, as a Section is.
    final section =
        id.startsWith('section:') || id.startsWith(TallyKeys.entryPrefix);
    final frog = id.startsWith(FrogKeys.prefix);
    final line = id.startsWith(ChecklistKeys.linePrefix);
    if (!section && !frog && !line && !id.startsWith('routine:')) continue;
    final list = byEntity[id] ?? const <Entry>[];
    String? note;
    for (final e in list) {
      if (e.field == SectionKeys.note) {
        note = e.value;
        break;
      }
    }
    final filings = noteFilings[note] ?? const [];
    for (final e in list) {
      final area = areaAt(filings, e);
      final a = access(area, e);
      final ok = switch (a) {
        AreaAccess.owner || AreaAccess.manager => true,
        AreaAccess.none => false,
        AreaAccess.member =>
          section
              ? ownerIn(area, list) == areas[area]?.personOf(e.device) &&
                    ownerIn(area, list) != null
              // A Member checks any Line and adds their own; the rest
              // is for the author of the Line or of its Note (ADR 0030).
              : line
              ? e.field == ChecklistKeys.checked ||
                    switch (areas[area]?.personOf(e.device)) {
                      final writer? =>
                        writer == ownerIn(area, list) ||
                            writer == ownerIn(area, byEntity[note] ?? const []),
                      null => false,
                    }
              // A Member checks off, and a Session's picks and the
              // entries recorded for them are anyone's (ADR 0029), as
              // are the checks of Lines for an Occurrence (ADR 0030);
              // the Frog and the Routine are the Manager's.
              : frog
              ? e.field == FrogKeys.done
              : e.field.startsWith(RoutineKeys.donePrefix) ||
                    e.field.startsWith(SessionKeys.pickPrefix) ||
                    e.field.startsWith(SessionKeys.entryPrefix) ||
                    e.field.startsWith(ChecklistKeys.checkPrefix),
      };
      if (!ok) ignored.add(e.id);
    }
  }

  // An Area's own fields, but for the grants judged above.
  for (final MapEntry(key: entity, value: list) in byEntity.entries) {
    final grants = areas[entity];
    if (grants == null) continue;
    for (final e in list) {
      if (e.field.startsWith(ShareKeys.rolePrefix)) continue;
      if (!full(grants.accessAt(e.device, e))) ignored.add(e.id);
    }
  }
  return JudgedLog(ignored, areas);
}

/// The Area of the filing that counts at [e]: the last one before it,
/// else the first one.
String? areaAt(List<(Entry, String?)> filings, Entry e) {
  if (filings.isEmpty) return null;
  var area = filings.first.$2;
  for (final (f, a) in filings) {
    if (Store.latestWins(f, e) < 0) area = a;
  }
  return area;
}

extension SharedAreas on Store {
  /// The grants of the Shared Area [areaId], or null when it is not
  /// shared.
  AreaGrants? grantsOf(String areaId) => judged.areas[areaId];

  bool isShared(String areaId) => grantsOf(areaId) != null;

  /// The Area [entity] lies in now, or null when it lies in none.
  String? areaOf(String entity) {
    final scope = scopeOf(entity);
    return scope == personalScope ? null : scope;
  }

  /// What [device] (this one by default) may do in [areaId] now.
  AreaAccess accessIn(String areaId, {String? device}) {
    final grants = grantsOf(areaId);
    if (grants == null) return AreaAccess.owner;
    return AreaGrants._access(grants.grantOf(device ?? deviceId));
  }

  /// Whether this Device may change the Area [areaId] itself, its Topics
  /// and its Routines and Reminders: everything a Member may not.
  bool mayManage(String? areaId) {
    if (areaId == null) return true;
    final a = accessIn(areaId);
    return a == AreaAccess.owner || a == AreaAccess.manager;
  }

  /// Whether this Device may change the Note or Section [id]: in no Area
  /// or one it manages, or as a Member when it is theirs.
  bool mayChange(String id) {
    final area = areaOf(id);
    if (mayManage(area)) return true;
    if (area == null || accessIn(area) != AreaAccess.member) return false;
    final grants = grantsOf(area);
    if (grants == null) return false;
    final mine = grants.personOf(deviceId);
    return mine != null && _creatorPerson(id, grants) == mine;
  }

  /// Whether this Device may write anything at all in the Area [entity]
  /// lies in (an Area, Topic, Note or Section): in none, or holding a
  /// role there. A Device removed from a Shared Area may not.
  bool mayWriteIn(String entity) {
    final area = entity.startsWith('area:') ? entity : areaOf(entity);
    return area == null || accessIn(area) != AreaAccess.none;
  }

  /// Whether this Device may pick a version of a Conflict on [entity] of
  /// [type] (`$type`): the pick is an edit like any other.
  bool mayResolve(String entity, String type) => switch (type) {
    NoteKeys.typeNote ||
    SectionKeys.typeSection ||
    TallyKeys.typeEntry => mayChange(entity),
    ChecklistKeys.typeLine => Checklists(this).mayEdit(entity),
    RoutineKeys.typeRoutine || FrogKeys.typeFrog => mayChangeRoutine(entity),
    PlaceKeys.typeArea => mayManage(entity),
    PlaceKeys.typeTopic => mayManage(areaOf(entity)),
    _ => true,
  };

  /// Whether this Device may change the Routine or Reminder [id].
  bool mayChangeRoutine(String id) => mayManage(areaOf(id));

  /// Whether this Device may check off Occurrences of the Routine [id]:
  /// everywhere it holds a role.
  bool mayCheckOff(String id) {
    final area = areaOf(id);
    return area == null || accessIn(area) != AreaAccess.none;
  }

  String? _creatorPerson(String id, AreaGrants grants) {
    for (final e in history(id)) {
      if (e.field == r'$type') return grants.personOf(e.device);
    }
    return null;
  }

  /// The people of the Shared Area [areaId] as its grants say now, this
  /// Device's own first, then by the name of their Profile.
  List<Person> people(String areaId) {
    final grants = grantsOf(areaId);
    if (grants == null) return const [];
    final byPerson = <String, (Grant, List<String>)>{};
    for (final device in grants.devices) {
      final grant = grants.grantOf(device);
      if (grant == null) continue;
      final held = byPerson[grant.profile];
      if (held == null) {
        byPerson[grant.profile] = (grant, [device]);
      } else {
        held.$2.add(device);
        // One grant not known here makes the person's role one too.
        if (grant.newer) byPerson[grant.profile] = (grant, held.$2);
      }
    }
    final mine = grants.personOf(deviceId);
    final names = {for (final id in byPerson.keys) id: personShown(id).name};
    return [
      for (final MapEntry(:key, value: (grant, devices)) in byPerson.entries)
        Person(id: key, role: grant.role, devices: devices),
    ]..sort((a, b) {
      if ((a.id == mine) != (b.id == mine)) return a.id == mine ? -1 : 1;
      final byName = names[a.id]!.compareTo(names[b.id]!);
      return byName != 0 ? byName : a.id.compareTo(b.id);
    });
  }

  /// The Shared Areas the Device [device] holds a role in now, sorted.
  Set<String> areasSharedWith(String device) => {
    for (final MapEntry(:key, :value) in judged.areas.entries)
      if (value.grantOf(device) != null) key,
  };

  /// Shares the Area [areaId]: moves it to fresh ids first when every
  /// Device plants it under the same one (a Starter Set Area), then
  /// makes this Device its first Manager, under the own Profile, and
  /// every own paired Device a Manager with it. Returns the id the Area
  /// goes on under. An Area already shared stays as it is.
  String share(String areaId) {
    if (isShared(areaId)) return areaId;
    return batch(() {
      final id = areaId == _starterAreaId(areaId) ? _rehome(areaId) : areaId;
      // Without a Profile yet (only in tests) the Device stands for it.
      final profile = ownProfile?.id ?? deviceId;
      final grant = Grant(AreaRole.manager, profile).encode();
      append(id, ShareKeys.role(deviceId), grant);
      for (final device in pairedDevices()) {
        if (!device.shared) append(id, ShareKeys.role(device.id), grant);
      }
      return id;
    });
  }

  /// Grants [device] [role] in the Shared Area [areaId], as the person
  /// whose Profile is [profile]: under the person it already belongs to
  /// there, else that one. Nothing for a Device of this one's own group,
  /// nor for one holding a grant not known here.
  void invite(
    String areaId,
    String device, {
    required AreaRole role,
    required String profile,
  }) {
    // A Device of this one's own group is no other person: it keeps the
    // role it has.
    if (isOwnDevice(device)) return;
    // Nor one whose grant is not known here: it stays (ADR 0034).
    if (grantsOf(areaId)?.grantOf(device)?.newer ?? false) return;
    final person = grantsOf(areaId)?.personOf(device) ?? profile;
    append(areaId, ShareKeys.role(device), Grant(role, person).encode());
  }

  /// Gives every Device of [person] in [areaId] the role [role]. Refused
  /// while a grant of theirs is not known here (ADR 0034).
  void setRole(String areaId, String person, AreaRole role) {
    final grants = grantsOf(areaId);
    if (grants == null) return;
    _refuseNewer(grants, person);
    batch(() {
      for (final device in grants.devices) {
        final grant = grants.grantOf(device);
        if (grant == null || grant.profile != person) continue;
        if (grant.role == role) continue;
        append(areaId, ShareKeys.role(device), Grant(role, person).encode());
      }
    });
  }

  /// Takes [person] out of [areaId]: their Devices' grants end, and no
  /// Device that learns of it sends them anything more of the Area. They
  /// keep what they have; only a new key would take that (ADR 0013).
  /// Refused while a grant of theirs is not known here (ADR 0034).
  void removePerson(String areaId, String person) {
    final grants = grantsOf(areaId);
    if (grants == null) return;
    _refuseNewer(grants, person);
    batch(() {
      for (final device in grants.devices) {
        if (grants.grantOf(device)?.profile == person) {
          append(areaId, ShareKeys.role(device), null);
        }
      }
    });
  }

  /// Throws when a grant of [person] in [grants] is not known here.
  void _refuseNewer(AreaGrants grants, String person) {
    for (final device in grants.devices) {
      final grant = grants.grantOf(device);
      if (grant != null && grant.profile == person && grant.newer) {
        throw StateError('a grant of $person is not known here');
      }
    }
  }

  /// Leaves the Shared Area [areaId] (#90): ends the grant of every
  /// Device of the user's there, this Device's last, so nothing more of
  /// it comes or goes. What is here stays as it was. A Member may leave
  /// too: a person may end the grants of their own Devices.
  void leaveArea(String areaId) {
    final grants = grantsOf(areaId);
    if (grants == null) return;
    final self = deviceId;
    final mine = grants.grantOf(self)?.profile;
    final devices = [
      for (final device in grants.devices)
        if (grants.grantOf(device) case final grant?
            when grant.profile == mine || isOwnPerson(grant.profile))
          device,
    ]..sort((a, b) => (a == self ? 1 : 0) - (b == self ? 1 : 0));
    if (devices.isEmpty) return;
    batch(() {
      for (final device in devices) {
        append(areaId, ShareKeys.role(device), null);
      }
    });
  }

  /// Gives the Device [peer] of this Device's own group, met in a sync,
  /// this Device's role and person in every Shared Area where it
  /// holds one and [peer] never held any; not a grant this version
  /// cannot read, which only a newer one may pass on. Returns how many
  /// it gave.
  int extendRolesTo(String peer) {
    if (peer.isEmpty || peer == deviceId) return 0;
    final self = deviceId;
    final given = [
      for (final MapEntry(:key, :value) in judged.areas.entries)
        if (value.grantOf(self) case final grant?
            when !grant.newer && !value.everGranted(peer))
          (key, grant),
    ]..sort((a, b) => a.$1.compareTo(b.$1));
    if (given.isEmpty) return 0;
    batch(() {
      for (final (area, grant) in given) {
        append(area, ShareKeys.role(peer), grant.encode());
      }
    });
    return given.length;
  }

  /// Whether this Device must refuse an invite to [areaId]: it holds an
  /// Area of that id that nobody shared, so it would be its own.
  bool refusesInvite(String areaId) =>
      !isShared(areaId) && history(areaId).isNotEmpty;

  /// The starter id [areaId] would have, when it has one.
  String? _starterAreaId(String areaId) {
    final slug = current(areaId, PlaceKeys.starter);
    return slug == null ? null : Places.starterId(PlaceKeys.typeArea, slug);
  }

  /// Moves the Area [areaId] with its Topics to fresh ids, files every
  /// Note in them again (trashed ones too) and deletes the old ones; the
  /// old Area points at the new one, so an Area Lock follows. Returns
  /// the new id. Runs inside a batch.
  String _rehome(String areaId) {
    final places = Places(this);
    final area = places.areaById(areaId);
    if (area == null) return areaId;
    final id = 'area:${Store.newId()}';
    append(id, PlaceKeys.type, PlaceKeys.typeArea);
    append(id, PlaceKeys.name, area.name);
    if (area.starter case final starter?) {
      append(id, PlaceKeys.starter, starter);
    }
    if (area.colour case final colour?) append(id, PlaceKeys.colour, colour);
    if (area.icon case final icon?) append(id, PlaceKeys.icon, icon);
    final moved = {areaId: id};
    for (final topic in places.topics(areaId)) {
      final topicId = 'topic:${Store.newId()}';
      append(topicId, PlaceKeys.type, PlaceKeys.typeTopic);
      append(topicId, PlaceKeys.area, id);
      append(topicId, PlaceKeys.name, topic.name);
      if (topic.starter case final starter?) {
        append(topicId, PlaceKeys.starter, starter);
      }
      moved[topic.id] = topicId;
    }
    for (final note in entitiesOfType(NoteKeys.typeNote)..sort()) {
      final to = moved[current(note, NoteKeys.place)];
      if (to != null) append(note, NoteKeys.place, to);
    }
    for (final old in moved.keys.toList()..sort()) {
      if (old != areaId) append(old, PlaceKeys.type, null);
    }
    append(areaId, ShareKeys.movedTo, id);
    append(areaId, PlaceKeys.type, null);
    return id;
  }

  // ------------------------------------------------------------ sending

  /// What goes to the Device [peer] paired by invite, of the entries a
  /// Device holding [vector] lacks: those of the Shared Areas [peer]
  /// holds a role in now, and the whole history of every Note filed into
  /// one of them meanwhile ([withFiled]). [whole] sends the Areas' whole
  /// history, as an invite does. In (device, dseq) order.
  List<Entry> entriesFor(
    String peer,
    Map<String, int> vector, {
    bool whole = false,
    Set<String> also = const {},
  }) {
    final areas = {...areasSharedWith(peer), ...also};
    if (areas.isEmpty) return const [];
    final delta = entriesSince(whole ? const {} : vector);
    final scopes = <String, String>{};
    return [
      for (final e in whole ? delta : withFiled(delta, areas.contains))
        if (areas.contains(
          scopes.putIfAbsent(e.entity, () => scopeOf(e.entity)),
        ))
          e,
    ];
  }

  /// [delta] with the whole history of every Note a filing in it put
  /// into an Area [wanted] says yes to, with its Sections, Routines,
  /// Frogs, Tally entries and Checklist Lines:
  /// a Note captured Unsorted and filed later began below the vector. In
  /// (device, dseq) order, each entry once.
  List<Entry> withFiled(List<Entry> delta, bool Function(String area) wanted) {
    final notes = <String>{
      for (final e in delta)
        if (e.field == NoteKeys.place && e.entity.startsWith('note:'))
          if (areaOf(e.entity) case final area? when wanted(area)) e.entity,
    };
    if (notes.isEmpty) return delta;
    final entities = {...notes};
    for (final type in [
      SectionKeys.typeSection,
      RoutineKeys.typeRoutine,
      FrogKeys.typeFrog,
      TallyKeys.typeEntry,
      ChecklistKeys.typeLine,
    ]) {
      for (final id in entitiesOfType(type)) {
        if (notes.contains(current(id, SectionKeys.note))) entities.add(id);
      }
    }
    final seen = {for (final e in delta) e.id};
    return [
      ...delta,
      for (final e in rawHistoryOf(entities))
        if (seen.add(e.id)) e,
    ]..sort((a, b) {
      final byDevice = a.device.compareTo(b.device);
      return byDevice != 0 ? byDevice : a.dseq.compareTo(b.dseq);
    });
  }

  /// Every entry that lies in the Area [areaId] now, by (device, dseq):
  /// its whole history, as an offer from afar carries it (#108).
  List<Entry> areaHistory(String areaId) {
    final scopes = <String, String>{};
    return [
      for (final e in entriesSince(const {}))
        if (scopes.putIfAbsent(e.entity, () => scopeOf(e.entity)) == areaId) e,
    ];
  }

  /// The Devices the grants of the Shared Areas give [person] a role on
  /// now, never one of the user's own; sorted.
  List<String> devicesOf(String person) {
    final target = resolvePerson(person);
    final out = <String>{};
    for (final grants in judged.areas.values) {
      for (final device in grants.devices) {
        if (grants.grantOf(device) case final grant?
            when resolvePerson(grant.profile) == target &&
                !writtenByOwn(device)) {
          out.add(device);
        }
      }
    }
    return out.toList()..sort();
  }

  /// What a bundle for the person [person] carries (#108): what lies in
  /// the Areas shared with any Device of theirs ([devicesOf]), beyond
  /// what any of those Devices said it holds, with the whole history of
  /// Notes filed into them meanwhile. Their Devices sync with each other,
  /// so the one that reads it passes it on.
  List<Entry> entriesForPerson(String person) {
    final devices = devicesOf(person);
    if (devices.isEmpty) return const [];
    final vector = <String, int>{};
    final areas = <String>{};
    for (final device in devices) {
      for (final MapEntry(:key, :value) in heldBy(device).entries) {
        if (value > (vector[key] ?? 0)) vector[key] = value;
      }
      areas.addAll(areasSharedWith(device));
    }
    return entriesFor(devices.first, vector, also: areas);
  }

  /// The signing keys the people of the Shared Area [areaId] check its
  /// entries with (#108): this Device's own and those held here of every
  /// Device its grants ever named, by device.
  List<KeyRecord> signingKeysOf(String areaId) {
    final self = deviceId;
    return [
      ownKeyRecord,
      for (final device in grantsOf(areaId)?.devices ?? const <String>[])
        if (device != self) ?pinnedKey(device)?.record,
    ]..sort((a, b) => a.device.compareTo(b.device));
  }

  /// The Area keys [peer], paired by invite, gets: those of the Areas it
  /// holds a role in, and of [also].
  List<AreaKey> areaKeysFor(String peer, {Set<String> also = const {}}) {
    final areas = {...areasSharedWith(peer), ...also};
    return [
      for (final k in areaKeys())
        if (areas.contains(k.scope)) k,
    ];
  }

  /// Whether the blob [ref] (`photo/<hash>`) lies on a Note or Section
  /// of one of [areas].
  bool blobIn(String ref, Set<String> areas) {
    final slash = ref.indexOf('/');
    if (slash < 0) return false;
    final prefix = switch (ref.substring(0, slash)) {
      'photo' => NoteKeys.photoPrefix,
      'audio' => NoteKeys.audioPrefix,
      _ => null,
    };
    if (prefix == null) return false;
    final field = '$prefix${ref.substring(slash + 1)}';
    for (final (entity, f) in fieldsWithPrefix(field)) {
      if (f != field || current(entity, f) == null) continue;
      if (areas.contains(scopeOf(entity))) return true;
    }
    return false;
  }
}

/// A paired Device met by invite is another person's: it gets only what
/// it shares (ADR 0013).
extension SharedPeers on Store {
  /// Whether the Device [id] is another person's, paired by invite.
  bool isSharedPeer(String id) => pairedDevice(id)?.shared ?? false;

  /// Whether the Device [id] is one of this Device's own group: paired
  /// without an invite.
  bool isOwnDevice(String id) => pairedDevice(id)?.shared == false;

  /// Whether the Device [id] is another person's as far as this Device
  /// knows (#108): paired by invite, given a role in a Shared Area for
  /// another person, or bound to the Profile of another person kept
  /// here, as one added from a Profile card; never one of the user's
  /// own.
  bool isOthersDevice(String id) {
    if (isSharedPeer(id)) return true;
    if (writtenByOwn(id)) return false;
    for (final grants in judged.areas.values) {
      final person = grants.personOf(id);
      if (person != null && !isOwnPerson(person)) return true;
    }
    return keptPeople.any((p) => pinnedDevicesOf(p.id).contains(id));
  }
}
