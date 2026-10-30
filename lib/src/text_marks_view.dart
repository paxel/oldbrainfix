import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:url_launcher/url_launcher.dart';

import '../l10n/app_localizations.dart';
import 'box_ids.dart';
import 'l10n.dart';
import 'profile.dart';
import 'table_screen.dart';

/// What a screen reader says for a box showing [state].
String boxStateLabel(AppLocalizations t, BoxState state) => switch (state) {
  BoxState.open => t.textMarkOpen,
  BoxState.done => t.textMarkChecked,
  BoxState.failed => t.textMarkFailed,
};

/// The local setting of this Device that says whether the Note [noteId]
/// shows after each box who ticked it last and when (ADR 0035).
String showTicksKey(String noteId) => 'showTicks:$noteId';

/// Whether the Note [noteId] shows its ticks on this Device.
bool showsTicks(Store store, String noteId) =>
    store.localSetting(showTicksKey(noteId)) == 'true';

/// Shows the ticks of the Note [noteId] on this Device, or no more.
void setShowsTicks(Store store, String noteId, bool on) =>
    store.setLocalSetting(showTicksKey(noteId), '$on');

/// The boxes of one text, a Note's or a Section's [entity], as they are
/// ticked (ADR 0035): the latest tick of each by its id ([ticks]),
/// whether this Device may tick them ([mayTick]), and whether each box
/// says after it who ticked it last and when ([showTicks]). A tick is an
/// entry beside the text, never an edit of it; [changed] runs after one.
class BoxTicking {
  final Notes notes;
  final String entity;
  final Map<String, BoxTick> ticks;
  final bool mayTick;
  final bool showTicks;
  final VoidCallback? changed;

  BoxTicking({
    required this.notes,
    required this.entity,
    required this.ticks,
    required this.mayTick,
    this.showTicks = false,
    this.changed,
  });

  /// The ticks of [entity] as the log holds them now.
  factory BoxTicking.read(
    Notes notes,
    String entity, {
    required bool mayTick,
    bool showTicks = false,
    VoidCallback? changed,
  }) => BoxTicking(
    notes: notes,
    entity: entity,
    ticks: Boxes(notes.store).ticks(entity),
    mayTick: mayTick,
    showTicks: showTicks,
    changed: changed,
  );

  /// Who wrote the latest tick of each box, read once on first use: null
  /// for the user's own, a person not known or one blocked.
  late final Map<String, Profile?> _by = {
    for (final MapEntry(:key, :value) in ticks.entries)
      key: notes.store.tickedBy(entity, value),
  };

  /// The same boxes with their ticks read again.
  BoxTicking again() => BoxTicking.read(
    notes,
    entity,
    mayTick: mayTick,
    showTicks: showTicks,
    changed: changed,
  );

  /// The state the box [id] shows, written as [starting].
  BoxState stateOf(String id, BoxState starting) =>
      ticks[id]?.state ?? starting;

  /// Ticks the box [id] to [to]; whether it was written. A sync may have
  /// taken the box or the role meanwhile: then nothing is.
  bool tick(String id, BoxState to) {
    try {
      Boxes(notes.store).tick(entity, id, to);
    } on ArgumentError {
      return false;
    } on StateError {
      return false;
    }
    changed?.call();
    return true;
  }

  /// Every tick of the box [id], the oldest first, with who wrote it.
  List<({BoxTick tick, Profile? by})> history(String id) => [
    for (final tick in Boxes(notes.store).history(entity, id))
      (tick: tick, by: notes.store.tickedBy(entity, tick)),
  ];

  /// What a box with Show ticks on says after it: who ticked it last and
  /// when; null while nobody did.
  String? labelOf(BuildContext context, String id) {
    final tick = ticks[id];
    if (tick == null) return null;
    return tickText(context, tick, _by[id], now: notes.store.clock());
  }
}

/// [text], stored with the ids of its boxes, with each box's glyph the
/// one of the state [ticking] ticks it to; every offset stays.
String tickedGlyphs(String text, BoxTicking ticking) {
  final runs = tagRuns(text);
  if (runs.isEmpty) return text;
  final units = text.codeUnits.toList();
  for (final run in runs) {
    if (!run.box) continue;
    final starting = BoxState.values.firstWhere(
      (s) => s.glyph.codeUnitAt(0) == units[run.base],
    );
    units[run.base] = ticking
        .stateOf(runId(text, run), starting)
        .glyph
        .codeUnitAt(0);
  }
  return String.fromCharCodes(units);
}

/// Who wrote [tick] and when, as "Ann, Tue 14:02": "You" for the user's
/// own, the time alone when the person is not known. The weekday stands
/// for the time within the last six days of [now], the date before.
String tickText(
  BuildContext context,
  BoxTick tick,
  Profile? by, {
  required DateTime now,
  bool full = false,
}) {
  final t = context.t;
  final locale = Localizations.localeOf(context).toString();
  final at = tick.at.toLocal();
  final recent =
      !full && now.toLocal().difference(at) < const Duration(days: 6);
  final when =
      (full
              ? DateFormat.yMMMEd(locale)
              : recent
              ? DateFormat.E(locale)
              : DateFormat.yMMMd(locale))
          .add_Hm()
          .format(at);
  final name = tick.own
      ? t.boxTickYou
      : by == null
      ? null
      : profileDisplayName(t, by.name);
  return name == null ? when : t.boxTickBy(name, when);
}

/// What the menu of a box offers.
sealed class _BoxPick {}

final class _ToState extends _BoxPick {
  final BoxState state;
  _ToState(this.state);
}

final class _History extends _BoxPick {}

/// Opens the menu of the box [id] showing [state] where the finger is
/// ([at], global): Check, Fail and Open, [state] left out, while this
/// Device may tick it, and its History once it has been ticked. Ticks it
/// to the state picked, or shows the History.
Future<void> openBoxMenu(
  BuildContext context,
  Offset at, {
  required String id,
  required BoxState state,
  required BoxTicking ticking,
}) async {
  final t = context.t;
  final overlay = Overlay.of(context).context.findRenderObject() as RenderBox?;
  final ticked = ticking.ticks.containsKey(id);
  if (overlay == null || (!ticking.mayTick && !ticked)) return;
  final picked = await showMenu<_BoxPick>(
    context: context,
    position: RelativeRect.fromRect(
      overlay.globalToLocal(at) & Size.zero,
      Offset.zero & overlay.size,
    ),
    items: [
      if (ticking.mayTick)
        for (final to in BoxState.values)
          if (to != state)
            PopupMenuItem(
              key: ValueKey('box-${to.name}'),
              value: _ToState(to),
              child: Row(
                children: [
                  Text(to.glyph),
                  const SizedBox(width: 12),
                  Flexible(
                    child: Text(switch (to) {
                      BoxState.done => t.textMarkCheck,
                      BoxState.failed => t.textMarkFail,
                      BoxState.open => t.textMarkReopen,
                    }),
                  ),
                ],
              ),
            ),
      if (ticked)
        PopupMenuItem(
          key: const ValueKey('box-history'),
          value: _History(),
          child: Row(
            children: [
              const Icon(Icons.history),
              const SizedBox(width: 12),
              Flexible(child: Text(t.historyTitle)),
            ],
          ),
        ),
    ],
  );
  if (!context.mounted) return;
  switch (picked) {
    case _ToState(:final state):
      ticking.tick(id, state);
    case _History():
      await showBoxHistory(context, ticking, id);
    case null:
      break;
  }
}

/// Shows every change of the state of the box [id]: the state, who
/// changed it and when, the oldest first.
Future<void> showBoxHistory(
  BuildContext context,
  BoxTicking ticking,
  String id,
) {
  final t = context.t;
  final now = ticking.notes.store.clock();
  final changes = ticking.history(id);
  return showDialog<void>(
    context: context,
    builder: (context) => AlertDialog(
      key: const ValueKey('box-history-dialog'),
      title: Text(t.historyTitle),
      content: SizedBox(
        width: double.maxFinite,
        child: ListView(
          shrinkWrap: true,
          children: [
            Text(t.boxHistoryHint),
            const SizedBox(height: 8),
            for (final (:tick, :by) in changes)
              ListTile(
                contentPadding: EdgeInsets.zero,
                leading: Text(
                  tick.state.glyph,
                  style: Theme.of(context).textTheme.titleLarge,
                ),
                title: Text(boxStateLabel(t, tick.state)),
                subtitle: Text(
                  tickText(context, tick, by, now: now, full: true),
                ),
              ),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(t.close),
        ),
      ],
    ),
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
///
/// The text holds its boxes with their ids (ADR 0035): each box is drawn
/// in the state its [boxes] tick it to, with Show ticks who ticked it
/// last and when after it, and its id is never seen; the cursor and a
/// selection never end inside a box ([snapSelection]).
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

  BoxTicking? _boxes;

  /// The ticks of the text's boxes; it is drawn anew when they change.
  BoxTicking? get boxes => _boxes;
  set boxes(BoxTicking? value) {
    if (identical(value, _boxes)) return;
    _boxes = value;
    notifyListeners();
  }

  @override
  set value(TextEditingValue newValue) {
    final selection = snapSelection(
      newValue.text,
      newValue.selection,
      before: super.value.selection,
    );
    super.value = identical(selection, newValue.selection)
        ? newValue
        : newValue.copyWith(selection: selection);
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
    final runs = tagRuns(text);
    if (_markdown || runs.isNotEmpty) {
      return _formatted(context, style, composing, runs);
    }
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
  static const _id = 4096;

  /// The text formatted, character for character: each one gets the bits
  /// of what it is, and neighbours with the same bits share a span. A
  /// plain text is drawn so too when it has boxes ([runs]): each glyph in
  /// its state, its id as nothing.
  TextSpan _formatted(
    BuildContext context,
    TextStyle? style,
    TextRange composing,
    List<TagRun> runs,
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

    for (final block in _markdown ? Markdown.parse(text) : <MarkdownBlock>[]) {
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
        case MarkdownBlockKind.table:
          // A table's pipes, its delimiter row and its comment line are
          // marks; its cells' words are not.
          final row = switch ((block.table, block.row)) {
            (final table?, final row?) => table.rows[row],
            _ => null,
          };
          if (row == null) {
            set(block.start, block.end, _mark);
          } else {
            var from = row.start;
            for (final cell in [...row.cells, null]) {
              final to = cell?.start ?? row.end;
              for (var i = from; i < to; i++) {
                if (text[i] == '|') set(i, i + 1, _mark);
              }
              if (cell == null) break;
              from = math.max(from, cell.end);
            }
          }
        case MarkdownBlockKind.paragraph:
          break;
      }
      block.spans.forEach(walk);
    }
    final tags = _markdown ? Markdown.tags(text) : TextTags.find(text);
    for (final tag in tags) {
      set(tag.start, tag.end, _tag);
    }
    // Each box in the state it is ticked to, its id never seen; with
    // Show ticks, who ticked it last after it.
    final boxes = _boxes;
    final labels = <int, String>{};
    for (final run in runs) {
      if (!run.box) continue;
      set(run.base + 1, run.end, _id);
      if (boxes == null) continue;
      final id = runId(text, run);
      final starting = BoxState.values.firstWhere(
        (s) => s.glyph.codeUnitAt(0) == text.codeUnitAt(run.base),
      );
      shown[run.base] = boxes.stateOf(id, starting).glyph.codeUnitAt(0);
      if (boxes.showTicks) {
        if (boxes.labelOf(context, id) case final label?) {
          labels[run.base] = label;
        }
      }
    }
    if (composing.isValid) set(composing.start, composing.end, _composing);
    final drawn = String.fromCharCodes(shown);
    final styles = <int, TextStyle>{};
    TextStyle styleOf(int b) =>
        styles.putIfAbsent(b, () => _styleOf(context, style, b));
    final children = <InlineSpan>[];
    var from = 0;
    for (var i = 1; i <= text.length; i++) {
      // A box with its label is one character of its own.
      if (i < text.length &&
          bits[i] == bits[from] &&
          !labels.containsKey(i) &&
          !labels.containsKey(from)) {
        continue;
      }
      final b = bits[from];
      if (labels[from] case final label?) {
        children.add(
          WidgetSpan(
            alignment: PlaceholderAlignment.baseline,
            baseline: TextBaseline.alphabetic,
            style: styleOf(b),
            child: _BoxWithTick(
              glyph: drawn.substring(from, i),
              label: label,
              style: (style ?? const TextStyle()).merge(styleOf(b)),
            ),
          ),
        );
      } else {
        children.add(
          TextSpan(text: drawn.substring(from, i), style: styleOf(b)),
        );
      }
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
    if (has(_id)) {
      return style.copyWith(
        fontSize: 0.01,
        color: Colors.transparent,
        backgroundColor: Colors.transparent,
      );
    }
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

/// A box where a text is drawn: its glyph from [start] to [end] of that
/// text, its [id], and the [state] it shows.
typedef DrawnBox = ({int start, int end, String id, BoxState state});

/// The boxes of [text], a text as stored with the ids of its boxes, where
/// it is drawn as it is: each from its glyph to the end of its id, in the
/// state [ticking] ticks it to, else the one it was written with.
List<DrawnBox> storedBoxes(String text, BoxTicking? ticking) => [
  for (final b in TextBoxes.find(text))
    (
      start: b.start,
      end: b.end,
      id: b.id,
      state: ticking?.stateOf(b.id, b.state) ?? b.state,
    ),
];

/// Where the offset [shown] of a shown text lies in the text stored with
/// [boxes]: each box before it is longer there by its id.
int storedOffset(List<ShownBox> boxes, int shown) {
  var at = shown;
  for (final box in boxes) {
    if (box.end > shown) break;
    at += (box.sourceEnd - box.sourceStart) - (box.end - box.start);
  }
  return at;
}

/// The part [start] to [end] of [text] (all of it while they are null),
/// its boxes ([boxes], in [text] as it shows, ADR 0035) read by a screen
/// reader as the state they show. With [ticking] each box is a button
/// that opens Check, Fail and Open at the finger, while this Device may
/// tick it, and its History, and with Show ticks it says after it who
/// ticked it last and when; without it they are plain characters. Each
/// tag is highlighted and, under a [TagSearchScope], a link that opens
/// the search for it (ADR 0032).
///
/// Without [boxes] the text may be one stored with the ids of its boxes,
/// such as a Sent Item's: it is drawn without them, each box in the
/// glyph it was written with.
///
/// A text in [TextFormats.markdown] is drawn formatted (ADR 0033), each
/// of its lines that [start] to [end] touches: headings larger, items
/// after a bullet or their number, quotes beside a bar, bold, italic,
/// struck through, code in a fixed width on grey and links. A link to a
/// web or e-mail address opens with a tap ([LinkOpenScope]); any other
/// stays text. Boxes and tags work in it as in a plain text, except a
/// tag in a link's label, which is part of the link.
///
/// A table is drawn as a grid, made smaller to fit the width, its header
/// row in bold, its boxes as in a line. A tap on it opens it full screen
/// ([openTable]) in the text as stored ([source]), where [tableEdit]
/// changes it; it is only read there without one. A compact line
/// ([maxLines]) of a table shows the words of its row.
class MarkedText extends StatefulWidget {
  final String text;
  final String format;
  final int? start;
  final int? end;
  final TextStyle? style;
  final int? maxLines;
  final TextOverflow? overflow;
  final List<ShownBox>? boxes;
  final String? source;
  final BoxTicking? ticking;
  final TableEditing? tableEdit;

  const MarkedText(
    this.text, {
    super.key,
    this.format = TextFormats.plain,
    this.start,
    this.end,
    this.style,
    this.maxLines,
    this.overflow,
    this.boxes,
    this.source,
    this.ticking,
    this.tableEdit,
  }) : assert(
         boxes == null || source != null || tableEdit == null,
         'a table is changed in the text as stored, its ids in it',
       );

  @override
  State<MarkedText> createState() => _MarkedTextState();
}

class _MarkedTextState extends State<MarkedText> with _InlineSpans {
  @override
  String get _source => _foundIn.shown;

  @override
  BoxTicking? get _ticking => widget.ticking;

  _FoundIn? _found;

  /// What the text holds, found once for each text and boxes it shows.
  _FoundIn get _foundIn {
    final found = _found;
    if (found != null &&
        found.text == widget.text &&
        identical(found.given, widget.boxes)) {
      return found;
    }
    return _found = _FoundIn(widget.text, widget.boxes);
  }

  @override
  Widget build(BuildContext context) {
    if (widget.format == TextFormats.markdown) return _formatted(context);
    final found = _foundIn;
    final text = found.shown;
    final from = (widget.start ?? 0).clamp(0, text.length);
    final to = (widget.end ?? text.length).clamp(from, text.length);
    final boxes = [
      for (final box in found.boxes)
        if (box.start >= from && box.end <= to) box,
    ];
    final tags = [
      for (final tag in found.tags)
        if (tag.start >= from && tag.end <= to) tag,
    ];
    if (boxes.isEmpty && tags.isEmpty) {
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
    // Boxes and tags never overlap: a glyph is no tag character.
    final parts = <({int start, int end, List<InlineSpan> spans})>[
      for (final box in boxes)
        (
          start: box.start,
          end: box.end,
          spans: _boxSpans(context, t, box, widget.style),
        ),
      for (final tag in tags)
        (
          start: tag.start,
          end: tag.end,
          spans: [_tagSpan(context, t, tag, highlighted, onSearch)],
        ),
    ]..sort((a, b) => a.start.compareTo(b.start));
    var at = from;
    final spans = <InlineSpan>[];
    for (final part in parts) {
      if (part.start > at) {
        spans.add(TextSpan(text: text.substring(at, part.start)));
      }
      spans.addAll(part.spans);
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
  /// other; a table once, as a grid.
  Widget _formatted(BuildContext context) {
    final found = _foundIn;
    final text = found.shown;
    final from = widget.start ?? 0;
    final to = widget.end ?? text.length;
    final boxes = found.boxes;
    final tags = found.markdownTags;
    final base = DefaultTextStyle.of(context).style.merge(widget.style);
    final compact = widget.maxLines != null;
    final lines = <Widget>[];
    for (final block in found.blocks) {
      if (block.end < from || block.start > to) continue;
      final table = block.table;
      if (table == null) {
        lines.add(_line(context, block, base, boxes, tags));
      } else if (compact) {
        // Its comment line reads as nothing: the header row stands in.
        final row = block.row ?? (block.line == table.firstLine ? 0 : null);
        if (row != null) {
          lines.add(_rowLine(context, table.rows[row], base, boxes, tags));
        }
      } else if (block.line == table.firstLine) {
        lines.add(_table(context, table, base, boxes, tags));
      }
    }
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
    List<DrawnBox> boxes,
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
      _inline(context, span, style, null, boxes, tags, spans);
    }
    final words = Text.rich(
      TextSpan(children: spans),
      style: style,
      maxLines: widget.maxLines,
      overflow: widget.overflow,
    );
    return switch (block.kind) {
      MarkdownBlockKind.paragraph || MarkdownBlockKind.table => words,
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

  /// A row of a table on one line: the cells that show something, a
  /// space between them.
  Widget _rowLine(
    BuildContext context,
    MarkdownTableRow row,
    TextStyle base,
    List<DrawnBox> boxes,
    List<FoundTag> tags,
  ) {
    final spans = <InlineSpan>[];
    for (final cell in row.cells) {
      if (cell.spans.isEmpty) continue;
      if (spans.isNotEmpty) spans.add(const TextSpan(text: ' '));
      for (final span in cell.spans) {
        _inline(context, span, base, null, boxes, tags, spans);
      }
    }
    return Text.rich(
      TextSpan(children: spans),
      style: base,
      maxLines: widget.maxLines,
      overflow: widget.overflow,
    );
  }

  /// [table] as a grid, made smaller when it is wider than the room it
  /// has; a tap opens it full screen.
  Widget _table(
    BuildContext context,
    MarkdownTable table,
    TextStyle base,
    List<DrawnBox> boxes,
    List<FoundTag> tags,
  ) {
    final scheme = Theme.of(context).colorScheme;
    final bold = base.copyWith(fontWeight: FontWeight.bold);
    final grid = Table(
      defaultColumnWidth: const IntrinsicColumnWidth(),
      border: TableBorder.all(color: scheme.outline),
      children: [
        for (final (r, row) in table.rows.indexed)
          TableRow(
            decoration: r == 0
                ? BoxDecoration(color: scheme.surfaceContainerHighest)
                : null,
            children: [
              for (final (c, cell) in row.cells.indexed)
                Padding(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 8,
                    vertical: 4,
                  ),
                  child: Text.rich(
                    TextSpan(
                      children: [
                        for (final span in cell.spans)
                          ..._inlineOf(
                            context,
                            span,
                            r == 0 || (c == 0 && table.headingColumn)
                                ? bold
                                : base,
                            boxes,
                            tags,
                          ),
                      ],
                    ),
                    textAlign: cellAlign(table.aligns[c]),
                    softWrap: false,
                  ),
                ),
            ],
          ),
      ],
    );
    final found = _foundIn;
    final edit = widget.tableEdit;
    final ticking = widget.ticking;
    return Semantics(
      container: true,
      onTapHint: context.t.tableOpen,
      child: GestureDetector(
        key: ValueKey('table-${table.firstLine}'),
        behavior: HitTestBehavior.opaque,
        onTap: () => unawaited(
          openTable(
            context,
            text: found.stored(widget.source),
            at: found.storedAt(table.start),
            edit: edit,
            ticking: ticking,
          ),
        ),
        child: Padding(
          padding: const EdgeInsets.symmetric(vertical: 4),
          child: Align(
            alignment: AlignmentDirectional.centerStart,
            child: FittedBox(
              fit: BoxFit.scaleDown,
              alignment: AlignmentDirectional.centerStart,
              child: grid,
            ),
          ),
        ),
      ),
    );
  }

  List<InlineSpan> _inlineOf(
    BuildContext context,
    MarkdownSpan span,
    TextStyle style,
    List<DrawnBox> boxes,
    List<FoundTag> tags,
  ) {
    final out = <InlineSpan>[];
    _inline(context, span, style, null, boxes, tags, out);
    return out;
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
}

/// How a cell of a column that lines up as [align] sets its words.
TextAlign cellAlign(MarkdownAlign align) => switch (align) {
  MarkdownAlign.none || MarkdownAlign.left => TextAlign.start,
  MarkdownAlign.center => TextAlign.center,
  MarkdownAlign.right => TextAlign.end,
};

/// The boxes, tags and Markdown blocks of [text], each read on first
/// use: kept by [MarkedText] while its text and boxes stay the same, so
/// a build reads none of them again. Without [given] boxes the text may
/// hold ids: it is drawn as [shown], without them.
class _FoundIn {
  final String text;
  final List<ShownBox>? given;

  _FoundIn(this.text, this.given);

  late final ShownText _shownText = given == null
      ? TextBoxes.shown(text)
      : (text: text, boxes: given!);

  /// The text as it is drawn.
  late final String shown = _shownText.text;

  late final List<ShownBox> _shownBoxes = _shownText.boxes;

  late final List<DrawnBox> boxes = [
    for (final b in _shownBoxes)
      (start: b.start, end: b.end, id: b.id, state: b.state),
  ];

  /// The text as it is stored: [source] when given, else the text.
  String stored(String? source) => source ?? text;

  /// Where [shown]'s offset [at] lies in the text as stored.
  int storedAt(int at) => storedOffset(_shownBoxes, at);

  late final tags = TextTags.find(shown);
  late final blocks = Markdown.parse(shown);
  late final markdownTags = Markdown.tags(shown);
}

/// The words of [cell] of a table of the formatted [text] as [MarkedText]
/// draws them: bold, italic, struck through, code and links, its boxes
/// ([boxes], in [text]) buttons with [ticking], its tags links to the
/// search. [boxes] and [tags] are those of the whole text.
class MarkedCell extends StatefulWidget {
  final String text;
  final MarkdownTableCell cell;
  final List<DrawnBox> boxes;
  final List<FoundTag> tags;
  final TextStyle? style;
  final TextAlign textAlign;
  final BoxTicking? ticking;

  const MarkedCell(
    this.text,
    this.cell, {
    super.key,
    required this.boxes,
    required this.tags,
    this.style,
    this.textAlign = TextAlign.start,
    this.ticking,
  });

  @override
  State<MarkedCell> createState() => _MarkedCellState();
}

class _MarkedCellState extends State<MarkedCell> with _InlineSpans {
  @override
  String get _source => widget.text;

  @override
  BoxTicking? get _ticking => widget.ticking;

  @override
  Widget build(BuildContext context) {
    final style = DefaultTextStyle.of(context).style.merge(widget.style);
    final spans = <InlineSpan>[];
    for (final span in widget.cell.spans) {
      _inline(context, span, style, null, widget.boxes, widget.tags, spans);
    }
    return Text.rich(
      TextSpan(children: spans),
      style: style,
      textAlign: widget.textAlign,
    );
  }
}

/// The spans of a formatted text's parts, with the taps of its links,
/// its boxes and its tags; the taps go when it is drawn anew.
mixin _InlineSpans<T extends StatefulWidget> on State<T> {
  /// The taps of the links drawn last.
  final _links = <TapGestureRecognizer>[];

  /// The whole text the parts are of, and how its boxes are ticked.
  String get _source;
  BoxTicking? get _ticking;

  // What is drawn anew follows one of these: the taps of the links
  // drawn before go with them.
  @override
  void didUpdateWidget(covariant T oldWidget) {
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

  /// The spans [span] shows, in [style], a part of the link that [link]
  /// opens when there is one.
  void _inline(
    BuildContext context,
    MarkdownSpan span,
    TextStyle style,
    TapGestureRecognizer? link,
    List<DrawnBox> boxes,
    List<FoundTag> tags,
    List<InlineSpan> out,
  ) {
    final scheme = Theme.of(context).colorScheme;
    void inner(TextStyle style, [TapGestureRecognizer? to]) {
      for (final child in span.children) {
        _inline(context, child, style, to ?? link, boxes, tags, out);
      }
    }

    switch (span.kind) {
      case MarkdownSpanKind.text when span.contentStart > span.start:
        // An escaped character.
        out.add(TextSpan(text: span.text, style: style, recognizer: link));
      case MarkdownSpanKind.text:
        _literal(context, span.start, span.end, style, link, boxes, tags, out);
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
  /// boxes and tags among them.
  void _literal(
    BuildContext context,
    int start,
    int end,
    TextStyle style,
    TapGestureRecognizer? link,
    List<DrawnBox> boxes,
    List<FoundTag> tags,
    List<InlineSpan> out,
  ) {
    final text = _source;
    final t = context.t;
    final onSearch = TagSearchScope.of(context);
    final parts = <({int start, int end, List<InlineSpan> spans})>[
      for (final box in boxes)
        if (box.start >= start && box.end <= end)
          (
            start: box.start,
            end: box.end,
            spans: _boxSpans(context, t, box, style),
          ),
      // A tag in a link's label is part of the link.
      if (link == null)
        for (final tag in tags)
          if (tag.start >= start && tag.end <= end)
            (
              start: tag.start,
              end: tag.end,
              spans: [
                _tagSpan(context, t, tag, tagStyle(context, style), onSearch),
              ],
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
      out.addAll(part.spans);
      at = part.end;
    }
    if (end > at) {
      out.add(
        TextSpan(text: text.substring(at, end), style: style, recognizer: link),
      );
    }
  }

  /// [box], a button while there is [_ticking], keyed `box-<id>`; with
  /// Show ticks, who ticked it last and when after it.
  List<InlineSpan> _boxSpans(
    BuildContext context,
    AppLocalizations t,
    DrawnBox box,
    TextStyle? style,
  ) {
    final ticking = _ticking;
    final glyph = box.state.glyph;
    final label = boxStateLabel(t, box.state);
    final ticked = ticking?.labelOf(context, box.id);
    if (ticking == null || (!ticking.mayTick && ticked == null)) {
      return [TextSpan(text: glyph, semanticsLabel: label, style: style)];
    }
    return [
      WidgetSpan(
        alignment: PlaceholderAlignment.baseline,
        baseline: TextBaseline.alphabetic,
        child: _BoxButton(
          key: ValueKey('box-${box.id}'),
          glyph: glyph,
          label: label,
          style: style,
          onTap: (at) => unawaited(
            openBoxMenu(
              context,
              at,
              id: box.id,
              state: box.state,
              ticking: ticking,
            ),
          ),
        ),
      ),
      if (ticking.showTicks && ticked != null)
        TextSpan(text: ' $ticked ', style: _tickStyle(context, style)),
    ];
  }

  InlineSpan _tagSpan(
    BuildContext context,
    AppLocalizations t,
    FoundTag tag,
    TextStyle highlighted,
    TagSearch? onSearch,
  ) {
    final written = _source.substring(tag.start, tag.end);
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

/// How who ticked a box and when shows after it: small and grey.
TextStyle _tickStyle(BuildContext context, TextStyle? style) {
  final size = style?.fontSize ?? DefaultTextStyle.of(context).style.fontSize;
  return (style ?? const TextStyle()).copyWith(
    fontSize: (size ?? 14) * 0.75,
    fontWeight: FontWeight.normal,
    fontStyle: FontStyle.normal,
    decoration: TextDecoration.none,
    color: Theme.of(context).colorScheme.onSurfaceVariant,
  );
}

/// A box of a text being typed with who ticked it last and when after it
/// (Show ticks), drawn as one character of the text.
class _BoxWithTick extends StatelessWidget {
  final String glyph;
  final String label;
  final TextStyle style;

  const _BoxWithTick({
    required this.glyph,
    required this.label,
    required this.style,
  });

  @override
  Widget build(BuildContext context) => Text.rich(
    TextSpan(
      children: [
        TextSpan(text: glyph),
        TextSpan(text: ' $label ', style: _tickStyle(context, style)),
      ],
    ),
    style: style,
    textScaler: TextScaler.noScaling,
  );
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

/// A box that can be tapped, read as its state with a hint.
class _BoxButton extends StatelessWidget {
  final String glyph;
  final String label;
  final TextStyle? style;
  final void Function(Offset at) onTap;

  const _BoxButton({
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
    onTapHint: context.t.boxChange,
    onTap: () {
      // A screen reader has no finger: the menu opens at the box.
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
