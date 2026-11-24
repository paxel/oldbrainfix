import 'package:flutter/material.dart';

import 'area_deletion.dart';
import 'auto_sync.dart';
import 'import_run.dart';
import 'running.dart';

/// How thick the line is.
const syncLineHeight = 3.0;

/// How high the room over the line is that a tap opens the page Running
/// in (obf-1078): the line itself is too thin to hit.
const syncLineTapHeight = 16.0;

/// How long the line takes from one side to the other.
const syncLineSweep = Duration(milliseconds: 900);

/// The line at the bottom of a page's bar while a sync of this Device
/// runs (#225, UI laws "The sync line"), while an Import does
/// (obf-1061) and while an Area is being deleted (obf-1093): a short
/// stretch in the accent
/// moving back and forth, the whole width standing still while the
/// system's "remove animations" setting is on. It takes no room of its
/// own and nothing else on the page changes; nothing without the app's
/// [AutoSyncScope], [ImportJobsScope] and [AreaDeletionsScope]. A tap on
/// it, or a little above it, opens the page Running (obf-1078); the
/// bar's buttons over it keep their taps. Screen readers hear of the sync
/// from the app, once, not from every page's bar, and reach the page
/// through ⋮.
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
              child: _OpensRunning(
                top: syncLineTapHeight - syncLineHeight,
                bottom: 0,
                child: ExcludeSemantics(
                  child: SizedBox(
                    height: syncLineHeight,
                    width: double.infinity,
                    child: _Sweep(),
                  ),
                ),
              ),
            )
          : const SizedBox.shrink(),
    );
  }
}

/// The sync line's moving stretch on its own, 3 px across the room it is
/// given, for work shown where it happens rather than under the bar: a
/// voice recording being transcribed on the quick note (obf-1082). A tap
/// on it, or a little above or below it, opens the page Running
/// (obf-1078).
class SweepLine extends StatelessWidget {
  const SweepLine({super.key});

  @override
  Widget build(BuildContext context) => const _OpensRunning(
    top: 6,
    bottom: 6,
    child: ExcludeSemantics(
      child: SizedBox(
        height: syncLineHeight,
        width: double.infinity,
        child: _Sweep(),
      ),
    ),
  );
}

/// [child] with room [top] above and [bottom] below it in which a tap
/// opens the page Running (obf-1078), where the app has one; the room
/// stays without it, so the line sits where it does either way.
class _OpensRunning extends StatelessWidget {
  final double top;
  final double bottom;
  final Widget child;

  const _OpensRunning({
    required this.top,
    required this.bottom,
    required this.child,
  });

  @override
  Widget build(BuildContext context) {
    final padded = Padding(
      padding: EdgeInsets.only(top: top, bottom: bottom),
      child: child,
    );
    if (RunningScope.of(context) == null) return padded;
    return GestureDetector(
      key: const Key('syncLineTap'),
      behavior: HitTestBehavior.opaque,
      // Out of the screen reader's way, which reaches it through ⋮.
      excludeFromSemantics: true,
      onTap: () => openRunning(context),
      child: padded,
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
