import 'dart:convert';

/// The version of the entries' structure this build applies (ADR 0019):
/// an entry with a higher `v` is kept and passed on, never applied.
const entryVersion = 1;

/// The keys of an entry of version 1, in the order they are written.
/// Any other key, `v` among them, was added by a later version: it is
/// written after these, sorted, signed with the entry and passed on as
/// it came (ADR 0019).
const entryKeys = [
  'device',
  'dseq',
  'entity',
  'field',
  'value',
  'date',
  'author',
  'recorded',
  'ctx',
  'sig',
];

/// One immutable fact in the append-only log (ADR 0002).
///
/// Entries are never updated or removed. The current value of a field is
/// the entry with the highest ([date], [recorded], [device], [dseq]) for
/// its (entity, field) pair.
class Entry {
  /// Local insertion order — a local handle only. Differs between
  /// Devices; never used for cross-device ordering.
  final int seq;

  /// Origin device id. Together with [dseq] globally unique and the
  /// basis of delta sync.
  final String device;

  /// Per-origin-device monotonic counter.
  final int dseq;

  /// Entity id, e.g. `note:<uuid>`.
  final String entity;

  /// Field key, e.g. `text`, `$type`.
  final String field;

  /// The value as a string; `null` clears the field.
  final String? value;

  /// Effective date of the fact.
  final DateTime date;

  /// Author name the recording Device was configured with.
  final String author;

  /// Wall clock at recording time; tiebreak after [date].
  final DateTime recorded;

  /// The writing Device's signature over the row (base64).
  final String? sig;

  /// The causal context of an edit to a field concurrent edits are a
  /// Conflict on ([conflictUnit]): how far the writer had seen each other
  /// Device that wrote the field, as `<device>:<dseq>` pairs by device,
  /// space-separated; empty when only the writer itself had. Null for the
  /// first write of such a field, for every other field, and for entries
  /// written before contexts were kept.
  final String? ctx;

  /// The keys beyond [entryKeys] the entry came with, `v` among them
  /// when it was written: signed with the entry and passed on as they
  /// are. Empty for every entry of version 1.
  final Map<String, Object?> extra;

  /// The entry's JSON as it came, kept when it is not what this version
  /// writes for it (any [extra] key, or bytes written otherwise): passed
  /// on byte for byte, so its signature holds wherever it goes.
  final String? raw;

  /// Identity on the wire.
  String get id => '$device:$dseq';

  const Entry({
    required this.seq,
    required this.device,
    required this.dseq,
    required this.entity,
    required this.field,
    required this.value,
    required this.date,
    required this.author,
    required this.recorded,
    this.sig,
    this.ctx,
    this.extra = const {},
    this.raw,
  });

  /// The version of the entry's structure: `v`, 1 when missing; 1 is
  /// never written.
  int get v => switch (extra['v']) {
    final int v => v,
    _ => 1,
  };

  /// Whether this version applies the entry: one of a later structure is
  /// only kept and passed on.
  bool get applies => v <= entryVersion;

  @override
  String toString() =>
      'Entry($seq $entity $field=$value @${date.toIso8601String()} by $author)';

  /// Wire format for sync transports. `seq` is intentionally absent —
  /// it is a local handle; identity on the wire is (device, dseq). An
  /// entry kept as it came ([raw]) goes out as it came.
  Map<String, dynamic> toJson() {
    final raw = this.raw;
    if (raw != null) return (jsonDecode(raw) as Map).cast<String, dynamic>();
    return _known();
  }

  /// The entry's JSON line in a segment, a bundle or a Backup: [raw]
  /// itself when kept.
  String get wire => raw ?? jsonEncode(_known());

  Map<String, dynamic> _known() => {
    'device': device,
    'dseq': dseq,
    'entity': entity,
    'field': field,
    'value': value,
    'date': date.toIso8601String(),
    'author': author,
    'recorded': recorded.toIso8601String(),
    if (ctx != null) 'ctx': ctx,
    if (sig != null) 'sig': sig,
  };

  /// The entry [json] holds. [text] is its JSON as it came, where the
  /// bytes are at hand (a segment, a bundle, a Backup); an entry from a
  /// LAN body is taken in its canonical form ([canonicalEntry]). A `v`
  /// that is no whole number from 1 makes it no entry: a
  /// [FormatException].
  factory Entry.fromJson(Map<String, dynamic> json, {String? text}) {
    final extra = extraOf(json);
    if (extra.containsKey('v') && !_isVersion(extra['v'])) {
      throw const FormatException('An entry version is a whole number from 1');
    }
    final entry = Entry(
      seq: -1,
      device: json['device'] as String,
      dseq: json['dseq'] as int,
      entity: json['entity'] as String,
      field: json['field'] as String,
      value: json['value'] as String?,
      date: DateTime.parse(json['date'] as String),
      author: json['author'] as String,
      recorded: DateTime.parse(json['recorded'] as String),
      sig: json['sig'] as String?,
      ctx: json['ctx'] as String?,
    );
    final came = text ?? canonicalEntry(json);
    if (extra.isEmpty && came == entry.wire) return entry;
    return entry._as(extra, came);
  }

  /// This entry with [extra] and kept as [raw].
  Entry _as(Map<String, Object?> extra, String raw) => Entry(
    seq: seq,
    device: device,
    dseq: dseq,
    entity: entity,
    field: field,
    value: value,
    date: date,
    author: author,
    recorded: recorded,
    sig: sig,
    ctx: ctx,
    extra: extra,
    raw: raw,
  );

  /// This entry as the store read it, with what its kept [raw] carries.
  Entry withRaw(String? raw) {
    if (raw == null) return this;
    final json = jsonDecode(raw);
    return _as(json is Map ? extraOf(json.cast<String, dynamic>()) : {}, raw);
  }
}

bool _isVersion(Object? v) => v is int && v >= 1;

/// The keys of [json] beyond [entryKeys], sorted.
Map<String, Object?> extraOf(Map<String, dynamic> json) => {
  for (final key
      in json.keys.where((k) => !entryKeys.contains(k)).toList()..sort())
    key: json[key],
};

/// An entry's JSON in its canonical form (ADR 0019): the keys of
/// [entryKeys] in their order, then any other key sorted, the keys of
/// objects inside sorted too, no spaces. Both cores write entries so.
String canonicalEntry(Map<String, dynamic> json) => jsonEncode({
  for (final key in entryKeys)
    if (json.containsKey(key)) key: json[key],
  for (final MapEntry(:key, :value) in extraOf(json).entries)
    key: sortedJson(value),
});

/// [value] with the keys of every object in it sorted, as the desktop
/// core's JSON values keep them.
Object? sortedJson(Object? value) => switch (value) {
  final Map m => {
    for (final key in m.keys.cast<String>().toList()..sort())
      key: sortedJson(m[key]),
  },
  final List l => [for (final e in l) sortedJson(e)],
  _ => value,
};
