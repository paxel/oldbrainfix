import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'browser.dart';
import 'l10n.dart';
import 'page_bar.dart';
import 'system_bars.dart';

/// The tag tree (#267, Q623): every tag of the Notes by its levels, each
/// with how many Notes carry it or a tag below it. A tap on a tag closes
/// the page with it, for the Areas list to search for it, as a tap on a
/// tag in a text does (ADR 0032). The levels below a tag unfold by its
/// arrow; at first only the top level shows. The tree is read when the
/// page opens.
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

  @override
  void initState() {
    super.initState();
    _tree = TagTree(widget.store).tree();
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
