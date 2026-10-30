import 'dart:async';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:url_launcher/url_launcher.dart';

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

/// Opens the address of a link in another app.
typedef LinkOpen = Future<void> Function(Uri address);

/// How a tapped link in a formatted text below is opened (ADR 0033);
/// without one, the system opens it in the app it belongs to. Only a
/// web or e-mail address ever gets here ([Markdown.mayOpen]).
class LinkOpenScope extends InheritedWidget {
  final LinkOpen onOpen;

  const LinkOpenScope({super.key, required this.onOpen, required super.child});

  static LinkOpen of(BuildContext context) =>
      context.getInheritedWidgetOfExactType<LinkOpenScope>()?.onOpen ?? _launch;

  static Future<void> _launch(Uri address) async {
    await launchUrl(address, mode: LaunchMode.externalApplication);
  }

  @override
  bool updateShouldNotify(LinkOpenScope oldWidget) =>
      onOpen != oldWidget.onOpen;
}

/// How a tag is drawn: in the accent colour.
TextStyle tagStyle(BuildContext context, [TextStyle? style]) =>
    (style ?? const TextStyle()).copyWith(
      color: Theme.of(context).colorScheme.primary,
    );

/// A text field's controller that draws each tag of its text highlighted
/// as it is typed (ADR 0032); a tap in the field still puts the cursor.
///
/// A [markdown] text is drawn formatted as it is typed (ADR 0033): its
/// headings larger, bold, italic, struck through, code and links as they
/// show, a bullet for an item's mark and a bar for a quote's. The marks
/// themselves are dimmed, or with [hideMarks] drawn as nothing, for a
/// field that shows the source of the cursor's line elsewhere. Every
/// character of the text stays where it is, so the cursor counts as in
/// the source.
class TagTextController extends TextEditingController {
  final bool hideMarks;

  TagTextController({super.text, this.hideMarks = false});

  bool _markdown = false;

  /// Whether the text is formatted; it is drawn anew when this changes.
  bool get markdown => _markdown;
  set markdown(bool value) {
    if (value == _markdown) return;
    _markdown = value;
    notifyListeners();
  }

  @override
  TextSpan buildTextSpan({
    required BuildContext context,
    TextStyle? style,
    required bool withComposing,
  }) {
    final composing = withComposing && value.isComposingRangeValid
        ? value.composing
        : TextRange.empty;
    if (_markdown) return _formatted(context, style, composing);
    final tags = TextTags.find(text);
    if (tags.isEmpty) {
      return super.buildTextSpan(
        context: context,
        style: style,
        withComposing: withComposing,
      );
    }
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

  // What a character of a formatted text is, as bits.
  static const _mark = 1;
  static const _bold = 2;
  static const _italic = 4;
  static const _strike = 8;
  static const _code = 16;
  static const _link = 32;
  static const _tag = 64;
  static const _heading1 = 128;
  static const _heading2 = 256;
  static const _quote = 512;
  static const _sign = 1024;
  static const _composing = 2048;

  /// The text formatted, character for character: each one gets the bits
  /// of what it is, and neighbours with the same bits share a span.
  TextSpan _formatted(
    BuildContext context,
    TextStyle? style,
    TextRange composing,
  ) {
    final text = this.text;
    final bits = List<int>.filled(text.length, 0);
    final shown = text.codeUnits.toList();
    void set(int from, int to, int bit) {
      for (var i = from; i < to && i < bits.length; i++) {
        bits[i] |= bit;
      }
    }

    void walk(MarkdownSpan span) {
      switch (span.kind) {
        case MarkdownSpanKind.text:
          // An escape's backslash.
          set(span.start, span.contentStart, _mark);
        case MarkdownSpanKind.code:
          set(span.contentStart, span.contentEnd, _code);
          set(span.start, span.contentStart, _mark);
          set(span.contentEnd, span.end, _mark);
        case MarkdownSpanKind.bold ||
            MarkdownSpanKind.italic ||
            MarkdownSpanKind.strike ||
            MarkdownSpanKind.link:
          set(span.contentStart, span.contentEnd, switch (span.kind) {
            MarkdownSpanKind.bold => _bold,
            MarkdownSpanKind.italic => _italic,
            MarkdownSpanKind.strike => _strike,
            _ => _link,
          });
          set(span.start, span.contentStart, _mark);
          set(span.contentEnd, span.end, _mark);
          span.children.forEach(walk);
      }
    }

    for (final block in Markdown.parse(text)) {
      set(block.start, block.end, switch (block.kind) {
        MarkdownBlockKind.heading1 => _heading1,
        MarkdownBlockKind.heading2 => _heading2,
        MarkdownBlockKind.quote => _quote,
        _ => 0,
      });
      var sign = block.start;
      while (sign < block.contentStart &&
          (text[sign] == ' ' || text[sign] == '\t')) {
        sign++;
      }
      switch (block.kind) {
        case MarkdownBlockKind.heading1 || MarkdownBlockKind.heading2:
          set(block.start, block.contentStart, _mark);
        case MarkdownBlockKind.bullet:
          shown[sign] = (block.level == 0 ? '•' : '◦').codeUnitAt(0);
          set(sign, sign + 1, _sign);
        case MarkdownBlockKind.quote:
          shown[sign] = '▎'.codeUnitAt(0);
          set(sign, sign + 1, _sign);
        case MarkdownBlockKind.numbered:
          var dot = sign;
          while (dot < block.contentStart && text[dot] != '.') {
            dot++;
          }
          // The number it counts to, where it fits in the digits written.
          final number = '${block.number}';
          if (number.length == dot - sign) {
            shown.setRange(sign, dot, number.codeUnits);
          }
          set(sign, dot + 1, _sign);
        case MarkdownBlockKind.paragraph:
          break;
      }
      block.spans.forEach(walk);
    }
    for (final tag in Markdown.tags(text)) {
      set(tag.start, tag.end, _tag);
    }
    if (composing.isValid) set(composing.start, composing.end, _composing);
    final drawn = String.fromCharCodes(shown);
    final styles = <int, TextStyle>{};
    final children = <TextSpan>[];
    var from = 0;
    for (var i = 1; i <= text.length; i++) {
      if (i < text.length && bits[i] == bits[from]) continue;
      final b = bits[from];
      children.add(
        TextSpan(
          text: drawn.substring(from, i),
          style: styles.putIfAbsent(b, () => _styleOf(context, style, b)),
        ),
      );
      from = i;
    }
    return TextSpan(style: style, children: children);
  }

  TextStyle _styleOf(BuildContext context, TextStyle? base, int bits) {
    final scheme = Theme.of(context).colorScheme;
    final size = base?.fontSize ?? 16;
    bool has(int bit) => bits & bit != 0;
    final lines = [
      if (has(_strike)) TextDecoration.lineThrough,
      if (has(_link) || has(_composing)) TextDecoration.underline,
    ];
    final style = TextStyle(
      fontSize: has(_heading1)
          ? size * 1.5
          : has(_heading2)
          ? size * 1.25
          : null,
      fontWeight: has(_bold) || has(_heading1) || has(_heading2)
          ? FontWeight.bold
          : null,
      fontStyle: has(_italic) ? FontStyle.italic : null,
      fontFamily: has(_code) ? 'monospace' : null,
      backgroundColor: has(_code) ? scheme.surfaceContainerHighest : null,
      color: has(_link) || has(_tag) || has(_sign)
          ? scheme.primary
          : has(_quote)
          ? scheme.onSurfaceVariant
          : null,
      decoration: lines.isEmpty ? null : TextDecoration.combine(lines),
    );
    if (!has(_mark)) return style;
    return hideMarks
        ? style.copyWith(
            fontSize: 0.01,
            color: Colors.transparent,
            backgroundColor: Colors.transparent,
          )
        : style.copyWith(color: scheme.outline);
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
///
/// A text in [TextFormats.markdown] is drawn formatted (ADR 0033), each
/// of its lines that [start] to [end] touches: headings larger, items
/// after a bullet or their number, quotes beside a bar, bold, italic,
/// struck through, code in a fixed width on grey and links. A link to a
/// web or e-mail address opens with a tap ([LinkOpenScope]); any other
/// stays text. Status marks and tags work in it as in a plain text,
/// except a tag in a link's label, which is part of the link.
class MarkedText extends StatefulWidget {
  final String text;
  final String format;
  final int? start;
  final int? end;
  final TextStyle? style;
  final int? maxLines;
  final TextOverflow? overflow;
  final void Function(int line, TextMark to)? onMark;

  const MarkedText(
    this.text, {
    super.key,
    this.format = TextFormats.plain,
    this.start,
    this.end,
    this.style,
    this.maxLines,
    this.overflow,
    this.onMark,
  });

  @override
  State<MarkedText> createState() => _MarkedTextState();
}

class _MarkedTextState extends State<MarkedText> {
  /// The taps of the links drawn last.
  final _links = <TapGestureRecognizer>[];

  // What is drawn anew follows one of these: the taps of the links
  // drawn before go with them.
  @override
  void didUpdateWidget(MarkedText oldWidget) {
    super.didUpdateWidget(oldWidget);
    _dropLinks();
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _dropLinks();
  }

  @override
  void dispose() {
    _dropLinks();
    super.dispose();
  }

  void _dropLinks() {
    for (final link in _links) {
      link.dispose();
    }
    _links.clear();
  }

  @override
  Widget build(BuildContext context) {
    if (widget.format == TextFormats.markdown) return _formatted(context);
    final text = widget.text;
    final from = widget.start ?? 0;
    final to = widget.end ?? text.length;
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
        style: widget.style,
        maxLines: widget.maxLines,
        overflow: widget.overflow,
      );
    }
    final t = context.t;
    final onSearch = TagSearchScope.of(context);
    final highlighted = tagStyle(
      context,
      widget.style ?? DefaultTextStyle.of(context).style,
    );
    // Marks and tags never overlap: a mark is no tag character.
    final parts = <({int start, int end, InlineSpan span})>[
      for (final mark in marks)
        (
          start: mark.start,
          end: mark.end,
          span: _markSpan(context, t, mark, widget.style),
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
      style: widget.style,
      maxLines: widget.maxLines,
      overflow: widget.overflow,
    );
  }

  /// Each line of a formatted text in [start] to [end], one under the
  /// other.
  Widget _formatted(BuildContext context) {
    final text = widget.text;
    final from = widget.start ?? 0;
    final to = widget.end ?? text.length;
    final marks = TextMarks.find(text);
    final tags = Markdown.tags(text);
    final base = DefaultTextStyle.of(context).style.merge(widget.style);
    final lines = [
      for (final block in Markdown.parse(text))
        if (block.end >= from && block.start <= to)
          _line(context, block, base, marks, tags),
    ];
    if (lines.length == 1) return lines.single;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: lines,
    );
  }

  Widget _line(
    BuildContext context,
    MarkdownBlock block,
    TextStyle base,
    List<MarkedLine> marks,
    List<FoundTag> tags,
  ) {
    final scheme = Theme.of(context).colorScheme;
    // A compact card's line keeps the size of the text around it.
    final grows = widget.maxLines == null;
    final size = base.fontSize ?? 14;
    final style = switch (block.kind) {
      MarkdownBlockKind.heading1 => base.copyWith(
        fontWeight: FontWeight.bold,
        fontSize: grows ? size * 1.5 : null,
      ),
      MarkdownBlockKind.heading2 => base.copyWith(
        fontWeight: FontWeight.bold,
        fontSize: grows ? size * 1.25 : null,
      ),
      MarkdownBlockKind.quote => base.copyWith(color: scheme.onSurfaceVariant),
      _ => base,
    };
    final spans = <InlineSpan>[];
    for (final span in block.spans) {
      _inline(context, span, style, null, marks, tags, spans);
    }
    final words = Text.rich(
      TextSpan(children: spans),
      style: style,
      maxLines: widget.maxLines,
      overflow: widget.overflow,
    );
    return switch (block.kind) {
      MarkdownBlockKind.paragraph => words,
      MarkdownBlockKind.heading1 ||
      MarkdownBlockKind.heading2 => Semantics(header: true, child: words),
      MarkdownBlockKind.bullet => _item(
        block.level == 0 ? '•' : '◦',
        block.level,
        style,
        words,
      ),
      MarkdownBlockKind.numbered => _item(
        '${block.number}.',
        block.level,
        style,
        words,
      ),
      MarkdownBlockKind.quote => DecoratedBox(
        decoration: BoxDecoration(
          border: BorderDirectional(
            start: BorderSide(color: scheme.primary, width: 3),
          ),
        ),
        child: Padding(
          padding: const EdgeInsetsDirectional.only(start: 8),
          child: words,
        ),
      ),
    };
  }

  /// An item of a list: its bullet or number, the words beside it.
  Widget _item(String sign, int level, TextStyle style, Widget words) =>
      Padding(
        padding: EdgeInsetsDirectional.only(start: level * 20.0),
        child: Row(
          crossAxisAlignment: CrossAxisAlignment.baseline,
          textBaseline: TextBaseline.alphabetic,
          children: [
            ConstrainedBox(
              constraints: const BoxConstraints(minWidth: 20),
              child: Text(sign, style: style),
            ),
            const SizedBox(width: 4),
            Flexible(child: words),
          ],
        ),
      );

  /// The spans [span] shows, in [style], a part of the link that [link]
  /// opens when there is one.
  void _inline(
    BuildContext context,
    MarkdownSpan span,
    TextStyle style,
    TapGestureRecognizer? link,
    List<MarkedLine> marks,
    List<FoundTag> tags,
    List<InlineSpan> out,
  ) {
    final scheme = Theme.of(context).colorScheme;
    void inner(TextStyle style, [TapGestureRecognizer? to]) {
      for (final child in span.children) {
        _inline(context, child, style, to ?? link, marks, tags, out);
      }
    }

    switch (span.kind) {
      case MarkdownSpanKind.text when span.contentStart > span.start:
        // An escaped character.
        out.add(TextSpan(text: span.text, style: style, recognizer: link));
      case MarkdownSpanKind.text:
        _literal(context, span.start, span.end, style, link, marks, tags, out);
      case MarkdownSpanKind.code:
        out.add(
          TextSpan(
            text: span.text,
            style: style.copyWith(
              fontFamily: 'monospace',
              backgroundColor: scheme.surfaceContainerHighest,
            ),
            recognizer: link,
          ),
        );
      case MarkdownSpanKind.bold:
        inner(style.copyWith(fontWeight: FontWeight.bold));
      case MarkdownSpanKind.italic:
        inner(style.copyWith(fontStyle: FontStyle.italic));
      case MarkdownSpanKind.strike:
        inner(
          style.copyWith(
            decoration: TextDecoration.combine([
              ?style.decoration,
              TextDecoration.lineThrough,
            ]),
          ),
        );
      case MarkdownSpanKind.link:
        // Another person may have written it: only the web and e-mail.
        if (!Markdown.mayOpen(span.url)) {
          inner(style);
          return;
        }
        final address = Uri.tryParse(span.url);
        if (address == null) {
          inner(style);
          return;
        }
        final open = LinkOpenScope.of(context);
        final tap = TapGestureRecognizer()
          ..onTap = () => unawaited(_open(open, address));
        _links.add(tap);
        inner(
          style.copyWith(
            color: scheme.primary,
            decoration: TextDecoration.combine([
              ?style.decoration,
              TextDecoration.underline,
            ]),
          ),
          tap,
        );
    }
  }

  static Future<void> _open(LinkOpen open, Uri address) async {
    try {
      await open(address);
    } on Object {
      // No app takes it: the tap does nothing.
    }
  }

  /// The characters [start] to [end], written as they show, with the
  /// status marks and tags among them.
  void _literal(
    BuildContext context,
    int start,
    int end,
    TextStyle style,
    TapGestureRecognizer? link,
    List<MarkedLine> marks,
    List<FoundTag> tags,
    List<InlineSpan> out,
  ) {
    final text = widget.text;
    final t = context.t;
    final onSearch = TagSearchScope.of(context);
    final parts = <({int start, int end, InlineSpan span})>[
      for (final mark in marks)
        if (mark.start >= start && mark.end <= end)
          (
            start: mark.start,
            end: mark.end,
            span: _markSpan(context, t, mark, style),
          ),
      // A tag in a link's label is part of the link.
      if (link == null)
        for (final tag in tags)
          if (tag.start >= start && tag.end <= end)
            (
              start: tag.start,
              end: tag.end,
              span: _tagSpan(
                context,
                t,
                tag,
                tagStyle(context, style),
                onSearch,
              ),
            ),
    ]..sort((a, b) => a.start.compareTo(b.start));
    var at = start;
    for (final part in parts) {
      if (part.start > at) {
        out.add(
          TextSpan(
            text: text.substring(at, part.start),
            style: style,
            recognizer: link,
          ),
        );
      }
      out.add(part.span);
      at = part.end;
    }
    if (end > at) {
      out.add(
        TextSpan(text: text.substring(at, end), style: style, recognizer: link),
      );
    }
  }

  InlineSpan _markSpan(
    BuildContext context,
    AppLocalizations t,
    MarkedLine mark,
    TextStyle? style,
  ) {
    final onMark = widget.onMark;
    final glyph = widget.text.substring(mark.start, mark.end);
    final label = textMarkLabel(t, mark.mark);
    if (onMark == null) {
      return TextSpan(text: glyph, semanticsLabel: label, style: style);
    }
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
    final written = widget.text.substring(tag.start, tag.end);
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
