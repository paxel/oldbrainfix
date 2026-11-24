import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// The local setting of "End a recording after a pause" (obf-1053): the
/// pause's seconds, one of [recordingPauseChoices]; absent while it is
/// Off, as by default, and pauses are ignored. Kept on this Device only.
const recordingPauseKey = 'recordingPauseSeconds';

/// The pauses offered, in seconds, besides Off.
const recordingPauseChoices = [2, 3, 5, 10];

/// The pause after which a recording on [store]'s Device ends on its
/// own; null while it is Off, also for anything unknown kept.
Duration? recordingPause(Store store) {
  final seconds = int.tryParse(store.localSetting(recordingPauseKey) ?? '');
  if (seconds == null || !recordingPauseChoices.contains(seconds)) return null;
  return Duration(seconds: seconds);
}

/// Keeps [seconds], one of [recordingPauseChoices], as the pause on
/// [store]'s Device; null turns it Off.
void setRecordingPause(Store store, int? seconds) {
  if (seconds == null) return store.removeLocalSetting(recordingPauseKey);
  if (!recordingPauseChoices.contains(seconds)) {
    throw ArgumentError.value(seconds, 'seconds', 'not a pause offered');
  }
  store.setLocalSetting(recordingPauseKey, '$seconds');
}
