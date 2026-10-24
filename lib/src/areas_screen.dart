import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'area_look.dart';
import 'l10n.dart';
import 'look.dart';
import 'place_names.dart';

/// The Areas, with "+" to add one. Tapping an Area opens its Topics.
/// The first level of what grows into the notes browser.
class AreasScreen extends StatefulWidget {
  final Places places;

  const AreasScreen({super.key, required this.places});

  @override
  State<AreasScreen> createState() => _AreasScreenState();
}

class _AreasScreenState extends State<AreasScreen> {
  List<Area> _areas = const [];
  bool _loaded = false;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (_loaded) return;
    _load();
    _loaded = true;
  }

  /// Reads the Areas, sorted by the name the reader sees.
  void _load() {
    final t = context.t;
    _areas = widget.places.areas()
      ..sort(
        (a, b) => areaName(
          t,
          a,
        ).toLowerCase().compareTo(areaName(t, b).toLowerCase()),
      );
  }

  void _reload() => setState(_load);

  Future<void> _add() async {
    final name = await askName(context, title: context.t.newArea);
    if (!mounted || name == null) return;
    widget.places.addArea(name);
    _reload();
  }

  Future<void> _open(Area area) async {
    await Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) => AreaScreen(places: widget.places, areaId: area.id),
      ),
    );
    if (!mounted) return;
    _reload();
  }

  Future<void> _act(Area area, _AreaAction action) async {
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
        if (!mounted || !go) return;
        widget.places.deleteArea(area.id);
    }
    _reload();
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return Scaffold(
      appBar: AppBar(title: Text(t.areasTitle)),
      body: _areas.isEmpty
          ? _Empty(text: t.noAreasYet)
          : ListView(
              padding: const EdgeInsets.symmetric(vertical: 8),
              children: [
                for (final area in _areas)
                  _PlaceRow(
                    title: areaName(t, area),
                    leading: _AreaMark(area: area),
                    onTap: () => _open(area),
                    menu: PopupMenuButton<_AreaAction>(
                      onSelected: (action) => _act(area, action),
                      itemBuilder: (_) => [
                        PopupMenuItem(
                          value: _AreaAction.rename,
                          child: Text(t.rename),
                        ),
                        PopupMenuItem(
                          value: _AreaAction.look,
                          child: Text(t.areaLook),
                        ),
                        PopupMenuItem(
                          value: _AreaAction.delete,
                          child: Text(t.delete),
                        ),
                      ],
                    ),
                  ),
              ],
            ),
      floatingActionButton: FloatingActionButton(
        tooltip: t.newArea,
        onPressed: _add,
        child: const Icon(Icons.add),
      ),
    );
  }
}

enum _AreaAction { rename, look, delete }

enum _TopicAction { rename, delete }

/// One Area: its Topics, with "+" to add one.
class AreaScreen extends StatefulWidget {
  final Places places;
  final String areaId;

  const AreaScreen({super.key, required this.places, required this.areaId});

  @override
  State<AreaScreen> createState() => _AreaScreenState();
}

class _AreaScreenState extends State<AreaScreen> {
  Area? _area;
  List<Topic> _topics = const [];
  bool _loaded = false;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (_loaded) return;
    _load();
    _loaded = true;
  }

  /// Reads the Area and its Topics, sorted by the name the reader sees.
  void _load() {
    final t = context.t;
    _area = widget.places.areaById(widget.areaId);
    _topics = widget.places.topics(widget.areaId)
      ..sort(
        (a, b) => topicName(
          t,
          a,
        ).toLowerCase().compareTo(topicName(t, b).toLowerCase()),
      );
  }

  void _reload() => setState(_load);

  Future<void> _add() async {
    final name = await askName(context, title: context.t.newTopic);
    if (!mounted || name == null) return;
    widget.places.addTopic(widget.areaId, name);
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

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final area = _area;
    if (area == null) return Scaffold(appBar: AppBar());
    return Scaffold(
      appBar: AppBar(title: Text(areaName(t, area))),
      body: _topics.isEmpty
          ? _Empty(text: t.noTopicsYet)
          : ListView(
              padding: const EdgeInsets.symmetric(vertical: 8),
              children: [
                for (final topic in _topics)
                  _PlaceRow(
                    title: topicName(t, topic),
                    menu: PopupMenuButton<_TopicAction>(
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
              ],
            ),
      floatingActionButton: FloatingActionButton(
        tooltip: t.newTopic,
        onPressed: _add,
        child: const Icon(Icons.add),
      ),
    );
  }
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
  final Widget? leading;
  final VoidCallback? onTap;
  final Widget menu;

  const _PlaceRow({
    required this.title,
    required this.menu,
    this.leading,
    this.onTap,
  });

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 6),
    child: Embossed(
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
                  child: Text(
                    title,
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                ),
                menu,
              ],
            ),
          ),
        ),
      ),
    ),
  );
}

class _Empty extends StatelessWidget {
  final String text;

  const _Empty({required this.text});

  @override
  Widget build(BuildContext context) => Center(
    child: Padding(
      padding: const EdgeInsets.all(24),
      child: Embossed(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Text(text, textAlign: TextAlign.center),
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

/// Asks once whether to delete [area]. With Notes inside, the question
/// is where they go; for now that is only Unsorted (the trash comes with
/// the Note editor). True when the user confirmed.
Future<bool> confirmDeleteArea(
  BuildContext context,
  Places places,
  Area area,
) async {
  final t = context.t;
  final count = places.notesInside(area.id);
  final hasTopics = places.topics(area.id).isNotEmpty;
  final go = await showDialog<bool>(
    context: context,
    builder: (context) => AlertDialog(
      title: Text(t.deleteAreaQuestion(areaName(t, area))),
      content: _lines([
        if (count > 0) t.notesInside(count),
        if (hasTopics) t.topicsGoToo,
      ]),
      actions: [
        _cancel(context, t),
        _destructive(
          context,
          count > 0 ? t.moveToUnsorted : t.delete,
          () => Navigator.of(context).pop(true),
        ),
      ],
    ),
  );
  return go ?? false;
}

/// Asks once whether to delete [topic]. With Notes inside, the question
/// is where they go: Unsorted or [area] (the trash comes with the Note
/// editor). Null when the user cancelled.
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
