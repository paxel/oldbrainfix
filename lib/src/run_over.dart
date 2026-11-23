import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// The stores whose run of the app is over: the crash guard's Restart
/// stopped that run and closes them (obf-1059).
final _over = Expando<bool>();

/// Whether the run of the app on [store] is over; the screens of that
/// run, which may never be taken down, read nothing from it then.
bool runOver(Store store) => _over[store] ?? false;

/// Ends the run of the app on [store], before Restart closes it.
void endRun(Store store) => _over[store] = true;
