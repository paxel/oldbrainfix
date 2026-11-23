import 'package:flutter/widgets.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'import_run.dart';

/// Deleting the Area [areaId] as a long job (obf-1093), its Notes going
/// where [notesGo] says, how many have gone told as it goes. It crosses
/// into an isolate: it holds only the id and the choice.
LongWork<void> areaDeletion(String areaId, NotesGo notesGo) =>
    (store, _, progress) async =>
        Places(store).deleteArea(areaId, notesGo, progress: progress);

/// How far deleting an Area is: [moved] of [of] of its Notes have gone;
/// [of] is null until they are counted.
typedef AreaDeleting = ({int moved, int? of});

/// The Areas being deleted beside the app (obf-1093), run by [runner] on
/// [store], by Area id: what their rows show, and what keeps the page
/// bar's line moving (UI laws, "The sync line"). It tells its listeners
/// when one starts, moves on or ends.
class AreaDeletions extends ChangeNotifier {
  final Store store;
  final JobRunner runner;

  final _running = <String, AreaDeleting>{};
  var _disposed = false;

  AreaDeletions({required this.store, required this.runner});

  /// How far deleting the Area [areaId] is; null while it is not being
  /// deleted.
  AreaDeleting? of(String areaId) => _running[areaId];

  /// Whether any Area is being deleted.
  bool get running => _running.isNotEmpty;

  /// Deletes the Area [areaId], its Notes going where [notesGo] says;
  /// throws what deleting threw. An Area already being deleted is not
  /// started again.
  Future<void> run(String areaId, NotesGo notesGo) async {
    if (_running.containsKey(areaId)) return;
    _tell(() => _running[areaId] = (moved: 0, of: null));
    try {
      await runner.run(
        store,
        areaDeletion(areaId, notesGo),
        (moved, of) => _tell(() => _running[areaId] = (moved: moved, of: of)),
      );
    } finally {
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
