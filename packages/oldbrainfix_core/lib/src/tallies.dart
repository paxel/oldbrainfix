import 'checklists.dart';
import 'notes.dart';
import 'profiles.dart';
import 'schedule.dart';
import 'shared_areas.dart';
import 'store.dart';
import 'tally_units.dart';

/// Field keys of a Tally (ADR 0028). Its fields lie on its Note, each
/// under `tally:<field id>:<key>`; its entries are entities of their own.
abstract final class TallyKeys {
  /// What every key of a field on the Note starts with.
  static const fieldPrefix = 'tally:';

  /// A field's name, unit, target, rule and whether it is hidden.
  static const name = 'name';
  static const unit = 'unit';
  static const target = 'target';
  static const rule = 'rule';
  static const hidden = 'hidden';

  /// The key of [key] of the field [fieldId] on the Note.
  static String field(String fieldId, String key) =>
      '$fieldPrefix$fieldId:$key';

  static const type = r'$type';
  static const typeEntry = 'tallyEntry';

  /// The id prefix of an entry.
  static const entryPrefix = 'tallyEntry:';

  /// The Note of the Tally the entry belongs to.
  static const note = 'note';

  /// When the entry counts for, an instant.
  static const at = 'at';

  /// The value of a field, under `v:<field id>`, in the base unit.
  static const valuePrefix = 'v:';

  static String value(String fieldId) => '$valuePrefix$fieldId';

  /// The entry's free text line.
  static const line = 'line';

  /// When the entry was removed; absent while it is not.
  static const trashed = 'trashed';
}

/// One number a Tally records per entry, with its unit (CONTEXT.md).
class TallyField {
  final String id;

  /// The Note the field lies on: the Tally's own, or one merged into it.
  final String noteId;
  final String name;

  /// A known unit's code ([TallyUnits.known]) or the unit as typed.
  final String unit;

  /// The target in the base unit, or null for none.
  final double? target;
  final TallyRule rule;
  final bool hidden;

  /// Whether an entry holds a value for it, a removed one included: its
  /// kind of unit is fixed then.
  final bool hasEntries;

  /// When it was added: the fields show in that order.
  final DateTime added;

  const TallyField({
    required this.id,
    required this.noteId,
    required this.name,
    required this.unit,
    required this.target,
    required this.rule,
    required this.hidden,
    required this.hasEntries,
    required this.added,
  });

  UnitKind get kind => TallyUnits.kindOf(unit);
}

/// A Note that records how much was done (CONTEXT.md).
class Tally {
  final String noteId;

  /// Every field, the hidden ones included, in the order they were added.
  final List<TallyField> fields;

  const Tally(this.noteId, this.fields);

  /// The fields that show: those not hidden.
  List<TallyField> get shown => [
    for (final field in fields)
      if (!field.hidden) field,
  ];

  /// The field [id], hidden or not, or null.
  TallyField? field(String id) {
    for (final field in fields) {
      if (field.id == id) return field;
    }
    return null;
  }

  /// What the entry form starts with: the target of every field shown
  /// that has one, by field id.
  Map<String, double> get targets => {
    for (final field in shown) field.id: ?field.target,
  };
}

/// One record of a Tally: values for its fields, a date, a text line and
/// its author, who is the Device that wrote its first `$type`
/// ([People.authorOf]).
class TallyEntry {
  /// Entity id, `tallyEntry:<id>`.
  final String id;

  /// The Note of the Tally, as stored.
  final String noteId;

  /// When it counts for.
  final DateTime at;

  /// The values by field id, in the base unit; a field added later has
  /// none.
  final Map<String, double> values;
  final String line;

  /// When it was removed, or null.
  final DateTime? trashed;

  const TallyEntry({
    required this.id,
    required this.noteId,
    required this.at,
    required this.values,
    required this.line,
    this.trashed,
  });

  bool get isTrashed => trashed != null;
}

/// How long one bar of a chart is.
enum TallyPeriod { day, week, month, year }

/// One bar of a field's chart: the days [start] to [end] and what the
/// field's rule makes of the values in them.
class TallyBucket {
  final Day start;
  final Day end;

  /// The sum or the largest of the values, in the base unit; null when
  /// no entry holds one.
  final double? value;

  /// How many entries hold a value.
  final int entries;

  const TallyBucket({
    required this.start,
    required this.end,
    required this.value,
    required this.entries,
  });
}

/// Making Notes Tallies, changing their fields, recording entries and
/// reading them as tables and charts.
class Tallies {
  final Store store;

  Tallies(this.store);

  /// How many bars a chart holds at most: the latest ones.
  static const maxBuckets = 1000;

  /// The Tally of the Note [noteId], with the fields of the Notes merged
  /// into it, or null while it has no field.
  Tally? byNote(String noteId) {
    final holders = [
      for (final MapEntry(:key, :value) in _shownUnder().entries)
        if (value == noteId) key,
    ]..sort();
    if (holders.isEmpty) return null;
    final counted = _counted(noteId);
    final fields = [
      for (final holder in holders) ..._fieldsOn(holder, counted),
    ];
    if (fields.isEmpty) return null;
    fields.sort((a, b) {
      final byAdded = a.added.compareTo(b.added);
      return byAdded != 0 ? byAdded : a.id.compareTo(b.id);
    });
    return Tally(noteId, fields);
  }

  /// Whether the Note [noteId] is a Tally.
  bool isTally(String noteId) => byNote(noteId) != null;

  /// Every Tally whose Note is in the lists, newest capture first.
  List<Tally> all() {
    final notes = {..._shownUnder().values};
    return [
      for (final note in Notes(store).all())
        if (notes.contains(note.id)) ?byNote(note.id),
    ];
  }

  /// Adds a field to the Note [noteId], which makes it a Tally the first
  /// time and takes it out of Unsorted; returns the field's id. [target]
  /// is in the base unit; [rule] defaults by the kind of [unit]. A Note
  /// that is a list, such as a Checklist, is never a Tally too.
  String addField(
    String noteId, {
    required String name,
    String unit = '',
    double? target,
    TallyRule? rule,
  }) {
    final note = Notes(store).byId(noteId);
    if (note == null || note.isTrashed || note.isMerged) {
      throw ArgumentError.value(noteId, 'noteId', 'not a Note in the lists');
    }
    if (parseListKind(store.current(noteId, ChecklistKeys.kind)) != null) {
      throw StateError('$noteId is a list');
    }
    if (!store.mayChange(noteId)) {
      throw StateError('may not change $noteId');
    }
    final named = _name(name);
    final typed = unit.trim();
    final goal = target == null ? null : encodeTallyNumber(target);
    final id = Store.newId();
    store.batch(() {
      store.append(noteId, TallyKeys.field(id, TallyKeys.name), named);
      store.append(noteId, TallyKeys.field(id, TallyKeys.unit), typed);
      store.append(
        noteId,
        TallyKeys.field(id, TallyKeys.rule),
        (rule ?? TallyUnits.defaultRule(TallyUnits.kindOf(typed))).name,
      );
      if (goal != null) {
        store.append(noteId, TallyKeys.field(id, TallyKeys.target), goal);
      }
      if (!note.spawned) {
        store.append(
          noteId,
          NoteKeys.spawned,
          store.clock().toUtc().toIso8601String(),
        );
      }
    });
    return id;
  }

  /// Renames the field [fieldId] of the Tally of [noteId].
  void renameField(String noteId, String fieldId, String name) =>
      _set(noteId, fieldId, TallyKeys.name, _name(name));

  /// Sets the unit of the field [fieldId]: to one of another kind only
  /// while no entry holds a value for it.
  void setUnit(String noteId, String fieldId, String unit) {
    final field = _requireField(noteId, fieldId);
    final typed = unit.trim();
    if (field.hasEntries && TallyUnits.kindOf(typed) != field.kind) {
      throw StateError('the kind of unit of $fieldId is fixed by its entries');
    }
    _set(noteId, fieldId, TallyKeys.unit, typed);
  }

  /// Sets the target of the field [fieldId], in the base unit; null
  /// takes it away.
  void setTarget(String noteId, String fieldId, double? target) => _set(
    noteId,
    fieldId,
    TallyKeys.target,
    target == null ? null : encodeTallyNumber(target),
  );

  /// Sets how the field [fieldId] counts a day with several entries.
  void setRule(String noteId, String fieldId, TallyRule rule) =>
      _set(noteId, fieldId, TallyKeys.rule, rule.name);

  /// Hides the field [fieldId]: its values stay stored and return when
  /// it is restored.
  void hideField(String noteId, String fieldId) =>
      _set(noteId, fieldId, TallyKeys.hidden, 'true');

  /// Shows the hidden field [fieldId] again, with its values.
  void restoreField(String noteId, String fieldId) =>
      _set(noteId, fieldId, TallyKeys.hidden, null);

  /// Records an entry in the Tally of [noteId]: [values] by field id, in
  /// the base unit, and a text [line], counting for [at] or the store's
  /// now. Anyone with a role in its Area may. [within] writes more in
  /// the same batch, given the entry's id.
  TallyEntry add(
    String noteId, {
    Map<String, double> values = const {},
    String line = '',
    DateTime? at,
    void Function(String id)? within,
  }) {
    final tally = byNote(noteId);
    final note = Notes(store).byId(noteId);
    if (tally == null || note == null || note.isTrashed || note.isMerged) {
      throw ArgumentError.value(noteId, 'noteId', 'not a Tally in the lists');
    }
    if (!store.mayWriteIn(noteId)) {
      throw StateError('no role in the Shared Area of $noteId');
    }
    final encoded = _encoded(tally, values);
    final text = line.trim();
    if (encoded.values.every((v) => v == null) && text.isEmpty) {
      throw ArgumentError.value(values, 'values', 'an entry holds something');
    }
    final id = '${TallyKeys.entryPrefix}${Store.newId()}';
    final now = store.clock().toUtc();
    store.batch(() {
      store.append(id, TallyKeys.type, TallyKeys.typeEntry, date: now);
      store.append(id, TallyKeys.note, noteId, date: now);
      store.append(
        id,
        TallyKeys.at,
        (at ?? now).toUtc().toIso8601String(),
        date: now,
      );
      for (final MapEntry(:key, :value) in encoded.entries) {
        if (value != null) {
          store.append(id, TallyKeys.value(key), value, date: now);
        }
      }
      if (text.isNotEmpty) {
        store.append(id, TallyKeys.line, text, date: now);
      }
      within?.call(id);
    });
    return entryById(id)!;
  }

  /// Changes the entry [id]: the values given (null takes one away), the
  /// text line and when it counts for. Only what differs is written; only
  /// its author, or a Manager of its Area, may.
  void editEntry(
    String id, {
    Map<String, double?> values = const {},
    String? line,
    DateTime? at,
  }) {
    final entry = _requireEntry(id);
    final tally = byNote(_shownUnder()[entry.noteId] ?? entry.noteId);
    if (tally == null) throw ArgumentError.value(id, 'id', 'not a Tally');
    final encoded = _encoded(tally, values);
    store.batch(() {
      if (at != null && !at.isAtSameMomentAs(entry.at)) {
        store.append(id, TallyKeys.at, at.toUtc().toIso8601String());
      }
      for (final MapEntry(:key, :value) in encoded.entries) {
        final held = entry.values[key];
        if (value == (held == null ? null : encodeTallyNumber(held))) continue;
        store.append(id, TallyKeys.value(key), value);
      }
      if (line != null && line.trim() != entry.line) {
        store.append(id, TallyKeys.line, line.trim());
      }
    });
  }

  /// Removes the entry [id] from the table and the charts; it can be
  /// restored.
  void trashEntry(String id) {
    _requireEntry(id);
    store.append(
      id,
      TallyKeys.trashed,
      store.clock().toUtc().toIso8601String(),
    );
  }

  /// Brings the removed entry [id] back.
  void restoreEntry(String id) {
    final entry = entryById(id);
    if (entry == null || !entry.isTrashed) {
      throw ArgumentError.value(id, 'id', 'not a removed entry');
    }
    if (!store.mayChange(id)) throw StateError('may not change $id');
    store.append(id, TallyKeys.trashed, null);
  }

  /// The entry [id], removed or not, or null.
  TallyEntry? entryById(String id) {
    final fields = store.currentFields(id);
    if (fields[TallyKeys.type] != TallyKeys.typeEntry) return null;
    final noteId = fields[TallyKeys.note];
    final at = DateTime.tryParse(fields[TallyKeys.at] ?? '');
    if (noteId == null || at == null) return null;
    return TallyEntry(
      id: id,
      noteId: noteId,
      at: at.toUtc(),
      values: {
        for (final MapEntry(:key, :value) in fields.entries)
          if (key.startsWith(TallyKeys.valuePrefix))
            key.substring(TallyKeys.valuePrefix.length): ?decodeTallyNumber(
              value,
            ),
      },
      line: fields[TallyKeys.line] ?? '',
      trashed: DateTime.tryParse(fields[TallyKeys.trashed] ?? '')?.toUtc(),
    );
  }

  /// The entries of the Tally of [noteId], those of the Notes merged into
  /// it included, newest first, without removed ones; [filter] keeps
  /// those of the user, of everyone or of one person.
  List<TallyEntry> entries(
    String noteId, {
    PersonFilter filter = PersonFilter.everyone,
  }) {
    final shownUnder = _shownUnder();
    final entries = [
      for (final entry in _entries())
        if (!entry.isTrashed &&
            (shownUnder[entry.noteId] ?? entry.noteId) == noteId &&
            store.wroteBy(entry.id, filter))
          entry,
    ];
    entries.sort(_newestFirst);
    return entries;
  }

  /// The removed entries of the Tally of [noteId], those of the Notes
  /// merged into it included, newest first, everyone's; they can be
  /// restored.
  List<TallyEntry> removedEntries(String noteId) {
    final shownUnder = _shownUnder();
    final entries = [
      for (final entry in _entries())
        if (entry.isTrashed &&
            (shownUnder[entry.noteId] ?? entry.noteId) == noteId)
          entry,
    ];
    entries.sort(_newestFirst);
    return entries;
  }

  /// The chart of the field [fieldId] of the Tally of [noteId]: one bar
  /// per [period] on [wallClock], weeks from Monday, from the first with
  /// a value to today's, at most [maxBuckets] of the latest. Each bar
  /// holds the sum or the largest of the values in it, by the field's
  /// rule; entries after today stay out.
  List<TallyBucket> chart(
    String noteId,
    String fieldId,
    TallyPeriod period,
    WallClock wallClock, {
    PersonFilter filter = PersonFilter.everyone,
  }) {
    final field = byNote(noteId)?.field(fieldId);
    if (field == null) return const [];
    final today = Day.today(store.clock(), wallClock);
    final last = _bucketStart(today, period);
    final micros = <Day, List<int>>{};
    for (final entry in entries(noteId, filter: filter)) {
      final value = entry.values[fieldId];
      final micro = value == null ? null : tallyMicro(value);
      if (micro == null) continue;
      final day = Day.today(entry.at, wallClock);
      if (day > today) continue;
      micros.putIfAbsent(_bucketStart(day, period), () => []).add(micro);
    }
    if (micros.isEmpty) return const [];
    final first = micros.keys.reduce((a, b) => a < b ? a : b);
    final starts = <Day>[];
    for (
      var start = last;
      first <= start && starts.length < maxBuckets;
      start = _previousBucket(start, period)
    ) {
      starts.add(start);
    }
    return [
      for (final start in starts.reversed)
        TallyBucket(
          start: start,
          end: _nextBucket(start, period).addDays(-1),
          value: _bar(micros[start], field.rule),
          entries: micros[start]?.length ?? 0,
        ),
    ];
  }

  /// What [rule] makes of [micros], or null for none.
  static double? _bar(List<int>? micros, TallyRule rule) {
    if (micros == null || micros.isEmpty) return null;
    var held = micros.first;
    for (final micro in micros.skip(1)) {
      held = switch (rule) {
        TallyRule.sum => _add(held, micro),
        TallyRule.max => micro > held ? micro : held,
      };
    }
    return held / 1000000;
  }

  /// [a] + [b], held at the largest and smallest 64-bit numbers.
  static int _add(int a, int b) {
    const max = 0x7fffffffffffffff;
    const min = -max - 1;
    if (b > 0 && a > max - b) return max;
    if (b < 0 && a < min - b) return min;
    return a + b;
  }

  /// The first day of the [period] [day] lies in.
  static Day _bucketStart(Day day, TallyPeriod period) => switch (period) {
    TallyPeriod.day => day,
    TallyPeriod.week => day.addDays(1 - day.weekday),
    TallyPeriod.month => Day(day.year, day.month, 1),
    TallyPeriod.year => Day(day.year, 1, 1),
  };

  /// The first day of the [period] after the one starting on [start].
  static Day _nextBucket(Day start, TallyPeriod period) => switch (period) {
    TallyPeriod.day => start.addDays(1),
    TallyPeriod.week => start.addDays(7),
    TallyPeriod.month => Day(start.year, start.month + 1, 1),
    TallyPeriod.year => Day(start.year + 1, 1, 1),
  };

  /// The first day of the [period] before the one starting on [start].
  static Day _previousBucket(Day start, TallyPeriod period) => switch (period) {
    TallyPeriod.day => start.addDays(-1),
    TallyPeriod.week => start.addDays(-7),
    TallyPeriod.month => Day(start.year, start.month - 1, 1),
    TallyPeriod.year => Day(start.year - 1, 1, 1),
  };

  /// Writes [value] under [key] of the field [fieldId], on the Note it
  /// lies on, unless it holds that already.
  void _set(String noteId, String fieldId, String key, String? value) {
    final field = _requireField(noteId, fieldId);
    if (!store.mayChange(field.noteId)) {
      throw StateError('may not change ${field.noteId}');
    }
    final name = TallyKeys.field(fieldId, key);
    if (store.current(field.noteId, name) == value) return;
    store.append(field.noteId, name, value);
  }

  /// The field [fieldId] of the Tally of [noteId].
  TallyField _requireField(String noteId, String fieldId) {
    final field = byNote(noteId)?.field(fieldId);
    if (field == null) {
      throw ArgumentError.value(fieldId, 'fieldId', 'not a field of $noteId');
    }
    return field;
  }

  /// The entry [id], not removed, which this Device may change.
  TallyEntry _requireEntry(String id) {
    final entry = entryById(id);
    if (entry == null || entry.isTrashed) {
      throw ArgumentError.value(id, 'id', 'not an entry');
    }
    if (!store.mayChange(id)) throw StateError('may not change $id');
    return entry;
  }

  /// [values] as the log keeps them, in the order of the fields of
  /// [tally]; a value for no field of it is refused.
  static Map<String, String?> _encoded(
    Tally tally,
    Map<String, double?> values,
  ) {
    for (final id in values.keys) {
      if (tally.field(id) == null) {
        throw ArgumentError.value(id, 'values', 'not a field of the Tally');
      }
    }
    return {
      for (final field in tally.fields)
        if (values.containsKey(field.id))
          field.id: switch (values[field.id]) {
            final value? => encodeTallyNumber(value),
            null => null,
          },
    };
  }

  /// The fields lying on the Note [holder], with whether [counted] holds
  /// a value for each.
  List<TallyField> _fieldsOn(String holder, Set<String> counted) {
    final fields = store.currentFields(holder);
    final added = <String, DateTime>{};
    for (final e in store.history(holder)) {
      final id = _fieldId(e.field, TallyKeys.name);
      if (id == null) continue;
      final held = added[id];
      if (held == null || e.date.isBefore(held)) added[id] = e.date;
    }
    return [
      for (final MapEntry(key: id, value: date) in added.entries)
        if (fields[TallyKeys.field(id, TallyKeys.name)] case final name?)
          TallyField(
            id: id,
            noteId: holder,
            name: name,
            unit: fields[TallyKeys.field(id, TallyKeys.unit)] ?? '',
            target: decodeTallyNumber(
              fields[TallyKeys.field(id, TallyKeys.target)],
            ),
            rule:
                TallyRule.values.asNameMap()[fields[TallyKeys.field(
                  id,
                  TallyKeys.rule,
                )]] ??
                TallyRule.sum,
            hidden: fields[TallyKeys.field(id, TallyKeys.hidden)] == 'true',
            hasEntries: counted.contains(id),
            added: date.toUtc(),
          ),
    ];
  }

  /// The field id in [key] when it is the [part] of a field on a Note.
  static String? _fieldId(String key, String part) {
    if (!key.startsWith(TallyKeys.fieldPrefix) ||
        !key.endsWith(':$part') ||
        key.length <= TallyKeys.fieldPrefix.length + part.length + 1) {
      return null;
    }
    final id = key.substring(
      TallyKeys.fieldPrefix.length,
      key.length - part.length - 1,
    );
    return id.isEmpty || id.contains(':') ? null : id;
  }

  /// The ids of the fields some entry of the Tally of [noteId] holds a
  /// value for, removed entries included.
  Set<String> _counted(String noteId) {
    final shownUnder = _shownUnder();
    return {
      for (final entry in _entries())
        if ((shownUnder[entry.noteId] ?? entry.noteId) == noteId)
          ...entry.values.keys,
    };
  }

  /// Every Note with a field of a Tally on it, by the Note it shows
  /// under: itself, or the one it was merged into.
  Map<String, String> _shownUnder() {
    final notes = Notes(store);
    final under = <String, String>{};
    for (final (entity, _) in store.fieldsWithPrefix(TallyKeys.fieldPrefix)) {
      if (under.containsKey(entity)) continue;
      final note = notes.byId(entity);
      if (note != null) under[entity] = note.mergedInto ?? entity;
    }
    return under;
  }

  /// Newest first, by id on the same moment.
  static int _newestFirst(TallyEntry a, TallyEntry b) {
    final byAt = b.at.compareTo(a.at);
    return byAt != 0 ? byAt : a.id.compareTo(b.id);
  }

  /// Every entry, removed ones included.
  List<TallyEntry> _entries() => [
    for (final id in store.entitiesOfType(TallyKeys.typeEntry)) ?entryById(id),
  ];

  static String _name(String name) {
    final trimmed = name.trim();
    if (trimmed.isEmpty) {
      throw ArgumentError.value(name, 'name', 'must not be empty');
    }
    return trimmed;
  }
}
