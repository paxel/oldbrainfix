import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/scheduler.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'l10n.dart';
import 'look.dart';
import 'note_card.dart';
import 'note_editor_screen.dart';
import 'profile.dart';

/// Reloads a page's list once after the log changed, never in the middle
/// of whatever changed it, so what a page opened on top sorted, like the
/// Nudge screen from its notification, or what a sync brought, is gone
/// or there once the page shows again.
mixin ReloadsOnLogChange<T extends StatefulWidget> on State<T> {
  /// The log whose changes the page follows.
  Store get log;

  /// Reads the list again and shows it.
  void reloadLog();

  var _reloadQueued = false;

  @override
  void initState() {
    super.initState();
    log.addListener(_logChanged);
  }

  @override
  void dispose() {
    log.removeListener(_logChanged);
    super.dispose();
  }

  void _logChanged() {
    if (_reloadQueued) return;
    _reloadQueued = true;
    scheduleMicrotask(() {
      _reloadQueued = false;
      if (mounted) reloadLog();
    });
  }
}

/// A Note list's full/compact switch (#114): full unless switched to
/// compact, remembered per list in a setting of this Device, and the
/// Notes a tap unfolded while compact, for as long as the page shows.
mixin FoldsNoteCards<T extends StatefulWidget> on State<T> {
  /// The log whose Device settings keep the choice.
  Store get log;

  /// Which list this is, e.g. `area:<id>`; each keeps its own choice.
  String get cardsKey;

  /// Whether the cards show compact, by their first line.
  bool compact = false;

  /// The Notes shown in full while [compact].
  Set<String> unfolded = const {};

  String get _setting => 'compactNotes:$cardsKey';

  @override
  void initState() {
    super.initState();
    readCompact();
  }

  /// Reads the choice again, as another page showing the same list may
  /// have changed it.
  void readCompact() => compact = log.localSetting(_setting) == 'true';

  /// Switches the list to compact or full and remembers it; what was
  /// unfolded folds again.
  void setCompact(bool on) {
    setState(() {
      compact = on;
      unfolded = const {};
    });
    // Nothing kept means full, the default.
    on
        ? log.setLocalSetting(_setting, 'true')
        : log.removeLocalSetting(_setting);
  }

  /// Shows [note] in full, in place.
  void unfold(Note note) => setState(() => unfolded = {...unfolded, note.id});
}

/// The bar button of a Note list's full/compact switch: it says what
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

/// The page of a browser level (Areas list, Area, Topic): it slides in
/// from the right while the level it covers slides out to the left, side
/// by side, since the pages are transparent over the question marks. A
/// swipe moves both with the finger ([BrowserSwipes]). With the
/// system's animations off the change is immediate.
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
  /// page is no browser level itself: the Agenda.
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
/// selected Area or Topic, when there is one. The Agenda is the level
/// above the Areas list: there swipe left opens it and swipe right,
/// with [back] false, does nothing. Wraps a level's whole page.
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

  /// Whether swipe right leaves this level.
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
  Widget build(BuildContext context) => GestureDetector(
    // Empty space swipes too, not only the cards and rows.
    behavior: HitTestBehavior.opaque,
    onHorizontalDragStart: _started,
    onHorizontalDragUpdate: _updated,
    onHorizontalDragEnd: _ended,
    onHorizontalDragCancel: _cancelled,
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

/// The search field on top of a browser level, with ✕ to clear it.
class BrowserSearchField extends StatelessWidget {
  final TextEditingController controller;
  final String hint;
  final ValueChanged<String> onChanged;

  const BrowserSearchField({
    super.key,
    required this.controller,
    required this.hint,
    required this.onChanged,
  });

  void _clear() {
    controller.clear();
    onChanged('');
  }

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.fromLTRB(16, 8, 16, 0),
    child: TextField(
      controller: controller,
      onChanged: onChanged,
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
  );
}

/// Notes as a diary of cards, each with "file into…", "merge into…",
/// with [onSchedule] Routine, Reminder, Frog or Session, and ⋮ for the
/// rest the Note editor offers, as far as this Device may. Each shows in
/// full unless [compact]; a tap on a compact one runs [onUnfold], and
/// the Notes in [unfolded] show in full. A long press or Edit in ⋮ opens
/// the editor (#114).
/// [onChanged] runs after a step of ⋮ that may have changed a Note.
/// [showPlace] says which cards name where they are filed: those the
/// screen does not name already.
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
  Set<String> unfolded = const {},
  void Function(Note note)? onUnfold,
}) {
  final when = DateFormat.yMd(Localizations.localeOf(context).toString())
      .add_Hm();
  // The Sections of every listed Note, read in one pass when the first
  // full card reads its own, not while building and not once per card.
  Map<String, List<Section>>? read;
  List<Section> sectionsOf(Note note) =>
      (read ??= notes.sectionsOf([
        for (final placed in shown) placed.note.id,
      ]))[note.id] ??
      const [];
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
        NoteCard(
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
            onEdit: () => onOpen(note),
          ),
          full: !compact || unfolded.contains(note.id),
          sections: () => sectionsOf(note),
          // A full Note has nothing to unfold (#114).
          onTap: !compact || unfolded.contains(note.id) || onUnfold == null
              ? null
              : () => onUnfold(note),
          onEdit: () => onOpen(note),
        ),
  ];
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
