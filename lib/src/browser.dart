import 'dart:async';

import 'package:flutter/foundation.dart' show ValueListenable, setEquals;
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/scheduler.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'app_lock.dart';
import 'card_glance.dart';
import 'found_marks.dart';
import 'l10n.dart';
import 'look.dart';
import 'note_card.dart';
import 'note_editor_screen.dart';
import 'profile.dart';
import 'routine_editor_screen.dart';

/// Reloads a page's list once after the log changed, never in the middle
/// of whatever changed it, so what a page opened on top sorted, like the
/// Nudge screen from its notification, or what a sync brought, is gone
/// or there once the page shows again. A page out of sight, under
/// another page or behind the lock, reads nothing on a change; it reads
/// once as it shows again, before the frame that shows it (#339).
mixin ReloadsOnLogChange<T extends StatefulWidget> on State<T> {
  /// The log whose changes the page follows.
  Store get log;

  /// Reads the list again and shows it.
  void reloadLog();

  var _reloadQueued = false;

  /// Whether the log changed while the page was out of sight.
  var _missed = false;

  /// Whether the page is in sight: its tickers run.
  ValueListenable<TickerModeData>? _sight;

  bool get _inSight => _sight?.value.enabled ?? true;

  @override
  void initState() {
    super.initState();
    log.addListener(_logChanged);
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final sight = TickerMode.getValuesNotifier(context);
    if (sight == _sight) return;
    _sight?.removeListener(_sightChanged);
    _sight = sight..addListener(_sightChanged);
    _sightChanged();
  }

  @override
  void dispose() {
    _sight?.removeListener(_sightChanged);
    log.removeListener(_logChanged);
    super.dispose();
  }

  /// Back in sight after a change: reads now, in the build that shows
  /// it again.
  void _sightChanged() {
    if (!_missed || !_inSight || !mounted) return;
    _missed = false;
    reloadLog();
  }

  /// Reads the list again once, with the next microtask, however many
  /// changes and calls ask for it meanwhile: a page that saved calls it
  /// rather than reading at once, as the log's change reads it too.
  void reloadSoon() {
    if (_reloadQueued) return;
    _reloadQueued = true;
    scheduleMicrotask(() {
      _reloadQueued = false;
      if (!mounted) return;
      if (!_inSight) {
        _missed = true;
        return;
      }
      reloadLog();
    });
  }

  void _logChanged() => reloadSoon();
}

/// A Note list's switch between the compact and the detailed view (#114,
/// #189): detailed unless switched to compact, remembered per list in a
/// setting of this Device, and the Notes a tap switched to the other view,
/// remembered in another setting of this Device until the switch changes.
mixin FoldsNoteCards<T extends StatefulWidget> on State<T> {
  /// The log whose Device settings keep the choice.
  Store get log;

  /// Which list this is, e.g. `area:<id>`; each keeps its own choice.
  String get cardsKey;

  /// Whether the cards start in the compact view.
  bool compact = false;

  /// The Notes a tap switched to the view the list does not start in.
  Set<String> switched = const {};

  String get _setting => 'compactNotes:$cardsKey';

  /// The Notes in [switched], joined by commas.
  String get _switchedSetting => 'switchedNotes:$cardsKey';

  @override
  void initState() {
    super.initState();
    readCompact();
  }

  /// Reads the choice and the Notes switched again, as another page
  /// showing the same list may have changed them.
  void readCompact() {
    compact = log.localSetting(_setting) == 'true';
    final kept = log.localSetting(_switchedSetting) ?? '';
    switched = {
      for (final id in kept.split(','))
        if (id.isNotEmpty) id,
    };
  }

  /// Switches the list to compact or detailed and remembers it; every card
  /// shows that view again.
  void setCompact(bool on) {
    setState(() {
      compact = on;
      switched = const {};
    });
    // Nothing kept means detailed, the default.
    on
        ? log.setLocalSetting(_setting, 'true')
        : log.removeLocalSetting(_setting);
    log.removeLocalSetting(_switchedSetting);
  }

  /// Switches [note]'s card to its other view, in place, and remembers it.
  void switchView(Note note) {
    final next = {...switched};
    if (!next.remove(note.id)) next.add(note.id);
    setState(() => switched = next);
    next.isEmpty
        ? log.removeLocalSetting(_switchedSetting)
        : log.setLocalSetting(_switchedSetting, next.join(','));
  }
}

/// The bar button of a Note list's compact/detailed switch: it says what
/// tapping it does and shows as selected while the list is compact.
class CompactNotesButton extends StatelessWidget {
  final bool compact;
  final ValueChanged<bool> onChanged;

  const CompactNotesButton({
    super.key = const ValueKey('compactNotes'),
    required this.compact,
    required this.onChanged,
  });

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return IconButton(
      tooltip: compact ? t.showNotesFull : t.showNotesCompact,
      isSelected: compact,
      onPressed: () => onChanged(!compact),
      icon: const Icon(Icons.view_agenda_outlined),
      selectedIcon: const Icon(Icons.view_headline),
    );
  }
}

/// Where swipe left leads in the notes browser: the last selected Area
/// and Topic (UI laws, "Navigation"). Lives as long as the app, so the
/// way back in survives leaving the browser.
class BrowserTrail {
  String? _area;
  String? _topic;

  /// The last selected Area, or null when none or it is gone.
  Area? area(Places places) {
    final id = _area;
    return id == null ? null : places.areaById(id);
  }

  /// The last selected Topic if it lives in [areaId], or null.
  Topic? topicIn(Places places, String areaId) {
    final id = _topic;
    final topic = id == null ? null : places.topicById(id);
    return topic?.areaId == areaId ? topic : null;
  }

  void choseArea(Area area) => _area = area.id;

  void choseTopic(Topic topic) {
    _area = topic.areaId;
    _topic = topic.id;
  }
}

/// The page of a browser level (Area, Topic, and the Areas list of a
/// tag's search): it slides in from the right while the level it covers
/// slides out to the left, side by side, since the pages are transparent
/// over the question marks. A swipe moves both with the finger
/// ([BrowserSwipes]). With the system's animations off the change is
/// immediate.
class BrowserRoute<T> extends MaterialPageRoute<T> {
  static const _duration = Duration(milliseconds: 300);
  static const _curve = Curves.easeInOutCubic;
  static final _enter = Tween(begin: const Offset(1, 0), end: Offset.zero);
  static final _leave = Tween(begin: Offset.zero, end: const Offset(-1, 0));

  BrowserRoute({required super.builder});

  /// Whether a finger moves the page: then it follows the finger
  /// exactly, and lets go with its own curve.
  var _following = false;

  /// Whether this follow is the navigator's back gesture, which keeps
  /// the pages from taking taps until it ends.
  var _gesture = false;

  bool get _still =>
      navigator?.context
          .getInheritedWidgetOfExactType<MediaQuery>()
          ?.data
          .disableAnimations ??
      false;

  @override
  Duration get transitionDuration => _still ? Duration.zero : _duration;

  @override
  Duration get reverseTransitionDuration => transitionDuration;

  @override
  Animation<double> createAnimation() =>
      _Followed(super.createAnimation(), this);

  @override
  Widget buildTransitions(
    BuildContext context,
    Animation<double> animation,
    Animation<double> secondaryAnimation,
    Widget child,
  ) => SlideTransition(
    position: animation.drive(_enter),
    child: SlideTransition(
      position: secondaryAnimation.drive(_leave),
      child: child,
    ),
  );

  /// How the page under a browser level leaves alongside it, when that
  /// page is no browser level itself, as a Note's page under a tag's
  /// search. The Agenda leaves in its own transitions
  /// ([BrowserHomeRoute]).
  @override
  DelegatedTransitionBuilder? get delegatedTransition => _slideAway;

  static Widget? _slideAway(
    BuildContext context,
    Animation<double> animation,
    Animation<double> secondaryAnimation,
    bool allowSnapshotting,
    Widget? child,
  ) =>
      SlideTransition(position: secondaryAnimation.drive(_leave), child: child);

  /// Whether swipe right may take this page back with the finger: it is
  /// on top, standing still, and nothing keeps it from leaving.
  bool get _mayFollowBack {
    final navigator = this.navigator;
    return navigator != null &&
        isCurrent &&
        !isFirst &&
        !willHandlePopInternally &&
        popDisposition == RoutePopDisposition.pop &&
        !navigator.userGestureInProgress &&
        controller?.status == AnimationStatus.completed;
  }

  /// Hands the page to the finger going back.
  void _followBack() {
    _gesture = true;
    navigator?.didStartUserGesture();
    _following = true;
  }

  /// Puts the page at [value] of the way in.
  void _follow(double value) => controller?.value = value;

  /// Takes over, just pushed, from the preview the finger let go at
  /// [from] of the way in, and goes on all the way. Heroes, such as the
  /// "+", take off from the start of a push, so it takes over only after
  /// that frame; [shown] then drops the preview.
  void _takeOver(double from, VoidCallback shown) {
    _following = true;
    SchedulerBinding.instance.addPostFrameCallback((_) {
      shown();
      if (!isActive) return _done(navigator);
      _follow(from);
      _letGo(stays: true);
    });
  }

  /// The finger lets go: the page goes all the way in when [stays],
  /// otherwise it leaves.
  void _letGo({required bool stays}) {
    final controller = this.controller;
    final navigator = this.navigator;
    if (controller == null || navigator == null) return _done(navigator);
    // Something came on top meanwhile: the page stays if it still can.
    final keep = isCurrent ? stays : isActive;
    if (keep) {
      controller.animateTo(1, curve: Curves.easeOut);
    } else {
      if (isCurrent) navigator.pop();
      if (controller.isAnimating) {
        controller.animateBack(0, curve: Curves.easeOut);
      }
    }
    if (!controller.isAnimating) return _done(navigator);
    late AnimationStatusListener settled;
    settled = (_) {
      controller.removeStatusListener(settled);
      _done(navigator);
    };
    controller.addStatusListener(settled);
  }

  /// The finger is gone for good.
  void _done(NavigatorState? navigator) {
    _following = false;
    if (_gesture) navigator?.didStopUserGesture();
    _gesture = false;
  }
}

/// The app's first page, the Agenda and the Areas list side by side
/// (#258), under the browser levels pushed from it, an Area's first: it
/// slides out to the left as such a level comes in, as a [BrowserRoute]
/// does. It does so in its own transitions, which keep their shape
/// whatever comes on top, rather than in the level's delegated one,
/// which wraps the page in one more widget when the level is pushed:
/// moving the page into it built the whole Agenda anew in the frame the
/// finger let go (#210).
class BrowserHomeRoute<T> extends MaterialPageRoute<T> {
  BrowserHomeRoute({required super.builder, super.settings});

  /// Whether a browser level is on top, coming or going.
  var _browserNext = false;

  /// The level's own while one is on top, so the navigator leaves this
  /// page's transitions as they are instead of wrapping them in it.
  @override
  DelegatedTransitionBuilder? get delegatedTransition =>
      _browserNext ? BrowserRoute._slideAway : super.delegatedTransition;

  @override
  void didChangeNext(Route<dynamic>? nextRoute) {
    _browserNext = nextRoute is BrowserRoute;
    super.didChangeNext(nextRoute);
  }

  @override
  void didPopNext(Route<dynamic> nextRoute) {
    _browserNext = nextRoute is BrowserRoute;
    super.didPopNext(nextRoute);
  }

  @override
  Widget buildTransitions(
    BuildContext context,
    Animation<double> animation,
    Animation<double> secondaryAnimation,
    Widget child,
  ) {
    final browser = _browserNext;
    return SlideTransition(
      position: (browser ? secondaryAnimation : kAlwaysDismissedAnimation)
          .drive(BrowserRoute._leave),
      child: super.buildTransitions(
        context,
        animation,
        browser ? kAlwaysDismissedAnimation : secondaryAnimation,
        child,
      ),
    );
  }
}

/// A browser level's animation, eased unless a finger moves it.
class _Followed extends Animation<double>
    with AnimationWithParentMixin<double> {
  @override
  final Animation<double> parent;
  final BrowserRoute<dynamic> route;

  _Followed(this.parent, this.route);

  @override
  double get value => route._following
      ? parent.value
      : BrowserRoute._curve.transform(parent.value);
}

/// Swipe right goes back one level, swipe left forward into the last
/// selected Area or Topic, when there is one. Wraps a level's whole
/// page. The Areas list beside the Agenda takes only swipe left, with
/// [back] false: swipe right there is the pages' own (#258).
///
/// The pages follow the finger; letting go past a third of the way or
/// with a fling completes the change, otherwise it springs back. Going back
/// moves this level's [BrowserRoute] like the system's back gesture.
/// Going forward cannot push while the finger is down (the navigator
/// would cancel the finger), so the next level shows as a preview
/// beside this page and its route takes over where the finger let go.
/// With the system's animations off nothing follows and the change is
/// immediate.
class BrowserSwipes extends StatefulWidget {
  /// The next level's page, or null when there is none or it cannot
  /// show yet. Built as the finger starts left, for the preview; pushed
  /// only when the swipe completes.
  final BrowserRoute<void>? Function()? forwardRoute;

  /// Opens the next level when [forwardRoute] has none, once the finger
  /// lets go: a closed locked Area asks for the PIN first.
  final VoidCallback? onForward;

  /// Whether swipe right leaves this level; when false this does not
  /// take swipe right at all.
  final bool back;
  final Widget child;

  const BrowserSwipes({
    super.key,
    required this.child,
    this.forwardRoute,
    this.onForward,
    this.back = true,
  });

  @override
  State<BrowserSwipes> createState() => _BrowserSwipesState();
}

class _BrowserSwipesState extends State<BrowserSwipes>
    with SingleTickerProviderStateMixin {
  /// Above this horizontal speed (pixels per second) letting go is a
  /// fling: it goes the way it flings, wherever the finger is.
  static const _minSpeed = 300.0;

  /// How far along the way, 0 to 1, a slow drag must be let go to
  /// complete; short of it the pages spring back.
  static const _turnAt = 1 / 3;

  /// How far the next level's preview is in, 0 to 1.
  late final _pull = AnimationController(
    vsync: this,
    duration: BrowserRoute._duration,
  );
  late final _away = _pull.drive(BrowserRoute._leave);
  late final _in = _pull.drive(BrowserRoute._enter);

  /// The next level's route and its page beside this one, while the
  /// finger pulls it in or it springs back.
  BrowserRoute<void>? _next;
  Widget? _preview;

  /// Which way this drag goes, once its first move says so.
  bool? _forward;

  /// Whether this drag does nothing: it began while a preview was still
  /// on its way.
  var _idle = false;

  /// How far the finger moved since the drag began.
  var _dx = 0.0;

  /// This level going back under the finger, or null.
  BrowserRoute<dynamic>? _moving;

  /// How far along the way the finger is, 0 to 1.
  double get _progress {
    final width = context.size?.width ?? 0;
    if (width <= 0) return 0;
    final along = _forward == true ? -_dx : _dx;
    return (along / width).clamp(0.0, 1.0);
  }

  @override
  void dispose() {
    // The page left under the finger: the gesture is over.
    _moving?._done(_moving?.navigator);
    _pull.dispose();
    super.dispose();
  }

  void _started(DragStartDetails details) {
    _forward = null;
    _idle = _preview != null;
    _dx = 0;
  }

  void _updated(DragUpdateDetails details) {
    if (_idle) return;
    _dx += details.primaryDelta ?? 0;
    if (_forward == null) {
      if (_dx == 0) return;
      final forward = _forward = _dx < 0;
      if (!MediaQuery.disableAnimationsOf(context)) {
        if (forward) {
          _showNext();
        } else {
          _moving = _takeBack();
        }
      }
    }
    if (_preview != null) _pull.value = _progress;
    _moving?._follow(1 - _progress);
  }

  /// Shows the next level beside this one, if there is one and this
  /// level is on top and still.
  void _showNext() {
    final route = ModalRoute.of(context);
    if (route == null ||
        !route.isCurrent ||
        route.animation?.status != AnimationStatus.completed) {
      return;
    }
    final next = widget.forwardRoute?.call();
    if (next == null) return;
    setState(() {
      _next = next;
      _preview = next.builder(context);
    });
  }

  /// This level for the finger to pull away, if it may go back.
  BrowserRoute<dynamic>? _takeBack() {
    final route = ModalRoute.of(context);
    if (!widget.back || route is! BrowserRoute || !route._mayFollowBack) {
      return null;
    }
    route._followBack();
    return route;
  }

  void _ended(DragEndDetails details) {
    final forward = _forward;
    if (_idle || forward == null) return _reset();
    final speed = details.primaryVelocity ?? 0;
    // Positive is the way the swipe goes.
    final along = forward ? -speed : speed;
    final complete =
        along > _minSpeed || (along >= -_minSpeed && _progress > _turnAt);
    final moving = _moving;
    final next = _next;
    _reset();
    if (moving != null) {
      moving._letGo(stays: !complete);
    } else if (next != null) {
      complete ? _push(next) : _springBack();
    } else if (complete) {
      if (forward) {
        _openNext();
      } else if (widget.back) {
        Navigator.of(context).maybePop();
      }
    }
  }

  void _cancelled() {
    final moving = _moving;
    final next = _next;
    _reset();
    moving?._letGo(stays: true);
    if (next != null) _springBack();
  }

  void _reset() {
    _forward = null;
    _idle = false;
    _dx = 0;
    _moving = null;
  }

  /// Pushes the previewed level, which takes over where it is.
  void _push(BrowserRoute<void> next) {
    Navigator.of(context).push(next);
    next._takeOver(_pull.value, () {
      if (!mounted) return;
      _pull.value = 0;
      _dropPreview();
    });
  }

  void _springBack() {
    _pull
        .animateBack(0, curve: Curves.easeOut)
        .whenCompleteOrCancel(_dropPreview);
  }

  void _dropPreview() {
    if (!mounted) return;
    setState(() {
      _next = null;
      _preview = null;
    });
  }

  void _openNext() {
    if (widget.forwardRoute?.call() case final next?) {
      Navigator.of(context).push(next);
    } else {
      widget.onForward?.call();
    }
  }

  @override
  Widget build(BuildContext context) => RawGestureDetector(
    // Empty space swipes too, not only the cards and rows.
    behavior: HitTestBehavior.opaque,
    gestures: {
      _SwipeDrag: GestureRecognizerFactoryWithHandlers<_SwipeDrag>(
        () => _SwipeDrag(debugOwner: this),
        (drag) => drag
          ..rightToo = widget.back
          // The way to the touch slop counts: the pages start where the
          // finger is, and a quick flick that passes the slop with its
          // last move before letting go still knows its way.
          ..dragStartBehavior = DragStartBehavior.down
          ..onStart = _started
          ..onUpdate = _updated
          ..onEnd = _ended
          ..onCancel = _cancelled,
      ),
    },
    child: Stack(
      fit: StackFit.expand,
      children: [
        // Always here, so the page keeps its state when a preview comes.
        SlideTransition(position: _away, child: widget.child),
        if (_preview case final preview?)
          SlideTransition(
            position: _in,
            child: IgnorePointer(
              child: ExcludeSemantics(
                child: HeroMode(enabled: false, child: preview),
              ),
            ),
          ),
      ],
    ),
  );
}

/// A sideways drag; with [rightToo] false only one to the left, one to
/// the right left to the pages around (#258).
class _SwipeDrag extends HorizontalDragGestureRecognizer {
  var rightToo = true;

  _SwipeDrag({super.debugOwner});

  @override
  bool hasSufficientGlobalDistanceToAccept(
    PointerDeviceKind pointerDeviceKind,
    double? deviceTouchSlop,
  ) =>
      (rightToo || globalDistanceMoved < 0) &&
      super.hasSufficientGlobalDistanceToAccept(
        pointerDeviceKind,
        deviceTouchSlop,
      );
}

/// The search field on top of a browser level, with ✕ to clear it. With
/// [tags], typing `#` shows the tags of the tag tree that start with what
/// is typed as chips under it (#267, Q639), whose tap fills one in; the
/// Notes of the Areas of [closedTags], the locked Areas closed now, are
/// left out of them. Each letter typed shows at once, ✕ with it, and
/// runs [onChanged]; a text set as a whole, cleared, a tag picked or
/// Search on the keyboard, runs [onSet], or [onChanged] without it.
class BrowserSearchField extends StatelessWidget {
  /// How long typing pauses before a search over the Notes runs
  /// (obf-1068, C11): a search per letter made each letter wait on it.
  static const pause = Duration(milliseconds: 300);

  final TextEditingController controller;
  final String hint;
  final ValueChanged<String> onChanged;
  final ValueChanged<String>? onSet;
  final Store? tags;
  final Set<String> closedTags;

  const BrowserSearchField({
    super.key,
    required this.controller,
    required this.hint,
    required this.onChanged,
    this.onSet,
    this.tags,
    this.closedTags = const {},
  });

  ValueChanged<String> get _set => onSet ?? onChanged;

  void _clear() {
    controller.clear();
    _set('');
  }

  void _pick(String tag) {
    controller.value = TextEditingValue(
      text: tag,
      selection: TextSelection.collapsed(offset: tag.length),
    );
    _set(tag);
  }

  @override
  Widget build(BuildContext context) {
    final field = Padding(
      padding: const EdgeInsets.fromLTRB(16, 8, 16, 0),
      // ✕ follows the text itself, not the page's next search.
      child: ListenableBuilder(
        listenable: controller,
        builder: (context, _) => TextField(
          controller: controller,
          onChanged: onChanged,
          onSubmitted: _set,
          textInputAction: TextInputAction.search,
          decoration: InputDecoration(
            hintText: hint,
            prefixIcon: const Icon(Icons.search),
            suffixIcon: controller.text.isEmpty
                ? null
                : IconButton(
                    tooltip: context.t.clearSearch,
                    onPressed: _clear,
                    icon: const Icon(Icons.close),
                  ),
            border: OutlineInputBorder(borderRadius: BorderRadius.circular(16)),
          ),
        ),
      ),
    );
    final tags = this.tags;
    if (tags == null) return field;
    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        field,
        TagSuggestions(
          store: tags,
          controller: controller,
          onPick: _pick,
          closed: closedTags,
        ),
      ],
    );
  }
}

/// A page whose search over the Notes runs [BrowserSearchField.pause]
/// after the last letter typed (obf-1068, C11): each letter shows at
/// once, and a letter typed before the pause is over puts the search
/// off again, so no result of an older query replaces a newer one. A
/// search runs on the field's text as it is then.
mixin SearchesAfterPause<T extends StatefulWidget> on State<T> {
  Timer? _searchTimer;

  /// Runs [search] once typing pauses, in place of any still waiting.
  void searchAfterPause(VoidCallback search) {
    _searchTimer?.cancel();
    _searchTimer = Timer(BrowserSearchField.pause, () {
      _searchTimer = null;
      if (mounted) search();
    });
  }

  /// Runs [search] now, in place of any still waiting: for a text set as
  /// a whole.
  void searchNow(VoidCallback search) {
    _searchTimer?.cancel();
    _searchTimer = null;
    search();
  }

  @override
  void dispose() {
    _searchTimer?.cancel();
    super.dispose();
  }
}

/// The fill of the header of a page with a [SearchHeaderSliver], its bar
/// and the band under it (#351, obf-1041): the tone of the question
/// marks, set off from the page's ground in the light and the dark look
/// (UI laws, "Design system", obf-1081).
Color searchHeaderColour(BuildContext context) =>
    LookTones.of(Theme.of(context).brightness).mark;

/// The band a search bar sits on (#351, obf-1041): filled with
/// [searchHeaderColour], which its page's bar takes too, with a line along
/// its bottom, so bar and band read as one header set off from the page
/// and what scrolls under it goes under a line, not an edge in the page's
/// own colour. The list pages have it in their [SearchHeaderSliver]; the
/// Settings, File into…, Merge into…, Add to Note… and Link to… keep it
/// standing over what they list (obf-1088).
class SearchBand extends StatelessWidget {
  /// The search field, with what goes under it.
  final Widget child;

  const SearchBand({super.key, required this.child});

  @override
  Widget build(BuildContext context) => DecoratedBox(
    decoration: BoxDecoration(
      color: searchHeaderColour(context),
      border: Border(
        // Material's outline: its variant fades into the marks' tone.
        bottom: BorderSide(color: Theme.of(context).colorScheme.outline),
      ),
    ),
    // Room under the bar, so its line runs along the band's bottom and
    // not along the field's edge (#351, obf-1041).
    child: Padding(padding: const EdgeInsets.only(bottom: 8), child: child),
  );
}

/// The header of a list page with a search field, the Areas list, an
/// Area and a Topic (#336, Q749): the first sliver of the list, it scrolls
/// away with it and comes back over it, the list staying where it is, as
/// soon as the list is scrolled up a little; one half shown at the end of
/// a scroll snaps shown or gone. While [child]'s field has the focus or
/// [controller] holds a search it stays, so a search being typed or shown
/// never goes away. It sits on a [SearchBand] (#351, obf-1041), and the
/// cards go under it, shown over the list or with the list pulled down.
class SearchHeaderSliver extends StatefulWidget {
  /// The field's text: a search in it keeps the header.
  final TextEditingController controller;

  /// The search field, with what goes under it.
  final Widget child;

  const SearchHeaderSliver({
    super.key,
    required this.controller,
    required this.child,
  });

  @override
  State<SearchHeaderSliver> createState() => _SearchHeaderSliverState();
}

class _SearchHeaderSliverState extends State<SearchHeaderSliver> {
  /// Keeps the field with its focus, text and keyboard while the sliver
  /// around it changes between scrolling away and staying.
  final _kept = GlobalKey();
  var _focused = false;
  late var _searching = _search;

  bool get _search => widget.controller.text.trim().isNotEmpty;

  @override
  void initState() {
    super.initState();
    widget.controller.addListener(_typed);
  }

  @override
  void didUpdateWidget(SearchHeaderSliver old) {
    super.didUpdateWidget(old);
    if (old.controller == widget.controller) return;
    old.controller.removeListener(_typed);
    widget.controller.addListener(_typed);
    _searching = _search;
  }

  @override
  void dispose() {
    widget.controller.removeListener(_typed);
    super.dispose();
  }

  void _typed() {
    if (_search != _searching) setState(() => _searching = _search);
  }

  @override
  Widget build(BuildContext context) {
    final header = KeyedSubtree(
      key: _kept,
      child: SearchBand(
        child: Focus(
          canRequestFocus: false,
          skipTraversal: true,
          onFocusChange: (focused) => setState(() => _focused = focused),
          child: widget.child,
        ),
      ),
    );
    return _focused || _searching
        ? PinnedHeaderSliver(child: header)
        : SliverFloatingHeader(child: header);
  }
}

/// The tags of the tag tree that start with the `#…` typed into
/// [controller], as chips that fill one in (#267, Q639), the one typed
/// exactly left out; nothing while what is typed starts otherwise. They
/// are read as the text or [closed] changes, never while building, and at
/// most [shown] of them show, the tree's order kept. The Notes of the
/// Areas of [closed], the locked Areas closed now, are left out.
class TagSuggestions extends StatefulWidget {
  final Store store;
  final TextEditingController controller;
  final ValueChanged<String> onPick;
  final Set<String> closed;

  static const shown = 12;

  const TagSuggestions({
    super.key,
    required this.store,
    required this.controller,
    required this.onPick,
    this.closed = const {},
  });

  @override
  State<TagSuggestions> createState() => _TagSuggestionsState();
}

class _TagSuggestionsState extends State<TagSuggestions> {
  List<TagNode> _found = const [];
  String _read = '';

  @override
  void initState() {
    super.initState();
    widget.controller.addListener(_changed);
    _find();
  }

  @override
  void didUpdateWidget(TagSuggestions old) {
    super.didUpdateWidget(old);
    if (old.controller != widget.controller) {
      old.controller.removeListener(_changed);
      widget.controller.addListener(_changed);
      _find();
    } else if (!setEquals(old.closed, widget.closed)) {
      _find();
    }
  }

  @override
  void dispose() {
    widget.controller.removeListener(_changed);
    super.dispose();
  }

  void _changed() {
    if (widget.controller.text == _read) return;
    setState(_find);
  }

  void _find() {
    final typed = widget.controller.text;
    _read = typed;
    final wanted = typed.trim().toLowerCase();
    _found = !wanted.startsWith('#')
        ? const []
        : [
            for (final node in TagTree(
              widget.store,
            ).suggestions(typed, closed: widget.closed))
              if (node.tag.toLowerCase() != wanted) node,
          ].take(TagSuggestions.shown).toList();
  }

  @override
  Widget build(BuildContext context) {
    if (_found.isEmpty) return const SizedBox.shrink();
    return Padding(
      key: const ValueKey('search-tag-suggestions'),
      padding: const EdgeInsets.fromLTRB(16, 8, 16, 0),
      // Not a sideways list: swipes belong to the browser.
      child: Wrap(
        spacing: 8,
        runSpacing: 4,
        children: [
          for (final node in _found)
            ActionChip(
              key: ValueKey('search-tag-${node.tag}'),
              label: Text(node.tag),
              onPressed: () => widget.onPick(node.tag),
            ),
        ],
      ),
    );
  }
}

/// Notes as a diary of cards, each with "file into…", "merge into…",
/// with [onSchedule] Routine, Reminder, Frog or Session, and ⋮ for the
/// rest the Note editor offers, as far as this Device may. Each shows in
/// its detailed view unless [compact], those in [switched] in the other
/// view; a tap on a card runs [onSwitch] (#189). A long press or Edit in
/// ⋮ opens the editor (#114).
/// [onChanged] runs after a step of ⋮ that may have changed a Note.
/// [showPlace] says which cards name where they are filed: all but those
/// in the Area or Topic the list is of (#330). While [selected] is given
/// the list selects (#273): each card this Device may change has a box,
/// ticked for those in it, whose tap runs [onSelect].
///
/// The cards of a search for [find] list the places it found in their
/// Notes (obf-1047), read as each card is first built, and draw them on a
/// tint; a tap on one runs [onFind] with it, and a tap on the card, a
/// long press and Edit run it with the first, where there is one.
List<Widget> diaryCards(
  BuildContext context,
  List<PlacedNote> shown,
  void Function(Note note) onFile,
  void Function(Note note) onMerge,
  void Function(Note note) onOpen, {
  required Notes notes,
  required VoidCallback onChanged,
  required bool Function(PlacedNote placed) showPlace,
  void Function(Note note, Spawn spawn)? onSchedule,
  bool compact = false,
  Set<String> switched = const {},
  void Function(Note note)? onSwitch,
  Set<String>? selected,
  void Function(Note note, bool on)? onSelect,
  String find = '',
  void Function(Note note, Finding at)? onFind,
}) {
  final locale = Localizations.localeOf(context).toString();
  final when = DateFormat.yMd(locale).add_Hm();
  // What the cards read, kept with the list as loaded (obf-1069): a tap
  // that switches a card or ticks its box reads none of it again; a list
  // read anew, after any change to the log, reads it anew, as does a new
  // day or language.
  final reads = _ListReads.of(
    shown,
    day: Day.today(notes.store.clock(), deviceWallClock),
    locale: locale,
  );
  List<String> ids() => [for (final placed in shown) placed.note.id];
  // The Sections of every listed Note, read in one pass when the first
  // full card reads its own, not while building and not once per card.
  List<Section> sectionsOf(Note note) =>
      (reads.sections ??= notes.sectionsOf(ids()))[note.id] ?? const [];
  // Their Checklists alike, each as it shows now, for its Occurrence
  // (#121).
  final lists = Checklists(notes.store);
  Checklist? checklistOf(Note note) =>
      (reads.checklists ??= lists.ofNotesNow(ids(), deviceWallClock))[note.id];
  // And their Shopping Lists, as they show today (#122).
  final shopping = ShoppingLists(notes.store);
  ShoppingList? shoppingOf(Note note) =>
      (reads.shopping ??= shopping.ofNotes(ids(), deviceWallClock))[note.id];
  // What their compact cards say of a Tally or a Session (#189), read
  // when the first compact card asks.
  final glances = CardGlances.inContext(context, notes.store);
  // Today's Occurrences of their Routines and Reminders (#215), read in
  // one pass when the first card asks.
  final routines = Routines(notes.store);
  List<AgendaRow> todayOf(Note note) =>
      (reads.today ??= routines.todayByNote(deviceWallClock))[note.id] ??
      const [];
  // The Notes Hidden from AI (#267), read once for the list and kept by
  // the core until the log changes; without a mark no card walks up from
  // its Note.
  bool hiddenFromAi(Note note) =>
      (reads.hidden ??= notes.store.hiddenFromAiAmong(ids())).contains(note.id);
  // The Notes linking to each full card's Note (obf-1035), from what the
  // core keeps until the log changes; a closed locked Area's left out.
  final backlinks = Backlinks(notes.store);
  final lock = LockScope.of(context);
  List<Note> linkedFrom(Note note) => reads.linked[note.id] ??= backlinks.to(
    note.id,
    closed: reads.closed ??= lock?.closedAreas ?? const <String>{},
  );
  bool full(Note note) => compact == switched.contains(note.id);
  // The places found, with the Sections the list read or else the
  // Note's own: a search with many hits reads only those built.
  final query = find.trim();
  if (reads.findQuery != query) {
    reads.findings.clear();
    reads.findQuery = query;
  }
  List<Finding> findingsOf(Note note) =>
      reads.findings[note.id] ??= Findings.ofNote(
        note,
        query,
        sections: reads.sections?[note.id] ?? notes.sections(note.id),
      );
  return [
    for (final placed in shown)
      if (placed case (
        :final note,
        :final area,
        :final topic,
        :final changes,
        :final schedules,
        amends: _,
        :final author,
      ))
        // Built as it scrolls into sight: a card out of sight reads
        // nothing.
        _FoundCard(
          read: query.isEmpty || onFind == null ? null : () => findingsOf(note),
          noteId: note.id,
          card: (found) => NoteCard(
            note: note,
            when: when,
            area: area,
            topic: topic,
            author: author,
            showPlace: showPlace(placed),
            // What a Member may not do to it is not offered (ADR 0013).
            onFile: changes ? () => onFile(note) : null,
            onMerge: changes ? () => onMerge(note) : null,
            onSchedule: onSchedule == null || !schedules
                ? null
                : (spawn) => onSchedule(note, spawn),
            actions: noteActions(
              context,
              notes,
              placed,
              onChanged,
              onEdit: switch ((found.firstOrNull, onFind)) {
                (final at?, final find?) => () => find(note, at),
                _ => () => onOpen(note),
              },
              // Read once for the list, not for every card (obf-1069),
              // the Pack badges of all its Notes in one query (obf-1068).
              advanced: reads.advanced ??= notes.store.advanced,
              packFrom: () =>
                  (reads.packFrom ??= notes.store.packFromsOf(ids()))[note.id],
            ),
            full: full(note),
            sections: () => sectionsOf(note),
            linkedFrom: full(note) ? linkedFrom(note) : const [],
            checklist: (read: () => checklistOf(note), lists: lists),
            shopping: (read: () => shoppingOf(note), lists: shopping),
            tally: notes,
            session: notes,
            kind: () =>
                reads.kinds[note.id] ??= NoteKinds(notes.store).of(note.id),
            glance: () => reads.glances[note.id] ??= glances.of(note.id),
            boxes: boxTicks(notes, placed, onChanged),
            onTable: tableEdit(notes, placed, onChanged),
            occurrences: (read: () => todayOf(note), routines: routines),
            onTap: switch ((found.firstOrNull, onFind)) {
              (final at?, final find?) => () => find(note, at),
              _ => onSwitch == null ? null : () => onSwitch(note),
            },
            onEdit: switch ((found.firstOrNull, onFind)) {
              (final at?, final find?) => () => find(note, at),
              _ => () => onOpen(note),
            },
            hiddenFromAi: hiddenFromAi(note),
            onSelect: selected == null || onSelect == null || !changes
                ? null
                : (on) => onSelect(note, on),
            selected: selected?.contains(note.id) ?? false,
            findings: found,
            onFinding: onFind == null ? null : (at) => onFind(note, at),
          ),
        ),
  ];
}

/// A card of a list, built as it scrolls into sight: with [read], the
/// places a search found in its Note [noteId] (obf-1047), read once it
/// is first built rather than while the list is, and drawn on a tint by
/// the texts below it ([FindScope]).
class _FoundCard extends StatefulWidget {
  final List<Finding> Function()? read;
  final String noteId;
  final Widget Function(List<Finding> found) card;

  const _FoundCard({
    required this.read,
    required this.noteId,
    required this.card,
  });

  @override
  State<_FoundCard> createState() => _FoundCardState();
}

class _FoundCardState extends State<_FoundCard> {
  List<Finding> _found = const [];
  Map<(String, FindingPart, String), FoundMarks> _marks = const {};

  @override
  void initState() {
    super.initState();
    _read();
  }

  @override
  void didUpdateWidget(_FoundCard old) {
    super.didUpdateWidget(old);
    // Kept with the list: read anew only for a new list or search.
    _read();
  }

  void _read() {
    final found = widget.read?.call() ?? const <Finding>[];
    if (identical(found, _found)) return;
    _found = found;
    _marks = foundMarksOf(widget.noteId, found);
  }

  @override
  Widget build(BuildContext context) {
    return FindScope(marks: _marks, child: widget.card(_found));
  }
}

/// What the cards of one list read from the store, kept with the list
/// as it was loaded (obf-1069) for its day and language: a list loaded
/// anew starts with nothing read.
class _ListReads {
  final Day day;
  final String locale;

  _ListReads(this.day, this.locale);

  static final _kept = Expando<_ListReads>('diary reads');

  /// The reads kept for [shown], or none read yet when it is another
  /// day or language than they were read for.
  static _ListReads of(
    List<PlacedNote> shown, {
    required Day day,
    required String locale,
  }) {
    final kept = _kept[shown];
    if (kept != null && kept.day == day && kept.locale == locale) return kept;
    return _kept[shown] = _ListReads(day, locale);
  }

  Map<String, List<Section>>? sections;
  Map<String, Checklist>? checklists;
  Map<String, ShoppingList>? shopping;
  Map<String, List<AgendaRow>>? today;
  Set<String>? hidden;
  Set<String>? closed;
  bool? advanced;
  final linked = <String, List<Note>>{};
  final kinds = <String, NoteKind>{};
  final glances = <String, List<String>>{};
  Map<String, ({String id, String name})>? packFrom;

  /// The places the search [findQuery] found in each Note (obf-1047).
  String findQuery = '';
  final findings = <String, List<Finding>>{};
}

/// The app bar buttons of a diary: order by time or by Importance, and
/// hide or show the unimportant Notes. Each says what tapping it does.
List<Widget> diaryActions(
  BuildContext context, {
  required DiaryOrder order,
  required bool hideUnimportant,
  required ValueChanged<DiaryOrder> onOrder,
  required ValueChanged<bool> onHideUnimportant,
}) {
  final t = context.t;
  final byImportance = order == DiaryOrder.importance;
  return [
    IconButton(
      tooltip: byImportance ? t.orderByTime : t.orderByImportance,
      isSelected: byImportance,
      onPressed: () =>
          onOrder(byImportance ? DiaryOrder.time : DiaryOrder.importance),
      icon: const Icon(Icons.sort),
      selectedIcon: const Icon(Icons.label_important_outline),
    ),
    IconButton(
      tooltip: hideUnimportant ? t.showUnimportant : t.hideUnimportant,
      isSelected: hideUnimportant,
      onPressed: () => onHideUnimportant(!hideUnimportant),
      icon: const Icon(Icons.visibility_outlined),
      selectedIcon: const Icon(Icons.visibility_off_outlined),
    ),
  ];
}

/// The app bar button of the person filter (#53): a menu at the button
/// with "Me", every person of People and, while it filters, "Everyone";
/// each choice puts that person in or out. It shows as selected while
/// it filters.
class PersonFilterButton extends StatelessWidget {
  final Store store;
  final PersonFilter filter;
  final ValueChanged<PersonFilter> onChanged;

  const PersonFilterButton({
    super.key,
    required this.store,
    required this.filter,
    required this.onChanged,
  });

  Future<void> _open(BuildContext button) async {
    final t = button.t;
    final box = button.findRenderObject() as RenderBox?;
    final overlay = Overlay.of(button).context.findRenderObject() as RenderBox?;
    if (box == null || overlay == null) return;
    final picked = await showMenu<PersonFilter>(
      context: button,
      position: RelativeRect.fromRect(
        box.localToGlobal(Offset.zero, ancestor: overlay) & box.size,
        Offset.zero & overlay.size,
      ),
      items: [
        // The heading the choices continue: "Show Notes by…" "Me".
        PopupMenuItem(enabled: false, child: Text(t.personFilter)),
        CheckedPopupMenuItem(
          key: const ValueKey('personFilter-me'),
          value: filter.toggledMe(),
          checked: filter.me,
          child: Text(t.personFilterMe),
        ),
        for (final person in store.peopleShown)
          CheckedPopupMenuItem(
            key: ValueKey('personFilter-${person.id}'),
            value: filter.toggled(person.id),
            checked: filter.people.contains(person.id),
            child: Row(
              children: [
                ProfileAvatar(profile: person, size: 24),
                const SizedBox(width: 8),
                Flexible(child: Text(profileDisplayName(t, person.name))),
              ],
            ),
          ),
        if (!filter.isEveryone)
          PopupMenuItem(
            key: const ValueKey('personFilter-everyone'),
            value: PersonFilter.everyone,
            child: Text(t.personFilterEveryone),
          ),
      ],
    );
    if (picked != null) onChanged(picked);
  }

  @override
  Widget build(BuildContext context) => Builder(
    builder: (button) => IconButton(
      key: const ValueKey('personFilter'),
      tooltip: context.t.personFilter,
      isSelected: !filter.isEveryone,
      onPressed: () => _open(button),
      icon: const Icon(Icons.people_outline),
      selectedIcon: const Icon(Icons.people),
    ),
  );
}

/// Who the person filter keeps, as chips under the search field, each
/// with ✕ to take that person out; nothing while it keeps everyone, so
/// Notes never leave the list unseen.
class PersonFilterChips extends StatefulWidget {
  final Store store;
  final PersonFilter filter;
  final ValueChanged<PersonFilter> onChanged;

  const PersonFilterChips({
    super.key,
    required this.store,
    required this.filter,
    required this.onChanged,
  });

  @override
  State<PersonFilterChips> createState() => _PersonFilterChipsState();
}

class _PersonFilterChipsState extends State<PersonFilterChips> {
  /// The people of the filter as they show, read when the filter or
  /// their Profiles change rather than on every build, so a picture
  /// keeps its bytes and is not decoded again.
  Map<String, Profile> _shown = const {};

  @override
  void initState() {
    super.initState();
    widget.store.addProfileListener(_changed);
    _read();
  }

  @override
  void didUpdateWidget(PersonFilterChips old) {
    super.didUpdateWidget(old);
    if (old.store != widget.store) {
      old.store.removeProfileListener(_changed);
      widget.store.addProfileListener(_changed);
    }
    if (old.store != widget.store || old.filter != widget.filter) _read();
  }

  @override
  void dispose() {
    widget.store.removeProfileListener(_changed);
    super.dispose();
  }

  void _changed() {
    if (!mounted) return;
    setState(_read);
  }

  void _read() {
    final store = widget.store;
    _shown = {for (final id in widget.filter.people) id: store.personShown(id)};
  }

  @override
  Widget build(BuildContext context) {
    final filter = widget.filter;
    final onChanged = widget.onChanged;
    if (filter.isEveryone) return const SizedBox.shrink();
    final t = context.t;
    final people = filter.people.toList()..sort();
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 8, 16, 0),
      child: Wrap(
        spacing: 8,
        runSpacing: 4,
        crossAxisAlignment: WrapCrossAlignment.center,
        children: [
          Text(t.personFilterShowing),
          if (filter.me)
            InputChip(
              key: const ValueKey('personChip-me'),
              label: Text(t.personFilterMe),
              onDeleted: () => onChanged(filter.toggledMe()),
            ),
          for (final id in people)
            if (_shown[id] case final person?)
              InputChip(
                key: ValueKey('personChip-$id'),
                avatar: ProfileAvatar(profile: person, size: 24),
                label: Text(profileDisplayName(t, person.name)),
                onDeleted: () => onChanged(filter.toggled(id)),
              ),
        ],
      ),
    );
  }
}

/// A line of text on an embossed panel, for a level with nothing to show.
class BrowserEmpty extends StatelessWidget {
  final String text;

  const BrowserEmpty({super.key, required this.text});

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.all(24),
    child: Embossed(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Text(text, textAlign: TextAlign.center),
      ),
    ),
  );
}
