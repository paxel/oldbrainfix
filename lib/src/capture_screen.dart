import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'app_lock.dart';
import 'announcements.dart';
import 'capture_choices.dart';
import 'context_platform.dart';
import 'failure_dialog.dart';
import 'file_into_screen.dart';
import 'l10n.dart';
import 'location.dart';
import 'note_card.dart';
import 'photo_platform.dart';
import 'photo_views.dart';
import 'place_names.dart';
import 'routine_editor_screen.dart';
import 'schedule_text.dart';
import 'tally_field_screen.dart';
import 'voice_platform.dart';

/// A Note just captured, and for its location and for the rest of its
/// Capture Context whether it got attached, each once that is known.
typedef Captured = ({String noteId, List<Future<bool>> attached});

/// Opens text Capture, or voice Capture recording at once when [voice] is
/// set, or the camera at once when [photo] is set, filing the new Note into the Area or Topic [place] when one is
/// given, or with the [shared] item from another app filled in. [onChanged] runs once the user left Capture, for the caller to
/// show the new Note, and again when its location or more of its Capture
/// Context arrived later.
/// [onCaptured] gets the new Note's id first, when there is one.
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
  required VoidCallback onChanged,
  ValueChanged<String>? onCaptured,
}) async {
  final location = LocationScope.of(context);
  final photos = PhotoScope.of(context);
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
        place: place,
        shared: shared,
        startVoice: voice,
        startPhoto: photo,
        startGallery: gallery,
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

/// Text capture: opens with the keyboard up, Save writes the Note. The
/// position, and the calendar events and Bluetooth devices where they
/// are switched on, are looked up from the start, without asking, and
/// attached when they arrive: they never hold the Note back. Leaving by
/// Back saves too, and so does the app going to the background. While
/// Unsorted the Note is nudged in 10 min, 30 min or 1 h, picked with the
/// buttons at the bottom, each of which saves; Save means 1 h (UI laws,
/// "Capture").
///
/// Above them sits the Note card's action row (#113): "File into…", a
/// Routine, Reminder, Frog or Session, and ⋮ with the Importance and
/// "Make it a Tally". Each opens its usual page and comes back here with
/// the choice shown at the top, not written yet; all of it is written
/// with the Note when the screen is left, nothing of it while the Note is
/// empty. A sorted Note, filed into an Area or Topic or with something
/// spawned from it, needs no Nudge: one Save takes the three buttons'
/// place. The App Lock keeps the Areas and Tallies out of reach here, so
/// "File into…" and the Session are not offered behind it.
///
/// The Photo button takes a photo into the Note, and the Gallery button
/// picks pictures from the Device into it, several at once; the text is
/// then optional. Closing the picker without picking changes nothing. Something shared in from another app opens here with its
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

  const CaptureScreen({
    super.key,
    required this.notes,
    this.location = const NoLocation(),
    this.photos = const NoPhotos(),
    this.voice = const NoVoice(),
    this.contextSource = const NoContext(),
    this.place,
    this.shared,
    this.startVoice = false,
    this.startPhoto = false,
    this.startGallery = false,
  });

  @override
  State<CaptureScreen> createState() => _CaptureScreenState();
}

class _CaptureScreenState extends State<CaptureScreen> {
  final _text = TextEditingController();
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

  AppLock? _lock;

  @override
  void initState() {
    super.initState();
    _placed();
    // The fix may take a while; it starts while the user types.
    _here = currentIfGranted(widget.location);
    _extras = currentExtras(widget.notes.store, widget.contextSource);
    // A share is meant to be kept: it is written at once, as one Note.
    final shared = widget.shared;
    if (shared != null && !shared.isEmpty) {
      _text.text = shared.text;
      _create(
        shared.text.trim(),
        photos: shared.photos,
        recordings: [
          for (final recording in shared.recordings) recording.bytes,
        ],
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
    _text.dispose();
    super.dispose();
  }

  /// Whether the Note leaves Unsorted: filed, or with something spawned
  /// from it. It needs no Nudge then.
  bool get _sorted =>
      _place != null ||
      _schedules.isNotEmpty ||
      _writtenSchedules.isNotEmpty ||
      _frog ||
      _tally != null;

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
      if (captured == null) {
        final note = _captureNote(text, nudgeIn: nudgeIn);
        return (note, _writeChoices(note.id));
      }
      final note = notes.byId(captured.noteId);
      if (text.isNotEmpty &&
          note != null &&
          note.textEditable &&
          note.text != text) {
        notes.edit(captured.noteId, text);
      }
      if (nudgeIn != null && !_sorted) {
        notes.setNudge(captured.noteId, nudgeIn);
      }
      return (null, _writeChoices(captured.noteId));
    });
    if (created != null) _adopt(created);
    if (choices case (:final filed)) _choicesWritten(filed);
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
    if (place != null && note.place != place) notes.file(noteId, place);
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
      );
    }
    for (final choice in _schedules) {
      choice.spawnFrom(store, noteId);
    }
    if (_frog && !_frogWritten) Frogs(store).spawn(noteId);
    return (filed: place ?? note.place);
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

  /// Sets up the first field of the Tally the Note becomes.
  Future<void> _makeTally() async {
    final field = await chooseTallyField(context, widget.notes);
    if (!mounted || field == null) return;
    setState(() => _tally = field);
  }

  /// Captures the new Note with [text], [photos] and [recordings].
  void _create(
    String text, {
    List<Uint8List> photos = const [],
    List<Uint8List> recordings = const [],
  }) => _adopt(_captureNote(text, photos: photos, recordings: recordings));

  /// Writes the new Note with [text], [photos] and [recordings], nudged
  /// [nudgeIn] from now, 1 h when null, unless it is sorted.
  Note _captureNote(
    String text, {
    Duration? nudgeIn,
    List<Uint8List> photos = const [],
    List<Uint8List> recordings = const [],
  }) => widget.notes.capture(
    text,
    place: _place,
    // A sorted Note needs no Nudge (#113).
    nudgeIn: _sorted ? null : nudgeIn ?? Notes.defaultNudge,
    photos: photos,
    recordings: recordings,
    sourceApp: widget.shared?.sourceApp,
  );

  /// Holds the written [note] as the one being captured, and attaches
  /// where and amid what it was captured once that is known.
  void _adopt(Note note) {
    _captured = (
      noteId: note.id,
      attached: [
        attachLocation(widget.notes, note.id, _here),
        attachExtras(widget.notes, note.id, _extras),
      ],
    );
    _filed = note.place;
    _photos = note.photos;
    _recordings = note.recordings;
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
      _write(nudgeIn);
    } catch (_) {
      // Nothing of it was written: the screen stays, to change and leave
      // again.
      _leaving = false;
      rethrow;
    }
    Navigator.of(context).pop(_captured);
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final recording = _voiceMode && !_micMissing;
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
        ),
        body: SafeArea(
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Column(
              children: [
                // With the keyboard up the room is small: the photos and
                // recordings scroll with the text, the buttons stay.
                Expanded(
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
                            SliverFillRemaining(
                              hasScrollBody: false,
                              child: TextField(
                                controller: _text,
                                autofocus:
                                    !widget.startVoice &&
                                    !widget.startPhoto &&
                                    !widget.startGallery,
                                maxLines: null,
                                decoration: InputDecoration(
                                  hintText:
                                      _photos.isEmpty && _recordings.isEmpty
                                      ? t.captureHint
                                      : t.capturePhotoHint,
                                ),
                              ),
                            ),
                          ],
                        ),
                ),
                if (!recording) ...[
                  _captureButtons(context),
                  _actions(context),
                ],
                if (_sorted)
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
              ],
            ),
          ),
        ),
      ),
    );
  }

  /// The Note card's action row, above the Nudge buttons or Save; what
  /// it opens comes back here. Behind the App Lock it offers neither
  /// "File into…" nor the Session, which show Areas and Tallies.
  Widget _actions(BuildContext context) {
    final locked = _lock?.locked ?? false;
    return NoteActionRow(
      importance: _importance,
      onFile: locked ? null : _file,
      onSchedule: _mayManage ? _spawn : null,
      spawns: [
        for (final spawn in Spawn.values)
          if (!locked || spawn != Spawn.session) spawn,
      ],
      actions: NoteActions(
        onImportance: (picked) => setState(() => _importance = picked),
        isTally: () => _tally != null,
        onMakeTally: _makeTally,
      ),
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
  ];

  /// The Photo, Voice and Gallery buttons above the Nudge choices, side
  /// by side, each with its icon above its one-word label so three fit
  /// the row; none while recording, where the big stop button comes
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

  /// One big capture button: the icon above the label.
  Widget _captureButton({
    required Key key,
    required VoidCallback onPressed,
    required IconData icon,
    required String label,
  }) => FilledButton.tonal(
    key: key,
    onPressed: onPressed,
    style: _bigButton,
    child: Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        Icon(icon, size: 28),
        const SizedBox(height: 4),
        Text(label, maxLines: 1, softWrap: false),
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
