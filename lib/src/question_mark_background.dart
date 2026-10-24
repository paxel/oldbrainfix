import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'look.dart';

/// How much slower than the page the question marks move.
const parallaxFactor = 0.3;

/// Where the background currently sits: follows the vertical scrolling
/// of the page on top, so a page coming back keeps its own position.
/// Registered as a navigator observer (adapted from Catlog's fur).
class ParallaxScroll extends NavigatorObserver {
  /// Pixels the page on top is scrolled by.
  final offset = ValueNotifier<double>(0);
  final _offsets = <Route<dynamic>, double>{};

  void dispose() => offset.dispose();

  /// Feed to a [NotificationListener] above the navigator. Never
  /// swallows the notification.
  bool onNotification(Notification notification) {
    switch (notification) {
      case ScrollNotification(:final metrics, :final context, :final depth):
        return _track(metrics, context, depth);
      case ScrollMetricsNotification(
        :final metrics,
        :final context,
        :final depth,
      ):
        return _track(metrics, context, depth);
    }
    return false;
  }

  /// Only the page's outermost scrollable counts: a list or text field
  /// nested inside it (depth > 0) scrolls on its own.
  bool _track(ScrollMetrics metrics, BuildContext? context, int depth) {
    if (depth != 0 || metrics.axis != Axis.vertical || context == null) {
      return false;
    }
    final route = ModalRoute.of(context);
    if (route is! PageRoute) return false;
    _offsets[route] = metrics.pixels;
    if (route.isCurrent) offset.value = metrics.pixels;
    return false;
  }

  /// Shows [route]'s position if it is the page now on top.
  void _show(Route<dynamic>? route) {
    if (route is PageRoute && route.isCurrent) {
      offset.value = _offsets[route] ?? 0;
    }
  }

  @override
  void didPush(Route<dynamic> route, Route<dynamic>? previousRoute) =>
      _show(route);

  @override
  void didPop(Route<dynamic> route, Route<dynamic>? previousRoute) {
    _offsets.remove(route);
    _show(previousRoute);
  }

  @override
  void didRemove(Route<dynamic> route, Route<dynamic>? previousRoute) {
    _offsets.remove(route);
    _show(previousRoute);
  }

  @override
  void didReplace({Route<dynamic>? newRoute, Route<dynamic>? oldRoute}) {
    _offsets.remove(oldRoute);
    _show(newRoute);
  }
}

/// The ground of every page: hand-drawn question marks in slow parallax.
/// The parallax stops when the system asks to remove animations.
class QuestionMarkBackground extends StatelessWidget {
  final ParallaxScroll scroll;
  final Widget child;

  const QuestionMarkBackground({
    super.key,
    required this.scroll,
    required this.child,
  });

  @override
  Widget build(BuildContext context) => NotificationListener<Notification>(
    onNotification: scroll.onNotification,
    child: Stack(
      children: [
        Positioned.fill(
          child: RepaintBoundary(
            child: CustomPaint(
              painter: QuestionMarkPainter(
                tones: LookTones.of(Theme.of(context).brightness),
                scroll: scroll.offset,
                still: MediaQuery.disableAnimationsOf(context),
              ),
            ),
          ),
        ),
        child,
      ],
    ),
  );
}

/// Draws the ground and the marks of [questionMarksIn], shifted by
/// [shift].
class QuestionMarkPainter extends CustomPainter {
  final LookTones tones;

  /// The page's scroll offset; repaints on change.
  final ValueListenable<double> scroll;

  /// No parallax: the system's "remove animations" is on.
  final bool still;

  QuestionMarkPainter({
    required this.tones,
    required this.scroll,
    required this.still,
  }) : super(repaint: scroll);

  /// Pixels the plane is moved up by now.
  double get shift => still ? 0 : scroll.value * parallaxFactor;

  /// Each shape's path, built once, in its [questionMarkBox].
  static final List<({Path strokes, Path dot})> _paths = [
    for (final shape in questionMarkShapes) _pathsOf(shape),
  ];

  static ({Path strokes, Path dot}) _pathsOf(QuestionMarkShape shape) {
    final strokes = Path();
    for (final s in shape.strokes) {
      strokes.moveTo(s[0], s[1]);
      for (var i = 2; i + 3 < s.length; i += 4) {
        strokes.quadraticBezierTo(s[i], s[i + 1], s[i + 2], s[i + 3]);
      }
    }
    final dot = Path();
    final d = shape.dot;
    if (d != null) {
      dot.addOval(Rect.fromCircle(center: Offset(d[0], d[1]), radius: d[2]));
    }
    return (strokes: strokes, dot: dot);
  }

  @override
  void paint(Canvas canvas, Size size) {
    canvas.drawRect(Offset.zero & size, Paint()..color = tones.background);
    final top = shift;
    const pad = questionMarkMaxSize;
    final fill = Paint()..color = tones.mark;
    for (final mark in questionMarksIn(
      -pad,
      top - pad,
      size.width + pad,
      top + size.height + pad,
    )) {
      final shape = questionMarkShapes[mark.shape];
      final paths = _paths[mark.shape];
      final scale = mark.size / questionMarkBox;
      canvas
        ..save()
        ..translate(mark.x, mark.y - top)
        ..rotate(mark.rotation)
        ..scale(scale)
        ..translate(-questionMarkBox / 2, -questionMarkBox / 2)
        ..drawPath(
          paths.strokes,
          Paint()
            ..color = tones.mark
            ..style = PaintingStyle.stroke
            ..strokeWidth = shape.weight
            ..strokeCap = StrokeCap.round
            ..strokeJoin = StrokeJoin.round,
        )
        ..drawPath(paths.dot, fill)
        ..restore();
    }
  }

  @override
  bool shouldRepaint(QuestionMarkPainter old) =>
      old.tones != tones || old.still != still || old.scroll != scroll;
}
