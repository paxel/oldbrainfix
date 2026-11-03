import 'dart:typed_data';

import 'checklists.dart';
import 'notes.dart';
import 'places.dart';
import 'profiles.dart';
import 'shopping_lists.dart';
import 'store.dart';
import 'text_format.dart';

/// What the help Notes say, in the language of the Device that makes
/// them (#180); the app builds it from its strings and the What's new it
/// bundles.
class HelpTexts {
  /// The Area's name, "Old Brain Fix".
  final String areaName;

  /// "Quick start" and its Lines, the steps to try.
  final String quickStartTitle;
  final List<String> quickStartLines;

  /// "What the icons mean" in full, its title the first line, and the
  /// pictures of the icons, one per group.
  final String iconsText;
  final List<Uint8List> iconPictures;

  /// The title of a What's new Note for a version, "What's new in 0.4.0".
  final String Function(String version) whatsNewTitle;

  const HelpTexts({
    required this.areaName,
    required this.quickStartTitle,
    required this.quickStartLines,
    required this.iconsText,
    required this.iconPictures,
    required this.whatsNewTitle,
  });
}

/// The app's own Notes that explain it (#180, ADR 0042): "Quick start",
/// "What the icons mean" and one "What's new in X.Y.Z" per release, filed
/// into their own Area "Old Brain Fix". Each has an id fixed by the
/// person and the version, so a second Device of the same person finds
/// it made and makes none, two Devices that make it apart write the same
/// entities, and a Note the person deleted stays deleted.
class HelpNotes {
  /// The version before the first one that makes help Notes: an update
  /// from a Device that recorded none passed every version after it.
  static const since = '0.3.1';

  /// The local setting holding the version this Device last made the
  /// help Notes for.
  static const versionSetting = 'helpVersion';

  /// The Area's look: the app's teal and a brain, as its icon.
  static const areaColour = '00796B';
  static const areaIcon = 'brain';

  final Store store;

  HelpNotes(this.store);

  /// The Area every help Note of the person [person] is filed into.
  static String areaId(String person) => 'area:help-$person';

  static String quickStartId(String person) => 'note:help-$person-quick-start';

  static String iconsId(String person) => 'note:help-$person-icons';

  static String whatsNewId(String person, String version) =>
      'note:help-$person-whats-new-$version';

  /// The [index]th Line of the Quick start, from 0.
  static String quickStartLineId(String person, int index) =>
      '${ChecklistKeys.linePrefix}help-$person-quick-start-$index';

  /// The making of the Quick start's Checklist.
  static String quickStartMaking(String person) => 'help-$person-quick-start';

  /// Makes the help Notes due on this Device running [version], in
  /// [language] with [texts]; [fresh] when this start began with the
  /// first start (a new person, a Device added or a Backup restored).
  /// [whatsNew] holds each release's Play notes by version and language.
  ///
  /// Quick start and What the icons mean are made once per person. Each
  /// What's new of a version after the one this Device last ran (or
  /// after [since], for a Device that recorded none, unless [fresh]) up
  /// to [version] is made once per person, oldest first. Nothing is made
  /// before the own Profile exists; then the version is recorded and the
  /// same start again makes nothing. Returns the ids of the Notes made.
  List<String> makeDue({
    required String version,
    required bool fresh,
    required String language,
    required HelpTexts texts,
    required Map<String, Map<String, String>> whatsNew,
  }) {
    final last = store.localSetting(versionSetting);
    if (last == version) return const [];
    final profile = store.ownProfile;
    if (profile == null) return const [];
    final person = profile.id;
    final persons = [person, ...store.ownProfileAliases.toList()..sort()];
    final from = last ?? (fresh ? version : since);
    final due = [
      for (final v in whatsNew.keys)
        if (compareVersions(v, from) > 0 && compareVersions(v, version) <= 0) v,
    ]..sort(compareVersions);
    final made = <String>[];
    store.batch(() {
      String? area;
      String areaFor() => area ??= _area(person, persons, texts.areaName);
      if (!_made(persons, quickStartId)) {
        _quickStart(person, areaFor(), texts);
        made.add(quickStartId(person));
      }
      if (!_made(persons, iconsId)) {
        _note(iconsId(person), areaFor(), texts.iconsText, texts.iconPictures);
        made.add(iconsId(person));
      }
      for (final v in due) {
        if (_made(persons, (p) => whatsNewId(p, v))) continue;
        final notes = whatsNew[v]!;
        final text = notes[language] ?? notes['en'];
        if (text == null) continue;
        _note(
          whatsNewId(person, v),
          areaFor(),
          '${texts.whatsNewTitle(v)}\n\n$text',
          const [],
        );
        made.add(whatsNewId(person, v));
      }
      store.setLocalSetting(versionSetting, version);
    });
    return made;
  }

  /// Whether one of the user's own Devices ever wrote the Note [id] makes
  /// for one of [persons]: in the trash or purged, it counts as made.
  bool _made(List<String> persons, String Function(String person) id) {
    final own = <String, bool>{};
    for (final person in persons) {
      for (final e in store.loggedHistory(id(person))) {
        if (own.putIfAbsent(e.device, () => store.writtenByOwn(e.device))) {
          return true;
        }
      }
    }
    return false;
  }

  /// The Area the help Notes go into: the person's own, made now when
  /// it never was, and made again with the look it had when the user
  /// deleted it, so no help Note is ever Unsorted.
  String _area(String person, List<String> persons, String name) {
    final places = Places(store);
    for (final p in persons) {
      if (places.areaById(areaId(p)) != null) return areaId(p);
    }
    final id = areaId(person);
    final never = store.loggedHistory(id).isEmpty;
    final now = store.clock().toUtc();
    store.append(id, PlaceKeys.type, PlaceKeys.typeArea, date: now);
    if (never) {
      store.append(id, PlaceKeys.name, name, date: now);
      store.append(id, PlaceKeys.colour, areaColour, date: now);
      store.append(id, PlaceKeys.icon, areaIcon, date: now);
    }
    return id;
  }

  /// Writes the plain Note [id] with [text] and [photos], filed into
  /// [area], with no Nudge.
  void _note(String id, String area, String text, List<Uint8List> photos) {
    final now = store.clock().toUtc();
    final hashes = [for (final bytes in photos) store.putPhoto(bytes)];
    store.append(id, NoteKeys.type, NoteKeys.typeNote, date: now);
    store.append(id, NoteKeys.captured, now.toIso8601String(), date: now);
    store.append(
      id,
      NoteKeys.text,
      StoredText.of(TextFormats.plain, text).encode(),
      date: now,
    );
    store.append(id, NoteKeys.place, area, date: now);
    for (final hash in hashes) {
      store.append(id, NoteKeys.photo(hash), NoteKeys.photoAdded, date: now);
    }
  }

  /// Writes the Quick start: a Checklist whose Lines are the steps to try,
  /// under ids fixed by the person and the Line's place, shown on the
  /// Agenda as one row with its count.
  void _quickStart(String person, String area, HelpTexts texts) {
    final id = quickStartId(person);
    final making = quickStartMaking(person);
    final now = store.clock().toUtc();
    _note(id, area, texts.quickStartTitle, const []);
    store.append(
      id,
      ChecklistKeys.kind,
      ChecklistKeys.kindOf(making),
      date: now,
    );
    store.append(id, NoteKeys.spawned, now.toIso8601String(), date: now);
    var key = '';
    for (final (i, line) in texts.quickStartLines.indexed) {
      key = LineOrder.after(key);
      writeLine(
        store,
        id,
        making,
        text: line,
        order: key,
        date: now,
        id: quickStartLineId(person, i),
      );
    }
    // One row on the Agenda with its count (Q549), the person's own
    // choice as "Show on the Agenda" writes it.
    store.append(
      '${ChecklistKeys.agendaPrefix}$id',
      ChecklistKeys.agenda,
      ListOnAgenda.count.code,
      date: now,
    );
  }
}

/// Orders the versions [a] and [b], `X.Y.Z` each, by their numbers; a
/// part that is no number counts as 0.
int compareVersions(String a, String b) {
  List<int> parts(String v) => [
    for (final p in v.split('.')) int.tryParse(p) ?? 0,
  ];
  final x = parts(a);
  final y = parts(b);
  for (var i = 0; i < x.length || i < y.length; i++) {
    final d = (i < x.length ? x[i] : 0) - (i < y.length ? y[i] : 0);
    if (d != 0) return d.sign;
  }
  return 0;
}
