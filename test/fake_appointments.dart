import 'package:oldbrainfix/src/appointments.dart';

/// An [AppointmentSource] the test drives: the Device's calendars and
/// their appointments, the mirror's own marked as such, and calendar
/// access to allow or refuse.
class FakeAppointments implements AppointmentSource {
  /// Whether reading calendars is allowed.
  bool allowed;

  /// What the user answers when asked for calendar access.
  bool allowOnRequest;

  /// How often the user was asked, and how often appointments were read.
  var requests = 0;
  var reads = 0;

  /// The calendars on the Device.
  final List<ReadCalendar> deviceCalendars;

  /// Every appointment in every calendar, with whether the calendar
  /// mirror wrote it.
  final events = <({Appointment appointment, bool mirrored})>[];

  FakeAppointments({
    this.allowed = false,
    this.allowOnRequest = true,
    List<ReadCalendar>? calendars,
  }) : deviceCalendars =
           calendars ??
           [
             (
               id: '1',
               name: 'Personal',
               account: 'me@example.org',
               colour: 0xFF3366CC,
             ),
             (
               id: '2',
               name: 'Work',
               account: 'me@example.org',
               colour: 0x00CC6633,
             ),
           ];

  /// Adds [appointment]; [mirrored] for one the calendar mirror wrote.
  void add(Appointment appointment, {bool mirrored = false}) =>
      events.add((appointment: appointment, mirrored: mirrored));

  @override
  bool get available => true;

  @override
  Future<bool> granted() async => allowed;

  @override
  Future<bool> request() async {
    requests++;
    allowed = allowOnRequest;
    return allowed;
  }

  @override
  Future<List<ReadCalendar>> calendars() async =>
      allowed ? deviceCalendars : const [];

  /// As the platform answers: the picked calendars' appointments in the
  /// range, the mirror's own left out (android/…/Appointments.kt).
  @override
  Future<List<Appointment>?> appointments(
    DateTime from,
    DateTime until,
    Set<String> calendarIds,
  ) async {
    reads++;
    if (!allowed) return null;
    return [
      for (final (:appointment, :mirrored) in events)
        if (!mirrored &&
            calendarIds.contains(appointment.calendarId) &&
            appointment.end.isAfter(from) &&
            appointment.begin.isBefore(until))
          appointment,
    ];
  }
}
