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

/// Opens the search with [query], a tag as written; from [context], the
/// tag that was tapped.
typedef TagSearch = void Function(BuildContext context, String query);

/// Where a tap on a tag in a text below goes (ADR 0032): the search with
/// the tag as written. A tag is highlighted but not tapped where none is
/// given, or where [onSearch] is null.
class TagSearchScope extends InheritedWidget {
  final TagSearch? onSearch;

  const TagSearchScope({
    super.key,
    required this.onSearch,
    required super.child,
  });

  static TagSearch? of(BuildContext context) =>
      context.getInheritedWidgetOfExactType<TagSearchScope>()?.onSearch;

  @override
  bool updateShouldNotify(TagSearchScope oldWidget) =>
      onSearch != oldWidget.onSearch;
}

/// How a tag is drawn: in the accent colour.
TextStyle tagStyle(BuildContext context, [TextStyle? style]) =>
    (style ?? const TextStyle()).copyWith(
      color: Theme.of(context).colorScheme.primary,
    );

/// A text field's controller that draws each tag of its text highlighted
/// as it is typed (ADR 0032); a tap in the field still puts the cursor.
class TagTextController extends TextEditingController {
  TagTextController({super.text});

  @override
  TextSpan buildTextSpan({
    required BuildContext context,
    TextStyle? style,
    required bool withComposing,
  }) {
    final tags = TextTags.find(text);
    if (tags.isEmpty) {
      return super.buildTextSpan(
        context: context,
        style: style,
        withComposing: withComposing,
      );
    }
    final composing = withComposing && value.isComposingRangeValid
        ? value.composing
        : TextRange.empty;
    final cuts = <int>{
      0,
      text.length,
      for (final tag in tags) ...[tag.start, tag.end],
      if (composing.isValid) ...[composing.start, composing.end],
    }.toList()..sort();
    final highlighted = tagStyle(context);
    const underlined = TextStyle(decoration: TextDecoration.underline);
    return TextSpan(
      style: style,
      children: [
        for (var i = 0; i + 1 < cuts.length; i++)
          if (cuts[i] < cuts[i + 1])
            TextSpan(
              text: text.substring(cuts[i], cuts[i + 1]),
              style: switch ((
                tags.any((t) => t.start <= cuts[i] && cuts[i + 1] <= t.end),
                composing.isValid &&
                    composing.start <= cuts[i] &&
                    cuts[i + 1] <= composing.end,
              )) {
                (true, true) => highlighted.merge(underlined),
                (true, false) => highlighted,
                (false, true) => underlined,
                (false, false) => null,
              },
            ),
      ],
    );
  }
}

/// The part [start] to [end] of [text] (all of it while they are null),
/// its status marks at the start of a line read by a screen reader as
/// the state they show. With [onMark] each of them is a button that
/// opens Check, Fail and Open at the finger and gives the line and the
/// state picked; without it they are plain characters. A mark in the
/// middle of a line is never one (ADR 0031). Each tag is highlighted
/// and, under a [TagSearchScope], a link that opens the search for it
/// (ADR 0032).
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
    final tags = [
      for (final tag in TextTags.find(text))
        if (tag.start >= from && tag.end <= to) tag,
    ];
    if (marks.isEmpty && tags.isEmpty) {
      return Text(
        text.substring(from, to),
        style: style,
        maxLines: maxLines,
        overflow: overflow,
      );
    }
    final t = context.t;
    final onMark = this.onMark;
    final onSearch = TagSearchScope.of(context);
    final highlighted = tagStyle(
      context,
      style ?? DefaultTextStyle.of(context).style,
    );
    // Marks and tags never overlap: a mark is no tag character.
    final parts = <({int start, int end, InlineSpan span})>[
      for (final mark in marks)
        (
          start: mark.start,
          end: mark.end,
          span: _markSpan(context, t, mark, onMark),
        ),
      for (final tag in tags)
        (
          start: tag.start,
          end: tag.end,
          span: _tagSpan(context, t, tag, highlighted, onSearch),
        ),
    ]..sort((a, b) => a.start.compareTo(b.start));
    var at = from;
    final spans = <InlineSpan>[];
    for (final part in parts) {
      if (part.start > at) {
        spans.add(TextSpan(text: text.substring(at, part.start)));
      }
      spans.add(part.span);
      at = part.end;
    }
    if (to > at) spans.add(TextSpan(text: text.substring(at, to)));
    return Text.rich(
      TextSpan(children: spans),
      style: style,
      maxLines: maxLines,
      overflow: overflow,
    );
  }

  InlineSpan _markSpan(
    BuildContext context,
    AppLocalizations t,
    MarkedLine mark,
    void Function(int line, TextMark to)? onMark,
  ) {
    final glyph = text.substring(mark.start, mark.end);
    final label = textMarkLabel(t, mark.mark);
    if (onMark == null) return TextSpan(text: glyph, semanticsLabel: label);
    return WidgetSpan(
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
    );
  }

  InlineSpan _tagSpan(
    BuildContext context,
    AppLocalizations t,
    FoundTag tag,
    TextStyle highlighted,
    TagSearch? onSearch,
  ) {
    final written = text.substring(tag.start, tag.end);
    if (onSearch == null) {
      return TextSpan(
        text: written,
        style: highlighted,
        semanticsLabel: written,
      );
    }
    return WidgetSpan(
      alignment: PlaceholderAlignment.baseline,
      baseline: TextBaseline.alphabetic,
      child: _TagLink(
        key: ValueKey('tag-${tag.start}'),
        tag: written,
        label: t.searchForTag(written),
        style: highlighted,
        onTap: () => onSearch(context, written),
      ),
    );
  }
}

/// A tag that can be tapped, read as a link to the search for it.
class _TagLink extends StatelessWidget {
  final String tag;
  final String label;
  final TextStyle style;
  final VoidCallback onTap;

  const _TagLink({
    super.key,
    required this.tag,
    required this.label,
    required this.style,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) => Semantics(
    // Its own node, not merged into the card around it.
    container: true,
    link: true,
    label: label,
    onTap: onTap,
    excludeSemantics: true,
    child: GestureDetector(
      behavior: HitTestBehavior.opaque,
      onTap: onTap,
      child: Text(tag, style: style),
    ),
  );
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
