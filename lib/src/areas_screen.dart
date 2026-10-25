import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'app_lock.dart';
import 'area_look.dart';
import 'browser.dart';
import 'capture_screen.dart';
import 'file_into_screen.dart';
import 'l10n.dart';
import 'look.dart';
import 'merge_into_screen.dart';
import 'note_card.dart';
import 'note_editor_screen.dart';
import 'notes_screen.dart';
import 'place_names.dart';
import 'profile.dart';
import 'routine_editor_screen.dart';
import 'share_area_screen.dart';
import 'trash_screen.dart';

/// The notes browser's first level: a search field over every Note, and
/// the Areas while it is empty, with "+" to add an Area. Tapping an Area
/// opens its Topics; the app bar leads to all Notes and the trash.
class AreasScreen extends StatefulWidget {
  final Notes notes;
  final Places places;
  final BrowserTrail trail;

  const AreasScreen({
    super.key,
    required this.notes,
    required this.places,
    required this.trail,
  });

  @override
  State<AreasScreen> createState() => _AreasScreenState();
}

class _AreasScreenState extends State<AreasScreen> {
  final _query = TextEditingController();
  List<Area> _areas = const [];

  /// What this Device may do in each Area, and who shares the shared
  /// ones (ADR 0013).
  Map<String, AreaAccess> _access = const {};
  Map<String, List<Person>> _people = const {};

  /// The name each person's Profile shows, by Profile id.
  Map<String, String> _names = const {};
  List<PlacedNote> _hits = const [];
  PersonFilter _filter = PersonFilter.everyone;
  bool _loaded = false;

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

  /// Reads the Areas, sorted by the name the reader sees, and the hits
  /// of the search.
  void _load() {
    final t = context.t;
    _areas = widget.places.areas()
      ..sort(
        (a, b) => areaName(
          t,
          a,
        ).toLowerCase().compareTo(areaName(t, b).toLowerCase()),
      );
    final store = widget.places.store;
    _access = {for (final a in _areas) a.id: store.accessIn(a.id)};
    _people = {
      for (final a in _areas)
        if (store.isShared(a.id)) a.id: store.people(a.id),
    };
    _names = {
      for (final people in _people.values)
        for (final p in people) p.id: store.personShown(p.id).name,
    };
    _hits = _search(
      widget.places,
      _query.text,
      lock: LockScope.of(context),
      people: _filter,
    );
  }

  void _filtered(PersonFilter people) => setState(() {
    _filter = people;
    _load();
  });

  void _reload() => setState(_load);

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
    widget.trail.choseArea(area);
    setState(() {});
    await Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) => AreaScreen(
          notes: widget.notes,
          places: widget.places,
          trail: widget.trail,
          areaId: area.id,
        ),
      ),
    );
    if (!mounted) return;
    _reload();
  }

  void _forward() {
    if (widget.trail.area(widget.places) case final area?) _open(area);
  }

  Future<void> _openAllNotes() async {
    await Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) =>
            AllNotesScreen(notes: widget.notes, places: widget.places),
      ),
    );
    if (!mounted) return;
    _reload();
  }

  Future<void> _openTrash() async {
    await Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) => TrashScreen(notes: widget.notes, places: widget.places),
      ),
    );
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

  Future<void> _schedule(Note note, bool reminder) async {
    await spawnFromNote(
      context,
      widget.notes,
      widget.places,
      note,
      reminder: reminder,
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
        widget.places.deleteArea(area.id, go);
      case _AreaAction.share:
        await Navigator.of(context).push(
          MaterialPageRoute<bool>(
            builder: (_) =>
                ShareAreaScreen(store: widget.places.store, areaId: area.id),
          ),
        );
        if (!mounted) return;
      case _AreaAction.people:
        await Navigator.of(context).push(
          MaterialPageRoute<void>(
            builder: (_) =>
                PeopleScreen(store: widget.places.store, areaId: area.id),
          ),
        );
        if (!mounted) return;
    }
    _reload();
  }

  List<Widget> _areaRows(AppLocalizations t) {
    if (_areas.isEmpty) return [BrowserEmpty(text: t.noAreasYet)];
    final last = widget.trail.area(widget.places)?.id;
    final lock = LockScope.of(context);
    return [
      for (final area in _areas)
        _PlaceRow(
          title: areaName(t, area),
          subtitle: _people[area.id] == null
              ? null
              : [
                  for (final p in _people[area.id] ?? const <Person>[])
                    profileDisplayName(t, _names[p.id] ?? ''),
                ].join(', '),
          leading: _AreaMark(area: area),
          locked: lock?.isAreaClosed(area.id) ?? false,
          shared: _people.containsKey(area.id),
          highlighted: area.id == last,
          onTap: () => _open(area),
          menu: _areaMenu(t, area),
        ),
    ];
  }

  /// What may be done to [area] here: everything by its owner or a
  /// Manager, only seeing who is in by anyone else.
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
        if (shared)
          PopupMenuItem(value: _AreaAction.people, child: Text(t.peopleTitle)),
        if (manages)
          PopupMenuItem(value: _AreaAction.delete, child: Text(t.delete)),
      ],
    );
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final searching = _query.text.trim().isNotEmpty;
    return Scaffold(
      appBar: AppBar(
        title: Text(t.areasTitle),
        actions: [
          // Only the search's hits are Notes on this level.
          if (searching)
            PersonFilterButton(
              store: widget.places.store,
              filter: _filter,
              onChanged: _filtered,
            ),
          IconButton(
            tooltip: t.allNotesTitle,
            onPressed: _openAllNotes,
            icon: const Icon(Icons.notes),
          ),
          IconButton(
            tooltip: t.trashTitle,
            onPressed: _openTrash,
            icon: const Icon(Icons.delete_outline),
          ),
        ],
      ),
      body: BrowserSwipes(
        onForward: _forward,
        child: Column(
          children: [
            BrowserSearchField(
              controller: _query,
              hint: t.searchAllHint,
              onChanged: (_) => _reload(),
            ),
            if (searching)
              PersonFilterChips(
                store: widget.places.store,
                filter: _filter,
                onChanged: _filtered,
              ),
            Expanded(
              child: ListView(
                padding: const EdgeInsets.symmetric(vertical: 8),
                children: searching
                    ? _hitCards(
                        context,
                        _hits,
                        _file,
                        _merge,
                        _openNote,
                        _schedule,
                      )
                    : _areaRows(t),
              ),
            ),
          ],
        ),
      ),
      floatingActionButton: FloatingActionButton(
        tooltip: t.newArea,
        onPressed: _add,
        child: const Icon(Icons.add),
      ),
    );
  }
}

enum _AreaAction { rename, look, share, people, delete }

enum _TopicAction { rename, delete }

enum _AreaAdd { note, topic }

/// The browser's second level, one Area: its Topics and its diary of
/// cards, with a search field over the Area's Notes and "+" to add a
/// Topic.
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

class _AreaScreenState extends State<AreaScreen> {
  final _query = TextEditingController();
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
  /// its Notes that match the search.
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
    _diary = _search(
      widget.places,
      _query.text,
      within: widget.areaId,
      lock: LockScope.of(context),
      order: _order,
      hideUnimportant: _hideUnimportant,
      people: _filter,
    );
  }

  void _reload() => setState(_load);

  void _filtered(PersonFilter people) => setState(() {
    _filter = people;
    _load();
  });

  void _reloadIfMounted() {
    if (mounted) _reload();
  }

  /// "+" opens a menu at the button: a Note filed straight into this
  /// Area, or a new Topic.
  Future<void> _add(BuildContext button) async {
    final t = context.t;
    if (!_manages) {
      // A Member adds Notes only.
      await captureNote(
        context,
        widget.notes,
        place: widget.areaId,
        onChanged: _reloadIfMounted,
      );
      return;
    }
    final box = button.findRenderObject() as RenderBox?;
    final overlay =
        Overlay.of(context).context.findRenderObject() as RenderBox?;
    if (box == null || overlay == null) return;
    final choice = await showMenu<_AreaAdd>(
      context: context,
      position: RelativeRect.fromRect(
        box.localToGlobal(Offset.zero, ancestor: overlay) & box.size,
        Offset.zero & overlay.size,
      ),
      items: [
        PopupMenuItem(value: _AreaAdd.note, child: Text(t.noteHere)),
        PopupMenuItem(value: _AreaAdd.topic, child: Text(t.topicChoice)),
      ],
    );
    if (!mounted || choice == null) return;
    switch (choice) {
      case _AreaAdd.note:
        await captureNote(
          context,
          widget.notes,
          place: widget.areaId,
          onChanged: _reloadIfMounted,
        );
        return;
      case _AreaAdd.topic:
        final name = await askName(context, title: t.newTopic);
        if (!mounted || name == null) return;
        widget.places.addTopic(widget.areaId, name);
    }
    _reload();
  }

  Future<void> _open(Topic topic) async {
    widget.trail.choseTopic(topic);
    setState(() {});
    await Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) => TopicScreen(
          notes: widget.notes,
          places: widget.places,
          topicId: topic.id,
        ),
      ),
    );
    if (!mounted) return;
    _reload();
  }

  void _forward() {
    final topic = widget.trail.topicIn(widget.places, widget.areaId);
    if (topic != null) _open(topic);
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

  Future<void> _schedule(Note note, bool reminder) async {
    await spawnFromNote(
      context,
      widget.notes,
      widget.places,
      note,
      reminder: reminder,
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
          menu: !_manages
              ? null
              : PopupMenuButton<_TopicAction>(
                  onSelected: (action) => _act(area, topic, action),
                  itemBuilder: (_) => [
                    PopupMenuItem(
                      value: _TopicAction.rename,
                      child: Text(t.rename),
                    ),
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
        onSchedule: _schedule,
      ),
    ];
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final area = _area;
    if (area == null) return Scaffold(appBar: AppBar());
    final searching = _query.text.trim().isNotEmpty;
    return Scaffold(
      appBar: AppBar(
        title: Text(areaName(t, area)),
        actions: [
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
        ],
      ),
      body: BrowserSwipes(
        onForward: _forward,
        child: Column(
          children: [
            BrowserSearchField(
              controller: _query,
              hint: t.searchAreaHint,
              onChanged: (_) => _reload(),
            ),
            PersonFilterChips(
              store: widget.places.store,
              filter: _filter,
              onChanged: _filtered,
            ),
            Expanded(
              child: ListView(
                padding: const EdgeInsets.symmetric(vertical: 8),
                children: searching
                    ? _hitCards(
                        context,
                        _diary,
                        _file,
                        _merge,
                        _openNote,
                        _schedule,
                      )
                    : _browsing(t, area),
              ),
            ),
          ],
        ),
      ),
      floatingActionButton: !_writes
          ? null
          : Builder(
              builder: (button) => FloatingActionButton(
                tooltip: t.addHere,
                onPressed: () => _add(button),
                child: const Icon(Icons.add),
              ),
            ),
    );
  }
}

/// The browser's last level, one Topic: its diary of cards with a
/// search field over them, and "+" to capture a Note into it.
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

class _TopicScreenState extends State<TopicScreen> {
  final _query = TextEditingController();
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
    _diary = _search(
      widget.places,
      _query.text,
      within: widget.topicId,
      lock: LockScope.of(context),
      order: _order,
      hideUnimportant: _hideUnimportant,
      people: _filter,
    );
  }

  void _reload() => setState(_load);

  void _filtered(PersonFilter people) => setState(() {
    _filter = people;
    _load();
  });

  Future<void> _add() async {
    await captureNote(
      context,
      widget.notes,
      place: widget.topicId,
      onChanged: _reloadIfMounted,
    );
  }

  void _reloadIfMounted() {
    if (mounted) _reload();
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

  Future<void> _schedule(Note note, bool reminder) async {
    await spawnFromNote(
      context,
      widget.notes,
      widget.places,
      note,
      reminder: reminder,
    );
    if (!mounted) return;
    _reload();
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final topic = _topic;
    if (topic == null) return Scaffold(appBar: AppBar());
    final searching = _query.text.trim().isNotEmpty;
    return Scaffold(
      appBar: AppBar(
        title: Text(topicName(t, topic)),
        actions: [
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
        ],
      ),
      body: BrowserSwipes(
        child: Column(
          children: [
            BrowserSearchField(
              controller: _query,
              hint: t.searchTopicHint,
              onChanged: (_) => _reload(),
            ),
            PersonFilterChips(
              store: widget.places.store,
              filter: _filter,
              onChanged: _filtered,
            ),
            Expanded(
              child: ListView(
                padding: const EdgeInsets.symmetric(vertical: 8),
                children: searching
                    ? _hitCards(
                        context,
                        _diary,
                        _file,
                        _merge,
                        _openNote,
                        _schedule,
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
                        onSchedule: _schedule,
                      ),
              ),
            ),
          ],
        ),
      ),
      floatingActionButton: !_writes
          ? null
          : FloatingActionButton(
              tooltip: t.captureTooltip,
              onPressed: _add,
              child: const Icon(Icons.add),
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
}) => [
  for (final note in diary(
    places.store.notesBy(
      Search(places.store).notes(query, within: within),
      people,
    ),
    order: order,
    hideUnimportant: hideUnimportant,
  ))
    if (!(lock?.hides(note) ?? false)) placeNote(places, note),
];

/// Every hit of a search on one page, or a line saying there is none.
List<Widget> _hitCards(
  BuildContext context,
  List<PlacedNote> hits,
  void Function(Note note) onFile,
  void Function(Note note) onMerge,
  void Function(Note note) onOpen,
  void Function(Note note, bool reminder) onSchedule,
) => hits.isEmpty
    ? [BrowserEmpty(text: context.t.nothingFound)]
    : diaryCards(
        context,
        hits,
        onFile,
        onMerge,
        onOpen,
        onSchedule: onSchedule,
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
  if (placeId == null) return false;
  notes.file(note.id, placeId);
  return true;
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

  const _PlaceRow({
    required this.title,
    required this.menu,
    this.subtitle,
    this.leading,
    this.onTap,
    this.highlighted = false,
    this.locked = false,
    this.shared = false,
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

/// Asks for a name; null when cancelled or left empty.
Future<String?> askName(
  BuildContext context, {
  required String title,
  String initial = '',
}) async {
  final name = await showDialog<String>(
    context: context,
    builder: (_) => _NameDialog(title: title, initial: initial),
  );
  final trimmed = name?.trim();
  return trimmed == null || trimmed.isEmpty ? null : trimmed;
}

class _NameDialog extends StatefulWidget {
  final String title;
  final String initial;

  const _NameDialog({required this.title, required this.initial});

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
          child: Text(widget.initial.isEmpty ? t.create : t.rename),
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

/// A confirm button for a destructive step: its verb in the error colour
/// (UI laws).
Widget _destructive(BuildContext context, String label, VoidCallback onTap) =>
    TextButton(
      style: TextButton.styleFrom(
        foregroundColor: Theme.of(context).colorScheme.error,
      ),
      onPressed: onTap,
      child: Text(label),
    );
