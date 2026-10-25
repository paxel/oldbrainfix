import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'app_lock.dart';
import 'area_look.dart';
import 'l10n.dart';
import 'look.dart';
import 'place_names.dart';

/// Picks the Area or Topic to file a Note into; pops with its id.
class FileIntoScreen extends StatefulWidget {
  final Places places;

  /// Where the Note sits now, marked in the list.
  final String? current;

  const FileIntoScreen({super.key, required this.places, this.current});

  @override
  State<FileIntoScreen> createState() => _FileIntoScreenState();
}

class _FileIntoScreenState extends State<FileIntoScreen> {
  List<(Area, List<Topic>)> _tree = const [];
  bool _loaded = false;

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
    final store = widget.places.store;
    _tree = [
      for (final area
          in widget.places.areas()
            ..sort((a, b) => byName(areaName(t, a), areaName(t, b))))
        if (store.mayWriteIn(area.id))
          (
            area,
            (lock?.isAreaClosed(area.id) ?? false)
                ? <Topic>[]
                : (widget.places.topics(area.id)
                    ..sort((a, b) => byName(topicName(t, a), topicName(t, b)))),
          ),
    ];
    _loaded = true;
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final scheme = Theme.of(context).colorScheme;
    Widget? marked(String id) =>
        id == widget.current ? Icon(Icons.check, color: scheme.primary) : null;
    return Scaffold(
      appBar: AppBar(title: Text(t.fileInto)),
      body: _tree.isEmpty
          ? Center(
              child: Padding(
                padding: const EdgeInsets.all(24),
                child: Embossed(
                  child: Padding(
                    padding: const EdgeInsets.all(24),
                    child: Text(t.noAreasYet, textAlign: TextAlign.center),
                  ),
                ),
              ),
            )
          : ListView(
              padding: const EdgeInsets.symmetric(vertical: 8),
              children: [
                for (final (area, topics) in _tree)
                  Padding(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 16,
                      vertical: 8,
                    ),
                    child: Embossed(
                      // The rows' ink shows on the card, not under it.
                      child: Material(
                        type: MaterialType.transparency,
                        child: Column(
                          children: [
                            ListTile(
                              leading: Icon(
                                areaIconData(area.icon) ?? Icons.folder,
                                color:
                                    areaColour(area.colour) ?? scheme.outline,
                              ),
                              title: Text(
                                areaName(t, area),
                                style: Theme.of(context).textTheme.titleMedium,
                              ),
                              trailing: marked(area.id),
                              onTap: () => Navigator.of(context).pop(area.id),
                            ),
                            for (final topic in topics)
                              ListTile(
                                contentPadding: const EdgeInsets.only(
                                  left: 72,
                                  right: 16,
                                ),
                                title: Text(topicName(t, topic)),
                                trailing: marked(topic.id),
                                onTap: () =>
                                    Navigator.of(context).pop(topic.id),
                              ),
                          ],
                        ),
                      ),
                    ),
                  ),
              ],
            ),
    );
  }
}
