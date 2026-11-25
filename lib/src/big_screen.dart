import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'app_lock.dart';
import 'area_look.dart';
import 'areas_screen.dart';
import 'browser.dart';
import 'frogs_screen.dart';
import 'home_pages.dart';
import 'l10n.dart';
import 'notes_screen.dart';
import 'nudge_screen.dart';
import 'place_names.dart';
import 'tag_tree_screen.dart';
import 'trash_screen.dart';

/// The width from which the app takes the desktop's layout (obf-1148): a
/// sidebar beside the list, a Note opening over it as a dialog.
const bigScreenWidth = 700.0;

/// How wide the sidebar is.
const sidebarWidth = 240.0;

/// How wide the Note dialog is at most.
const noteDialogWidth = 840.0;

/// From this height the Note dialog keeps 32 from the window's top and
/// bottom, below it 8.
const _tallWindow = 600.0;

/// The kinds of place the sidebar leads to.
enum PlaceKind {
  agenda,
  unsorted,
  allNotes,
  frogs,
  areas,
  area,
  topic,
  tags,
  trash,
}

/// A place the list shows, which the sidebar marks: an Area and a Topic
/// with their id.
@immutable
class ShownPlace {
  final PlaceKind kind;
  final String? id;

  const ShownPlace(this.kind) : id = null;

  const ShownPlace.area(String this.id) : kind = PlaceKind.area;

  const ShownPlace.topic(String this.id) : kind = PlaceKind.topic;

  @override
  bool operator ==(Object other) =>
      other is ShownPlace && other.kind == kind && other.id == id;

  @override
  int get hashCode => Object.hash(kind, id);
}

/// Whether the big-screen layout shows now (obf-1148), with what its
/// sidebar opens places with. Above the app's navigator, so every route
/// reads it.
class BigScreen extends InheritedWidget {
  final bool on;
  final Notes notes;
  final Places places;
  final BrowserTrail trail;
  final HomePager pager;

  const BigScreen({
    super.key,
    required this.on,
    required this.notes,
    required this.places,
    required this.trail,
    required this.pager,
    required super.child,
  });

  /// Whether the big-screen layout shows now; false where there is none,
  /// a page on its own in a test.
  static bool of(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<BigScreen>()?.on ?? false;

  /// The scope, read once, so it does not listen.
  static BigScreen? scopeOf(BuildContext context) =>
      context.getInheritedWidgetOfExactType<BigScreen>();

  @override
  bool updateShouldNotify(BigScreen old) =>
      on != old.on ||
      notes != old.notes ||
      places != old.places ||
      trail != old.trail ||
      pager != old.pager;
}

/// Puts [BigScreen] over [child], on while [allowed] and the window is at
/// least [bigScreenWidth] wide.
class BigScreenScope extends StatelessWidget {
  final bool allowed;
  final Notes notes;
  final Places places;
  final BrowserTrail trail;
  final HomePager pager;
  final Widget child;

  const BigScreenScope({
    super.key,
    required this.allowed,
    required this.notes,
    required this.places,
    required this.trail,
    required this.pager,
    required this.child,
  });

  @override
  Widget build(BuildContext context) => BigScreen(
    on: allowed && MediaQuery.sizeOf(context).width >= bigScreenWidth,
    notes: notes,
    places: places,
    trail: trail,
    pager: pager,
    child: child,
  );
}

/// [child] beside the sidebar on the big screen, marking [shown] or,
/// with [homePage], the first page shown; on the phone [child] alone.
/// The page keeps its state as the layout changes, so turning the
/// screen across [bigScreenWidth] keeps the place shown.
class BesideSidebar extends StatelessWidget {
  final ShownPlace? shown;
  final ValueListenable<int>? homePage;
  final Widget child;

  const BesideSidebar({
    super.key,
    this.shown,
    this.homePage,
    required this.child,
  });

  @override
  Widget build(BuildContext context) {
    final scope = context.dependOnInheritedWidgetOfExactType<BigScreen>();
    final on = scope?.on ?? false;
    return Row(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        if (scope != null && on)
          SizedBox(
            key: const ValueKey('sidebar'),
            width: sidebarWidth,
            child: Sidebar(scope: scope, shown: shown, homePage: homePage),
          ),
        Expanded(
          key: const ValueKey('sidebar-content'),
          // The list takes the room left of the window as its own, the
          // sidebar its edge.
          child: on
              ? MediaQuery(data: _beside(MediaQuery.of(context)), child: child)
              : child,
        ),
      ],
    );
  }

  /// The window's [media] for the list beside the sidebar.
  static MediaQueryData _beside(MediaQueryData media) => media.copyWith(
    size: Size(media.size.width - sidebarWidth, media.size.height),
    padding: media.padding.copyWith(left: 0),
    viewPadding: media.viewPadding.copyWith(left: 0),
  );
}

/// A page of a place the sidebar leads to: on the big screen the sidebar
/// stands beside it and stays put as it comes and goes, marking [place].
class PlaceRoute<T> extends MaterialPageRoute<T> {
  final ShownPlace place;

  PlaceRoute({required this.place, required super.builder});

  @override
  Widget buildTransitions(
    BuildContext context,
    Animation<double> animation,
    Animation<double> secondaryAnimation,
    Widget child,
  ) => BesideSidebar(
    shown: place,
    child: ClipRect(
      child: super.buildTransitions(
        context,
        animation,
        secondaryAnimation,
        child,
      ),
    ),
  );
}

/// The route the Note editor opens in (obf-1148): over the list as a
/// dialog on the big screen, else a page of its own.
Route<T> noteRoute<T>(BuildContext context, WidgetBuilder builder) =>
    (BigScreen.scopeOf(context)?.on ?? false)
    ? NoteDialogRoute<T>(context: context, builder: builder)
    : MaterialPageRoute<T>(builder: builder);

/// A Note's editor as a dialog over the list and the sidebar, as the
/// desktop's Note dialog (obf-1148). A tap beside it leaves the editor as
/// back does, keeping what was typed.
class NoteDialogRoute<T> extends DialogRoute<T> {
  NoteDialogRoute({required super.context, required WidgetBuilder builder})
    : super(
        builder: (context) => _NoteDialog(child: builder(context)),
        barrierLabel: MaterialLocalizations.of(context)
            .modalBarrierDismissLabel,
      );
}

class _NoteDialog extends StatelessWidget {
  final Widget child;

  const _NoteDialog({required this.child});

  @override
  Widget build(BuildContext context) {
    final window = MediaQuery.sizeOf(context);
    return Dialog(
      // A window short or narrow, such as a phone's, loses little to the
      // edges.
      insetPadding: EdgeInsets.symmetric(
        horizontal: window.width >= bigScreenWidth ? 32 : 8,
        vertical: window.height >= _tallWindow ? 32 : 8,
      ),
      clipBehavior: Clip.antiAlias,
      child: SizedBox(
        width: noteDialogWidth,
        height: double.infinity,
        // The editor lays itself out in the dialog's room, not the
        // window's.
        child: LayoutBuilder(
          builder: (context, box) => MediaQuery(
            data: MediaQuery.of(context).copyWith(size: box.biggest),
            child: child,
          ),
        ),
      ),
    );
  }
}

/// The big screen's sidebar (obf-1148), as the desktop's: the Agenda,
/// Unsorted with its count, All Notes, Frogs, the Areas list with each
/// Area and its Topics, the tag tree and the Trash, marking the place
/// shown. A closed locked Area lists no Topics, and opens with its PIN.
class Sidebar extends StatefulWidget {
  /// What it reads and opens places with.
  final BigScreen scope;
  final ShownPlace? shown;

  /// The first page's, which marks the Agenda or the Areas list.
  final ValueListenable<int>? homePage;

  const Sidebar({super.key, required this.scope, this.shown, this.homePage});

  @override
  State<Sidebar> createState() => _SidebarState();
}

class _SidebarState extends State<Sidebar> with ReloadsOnLogChange {
  List<Area> _areas = const [];
  Map<String, List<Topic>> _topics = const {};
  Set<String> _closed = const {};
  int _unsorted = 0;
  AppLock? _lock;
  var _loaded = false;

  @override
  Store get log => widget.scope.places.store;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final lock = LockScope.of(context);
    if (lock != _lock) {
      _lock?.removeListener(_reload);
      _lock = lock?..addListener(_reload);
    }
    if (_loaded) return;
    _load();
    _loaded = true;
  }

  @override
  void dispose() {
    _lock?.removeListener(_reload);
    super.dispose();
  }

  @override
  void reloadLog() => _reload();

  void _reload() {
    if (mounted) setState(_load);
  }

  /// Reads the Areas in the person's own order, their Topics by name,
  /// the closed locked Areas and how many Notes are Unsorted.
  void _load() {
    final scope = widget.scope;
    final t = context.t;
    final store = scope.places.store;
    _areas = store.areasInOrder(shown: (area) => areaName(t, area));
    _closed = {
      for (final area in _areas)
        if (_lock?.isAreaClosed(area.id) ?? false) area.id,
    };
    _topics = {
      for (final area in _areas)
        if (!_closed.contains(area.id))
          area.id: scope.places.topics(area.id)
            ..sort(
              (a, b) => topicName(
                t,
                a,
              ).toLowerCase().compareTo(topicName(t, b).toLowerCase()),
            ),
    };
    _unsorted = scope.notes.unsortedCount();
  }

  /// Shows [place] in the list, from the first page up: the pages on top
  /// of it go, as the desktop's sidebar shows a place anew.
  Future<void> _go(ShownPlace place) async {
    final scope = widget.scope;
    final areaId = switch (place.kind) {
      PlaceKind.area => place.id,
      PlaceKind.topic => scope.places.topicById(place.id ?? '')?.areaId,
      _ => null,
    };
    final area = areaId == null ? null : scope.places.areaById(areaId);
    if (area != null &&
        !await openLockedArea(
          context,
          _lock,
          area.id,
          areaName(context.t, area),
        )) {
      return;
    }
    if (!mounted) return;
    final navigator = Navigator.of(context);
    navigator.popUntil((route) => route.isFirst);
    final notes = scope.notes;
    final places = scope.places;
    final trail = scope.trail;
    final pager = scope.pager;
    void push(Route<void> route) => unawaited(navigator.push(route));
    switch (place.kind) {
      case PlaceKind.agenda:
        pager.jumpToAgenda();
      case PlaceKind.areas:
        pager.jumpToAreas();
      case PlaceKind.unsorted:
        push(unsortedRoute(notes, places));
      case PlaceKind.allNotes:
        push(allNotesRoute(notes, places));
      case PlaceKind.frogs:
        push(frogsRoute(notes, places));
      case PlaceKind.trash:
        push(trashRoute(notes, places));
      case PlaceKind.tags:
        final tag = await navigator.push(tagTreeRoute(places.store));
        if (tag == null || !navigator.mounted) return;
        // As a tag tapped in a text: the Areas list searching for it.
        pager.jumpToAreas();
        push(
          BrowserRoute<void>(
            place: const ShownPlace(PlaceKind.areas),
            builder: (_) => AreasScreen(
              notes: notes,
              places: places,
              trail: trail,
              query: tag,
            ),
          ),
        );
      case PlaceKind.area || PlaceKind.topic:
        if (area == null) return;
        // Over the Areas list, as on the phone, so back leads up.
        pager.jumpToAreas();
        trail.choseArea(area);
        push(
          BrowserRoute<void>(
            place: ShownPlace.area(area.id),
            builder: (_) => AreaScreen(
              notes: notes,
              places: places,
              trail: trail,
              areaId: area.id,
            ),
          ),
        );
        if (place.kind != PlaceKind.topic) return;
        final topic = places.topicById(place.id ?? '');
        if (topic == null) return;
        trail.choseTopic(topic);
        push(
          BrowserRoute<void>(
            place: ShownPlace.topic(topic.id),
            builder: (_) =>
                TopicScreen(notes: notes, places: places, topicId: topic.id),
          ),
        );
    }
  }

  Widget _entry(
    ShownPlace place,
    ShownPlace? shown, {
    required Widget icon,
    required String label,
    Widget? trailing,
    double indent = 0,
  }) {
    final selected = place == shown;
    final scheme = Theme.of(context).colorScheme;
    return Padding(
      padding: EdgeInsetsDirectional.only(start: 8 + indent, end: 8),
      child: ListTile(
        key: ValueKey(switch (place.id) {
          final id? => 'sidebar-${place.kind.name}-$id',
          null => 'sidebar-${place.kind.name}',
        }),
        dense: true,
        selected: selected,
        // The place shown, its fill tinted with the accent, as the
        // desktop's sidebar marks it.
        selectedColor: scheme.onSurface,
        selectedTileColor: scheme.primary.withValues(alpha: 0.2),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
        leading: icon,
        minLeadingWidth: 24,
        title: Text(label, maxLines: 1, overflow: TextOverflow.ellipsis),
        trailing: trailing,
        onTap: () => unawaited(_go(place)),
      ),
    );
  }

  Widget _list(ShownPlace? shown) {
    final t = context.t;
    final weak = Theme.of(context).colorScheme.onSurfaceVariant;
    return ListView(
      padding: const EdgeInsets.symmetric(vertical: 8),
      children: [
        _entry(
          const ShownPlace(PlaceKind.agenda),
          shown,
          icon: const Icon(Icons.calendar_today_outlined),
          label: t.sidebarAgenda,
        ),
        _entry(
          const ShownPlace(PlaceKind.unsorted),
          shown,
          icon: const Icon(Icons.inbox_outlined),
          label: t.unsorted,
          // Only Unsorted asks to be emptied, so only it counts.
          trailing: _unsorted == 0 ? null : Text('$_unsorted'),
        ),
        _entry(
          const ShownPlace(PlaceKind.allNotes),
          shown,
          icon: const Icon(Icons.notes),
          label: t.allNotesTitle,
        ),
        _entry(
          const ShownPlace(PlaceKind.frogs),
          shown,
          icon: const FrogIcon(),
          label: t.frogsTitle,
        ),
        _entry(
          const ShownPlace(PlaceKind.areas),
          shown,
          icon: const Icon(Icons.folder_outlined),
          label: t.areasTitle,
        ),
        for (final area in _areas) ...[
          _entry(
            ShownPlace.area(area.id),
            shown,
            indent: 8,
            icon: Icon(
              areaIconData(area.icon) ?? Icons.folder,
              color:
                  areaColour(area.colour) ??
                  Theme.of(context).colorScheme.outline,
            ),
            label: areaName(t, area),
            trailing: _closed.contains(area.id)
                ? Icon(
                    Icons.lock_outline,
                    color: weak,
                    semanticLabel: t.areaLocked,
                  )
                : null,
          ),
          for (final topic in _topics[area.id] ?? const <Topic>[])
            _entry(
              ShownPlace.topic(topic.id),
              shown,
              indent: 48,
              icon: const SizedBox.shrink(),
              label: topicName(t, topic),
            ),
        ],
        _entry(
          const ShownPlace(PlaceKind.tags),
          shown,
          icon: const Icon(Icons.tag),
          label: t.tagsTitle,
        ),
        _entry(
          const ShownPlace(PlaceKind.trash),
          shown,
          // A bin with an arrow up opens the Trash; the plain bin deletes
          // (#166).
          icon: const Icon(Icons.restore_from_trash),
          label: t.trashTitle,
        ),
      ],
    );
  }

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final homePage = widget.homePage;
    return Material(
      color: searchHeaderColour(context),
      child: DecoratedBox(
        decoration: BoxDecoration(
          border: BorderDirectional(
            end: BorderSide(color: scheme.outlineVariant),
          ),
        ),
        child: SafeArea(
          right: false,
          child: homePage == null
              ? _list(widget.shown)
              : ValueListenableBuilder<int>(
                  valueListenable: homePage,
                  builder: (_, page, _) => _list(
                    ShownPlace(
                      page == HomePager.areas
                          ? PlaceKind.areas
                          : PlaceKind.agenda,
                    ),
                  ),
                ),
        ),
      ),
    );
  }
}

/// The Unsorted list's page.
PlaceRoute<void> unsortedRoute(Notes notes, Places places) => PlaceRoute(
  place: const ShownPlace(PlaceKind.unsorted),
  builder: (_) => UnsortedScreen(notes: notes, places: places),
);

/// All Notes' page.
PlaceRoute<void> allNotesRoute(Notes notes, Places places) => PlaceRoute(
  place: const ShownPlace(PlaceKind.allNotes),
  builder: (_) => AllNotesScreen(notes: notes, places: places),
);

/// The Frog view's page.
PlaceRoute<void> frogsRoute(Notes notes, Places places) => PlaceRoute(
  place: const ShownPlace(PlaceKind.frogs),
  builder: (_) => FrogsScreen(notes: notes, places: places),
);

/// The Trash's page.
PlaceRoute<void> trashRoute(Notes notes, Places places) => PlaceRoute(
  place: const ShownPlace(PlaceKind.trash),
  builder: (_) => TrashScreen(notes: notes, places: places),
);

/// The tag tree's page; it closes with the tag tapped.
PlaceRoute<String> tagTreeRoute(Store store) => PlaceRoute(
  place: const ShownPlace(PlaceKind.tags),
  builder: (_) => TagTreeScreen(store: store),
);
