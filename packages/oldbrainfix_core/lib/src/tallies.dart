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

  /// The id of the other field a field multiplied (#200), as stored:
  /// no app writes it any more, but a field it names still counts the
  /// sets of the older entries (#264).
  static const multiplies = 'multiplies';

  /// A field's kind: [kindCheck] for a Check field, absent for a number.
  static const kind = 'kind';
  static const kindCheck = 'check';

  /// The length of a bar of the Tally's charts, a [TallyPeriod]'s name,
  /// on the Tally's Note; absent or unknown reads as a day (#264).
  static const bar = 'tally:bar';

  /// Whether the Tally counts in sets, `true` or `false`, on the Tally's
  /// Note; absent, it does while a field counts sets as a multiplier did
  /// (#264).
  static const sets = 'tally:sets';

  /// How many sets an entry holds at most.
  static const maxSets = 99;

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

  /// The value of a field in the set [set] of an entry, counting from 1:
  /// the first under `v:<field id>`, each further one under
  /// `v<set>:<field id>` (#264).
  static String setValue(int set, String fieldId) =>
      set == 1 ? value(fieldId) : 'v$set:$fieldId';

  /// The set and the field id [key] holds a value of, or null: `v:` for
  /// the first, `v2:` to `v99:`, without a leading zero, for the others.
  static (int, String)? valueOf(String key) {
    if (key.startsWith(valuePrefix)) {
      return (1, key.substring(valuePrefix.length));
    }
    final match = _setKey.firstMatch(key);
    if (match == null) return null;
    final (set, id) = (match.group(1), match.group(2));
    if (set == null || id == null) return null;
    return (int.parse(set), id);
  }

  static final _setKey = RegExp(r'^v([2-9]|[1-9][0-9]):(.+)$');

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

  /// The target in the base unit, per bar of the Tally's charts, or null
  /// for none; a field that counts sets has none.
  final double? target;
  final TallyRule rule;
  final bool hidden;

  /// Whether it is a Check field (#264): a box per entry, a tick counting
  /// as 1, summed.
  final bool check;

  /// The id of the field this one multiplied (#200), or null: such a
  /// field counts the sets of the entries that hold no sets of their own
  /// (#264) and counts no value of its own.
  final String? multiplies;

  /// The id of the field its setting names, read or not.
  final String? _stored;

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
    this.check = false,
    this.multiplies,
    this._stored,
  });

  UnitKind get kind => TallyUnits.kindOf(unit);

  /// This field with [hasEntries] as read.
  TallyField _counting(bool hasEntries) => TallyField(
    id: id,
    noteId: noteId,
    name: name,
    unit: unit,
    target: target,
    rule: rule,
    hidden: hidden,
    hasEntries: hasEntries,
    added: added,
    check: check,
    multiplies: multiplies,
    stored: _stored,
  );

  /// This field with [multiplies] as read, without its target while it
  /// counts sets.
  TallyField _multiplying(String? multiplies) => TallyField(
    id: id,
    noteId: noteId,
    name: name,
    unit: unit,
    target: multiplies == null ? target : null,
    rule: rule,
    hidden: hidden,
    hasEntries: hasEntries,
    added: added,
    check: check,
    multiplies: multiplies,
    stored: _stored,
  );
}

/// A Note that records how much was done (CONTEXT.md).
class Tally {
  final String noteId;

  /// Every field, the hidden ones included, in the order they were added.
  final List<TallyField> fields;

  /// How long a bar of its charts is; its targets count per bar (#264).
  final TallyPeriod bar;

  /// Whether its entry form has "+ Set" (#264).
  final bool countsInSets;

  const Tally(
    this.noteId,
    this.fields, {
    this.bar = TallyPeriod.day,
    this.countsInSets = false,
  });

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

  /// The fields shown that count values of their own: all but those that
  /// count sets as a multiplier did (#264). Totals, targets and charts
  /// are theirs.
  List<TallyField> get counting => [
    for (final field in shown)
      if (field.multiplies == null) field,
  ];

  /// The number fields of [counting]: a set is a row across them.
  List<TallyField> get numbers => [
    for (final field in counting)
      if (!field.check) field,
  ];

  /// The Check fields of [counting]: ticked once per entry, above its
  /// sets.
  List<TallyField> get checks => [
    for (final field in counting)
      if (field.check) field,
  ];

  /// What the entry form starts with: the target of every number field
  /// shown that has one, by field id; a Check field starts unticked.
  Map<String, double> get targets => {
    for (final field in shown)
      if (!field.check) field.id: ?field.target,
  };

  /// The sets of [entry] (#264), each a row of values by field id across
  /// its number fields, hidden ones included, in the base unit; none when
  /// it holds no number. An entry without sets of its own written that a
  /// field counting sets holds a value for reads as that many sets of its
  /// values: the first such field in their order, its value rounded half
  /// away from zero, at least 1 and at most [TallyKeys.maxSets].
  List<Map<String, double>> setsOf(TallyEntry entry) {
    Map<String, double> row(Map<String, double> values) => {
      for (final field in fields)
        if (!field.check && field.multiplies == null)
          field.id: ?values[field.id],
    };
    final first = row(entry.values);
    final rows = [
      if (first.isNotEmpty) first,
      for (final more in entry.more)
        if (row(more) case final held when held.isNotEmpty) held,
    ];
    if (entry.more.isNotEmpty || first.isEmpty) return rows;
    for (final field in fields) {
      if (field.multiplies == null) continue;
      final micro = switch (entry.values[field.id]) {
        final value? => tallyMicro(value),
        null => null,
      };
      if (micro == null) continue;
      final whole = _round(micro);
      final count = whole < 1
          ? 1
          : whole > TallyKeys.maxSets
          ? TallyKeys.maxSets
          : whole;
      return [for (var i = 0; i < count; i++) first];
    }
    return rows;
  }

  /// What [entry] counts for the field [id]: a Check field its tick, a
  /// number field its rule over the entry's sets, the sum or the largest
  /// (#264); null for a field counting sets or a value it does not hold.
  double? counted(TallyEntry entry, String id) {
    final micro = countedMicro(entry, id);
    return micro == null ? null : micro / 1000000;
  }

  /// [counted] in millionths, sums held at the 64-bit limits, as in both
  /// cores.
  int? countedMicro(TallyEntry entry, String id) {
    final field = this.field(id);
    if (field == null || field.multiplies != null) return null;
    if (field.check) {
      final value = entry.values[id];
      return value == null ? null : tallyMicro(value);
    }
    return Tallies._fold([
      for (final set in setsOf(entry))
        if (set[id] case final value?) ?tallyMicro(value),
    ], field.rule);
  }

  /// Whether the setting of [field] names a shown field other than it:
  /// such a field is never multiplied, so no chain forms.
  bool _wouldMultiply(TallyField field) => _named(field) != null;

  /// The shown field other than [field] its setting names, if any.
  TallyField? _named(TallyField field) {
    final other = switch (field._stored) {
      final id? => this.field(id),
      null => null,
    };
    return !field.hidden &&
            other != null &&
            other.id != field.id &&
            !other.hidden
        ? other
        : null;
  }

  /// [micro] millionths as a whole number, rounded half away from zero.
  static int _round(int micro) {
    final whole = micro ~/ 1000000;
    final rest = (micro - whole * 1000000).abs();
    if (rest * 2 < 1000000) return whole;
    return micro < 0 ? whole - 1 : whole + 1;
  }
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

  /// The values by field id, in the base unit, of its first set and its
  /// Check fields; a field added later has none.
  final Map<String, double> values;

  /// The values of its further sets (#264), in their order, each by field
  /// id in the base unit; as stored, read as [Tally.setsOf] says.
  final List<Map<String, double>> more;
  final String line;

  /// When it was removed, or null.
  final DateTime? trashed;

  const TallyEntry({
    required this.id,
    required this.noteId,
    required this.at,
    required this.values,
    required this.line,
    this.more = const [],
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
  /// into it, or null while it has no field. Its bar length and whether
  /// it counts in sets lie on its own Note.
  Tally? byNote(String noteId) {
    final shape = _shapes()[noteId];
    if (shape == null) return null;
    final counted = _counted()[noteId] ?? const <String>{};
    final fields = _multiplied(
      Tally(noteId, [
        for (final field in shape.fields)
          field._counting(counted.contains(field.id)),
      ]),
    );
    return Tally(
      noteId,
      fields,
      bar: TallyPeriod.values.asNameMap()[shape.bar] ?? TallyPeriod.day,
      countsInSets: switch (shape.sets) {
        'true' => true,
        'false' => false,
        _ => fields.any((f) => f.multiplies != null),
      },
    );
  }

  /// The fields of [tally] with the multipliers its settings make (#200):
  /// a field whose setting names a shown other field multiplied it, unless
  /// that field's own setting names a shown other field too. A hidden
  /// field multiplies nothing and is multiplied by nothing. Such a field
  /// now counts sets (#264).
  static List<TallyField> _multiplied(Tally tally) => [
    for (final field in tally.fields)
      field._multiplying(switch (tally._named(field)) {
        final other? when !tally._wouldMultiply(other) => other.id,
        _ => null,
      }),
  ];

  /// Whether the Note [noteId] is a Tally: told by its fields alone,
  /// without reading its entries (#216).
  bool isTally(String noteId) => _shapes().containsKey(noteId);

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
  /// is in the base unit, per bar; [rule] defaults by the kind of [unit].
  /// A [check] field (#264) has no unit and sums its ticks, and stays a
  /// Check field. A Note that is a list, such as a Checklist, is never a
  /// Tally too.
  String addField(
    String noteId, {
    required String name,
    String unit = '',
    double? target,
    TallyRule? rule,
    bool check = false,
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
    final typed = check ? '' : unit.trim();
    final goal = target == null ? null : encodeTallyNumber(target);
    final id = Store.newId();
    store.batch(() {
      store.append(noteId, TallyKeys.field(id, TallyKeys.name), named);
      store.append(noteId, TallyKeys.field(id, TallyKeys.unit), typed);
      store.append(
        noteId,
        TallyKeys.field(id, TallyKeys.rule),
        (check
                ? TallyRule.sum
                : rule ?? TallyUnits.defaultRule(TallyUnits.kindOf(typed)))
            .name,
      );
      if (goal != null) {
        store.append(noteId, TallyKeys.field(id, TallyKeys.target), goal);
      }
      if (check) {
        store.append(
          noteId,
          TallyKeys.field(id, TallyKeys.kind),
          TallyKeys.kindCheck,
        );
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
  /// while no entry holds a value for it; a Check field has none.
  void setUnit(String noteId, String fieldId, String unit) {
    final field = _requireField(noteId, fieldId);
    if (field.check) throw StateError('$fieldId is a Check field');
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

  /// Sets how the field [fieldId] counts the sets of an entry and a bar
  /// with several entries; a Check field always sums.
  void setRule(String noteId, String fieldId, TallyRule rule) {
    if (_requireField(noteId, fieldId).check) {
      throw StateError('$fieldId is a Check field');
    }
    _set(noteId, fieldId, TallyKeys.rule, rule.name);
  }

  /// Sets how long a bar of the charts of the Tally of [noteId] is
  /// (#264), on its Note; its targets count per bar.
  void setBar(String noteId, TallyPeriod bar) =>
      _setOnTally(noteId, TallyKeys.bar, bar.name);

  /// Switches whether the Tally of [noteId] counts in sets (#264): its
  /// entry form has "+ Set" then. The entries keep their sets either way.
  void setCountsInSets(String noteId, bool on) =>
      _setOnTally(noteId, TallyKeys.sets, on ? 'true' : 'false');

  /// Writes [value] under [key] on the Note of the Tally [noteId], unless
  /// it holds that already.
  void _setOnTally(String noteId, String key, String value) {
    final note = Notes(store).byId(noteId);
    if (byNote(noteId) == null ||
        note == null ||
        note.isTrashed ||
        note.isMerged) {
      throw ArgumentError.value(noteId, 'noteId', 'not a Tally in the lists');
    }
    if (!store.mayChange(noteId)) {
      throw StateError('may not change $noteId');
    }
    if (store.current(noteId, key) == value) return;
    store.append(noteId, key, value);
  }

  /// Hides the field [fieldId]: its values stay stored and return when
  /// it is restored.
  void hideField(String noteId, String fieldId) =>
      _set(noteId, fieldId, TallyKeys.hidden, 'true');

  /// Shows the hidden field [fieldId] again, with its values.
  void restoreField(String noteId, String fieldId) =>
      _set(noteId, fieldId, TallyKeys.hidden, null);

  /// Records an entry in the Tally of [noteId]: [values] by field id, in
  /// the base unit, its first set and its Check fields' ticks (1), the
  /// further sets [more] (#264), each by number field id, and a text
  /// [line], counting for [at] or the store's now. A set that holds
  /// nothing is left out. Anyone with a role in its Area may. [within]
  /// writes more in the same batch, given the entry's id.
  TallyEntry add(
    String noteId, {
    Map<String, double> values = const {},
    List<Map<String, double>> more = const [],
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
    final sets = _encodedSets(tally, more);
    final text = line.trim();
    if (encoded.values.every((v) => v == null) &&
        sets.isEmpty &&
        text.isEmpty) {
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
      for (final (i, set) in sets.indexed) {
        for (final MapEntry(:key, :value) in set.entries) {
          store.append(id, TallyKeys.setValue(i + 2, key), value, date: now);
        }
      }
      if (text.isNotEmpty) {
        store.append(id, TallyKeys.line, text, date: now);
      }
      within?.call(id);
    });
    return entryById(id) ??
        (throw StateError('$id does not read back after it was written'));
  }

  /// Changes the entry [id]: the values given (null takes one away), the
  /// further sets when [more] is given (#264), which then replace those
  /// it held and take away the values of the fields that counted its
  /// sets as multipliers, the text line and when it counts for. Only what
  /// differs is written; only its author, or a Manager of its Area, may.
  void editEntry(
    String id, {
    Map<String, double?> values = const {},
    List<Map<String, double>>? more,
    String? line,
    DateTime? at,
  }) {
    final entry = _requireEntry(id);
    final tally = byNote(_shownUnder()[entry.noteId] ?? entry.noteId);
    if (tally == null) throw ArgumentError.value(id, 'id', 'not a Tally');
    final encoded = _encoded(tally, values);
    final sets = more == null ? null : _encodedSets(tally, more);
    store.batch(() {
      if (at != null && !at.isAtSameMomentAs(entry.at)) {
        store.append(id, TallyKeys.at, at.toUtc().toIso8601String());
      }
      for (final MapEntry(:key, :value) in encoded.entries) {
        final held = entry.values[key];
        if (value == (held == null ? null : encodeTallyNumber(held))) continue;
        store.append(id, TallyKeys.value(key), value);
      }
      if (sets != null) {
        for (final field in tally.fields) {
          if (field.multiplies != null &&
              !encoded.containsKey(field.id) &&
              entry.values.containsKey(field.id)) {
            store.append(id, TallyKeys.value(field.id), null);
          }
        }
        final held = store.currentFields(id);
        for (var set = 2; set <= TallyKeys.maxSets; set++) {
          for (final field in tally.fields) {
            final key = TallyKeys.setValue(set, field.id);
            final want = set - 2 < sets.length ? sets[set - 2][field.id] : null;
            if (held[key] == want) continue;
            store.append(id, key, want);
          }
        }
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
  TallyEntry? entryById(String id) => _entryFrom(id, store.currentFields(id));

  /// The entries of [ids], removed or not, by id, read in one query
  /// (#204); what is no entry is left out.
  Map<String, TallyEntry> entriesById(Iterable<String> ids) => {
    for (final MapEntry(key: id, value: fields)
        in store.currentFieldsOf(ids).entries)
      id: ?_entryFrom(id, fields),
  };

  /// The entry [id] its current [fields] hold, or null.
  static TallyEntry? _entryFrom(String id, Map<String, String?> fields) {
    if (fields[TallyKeys.type] != TallyKeys.typeEntry) return null;
    final noteId = fields[TallyKeys.note];
    final at = DateTime.tryParse(fields[TallyKeys.at] ?? '');
    if (noteId == null || at == null) return null;
    final values = <String, double>{};
    final more = <int, Map<String, double>>{};
    for (final MapEntry(:key, :value) in fields.entries) {
      final (set, field) = TallyKeys.valueOf(key) ?? (0, '');
      final number = decodeTallyNumber(value);
      if (set == 0 || number == null) continue;
      (set == 1 ? values : more.putIfAbsent(set, () => {}))[field] = number;
    }
    return TallyEntry(
      id: id,
      noteId: noteId,
      at: at.toUtc(),
      values: values,
      more: [for (final set in more.keys.toList()..sort()) ?more[set]],
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
  /// per the Tally's bar length, or [period] when given, on [wallClock],
  /// weeks from Monday, from the first with a value to today's, at most
  /// [maxBuckets] of the latest. Each bar holds the sum or the largest of
  /// what its entries count ([Tally.counted]), by the field's rule; a
  /// Check field sums its ticks, a field counting sets has none. Entries
  /// after today stay out.
  List<TallyBucket> chart(
    String noteId,
    String fieldId,
    WallClock wallClock, {
    TallyPeriod? period,
    PersonFilter filter = PersonFilter.everyone,
  }) {
    final tally = byNote(noteId);
    final field = tally?.field(fieldId);
    if (tally == null || field == null) return const [];
    period ??= tally.bar;
    final today = Day.today(store.clock(), wallClock);
    final last = _bucketStart(today, period);
    final micros = <Day, List<int>>{};
    for (final entry in entries(noteId, filter: filter)) {
      final micro = tally.countedMicro(entry, fieldId);
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
          value: switch (_fold(micros[start] ?? const [], field.rule)) {
            final micro? => micro / 1000000,
            null => null,
          },
          entries: micros[start]?.length ?? 0,
        ),
    ];
  }

  /// What [rule] makes of [micros], or null for none.
  static int? _fold(List<int> micros, TallyRule rule) {
    if (micros.isEmpty) return null;
    var held = micros.first;
    for (final micro in micros.skip(1)) {
      held = switch (rule) {
        TallyRule.sum => _add(held, micro),
        TallyRule.max => micro > held ? micro : held,
      };
    }
    return held;
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
    for (final MapEntry(key: id, :value) in values.entries) {
      final field = tally.field(id);
      if (field == null) {
        throw ArgumentError.value(id, 'values', 'not a field of the Tally');
      }
      if (field.check && value != null && value != 1) {
        throw ArgumentError.value(value, 'values', 'a tick counts as 1');
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

  /// The further sets [more] as the log keeps them, each by number field
  /// in the order of the fields of [tally], those that hold nothing left
  /// out; a value for a Check field, a field counting sets or no field of
  /// it is refused, as are more than [TallyKeys.maxSets] sets in all.
  static List<Map<String, String>> _encodedSets(
    Tally tally,
    List<Map<String, double>> more,
  ) {
    final sets = <Map<String, String>>[];
    for (final set in more) {
      for (final id in set.keys) {
        final field = tally.field(id);
        if (field == null || field.check || field.multiplies != null) {
          throw ArgumentError.value(id, 'more', 'not a number field of it');
        }
      }
      final encoded = {
        for (final field in tally.fields)
          if (set[field.id] case final value?)
            field.id: encodeTallyNumber(value),
      };
      if (encoded.isNotEmpty) sets.add(encoded);
    }
    if (sets.length >= TallyKeys.maxSets) {
      throw ArgumentError.value(more, 'more', 'too many sets');
    }
    return sets;
  }

  /// The fields of every Tally, by its Note, in the order they were
  /// added, each as if it had no entries: read from the Notes alone, not
  /// from the entries, and kept until the log changes (#216). Shared by
  /// every caller, so nobody changes it.
  Map<String, ({List<TallyField> fields, String? bar, String? sets})>
  _shapes() => store.workedOut((Tallies, #shapes), () {
    final holders = <String, List<String>>{};
    for (final MapEntry(key: holder, value: note) in _shownUnder().entries) {
      (holders[note] ??= []).add(holder);
    }
    final shapes =
        <String, ({List<TallyField> fields, String? bar, String? sets})>{};
    for (final MapEntry(key: note, value: on) in holders.entries) {
      final fields = <TallyField>[];
      String? bar;
      String? sets;
      for (final holder in on..sort()) {
        final current = store.currentFields(holder);
        fields.addAll(_fieldsOn(holder, current));
        if (holder == note) {
          bar = current[TallyKeys.bar];
          sets = current[TallyKeys.sets];
        }
      }
      if (fields.isEmpty) continue;
      fields.sort((a, b) {
        final byAdded = a.added.compareTo(b.added);
        return byAdded != 0 ? byAdded : a.id.compareTo(b.id);
      });
      shapes[note] = (fields: fields, bar: bar, sets: sets);
    }
    return shapes;
  });

  /// The fields lying on the Note [holder], as if none had entries, its
  /// current [fields] given.
  List<TallyField> _fieldsOn(String holder, Map<String, String?> fields) {
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
          _readField(holder, id, name, date, fields),
    ];
  }

  /// The field [id] on [holder], named [name] and added at [date], as its
  /// current [fields] hold it; a Check field always sums.
  static TallyField _readField(
    String holder,
    String id,
    String name,
    DateTime date,
    Map<String, String?> fields,
  ) {
    final check =
        fields[TallyKeys.field(id, TallyKeys.kind)] == TallyKeys.kindCheck;
    return TallyField(
      id: id,
      noteId: holder,
      name: name,
      unit: fields[TallyKeys.field(id, TallyKeys.unit)] ?? '',
      target: decodeTallyNumber(fields[TallyKeys.field(id, TallyKeys.target)]),
      rule: check
          ? TallyRule.sum
          : TallyRule.values.asNameMap()[fields[TallyKeys.field(
                  id,
                  TallyKeys.rule,
                )]] ??
                TallyRule.sum,
      hidden: fields[TallyKeys.field(id, TallyKeys.hidden)] == 'true',
      check: check,
      hasEntries: false,
      added: date.toUtc(),
      stored: fields[TallyKeys.field(id, TallyKeys.multiplies)],
    );
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

  /// The ids of the fields some entry holds a value for, removed entries
  /// included, by the Note of the Tally the entry shows under: every
  /// entry read in one query, kept until the log changes (#216). Shared
  /// by every caller, so nobody changes it.
  Map<String, Set<String>> _counted() =>
      store.workedOut((Tallies, #counted), () {
        final shownUnder = _shownUnder();
        final counted = <String, Set<String>>{};
        for (final MapEntry(key: id, value: fields)
            in store.currentFieldsOfType(TallyKeys.typeEntry).entries) {
          if (_entryFrom(id, fields) case final entry?) {
            (counted[shownUnder[entry.noteId] ?? entry.noteId] ??= {})
              ..addAll(entry.values.keys)
              ..addAll([for (final set in entry.more) ...set.keys]);
          }
        }
        return counted;
      });

  /// Every Note with a field of a Tally on it, by the Note it shows
  /// under: itself, or the one it was merged into. Kept until the log
  /// changes (#216) and shared by every caller, so nobody changes it.
  Map<String, String> _shownUnder() => store.workedOut(
    (Tallies, #shownUnder),
    () {
      final notes = Notes(store);
      final under = <String, String>{};
      for (final (entity, _) in store.fieldsWithPrefix(TallyKeys.fieldPrefix)) {
        if (under.containsKey(entity)) continue;
        final note = notes.byId(entity);
        if (note != null) under[entity] = note.mergedInto ?? entity;
      }
      return under;
    },
  );

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
