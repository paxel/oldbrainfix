import 'package:flutter/material.dart';

import 'l10n.dart';
import 'photo_platform.dart';
import 'voice_platform.dart';

/// A way the Agenda's "+" fans out to capture a Note.
enum CaptureWay { text, voice, photo, gallery, paste }

/// The Agenda's "+" opened: the [ways] fanned out upward from the button
/// at [anchor], each an icon with its words, Text nearest the button,
/// and the button itself reading as close (UI laws, "Capture"). Picking a
/// way pops it; a second tap on the button, a tap outside or Back closes
/// it with null. Nothing slides in from the bottom.
class CaptureFanRoute extends PopupRoute<CaptureWay> {
  /// Where the "+" button is, in the navigator's coordinates.
  final Rect anchor;

  /// The ways to show, in the order of [CaptureWay].
  final List<CaptureWay> ways;

  @override
  final String barrierLabel;

  CaptureFanRoute({
    required this.anchor,
    required this.ways,
    required this.barrierLabel,
  });

  @override
  Color get barrierColor => Colors.black26;

  @override
  bool get barrierDismissible => true;

  @override
  Duration get transitionDuration => const Duration(milliseconds: 150);

  @override
  Widget buildPage(
    BuildContext context,
    Animation<double> animation,
    Animation<double> secondaryAnimation,
  ) {
    final t = context.t;
    Widget entry(CaptureWay way) {
      final (icon, label) = switch (way) {
        CaptureWay.text => (Icons.edit_note, t.text),
        CaptureWay.voice => (Icons.mic_none, t.voice),
        CaptureWay.photo => (Icons.photo_camera_outlined, t.photo),
        CaptureWay.gallery => (Icons.photo_library_outlined, t.gallery),
        CaptureWay.paste => (Icons.content_paste, t.paste),
      };
      return Padding(
        padding: const EdgeInsets.only(top: 12),
        child: FloatingActionButton.extended(
          key: ValueKey('captureFan-${way.name}'),
          heroTag: null,
          onPressed: () => Navigator.of(context).pop(way),
          icon: Icon(icon),
          label: Text(label),
        ),
      );
    }

    return FadeTransition(
      opacity: animation,
      child: LayoutBuilder(
        builder: (context, box) => Stack(
          children: [
            Positioned(
              right: box.maxWidth - anchor.right,
              bottom: box.maxHeight - anchor.top + 4,
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.end,
                children: [for (final way in ways.reversed) entry(way)],
              ),
            ),
            Positioned.fromRect(
              rect: anchor,
              child: FloatingActionButton(
                key: const ValueKey('captureFanClose'),
                heroTag: null,
                tooltip: t.captureFanClose,
                onPressed: () => Navigator.of(context).pop(),
                child: const Icon(Icons.close),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// The fan for the "+" button laid out at [plus], with the ways this
/// Device has; Text always. Null while [plus] is not laid out.
CaptureFanRoute? captureFanFor(BuildContext context, GlobalKey plus) {
  final button = plus.currentContext?.findRenderObject();
  final over = Navigator.of(context).context.findRenderObject();
  if (button is! RenderBox || over is! RenderBox) return null;
  final photos = PhotoScope.of(context);
  return CaptureFanRoute(
    anchor: button.localToGlobal(Offset.zero, ancestor: over) & button.size,
    ways: [
      CaptureWay.text,
      if (VoiceScope.of(context).available) CaptureWay.voice,
      if (photos.available) CaptureWay.photo,
      if (photos.pickable) CaptureWay.gallery,
      if (photos.pasteable) CaptureWay.paste,
    ],
    barrierLabel: MaterialLocalizations.of(context).modalBarrierDismissLabel,
  );
}
