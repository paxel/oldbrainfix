import 'package:flutter/widgets.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'folder_platform.dart';
import 'import_run.dart';

// Top-level, so what crosses into the isolate holds nothing but the
// store it is handed there.
Future<TrashEmptied> _toEmpty(
  Store store,
  FolderPlatform _,
  JobProgress _,
  Stopping _,
) async => Trash(store).toEmpty();

Future<TrashEmptied> _empty(
  Store store,
  FolderPlatform _,
  JobProgress _,
  Stopping _,
) async => Trash(store).empty();

/// "Empty the trash now" of the Trash (obf-1095), run by [runner] on
/// [store] beside the app: [toEmpty] counts what it removes for the
/// question, [run] removes it, which keeps the page bar's line moving
/// meanwhile (UI laws, "The sync line"). It purges in small batches
/// and offers no Abort on the page Running; it tells its listeners when
/// it starts and ends.
class TrashEmptying extends ChangeNotifier {
  final Store store;
  final JobRunner runner;

  var _running = false;
  var _disposed = false;

  TrashEmptying({required this.store, required this.runner});

  /// Whether the trash is being emptied.
  bool get running => _running;

  /// What [run] removes now, counted beside the app: it reads every Note
  /// in the trash.
  Future<TrashEmptied> toEmpty() => _job(_toEmpty);

  /// Empties the trash; throws what emptying threw. Not started again
  /// while it runs: then it removes nothing.
  Future<TrashEmptied> run() async {
    if (_running) return const TrashEmptied();
    _tell(() => _running = true);
    try {
      return await _job(_empty);
    } finally {
      _tell(() => _running = false);
    }
  }

  Future<TrashEmptied> _job(LongWork<TrashEmptied> work) async {
    final stop = JobStop();
    try {
      return await runner.run(store, work, (_, _) {}, stop);
    } finally {
      stop.clear();
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

/// Hands the app's [TrashEmptying] down to the Trash, the page bar's line
/// and the page Running.
class TrashEmptyingScope extends InheritedWidget {
  final TrashEmptying emptying;

  const TrashEmptyingScope({
    super.key,
    required this.emptying,
    required super.child,
  });

  /// The app's [TrashEmptying], or null where there is none; read once,
  /// so it does not listen.
  static TrashEmptying? of(BuildContext context) =>
      context.getInheritedWidgetOfExactType<TrashEmptyingScope>()?.emptying;

  @override
  bool updateShouldNotify(TrashEmptyingScope oldWidget) =>
      emptying != oldWidget.emptying;
}
