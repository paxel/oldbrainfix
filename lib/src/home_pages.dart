import 'dart:async';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';

/// Which of the app's two first pages shows, the Agenda or the Areas
/// list, and the way between them (#258).
class HomePager {
  static const _duration = Duration(milliseconds: 300);
  static const _curve = Curves.easeInOutCubic;

  /// The page the Agenda is.
  static const agenda = 0;

  /// The page the Areas list is.
  static const areas = 1;

  final controller = PageController();

  /// The page the pages came to rest on; it changes only once a swipe,
  /// the folder or back has finished, never in the middle.
  final shown = ValueNotifier<int>(agenda);

  /// Whether the system's animations are off: then the pages change at
  /// once.
  var still = false;

  /// The page the folder, back or the lock moves to, until it is there.
  int? _goingTo;

  void dispose() {
    controller.dispose();
    shown.dispose();
  }

  /// Goes to the Areas list, as the folder does.
  void showAreas() => _show(areas);

  /// Goes back to the Agenda, as back on the Areas list does.
  void showAgenda() => _show(agenda);

  /// Shows the Agenda at once, as when the app locks.
  void jumpToAgenda() => _jump(agenda);

  void _jump(int page) {
    if (!controller.hasClients || controller.page == page) return;
    _goingTo = page;
    controller.jumpToPage(page);
  }

  void _show(int page) {
    if (!controller.hasClients || controller.page == page) return;
    if (still) return _jump(page);
    _goingTo = page;
    // The folder or back: nothing waits for it.
    unawaited(
      controller.animateToPage(page, duration: _duration, curve: _curve),
    );
  }
}

/// The app's first page: the Agenda and the Areas list side by side, one
/// swipe apart, with the platform's own start and fling of a drag
/// (#258). Both are built from the start and kept, so a swipe builds
/// neither. Back on the Areas list, by the system or its bar, goes to
/// the Agenda. On iOS a drag from the left edge moves neither (Q570).
class HomePages extends StatefulWidget {
  final HomePager pager;
  final Widget agenda;
  final Widget areas;

  const HomePages({
    super.key,
    required this.pager,
    required this.agenda,
    required this.areas,
  });

  @override
  State<HomePages> createState() => _HomePagesState();
}

class _HomePagesState extends State<HomePages> {
  /// How wide the left edge is that iOS keeps for its back swipe.
  static const _edge = 20.0;

  /// Fingers on the pages now.
  var _fingers = 0;

  /// Whether a finger moved the pages since they last came to rest.
  var _byFinger = false;

  bool _scrolled(ScrollNotification scroll) {
    if (scroll.depth != 0) return false;
    switch (scroll) {
      case ScrollStartNotification(:final dragDetails):
        // Also the glide after a finger caught the pages and let go.
        if (dragDetails != null || _fingers > 0) _byFinger = true;
      case ScrollEndNotification(:final metrics):
        _rested(metrics);
    }
    return false;
  }

  /// Only a swipe, the folder, back and the lock change the page shown.
  void _rested(ScrollMetrics metrics) {
    // A finger caught the pages: they rest only once it lets go.
    if (_fingers > 0) return;
    if (metrics case PageMetrics(:final page?)) {
      final pager = widget.pager;
      final at = page.round();
      if (_byFinger) {
        _byFinger = false;
        pager._goingTo = null;
      } else if (pager._goingTo case final target?) {
        // Stopped on the way, to go on from there.
        if (at != target) return;
        pager._goingTo = null;
      } else {
        // Something else moved the pages, such as revealing what sits on
        // the far side of the page shown (Scrollable.ensureVisible): they
        // go back.
        if (at != pager.shown.value) {
          WidgetsBinding.instance.addPostFrameCallback((_) {
            if (mounted) pager._jump(pager.shown.value);
          });
        }
        return;
      }
      if (at != pager.shown.value) {
        // The search field of a page no longer shown keeps no keyboard.
        FocusManager.instance.primaryFocus?.unfocus();
      }
      pager.shown.value = at;
    }
  }

  /// [page] in its place: kept while the other shows, its heroes and
  /// what a screen reader reads of it only while it is the one shown.
  Widget _page(int index, Widget page) => _Kept(
    child: ValueListenableBuilder<int>(
      valueListenable: widget.pager.shown,
      builder: (_, shown, child) => ExcludeSemantics(
        excluding: shown != index,
        child: HeroMode(enabled: shown == index, child: child ?? page),
      ),
      child: page,
    ),
  );

  @override
  Widget build(BuildContext context) {
    final pager = widget.pager;
    final still = pager.still = MediaQuery.disableAnimationsOf(context);
    final pages = Listener(
      onPointerDown: (_) => _fingers++,
      onPointerUp: (_) => _fingers--,
      onPointerCancel: (_) => _fingers--,
      child: NotificationListener<ScrollNotification>(
        onNotification: _scrolled,
        child: PageView(
          key: const ValueKey('homePages'),
          controller: pager.controller,
          // The page not shown is laid out beside it, built ahead.
          allowImplicitScrolling: true,
          // The way to the touch slop counts: a quick flick that passes the
          // slop only with its last move still moves the pages, and so
          // flings to the other one; dropped, it moved them by nothing and
          // a fling back from the Areas list stayed there (#258).
          dragStartBehavior: DragStartBehavior.down,
          // With animations off the pages snap in their own physics, which
          // the snapping PageView would otherwise wrap.
          pageSnapping: !still,
          physics: still ? const _StillPagePhysics() : null,
          children: [
            _page(HomePager.agenda, widget.agenda),
            _page(HomePager.areas, widget.areas),
          ],
        ),
      ),
    );
    return ValueListenableBuilder<int>(
      valueListenable: pager.shown,
      builder: (_, shown, child) => PopScope(
        canPop: shown == HomePager.agenda,
        onPopInvokedWithResult: (didPop, _) {
          if (!didPop) pager.showAgenda();
        },
        child: child ?? pages,
      ),
      child: Theme.of(context).platform == TargetPlatform.iOS
          ? Stack(
              fit: StackFit.expand,
              children: [
                pages,
                // Lets everything else through: taps, and drags along it.
                const Positioned(
                  left: 0,
                  top: 0,
                  bottom: 0,
                  width: _edge,
                  child: _HeldEdge(),
                ),
              ],
            )
          : pages,
    );
  }
}

/// Takes a sideways drag that starts on it, and does nothing with it, so
/// the pages under it stay put.
class _HeldEdge extends StatelessWidget {
  const _HeldEdge();

  @override
  Widget build(BuildContext context) => RawGestureDetector(
    behavior: HitTestBehavior.translucent,
    gestures: {
      HorizontalDragGestureRecognizer:
          GestureRecognizerFactoryWithHandlers<HorizontalDragGestureRecognizer>(
            () => HorizontalDragGestureRecognizer(debugOwner: this),
            (drag) => drag.onStart = (_) {},
          ),
    },
  );
}

/// Keeps [child] alive while it is out of view.
class _Kept extends StatefulWidget {
  final Widget child;

  const _Kept({required this.child});

  @override
  State<_Kept> createState() => _KeptState();
}

class _KeptState extends State<_Kept> with AutomaticKeepAliveClientMixin {
  @override
  bool get wantKeepAlive => true;

  @override
  Widget build(BuildContext context) {
    super.build(context);
    return widget.child;
  }
}

/// The pages' physics with the system's animations off: letting go
/// lands on the page at once instead of gliding there.
class _StillPagePhysics extends PageScrollPhysics {
  const _StillPagePhysics({super.parent});

  @override
  _StillPagePhysics applyTo(ScrollPhysics? ancestor) =>
      _StillPagePhysics(parent: buildParent(ancestor));

  @override
  Simulation? createBallisticSimulation(
    ScrollMetrics position,
    double velocity,
  ) {
    final glide = super.createBallisticSimulation(position, velocity);
    if (glide == null) return null;
    // Where the glide would end, long after it settled.
    return _At(glide.x(1e9));
  }
}

/// Is at [end] from the start.
class _At extends Simulation {
  final double end;

  _At(this.end);

  @override
  double x(double time) => end;

  @override
  double dx(double time) => 0;

  @override
  bool isDone(double time) => true;
}
