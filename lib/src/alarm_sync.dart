import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'agenda_widget.dart';
import 'alarm_platform.dart';
import 'app_lock.dart';
import 'l10n.dart';
import 'profile.dart';
import 'schedule_text.dart';

/// Keeps the platform's alarms in step with the core's alarm plan: on
/// start, on every change to the log, and whenever the app comes back;
/// the home-screen Agenda widget's snapshot goes along with them.
/// Before each full sync it applies what was tapped on notifications
/// meanwhile, so a Done reaches the log even when it was tapped while
/// the app was not running. Tells its listeners when taps were applied,
/// the granted permissions changed, Escalation was turned on or off
/// anywhere, a full-screen alarm started or stopped, or the Nudge's or
/// the Changes Nudge's notification or the Agenda widget's Unsorted
/// count was tapped. The Changes Nudge (#109) is armed anew when a
/// Profile changes, and when [replan] says changes were sent.
class AlarmSync extends ChangeNotifier {
  /// The permissions whose absence the red banner always shows; the
  /// full-screen one counts as well while any Routine escalates.
  static const needed = [
    AlarmPermission.notifications,
    AlarmPermission.exactAlarms,
  ];

  final Store store;
  final AlarmPlatform platform;
  final WallClock wallClock;

  AppLocalizations? _t;
  String _locale = '';
  Map<AlarmPermission, bool> _granted = const {};
  StreamSubscription<void>? _arrivals;
  Future<void> _tail = Future.value();
  var _changeQueued = false;
  var _disposed = false;

  /// What was handed to the platform last, to skip handing it again.
  String? _lastArmed;

  /// The Agenda widget's snapshot handed over last, likewise.
  String? _lastShown;

  /// The Agenda's read handed over last ([agendaRead]), with the log's
  /// mark it was read at; null once the log changed since.
  ({AgendaRead read, int seq})? _agendaRead;

  /// Set while the snapshot waits for the Agenda's next read, or for
  /// [showDelay] without one; whether it is handed over even unchanged.
  bool? _showWaiting;
  Timer? _showTimer;

  /// How long the snapshot waits for the Agenda's read before it reads
  /// the log itself, as when the Agenda is not up.
  static const showDelay = Duration(milliseconds: 500);

  /// Whether any Routine escalates, as of the last arming.
  var _escalating = false;
  Ringing? _ringing;
  var _nudgeOpened = false;
  var _changesOpened = false;
  var _unsortedOpened = false;
  WidgetPick? _pickOpened;

  AlarmSync({
    required this.store,
    required this.platform,
    required this.wallClock,
  }) {
    store.addListener(_logChanged);
    store.addProfileListener(_changed);
    _arrivals = platform.actionsArrived.listen((_) => refresh());
  }

  /// The texts the notifications show; the first ones start the sync.
  void setTexts(AppLocalizations t, String locale) {
    final first = _t == null;
    final changed = t != _t || locale != _locale;
    _t = t;
    _locale = locale;
    if (first) {
      unawaited(refresh());
    } else if (changed) {
      _changed();
    }
  }

  /// Whether [permission] is granted; unknown counts as granted until
  /// the platform has answered.
  bool granted(AlarmPermission permission) => _granted[permission] ?? true;

  /// The [needed] permissions missing, for the red banner, and the
  /// full-screen one while any Routine escalates; none where there is no
  /// alarm layer.
  List<AlarmPermission> get missing => [
    if (platform.available)
      for (final p in [...needed, if (_escalating) AlarmPermission.fullScreen])
        if (!granted(p)) p,
  ];

  /// The Occurrence ringing as a full-screen alarm, as of the last
  /// [refresh]; null when none rings.
  Ringing? get ringing => _ringing;

  /// Whether the Nudge's notification was tapped and the Nudge screen
  /// is still to show; taking it clears it.
  bool takeNudgeOpened() {
    final opened = _nudgeOpened;
    _nudgeOpened = false;
    return opened;
  }

  /// Whether the Changes Nudge's notification was tapped and its
  /// bundles are still to send; taking it clears it.
  bool takeChangesOpened() {
    final opened = _changesOpened;
    _changesOpened = false;
    return opened;
  }

  /// The people the Changes Nudge covers now, in its order; empty when
  /// nobody waits.
  List<String> get changesWaiting =>
      Alarms(store).changesNudge(wallClock)?.people ?? const [];

  /// Arms the plan anew after something the log does not show changed,
  /// such as changes sent to a person by bundle.
  void replan() => _changed();

  /// Whether the Agenda widget's Unsorted count was tapped and the
  /// Unsorted list is still to show; taking it clears it.
  bool takeUnsortedOpened() {
    final opened = _unsortedOpened;
    _unsortedOpened = false;
    return opened;
  }

  /// The Session's pick tapped on the Agenda widget whose entry form is
  /// still to open, or null; taking it clears it (#209).
  WidgetPick? takePickOpened() {
    final opened = _pickOpened;
    _pickOpened = null;
    return opened;
  }

  /// Done, or a snooze for [snooze], from the full-screen alarm: the
  /// platform stops ringing and queues it, and it is applied at once.
  Future<void> answer(Ringing ring, {Duration? snooze}) async {
    await platform.answerRing(ring, snooze: snooze);
    await refresh();
  }

  /// Applies the taps waiting on the platform, reads the permissions and
  /// arms the plan anew.
  Future<void> refresh() => _serial(() async {
    if (_disposed) return;
    final actions = await platform.pendingActions();
    final alarms = Alarms(store);
    for (final action in actions) {
      switch (action) {
        case DoneTapped(:final routineId, :final occurrenceKey, :final at):
          alarms.applyDone(routineId, occurrenceKey, at);
        case SnoozeTapped(:final routineId, :final occurrenceKey, :final until):
          alarms.applySnooze(routineId, occurrenceKey, until);
        case NudgeSnoozed(:final at, :final until):
          alarms.applyNudgeSnooze(at, until);
        case LineTapped(:final routineId, :final occurrenceKey, :final lineId):
          _applyLine(routineId, occurrenceKey, lineId);
        case FrogTapped(:final frogId, :final at):
          _applyFrog(frogId, at);
        case ShoppingTapped(:final lineId):
          _applyShopping(lineId);
        case ChecklistTapped(:final lineId):
          _applyChecklist(lineId);
      }
    }
    final granted = await platform.permissions();
    final ringing = await platform.ringing();
    final nudgeOpened = await platform.nudgeOpened();
    final changesOpened = await platform.changesOpened();
    final unsortedOpened = await platform.unsortedOpened();
    final pickOpened = await platform.pickOpened();
    if (_disposed) return;
    if (nudgeOpened) _nudgeOpened = true;
    if (changesOpened) _changesOpened = true;
    if (unsortedOpened) _unsortedOpened = true;
    if (pickOpened != null) _pickOpened = pickOpened;
    final permissionsChanged = !mapEquals(granted, _granted);
    final ringingChanged = ringing != _ringing;
    _granted = granted;
    _ringing = ringing;
    await _arm(force: true);
    // Acknowledged only once applied and armed: a crash in between
    // applies them again, which changes nothing.
    if (actions.isNotEmpty) {
      await platform.acknowledge(
        actions.map((a) => a.seq).reduce((a, b) => a > b ? a : b),
      );
    }
    if (!_disposed &&
        (actions.isNotEmpty ||
            permissionsChanged ||
            ringingChanged ||
            nudgeOpened ||
            changesOpened ||
            unsortedOpened ||
            pickOpened != null)) {
      notifyListeners();
    }
  });

  /// Checks the Line [lineId] for the Occurrence [occurrenceKey] of the
  /// Routine [routineId], as the Agenda's row does; one gone or no
  /// longer allowed changes nothing.
  void _applyLine(String routineId, String occurrenceKey, String lineId) {
    final occurrence = Occurrence.tryParse(occurrenceKey);
    if (occurrence == null) return;
    try {
      Checklists(store)
          .check(lineId, on: (routineId: routineId, occurrence: occurrence));
    } on ArgumentError {
      return;
    } on StateError {
      return;
    }
  }

  /// Checks off the Frog [frogId] at [at], as the Agenda's row does.
  void _applyFrog(String frogId, DateTime at) {
    final frogs = Frogs(store);
    final frog = frogs.byId(frogId);
    if (frog == null || frog.isDone || frog.isTrashed) return;
    if (!store.mayCheckOff(frogId)) return;
    try {
      frogs.markDone(frogId, at: at);
    } on ArgumentError {
      return;
    } on StateError {
      return;
    }
  }

  /// Checks the Shopping List's Line [lineId], as the Agenda's row does.
  void _applyShopping(String lineId) {
    try {
      ShoppingLists(store).check(lineId, wallClock);
    } on ArgumentError {
      return;
    } on StateError {
      return;
    }
  }

  /// Checks the Line [lineId] of a Checklist without a Routine, as the
  /// Agenda's box does (#209); one gone or no longer allowed changes
  /// nothing.
  void _applyChecklist(String lineId) {
    try {
      Checklists(store).check(lineId);
    } on ArgumentError {
      return;
    } on StateError {
      return;
    }
  }

  /// Asks for [permission]; the answer arrives with the next [refresh]
  /// when the app comes back.
  Future<void> request(AlarmPermission permission) async {
    await platform.request(permission);
    await refresh();
  }

  /// The Agenda read [read] as it loaded (#204): the widget's snapshot
  /// takes it while the log has not changed since, rather than reading
  /// the log a second time.
  void agendaRead(AgendaRead read) {
    if (_disposed) return;
    _agendaRead = (read: read, seq: store.currentSeq());
    final force = _showWaiting;
    if (force == null) return;
    _showWaiting = null;
    _showTimer?.cancel();
    unawaited(_serial(() => _showNow(force: force)));
  }

  /// The log changed: the Agenda's read is not current any more.
  void _logChanged() {
    _agendaRead = null;
    _changed();
  }

  void _changed() {
    if (_changeQueued) return;
    _changeQueued = true;
    scheduleMicrotask(() {
      _changeQueued = false;
      if (!_disposed) unawaited(_serial(_arm));
    });
  }

  Future<void> _arm({bool force = false}) async {
    final t = _t;
    if (t == null || _disposed) return;
    final escalating = Routines(store)
        .all()
        .any((routine) => routine.escalation != null);
    if (escalating != _escalating) {
      _escalating = escalating;
      notifyListeners();
    }
    final routines = Routines(store);
    final locks = AreaLocks(store);
    String title(PlannedAlarm alarm) {
      final routine = routines.byId(alarm.routineId);
      // From a locked Area set to Masked, on the lock screen too.
      if (routine != null && locks.masks(routine)) {
        return maskedText(t, _locale, routine, alarm.occurrence);
      }
      // A photo or voice Note without words is still there.
      return _title(alarm.text) ??
          (alarm.text == null ? t.noteGone : wordlessTitle(t, alarm.wordless));
    }

    final armed = [
      for (final alarm in Alarms(store).plan(wallClock))
        ArmedAlarm(
          alarm: alarm,
          title: title(alarm),
          body: occurrenceTime(t, _locale, alarm.occurrence),
        ),
    ];
    final done = Alarms(store).recentlyDone(wallClock);
    final planned = Alarms(store).nudge(wallClock);
    final nudge = planned == null
        ? null
        : ArmedNudge(
            nudge: planned,
            title: t.nudgeTitle(planned.count),
            body: t.nudgeBody,
          );
    final plannedChanges = Alarms(store).changesNudge(wallClock);
    final changes = plannedChanges == null
        ? null
        : ArmedChangesNudge(
            nudge: plannedChanges,
            title: t.changesNudgeTitle(
              namesText(t, [
                for (final person in plannedChanges.people)
                  profileDisplayName(t, store.personShown(person).name),
              ]),
            ),
            body: t.changesNudgeBody,
            public: t.changesNudgePublic,
          );
    final sound = store.alarmSound;
    final signature = [
      'sound $sound',
      for (final a in armed) '${a.alarm}|${a.title}|${a.body}',
      for (final (routine, occurrence) in done) 'done $routine@$occurrence',
      if (nudge != null) '${nudge.nudge}|${nudge.title}|${nudge.body}',
      if (changes != null)
        '${changes.nudge}|${changes.title}|${changes.body}|${changes.public}',
    ].join('\n');
    await _show(force: force);
    if (!force && signature == _lastArmed) return;
    _lastArmed = signature;
    await platform.arm(
      armed,
      AlarmLabels(
        channel: t.alarmChannel,
        done: t.markDone,
        snooze: t.snooze,
        snoozeQuestion: t.snoozeQuestion,
        snooze10: t.snooze10,
        snooze30: t.snooze30,
        snooze60: t.snooze60,
        catchUpTitle: t.catchUpTitle,
        catchUpOne: t.catchUpOne,
        // The platform puts in the number.
        catchUpMany: t.catchUpMany('{count}'),
        ringChannel: t.ringChannel,
        escalatedChannel: t.escalatedChannel,
        nudgeChannel: t.nudgeChannel,
      ),
      done: done,
      nudge: nudge,
      changes: changes,
      sound: sound,
    );
  }

  /// Hands the Agenda widget its snapshot when it changed: the
  /// Unsorted count, a Note's text or a Done change it without changing
  /// the alarms. Made of the Agenda's read while that is current (#204);
  /// else it waits for the Agenda's next read, or reads the log itself
  /// after [showDelay], once however many changes came meanwhile.
  Future<void> _show({required bool force}) async {
    if (_current() == null) {
      _showWaiting = (_showWaiting ?? false) || force;
      _showTimer?.cancel();
      _showTimer = Timer(showDelay, () {
        final force = _showWaiting;
        _showWaiting = null;
        if (force != null && !_disposed) {
          unawaited(_serial(() => _showNow(force: force)));
        }
      });
      return;
    }
    await _showNow(force: force);
  }

  /// The Agenda's read while the log has not changed since and it shows
  /// today, on this wall clock; else null.
  AgendaRead? _current() {
    final held = _agendaRead;
    if (held == null ||
        held.read.wallClock != wallClock ||
        held.seq != store.currentSeq() ||
        held.read.days.today != Day.today(store.clock(), wallClock)) {
      return null;
    }
    return held.read;
  }

  Future<void> _showNow({required bool force}) async {
    final t = _t;
    if (t == null || _disposed) return;
    final read = _current();
    final snapshot = read == null
        ? AgendaSnapshot.of(store, wallClock, t, _locale)
        : AgendaSnapshot.from(store, read, t, _locale);
    final signature = jsonEncode(snapshot.toMap());
    if (!force && signature == _lastShown) return;
    _lastShown = signature;
    await platform.showAgenda(snapshot);
  }

  /// The first line of a Note's text, or null when there is none.
  static String? _title(String? text) {
    final line = text?.trim().split('\n').first.trim();
    return line == null || line.isEmpty ? null : line;
  }

  /// Runs [job] after the jobs before it, one at a time.
  Future<void> _serial(Future<void> Function() job) {
    final next = _tail.then((_) => job());
    _tail = next.then<void>((_) {}, onError: (Object _) {});
    return next;
  }

  @override
  void dispose() {
    _disposed = true;
    _showTimer?.cancel();
    store.removeListener(_logChanged);
    store.removeProfileListener(_changed);
    unawaited(_arrivals?.cancel());
    super.dispose();
  }
}
