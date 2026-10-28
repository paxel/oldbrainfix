import 'dart:convert';

import 'area_keys.dart';
import 'area_offers.dart';
import 'entry.dart';
import 'profiles.dart';
import 'shared_areas.dart';
import 'store.dart';

/// Who waits for changes by bundle (#109): the people changes travel
/// with by bundles ([Transports]) lacking a change that no bundle for
/// them carried yet, as the Changes Nudge covers them (CONTEXT.md,
/// [changesNudgePlan]).
///
/// What a bundle for a person carries stays what lies beyond what their
/// Devices said they hold ([SharedAreas.entriesForPerson]), so a bundle
/// that never arrived loses nothing. Beside it this Device keeps, per
/// person and Shared Area, how far the log had come when the last bundle
/// for them was sent: a local setting (ADR 0008), the mark of the log
/// ([Store.currentSeq]) by Area, each only ever growing. A change lies
/// beyond it until a bundle carried it.
extension ChangesNudges on Store {
  static const _prefix = 'changesSent:';

  /// How far the log had come, by Area, when changes were last sent to
  /// [person]; an Area missing was never sent to.
  Map<String, int> changesSentTo(String person) {
    final raw = localSetting('$_prefix${resolvePerson(person)}');
    if (raw == null) return const {};
    try {
      final json = jsonDecode(raw);
      return {
        if (json is Map)
          for (final MapEntry(:key, :value) in json.entries)
            if (key is String && value is int) key: value,
      };
    } on FormatException {
      return const {};
    }
  }

  /// A bundle for [person] went out, written when the log stood at
  /// [seq] ([Store.currentSeq], read before writing it): the changes up
  /// to there in every Area shared with them count as sent. The user's
  /// tap on the share sheet or the save dialog, never the writing alone,
  /// says it went.
  void noteChangesSent(String person, int seq) {
    final areas = <String>{
      for (final device in devicesOf(person)) ...areasSharedWith(device),
    };
    _noteSent(person, areas, seq);
  }

  /// The changes up to [seq] in [area] count as sent to [person]: the
  /// Area was offered to them, or taken from their offer (#108), so they
  /// hold its history.
  void noteAreaSent(String person, String area, int seq) =>
      _noteSent(person, {area}, seq);

  void _noteSent(String person, Set<String> areas, int seq) {
    if (areas.isEmpty) return;
    final sent = Map.of(changesSentTo(person));
    for (final area in areas) {
      if (seq > (sent[area] ?? 0)) sent[area] = seq;
    }
    final sorted = {for (final a in sent.keys.toList()..sort()) a: sent[a]};
    setLocalSetting('$_prefix${resolvePerson(person)}', jsonEncode(sorted));
  }

  /// Whether changes travel with [person] by bundles: so chosen, not
  /// blocked, and holding a role in an Area shared with them.
  bool sendsByBundle(String person) {
    final id = resolvePerson(person);
    return !isOwnPerson(id) &&
        !isBlocked(id) &&
        transportWith(id) == Transport.bundle &&
        devicesOf(id).isNotEmpty;
  }

  /// The changes a bundle for [person] carries that no bundle for them
  /// carried yet: of [SharedAreas.entriesForPerson] those beyond what was
  /// sent in their Area ([changesSentTo]), never one their own Devices
  /// wrote. In (device, dseq) order.
  List<Entry> unsentTo(String person) {
    final id = resolvePerson(person);
    final sent = changesSentTo(id);
    final theirs = devicesOf(id).toSet();
    final scopes = <String, String>{};
    return [
      for (final e in entriesForPerson(id))
        if (!theirs.contains(e.device) &&
            e.seq >
                (sent[scopes.putIfAbsent(e.entity, () => scopeOf(e.entity))] ??
                    0))
          e,
    ];
  }

  /// The people waiting for changes by bundle ([sendsByBundle]), by
  /// Profile id, with the time of the first change each lacks
  /// ([unsentTo]). A change this Device wrote counts from when it was
  /// written; any other from when it came here, the Moment it came with:
  /// another Device's clock never moves the Changes Nudge.
  Map<String, DateTime> waitingForChanges() {
    final people = <String>{
      for (final (key, value) in localSettingsWithPrefix(Transports.prefix))
        if (value == Transport.bundle.name)
          resolvePerson(key.substring(Transports.prefix.length)),
    };
    final out = <String, DateTime>{};
    // Newest first, read once and only when another Device's change
    // waits.
    late final moments = momentRows();
    for (final person in people.toList()..sort()) {
      if (!sendsByBundle(person)) continue;
      DateTime? first;
      for (final e in unsentTo(person)) {
        var at = e.recorded.toUtc();
        if (e.device != deviceId) {
          // The last Moment recorded before it came.
          for (final m in moments) {
            if (m.seq < e.seq) {
              at = m.at.toUtc();
              break;
            }
          }
        }
        if (first == null || at.isBefore(first)) first = at;
      }
      if (first != null) out[person] = first;
    }
    return out;
  }
}
