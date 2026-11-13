import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/semantics.dart';

import 'file_platform.dart';
import 'l10n.dart';
import 'photo_platform.dart';
import 'voice_platform.dart';

/// A way the Agenda's "+" fans out to capture a Note.
enum CaptureWay { text, voice, photo, gallery, paste, file }

/// One entry of a fan: its key, its icon and its words.
typedef FanEntry = ({Key key, Widget icon, String label});

/// A "+" opened: the [values] fanned out upward from the button at
/// [anchor], each an icon with its words as [describe] gives them, the
/// first nearest the button, and the button itself reading as close (UI
/// laws, "Capture"). Picking one pops it; a second tap on the button, a
/// tap outside or Back closes it with null. Nothing slides in from the
/// bottom; a fan taller than the screen scrolls.
class FanRoute<T> extends PopupRoute<T> {
  /// Where the "+" button is, in the navigator's coordinates.
  final Rect anchor;

  /// The values to show, the first nearest the button.
  final List<T> values;

  /// The entry of each value, in the words of the context given.
  final FanEntry Function(BuildContext context, T value) describe;

  /// The key of the button while it reads as close.
  final Key closeKey;

  @override
  final String barrierLabel;

  FanRoute({
    required this.anchor,
    required this.values,
    required this.describe,
    required this.closeKey,
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
    Widget entry(T value) {
      final (:key, :icon, :label) = describe(context, value);
      return Padding(
        padding: const EdgeInsets.only(top: 12),
        child: FloatingActionButton.extended(
          key: key,
          heroTag: null,
          onPressed: () => Navigator.of(context).pop(value),
          icon: icon,
          label: Text(label),
        ),
      );
    }

    final top = MediaQuery.paddingOf(context).top;
    return FadeTransition(
      opacity: animation,
      child: LayoutBuilder(
        builder: (context, box) => Stack(
          children: [
            Positioned(
              top: top,
              right: box.maxWidth - anchor.right,
              bottom: box.maxHeight - anchor.top + 4,
              child: Align(
                alignment: Alignment.bottomRight,
                child: SingleChildScrollView(
                  // The entry nearest the button stays in view.
                  reverse: true,
                  clipBehavior: Clip.none,
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    crossAxisAlignment: CrossAxisAlignment.end,
                    children: [
                      for (final value in values.reversed) entry(value),
                    ],
                  ),
                ),
              ),
            ),
            Positioned.fromRect(
              rect: anchor,
              child: FloatingActionButton(
                key: closeKey,
                heroTag: null,
                tooltip: context.t.captureFanClose,
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

/// Where the button laid out under [button] is, in the coordinates of
/// [context]'s navigator; null while it is not laid out.
Rect? fanAnchor(BuildContext context, BuildContext? button) {
  final box = button?.findRenderObject();
  final over = Navigator.of(context).context.findRenderObject();
  if (box is! RenderBox || over is! RenderBox || !box.hasSize) return null;
  return box.localToGlobal(Offset.zero, ancestor: over) & box.size;
}

/// The "+" of the Agenda and the lock screen (UI laws, "Capture"): a
/// tap runs [onPressed], which fans out the ways to capture; a long press
/// runs [onLongPress], which opens text Capture at once (#282), with the
/// platform's touch as a card's long press gives it. A screen reader
/// hears the long press with the button, and on the iPhone, whose
/// VoiceOver has no long press, finds it among the button's actions.
/// [plusKey] and [heroTag] are the button's, where the fan opens from.
class CapturePlus extends StatelessWidget {
  final Key plusKey;
  final Object heroTag;
  final VoidCallback onPressed;
  final VoidCallback onLongPress;

  const CapturePlus({
    super.key,
    required this.plusKey,
    required this.heroTag,
    required this.onPressed,
    required this.onLongPress,
  });

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return MergeSemantics(
      child: Semantics(
        onLongPressHint: t.captureLongPressHint,
        customSemanticsActions: defaultTargetPlatform == TargetPlatform.iOS
            ? {
                CustomSemanticsAction(label: t.captureLongPressAction):
                    onLongPress,
              }
            : null,
        // Its own label and hover, without the tooltip's long press,
        // which is the capture's.
        child: Tooltip(
          message: t.captureTooltip,
          triggerMode: TooltipTriggerMode.manual,
          child: GestureDetector(
            onLongPress: () {
              unawaited(Feedback.forLongPress(context));
              onLongPress();
            },
            child: FloatingActionButton(
              key: plusKey,
              heroTag: heroTag,
              onPressed: onPressed,
              child: const Icon(Icons.add),
            ),
          ),
        ),
      ),
    );
  }
}

/// The Agenda's "+" opened: the capture [ways] fanned out, Text nearest
/// the button.
class CaptureFanRoute extends FanRoute<CaptureWay> {
  CaptureFanRoute({
    required super.anchor,
    required List<CaptureWay> ways,
    required super.barrierLabel,
  }) : super(
         values: ways,
         describe: _describe,
         closeKey: const ValueKey('captureFanClose'),
       );

  static FanEntry _describe(BuildContext context, CaptureWay way) {
    final t = context.t;
    final (icon, label) = switch (way) {
      CaptureWay.text => (Icons.edit_note, t.text),
      CaptureWay.voice => (Icons.mic_none, t.voice),
      CaptureWay.photo => (Icons.photo_camera_outlined, t.photo),
      CaptureWay.gallery => (Icons.photo_library_outlined, t.gallery),
      CaptureWay.paste => (Icons.content_paste, t.paste),
      CaptureWay.file => (Icons.attach_file, t.file),
    };
    return (
      key: ValueKey('captureFan-${way.name}'),
      icon: Icon(icon),
      label: label,
    );
  }
}

/// The fan for the "+" button laid out at [plus], with the ways this
/// Device has; Text always. Null while [plus] is not laid out.
CaptureFanRoute? captureFanFor(BuildContext context, GlobalKey plus) {
  final anchor = fanAnchor(context, plus.currentContext);
  if (anchor == null) return null;
  final photos = PhotoScope.of(context);
  return CaptureFanRoute(
    anchor: anchor,
    ways: [
      CaptureWay.text,
      if (VoiceScope.of(context).available) CaptureWay.voice,
      if (photos.available) CaptureWay.photo,
      if (photos.pickable) CaptureWay.gallery,
      if (photos.pasteable) CaptureWay.paste,
      if (FileScope.of(context).pickable) CaptureWay.file,
    ],
    barrierLabel: MaterialLocalizations.of(context).modalBarrierDismissLabel,
  );
}
