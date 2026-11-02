import 'package:flutter/material.dart';

/// Width from which a screen counts as wide (UI laws, "The bar"): below
/// it, a phone in either orientation, the title has a line of its own.
const wideScreenWidth = 840.0;

/// Height of the title's line under the buttons, for a title on one line.
const titleLineHeight = 44.0;

/// Lines a title wraps to under the buttons before it ends in "…".
const titleLines = 2;

/// Room [titleIcon] takes before the title: 36 wide and a gap of 8.
const _iconRoom = 44.0;

/// The one bar of every phone page with a title (UI laws, "The bar").
///
/// On a phone the back button and [actions] sit on their own row and
/// [title] on the line under them, wrapped onto a second line rather than
/// cut, so it is read in full and the buttons have the whole width; on a
/// wide screen it is the ordinary bar with the title beside them. A page
/// with no button at all, neither a way back nor an action, keeps its
/// title on the one row: an empty row above it would hold nothing.
/// [titleIcon], at most 36 wide, stands before the title.
AppBar pageBar(
  BuildContext context, {
  required String title,
  Widget? titleIcon,
  List<Widget> actions = const [],
}) {
  // Only whether it can go back, not the whole route: a page coming on
  // top does not build this page again (#210).
  final back = ModalRoute.canPopOf(context) ?? false;
  final width = MediaQuery.sizeOf(context).width;
  if (width >= wideScreenWidth || (actions.isEmpty && !back)) {
    return AppBar(
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
    );
  }
  final theme = Theme.of(context);
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
    actions: actions,
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
