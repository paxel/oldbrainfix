import 'dart:async';

import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'app_lock.dart';
import 'l10n.dart';
import 'voice_platform.dart';

/// While a recording plays, the slim bar at the top of every screen,
/// under the status bar and above any announcement, pushing [child] down
/// below it (obf-1155, UI laws "The playing bar"): the Note's name, a tap
/// on it opening the Note, and Stop. A recording of a locked Area, or
/// any while the App Lock is locked, shows "Private recording" instead
/// of the name. It goes once the recording ends or stops, and does not
/// show on the page of the Note playing, whose own stop button is there.
class PlayingBar extends StatelessWidget {
  final Playback playback;
  final AppLock lock;
  final Notes notes;

  /// The page on top now, popups and menus over it left out.
  final Route<dynamic>? Function() page;

  /// Opens the Note [noteId], unlocking first where it must.
  final void Function(String noteId) onOpen;
  final Widget child;

  const PlayingBar({
    super.key,
    required this.playback,
    required this.lock,
    required this.notes,
    required this.page,
    required this.onOpen,
    required this.child,
  });

  @override
  Widget build(BuildContext context) => ListenableBuilder(
    listenable: Listenable.merge([playback, playback.pages, lock]),
    builder: (context, _) {
      final noteId = playback.noteId;
      final top = page();
      final onItsPage =
          noteId != null && top != null && playback.pages.noteOf(top) == noteId;
      final shown = playback.value != null && !onItsPage;
      return Column(
        children: [
          if (shown) _bar(context, noteId),
          Expanded(
            // The bar took the status bar's height; the page must not pad
            // for it again. Always this widget, so the page keeps its
            // state as the bar comes and goes.
            child: MediaQuery.removePadding(
              context: context,
              removeTop: shown,
              child: child,
            ),
          ),
        ],
      );
    },
  );

  Widget _bar(BuildContext context, String? noteId) {
    final t = context.t;
    final scheme = Theme.of(context).colorScheme;
    final note = noteId == null ? null : notes.byId(noteId);
    final private = lock.locked || (note != null && lock.hides(note));
    final name = private
        ? t.privateRecording
        : note == null
        ? t.voiceRecording
        : noteText(t, note).split('\n').first;
    final style = TextStyle(color: scheme.onSecondaryContainer);
    return Material(
      key: const ValueKey('playingBar'),
      color: scheme.secondaryContainer,
      child: SafeArea(
        bottom: false,
        child: Padding(
          padding: const EdgeInsets.fromLTRB(16, 0, 8, 0),
          child: Row(
            children: [
              Icon(Icons.graphic_eq, color: scheme.onSecondaryContainer),
              const SizedBox(width: 8),
              Expanded(
                child: InkWell(
                  key: const ValueKey('playingBarName'),
                  onTap: noteId == null ? null : () => onOpen(noteId),
                  child: Padding(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 4,
                      vertical: 12,
                    ),
                    child: Text(
                      name,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: style,
                    ),
                  ),
                ),
              ),
              // No tooltip: there is no overlay above the pages.
              TextButton.icon(
                key: const ValueKey('playingBarStop'),
                onPressed: () => unawaited(playback.stop()),
                icon: const Icon(Icons.stop),
                label: Text(t.stopPlaying),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
