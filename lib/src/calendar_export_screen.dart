import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'app_lock.dart';
import 'browser.dart';
import 'calendar_platform.dart';
import 'l10n.dart';
import 'schedule_text.dart';
import 'system_bars.dart';

/// Opens the calendar export; returns once the user left it.
Future<void> openCalendarExport(BuildContext context, Notes notes) =>
    Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) => CalendarExportScreen(notes: notes),
      ),
    );

/// Every Routine and Reminder to pick for one calendar file, all picked
/// at first; "All" picks or drops them together. Sharing hands the file
/// with their RRULEs and alarms to the app the user chooses.
class CalendarExportScreen extends StatefulWidget {
  final Notes notes;

  const CalendarExportScreen({super.key, required this.notes});

  @override
  State<CalendarExportScreen> createState() => _CalendarExportScreenState();
}

class _CalendarExportScreenState extends State<CalendarExportScreen> {
  late final _routines = Routines(widget.notes.store);
  late final List<(Routine, Note?)> _rows;
  late final Set<String> _picked;

  @override
  void initState() {
    super.initState();
    // Those of closed locked Areas stay out, of the file too.
    final lock = LockScope.of(context);
    _rows =
        [
          for (final r in _routines.all())
            if (!(lock?.hidesRoutine(r) ?? false)) (r, _routines.note(r)),
        ]..sort((a, b) {
          final byText = (a.$2?.text ?? '').compareTo(b.$2?.text ?? '');
          return byText != 0 ? byText : a.$1.id.compareTo(b.$1.id);
        });
    _picked = {for (final (routine, _) in _rows) routine.id};
  }

  void _pick(String id, bool picked) => setState(() {
    if (picked) {
      _picked.add(id);
    } else {
      _picked.remove(id);
    }
  });

  void _pickAll(bool picked) => setState(() {
    _picked.clear();
    if (picked) _picked.addAll([for (final (routine, _) in _rows) routine.id]);
  });

  Future<void> _share() => shareCalendarFile(context, widget.notes.store, [
    for (final (routine, _) in _rows)
      if (_picked.contains(routine.id))
        ...eventsOf(context, _routines, routine),
  ]);

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final locale = Localizations.localeOf(context).toString();
    return Scaffold(
      appBar: AppBar(
        title: Text(t.calendarExportTitle),
        actions: [
          IconButton(
            tooltip: t.shareCalendarFile,
            onPressed: _picked.isEmpty ? null : _share,
            icon: const Icon(Icons.share),
          ),
        ],
      ),
      body: ListView(
        padding: clearOfSystemBars(context, const EdgeInsets.only(bottom: 16)),
        children: [
          if (_rows.isEmpty)
            BrowserEmpty(text: t.calendarExportEmpty)
          else ...[
            CheckboxListTile(
              value: _picked.length == _rows.length
                  ? true
                  : _picked.isEmpty
                  ? false
                  : null,
              tristate: true,
              title: Text(t.calendarExportAll),
              onChanged: (_) => _pickAll(_picked.length != _rows.length),
            ),
            const Divider(),
            for (final (routine, note) in _rows)
              CheckboxListTile(
                value: _picked.contains(routine.id),
                title: Text(
                  note == null ? t.noteGone : noteText(t, note),
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                ),
                subtitle: Text(scheduleText(t, locale, routine.schedule)),
                secondary: Icon(
                  routine.isReminder
                      ? Icons.notifications_outlined
                      : Icons.repeat,
                ),
                onChanged: (picked) => _pick(routine.id, picked ?? false),
              ),
          ],
        ],
      ),
    );
  }
}
