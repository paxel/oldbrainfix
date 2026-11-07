import 'dart:convert';
import 'dart:typed_data';

import 'blobs.dart';
import 'checklists.dart';
import 'frogs.dart';
import 'note_kinds.dart';
import 'notes.dart';
import 'places.dart';
import 'profiles.dart';
import 'routines.dart';
import 'schedule.dart';
import 'sessions.dart';
import 'shared_areas.dart';
import 'shopping_lists.dart';
import 'signing.dart';
import 'store.dart';
import 'tallies.dart';
import 'tally_units.dart';
import 'text_boxes.dart';
import 'text_format.dart';

/// Packs (#164, ADR 0043): a file of copies of Notes picked from an Area,
/// a Topic or a Note, handed over through any messenger. The receiver
/// keeps the copies as their own, in new Areas and Topics; nothing links
/// them back, nothing updates or cancels them. A Pack is no Sent Item
/// (one Note, encrypted, owned by the sender) and no Share to an app.
///
/// What a Note brings is what it is: its text and Sections, its kind
/// (the Lines of a Checklist unticked, a Shopping List's open with their
/// reappear times, a Tally's fields, a Session's pool, counts and rule),
/// its Routines and Reminders with their schedules, an open Frog, and
/// those of its photos, recordings, Photo Texts and Transcripts that are
/// ticked. Never a Tally's entries, ticks, check-offs, its Capture
/// Context or its history.
///
/// The file is one JSON object, keys sorted and no spaces, the same
/// bytes from both cores:
/// `{"pack":<payload>,"sig":<signature>,"type":"oldbrainfix-pack"}`. `pack` is the payload as a string, `sig`
/// is there only when the payload carries the sender's Profile card
/// ("Add my Profile"): the signature of the Device that made the card
/// over [packBytes] of the payload. A later format writes `format`; only
/// an absent one or the whole number 1 is this version's (ADR 0034).
const packExtension = 'obfpack';

/// The type a Pack is shared as.
const packMimeType = 'application/vnd.oldbrainfix.pack';

/// What a Pack file says it is.
const _packType = 'oldbrainfix-pack';

/// What the field of a Note a Pack brought, kept with "Claim as mine"
/// unticked, holds: `{"id":<Profile id>,"name":<name>}`, the sender the
/// "From …" badge names. Null once the receiver removed the badge.
abstract final class PackKeys {
  static const from = 'packFrom';
}

/// The bytes a Pack's signature covers: the payload as sent.
Uint8List packBytes(String payload) => utf8.encode('oldbrainfix:pack:$payload');

/// The name a Pack written on [day] goes by. It says nothing of what is
/// in it.
String packFileName(DateTime day) {
  String two(int n) => n.toString().padLeft(2, '0');
  return 'oldbrainfix-pack-${day.year}-${two(day.month)}-${two(day.day)}'
      '.$packExtension';
}

/// A file that is no Pack this app can read: no JSON, damaged, or
/// carrying a Profile whose signature does not hold over what it holds,
/// as when it was changed after it was signed.
class NotAPack implements Exception {
  const NotAPack();

  @override
  String toString() => 'NotAPack';
}

/// A Pack of a format a later version of the app writes (ADR 0034): this
/// app needs an update to open it.
class PackNeedsUpdate implements Exception {
  const PackNeedsUpdate();

  @override
  String toString() => 'PackNeedsUpdate';
}

/// A Pack carrying the Profile of a person the user blocked: it is not
/// opened (Q471).
class PackFromBlocked implements Exception {
  const PackFromBlocked();

  @override
  String toString() => 'PackFromBlocked';
}

/// An Area of a Pack: its name and the names of its Topics the Pack's
/// Notes are filed into. Nothing else of the Area goes along (Q469).
class PackArea {
  final String name;
  final List<String> topics;

  const PackArea(this.name, [this.topics = const []]);
}

/// A Section of a Pack's Note: its text, the format it is in and the
/// version of its structure; not when it was captured.
class PackSection {
  final String text;
  final String format;
  final int version;

  const PackSection(
    this.text, {
    this.format = TextFormats.plain,
    this.version = TextFormats.version,
  });
}

/// A Line of a Pack's Checklist or Shopping List: its text, whether it is
/// under the Line above (a Checklist's), and its reappear time (a
/// Shopping List's). It arrives unticked, open.
class PackLine {
  final String text;
  final bool indent;
  final Reappear? reappear;

  const PackLine(this.text, {this.indent = false, this.reappear});
}

/// A field of a Pack's Tally: as set up, without a value.
class PackField {
  final String name;
  final String unit;
  final TallyRule rule;

  /// The target as the log keeps it ([encodeTallyNumber]), or null.
  final String? target;
  final bool hidden;

  /// Whether it is a Check field (#264).
  final bool check;

  /// The index of the field among the Tally's fields this one
  /// multiplied (#200), or null.
  final int? multiplies;

  const PackField({
    required this.name,
    this.unit = '',
    this.rule = TallyRule.sum,
    this.target,
    this.hidden = false,
    this.check = false,
    this.multiplies,
  });
}

/// A Session of a Pack's Note: its pool as the indices of the Pack's
/// Notes that are its Tallies, its counts and its rule.
class PackSession {
  final List<int> pool;
  final int count;

  /// The count of a weekday (1 for Monday to 7 for Sunday) with its own.
  final Map<int, int> counts;
  final SessionRule rule;

  const PackSession({
    required this.pool,
    required this.count,
    this.counts = const {},
    required this.rule,
  });
}

/// A photo or a voice recording of a Pack's Note: its hash, whether its
/// file goes along, its size then, and its Photo Text or Transcript.
class PackFile {
  final String hash;

  /// The file's bytes as the Pack carries them; null when the file stays
  /// out. On export they are read when the Pack is written.
  final Uint8List? bytes;

  /// Whether the file goes along.
  final bool hasFile;

  /// How many bytes the file has; 0 without it.
  final int size;

  /// The Photo Text or Transcript, or null when none goes along.
  final String? text;

  const PackFile(
    this.hash, {
    this.bytes,
    this.hasFile = false,
    this.size = 0,
    this.text,
  });
}

/// A Note of a Pack.
class PackNote {
  /// The index of its Area among the Pack's, null for none; and of its
  /// Topic among that Area's.
  final int? area;
  final int? topic;

  /// Whether a Session of the Pack brought it from outside what the Pack
  /// was made from: it shows under that Session.
  final bool outside;
  final String text;
  final String format;
  final int version;
  final List<PackSection> sections;

  /// The Lines of its Checklist or Shopping List; null for neither.
  final List<PackLine>? lines;
  final bool shopping;

  /// The fields of its Tally; empty for none.
  final List<PackField> fields;

  /// Its Tally's bar length (#264), null for a day as never set; and
  /// whether it counts in sets.
  final TallyPeriod? bar;
  final bool sets;
  final PackSession? session;

  /// Its Routines and Reminders, as their schedules.
  final List<Schedule> routines;

  /// Whether it carries an open Frog.
  final bool frog;
  final List<PackFile> photos;
  final List<PackFile> recordings;

  const PackNote({
    this.area,
    this.topic,
    this.outside = false,
    required this.text,
    this.format = TextFormats.plain,
    this.version = TextFormats.version,
    this.sections = const [],
    this.lines,
    this.shopping = false,
    this.fields = const [],
    this.bar,
    this.sets = false,
    this.session,
    this.routines = const [],
    this.frog = false,
    this.photos = const [],
    this.recordings = const [],
  });

  /// The kind it is.
  NoteKind get kind => fields.isNotEmpty
      ? NoteKind.tally
      : session != null
      ? NoteKind.session
      : lines == null
      ? NoteKind.plain
      : shopping
      ? NoteKind.shoppingList
      : NoteKind.checklist;
}

/// What a Pack holds: Areas, Notes, and the sender's Profile card when
/// it says who it is from.
class Pack {
  final List<PackArea> areas;
  final List<PackNote> notes;

  /// The sender's Profile card, `<payload>.<signature>`, or null.
  final String? card;

  const Pack({this.areas = const [], this.notes = const [], this.card});
}

/// What an item of a Pack's tree is.
enum PackItemKind {
  area,
  topic,
  note,
  photo,
  photoText,
  recording,
  transcript,
  routine,
  reminder,
  frog,
}

/// One item of a Pack's tree, ticked or not (Q468). An Area or a Topic
/// only groups the items under it; a Note's photos, recordings, their
/// texts, Routines, Reminders and Frog lie under it, and the Tallies a
/// Session brought from outside lie under the Session.
class PackItem {
  /// The key it is ticked by: `n<i>` for the Pack's Note i, `n<i>.p<j>`
  /// and `n<i>.pt<j>` for its photo j and that photo's text, `n<i>.a<j>`
  /// and `n<i>.at<j>` for a recording and its Transcript, `n<i>.r<j>` for
  /// its Routine or Reminder j, `n<i>.f` for its Frog; `a<i>` and
  /// `a<i>.t<j>` for an Area and a Topic, which are never ticked.
  final String key;
  final PackItemKind kind;

  /// The Pack's Note it is or belongs to; null for an Area or a Topic.
  final int? note;

  /// The index of the photo, recording, Routine or Reminder.
  final int? index;

  /// How many bytes it adds to the Pack.
  final int size;
  final List<PackItem> children;

  const PackItem({
    required this.key,
    required this.kind,
    this.note,
    this.index,
    this.size = 0,
    this.children = const [],
  });
}

/// The keys of Pack items.
abstract final class PackItemKeys {
  static String note(int i) => 'n$i';
  static String photo(int i, int j) => 'n$i.p$j';
  static String photoText(int i, int j) => 'n$i.pt$j';
  static String recording(int i, int j) => 'n$i.a$j';
  static String transcript(int i, int j) => 'n$i.at$j';
  static String routine(int i, int j) => 'n$i.r$j';
  static String frog(int i) => 'n$i.f';
}

/// How many characters [bytes] take as base64.
int _base64Length(int bytes) => (bytes + 2) ~/ 3 * 4;

int _utf8Length(String text) => utf8.encode(text).length;

extension PackTree on Pack {
  /// The tree of the Pack (Q468, Q459): its Areas, each with the Notes
  /// filed into it alone and then its Topics with theirs, then the Notes
  /// of no Area; under each Note its photos, each with its Photo Text,
  /// its recordings, each with its Transcript, its Routines and
  /// Reminders and its Frog; under a Session the Tallies it brought from
  /// outside, and under a Tally its own items. A Note shows where its
  /// Area and Topic say; one brought from outside only under the
  /// Sessions that pick it.
  List<PackItem> tree() {
    PackItem noteItem(int i, Set<int> seen) {
      final note = notes[i];
      final children = <PackItem>[
        for (final (j, p) in note.photos.indexed) ...[
          if (p.hasFile)
            PackItem(
              key: PackItemKeys.photo(i, j),
              kind: PackItemKind.photo,
              note: i,
              index: j,
              size: _base64Length(p.size),
            ),
          if (p.text case final text?)
            PackItem(
              key: PackItemKeys.photoText(i, j),
              kind: PackItemKind.photoText,
              note: i,
              index: j,
              size: _utf8Length(text),
            ),
        ],
        for (final (j, a) in note.recordings.indexed) ...[
          if (a.hasFile)
            PackItem(
              key: PackItemKeys.recording(i, j),
              kind: PackItemKind.recording,
              note: i,
              index: j,
              size: _base64Length(a.size),
            ),
          if (a.text case final text?)
            PackItem(
              key: PackItemKeys.transcript(i, j),
              kind: PackItemKind.transcript,
              note: i,
              index: j,
              size: _utf8Length(text),
            ),
        ],
        for (final (j, r) in note.routines.indexed)
          PackItem(
            key: PackItemKeys.routine(i, j),
            kind: r.once ? PackItemKind.reminder : PackItemKind.routine,
            note: i,
            index: j,
            size: _utf8Length(jsonEncode(_routineJson(r))),
          ),
        if (note.frog)
          PackItem(
            key: PackItemKeys.frog(i),
            kind: PackItemKind.frog,
            note: i,
            size: _utf8Length(_frogJson),
          ),
        if (note.session case final session?)
          for (final t in session.pool)
            if (notes[t].outside && !seen.contains(t))
              noteItem(t, {...seen, i}),
      ];
      return PackItem(
        key: PackItemKeys.note(i),
        kind: PackItemKind.note,
        note: i,
        size: _utf8Length(jsonEncode(_noteJson(note, files: false))),
        children: children,
      );
    }

    final shown = [
      for (final (i, n) in notes.indexed)
        if (!n.outside) i,
    ];
    return [
      for (final (a, area) in areas.indexed)
        if (shown.any((i) => notes[i].area == a))
          PackItem(
            key: 'a$a',
            kind: PackItemKind.area,
            children: [
              for (final i in shown)
                if (notes[i].area == a && notes[i].topic == null)
                  noteItem(i, {}),
              for (final (t, _) in area.topics.indexed)
                if (shown.any((i) => notes[i].area == a && notes[i].topic == t))
                  PackItem(
                    key: 'a$a.t$t',
                    kind: PackItemKind.topic,
                    children: [
                      for (final i in shown)
                        if (notes[i].area == a && notes[i].topic == t)
                          noteItem(i, {}),
                    ],
                  ),
            ],
          ),
      for (final i in shown)
        if (notes[i].area == null) noteItem(i, {}),
    ];
  }

  /// The keys of every item that can be ticked: "Tick all".
  Set<String> allKeys() => {
    for (final (i, note) in notes.indexed) ...[
      PackItemKeys.note(i),
      for (final (j, p) in note.photos.indexed) ...[
        if (p.hasFile) PackItemKeys.photo(i, j),
        if (p.text != null) PackItemKeys.photoText(i, j),
      ],
      for (final (j, a) in note.recordings.indexed) ...[
        if (a.hasFile) PackItemKeys.recording(i, j),
        if (a.text != null) PackItemKeys.transcript(i, j),
      ],
      for (final (j, _) in note.routines.indexed) PackItemKeys.routine(i, j),
      if (note.frog) PackItemKeys.frog(i),
    ],
  };

  /// What is ticked when an export starts (Q468): everything but the
  /// Reminders, the photos and the recordings.
  Set<String> exportTicks() => {
    for (final (i, note) in notes.indexed) ...[
      PackItemKeys.note(i),
      for (final (j, p) in note.photos.indexed)
        if (p.text != null) PackItemKeys.photoText(i, j),
      for (final (j, a) in note.recordings.indexed)
        if (a.text != null) PackItemKeys.transcript(i, j),
      for (final (j, r) in note.routines.indexed)
        if (!r.once) PackItemKeys.routine(i, j),
      if (note.frog) PackItemKeys.frog(i),
    ],
  };

  /// [ticks] with the item [key] ticked [on], as the Session rule says
  /// (Q458): ticking a Session ticks the Tallies of its pool; unticking a
  /// Tally takes it out of the pool of each Session, and a Session left
  /// with none of its pool ticked is unticked too.
  Set<String> tick(Set<String> ticks, String key, bool on) {
    final out = {...ticks};
    final i = _noteOf(key);
    if (on) {
      out.add(key);
      if (i != null) {
        for (final t in notes[i].session?.pool ?? const <int>[]) {
          out.add(PackItemKeys.note(t));
        }
      }
      return out;
    }
    out.remove(key);
    if (i == null) return out;
    for (final (s, note) in notes.indexed) {
      final pool = note.session?.pool;
      if (pool == null || !pool.contains(i)) continue;
      if (!pool.any((t) => out.contains(PackItemKeys.note(t)))) {
        out.remove(PackItemKeys.note(s));
      }
    }
    return out;
  }

  /// The Note whose own item [key] is, `n<i>`; null for anything else.
  int? _noteOf(String key) {
    final match = RegExp(r'^n(\d+)$').firstMatch(key);
    if (match == null) return null;
    final i = int.tryParse(match[1] ?? '');
    return i != null && i < notes.length ? i : null;
  }

  /// How many bytes the Pack has with [ticks] (Q470): each ticked Note
  /// with its ticked items, as [tree] sizes them.
  int size(Set<String> ticks) {
    var total = 0;
    // A Note under two Sessions counts once.
    final counted = <String>{};
    void walk(PackItem item, bool counts) {
      if (item.kind == PackItemKind.note && !counted.add(item.key)) return;
      final ticked = switch (item.kind) {
        PackItemKind.area || PackItemKind.topic => true,
        PackItemKind.note => ticks.contains(item.key),
        _ => counts && ticks.contains(item.key),
      };
      if (ticked) total += item.size;
      for (final child in item.children) {
        walk(child, item.kind == PackItemKind.note ? ticked : counts);
      }
    }

    for (final item in tree()) {
      walk(item, true);
    }
    return total;
  }

  /// The Pack as [ticks] picks from it: the ticked Notes, each with its
  /// ticked items, a file without its text or a text without its file as
  /// ticked; a Session's pool only its ticked Tallies, and a Session with
  /// none left out (the core needs at least one); the Areas and Topics
  /// the Notes kept are filed into, in their order.
  Pack picked(Set<String> ticks) {
    final kept = <int>[
      for (final (i, _) in notes.indexed)
        if (ticks.contains(PackItemKeys.note(i))) i,
    ];
    // A Session with none of its pool ticked goes neither.
    kept.removeWhere(
      (i) =>
          notes[i].session?.pool.any(
            (t) => ticks.contains(PackItemKeys.note(t)),
          ) ==
          false,
    );
    final newIndex = {for (final (n, i) in kept.indexed) i: n};
    final areaIndex = <int, int>{};
    final topicIndex = <(int, int), int>{};
    final areasOut = <({String name, List<String> topics})>[];
    for (final i in kept) {
      final note = notes[i];
      final a = note.area;
      if (a == null) continue;
      final at = areaIndex.putIfAbsent(a, () {
        areasOut.add((name: areas[a].name, topics: []));
        return areasOut.length - 1;
      });
      if (note.topic case final t?) {
        topicIndex.putIfAbsent((a, t), () {
          areasOut[at].topics.add(areas[a].topics[t]);
          return areasOut[at].topics.length - 1;
        });
      }
    }
    final pooled = <int>{
      for (final i in kept)
        for (final t in notes[i].session?.pool ?? const <int>[])
          if (newIndex.containsKey(t)) t,
    };
    PackFile file(PackFile f, bool withFile, bool withText) => PackFile(
      f.hash,
      bytes: withFile ? f.bytes : null,
      hasFile: withFile && f.hasFile,
      size: withFile && f.hasFile ? f.size : 0,
      text: withText ? f.text : null,
    );
    return Pack(
      card: card,
      areas: [for (final a in areasOut) PackArea(a.name, a.topics)],
      notes: [
        for (final i in kept)
          PackNote(
            area: notes[i].area == null ? null : areaIndex[notes[i].area],
            topic: switch ((notes[i].area, notes[i].topic)) {
              (final a?, final t?) => topicIndex[(a, t)],
              _ => null,
            },
            outside: notes[i].outside && pooled.contains(i),
            text: notes[i].text,
            format: notes[i].format,
            version: notes[i].version,
            sections: notes[i].sections,
            lines: notes[i].lines,
            shopping: notes[i].shopping,
            fields: notes[i].fields,
            bar: notes[i].bar,
            sets: notes[i].sets,
            session: switch (notes[i].session) {
              final s? => PackSession(
                pool: [for (final t in s.pool) ?newIndex[t]],
                count: s.count,
                counts: s.counts,
                rule: s.rule,
              ),
              null => null,
            },
            routines: [
              for (final (j, r) in notes[i].routines.indexed)
                if (ticks.contains(PackItemKeys.routine(i, j))) r,
            ],
            frog: notes[i].frog && ticks.contains(PackItemKeys.frog(i)),
            photos: [
              for (final (j, p) in notes[i].photos.indexed)
                if (file(
                      p,
                      ticks.contains(PackItemKeys.photo(i, j)),
                      ticks.contains(PackItemKeys.photoText(i, j)),
                    )
                    case final f when f.hasFile || f.text != null)
                  f,
            ],
            recordings: [
              for (final (j, a) in notes[i].recordings.indexed)
                if (file(
                      a,
                      ticks.contains(PackItemKeys.recording(i, j)),
                      ticks.contains(PackItemKeys.transcript(i, j)),
                    )
                    case final f when f.hasFile || f.text != null)
                  f,
            ],
          ),
      ],
    );
  }
}

// ------------------------------------------------------------ the bytes

const _frogJson = 'true';

Map<String, Object?> _routineJson(Schedule s) => {
  if (s.rule case final rule?) 'rule': rule.format(),
  'start': s.start.toString(),
  'times': s.times.join(','),
};

/// A Note as the payload holds it, keys sorted; with [files] its photos
/// and recordings with their bytes read by [bytesOf] when the Pack holds
/// none yet.
Map<String, Object?> _noteJson(
  PackNote n, {
  required bool files,
  Uint8List? Function(String kind, String hash)? bytesOf,
}) {
  List<Map<String, Object?>> filesJson(List<PackFile> list, String kind) => [
    for (final f in list)
      {
        if (f.hasFile)
          'bytes': base64.encode(
            f.bytes ??
                bytesOf?.call(kind, f.hash) ??
                (throw StateError('no bytes of $kind ${f.hash}')),
          ),
        'hash': f.hash,
        'text': ?f.text,
      },
  ];
  return {
    'area': ?n.area,
    if (n.fields.isNotEmpty) 'bar': ?n.bar?.name,
    if (n.fields.isNotEmpty)
      'fields': [
        for (final f in n.fields)
          {
            if (f.check) 'check': true,
            if (f.hidden) 'hidden': true,
            'multiplies': ?f.multiplies,
            'name': f.name,
            'rule': f.rule.name,
            'target': ?f.target,
            'unit': f.unit,
          },
      ],
    'format': n.format,
    if (files && n.frog) 'frog': true,
    if (n.lines case final lines?) ...{
      'lines': [
        for (final l in lines)
          {
            if (l.indent) 'indent': true,
            if (l.reappear case final r?) 'reappear': r.code,
            'text': l.text,
          },
      ],
      'list': n.shopping ? ShoppingKeys.shopping : ChecklistKeys.checklist,
    },
    if (files && n.outside) 'outside': true,
    if (files && n.photos.isNotEmpty) 'photos': filesJson(n.photos, 'photo'),
    if (files && n.recordings.isNotEmpty)
      'recordings': filesJson(n.recordings, 'audio'),
    if (files && n.routines.isNotEmpty)
      'routines': [for (final r in n.routines) _routineJson(r)],
    if (n.sections.isNotEmpty)
      'sections': [
        for (final s in n.sections)
          {
            'format': s.format,
            'text': s.text,
            if (s.version != TextFormats.version) 'v': s.version,
          },
      ],
    if (n.session case final s?)
      'session': {
        'count': s.count,
        if (s.counts.isNotEmpty)
          'counts': {
            for (final d in s.counts.keys.toList()..sort()) '$d': s.counts[d],
          },
        'pool': s.pool,
        'rule': s.rule.name,
      },
    if (n.fields.isNotEmpty && n.sets) 'sets': true,
    'text': n.text,
    'topic': ?n.topic,
    if (n.version != TextFormats.version) 'v': n.version,
  };
}

/// The payload of [pack], keys sorted and no spaces.
String _payload(
  Pack pack,
  Uint8List? Function(String kind, String hash)? bytesOf,
) => jsonEncode({
  'areas': [
    for (final a in pack.areas) {'name': a.name, 'topics': a.topics},
  ],
  'notes': [
    for (final n in pack.notes) _noteJson(n, files: true, bytesOf: bytesOf),
  ],
  'profile': ?pack.card,
});

/// The file of [pack], as it is: signed by [key] when it carries a
/// Profile card, the card [key]'s Device made. The bytes of its files
/// are read with [bytesOf] where the Pack holds none.
Uint8List encodePack(
  Pack pack, {
  SigningKey? key,
  Uint8List? Function(String kind, String hash)? bytesOf,
}) {
  final payload = _payload(pack, bytesOf);
  final sig = switch ((pack.card, key)) {
    (null, _) => null,
    (_, final key?) => key.sign(packBytes(payload)),
    (_, null) => throw ArgumentError('a Pack with a Profile is signed'),
  };
  return utf8.encode(
    jsonEncode({'pack': payload, 'sig': ?sig, 'type': _packType}),
  );
}

/// The JSON object of the file [bytes] when it says it is a Pack, of any
/// format; null for anything else.
Map<String, Object?>? _packJson(List<int> bytes) {
  if (!startsLikeJson(bytes)) return null;
  final Object? json;
  try {
    json = jsonDecode(utf8.decode(bytes));
  } on FormatException {
    return null;
  }
  return json is Map<String, Object?> && json['type'] == _packType
      ? json
      : null;
}

/// Whether the first byte of [bytes] that is no blank opens a JSON
/// object, as a Pack's file does: its first bytes are enough to tell.
bool startsLikeJson(List<int> bytes) {
  for (final b in bytes) {
    if (b == 0x20 || b == 0x09 || b == 0x0a || b == 0x0d) continue;
    return b == 0x7b;
  }
  return false;
}

/// Whether [bytes] are a Pack rather than a Sent Item, a Profile card
/// file or a bundle, in a format this version reads or a later one.
bool looksLikePack(List<int> bytes) => _packJson(bytes) != null;

/// The JSON object of the payload [text]; throws [NotAPack] when it is
/// none.
Map<String, Object?> _payloadJson(String text) {
  final Object? json;
  try {
    json = jsonDecode(text);
  } on FormatException {
    throw const NotAPack();
  }
  if (json is! Map<String, Object?>) throw const NotAPack();
  return json;
}

/// The Profile card the payload [json] carries, or null; throws
/// [NotAPack] when it is no text.
String? _cardOf(Map<String, Object?> json) => switch (json['profile']) {
  null => null,
  final String card => card,
  _ => throw const NotAPack(),
};

/// The Pack the payload [json] holds; throws [NotAPack] when it is none.
Pack _packOf(Map<String, Object?> json) {
  if (json case {
    'areas': final List<Object?> areas,
    'notes': final List<Object?> notes,
  }) {
    final out = Pack(
      areas: [for (final a in areas) _area(a)],
      notes: [for (final n in notes) _note(n)],
      card: _cardOf(json),
    );
    _check(out);
    return out;
  }
  throw const NotAPack();
}

/// Refuses a Pack whose indices point at nothing.
void _check(Pack pack) {
  for (final n in pack.notes) {
    final a = n.area;
    if (a != null && a >= pack.areas.length) throw const NotAPack();
    final t = n.topic;
    if (t != null && (a == null || t >= pack.areas[a].topics.length)) {
      throw const NotAPack();
    }
    for (final i in n.session?.pool ?? const <int>[]) {
      if (i >= pack.notes.length || pack.notes[i].fields.isEmpty) {
        throw const NotAPack();
      }
    }
    for (final f in n.fields) {
      final m = f.multiplies;
      if (m != null && m >= n.fields.length) throw const NotAPack();
    }
  }
}

PackArea _area(Object? json) {
  if (json case {'name': final String name, 'topics': final List<Object?> t}
      when name.trim().isNotEmpty) {
    return PackArea(name, [
      for (final topic in t)
        if (topic is String && topic.trim().isNotEmpty)
          topic
        else
          throw const NotAPack(),
    ]);
  }
  throw const NotAPack();
}

T? _optional<T>(Map<Object?, Object?> json, String key) {
  final value = json[key];
  if (value == null) return null;
  if (value is! T) throw const NotAPack();
  return value as T;
}

int? _index(Map<Object?, Object?> json, String key) {
  final value = _optional<int>(json, key);
  if (value != null && value < 0) throw const NotAPack();
  return value;
}

String _formatOf(Map<Object?, Object?> json) =>
    _optional<String>(json, 'format') ?? TextFormats.plain;

int _versionOf(Map<Object?, Object?> json) {
  final v = _optional<int>(json, 'v') ?? TextFormats.version;
  if (v < 1) throw const NotAPack();
  return v;
}

PackNote _note(Object? json) {
  if (json is! Map<Object?, Object?>) throw const NotAPack();
  final text = json['text'];
  if (text is! String) throw const NotAPack();
  final list = _optional<String>(json, 'list');
  final linesJson = _optional<List<Object?>>(json, 'lines');
  if ((list == null) != (linesJson == null)) throw const NotAPack();
  if (list != null &&
      list != ShoppingKeys.shopping &&
      list != ChecklistKeys.checklist) {
    throw const NotAPack();
  }
  final shopping = list == ShoppingKeys.shopping;
  return PackNote(
    area: _index(json, 'area'),
    topic: _index(json, 'topic'),
    outside: _optional<bool>(json, 'outside') ?? false,
    text: text,
    format: _formatOf(json),
    version: _versionOf(json),
    sections: [
      for (final s in _optional<List<Object?>>(json, 'sections') ?? const [])
        if (s case {'text': final String text})
          PackSection(text, format: _formatOf(s), version: _versionOf(s))
        else
          throw const NotAPack(),
    ],
    lines: linesJson == null
        ? null
        : [
            for (final l in linesJson)
              if (l case {'text': final String text})
                PackLine(
                  text,
                  indent: !shopping && (_optional<bool>(l, 'indent') ?? false),
                  reappear: shopping
                      ? Reappear.read(_optional<String>(l, 'reappear'))
                      : null,
                )
              else
                throw const NotAPack(),
          ],
    shopping: shopping,
    fields: [
      for (final f in _optional<List<Object?>>(json, 'fields') ?? const [])
        _field(f),
    ],
    bar: switch (_optional<String>(json, 'bar')) {
      null => null,
      final bar =>
        TallyPeriod.values.asNameMap()[bar] ?? (throw const NotAPack()),
    },
    sets: _optional<bool>(json, 'sets') ?? false,
    session: switch (json['session']) {
      null => null,
      final s => _session(s),
    },
    routines: [
      for (final r in _optional<List<Object?>>(json, 'routines') ?? const [])
        _routine(r),
    ],
    frog: _optional<bool>(json, 'frog') ?? false,
    photos: [
      for (final p in _optional<List<Object?>>(json, 'photos') ?? const [])
        _file(p),
    ],
    recordings: [
      for (final a in _optional<List<Object?>>(json, 'recordings') ?? const [])
        _file(a),
    ],
  );
}

PackField _field(Object? json) {
  if (json case {'name': final String name, 'rule': final String rule}
      when name.trim().isNotEmpty) {
    final target = _optional<String>(json, 'target');
    if (target != null && decodeTallyNumber(target) == null) {
      throw const NotAPack();
    }
    return PackField(
      name: name,
      unit: _optional<String>(json, 'unit') ?? '',
      rule: TallyRule.values.asNameMap()[rule] ?? (throw const NotAPack()),
      target: target,
      hidden: _optional<bool>(json, 'hidden') ?? false,
      check: _optional<bool>(json, 'check') ?? false,
      multiplies: _index(json, 'multiplies'),
    );
  }
  throw const NotAPack();
}

PackSession _session(Object? json) {
  if (json
      case {
        'count': final int count,
        'pool': final List<Object?> pool,
        'rule': final String rule,
      }
      when pool.isNotEmpty) {
    final counts = <int, int>{};
    for (final MapEntry(:key, :value)
        in (_optional<Map<Object?, Object?>>(json, 'counts') ?? const {})
            .entries) {
      final day = int.tryParse('$key');
      if (day == null || day < 1 || day > 7 || value is! int) {
        throw const NotAPack();
      }
      counts[day] = value;
    }
    for (final c in [count, ...counts.values]) {
      if (c < 1 || c > SessionKeys.maxCount) throw const NotAPack();
    }
    return PackSession(
      pool: [
        for (final i in pool)
          if (i is int && i >= 0) i else throw const NotAPack(),
      ],
      count: count,
      counts: counts,
      rule: SessionRule.values.asNameMap()[rule] ?? (throw const NotAPack()),
    );
  }
  throw const NotAPack();
}

Schedule _routine(Object? json) {
  if (json case {'start': final String start, 'times': final String times}) {
    return Routines.scheduleOf({
          RoutineKeys.start: start,
          RoutineKeys.times: times,
          RoutineKeys.rule: _optional<String>(json, 'rule'),
        }) ??
        (throw const NotAPack());
  }
  throw const NotAPack();
}

PackFile _file(Object? json) {
  if (json case {'hash': final String hash}
      when RegExp(r'^[0-9a-f]{64}$').hasMatch(hash)) {
    final b = _optional<String>(json, 'bytes');
    final Uint8List? bytes;
    try {
      bytes = b == null ? null : base64.decode(b);
    } on FormatException {
      throw const NotAPack();
    }
    final text = _optional<String>(json, 'text');
    if (bytes == null && text == null) throw const NotAPack();
    return PackFile(
      hash,
      bytes: bytes,
      hasFile: bytes != null,
      size: bytes?.length ?? 0,
      text: text,
    );
  }
  throw const NotAPack();
}

// --------------------------------------------------------------- export

extension PackExport on Store {
  /// The Pack of everything the tree may offer when exporting from the
  /// Area, Topic or Note [from] (Q468): from a Note, that Note; from a
  /// Topic, its Notes; from an Area, its Notes and its Topics' Notes,
  /// newest capture first; Notes of other people in a Shared Area too
  /// (Q466). Each with what it is (Q455), its photos and recordings with
  /// their texts (Q456, Q457), its Routines and Reminders (Q465) and an
  /// open Frog (Q473); the Tallies its Session picks from that lie
  /// elsewhere after them, marked as brought from outside, with their
  /// own Area and Topic, but for those filed into an Area of [closed]:
  /// the locked Areas not open now, whose Tallies the app does not show.
  /// Only the names of the Areas and Topics go along (Q469). Nothing in
  /// the trash or merged into another Note.
  Pack packOf(String from, {Set<String> closed = const {}}) {
    final notes = Notes(this);
    final places = Places(this);
    final List<Note> picked;
    if (places.areaById(from) != null || places.topicById(from) != null) {
      // The Area's own Notes first, then each Topic's, by name.
      final ids = [
        from,
        if (places.areaById(from) != null)
          for (final t in places.topics(from)) t.id,
      ];
      final all = notes.all();
      picked = [
        for (final id in ids)
          for (final n in all)
            if (n.place == id) n,
      ];
    } else {
      final note = notes.byId(from);
      if (note == null || note.isTrashed || note.isMerged) {
        throw ArgumentError.value(from, 'from', 'not an Area, Topic or Note');
      }
      picked = [note];
    }
    return _packOfNotes(picked, closed);
  }

  /// The Pack of everything the tree may offer for feedback (#214): each
  /// Area by name, but those of [closed], with its own Notes and then its
  /// Topics' Notes as [packOf] lists them, then the Notes filed into no
  /// Area, newest capture first; nothing in the trash or merged.
  Pack packOfAll({Set<String> closed = const {}}) {
    final places = Places(this);
    final all = Notes(this).all();
    final ids = [
      for (final area in places.areas())
        if (!closed.contains(area.id)) ...[
          area.id,
          for (final t in places.topics(area.id)) t.id,
        ],
    ];
    return _packOfNotes([
      for (final id in ids)
        for (final n in all)
          if (n.place == id) n,
      for (final n in all)
        if (n.place == null) n,
    ], closed);
  }

  /// The Pack of [picked], in their order, and the Tallies their Sessions
  /// pick from that are not among them after them, but for those filed
  /// into an Area of [closed].
  Pack _packOfNotes(List<Note> picked, Set<String> closed) {
    final notes = Notes(this);
    final places = Places(this);
    final order = [...picked];
    final sessions = Sessions(this);
    // The Tallies the Sessions pick from that are not among them.
    for (var k = 0; k < order.length; k++) {
      final setup = sessions.setupOf(order[k].id);
      if (setup == null) continue;
      for (final t in sessions.pool(setup)) {
        if (order.any((n) => n.id == t)) continue;
        final tally = notes.byId(t);
        if (tally == null) continue;
        final place = tally.place;
        final area = place == null
            ? null
            : places.topicById(place)?.areaId ?? place;
        if (area != null && closed.contains(area)) continue;
        order.add(tally);
      }
    }
    final index = {for (final (i, n) in order.indexed) n.id: i};
    final areas = <String, int>{};
    final areaNames = <({String name, List<String> topics})>[];
    final topics = <String, int>{};
    (int?, int?) placeOf(Note n) {
      final place = n.place;
      if (place == null) return (null, null);
      final topic = places.topicById(place);
      final areaId = topic?.areaId ?? place;
      final area = places.areaById(areaId);
      if (area == null) return (null, null);
      final a = areas.putIfAbsent(areaId, () {
        areaNames.add((name: area.name, topics: []));
        return areaNames.length - 1;
      });
      if (topic == null) return (a, null);
      final t = topics.putIfAbsent(topic.id, () {
        areaNames[a].topics.add(topic.name);
        return areaNames[a].topics.length - 1;
      });
      return (a, t);
    }

    final out = <PackNote>[];
    for (final (i, note) in order.indexed) {
      final (area, topic) = placeOf(note);
      out.add(_packNote(note, area, topic, i >= picked.length, index));
    }
    return Pack(
      areas: [for (final a in areaNames) PackArea(a.name, a.topics)],
      notes: out,
    );
  }

  /// The Note [note] as a Pack carries it, filed into the Pack's Area
  /// [area] and Topic [topic], its Session's pool by [index].
  PackNote _packNote(
    Note note,
    int? area,
    int? topic,
    bool outside,
    Map<String, int> index,
  ) {
    final id = note.id;
    final kind = NoteKinds(this).of(id);
    final setup = kind == NoteKind.session ? Sessions(this).setupOf(id) : null;
    final tally = kind == NoteKind.tally ? Tallies(this).byNote(id) : null;
    final fields = tally?.fields ?? const <TallyField>[];
    final fieldIndex = {for (final (i, f) in fields.indexed) f.id: i};
    PackFile file(String kind, String hash, String? text) {
      final bytes = blob(kind, hash);
      return PackFile(
        hash,
        hasFile: bytes != null,
        size: bytes?.length ?? 0,
        text: text,
      );
    }

    final routines = Routines(this).ofNote(id)
      ..sort((a, b) => a.id.compareTo(b.id));
    return PackNote(
      area: area,
      topic: topic,
      outside: outside,
      text: _baked(note.text, note.textEditable, note.boxes),
      format: note.textFormat,
      version: note.textVersion,
      sections: [
        for (final s in Notes(this).sections(id))
          if (s.text.trim().isNotEmpty)
            PackSection(
              _baked(s.text, s.textEditable, s.boxes),
              format: s.textFormat,
              version: s.textVersion,
            ),
      ],
      lines: switch (kind) {
        NoteKind.checklist => [
          for (final l in Checklists(this).byNote(id)?.lines ?? const <Line>[])
            PackLine(l.text, indent: l.inGroup),
        ],
        NoteKind.shoppingList => [
          for (final l
              in ShoppingLists(this).byNote(id, _utc)?.lines ??
                  const <ShoppingLine>[])
            PackLine(l.text, reappear: l.reappear),
        ],
        _ => null,
      },
      shopping: kind == NoteKind.shoppingList,
      fields: [
        for (final f in fields)
          PackField(
            name: f.name,
            unit: f.unit,
            rule: f.rule,
            target: switch (f.target) {
              final target? => encodeTallyNumber(target),
              null => null,
            },
            hidden: f.hidden,
            check: f.check,
            multiplies: f.multiplies == null ? null : fieldIndex[f.multiplies],
          ),
      ],
      bar: switch (tally?.bar) {
        null || TallyPeriod.day => null,
        final bar => bar,
      },
      sets: tally?.countsInSets ?? false,
      session: setup == null
          ? null
          : PackSession(
              pool: [for (final t in Sessions(this).pool(setup)) ?index[t]],
              count: setup.count,
              counts: setup.counts,
              rule: setup.rule,
            ),
      routines: [for (final r in routines) r.schedule],
      frog: Frogs(this).ofNote(id).any((f) => !f.isDone),
      photos: [
        for (final hash in [...note.photos, ...note.removedPhotos])
          file('photo', hash, note.photoTexts[hash]),
      ],
      recordings: [
        for (final hash in [...note.recordings, ...note.removedRecordings])
          file('audio', hash, note.transcripts[hash]),
      ],
    );
  }

  /// The file of [pack] as [ticks] picks from it ([PackTree.picked]):
  /// with [addProfile] it carries the own Profile card and is signed by
  /// this Device (Q462). Throws a [StateError] when [addProfile] is
  /// asked before the own Profile can make a card.
  Uint8List writePack(
    Pack pack,
    Set<String> ticks, {
    bool addProfile = false,
  }) => packWrite(pack, ticks, addProfile: addProfile).encode();

  /// [writePack] as far as it needs this Store: what it writes, to be
  /// written with [PackWrite.encode] off this isolate.
  PackWrite packWrite(Pack pack, Set<String> ticks, {bool addProfile = false}) {
    final picked = pack.picked(ticks);
    final card = addProfile
        ? (profileCardText() ?? (throw StateError('no own Profile card')))
        : null;
    return PackWrite._(
      Pack(areas: picked.areas, notes: picked.notes, card: card),
      card == null ? null : signingKey,
      blobs,
    );
  }
}

/// A Pack to write, with what it needs of a Store: its key and the
/// photos and recordings its files are read from. It can be sent to
/// another isolate, which writes it there.
class PackWrite {
  final Pack _pack;
  final SigningKey? _key;
  final ({Blobs photos, Blobs audio}) _blobs;

  const PackWrite._(this._pack, this._key, this._blobs);

  /// The file of the Pack ([encodePack]).
  Uint8List encode() => encodePack(
    _pack,
    key: _key,
    // A Pack's files are photos and recordings, nothing else.
    bytesOf: (kind, hash) =>
        (kind == 'photo' ? _blobs.photos : _blobs.audio).get(hash),
  );
}

/// The wall clock a Shopping List is read on for a Pack: UTC, the same
/// on every Device and in both cores; it decides only the order of its
/// Lines, which all arrive open.
DateTime _utc(DateTime utc) => utc;

/// [text] with each of its [boxes] in the glyph of its current state, as
/// a Sent Item carries it (ADR 0035); a text this version may not change
/// as it is.
String _baked(String text, bool editable, List<ShownBox> boxes) => editable
    ? TextBoxes.baked(text, {for (final b in boxes) b.id: ?b.last})
    : text;

// --------------------------------------------------------------- import

/// A Pack opened here: what it holds and who it is from.
class OpenedPack {
  final Pack pack;

  /// The sender's Profile as the Pack's card carries it, sanitised, with
  /// the picture made again and the user's Override; null for a Pack
  /// without a Profile, or one the user made.
  final Profile? from;

  /// Whether the sender's Profile is kept here, bound to the key that
  /// signed the Pack.
  final bool fromKept;

  const OpenedPack({required this.pack, this.from, this.fromKept = false});
}

/// What adding a Pack made.
class PackAdded {
  /// The new Areas, in the order they were made.
  final List<String> areas;

  /// How many Notes were made.
  final int notes;

  const PackAdded(this.areas, this.notes);
}

/// A Pack's file checked as far as no Store is needed ([checkPack]),
/// its Areas and Notes not read yet ([readPack]).
class CheckedPack {
  /// The payload, as it was signed.
  final String payload;

  /// The sender's Profile card the Pack carries, its signature holding
  /// over the payload; null for a Pack without one.
  final CheckedProfileCard? card;

  const CheckedPack._(this.payload, this.card);
}

/// Checks the Pack [bytes] (a file opened with the app) as far as no
/// Store is needed, before any Area or Note is read (ADR 0043): its
/// format, and the card it carries with the signature over its payload.
/// Throws [PackNeedsUpdate] for one of a later format, and [NotAPack]
/// for a file that is none, damaged, or whose signature does not hold
/// over what it holds.
CheckedPack checkPack(List<int> bytes) {
  final json = _packJson(bytes);
  if (json == null) throw const NotAPack();
  // `1.0 == 1` in Dart: only the whole number 1 is version 1.
  final format = json['format'];
  if (json.containsKey('format') && !(format is int && format == 1)) {
    throw const PackNeedsUpdate();
  }
  final payload = json['pack'];
  final sig = json['sig'];
  if (payload is! String || (sig != null && sig is! String)) {
    throw const NotAPack();
  }
  final card = _cardOf(_payloadJson(payload));
  if ((card == null) != (sig == null)) throw const NotAPack();
  if (card == null) return CheckedPack._(payload, null);
  final checked = checkProfileCard(card);
  if (checked == null ||
      !verifySignature(
        checked.key.publicKey,
        packBytes(payload),
        sig as String,
      )) {
    throw const NotAPack();
  }
  return CheckedPack._(payload, checked);
}

/// The Areas and Notes of the Pack [checked]; throws [NotAPack] when
/// they are none.
Pack readPack(CheckedPack checked) => _packOf(_payloadJson(checked.payload));

extension PackImport on Store {
  /// Opens the Pack [bytes] (a file opened with the app): checks it and
  /// says who it is from. Throws [PackNeedsUpdate] for one of a later
  /// format, [NotAPack] for a file that is none, damaged, or whose
  /// signature does not hold over what it holds or carries a Profile not
  /// bound to the key that signed it, and [PackFromBlocked] for one
  /// carrying the Profile of a blocked person, or made by a Device of
  /// theirs (Q471). Nothing is kept. The card and the signature are
  /// checked, and a blocked sender refused, before any Area or Note is
  /// read (ADR 0043); [checkPack] and [readPack] can run off this
  /// isolate.
  OpenedPack openPack(List<int> bytes) {
    final checked = checkPack(bytes);
    refuseBlockedSender(checked);
    return openedPack(checked, readPack(checked));
  }

  /// Throws [PackFromBlocked] when the Pack [checked] carries the Profile
  /// of a blocked person, or was made by a Device of theirs (Q471).
  void refuseBlockedSender(CheckedPack checked) {
    final card = checked.card;
    if (card != null &&
        (isBlocked(card.id) || blockedDevices().contains(card.key.device))) {
      throw const PackFromBlocked();
    }
  }

  /// The Pack [checked], its Areas and Notes [pack], opened here: who it
  /// is from. Throws [NotAPack] when it carries a Profile not bound to
  /// the key that signed it.
  OpenedPack openedPack(CheckedPack checked, Pack pack) {
    final card = checked.card;
    if (card == null || isOwnPerson(card.id)) return OpenedPack(pack: pack);
    final sender = sentItemProfile(card.profile.toJson(), card.key.publicKey);
    if (sender == null) throw const NotAPack();
    return OpenedPack(pack: pack, from: sender.profile, fromKept: sender.kept);
  }

  /// "Add" on the Pack [opened] with [ticks] (Q459): the ticked Notes,
  /// as [PackTree.picked] picks them, as new Notes of this Device's,
  /// captured now, in new private Areas and Topics, never in an Area
  /// there already (Q460, Q461): each of the Pack's Areas becomes a new
  /// Area of its name, or of its name with ` (<suffix>)` when an Area has
  /// it, ` (<suffix> 2)` and so on when that is taken too; a Note of no
  /// Area lands in a new Area named [noArea] by the same rule. Routines
  /// start today by [wallClock], their times and rules as they are
  /// (Q464); a Reminder keeps its day and time, and one already past is
  /// left out (Q465). An open Frog is a new Frog. Without [claim], each
  /// Note keeps a "From …" badge ([PackKeys.from], Q463), when the
  /// Pack says who it is from. All of it is written in one batch.
  PackAdded addPack(
    OpenedPack opened,
    Set<String> ticks, {
    required bool claim,
    required WallClock wallClock,
    required String suffix,
    required String noArea,
  }) {
    final pack = opened.pack.picked(ticks);
    if (pack.notes.isEmpty) return const PackAdded([], 0);
    final now = clock().toUtc();
    final today = Day.today(now, wallClock);
    final wall = wallClock(now);
    final from = opened.from;
    final badge = claim || from == null
        ? null
        : jsonEncode({'id': from.id, 'name': from.name});
    // The bytes are kept before the log names them.
    final hashes = [
      for (final n in pack.notes)
        (
          photos: [
            for (final p in n.photos)
              switch (p.bytes) {
                final bytes? => putPhoto(bytes),
                null => p.hash,
              },
          ],
          recordings: [
            for (final a in n.recordings)
              switch (a.bytes) {
                final bytes? => putAudio(bytes),
                null => a.hash,
              },
          ],
        ),
    ];
    return batch(() {
      final taken = {for (final a in Places(this).areas()) a.name};
      String area(String name) {
        final named = _landing(name.trim(), suffix, taken);
        taken.add(named);
        final id = 'area:${Store.newId()}';
        append(id, PlaceKeys.type, PlaceKeys.typeArea);
        append(id, PlaceKeys.name, named);
        return id;
      }

      String topic(String areaId, String name) {
        final id = 'topic:${Store.newId()}';
        append(id, PlaceKeys.type, PlaceKeys.typeTopic);
        append(id, PlaceKeys.area, areaId);
        append(id, PlaceKeys.name, name.trim());
        return id;
      }

      final made = <String>[];
      final areaIds = <String>[];
      for (final a in pack.areas) {
        areaIds.add(area(a.name));
        made.add(areaIds.last);
      }
      final topicIds = [
        for (final (a, pa) in pack.areas.indexed)
          [for (final t in pa.topics) topic(areaIds[a], t)],
      ];
      // The Area for the Notes the Pack files nowhere, made before the
      // Notes as soon as one is unfiled.
      String? none;
      String unfiled() {
        if (none case final id?) return id;
        final id = area(noArea);
        made.add(id);
        return none = id;
      }

      if (pack.notes.any((n) => n.area == null)) unfiled();
      final noteIds = <String>[];
      for (final (i, n) in pack.notes.indexed) {
        final id = 'note:${Store.newId()}';
        noteIds.add(id);
        final place = switch ((n.area, n.topic)) {
          (final a?, final t?) => topicIds[a][t],
          (final a?, null) => areaIds[a],
          _ => unfiled(),
        };
        _writeNote(id, n, place, now, badge, hashes[i]);
        for (final s in n.routines) {
          if (s.once && _past(s, today, wall)) continue;
          _writeRoutine(id, s.once ? s : _from(s, today));
        }
        if (n.frog) {
          final frog = '${FrogKeys.prefix}${Store.newId()}';
          append(frog, FrogKeys.type, FrogKeys.typeFrog);
          append(frog, FrogKeys.note, id);
        }
      }
      // The pools name the Notes made, so they come last.
      for (final (i, n) in pack.notes.indexed) {
        final s = n.session;
        if (s == null) continue;
        final id = noteIds[i];
        append(
          id,
          SessionKeys.pool,
          [for (final t in s.pool) noteIds[t]].join(','),
        );
        append(id, SessionKeys.count, '${s.count}');
        for (final d in s.counts.keys.toList()..sort()) {
          append(id, SessionKeys.countOn(d), '${s.counts[d]}');
        }
        append(id, SessionKeys.rule, s.rule.name);
      }
      return PackAdded(made, pack.notes.length);
    });
  }

  /// Writes the Note [id] as [n] is, filed into [place], captured [now],
  /// with its Sections, files, Lines and fields; with [badge] as its
  /// "From …".
  void _writeNote(
    String id,
    PackNote n,
    String place,
    DateTime now,
    String? badge,
    ({List<String> photos, List<String> recordings}) hashes,
  ) {
    append(id, NoteKeys.type, NoteKeys.typeNote);
    append(id, NoteKeys.captured, now.toIso8601String());
    append(
      id,
      NoteKeys.text,
      StoredText.of(n.format, n.text, n.version).encode(),
    );
    append(id, NoteKeys.place, place);
    if (badge != null) append(id, PackKeys.from, badge);
    for (final (j, s) in n.sections.indexed) {
      // Captured with the Note, in their order.
      final section = 'section:${Store.newId()}';
      append(section, SectionKeys.type, SectionKeys.typeSection);
      append(section, SectionKeys.note, id);
      append(
        section,
        SectionKeys.captured,
        now.add(Duration(microseconds: j + 1)).toIso8601String(),
      );
      append(
        section,
        SectionKeys.text,
        StoredText.of(s.format, s.text, s.version).encode(),
      );
    }
    for (final (j, p) in n.photos.indexed) {
      final hash = hashes.photos[j];
      append(id, NoteKeys.photo(hash), p.hasFile ? NoteKeys.photoAdded : null);
      if (p.text case final text?) append(id, NoteKeys.ocr(hash), text);
    }
    for (final (j, a) in n.recordings.indexed) {
      final hash = hashes.recordings[j];
      append(id, NoteKeys.audio(hash), a.hasFile ? NoteKeys.audioAdded : null);
      if (a.text case final text?) append(id, NoteKeys.transcript(hash), text);
    }
    if (n.lines case final lines?) {
      final making = Store.newId();
      append(
        id,
        ChecklistKeys.kind,
        n.shopping ? ShoppingKeys.kindOf(making) : ChecklistKeys.kindOf(making),
      );
      var key = '';
      for (final (j, l) in lines.indexed) {
        key = LineOrder.after(key);
        final line = writeLine(
          this,
          id,
          making,
          text: l.text,
          order: key,
          indent: !n.shopping && l.indent && j > 0,
        );
        if (n.shopping) {
          append(
            line,
            ShoppingKeys.reappear,
            (l.reappear ?? Reappear.fallback).code,
          );
        }
      }
    }
    final fieldIds = [for (final _ in n.fields) Store.newId()];
    for (final (j, f) in n.fields.indexed) {
      final field = fieldIds[j];
      append(id, TallyKeys.field(field, TallyKeys.name), f.name.trim());
      append(id, TallyKeys.field(field, TallyKeys.unit), f.unit.trim());
      append(id, TallyKeys.field(field, TallyKeys.rule), f.rule.name);
      if (f.target case final t?) {
        append(id, TallyKeys.field(field, TallyKeys.target), t);
      }
      if (f.hidden) {
        append(id, TallyKeys.field(field, TallyKeys.hidden), 'true');
      }
      if (f.check) {
        append(id, TallyKeys.field(field, TallyKeys.kind), TallyKeys.kindCheck);
      }
    }
    for (final (j, f) in n.fields.indexed) {
      final m = f.multiplies;
      if (m == null || m == j) continue;
      append(
        id,
        TallyKeys.field(fieldIds[j], TallyKeys.multiplies),
        fieldIds[m],
      );
    }
    if (n.fields.isNotEmpty) {
      if (n.bar case final bar?) append(id, TallyKeys.bar, bar.name);
      if (n.sets) append(id, TallyKeys.sets, 'true');
    }
  }

  /// Writes a Routine or Reminder of the Note [noteId] on [schedule].
  void _writeRoutine(String noteId, Schedule schedule) {
    final id = 'routine:${Store.newId()}';
    append(id, RoutineKeys.type, RoutineKeys.typeRoutine);
    append(id, RoutineKeys.note, noteId);
    append(id, RoutineKeys.start, schedule.start.toString());
    append(id, RoutineKeys.times, schedule.times.join(','));
    if (schedule.rule case final rule?) {
      append(id, RoutineKeys.rule, rule.format());
    }
  }

  /// The "From …" badge of the Note [noteId] (Q463): the sender's
  /// Profile id and the name it shows with, the user's Override or the
  /// name kept here when their Profile is kept, else the name the Pack
  /// carried; null when it has none, or it was removed. Only what the
  /// user's own Devices wrote counts ([currentByOwn], Q554 (a)): another
  /// person's Device in a Shared Area can neither set nor change it.
  ({String id, String name})? packFrom(String noteId) {
    try {
      if (jsonDecode(currentByOwn(noteId, PackKeys.from) ?? '') case {
        'id': final String id,
        'name': final String name,
      }) {
        final kept = keptPeople.any((p) => resolvePerson(p.id) == id);
        return (id: id, name: kept ? personShown(id).name : name);
      }
    } on FormatException {
      // None.
    }
    return null;
  }

  /// Removes the "From …" badge of the Note [noteId].
  void removePackFrom(String noteId) {
    if (currentByOwn(noteId, PackKeys.from) == null) return;
    if (!mayChange(noteId)) throw StateError('may not change $noteId');
    append(noteId, PackKeys.from, null);
  }
}

/// [name], or with ` (<suffix>)` after it when [taken] holds it, or the
/// first of ` (<suffix> 2)`, ` (<suffix> 3)` and on that is free.
String _landing(String name, String suffix, Set<String> taken) {
  if (!taken.contains(name)) return name;
  final first = '$name ($suffix)';
  if (!taken.contains(first)) return first;
  for (var n = 2; ; n++) {
    final candidate = '$name ($suffix $n)';
    if (!taken.contains(candidate)) return candidate;
  }
}

/// Whether the Reminder [s] is past on [today] at the wall clock [wall]:
/// on a day before, or today at a time before this minute.
bool _past(Schedule s, Day today, DateTime wall) {
  if (s.start < today) return true;
  if (today < s.start) return false;
  final time = s.times.isEmpty ? null : s.times.first;
  if (time == null) return false;
  return time.hour * 60 + time.minute < wall.hour * 60 + wall.minute;
}

/// The Routine [s] starting on [today] (Q464), its times and rule kept.
Schedule _from(Schedule s, Day today) =>
    Schedule(start: today, times: s.times, rule: s.rule);
