import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'app_lock.dart';
import 'announcements.dart';
import 'attachment_views.dart';
import 'context_platform.dart';
import 'drafts.dart';
import 'failure_dialog.dart';
import 'file_platform.dart';
import 'l10n.dart';
import 'location.dart';
import 'note_editor_screen.dart';
import 'photo_platform.dart';
import 'photo_views.dart';
import 'routine_editor_screen.dart';
import 'spotlight.dart';
import 'sync_line.dart';
import 'voice_recorder.dart';
import 'voice_platform.dart';

/// A Note just captured, and for its location and for the rest of its
/// Capture Context whether it got attached, each once that is known;
/// with [editing] when Open in the editor opened the full editor on it, done once the user
/// left the editor.
typedef Captured = ({
  String noteId,
  List<Future<bool>> attached,
  Future<void>? editing,
});

/// Opens text Capture, or voice Capture recording at once when [voice] is
/// set, or the camera at once when [photo] is set, or filled from the
/// clipboard when [paste] is set, filing the new Note into the Area or Topic [place] when one is
/// given, or with the [shared] item from another app filled in. [onChanged] runs once the user left Capture, for the caller to
/// show the new Note, and again when its location or more of its Capture
/// Context arrived later.
/// [onCaptured] gets the new Note's id first, when there is one.
/// Open in the editor leaves Capture for the Note's editor; leaving that
/// comes back here, and [onChanged] runs once more.
///
/// A shared sound file this Device cannot transcribe is kept all the
/// same, and announced at the top ([announceUntranscribable]).
///
/// After this Device's first Capture, while the location is not allowed
/// yet, it explains why Notes record where they were captured and asks;
/// Capture itself never asks anything (UI laws, "Capture").
Future<void> captureNote(
  BuildContext context,
  Notes notes, {
  String? place,
  SharedItem? shared,
  bool voice = false,
  bool photo = false,
  bool gallery = false,
  bool paste = false,
  bool file = false,
  required VoidCallback onChanged,
  ValueChanged<String>? onCaptured,
}) async {
  final location = LocationScope.of(context);
  final photos = PhotoScope.of(context);
  final files = FileScope.of(context);
  final microphone = VoiceScope.of(context);
  final contextSource = ContextScope.of(context);
  if (shared != null) announceUntranscribable(context, shared.recordings);
  final captured = await Navigator.of(context).push<Captured>(
    MaterialPageRoute(
      // Capture never needs unlocking.
      settings: const RouteSettings(name: LockFreeRoutes.capture),
      builder: (_) => CaptureScreen(
        notes: notes,
        location: location,
        photos: photos,
        voice: microphone,
        contextSource: contextSource,
        files: files,
        place: place,
        shared: shared,
        startVoice: voice,
        startPhoto: photo,
        startGallery: gallery,
        startPaste: paste,
        startFile: file,
      ),
    ),
  );
  if (captured != null) onCaptured?.call(captured.noteId);
  onChanged();
  if (captured == null) return;
  // Each shows as soon as it arrives, without waiting for the others.
  for (final attached in captured.attached) {
    unawaited(attached.then((attached) => attached ? onChanged() : null));
  }
  // The editor first: nothing is asked over it.
  if (captured.editing case final editing?) {
    await editing;
    if (!context.mounted) return;
    onChanged();
  }
  final store = notes.store;
  if (!location.available || store.locationExplained) return;
  if (await location.granted()) return;
  if (!context.mounted) return;
  store.locationExplained = true;
  if (await explainAndAskLocation(context, location)) {
    // The position may take a while; the list does not wait for it.
    unawaited(
      attachLocation(
        notes,
        captured.noteId,
        location.current(),
      ).then((located) => located ? onChanged() : null),
    );
  }
}

/// Makes each line of the text of the Note [noteId] a Line of the
/// Shopping List [listId], a name the list has bringing its Line back,
/// and discards the Note, all in one batch (#124): a capture without a
/// recording, which [ShoppingLists.addVoiceCapture] keeps on the list.
/// True when it did; false when the list can no longer be added to, and
/// null when the Note may no longer go: deleted, merged or with something
/// spawned from it meanwhile. Either way the Note stays.
bool? pourIntoShopping(Notes notes, String noteId, String listId) {
  final note = notes.byId(noteId);
  if (note == null || note.isTrashed || note.isMerged || note.spawned) {
    return null;
  }
  try {
    notes.store.batch(() {
      // Its boxes as they show, without their ids (ADR 0035).
      ShoppingLists(notes.store)
          .addLines(listId, note.shownText, deviceWallClock);
      notes.discard(noteId);
    });
    return true;
  } on StateError {
    return false;
  } on ArgumentError {
    return false;
  }
}

/// Announces at the top that some of the shared [recordings] cannot be
/// transcribed on this Device, when some cannot; the tap opens why, what
/// to do and each one's problem and format (UI laws, "Announcements").
void announceUntranscribable(
  BuildContext context,
  List<SharedRecording> recordings,
) {
  final failed = [
    for (final recording in recordings)
      if (recording.problem case final problem?) (problem, recording.format),
  ];
  final announcements = AnnouncementsScope.of(context);
  if (failed.isEmpty || announcements == null) return;
  final t = context.t;
  final navigator = Navigator.of(context);
  announcements.announce(
    t.sharedRecordingFailed(failed.length),
    onTap: () => unawaited(
      showFailure(
        navigator.context,
        title: t.sharedRecordingFailedTitle,
        cause: switch (failed.first.$1) {
          SoundProblem.unreadable => t.sharedRecordingUnreadable,
          SoundProblem.unsupported => t.sharedRecordingUnsupported,
        },
        fix: t.sharedRecordingFix,
        details: [
          for (final (problem, format) in failed)
            format == null ? problem.name : '${problem.name}: $format',
        ].join('\n'),
      ),
    ),
  );
}

/// Text capture, the quick note (#352): the text field at the top, opened
/// with the keyboard up, right under it the three Nudge buttons and Open
/// in the editor, which stay put when the keyboard opens (#396), nothing else, in Simple
/// and Advanced alike. The position, and the calendar events and
/// Bluetooth devices where they are switched on, are looked up from the
/// start, without asking, and attached when they arrive: they never hold
/// the Note back. While Unsorted the Note is nudged in 10 min, 30 min
/// or 1 h, picked with the buttons under the text, each of which saves
/// (UI laws, "Capture"). There is no Save and no Back on top: Android's
/// Back saves, nudged in 1 h, and so does the back arrow the iPhone keeps
/// top left, and the app going to the background writes the text too; an
/// empty text saves nothing. Where the Nudge buttons are switched off
/// (Settings ▸ Quick note), one Save takes their place and nudges in 1 h.
///
/// Open in the editor saves the Note at once, Unsorted and nudged in 1 h,
/// and opens it in the Note editor in place of this screen (#142), where Photo, Voice,
/// Gallery, Paste, File, the Kind, Add to Note…, Remind me…, File into…,
/// the Frog, the Importance and the format bar are; not offered behind
/// the App Lock, as the editor shows the Areas, and not while there is
/// nothing to save.
///
/// Opened from an Area's or Topic's "+" the Note is filed there: sorted,
/// it needs no Nudge, and one Save takes the three buttons' place.
///
/// An entry point that starts with something opens this screen with it
/// shown above the text: [startPhoto] takes a photo into the Note,
/// [startGallery] picks pictures from the Device into it, several at
/// once, and [startFile] files of any kind; the text is then optional.
/// Closing the picker without picking changes nothing. [startPaste]
/// reads the clipboard, only then, as Android shows a notice for every
/// read: its text goes into the field, its pictures into the Note as
/// photos, and a clipboard with neither is said in a line above the
/// field, which goes once the user types.
///
/// Something shared in from another app opens here with its
/// text, photos and sound files filled in, the sound files as voice
/// recordings, and the app it came from recorded. All of it goes into one
/// Note written at once, so nothing is lost.
///
/// [startVoice] opens voice: recording starts at once, the elapsed time
/// runs, and one big stop button ends it and comes back to the text with
/// the recording; the Nudge buttons and Back end it and save; there is no
/// hold-to-talk and no stop on silence (UI laws, "Capture"). The first
/// time, while the microphone is not allowed yet, it explains why and
/// asks. The screen stays on while recording (#350). On the iPhone the
/// recording goes on with the app in the background and the phone locked,
/// until Stop here, Back or a Nudge button ends and keeps it; on Android
/// leaving the app or locking the phone ends and keeps it too (#368)
/// ([VoiceRecorder]). Text typed before goes into the same Note.
class CaptureScreen extends StatefulWidget {
  final Notes notes;
  final LocationSource location;
  final PhotoPlatform photos;
  final VoicePlatform voice;
  final ContextSource contextSource;

  /// The Area or Topic the new Note is filed into; Unsorted when null.
  final String? place;

  /// What another app shared, filled in; null for a plain Capture.
  final SharedItem? shared;

  /// Opens in voice, recording at once.
  final bool startVoice;

  /// Opens the camera at once; back from it, the screen is text Capture
  /// with the photo.
  final bool startPhoto;

  /// Opens the system's picture picker at once; back from it, the screen
  /// is text Capture with the pictures picked.
  final bool startGallery;

  /// Fills the screen from the clipboard at once.
  final bool startPaste;

  /// The system's file picker, for [startFile] and files shared in (#267).
  final FilePlatform files;

  /// Opens the system's file picker at once; back from it, the screen is
  /// text Capture with the files attached.
  final bool startFile;

  const CaptureScreen({
    super.key,
    required this.notes,
    this.location = const NoLocation(),
    this.photos = const NoPhotos(),
    this.voice = const NoVoice(),
    this.contextSource = const NoContext(),
    this.files = const NoFiles(),
    this.place,
    this.shared,
    this.startVoice = false,
    this.startPhoto = false,
    this.startGallery = false,
    this.startPaste = false,
    this.startFile = false,
  });

  @override
  State<CaptureScreen> createState() => _CaptureScreenState();
}

class _CaptureScreenState extends State<CaptureScreen> {
  late final _nudgeButtons = widget.notes.store.quickNoteNudgeButtonsOn;

  final _text = TextEditingController();

  /// The text as last seen, to tell when it changed.
  var _typed = '';
  late final Future<Coordinates?> _here;
  late final Future<ContextExtras> _extras;
  late final AppLifecycleListener _lifecycle;

  /// The recording of [startVoice], one at a time with the Note editor's.
  late final VoiceRecorder _recorder;

  /// The Note once written: by leaving, by a photo, or when the app went
  /// to the background meanwhile, after which typing changes the same
  /// Note.
  Captured? _captured;

  /// The hashes of the Note's photos, as written.
  List<String> _photos = const [];

  /// The hashes of the Note's voice recordings, as written.
  List<String> _recordings = const [];

  /// The Note's files, as written (#267).
  List<Attachment> _files = const [];

  /// The files being picked or shared in and written; null while none
  /// are.
  Future<void>? _filing;

  /// Whether the screen shows voice instead of the text field.
  var _voiceMode = false;

  /// Whether voice found the microphone not allowed or not working.
  var _micMissing = false;

  /// The pictures being picked and written; null while none are.
  Future<void>? _picking;

  /// The clipboard being read and filled in; null while it is not.
  Future<void>? _pasting;

  /// Whether the last Paste found nothing to paste, said above the field.
  var _pasteEmpty = false;

  /// Set once leaving began, so a second tap does not leave twice.
  var _leaving = false;

  /// The Area or Topic the Note goes into, from a list's "+"; Unsorted
  /// while null.
  late final String? _place = widget.place;

  AppLock? _lock;

  @override
  void initState() {
    super.initState();
    _text.addListener(_textChanged);
    // The fix may take a while; it starts while the user types.
    _here = currentIfGranted(widget.location);
    _extras = currentExtras(widget.notes.store, widget.contextSource);
    _recorder = VoiceRecorder(
      widget.voice,
      keep: _addRecording,
      ended: _recordingEnded,
    );
    // A share is meant to be kept: it is written at once, as one Note.
    final shared = widget.shared;
    if (shared != null && !shared.holdsNoNote) {
      _text.text = shared.text;
      _create(
        shared.text.trim(),
        photos: shared.photos,
        recordings: [
          for (final recording in shared.recordings) recording.bytes,
        ],
      );
    }
    // Its files are read in first, into the same Note (#267).
    if (shared != null && shared.files.isNotEmpty) {
      WidgetsBinding.instance.addPostFrameCallback(
        (_) => _attachFiles(Future.value(shared.files)),
      );
    }
    // The app may be killed in the background: the text is kept.
    _lifecycle = AppLifecycleListener(
      onStateChange: (state) {
        if (state == AppLifecycleState.paused ||
            state == AppLifecycleState.hidden) {
          _write(null);
          // What was written shows as such.
          if (mounted) setState(() {});
          // A recording ends there on Android, on the iPhone it goes on
          // (#350, #368): [VoiceRecorder] sees to it.
        }
      },
    );
    if (widget.startVoice) {
      WidgetsBinding.instance.addPostFrameCallback((_) => _openVoice());
    } else if (widget.startPhoto && widget.photos.available) {
      WidgetsBinding.instance.addPostFrameCallback((_) => _takePhoto());
    } else if (widget.startGallery && widget.photos.pickable) {
      WidgetsBinding.instance.addPostFrameCallback((_) => _pickPhotos());
    } else if (widget.startPaste && widget.photos.pasteable) {
      WidgetsBinding.instance.addPostFrameCallback((_) => _paste());
    } else if (widget.startFile && widget.files.pickable) {
      WidgetsBinding.instance.addPostFrameCallback((_) => _pickFiles());
    }
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final lock = LockScope.of(context);
    if (lock == _lock) return;
    _lock?.removeListener(_lockChanged);
    _lock = lock?..addListener(_lockChanged);
  }

  /// Locking again closes what this screen opened, such as the
  /// microphone's explanation, and hides Open in the editor: only Capture itself needs
  /// no unlocking.
  void _lockChanged() {
    if (!mounted) return;
    final route = ModalRoute.of(context);
    if ((_lock?.locked ?? false) && route != null && !route.isCurrent) {
      Navigator.of(context)
          .popUntil((top) => top == route || LockFreeRoutes.contains(top));
    }
    setState(() {});
  }

  @override
  void dispose() {
    _lock?.removeListener(_lockChanged);
    _lifecycle.dispose();
    // Closed from elsewhere while recording: the recording ends, so the
    // screen times out again, and is kept.
    _recorder.dispose();
    _text.removeListener(_textChanged);
    _text.dispose();
    super.dispose();
  }

  /// Whether the Note leaves Unsorted, filed from a list's "+": it needs
  /// no Nudge then.
  bool get _sorted => _place != null;

  /// What is typed or pasted answers "nothing to paste".
  void _textChanged() {
    final text = _text.text;
    if (text == _typed) return;
    _typed = text;
    if (_pasteEmpty) setState(() => _pasteEmpty = false);
  }

  /// Writes the text as a Note, or into the one already written; nothing
  /// while it is empty and there is no Note yet, so no empty Note is
  /// saved. A new Unsorted Note is nudged [nudgeIn] from now, 1 h when
  /// null; an existing one only when [nudgeIn] is given. A photo Note
  /// keeps its text when it is emptied. All of it is one batch: when a
  /// write on the way fails, none of it is written, and the screen still
  /// holds all of it (#113).
  void _write(Duration? nudgeIn) {
    final text = _text.text.trim();
    final captured = _captured;
    if (captured == null && text.isEmpty) return;
    final notes = widget.notes;
    final created = notes.store.batch(() {
      if (captured == null) {
        return _captureNote(text, nudgeIn: nudgeIn);
      }
      final note = notes.byId(captured.noteId);
      if (text.isNotEmpty &&
          note != null &&
          note.textEditable &&
          // Its boxes got ids when it was kept (ADR 0035).
          TextBoxes.strip(note.text) != text) {
        notes.edit(captured.noteId, text, format: _formatOf(text));
      }
      if (nudgeIn != null && !_sorted) {
        notes.setNudge(captured.noteId, nudgeIn);
      }
      return null;
    });
    if (created != null) _adopt(created);
  }

  /// Captures the new Note with [text], [photos], [recordings] and
  /// [files].
  void _create(
    String text, {
    List<Uint8List> photos = const [],
    List<Uint8List> recordings = const [],
    List<NewFile> files = const [],
  }) => _adopt(
    _captureNote(text, photos: photos, recordings: recordings, files: files),
  );

  /// Writes the new Note with [text], [photos], [recordings] and [files],
  /// nudged [nudgeIn] from now, 1 h when null, unless it is filed.
  Note _captureNote(
    String text, {
    Duration? nudgeIn,
    List<Uint8List> photos = const [],
    List<Uint8List> recordings = const [],
    List<NewFile> files = const [],
  }) => widget.notes.capture(
    text,
    format: _formatOf(text) ?? TextFormats.plain,
    place: _place,
    // A sorted Note needs no Nudge (#113).
    nudgeIn: _sorted ? null : nudgeIn ?? Notes.defaultNudge,
    photos: photos,
    recordings: recordings,
    files: files,
    sourceApp: widget.shared?.sourceApp,
  );

  /// Formatted for a text that uses a format, pasted, shared or typed as
  /// Markdown (ADR 0033); null leaves it as it is.
  String? _formatOf(String text) =>
      Markdown.uses(text) ? TextFormats.markdown : null;

  /// Holds the written [note] as the one being captured, and attaches
  /// where and amid what it was captured once that is known.
  void _adopt(Note note) {
    _captured = (
      noteId: note.id,
      attached: [
        attachLocation(widget.notes, note.id, _here),
        attachExtras(widget.notes, note.id, _extras),
      ],
      editing: null,
    );
    _photos = note.photos;
    _recordings = note.recordings;
    _files = note.attachments;
  }

  /// Writes [photos] into the Note, capturing it with the text so far
  /// when there is none yet.
  void _addPhotos(List<Uint8List> photos) {
    if (photos.isEmpty) return;
    final captured = _captured;
    if (captured == null) {
      _create(_text.text.trim(), photos: photos);
      return;
    }
    for (final bytes in photos) {
      widget.notes.addPhoto(captured.noteId, bytes);
    }
    _photos = widget.notes.byId(captured.noteId)?.photos ?? _photos;
  }

  /// Writes [files] into the Note, capturing it with the text so far
  /// when there is none yet.
  void _addFiles(List<NewFile> files) {
    if (files.isEmpty) return;
    final captured = _captured;
    if (captured == null) {
      _create(_text.text.trim(), files: files);
      return;
    }
    attachFiles(widget.notes, captured.noteId, files);
    _files = widget.notes.byId(captured.noteId)?.attachments ?? _files;
  }

  /// Picks files with the system's file picker into the Note (#267); no
  /// second picker while files are being attached.
  Future<void> _pickFiles() =>
      _filing ?? _attachFiles(widget.files.pickFiles());

  /// Waits for the files [coming], reads them in and writes them into
  /// the Note, after those already coming, also when leaving began
  /// meanwhile, so none is lost; one that cannot be attached is said at
  /// the top.
  Future<void> _attachFiles(Future<List<DeviceFile>> coming) {
    final t = context.t;
    final announcements = AnnouncementsScope.of(context);
    final navigator = Navigator.of(context, rootNavigator: true);
    final before = _filing;
    late final Future<void> run;
    run =
        () async {
          if (before != null) await before;
          final picked = await coming;
          if (picked.isEmpty) return;
          final read = await readPickedFiles(
            widget.files,
            picked,
            widget.notes.store,
          );
          announceNotAttached(t, announcements, navigator, read);
          if (mounted && !_leaving) {
            setState(() => _addFiles(read.files));
          } else {
            _addFiles(read.files);
          }
        }().whenComplete(() {
          if (identical(_filing, run)) _filing = null;
        });
    return _filing = run;
  }

  /// Takes a photo with the camera into the Note.
  Future<void> _takePhoto() async {
    final bytes = await widget.photos.takePhoto();
    if (!mounted || bytes == null) return;
    setState(() => _addPhotos([bytes]));
  }

  /// Picks pictures with the system's picker into the Note; the same
  /// picking when asked again while it runs.
  Future<void> _pickPhotos() =>
      _picking ??= _pick().whenComplete(() => _picking = null);

  /// Waits for the pictures picked and writes them into the Note, also
  /// when leaving began meanwhile, so none is lost.
  Future<void> _pick() async {
    final picked = await widget.photos.pickPhotos();
    if (picked.isEmpty) return;
    if (mounted && !_leaving) {
      setState(() => _addPhotos(picked));
    } else {
      _addPhotos(picked);
    }
  }

  /// Fills in what the clipboard holds; the same reading when asked
  /// again while it runs.
  Future<void> _paste() =>
      _pasting ??= _takeClipboard().whenComplete(() => _pasting = null);

  /// Reads the clipboard and fills in its text, then its pictures, so a
  /// Note written for them holds the text too. Leaving began meanwhile,
  /// nothing is filled in; the clipboard still holds it.
  Future<void> _takeClipboard() async {
    final pasted = await widget.photos.takeClipboard();
    if (!mounted || _leaving) return;
    setState(() {
      _pasteEmpty = pasted == null;
      if (pasted == null) return;
      if (pasted.text.trim().isNotEmpty) insertAtCursor(_text, pasted.text);
      _addPhotos(pasted.photos);
    });
  }

  /// Writes the recording [bytes] into the Note, capturing it with the
  /// text so far when there is none yet.
  void _addRecording(Uint8List bytes) {
    final captured = _captured;
    if (captured == null) {
      _create(_text.text.trim(), recordings: [bytes]);
      return;
    }
    widget.notes.addRecording(captured.noteId, bytes);
    _recordings = widget.notes.byId(captured.noteId)?.recordings ?? _recordings;
  }

  /// Switches to voice and starts recording, asking for the microphone
  /// first, with an explanation, while it is not allowed.
  Future<void> _openVoice() async {
    if (_recorder.recording || _leaving) return;
    FocusScope.of(context).unfocus();
    setState(() {
      _voiceMode = true;
      _micMissing = false;
    });
    final started = await _recorder.start(
      context,
      wanted: () => mounted && !_leaving,
    );
    if (!mounted || _leaving || started != false) return;
    setState(() => _micMissing = true);
  }

  /// Ends the recording and writes it into the Note; the same ending
  /// when asked again, nothing when none runs.
  Future<void> _stopRecording() => _recorder.stop();

  /// Back to the text field once the recording is kept, with the
  /// recording; leaving, the text field never comes back to pop the
  /// keyboard up.
  void _recordingEnded() {
    if (mounted && !_leaving) setState(() => _voiceMode = false);
  }

  /// Saves and leaves: by Back, the iPhone's back arrow or Save (1 h), or
  /// a picked Nudge time. A running recording is ended and kept first,
  /// and pictures still being picked are written first.
  Future<void> _save([Duration? nudgeIn]) async {
    if (_leaving) return;
    _leaving = true;
    await _stopRecording();
    await _picking;
    if (!mounted) return;
    try {
      _write(nudgeIn);
    } catch (_) {
      // Nothing of it was written: the screen stays, to change and leave
      // again.
      _leaving = false;
      rethrow;
    }
    Navigator.of(context).pop(_captured);
  }

  /// Open in the editor: saves the Note as leaving does, Unsorted and nudged in 1 h
  /// unless it is filed, and opens its editor in place of this screen, so
  /// leaving the editor comes back to where Capture was opened from. A
  /// recording running is ended and kept first, and pictures still being
  /// picked are written first. Nothing saved, as for an empty Note, the
  /// screen stays.
  Future<void> _toEditor() async {
    if (_leaving) return;
    _leaving = true;
    await _stopRecording();
    await _picking;
    if (!mounted) return;
    try {
      _write(null);
    } catch (_) {
      _leaving = false;
      rethrow;
    }
    final captured = _captured;
    if (captured == null) {
      _leaving = false;
      return;
    }
    final notes = widget.notes;
    final drafts = DraftsScope.of(context);
    final editing = Completer<void>();
    final navigator = Navigator.of(context);
    // A tip that came up over this screen while it was saving closes
    // first, answering nothing: the editor takes this screen's place,
    // never the tip's (#371).
    if (ModalRoute.of(context) case final route?) {
      navigator.popUntil((top) => top == route);
    }
    unawaited(
      navigator
          .pushReplacement<void, Captured>(
            MaterialPageRoute(
              builder: (_) => NoteEditorScreen(
                notes: notes,
                places: Places(notes.store),
                drafts: drafts,
                noteId: captured.noteId,
              ),
            ),
            result: (
              noteId: captured.noteId,
              attached: captured.attached,
              editing: editing.future,
            ),
          )
          .whenComplete(editing.complete),
    );
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final recording = _voiceMode && !_micMissing;
    // The buttons sit right under the text and do not move when the
    // keyboard opens (#396); once the text fills the room, the photos,
    // recordings and text scroll and the buttons stay above the keyboard.
    final Widget text = _voiceMode
        ? Expanded(
            child: Column(
              children: [
                ..._attachments(context),
                Expanded(
                  child: VoicePanel(
                    typed: _text.text.trim(),
                    micMissing: _micMissing,
                    elapsed: _recorder.elapsed,
                    onStop: _stopRecording,
                    onRetry: _openVoice,
                    onTypeInstead: () => setState(() => _voiceMode = false),
                  ),
                ),
              ],
            ),
          )
        : Flexible(
            child: CustomScrollView(
              shrinkWrap: true,
              slivers: [
                for (final attachment in _attachments(context))
                  SliverToBoxAdapter(child: attachment),
                if (_pasteEmpty)
                  SliverToBoxAdapter(
                    child: Padding(
                      padding: const EdgeInsets.only(bottom: 8),
                      child: Text(
                        t.pasteEmpty,
                        key: const ValueKey('paste-empty'),
                        style: TextStyle(
                          color: Theme.of(context).colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ),
                  ),
                SliverToBoxAdapter(child: _field(context)),
              ],
            ),
          );
    // Leaving by Back saves as well (UI laws, "Capture").
    return PopScope(
      canPop: false,
      onPopInvokedWithResult: (didPop, _) {
        if (!didPop) unawaited(_save());
      },
      child: Scaffold(
        // No Save and no Back on top (#352): Android's Back saves, the
        // iPhone keeps a back arrow that saves; the bar carries the sync
        // line under it.
        appBar: defaultTargetPlatform == TargetPlatform.iOS
            ? AppBar(
                leading: BackButton(onPressed: () => unawaited(_save())),
                flexibleSpace: const SyncLine(),
              )
            : AppBar(
                automaticallyImplyLeading: false,
                toolbarHeight: 0,
                flexibleSpace: const SyncLine(),
              ),
        body: SafeArea(
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              children: [
                text,
                const SizedBox(height: 8),
                if (_sorted || !_nudgeButtons)
                  SizedBox(
                    width: double.infinity,
                    child: FilledButton(
                      key: const ValueKey('captureSave'),
                      onPressed: _save,
                      child: Text(t.save),
                    ),
                  )
                // No tip over the stop button while a recording runs.
                else if (_voiceMode)
                  NudgeButtons(onPicked: _save)
                else
                  Spotlight(
                    id: 'capture-nudge',
                    child: NudgeButtons(onPicked: _save),
                  ),
                // While recording, the big stop button comes first.
                if (!recording) ?_moreButton(context),
              ],
            ),
          ),
        ),
      ),
    );
  }

  /// The text field.
  Widget _field(BuildContext context) => TextField(
    controller: _text,
    autofocus:
        !widget.startVoice &&
        !widget.startPhoto &&
        !widget.startGallery &&
        !widget.startFile,
    maxLines: null,
    decoration: InputDecoration(
      hintText: _photos.isEmpty && _recordings.isEmpty && _files.isEmpty
          ? context.t.captureHint
          : context.t.capturePhotoHint,
    ),
  );

  /// Open in the editor, a full-width button under the Nudge buttons
  /// (obf-1080): none behind the App Lock, and it waits while there is
  /// nothing to save.
  Widget? _moreButton(BuildContext context) {
    if (_lock?.locked ?? false) return null;
    return Padding(
      padding: const EdgeInsets.only(top: 8),
      child: Spotlight(
        id: 'capture-more',
        child: ListenableBuilder(
          listenable: _text,
          builder: (context, _) => SizedBox(
            width: double.infinity,
            child: OutlinedButton.icon(
              key: const ValueKey('captureToEditor'),
              onPressed: _captured != null || _text.text.trim().isNotEmpty
                  ? _toEditor
                  : null,
              icon: const Icon(Icons.edit_outlined),
              label: Text(context.t.captureOpenInEditor),
            ),
          ),
        ),
      ),
    );
  }

  /// The Note's photos, its recordings outside voice and its files,
  /// above the text.
  List<Widget> _attachments(BuildContext context) => [
    if (_photos.isNotEmpty) ...[
      PhotoStrip(
        hashes: _photos,
        size: 96,
        onTap: (hash) => viewPhoto(context, hash),
      ),
      const SizedBox(height: 8),
    ],
    if (_recordings.isNotEmpty && !_voiceMode) ...[
      Align(
        alignment: Alignment.centerLeft,
        child: Wrap(
          spacing: 8,
          children: [
            for (final hash in _recordings)
              RecordingButton(key: ValueKey(hash), hash: hash),
          ],
        ),
      ),
      const SizedBox(height: 8),
    ],
    for (final file in _files)
      AttachmentRow(key: ValueKey('capture-file-${file.hash}'), file: file),
    if (_files.isNotEmpty) const SizedBox(height: 8),
  ];
}

/// Puts [text] into [field] in place of its selection, the cursor after
/// it; without a cursor, after what is there, on a line of its own.
void insertAtCursor(TextEditingController field, String text) {
  final value = field.value;
  final selection = value.selection;
  final String joined;
  final int end;
  if (selection.isValid) {
    joined = value.text.replaceRange(selection.start, selection.end, text);
    end = selection.start + text.length;
  } else {
    final before = value.text.isEmpty || value.text.endsWith('\n')
        ? value.text
        : '${value.text}\n';
    joined = '$before$text';
    end = joined.length;
  }
  field.value = TextEditingValue(
    text: joined,
    selection: TextSelection.collapsed(offset: end),
  );
}

/// The Nudge choices of the quick note, under its text: remind me in
/// 10 min, 30 min or 1 h. Part of the page, not a bar (UI laws,
/// "Capture").
class NudgeButtons extends StatelessWidget {
  /// The choices, each with the key its button carries.
  static const choices = [
    (Duration(minutes: 10), ValueKey('nudge10')),
    (Duration(minutes: 30), ValueKey('nudge30')),
    (Duration(hours: 1), ValueKey('nudge60')),
  ];

  final ValueChanged<Duration> onPicked;

  const NudgeButtons({super.key, required this.onPicked});

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final labels = [t.snooze10, t.snooze30, t.snooze60];
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(t.nudgeHeading, style: Theme.of(context).textTheme.labelLarge),
        const SizedBox(height: 8),
        Row(
          children: [
            for (final (i, (delay, key)) in choices.indexed) ...[
              if (i > 0) const SizedBox(width: 8),
              Expanded(
                child: OutlinedButton(
                  key: key,
                  onPressed: () => onPicked(delay),
                  child: Text(labels[i]),
                ),
              ),
            ],
          ],
        ),
      ],
    );
  }
}
