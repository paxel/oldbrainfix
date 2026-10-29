import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'l10n.dart';
import 'schedule_text.dart';

/// A calendar on the Device whose appointments the Agenda can show, with
/// its colour as ARGB.
typedef ReadCalendar = ({String id, String name, String account, int colour});

/// One appointment of a Device calendar, read for the Agenda and never
/// kept: an all-day one runs from midnight UTC to midnight UTC, as the
/// calendar keeps those.
class Appointment {
  /// The calendar's own id of this one occurrence of the event.
  final String id;
  final String calendarId;
  final String title;
  final DateTime begin;
  final DateTime end;
  final bool allDay;
  final String location;
  final String description;

  const Appointment({
    required this.id,
    required this.calendarId,
    required this.title,
    required this.begin,
    required this.end,
    required this.allDay,
    this.location = '',
    this.description = '',
  });
}

/// The platform seam for showing the calendar's appointments on the
/// Agenda: the calendar permission of the Capture Context, the
/// calendars to pick from, and their appointments, without the calendar
/// mirror's own (android/…/Appointments.kt). Widget tests hand in a
/// fake.
abstract interface class AppointmentSource {
  /// Whether this Device has calendars to read; the switch only shows
  /// where it has.
  bool get available;

  /// Whether the user allowed reading calendars. Never asks.
  Future<bool> granted();

  /// Asks the user to allow reading calendars; whether it is allowed now.
  Future<bool> request();

  /// The calendars the calendar app shows; none when that is not
  /// allowed.
  Future<List<ReadCalendar>> calendars();

  /// The appointments of [calendarIds] overlapping [from] until [until],
  /// without the events the calendar mirror wrote; null when that is not
  /// allowed or not known. Never asks.
  Future<List<Appointment>?> appointments(
    DateTime from,
    DateTime until,
    Set<String> calendarIds,
  );
}

/// No calendar: the default where none is handed in.
class NoAppointments implements AppointmentSource {
  const NoAppointments();

  @override
  bool get available => false;

  @override
  Future<bool> granted() async => false;

  @override
  Future<bool> request() async => false;

  @override
  Future<List<ReadCalendar>> calendars() async => const [];

  @override
  Future<List<Appointment>?> appointments(
    DateTime from,
    DateTime until,
    Set<String> calendarIds,
  ) async => null;
}

/// Android's calendar provider behind the app's method channels; the
/// permission is the one the running appointment at Capture asks for.
class AndroidAppointments implements AppointmentSource {
  static const _context = MethodChannel('io.github.paxel.oldbrainfix/context');
  static const _calendar = MethodChannel(
    'io.github.paxel.oldbrainfix/calendar',
  );

  const AndroidAppointments();

  @override
  bool get available => true;

  @override
  Future<bool> granted() async =>
      await _call<bool>(_context, 'granted', 'calendar') ?? false;

  @override
  Future<bool> request() async =>
      await _call<bool>(_context, 'request', 'calendar') ?? false;

  @override
  Future<List<ReadCalendar>> calendars() async {
    final rows =
        await _call<List<Object?>>(_calendar, 'readCalendars', null) ??
        const [];
    return [
      for (final row in rows)
        if (row case {
          'id': final String id,
          'name': final String name,
          'account': final String account,
          'colour': final int colour,
        })
          (id: id, name: name, account: account, colour: colour),
    ];
  }

  @override
  Future<List<Appointment>?> appointments(
    DateTime from,
    DateTime until,
    Set<String> calendarIds,
  ) async {
    final rows = await _call<List<Object?>>(_calendar, 'appointments', {
      'from': from.millisecondsSinceEpoch,
      'until': until.millisecondsSinceEpoch,
      'calendars': [...calendarIds],
    });
    if (rows == null) return null;
    return [
      for (final row in rows)
        if (row case {
          'id': final String id,
          'calendar': final String calendarId,
          'title': final String title,
          'begin': final int begin,
          'end': final int end,
          'allDay': final bool allDay,
          'location': final String location,
          'description': final String description,
        })
          Appointment(
            id: id,
            calendarId: calendarId,
            title: title,
            begin: DateTime.fromMillisecondsSinceEpoch(begin, isUtc: true),
            end: DateTime.fromMillisecondsSinceEpoch(end, isUtc: true),
            allDay: allDay,
            location: location,
            description: description,
          ),
    ];
  }

  static Future<T?> _call<T>(
    MethodChannel channel,
    String method,
    Object? args,
  ) async {
    try {
      return await channel.invokeMethod<T>(method, args);
    } on PlatformException {
      return null;
    }
  }
}

/// Hands the app's [AppointmentSource] down to every screen.
class AppointmentScope extends InheritedWidget {
  final AppointmentSource source;

  const AppointmentScope({
    super.key,
    required this.source,
    required super.child,
  });

  /// The app's [AppointmentSource]; read once, so it does not listen.
  static AppointmentSource of(BuildContext context) =>
      context.getInheritedWidgetOfExactType<AppointmentScope>()?.source ??
      const NoAppointments();

  @override
  bool updateShouldNotify(AppointmentScope oldWidget) =>
      source != oldWidget.source;
}

/// A calendar's colour, opaque: some calendars keep theirs without
/// alpha.
Color calendarColour(int argb) => Color(argb | 0xFF000000);

/// The part of an [appointment] that falls on one day of the Agenda:
/// from when (null when it began before the day) until when (null when
/// it runs past it); neither for an all-day one or one that fills the
/// whole day.
class AppointmentPiece {
  final Appointment appointment;
  final Day day;
  final ClockTime? from;
  final ClockTime? until;

  /// Whether it shows among the all-day Occurrences: an all-day
  /// appointment, or a timed one that fills the whole day.
  final bool allDay;

  const AppointmentPiece({
    required this.appointment,
    required this.day,
    required this.from,
    required this.until,
    required this.allDay,
  });

  /// Where it sorts among the timed Occurrences: its start, or the
  /// day's beginning when it began before; null among the all-day ones.
  ClockTime? get at => allDay ? null : from ?? ClockTime(0, 0);

  /// Its key on the Agenda, one per appointment and day.
  String get key => '${appointment.id}@$day';
}

/// The parts of [appointments] on [day], as [wallClock] shows days: the
/// all-day ones first, then by time, each by title after that.
List<AppointmentPiece> appointmentsOn(
  Day day,
  Iterable<Appointment> appointments,
  WallClock wallClock,
) {
  final pieces = <AppointmentPiece>[];
  for (final appointment in appointments) {
    if (appointment.allDay) {
      // Midnight UTC to midnight UTC, the end not counted.
      final first = Day.of(appointment.begin.toUtc());
      final end = Day.of(appointment.end.toUtc());
      final last = end.compareTo(first) > 0 ? end.addDays(-1) : first;
      if (day.compareTo(first) < 0 || day.compareTo(last) > 0) continue;
      pieces.add(
        AppointmentPiece(
          appointment: appointment,
          day: day,
          from: null,
          until: null,
          allDay: true,
        ),
      );
      continue;
    }
    final begin = wallClock(appointment.begin.toUtc());
    final end = wallClock(appointment.end.toUtc());
    final first = Day.of(begin);
    final endDay = Day.of(end);
    // An end at midnight closes the day before.
    final endsAtMidnight =
        end.hour == 0 && end.minute == 0 && endDay.compareTo(first) > 0;
    final last = endsAtMidnight ? endDay.addDays(-1) : endDay;
    if (day.compareTo(first) < 0 || day.compareTo(last) > 0) continue;
    final from = day == first ? ClockTime(begin.hour, begin.minute) : null;
    final until = day == endDay ? ClockTime(end.hour, end.minute) : null;
    pieces.add(
      AppointmentPiece(
        appointment: appointment,
        day: day,
        from: from,
        until: until,
        allDay: from == null && until == null,
      ),
    );
  }
  pieces.sort((a, b) {
    final x = a.at;
    final y = b.at;
    if (x == null || y == null) {
      final byKind = (x == null ? 0 : 1).compareTo(y == null ? 0 : 1);
      if (byKind != 0) return byKind;
    } else {
      final byTime = x.compareTo(y);
      if (byTime != 0) return byTime;
    }
    return a.appointment.title.compareTo(b.appointment.title);
  });
  return pieces;
}

/// When [piece] runs on its day, in words: "All day", "14:30–15:30",
/// "from 22:00" or "until 10:00".
String pieceTime(AppLocalizations t, String locale, AppointmentPiece piece) =>
    switch ((piece.from, piece.until)) {
      _ when piece.allDay => t.allDay,
      (final from?, final until?) when from == until => clockText(locale, from),
      (final from?, final until?) =>
        '${clockText(locale, from)}–${clockText(locale, until)}',
      (final from?, null) => t.appointmentSince(clockText(locale, from)),
      (null, final until?) => t.appointmentUntil(clockText(locale, until)),
      (null, null) => t.allDay,
    };

/// [appointment]'s title, or a word for one without.
String appointmentTitle(AppLocalizations t, Appointment appointment) =>
    appointment.title.trim().isEmpty
    ? t.appointmentNoTitle
    : appointment.title.trim();

/// The text of the Note "Copy into a Note" makes of [appointment]: its
/// title, when it runs, its place, and after a blank line its
/// description, each only when there is one.
String appointmentNoteText(
  AppLocalizations t,
  String locale,
  Appointment appointment,
  WallClock wallClock,
) {
  String when() {
    if (appointment.allDay) {
      final first = Day.of(appointment.begin.toUtc());
      final end = Day.of(appointment.end.toUtc());
      final last = end.compareTo(first) > 0 ? end.addDays(-1) : first;
      return last == first
          ? '${dayText(locale, first)}, ${t.allDay}'
          : '${dayText(locale, first)} – ${dayText(locale, last)}';
    }
    final begin = wallClock(appointment.begin.toUtc());
    final end = wallClock(appointment.end.toUtc());
    final from = clockText(locale, ClockTime(begin.hour, begin.minute));
    final until = clockText(locale, ClockTime(end.hour, end.minute));
    final first = Day.of(begin);
    if (Day.of(end) == first) {
      return '${dayText(locale, first)}, '
          '${from == until ? from : '$from–$until'}';
    }
    return '${dayText(locale, first)} $from – '
        '${dayText(locale, Day.of(end))} $until';
  }

  final location = appointment.location.trim();
  final description = appointment.description.trim();
  return [
    appointmentTitle(t, appointment),
    when(),
    if (location.isNotEmpty) t.appointmentPlace(location),
    if (description.isNotEmpty) ...['', description],
  ].join('\n');
}

/// Explains that the Agenda shows the appointments of the calendars
/// picked and keeps no copy, then asks for calendar access when the user
/// agrees. Returns whether it is allowed now.
Future<bool> explainAndAskAppointments(
  BuildContext context,
  AppointmentSource source,
) async {
  final agreed = await showDialog<bool>(
    context: context,
    builder: (context) {
      final t = context.t;
      return AlertDialog(
        icon: const Icon(Icons.event_note_outlined),
        title: Text(t.agendaCalendarExplainTitle),
        content: Text(t.agendaCalendarExplainBody),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: Text(t.notNow),
          ),
          TextButton(
            onPressed: () => Navigator.of(context).pop(true),
            child: Text(t.allowAccess),
          ),
        ],
      );
    },
  );
  if (agreed != true) return false;
  return source.request();
}
