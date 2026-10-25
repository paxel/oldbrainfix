import 'entry.dart';
import 'named_places.dart';
import 'notes.dart';
import 'places.dart';
import 'routines.dart';
import 'store.dart';
import 'text_format.dart';

/// Concurrent edits (ADR 0002, Catlog's conflicts adapted): two Devices
/// changed the same thing without either having seen the other's
/// change. Latest wins keeps showing one of them, the same on every
/// Device, until the user picks the version that stays.
///
/// Every edit to a field it applies to carries its causal context
/// ([Entry.ctx]): how far the writer had seen each other Device that
/// wrote the field. An entry has seen another when it comes later from
/// the same Device or its context reaches that entry's dseq; an entry
/// without a context (a first write, or written before contexts were
/// kept) has been seen by everything newer by latest wins. A Conflict is a
/// field whose entries nobody has seen since (its heads) hold more than
/// one value. It is read from the log, so a pick that has seen every
/// head ends it on every Device it reaches; nothing else is written.
///
/// A Note's or Section's text, the name of an Area, a Topic or a Named
/// Place and a Named Place's radius each conflict on their own; a
/// Routine's start, times and rule conflict together as its schedule.
/// Done marks merge, and trash, restore, filing, Importance, colours and
/// icons go by latest wins.
///
/// A text carries its format in the same value (ADR 0018), so text and
/// format conflict as one; a Conflict one of whose versions is in a
/// format this version does not know is left for a Device that knows it.

/// The unit a Routine's schedule fields conflict as.
const scheduleUnit = 'schedule';

/// Every field concurrent edits are a Conflict on.
const conflictFields = [
  NoteKeys.text,
  PlaceKeys.name,
  NamedPlaceKeys.radius,
  ...RoutineKeys.scheduleFields,
];

/// The unit [field] conflicts as, or null when concurrent edits to it
/// simply go by latest wins. Section text shares [NoteKeys.text], a Named
/// Place's name [PlaceKeys.name].
String? conflictUnit(String field) => switch (field) {
  NoteKeys.text || PlaceKeys.name || NamedPlaceKeys.radius => field,
  RoutineKeys.start || RoutineKeys.times || RoutineKeys.rule => scheduleUnit,
  _ => null,
};

/// The fields of [unit].
List<String> conflictUnitFields(String unit) =>
    unit == scheduleUnit ? RoutineKeys.scheduleFields : [unit];

/// [Entry.ctx] for [seen]: `<device>:<dseq>` pairs by device.
String encodeContext(Map<String, int> seen) {
  final devices = seen.keys.toList()..sort();
  return [for (final d in devices) '$d:${seen[d]}'].join(' ');
}

/// What [encodeContext] wrote; pairs that are not pairs are skipped.
Map<String, int> decodeContext(String ctx) {
  final seen = <String, int>{};
  for (final pair in ctx.split(' ')) {
    final colon = pair.lastIndexOf(':');
    if (colon <= 0) continue;
    final dseq = int.tryParse(pair.substring(colon + 1));
    if (dseq != null) seen[pair.substring(0, colon)] = dseq;
  }
  return seen;
}

/// Whether [later] was written having seen [earlier]: what its context
/// reaches. An entry without a context (a first write, or written before
/// contexts were kept) has been seen by everything newer by latest wins,
/// and sees nothing that has one, so it never starts a Conflict.
bool _saw(Entry later, Entry earlier) {
  if (later.device == earlier.device) return earlier.dseq < later.dseq;
  final ctx = later.ctx;
  if (ctx != null &&
      (decodeContext(ctx)[earlier.device] ?? 0) >= earlier.dseq) {
    return true;
  }
  return earlier.ctx == null && Store.latestWins(earlier, later) < 0;
}

/// One side of a Conflict: what its unit held for the Device that wrote
/// [entry], the newest edit holding it nobody has seen since.
class ConflictVersion {
  /// Who wrote it and when: its device and recording time.
  final Entry entry;

  /// The value of every field of the unit, null when cleared or never
  /// written.
  final Map<String, String?> values;

  const ConflictVersion(this.entry, this.values);

  /// The value of a unit of one field.
  String? get value => values.values.firstOrNull;

  /// The text and format of a text unit's [value].
  StoredText get text => StoredText.read(value);
}

/// Concurrent edits to one unit of one entity.
class Conflict {
  final String entity;

  /// The field, or [scheduleUnit].
  final String unit;

  /// The entity's `$type`: note, section, area, topic, routine or
  /// namedPlace.
  final String type;

  /// At least two, the newest by latest wins first.
  final List<ConflictVersion> versions;

  const Conflict({
    required this.entity,
    required this.unit,
    required this.type,
    required this.versions,
  });

  /// Whether this version may pick one: not when a version is a text in a
  /// format it does not know, which picking another would overwrite and
  /// keeping could not show as it is (ADR 0018).
  bool get resolvable =>
      unit != NoteKeys.text || versions.every((v) => v.text.editable);
}

extension Conflicts on Store {
  /// Every open Conflict, by entity and unit.
  List<Conflict> conflicts() {
    final devices = <(String, String), Set<String>>{};
    for (final (entity, field, device) in writersOf(conflictFields)) {
      final unit = conflictUnit(field);
      if (unit == null) continue;
      devices.putIfAbsent((entity, unit), () => {}).add(device);
    }
    return _on([
      for (final MapEntry(:key, :value) in devices.entries)
        if (value.length > 1) key,
    ]);
  }

  /// The open Conflicts [entries] touch, by entity and unit: what an
  /// import brought.
  List<Conflict> conflictsAmong(Iterable<Entry> entries) => _on({
    for (final e in entries)
      if (conflictUnit(e.field) case final unit?) (e.entity, unit),
  });

  List<Conflict> _on(Iterable<(String, String)> units) {
    final sorted = units.toList()
      ..sort((a, b) {
        final byEntity = a.$1.compareTo(b.$1);
        return byEntity != 0 ? byEntity : a.$2.compareTo(b.$2);
      });
    return [for (final (entity, unit) in sorted) ?conflictOn(entity, unit)];
  }

  /// The open Conflict on [unit] of [entity], or null.
  Conflict? conflictOn(String entity, String unit) {
    final type = current(entity, r'$type');
    if (type == null) return null;
    final fields = conflictUnitFields(unit);
    final written = [
      for (final e in history(entity))
        if (fields.contains(e.field)) e,
    ];
    if ({for (final e in written) e.device}.length < 2) return null;
    final heads = [
      for (final h in written)
        if (!written.any((x) => !identical(x, h) && _saw(x, h))) h,
    ]..sort((a, b) => Store.latestWins(b, a));
    final versions = <ConflictVersion>[];
    for (final head in heads) {
      // What the head's writer had of each field: the last it had of it,
      // the newest by latest wins among several.
      final values = <String, String?>{};
      for (final field in fields) {
        final had = [
          for (final x in written)
            if (x.field == field && (identical(x, head) || _saw(head, x))) x,
        ];
        Entry? newest;
        for (final x in had) {
          if (had.any((y) => !identical(y, x) && _saw(y, x))) continue;
          if (newest == null || Store.latestWins(x, newest) > 0) newest = x;
        }
        values[field] = newest?.value;
      }
      // A text written before its format was recorded is the same as
      // that text marked plain.
      final same = versions.any(
        (v) => unit == NoteKeys.text
            ? v.text == StoredText.read(values[unit])
            : fields.every((f) => v.values[f] == values[f]),
      );
      if (!same) versions.add(ConflictVersion(head, values));
    }
    if (versions.length < 2) return null;
    return Conflict(entity: entity, unit: unit, type: type, versions: versions);
  }

  /// Keeps [kept]: writes every field of the unit again as it holds
  /// them, having seen every version, so the Conflict ends wherever this
  /// reaches. Dated after every edit of the unit, so a Device whose clock
  /// ran ahead cannot keep showing its value. A Conflict that is not
  /// [Conflict.resolvable] here is refused.
  void resolveConflict(Conflict conflict, ConflictVersion kept) {
    if (!conflict.resolvable) {
      throw StateError(
        'a version of ${conflict.entity} is in a format not '
        'known here',
      );
    }
    final fields = conflictUnitFields(conflict.unit);
    var date = clock().toUtc();
    for (final e in history(conflict.entity)) {
      if (!fields.contains(e.field)) continue;
      final after = e.date.toUtc().add(const Duration(milliseconds: 1));
      if (after.isAfter(date)) date = after;
    }
    batch(() {
      for (final field in fields) {
        append(conflict.entity, field, kept.values[field], date: date);
      }
    });
  }
}
