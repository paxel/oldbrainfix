import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'l10n.dart';
import 'look.dart';
import 'place_names.dart';

/// First start: the Starter Set, every Area and Topic ticked. A Topic
/// can only stay ticked with its Area. Restoring a Backup is offered
/// above it, where [onRestore] is given.
class StarterSetScreen extends StatefulWidget {
  /// Gets the slugs left ticked.
  final ValueChanged<Set<String>> onDone;

  /// Opens the restore from a Backup; none where it is not offered.
  final VoidCallback? onRestore;

  const StarterSetScreen({super.key, required this.onDone, this.onRestore});

  @override
  State<StarterSetScreen> createState() => _StarterSetScreenState();
}

class _StarterSetScreenState extends State<StarterSetScreen> {
  final _picked = {
    for (final area in starterSet) ...{
      area.slug,
      for (final topic in area.topics) topic.slug,
    },
  };

  void _tickArea(
    ({String slug, String name, List<StarterPlace> topics}) area,
    bool on,
  ) {
    setState(() {
      if (on) {
        _picked.add(area.slug);
      } else {
        _picked.remove(area.slug);
        _picked.removeAll(area.topics.map((topic) => topic.slug));
      }
    });
  }

  void _tickTopic(String areaSlug, String topicSlug, bool on) {
    setState(() {
      if (on) {
        _picked.addAll({areaSlug, topicSlug});
      } else {
        _picked.remove(topicSlug);
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return Scaffold(
      appBar: AppBar(
        title: Text(t.starterSetTitle),
        actions: [
          TextButton(
            onPressed: () => widget.onDone({..._picked}),
            child: Text(t.starterSetDone),
          ),
        ],
      ),
      body: ListView(
        padding: const EdgeInsets.symmetric(vertical: 8),
        children: [
          if (widget.onRestore case final restore?)
            ListTile(
              key: const ValueKey('restore-offer'),
              leading: const Icon(Icons.settings_backup_restore),
              title: Text(t.restoreOffer),
              subtitle: Text(t.restoreOfferDetail),
              onTap: restore,
            ),
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 8, 16, 8),
            child: Text(t.starterSetIntro),
          ),
          for (final area in starterSet)
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
              child: Embossed(
                // The ticks' ink shows on the card, not under it.
                child: Material(
                  type: MaterialType.transparency,
                  child: Column(
                    children: [
                      CheckboxListTile(
                        value: _picked.contains(area.slug),
                        onChanged: (on) => _tickArea(area, on ?? false),
                        title: Text(
                          starterName(t, (slug: area.slug, name: area.name)),
                          style: Theme.of(context).textTheme.titleMedium,
                        ),
                      ),
                      for (final topic in area.topics)
                        CheckboxListTile(
                          contentPadding: const EdgeInsets.only(
                            left: 48,
                            right: 16,
                          ),
                          value: _picked.contains(topic.slug),
                          onChanged: (on) =>
                              _tickTopic(area.slug, topic.slug, on ?? false),
                          title: Text(starterName(t, topic)),
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
