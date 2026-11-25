import 'dart:async';
import 'dart:ui' show lerpDouble;

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'announcements.dart';
import 'app_lock.dart';
import 'area_deletion.dart';
import 'area_look.dart';
import 'big_screen.dart';
import 'browser.dart';
import 'browser_add.dart';
import 'failure_dialog.dart';
import 'file_into_screen.dart';
import 'frogs_screen.dart';
import 'hidden_from_ai_view.dart';
import 'home_pages.dart';
import 'imports_screen.dart';
import 'l10n.dart';
import 'look.dart';
import 'merge_into_screen.dart';
import 'note_card.dart';
import 'note_editor_screen.dart';
import 'note_selection.dart';
import 'pack_screen.dart';
import 'page_bar.dart';
import 'place_names.dart';
import 'profile.dart';
import 'routine_editor_screen.dart';
import 'run_over.dart';
import 'running.dart';
import 'share_area_screen.dart';
import 'spotlight.dart';
import 'system_bars.dart';
import 'tag_tree_screen.dart';
import 'tap_mark.dart';
import 'text_marks_view.dart';

/// The notes browser's first level: a search field over every Note, and
/// the Areas while it is empty, with "+" to add an Area. Tapping an Area
/// opens its Topics; the app bar leads to all Notes and the trash. Under
/// the empty field a row of the most used tags searches for one, and its
/// "All tags…" opens the tag tree (obf-1074).
/// [query] starts the search, as a tap on a tag does (ADR 0032).
class AreasScreen extends StatefulWidget {
  final Notes notes;
  final Places places;
  final BrowserTrail trail;
  final String query;

  /// Back to the Agenda, when this is the page beside it rather than a
  /// route of its own (#258): swipe right is then the pages', not this
  /// page's.
  final VoidCallback? onBack;

  /// The page of the two shown, when this is the page beside the Agenda:
  /// leaving it drops the selection (#273), as back there leaves too.
  final ValueListenable<int>? pageShown;

  const AreasScreen({
    super.key,
    required this.notes,
    required this.places,
    required this.trail,
    this.query = '',
    this.onBack,
    this.pageShown,
  });

  @override
  State<AreasScreen> createState() => _AreasScreenState();
}

class _AreasScreenState extends State<AreasScreen>
    with ReloadsOnLogChange, FoldsNoteCards, SelectsNotes, SearchesAfterPause {
  late final _query = TextEditingController(text: widget.query);

  /// The text [_hits] were searched for; the field's own may be newer.
  String _searched = '';
  List<Area> _areas = const [];

  /// What this Device may do in each Area, and who shares the shared
  /// ones (ADR 0013).
  Map<String, AreaAccess> _access = const {};
  Map<String, List<Person>> _people = const {};

  /// The marks Hidden from AI, of Areas and Notes alike (#267).
  Set<String> _hidden = const {};

  /// The name each person's Profile shows, by Profile id.
  Map<String, String> _names = const {};
  List<PlacedNote> _hits = const [];
  PersonFilter _filter = PersonFilter.everyone;
  bool _loaded = false;

  /// Whether the Areas show their handles (obf-1089): from Rearrange in
  /// ⋮ until Done, a search or leaving the page; a tap or a swipe on a row
  /// only moves it meanwhile.
  bool _rearranging = false;

  /// The Areas being deleted beside the app (obf-1093), whose rows say
  /// how far they are; none without the app.
  AreaDeletions? _deletions;

  /// The most used tags under the empty search field (obf-1074), without
  /// the Notes of the locked Areas closed, as every list. They are read
  /// only while this page shows with no search, as reading them reads
  /// every Note: beside the Agenda a change there would read them each
  /// time. [_tagsStale] says a change came meanwhile. Only the chip row
  /// follows them, so arriving here builds no more than that row.
  final _tags = ValueNotifier<List<TagNode>>(const []);
  var _tagsStale = true;

  /// What the chip row follows: the field, which hides it, and the tags.
  late final _tagRow = Listenable.merge([_query, _tags]);

  /// The app's lock, followed for the tags.
  AppLock? _lock;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final deletions = AreaDeletionsScope.of(context);
    if (deletions != _deletions) {
      _deletions?.removeListener(_deletionsChanged);
      _deletions = deletions?..addListener(_deletionsChanged);
    }
    final lock = LockScope.of(context);
    if (lock != _lock) {
      _lock?.removeListener(_locksChanged);
      _lock = lock?..addListener(_locksChanged);
      _tagsStale = true;
    }
    if (_loaded) return;
    _load();
    _loaded = true;
  }

  @override
  void initState() {
    super.initState();
    widget.pageShown?.addListener(_pageChanged);
  }

  /// An Area's deletion started, moved on or ended: its row shows it;
  /// the Area leaves the list as the log says it is gone.
  void _deletionsChanged() => setState(() {});

  @override
  void didUpdateWidget(AreasScreen old) {
    super.didUpdateWidget(old);
    if (old.pageShown != widget.pageShown) {
      old.pageShown?.removeListener(_pageChanged);
      widget.pageShown?.addListener(_pageChanged);
    }
  }

  @override
  void dispose() {
    widget.pageShown?.removeListener(_pageChanged);
    _deletions?.removeListener(_deletionsChanged);
    _lock?.removeListener(_locksChanged);
    _query.dispose();
    _tags.dispose();
    super.dispose();
  }

  /// The Agenda came in its place: the selection goes (#273).
  void _pageChanged() {
    if (widget.pageShown?.value != HomePager.areas) {
      stopSelecting();
      _stopRearranging();
    } else if (_tagsStale) {
      _readTags();
    }
  }

  /// Whether this page shows with an empty search field, the tags under
  /// it.
  bool get _tagsShown =>
      (widget.pageShown?.value ?? HomePager.areas) == HomePager.areas &&
      _query.text.trim().isEmpty;

  /// Reads the most used tags when they show, else notes that they wait.
  void _readTags() {
    if (!_tagsShown) {
      _tagsStale = true;
      return;
    }
    _tagsStale = false;
    _tags.value = mostUsedTags(
      TagTree(widget.places.store)
          .tree(closed: _lock?.closedAreas ?? const <String>{}),
    );
  }

  /// An Area opened or closed, or the app locked: the tags again. Not
  /// shown now, none stay until they are read, so a closed Area's tags
  /// never show on the way back.
  void _locksChanged() {
    if (!mounted) return;
    setState(() {
      _readTags();
      if (_tagsStale) _tags.value = const [];
    });
  }

  /// Leaving the page ends rearranging (obf-1089); each move is already
  /// kept.
  void _stopRearranging() {
    if (_rearranging) setState(() => _rearranging = false);
  }

  /// [child] under the selection's back, when this page is a route of
  /// its own; beside the Agenda back leaves for it, which drops the
  /// selection.
  Widget _selectionBack(Widget child) =>
      widget.onBack == null ? selectionBack(child) : child;

  /// The name an Area is shown by here, which orders those missing from
  /// the person's own order (#280).
  String _shown(Area area) => areaName(context.t, area);

  /// Reads the Areas, in the person's own order (#280), and the hits of
  /// the search.
  void _load() {
    final store = widget.places.store;
    _areas = store.areasInOrder(shown: _shown);
    _access = {for (final a in _areas) a.id: store.accessIn(a.id)};
    _hidden = store.hiddenFromAiMarks();
    _people = {
      for (final a in _areas)
        if (store.isShared(a.id)) a.id: store.people(a.id),
    };
    _names = {
      for (final people in _people.values)
        for (final p in people) p.id: store.personShown(p.id).name,
    };
    // Only a search shows Notes on this level; without one, reading
    // every Note for nothing made each change wait on all of them.
    _searched = _query.text;
    _hits = _searched.trim().isEmpty
        ? const []
        : _search(
            widget.places,
            _searched,
            lock: LockScope.of(context),
            people: _filter,
          );
    keepShown(_hits);
    // A search shows no Areas to move.
    if (_searched.trim().isNotEmpty) _rearranging = false;
    _readTags();
  }

  void _filtered(PersonFilter people) => setState(() {
    _filter = people;
    _load();
  });

  void _reload() => setState(_load);

  /// A letter typed: the search runs once typing pauses, an emptied
  /// field shows the Areas at once.
  void _typed(String query) {
    if (query.trim().isEmpty) return _set(query);
    searchAfterPause(_reload);
  }

  /// The search set as a whole runs now; with no hits to show the
  /// selection goes too.
  void _set(String query) {
    if (query.trim().isEmpty) stopSelecting();
    searchNow(_reload);
  }

  /// A tag tapped on a hit searches for it here.
  void _searchTag(BuildContext _, String query) {
    _query.text = query;
    searchNow(_reload);
  }

  @override
  Store get log => widget.places.store;

  /// The hits of the search over every Area.
  @override
  String get cardsKey => 'search';

  @override
  void reloadLog() => _reload();

  Future<void> _add() async {
    final name = await askName(context, title: context.t.newArea);
    if (!mounted || name == null) return;
    widget.places.addArea(name);
    _reload();
  }

  /// Opens the locked [area] first when it is closed; whether it is
  /// open now.
  Future<bool> _unlock(Area area) => openLockedArea(
    context,
    LockScope.of(context),
    area.id,
    areaName(context.t, area),
  );

  Future<void> _open(Area area) async {
    if (!await _unlock(area) || !mounted) return;
    await Navigator.of(context).push(_areaRoute(area));
  }

  /// [area]'s page, chosen as the last selected Area.
  BrowserRoute<void> _areaRoute(Area area) {
    widget.trail.choseArea(area);
    setState(() {});
    final route = BrowserRoute<void>(
      place: ShownPlace.area(area.id),
      builder: (_) => AreaScreen(
        notes: widget.notes,
        places: widget.places,
        trail: widget.trail,
        areaId: area.id,
      ),
    );
    // Fire and forget: popped never fails, and a failure in what runs
    // after it reaches the crash guard like any other callback's.
    unawaited(
      route.popped.then((_) {
        if (mounted) _reload();
      }),
    );
    return route;
  }

  /// The last selected Area's page for swipe left to pull in; none while
  /// that Area is closed, so its Notes never show before the PIN.
  BrowserRoute<void>? _forwardRoute() {
    final area = widget.trail.area(widget.places);
    if (area == null) return null;
    final closed = LockScope.of(context)?.isAreaClosed(area.id) ?? false;
    return closed ? null : _areaRoute(area);
  }

  Future<void> _forward() async {
    if (widget.trail.area(widget.places) case final area?) await _open(area);
  }

  Future<void> _openAllNotes() async {
    _stopRearranging();
    await Navigator.of(context)
        .push(allNotesRoute(widget.notes, widget.places));
    if (!mounted) return;
    _reload();
  }

  /// Opens the Frog view: the open Frogs and the eaten ones folded.
  Future<void> _openFrogs() async {
    _stopRearranging();
    await openFrogs(context, widget.notes, widget.places);
    if (!mounted) return;
    _reload();
  }

  /// Opens the tag tree, without the Notes of the locked Areas closed;
  /// a tag tapped there is searched for here.
  Future<void> _openTags() async {
    _stopRearranging();
    final tag = await Navigator.of(context)
        .push(tagTreeRoute(widget.places.store));
    if (!mounted) return;
    if (tag == null) return _reload();
    _searchTag(context, tag);
  }

  Future<void> _openTrash() async {
    _stopRearranging();
    await Navigator.of(context).push(trashRoute(widget.notes, widget.places));
    if (!mounted) return;
    _reload();
  }

  Future<void> _file(Note note) async {
    if (await fileNote(context, widget.notes, widget.places, note)) {
      if (!mounted) return;
      _reload();
    }
  }

  Future<void> _merge(Note note) async {
    if (await mergeNote(context, widget.notes, widget.places, note)) {
      if (!mounted) return;
      _reload();
    }
  }

  Future<void> _openNote(Note note) async {
    await openNote(context, widget.notes, widget.places, note);
    if (!mounted) return;
    _reload();
  }

  /// Opens [note] at [at], a place the search found in it, with the find
  /// bar for the search (obf-1047).
  Future<void> _openAt(Note note, Finding at) async {
    await openNote(
      context,
      widget.notes,
      widget.places,
      note,
      find: (query: _searched, at: at),
    );
    if (!mounted) return;
    _reload();
  }

  Future<void> _schedule(Note note, Spawn spawn) async {
    await spawnFromNote(
      context,
      widget.notes,
      widget.places,
      note,
      spawn: spawn,
    );
    if (!mounted) return;
    _reload();
  }

  Future<void> _act(Area area, _AreaAction action) async {
    if (!await _unlock(area) || !mounted) return;
    final t = context.t;
    switch (action) {
      case _AreaAction.rename:
        final shown = areaName(t, area);
        final name = await askName(context, title: t.rename, initial: shown);
        if (!mounted || name == null || name == shown) return;
        widget.places.rename(area.id, name);
      case _AreaAction.look:
        await Navigator.of(context).push(
          MaterialPageRoute<void>(
            builder: (_) =>
                AreaLookScreen(places: widget.places, areaId: area.id),
          ),
        );
        if (!mounted) return;
      case _AreaAction.delete:
        final go = await confirmDeleteArea(context, widget.places, area);
        if (!mounted || go == null) return;
        unawaited(_delete(area, go));
      case _AreaAction.share:
        // Nothing Hidden from AI goes to anyone (#267): asked before how.
        final store = widget.places.store;
        if (!_people.containsKey(area.id) &&
            !await showToAiFirst(
              context,
              store,
              store.hiddenFromAiIn(area.id),
              area: area.id,
            )) {
          return;
        }
        if (!mounted) return;
        await Navigator.of(context).push(
          MaterialPageRoute<bool>(
            builder: (_) =>
                ShareAreaScreen(store: widget.places.store, areaId: area.id),
          ),
        );
        if (!mounted) return;
      case _AreaAction.pack:
        await exportPack(context, widget.places.store, area.id);
        if (!mounted) return;
      case _AreaAction.people:
        await Navigator.of(context).push(
          MaterialPageRoute<void>(
            builder: (_) =>
                PeopleScreen(store: widget.places.store, areaId: area.id),
          ),
        );
        if (!mounted) return;
      case _AreaAction.hideFromAi:
        hiddenFromAiEntry(widget.places.store, area.id, () {})?.toggle();
    }
    _reload();
  }

  /// Deletes [area], its Notes going where [go] says, beside the app
  /// (obf-1093): its row shows how far it is meanwhile, and a failure is
  /// announced at the top in [AppLocalizations] taken before the wait,
  /// even after this page closed. Without the app, at once.
  Future<void> _delete(Area area, NotesGo go) async {
    final deletions = _deletions;
    if (deletions == null) {
      widget.places.deleteArea(area.id, go);
      return;
    }
    final t = context.t;
    final announcements = AnnouncementsScope.of(context);
    final navigator = Navigator.of(context, rootNavigator: true);
    final store = widget.places.store;
    try {
      await deletions.run(area.id, go);
    } on Object catch (e) {
      // A Restart ended this run of the app meanwhile: its store is
      // closed, and nobody is left to tell.
      if (runOver(store)) return;
      announceFailureWith(
        announcements,
        navigator,
        t.areaDeleteFailed,
        title: t.areaDeleteFailedTitle,
        cause: t.areaDeleteFailedCause,
        fix: t.areaDeleteFailedFix,
        details: '$e',
      );
    }
  }

  List<Widget> _areaRows(AppLocalizations t) {
    if (_areas.isEmpty) return [BrowserEmpty(text: t.noAreasYet)];
    final last = widget.trail.area(widget.places)?.id;
    final lock = LockScope.of(context);
    return [
      for (final (index, area) in _areas.indexed)
        if (_deletions?.of(area.id) case final deleting?)
          // Being deleted beside the app (obf-1093): how far it is, a
          // spinner where its menu was, nothing to tap or drag.
          _PlaceRow(
            key: ValueKey('area-row-${area.id}'),
            title: areaName(t, area),
            subtitle: switch (deleting.of) {
              final of? => t.areaDeleting(deleting.moved, of),
              null => t.areaDeletingStarted,
            },
            leading: _AreaMark(area: area),
            menu: SizedBox(
              key: ValueKey('area-deleting-${area.id}'),
              width: 48,
              height: 48,
              child: const Center(
                child: SizedBox.square(
                  dimension: 24,
                  child: CircularProgressIndicator(strokeWidth: 3),
                ),
              ),
            ),
          )
        else if (_rearranging)
          // Rearranging (obf-1089): a drag that starts anywhere on the row,
          // either way, takes it at once and moves it; a tap does nothing,
          // and the handle in ⋮'s place says so. The order is the
          // person's own, so every Area moves, a Shared one too (#280). A
          // screen reader moves it by the row's own actions, Move up and
          // Move down, so the handle says nothing.
          ReorderableDragStartListener(
            key: ValueKey('area-row-${area.id}'),
            index: index,
            child: _PlaceRow(
              title: areaName(t, area),
              subtitle: _sharedWith(t, area),
              leading: _AreaMark(area: area),
              locked: lock?.isAreaClosed(area.id) ?? false,
              shared: _people.containsKey(area.id),
              hiddenFromAi: _hidden.contains(area.id),
              highlighted: area.id == last,
              menu: SizedBox(
                key: ValueKey('area-move-${area.id}'),
                width: 48,
                height: 48,
                child: const Icon(Icons.drag_handle),
              ),
            ),
          )
        else
          _PlaceRow(
            key: ValueKey('area-row-${area.id}'),
            title: areaName(t, area),
            subtitle: _sharedWith(t, area),
            leading: _AreaMark(area: area),
            locked: lock?.isAreaClosed(area.id) ?? false,
            shared: _people.containsKey(area.id),
            hiddenFromAi: _hidden.contains(area.id),
            highlighted: area.id == last,
            onTap: () => _open(area),
            menu: _areaMenu(t, area),
          ),
    ];
  }

  /// Who a Shared [area] is shared with, under its name; null for one
  /// of the person's own.
  String? _sharedWith(AppLocalizations t, Area area) => _people[area.id] == null
      ? null
      : [
          for (final p in _people[area.id] ?? const <Person>[])
            profileDisplayName(t, _names[p.id] ?? ''),
        ].join(', ');

  /// The Area at [from] dragged to [to], counted after it was taken
  /// out: its new place, on all of the person's Devices (#280).
  void _moved(int from, int to) => setState(() {
    widget.places.store.moveArea(_areas[from].id, to, shown: _shown);
    _load();
  });

  /// What may be done to [area] here: everything by its owner or a
  /// Manager; by anyone else exporting it as a Pack (#164) and seeing who
  /// is in.
  Widget _areaMenu(AppLocalizations t, Area area) {
    final access = _access[area.id] ?? AreaAccess.owner;
    final manages = access == AreaAccess.owner || access == AreaAccess.manager;
    final shared = _people.containsKey(area.id);
    return PopupMenuButton<_AreaAction>(
      key: ValueKey('area-menu-${area.id}'),
      onSelected: (action) => _act(area, action),
      itemBuilder: (_) => [
        if (manages) ...[
          PopupMenuItem(value: _AreaAction.rename, child: Text(t.rename)),
          PopupMenuItem(value: _AreaAction.look, child: Text(t.areaLook)),
          PopupMenuItem(value: _AreaAction.share, child: Text(t.shareArea)),
        ],
        // Copies of what anyone may read (#164).
        PopupMenuItem(
          key: ValueKey('area-pack-${area.id}'),
          value: _AreaAction.pack,
          child: Text(t.exportAsPack),
        ),
        if (shared)
          PopupMenuItem(value: _AreaAction.people, child: Text(t.peopleTitle)),
        // Read as the menu opens: a sync may have shared it meanwhile.
        if (hiddenFromAiEntry(widget.places.store, area.id, () {}) case (
          :final on,
          toggle: _,
        ))
          hiddenFromAiMenuItem(
            context,
            key: ValueKey('area-hidden-from-ai-${area.id}'),
            value: _AreaAction.hideFromAi,
            on: on,
          ),
        if (manages)
          PopupMenuItem(value: _AreaAction.delete, child: Text(t.delete)),
      ],
    );
  }

  /// ⋮ of the Areas list: Rearrange, with two Areas or more, shows the
  /// Areas' handles (obf-1089), Import… opens the page of the Imports
  /// (obf-1096), Running the page of what runs in the background
  /// (obf-1078).
  Widget _areasMenu(AppLocalizations t) => PopupMenuButton<VoidCallback>(
    key: const ValueKey('areas-more'),
    tooltip: t.more,
    icon: const Icon(Icons.more_vert),
    onSelected: (run) => run(),
    itemBuilder: (_) => [
      if (_areas.length > 1)
        PopupMenuItem(
          key: const ValueKey('areas-rearrange'),
          value: () => setState(() => _rearranging = true),
          child: Text(t.rearrangeAreas),
        ),
      if (ImportsScreen.offered(context))
        PopupMenuItem(
          key: const ValueKey('areas-import'),
          value: _openImports,
          child: Text(t.areasImport),
        ),
      if (RunningScope.of(context) != null)
        PopupMenuItem(
          key: const ValueKey('areas-running'),
          value: () => openRunning(context),
          child: Text(t.jobsTitle),
        ),
    ],
  );

  /// Whether ⋮ holds anything: it stands also with fewer than two Areas
  /// (obf-1096).
  bool get _hasMenu =>
      _areas.length > 1 ||
      ImportsScreen.offered(context) ||
      RunningScope.of(context) != null;

  /// The page of the Imports on top (obf-1096).
  void _openImports() => unawaited(
    Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) =>
            ImportsScreen(store: widget.places.store, trail: widget.trail),
      ),
    ),
  );

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final searching = _searched.trim().isNotEmpty;
    // The tags offered leave out the Notes of a closed locked Area, as
    // every list does (#267).
    final lock = LockScope.of(context);
    final closed = {
      for (final area in _areas)
        if (lock?.isAreaClosed(area.id) ?? false) area.id,
    };
    final rows = searching ? const <Widget>[] : _areaRows(t);
    final padding = clearOfFloatingButtons(
      context,
      const EdgeInsets.symmetric(vertical: 8),
    );
    return BrowserSwipes(
      // Rearranging stays on the page: a swipe on a row moves it.
      forwardRoute: _rearranging ? null : _forwardRoute,
      onForward: _rearranging ? null : _forward,
      back: widget.onBack == null,
      child: TagSearchScope(
        onSearch: _searchTag,
        child: _selectionBack(
          Scaffold(
            appBar: pageBar(
              context,
              title: selecting
                  ? t.notesSelected(selected.length)
                  : t.areasTitle,
              background: searchHeaderColour(context),
              onBack: widget.onBack,
              actions: selecting
                  ? noteSelectionActions(
                      context,
                      widget.notes,
                      widget.places,
                      _hits,
                      _reload,
                    )
                  : [
                      // Only the search's hits are Notes on this level.
                      if (searching) ...[
                        PersonFilterButton(
                          store: widget.places.store,
                          filter: _filter,
                          onChanged: _filtered,
                        ),
                        CompactNotesButton(
                          compact: compact,
                          onChanged: setCompact,
                        ),
                      ],
                      IconButton(
                        tooltip: t.allNotesTitle,
                        onPressed: _openAllNotes,
                        icon: const Icon(Icons.notes),
                      ),
                      IconButton(
                        tooltip: t.frogsTitle,
                        onPressed: _openFrogs,
                        icon: const FrogIcon(),
                      ),
                      IconButton(
                        tooltip: t.trashTitle,
                        onPressed: _openTrash,
                        // A bin with an arrow up opens the Trash; the plain bin
                        // deletes (#166).
                        icon: const Icon(Icons.restore_from_trash),
                      ),
                      if (searching && mayTick(_hits)) selectButton(context),
                      if (_rearranging)
                        TextButton(
                          key: const ValueKey('areas-rearrange-done'),
                          onPressed: _stopRearranging,
                          child: Text(t.rearrangeDone),
                        )
                      else if (!searching && _hasMenu)
                        _areasMenu(t),
                    ],
            ),
            body: CustomScrollView(
              slivers: [
                SearchHeaderSliver(
                  controller: _query,
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      BrowserSearchField(
                        controller: _query,
                        hint: t.searchAllHint,
                        onChanged: _typed,
                        onSet: _set,
                        tags: widget.places.store,
                        closedTags: closed,
                      ),
                      // The most used tags, while nothing is typed
                      // (obf-1074); a search shows its own chips.
                      ListenableBuilder(
                        listenable: _tagRow,
                        builder: (_, _) => _query.text.trim().isEmpty
                            ? TagChipRow(
                                tags: _tags.value,
                                onPick: (tag) => _searchTag(context, tag),
                                onAll: _openTags,
                              )
                            : const SizedBox.shrink(),
                      ),
                      if (searching)
                        PersonFilterChips(
                          store: widget.places.store,
                          filter: _filter,
                          onChanged: _filtered,
                        ),
                    ],
                  ),
                ),
                const SearchHeaderLine(),
                if (searching || _areas.isEmpty)
                  SliverPadding(
                    padding: padding,
                    sliver: SliverList.list(
                      children: searching
                          ? _hitCards(
                              context,
                              _hits,
                              _file,
                              _merge,
                              _openNote,
                              _schedule,
                              widget.notes,
                              _reload,
                              // Hits from every Area.
                              (_) => true,
                              compact: compact,
                              switched: switched,
                              onSwitch: switchView,
                              selected: selecting ? selected : null,
                              onSelect: setSelected,
                              find: _searched,
                              onFind: _openAt,
                            )
                          : rows,
                    ),
                  )
                else ...[
                  // The Areas move only while rearranging (obf-1089): a
                  // long press never changes data (#280).
                  SliverPadding(
                    padding: padding,
                    sliver: SliverReorderableList(
                      itemCount: rows.length,
                      itemBuilder: (_, index) => rows[index],
                      onReorderItem: _moved,
                      proxyDecorator: _lifted,
                    ),
                  ),
                ],
              ],
            ),
            floatingActionButton: TapMarked(
              child: FloatingActionButton(
                tooltip: t.newArea,
                onPressed: _add,
                child: const Icon(Icons.add),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

/// An Area row while it is dragged: lifted, as a reorderable list lifts
/// it (#280).
Widget _lifted(Widget row, int _, Animation<double> lift) => AnimatedBuilder(
  animation: lift,
  builder: (_, row) => Material(
    elevation: lerpDouble(0, 6, Curves.easeInOut.transform(lift.value)) ?? 0,
    child: row,
  ),
  child: row,
);

enum _AreaAction { rename, look, share, pack, people, hideFromAi, delete }

enum _TopicAction { rename, pack, delete }

/// The browser's second level, one Area: its Topics and the diary of
/// its Notes without a Topic, with a search field over its Notes and
/// those of its Topics, and "+" to make a Note, a Checklist, a Shopping
/// List, a Tally, a Session, a Frog or a Topic in it.
class AreaScreen extends StatefulWidget {
  final Notes notes;
  final Places places;
  final BrowserTrail trail;
  final String areaId;

  const AreaScreen({
    super.key,
    required this.notes,
    required this.places,
    required this.trail,
    required this.areaId,
  });

  @override
  State<AreaScreen> createState() => _AreaScreenState();
}

class _AreaScreenState extends State<AreaScreen>
    with ReloadsOnLogChange, FoldsNoteCards, SelectsNotes, SearchesAfterPause {
  final _query = TextEditingController();

  /// The text [_diary] was searched for; the field's own may be newer.
  String _searched = '';
  Area? _area;
  List<Topic> _topics = const [];
  List<PlacedNote> _diary = const [];
  DiaryOrder _order = DiaryOrder.time;
  bool _hideUnimportant = false;
  PersonFilter _filter = PersonFilter.everyone;
  bool _loaded = false;

  /// Whether this Device may change the Area and its Topics, and write
  /// in it at all (ADR 0013).
  bool _manages = true;
  bool _writes = true;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (_loaded) return;
    _load();
    _loaded = true;
  }

  @override
  void dispose() {
    _query.dispose();
    super.dispose();
  }

  /// Reads the Area, its Topics sorted by the name the reader sees, and
  /// its Notes without a Topic, or while searching every match in it and
  /// its Topics.
  void _load() {
    final t = context.t;
    _area = widget.places.areaById(widget.areaId);
    _manages = widget.places.store.mayManage(widget.areaId);
    _writes = widget.places.store.mayWriteIn(widget.areaId);
    _topics = widget.places.topics(widget.areaId)
      ..sort(
        (a, b) => topicName(
          t,
          a,
        ).toLowerCase().compareTo(topicName(t, b).toLowerCase()),
      );
    _searched = _query.text;
    final found = _search(
      widget.places,
      _searched,
      within: widget.areaId,
      lock: LockScope.of(context),
      order: _order,
      hideUnimportant: _hideUnimportant,
      people: _filter,
    );
    // A Topic's Notes show inside the Topic; a search still finds them,
    // their cards naming the Topic.
    _diary = _searched.trim().isNotEmpty
        ? found
        : [
            for (final placed in found)
              if (placed.note.place == widget.areaId) placed,
          ];
    keepShown(_diary);
  }

  void _reload() => setState(_load);

  /// A letter typed: the search runs once typing pauses, an emptied
  /// field shows the list at once.
  void _typed(String query) =>
      query.trim().isEmpty ? _set(query) : searchAfterPause(_reload);

  /// The search set as a whole runs now.
  void _set(String _) => searchNow(_reload);

  void _filtered(PersonFilter people) => setState(() {
    _filter = people;
    _load();
  });

  /// "Show" of a Note made but hidden by the filter clears it (obf-1070).
  void _showEveryone() {
    if (mounted) _filtered(PersonFilter.everyone);
  }

  void _reloadIfMounted() {
    if (mounted) _reload();
  }

  @override
  Store get log => widget.places.store;

  /// The Area's Notes and the hits of a search in it.
  @override
  String get cardsKey => 'area:${widget.areaId}';

  @override
  void reloadLog() => _reload();

  /// "+" fans out what it makes, filed straight into this Area (#161),
  /// and for a Manager a new Topic.
  Future<void> _add(BuildContext button) async {
    final t = context.t;
    final kind = await pickNewKind(
      context,
      button,
      newKinds(widget.places.store, widget.areaId, topic: true),
    );
    if (!mounted || kind == null) return;
    if (kind != NewKind.topic) {
      final made = await makeNew(
        context,
        widget.notes,
        widget.places,
        kind,
        place: widget.areaId,
        onChanged: _reloadIfMounted,
      );
      if (!mounted) return;
      sayIfFilterHides(
        context,
        widget.places.store,
        made,
        _filter,
        show: _showEveryone,
      );
      return;
    }
    final name = await askName(context, title: t.newTopic);
    if (!mounted || name == null) return;
    widget.places.addTopic(widget.areaId, name);
    _reload();
  }

  Future<void> _open(Topic topic) =>
      Navigator.of(context).push(_topicRoute(topic));

  /// [topic]'s page, chosen as the last selected Topic.
  BrowserRoute<void> _topicRoute(Topic topic) {
    widget.trail.choseTopic(topic);
    setState(() {});
    final route = BrowserRoute<void>(
      place: ShownPlace.topic(topic.id),
      builder: (_) => TopicScreen(
        notes: widget.notes,
        places: widget.places,
        topicId: topic.id,
      ),
    );
    // Fire and forget: popped never fails, and a failure in what runs
    // after it reaches the crash guard like any other callback's.
    unawaited(
      route.popped.then((_) {
        if (mounted) _reload();
      }),
    );
    return route;
  }

  /// The last selected Topic's page for swipe left to pull in.
  BrowserRoute<void>? _forwardRoute() {
    final topic = widget.trail.topicIn(widget.places, widget.areaId);
    return topic == null ? null : _topicRoute(topic);
  }

  Future<void> _file(Note note) async {
    if (await fileNote(context, widget.notes, widget.places, note)) {
      if (!mounted) return;
      _reload();
    }
  }

  Future<void> _merge(Note note) async {
    if (await mergeNote(context, widget.notes, widget.places, note)) {
      if (!mounted) return;
      _reload();
    }
  }

  Future<void> _openNote(Note note) async {
    await openNote(context, widget.notes, widget.places, note);
    if (!mounted) return;
    _reload();
  }

  /// Opens [note] at [at], a place the search found in it, with the find
  /// bar for the search (obf-1047).
  Future<void> _openAt(Note note, Finding at) async {
    await openNote(
      context,
      widget.notes,
      widget.places,
      note,
      find: (query: _searched, at: at),
    );
    if (!mounted) return;
    _reload();
  }

  Future<void> _schedule(Note note, Spawn spawn) async {
    await spawnFromNote(
      context,
      widget.notes,
      widget.places,
      note,
      spawn: spawn,
    );
    if (!mounted) return;
    _reload();
  }

  Future<void> _act(Area area, Topic topic, _TopicAction action) async {
    final t = context.t;
    switch (action) {
      case _TopicAction.rename:
        final shown = topicName(t, topic);
        final name = await askName(context, title: t.rename, initial: shown);
        if (!mounted || name == null || name == shown) return;
        widget.places.rename(topic.id, name);
      case _TopicAction.pack:
        await exportPack(context, widget.places.store, topic.id);
        if (!mounted) return;
      case _TopicAction.delete:
        final go = await confirmDeleteTopic(
          context,
          widget.places,
          area,
          topic,
        );
        if (!mounted || go == null) return;
        widget.places.deleteTopic(topic.id, go);
    }
    _reload();
  }

  /// The Topics, then the Area's cards.
  List<Widget> _browsing(AppLocalizations t, Area area) {
    if (_topics.isEmpty && _diary.isEmpty) {
      return [BrowserEmpty(text: t.noTopicsYet)];
    }
    final last = widget.trail.topicIn(widget.places, area.id)?.id;
    return [
      for (final topic in _topics)
        _PlaceRow(
          title: topicName(t, topic),
          highlighted: topic.id == last,
          onTap: () => _open(topic),
          // Anyone may export copies (#164); renaming and deleting are a
          // Manager's.
          menu: PopupMenuButton<_TopicAction>(
            key: ValueKey('topic-menu-${topic.id}'),
            onSelected: (action) => _act(area, topic, action),
            itemBuilder: (_) => [
              if (_manages)
                PopupMenuItem(
                  value: _TopicAction.rename,
                  child: Text(t.rename),
                ),
              PopupMenuItem(
                key: ValueKey('topic-pack-${topic.id}'),
                value: _TopicAction.pack,
                child: Text(t.exportAsPack),
              ),
              if (_manages)
                PopupMenuItem(
                  value: _TopicAction.delete,
                  child: Text(t.delete),
                ),
            ],
          ),
        ),
      ...diaryCards(
        context,
        _diary,
        _file,
        _merge,
        _openNote,
        notes: widget.notes,
        onChanged: _reload,
        // The Area's own Notes, under its title.
        showPlace: (_) => false,
        onSchedule: _schedule,
        compact: compact,
        switched: switched,
        onSwitch: switchView,
        selected: selecting ? selected : null,
        onSelect: setSelected,
      ),
    ];
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final area = _area;
    if (area == null) return Scaffold(appBar: AppBar());
    final searching = _searched.trim().isNotEmpty;
    return BrowserSwipes(
      forwardRoute: _forwardRoute,
      child: selectionBack(
        Scaffold(
          appBar: pageBar(
            context,
            title: selecting
                ? t.notesSelected(selected.length)
                : areaName(t, area),
            background: searchHeaderColour(context),
            actions: selecting
                ? noteSelectionActions(
                    context,
                    widget.notes,
                    widget.places,
                    _diary,
                    _reload,
                  )
                : [
                    PersonFilterButton(
                      store: widget.places.store,
                      filter: _filter,
                      onChanged: _filtered,
                    ),
                    ...diaryActions(
                      context,
                      order: _order,
                      hideUnimportant: _hideUnimportant,
                      onOrder: (order) => setState(() {
                        _order = order;
                        _load();
                      }),
                      onHideUnimportant: (hide) => setState(() {
                        _hideUnimportant = hide;
                        _load();
                      }),
                    ),
                    Spotlight(
                      id: 'area-compact',
                      child: CompactNotesButton(
                        compact: compact,
                        onChanged: setCompact,
                      ),
                    ),
                    if (mayTick(_diary)) selectButton(context),
                  ],
          ),
          body: CustomScrollView(
            slivers: [
              SearchHeaderSliver(
                controller: _query,
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Spotlight(
                      id: 'area-search',
                      child: BrowserSearchField(
                        controller: _query,
                        hint: t.searchAreaHint,
                        onChanged: _typed,
                        onSet: _set,
                      ),
                    ),
                    PersonFilterChips(
                      store: widget.places.store,
                      filter: _filter,
                      onChanged: _filtered,
                    ),
                  ],
                ),
              ),
              const SearchHeaderLine(),
              SliverPadding(
                // Where this Device may not write, no "+" floats to clear.
                padding: _writes
                    ? clearOfFloatingButtons(
                        context,
                        const EdgeInsets.symmetric(vertical: 8),
                      )
                    : clearOfSystemBars(
                        context,
                        const EdgeInsets.symmetric(vertical: 8),
                      ),
                sliver: SliverList.list(
                  children: searching
                      ? _hitCards(
                          context,
                          _diary,
                          _file,
                          _merge,
                          _openNote,
                          _schedule,
                          widget.notes,
                          _reload,
                          // A hit in one of the Topics names it.
                          (placed) => placed.note.place != widget.areaId,
                          compact: compact,
                          switched: switched,
                          onSwitch: switchView,
                          selected: selecting ? selected : null,
                          onSelect: setSelected,
                          find: _searched,
                          onFind: _openAt,
                        )
                      : _browsing(t, area),
                ),
              ),
            ],
          ),
          floatingActionButton: !_writes
              ? null
              : Spotlight(
                  id: 'area-add',
                  child: Builder(
                    builder: (button) => TapMarked(
                      child: FloatingActionButton(
                        tooltip: t.addHere,
                        onPressed: () => _add(button),
                        child: const Icon(Icons.add),
                      ),
                    ),
                  ),
                ),
        ),
      ),
    );
  }
}

/// The browser's last level, one Topic: its diary of cards with a
/// search field over them, and "+" to make a Note, a Checklist, a
/// Shopping List, a Tally, a Session or a Frog in it.
class TopicScreen extends StatefulWidget {
  final Notes notes;
  final Places places;
  final String topicId;

  const TopicScreen({
    super.key,
    required this.notes,
    required this.places,
    required this.topicId,
  });

  @override
  State<TopicScreen> createState() => _TopicScreenState();
}

class _TopicScreenState extends State<TopicScreen>
    with ReloadsOnLogChange, FoldsNoteCards, SelectsNotes, SearchesAfterPause {
  final _query = TextEditingController();

  /// The text [_diary] was searched for; the field's own may be newer.
  String _searched = '';
  late Topic? _topic;
  late List<PlacedNote> _diary;
  DiaryOrder _order = DiaryOrder.time;
  bool _hideUnimportant = false;
  PersonFilter _filter = PersonFilter.everyone;

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void dispose() {
    _query.dispose();
    super.dispose();
  }

  /// Whether this Device may write in the Topic's Area (ADR 0013).
  bool _writes = true;

  void _load() {
    _topic = widget.places.topicById(widget.topicId);
    _writes = widget.places.store.mayWriteIn(widget.topicId);
    _searched = _query.text;
    _diary = _search(
      widget.places,
      _searched,
      within: widget.topicId,
      lock: LockScope.of(context),
      order: _order,
      hideUnimportant: _hideUnimportant,
      people: _filter,
    );
    keepShown(_diary);
  }

  void _reload() => setState(_load);

  /// A letter typed: the search runs once typing pauses, an emptied
  /// field shows the list at once.
  void _typed(String query) =>
      query.trim().isEmpty ? _set(query) : searchAfterPause(_reload);

  /// The search set as a whole runs now.
  void _set(String _) => searchNow(_reload);

  void _filtered(PersonFilter people) => setState(() {
    _filter = people;
    _load();
  });

  /// "+" fans out what it makes, filed straight into this Topic (#161).
  Future<void> _add(BuildContext button) async {
    final kind = await pickNewKind(
      context,
      button,
      newKinds(widget.places.store, widget.topicId),
    );
    if (!mounted || kind == null) return;
    final made = await makeNew(
      context,
      widget.notes,
      widget.places,
      kind,
      place: widget.topicId,
      onChanged: _reloadIfMounted,
    );
    if (!mounted) return;
    sayIfFilterHides(
      context,
      widget.places.store,
      made,
      _filter,
      show: _showEveryone,
    );
  }

  /// "Show" of a Note made but hidden by the filter clears it (obf-1070).
  void _showEveryone() {
    if (mounted) _filtered(PersonFilter.everyone);
  }

  void _reloadIfMounted() {
    if (mounted) _reload();
  }

  @override
  Store get log => widget.places.store;

  /// The Topic's Notes and the hits of a search in it.
  @override
  String get cardsKey => 'topic:${widget.topicId}';

  @override
  void reloadLog() => _reload();

  Future<void> _file(Note note) async {
    if (await fileNote(context, widget.notes, widget.places, note)) {
      if (!mounted) return;
      _reload();
    }
  }

  Future<void> _merge(Note note) async {
    if (await mergeNote(context, widget.notes, widget.places, note)) {
      if (!mounted) return;
      _reload();
    }
  }

  Future<void> _openNote(Note note) async {
    await openNote(context, widget.notes, widget.places, note);
    if (!mounted) return;
    _reload();
  }

  /// Opens [note] at [at], a place the search found in it, with the find
  /// bar for the search (obf-1047).
  Future<void> _openAt(Note note, Finding at) async {
    await openNote(
      context,
      widget.notes,
      widget.places,
      note,
      find: (query: _searched, at: at),
    );
    if (!mounted) return;
    _reload();
  }

  Future<void> _schedule(Note note, Spawn spawn) async {
    await spawnFromNote(
      context,
      widget.notes,
      widget.places,
      note,
      spawn: spawn,
    );
    if (!mounted) return;
    _reload();
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final topic = _topic;
    if (topic == null) return Scaffold(appBar: AppBar());
    final searching = _searched.trim().isNotEmpty;
    return BrowserSwipes(
      child: selectionBack(
        Scaffold(
          appBar: pageBar(
            context,
            title: selecting
                ? t.notesSelected(selected.length)
                : topicName(t, topic),
            background: searchHeaderColour(context),
            actions: selecting
                ? noteSelectionActions(
                    context,
                    widget.notes,
                    widget.places,
                    _diary,
                    _reload,
                  )
                : [
                    PersonFilterButton(
                      store: widget.places.store,
                      filter: _filter,
                      onChanged: _filtered,
                    ),
                    ...diaryActions(
                      context,
                      order: _order,
                      hideUnimportant: _hideUnimportant,
                      onOrder: (order) => setState(() {
                        _order = order;
                        _load();
                      }),
                      onHideUnimportant: (hide) => setState(() {
                        _hideUnimportant = hide;
                        _load();
                      }),
                    ),
                    Spotlight(
                      id: 'area-compact',
                      child: CompactNotesButton(
                        compact: compact,
                        onChanged: setCompact,
                      ),
                    ),
                    if (mayTick(_diary)) selectButton(context),
                  ],
          ),
          body: CustomScrollView(
            slivers: [
              SearchHeaderSliver(
                controller: _query,
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Spotlight(
                      id: 'area-search',
                      child: BrowserSearchField(
                        controller: _query,
                        hint: t.searchTopicHint,
                        onChanged: _typed,
                        onSet: _set,
                      ),
                    ),
                    PersonFilterChips(
                      store: widget.places.store,
                      filter: _filter,
                      onChanged: _filtered,
                    ),
                  ],
                ),
              ),
              const SearchHeaderLine(),
              SliverPadding(
                // Where this Device may not write, no "+" floats to clear.
                padding: _writes
                    ? clearOfFloatingButtons(
                        context,
                        const EdgeInsets.symmetric(vertical: 8),
                      )
                    : clearOfSystemBars(
                        context,
                        const EdgeInsets.symmetric(vertical: 8),
                      ),
                sliver: SliverList.list(
                  children: searching
                      ? _hitCards(
                          context,
                          _diary,
                          _file,
                          _merge,
                          _openNote,
                          _schedule,
                          widget.notes,
                          _reload,
                          // Every hit is in the Topic of the title.
                          (_) => false,
                          compact: compact,
                          switched: switched,
                          onSwitch: switchView,
                          selected: selecting ? selected : null,
                          onSelect: setSelected,
                          find: _searched,
                          onFind: _openAt,
                        )
                      : _diary.isEmpty
                      ? [
                          BrowserEmpty(
                            text: _filter.isEveryone
                                ? t.noNotesHere
                                : t.nothingFound,
                          ),
                        ]
                      : diaryCards(
                          context,
                          _diary,
                          _file,
                          _merge,
                          _openNote,
                          notes: widget.notes,
                          onChanged: _reload,
                          showPlace: (_) => false,
                          onSchedule: _schedule,
                          compact: compact,
                          switched: switched,
                          onSwitch: switchView,
                          selected: selecting ? selected : null,
                          onSelect: setSelected,
                        ),
                ),
              ),
            ],
          ),
          floatingActionButton: !_writes
              ? null
              : Spotlight(
                  id: 'area-add',
                  child: Builder(
                    builder: (button) => TapMarked(
                      child: FloatingActionButton(
                        tooltip: t.addHere,
                        onPressed: () => _add(button),
                        child: const Icon(Icons.add),
                      ),
                    ),
                  ),
                ),
        ),
      ),
    );
  }
}

/// The Notes matching [query] by the people [people] as a diary in
/// [order], with where they are filed; those of closed locked Areas stay
/// out.
List<PlacedNote> _search(
  Places places,
  String query, {
  String? within,
  AppLock? lock,
  DiaryOrder order = DiaryOrder.time,
  bool hideUnimportant = false,
  PersonFilter people = PersonFilter.everyone,
}) => placeNotesShown(
  places,
  diary(
    places.store.notesBy(
      Search(places.store).notes(query, within: within),
      people,
    ),
    order: order,
    hideUnimportant: hideUnimportant,
  ),
  lock,
);

/// Every hit of a search on one page, or a line saying there is none.
List<Widget> _hitCards(
  BuildContext context,
  List<PlacedNote> hits,
  void Function(Note note) onFile,
  void Function(Note note) onMerge,
  void Function(Note note) onOpen,
  void Function(Note note, Spawn spawn) onSchedule,
  Notes notes,
  VoidCallback onChanged,
  bool Function(PlacedNote placed) showPlace, {
  required bool compact,
  required Set<String> switched,
  required void Function(Note note) onSwitch,
  Set<String>? selected,
  void Function(Note note, bool on)? onSelect,
  required String find,
  required void Function(Note note, Finding at) onFind,
}) => hits.isEmpty
    ? [BrowserEmpty(text: context.t.nothingFound)]
    : diaryCards(
        context,
        hits,
        onFile,
        onMerge,
        onOpen,
        notes: notes,
        onChanged: onChanged,
        showPlace: showPlace,
        onSchedule: onSchedule,
        compact: compact,
        switched: switched,
        onSwitch: onSwitch,
        selected: selected,
        onSelect: onSelect,
        find: find,
        onFind: onFind,
      );

/// Opens "file into…" for [note] and files it; true when it moved.
Future<bool> fileNote(
  BuildContext context,
  Notes notes,
  Places places,
  Note note,
) async {
  final placeId = await Navigator.of(context).push(
    MaterialPageRoute<String>(
      builder: (_) => FileIntoScreen(places: places, current: note.place),
    ),
  );
  if (placeId == null || !context.mounted) return false;
  // Nothing Hidden from AI goes into a Shared Area (#267).
  final filed = await unlessHiddenFromAi(context, notes.store, () {
    notes.file(note.id, placeId);
    return true;
  });
  return filed ?? false;
}

/// An Area's icon in its colour, or a plain dot of its colour, or a
/// neutral folder when it has neither.
class _AreaMark extends StatelessWidget {
  final Area area;

  const _AreaMark({required this.area});

  @override
  Widget build(BuildContext context) {
    final colour =
        areaColour(area.colour) ?? Theme.of(context).colorScheme.outline;
    return Icon(areaIconData(area.icon) ?? Icons.folder, color: colour);
  }
}

/// One Area or Topic as an embossed row with its menu.
class _PlaceRow extends StatelessWidget {
  final String title;

  /// Under the title: who a Shared Area is shared with.
  final String? subtitle;
  final Widget? leading;
  final VoidCallback? onTap;

  /// What may be done to it; none for a Member.
  final Widget? menu;

  /// The last selected one, where swipe left leads: ringed in the accent.
  final bool highlighted;

  /// A locked Area that is closed: a padlock says so.
  final bool locked;

  /// A Shared Area: two people say so.
  final bool shared;

  /// An Area Hidden from AI (#267): a shield says so.
  final bool hiddenFromAi;

  const _PlaceRow({
    super.key,
    required this.title,
    required this.menu,
    this.subtitle,
    this.leading,
    this.onTap,
    this.highlighted = false,
    this.locked = false,
    this.shared = false,
    this.hiddenFromAi = false,
  });

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 6),
    child: Embossed(
      child: DecoratedBox(
        decoration: BoxDecoration(
          borderRadius: BorderRadius.circular(16),
          border: highlighted
              ? Border.all(
                  color: Theme.of(context).colorScheme.primary,
                  width: 3,
                )
              : null,
        ),
        // The row's ink shows on the card, not under it.
        child: Material(
          type: MaterialType.transparency,
          child: InkWell(
            borderRadius: BorderRadius.circular(16),
            onTap: onTap,
            child: Padding(
              padding: const EdgeInsets.fromLTRB(16, 4, 4, 4),
              child: Row(
                children: [
                  if (leading case final leading?) ...[
                    leading,
                    const SizedBox(width: 16),
                  ],
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        Text(
                          title,
                          style: Theme.of(context).textTheme.titleMedium,
                        ),
                        if (subtitle case final subtitle?)
                          Text(
                            subtitle,
                            style: Theme.of(context).textTheme.bodySmall,
                          ),
                      ],
                    ),
                  ),
                  if (shared)
                    Icon(
                      Icons.people_outline,
                      semanticLabel: context.t.areaShared,
                    ),
                  if (hiddenFromAi) const HiddenFromAiMark(),
                  if (locked)
                    Icon(
                      Icons.lock_outline,
                      semanticLabel: context.t.areaLocked,
                    ),
                  if (menu case final menu?)
                    menu
                  else
                    const SizedBox(width: 48, height: 48),
                ],
              ),
            ),
          ),
        ),
      ),
    ),
  );
}

/// Asks for a name; null when cancelled or left empty. Its button says
/// Rename when there is an [initial] name, unless [creates].
Future<String?> askName(
  BuildContext context, {
  required String title,
  String initial = '',
  bool creates = false,
}) async {
  final name = await showDialog<String>(
    context: context,
    builder: (_) => _NameDialog(
      title: title,
      initial: initial,
      creates: creates || initial.isEmpty,
    ),
  );
  final trimmed = name?.trim();
  return trimmed == null || trimmed.isEmpty ? null : trimmed;
}

class _NameDialog extends StatefulWidget {
  final String title;
  final String initial;

  /// Whether its button says Create rather than Rename.
  final bool creates;

  const _NameDialog({
    required this.title,
    required this.initial,
    required this.creates,
  });

  @override
  State<_NameDialog> createState() => _NameDialogState();
}

class _NameDialogState extends State<_NameDialog> {
  late final _name = TextEditingController(text: widget.initial);

  @override
  void dispose() {
    _name.dispose();
    super.dispose();
  }

  void _done() => Navigator.of(context).pop(_name.text);

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return AlertDialog(
      title: Text(widget.title),
      content: TextField(
        controller: _name,
        autofocus: true,
        textCapitalization: TextCapitalization.sentences,
        decoration: InputDecoration(hintText: t.nameHint),
        onSubmitted: (_) => _done(),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(t.cancel),
        ),
        TextButton(
          onPressed: _done,
          child: Text(widget.creates ? t.create : t.rename),
        ),
      ],
    );
  }
}

/// Asks once whether to delete [area]. With Notes inside, in it or in
/// its Topics, the question is where they go: Unsorted or the trash.
/// Null when the user cancelled.
Future<NotesGo?> confirmDeleteArea(
  BuildContext context,
  Places places,
  Area area,
) {
  final t = context.t;
  final count = places.notesInside(area.id);
  final hasTopics = places.topics(area.id).isNotEmpty;
  return showDialog<NotesGo>(
    context: context,
    builder: (context) => AlertDialog(
      title: Text(t.deleteAreaQuestion(areaName(t, area))),
      content: _lines([
        if (hasTopics) t.topicsGoToo,
        if (count > 0) ...[t.notesInside(count), t.whereNotesGo],
      ]),
      actions: [
        _cancel(context, t),
        if (count == 0)
          _destructive(
            context,
            t.delete,
            () => Navigator.of(context).pop(NotesGo.unsorted),
          )
        else ...[
          _destructive(
            context,
            t.moveToUnsorted,
            () => Navigator.of(context).pop(NotesGo.unsorted),
          ),
          _destructive(
            context,
            t.trashThem,
            () => Navigator.of(context).pop(NotesGo.trash),
          ),
        ],
      ],
    ),
  );
}

/// Asks once whether to delete [topic]. With Notes inside, the question
/// is where they go: Unsorted, [area] or the trash. Null when the user
/// cancelled.
Future<NotesGo?> confirmDeleteTopic(
  BuildContext context,
  Places places,
  Area area,
  Topic topic,
) {
  final t = context.t;
  final count = places.notesInside(topic.id);
  return showDialog<NotesGo>(
    context: context,
    builder: (context) => AlertDialog(
      title: Text(t.deleteTopicQuestion(topicName(t, topic))),
      content: count > 0
          ? _lines([t.notesInside(count), t.whereNotesGo])
          : null,
      actions: [
        _cancel(context, t),
        if (count == 0)
          _destructive(
            context,
            t.delete,
            () => Navigator.of(context).pop(NotesGo.unsorted),
          )
        else ...[
          _destructive(
            context,
            t.moveToUnsorted,
            () => Navigator.of(context).pop(NotesGo.unsorted),
          ),
          _destructive(
            context,
            t.moveToArea(areaName(t, area)),
            () => Navigator.of(context).pop(NotesGo.area),
          ),
          _destructive(
            context,
            t.trashThem,
            () => Navigator.of(context).pop(NotesGo.trash),
          ),
        ],
      ],
    ),
  );
}

Widget? _lines(List<String> lines) =>
    lines.isEmpty ? null : Text(lines.join('\n'));

Widget _cancel(BuildContext context, AppLocalizations t) => TextButton(
  onPressed: () => Navigator.of(context).pop(),
  child: Text(t.cancel),
);

/// A confirm button for a step that deletes: filled on the error colour
/// (UI laws, "Design system").
Widget _destructive(BuildContext context, String label, VoidCallback onTap) =>
    DestructiveButton(onPressed: onTap, child: Text(label));
