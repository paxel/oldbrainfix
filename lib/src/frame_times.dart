import 'dart:ui' show FrameTiming, FramePhase;

import 'package:flutter/material.dart';
import 'package:flutter/scheduler.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'l10n.dart';

/// Whether this Device shows how long its frames take, switched by
/// tapping the version under About seven times (#258); off until
/// switched on.
class FrameTimesSwitch extends ValueNotifier<bool> {
  static const _setting = 'frameTimes';

  final Store store;

  FrameTimesSwitch(this.store) : super(store.localSetting(_setting) == 'true');

  /// Switches the frame times on or off, and keeps it for this Device.
  void toggle() {
    value = !value;
    // Nothing kept means off, the default.
    value
        ? store.setLocalSetting(_setting, 'true')
        : store.removeLocalSetting(_setting);
  }
}

/// Where the About page finds the frame times' switch.
class FrameTimesScope extends InheritedWidget {
  final FrameTimesSwitch frameTimes;

  const FrameTimesScope({
    super.key,
    required this.frameTimes,
    required super.child,
  });

  static FrameTimesSwitch? maybeOf(BuildContext context) =>
      context.getInheritedWidgetOfExactType<FrameTimesScope>()?.frameTimes;

  @override
  bool updateShouldNotify(FrameTimesScope oldWidget) =>
      frameTimes != oldWidget.frameTimes;
}

/// The times of the frames lately drawn, as the overlay shows them: the
/// slowest build and raster since it last changed, at most every half
/// second, so its own frames stay few, and the slowest frame of the
/// last second.
class FrameTimesWindow {
  static const _second = 1000000;
  static const _refresh = _second ~/ 2;

  /// Each frame of the last second: when it started, and its total.
  final _recent = <({int start, Duration total})>[];
  var _shownAt = -_second;
  var _build = Duration.zero;
  var _raster = Duration.zero;

  /// The slowest build of the last frames.
  Duration lastBuild = Duration.zero;

  /// The slowest raster of the last frames.
  Duration lastRaster = Duration.zero;

  /// The slowest frame of the last second, start to end.
  Duration worst = Duration.zero;

  /// Takes in [timings]; whether what shows changed.
  bool add(List<FrameTiming> timings) {
    if (timings.isEmpty) return false;
    var newest = _shownAt;
    for (final timing in timings) {
      final start = timing.timestampInMicroseconds(FramePhase.vsyncStart);
      if (start > newest) newest = start;
      if (timing.buildDuration > _build) _build = timing.buildDuration;
      if (timing.rasterDuration > _raster) _raster = timing.rasterDuration;
      _recent.add((start: start, total: timing.totalSpan));
    }
    _recent.removeWhere((frame) => frame.start <= newest - _second);
    if (newest - _shownAt < _refresh) return false;
    _shownAt = newest;
    final before = (lastBuild, lastRaster, worst);
    lastBuild = _build;
    lastRaster = _raster;
    _build = Duration.zero;
    _raster = Duration.zero;
    worst = Duration.zero;
    for (final frame in _recent) {
      if (frame.total > worst) worst = frame.total;
    }
    return (lastBuild, lastRaster, worst) != before;
  }
}

/// [child], and while [frameTimes] is on, a small box at the bottom
/// left with the times of the last frames (#258).
class FrameTimesOverlay extends StatelessWidget {
  final FrameTimesSwitch frameTimes;
  final Widget child;

  const FrameTimesOverlay({
    super.key,
    required this.frameTimes,
    required this.child,
  });

  @override
  Widget build(BuildContext context) => Stack(
    fit: StackFit.expand,
    children: [
      child,
      ValueListenableBuilder<bool>(
        valueListenable: frameTimes,
        builder: (context, on, _) => on
            ? Positioned(
                left: 4,
                bottom: MediaQuery.paddingOf(context).bottom + 4,
                child: const IgnorePointer(
                  child: ExcludeSemantics(child: _FrameTimes()),
                ),
              )
            : const SizedBox.shrink(),
      ),
    ],
  );
}

class _FrameTimes extends StatefulWidget {
  const _FrameTimes();

  @override
  State<_FrameTimes> createState() => _FrameTimesState();
}

class _FrameTimesState extends State<_FrameTimes> {
  final _window = FrameTimesWindow();

  @override
  void initState() {
    super.initState();
    SchedulerBinding.instance.addTimingsCallback(_took);
  }

  @override
  void dispose() {
    SchedulerBinding.instance.removeTimingsCallback(_took);
    super.dispose();
  }

  void _took(List<FrameTiming> timings) {
    if (_window.add(timings) && mounted) setState(() {});
  }

  static String _ms(Duration d) => (d.inMicroseconds / 1000).toStringAsFixed(1);

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return DecoratedBox(
      key: const ValueKey('frameTimes'),
      decoration: BoxDecoration(
        color: const Color(0xCC000000),
        borderRadius: BorderRadius.circular(4),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 3),
        child: DefaultTextStyle(
          style: const TextStyle(
            color: Color(0xFFFFFFFF),
            fontSize: 11,
            fontFeatures: [FontFeature.tabularFigures()],
          ),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                t.frameTimesLast(
                  _ms(_window.lastBuild),
                  _ms(_window.lastRaster),
                ),
              ),
              Text(t.frameTimesWorst(_ms(_window.worst))),
            ],
          ),
        ),
      ),
    );
  }
}
