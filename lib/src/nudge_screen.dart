import 'dart:async';

import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'alarm_sync.dart';
import 'areas_screen.dart';
import 'browser.dart';
import 'l10n.dart';
import 'look.dart';
import 'merge_into_screen.dart';
import 'note_card.dart';
import 'note_editor_screen.dart';
import 'routine_editor_screen.dart';
import 'schedule_text.dart';

/// How many of the oldest Unsorted Notes the Nudge screen shows.
const nudgeCards = 3;

/// What the Nudge's notification opens (UI laws, "Alarms and
/// notifications"): the three oldest Unsorted cards, each opening its
/// Note and offering to sort it right here, and "+x" for the rest, which
/// opens the list of all Unsorted Notes. This Device's Nudge settings
/// fold out below them.
class NudgeScreen extends StatefulWidget {
  final Notes notes;
  final Places places;
  final AlarmSync alarms;

  const NudgeScreen({
    super.key,
    required this.notes,
    required this.places,
    required this.alarms,
  });

  @override
  State<NudgeScreen> createState() => _NudgeScreenState();
}

class _NudgeScreenState extends State<NudgeScreen> {
  late List<Note> _unsorted = widget.notes.unsorted();

  void _reload() {
    if (!mounted) return;
    setState(() => _unsorted = widget.notes.unsorted());
  }

  Future<void> _all() async {
    await openUnsorted(context, widget.notes, widget.places, widget.alarms);
    _reload();
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final more = _unsorted.length - nudgeCards;
    return Scaffold(
      appBar: AppBar(title: Text(t.nudgeScreenTitle)),
      body: ListView(
        padding: const EdgeInsets.symmetric(vertical: 8),
        children: [
          if (_unsorted.isEmpty)
            BrowserEmpty(text: t.noUnsorted)
          else
            ..._sortingCards(
              context,
              widget.notes,
              widget.places,
              _unsorted.take(nudgeCards),
              _reload,
            ),
          if (more > 0)
            Padding(
              padding: const EdgeInsets.fromLTRB(16, 8, 16, 8),
              child: Align(
                alignment: Alignment.centerLeft,
                child: FilledButton.tonal(
                  key: const ValueKey('nudgeMore'),
                  onPressed: _all,
                  child: Tooltip(
                    message: t.nudgeMoreTooltip,
                    child: Text(t.nudgeMore(more)),
                  ),
                ),
              ),
            ),
          NudgeSettingsPanel(store: widget.notes.store, alarms: widget.alarms),
        ],
      ),
    );
  }
}

/// Opens the list of all Unsorted Notes.
Future<void> openUnsorted(
  BuildContext context,
  Notes notes,
  Places places,
  AlarmSync alarms,
) => Navigator.of(context).push(
  MaterialPageRoute<void>(
    builder: (_) =>
        UnsortedScreen(notes: notes, places: places, alarms: alarms),
  ),
);

/// Every Unsorted Note as a card, oldest first, as the Nudge screen
/// starts them, each offering to sort it; reached from the "x Unsorted"
/// banner on the Agenda and from "+x" on the Nudge screen. This Device's
/// Nudge settings fold out below them.
class UnsortedScreen extends StatefulWidget {
  final Notes notes;
  final Places places;
  final AlarmSync alarms;

  const UnsortedScreen({
    super.key,
    required this.notes,
    required this.places,
    required this.alarms,
  });

  @override
  State<UnsortedScreen> createState() => _UnsortedScreenState();
}

class _UnsortedScreenState extends State<UnsortedScreen> {
  late List<Note> _unsorted = widget.notes.unsorted();

  void _reload() {
    if (!mounted) return;
    setState(() => _unsorted = widget.notes.unsorted());
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return Scaffold(
      appBar: AppBar(title: Text(t.unsorted)),
      body: ListView(
        padding: const EdgeInsets.symmetric(vertical: 8),
        children: [
          if (_unsorted.isEmpty)
            BrowserEmpty(text: t.noUnsorted)
          else
            ..._sortingCards(
              context,
              widget.notes,
              widget.places,
              _unsorted,
              _reload,
            ),
          NudgeSettingsPanel(store: widget.notes.store, alarms: widget.alarms),
        ],
      ),
    );
  }
}

/// [unsorted] as cards that open their Note and offer to file, merge or
/// schedule it; [changed] runs once the user is back from any of them.
List<Widget> _sortingCards(
  BuildContext context,
  Notes notes,
  Places places,
  Iterable<Note> unsorted,
  VoidCallback changed,
) {
  Future<void> then(Future<Object?> step) async {
    await step;
    changed();
  }

  return diaryCards(
    context,
    [for (final note in unsorted) placeNote(places, note)],
    (note) => then(fileNote(context, notes, places, note)),
    (note) => then(mergeNote(context, notes, places, note)),
    (note) => then(openNote(context, notes, places, note)),
    onSchedule: (note, reminder) =>
        then(spawnFromNote(context, notes, places, note, reminder: reminder)),
  );
}

/// The folded Nudge settings of this Device (ADR 0008): how long an
/// ignored Nudge waits before it comes again, and the Quiet Hours. Here
/// until the Settings behind the Agenda's gear exist. Changes are armed
/// at once: they are settings, not the log.
class NudgeSettingsPanel extends StatefulWidget {
  /// The repeat intervals offered, in minutes.
  static const repeatChoices = [10, 30, 60];

  final Store store;
  final AlarmSync alarms;

  const NudgeSettingsPanel({
    super.key,
    required this.store,
    required this.alarms,
  });

  @override
  State<NudgeSettingsPanel> createState() => _NudgeSettingsPanelState();
}

class _NudgeSettingsPanelState extends State<NudgeSettingsPanel> {
  late Duration _repeat = widget.store.nudgeRepeat;
  late QuietHours _quiet = widget.store.quietHours;
  late bool _quietOn = widget.store.quietHoursOn;

  void _changed() {
    setState(() {
      _repeat = widget.store.nudgeRepeat;
      _quiet = widget.store.quietHours;
      _quietOn = widget.store.quietHoursOn;
    });
    unawaited(widget.alarms.refresh());
  }

  Future<void> _pick({required bool start}) async {
    final shown = start ? _quiet.start : _quiet.end;
    final picked = await showTimePicker(
      context: context,
      initialTime: TimeOfDay(hour: shown.hour, minute: shown.minute),
    );
    if (picked == null || !mounted) return;
    final time = ClockTime(picked.hour, picked.minute);
    widget.store.quietHours = start
        ? QuietHours(time, _quiet.end)
        : QuietHours(_quiet.start, time);
    _changed();
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final locale = Localizations.localeOf(context).toString();
    String time(ClockTime c) => clockText(locale, c);
    final repeat = _repeat.inMinutes;
    final choices = {...NudgeSettingsPanel.repeatChoices, repeat}.toList()
      ..sort();
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 16, 16, 8),
      child: Embossed(
        child: ExpansionTile(
          key: const ValueKey('nudgeSettings'),
          shape: const Border(),
          collapsedShape: const Border(),
          title: Text(t.nudgeSettingsHeading),
          subtitle: Text(
            t.nudgeSettingsSummary(
              repeat,
              _quietOn
                  ? t.quietHoursRange(time(_quiet.start), time(_quiet.end))
                  : t.quietHoursOff,
            ),
          ),
          childrenPadding: const EdgeInsets.fromLTRB(16, 0, 16, 16),
          expandedCrossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(t.nudgeSettingsExplain),
            const SizedBox(height: 8),
            Row(
              children: [
                Expanded(child: Text(t.nudgeRepeat)),
                DropdownButton<int>(
                  key: const ValueKey('nudgeRepeat'),
                  value: repeat,
                  onChanged: (minutes) {
                    if (minutes == null) return;
                    widget.store.nudgeRepeat = Duration(minutes: minutes);
                    _changed();
                  },
                  items: [
                    for (final m in choices)
                      DropdownMenuItem(
                        value: m,
                        child: Text(t.renagMinutes(m)),
                      ),
                  ],
                ),
              ],
            ),
            SwitchListTile(
              key: const ValueKey('quietHours'),
              contentPadding: EdgeInsets.zero,
              title: Text(t.quietHours),
              value: _quietOn,
              onChanged: (on) {
                widget.store.quietHoursOn = on;
                _changed();
              },
            ),
            if (_quietOn) ...[
              ListTile(
                key: const ValueKey('quietFrom'),
                contentPadding: EdgeInsets.zero,
                title: Text(t.quietFrom),
                trailing: Text(time(_quiet.start)),
                onTap: () => _pick(start: true),
              ),
              ListTile(
                key: const ValueKey('quietUntil'),
                contentPadding: EdgeInsets.zero,
                title: Text(t.quietUntil),
                trailing: Text(time(_quiet.end)),
                onTap: () => _pick(start: false),
              ),
            ],
          ],
        ),
      ),
    );
  }
}
