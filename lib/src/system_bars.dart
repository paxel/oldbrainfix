import 'package:flutter/material.dart';

/// [padding] for a page's list, with room for the system bars added. A
/// list given its own padding no longer leaves that room by itself, and
/// the end of the page would sit under the navigation bar. While the
/// keyboard is up it covers the bar, and the room goes with it.
EdgeInsets clearOfSystemBars(BuildContext context, EdgeInsets padding) =>
    padding + MediaQuery.paddingOf(context);

/// The height of a standard floating button.
const floatingButtonHeight = 56.0;

/// [clearOfSystemBars] for a page with floating buttons [buttons] high:
/// the end of the list also scrolls above them, with a gap, so the last
/// row's ⋮, checkbox and text can be reached. The buttons sit their
/// margin above the system bars, and the room counts both.
EdgeInsets clearOfFloatingButtons(
  BuildContext context,
  EdgeInsets padding, {
  double buttons = floatingButtonHeight,
}) => clearOfSystemBars(
  context,
  padding + EdgeInsets.only(bottom: buttons + kFloatingActionButtonMargin + 16),
);
