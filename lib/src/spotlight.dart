import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'l10n.dart';

/// Spotlights (obf-1043): a screen explains itself on a person's first
/// visit, one element at a time. The page dims, the element shows
/// through, a card beside it says one sentence; Next goes on, Skip, a
/// tap outside or Back end the screen's tour. Never an unskippable
/// tutorial.
///
/// What this Device has shown is kept per tip, as local settings never
/// synced and never in a Backup (ADR 0008): the ids under
/// `spot:<screen>`, so a thing added to a screen later still gets its
/// own tip once. Tours run only under a [SpotlightScope], which only the
/// app started by `main.dart` puts in place: tests and screenshots see
/// none.
class SpotlightItem {
  final String id;
  final String Function(AppLocalizations t) text;

  const SpotlightItem(this.id, this.text);
}

/// The tips of every screen, in the order its tour shows them. A ticket
/// that adds something to a screen appends that thing's tip to its
/// screen here, and never adds it to [before070].
final Map<String, List<SpotlightItem>> spotlightManifest = {
  'agenda': [
    SpotlightItem('agenda-plus', (t) => t.spotAgendaPlus),
    SpotlightItem('agenda-today', (t) => t.spotAgendaToday),
    SpotlightItem('agenda-unsorted', (t) => t.spotAgendaUnsorted),
    SpotlightItem('agenda-browse', (t) => t.spotAgendaBrowse),
    SpotlightItem('agenda-quiet', (t) => t.spotAgendaQuiet),
    SpotlightItem('agenda-profile', (t) => t.spotAgendaProfile),
  ],
  'capture': [
    SpotlightItem('capture-nudge', (t) => t.spotCaptureNudge),
    SpotlightItem('capture-more', (t) => t.spotCaptureMore),
  ],
  'editor': [
    SpotlightItem('editor-kind', (t) => t.spotEditorKind),
    SpotlightItem('editor-place', (t) => t.spotEditorPlace),
    SpotlightItem('editor-remind', (t) => t.spotEditorRemind),
    SpotlightItem('editor-code-language', (t) => t.spotEditorCodeLanguage),
    SpotlightItem('editor-linked-from', (t) => t.spotEditorLinkedFrom),
    SpotlightItem('editor-add-part', (t) => t.spotEditorAddPart),
  ],
  // An Area's list and a Topic's alike.
  'area': [
    SpotlightItem('area-search', (t) => t.spotAreaSearch),
    SpotlightItem('area-add', (t) => t.spotAreaAdd),
    SpotlightItem('area-compact', (t) => t.spotCardsView),
  ],
  'unsorted': [
    SpotlightItem('unsorted-card', (t) => t.spotUnsortedCard),
    SpotlightItem('unsorted-compact', (t) => t.spotCardsView),
  ],
  'routine': [
    SpotlightItem('routine-sentence', (t) => t.spotRoutineSentence),
    SpotlightItem('routine-times', (t) => t.spotRoutineTimes),
    SpotlightItem('routine-heads-ups', (t) => t.spotRoutineHeadsUps),
    SpotlightItem('routine-save', (t) => t.spotRoutineSave),
  ],
  'settings': [
    SpotlightItem('settings-search', (t) => t.spotSettingsSearch),
    SpotlightItem('settings-show-tips', (t) => t.spotSettingsShowTips),
    SpotlightItem('settings-features', (t) => t.spotSettingsFeatures),
    SpotlightItem('settings-grammar', (t) => t.spotSettingsGrammar),
  ],
};

/// The tips of what the screens held up to 0.6.1, by screen: a Device
/// updated from there counts them as shown once ([seedSpotlights]), so
/// it sees only the tips of what is new. Frozen: never computed from
/// [spotlightManifest], and nothing is ever added.
const before070 = <String, List<String>>{
  'agenda': [
    'agenda-plus',
    'agenda-today',
    'agenda-unsorted',
    'agenda-browse',
    'agenda-profile',
  ],
  'capture': ['capture-nudge'],
  'editor': ['editor-kind', 'editor-place'],
  'area': ['area-search', 'area-add', 'area-compact'],
  'unsorted': ['unsorted-card', 'unsorted-compact'],
  'routine': ['routine-sentence', 'routine-times', 'routine-save'],
  'settings': ['settings-search', 'settings-features'],
};

/// The local setting that holds the ids of [screen]'s tips shown here.
String spotlightKey(String screen) => 'spot:$screen';

/// The local setting set once this Device first ran with Spotlights on.
const spotlightsStartedKey = 'spotStarted';

/// The screen whose manifest holds the tip [id]; none for an unknown id.
String? spotlightScreenOf(String id) {
  for (final MapEntry(key: screen, value: items) in spotlightManifest.entries) {
    if (items.any((item) => item.id == id)) return screen;
  }
  return null;
}

/// Pure due computation: every item of [items] not in [seenCsv], in
/// their order.
List<SpotlightItem> dueSpotlights(String seenCsv, List<SpotlightItem> items) {
  final seen = seenCsv.split(',').toSet();
  return [
    for (final item in items)
      if (!seen.contains(item.id)) item,
  ];
}

/// The ids of [screen]'s tips this Device has shown.
String seenSpotlights(Store store, String screen) =>
    store.localSetting(spotlightKey(screen)) ?? '';

/// Counts [ids] of [screen] as shown, besides those shown already.
void markSpotlightsSeen(Store store, String screen, Iterable<String> ids) {
  final seen = {
    for (final id in seenSpotlights(store, screen).split(','))
      if (id.isNotEmpty) id,
    ...ids,
  };
  store.setLocalSetting(spotlightKey(screen), seen.join(','));
}

/// Once per Device, at the app's start with Spotlights on and before the
/// first start or the help Notes run: a Device that ran an earlier
/// version, [firstStartDue] false, counts the tips of what it already had
/// as shown ([before070]). A fresh Device, a new person, a Device added
/// or a Backup restored, keeps every tip due.
void seedSpotlights(Store store, {required bool firstStartDue}) {
  if (store.localSetting(spotlightsStartedKey) != null) return;
  if (!firstStartDue) {
    for (final MapEntry(key: screen, value: ids) in before070.entries) {
      markSpotlightsSeen(store, screen, ids);
    }
  }
  store.setLocalSetting(spotlightsStartedKey, '1');
}

/// Counts every tip of every screen as shown: the screenshot harness's,
/// so no frame is taken with a tip on it.
void markAllSpotlightsSeen(Store store) {
  for (final MapEntry(key: screen, value: items) in spotlightManifest.entries) {
    markSpotlightsSeen(store, screen, [for (final item in items) item.id]);
  }
}

/// "Show tips again": every screen shows its tips again on its next
/// visit. The Device stays seeded: an update does not count them shown.
void resetSpotlights(Store store) {
  for (final screen in {...spotlightManifest.keys, ...before070.keys}) {
    store.removeLocalSetting(spotlightKey(screen));
  }
}

/// Tells the Spotlights a page came back on top, so its tips still due
/// show; the app's navigator observes with it.
final spotlightRoutes = RouteObserver<PageRoute<dynamic>>();

/// Where tours run: holds the store their marks go to, the anchors in
/// place and the one tour at a time. Without it a [Spotlight] is its
/// child alone.
class SpotlightScope extends StatefulWidget {
  final Store store;
  final Widget child;

  const SpotlightScope({super.key, required this.store, required this.child});

  @override
  State<SpotlightScope> createState() => _SpotlightScopeState();
}

class _SpotlightScopeState extends State<SpotlightScope> {
  /// The anchors in place by tip id, the latest last; a page under
  /// another keeps its own.
  final _anchors = <String, List<_SpotlightState>>{};

  /// The screens whose tours were asked for, each by one of its anchors.
  final _asked = <String, _SpotlightState>{};
  var _scheduled = false;
  var _running = false;

  static _SpotlightScopeState? _of(BuildContext context) =>
      context.getInheritedWidgetOfExactType<_SpotlightInherited>()?.scope;

  void _add(_SpotlightState anchor) =>
      _anchors.putIfAbsent(anchor.id, () => []).add(anchor);

  void _remove(_SpotlightState anchor) {
    final list = _anchors[anchor.id];
    list?.remove(anchor);
    if (list != null && list.isEmpty) _anchors.remove(anchor.id);
  }

  /// A tour of [anchor]'s screen once this frame is drawn.
  void _ask(_SpotlightState anchor) {
    final screen = spotlightScreenOf(anchor.id);
    if (screen == null) return;
    _asked[screen] = anchor;
    if (_scheduled || _running) return;
    _scheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      _scheduled = false;
      unawaited(_drain());
    });
    WidgetsBinding.instance.ensureVisualUpdate();
  }

  Future<void> _drain() async {
    if (_running) return;
    _running = true;
    try {
      while (mounted && _asked.isNotEmpty) {
        final screen = _asked.keys.first;
        if (_asked.remove(screen) case final asker? when asker.mounted) {
          await _tour(asker, screen);
        }
      }
    } finally {
      _running = false;
    }
  }

  /// The anchor of [id] on [route] that shows now, if any.
  _SpotlightState? _anchorOn(ModalRoute<Object?> route, String id) {
    for (final anchor in (_anchors[id] ?? const []).reversed) {
      if (anchor.mounted && anchor._route == route && anchor.rect() != null) {
        return anchor;
      }
    }
    return null;
  }

  /// [screen]'s tour on [asker]'s page: its due tips whose element shows
  /// now, one after another, each marked shown before it shows.
  Future<void> _tour(_SpotlightState asker, String screen) async {
    final items = spotlightManifest[screen];
    final route = asker._route;
    if (items == null || route == null) return;
    final store = widget.store;
    if (dueSpotlights(seenSpotlights(store, screen), items).isEmpty) return;
    // Anchors move while the page slides in, or back as the one over it
    // goes: measured early, the light falls beside them. The page
    // settles first.
    await _settled(route.animation);
    await _settled(route.secondaryAnimation);
    await _nextFrame();
    // Only on the page on top: never over a dialog, a menu or another
    // page, never under the App Lock.
    if (!mounted || !route.isCurrent) return;
    final due = [
      for (final item in dueSpotlights(seenSpotlights(store, screen), items))
        if (_anchorOn(route, item.id) != null) item,
    ];
    for (final (i, item) in due.indexed) {
      if (!mounted || !route.isCurrent) return;
      final anchor = _anchorOn(route, item.id);
      if (anchor == null) continue;
      await _bringOnScreen(anchor);
      if (!mounted || !anchor.mounted || !route.isCurrent) return;
      final rect = anchor.rect();
      if (rect == null) continue;
      // Marked before it shows: a tour cut short never comes again.
      markSpotlightsSeen(store, screen, [item.id]);
      final answer = await showDialog<SpotlightAnswer>(
        context: anchor.context,
        barrierColor: Colors.transparent,
        // The tip paints where the anchor was measured, in the overlay;
        // the safe area would shift it by the status bar.
        useSafeArea: false,
        builder: (context) => SpotlightTip(
          target: rect,
          measure: anchor.rect,
          text: item.text(context.t),
          isLast: i == due.length - 1,
        ),
      );
      switch (answer) {
        case SpotlightAnswer.next:
          continue;
        case SpotlightAnswer.gone:
          // Its page went out of sight, as under the App Lock: the rest
          // stays due for its next showing.
          return;
        case SpotlightAnswer.end || null:
          // Skip, a tap outside, Back: the tour ends, the rest of it
          // counts as shown.
          markSpotlightsSeen(store, screen, [for (final rest in due) rest.id]);
          return;
      }
    }
  }

  /// Waits until [animation] stands still, two seconds at most.
  static Future<void> _settled(Animation<double>? animation) async {
    if (animation == null || !animation.isAnimating) return;
    final settled = Completer<void>();
    void listener(AnimationStatus status) {
      if (!status.isAnimating && !settled.isCompleted) settled.complete();
    }

    animation.addStatusListener(listener);
    await settled.future.timeout(const Duration(seconds: 2), onTimeout: () {});
    animation.removeStatusListener(listener);
  }

  /// The end of the next frame, which is asked for: a tour starts after
  /// a frame, when none may be coming by itself.
  static Future<void> _nextFrame() {
    WidgetsBinding.instance.scheduleFrame();
    return WidgetsBinding.instance.endOfFrame;
  }

  /// An anchor out of sight in its list scrolls into view; one in view
  /// stays where it is. Only a list up and down moves, never the pages
  /// beside each other.
  Future<void> _bringOnScreen(_SpotlightState anchor) async {
    final rect = anchor.rect();
    final box = anchor.context.findRenderObject();
    final stage = anchor._stage();
    if (rect == null || box == null || stage == null) return;
    if (rect.top >= 0 && rect.bottom <= stage.size.height) return;
    var scrollable = Scrollable.maybeOf(anchor.context);
    while (scrollable != null &&
        axisDirectionToAxis(scrollable.axisDirection) != Axis.vertical) {
      scrollable = Scrollable.maybeOf(scrollable.context);
    }
    if (scrollable == null) return;
    await scrollable.position.ensureVisible(
      box,
      alignment: 0.5,
      duration: const Duration(milliseconds: 200),
    );
    await _nextFrame();
  }

  @override
  Widget build(BuildContext context) =>
      _SpotlightInherited(scope: this, child: widget.child);
}

class _SpotlightInherited extends InheritedWidget {
  final _SpotlightScopeState scope;

  const _SpotlightInherited({required this.scope, required super.child});

  @override
  bool updateShouldNotify(_SpotlightInherited old) => old.scope != scope;
}

/// Holds the tours of the Spotlights under it while [held], as the page
/// of the Agenda and the Areas list that is not shown; they run once it
/// is let go.
class SpotlightHold extends InheritedWidget {
  final bool held;

  const SpotlightHold({super.key, required this.held, required super.child});

  @override
  bool updateShouldNotify(SpotlightHold old) => old.held != held;
}

/// The element the tip [id] points at. Once it shows, on the page on
/// top, its screen's tour runs for the tips still due; again when its
/// page comes back on top or out from under the App Lock.
class Spotlight extends StatefulWidget {
  final String id;
  final Widget child;

  const Spotlight({super.key, required this.id, required this.child});

  @override
  State<Spotlight> createState() => _SpotlightState();
}

class _SpotlightState extends State<Spotlight> with RouteAware {
  _SpotlightScopeState? _scope;
  ModalRoute<Object?>? _route;
  late String id = widget.id;

  /// Whether it showed at the last look: drawn, not held.
  var _live = false;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final scope = _SpotlightScopeState._of(context);
    if (scope != _scope) {
      _scope?._remove(this);
      _scope = scope;
      scope?._add(this);
    }
    if (scope == null) return;
    final route = ModalRoute.of(context);
    if (route != _route) {
      spotlightRoutes.unsubscribe(this);
      _route = route;
      if (route is PageRoute<dynamic>) spotlightRoutes.subscribe(this, route);
    }
    final live =
        TickerMode.valuesOf(context).enabled &&
        !(context.dependOnInheritedWidgetOfExactType<SpotlightHold>()?.held ??
            false);
    if (live && !_live) scope._ask(this);
    _live = live;
  }

  @override
  void didUpdateWidget(Spotlight old) {
    super.didUpdateWidget(old);
    if (old.id == widget.id) return;
    _scope?._remove(this);
    id = widget.id;
    _scope?._add(this);
    _scope?._ask(this);
  }

  @override
  void didPopNext() => _scope?._ask(this);

  @override
  void dispose() {
    spotlightRoutes.unsubscribe(this);
    _scope?._remove(this);
    super.dispose();
  }

  /// The box tips paint in: the root navigator's overlay, which an
  /// announcement at the top pushes down.
  RenderBox? _stage() {
    final stage = Navigator.maybeOf(
      context,
      rootNavigator: true,
    )?.overlay?.context.findRenderObject();
    return stage is RenderBox && stage.attached && stage.hasSize ? stage : null;
  }

  /// Where it shows in the box tips paint in; none while it is out of
  /// sight, held, or of no size.
  Rect? rect() {
    if (!mounted) return null;
    if (!TickerMode.getValuesNotifier(context).value.enabled) return null;
    if (context.getInheritedWidgetOfExactType<SpotlightHold>()?.held ?? false) {
      return null;
    }
    final box = context.findRenderObject();
    if (box is! RenderBox || !box.attached || !box.hasSize) return null;
    final stage = _stage();
    if (box.size.isEmpty || stage == null) return null;
    return box.localToGlobal(Offset.zero, ancestor: stage) & box.size;
  }

  @override
  Widget build(BuildContext context) => widget.child;
}

/// How a tip was left.
enum SpotlightAnswer {
  /// Next, or the last one's Done.
  next,

  /// Skip or a tap outside the card; Back too, which answers nothing.
  end,

  /// Its element went out of sight while it showed.
  gone,
}

/// Widest a tip card gets: one sentence stretched across a tablet is the
/// wrong shape.
const tipMaxWidth = 400.0;

/// Smallest gap between the card and the edge of the screen.
const tipMargin = 24.0;

/// Gap between the card and the element it explains.
const tipGap = 16.0;

/// The room a card needs at least, its buttons included.
const tipMinHeight = 160.0;

/// How far the light reaches beyond the element.
const tipInflate = 6.0;

/// Where a tip sits for a highlighted [target] on a [screen] whose
/// [padding] the system bars take: centred on the target, capped in
/// width, inside the screen, and on the side of the target with more
/// room, so it never covers a target that leaves it room.
({double left, double width, double? top, double? bottom}) tipPlacement(
  Rect target,
  Size screen, {
  EdgeInsets padding = EdgeInsets.zero,
}) {
  final width = math.min(tipMaxWidth, screen.width - 2 * tipMargin);
  final rightmost = math.max(tipMargin, screen.width - tipMargin - width);
  final left = (target.center.dx - width / 2)
      .clamp(tipMargin, rightmost)
      .toDouble();
  final top = padding.top + tipMargin;
  final bottom = padding.bottom + tipMargin;
  final below = screen.height - target.bottom >= target.top;
  // A target at or past an edge, or one taller than the room, would push
  // the card off the screen: it stays inside, buttons included.
  final most = math.max(top, screen.height - bottom - tipMinHeight);
  return (
    left: left,
    width: width,
    top: below ? (target.bottom + tipGap).clamp(top, most).toDouble() : null,
    bottom: below
        ? null
        : (screen.height - target.top + tipGap)
              .clamp(bottom, math.max(bottom, most))
              .toDouble(),
  );
}

/// One tip: the page dimmed with [target] cut out of it and a card
/// beside it. [measure] follows the element should it move, as when the
/// keyboard goes; null from it closes the tip as gone. Public so a test
/// places it without a tour.
class SpotlightTip extends StatefulWidget {
  final Rect target;
  final Rect? Function()? measure;
  final String text;
  final bool isLast;

  const SpotlightTip({
    super.key,
    required this.target,
    this.measure,
    required this.text,
    required this.isLast,
  });

  @override
  State<SpotlightTip> createState() => _SpotlightTipState();
}

class _SpotlightTipState extends State<SpotlightTip>
    with WidgetsBindingObserver {
  late Rect _target = widget.target;
  var _closing = false;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _remeasure();
  }

  @override
  void didChangeMetrics() => _remeasure();

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) => _remeasure();

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }

  /// Looks where the element is once the frame is laid out.
  void _remeasure() {
    final measure = widget.measure;
    if (measure == null) return;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted || _closing) return;
      final now = measure();
      if (now == null) {
        _close(SpotlightAnswer.gone);
      } else if (now != _target) {
        setState(() => _target = now);
      }
    });
  }

  void _close(SpotlightAnswer answer) {
    if (_closing) return;
    _closing = true;
    Navigator.of(context).pop(answer);
  }

  @override
  Widget build(BuildContext context) => LayoutBuilder(
    // The overlay's size, which an announcement at the top makes less
    // than the screen's.
    builder: (context, constraints) => _tip(context, constraints.biggest),
  );

  Widget _tip(BuildContext context, Size screen) {
    final t = context.t;
    final lit = _target.inflate(tipInflate).intersect(Offset.zero & screen);
    final place = tipPlacement(
      lit,
      screen,
      padding: MediaQuery.paddingOf(context),
    );
    return Stack(
      children: [
        // A tap outside the card ends the tour; nothing under it is hit.
        Positioned.fill(
          child: GestureDetector(
            key: const ValueKey('spotOutside'),
            behavior: HitTestBehavior.opaque,
            onTap: () => _close(SpotlightAnswer.end),
            child: CustomPaint(painter: _DimPainter(lit)),
          ),
        ),
        Positioned(
          left: place.left,
          width: place.width,
          top: place.top,
          bottom: place.bottom,
          child: Material(
            color: Colors.transparent,
            child: Card(
              key: const ValueKey('spotCard'),
              child: Padding(
                padding: const EdgeInsets.all(16),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(widget.text),
                    const SizedBox(height: 8),
                    Wrap(
                      alignment: WrapAlignment.end,
                      spacing: 8,
                      children: [
                        TextButton(
                          key: const ValueKey('spotSkip'),
                          onPressed: () => _close(SpotlightAnswer.end),
                          child: Text(t.spotSkip),
                        ),
                        FilledButton(
                          key: const ValueKey('spotNext'),
                          autofocus: true,
                          onPressed: () => _close(SpotlightAnswer.next),
                          child: Text(widget.isLast ? t.spotDone : t.spotNext),
                        ),
                      ],
                    ),
                  ],
                ),
              ),
            ),
          ),
        ),
      ],
    );
  }
}

class _DimPainter extends CustomPainter {
  final Rect target;

  const _DimPainter(this.target);

  @override
  void paint(Canvas canvas, Size size) {
    final path = Path()
      ..addRect(Offset.zero & size)
      ..addRRect(RRect.fromRectAndRadius(target, const Radius.circular(12)))
      ..fillType = PathFillType.evenOdd;
    canvas.drawPath(path, Paint()..color = Colors.black.withValues(alpha: 0.6));
  }

  @override
  bool shouldRepaint(_DimPainter old) => old.target != target;
}
