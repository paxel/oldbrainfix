import 'package:flutter/widgets.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'import_run.dart';

/// Deleting the Area [areaId] as a long job (obf-1093), its Notes going
/// where [notesGo] says, how many have gone told as it goes; whether the
/// Area went, as Abort on the page Running may stop it before its next
/// Note (obf-1078). It crosses into an isolate: it holds only the id and
/// the choice.
LongWork<bool> areaDeletion(String areaId, NotesGo notesGo) =>
    (store, _, progress, stop) async => Places(store)
        .deleteAreaStopping(areaId, notesGo, progress: progress, stop: stop);

/// How far deleting an Area is: [moved] of [of] of its Notes have gone;
/// [of] is null until they are counted.
typedef AreaDeleting = ({int moved, int? of});

/// The Areas being deleted beside the app (obf-1093), run by [runner] on
/// [store], by Area id: what their rows show, and what keeps the page
/// bar's line moving (UI laws, "The sync line"). It tells its listeners
/// when one starts, moves on or ends. Abort on the page Running stops
/// one before its next Note (obf-1078).
class AreaDeletions extends ChangeNotifier {
  final Store store;
  final JobRunner runner;

  final _running = <String, AreaDeleting>{};
  final _stops = <String, JobStop>{};
  final _stopped = <({String areaId, int moved, int of})>[];
  var _disposed = false;

  AreaDeletions({required this.store, required this.runner});

  /// How far deleting the Area [areaId] is; null while it is not being
  /// deleted.
  AreaDeleting? of(String areaId) => _running[areaId];

  /// Whether any Area is being deleted.
  bool get running => _running.isNotEmpty;

  /// The Areas being deleted, by id, in the order they began.
  List<String> get areaIds => List.unmodifiable(_running.keys);

  /// Whether Abort was pressed for deleting the Area [areaId].
  bool aborting(String areaId) => _stops[areaId]?.raised ?? false;

  /// Stops deleting the Area [areaId] before its next Note (obf-1078):
  /// the Notes gone stay where they went, the Area stays with the others.
  void abort(String areaId) {
    final stop = _stops[areaId];
    if (stop == null || stop.raised) return;
    _tell(stop.raise);
  }

  /// The deletions Abort stopped since last asked, with how many of how
  /// many Notes had gone; taking them clears them.
  List<({String areaId, int moved, int of})> takeStopped() {
    final stopped = List.of(_stopped);
    _stopped.clear();
    return stopped;
  }

  /// Deletes the Area [areaId], its Notes going where [notesGo] says;
  /// throws what deleting threw. An Area already being deleted is not
  /// started again. Whether the Area went: Abort may have stopped it.
  Future<bool> run(String areaId, NotesGo notesGo) async {
    if (_running.containsKey(areaId)) return false;
    final stop = JobStop();
    _stops[areaId] = stop;
    _tell(() => _running[areaId] = (moved: 0, of: null));
    try {
      final deleted = await runner.run(
        store,
        areaDeletion(areaId, notesGo),
        (moved, of) => _tell(() => _running[areaId] = (moved: moved, of: of)),
        stop,
      );
      final at = _running[areaId];
      if (!deleted && at != null) {
        _stopped.add((areaId: areaId, moved: at.moved, of: at.of ?? 0));
      }
      return deleted;
    } finally {
      stop.clear();
      _stops.remove(areaId);
      _tell(() => _running.remove(areaId));
    }
  }

  void _tell(void Function() change) {
    if (_disposed) return;
    change();
    notifyListeners();
  }

  @override
  void dispose() {
    _disposed = true;
    super.dispose();
  }
}

/// Hands the app's [AreaDeletions] down to the Areas list and to the page
/// bar's line.
class AreaDeletionsScope extends InheritedWidget {
  final AreaDeletions deletions;

  const AreaDeletionsScope({
    super.key,
    required this.deletions,
    required super.child,
  });

  /// The app's [AreaDeletions], or null where there are none; read once,
  /// so it does not listen.
  static AreaDeletions? of(BuildContext context) =>
      context.getInheritedWidgetOfExactType<AreaDeletionsScope>()?.deletions;

  @override
  bool updateShouldNotify(AreaDeletionsScope oldWidget) =>
      deletions != oldWidget.deletions;
}
