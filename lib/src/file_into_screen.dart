import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'app_lock.dart';
import 'area_look.dart';
import 'areas_screen.dart';
import 'browser.dart';
import 'l10n.dart';
import 'look.dart';
import 'page_bar.dart';
import 'place_names.dart';
import 'system_bars.dart';

/// Picks the Area or Topic to file a Note into; pops with its id (#272):
/// a filter over the names, the places this Device filed into last at
/// the top, and "New Area…" and "New Topic…", which make the place and
/// pick it at once. The pick is kept as filed into last.
class FileIntoScreen extends StatefulWidget {
  final Places places;

  /// Where the Note sits now, marked in the list.
  final String? current;

  const FileIntoScreen({super.key, required this.places, this.current});

  @override
  State<FileIntoScreen> createState() => _FileIntoScreenState();
}

/// An Area as File into… lists it, with the Topics it shows and whether
/// "New Topic…" goes under it.
typedef _Branch = ({Area area, List<Topic> topics, bool addsTopic});

class _FileIntoScreenState extends State<FileIntoScreen> {
  final _query = TextEditingController();
  List<_Branch> _tree = const [];
  List<_Branch> _shown = const [];

  /// The places filed into last that may still be filed into, each as
  /// its Area and, for a Topic, the Topic.
  List<(Area, Topic?)> _recent = const [];
  bool _loaded = false;

  @override
  void dispose() {
    _query.dispose();
    super.dispose();
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (_loaded) return;
    final t = context.t;
    int byName(String a, String b) =>
        a.toLowerCase().compareTo(b.toLowerCase());
    final lock = LockScope.of(context);
    // A closed locked Area takes Notes, but keeps its Topics to itself;
    // a Shared Area this Device was removed from takes none (ADR 0013).
    // A new Topic goes where "+" would make one (ui-laws, #197).
    final places = widget.places;
    final store = places.store;
    _tree = [
      // The Areas in the person's own order (#280), their Topics by name.
      for (final area in store.areasInOrder())
        if (store.mayWriteIn(area.id))
          if (lock?.isAreaClosed(area.id) ?? false)
            (area: area, topics: <Topic>[], addsTopic: false)
          else
            (
              area: area,
              topics: places.topics(area.id)
                ..sort((a, b) => byName(topicName(t, a), topicName(t, b))),
              addsTopic: store.advanced && store.mayManage(area.id),
            ),
    ];
    _recent = [for (final id in store.recentlyFiledInto) ?_listed(id)];
    _shown = _tree;
    _loaded = true;
  }

  /// The place [id] as the tree lists it; null when it does not.
  (Area, Topic?)? _listed(String id) {
    for (final (:area, :topics, addsTopic: _) in _tree) {
      if (area.id == id) return (area, null);
      for (final topic in topics) {
        if (topic.id == id) return (area, topic);
      }
    }
    return null;
  }

  /// The tree with what the filter matches: an Area whose name matches
  /// with all its Topics, else with the Topics whose names match.
  List<_Branch> _filtered() {
    final t = context.t;
    final query = _query.text.trim().toLowerCase();
    if (query.isEmpty) return _tree;
    bool matches(String name) => name.toLowerCase().contains(query);
    return [
      for (final branch in _tree)
        if (matches(areaName(t, branch.area)))
          branch
        else if (branch.topics
                .where((topic) => matches(topicName(t, topic)))
                .toList()
            case final topics when topics.isNotEmpty)
          (area: branch.area, topics: topics, addsTopic: branch.addsTopic),
    ];
  }

  void _filter(String _) => setState(() => _shown = _filtered());

  /// Picks [placeId] and keeps it as filed into last.
  void _pick(String placeId) {
    widget.places.store.filedInto(placeId);
    Navigator.of(context).pop(placeId);
  }

  /// Asks the name of a new Area, the filter's text filled in, makes it
  /// and picks it.
  Future<void> _newArea() async {
    final name = await askName(
      context,
      title: context.t.newArea,
      initial: _query.text.trim(),
      creates: true,
    );
    if (!mounted || name == null) return;
    _pick(widget.places.addArea(name).id);
  }

  /// Asks the name of a new Topic in [area], makes it and picks it. The
  /// filter's text is left out: it matched a name there already.
  Future<void> _newTopic(Area area) async {
    final name = await askName(context, title: context.t.newTopic);
    if (!mounted || name == null) return;
    _pick(widget.places.addTopic(area.id, name).id);
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final scheme = Theme.of(context).colorScheme;
    final filtering = _query.text.trim().isNotEmpty;
    Widget? marked(String id) =>
        id == widget.current ? Icon(Icons.check, color: scheme.primary) : null;
    Widget areaMark(Area area) => Icon(
      areaIconData(area.icon) ?? Icons.folder,
      color: areaColour(area.colour) ?? scheme.outline,
    );
    // The rows' ink shows on the card, not under it.
    Widget card(List<Widget> rows) => Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
      child: Embossed(
        child: Material(
          type: MaterialType.transparency,
          child: Column(children: rows),
        ),
      ),
    );
    return Scaffold(
      appBar: pageBar(context, title: t.fileInto),
      body: Column(
        children: [
          BrowserSearchField(
            controller: _query,
            hint: t.fileIntoFilterHint,
            onChanged: _filter,
          ),
          Expanded(
            child: ListView(
              padding: clearOfSystemBars(
                context,
                const EdgeInsets.symmetric(vertical: 8),
              ),
              children: [
                if (_recent.isNotEmpty && !filtering)
                  card([
                    ListTile(
                      dense: true,
                      title: Text(
                        t.fileIntoRecent,
                        style: Theme.of(context).textTheme.labelLarge,
                      ),
                    ),
                    for (final (area, topic) in _recent)
                      ListTile(
                        key: ValueKey(
                          'file-into-recent-${topic?.id ?? area.id}',
                        ),
                        leading: areaMark(area),
                        title: Text(
                          topic == null
                              ? areaName(t, area)
                              : topicName(t, topic),
                        ),
                        subtitle: topic == null
                            ? null
                            : Text(areaName(t, area)),
                        trailing: marked(topic?.id ?? area.id),
                        onTap: () => _pick(topic?.id ?? area.id),
                      ),
                  ]),
                if (filtering && _shown.isEmpty)
                  BrowserEmpty(text: t.fileIntoNoMatch),
                for (final (:area, :topics, :addsTopic) in _shown)
                  card([
                    ListTile(
                      leading: areaMark(area),
                      title: Text(
                        areaName(t, area),
                        style: Theme.of(context).textTheme.titleMedium,
                      ),
                      trailing: marked(area.id),
                      onTap: () => _pick(area.id),
                    ),
                    for (final topic in topics)
                      ListTile(
                        contentPadding: const EdgeInsets.only(
                          left: 72,
                          right: 16,
                        ),
                        title: Text(topicName(t, topic)),
                        trailing: marked(topic.id),
                        onTap: () => _pick(topic.id),
                      ),
                    if (addsTopic)
                      ListTile(
                        key: ValueKey('file-into-new-topic-${area.id}'),
                        contentPadding: const EdgeInsets.only(
                          left: 72,
                          right: 16,
                        ),
                        title: Text(
                          t.fileIntoNewTopic,
                          style: TextStyle(color: scheme.primary),
                        ),
                        onTap: () => _newTopic(area),
                      ),
                  ]),
                card([
                  ListTile(
                    key: const ValueKey('file-into-new-area'),
                    leading: Icon(Icons.add, color: scheme.primary),
                    title: Text(
                      t.fileIntoNewArea,
                      style: TextStyle(color: scheme.primary),
                    ),
                    onTap: _newArea,
                  ),
                ]),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
