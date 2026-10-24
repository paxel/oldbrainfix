import 'dart:async';

import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'alarm_sync.dart';
import 'area_look.dart';
import 'areas_screen.dart';
import 'browser.dart';
import 'capture_screen.dart';
import 'l10n.dart';
import 'look.dart';
import 'merge_into_screen.dart';
import 'note_card.dart';
import 'note_editor_screen.dart';
import 'routine_editor_screen.dart';
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
/// setup. At local midnight the Agenda moves on to the new day.
class AgendaScreen extends StatefulWidget {
  /// How much tomorrow is faded.
  static const tomorrowOpacity = 0.55;

  final Notes notes;
  final Places places;

  /// The notes browser's last selections, kept between visits.
  final BrowserTrail trail;

  /// The Device's alarms: taps applied and permissions changed.
  final AlarmSync alarms;

  const AgendaScreen({
    super.key,
    required this.notes,
    required this.places,
    required this.trail,
    required this.alarms,
  });

  @override
  State<AgendaScreen> createState() => _AgendaScreenState();
}

class _AgendaScreenState extends State<AgendaScreen> {
  late final _routines = Routines(widget.notes.store);
  late final AppLifecycleListener _lifecycle;
  Timer? _midnight;
  List<_PlacedRow> _today = const [];
  List<_PlacedRow> _tomorrow = const [];

  /// The Note captured last from here, shown on top until it is let go.
  String? _capturedId;
  PlacedNote? _captured;

  @override
  void initState() {
    super.initState();
    _load();
    _lifecycle = AppLifecycleListener(
      // Back from the background, the last capture is old news. A
      // dialog or the notification shade only makes the app inactive.
      onShow: () => _capturedId = null,
      // Maybe a new day.
      onResume: _reload,
    );
    widget.alarms.addListener(_reload);
  }

  @override
  void dispose() {
    _midnight?.cancel();
    widget.alarms.removeListener(_reload);
    _lifecycle.dispose();
    super.dispose();
  }

  /// Reloads just after the next local midnight, while the Agenda is
  /// open.
  void _armMidnight() {
    _midnight?.cancel();
    final now = widget.notes.store.clock();
    final wall = deviceWallClock(now.toUtc());
    final next = DateTime(wall.year, wall.month, wall.day + 1);
    final wait = next.difference(now) + const Duration(seconds: 1);
    _midnight = Timer(wait.isNegative ? Duration.zero : wait, _reload);
  }

  void _load() {
    _armMidnight();
    final agenda = _routines.agenda(deviceWallClock);
    _today = [for (final row in agenda.todayRows) _place(row)];
    _tomorrow = [for (final row in agenda.tomorrowRows) _place(row)];
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

  Future<void> _capture() async {
    await captureNote(
      context,
      widget.notes,
      onCaptured: (id) => _capturedId = id,
      onChanged: _reloadIfMounted,
    );
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

  Future<void> _openRow(AgendaRow row) async {
    await openRoutine(context, widget.notes, widget.places, row.routine);
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
/// opens its Routine.
class _OccurrenceRow extends StatelessWidget {
  final AgendaRow row;
  final Area? area;
  final String time;
  final VoidCallback onToggle;
  final VoidCallback onTap;

  const _OccurrenceRow({
    required this.row,
    required this.area,
    required this.time,
    required this.onToggle,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final text = Theme.of(context).textTheme;
    final scheme = Theme.of(context).colorScheme;
    final colour = areaColour(area?.colour);
    final done = row.done != null;
    final title = row.note?.text ?? t.noteGone;
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 6),
      child: Embossed(
        child: ClipRRect(
          borderRadius: BorderRadius.circular(16),
          // The row's ink shows on the card, not under it.
          child: Material(
            type: MaterialType.transparency,
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
    );
  }
}
