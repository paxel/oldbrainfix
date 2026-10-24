import 'dart:async';

import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'alarm_sync.dart';
import 'area_look.dart';
import 'areas_screen.dart';
import 'browser.dart';
import 'capture_requests.dart';
import 'capture_screen.dart';
import 'l10n.dart';
import 'look.dart';
import 'merge_into_screen.dart';
import 'missed_summary_card.dart';
import 'note_card.dart';
import 'note_editor_screen.dart';
import 'nudge_screen.dart';
import 'photo_platform.dart';
import 'routine_editor_screen.dart';
import 'routine_history_screen.dart';
import 'schedule_text.dart';
import 'setup_screen.dart';

/// An Agenda row with where its Note is filed, read once per change.
typedef _PlacedRow = ({AgendaRow row, Area? area});

/// The home screen (CONTEXT.md): today's Occurrences, all-day first and
/// then by time, a visible gap, and tomorrow's in dimmer colours. Each
/// is checked off as Done on its own, tomorrow's already today. "+"
/// captures a Note, whose card then shows on top with its enrich options
/// until the browser opens or the app comes back; the magnifier opens
/// the notes browser; the app bar makes a Routine without a Note. A red
/// banner on top shows while alarms lack a permission, and leads to the
/// setup. Below it the Missed Summary shows until "seen", and under it
/// the "x Unsorted" banner while Notes are Unsorted, opening them. At local
/// midnight the Agenda moves on to the new day, and whenever an
/// Occurrence becomes Missed the summary takes it in. Whatever another
/// app shares in opens in Capture over the Agenda, filled in and
/// Unsorted, and so does the capture screen a shortcut, tile, widget or
/// hardware button asked for, just as from "+".
class AgendaScreen extends StatefulWidget {
  /// How much tomorrow is faded.
  static const tomorrowOpacity = 0.55;

  final Notes notes;
  final Places places;

  /// The notes browser's last selections, kept between visits.
  final BrowserTrail trail;

  /// The Device's alarms: taps applied and permissions changed.
  final AlarmSync alarms;

  /// The quick entry points asking for a capture screen.
  final CaptureRequests requests;

  const AgendaScreen({
    super.key,
    required this.notes,
    required this.places,
    required this.trail,
    required this.alarms,
    this.requests = const NoCaptureRequests(),
  });

  @override
  State<AgendaScreen> createState() => _AgendaScreenState();
}

class _AgendaScreenState extends State<AgendaScreen> {
  late final _routines = Routines(widget.notes.store);
  late final _alarms = Alarms(widget.notes.store);
  late final AppLifecycleListener _lifecycle;
  Timer? _timer;
  MissedSummary _missed = const MissedSummary(routines: [], nextChange: null);
  List<_PlacedRow> _today = const [];
  List<_PlacedRow> _tomorrow = const [];
  var _unsorted = 0;

  /// The Note captured last from here, shown on top until it is let go.
  String? _capturedId;
  PlacedNote? _captured;

  /// Where shares come from; set with the first dependencies.
  PhotoPlatform? _photos;
  StreamSubscription<void>? _shares;
  late final StreamSubscription<void> _requests;

  /// Set while a share or a requested capture is open, so two never
  /// stack.
  var _opening = false;

  @override
  void initState() {
    super.initState();
    _load();
    _lifecycle = AppLifecycleListener(
      // Back from the background, the last capture is old news. A
      // dialog or the notification shade only makes the app inactive.
      onShow: () => _capturedId = null,
      // Maybe a new day, maybe something shared or a capture asked
      // for meanwhile.
      onResume: () {
        _reload();
        unawaited(_openWaiting());
      },
    );
    widget.alarms.addListener(_reload);
    // Notes sorted on a page opened from elsewhere, like the Nudge
    // screen, change the Unsorted count.
    widget.notes.store.addListener(_logChanged);
    _requests = widget.requests.arrived.listen((_) => _openWaiting());
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (_photos != null) return;
    final photos = _photos = PhotoScope.of(context);
    _shares = photos.sharedArrived.listen((_) => _openWaiting());
    // Shared or asked for before the Agenda was up, as on a start by a
    // share or a shortcut.
    WidgetsBinding.instance.addPostFrameCallback((_) => _openWaiting());
  }

  @override
  void dispose() {
    unawaited(_shares?.cancel());
    unawaited(_requests.cancel());
    _timer?.cancel();
    widget.alarms.removeListener(_reload);
    widget.notes.store.removeListener(_logChanged);
    _lifecycle.dispose();
    super.dispose();
  }

  /// Reloads just after the next local midnight, or when the next
  /// Occurrence becomes Missed if that comes first, while the Agenda is
  /// open.
  void _armTimer() {
    _timer?.cancel();
    final now = widget.notes.store.clock();
    final wall = deviceWallClock(now.toUtc());
    final midnight = DateTime(wall.year, wall.month, wall.day + 1);
    var wait = midnight.difference(now);
    final missed = _missed.nextChange?.difference(now);
    if (missed != null && missed < wait) wait = missed;
    wait += const Duration(seconds: 1);
    _timer = Timer(wait.isNegative ? Duration.zero : wait, _reload);
  }

  void _load() {
    final agenda = _routines.agenda(deviceWallClock);
    _today = [for (final row in agenda.todayRows) _place(row)];
    _tomorrow = [for (final row in agenda.tomorrowRows) _place(row)];
    _missed = _alarms.missed(deviceWallClock);
    _unsorted = widget.notes.unsorted().length;
    _armTimer();
    final captured = _capturedId == null
        ? null
        : widget.notes.byId(_capturedId ?? '');
    _captured = captured == null || captured.isTrashed || captured.isMerged
        ? null
        : placeNote(widget.places, captured);
  }

  _PlacedRow _place(AgendaRow row) => (
    row: row,
    area: switch (row.note) {
      final note? => placeNote(widget.places, note).area,
      null => null,
    },
  );

  void _reload() => setState(_load);

  void _reloadIfMounted() {
    if (mounted) _reload();
  }

  var _reloadQueued = false;

  /// Reloads once after the log changed, never in the middle of whatever
  /// changed it.
  void _logChanged() {
    if (_reloadQueued) return;
    _reloadQueued = true;
    scheduleMicrotask(() {
      _reloadQueued = false;
      _reloadIfMounted();
    });
  }

  Future<void> _capture() async {
    await captureNote(
      context,
      widget.notes,
      onCaptured: (id) => _capturedId = id,
      onChanged: _reloadIfMounted,
    );
  }

  /// Opens Capture for each share waiting, one after the other, then
  /// the capture screen an entry point asked for, the same as "+" with
  /// voice or the camera started. A shared Note starts Unsorted.
  Future<void> _openWaiting() async {
    final photos = _photos;
    if (photos == null || _opening) return;
    _opening = true;
    try {
      while (mounted) {
        final shared = await photos.takeShared();
        if (!mounted) return;
        if (shared != null) {
          await captureNote(
            context,
            widget.notes,
            shared: shared,
            onCaptured: (id) => _capturedId = id,
            onChanged: _reloadIfMounted,
          );
          continue;
        }
        final mode = await widget.requests.take();
        if (mode == null || !mounted) return;
        await captureNote(
          context,
          widget.notes,
          voice: mode == CaptureMode.voice,
          photo: mode == CaptureMode.photo,
          onCaptured: (id) => _capturedId = id,
          onChanged: _reloadIfMounted,
        );
      }
    } finally {
      _opening = false;
    }
  }

  Future<void> _browse() async {
    _capturedId = null;
    await Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) => AreasScreen(
          notes: widget.notes,
          places: widget.places,
          trail: widget.trail,
        ),
      ),
    );
    if (!mounted) return;
    _reload();
  }

  Future<void> _openUnsorted() async {
    await openUnsorted(context, widget.notes, widget.places, widget.alarms);
    if (!mounted) return;
    _reload();
  }

  Future<void> _setUp() async {
    await Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (context) => SetupScreen(
          alarms: widget.alarms,
          onDone: () => Navigator.of(context).pop(),
        ),
      ),
    );
    if (!mounted) return;
    _reload();
  }

  Future<void> _newRoutine() async {
    await newRoutine(context, widget.notes, widget.places);
    if (!mounted) return;
    _routineEdited();
  }

  /// Back from a Routine's editor, which may have changed the Device's
  /// Re-nag interval: a setting, not the log, so the alarms are armed
  /// again here.
  void _routineEdited() {
    unawaited(widget.alarms.refresh());
    _reload();
  }

  void _toggle(AgendaRow row) {
    if (row.done == null) {
      _routines.markDone(row.routine.id, row.occurrence);
    } else {
      _routines.unmarkDone(row.routine.id, row.occurrence);
    }
    _reload();
  }

  /// Checks off a Missed Occurrence: Done, late.
  void _checkMissed(Routine routine, Occurrence occurrence) {
    _routines.markDone(routine.id, occurrence);
    _reload();
  }

  void _seen() {
    _alarms.markSeen();
    _reload();
  }

  Future<void> _openRow(AgendaRow row) async {
    await openRoutine(context, widget.notes, widget.places, row.routine);
    if (!mounted) return;
    _routineEdited();
  }

  /// A long press on a Routine's row opens a menu where the finger is,
  /// leading to its history.
  Future<void> _rowMenu(AgendaRow row, Offset at) async {
    final t = context.t;
    final overlay =
        Overlay.of(context).context.findRenderObject() as RenderBox?;
    if (overlay == null) return;
    final choice = await showMenu<bool>(
      context: context,
      position: RelativeRect.fromRect(
        overlay.globalToLocal(at) & Size.zero,
        Offset.zero & overlay.size,
      ),
      items: [PopupMenuItem(value: true, child: Text(t.historyTitle))],
    );
    if (!mounted || choice != true) return;
    await openHistory(context, widget.notes, row.routine);
    if (!mounted) return;
    _reload();
  }

  Future<void> _file(Note note) async {
    await fileNote(context, widget.notes, widget.places, note);
    if (!mounted) return;
    _reload();
  }

  Future<void> _merge(Note note) async {
    await mergeNote(context, widget.notes, widget.places, note);
    if (!mounted) return;
    _reload();
  }

  Future<void> _open(Note note) async {
    await openNote(context, widget.notes, widget.places, note);
    if (!mounted) return;
    _reload();
  }

  Future<void> _schedule(Note note, bool reminder) async {
    await spawnFromNote(
      context,
      widget.notes,
      widget.places,
      note,
      reminder: reminder,
    );
    if (!mounted) return;
    _reload();
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final locale = Localizations.localeOf(context).toString();
    final captured = _captured;
    List<Widget> rows(List<_PlacedRow> rows) => [
      for (final (:row, :area) in rows)
        _OccurrenceRow(
          row: row,
          area: area,
          time: occurrenceTime(t, locale, row.occurrence),
          onToggle: () => _toggle(row),
          onTap: () => _openRow(row),
          onMenu: row.routine.isReminder ? null : (at) => _rowMenu(row, at),
        ),
    ];
    return Scaffold(
      appBar: AppBar(
        title: Text(t.appTitle),
        actions: [
          IconButton(
            tooltip: t.newRoutineTooltip,
            onPressed: _newRoutine,
            icon: const Icon(Icons.add_alarm),
          ),
        ],
      ),
      body: ListView(
        // Room under the last row for the floating buttons.
        padding: const EdgeInsets.only(top: 8, bottom: 160),
        children: [
          if (widget.alarms.missing.isNotEmpty) _AlarmsBanner(onTap: _setUp),
          if (!_missed.isEmpty)
            MissedSummaryCard(
              summary: _missed,
              onCheck: _checkMissed,
              onSeen: _seen,
            ),
          if (_unsorted > 0)
            _UnsortedBanner(count: _unsorted, onTap: _openUnsorted),
          if (captured != null) ...[
            _Heading(t.justCaptured),
            NoteCard(
              note: captured.note,
              when: DateFormat.yMd(locale).add_Hm(),
              area: captured.area,
              topic: captured.topic,
              onFile: () => _file(captured.note),
              onMerge: () => _merge(captured.note),
              onSchedule: (reminder) => _schedule(captured.note, reminder),
              onTap: () => _open(captured.note),
            ),
          ],
          _Heading(t.agendaToday),
          if (_today.isEmpty)
            BrowserEmpty(text: t.nothingToday)
          else
            ...rows(_today),
          // The visible gap between today and tomorrow.
          const SizedBox(height: 32),
          Opacity(
            opacity: AgendaScreen.tomorrowOpacity,
            child: Column(
              children: [
                _Heading(t.agendaTomorrow),
                if (_tomorrow.isEmpty)
                  BrowserEmpty(text: t.nothingTomorrow)
                else
                  ...rows(_tomorrow),
              ],
            ),
          ),
        ],
      ),
      floatingActionButton: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          FloatingActionButton.small(
            heroTag: 'browse',
            tooltip: t.browseTooltip,
            onPressed: _browse,
            child: const Icon(Icons.search),
          ),
          const SizedBox(height: 16),
          FloatingActionButton(
            heroTag: 'capture',
            tooltip: t.captureTooltip,
            onPressed: _capture,
            child: const Icon(Icons.add),
          ),
        ],
      ),
    );
  }
}

/// The red banner while alarms lack a permission; tapping it opens the
/// setup.
class _AlarmsBanner extends StatelessWidget {
  final VoidCallback onTap;

  const _AlarmsBanner({required this.onTap});

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 0, 16, 8),
      child: Material(
        color: scheme.error,
        borderRadius: BorderRadius.circular(16),
        child: InkWell(
          borderRadius: BorderRadius.circular(16),
          onTap: onTap,
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Row(
              children: [
                Icon(Icons.alarm_off, color: scheme.onError),
                const SizedBox(width: 12),
                Expanded(
                  child: Text(
                    context.t.alarmsBanner,
                    style: TextStyle(color: scheme.onError),
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

/// "x Unsorted" while Notes wait to be sorted; tapping it opens them.
class _UnsortedBanner extends StatelessWidget {
  final int count;
  final VoidCallback onTap;

  const _UnsortedBanner({required this.count, required this.onTap});

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 0, 16, 8),
      child: Material(
        key: const ValueKey('unsortedBanner'),
        color: scheme.secondaryContainer,
        borderRadius: BorderRadius.circular(16),
        child: InkWell(
          borderRadius: BorderRadius.circular(16),
          onTap: onTap,
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Row(
              children: [
                Icon(Icons.move_to_inbox, color: scheme.onSecondaryContainer),
                const SizedBox(width: 12),
                Expanded(
                  child: Text(
                    context.t.unsortedBanner(count),
                    style: TextStyle(color: scheme.onSecondaryContainer),
                  ),
                ),
                Icon(Icons.chevron_right, color: scheme.onSecondaryContainer),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

/// A section title of the Agenda.
class _Heading extends StatelessWidget {
  final String text;

  const _Heading(this.text);

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.fromLTRB(24, 8, 24, 4),
    child: Align(
      alignment: Alignment.centerLeft,
      child: Text(text, style: Theme.of(context).textTheme.titleMedium),
    ),
  );
}

/// One Occurrence as an embossed row: the box to check it off, its time,
/// its Note's text, and its Area's colour and icon. Tapping the row
/// opens its Routine; a long press opens [onMenu] where the finger is,
/// when there is one.
class _OccurrenceRow extends StatelessWidget {
  final AgendaRow row;
  final Area? area;
  final String time;
  final VoidCallback onToggle;
  final VoidCallback onTap;
  final void Function(Offset at)? onMenu;

  const _OccurrenceRow({
    required this.row,
    required this.area,
    required this.time,
    required this.onToggle,
    required this.onTap,
    this.onMenu,
  });

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final text = Theme.of(context).textTheme;
    final scheme = Theme.of(context).colorScheme;
    final colour = areaColour(area?.colour);
    final done = row.done != null;
    final note = row.note;
    final title = note == null ? t.noteGone : noteText(t, note);
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 6),
      child: Embossed(
        child: ClipRRect(
          borderRadius: BorderRadius.circular(16),
          // The row's ink shows on the card, not under it.
          child: Material(
            type: MaterialType.transparency,
            child: GestureDetector(
              onLongPressStart: switch (onMenu) {
                null => null,
                final onMenu => (details) => onMenu(details.globalPosition),
              },
              child: InkWell(
                onTap: onTap,
                child: Container(
                  decoration: colour == null
                      ? null
                      : BoxDecoration(
                          border: Border(
                            left: BorderSide(color: colour, width: 6),
                          ),
                        ),
                  padding: const EdgeInsets.fromLTRB(4, 4, 16, 4),
                  child: Row(
                    children: [
                      Checkbox(
                        value: done,
                        semanticLabel: t.markDone,
                        onChanged: (_) => onToggle(),
                      ),
                      const SizedBox(width: 4),
                      Expanded(
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.start,
                          children: [
                            Text(time, style: text.labelLarge),
                            Text(
                              title,
                              maxLines: 2,
                              overflow: TextOverflow.ellipsis,
                              style: text.bodyLarge?.copyWith(
                                decoration: done
                                    ? TextDecoration.lineThrough
                                    : null,
                              ),
                            ),
                          ],
                        ),
                      ),
                      if (row.routine.isReminder)
                        Padding(
                          padding: const EdgeInsets.only(left: 8),
                          child: Icon(
                            Icons.notifications_outlined,
                            size: 18,
                            color: scheme.onSurfaceVariant,
                            semanticLabel: t.reminder,
                          ),
                        ),
                      if (areaIconData(area?.icon) case final icon?)
                        Padding(
                          padding: const EdgeInsets.only(left: 8),
                          child: Icon(icon, size: 20, color: colour),
                        ),
                    ],
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
