import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'area_look.dart';
import 'l10n.dart';
import 'look.dart';

/// How the Agenda's rows are measured (#208): tighter than 0.3.1's, not
/// as tight as the widget's, every box and button still [box] wide and
/// high to tap. All of it grows by the [LookScale] of the theme, with
/// "Fat fingers" (#224).
class AgendaSizes {
  /// [LookScale]'s factor.
  final double factor;

  const AgendaSizes(this.factor);

  /// The measures for the theme around [context].
  factory AgendaSizes.of(BuildContext context) =>
      AgendaSizes(LookScale.of(context));

  /// A box's and a button's side, the tap target included.
  double get box => 40 * factor;

  /// The room above and below a row's card; twice this between two rows.
  double get gap => 3 * factor;

  /// The room above and below a heading's words inside its card.
  double get inside => 2 * factor;

  /// How far an entry is indented: its box stands where the heading's
  /// words begin.
  double get indent => 4 + box + 4;

  /// How much further a Line under another Line is indented.
  double get step => 24 * factor;

  /// The size of an Area's icon on a heading.
  double get areaIcon => 20 * factor;

  /// The density that makes a shrink-wrapped box, 40 wide, [box] wide;
  /// the theme's own at the usual size.
  VisualDensity? get density {
    if (factor == 1) return null;
    final grown = (box - 40) / 4;
    return VisualDensity(horizontal: grown, vertical: grown);
  }

  /// The style of an icon button on a row, ⋮: [box] wide and high.
  ButtonStyle get button => IconButton.styleFrom(
    tapTargetSize: MaterialTapTargetSize.shrinkWrap,
    minimumSize: factor == 1 ? null : Size.square(box),
  );
}

/// The box of a heading or an entry on the Agenda, [AgendaSizes.box]
/// square. With [onChanged] null it only shows: checked or not, in its
/// usual colours rather than greyed, and a tap on it does nothing, not
/// even what a tap on its row would do (a Session's, #208).
class AgendaBox extends StatelessWidget {
  final bool value;
  final String label;
  final ValueChanged<bool>? onChanged;

  const AgendaBox({
    super.key,
    required this.value,
    required this.label,
    required this.onChanged,
  });

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final changed = onChanged;
    final box = Checkbox(
      value: value,
      semanticLabel: label,
      // 40 by 40, not 48, so the rows lie closer (#208).
      materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
      visualDensity: AgendaSizes.of(context).density,
      onChanged: changed == null ? null : (on) => changed(on ?? false),
      fillColor: changed == null
          ? WidgetStateProperty.resolveWith(
              (states) => states.contains(WidgetState.selected)
                  ? scheme.primary
                  : Colors.transparent,
            )
          : null,
      checkColor: changed == null ? scheme.onPrimary : null,
      side: changed == null
          ? WidgetStateBorderSide.resolveWith(
              (states) => states.contains(WidgetState.selected)
                  ? null
                  : BorderSide(color: scheme.onSurfaceVariant, width: 2),
            )
          : null,
    );
    if (changed != null) return box;
    // Taken here, so the row under it does not open.
    return GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: () {},
      child: box,
    );
  }
}

/// One group of the Agenda as an embossed card (#208): a heading, led by
/// its box or, where it has none, by its kind's icon, with its time, its
/// words and what goes [under] them, its Area's colour and icon, [marks]
/// before the icon and ⋮ with [onMenu]; then its [entries] indented under
/// it, each an [AgendaEntry] with its box. A tap on the heading does
/// [onTap]; a long press opens [onMenu] where the finger is, and ⋮ at the
/// button.
class AgendaGroup extends StatelessWidget {
  final Area? area;

  /// The heading's box, or its kind's icon.
  final Widget lead;
  final String? time;
  final String title;

  /// Whether the heading's words are struck through, as when Done.
  final bool struck;

  /// What shows under the heading's words, such as how many a Session
  /// picks on a later day.
  final Widget? under;

  /// Small icons before the Area's, such as a Reminder's bell.
  final List<Widget> marks;
  final VoidCallback onTap;
  final void Function(Offset at)? onMenu;
  final Key? menuKey;

  /// The entries, and what goes with them, under the heading.
  final List<Widget> entries;

  const AgendaGroup({
    super.key,
    required this.area,
    required this.lead,
    this.time,
    required this.title,
    this.struck = false,
    this.under,
    this.marks = const [],
    required this.onTap,
    this.onMenu,
    this.menuKey,
    this.entries = const [],
  });

  /// The icon that leads a heading without a box, in a box's room.
  static Widget icon(BuildContext context, IconData icon) => SizedBox.square(
    dimension: AgendaSizes.of(context).box,
    child: Icon(icon, color: Theme.of(context).colorScheme.onSurfaceVariant),
  );

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final text = Theme.of(context).textTheme;
    final colour = areaColour(area?.colour);
    final time = this.time;
    final onMenu = this.onMenu;
    final sizes = AgendaSizes.of(context);
    final heading = Padding(
      padding: EdgeInsets.fromLTRB(
        4,
        sizes.inside,
        onMenu == null ? 12 : 0,
        sizes.inside,
      ),
      child: ConstrainedBox(
        constraints: BoxConstraints(minHeight: sizes.box),
        child: Row(
          children: [
            lead,
            const SizedBox(width: 4),
            Expanded(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  if (time != null) Text(time, style: text.labelLarge),
                  Text(
                    title,
                    maxLines: 2,
                    overflow: TextOverflow.ellipsis,
                    style: text.bodyLarge?.copyWith(
                      decoration: struck ? TextDecoration.lineThrough : null,
                    ),
                  ),
                  ?under,
                ],
              ),
            ),
            for (final mark in marks)
              Padding(padding: const EdgeInsets.only(left: 8), child: mark),
            if (areaIconData(area?.icon) case final icon?)
              Padding(
                padding: const EdgeInsets.only(left: 8),
                child: Icon(icon, size: sizes.areaIcon, color: colour),
              ),
            if (onMenu != null)
              // The same menu as the long press, at the button.
              Builder(
                builder: (button) => IconButton(
                  key: menuKey,
                  tooltip: t.more,
                  // 40 by 40, as the boxes.
                  style: sizes.button,
                  icon: const Icon(Icons.more_vert),
                  onPressed: () {
                    final box = button.findRenderObject() as RenderBox?;
                    if (box == null) return;
                    onMenu(box.localToGlobal(box.size.bottomLeft(Offset.zero)));
                  },
                ),
              ),
          ],
        ),
      ),
    );
    return Padding(
      padding: EdgeInsets.symmetric(horizontal: 16, vertical: sizes.gap),
      child: Embossed(
        child: ClipRRect(
          borderRadius: BorderRadius.circular(16),
          // The row's ink shows on the card, not under it.
          child: Material(
            type: MaterialType.transparency,
            child: Container(
              decoration: colour == null
                  ? null
                  : BoxDecoration(
                      border: Border(left: BorderSide(color: colour, width: 6)),
                    ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  GestureDetector(
                    onLongPressStart: onMenu == null
                        ? null
                        : (details) => onMenu(details.globalPosition),
                    child: InkWell(onTap: onTap, child: heading),
                  ),
                  ...entries,
                  if (entries.isNotEmpty) SizedBox(height: sizes.inside),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

/// One entry under a heading of the Agenda (#208): a Line, a Shopping
/// List's Line or a Session's pick, all alike: its box and its words,
/// indented under the heading, [depth] steps further for a Line under
/// another. The box does [onBox] and is disabled while it is null; a tap
/// on the words does [onTap].
class AgendaEntry extends StatelessWidget {
  final Key? boxKey;
  final Key? textKey;
  final String text;
  final bool checked;

  /// Whether the words are struck through; as [checked] when not given.
  final bool? struck;
  final int depth;
  final VoidCallback? onBox;
  final VoidCallback? onTap;

  const AgendaEntry({
    super.key,
    this.boxKey,
    this.textKey,
    required this.text,
    required this.checked,
    this.struck,
    this.depth = 0,
    required this.onBox,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final struck = this.struck ?? checked;
    final onBox = this.onBox;
    final sizes = AgendaSizes.of(context);
    return InkWell(
      onTap: onTap,
      child: Padding(
        padding: EdgeInsets.only(
          left: sizes.indent + depth * sizes.step,
          right: 12,
        ),
        child: ConstrainedBox(
          constraints: BoxConstraints(minHeight: sizes.box),
          child: Row(
            children: [
              Checkbox(
                key: boxKey,
                value: checked,
                semanticLabel: text,
                materialTapTargetSize: MaterialTapTargetSize.shrinkWrap,
                visualDensity: sizes.density,
                onChanged: onBox == null ? null : (_) => onBox(),
              ),
              const SizedBox(width: 4),
              Expanded(
                child: Text(
                  text,
                  key: textKey,
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                  style: theme.textTheme.bodyLarge?.copyWith(
                    decoration: struck ? TextDecoration.lineThrough : null,
                    color: struck ? theme.colorScheme.onSurfaceVariant : null,
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
