import 'dart:async';

import 'package:flutter/material.dart';

import 'l10n.dart';

/// Everything the app has to say, oldest first. Only the first one is
/// shown; the rest wait behind it (UI laws, "Announcements").
class Announcements extends ChangeNotifier {
  /// How long an offer to undo shows before it goes by itself (#268).
  static const undoLasts = Duration(seconds: 10);

  final _waiting = <_Said>[];

  /// Counts the undos taken (#268), for a page that does not follow the
  /// log, such as the Note editor, to read what came back.
  final undone = ValueNotifier<int>(0);

  /// The announcement on screen, if any.
  String? get visible => _waiting.firstOrNull?.text;

  /// What tapping the visible announcement opens, if anything.
  VoidCallback? get visibleAction => _waiting.firstOrNull?.onTap;

  /// Whether the visible announcement offers to undo what was just done:
  /// its tap undoes it, and it goes by itself after [undoLasts].
  bool get visibleUndoes => _waiting.firstOrNull?.undoes ?? false;

  /// Queues [text], already in the reader's language. Tapping it
  /// dismisses it and then runs [onTap], when there is one.
  void announce(String text, {VoidCallback? onTap}) {
    _waiting.add(_Said(text, onTap));
    notifyListeners();
  }

  /// Says at once that [text] was just done, such as "Moved to the
  /// Trash", and offers to undo it (#268): a tap runs [undo]; left alone
  /// it goes by itself after [undoLasts]. Only the latest offer stays,
  /// and it shows before what waits.
  void offerUndo(String text, VoidCallback undo) {
    void undoing() {
      undo();
      undone.value++;
    }

    _waiting
      ..removeWhere((said) => said.undoes)
      ..insert(0, _Said(text, undoing, undoes: true));
    notifyListeners();
  }

  /// The visible announcement goes; the next one shows.
  void dismiss() {
    if (_waiting.isEmpty) return;
    _waiting.removeAt(0);
    notifyListeners();
  }

  @override
  void dispose() {
    undone.dispose();
    super.dispose();
  }

  /// The visible offer to undo goes by itself, as long as [said] is
  /// still the one showing.
  void _expire(_Said said) {
    if (_waiting.firstOrNull != said) return;
    _waiting.removeAt(0);
    notifyListeners();
  }
}

/// One announcement: its text, what its tap runs, and whether that
/// undoes what was just done.
class _Said {
  final String text;
  final VoidCallback? onTap;
  final bool undoes;

  _Said(this.text, this.onTap, {this.undoes = false});
}

/// Hands the app's [Announcements] down to every screen.
class AnnouncementsScope extends InheritedWidget {
  final Announcements announcements;

  const AnnouncementsScope({
    super.key,
    required this.announcements,
    required super.child,
  });

  /// The app's [Announcements], if there is an app; read once, so it
  /// does not listen.
  static Announcements? of(BuildContext context) => context
      .getInheritedWidgetOfExactType<AnnouncementsScope>()
      ?.announcements;

  @override
  bool updateShouldNotify(AnnouncementsScope oldWidget) =>
      announcements != oldWidget.announcements;
}

/// Shows the visible announcement at the top of the screen, under the
/// status bar, and pushes [child] down below it. Tapping it dismisses
/// it and opens what it is about, if anything; an offer to undo goes by
/// itself after [Announcements.undoLasts]. Nothing ever comes from the
/// bottom (adapted from Catlog's note strip).
class AnnouncementStrip extends StatefulWidget {
  final Announcements announcements;
  final Widget child;

  const AnnouncementStrip({
    super.key,
    required this.announcements,
    required this.child,
  });

  @override
  State<AnnouncementStrip> createState() => _AnnouncementStripState();
}

class _AnnouncementStripState extends State<AnnouncementStrip> {
  /// The offer to undo whose time runs, and its timer.
  _Said? _timed;
  Timer? _expiry;

  @override
  void initState() {
    super.initState();
    widget.announcements.addListener(_changed);
    _changed();
  }

  @override
  void didUpdateWidget(AnnouncementStrip old) {
    super.didUpdateWidget(old);
    if (old.announcements == widget.announcements) return;
    old.announcements.removeListener(_changed);
    widget.announcements.addListener(_changed);
    _changed();
  }

  @override
  void dispose() {
    widget.announcements.removeListener(_changed);
    _expiry?.cancel();
    super.dispose();
  }

  /// Starts the time of an offer to undo once it shows, and stops it
  /// once it no longer does.
  void _changed() {
    final announcements = widget.announcements;
    final shown = announcements._waiting.firstOrNull;
    final said = shown != null && shown.undoes ? shown : null;
    if (said == _timed) return;
    _expiry?.cancel();
    _timed = said;
    _expiry = said == null
        ? null
        : Timer(Announcements.undoLasts, () => announcements._expire(said));
  }

  @override
  Widget build(BuildContext context) => ListenableBuilder(
    listenable: widget.announcements,
    builder: (context, _) {
      final announcements = widget.announcements;
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
                : _Line(
                    text: text,
                    undoes: announcements.visibleUndoes,
                    onTap: () {
                      final action = announcements.visibleAction;
                      announcements.dismiss();
                      action?.call();
                    },
                  ),
          ),
          Expanded(
            // The strip took the status bar's height; the page must not
            // pad for it again.
            child: text == null
                ? widget.child
                : MediaQuery.removePadding(
                    context: context,
                    removeTop: true,
                    child: widget.child,
                  ),
          ),
        ],
      );
    },
  );
}

class _Line extends StatelessWidget {
  final String text;

  /// Whether the tap undoes what was just done: "Undo" shows at the end.
  final bool undoes;
  final VoidCallback onTap;

  const _Line({required this.text, required this.undoes, required this.onTap});

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final t = context.t;
    final style = TextStyle(color: scheme.onPrimary);
    return Semantics(
      liveRegion: true,
      onTapHint: undoes ? t.undo : t.dismissAnnouncement,
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
                child: undoes
                    ? Row(
                        children: [
                          Expanded(child: Text(text, style: style)),
                          const SizedBox(width: 16),
                          Text(
                            t.undo,
                            key: const ValueKey('announcement-undo'),
                            style: style.copyWith(fontWeight: FontWeight.bold),
                          ),
                        ],
                      )
                    : Text(text, style: style),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
