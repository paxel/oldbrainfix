import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'announcements.dart';
import 'failure_dialog.dart';
import 'l10n.dart';
import 'schedule_text.dart';

/// The platform seam for calendars: the calendar app's own insert
/// screen, which needs no calendar permission, and handing a file to
/// another app (android/…/CalendarExport.kt). Widget tests hand in a
/// fake.
abstract interface class CalendarPlatform {
  /// Whether this Device has calendar apps to add to and share with; the
  /// calendar buttons only show where it has.
  bool get available;

  /// Opens the calendar app filled in with [event]; false when there is
  /// none.
  Future<bool> insert(CalendarEvent event);

  /// Offers [ics] as the calendar file [name] to the app the user picks;
  /// false when no app can take it.
  Future<bool> share(String name, String ics);
}

/// No calendar: the default where none is handed in.
class NoCalendar implements CalendarPlatform {
  const NoCalendar();

  @override
  bool get available => false;

  @override
  Future<bool> insert(CalendarEvent event) async => false;

  @override
  Future<bool> share(String name, String ics) async => false;
}

/// Android's insert intent and share chooser behind the app's method
/// channel; no Google library, no calendar permission.
class AndroidCalendar implements CalendarPlatform {
  static const _channel = MethodChannel('io.github.paxel.oldbrainfix/calendar');

  const AndroidCalendar();

  @override
  bool get available => true;

  @override
  Future<bool> insert(CalendarEvent event) => _ask('insert', {
    'title': event.summary,
    'description': event.description,
    'start': event.start?.key,
    'rule': event.rule,
    'minutes': CalendarEvent.length.inMinutes,
  });

  @override
  Future<bool> share(String name, String ics) => _ask('share', {
    'name': name,
    'bytes': Uint8List.fromList(utf8.encode(ics)),
    'type': 'text/calendar',
  });

  static Future<bool> _ask(String method, Map<String, Object?> args) async {
    try {
      return await _channel.invokeMethod<bool>(method, args) ?? false;
    } on PlatformException {
      return false;
    }
  }
}

/// Hands the app's [CalendarPlatform] down to every screen.
class CalendarScope extends InheritedWidget {
  final CalendarPlatform calendar;

  const CalendarScope({
    super.key,
    required this.calendar,
    required super.child,
  });

  /// The app's [CalendarPlatform]; read once, so it does not listen.
  static CalendarPlatform of(BuildContext context) =>
      context.getInheritedWidgetOfExactType<CalendarScope>()?.calendar ??
      const NoCalendar();

  @override
  bool updateShouldNotify(CalendarScope oldWidget) =>
      calendar != oldWidget.calendar;
}

/// The name of an exported calendar file.
const calendarFileName = 'oldbrainfix.ics';

/// The calendar events of [routine], titled with its Note's text.
List<CalendarEvent> eventsOf(
  BuildContext context,
  Routines routines,
  Routine routine,
) {
  final t = context.t;
  final note = routines.note(routine);
  return routineEvents(routine, note == null ? t.noteGone : noteText(t, note));
}

/// One choice of a calendar menu: what it says and what it does.
typedef CalendarChoice = ({String label, Future<void> Function() run});

/// The choices to add each of [routine]'s events to the calendar, and
/// none when it has no event.
List<CalendarChoice> addChoices(
  BuildContext context,
  Routines routines,
  Routine routine,
) {
  final t = context.t;
  final locale = Localizations.localeOf(context).toString();
  final repeat = repeatText(t, locale, routine.schedule);
  return [
    for (final event in eventsOf(context, routines, routine))
      (
        label: t.addToCalendarAt(
          '$repeat · ${switch (event.start?.time) {
            null => t.allDay,
            final time => clockText(locale, time),
          }}',
        ),
        run: () => addToCalendar(context, event),
      ),
  ];
}

/// Opens the calendar app filled in with [event]; says so at the top
/// when there is none.
Future<void> addToCalendar(BuildContext context, CalendarEvent event) async {
  final announcements = AnnouncementsScope.of(context);
  final text = context.t.noCalendarApp;
  if (await CalendarScope.of(context).insert(event)) return;
  announcements?.announce(text);
}

/// Offers [events] as one calendar file, made at the store's now; says
/// so at the top when no app can take it.
Future<void> shareCalendarFile(
  BuildContext context,
  Store store,
  Iterable<CalendarEvent> events,
) async {
  final announcements = AnnouncementsScope.of(context);
  final text = context.t.nothingTakesCalendarFile;
  final ics = writeIcs(events, stamp: store.clock());
  if (await CalendarScope.of(context).share(calendarFileName, ics)) return;
  announcements?.announce(text);
}

/// Offers [list] as one calendar file, made at the store's now, each
/// titled with its Note's text: every one for Export, or one Routine as
/// its whole series or one Reminder for Share. Announces a failure when
/// no app can take it, why behind the tap.
Future<void> shareRoutinesFile(
  BuildContext context,
  Routines routines,
  List<Routine> list,
) async {
  final t = context.t;
  final announcements = AnnouncementsScope.of(context);
  final navigator = Navigator.of(context, rootNavigator: true);
  final ics = routinesIcs(list, (routine) {
    final note = routines.note(routine);
    return note == null ? t.noteGone : noteText(t, note);
  }, stamp: routines.store.clock());
  if (await CalendarScope.of(context).share(calendarFileName, ics)) return;
  announceFailureWith(
    announcements,
    navigator,
    t.calendarFileNotShared,
    title: t.calendarFileNotSharedTitle,
    cause: t.nothingTakesCalendarFile,
    fix: t.calendarFileFix,
    details: 'calendar file: no app took it',
  );
}

/// The calendar button of an app bar: does its only choice at once, or
/// opens a menu of [choices] where the finger is. Nothing shows without
/// a calendar or without choices.
class CalendarButton extends StatelessWidget {
  final List<CalendarChoice> Function() choices;

  const CalendarButton({super.key, required this.choices});

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    if (!CalendarScope.of(context).available) return const SizedBox.shrink();
    return IconButton(
      tooltip: t.calendarTooltip,
      icon: const Icon(Icons.event_outlined),
      onPressed: () => _pressed(context),
    );
  }

  Future<void> _pressed(BuildContext context) =>
      runCalendarChoice(context, choices());
}

/// Does the only one of [all] at once, or opens a menu of them where
/// [button] is and does the one picked.
Future<void> runCalendarChoice(
  BuildContext button,
  List<CalendarChoice> all,
) async {
  if (all.isEmpty) return;
  if (all.length == 1) return all.single.run();
  final box = button.findRenderObject() as RenderBox?;
  final overlay = Overlay.of(button).context.findRenderObject() as RenderBox?;
  if (box == null || overlay == null) return;
  final at = box.localToGlobal(box.size.bottomLeft(Offset.zero));
  final index = await showMenu<int>(
    context: button,
    position: RelativeRect.fromRect(
      overlay.globalToLocal(at) & Size.zero,
      Offset.zero & overlay.size,
    ),
    items: [
      for (final (i, choice) in all.indexed)
        PopupMenuItem(value: i, child: Text(choice.label)),
    ],
  );
  if (index == null || !button.mounted) return;
  await all[index].run();
}
