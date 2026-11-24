import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'app_lock.dart';
import 'browser.dart';
import 'l10n.dart';
import 'page_bar.dart';
import 'system_bars.dart';

/// The tag tree (#267, Q623), opened by "All tags…" at the end of the row
/// of tags under the Areas list's search (obf-1074): every tag of the
/// Notes by its levels, each with how many Notes carry it or a tag below
/// it. A tap on a tag closes
/// the page with it, for the Areas list to search for it, as a tap on a
/// tag in a text does (ADR 0032). The levels below a tag unfold by its
/// arrow; at first only the top level shows. The tree is read when the
/// page opens and again as an Area opens or closes, without the Notes of
/// the locked Areas closed then (Area Locks), as every list.
class TagTreeScreen extends StatefulWidget {
  final Store store;

  const TagTreeScreen({super.key, required this.store});

  @override
  State<TagTreeScreen> createState() => _TagTreeScreenState();
}

class _TagTreeScreenState extends State<TagTreeScreen> {
  List<TagNode> _tree = const [];

  /// The tags whose levels below show.
  final _open = <String>{};

  /// The app's lock, followed while the page shows.
  AppLock? _lock;
  var _started = false;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final lock = LockScope.of(context);
    if (_started && lock == _lock) return;
    _started = true;
    _lock?.removeListener(_locksChanged);
    _lock = lock?..addListener(_locksChanged);
    _tree = _read();
  }

  @override
  void dispose() {
    _lock?.removeListener(_locksChanged);
    super.dispose();
  }

  List<TagNode> _read() =>
      TagTree(widget.store)
          .tree(closed: _lock?.closedAreas ?? const <String>{});

  /// An Area opened or closed, or the app locked: read again.
  void _locksChanged() {
    if (mounted) setState(() => _tree = _read());
  }

  void _toggle(String tag) => setState(() {
    if (!_open.remove(tag)) _open.add(tag);
  });

  List<Widget> _rows(List<TagNode> level, int depth) => [
    for (final node in level) ...[
      _TagRow(
        key: ValueKey('tree-row-${node.tag}'),
        node: node,
        depth: depth,
        open: _open.contains(node.tag),
        onTap: () => Navigator.of(context).pop(node.tag),
        onToggle: node.children.isEmpty ? null : () => _toggle(node.tag),
      ),
      if (_open.contains(node.tag)) ..._rows(node.children, depth + 1),
    ],
  ];

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return Scaffold(
      appBar: pageBar(context, title: t.tagsTitle),
      body: ListView(
        padding: clearOfFloatingButtons(
          context,
          const EdgeInsets.symmetric(vertical: 8),
        ),
        children: _tree.isEmpty
            ? [BrowserEmpty(text: t.tagsNone)]
            : _rows(_tree, 0),
      ),
    );
  }
}

/// One tag of the tree, indented by its [depth]: the top level as the tag
/// is written, `#life`, a level below by its own part, `health`; its count
/// of Notes, and the arrow that unfolds the levels below when it has any.
class _TagRow extends StatelessWidget {
  final TagNode node;
  final int depth;
  final bool open;
  final VoidCallback onTap;
  final VoidCallback? onToggle;

  const _TagRow({
    super.key,
    required this.node,
    required this.depth,
    required this.open,
    required this.onTap,
    required this.onToggle,
  });

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final theme = Theme.of(context);
    final toggle = onToggle;
    return InkWell(
      onTap: onTap,
      child: Padding(
        padding: EdgeInsetsDirectional.only(start: 16.0 + 24.0 * depth),
        child: ConstrainedBox(
          constraints: const BoxConstraints(minHeight: 48),
          child: Row(
            children: [
              Expanded(
                child: Semantics(
                  button: true,
                  label: t.searchForTag(node.tag),
                  excludeSemantics: true,
                  child: Text(
                    depth == 0 ? node.tag : node.name,
                    style: theme.textTheme.titleMedium?.copyWith(
                      color: theme.colorScheme.primary,
                    ),
                  ),
                ),
              ),
              Text(
                t.tagNotes(node.notes),
                style: theme.textTheme.bodySmall?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
              ),
              if (toggle == null)
                const SizedBox(width: 48, height: 48)
              else
                IconButton(
                  key: ValueKey('tree-open-${node.tag}'),
                  tooltip: open ? t.tagFold : t.tagUnfold,
                  onPressed: toggle,
                  icon: Icon(open ? Icons.expand_less : Icons.expand_more),
                ),
            ],
          ),
        ),
      ),
    );
  }
}

/// The most used tags of [tree] for the row under the Areas list's search
/// (obf-1074): the tags of every level, those more Notes carry first, of
/// as many the one first in the tree, at most [count].
List<TagNode> mostUsedTags(List<TagNode> tree, {int count = 12}) {
  final all = <TagNode>[];
  void walk(List<TagNode> level) {
    for (final node in level) {
      all.add(node);
      walk(node.children);
    }
  }

  walk(tree);
  final order = {for (final (i, node) in all.indexed) node: i};
  all.sort((a, b) {
    final more = b.notes.compareTo(a.notes);
    return more != 0 ? more : (order[a] ?? 0).compareTo(order[b] ?? 0);
  });
  return all.take(count).toList();
}

/// One row of [tags] as chips under the Areas list's search field
/// (obf-1074), ending in "All tags…", which runs [onAll]; a tap on a tag
/// runs [onPick] with it. Wider than its room it scrolls sideways.
class TagChipRow extends StatelessWidget {
  final List<TagNode> tags;
  final ValueChanged<String> onPick;
  final VoidCallback onAll;

  const TagChipRow({
    super.key,
    required this.tags,
    required this.onPick,
    required this.onAll,
  });

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    // From the start edge, also where it fits.
    return SizedBox(
      width: double.infinity,
      child: SingleChildScrollView(
        key: const ValueKey('tag-chips'),
        scrollDirection: Axis.horizontal,
        padding: const EdgeInsets.fromLTRB(16, 8, 16, 0),
        child: Row(
          spacing: 8,
          children: [
            for (final node in tags)
              ActionChip(
                key: ValueKey('tag-chip-${node.tag}'),
                label: Text(node.tag),
                tooltip: t.searchForTag(node.tag),
                onPressed: () => onPick(node.tag),
              ),
            ActionChip(
              key: const ValueKey('tag-chips-all'),
              avatar: const Icon(Icons.tag),
              label: Text(t.allTags),
              onPressed: onAll,
            ),
          ],
        ),
      ),
    );
  }
}
