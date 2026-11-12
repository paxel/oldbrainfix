import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'agenda_reader.dart';
import 'agenda_widget.dart';
import 'alarm_platform.dart';
import 'unit_system.dart';

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
  /// The permissions only Escalation needs: their absence shows in the
  /// red banner only while any Routine escalates; that of the platform's
  /// other [AlarmPlatform.steps] always does.
  static const forEscalation = {
    AlarmPermission.fullScreen,
    AlarmPermission.alarms,
  };

  final Store store;
  final AlarmPlatform platform;
  final WallClock wallClock;

  /// Where the plan and the Agenda's reads run: the app hands in an
  /// isolate of their own (#207), tests read on [store].
  final AgendaReader reader;

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

  /// Whether the next arming is handed over even unchanged: one asked
  /// for it, and its plan was read before a newer change (#207).
  var _forceNext = false;

  /// Whether any Routine escalates, as of the last arming.
  var _escalating = false;
  Ringing? _ringing;
  var _nudgeOpened = false;
  var _changesOpened = false;
  var _unsortedOpened = false;
  WidgetPick? _pickOpened;
  OpenedOccurrence? _occurrenceOpened;

  AlarmSync({
    required this.store,
    required this.platform,
    required this.wallClock,
    this.reader = const InlineAgendaReader(),
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

  /// The platform's permissions missing, for the red banner, those
  /// [forEscalation] only while any Routine escalates; none where there
  /// is no alarm layer.
  List<AlarmPermission> get missing => [
    if (platform.available)
      for (final p in platform.steps)
        if ((_escalating || !forEscalation.contains(p)) && !granted(p)) p,
  ];

  /// The permissions the setup walks through, in order.
  List<AlarmPermission> get steps => platform.steps;

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

  /// The Occurrence whose notification was tapped and whose Note is
  /// still to open, or null; taking it clears it (#270).
  OpenedOccurrence? takeOccurrenceOpened() {
    final opened = _occurrenceOpened;
    _occurrenceOpened = null;
    return opened;
  }

  /// Done, or a snooze for [snooze], from the full-screen alarm: the
  /// platform stops ringing and queues it, and it is applied at once.
  Future<void> answer(Ringing ring, {Duration? snooze}) async {
    await platform.answerRing(ring, snooze: snooze);
    await refresh();
  }

  /// Snoozes [occurrence] of the Routine [routineId] from the app until
  /// [until] (#269), as a snooze tapped on its notification does, and
  /// arms the plan anew at once: a snooze is this Device's own setting,
  /// which changes nothing in the log.
  void snooze(String routineId, Occurrence occurrence, DateTime until) {
    Alarms(store).applySnooze(routineId, occurrence.key, until);
    _changed();
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
        case SkipTapped(:final routineId, :final occurrenceKey, :final at):
          alarms.applySkip(routineId, occurrenceKey, at);
        case TomorrowTapped(:final routineId, :final occurrenceKey, :final at):
          alarms.applyTomorrow(routineId, occurrenceKey, at);
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
    final occurrenceOpened = await platform.occurrenceOpened();
    if (_disposed) return;
    if (nudgeOpened) _nudgeOpened = true;
    if (changesOpened) _changesOpened = true;
    if (unsortedOpened) _unsortedOpened = true;
    if (pickOpened != null) _pickOpened = pickOpened;
    if (occurrenceOpened != null) _occurrenceOpened = occurrenceOpened;
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
            pickOpened != null ||
            occurrenceOpened != null)) {
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

  /// The Agenda read [read] as it loaded (#204), at the log's mark
  /// [seq], else now: the widget's snapshot takes it while the log has
  /// not changed since, rather than reading the log a second time.
  void agendaRead(AgendaRead read, {int? seq}) {
    if (_disposed) return;
    _agendaRead = (read: read, seq: seq ?? store.currentSeq());
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
    final locale = _locale;
    final plan = await reader.plan(store, wallClock, t, locale);
    if (_disposed) return;
    // Read before a newer change, whose arming comes next (#207).
    if (plan.seq != store.currentSeq()) {
      _forceNext = _forceNext || force;
      return;
    }
    force = force || _forceNext;
    _forceNext = false;
    if (plan.escalating != _escalating) {
      _escalating = plan.escalating;
      notifyListeners();
    }
    final AlarmPlanRead(:armed, :done, :nudge, :changes) = plan;
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
        notToday: t.notToday,
        notTodayQuestion: t.notTodayQuestion,
        skip: t.skip,
        tomorrow: t.notTodayTomorrow,
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
      _wait(force: force);
      return;
    }
    await _showNow(force: force);
  }

  /// Waits for the Agenda's next read, or [showDelay] without one.
  void _wait({required bool force}) {
    _showWaiting = (_showWaiting ?? false) || force;
    _showTimer?.cancel();
    _showTimer = Timer(showDelay, () {
      final force = _showWaiting;
      _showWaiting = null;
      if (force != null && !_disposed) {
        unawaited(_serial(() => _showNow(force: force)));
      }
    });
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
    var read = _current();
    if (read == null) {
      // Read off the UI thread where the app reads so (#207).
      final load = await reader.read(store, wallClock, deviceUnitSystem(store));
      if (_disposed) return;
      // Read before a newer change, which shows anew.
      if (load.seq != store.currentSeq()) {
        _wait(force: force);
        return;
      }
      read = AgendaRead.of(load, wallClock);
    }
    final snapshot = AgendaSnapshot.from(store, read, t, _locale);
    final signature = jsonEncode(snapshot.toMap());
    if (!force && signature == _lastShown) return;
    _lastShown = signature;
    await platform.showAgenda(snapshot);
  }

  /// Runs [job] after the jobs before it, one at a time.
  Future<void> _serial(Future<void> Function() job) {
    final next = _tail.then((_) => job());
    _tail = next.then<void>((_) {}, onError: (Object _) {});
    return next;
  }

  /// Hands the Agenda widget the snapshot still waiting for the Agenda's
  /// read, now: a background run has no Agenda and may end before
  /// [showDelay] is over.
  Future<void> settle() {
    final force = _showWaiting;
    _showTimer?.cancel();
    _showWaiting = null;
    if (force == null || _disposed) return _serial(() async {});
    return _serial(() => _showNow(force: force));
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
