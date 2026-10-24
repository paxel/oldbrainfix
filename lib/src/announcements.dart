import 'package:flutter/material.dart';

import 'l10n.dart';

/// Everything the app has to say, oldest first. Only the first one is
/// shown; the rest wait behind it (UI laws, "Announcements").
class Announcements extends ChangeNotifier {
  final _waiting = <String>[];

  /// The announcement on screen, if any.
  String? get visible => _waiting.firstOrNull;

  /// Queues [text], already in the reader's language.
  void announce(String text) {
    _waiting.add(text);
    notifyListeners();
  }

  /// The visible announcement goes; the next one shows.
  void dismiss() {
    if (_waiting.isEmpty) return;
    _waiting.removeAt(0);
    notifyListeners();
  }
}

/// Shows the visible announcement at the top of the screen, under the
/// status bar, and pushes [child] down below it. Tapping it dismisses
/// it. Nothing ever comes from the bottom (adapted from Catlog's
/// note strip).
class AnnouncementStrip extends StatelessWidget {
  final Announcements announcements;
  final Widget child;

  const AnnouncementStrip({
    super.key,
    required this.announcements,
    required this.child,
  });

  @override
  Widget build(BuildContext context) => ListenableBuilder(
    listenable: announcements,
    builder: (context, _) {
      final text = announcements.visible;
      return Column(
        children: [
          // The page moves down slowly enough to be noticed before it
          // moves under a thumb.
          AnimatedSize(
            duration: const Duration(milliseconds: 450),
            curve: Curves.easeIn,
            alignment: Alignment.topCenter,
            child: text == null
                ? const SizedBox(width: double.infinity)
                : _Line(text: text, onTap: announcements.dismiss),
          ),
          Expanded(
            // The strip took the status bar's height; the page must not
            // pad for it again.
            child: text == null
                ? child
                : MediaQuery.removePadding(
                    context: context,
                    removeTop: true,
                    child: child,
                  ),
          ),
        ],
      );
    },
  );
}

class _Line extends StatelessWidget {
  final String text;
  final VoidCallback onTap;

  const _Line({required this.text, required this.onTap});

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Semantics(
      liveRegion: true,
      onTapHint: context.t.dismissAnnouncement,
      child: Material(
        color: scheme.primary,
        child: InkWell(
          onTap: onTap,
          child: SafeArea(
            bottom: false,
            child: Padding(
              padding: const EdgeInsets.fromLTRB(16, 12, 16, 12),
              child: SizedBox(
                width: double.infinity,
                child: Text(text, style: TextStyle(color: scheme.onPrimary)),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
