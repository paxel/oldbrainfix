import 'checklists.dart';
import 'markdown.dart';
import 'notes.dart';
import 'schedule.dart';
import 'shopping_lists.dart';
import 'store.dart';
import 'text_format.dart';

/// One Line of a Checklist or Shopping List as it is copied (#323), the
/// same in both cores (`share.rs`): its text, whether it is checked, on a
/// Shopping List struck, and whether it is under the Line above, in its
/// group.
typedef TaskLine = ({String text, bool checked, bool inGroup});

/// What a Note hands to another app (#129), the same in both cores
/// (`share.rs`): its text, then each Section's, every Section under its
/// date, a blank line between them. A Note or Section without words of
/// its own gives its Photo Texts and then its Transcripts instead, in the
/// order its photos and recordings were added; one without either is
/// left out. When any of these texts is formatted, [text] holds them as
/// written, the Markdown source among them with each item that starts
/// with a box written as a task list item ([Markdown.taskLists], #263),
/// and [html] the whole as HTML ([Markdown.html]), its boxes as their
/// glyphs, the plain texts and the dates in it as they read; else [html]
/// is null. [photos] and [recordings] are the hashes of the
/// Note's and then its Sections' photos and recordings, each once, for
/// the app to hand along as files.
class NoteShare {
  final String text;
  final String? html;
  final List<String> photos;
  final List<String> recordings;

  const NoteShare({
    required this.text,
    required this.html,
    required this.photos,
    required this.recordings,
  });

  /// The Lines of the Note [noteId] as they show now, a Checklist's
  /// checked as for the Occurrence it is on ([Checklists.shownNow]), a
  /// Shopping List's struck ones after its open ones and those checked
  /// Once left out; null when it is no Checklist or Shopping List.
  static List<TaskLine>? taskLines(
    Store store,
    String noteId,
    WallClock wallClock,
  ) {
    if (Checklists(store).shownNow(noteId, wallClock) case final list?) {
      return [
        for (final l in list.lines)
          (text: l.text, checked: l.checked, inGroup: l.inGroup),
      ];
    }
    if (ShoppingLists(store).byNote(noteId, wallClock) case final list?) {
      return [
        for (final l in list.lines)
          (text: l.text, checked: l.struck, inGroup: false),
      ];
    }
    return null;
  }

  /// [lines] as a task list of GFM (#263, #323), as `share.rs` writes it:
  /// one item a line, `- [ ] ` before an open Line, `- [x] ` before a
  /// checked one, a Line of a group indented by two spaces, each text as
  /// written.
  static String taskList(Iterable<TaskLine> lines) => [
    for (final l in lines)
      '${l.inGroup ? ChecklistKeys.groupIndent : ''}'
          '- ${l.checked ? '[x]' : '[ ]'} ${l.text}',
  ].join('\n');

  /// What [note] with its [sections], oldest first, hands along; [date]
  /// writes a Section's capture time as the app shows dates.
  static NoteShare of(
    Note note,
    List<Section> sections, {
    required String Function(DateTime captured) date,
  }) {
    final parts = [
      _Part(
        null,
        note.shownText,
        note.textFormat,
        _generated(
          note.photos,
          note.removedPhotos,
          note.photoTexts,
          note.recordings,
          note.removedRecordings,
          note.transcripts,
        ),
      ),
      for (final s in sections)
        _Part(
          date(s.context.captured),
          s.shownText,
          s.textFormat,
          _generated(
            s.photos,
            s.removedPhotos,
            s.photoTexts,
            s.recordings,
            s.removedRecordings,
            s.transcripts,
          ),
        ),
    ];
    final written = <String>[];
    final source = <String>[];
    var formatted = false;
    for (final part in parts) {
      final own = _trimEnd(part.text);
      final markdown = part.format == TextFormats.markdown;
      final words = own.isNotEmpty
          ? [(own, markdown)]
          : [for (final t in part.generated) (t, false)];
      if (words.isEmpty) continue;
      formatted = formatted || words.any((w) => w.$2);
      written.add(
        [
          ?part.date,
          for (final (t, md) in words) md ? Markdown.taskLists(t) : t,
        ].join('\n'),
      );
      source.add(
        [
          if (part.date case final d?) Markdown.escape(d),
          for (final (t, md) in words) md ? t : Markdown.escape(t),
        ].join('\n'),
      );
    }
    List<String> once(Iterable<String> hashes) => {...hashes}.toList();
    return NoteShare(
      text: written.join('\n\n'),
      html: formatted ? Markdown.html(source.join('\n\n')) : null,
      photos: once(
        [note.photos, for (final s in sections) s.photos].expand((h) => h),
      ),
      recordings: once(
        [
          note.recordings,
          for (final s in sections) s.recordings,
        ].expand((h) => h),
      ),
    );
  }

  /// The Photo Texts and then the Transcripts that say something, in the
  /// order their photos and recordings were added, removed ones last.
  static List<String> _generated(
    List<String> photos,
    List<String> removedPhotos,
    Map<String, String> photoTexts,
    List<String> recordings,
    List<String> removedRecordings,
    Map<String, String> transcripts,
  ) => [
    for (final hash in [...photos, ...removedPhotos])
      if (_trimEnd(photoTexts[hash] ?? '') case final t when t.isNotEmpty) t,
    for (final hash in [...recordings, ...removedRecordings])
      if (_trimEnd(transcripts[hash] ?? '') case final t when t.isNotEmpty) t,
  ];

  /// [text] without spaces, tabs and line breaks at its end; empty when
  /// it holds nothing else.
  static String _trimEnd(String text) {
    var end = text.length;
    while (end > 0 && ' \t\r\n'.contains(text[end - 1])) {
      end--;
    }
    var start = 0;
    while (start < end && ' \t\r\n'.contains(text[start])) {
      start++;
    }
    return start == end ? '' : text.substring(0, end);
  }
}

class _Part {
  final String? date;
  final String text;
  final String format;
  final List<String> generated;

  const _Part(this.date, this.text, this.format, this.generated);
}
