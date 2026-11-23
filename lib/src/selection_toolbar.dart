import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'l10n.dart';

/// Where Search in the toolbar above a selection goes (obf-1055): the
/// find bar of the Note shown, finding the words selected. Below none,
/// as on a page or dialog of its own, the toolbar has no Search.
class SelectionSearchScope extends InheritedWidget {
  final ValueChanged<String> onSearch;

  const SelectionSearchScope({
    super.key,
    required this.onSearch,
    required super.child,
  });

  static ValueChanged<String>? of(BuildContext context) =>
      context.getInheritedWidgetOfExactType<SelectionSearchScope>()?.onSearch;

  @override
  bool updateShouldNotify(SelectionSearchScope oldWidget) =>
      onSearch != oldWidget.onSearch;
}

/// The words of [selected] the find bar looks for: as they read in
/// [format], without marks or the ids of boxes, their first line that
/// holds any, since a place found never spans lines; '' for none. A
/// selection that cuts a formatted span keeps no half of its marks.
String selectionWords(String selected, String format) {
  var visible = TextFormats.visible(format, selected);
  if (format == TextFormats.markdown) {
    visible = visible.replaceAll(_cutMarks, '');
  }
  for (final line in visible.split('\n')) {
    final words = line.trim().replaceAll(RegExp(r'\s+'), ' ');
    if (words.isNotEmpty) return words;
  }
  return '';
}

/// The marks of bold, italic, strike-through and code left where a
/// selection cut their span.
final _cutMarks = RegExp(r'[*~`]+|_{2,}');

/// The small toolbar just above a selection on the phone (obf-1055), in
/// place of the system's menu: Copy while something is selected, Paste
/// where the field takes it, Search with [words] when [onSearch] is
/// given, and the [extra] entries after them. [system] are the
/// framework's entries, whose Copy and Paste it keeps; a long press on
/// an empty spot of a field so shows only Paste, and nothing where there
/// is nothing to offer.
Widget selectionToolbar(
  BuildContext context, {
  required TextSelectionToolbarAnchors anchors,
  required List<ContextMenuButtonItem> system,
  required String words,
  required VoidCallback hide,
  ValueChanged<String>? onSearch,
  List<ContextMenuButtonItem> extra = const [],
}) {
  ContextMenuButtonItem? of(ContextMenuButtonType type) {
    for (final item in system) {
      if (item.type == type) return item;
    }
    return null;
  }

  final items = [
    if (words.isNotEmpty) ?of(ContextMenuButtonType.copy),
    ?of(ContextMenuButtonType.paste),
    if (onSearch != null && words.isNotEmpty)
      ContextMenuButtonItem(
        label: context.t.selectionSearch,
        onPressed: () {
          hide();
          onSearch(words);
        },
      ),
    ...extra,
  ];
  if (items.isEmpty) return const SizedBox.shrink();
  return AdaptiveTextSelectionToolbar.buttonItems(
    anchors: anchors,
    buttonItems: items,
  );
}
