import 'package:flutter/material.dart';

import 'area_deletion.dart';
import 'auto_sync.dart';
import 'import_run.dart';

/// How thick the line is.
const syncLineHeight = 3.0;

/// How long the line takes from one side to the other.
const syncLineSweep = Duration(milliseconds: 900);

/// The line at the bottom of a page's bar while a sync of this Device
/// runs (#225, UI laws "The sync line"), while an Import does
/// (obf-1061) and while an Area is being deleted (obf-1093): a short
/// stretch in the accent
/// moving back and forth, the whole width standing still while the
/// system's "remove animations" setting is on. It takes no room of its
/// own and nothing else on the page changes; nothing without the app's
/// [AutoSyncScope], [ImportJobsScope] and [AreaDeletionsScope]. Screen
/// readers hear of the sync from the app, once, not from every page's
/// bar.
class SyncLine extends StatelessWidget {
  const SyncLine({super.key});

  @override
  Widget build(BuildContext context) {
    final syncing = AutoSyncScope.of(context)?.syncing;
    final importing = ImportJobsScope.of(context)?.running;
    final deleting = AreaDeletionsScope.of(context);
    if (syncing == null && importing == null && deleting == null) {
      return const SizedBox.shrink();
    }
    return ListenableBuilder(
      listenable: Listenable.merge([?syncing, ?importing, ?deleting]),
      builder: (context, _) =>
          (syncing?.value ?? false) ||
              importing?.value != null ||
              (deleting?.running ?? false)
          ? const Align(
              alignment: Alignment.bottomCenter,
              child: ExcludeSemantics(
                child: SizedBox(
                  height: syncLineHeight,
                  width: double.infinity,
                  child: _Sweep(),
                ),
              ),
            )
          : const SizedBox.shrink(),
    );
  }
}

class _Sweep extends StatefulWidget {
  const _Sweep();

  @override
  State<_Sweep> createState() => _SweepState();
}

class _SweepState extends State<_Sweep> with SingleTickerProviderStateMixin {
  late final _moving = AnimationController(
    vsync: this,
    duration: syncLineSweep,
  );

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (MediaQuery.disableAnimationsOf(context)) {
      _moving.stop();
    } else if (!_moving.isAnimating) {
      _moving.repeat(reverse: true);
    }
  }

  @override
  void dispose() {
    _moving.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final accent = Theme.of(context).colorScheme.primary;
    if (MediaQuery.disableAnimationsOf(context)) {
      return ColoredBox(key: const Key('syncLineStill'), color: accent);
    }
    return AnimatedBuilder(
      animation: _moving,
      builder: (context, _) => Align(
        alignment: Alignment(
          Curves.easeInOut.transform(_moving.value) * 2 - 1,
          0,
        ),
        child: FractionallySizedBox(
          key: const Key('syncLineMoving'),
          widthFactor: 0.25,
          heightFactor: 1,
          child: ColoredBox(color: accent),
        ),
      ),
    );
  }
}
