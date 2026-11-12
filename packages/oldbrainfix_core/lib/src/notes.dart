import 'dart:convert';
import 'dart:typed_data';

import 'boxes.dart';
import 'entry.dart';
import 'file_text.dart';
import 'named_places.dart';
import 'places.dart';
import 'profiles.dart';
import 'shared_areas.dart';
import 'store.dart';
import 'sync.dart';
import 'text_boxes.dart';
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

  /// The Note this one was merged into; absent while it stands alone,
  /// null once its Merge was undone.
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
  /// words; absent means not transcribed (yet); null means removed while
  /// the recording stays (ADR 0025).
  static const transcriptPrefix = 'transcript:';

  static String transcript(String hash) => '$transcriptPrefix$hash';

  /// The Photo Text of a photo, `ocr:<hash>` = the text RustO recognised
  /// in it, on the Note holding `photo:<hash>`, next to the photo and
  /// never in its place (CONTEXT.md). An empty text means no text was
  /// found; absent means not read (yet); null means removed while the
  /// photo stays (ADR 0025).
  static const ocrPrefix = 'ocr:';

  static String ocr(String hash) => '$ocrPrefix$hash';

  /// One field per attached file (ADR 0047), `file:<hash>` = its name,
  /// media type and size as JSON ([Attachment.value]), so files attached
  /// on two Devices at once are both kept. The bytes are in the store's
  /// files, under the hash ([Store.file]). Null once the file was removed
  /// and its Attachment Text kept, as for photos (ADR 0020).
  static const filePrefix = 'file:';

  static String file(String hash) => '$filePrefix$hash';

  /// The Attachment Text of an attached file, `fileText:<hash>` = the
  /// text read out of it, on the Note holding `file:<hash>`. An empty
  /// text means no text was found; absent means not read (yet); null
  /// means removed while the file stays (ADR 0025).
  static const fileTextPrefix = 'fileText:';

  static String fileText(String hash) => '$fileTextPrefix$hash';

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

/// The largest file that can be attached: the most a bundle, a Backup and
/// the local network carry of one (ADR 0003, 0047).
const maxFileBytes = 256 << 20;

/// The largest whole number a size or a page count is read as: what a
/// double holds exactly, so both cores read the same.
const _maxCount = 1 << 53;

/// The most characters of a file's name kept.
const _maxNameChars = 255;

/// The media type of a file no app named one for.
const unknownType = 'application/octet-stream';

/// A file attached to a Note (ADR 0047): its bytes are a blob, kept by its
/// hash; the rest is what the attaching Device wrote down. Another
/// person's Device may have written it, so it is read as data only: a
/// name without folders or control characters, a media type of the usual
/// shape, numbers that are whole.
class Attachment {
  final String hash;

  /// Its file name, without any folder; empty when none was given.
  final String name;

  /// Its media type in lower case, such as `application/pdf`;
  /// [unknownType] when none was given.
  final String mediaType;

  /// Its size in bytes.
  final int size;

  /// How many pages it has, when the attaching Device counted them.
  final int? pages;

  /// Whether only a computer reads its text (Q645,
  /// [FileTexts.computerReads]); until one has, a phone shows that the
  /// text follows from the computer.
  final bool computer;

  const Attachment({
    required this.hash,
    required this.name,
    required this.mediaType,
    required this.size,
    this.pages,
    this.computer = false,
  });

  /// What the file is, as far as its text goes.
  FileKind get kind => FileTexts.kindOf(name, mediaType);

  /// The attachment [hash] from the [value] of its field: whatever does
  /// not read is left empty, never an error.
  factory Attachment.read(String hash, String value) {
    Object? json;
    try {
      json = jsonDecode(value);
    } on FormatException {
      json = null;
    }
    final object = json is Map<String, Object?>
        ? json
        : const <String, Object?>{};
    int? count(String key) => switch (object[key]) {
      final int n when n >= 0 && n <= _maxCount => n,
      _ => null,
    };
    return Attachment(
      hash: hash,
      name: cleanName(switch (object['name']) {
        final String n => n,
        _ => '',
      }),
      mediaType: cleanType(switch (object['type']) {
        final String t => t,
        _ => '',
      }),
      size: count('size') ?? 0,
      pages: count('pages'),
      computer: object['computer'] == true,
    );
  }

  /// The value of its field: JSON, keys sorted, `computer` only when true
  /// and `pages` only when counted.
  String get value => jsonEncode({
    if (computer) 'computer': true,
    'name': name,
    'pages': ?pages,
    'size': size,
    'type': mediaType,
  });

  @override
  bool operator ==(Object other) =>
      other is Attachment &&
      other.hash == hash &&
      other.name == name &&
      other.mediaType == mediaType &&
      other.size == size &&
      other.pages == pages &&
      other.computer == computer;

  @override
  int get hashCode => Object.hash(hash, name, mediaType, size, pages, computer);

  @override
  String toString() => 'Attachment($name, $mediaType, $size, $hash)';
}

/// A file to attach: its [name], the media type the app was told (empty,
/// or `application/octet-stream`, for the one its name's ending says,
/// [FileTexts.mediaTypeOf]), its [bytes] and, for a PDF, how many [pages]
/// it has when the app counted them.
class NewFile {
  final String name;
  final String mediaType;
  final Uint8List bytes;
  final int? pages;

  const NewFile({
    required this.name,
    required this.bytes,
    this.mediaType = '',
    this.pages,
  });
}

/// [name] as a file's name is kept: the part after its last `/` or `\`,
/// without control characters and the marks that turn the direction of
/// text (which could make `exe.pdf` of `fdp.exe`), its blanks at the ends
/// trimmed, at most 255 characters; `.` and `..` are no name.
String cleanName(String name) {
  final base = name.substring(
    [
          name.lastIndexOf('/'),
          name.lastIndexOf(r'\'),
        ].reduce((a, b) => a > b ? a : b) +
        1,
  );
  final kept = String.fromCharCodes([
    for (final c in base.runes)
      if (!(c < 0x20 ||
          (c >= 0x7f && c <= 0x9f) ||
          (c >= 0x202a && c <= 0x202e) ||
          (c >= 0x2066 && c <= 0x2069) ||
          c == 0x200e ||
          c == 0x200f))
        c,
  ]);
  final trimmed = String.fromCharCodes(kept.trim().runes.take(_maxNameChars))
      .trim();
  return trimmed == '.' || trimmed == '..' ? '' : trimmed;
}

/// [mediaType] as a file's media type is kept: in lower case, without its
/// parameters, when it has the shape `type/subtype` of letters, digits and
/// `!#$&^_.+-`, at most 127 characters; [unknownType] otherwise.
String cleanType(String mediaType) {
  final bare = mediaType.split(';').first.trim();
  final lower = String.fromCharCodes([
    for (final c in bare.codeUnits) c >= 0x41 && c <= 0x5a ? c + 0x20 : c,
  ]);
  bool part(String p) =>
      p.isNotEmpty &&
      p.codeUnits.every(
        (c) =>
            (c >= 0x61 && c <= 0x7a) ||
            (c >= 0x30 && c <= 0x39) ||
            '!#\$&^_.+-'.codeUnits.contains(c),
      );
  final slash = lower.indexOf('/');
  if (slash < 0 || lower.length > 127) return unknownType;
  final (top, sub) = (lower.substring(0, slash), lower.substring(slash + 1));
  return part(top) && part(sub) ? lower : unknownType;
}

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

  /// Whether [detail] shows in it.
  bool holds(CapturedDetail detail) => switch (detail) {
    CapturedDetail.place => location != null,
    CapturedDetail.event => calendarEvents.isNotEmpty,
    CapturedDetail.device => bluetoothDevices.isNotEmpty,
    CapturedDetail.app => sourceApp != null,
  };
}

/// A detail of a Capture Context the user can remove on its own (ADR
/// 0037): where it was captured (the coordinates, and with them the
/// Named Place), the calendar events running, the Bluetooth devices
/// connected and the app it was shared in from. The capture time is none:
/// it stays.
enum CapturedDetail { place, event, device, app }

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

  /// The hashes of its voice recordings whose Transcript was removed
  /// while the recording stays, in the order they were added; they are
  /// not transcribed again unless asked to (ADR 0025).
  final List<String> removedTranscripts;

  /// The hashes of its photos whose Photo Text was removed while the
  /// photo stays, as for [removedTranscripts].
  final List<String> removedPhotoTexts;

  /// The files attached to it (ADR 0047), in the order they were
  /// attached.
  final List<Attachment> attachments;

  /// The Attachment Texts of its files, by file hash; a file not read
  /// yet, or of a kind without text, has none.
  final Map<String, String> attachmentTexts;

  /// Its files that were removed while their Attachment Text stays, in
  /// the order they were attached, as they were before; the text is in
  /// [attachmentTexts].
  final List<Attachment> removedAttachments;

  /// The hashes of its files whose Attachment Text was removed while the
  /// file stays, as for [removedPhotoTexts].
  final List<String> removedAttachmentTexts;

  /// The boxes of [text] as they show now, in the order they come
  /// (ADR 0035); none in a text this version may not change.
  final List<ShownBox> boxes;

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
    this.removedTranscripts = const [],
    this.removedPhotoTexts = const [],
    this.attachments = const [],
    this.attachmentTexts = const {},
    this.removedAttachments = const [],
    this.removedAttachmentTexts = const [],
    this.boxes = const [],
  });

  /// When the Note was captured.
  DateTime get captured => context.captured;

  /// [text] as it shows: each box in the glyph of its current state,
  /// without its id; what [boxes] count their offsets in, and what is
  /// copied, shared and saved (ADR 0035). [text] as it is in a format
  /// not known here.
  String get shownText => _shownText(text, textEditable, boxes);

  /// [shownText] as it reads without its marks, for search and previews.
  String get visibleText => TextFormats.visible(textFormat, shownText);

  bool get isTrashed => trashed != null;

  bool get isMerged => mergedInto != null;

  /// Not yet filed into an Area or Topic, not merged, not deleted and
  /// without a Routine or Reminder spawned from it (CONTEXT.md).
  bool get isUnsorted => place == null && !isTrashed && !isMerged && !spawned;

  /// When the Note's Nudge is due while it is Unsorted: the time chosen
  /// at Capture, else [Notes.defaultNudge] after it was captured.
  DateTime get nudgeAt => nudge ?? captured.add(Notes.defaultNudge);

  /// The file a Note made from a file alone shows as its card (ADR 0047):
  /// its one attached file, while its text is blank and it has no photo,
  /// recording or other file, removed ones included.
  Attachment? get aloneFile =>
      attachments.length == 1 &&
          shownText.trim().isEmpty &&
          photos.isEmpty &&
          removedPhotos.isEmpty &&
          recordings.isEmpty &&
          removedRecordings.isEmpty &&
          removedAttachments.isEmpty
      ? attachments.first
      : null;

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

  /// The recordings and photos whose texts were removed while they stay,
  /// as for [Note.removedTranscripts] and [Note.removedPhotoTexts].
  final List<String> removedTranscripts;
  final List<String> removedPhotoTexts;

  /// A merged Note's attached files with their Attachment Texts, those
  /// removed and those whose texts were removed, as for
  /// [Note.attachments]; an Amend has none.
  final List<Attachment> attachments;
  final Map<String, String> attachmentTexts;
  final List<Attachment> removedAttachments;
  final List<String> removedAttachmentTexts;

  /// The boxes of [text] as they show now, as for [Note.boxes].
  final List<ShownBox> boxes;

  /// When [text] was last edited (#183): the date of the text that shows,
  /// once more than the text it was captured with is kept; null while it
  /// was never edited. Whatever date it shows, [created] stays.
  final DateTime? modified;

  /// Who edited [text] last, as their Profile shows ([People.editorOf]):
  /// the user's own Profile for an edit on one of their Devices; null
  /// while it was never edited ([modified] is null) and for a writer not
  /// known.
  final Profile? editedBy;

  const Section({
    required this.id,
    required this.noteId,
    required this.text,
    required this.context,
    this.modified,
    this.editedBy,
    this.textFormat = TextFormats.plain,
    this.textVersion = TextFormats.version,
    this.textEditable = true,
    this.photos = const [],
    this.recordings = const [],
    this.transcripts = const {},
    this.photoTexts = const {},
    this.removedPhotos = const [],
    this.removedRecordings = const [],
    this.removedTranscripts = const [],
    this.removedPhotoTexts = const [],
    this.attachments = const [],
    this.attachmentTexts = const {},
    this.removedAttachments = const [],
    this.removedAttachmentTexts = const [],
    this.boxes = const [],
  });

  /// When the Section was captured, by Amend or as the merged Note; an
  /// edit of its text leaves it as it is.
  DateTime get created => context.captured;

  /// [text] as it shows, as for [Note.shownText].
  String get shownText => _shownText(text, textEditable, boxes);

  /// [shownText] as it reads without its marks, as for
  /// [Note.visibleText].
  String get visibleText => TextFormats.visible(textFormat, shownText);
}

/// [text] with each of [boxes] in the glyph of its state and no ids,
/// when [editable]; else as it is.
String _shownText(String text, bool editable, List<ShownBox> boxes) {
  if (!editable) return text;
  if (boxes.isEmpty) return TextBoxes.strip(text);
  return TextBoxes.shown(text, {for (final b in boxes) b.id: ?b.last}).text;
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
  /// be empty for a photo or voice Note, and for a Note made from a file
  /// alone, which shows the file as its card ([Note.aloneFile]); it is in
  /// [format]. The photos are kept without their metadata, the attached
  /// [files] as they are (ADR 0047); one larger than [maxFileBytes] is
  /// refused and nothing is written. A Note shared in from another app
  /// records it as [sourceApp].
  Note capture(
    String text, {
    String format = TextFormats.plain,
    String? place,
    Duration? nudgeIn,
    List<Uint8List> photos = const [],
    List<Uint8List> recordings = const [],
    List<NewFile> files = const [],
    SourceApp? sourceApp,
  }) {
    _requireFormat(format);
    files.forEach(_requireSize);
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
    final attached = [for (final file in files) _keepFile(file)];
    store.batch(() {
      store.append(id, NoteKeys.type, NoteKeys.typeNote, date: now);
      store.append(id, NoteKeys.captured, now.toIso8601String(), date: now);
      store.append(
        id,
        NoteKeys.text,
        StoredText.of(format, _boxed(text, format)).encode(),
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
      for (final file in attached) {
        store.append(id, NoteKeys.file(file.hash), file.value, date: now);
      }
      if (sourceApp != null) {
        final app = sourceApp;
        store.append(id, NoteKeys.sourcePackage, app.package, date: now);
        if (app.label case final label?) {
          store.append(id, NoteKeys.sourceLabel, label, date: now);
        }
      }
    });
    return byId(id) ??
        (throw StateError('$id does not read back after it was written'));
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

  /// Attaches [file] to the Note [noteId] (ADR 0047), its bytes kept as
  /// they are; returns the hash they are kept under. A file larger than
  /// [maxFileBytes] is refused. The same file attached again keeps one
  /// field, written anew.
  String attachFile(String noteId, NewFile file) {
    _requireNote(noteId);
    _requireSize(file);
    final attachment = _keepFile(file);
    store.append(noteId, NoteKeys.file(attachment.hash), attachment.value);
    return attachment.hash;
  }

  static void _requireSize(NewFile file) {
    if (file.bytes.length > maxFileBytes) {
      throw ArgumentError.value(
        file.name,
        'file',
        'larger than $maxFileBytes bytes',
      );
    }
  }

  /// Keeps the bytes of [file] and gives what its field holds.
  Attachment _keepFile(NewFile file) {
    final hash = store.putFile(file.bytes);
    final name = cleanName(file.name);
    // A type not named is the one its name's ending says.
    final named = cleanType(file.mediaType);
    final mediaType = named == unknownType
        ? FileTexts.mediaTypeOf(name)
        : named;
    final size = file.bytes.length;
    final pages = switch (file.pages) {
      final int p when p >= 0 && p <= _maxCount => p,
      _ => null,
    };
    return Attachment(
      hash: hash,
      name: name,
      mediaType: mediaType,
      size: size,
      pages: pages,
      computer: FileTexts.computerReads(
        FileTexts.kindOf(name, mediaType),
        size,
        pages,
      ),
    );
  }

  /// The bytes of the attached file [hash], or null when they are not on
  /// this Device (yet).
  Uint8List? fileBytes(String hash) => store.file(hash);

  /// Keeps [text], tidied ([FileTexts.tidy]), as the Attachment Text of
  /// the file [hash] of the Note [noteId], a merged one included, next to
  /// the file. An empty text records that no text was found, so the file
  /// is not read again. Given again, as when reading the file again, it
  /// replaces the Attachment Text, a removed one included.
  void setAttachmentText(String noteId, String hash, String text) {
    _requireAttached(noteId, hash);
    store.append(noteId, NoteKeys.fileText(hash), FileTexts.tidy(text));
  }

  /// The attached files whose text is still to be read, newest capture
  /// first: those of a kind with text ([FileKind.hasText]) without an
  /// Attachment Text, nor one that was removed, on every Note not in the
  /// trash, merged ones included. On a phone ([computer] false) those only
  /// a computer reads are left out (Q645).
  List<({String noteId, String hash})> unreadAttachments({
    required bool computer,
  }) {
    bool waits(Note note, Attachment file) =>
        file.kind.hasText &&
        (computer || !file.computer) &&
        !note.attachmentTexts.containsKey(file.hash) &&
        !note.removedAttachmentTexts.contains(file.hash);
    final notes = [
      for (final id in store.entitiesOfType(NoteKeys.typeNote))
        if (byId(id) case final note? when !note.isTrashed)
          if (note.attachments.any((a) => waits(note, a))) note,
    ];
    // By id within one capture time, so both cores list them alike.
    notes.sort((a, b) {
      final byTime = b.captured.compareTo(a.captured);
      return byTime != 0 ? byTime : a.id.compareTo(b.id);
    });
    return [
      for (final note in notes)
        for (final file in note.attachments)
          if (waits(note, file)) (noteId: note.id, hash: file.hash),
    ];
  }

  /// Whether this Device may remove the attached file [hash] of the Note
  /// [noteId], a merged one included, and keep its Attachment Text, as for
  /// [mayRemovePhoto]: the file has one that is not empty, and the Note is
  /// this Device's to change.
  bool mayRemoveAttachment(String noteId, String hash) {
    final note = byId(noteId);
    return note != null &&
        note.attachments.any((a) => a.hash == hash) &&
        _hasText(note.attachmentTexts[hash]) &&
        store.mayChange(noteId);
  }

  /// Removes the attached file [hash] from the Note [noteId] and keeps its
  /// Attachment Text, which search still finds, as [removePhoto] does
  /// (ADR 0020). Refused unless [mayRemoveAttachment].
  void removeAttachment(String noteId, String hash) {
    _requireAttached(noteId, hash);
    if (!mayRemoveAttachment(noteId, hash)) {
      throw StateError('the file $hash of $noteId may not be removed here');
    }
    store.append(noteId, NoteKeys.file(hash), null);
    store.dropRemovedBlobs();
  }

  /// Whether this Device may remove the Attachment Text of the file
  /// [hash] of the Note [noteId] and keep the file, as for
  /// [mayRemovePhotoText].
  bool mayRemoveAttachmentText(String noteId, String hash) =>
      mayRemoveAttachment(noteId, hash);

  /// Removes the Attachment Text of the file [hash] from the Note
  /// [noteId]; the file stays (ADR 0025). Search no longer finds the text,
  /// and the file is not read again until [setAttachmentText] is given a
  /// new one. Refused unless [mayRemoveAttachmentText].
  void removeAttachmentText(String noteId, String hash) {
    _requireAttached(noteId, hash);
    if (!mayRemoveAttachmentText(noteId, hash)) {
      throw StateError('the Attachment Text of $hash may not be removed here');
    }
    store.append(noteId, NoteKeys.fileText(hash), null);
  }

  void _requireAttached(String noteId, String hash) {
    if (byId(noteId)?.attachments.any((a) => a.hash == hash) != true) {
      throw ArgumentError.value(hash, 'hash', 'not a file of $noteId');
    }
  }

  /// Keeps [text] as the Transcript of the voice recording [hash] of the
  /// Note [noteId], a merged one included, next to the recording; the
  /// recording stays as it is. An empty [text] records that no words
  /// were heard, so the recording is not transcribed again. Given again,
  /// as when transcribing again, it replaces the Transcript, a removed
  /// one included.
  void setTranscript(String noteId, String hash, String text) {
    if (byId(noteId)?.recordings.contains(hash) != true) {
      throw ArgumentError.value(hash, 'hash', 'not a recording of $noteId');
    }
    store.append(noteId, NoteKeys.transcript(hash), text.trim());
  }

  /// The voice recordings without a Transcript, newest capture first:
  /// those of every Note not in the trash, merged ones included. One
  /// whose Transcript was removed is not among them.
  List<({String noteId, String hash})> untranscribed() {
    final notes = [
      for (final id in store.entitiesOfType(NoteKeys.typeNote))
        if (byId(id) case final note? when !note.isTrashed)
          if (note.recordings.any((h) => _untranscribed(note, h))) note,
    ];
    notes.sort((a, b) => b.captured.compareTo(a.captured));
    return [
      for (final note in notes)
        for (final hash in note.recordings)
          if (_untranscribed(note, hash)) (noteId: note.id, hash: hash),
    ];
  }

  /// Keeps [text] as the Photo Text of the photo [hash] of the Note
  /// [noteId], a merged one included, next to the photo; the photo stays
  /// as it is. An empty [text] records that no text was found, so the
  /// photo is not read again. Given again, as when reading the text
  /// again, it replaces the Photo Text, a removed one included.
  void setPhotoText(String noteId, String hash, String text) {
    if (byId(noteId)?.photos.contains(hash) != true) {
      throw ArgumentError.value(hash, 'hash', 'not a photo of $noteId');
    }
    store.append(noteId, NoteKeys.ocr(hash), text.trim());
  }

  /// The photos without a Photo Text, newest capture first: those of
  /// every Note not in the trash, merged ones included. One whose Photo
  /// Text was removed is not among them.
  List<({String noteId, String hash})> unreadPhotos() {
    final notes = [
      for (final id in store.entitiesOfType(NoteKeys.typeNote))
        if (byId(id) case final note? when !note.isTrashed)
          if (note.photos.any((h) => _unread(note, h))) note,
    ];
    notes.sort((a, b) => b.captured.compareTo(a.captured));
    return [
      for (final note in notes)
        for (final hash in note.photos)
          if (_unread(note, hash)) (noteId: note.id, hash: hash),
    ];
  }

  static bool _untranscribed(Note note, String hash) =>
      !note.transcripts.containsKey(hash) &&
      !note.removedTranscripts.contains(hash);

  static bool _unread(Note note, String hash) =>
      !note.photoTexts.containsKey(hash) &&
      !note.removedPhotoTexts.contains(hash);

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

  /// Whether this Device may remove the Photo Text of the photo [hash]
  /// of the Note [noteId], a merged one included, and keep the photo:
  /// the photo is there with a text that is not empty, and the Note is
  /// this Device's to change ([SharedAreas.mayChange]).
  bool mayRemovePhotoText(String noteId, String hash) {
    final note = byId(noteId);
    return note != null &&
        note.photos.contains(hash) &&
        _hasText(note.photoTexts[hash]) &&
        store.mayChange(noteId);
  }

  /// Whether this Device may remove the Transcript of the voice
  /// recording [hash] of the Note [noteId] and keep the recording, as
  /// for [mayRemovePhotoText].
  bool mayRemoveTranscript(String noteId, String hash) {
    final note = byId(noteId);
    return note != null &&
        note.recordings.contains(hash) &&
        _hasText(note.transcripts[hash]) &&
        store.mayChange(noteId);
  }

  /// Removes the Photo Text of the photo [hash] from the Note [noteId];
  /// the photo stays (ADR 0025). Search no longer finds the text, and the
  /// photo is not read again until [setPhotoText] is given a new one.
  /// Refused unless [mayRemovePhotoText].
  void removePhotoText(String noteId, String hash) {
    if (byId(noteId)?.photos.contains(hash) != true) {
      throw ArgumentError.value(hash, 'hash', 'not a photo of $noteId');
    }
    if (!mayRemovePhotoText(noteId, hash)) {
      throw StateError('the Photo Text of $hash may not be removed here');
    }
    store.append(noteId, NoteKeys.ocr(hash), null);
  }

  /// Removes the Transcript of the voice recording [hash] from the Note
  /// [noteId] and keeps the recording, as [removePhotoText] does.
  /// Refused unless [mayRemoveTranscript].
  void removeTranscript(String noteId, String hash) {
    if (byId(noteId)?.recordings.contains(hash) != true) {
      throw ArgumentError.value(hash, 'hash', 'not a recording of $noteId');
    }
    if (!mayRemoveTranscript(noteId, hash)) {
      throw StateError('the Transcript of $hash may not be removed here');
    }
    store.append(noteId, NoteKeys.transcript(hash), null);
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

  /// Whether this Device may take the Note [noteId] out of its Area or
  /// Topic (#268): it shows filed in one that exists, and it is this
  /// Device's to change ([SharedAreas.mayChange]), as filing it was.
  bool mayUnfile(String noteId) =>
      byId(noteId)?.place != null && store.mayChange(noteId);

  /// Takes the Note [noteId] out of its Area or Topic, filed nowhere
  /// again: back in Unsorted unless something else keeps it out, such as
  /// a Routine it spawned. One entry clears the place [file] wrote;
  /// latest wins, so the user's other Devices follow. Refused unless
  /// [mayUnfile].
  void unfile(String noteId) {
    if (byId(noteId)?.place == null) {
      throw ArgumentError.value(noteId, 'noteId', 'not a filed Note');
    }
    if (!store.mayChange(noteId)) {
      throw StateError('$noteId may not be taken out of its place here');
    }
    store.append(noteId, NoteKeys.place, null);
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
  ///
  /// The text keeps the Note's format, or takes [format] when one is
  /// given; a formatted text stays formatted and a code text code,
  /// whatever [format] says (ADR 0033): [setFormat] changes them.
  void edit(String noteId, String text, {String? format}) {
    if (format != null) _requireFormat(format);
    final note = byId(noteId);
    if (note == null) {
      throw ArgumentError.value(noteId, 'noteId', 'not a Note');
    }
    if (!note.textEditable) {
      throw StateError('the text of $noteId is in a format not known here');
    }
    final written = switch (note.textFormat) {
      TextFormats.markdown || TextFormats.code => note.textFormat,
      _ => format ?? note.textFormat,
    };
    store.append(
      noteId,
      NoteKeys.text,
      StoredText.of(written, _boxed(_text(text), written)).encode(),
    );
  }

  /// Puts the text of the Note [noteId] in [format] as it is: a plain
  /// text becomes formatted, for its first format, and a plain or
  /// formatted text becomes code and back (#213). A formatted text never
  /// becomes plain, and nothing is written when the format is the same.
  /// Into code the text goes as it shows, each box in the glyph of its
  /// state and without its id; out of it, its glyphs become boxes.
  void setFormat(String noteId, String format) {
    _requireFormat(format);
    final note = byId(noteId);
    if (note == null) {
      throw ArgumentError.value(noteId, 'noteId', 'not a Note');
    }
    if (!note.textEditable) {
      throw StateError('the text of $noteId is in a format not known here');
    }
    if (note.textFormat == format ||
        (note.textFormat == TextFormats.markdown &&
            format == TextFormats.plain) ||
        note.text.trim().isEmpty) {
      return;
    }
    final text = format == TextFormats.code ? note.shownText : note.text;
    store.append(
      noteId,
      NoteKeys.text,
      StoredText.of(format, _boxed(text, format)).encode(),
    );
  }

  /// Takes the Transcript or Photo Text [generated] over into the text
  /// of the Note [noteId] (ADR 0025): it becomes the text, or with
  /// [append] goes on a line of its own after it. Into a formatted text
  /// it goes as plain text, escaped so that it reads as itself. Gives
  /// the text written, its boxes with their ids (ADR 0035).
  String takeOver(String noteId, String generated, {required bool append}) {
    final note = byId(noteId);
    if (note == null) {
      throw ArgumentError.value(noteId, 'noteId', 'not a Note');
    }
    final taken = TextFormats.literal(note.textFormat, generated);
    final text = note.text;
    final joined = !append || text.trim().isEmpty
        ? taken
        : '${text.trimRight()}\n$taken';
    edit(noteId, joined);
    // As written: its glyphs boxes with their ids.
    return byId(noteId)?.text ?? _text(joined);
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

  /// Amends the Note [noteId] with a new Section holding [text] in
  /// [format], captured at the store's now.
  Section amend(
    String noteId,
    String text, {
    String format = TextFormats.plain,
  }) {
    _requireFormat(format);
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
        StoredText.of(format, _boxed(trimmed, format)).encode(),
        date: now,
      );
    });
    return _section(id) ??
        (throw StateError('$id does not read back after it was written'));
  }

  /// Whether this Device may edit the text of the Section [id], an Amend
  /// or a Note merged into another (#183), as the text of a Note: it is
  /// in a format known here (ADR 0018), and the Section is this Device's
  /// to change ([SharedAreas.mayChange]), its author's or, in a Shared
  /// Area, a Manager's.
  bool mayEditSection(String id) =>
      section(id)?.textEditable == true && store.mayChange(id);

  /// Fixes the text of the Section [id] in place, as [edit] fixes the
  /// text of a Note: a new entry on the Section, the old text stays in
  /// the log only, and the Section keeps its place and its Capture
  /// Context. The text keeps the Section's format, or takes [format]
  /// when one is given; a formatted text stays formatted (ADR 0033), and
  /// its bare glyphs become Boxes (ADR 0035). Nothing is written when
  /// the text stays as it is. Refused unless [mayEditSection].
  void editSection(String id, String text, {String? format}) {
    if (format != null) _requireFormat(format);
    final section = this.section(id);
    if (section == null) {
      throw ArgumentError.value(id, 'id', 'not a Section');
    }
    if (!mayEditSection(id)) {
      throw StateError('the text of $id may not be edited here');
    }
    final written = section.textFormat == TextFormats.markdown
        ? TextFormats.markdown
        : format ?? section.textFormat;
    // A Note merged into another keeps its text under the same key.
    final value = StoredText.of(written, _boxed(_text(text), written)).encode();
    if (value == store.current(id, SectionKeys.text)) return;
    store.append(id, SectionKeys.text, value);
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

  /// Whether this Device may undo the Merge of the Note [noteId] (#268):
  /// it shows merged into another, and it is this Device's to change
  /// ([SharedAreas.mayChange]), in no Area or one it manages, or as a
  /// Member when they made it.
  bool mayUndoMerge(String noteId) =>
      byId(noteId)?.isMerged == true && store.mayChange(noteId);

  /// Undoes the Merge of the Note [noteId]: it stands alone again, in
  /// the lists and where it was filed, with its Sections, Routines,
  /// Reminders and Frogs, all of which never left it. One entry clears
  /// the pointer [merge] wrote; latest wins, so the user's other Devices
  /// follow. Refused unless [mayUndoMerge].
  void undoMerge(String noteId) {
    if (byId(noteId)?.isMerged != true) {
      throw ArgumentError.value(noteId, 'noteId', 'not a merged Note');
    }
    if (!store.mayChange(noteId)) {
      throw StateError('the Merge of $noteId may not be undone here');
    }
    store.append(noteId, NoteKeys.merged, null);
  }

  /// Merges every Note of [noteIds] into the Note [intoId] in one batch
  /// (#273), each as [merge] does: all of them or, when one is refused,
  /// none. A Note named twice merges once. Refused for the target among
  /// [noteIds], a Note or target not in the lists, and a Note this Device
  /// may not change ([SharedAreas.mayChange]). Gives the Notes merged, in
  /// the order named, for [undoMergeAll].
  List<String> mergeAll(Iterable<String> noteIds, String intoId) {
    final ids = noteIds.toSet().toList();
    for (final (id, name) in [
      for (final id in ids) (id, 'noteIds'),
      (intoId, 'intoId'),
    ]) {
      final note = byId(id);
      if (note == null || note.isTrashed || note.isMerged) {
        throw ArgumentError.value(id, name, 'not a Note in the lists');
      }
    }
    if (ids.contains(intoId)) {
      throw ArgumentError.value(intoId, 'intoId', 'among the Notes merged');
    }
    for (final id in ids) {
      if (!store.mayChange(id)) {
        throw StateError('$id may not be merged here');
      }
    }
    store.batch(() {
      for (final id in ids) {
        store.append(id, NoteKeys.merged, intoId);
      }
    });
    return ids;
  }

  /// Undoes the Merge of each Note of [noteIds] in one batch, as
  /// [undoMerge] does for each one that [mayUndoMerge]; the others, which
  /// a sync or another step got to first, are left as they are. Gives the
  /// Notes that stand alone again.
  List<String> undoMergeAll(Iterable<String> noteIds) {
    final undone = <String>[];
    store.batch(() {
      for (final id in noteIds.toSet()) {
        if (!mayUndoMerge(id)) continue;
        store.append(id, NoteKeys.merged, null);
        undone.add(id);
      }
    });
    return undone;
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
        if (byId(id) case final note?) _merged(note, noteId),
      for (final id in store.entitiesOfType(SectionKeys.typeSection))
        if (owners.contains(store.current(id, SectionKeys.note)))
          ?_section(id, noteId: noteId),
    ];
    sections.sort((a, b) => a.context.captured.compareTo(b.context.captured));
    return sections;
  }

  /// The Sections of each Note of [noteIds], exactly as [sections] gives
  /// them, read in one pass over the Notes and Sections: for a list that
  /// shows many Notes in full (#114). Every id asked for is a key, one
  /// that is not a Note with the Sections that name it, if any.
  Map<String, List<Section>> sectionsOf(Iterable<String> noteIds) {
    final of = {for (final id in noteIds) id: <Section>[]};
    // Where each Note merged into one of them shows: its Amends show
    // there as well as under itself.
    final into = <String, String>{};
    for (final id in store.entitiesOfType(NoteKeys.typeNote)) {
      final end = _mergedInto(id);
      if (end == null) continue;
      if (of[end] case final list?) {
        into[id] = end;
        if (byId(id) case final note?) list.add(_merged(note, end));
      }
    }
    for (final id in store.entitiesOfType(SectionKeys.typeSection)) {
      final owner = store.current(id, SectionKeys.note);
      if (owner == null) continue;
      for (final noteId in [owner, ?into[owner]]) {
        final list = of[noteId];
        if (list == null) continue;
        if (_section(id, noteId: noteId) case final section?) {
          list.add(section);
        }
      }
    }
    for (final list in of.values) {
      list.sort((a, b) => a.context.captured.compareTo(b.context.captured));
    }
    return of;
  }

  /// The Note [note], merged into [noteId], as a Section of it.
  Section _merged(Note note, String noteId) {
    final edit = _lastEdit(note.id);
    return Section(
      id: note.id,
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
      removedTranscripts: note.removedTranscripts,
      removedPhotoTexts: note.removedPhotoTexts,
      attachments: note.attachments,
      attachmentTexts: note.attachmentTexts,
      removedAttachments: note.removedAttachments,
      removedAttachmentTexts: note.removedAttachmentTexts,
      boxes: note.boxes,
      modified: edit?.at,
      editedBy: edit?.by,
    );
  }

  /// When the text of the Note or Section [id] was last edited, and who
  /// edited it (#183): the text that wins by latest wins, among those that
  /// count, once there is more than one; null while there is only the
  /// text it was captured with. Ordered by latest wins, not by the log,
  /// so every Device finds the same.
  ({DateTime at, Profile? by})? _lastEdit(String id) {
    Entry? latest;
    var texts = 0;
    for (final e in store.history(id)) {
      if (e.field != SectionKeys.text) continue;
      texts++;
      if (latest == null || Store.latestWins(e, latest) > 0) latest = e;
    }
    if (latest == null || texts < 2) return null;
    return (at: latest.date, by: store.editorOf(id, latest.device));
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

  /// Whether this Device may remove [detail] from the Capture Context of
  /// the Note or Section [id], a merged Note included: it shows there,
  /// and the Note or Section is this Device's to change
  /// ([SharedAreas.mayChange]).
  bool mayRemoveCaptured(String id, CapturedDetail detail) =>
      _capturedOf(id)?.holds(detail) == true && store.mayChange(id);

  /// Removes [detail] from the Capture Context of the Note or Section
  /// [id] (ADR 0037): its fields set to null, an edit like any other,
  /// signed, synced and judged by the roles of a Shared Area. The capture
  /// time and every other detail stay. Refused unless
  /// [mayRemoveCaptured].
  void removeCaptured(String id, CapturedDetail detail) {
    if (_capturedOf(id) == null) {
      throw ArgumentError.value(id, 'id', 'not a Note or Section');
    }
    if (!mayRemoveCaptured(id, detail)) {
      throw StateError('the ${detail.name} of $id may not be removed here');
    }
    store.batch(() {
      for (final field in _capturedFields(detail)) {
        store.append(id, field, null);
      }
    });
  }

  /// The fields [detail] is kept under, on a Note and a Section alike;
  /// the app's label goes with its package.
  static List<String> _capturedFields(CapturedDetail detail) =>
      switch (detail) {
        CapturedDetail.place => const [NoteKeys.location],
        CapturedDetail.event => const [NoteKeys.calendar],
        CapturedDetail.device => const [NoteKeys.bluetooth],
        CapturedDetail.app => const [
          NoteKeys.sourcePackage,
          NoteKeys.sourceLabel,
        ],
      };

  /// The Capture Context of the Note or Section [id], or null when it is
  /// neither.
  CaptureContext? _capturedOf(String id) =>
      switch (store.current(id, NoteKeys.type)) {
        NoteKeys.typeNote => byId(id)?.context,
        SectionKeys.typeSection => _section(id)?.context,
        _ => null,
      };

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

  /// Removes the Note [noteId] for good, without the trash, as purging
  /// does: for a Note just captured whose content went elsewhere, such as
  /// a capture sorted into a Shopping List (#124). Refused for a Note that
  /// spawned something, which would be left without its Note, and where
  /// this Device may not change it.
  void discard(String noteId) {
    final note = byId(noteId);
    if (note == null) {
      throw ArgumentError.value(noteId, 'noteId', 'not a Note');
    }
    if (note.spawned || !store.mayChange(noteId)) {
      throw StateError('$noteId may not be discarded');
    }
    store.append(noteId, NoteKeys.type, null);
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
    final photos = _blobs(fields, NoteKeys.photoPrefix, NoteKeys.photoAdded);
    final recordings = _blobs(
      fields,
      NoteKeys.audioPrefix,
      NoteKeys.audioAdded,
    );
    final attachmentTexts = _texts(fields, NoteKeys.fileTextPrefix);
    final attachments = [
      for (final MapEntry(:key, :value) in fields.entries)
        if (key.startsWith(NoteKeys.filePrefix) && value != null)
          Attachment.read(key.substring(NoteKeys.filePrefix.length), value),
    ];
    final gone = [
      for (final hash in _blobs(fields, NoteKeys.filePrefix, null))
        if (_hasText(attachmentTexts[hash])) hash,
    ];
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
      photos: photos,
      recordings: recordings,
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
      // Only while the file stays: a text is removed from it, never
      // from a file that is gone.
      removedTranscripts: [
        for (final hash in recordings)
          if (_removedText(fields, NoteKeys.transcript(hash))) hash,
      ],
      removedPhotoTexts: [
        for (final hash in photos)
          if (_removedText(fields, NoteKeys.ocr(hash))) hash,
      ],
      attachments: attachments,
      attachmentTexts: attachmentTexts,
      removedAttachments: gone.isEmpty ? const [] : _asAttached(id, gone),
      removedAttachmentTexts: [
        for (final file in attachments)
          if (_removedText(fields, NoteKeys.fileText(file.hash))) file.hash,
      ],
      boxes: _boxes(id, text, fields),
    );
  }

  /// The removed files [hashes] of the Note [id] as they were attached:
  /// each by the value of its field that wins by latest wins among those
  /// that name the file.
  List<Attachment> _asAttached(String id, List<String> hashes) {
    final history = store.history(id);
    return [
      for (final hash in hashes)
        Attachment.read(hash, () {
          final field = NoteKeys.file(hash);
          Entry? held;
          for (final e in history) {
            if (e.field != field || e.value == null) continue;
            if (held == null || Store.latestWins(e, held) > 0) held = e;
          }
          return held?.value ?? '';
        }()),
    ];
  }

  /// The boxes of the stored [text] of [id] with the ticks that count
  /// (ADR 0035); none in a text this version may not change, nor in a
  /// code text (#213).
  List<ShownBox> _boxes(
    String id,
    StoredText text,
    Map<String, String?> fields,
  ) {
    if (!text.editable || text.format == TextFormats.code) return const [];
    final ticked = fields.keys.any((k) => k.startsWith(BoxKeys.prefix));
    final ticks = ticked ? Boxes(store).ticks(id) : const <String, BoxTick>{};
    return TextBoxes.shown(text.text, ticks).boxes;
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

  /// Whether the text [field] was written and then removed: it holds
  /// null.
  static bool _removedText(Map<String, String?> fields, String field) =>
      fields.containsKey(field) && fields[field] == null;

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
    final edit = _lastEdit(id);
    return Section(
      id: id,
      noteId: noteId ?? owner,
      text: text.text,
      textFormat: text.format,
      textVersion: text.version,
      textEditable: text.editable,
      context: _context(captured, fields),
      boxes: _boxes(id, text, fields),
      modified: edit?.at,
      editedBy: edit?.by,
    );
  }

  /// The Section [id] as it shows under its Note: an Amend, or a Note
  /// merged into another; null when there is none.
  Section? section(String id) {
    if (!id.startsWith('note:')) return _section(id);
    final note = byId(id);
    final into = note?.mergedInto;
    return note == null || into == null ? null : _merged(note, into);
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

  static void _requireFormat(String format) {
    if (!TextFormats.known.contains(format)) {
      throw ArgumentError.value(format, 'format', 'not a format known here');
    }
  }

  /// [text] in [format] as it is saved: every bare ☐, ✅ and ❌ a box
  /// with an id of its own (ADR 0035).
  static String _boxed(String text, String format) =>
      TextBoxes.convert(text, format: format, newId: Store.newId);

  static String _text(String text) {
    final trimmed = text.trim();
    if (trimmed.isEmpty) {
      throw ArgumentError.value(text, 'text', 'must not be empty');
    }
    return trimmed;
  }
}
