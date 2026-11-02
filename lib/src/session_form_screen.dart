import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'capture_choices.dart';
import 'l10n.dart';
import 'look.dart';
import 'note_editor_screen.dart';
import 'page_bar.dart';
import 'routine_editor_screen.dart';
import 'schedule_text.dart';
import 'session_editor.dart';
import 'setting_badges.dart';
import 'system_bars.dart';
import 'tally_form_screen.dart';

/// A new Session made from the browser's "+" (#161), on one page with
/// Save in the top bar: its name; its pool, a checkbox per Tally it may
/// pick, with "New Tally" at the end of the list, which opens the New
/// Tally page and comes back with the Tally made there ticked; then its
/// settings as badges, how many it picks each time (− and +), how it
/// picks and its schedule, every day all day at first, each opening its
/// small editor, the schedule the Routine editor's. Save files its Note
/// into [place], Unsorted for none, and spawns the Session; it pops the
/// Session's id. Leaving without Save makes nothing but the Tallies made
/// on the way.
///
/// For the kind badge of a plain Note ([noteId], #191) it sets that Note
/// up instead: no name and no schedule, the Note's text being its name
/// and its Routines its schedule; Save makes it a Session and pops its
/// id.
class SessionFormScreen extends StatefulWidget {
  final Notes notes;
  final Places places;

  /// The Area or Topic it goes into; null for none.
  final String? place;

  /// The plain Note made a Session; null for a new one.
  final String? noteId;

  const SessionFormScreen({
    super.key,
    required this.notes,
    required this.places,
    this.place,
    this.noteId,
  });

  @override
  State<SessionFormScreen> createState() => _SessionFormScreenState();
}

class _SessionFormScreenState extends State<SessionFormScreen> {
  /// The Tallies of where it goes; while Unsorted, any of the user's.
  late final _draft = SessionDraft.of(
    widget.notes.store,
    widget.noteId ?? widget.place ?? '',
  );

  /// Where a Tally made here goes: where the Session goes.
  late final String? _place = switch (widget.noteId) {
    final id? => widget.notes.byId(id)?.place,
    null => widget.place,
  };
  final _name = TextEditingController();
  final _nameFocus = FocusNode();

  /// Every day, all day (#163), with this Device's Re-nag and no
  /// Escalation.
  late var _schedule = ScheduleChoice(
    Schedule(
      start: Day.today(widget.notes.store.clock(), deviceWallClock),
      rule: const SimpleRepeat.every(1, RepeatUnit.days).toRule(),
    ),
  );
  var _nameMissing = false;

  @override
  void dispose() {
    _name.dispose();
    _nameFocus.dispose();
    _draft.dispose();
    super.dispose();
  }

  /// "New Tally": its page, filed where the Session goes; the Tally made
  /// there comes first in the pool, ticked.
  Future<void> _newTally() async {
    final made = await Navigator.of(context).push<String>(
      MaterialPageRoute(
        builder: (_) => TallyFormScreen(notes: widget.notes, place: _place),
      ),
    );
    if (!mounted || made == null) return;
    setState(() {
      _draft.order.insert(0, made);
      _draft.chosen.add(made);
      _draft.notes[made] = widget.notes.byId(made);
    });
  }

  Future<void> _editCount() => showDialog<void>(
    context: context,
    builder: (context) => BadgeDialog(
      setting: context.t.sessionCount,
      icon: Icons.tag,
      content: StatefulBuilder(
        builder: (context, setInner) => CountStepper(
          key: const ValueKey('session-count'),
          value: _draft.count,
          max: SessionKeys.maxCount,
          onChanged: (n) {
            setState(() => _draft.count = n);
            setInner(() {});
          },
        ),
      ),
    ),
  );

  Future<void> _editRule() async {
    final picked = await pickSessionRule(context, _draft.rule);
    if (!mounted || picked == null) return;
    setState(() => _draft.rule = picked);
  }

  /// The schedule's editor: the Routine editor's, starting from the
  /// schedule held, for the Note not written yet.
  Future<void> _editSchedule() async {
    final picked = await Navigator.of(context).push<ScheduleChoice>(
      MaterialPageRoute(
        builder: (_) => RoutineEditorScreen(
          notes: widget.notes,
          places: widget.places,
          capture: (text: _name.text, place: widget.place),
          initial: _schedule,
          title: context.t.sessionSchedule,
        ),
      ),
    );
    if (!mounted || picked == null) return;
    setState(
      () => _schedule = ScheduleChoice(
        picked.schedule,
        renag: picked.renag,
        escalation: picked.escalation,
      ),
    );
  }

  void _save() {
    if (widget.noteId case final noteId?) {
      setState(() => _draft.checked = true);
      if (!_draft.valid) return;
      Sessions(
        widget.notes.store,
      ).make(noteId, pool: _draft.pool, count: _draft.count, rule: _draft.rule);
      Navigator.of(context).pop(noteId);
      return;
    }
    final name = _name.text.trim();
    setState(() {
      _nameMissing = name.isEmpty;
      _draft.checked = true;
    });
    if (_nameMissing) {
      _nameFocus.requestFocus();
      return;
    }
    if (!_draft.valid) return;
    final store = widget.notes.store;
    final schedule = _schedule;
    final id = store.batch(() {
      final note = widget.notes.capture(name, place: widget.place);
      return ScheduleChoice(
        schedule.schedule,
        renag: schedule.renag,
        escalation: schedule.escalation,
        session: _draft.choice,
      ).spawnFrom(store, note.id);
    });
    Navigator.of(context).pop(id);
  }

  Widget _pool(AppLocalizations t) {
    final error = TextStyle(color: Theme.of(context).colorScheme.error);
    return EditorPanel(
      key: const ValueKey('session-pool'),
      heading: t.sessionPool,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(t.sessionPoolExplain),
          Material(
            type: MaterialType.transparency,
            child: Column(
              children: [
                for (final id in _draft.order)
                  CheckboxListTile(
                    key: ValueKey('session-pool-$id'),
                    contentPadding: EdgeInsets.zero,
                    controlAffinity: ListTileControlAffinity.leading,
                    value: _draft.chosen.contains(id),
                    title: Text(switch (_draft.notes[id]) {
                      final note? => noteText(t, note),
                      null => t.noteGone,
                    }),
                    onChanged: (on) => setState(
                      () => on ?? false
                          ? _draft.chosen.add(id)
                          : _draft.chosen.remove(id),
                    ),
                  ),
                // Making one stays inside the list, on this page.
                ListTile(
                  key: const ValueKey('session-new-tally'),
                  contentPadding: EdgeInsets.zero,
                  leading: const Icon(Icons.add),
                  title: Text(t.newTally),
                  onTap: _newTally,
                ),
              ],
            ),
          ),
          if (_draft.checked && _draft.pool.isEmpty)
            Text(
              t.sessionPoolMissing,
              key: const ValueKey('session-pool-error'),
              style: error,
            ),
        ],
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final locale = Localizations.localeOf(context).toString();
    final rule = sessionRuleName(t, _draft.rule);
    return Scaffold(
      appBar: pageBar(
        context,
        title: widget.noteId == null ? t.newSession : t.session,
        actions: [
          TextButton(
            key: const ValueKey('session-form-save'),
            onPressed: _save,
            child: Text(t.save),
          ),
        ],
      ),
      body: ListView(
        padding: clearOfSystemBars(context, const EdgeInsets.all(16)),
        children: [
          if (widget.noteId == null)
            EditorPanel(
              child: TextField(
                key: const ValueKey('session-form-name'),
                controller: _name,
                focusNode: _nameFocus,
                autofocus: true,
                textCapitalization: TextCapitalization.sentences,
                decoration: InputDecoration(
                  labelText: t.kindName,
                  errorText: _nameMissing ? t.kindNameMissing : null,
                ),
              ),
            ),
          _pool(t),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              SettingBadge(
                key: const ValueKey('session-form-count'),
                setting: t.sessionCount,
                icon: Icons.tag,
                value: t.sessionPicks(_draft.count),
                onTap: (_) => _editCount(),
              ),
              SettingBadge(
                key: const ValueKey('session-form-rule'),
                setting: t.sessionRule,
                icon: Icons.shuffle,
                value: rule,
                onTap: (_) => _editRule(),
              ),
              if (widget.noteId == null)
                SettingBadge(
                  key: const ValueKey('session-form-schedule'),
                  setting: t.sessionSchedule,
                  icon: Icons.event_repeat,
                  value: scheduleText(t, locale, _schedule.schedule),
                  onTap: (_) => _editSchedule(),
                ),
            ],
          ),
        ],
      ),
    );
  }
}

/// The rule [rule] in words, the value of its badge.
String sessionRuleName(AppLocalizations t, SessionRule rule) => switch (rule) {
  SessionRule.random => t.sessionRuleRandom,
  SessionRule.rolling => t.sessionRuleRolling,
  SessionRule.least => t.sessionRuleLeast,
};

/// The small editor of a Session's rule badge: the rules as radio
/// buttons, [current] marked; a tap on one picks it and closes. Null when
/// left without a pick.
Future<SessionRule?> pickSessionRule(
  BuildContext context,
  SessionRule current,
) {
  final t = context.t;
  return showDialog<SessionRule>(
    context: context,
    builder: (context) => BadgeDialog(
      setting: t.sessionRule,
      icon: Icons.shuffle,
      content: RadioColumn<SessionRule>(
        key: const ValueKey('session-rule'),
        value: current,
        onChanged: (rule) => Navigator.of(context).pop(rule),
        options: [
          for (final rule in SessionRule.values)
            RadioOption(
              key: ValueKey('session-rule-${rule.name}'),
              value: rule,
              title: Text(sessionRuleName(t, rule)),
              subtitle: Text(switch (rule) {
                SessionRule.random => t.sessionRuleRandomDetail,
                SessionRule.rolling => t.sessionRuleRollingDetail,
                SessionRule.least => t.sessionRuleLeastDetail,
              }),
            ),
        ],
      ),
    ),
  );
}
