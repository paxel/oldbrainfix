import 'dart:async';
import 'dart:math' as math;
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'app_lock.dart';
import 'announcements.dart';
import 'attachment_views.dart';
import 'capture_choices.dart';
import 'context_platform.dart';
import 'drafts.dart';
import 'failure_dialog.dart';
import 'file_into_screen.dart';
import 'file_platform.dart';
import 'l10n.dart';
import 'location.dart';
import 'note_card.dart';
import 'note_editor_screen.dart';
import 'photo_platform.dart';
import 'photo_views.dart';
import 'place_names.dart';
import 'routine_editor_screen.dart';
import 'schedule_text.dart';
import 'shopping_pick_screen.dart';
import 'speech_platform.dart';
import 'sync_line.dart';
import 'tally_field_screen.dart';
import 'text_marks_view.dart';
import 'two_row_text_field.dart';
import 'voice_platform.dart';

/// A Note just captured, and for its location and for the rest of its
/// Capture Context whether it got attached, each once that is known;
/// with [editing] when "To full editor" opened it, done once the user
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
/// "To full editor" leaves Capture for the Note's editor; leaving that
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

/// Makes each line of the text of the Note [noteId] and of its
/// recordings' Transcripts a Line of the Shopping List [listId], a name
/// the list has bringing its Line back, and discards the Note, all in one
/// batch (#124). True when it did; false when the list can no longer be
/// added to, and null when the Note may no longer go: deleted, merged or
/// with something spawned from it meanwhile. Either way the Note stays.
bool? pourIntoShopping(Notes notes, String noteId, String listId) {
  final note = notes.byId(noteId);
  if (note == null || note.isTrashed || note.isMerged || note.spawned) {
    return null;
  }
  final text = [
    // Its boxes as they show, without their ids (ADR 0035).
    note.shownText,
    for (final hash in note.recordings) ?note.transcripts[hash],
  ].join('\n');
  try {
    notes.store.batch(() {
      ShoppingLists(notes.store).addLines(listId, text, deviceWallClock);
      notes.discard(noteId);
    });
    return true;
  } on StateError {
    return false;
  } on ArgumentError {
    return false;
  }
}

/// Waits for the Transcripts of the recordings [hashes] of the Note
/// [noteId], then pours it into the Shopping List [listId] as
/// [pourIntoShopping] does; whether it did. Until then the Note holds
/// all of it, so nothing is lost when the app ends meanwhile. Without a
/// Transcript, or when the list can no longer be added to, the Note
/// stays and [announce] tells why at the top.
Future<bool> intoShoppingLater(
  Notes notes,
  Transcriber? transcriber,
  String noteId,
  String listId,
  List<String> hashes, {
  required void Function({required bool untranscribed}) announce,
}) async {
  for (final hash in hashes) {
    if (transcriber == null || !await transcriber.transcribed(noteId, hash)) {
      if (notes.byId(noteId) case final note? when !note.isTrashed) {
        announce(untranscribed: true);
      }
      return false;
    }
  }
  final poured = pourIntoShopping(notes, noteId, listId);
  if (poured == false) announce(untranscribed: false);
  return poured ?? false;
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

/// Text capture, the quick note: opens with the keyboard up, Save writes the Note. The
/// position, and the calendar events and Bluetooth devices where they
/// are switched on, are looked up from the start, without asking, and
/// attached when they arrive: they never hold the Note back. Leaving by
/// Back saves too, and so does the app going to the background. While
/// Unsorted the Note is nudged in 10 min, 30 min or 1 h, picked with the
/// buttons at the bottom, each of which saves; Save means 1 h (UI laws,
/// "Capture").
///
/// "To full editor" saves the Note at once, Unsorted and nudged in 1 h,
/// and opens it in the Note editor in place of this screen (#142); not
/// offered behind the App Lock, as the editor shows the Areas, and not
/// while there is nothing to save.
///
/// Uncluttered by default (#142): what sorts the Note shows only once
/// this Device switched it on in Settings ▸ Quick note ([QuickNoteControl]),
/// in the Note card's action row (#113): "File into…", a Frog or Session
/// in the alarm clock's menu, and ⋮ with the Importance, "Make it a
/// Tally", which goes with the Frog and Session (Q481), and "Add to a
/// Shopping List"; and so does the format bar. The alarm clock itself,
/// for a Routine or Reminder, is always there (#223). Each opens its usual page
/// and comes back here with the choice shown at the top, not written yet;
/// all of it is written with the Note when the screen is left, nothing of
/// it while the Note is empty. A sorted Note, filed into an Area or Topic
/// or with something spawned from it, needs no Nudge: one Save takes the
/// three buttons' place, and so it does where the Nudge buttons are
/// switched off. The App Lock keeps the Areas and Tallies out of reach
/// here, so "File into…" and the Session are not offered behind it.
///
/// "Add to a Shopping List" (#124) makes each line typed, and of
/// each recording's Transcript, a Line of the list picked, and keeps no
/// Note; the other choices wait unshown meanwhile. It is not offered
/// with a photo, which no Line holds, nor behind the App Lock.
///
/// The Photo button takes a photo into the Note, and the Gallery button
/// picks pictures from the Device into it, several at once; the text is
/// then optional. Closing the picker without picking changes nothing.
///
/// The Paste button reads the clipboard, only then, as Android shows a
/// notice for every read: its text goes into the field at the cursor, its
/// pictures into the Note as photos, and a clipboard with neither is said
/// in a line above the field, which goes once the user types or pastes.
///
/// Something shared in from another app opens here with its
/// text, photos and sound files filled in, the sound files as voice
/// recordings, and the app it came from recorded. All of it goes into one
/// Note written at once, so nothing is lost.
///
/// The Voice button switches to voice: recording starts at once, the
/// elapsed time runs, and one big stop button ends it and comes back to
/// the text with the recording; the Nudge buttons and Back end it and
/// save; there is no hold-to-talk and no stop on
/// silence (UI laws, "Capture"). The first time, while the microphone is
/// not allowed yet, it explains why and asks. The app going to the
/// background ends the recording and keeps it. Text typed before goes
/// into the same Note.
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

  /// Fills the screen from the clipboard at once, as its Paste button
  /// does.
  final bool startPaste;

  /// The system's file picker, for the File button (#267).
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
  /// The room the text keeps, about two lines, where the buttons under it
  /// scroll instead.
  static const _textRoom = 56.0;

  /// What this Device's quick note shows, read once (ADR 0008); the
  /// format bar only in Advanced (#197).
  late final _shows = {
    for (final control in QuickNoteControl.values)
      if (widget.notes.store.quickNoteShows(control) &&
          (_advanced || control != QuickNoteControl.formatBar))
        control,
  };

  /// Whether Frogs, Sessions, Tallies and the format bar are offered,
  /// read once (#197).
  late final _advanced = widget.notes.store.advanced;
  late final _nudgeButtons = widget.notes.store.quickNoteNudgeButtonsOn;

  /// With the format bar, its tags highlighted and a formatted text
  /// formatted as typed (ADR 0033).
  late final TextEditingController _text =
      _shows.contains(QuickNoteControl.formatBar)
      ? TagTextController()
      : TextEditingController();

  /// The text as last seen, to tell when it changed.
  var _typed = '';
  late final Future<Coordinates?> _here;
  late final Future<ContextExtras> _extras;
  late final AppLifecycleListener _lifecycle;

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

  /// Set when the Note, Hidden from AI, was kept out of the Shared Area
  /// picked (#267), to say so at the top once.
  var _notFiledHidden = false;

  /// The files being picked or shared in and written; null while none
  /// are.
  Future<void>? _filing;

  /// Whether the screen shows voice instead of the text field.
  var _voiceMode = false;

  /// Whether voice found the microphone not allowed or not working.
  var _micMissing = false;

  /// The recording being started and then running, whether it did start;
  /// null while none runs.
  Future<bool>? _recording;

  /// Ending the recording and keeping it, once asked for.
  Future<void>? _stopping;

  /// The pictures being picked and written; null while none are.
  Future<void>? _picking;

  /// The clipboard being read and filled in; null while it is not.
  Future<void>? _pasting;

  /// Whether the last Paste found nothing to paste, said above the field.
  var _pasteEmpty = false;

  /// Counts the recording's seconds while it runs.
  Timer? _ticker;
  var _elapsed = Duration.zero;

  /// Set once leaving began, so a second tap does not leave twice.
  var _leaving = false;

  /// The Area or Topic the Note goes into, picked with "File into…";
  /// Unsorted while null.
  late String? _place = widget.place;

  /// The Area and Topic of [_place], to show it.
  Area? _area;
  Topic? _topic;

  /// The place the written Note holds; null while it holds none.
  String? _filed;

  /// Whether this Device may spawn from a Note where [_place] is: only a
  /// Manager does in a Shared Area (ADR 0013).
  var _mayManage = true;

  /// The Importance picked in ⋮.
  var _importance = Importance.normal;

  /// The Routines, Reminders and Sessions set up here, not written yet,
  /// and those written meanwhile, as the app went to the background.
  final _schedules = <ScheduleChoice>[];
  final _writtenSchedules = <ScheduleChoice>[];

  /// Whether a Frog was asked for, and whether it is written.
  var _frog = false;
  var _frogWritten = false;

  /// The first field of the Tally the Note becomes, and whether written.
  TallyFieldChoice? _tally;
  var _tallyWritten = false;

  /// The Shopping List the lines go into instead of a Note (#124), with
  /// its title; null while they make a Note. While one is picked the
  /// other choices wait unshown and unwritten, and a Note written
  /// meanwhile holds only the text and recordings.
  ({String id, String title})? _shopping;

  AppLock? _lock;

  @override
  void initState() {
    super.initState();
    _text.addListener(_textChanged);
    _placed();
    // The fix may take a while; it starts while the user types.
    _here = currentIfGranted(widget.location);
    _extras = currentExtras(widget.notes.store, widget.contextSource);
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
          // Recording stops in the background: what is there is kept.
          unawaited(_stopRecording());
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

  /// Locking again closes what this screen opened, such as "File
  /// into…" with its Areas: only Capture itself needs no unlocking. What
  /// was set up stays.
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
    _ticker?.cancel();
    _lifecycle.dispose();
    _text.removeListener(_textChanged);
    _text.dispose();
    super.dispose();
  }

  /// Whether the Note leaves Unsorted: filed, with something spawned
  /// from it, or going into a Shopping List. It needs no Nudge then.
  bool get _sorted =>
      _shopping != null ||
      _place != null ||
      _schedules.isNotEmpty ||
      _writtenSchedules.isNotEmpty ||
      _frog ||
      _tally != null;

  /// What is typed or pasted answers "nothing to paste".
  void _textChanged() {
    final text = _text.text;
    if (text == _typed) return;
    _typed = text;
    if (_pasteEmpty) setState(() => _pasteEmpty = false);
  }

  /// Writes the text as a Note, or into the one already written, and
  /// what was set up for it; nothing while it is empty and there is no
  /// Note yet, so nothing hangs on an empty Note. A new Unsorted Note is
  /// nudged [nudgeIn] from now, 1 h when null; an existing one only when
  /// [nudgeIn] is given. A photo Note keeps its text when it is emptied.
  /// All of it is one batch: when a write on the way fails, none of it
  /// is written, and the screen still holds all of it (#113).
  void _write(Duration? nudgeIn) {
    final text = _text.text.trim();
    final captured = _captured;
    if (captured == null && text.isEmpty) return;
    final notes = widget.notes;
    final (created, choices) = notes.store.batch(() {
      // Going into a Shopping List, the Note waits without the rest.
      final choose = _shopping == null;
      if (captured == null) {
        final note = _captureNote(text, nudgeIn: nudgeIn);
        return (note, choose ? _writeChoices(note.id) : null);
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
      return (null, choose ? _writeChoices(captured.noteId) : null);
    });
    if (created != null) _adopt(created);
    if (choices case (:final filed)) _choicesWritten(filed);
    if (_notFiledHidden && mounted) {
      _notFiledHidden = false;
      final t = context.t;
      announceFailure(
        context,
        t.captureNotFiledHidden,
        title: t.captureNotFiledHiddenTitle,
        cause: t.captureNotFiledHiddenCause,
        fix: t.captureNotFiledHiddenFix,
        details: 'capture: hidden from AI, not filed into $_place',
      );
    }
  }

  /// Writes what was set up here into the Note [noteId], one after the
  /// other with nothing in between: where it goes first, so what is
  /// spawned lies in its Area; each spawn once. Returns where the Note
  /// goes, or null when there is no such Note and nothing was written;
  /// [_choicesWritten] marks them written once the batch is committed.
  ({String? filed})? _writeChoices(String noteId) {
    final notes = widget.notes;
    final store = notes.store;
    final note = notes.byId(noteId);
    if (note == null) return null;
    final place = _place;
    var filed = place ?? note.place;
    if (place != null && note.place != place) {
      // A Note Hidden from AI, as by the Area it was captured in, goes
      // into no Shared Area (#267): it stays, said at the top, as
      // Capture asks nothing.
      final area = store.areaOf(place);
      if (area != null &&
          store.isShared(area) &&
          store.hiddenFromAiBy(noteId).isNotEmpty) {
        _notFiledHidden = true;
        filed = note.place;
      } else {
        notes.file(noteId, place);
      }
    }
    if (note.importance != _importance) {
      notes.setImportance(noteId, _importance);
    }
    if (_tally case final tally? when !_tallyWritten) {
      Tallies(store).addField(
        noteId,
        name: tally.name,
        unit: tally.unit,
        target: tally.target,
        rule: tally.rule,
        check: tally.check,
      );
    }
    for (final choice in _schedules) {
      choice.spawnFrom(store, noteId);
    }
    if (_frog && !_frogWritten) Frogs(store).spawn(noteId);
    return (filed: filed);
  }

  /// Marks what [_writeChoices] wrote as written, the Note in [filed].
  void _choicesWritten(String? filed) {
    _filed = filed;
    if (_tally != null) _tallyWritten = true;
    _writtenSchedules.addAll(_schedules);
    _schedules.clear();
    if (_frog) _frogWritten = true;
  }

  /// Reads where [_place] is, and what may be spawned there.
  void _placed() {
    final store = widget.notes.store;
    final places = Places(store);
    final place = _place;
    _topic = place == null ? null : places.topicById(place);
    _area = place == null ? null : places.areaById(_topic?.areaId ?? place);
    _mayManage = store.mayManage(store.areaOf(place ?? ''));
  }

  /// Opens "File into…" for the Note; the Area or Topic picked shows.
  Future<void> _file() async {
    final places = Places(widget.notes.store);
    final picked = await Navigator.of(context).push(
      MaterialPageRoute<String>(
        builder: (_) => FileIntoScreen(places: places, current: _place),
      ),
    );
    if (!mounted || picked == null) return;
    setState(() {
      _place = picked;
      _placed();
      _keepWhatFits();
    });
  }

  /// Drops what was set up and may not go where the Note now goes: in a
  /// Shared Area only a Manager spawns (ADR 0013), and a Session there
  /// picks only Tallies of that Area (ADR 0029). Nothing written yet is
  /// lost but these choices, which the screen stops showing.
  void _keepWhatFits() {
    if (!_mayManage) {
      _schedules.clear();
      if (!_frogWritten) _frog = false;
      return;
    }
    final fits = Sessions(widget.notes.store).choices(_place ?? '').toSet();
    _schedules.removeWhere(
      (choice) => choice.session?.pool.any((id) => !fits.contains(id)) ?? false,
    );
  }

  /// The Note being captured, as an editor opened from here shows it.
  CaptureDraft get _draft => (text: _text.text, place: _place);

  /// Sets up a Routine, Reminder or Session in its editor, or asks for a
  /// Frog, which needs nothing more.
  Future<void> _spawn(Spawn spawn) async {
    if (spawn == Spawn.frog) {
      setState(() => _frog = true);
      return;
    }
    final choice = await chooseSchedule(
      context,
      widget.notes,
      Places(widget.notes.store),
      _draft,
      spawn: spawn,
    );
    if (!mounted || choice == null) return;
    setState(() => _schedules.add(choice));
  }

  /// "Add to a Shopping List" in ⋮, read as it opens: offered while the
  /// capture holds no photo, which no Line can hold, nothing written
  /// that hangs on the Note, and a recording only where a speech model
  /// can give its Transcript; never behind the App Lock, which keeps the
  /// lists out of reach, nor without a list to add to. With one list to
  /// add to it names it, "Add to Weekly", and picks it without the page
  /// (#276).
  ({String label, VoidCallback run})? _shoppingOffer() {
    if ((_lock?.locked ?? false) || _photos.isNotEmpty || _files.isNotEmpty) {
      return null;
    }
    if (_writtenSchedules.isNotEmpty || _frogWritten || _tallyWritten) {
      return null;
    }
    if (_recordings.isNotEmpty &&
        SpeechScope.of(context)?.model.current == null) {
      return null;
    }
    final t = context.t;
    return switch (shoppingListsToAddTo(widget.notes.store, _lock)) {
      [] => null,
      [final only] => (
        label: t.addToNamedShoppingList(shoppingTitle(t, only)),
        run: () => _intoList(only),
      ),
      _ => (label: t.addToShoppingList, run: _pickShopping),
    };
  }

  /// The lines go into the Shopping List [note]; it shows at the top.
  void _intoList(Note note) {
    if (_photos.isNotEmpty || _files.isNotEmpty) return;
    setState(
      () => _shopping = (id: note.id, title: shoppingTitle(context.t, note)),
    );
  }

  /// Picks the Shopping List the lines go into; it shows at the top.
  Future<void> _pickShopping() async {
    final store = widget.notes.store;
    final picked = await Navigator.of(context).push(
      MaterialPageRoute<String>(
        builder: (_) =>
            ShoppingPickScreen(store: store, current: _shopping?.id),
      ),
    );
    if (!mounted || picked == null) return;
    if (widget.notes.byId(picked) case final note?) _intoList(note);
  }

  /// Sets up the first field of the Tally the Note becomes.
  Future<void> _makeTally() async {
    final field = await chooseTallyField(context, widget.notes);
    if (!mounted || field == null) return;
    setState(() => _tally = field);
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
  /// nudged [nudgeIn] from now, 1 h when null, unless it is sorted.
  Note _captureNote(
    String text, {
    Duration? nudgeIn,
    List<Uint8List> photos = const [],
    List<Uint8List> recordings = const [],
    List<NewFile> files = const [],
  }) => widget.notes.capture(
    text,
    format: _formatOf(text) ?? TextFormats.plain,
    place: _shopping == null ? _place : null,
    // A sorted Note needs no Nudge (#113).
    nudgeIn: _sorted ? null : nudgeIn ?? Notes.defaultNudge,
    photos: photos,
    recordings: recordings,
    files: files,
    sourceApp: widget.shared?.sourceApp,
  );

  /// Formatted for a text formatted with the format bar, or that uses a
  /// format, pasted, shared or typed as Markdown (ADR 0033); null leaves
  /// it as it is.
  String? _formatOf(String text) {
    final formatted = switch (_text) {
      TagTextController(markdown: true) => true,
      _ => Markdown.uses(text),
    };
    return formatted ? TextFormats.markdown : null;
  }

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
    _filed = note.place;
    _photos = note.photos;
    _recordings = note.recordings;
    _files = note.attachments;
  }

  /// Writes [photos] into the Note, capturing it with the text so far
  /// when there is none yet.
  void _addPhotos(List<Uint8List> photos) {
    if (photos.isEmpty) return;
    // No Line holds a photo: the capture stays a Note, to keep it.
    _shopping = null;
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
    // No Line holds a file: the capture stays a Note, to keep it.
    _shopping = null;
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
      if (pasted.text.trim().isNotEmpty) _insert(pasted.text);
      _addPhotos(pasted.photos);
    });
  }

  /// Puts [text] into the field in place of the selection, the cursor
  /// after it; without a cursor, after what is there, on a line of its
  /// own.
  void _insert(String text) {
    final value = _text.value;
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
    _text.value = TextEditingValue(
      text: joined,
      selection: TextSelection.collapsed(offset: end),
    );
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
    if (_recording != null || _leaving) return;
    FocusScope.of(context).unfocus();
    setState(() {
      _voiceMode = true;
      _micMissing = false;
      _elapsed = Duration.zero;
    });
    final voice = widget.voice;
    var allowed = await voice.granted();
    if (!mounted || _leaving) return;
    if (!allowed) allowed = await explainAndAskMicrophone(context, voice);
    if (!mounted || _leaving) return;
    if (!allowed) {
      setState(() => _micMissing = true);
      return;
    }
    // Recording ends any playing, and the play buttons show it.
    await VoiceScope.playbackOf(context)?.stop();
    if (!mounted || _leaving) return;
    final recording = _recording = voice.start();
    final started = await recording;
    // Stopped meanwhile, by Back or the background: nothing to count.
    if (!mounted || _recording != recording) return;
    if (!started) {
      _recording = null;
      setState(() => _micMissing = true);
      return;
    }
    _ticker = Timer.periodic(const Duration(seconds: 1), (timer) {
      setState(() => _elapsed = Duration(seconds: timer.tick));
    });
  }

  /// Ends the recording and writes it into the Note; the same ending
  /// when asked again, nothing when none runs.
  Future<void> _stopRecording() {
    final recording = _recording;
    if (recording != null) {
      _recording = null;
      _ticker?.cancel();
      _stopping = _keep(recording);
    }
    return _stopping ?? Future.value();
  }

  /// Waits for [recording] to have started, ends it and keeps what was
  /// recorded; back to the text field afterwards, with the recording.
  Future<void> _keep(Future<bool> recording) async {
    if (await recording) {
      final bytes = await widget.voice.stop();
      if (bytes != null) _addRecording(bytes);
    }
    _stopping = null;
    // Leaving, the text field never comes back to pop the keyboard up.
    if (mounted && !_leaving) setState(() => _voiceMode = false);
  }

  /// Saves and leaves: by Save or Back (1 h), or a picked Nudge time. A running recording is ended and kept first, and
  /// pictures still being picked are written first.
  Future<void> _save([Duration? nudgeIn]) async {
    if (_leaving) return;
    _leaving = true;
    await _stopRecording();
    await _picking;
    if (!mounted) return;
    try {
      if (_shopping case final shopping?) {
        Navigator.of(context).pop(_intoShopping(shopping.id));
        return;
      }
      _write(nudgeIn);
    } catch (_) {
      // Nothing of it was written: the screen stays, to change and leave
      // again.
      _leaving = false;
      rethrow;
    }
    Navigator.of(context).pop(_captured);
  }

  /// "To full editor": saves the Note as leaving does, Unsorted and
  /// nudged in 1 h unless it is sorted, and opens its editor in place of
  /// this screen, so leaving the editor comes back to where Capture was
  /// opened from. A recording running is ended and kept first, and
  /// pictures still being picked are written first. Nothing saved, as
  /// for an empty Note, the screen stays.
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
    unawaited(
      Navigator.of(context)
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

  /// Makes each line of the text a Line of the Shopping List [listId],
  /// and the Note written meanwhile goes, with the lines of its
  /// recordings' Transcripts (#124). While a recording has no Transcript
  /// yet, the Note stays with all of it and goes once it has one
  /// ([intoShoppingLater]). What cannot go stays a Note, announced at the
  /// top. Returns the Note left to show; null when none is.
  Captured? _intoShopping(String listId) {
    final notes = widget.notes;
    final t = context.t;
    final announcements = AnnouncementsScope.of(context);
    // Taken now: the announcement may come once this page has closed.
    final navigator = Navigator.of(context, rootNavigator: true);
    void announce({required bool untranscribed}) => announceFailureWith(
      announcements,
      navigator,
      untranscribed ? t.captureShoppingUntranscribed : t.captureShoppingGone,
      title: t.captureShoppingNotAddedTitle,
      cause: untranscribed
          ? t.captureShoppingUntranscribedCause
          : t.captureShoppingGoneCause,
      fix: t.captureShoppingKeptFix,
      details: untranscribed
          ? 'capture into a Shopping List: a recording got no Transcript'
          : 'capture into a Shopping List: the list can no longer be added to',
    );
    // The list used last goes to the top of the pick (#276).
    void used() => notes.store.lastShoppingList = listId;
    if (_captured == null) {
      try {
        final added = ShoppingLists(notes.store)
            .addLines(listId, _text.text.trim(), deviceWallClock);
        if (added.isNotEmpty) used();
        return null;
      } on StateError {
        // The list went meanwhile: a Note with all that was set up.
      } on ArgumentError {
        // The same.
      }
      _shopping = null;
      _write(null);
      announce(untranscribed: false);
      return _captured;
    }
    // The Note holds what was typed last, in case it stays.
    _write(null);
    final captured = _captured;
    if (captured == null) return null;
    final note = notes.byId(captured.noteId);
    final waiting = [
      if (note != null)
        for (final hash in note.recordings)
          if (!note.transcripts.containsKey(hash) &&
              !note.removedTranscripts.contains(hash))
            hash,
    ];
    if (waiting.isEmpty) {
      if (pourIntoShopping(notes, captured.noteId, listId) == true) {
        used();
        return null;
      }
      announce(untranscribed: false);
      return captured;
    }
    used();
    return (
      noteId: captured.noteId,
      editing: null,
      attached: [
        ...captured.attached,
        intoShoppingLater(
          notes,
          SpeechScope.of(context)?.transcriber,
          captured.noteId,
          listId,
          waiting,
          announce: announce,
        ),
      ],
    );
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final recording = _voiceMode && !_micMissing;
    // With the keyboard up the room is small: the photos and recordings
    // scroll with the text, the buttons stay.
    final text = Expanded(
      child: _voiceMode
          ? Column(
              children: [
                ..._attachments(context),
                Expanded(child: _voice(context)),
              ],
            )
          : CustomScrollView(
              slivers: [
                if (_choices(context) case final choices?)
                  SliverToBoxAdapter(child: choices),
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
                SliverFillRemaining(
                  hasScrollBody: false,
                  child: _field(context),
                ),
              ],
            ),
    );
    final below = [
      if (!recording) ...[_captureButtons(context), ?_actionLine(context)],
      if (_sorted || !_nudgeButtons)
        SizedBox(
          width: double.infinity,
          child: FilledButton(
            key: const ValueKey('captureSave'),
            onPressed: _save,
            child: Text(t.save),
          ),
        )
      else
        NudgeButtons(onPicked: _save),
      // The format bar rides on the keyboard, over the bottom.
      if (_shows.contains(QuickNoteControl.formatBar) &&
          !recording &&
          MediaQuery.viewInsetsOf(context).bottom > 0)
        const SizedBox(height: TwoRowTextField.barHeight),
    ];
    // The buttons under the text, in at most [room] when given.
    Widget column(double? room) => Column(
      children: [
        text,
        if (room == null)
          ...below
        else
          ConstrainedBox(
            constraints: BoxConstraints(maxHeight: room),
            child: SingleChildScrollView(
              child: Column(mainAxisSize: MainAxisSize.min, children: below),
            ),
          ),
      ],
    );
    // Leaving by Back saves as well (UI laws, "Capture").
    return PopScope(
      canPop: false,
      onPopInvokedWithResult: (didPop, _) {
        if (!didPop) unawaited(_save());
      },
      child: Scaffold(
        appBar: AppBar(
          // While recording, the big stop button comes first.
          actions: recording
              ? null
              : [TextButton(onPressed: _save, child: Text(t.save))],
          flexibleSpace: const SyncLine(),
        ),
        body: SafeArea(
          child: Padding(
            padding: const EdgeInsets.all(16),
            // The format bar rides on the keyboard: on a small screen the
            // buttons under the text scroll, and the text keeps two lines.
            child: _shows.contains(QuickNoteControl.formatBar)
                ? LayoutBuilder(
                    builder: (context, constraints) => column(
                      math.max(0.0, constraints.maxHeight - _textRoom),
                    ),
                  )
                : column(null),
          ),
        ),
      ),
    );
  }

  /// The text field; with the format bar where it is switched on.
  Widget _field(BuildContext context) {
    final t = context.t;
    final autofocus =
        !widget.startVoice &&
        !widget.startPhoto &&
        !widget.startGallery &&
        !widget.startFile;
    final decoration = InputDecoration(
      hintText: _photos.isEmpty && _recordings.isEmpty && _files.isEmpty
          ? t.captureHint
          : t.capturePhotoHint,
    );
    if (_shows.contains(QuickNoteControl.formatBar)) {
      return TwoRowTextField(
        controller: _text,
        twoRows: false,
        autofocus: autofocus,
        formats: true,
        decoration: decoration,
      );
    }
    return TextField(
      controller: _text,
      autofocus: autofocus,
      maxLines: null,
      decoration: decoration,
    );
  }

  /// The line above the Nudge buttons or Save: "To full editor", then
  /// the action row with what Settings ▸ Quick note switched on; null
  /// while neither shows.
  Widget? _actionLine(BuildContext context) {
    final editor = _toEditorButton(context);
    final actions = _actions(context);
    if (editor == null) return actions;
    return Row(
      children: [
        Expanded(
          child: Align(alignment: Alignment.centerLeft, child: editor),
        ),
        // The row's icons side by side, at the end.
        if (actions != null) IntrinsicWidth(child: actions),
      ],
    );
  }

  /// "To full editor": none behind the App Lock or while the lines go
  /// into a Shopping List, and it waits while there is nothing to save.
  Widget? _toEditorButton(BuildContext context) {
    if ((_lock?.locked ?? false) || _shopping != null) return null;
    return ListenableBuilder(
      listenable: _text,
      builder: (context, _) => TextButton.icon(
        key: const ValueKey('captureToEditor'),
        onPressed: _captured != null || _text.text.trim().isNotEmpty
            ? _toEditor
            : null,
        icon: const Icon(Icons.open_in_full),
        // Beside the row's icons: a long word shrinks rather than being
        // cut.
        label: FittedBox(
          fit: BoxFit.scaleDown,
          child: Text(context.t.toFullEditor, maxLines: 1, softWrap: false),
        ),
      ),
    );
  }

  /// The Note card's action row, after "To full editor": the alarm clock
  /// for a Routine or Reminder (#223), and what Settings ▸ Quick note
  /// switched on; null while that is nothing, as while the lines go into
  /// a Shopping List or a Shared Area lets only its Managers set them up.
  /// What it opens comes back here. Behind the App Lock it offers neither
  /// "File into…" nor the Session, which show Areas and Tallies.
  Widget? _actions(BuildContext context) {
    final locked = _lock?.locked ?? false;
    // Lines need none of these: they wait while a Shopping List is picked.
    final shopping = _shopping != null;
    final importance = _shows.contains(QuickNoteControl.importance);
    final toShopping = _shows.contains(QuickNoteControl.shopping);
    final onFile = _shows.contains(QuickNoteControl.fileInto)
        ? (locked || shopping ? null : _file)
        : null;
    // The alarm clock is always there for a Routine or Reminder (#223);
    // the switch adds Frog and Session to it.
    final schedule = _shows.contains(QuickNoteControl.schedule);
    final onSchedule = _mayManage && !shopping ? _spawn : null;
    // "Make it a Tally" goes with the Frog and Session (Q481), only in
    // Advanced (#197).
    final onMakeTally = schedule && !shopping && _advanced ? _makeTally : null;
    if (onFile == null &&
        onSchedule == null &&
        onMakeTally == null &&
        !importance &&
        !toShopping) {
      return null;
    }
    return NoteActionRow(
      importance: _importance,
      onFile: onFile,
      onSchedule: onSchedule,
      spawns: [
        for (final spawn in Spawn.values)
          if (spawn == Spawn.routine ||
              spawn == Spawn.reminder ||
              // Frogs and Sessions are made only in Advanced (#197), and
              // only once switched on.
              (schedule && _advanced && (!locked || spawn != Spawn.session)))
            spawn,
      ],
      actions: importance || toShopping || onMakeTally != null
          ? NoteActions(
              onImportance: importance && !shopping
                  ? (picked) => setState(() => _importance = picked)
                  : null,
              isTally: () => _tally != null,
              onMakeTally: onMakeTally,
              addToShopping: toShopping ? _shoppingOffer : null,
            )
          : null,
    );
  }

  /// What was set up for the Note, at the top: where it goes, its
  /// Importance, its Tally, Routines, Reminders, Sessions and Frog, each
  /// with a button that drops it again while it is not written; null
  /// while there is nothing.
  Widget? _choices(BuildContext context) {
    final t = context.t;
    final locale = Localizations.localeOf(context).toString();
    Widget chip(String key, String label, VoidCallback? onDropped) => InputChip(
      key: ValueKey(key),
      label: Text(label),
      onDeleted: onDropped,
      deleteButtonTooltipMessage: t.remove,
    );
    String scheduled(ScheduleChoice choice) {
      final when = scheduleText(t, locale, choice.schedule);
      return choice.session != null
          ? t.captureChoiceSession(when)
          : choice.schedule.once
          ? t.captureChoiceReminder(when)
          : t.captureChoiceRoutine(when);
    }

    // The other choices wait unshown while the lines go into a list.
    if (_shopping case final shopping?) {
      return Padding(
        padding: const EdgeInsets.only(bottom: 8),
        child: Align(
          alignment: Alignment.centerLeft,
          child: chip(
            'choice-shopping',
            t.captureChoiceShopping(shopping.title),
            () => setState(() => _shopping = null),
          ),
        ),
      );
    }
    final area = _area;
    final topic = _topic;
    final chips = [
      if (area != null)
        chip(
          'choice-place',
          t.captureChoicePlace(
            [
              areaName(t, area),
              if (topic != null) topicName(t, topic),
            ].join(' / '),
          ),
          // A written Note goes on into an Area, not back to Unsorted.
          _filed == null
              ? () => setState(() {
                  _place = null;
                  _placed();
                })
              : null,
        ),
      if (_importance != Importance.normal)
        chip(
          'choice-importance',
          _importance == Importance.important
              ? t.importanceImportant
              : t.importanceUnimportant,
          null,
        ),
      if (_tally case final tally?)
        chip(
          'choice-tally',
          t.captureChoiceTally(tally.name),
          _tallyWritten ? null : () => setState(() => _tally = null),
        ),
      for (final (i, choice) in _writtenSchedules.indexed)
        chip('choice-written-$i', scheduled(choice), null),
      for (final (i, choice) in _schedules.indexed)
        chip(
          'choice-schedule-$i',
          scheduled(choice),
          () => setState(() => _schedules.remove(choice)),
        ),
      if (_frog)
        chip(
          'choice-frog',
          t.frog,
          _frogWritten ? null : () => setState(() => _frog = false),
        ),
    ];
    if (chips.isEmpty) return null;
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Align(
        alignment: Alignment.centerLeft,
        child: Wrap(spacing: 8, runSpacing: 4, children: chips),
      ),
    );
  }

  /// The Note's photos, and its recordings outside voice, above the text.
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

  /// The Photo, Voice, Gallery, Paste and File buttons above the Nudge
  /// choices, side by side, each with its icon above its one-word label
  /// so five fit the row; none while recording, where the big stop button comes
  /// first. Below the text field, they rise with the keyboard.
  Widget _captureButtons(BuildContext context) {
    final t = context.t;
    final buttons = [
      if (widget.photos.available)
        _captureButton(
          key: const ValueKey('capturePhoto'),
          onPressed: _takePhoto,
          icon: Icons.photo_camera_outlined,
          label: t.photo,
        ),
      if (widget.voice.available && !_voiceMode)
        _captureButton(
          key: const ValueKey('captureVoice'),
          onPressed: _openVoice,
          icon: Icons.mic_none,
          label: t.voice,
        ),
      if (widget.photos.pickable)
        _captureButton(
          key: const ValueKey('captureGallery'),
          onPressed: _pickPhotos,
          icon: Icons.photo_library_outlined,
          label: t.gallery,
        ),
      if (widget.photos.pasteable)
        _captureButton(
          key: const ValueKey('capturePaste'),
          onPressed: _paste,
          icon: Icons.content_paste,
          label: t.paste,
        ),
      if (widget.files.pickable)
        _captureButton(
          key: const ValueKey('captureFile'),
          onPressed: _pickFiles,
          icon: Icons.attach_file,
          label: t.file,
        ),
    ];
    if (buttons.isEmpty) return const SizedBox.shrink();
    return Padding(
      padding: const EdgeInsets.only(top: 8, bottom: 16),
      child: Row(
        children: [
          for (final (i, button) in buttons.indexed) ...[
            if (i > 0) const SizedBox(width: 8),
            Expanded(child: button),
          ],
        ],
      ),
    );
  }

  /// One big capture button: the icon above the label. In the theme's
  /// full accent, as the "+": a pale tonal fill melts into the light
  /// teal ground (UI laws, "Look").
  Widget _captureButton({
    required Key key,
    required VoidCallback onPressed,
    required IconData icon,
    required String label,
  }) => FilledButton(
    key: key,
    onPressed: onPressed,
    style: _bigButton,
    child: Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(icon, size: 28),
        const SizedBox(height: 4),
        // Five in a row: a long word shrinks rather than being cut.
        FittedBox(
          fit: BoxFit.scaleDown,
          child: Text(label, maxLines: 1, softWrap: false),
        ),
      ],
    ),
  );

  static final _bigButton = FilledButton.styleFrom(
    minimumSize: const Size.fromHeight(56),
    padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 8),
  );

  /// Voice: the text typed so far, then the running time and the big
  /// stop button; or, without the microphone, why there is none.
  Widget _voice(BuildContext context) {
    final t = context.t;
    final theme = Theme.of(context);
    final typed = _text.text.trim();
    return Column(
      children: [
        if (typed.isNotEmpty)
          Align(
            alignment: Alignment.centerLeft,
            child: Text(
              typed,
              maxLines: 3,
              overflow: TextOverflow.ellipsis,
              style: theme.textTheme.bodyLarge,
            ),
          ),
        Expanded(
          child: Center(
            child: SingleChildScrollView(
              child: _micMissing
                  ? Column(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Text(t.microphoneMissing, textAlign: TextAlign.center),
                        const SizedBox(height: 16),
                        FilledButton(
                          onPressed: _openVoice,
                          child: Text(t.allowAccess),
                        ),
                        TextButton(
                          onPressed: () => setState(() => _voiceMode = false),
                          child: Text(t.typeInstead),
                        ),
                      ],
                    )
                  : Column(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Text(t.recording, style: theme.textTheme.titleMedium),
                        const SizedBox(height: 8),
                        Text(
                          elapsedText(_elapsed),
                          key: const ValueKey('elapsed'),
                          style: theme.textTheme.displayMedium,
                        ),
                        const SizedBox(height: 24),
                        SizedBox.square(
                          dimension: 160,
                          child: FilledButton(
                            key: const ValueKey('stopRecording'),
                            onPressed: _stopRecording,
                            style: FilledButton.styleFrom(
                              shape: const CircleBorder(),
                              padding: EdgeInsets.zero,
                            ),
                            child: Icon(
                              Icons.stop,
                              size: 96,
                              semanticLabel: t.stopRecording,
                            ),
                          ),
                        ),
                        const SizedBox(height: 8),
                        Text(t.stopRecording),
                      ],
                    ),
            ),
          ),
        ),
      ],
    );
  }
}

/// The Nudge choices at the bottom of a capture screen: remind me in
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
