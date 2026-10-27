import 'dart:convert';
import 'dart:typed_data';

import 'named_places.dart';
import 'places.dart';
import 'shared_areas.dart';
import 'store.dart';
import 'sync.dart';
import 'text_format.dart';

/// Field keys of a Note.
abstract final class NoteKeys {
  static const type = r'$type';
  static const typeNote = 'note';

  /// The Note's text with its format, as [StoredText.encode] writes it.
  static const text = 'text';
  static const captured = 'captured';

  /// Where the Note was captured, as [Coordinates.encode] writes it;
  /// absent while unknown.
  static const location = 'location';

  /// The Area or Topic the Note is filed in; absent while Unsorted.
  static const place = 'place';

  /// The Note's [Importance] by name; absent means normal.
  static const importance = 'importance';

  /// When the Note was moved to the trash; absent while it is not there.
  static const trashed = 'trashed';

  /// The Note this one was merged into; absent while it stands alone.
  static const merged = 'merged';

  /// When the Note first spawned a Routine or Reminder; absent before.
  /// Kept when its Routines are gone, so the Note stays out of Unsorted:
  /// spawning was a conscious decision (CONTEXT.md). Only sending the
  /// Notes of a deleted place back to Unsorted clears it.
  static const spawned = 'spawned';

  /// When the Note's Nudge is due, chosen at Capture; absent means
  /// [Notes.defaultNudge] after its capture.
  static const nudge = 'nudge';

  /// One field per photo, `photo:<hash>` = [photoAdded], so photos added
  /// on two Devices at once are both kept. The bytes are in the store's
  /// photos, under the hash ([Store.photo]). Null once the photo was
  /// removed and its Photo Text kept (ADR 0020).
  static const photoPrefix = 'photo:';
  static const photoAdded = 'added';

  static String photo(String hash) => '$photoPrefix$hash';

  /// One field per voice recording, `audio:<hash>` = [audioAdded], as for
  /// photos. The bytes are in the store's audio, under the hash
  /// ([Store.audio]). Null once the recording was removed and its
  /// Transcript kept (ADR 0020).
  static const audioPrefix = 'audio:';
  static const audioAdded = 'added';

  static String audio(String hash) => '$audioPrefix$hash';

  /// The Transcript of a voice recording, `transcript:<hash>` = its text,
  /// on the Note holding `audio:<hash>`, next to the recording and never
  /// in its place (CONTEXT.md). An empty text means Whisper heard no
  /// words; absent means not transcribed (yet).
  static const transcriptPrefix = 'transcript:';

  static String transcript(String hash) => '$transcriptPrefix$hash';

  /// The Photo Text of a photo, `ocr:<hash>` = the text ocrs recognised
  /// in it, on the Note holding `photo:<hash>`, next to the photo and
  /// never in its place (CONTEXT.md). An empty text means no text was
  /// found; absent means not read (yet).
  static const ocrPrefix = 'ocr:';

  static String ocr(String hash) => '$ocrPrefix$hash';

  /// The app something was shared in from, as [SourceApp.package] and
  /// [SourceApp.label]; absent for anything not shared in, or when the
  /// platform did not say.
  static const sourcePackage = 'sourcePackage';
  static const sourceLabel = 'sourceLabel';

  /// The titles of the calendar events running at Capture, as a JSON
  /// list; absent while unknown or off.
  static const calendar = 'calendar';

  /// The names of the Bluetooth devices connected at Capture, as a JSON
  /// list; absent while unknown or off.
  static const bluetooth = 'bluetooth';
}

/// Field keys of a Section.
abstract final class SectionKeys {
  static const type = r'$type';
  static const typeSection = 'section';

  /// The Note the Section belongs to.
  static const note = 'note';

  /// The Section's text with its format, as for [NoteKeys.text].
  static const text = 'text';
  static const captured = 'captured';

  /// Where the Section was captured, as for [NoteKeys.location].
  static const location = 'location';

  /// The calendar events and Bluetooth devices at its capture, as for
  /// [NoteKeys.calendar] and [NoteKeys.bluetooth].
  static const calendar = NoteKeys.calendar;
  static const bluetooth = NoteKeys.bluetooth;
}

/// How much a Note matters (CONTEXT.md).
enum Importance { unimportant, normal, important }

/// What a Note without words is titled as where only its words fit, as
/// in a row or an alarm.
enum Wordless { photo, voiceRecording }

/// The app something was shared in from (CONTEXT.md, Capture Context).
class SourceApp {
  /// Its Android package, such as `com.whatsapp`.
  final String package;

  /// Its name as the Device showed it at capture time, or null when the
  /// Device could not tell.
  final String? label;

  const SourceApp(this.package, {this.label});

  /// What to show: the label, else the package.
  String get name {
    final label = this.label?.trim() ?? '';
    return label.isEmpty ? package : label;
  }

  @override
  bool operator ==(Object other) =>
      other is SourceApp && other.package == package && other.label == label;

  @override
  int get hashCode => Object.hash(package, label);
}

/// A calendar event the platform found running at capture time.
typedef RunningEvent = ({String title, bool allDay});

/// The titles of [events] worth keeping: the timed ones, or the all-day
/// ones when nothing else runs, as a birthday says little about the
/// moment. Blank and repeated titles are dropped.
List<String> runningEventTitles(Iterable<RunningEvent> events) {
  List<String> titles(bool allDay) => {
    for (final event in events)
      if (event.allDay == allDay && event.title.trim().isNotEmpty)
        event.title.trim(),
  }.toList();
  final timed = titles(false);
  return timed.isNotEmpty ? timed : titles(true);
}

/// What was true when a Note or Section was captured (CONTEXT.md): the
/// time, where it was, the app it was shared in from, the calendar
/// events running and the Bluetooth devices connected. All of it stays
/// in the Note, on the user's Devices.
class CaptureContext {
  final DateTime captured;

  /// Where it was captured, or null when that is unknown: denied,
  /// unavailable, or not arrived yet.
  final Coordinates? location;

  /// The Named Place [location] lies in, resolved when read, so a
  /// renamed, deleted or new Named Place shows on Notes captured before.
  final NamedPlace? namedPlace;

  /// The app it was shared in from, or null when it was not shared in or
  /// the platform did not say.
  final SourceApp? sourceApp;

  /// The titles of the calendar events running at capture; empty when
  /// none, unknown or off.
  final List<String> calendarEvents;

  /// The names of the Bluetooth devices connected at capture; empty when
  /// none, unknown or off.
  final List<String> bluetoothDevices;

  const CaptureContext({
    required this.captured,
    this.location,
    this.namedPlace,
    this.sourceApp,
    this.calendarEvents = const [],
    this.bluetoothDevices = const [],
  });

  /// Whether there is more than the time and place a card shows (UI
  /// laws, "Cards").
  bool get hasMore =>
      sourceApp != null ||
      calendarEvents.isNotEmpty ||
      bluetoothDevices.isNotEmpty;
}

/// A Note as the screens want it (CONTEXT.md).
class Note {
  /// Entity id, `note:<id>`.
  final String id;

  /// The text, the source text when its format is not known here.
  final String text;

  /// The format [text] is in ([TextFormats]).
  final String textFormat;

  /// The version of the structure [text] is stored in ([StoredText]).
  final int textVersion;

  /// Whether this version may change [text]: never one in a format it
  /// does not know, which it shows as its source text (ADR 0018).
  final bool textEditable;
  final CaptureContext context;

  /// The Area or Topic the Note is filed in, or null when it has none.
  final String? place;
  final Importance importance;

  /// When the Note was moved to the trash, or null when it is not there.
  final DateTime? trashed;

  /// The Note this one was merged into and now is a Section of, or null
  /// when it stands alone.
  final String? mergedInto;

  /// Whether the Note ever spawned a Routine or Reminder.
  final bool spawned;

  /// When its Nudge was asked for at Capture, or null when it was not.
  final DateTime? nudge;

  /// The hashes of its photos, in the order they were added.
  final List<String> photos;

  /// The hashes of its voice recordings, in the order they were added.
  final List<String> recordings;

  /// The Transcripts of its voice recordings, by recording hash; a
  /// recording not transcribed yet has none.
  final Map<String, String> transcripts;

  /// The Photo Texts of its photos, by photo hash; a photo not read yet
  /// has none.
  final Map<String, String> photoTexts;

  /// The hashes of its photos that were removed while their Photo Text
  /// stays, in the order they were added; the text is in [photoTexts].
  final List<String> removedPhotos;

  /// The hashes of its voice recordings that were removed while their
  /// Transcript stays, in the order they were added; the text is in
  /// [transcripts].
  final List<String> removedRecordings;

  const Note({
    required this.id,
    required this.text,
    required this.context,
    this.textFormat = TextFormats.plain,
    this.textVersion = TextFormats.version,
    this.textEditable = true,
    this.place,
    this.importance = Importance.normal,
    this.trashed,
    this.mergedInto,
    this.spawned = false,
    this.nudge,
    this.photos = const [],
    this.recordings = const [],
    this.transcripts = const {},
    this.photoTexts = const {},
    this.removedPhotos = const [],
    this.removedRecordings = const [],
  });

  /// When the Note was captured.
  DateTime get captured => context.captured;

  bool get isTrashed => trashed != null;

  bool get isMerged => mergedInto != null;

  /// Not yet filed into an Area or Topic, not merged, not deleted and
  /// without a Routine or Reminder spawned from it (CONTEXT.md).
  bool get isUnsorted => place == null && !isTrashed && !isMerged && !spawned;

  /// When the Note's Nudge is due while it is Unsorted: the time chosen
  /// at Capture, else [Notes.defaultNudge] after it was captured.
  DateTime get nudgeAt => nudge ?? captured.add(Notes.defaultNudge);

  /// What the Note is titled as while its text is blank: a voice
  /// recording when it has one and no photo, removed ones whose texts
  /// stay included; a photo otherwise.
  Wordless get wordless =>
      photos.isEmpty &&
          removedPhotos.isEmpty &&
          (recordings.isNotEmpty || removedRecordings.isNotEmpty)
      ? Wordless.voiceRecording
      : Wordless.photo;
}

/// A dated part of a Note, added by Amend or Merge (CONTEXT.md).
class Section {
  /// Entity id: `section:<id>` for an Amend, the merged Note's
  /// `note:<id>` for a Merge.
  final String id;
  final String noteId;

  /// The text, the source text when its format is not known here.
  final String text;

  /// The format [text] is in, the version of its structure and whether
  /// this version may change it, as for [Note.textFormat],
  /// [Note.textVersion] and [Note.textEditable].
  final String textFormat;
  final int textVersion;
  final bool textEditable;
  final CaptureContext context;

  /// The hashes of its photos: a merged Note's photos travel with it.
  final List<String> photos;

  /// The hashes of its voice recordings: a merged Note's recordings
  /// travel with it.
  final List<String> recordings;

  /// The Transcripts of [recordings], by recording hash, as for
  /// [Note.transcripts].
  final Map<String, String> transcripts;

  /// The Photo Texts of [photos], by photo hash, as for
  /// [Note.photoTexts].
  final Map<String, String> photoTexts;

  /// The photos and voice recordings removed while their texts stay, as
  /// for [Note.removedPhotos] and [Note.removedRecordings].
  final List<String> removedPhotos;
  final List<String> removedRecordings;

  const Section({
    required this.id,
    required this.noteId,
    required this.text,
    required this.context,
    this.textFormat = TextFormats.plain,
    this.textVersion = TextFormats.version,
    this.textEditable = true,
    this.photos = const [],
    this.recordings = const [],
    this.transcripts = const {},
    this.photoTexts = const {},
    this.removedPhotos = const [],
    this.removedRecordings = const [],
  });
}

/// How a diary of cards is ordered.
enum DiaryOrder {
  /// Newest capture first.
  time,

  /// Important first, unimportant last; newest capture first within each.
  importance,
}

/// [notes] as a diary in [order], without the unimportant ones when
/// [hideUnimportant] is set.
List<Note> diary(
  Iterable<Note> notes, {
  DiaryOrder order = DiaryOrder.time,
  bool hideUnimportant = false,
}) {
  final shown = [
    for (final note in notes)
      if (!hideUnimportant || note.importance != Importance.unimportant) note,
  ];
  shown.sort((a, b) {
    if (order == DiaryOrder.importance) {
      final byImportance = b.importance.index.compareTo(a.importance.index);
      if (byImportance != 0) return byImportance;
    }
    return b.captured.compareTo(a.captured);
  });
  return shown;
}

/// Capturing, changing and reading Notes.
class Notes {
  /// How long a deleted Note stays in the trash before it is purged.
  static const trashKeep = Duration(days: 30);

  /// How long after Capture the Nudge comes when none was picked (UI
  /// laws, "Capture").
  static const defaultNudge = Duration(hours: 1);

  final Store store;

  Notes(this.store);

  /// Captures a new Note at the store's now with [text], [photos] and
  /// voice [recordings], filed into the Area or Topic [place] when one is
  /// given, nudged [nudgeIn] from now while it is Unsorted. The text may
  /// be empty for a photo or voice Note. The photos are kept without
  /// their metadata. A Note shared in from another app records it as
  /// [sourceApp].
  Note capture(
    String text, {
    String? place,
    Duration? nudgeIn,
    List<Uint8List> photos = const [],
    List<Uint8List> recordings = const [],
    SourceApp? sourceApp,
  }) {
    if (place != null && !Places(store).isPlace(place)) {
      throw ArgumentError.value(place, 'place', 'not an Area or Topic');
    }
    if (place != null && !store.mayWriteIn(place)) {
      throw StateError('no role in the Shared Area of $place');
    }
    final id = 'note:${Store.newId()}';
    final now = store.clock().toUtc();
    // The bytes are kept before the log names them.
    final hashes = [for (final bytes in photos) store.putPhoto(bytes)];
    final audio = [for (final bytes in recordings) store.putAudio(bytes)];
    store.batch(() {
      store.append(id, NoteKeys.type, NoteKeys.typeNote, date: now);
      store.append(id, NoteKeys.captured, now.toIso8601String(), date: now);
      store.append(
        id,
        NoteKeys.text,
        StoredText.plain(text).encode(),
        date: now,
      );
      if (place != null) store.append(id, NoteKeys.place, place, date: now);
      if (nudgeIn != null) {
        final due = now.add(nudgeIn).toIso8601String();
        store.append(id, NoteKeys.nudge, due, date: now);
      }
      for (final hash in hashes) {
        store.append(id, NoteKeys.photo(hash), NoteKeys.photoAdded, date: now);
      }
      for (final hash in audio) {
        store.append(id, NoteKeys.audio(hash), NoteKeys.audioAdded, date: now);
      }
      if (sourceApp != null) {
        final app = sourceApp;
        store.append(id, NoteKeys.sourcePackage, app.package, date: now);
        if (app.label case final label?) {
          store.append(id, NoteKeys.sourceLabel, label, date: now);
        }
      }
    });
    return byId(id)!;
  }

  /// Adds the photo [bytes] to the Note [noteId], without their metadata;
  /// returns the hash it is kept under.
  String addPhoto(String noteId, Uint8List bytes) {
    _requireNote(noteId);
    final hash = store.putPhoto(bytes);
    store.append(noteId, NoteKeys.photo(hash), NoteKeys.photoAdded);
    return hash;
  }

  /// The bytes of the photo [hash], or null when they are not on this
  /// Device (yet).
  Uint8List? photoBytes(String hash) => store.photo(hash);

  /// Adds the voice recording [bytes] to the Note [noteId]; returns the
  /// hash it is kept under.
  String addRecording(String noteId, Uint8List bytes) {
    _requireNote(noteId);
    final hash = store.putAudio(bytes);
    store.append(noteId, NoteKeys.audio(hash), NoteKeys.audioAdded);
    return hash;
  }

  /// The bytes of the voice recording [hash], or null when they are not
  /// on this Device (yet).
  Uint8List? recordingBytes(String hash) => store.audio(hash);

  /// Keeps [text] as the Transcript of the voice recording [hash] of the
  /// Note [noteId], a merged one included, next to the recording; the
  /// recording stays as it is. An empty [text] records that no words
  /// were heard, so the recording is not transcribed again.
  void setTranscript(String noteId, String hash, String text) {
    if (byId(noteId)?.recordings.contains(hash) != true) {
      throw ArgumentError.value(hash, 'hash', 'not a recording of $noteId');
    }
    store.append(noteId, NoteKeys.transcript(hash), text.trim());
  }

  /// The voice recordings without a Transcript, newest capture first:
  /// those of every Note not in the trash, merged ones included.
  List<({String noteId, String hash})> untranscribed() {
    final notes = [
      for (final id in store.entitiesOfType(NoteKeys.typeNote))
        if (byId(id) case final note? when !note.isTrashed)
          if (note.recordings.any((h) => !note.transcripts.containsKey(h)))
            note,
    ];
    notes.sort((a, b) => b.captured.compareTo(a.captured));
    return [
      for (final note in notes)
        for (final hash in note.recordings)
          if (!note.transcripts.containsKey(hash))
            (noteId: note.id, hash: hash),
    ];
  }

  /// Keeps [text] as the Photo Text of the photo [hash] of the Note
  /// [noteId], a merged one included, next to the photo; the photo stays
  /// as it is. An empty [text] records that no text was found, so the
  /// photo is not read again.
  void setPhotoText(String noteId, String hash, String text) {
    if (byId(noteId)?.photos.contains(hash) != true) {
      throw ArgumentError.value(hash, 'hash', 'not a photo of $noteId');
    }
    store.append(noteId, NoteKeys.ocr(hash), text.trim());
  }

  /// The photos without a Photo Text, newest capture first: those of
  /// every Note not in the trash, merged ones included.
  List<({String noteId, String hash})> unreadPhotos() {
    final notes = [
      for (final id in store.entitiesOfType(NoteKeys.typeNote))
        if (byId(id) case final note? when !note.isTrashed)
          if (note.photos.any((h) => !note.photoTexts.containsKey(h))) note,
    ];
    notes.sort((a, b) => b.captured.compareTo(a.captured));
    return [
      for (final note in notes)
        for (final hash in note.photos)
          if (!note.photoTexts.containsKey(hash)) (noteId: note.id, hash: hash),
    ];
  }

  /// Whether this Device may remove the photo [hash] of the Note
  /// [noteId], a merged one included, and keep its Photo Text: the photo
  /// has one that is not empty, and the Note is this Device's to change
  /// ([SharedAreas.mayChange]).
  bool mayRemovePhoto(String noteId, String hash) {
    final note = byId(noteId);
    return note != null &&
        note.photos.contains(hash) &&
        _hasText(note.photoTexts[hash]) &&
        store.mayChange(noteId);
  }

  /// Whether this Device may remove the voice recording [hash] of the
  /// Note [noteId], a merged one included, and keep its Transcript, as
  /// for [mayRemovePhoto].
  bool mayRemoveRecording(String noteId, String hash) {
    final note = byId(noteId);
    return note != null &&
        note.recordings.contains(hash) &&
        _hasText(note.transcripts[hash]) &&
        store.mayChange(noteId);
  }

  /// Removes the photo [hash] from the Note [noteId] and keeps its Photo
  /// Text, which search still finds (ADR 0020). The edit syncs like any
  /// other; every Device lets the bytes go once no Note refers to them.
  /// Refused unless [mayRemovePhoto].
  void removePhoto(String noteId, String hash) {
    if (byId(noteId)?.photos.contains(hash) != true) {
      throw ArgumentError.value(hash, 'hash', 'not a photo of $noteId');
    }
    if (!mayRemovePhoto(noteId, hash)) {
      throw StateError('the photo $hash of $noteId may not be removed here');
    }
    store.append(noteId, NoteKeys.photo(hash), null);
    store.dropRemovedBlobs();
  }

  /// Removes the voice recording [hash] from the Note [noteId] and keeps
  /// its Transcript, as [removePhoto] does. Refused unless
  /// [mayRemoveRecording].
  void removeRecording(String noteId, String hash) {
    if (byId(noteId)?.recordings.contains(hash) != true) {
      throw ArgumentError.value(hash, 'hash', 'not a recording of $noteId');
    }
    if (!mayRemoveRecording(noteId, hash)) {
      throw StateError(
        'the recording $hash of $noteId may not be removed here',
      );
    }
    store.append(noteId, NoteKeys.audio(hash), null);
    store.dropRemovedBlobs();
  }

  /// Whether a Transcript or Photo Text says anything: an empty one
  /// means nothing was found.
  static bool _hasText(String? text) => text != null && text.trim().isNotEmpty;

  /// Files the Note [noteId] into the Area or Topic [placeId], out of
  /// wherever it was before.
  void file(String noteId, String placeId) {
    _requireNote(noteId);
    if (!Places(store).isPlace(placeId)) {
      throw ArgumentError.value(placeId, 'placeId', 'not an Area or Topic');
    }
    store.append(noteId, NoteKeys.place, placeId);
  }

  /// Nudges the Note [noteId] [nudgeIn] from now instead of when it was
  /// set to before.
  void setNudge(String noteId, Duration nudgeIn) {
    _requireNote(noteId);
    final due = store.clock().toUtc().add(nudgeIn);
    store.append(noteId, NoteKeys.nudge, due.toIso8601String());
  }

  /// Fixes the text of the Note [noteId] in place. The old text stays in
  /// the log only. A text in a format this version does not know is
  /// never overwritten (ADR 0018).
  void edit(String noteId, String text) {
    final note = byId(noteId);
    if (note == null) {
      throw ArgumentError.value(noteId, 'noteId', 'not a Note');
    }
    if (!note.textEditable) {
      throw StateError('the text of $noteId is in a format not known here');
    }
    store.append(noteId, NoteKeys.text, StoredText.plain(_text(text)).encode());
  }

  /// When the text of the Note [noteId] was last set, by capture or edit;
  /// null when there is no such Note.
  DateTime? textSetAt(String noteId) {
    DateTime? latest;
    for (final e in store.history(noteId)) {
      if (e.field != NoteKeys.text) continue;
      if (latest == null || e.date.isAfter(latest)) latest = e.date;
    }
    return latest;
  }

  /// Amends the Note [noteId] with a new Section holding [text], captured
  /// at the store's now.
  Section amend(String noteId, String text) {
    _requireNote(noteId);
    if (!store.mayWriteIn(noteId)) {
      throw StateError('no role in the Shared Area of $noteId');
    }
    final trimmed = _text(text);
    final id = 'section:${Store.newId()}';
    final now = store.clock().toUtc();
    store.batch(() {
      store.append(id, SectionKeys.type, SectionKeys.typeSection, date: now);
      store.append(id, SectionKeys.note, noteId, date: now);
      store.append(id, SectionKeys.captured, now.toIso8601String(), date: now);
      store.append(
        id,
        SectionKeys.text,
        StoredText.plain(trimmed).encode(),
        date: now,
      );
    });
    return _section(id)!;
  }

  /// Merges the Note [noteId] into the Note [intoId]: it becomes a dated
  /// Section of it, with its own Capture Context, and leaves every list
  /// and Unsorted.
  ///
  /// Nothing is copied: the merged Note keeps its entity and only points
  /// at its target, which shows it as a Section. Whatever belongs to it,
  /// its Sections and whatever reaches it from other Devices after the
  /// Merge included, travels along.
  void merge(String noteId, String intoId) {
    for (final (id, name) in [(noteId, 'noteId'), (intoId, 'intoId')]) {
      final note = byId(id);
      if (note == null || note.isTrashed || note.isMerged) {
        throw ArgumentError.value(id, name, 'not a Note in the lists');
      }
    }
    if (noteId == intoId) {
      throw ArgumentError.value(intoId, 'intoId', 'the Note itself');
    }
    store.append(noteId, NoteKeys.merged, intoId);
  }

  /// The Sections of the Note [noteId], oldest first: its Amends, the
  /// Notes merged into it, and their Amends.
  List<Section> sections(String noteId) {
    final merged = {
      for (final id in store.entitiesOfType(NoteKeys.typeNote))
        if (_mergedInto(id) == noteId) id,
    };
    final owners = {noteId, ...merged};
    final sections = [
      for (final id in merged)
        if (byId(id) case final note?)
          Section(
            id: id,
            noteId: noteId,
            text: note.text,
            textFormat: note.textFormat,
            textVersion: note.textVersion,
            textEditable: note.textEditable,
            context: note.context,
            photos: note.photos,
            recordings: note.recordings,
            transcripts: note.transcripts,
            photoTexts: note.photoTexts,
            removedPhotos: note.removedPhotos,
            removedRecordings: note.removedRecordings,
          ),
      for (final id in store.entitiesOfType(SectionKeys.typeSection))
        if (owners.contains(store.current(id, SectionKeys.note)))
          ?_section(id, noteId: noteId),
    ];
    sections.sort((a, b) => a.context.captured.compareTo(b.context.captured));
    return sections;
  }

  /// Records where the Note or Section [id] was captured. The position
  /// arrives after Capture, so it is attached later and never holds the
  /// Note back.
  void locate(String id, Coordinates location) {
    final field = switch (store.current(id, NoteKeys.type)) {
      NoteKeys.typeNote => NoteKeys.location,
      SectionKeys.typeSection => SectionKeys.location,
      _ => throw ArgumentError.value(id, 'id', 'not a Note or Section'),
    };
    store.append(id, field, location.encode());
  }

  /// Records the calendar events running and the Bluetooth devices
  /// connected when the Note or Section [id] was captured. Like the
  /// position they are looked up while capturing and attached when they
  /// arrive; an empty or missing list writes nothing.
  void attachContext(
    String id, {
    List<String> calendarEvents = const [],
    List<String> bluetoothDevices = const [],
  }) {
    final type = store.current(id, NoteKeys.type);
    if (type != NoteKeys.typeNote && type != SectionKeys.typeSection) {
      throw ArgumentError.value(id, 'id', 'not a Note or Section');
    }
    store.batch(() {
      if (calendarEvents.isNotEmpty) {
        store.append(id, NoteKeys.calendar, jsonEncode(calendarEvents));
      }
      if (bluetoothDevices.isNotEmpty) {
        store.append(id, NoteKeys.bluetooth, jsonEncode(bluetoothDevices));
      }
    });
  }

  /// Sets the Importance of the Note [noteId].
  void setImportance(String noteId, Importance importance) {
    _requireNote(noteId);
    store.append(noteId, NoteKeys.importance, importance.name);
  }

  /// Moves the Note [noteId] to the trash: it leaves every list and
  /// Unsorted, and can be restored until [trashKeep] has passed.
  void trash(String noteId) {
    _requireNote(noteId);
    store.append(
      noteId,
      NoteKeys.trashed,
      store.clock().toUtc().toIso8601String(),
    );
  }

  /// Brings the Note [noteId] back from the trash, to where it was filed.
  void restore(String noteId) {
    _requireNote(noteId);
    store.append(noteId, NoteKeys.trashed, null);
  }

  /// When the trashed [note] is purged; null when it is not in the trash.
  DateTime? purgeAt(Note note) => note.trashed?.add(trashKeep);

  /// The Notes in the trash that can still be restored, most recently
  /// deleted first.
  List<Note> trashed() {
    final now = store.clock().toUtc();
    final notes = [
      for (final id in store.entitiesOfType(NoteKeys.typeNote))
        if (byId(id) case final note?)
          if (!note.isMerged)
            if (purgeAt(note) case final at? when now.isBefore(at)) note,
    ];
    notes.sort(
      (a, b) => (b.trashed ?? b.captured).compareTo(a.trashed ?? a.captured),
    );
    return notes;
  }

  /// Purges every Note that has been in the trash for [trashKeep], with
  /// its Sections, the Notes merged into it included. The log keeps their
  /// entries; nothing shows them again.
  void purgeTrash() {
    final now = store.clock().toUtc();
    store.batch(() {
      for (final id in store.entitiesOfType(NoteKeys.typeNote)) {
        final note = byId(id);
        // A merged Note goes with its target, never on its own.
        final at = note == null || note.isMerged ? null : purgeAt(note);
        // What this Device may not change in a Shared Area waits for
        // someone who may: written here, it would be ignored every time.
        if (at == null || now.isBefore(at) || !store.mayChange(id)) continue;
        // A merged Note's Section is the Note itself; both keep their
        // type under the same key.
        for (final section in sections(id)) {
          if (store.mayChange(section.id)) {
            store.append(section.id, SectionKeys.type, null);
          }
        }
        store.append(id, NoteKeys.type, null);
      }
    });
  }

  /// The Note [id], trashed or not, or null when there is none.
  Note? byId(String id) {
    final fields = store.currentFields(id);
    if (fields[NoteKeys.type] != NoteKeys.typeNote) return null;
    final captured = fields[NoteKeys.captured];
    if (captured == null) return null;
    // A place that is gone leaves the Note Unsorted.
    final place = fields[NoteKeys.place];
    final trashed = fields[NoteKeys.trashed];
    final text = StoredText.read(fields[NoteKeys.text]);
    final transcripts = _texts(fields, NoteKeys.transcriptPrefix);
    final photoTexts = _texts(fields, NoteKeys.ocrPrefix);
    return Note(
      id: id,
      text: text.text,
      textFormat: text.format,
      textVersion: text.version,
      textEditable: text.editable,
      context: _context(captured, fields),
      place: place != null && Places(store).isPlace(place) ? place : null,
      importance:
          Importance.values.asNameMap()[fields[NoteKeys.importance]] ??
          Importance.normal,
      trashed: trashed == null ? null : DateTime.parse(trashed),
      mergedInto: _mergedInto(id),
      spawned: fields[NoteKeys.spawned] != null,
      nudge: DateTime.tryParse(fields[NoteKeys.nudge] ?? '')?.toUtc(),
      // The fields come in the order they were first written.
      photos: _blobs(fields, NoteKeys.photoPrefix, NoteKeys.photoAdded),
      recordings: _blobs(fields, NoteKeys.audioPrefix, NoteKeys.audioAdded),
      transcripts: transcripts,
      photoTexts: photoTexts,
      removedPhotos: [
        for (final hash in _blobs(fields, NoteKeys.photoPrefix, null))
          if (_hasText(photoTexts[hash])) hash,
      ],
      removedRecordings: [
        for (final hash in _blobs(fields, NoteKeys.audioPrefix, null))
          if (_hasText(transcripts[hash])) hash,
      ],
    );
  }

  /// The hashes of the fields under [prefix] whose value is [value], in
  /// the order they were first written.
  static List<String> _blobs(
    Map<String, String?> fields,
    String prefix,
    String? value,
  ) => [
    for (final MapEntry(:key, value: held) in fields.entries)
      if (key.startsWith(prefix) && held == value) key.substring(prefix.length),
  ];

  /// The texts under [prefix], by the hash they belong to.
  static Map<String, String> _texts(
    Map<String, String?> fields,
    String prefix,
  ) => {
    for (final MapEntry(:key, :value) in fields.entries)
      if (key.startsWith(prefix) && value != null)
        key.substring(prefix.length): value,
  };

  /// Every Note neither in the trash nor merged, newest capture first.
  List<Note> all() {
    final notes = [
      for (final id in store.entitiesOfType(NoteKeys.typeNote))
        if (byId(id) case final note? when !note.isTrashed && !note.isMerged)
          note,
    ];
    notes.sort((a, b) => b.captured.compareTo(a.captured));
    return notes;
  }

  /// Every Unsorted Note, oldest capture first.
  List<Note> unsorted() => [
    for (final note in all().reversed)
      if (note.isUnsorted) note,
  ];

  /// The Notes filed in the Area or Topic [placeId], newest capture
  /// first; an Area holds its Topics' Notes too.
  List<Note> inside(String placeId) {
    final ids = {
      placeId,
      for (final topic in Places(store).topics(placeId)) topic.id,
    };
    return [
      for (final note in all())
        if (ids.contains(note.place)) note,
    ];
  }

  /// The Note [id] ends up in at the end of its Merge chain, or null when
  /// it stands alone. A chain stops before a Note this Device does not
  /// hold. A loop, from Devices merging both ways before they synced,
  /// stops at its lowest id, so every Device keeps the same Note.
  String? _mergedInto(String id) {
    final chain = [id];
    while (true) {
      final next = store.current(chain.last, NoteKeys.merged);
      if (next == null ||
          store.current(next, NoteKeys.type) != NoteKeys.typeNote) {
        break;
      }
      final loop = chain.indexOf(next);
      if (loop >= 0) {
        final kept = chain
            .sublist(loop)
            .reduce((a, b) => a.compareTo(b) <= 0 ? a : b);
        return kept == id ? null : kept;
      }
      chain.add(next);
    }
    return chain.last == id ? null : chain.last;
  }

  /// The Section [id]; shown under [noteId] when given, for the Amends of
  /// a merged Note.
  Section? _section(String id, {String? noteId}) {
    final fields = store.currentFields(id);
    if (fields[SectionKeys.type] != SectionKeys.typeSection) return null;
    final owner = fields[SectionKeys.note];
    final captured = fields[SectionKeys.captured];
    if (owner == null || captured == null) return null;
    final text = StoredText.read(fields[SectionKeys.text]);
    return Section(
      id: id,
      noteId: noteId ?? owner,
      text: text.text,
      textFormat: text.format,
      textVersion: text.version,
      textEditable: text.editable,
      context: _context(captured, fields),
    );
  }

  /// The Capture Context from the stored capture time and the [fields]
  /// of a Note or Section, with the Named Place its location lies in.
  CaptureContext _context(String captured, Map<String, String?> fields) {
    // Notes and Sections keep their Capture Context under the same keys.
    final at = Coordinates.decode(fields[NoteKeys.location]);
    final package = fields[NoteKeys.sourcePackage];
    return CaptureContext(
      captured: DateTime.parse(captured),
      location: at,
      namedPlace: at == null ? null : NamedPlaces(store).at(at),
      sourceApp: package == null || package.isEmpty
          ? null
          : SourceApp(package, label: fields[NoteKeys.sourceLabel]),
      calendarEvents: _names(fields[NoteKeys.calendar]),
      bluetoothDevices: _names(fields[NoteKeys.bluetooth]),
    );
  }

  /// A stored JSON list of names; empty when absent or unreadable.
  static List<String> _names(String? json) {
    if (json == null) return const [];
    try {
      final decoded = jsonDecode(json);
      return decoded is List ? decoded.whereType<String>().toList() : const [];
    } on FormatException {
      return const [];
    }
  }

  void _requireNote(String id) {
    if (byId(id) == null) throw ArgumentError.value(id, 'noteId', 'not a Note');
  }

  static String _text(String text) {
    final trimmed = text.trim();
    if (trimmed.isEmpty) {
      throw ArgumentError.value(text, 'text', 'must not be empty');
    }
    return trimmed;
  }
}
