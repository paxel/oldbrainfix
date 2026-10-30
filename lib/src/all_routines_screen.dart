import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'app_lock.dart';
import 'area_look.dart';
import 'browser.dart';
import 'calendar_platform.dart';
import 'l10n.dart';
import 'look.dart';
import 'note_card.dart';
import 'note_editor_screen.dart';
import 'routine_editor_screen.dart';
import 'schedule_text.dart';
import 'system_bars.dart';

/// Opens the list of all Routines and Reminders; returns once the user
/// left it.
Future<void> openAllRoutines(
  BuildContext context,
  Notes notes,
  Places places,
) => Navigator.of(context).push(
  MaterialPageRoute<void>(
    builder: (_) => AllRoutinesScreen(notes: notes, places: places),
  ),
);

/// Every Routine and Reminder in alphabetical order of its name, also
/// those beyond the Agenda's week; a tap opens its editor. Export hands
/// one calendar file of all of them to the app the user chooses. Those
/// of closed locked Areas stay out, of the file too. Each card carries
/// its Area's colour and icon, as on the Agenda. Those whose schedule a
/// newer version of the app saved follow, saying so, a tap opening their
/// Note (ADR 0034); they are in no file.
class AllRoutinesScreen extends StatefulWidget {
  final Notes notes;
  final Places places;

  const AllRoutinesScreen({
    super.key,
    required this.notes,
    required this.places,
  });

  @override
  State<AllRoutinesScreen> createState() => _AllRoutinesScreenState();
}

class _AllRoutinesScreenState extends State<AllRoutinesScreen> {
  late final _routines = Routines(widget.notes.store);

  /// Each with the name it shows and its Note's Area, read once per
  /// change.
  List<(Routine, String, Area?)>? _rows;

  /// Those whose schedule a newer version of the app saved, with the
  /// name and Area of their Note, which a tap opens.
  List<(NewerRoutine, Note, String, Area?)> _newer = const [];

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    // The names need the localisations, which initState cannot read.
    if (_rows == null) _load();
  }

  void _load() {
    final t = context.t;
    final lock = LockScope.of(context);
    final shown = {
      for (final r in _routines.all())
        if (!(lock?.hidesRoutine(r) ?? false))
          r: switch (_routines.note(r)) {
            null => (t.noteGone, null),
            final note => (
              noteText(t, note),
              placeNote(widget.places, note).area,
            ),
          },
    };
    _rows = [
      for (final r in routinesByName(shown.keys, (r) => shown[r]?.$1 ?? ''))
        (r, shown[r]?.$1 ?? '', shown[r]?.$2),
    ];
    _newer = [
      for (final r in _routines.newer())
        if (_routines.noteOfNewer(r) case final note?
            when !(lock?.hides(note) ?? false))
          (r, note, noteText(t, note), placeNote(widget.places, note).area),
    ];
  }

  Future<void> _openNote(Note note) async {
    await openNote(context, widget.notes, widget.places, note);
    if (!mounted) return;
    setState(_load);
  }

  Future<void> _open(Routine routine) async {
    await openRoutine(context, widget.notes, widget.places, routine);
    if (!mounted) return;
    setState(_load);
  }

  Future<void> _export(List<(Routine, String, Area?)> rows) =>
      shareRoutinesFile(context, _routines, [
        for (final (routine, _, _) in rows) routine,
      ]);

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final locale = Localizations.localeOf(context).toString();
    final rows = _rows ?? const <(Routine, String, Area?)>[];
    return Scaffold(
      appBar: AppBar(
        title: Text(t.allRoutinesTitle),
        actions: [
          if (CalendarScope.of(context).available)
            IconButton(
              key: const ValueKey('export'),
              tooltip: t.exportCalendarFile,
              onPressed: rows.isEmpty ? null : () => _export(rows),
              icon: const Icon(Icons.file_upload_outlined),
            ),
        ],
      ),
      body: ListView(
        padding: clearOfSystemBars(context, const EdgeInsets.only(bottom: 16)),
        children: [
          if (rows.isEmpty && _newer.isEmpty)
            BrowserEmpty(text: t.allRoutinesEmpty)
          else ...[
            for (final (routine, name, area) in rows)
              _RoutineCard(
                key: ValueKey('routine-${routine.id}'),
                icon: routine.isReminder
                    ? Icons.notifications_outlined
                    : Icons.repeat,
                name: name,
                area: area,
                schedule: scheduleText(t, locale, routine.schedule),
                onTap: () => _open(routine),
              ),
            for (final (routine, note, name, area) in _newer)
              _RoutineCard(
                key: ValueKey('routine-${routine.id}'),
                icon: Icons.update,
                name: name,
                area: area,
                schedule: t.newerVersionNeeded,
                onTap: () => _openNote(note),
              ),
          ],
        ],
      ),
    );
  }
}

/// One Routine or Reminder as an embossed card: its kind, its name, its
/// schedule, and its Area's colour down the side and its icon, as the
/// Agenda's rows show them.
class _RoutineCard extends StatelessWidget {
  /// Its kind: a Routine's, a Reminder's, or one a newer version saved.
  final IconData icon;
  final String name;
  final Area? area;
  final String schedule;
  final VoidCallback onTap;

  const _RoutineCard({
    super.key,
    required this.icon,
    required this.name,
    required this.area,
    required this.schedule,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    final colour = areaColour(area?.colour);
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 6),
      child: Embossed(
        child: ClipRRect(
          borderRadius: BorderRadius.circular(16),
          // The row's ink shows on the card, not under it.
          child: Material(
            type: MaterialType.transparency,
            child: Container(
              decoration: colour == null
                  ? null
                  : BoxDecoration(
                      border: Border(left: BorderSide(color: colour, width: 6)),
                    ),
              child: ListTile(
                leading: Icon(icon),
                title: Text(name, maxLines: 2, overflow: TextOverflow.ellipsis),
                subtitle: Text(schedule),
                trailing: switch (areaIconData(area?.icon)) {
                  final icon? => Icon(icon, size: 20, color: colour),
                  null => null,
                },
                onTap: onTap,
              ),
            ),
          ),
        ),
      ),
    );
  }
}
