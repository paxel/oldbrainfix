import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'app_lock.dart';
import 'browser.dart';
import 'calendar_platform.dart';
import 'l10n.dart';
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
/// of closed locked Areas stay out, of the file too.
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

  /// Each with the name it shows, read once per change.
  List<(Routine, String)>? _rows;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    // The names need the localisations, which initState cannot read.
    if (_rows == null) _load();
  }

  void _load() {
    final t = context.t;
    final lock = LockScope.of(context);
    final names = {
      for (final r in _routines.all())
        if (!(lock?.hidesRoutine(r) ?? false))
          r: switch (_routines.note(r)) {
            null => t.noteGone,
            final note => noteText(t, note),
          },
    };
    _rows = [
      for (final r in routinesByName(names.keys, (r) => names[r] ?? ''))
        (r, names[r] ?? ''),
    ];
  }

  Future<void> _open(Routine routine) async {
    await openRoutine(context, widget.notes, widget.places, routine);
    if (!mounted) return;
    setState(_load);
  }

  Future<void> _export(List<(Routine, String)> rows) => shareRoutinesFile(
    context,
    _routines,
    [for (final (routine, _) in rows) routine],
  );

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final locale = Localizations.localeOf(context).toString();
    final rows = _rows ?? const <(Routine, String)>[];
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
          if (rows.isEmpty)
            BrowserEmpty(text: t.allRoutinesEmpty)
          else
            for (final (routine, name) in rows)
              ListTile(
                key: ValueKey('routine-${routine.id}'),
                leading: Icon(
                  routine.isReminder
                      ? Icons.notifications_outlined
                      : Icons.repeat,
                ),
                title: Text(name, maxLines: 2, overflow: TextOverflow.ellipsis),
                subtitle: Text(scheduleText(t, locale, routine.schedule)),
                onTap: () => _open(routine),
              ),
        ],
      ),
    );
  }
}
