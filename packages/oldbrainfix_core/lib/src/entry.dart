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
  });

  @override
  String toString() =>
      'Entry($seq $entity $field=$value @${date.toIso8601String()} by $author)';

  /// Wire format for sync transports. `seq` is intentionally absent —
  /// it is a local handle; identity on the wire is (device, dseq).
  Map<String, dynamic> toJson() => {
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

  factory Entry.fromJson(Map<String, dynamic> json) => Entry(
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
}
