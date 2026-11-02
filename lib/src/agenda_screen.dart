import 'dart:async';

import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';

import 'agenda_group.dart';
import 'agenda_rows.dart';
import 'agenda_widget.dart';
import 'alarm_sync.dart';
import 'app_lock.dart';
import 'appointments.dart';
import 'area_offer_screen.dart';
import 'areas_screen.dart';
import 'browser.dart';
import 'all_routines_screen.dart';
import 'calendar_platform.dart';
import 'calendar_screen.dart';
import 'card_glance.dart';
import 'capture_fan.dart';
import 'capture_requests.dart';
import 'capture_screen.dart';
import 'frogs_screen.dart';
import 'held_sent_items_screen.dart';
import 'l10n.dart';
import 'look.dart';
import 'merge_into_screen.dart';
import 'missed_summary_card.dart';
import 'note_card.dart';
import 'note_editor_screen.dart';
import 'page_bar.dart';
import 'place_names.dart';
import 'nudge_screen.dart';
import 'photo_platform.dart';
import 'profile.dart';
import 'profile_screen.dart';
import 'routine_editor_screen.dart';
import 'routine_history_screen.dart';
import 'schedule_text.dart';
import 'session_view.dart';
import 'setup_screen.dart';
import 'system_bars.dart';
import 'tally_view.dart';
import 'text_marks_view.dart';

/// What the menu of an Occurrence's row does.
enum _RowChoice { history, share }

/// The home screen (CONTEXT.md): today's Occurrences, all-day first and
/// then by time, a visible gap, and tomorrow's in dimmer colours, then
/// the five days after the same way, each under its weekday and date.
/// Each is checked off as Done on its own, the coming days' already
/// today. "+"
/// fans out Text, Voice, Photo, Gallery and Paste to capture a Note, whose card
/// then shows on top with its enrich options
/// until the browser opens or the app comes back. The app bar's folder,
/// its first action, opens the notes browser; the bar also opens the list
/// of all Routines and Reminders and the Calendar page, makes a Routine
/// without a Note, and has the Profile picture in its corner, the only
/// way to Settings. Each
/// row's ⋮, or a long press on it, shares its Routine or Reminder as a
/// calendar file, and a Routine's leads to its history. A red
/// banner on top shows while alarms lack a permission, and leads to the
/// setup. Below it the Missed Summary shows until "seen", and under it
/// the "x Unsorted" banner while Notes are Unsorted, opening them, and
/// the "x Sent Items waiting" line while strangers' Sent Items wait to
/// be shown, accepted or declined, opening their list. At local
/// midnight the Agenda moves on to the new day, and whenever an
/// Occurrence becomes Missed the summary takes it in. Whatever another
/// app shares in opens in Capture over the Agenda, filled in and
/// Unsorted, and so does the capture screen a shortcut, tile, widget or
/// hardware button asked for, just as from "+". An Occurrence from a
/// locked Area set to Masked shows its masked text and can still be
/// checked off; opening it, or its history, asks to open its Area first.
/// Back on the Agenda, every Area opened meanwhile is closed again.
///
/// Under today's timed Occurrences the "Eat a frog" section offers
/// today's pick of two Frogs (ADR 0027), written for the user's own
/// Devices as it loads; each has "Another one" to swap it. Once one is
/// checked off, only that one shows, struck through, for the day. Its
/// heading opens the Frog view; it shows while a Frog is open or one was
/// eaten today. A Frog of a Masked Area shows masked, as an Occurrence.
///
/// A Session's group today lists its picks under its title (ADR 0029,
/// #208), written for everyone the Session reaches as it loads: each an
/// entry with its box, which with its words opens its entry form, and
/// "Another one" until it has an entry. The title's box only shows Done:
/// once each pick has an entry it checks itself, and a tap on it does
/// nothing. On later days the row says how many it picks. A Masked row
/// lists none, and a pick from a Masked Area shows masked.
///
/// Where switched on, every day also shows the appointments of the
/// calendars picked on the Calendar page (#116), read as the Agenda
/// loads, on resume and on a new day, and never kept: the all-day ones
/// after the all-day Occurrences, the timed ones among the timed
/// Occurrences by time. They are read only, marked with their calendar,
/// and "Copy into a Note" makes a Note of one.
///
/// Today also shows the Shopping Lists with open Lines, after the
/// all-day rows, each as the user chose for it (#186, #208): its name as
/// a heading with each open Line under it with its box, or the heading
/// alone with how many are open, or none. A tap opens the list; a Masked
/// one is the heading alone and names no Line.
///
/// After them today shows each Checklist without a Routine or Reminder
/// that has open Lines and that the user did not switch off for their
/// own Agenda (#143, #208): its name as a heading with its open Lines
/// under it, each with its box; a tap opens it, and a Masked one is the
/// heading alone with its counts, naming nothing of it.
class AgendaScreen extends StatefulWidget {
  /// How much tomorrow and the days after are faded.
  static const tomorrowOpacity = 0.55;

  /// How many days the Agenda shows, today included.
  static const days = 7;

  final Notes notes;
  final Places places;

  /// The notes browser's last selections, kept between visits.
  final BrowserTrail trail;

  /// The Device's alarms: taps applied and permissions changed.
  final AlarmSync alarms;

  /// The quick entry points asking for a capture screen.
  final CaptureRequests requests;

  /// The user's own Profile, whose picture opens the Profile page.
  final OwnProfileModel profile;

  /// Called after the theme was changed in Settings.
  final VoidCallback? onThemeChanged;

  const AgendaScreen({
    super.key,
    required this.notes,
    required this.places,
    required this.trail,
    required this.alarms,
    required this.profile,
    this.requests = const NoCaptureRequests(),
    this.onThemeChanged,
  });

  /// How many times an Agenda read its rows, for the tests that count
  /// them: once per change (#205).
  @visibleForTesting
  static var loads = 0;

  @override
  State<AgendaScreen> createState() => _AgendaScreenState();
}

class _AgendaScreenState extends State<AgendaScreen> {
  late final _routines = Routines(widget.notes.store);
  late final _alarms = Alarms(widget.notes.store);
  late final _locks = AreaLocks(widget.notes.store);
  late final _frogs = Frogs(widget.notes.store);
  late final _sessions = Sessions(widget.notes.store);
  late final _checklists = Checklists(widget.notes.store);
  late final _shopping = ShoppingLists(widget.notes.store);

  late final AppLifecycleListener _lifecycle;
  Timer? _timer;
  MissedSummary _missed = const MissedSummary(routines: [], nextChange: null);

  /// The days as read last, for a calendar read to go into (#205).
  AgendaDays? _days;
  List<DayEntry> _today = const [];
  List<DayEntry> _tomorrow = const [];

  /// The days after tomorrow, each with its rows.
  List<({Day day, List<DayEntry> rows})> _later = const [];

  /// The appointments of the calendars picked to show here, as read
  /// last, with those calendars by id; none while that is off (#116).
  List<Appointment> _appointments = const [];
  Map<String, ReadCalendar> _readCalendars = const {};

  /// Counts the calendar reads, so only the latest one's answer shows.
  var _calendarReads = 0;

  /// The appointment rows a tap unfolded, by [AppointmentPiece.key].
  final _unfoldedAppointments = <String>{};
  var _unsorted = 0;

  /// The units the targets of a Session's picks show in.
  var _system = UnitSystem.metric;

  /// Today's "Eat a frog" section, and its Frogs as they show.
  FrogsToday? _frogsToday;
  List<ShownFrog> _frogRows = const [];

  /// How many strangers' Sent Items wait (#91).
  var _waiting = 0;
  var _offers = 0;

  /// The Note captured last from here, shown on top until it is let go.
  String? _capturedId;
  PlacedNote? _captured;

  /// The captured Note when a tap switched its card to the compact view;
  /// it starts detailed, its buttons at hand for what was not done while
  /// capturing (#189).
  String? _compactId;

  /// Where shares come from; set with the first dependencies.
  PhotoPlatform? _photos;
  StreamSubscription<void>? _shares;
  late final StreamSubscription<void> _requests;

  /// Set while a share or a requested capture is open, so two never
  /// stack.
  var _opening = false;

  /// Where "+" sits, so the fan opens from it.
  final _plus = GlobalKey();

  @override
  void initState() {
    super.initState();
    _load();
    unawaited(_readCalendar());
    _lifecycle = AppLifecycleListener(
      // Back from the background, the last capture is old news. A
      // dialog or the notification shade only makes the app inactive.
      onShow: () => _capturedId = null,
      // Maybe a new day, maybe something shared or a capture asked
      // for meanwhile.
      onResume: () {
        _reload();
        unawaited(_readCalendar());
        unawaited(_openWaiting());
      },
    );
    widget.alarms.addListener(_alarmsChanged);
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
    widget.alarms.removeListener(_alarmsChanged);
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
    _timer = Timer(wait.isNegative ? Duration.zero : wait, () {
      _reload();
      unawaited(_readCalendar());
    });
  }

  /// Reads the appointments of the calendars picked to show here, over
  /// the Agenda's days and a day either side, off the UI thread on the
  /// platform's side; shows them once they arrive. Nothing is kept.
  Future<void> _readCalendar() async {
    final store = widget.notes.store;
    final source = AppointmentScope.of(context);
    final read = ++_calendarReads;
    final picked = store.agendaCalendarOn
        ? store.agendaCalendars
        : const <String>{};
    var appointments = const <Appointment>[];
    var calendars = const <ReadCalendar>[];
    if (source.available && picked.isNotEmpty && await source.granted()) {
      final today = Day.today(store.clock(), deviceWallClock);
      final from = DateTime(today.year, today.month, today.day - 1);
      final until = DateTime(
        today.year,
        today.month,
        today.day + AgendaScreen.days + 1,
      );
      calendars = await source.calendars();
      appointments =
          await source.appointments(from.toUtc(), until.toUtc(), picked) ??
          const [];
    }
    if (!mounted || read != _calendarReads) return;
    final shown = [
      for (final appointment in appointments)
        if (picked.contains(appointment.calendarId)) appointment,
    ];
    // Nothing shown before nor now, as while the calendar is off.
    if (shown.isEmpty &&
        _appointments.isEmpty &&
        calendars.isEmpty &&
        _readCalendars.isEmpty) {
      return;
    }
    // Into the days as read, without reading the log again (#205).
    setState(() {
      _appointments = shown;
      _readCalendars = {
        for (final calendar in calendars) calendar.id: calendar,
      };
      if (_days case final days?) _show(days.withAppointments(shown));
    });
  }

  /// Set while [_load] runs, so the log's change by its own picks reads
  /// nothing again (#205).
  var _loading = false;

  void _load() {
    AgendaScreen.loads++;
    _loading = true;
    try {
      _read();
    } finally {
      _loading = false;
    }
  }

  void _read() {
    // Writes today's picks when they are new, before the rows read their
    // Done; the log's change they make is no reason to read again.
    final sessions = _sessions.pickToday(deviceWallClock);
    final frogs = _frogsToday = _frogs.pickToday(deviceWallClock);
    final days = AgendaDays.read(
      store: widget.notes.store,
      places: widget.places,
      wallClock: deviceWallClock,
      days: AgendaScreen.days,
      sessions: sessions,
      frogsToday: frogs,
      appointments: _appointments,
    );
    _show(days);
    _missed = _alarms.missed(deviceWallClock);
    _unsorted = widget.notes.unsorted().length;
    // The widget's snapshot is made of this read, not of one of its own
    // (#204); it reads the calendar itself.
    widget.alarms.agendaRead(
      AgendaRead(
        wallClock: deviceWallClock,
        days: days.withAppointments(const []),
        frogs: frogs,
        missed: _missed,
        unsorted: _unsorted,
      ),
    );
    // Counted from the pending sets, without reading each held file.
    _waiting = widget.notes.store.pendingSets
        .where((s) => s.kind == sentItemPendingKind)
        .length;
    _offers = widget.notes.store.pendingSets
        .where((s) => s.kind == areaOfferPendingKind)
        .length;
    _armTimer();
    final captured = _capturedId == null
        ? null
        : widget.notes.byId(_capturedId ?? '');
    _captured =
        captured == null ||
            captured.isTrashed ||
            captured.isMerged ||
            (LockScope.of(context)?.hides(captured) ?? false)
        ? null
        : placeNote(widget.places, captured);
  }

  /// Whether this Device hides what is checked on the Agenda (#217), as
  /// the rows were read.
  bool get _hidesChecked => _days?.hidesChecked ?? false;

  /// Shows the rows of [days].
  void _show(AgendaDays days) {
    _days = days;
    _system = days.system;
    _today = days.todayRows;
    _tomorrow = days.tomorrowRows;
    _later = days.later;
    _frogRows = days.frogs;
  }

  /// Back on the Agenda: the Areas opened meanwhile close again.
  void _closeAreas() => LockScope.of(context)?.closeAreas();

  /// Opens the locked Area [row] comes from first, when it is closed;
  /// whether the way is open.
  Future<bool> _openAreaOf(AgendaRow row) => _openAreaOfNote(row.note);

  /// Opens the locked Area [note] is filed in first, when it is closed;
  /// whether the way is open.
  Future<bool> _openAreaOfNote(Note? note) async {
    final areaId = note == null ? null : _locks.lockedAreaOf(note);
    if (areaId == null) return true;
    final area = widget.places.areaById(areaId);
    return openLockedArea(
      context,
      LockScope.of(context),
      areaId,
      area == null ? '' : areaName(context.t, area),
    );
  }

  var _reloadQueued = false;

  /// Reads the rows again once, however many changes asked for it
  /// meanwhile, and never in the middle of whatever changed (#205): a
  /// check-off and the log's change it makes, a sync's writes, a resume
  /// and its calendar read.
  void _reload() {
    if (_reloadQueued) return;
    _reloadQueued = true;
    scheduleMicrotask(() {
      _reloadQueued = false;
      if (mounted) setState(_load);
    });
  }

  void _reloadIfMounted() {
    if (mounted) _reload();
  }

  /// The log changed: read again, unless the change is the Agenda's own
  /// picks written as it read.
  void _logChanged() {
    if (_loading) return;
    _reload();
  }

  /// The alarms' permissions or ringing changed, which only the banner
  /// shows; what a tap applied changed the log, which reads again.
  void _alarmsChanged() {
    if (mounted) setState(() {});
  }

  /// Fans out the ways to capture from "+", the ones this Device has,
  /// and opens the capture screen of the way picked.
  Future<void> _capture() async {
    final fan = captureFanFor(context, _plus);
    if (fan == null) return;
    final way = await Navigator.of(context).push(fan);
    if (way == null || !mounted) return;
    await captureNote(
      context,
      widget.notes,
      voice: way == CaptureWay.voice,
      photo: way == CaptureWay.photo,
      gallery: way == CaptureWay.gallery,
      paste: way == CaptureWay.paste,
      onCaptured: (id) => _capturedId = id,
      onChanged: _reloadIfMounted,
    );
  }

  /// Closes the fan of "+", here or on the lock screen, when it is open,
  /// before a share or a quick entry point opens Capture over it.
  void _closeFan() =>
      Navigator.of(context).popUntil((route) => route is! CaptureFanRoute);

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
          _closeFan();
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
        _closeFan();
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

  Future<void> _browse() => Navigator.of(context).push(_areasRoute());

  void _searchTag(BuildContext _, String query) =>
      unawaited(Navigator.of(context).push(_areasRoute(query)));

  /// The Areas list's page, for the magnifier, for swipe left to pull
  /// in and searching for [query], a tag tapped (ADR 0032); coming back
  /// closes the Areas opened on the way.
  BrowserRoute<void> _areasRoute([String query = '']) {
    _capturedId = null;
    final route = BrowserRoute<void>(
      builder: (_) => AreasScreen(
        notes: widget.notes,
        places: widget.places,
        trail: widget.trail,
        query: query,
      ),
    );
    route.popped.then((_) {
      if (!mounted) return;
      _closeAreas();
      _reload();
    });
    return route;
  }

  Future<void> _openUnsorted() async {
    await openUnsorted(context, widget.notes, widget.places);
    if (!mounted) return;
    _reload();
  }

  Future<void> _openHeld() async {
    await Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) => HeldSentItemsScreen(store: widget.notes.store),
      ),
    );
    if (!mounted) return;
    _reload();
  }

  Future<void> _openOffers() async {
    await Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) => HeldAreaOffersScreen(store: widget.notes.store),
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

  Future<void> _openProfile() async {
    await openProfile(
      context,
      widget.notes.store,
      widget.alarms,
      widget.profile,
      onThemeChanged: widget.onThemeChanged,
      trail: widget.trail,
    );
    if (!mounted) return;
    _reload();
    // Settings lead to the Calendar page too.
    unawaited(_readCalendar());
  }

  Future<void> _openAllRoutines() async {
    await openAllRoutines(context, widget.notes, widget.places);
    if (!mounted) return;
    _routineEdited();
  }

  Future<void> _openCalendar() async {
    await openCalendarPage(context, widget.notes.store);
    if (!mounted) return;
    _reload();
    unawaited(_readCalendar());
  }

  Future<void> _newRoutine() async {
    await newRoutine(context, widget.notes, widget.places);
    if (!mounted) return;
    _routineEdited();
  }

  /// Back from a Routine's editor: the alarms are armed again.
  void _routineEdited() {
    unawaited(widget.alarms.refresh());
    _reload();
  }

  /// Checks or unchecks the Line [lineId] of [row]'s Checklist for its
  /// Occurrence only; the last Line checked makes it Done.
  void _checkLine(AgendaRow row, String lineId, bool checked) {
    final on = (routineId: row.routine.id, occurrence: row.occurrence);
    checked
        ? _checklists.check(lineId, on: on)
        : _checklists.uncheck(lineId, on: on);
    _reload();
  }

  /// Checks the Line [lineId] of a Checklist without a Routine from its
  /// group today: it leaves the Agenda, and the list with its last.
  void _checkListLine(String lineId) {
    _checklists.check(lineId);
    _reload();
  }

  /// Checks the Line [lineId] of a Shopping List from its row today: it
  /// is struck through and leaves the Agenda until it comes back.
  void _checkShopping(String lineId) {
    _shopping.check(lineId, deviceWallClock);
    _reload();
  }

  /// Opens the Shopping List or Checklist of [note] in its editor, its
  /// locked Area first.
  Future<void> _openList(Note note) async {
    if (!await _openAreaOfNote(note) || !mounted) return;
    await openNote(context, widget.notes, widget.places, note);
    if (!mounted) return;
    _closeAreas();
    _reload();
  }

  void _toggle(AgendaRow row) {
    if (row.done == null) {
      // Its Note may go to the trash with it (#211).
      _routines.checkOff(row.routine.id, row.occurrence);
    } else {
      _routines.unmarkDone(row.routine.id, row.occurrence);
    }
    _reload();
  }

  /// Checks off a Missed Occurrence: Done, late.
  void _checkMissed(Routine routine, Occurrence occurrence) {
    _routines.checkOff(routine.id, occurrence);
    _reload();
  }

  void _seen() {
    _alarms.markSeen();
    _reload();
  }

  Future<void> _openRow(AgendaRow row) async {
    if (!await _openAreaOf(row) || !mounted) return;
    await openRoutine(context, widget.notes, widget.places, row.routine);
    if (!mounted) return;
    _closeAreas();
    _routineEdited();
  }

  /// The ⋮ of an Occurrence's row, or a long press on it, opens a menu
  /// where the finger is: a Routine's history, and Share, which hands the
  /// Routine as its whole series, or the Reminder, to another app as a
  /// calendar file. Both open a locked Area first, as a Masked row tells
  /// nothing until then. Null when there is nothing to offer.
  void Function(Offset at)? _rowMenu(AgendaRow row) {
    final history = !row.routine.isReminder;
    final share = CalendarScope.of(context).available;
    if (!history && !share) return null;
    return (at) => _showRowMenu(row, at, history: history, share: share);
  }

  Future<void> _showRowMenu(
    AgendaRow row,
    Offset at, {
    required bool history,
    required bool share,
  }) async {
    final t = context.t;
    final overlay =
        Overlay.of(context).context.findRenderObject() as RenderBox?;
    if (overlay == null) return;
    PopupMenuItem<_RowChoice> item(
      _RowChoice value,
      IconData icon,
      String label,
    ) => PopupMenuItem(
      key: ValueKey('row-${value.name}'),
      value: value,
      child: Row(
        children: [
          Icon(icon),
          const SizedBox(width: 12),
          Flexible(child: Text(label)),
        ],
      ),
    );
    final choice = await showMenu<_RowChoice>(
      context: context,
      position: RelativeRect.fromRect(
        overlay.globalToLocal(at) & Size.zero,
        Offset.zero & overlay.size,
      ),
      items: [
        if (history) item(_RowChoice.history, Icons.history, t.historyTitle),
        if (share) item(_RowChoice.share, Icons.share_outlined, t.share),
      ],
    );
    if (!mounted || choice == null) return;
    if (!await _openAreaOf(row) || !mounted) return;
    switch (choice) {
      case _RowChoice.history:
        await openHistory(context, widget.notes, row.routine);
      case _RowChoice.share:
        await shareRoutinesFile(context, _routines, [row.routine]);
    }
    if (!mounted) return;
    _closeAreas();
    _reload();
  }

  Future<void> _openPick(SessionOccurrence today, SessionPick pick) async {
    if (!await _openAreaOfNote(pick.note) || !mounted) return;
    await openPick(context, widget.notes, today, pick);
    if (!mounted) return;
    _closeAreas();
    _reload();
  }

  void _anotherPick(SessionOccurrence today, SessionPick pick) {
    _sessions.another(today.session.id, today.occurrence, pick.noteId);
    _reload();
  }

  /// An Occurrence's group (#208): its box, time and title as the
  /// heading, and under it a Session's picks or a Checklist Routine's
  /// Lines, each with its box.
  Widget _occurrenceRow(AppLocalizations t, String locale, PlacedRow placed) {
    final (:row, :area, :masked, :picks, :lines, :session) = placed;
    final note = row.note;
    // Done from the entries of every pick, or from every Line checked,
    // stays checked.
    final complete =
        (picks?.today.complete ?? false) || (lines?.list.allChecked ?? false);
    final done = row.done != null || complete;
    // A Session's box only shows Done: its picks make it (#208).
    final checks = !complete && session == null;
    return AgendaGroup(
      key: ValueKey('row-${row.routine.id}@${row.occurrence.key}'),
      // A Masked row tells nothing of its Area either.
      area: masked ? null : area,
      lead: AgendaBox(
        value: done,
        label: t.markDone,
        onChanged: checks ? (_) => _toggle(row) : null,
      ),
      time: occurrenceTime(t, locale, row.occurrence),
      title: masked
          ? maskedText(t, locale, row.routine, row.occurrence)
          : note == null
          ? t.noteGone
          : noteText(t, note),
      struck: row.done != null,
      under: _sessionUnder(t, placed),
      marks: [
        if (row.routine.isReminder)
          Icon(
            Icons.notifications_outlined,
            size: 18,
            color: Theme.of(context).colorScheme.onSurfaceVariant,
            semanticLabel: t.reminder,
          ),
      ],
      onTap: () => _openRow(row),
      onMenu: _rowMenu(row),
      menuKey: ValueKey('row-menu-${row.routine.id}'),
      entries: [..._sessionPicks(t, placed), ..._routineLines(placed)],
    );
  }

  /// What shows under a Session's title: on later days how many it
  /// picks, today that there is nothing left to pick; nothing on a Masked
  /// row.
  Widget? _sessionUnder(AppLocalizations t, PlacedRow placed) {
    final (:row, :masked, :session, :picks, area: _, lines: _) = placed;
    if (masked || session == null) return null;
    if (picks == null) {
      return Text(t.sessionPicks(session.countOn(row.occurrence.day)));
    }
    if (picks.today.picks.isEmpty) {
      return Text(
        t.sessionNothingToPick,
        style: Theme.of(context).textTheme.bodyMedium,
      );
    }
    return null;
  }

  /// Today's picks of a Session under its title, each with its box, which
  /// shows whether it has an entry; the box and the words open its entry
  /// form, and "Another one" goes under each without an entry while it
  /// swaps; nothing on a Masked row.
  List<Widget> _sessionPicks(AppLocalizations t, PlacedRow placed) {
    final (:masked, :picks, row: _, session: _, area: _, lines: _) = placed;
    if (masked || picks == null) return const [];
    final (:today, masked: hidden, :opens, :swaps) = picks;
    final decimal = decimalSign(context);
    return [
      for (final (i, pick) in agendaPicks(
        picks,
        hidesChecked: _hidesChecked,
      )) ...[
        AgendaEntry(
          key: ValueKey('session-pick-${pick.noteId}'),
          boxKey: ValueKey('session-check-${pick.noteId}'),
          text: hidden[i] ? t.maskedTally : pickText(t, pick, _system, decimal),
          checked: pick.done,
          onBox: opens[i] ? () => _openPick(today, pick) : null,
          onTap: opens[i] ? () => _openPick(today, pick) : null,
        ),
        // The control goes below its text.
        if (swaps && !pick.done)
          Padding(
            padding: const EdgeInsets.only(left: AgendaSizes.indent),
            child: Align(
              alignment: Alignment.centerLeft,
              child: TextButton.icon(
                key: ValueKey('session-another-${pick.noteId}'),
                onPressed: () => _anotherPick(today, pick),
                icon: const Icon(Icons.swap_horiz),
                label: Text(t.sessionAnother),
              ),
            ),
          ),
      ],
    ];
  }

  /// The Lines of a Routine spawned from a Checklist under its title, as
  /// checked for this Occurrence, checkable where this Device may check
  /// it off, on every day shown; a tap on one's words opens the Routine
  /// as the heading does. Nothing on a Masked row.
  List<Widget> _routineLines(PlacedRow placed) {
    final (:row, :masked, :lines, area: _, session: _, picks: _) = placed;
    if (masked || lines == null) return const [];
    final (:list, :checks) = lines;
    return [
      KeyedSubtree(
        key: ValueKey('row-checklist-${row.routine.id}@${row.occurrence.key}'),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            for (final line in agendaLines(list, hidesChecked: _hidesChecked))
              _lineEntry(
                line,
                onBox: checks
                    ? () => _checkLine(row, line.id, !line.checked)
                    : null,
                onTap: () => _openRow(row),
              ),
          ],
        ),
      ),
    ];
  }

  /// A Checklist's Line as an entry, a Line under another one step
  /// further in.
  Widget _lineEntry(
    Line line, {
    required VoidCallback? onBox,
    required VoidCallback onTap,
  }) => AgendaEntry(
    boxKey: ValueKey('checklist-check-${line.id}'),
    textKey: ValueKey('checklist-line-${line.id}'),
    text: line.text,
    checked: line.checked,
    depth: line.inGroup ? 1 : 0,
    onBox: onBox,
    onTap: onTap,
  );

  /// The title of the list [note]: its first line, else [fallback].
  String _listTitle(AppLocalizations t, Note note, String fallback) =>
      switch (noteText(t, note).trim().split('\n').first.trim()) {
        '' => fallback,
        final title => title,
      };

  /// A Shopping List's group today (#208): its name as the heading, with
  /// no box, and its open Lines under it, each with its box, which checks
  /// it and takes it off; or, as chosen for it or Masked, the heading
  /// alone with how many are open. A tap opens the list.
  Widget _shoppingRow(AppLocalizations t, ShoppingRow shopping) {
    final (:note, :area, :masked, :lines, :count, :checks) = shopping;
    final title = _listTitle(t, note, t.shoppingHeading);
    return AgendaGroup(
      key: ValueKey('shopping-row-${note.id}'),
      area: masked ? null : area,
      lead: AgendaGroup.icon(context, Icons.shopping_cart_outlined),
      title: lines.isNotEmpty
          ? title
          : t.shoppingRowOpen(masked ? t.maskedShoppingList : title, count),
      onTap: () => _openList(note),
      entries: [
        for (final line in lines)
          AgendaEntry(
            key: ValueKey('shopping-line-${line.id}'),
            boxKey: ValueKey('shopping-check-${line.id}'),
            text: line.text,
            checked: false,
            onBox: checks ? () => _checkShopping(line.id) : null,
            onTap: () => _openList(note),
          ),
      ],
    );
  }

  /// A Checklist's group today (#208), as a Shopping List's: its name as
  /// the heading, with no box, and its open Lines under it, each with its
  /// box; without Lines, as a Masked one, the heading alone with the count,
  /// "Private Checklist: 3 of 7 open". A tap opens the list.
  Widget _checklistRow(AppLocalizations t, ChecklistRow row) {
    final (:note, :area, :masked, :list, :lines, :checks) = row;
    final title = _listTitle(t, note, t.checklistHeading);
    return AgendaGroup(
      key: ValueKey('checklist-row-${note.id}'),
      area: masked ? null : area,
      lead: AgendaGroup.icon(context, Icons.checklist),
      title: lines.isNotEmpty
          ? title
          : t.checklistRowOpen(
              masked ? t.maskedChecklist : title,
              list.openCount,
              list.count,
            ),
      onTap: () => _openList(note),
      entries: [
        for (final line in lines)
          _lineEntry(
            line,
            onBox: checks ? () => _checkListLine(line.id) : null,
            onTap: () => _openList(note),
          ),
      ],
    );
  }

  /// An appointment's row: read only, marked with its calendar's icon,
  /// colour and name; a tap unfolds its place and description, and its ⋮
  /// or a long press offers "Copy into a Note".
  Widget _appointmentRow(
    AppLocalizations t,
    String locale,
    AppointmentPiece piece,
  ) {
    final unfolded = _unfoldedAppointments.contains(piece.key);
    return _AppointmentRow(
      key: ValueKey('appointment-${piece.key}'),
      piece: piece,
      calendar: _readCalendars[piece.appointment.calendarId],
      time: pieceTime(t, locale, piece),
      unfolded: unfolded,
      onTap: () => setState(
        () => unfolded
            ? _unfoldedAppointments.remove(piece.key)
            : _unfoldedAppointments.add(piece.key),
      ),
      onMenu: (at) => _showAppointmentMenu(piece.appointment, at),
    );
  }

  Future<void> _showAppointmentMenu(Appointment appointment, Offset at) async {
    final t = context.t;
    final overlay =
        Overlay.of(context).context.findRenderObject() as RenderBox?;
    if (overlay == null) return;
    final copy = await showMenu<bool>(
      context: context,
      position: RelativeRect.fromRect(
        overlay.globalToLocal(at) & Size.zero,
        Offset.zero & overlay.size,
      ),
      items: [
        PopupMenuItem(
          key: const ValueKey('appointment-copy'),
          value: true,
          child: Row(
            children: [
              const Icon(Icons.note_add_outlined),
              const SizedBox(width: 12),
              Flexible(child: Text(t.appointmentCopy)),
            ],
          ),
        ),
      ],
    );
    if (copy != true || !mounted) return;
    _copyAppointment(appointment);
  }

  /// Makes a new Note of [appointment]'s details, Unsorted and nudged as
  /// a Note captured without a choice; its card shows on top with its
  /// enrich options. The appointment stays as it is.
  void _copyAppointment(Appointment appointment) {
    final t = context.t;
    final locale = Localizations.localeOf(context).toString();
    final note = widget.notes.capture(
      appointmentNoteText(t, locale, appointment, deviceWallClock),
      nudgeIn: Notes.defaultNudge,
    );
    _capturedId = note.id;
    _compactId = null;
    _reload();
  }

  void _toggleFrog(Frog frog) {
    if (frog.isDone) {
      _frogs.unmarkDone(frog.id);
    } else {
      _frogs.markDone(frog.id);
    }
    _reload();
  }

  void _anotherFrog(Frog frog) {
    _frogs.another(deviceWallClock, frog.id);
    _reload();
  }

  Future<void> _openFrogNote(Note note) async {
    if (!await _openAreaOfNote(note) || !mounted) return;
    await openNote(context, widget.notes, widget.places, note);
    if (!mounted) return;
    _closeAreas();
    _reload();
  }

  Future<void> _openFrogs() async {
    await openFrogs(context, widget.notes, widget.places);
    if (!mounted) return;
    _closeAreas();
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

  Future<void> _schedule(Note note, Spawn spawn) async {
    await spawnFromNote(
      context,
      widget.notes,
      widget.places,
      note,
      spawn: spawn,
    );
    if (!mounted) return;
    _reload();
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final locale = Localizations.localeOf(context).toString();
    final captured = _captured;
    // Read only when its compact view shows.
    final glances = CardGlances.inContext(context, widget.notes.store);
    List<Widget> rows(List<DayEntry> entries) => [
      for (final entry in entries)
        if (entry.piece case final piece?)
          _appointmentRow(t, locale, piece)
        else if (entry.row case final placed?)
          _occurrenceRow(t, locale, placed)
        else if (entry.shopping case final shopping?)
          _shoppingRow(t, shopping)
        else if (entry.checklist case final checklist?)
          _checklistRow(t, checklist),
    ];
    // Swipe left opens the notes browser, as the folder does.
    return BrowserSwipes(
      forwardRoute: _areasRoute,
      back: false,
      child: TagSearchScope(
        onSearch: _searchTag,
        child: Scaffold(
          appBar: pageBar(
            context,
            // The band-aid brain of the feature graphic, title-high and
            // silent: the title is still read as the app's name.
            title: t.appTitle,
            titleIcon: Image.asset(
              'assets/brain.png',
              key: const ValueKey('brain'),
              height: 28,
              excludeFromSemantics: true,
            ),
            actions: [
              // The Areas list, first in the bar (#221).
              IconButton(
                key: const ValueKey('browse'),
                tooltip: t.browseTooltip,
                onPressed: _browse,
                icon: const Icon(Icons.folder_outlined),
              ),
              IconButton(
                key: const ValueKey('allRoutines'),
                tooltip: t.allRoutinesTitle,
                onPressed: _openAllRoutines,
                icon: const Icon(Icons.format_list_bulleted),
              ),
              if (hasCalendarPage(context))
                IconButton(
                  key: const ValueKey('calendarPage'),
                  tooltip: t.calendarPageTitle,
                  onPressed: _openCalendar,
                  icon: const Icon(Icons.event_outlined),
                ),
              IconButton(
                tooltip: t.newRoutineTooltip,
                onPressed: _newRoutine,
                icon: const Icon(Icons.add_alarm),
              ),
              // The Profile picture, where the settings gear was: Settings,
              // Sync and Backups are reached through the Profile page.
              IconButton(
                key: const ValueKey('profile'),
                tooltip: t.profileTitle,
                onPressed: _openProfile,
                icon: OwnProfileAvatar(model: widget.profile),
              ),
            ],
          ),
          body: ListView(
            padding: clearOfFloatingButtons(
              context,
              const EdgeInsets.only(top: 8),
            ),
            children: [
              if (widget.alarms.missing.isNotEmpty)
                _AlarmsBanner(onTap: _setUp),
              if (!_missed.isEmpty)
                MissedSummaryCard(
                  summary: _missed,
                  masks: _locks.masks,
                  onCheck: _checkMissed,
                  onSeen: _seen,
                ),
              if (_unsorted > 0)
                _CountBanner(
                  bannerKey: const ValueKey('unsortedBanner'),
                  icon: Icons.move_to_inbox,
                  text: t.unsortedBanner(_unsorted),
                  onTap: _openUnsorted,
                ),
              if (_waiting > 0)
                _CountBanner(
                  bannerKey: const ValueKey('sentItemsWaiting'),
                  icon: Icons.mark_email_unread_outlined,
                  text: t.sentItemsWaiting(_waiting),
                  onTap: _openHeld,
                ),
              if (_offers > 0)
                _CountBanner(
                  bannerKey: const ValueKey('areaOffersWaiting'),
                  icon: Icons.group_add_outlined,
                  text: t.areaOffersWaiting(_offers),
                  onTap: _openOffers,
                ),
              if (captured != null) ...[
                _Heading(t.justCaptured),
                NoteCard(
                  note: captured.note,
                  when: DateFormat.yMd(locale).add_Hm(),
                  area: captured.area,
                  topic: captured.topic,
                  showPlace: true,
                  onFile: () => _file(captured.note),
                  onMerge: () => _merge(captured.note),
                  // Only a Manager spawns in a Shared Area (ADR 0013).
                  onSchedule: captured.schedules
                      ? (spawn) => _schedule(captured.note, spawn)
                      : null,
                  actions: noteActions(
                    context,
                    widget.notes,
                    captured,
                    _reload,
                    onEdit: () => _open(captured.note),
                  ),
                  full: _compactId != captured.note.id,
                  sections: () => widget.notes.sections(captured.note.id),
                  checklist: (
                    read: () =>
                        Checklists(widget.notes.store)
                            .shownNow(captured.note.id, deviceWallClock),
                    lists: Checklists(widget.notes.store),
                  ),
                  shopping: (
                    read: () =>
                        ShoppingLists(widget.notes.store)
                            .byNote(captured.note.id, deviceWallClock),
                    lists: ShoppingLists(widget.notes.store),
                  ),
                  tally: widget.notes,
                  session: widget.notes,
                  kind: () =>
                      NoteKinds(widget.notes.store).of(captured.note.id),
                  glance: () => glances.of(captured.note.id),
                  boxes: boxTicks(widget.notes, captured, _reload),
                  onTable: tableEdit(widget.notes, captured, _reload),
                  onTap: () => setState(
                    () => _compactId = _compactId == captured.note.id
                        ? null
                        : captured.note.id,
                  ),
                  onEdit: () => _open(captured.note),
                ),
              ],
              _Heading(t.agendaToday),
              if (_today.isEmpty)
                BrowserEmpty(text: t.nothingToday)
              else
                ...rows(_today),
              // Under what is due, never above it (#101).
              if (_frogsToday case final frogs?
                  when _days?.showsFrogs(frogs) ?? frogs.visible) ...[
                _FrogsHeading(onTap: _openFrogs),
                for (final (:placed, :masked) in _frogRows)
                  FrogRow(
                    key: ValueKey('frog-${placed.frog.id}'),
                    frog: placed.frog,
                    // A Masked row tells nothing of its Area either.
                    area: masked ? null : placed.area,
                    title: masked ? t.maskedFrog : noteText(t, placed.note),
                    onToggle: widget.notes.store.mayCheckOff(placed.frog.id)
                        ? () => _toggleFrog(placed.frog)
                        : null,
                    onTap: () => _openFrogNote(placed.note),
                    onMenu: (at) => frogMenu(context, _frogs, placed.frog, at),
                    onAnother: frogs.canSwap && !frogs.eaten
                        ? () => _anotherFrog(placed.frog)
                        : null,
                  ),
              ],
              // The visible gap between today and tomorrow.
              const SizedBox(height: 32),
              Opacity(
                opacity: AgendaScreen.tomorrowOpacity,
                child: Column(
                  children: [
                    _Heading(t.agendaTomorrow),
                    // Only today's empty day tells how to capture.
                    if (_tomorrow.isEmpty)
                      _NothingPlanned(t.nothingPlanned)
                    else
                      ...rows(_tomorrow),
                  ],
                ),
              ),
              // The days after, each under its weekday and date; one with
              // nothing due still shows.
              for (final (:day, rows: later) in _later) ...[
                const SizedBox(height: 16),
                Opacity(
                  opacity: AgendaScreen.tomorrowOpacity,
                  child: Column(
                    children: [
                      _Heading(t.agendaDay(day.wall)),
                      if (later.isEmpty)
                        _NothingPlanned(t.nothingPlanned)
                      else
                        ...rows(later),
                    ],
                  ),
                ),
              ],
            ],
          ),
          floatingActionButton: FloatingActionButton(
            key: _plus,
            heroTag: 'capture',
            tooltip: t.captureTooltip,
            onPressed: _capture,
            child: const Icon(Icons.add),
          ),
        ),
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

/// "x Unsorted" while Notes wait to be sorted, or "x Sent Items
/// waiting"; tapping it opens them.
class _CountBanner extends StatelessWidget {
  /// Finds the banner itself.
  final Key bannerKey;
  final IconData icon;
  final String text;
  final VoidCallback onTap;

  const _CountBanner({
    required this.bannerKey,
    required this.icon,
    required this.text,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 0, 16, 8),
      child: Material(
        key: bannerKey,
        color: scheme.secondaryContainer,
        borderRadius: BorderRadius.circular(16),
        child: InkWell(
          borderRadius: BorderRadius.circular(16),
          onTap: onTap,
          child: Padding(
            padding: const EdgeInsets.all(16),
            child: Row(
              children: [
                Icon(icon, color: scheme.onSecondaryContainer),
                const SizedBox(width: 12),
                Expanded(
                  child: Text(
                    text,
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

/// The heading of the "Eat a frog" section, with the frog; tapping it
/// opens the Frog view.
class _FrogsHeading extends StatelessWidget {
  final VoidCallback onTap;

  const _FrogsHeading({required this.onTap});

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.fromLTRB(12, 12, 12, 0),
    child: InkWell(
      key: const ValueKey('frogsHeading'),
      borderRadius: BorderRadius.circular(12),
      onTap: onTap,
      child: Padding(
        padding: const EdgeInsets.fromLTRB(12, 4, 12, 4),
        child: Row(
          children: [
            const FrogIcon(),
            const SizedBox(width: 8),
            Expanded(
              child: Text(
                context.t.eatAFrog,
                style: Theme.of(context).textTheme.titleMedium,
              ),
            ),
            const Icon(Icons.chevron_right),
          ],
        ),
      ),
    ),
  );
}

/// The line under a day with nothing due.
class _NothingPlanned extends StatelessWidget {
  final String text;

  const _NothingPlanned(this.text);

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.fromLTRB(24, 0, 24, 8),
    child: Align(
      alignment: Alignment.centerLeft,
      child: Text(text, style: Theme.of(context).textTheme.bodyLarge),
    ),
  );
}

/// A calendar appointment's part on one day as an embossed row, read
/// only: its calendar's colour down the side and its icon, its time, its
/// title, its calendar's name, and ⋮; no box, as nothing here checks it
/// off. A tap unfolds its place and description, or folds them again; a
/// long press opens [onMenu] where the finger is, and ⋮ opens it at the
/// button.
class _AppointmentRow extends StatelessWidget {
  final AppointmentPiece piece;

  /// Its calendar, when the calendar told it.
  final ReadCalendar? calendar;
  final String time;
  final bool unfolded;
  final VoidCallback onTap;
  final void Function(Offset at) onMenu;

  const _AppointmentRow({
    super.key,
    required this.piece,
    required this.calendar,
    required this.time,
    required this.unfolded,
    required this.onTap,
    required this.onMenu,
  });

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final text = Theme.of(context).textTheme;
    final scheme = Theme.of(context).colorScheme;
    final appointment = piece.appointment;
    final calendar = this.calendar;
    final colour = calendar == null
        ? scheme.onSurfaceVariant
        : calendarColour(calendar.colour);
    final location = appointment.location.trim();
    final description = appointment.description.trim();
    final dim = text.bodyMedium?.copyWith(color: scheme.onSurfaceVariant);
    return Padding(
      padding: const EdgeInsets.symmetric(
        horizontal: 16,
        vertical: AgendaSizes.gap,
      ),
      child: Embossed(
        child: ClipRRect(
          borderRadius: BorderRadius.circular(16),
          // The row's ink shows on the card, not under it.
          child: Material(
            type: MaterialType.transparency,
            child: GestureDetector(
              onLongPressStart: (details) => onMenu(details.globalPosition),
              child: InkWell(
                onTap: onTap,
                child: Container(
                  decoration: BoxDecoration(
                    border: Border(left: BorderSide(color: colour, width: 6)),
                  ),
                  padding: const EdgeInsets.fromLTRB(12, 2, 0, 2),
                  child: Row(
                    children: [
                      Icon(
                        Icons.event_note_outlined,
                        color: colour,
                        semanticLabel: t.appointmentFromCalendar(
                          calendar?.name ?? '',
                        ),
                      ),
                      const SizedBox(width: 12),
                      Expanded(
                        child: Padding(
                          padding: const EdgeInsets.symmetric(vertical: 2),
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            children: [
                              Text(time, style: text.labelLarge),
                              Text(
                                appointmentTitle(t, appointment),
                                maxLines: unfolded ? null : 2,
                                overflow: unfolded
                                    ? null
                                    : TextOverflow.ellipsis,
                                style: text.bodyLarge,
                              ),
                              if (calendar != null)
                                Text(calendar.name, style: text.labelSmall),
                              if (unfolded && location.isNotEmpty)
                                Text(t.appointmentPlace(location), style: dim),
                              if (unfolded && description.isNotEmpty)
                                Padding(
                                  padding: const EdgeInsets.only(top: 4),
                                  child: Text(description, style: dim),
                                ),
                            ],
                          ),
                        ),
                      ),
                      // The same menu as the long press, at the button.
                      Builder(
                        builder: (button) => IconButton(
                          key: ValueKey('appointment-menu-${piece.key}'),
                          tooltip: t.more,
                          // 40 by 40, as the boxes (#208).
                          style: IconButton.styleFrom(
                            tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                          ),
                          icon: const Icon(Icons.more_vert),
                          onPressed: () {
                            final box = button.findRenderObject() as RenderBox?;
                            if (box == null) return;
                            onMenu(
                              box.localToGlobal(
                                box.size.bottomLeft(Offset.zero),
                              ),
                            );
                          },
                        ),
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
