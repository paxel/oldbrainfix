import 'package:flutter/material.dart';

import 'l10n.dart';
import 'look.dart';
import 'sync_line.dart';

/// Width from which a screen counts as wide (UI laws, "The bar"): below
/// it, a phone in either orientation, the title has a line of its own.
const wideScreenWidth = 840.0;

/// Height of the title's line under the buttons, for a title on one line.
const titleLineHeight = 44.0;

/// Lines a title wraps to under the buttons before it ends in "…".
const titleLines = 2;

/// Room [titleIcon] takes before the title: 36 wide and a gap of 8.
const _iconRoom = 44.0;

/// An action of a page's bar that goes into ⋮ when the bar is full (UI
/// laws, "The bar"): on the bar its icon with [label] as its tooltip, in
/// ⋮ its icon and [label], greyed in both while [onPressed] is null. Any
/// other widget among a bar's actions stays on the bar.
class BarAction extends StatelessWidget {
  final IconData icon;
  final String label;
  final VoidCallback? onPressed;

  const BarAction({
    super.key,
    required this.icon,
    required this.label,
    required this.onPressed,
  });

  @override
  Widget build(BuildContext context) =>
      IconButton(tooltip: label, onPressed: onPressed, icon: Icon(icon));
}

/// [actions] as a bar with room for [slots] buttons shows them (UI laws,
/// "The bar"): all of them while they fit, else the [BarAction]s that
/// fit, in their order, and ⋮ last with the rest. What is not a
/// [BarAction] never goes into ⋮.
List<Widget> barActions(List<Widget> actions, int slots) {
  if (actions.length <= slots) return actions;
  final fixed = actions.where((a) => a is! BarAction).length;
  var free = slots - 1 - fixed;
  final shown = <Widget>[];
  final more = <BarAction>[];
  for (final action in actions) {
    if (action is! BarAction) {
      shown.add(action);
    } else if (free > 0) {
      free--;
      shown.add(action);
    } else {
      more.add(action);
    }
  }
  if (more.isEmpty) return shown;
  return [...shown, _BarMore(actions: more)];
}

/// ⋮ at the end of a full bar, with the actions that did not fit.
class _BarMore extends StatelessWidget {
  final List<BarAction> actions;

  const _BarMore({required this.actions});

  @override
  Widget build(BuildContext context) => PopupMenuButton<VoidCallback>(
    key: const ValueKey('bar-more'),
    tooltip: context.t.more,
    icon: const Icon(Icons.more_vert),
    onSelected: (run) => run(),
    itemBuilder: (_) => [
      for (final action in actions)
        PopupMenuItem(
          key: action.key,
          value: action.onPressed,
          enabled: action.onPressed != null,
          child: Row(
            children: [
              Icon(action.icon),
              const SizedBox(width: 12),
              Flexible(child: Text(action.label)),
            ],
          ),
        ),
    ],
  );
}

/// The one bar of every phone page with a title (UI laws, "The bar").
///
/// On a phone the back button and [actions] sit on their own row and
/// [title] on the line under them, wrapped onto a second line rather than
/// cut, so it is read in full and the buttons have the whole width; on a
/// wide screen it is the ordinary bar with the title beside them. A page
/// with no button at all, neither a way back nor an action, keeps its
/// title on the one row: an empty row above it would hold nothing.
/// [titleIcon], at most 36 wide, stands before the title. The [SyncLine]
/// runs along its bottom while this Device syncs (#225). [onBack] is the
/// way back of a page that is no route of its own, the Areas list beside
/// the Agenda (#258).
AppBar pageBar(
  BuildContext context, {
  required String title,
  Widget? titleIcon,
  List<Widget> actions = const [],
  VoidCallback? onBack,
}) {
  // Only whether it can go back, not the whole route: a page coming on
  // top does not build this page again (#210).
  final back = onBack != null || (ModalRoute.canPopOf(context) ?? false);
  final leading = onBack == null ? null : BackButton(onPressed: onBack);
  final width = MediaQuery.sizeOf(context).width;
  if (width >= wideScreenWidth || (actions.isEmpty && !back)) {
    return AppBar(
      leading: leading,
      title: titleIcon == null
          ? Text(title)
          : Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                titleIcon,
                const SizedBox(width: 8),
                Flexible(child: Text(title, overflow: TextOverflow.ellipsis)),
              ],
            ),
      actions: actions,
      flexibleSpace: const SyncLine(),
    );
  }
  final theme = Theme.of(context);
  // A button is 48 wide, grown with "Fat fingers" (#224); the way back
  // takes the bar's leading room.
  final slot = kMinInteractiveDimension * LookScale.of(context);
  final room =
      width -
      MediaQuery.paddingOf(context).horizontal -
      (back ? theme.appBarTheme.leadingWidth ?? kToolbarHeight : 0);
  final style =
      (theme.appBarTheme.titleTextStyle ??
              theme.textTheme.titleLarge ??
              const TextStyle())
          .copyWith(color: theme.colorScheme.onSurface);
  // The line grows with a title that wraps, so the second line is not
  // cut off by the bar's edge. The bar keeps clear of a notch at the side.
  final measure =
      TextPainter(
        text: TextSpan(text: title, style: style),
        textDirection: Directionality.of(context),
        textScaler: MediaQuery.textScalerOf(context),
        maxLines: titleLines,
      )..layout(
        maxWidth:
            width -
            32 -
            MediaQuery.paddingOf(context).horizontal -
            (titleIcon == null ? 0 : _iconRoom),
      );
  final height = measure.height + 16 > titleLineHeight
      ? measure.height + 16
      : titleLineHeight;
  measure.dispose();
  final text = Text(
    title,
    style: style,
    maxLines: titleLines,
    overflow: TextOverflow.ellipsis,
  );
  return AppBar(
    leading: leading,
    actions: barActions(actions, (room / slot).floor()),
    flexibleSpace: const SyncLine(),
    bottom: PreferredSize(
      preferredSize: Size.fromHeight(height),
      child: SizedBox(
        height: height,
        width: double.infinity,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16),
          child: Align(
            alignment: AlignmentDirectional.centerStart,
            // Read as the page's name, as the bar's own title is.
            child: Semantics(
              header: true,
              namesRoute: switch (theme.platform) {
                TargetPlatform.android ||
                TargetPlatform.fuchsia ||
                TargetPlatform.linux ||
                TargetPlatform.windows => true,
                TargetPlatform.iOS || TargetPlatform.macOS => null,
              },
              child: titleIcon == null
                  ? text
                  : Row(
                      children: [
                        titleIcon,
                        const SizedBox(width: 8),
                        Expanded(child: text),
                      ],
                    ),
            ),
          ),
        ),
      ),
    ),
  );
}
