import 'package:flutter/widgets.dart';

/// [padding] for a page's list, with room for the system bars added. A
/// list given its own padding no longer leaves that room by itself, and
/// the end of the page would sit under the navigation bar. While the
/// keyboard is up it covers the bar, and the room goes with it.
EdgeInsets clearOfSystemBars(BuildContext context, EdgeInsets padding) =>
    padding + MediaQuery.paddingOf(context);
