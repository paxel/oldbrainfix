import 'package:oldbrainfix/src/calendar_platform.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// A [CalendarPlatform] the test drives: what was added and shared, and
/// whether a calendar app or a share target is there to take it.
class FakeCalendar implements CalendarPlatform {
  /// Whether a calendar app takes inserts.
  bool hasCalendarApp;

  /// Whether some app takes shared files.
  bool hasShareTarget;

  /// The events handed to the calendar app, in order.
  final inserted = <CalendarEvent>[];

  /// The files shared, in order, as (name, content).
  final shared = <(String, String)>[];

  FakeCalendar({this.hasCalendarApp = true, this.hasShareTarget = true});

  @override
  bool get available => true;

  @override
  Future<bool> insert(CalendarEvent event) async {
    inserted.add(event);
    return hasCalendarApp;
  }

  @override
  Future<bool> share(String name, String ics) async {
    shared.add((name, ics));
    return hasShareTarget;
  }
}
