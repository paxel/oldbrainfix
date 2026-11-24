import 'dart:async';

import 'package:flutter/material.dart';

import 'l10n.dart';

/// The most entries a menu of actions shows (UI laws, #321): the rest go
/// under "More…".
const menuMost = 7;

/// An entry that opens a group of more entries in a second menu at the
/// same place (obf-1014): its [icon], its [label] and a ▸ at its end.
PopupMenuItem<T> groupMenuItem<T>({
  required Key key,
  required T value,
  required Widget icon,
  required String label,
}) => PopupMenuItem(
  key: key,
  value: value,
  child: Row(
    children: [
      icon,
      const SizedBox(width: 12),
      Expanded(child: Text(label)),
      const SizedBox(width: 12),
      const Icon(Icons.chevron_right),
    ],
  ),
);

/// Opens a menu of [items] at [at] (global), as a menu at the finger
/// does; the value picked, or null.
Future<T?> showMenuAt<T>(
  BuildContext context,
  Offset at,
  List<PopupMenuEntry<T>> items,
) {
  final overlay = Overlay.of(context).context.findRenderObject() as RenderBox?;
  if (overlay == null) return Future.value();
  return showMenu<T>(
    context: context,
    position: RelativeRect.fromRect(
      overlay.globalToLocal(at) & Size.zero,
      Offset.zero & overlay.size,
    ),
    items: items,
  );
}

/// The entries of a group of a card's ⋮ ([items], each doing what it
/// says when picked), at most [menuMost]: past that the last one is
/// "More…", which opens the rest where [anchor] is.
List<PopupMenuEntry<VoidCallback>> cappedMenu(
  BuildContext anchor,
  List<PopupMenuEntry<VoidCallback>> items,
) {
  if (items.length <= menuMost) return items;
  final rest = items.sublist(menuMost - 1);
  return [
    ...items.take(menuMost - 1),
    groupMenuItem(
      key: const ValueKey('menu-more'),
      value: () => unawaited(openMenuGroup(anchor, rest)),
      icon: const Icon(Icons.more_horiz),
      label: anchor.t.menuMore,
    ),
  ];
}

/// Opens the group [items] as a second menu under [anchor], the button
/// the first menu opened at, and does the one picked (obf-1014).
Future<void> openMenuGroup(
  BuildContext anchor,
  List<PopupMenuEntry<VoidCallback>> items,
) async {
  final box = anchor.findRenderObject() as RenderBox?;
  if (box == null) return;
  final picked = await showMenuAt(
    anchor,
    box.localToGlobal(box.size.bottomLeft(Offset.zero)),
    cappedMenu(anchor, items),
  );
  if (picked == null || !anchor.mounted) return;
  picked();
}
