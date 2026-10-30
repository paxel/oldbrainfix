import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'l10n.dart';

/// What a screen reader says for [mark]: the state it shows.
String textMarkLabel(AppLocalizations t, TextMark mark) => switch (mark) {
  TextMark.open => t.textMarkOpen,
  TextMark.checked => t.textMarkChecked,
  TextMark.failed => t.textMarkFailed,
};

/// Opens the menu of a status mark showing [current] where the finger is
/// ([at], global): Check, Fail and Open, [current] left out (ADR 0031).
/// Gives the state picked, or null when the menu was closed.
Future<TextMark?> pickTextMark(
  BuildContext context,
  Offset at,
  TextMark current,
) {
  final t = context.t;
  final overlay = Overlay.of(context).context.findRenderObject() as RenderBox?;
  if (overlay == null) return Future.value();
  return showMenu<TextMark>(
    context: context,
    position: RelativeRect.fromRect(
      overlay.globalToLocal(at) & Size.zero,
      Offset.zero & overlay.size,
    ),
    items: [
      for (final to in TextMark.values)
        if (to != current)
          PopupMenuItem(
            key: ValueKey('mark-${to.name}'),
            value: to,
            child: Row(
              children: [
                Text(to.glyph),
                const SizedBox(width: 12),
                Flexible(
                  child: Text(switch (to) {
                    TextMark.checked => t.textMarkCheck,
                    TextMark.failed => t.textMarkFail,
                    TextMark.open => t.textMarkReopen,
                  }),
                ),
              ],
            ),
          ),
    ],
  );
}

/// The part [start] to [end] of [text] (all of it while they are null),
/// its status marks at the start of a line read by a screen reader as
/// the state they show. With [onMark] each of them is a button that
/// opens Check, Fail and Open at the finger and gives the line and the
/// state picked; without it they are plain characters. A mark in the
/// middle of a line is never one (ADR 0031).
class MarkedText extends StatelessWidget {
  final String text;
  final int? start;
  final int? end;
  final TextStyle? style;
  final int? maxLines;
  final TextOverflow? overflow;
  final void Function(int line, TextMark to)? onMark;

  const MarkedText(
    this.text, {
    super.key,
    this.start,
    this.end,
    this.style,
    this.maxLines,
    this.overflow,
    this.onMark,
  });

  @override
  Widget build(BuildContext context) {
    final from = start ?? 0;
    final to = end ?? text.length;
    final marks = [
      for (final mark in TextMarks.find(text))
        if (mark.start >= from && mark.end <= to) mark,
    ];
    if (marks.isEmpty) {
      return Text(
        text.substring(from, to),
        style: style,
        maxLines: maxLines,
        overflow: overflow,
      );
    }
    final t = context.t;
    final onMark = this.onMark;
    var at = from;
    final spans = <InlineSpan>[];
    for (final mark in marks) {
      if (mark.start > at) {
        spans.add(TextSpan(text: text.substring(at, mark.start)));
      }
      final glyph = text.substring(mark.start, mark.end);
      final label = textMarkLabel(t, mark.mark);
      spans.add(
        onMark == null
            ? TextSpan(text: glyph, semanticsLabel: label)
            : WidgetSpan(
                alignment: PlaceholderAlignment.baseline,
                baseline: TextBaseline.alphabetic,
                child: _MarkButton(
                  key: ValueKey('mark-${mark.line}'),
                  glyph: glyph,
                  label: label,
                  style: style,
                  onTap: (at) async {
                    final picked = await pickTextMark(context, at, mark.mark);
                    if (!context.mounted || picked == null) return;
                    onMark(mark.line, picked);
                  },
                ),
              ),
      );
      at = mark.end;
    }
    if (to > at) spans.add(TextSpan(text: text.substring(at, to)));
    return Text.rich(
      TextSpan(children: spans),
      style: style,
      maxLines: maxLines,
      overflow: overflow,
    );
  }
}

/// A status mark that can be tapped, read as its state with a hint.
class _MarkButton extends StatelessWidget {
  final String glyph;
  final String label;
  final TextStyle? style;
  final void Function(Offset at) onTap;

  const _MarkButton({
    super.key,
    required this.glyph,
    required this.label,
    required this.style,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) => Semantics(
    // Its own node, not merged into the card around it.
    container: true,
    button: true,
    label: label,
    onTapHint: context.t.textMarkChange,
    onTap: () {
      // A screen reader has no finger: the menu opens at the mark.
      final box = context.findRenderObject() as RenderBox?;
      if (box == null) return;
      onTap(box.localToGlobal(box.size.center(Offset.zero)));
    },
    excludeSemantics: true,
    child: GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTapUp: (details) => onTap(details.globalPosition),
      child: Text(glyph, style: style),
    ),
  );
}
