import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'app_lock.dart';
import 'l10n.dart';
import 'schedule_text.dart';
import 'system_bars.dart';

/// The Conflicts (Catlog's conflicts screen adapted): what two Devices
/// changed apart, each with every version, the Device it came from and
/// when. Tapping a version keeps it, unless a version is a text in a
/// format this version does not know (ADR 0018); the Conflict ends here at once and
/// on the other Devices once they sync. Reached from Sync in Settings
/// and from the announcement after a sync that found some. What closed
/// locked Areas hide stays out ([visibleConflicts]).
class ConflictsScreen extends StatefulWidget {
  final Store store;

  const ConflictsScreen({super.key, required this.store});

  @override
  State<ConflictsScreen> createState() => _ConflictsScreenState();
}

class _ConflictsScreenState extends State<ConflictsScreen> {
  var _conflicts = <Conflict>[];

  /// The Conflicts this Device may pick a version of, by entity and unit.
  var _picks = <(String, String)>{};

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _read();
  }

  void _read() {
    _conflicts = visibleConflicts(
      widget.store,
      LockScope.of(context),
      widget.store.conflicts(),
    );
    // In a Shared Area only who may change a thing picks for it; a text
    // in a format not known here waits for a newer version.
    _picks = {
      for (final c in _conflicts)
        if (c.resolvable && widget.store.mayResolve(c.entity, c.type))
          (c.entity, c.unit),
    };
  }

  void _keep(Conflict conflict, ConflictVersion version) {
    widget.store.resolveConflict(conflict, version);
    setState(_read);
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return Scaffold(
      appBar: AppBar(title: Text(t.conflictsTitle)),
      body: ListView(
        padding: clearOfSystemBars(
          context,
          const EdgeInsets.fromLTRB(16, 8, 16, 24),
        ),
        children: [
          Text(_conflicts.isEmpty ? t.conflictsNone : t.conflictsIntro),
          for (final conflict in _conflicts)
            _ConflictCard(
              store: widget.store,
              conflict: conflict,
              onKeep: _picks.contains((conflict.entity, conflict.unit))
                  ? (version) => _keep(conflict, version)
                  : null,
            ),
        ],
      ),
    );
  }
}

class _ConflictCard extends StatelessWidget {
  final Store store;
  final Conflict conflict;

  /// Keeps a version; null where this Device may not pick.
  final ValueChanged<ConflictVersion>? onKeep;

  const _ConflictCard({
    required this.store,
    required this.conflict,
    required this.onKeep,
  });

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final locale = Localizations.localeOf(context).toString();
    final when = DateFormat.yMd(locale).add_Hm();
    final note = conflict.unit == scheduleUnit
        ? switch (Routines(store).byId(conflict.entity)) {
            final routine? => Routines(store).note(routine)?.text,
            null => null,
          }
        : null;
    return Card(
      margin: const EdgeInsets.only(top: 16),
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(
              conflictLabel(t, conflict),
              style: Theme.of(context).textTheme.titleSmall,
            ),
            if (note != null) Text(note),
            if (!conflict.resolvable)
              Text(t.conflictNewerFormat)
            else if (onKeep == null)
              Text(t.onlyManagers),
            for (final version in conflict.versions)
              Padding(
                padding: const EdgeInsets.only(top: 8),
                child: OutlinedButton(
                  onPressed: switch (onKeep) {
                    final keep? => () => keep(version),
                    null => null,
                  },
                  child: Semantics(
                    hint: t.conflictKeep,
                    child: Padding(
                      padding: const EdgeInsets.symmetric(vertical: 8),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(conflictValue(t, locale, conflict, version)),
                          Text(
                            conflictOrigin(
                              t,
                              store,
                              version.entry,
                              when.format(version.entry.recorded.toLocal()),
                            ),
                            style: Theme.of(context).textTheme.bodySmall,
                          ),
                        ],
                      ),
                    ),
                  ),
                ),
              ),
          ],
        ),
      ),
    );
  }
}

/// [conflicts] without those about what a closed locked Area hides: a
/// Note filed in it, a Section of such a Note, a Routine spawned from
/// one, or one of its Topics. Area names show, as they do in the list.
List<Conflict> visibleConflicts(
  Store store,
  AppLock? lock,
  List<Conflict> conflicts,
) {
  if (lock == null) return conflicts;
  final notes = Notes(store);
  bool hidden(Conflict c) {
    switch (c.type) {
      case NoteKeys.typeNote:
        final note = notes.byId(c.entity);
        return note != null && lock.hides(note);
      case SectionKeys.typeSection:
        final noteId = store.current(c.entity, SectionKeys.note);
        final note = noteId == null ? null : notes.byId(noteId);
        return note != null && lock.hides(note);
      case RoutineKeys.typeRoutine:
        final routine = Routines(store).byId(c.entity);
        return routine != null && lock.hidesRoutine(routine);
      case PlaceKeys.typeTopic:
        final topic = Places(store).topicById(c.entity);
        return topic != null && lock.isAreaClosed(topic.areaId);
    }
    return false;
  }

  return [
    for (final c in conflicts)
      if (!hidden(c)) c,
  ];
}

/// What [conflict] is about: which field of which kind of thing.
String conflictLabel(AppLocalizations t, Conflict conflict) => switch ((
  conflict.type,
  conflict.unit,
)) {
  (NoteKeys.typeNote, NoteKeys.text) => t.conflictNoteText,
  (SectionKeys.typeSection, SectionKeys.text) => t.conflictSectionText,
  (PlaceKeys.typeArea, PlaceKeys.name) => t.conflictAreaName,
  (PlaceKeys.typeTopic, PlaceKeys.name) => t.conflictTopicName,
  (NamedPlaceKeys.typeNamedPlace, NamedPlaceKeys.name) => t.conflictPlaceName,
  (NamedPlaceKeys.typeNamedPlace, NamedPlaceKeys.radius) =>
    t.conflictPlaceRadius,
  (RoutineKeys.typeRoutine, scheduleUnit) => t.conflictSchedule,
  _ => t.conflictOther,
};

/// What [version] holds, in words.
String conflictValue(
  AppLocalizations t,
  String locale,
  Conflict conflict,
  ConflictVersion version,
) {
  if (conflict.unit == scheduleUnit) {
    final schedule = Routines.scheduleOf(version.values);
    return schedule == null
        ? t.conflictEmpty
        : scheduleText(t, locale, schedule);
  }
  // A text shows as its source text, whatever its format.
  final value = conflict.unit == NoteKeys.text
      ? version.text.text
      : version.value;
  if (value == null || value.isEmpty) return t.conflictEmpty;
  if (conflict.unit == NamedPlaceKeys.radius) {
    final metres = double.tryParse(value);
    if (metres != null) return t.radiusMetres(metres.round());
  }
  return value;
}

/// Where and when [entry] was written: this Device, a paired Device by
/// the name it goes by, or the author name it carries.
String conflictOrigin(
  AppLocalizations t,
  Store store,
  Entry entry,
  String when,
) {
  if (entry.device == store.deviceId) return t.conflictOnThisDevice(when);
  final name = store.pairedDevice(entry.device)?.name ?? '';
  if (name.isNotEmpty) return t.conflictOnDevice(name, when);
  if (entry.author.isNotEmpty) return t.conflictOnDevice(entry.author, when);
  return t.conflictOnUnnamed(when);
}
