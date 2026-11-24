import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:url_launcher/url_launcher.dart';

import '../l10n/app_localizations.dart';
import 'app_links.dart';
import 'box_ids.dart';
import 'found_marks.dart';
import 'l10n.dart';
import 'look.dart';
import 'person_screen.dart';
import 'profile.dart';
import 'table_screen.dart';
import 'tick_saves.dart';

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

  /// The state the box [id] shows, written as [starting]: as tapped
  /// while its save waits (#338), else as ticked in the log.
  BoxState stateOf(String id, BoxState starting) =>
      TickSaves.instance.waiting<BoxState>(_key(id)) ??
      ticks[id]?.state ??
      starting;

  String _key(String id) => tickKey(entity: entity, box: id);

  /// Ticks the box [id] to [to] at once, as [stateOf] shows it once
  /// [redraw] drew it, and writes it after the frame, then runs
  /// [changed] (#338). A sync may have taken the box or the role
  /// meanwhile: then nothing is written, the box shows as stored again
  /// and [failed] runs.
  void tick(
    String id,
    BoxState to, {
    required VoidCallback redraw,
    required void Function(Object error) failed,
  }) {
    TickSaves.instance.tick(
      _key(id),
      to,
      save: () {
        Boxes(notes.store).tick(entity, id, to);
        changed?.call();
      },
      failed: (error) {
        redraw();
        failed(error);
      },
    );
    redraw();
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
                  Text(to.glyph, style: _glyphStyle()),
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
      // The page the box is on draws it ticked in the next frame.
      void redraw() {
        if (context.mounted) (context as Element).markNeedsBuild();
      }
      ticking.tick(
        id,
        state,
        redraw: redraw,
        failed: (error) {
          if (context.mounted) announceTickFailed(context, error);
        },
      );
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
                  style: _glyphStyle(Theme.of(context).textTheme.titleLarge),
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
/// web or e-mail address ever gets here ([Markdown.mayOpen]), and a
/// guide page an app link names (ADR 0046); other app links go to the
/// [AppLinkScope].
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
/// How a formatted text is spaced and drawn (#327, obf-1020), alike on
/// the desktop: everything in parts of its font's size, so a bigger text
/// keeps its look. A line is [formattedLineHeight] high; code is
/// [codeScale] of the words around it, on a light tint ([codeTint]) only
/// as high as [codeLineHeight] says, never a framed box.
const formattedLineHeight = 1.5;

/// Code's line height, of inline code and of a code block's lines.
const codeLineHeight = 1.3;

/// Code's font size, in parts of the words' around it.
const codeScale = 0.9;

/// A first and a second heading's size, in parts of the words' size.
const heading1Scale = 1.3;
const heading2Scale = 1.15;

/// A heading's weight: lighter than bold.
const headingWeight = FontWeight.w600;

/// The room a blank line between two lines makes as the text reads.
const blankGap = 1.0;

/// How strongly code is tinted behind it with the text's colour.
const codeTintAlpha = 0.08;

/// The ground of code: a light tint of the text's colour, in light and
/// dark alike.
Color codeTint(ColorScheme scheme) =>
    scheme.onSurface.withValues(alpha: codeTintAlpha);

/// What a formatted text's lines are at least as high as: [base]'s font,
/// [formattedLineHeight] high.
StrutStyle formattedStrut(TextStyle base) =>
    StrutStyle.fromTextStyle(base, height: formattedLineHeight);

/// [base] as code is drawn in a formatted text: fixed-width, a little
/// smaller, on its tint just behind its words.
TextStyle codeStyle(ColorScheme scheme, TextStyle base) => base.copyWith(
  fontFamily: 'monospace',
  fontSize: (base.fontSize ?? 14) * codeScale,
  height: codeLineHeight,
  backgroundColor: codeTint(scheme),
);

/// The colour code's part of the kind [colour] is drawn in (obf-1036),
/// in the light or the dark look ([brightness]); only the glyphs take
/// it, the font, the size and the tint stay.
Color codeColour(CodeColour colour, Brightness brightness) {
  final dark = brightness == Brightness.dark;
  return Color(switch (colour) {
    CodeColour.comment => dark ? 0xFF8B949E : 0xFF6A737D,
    CodeColour.string => dark ? 0xFFA5D6FF : 0xFF0A3069,
    CodeColour.number ||
    CodeColour.constant ||
    CodeColour.attribute => dark ? 0xFF79C0FF : 0xFF0550AE,
    CodeColour.keyword || CodeColour.operator => dark ? 0xFFFF7B72 : 0xFFCF222E,
    CodeColour.type || CodeColour.variable => dark ? 0xFFFFA657 : 0xFF953800,
    CodeColour.function => dark ? 0xFFD2A8FF : 0xFF8250DF,
    CodeColour.tag => dark ? 0xFF7EE787 : 0xFF116329,
  });
}

/// The grammars code is coloured with below [context] (obf-1037): the
/// person's own with the built-in ones, as the store has them now; the
/// built-in ones alone outside the app.
Grammars codeGrammars(BuildContext context) =>
    PersonScope.of(context)?.store.grammars ?? Grammars.builtIn;

/// The coloured parts of [text] in [grammar]'s language, read once for
/// each grammar and text of the last few: a rebuild, or a card shown
/// again, does not read it anew. The grammar itself is the key, not its
/// scope, so one of the person's own in a built-in one's place, or that
/// one back after Remove, colours anew (obf-1037).
List<CodeSpan> codeSpans(Grammar grammar, String text) {
  final key = (grammar, text);
  final known = _codeSpans.remove(key);
  if (known != null) return _codeSpans[key] = known;
  final spans = grammar.colour(text);
  _codeSpans[key] = spans;
  if (_codeSpans.length > 64) _codeSpans.remove(_codeSpans.keys.first);
  return spans;
}

// Oldest first: a hit goes to the end again.
final _codeSpans = <(Grammar, String), List<CodeSpan>>{};

/// [text] from [from] to [to] as spans in [style], each coloured part of
/// [spans] (in [text]) in its colour; null when none of them lies there,
/// so it is drawn as without a language.
List<InlineSpan>? colouredCode(
  BuildContext context,
  String text,
  List<CodeSpan> spans,
  int from,
  int to,
) {
  final brightness = Theme.of(context).brightness;
  final children = <InlineSpan>[];
  var at = from;
  for (final span in spans) {
    final start = math.max(span.start, from);
    final end = math.min(span.end, to);
    if (start >= end) continue;
    if (start > at) children.add(TextSpan(text: text.substring(at, start)));
    children.add(
      TextSpan(
        text: text.substring(start, end),
        style: TextStyle(color: codeColour(span.colour, brightness)),
      ),
    );
    at = end;
  }
  if (children.isEmpty) return null;
  if (to > at) children.add(TextSpan(text: text.substring(at, to)));
  return children;
}

/// The room before a line of the kind [next] after one of [prev], in
/// parts of the font's size: more before a heading, less after one,
/// little between two items or two quote lines, half the size between
/// all others (#327).
double lineGap(MarkdownBlockKind prev, MarkdownBlockKind next) {
  bool item(MarkdownBlockKind k) =>
      k == MarkdownBlockKind.bullet || k == MarkdownBlockKind.numbered;
  bool heading(MarkdownBlockKind k) =>
      k == MarkdownBlockKind.heading1 || k == MarkdownBlockKind.heading2;
  if (heading(next)) return 0.8;
  if (heading(prev)) return 0.3;
  if (prev == MarkdownBlockKind.quote && next == MarkdownBlockKind.quote) {
    return 0.2;
  }
  if (item(prev) && item(next)) return 0.2;
  return 0.5;
}

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
///
/// A table of a formatted text is drawn as the grid [MarkedText] draws,
/// its source never seen (#152): the cursor and a selection pass it as
/// one block, never ending inside it ([snapOutOfTables]). With [grid] the
/// field draws each table as what [grid] gives, the grid it is edited in
/// (#257); the table [rawTable] shows as its lines, edited as text.
///
/// The places of [found] are drawn on a tint (obf-1047), the current one
/// on a stronger one, in a table's grid too.
class TagTextController extends TextEditingController {
  final bool hideMarks;

  TagTextController({super.text, this.hideMarks = false});

  /// The places the find bar found in [text] (obf-1047). Set, they show
  /// once the field is built again, as the page that sets them does: no
  /// listener of the text hears of them.
  FoundMarks? found;

  bool get _finds => !(found?.isEmpty ?? true);

  bool _markdown = false;

  /// Whether the text is formatted; it is drawn anew when this changes.
  bool get markdown => _markdown;
  set markdown(bool formatted) {
    if (formatted == _markdown) return;
    _markdown = formatted;
    final selection = snapOutOfTables(tables, value.selection);
    if (selection != value.selection) {
      super.value = value.copyWith(selection: selection);
      return;
    }
    notifyListeners();
  }

  bool _codeText = false;

  /// Whether the text is a Code text (#213): drawn as written in the
  /// fixed-width font, nothing in it a format, a tag or a box.
  bool get code => _codeText;
  set code(bool value) {
    if (value == _codeText) return;
    _codeText = value;
    notifyListeners();
  }

  String? _language;

  /// The language a Code text is coloured by (obf-1036), a grammar's
  /// scope; without one, or one not known here, it is in one colour.
  String? get codeLanguage => _language;
  set codeLanguage(String? scope) {
    if (scope == _language) return;
    _language = scope;
    notifyListeners();
  }

  String? _tablesText;
  List<MarkdownTable> _tablesOf = const [];

  /// The tables of [text], read once for each text.
  List<MarkdownTable> _tablesIn(String text) {
    if (text != _tablesText) {
      _tablesOf = MarkdownTables.all(text);
      _tablesText = text;
    }
    return _tablesOf;
  }

  /// The tables drawn as grids: those of a formatted text, top first,
  /// but [rawTable].
  List<MarkdownTable> get tables =>
      _markdown ? _gridsIn(text) : const <MarkdownTable>[];

  List<MarkdownTable> _gridsIn(String text) {
    final all = _tablesIn(text);
    final raw = _raw;
    if (raw == null || raw >= all.length) return all;
    return [...all.take(raw), ...all.skip(raw + 1)];
  }

  int? _raw;

  /// The table, by its place among the tables of the text, that shows as
  /// its Markdown lines rather than as a grid; null when none does. It is
  /// only how the field shows it now, never stored.
  int? get rawTable => _raw;
  set rawTable(int? index) {
    if (index == _raw) return;
    _raw = index;
    final selection = snapOutOfTables(tables, value.selection);
    if (selection != value.selection) {
      super.value = value.copyWith(selection: selection);
      return;
    }
    notifyListeners();
  }

  /// What draws the table [index] of the text, [table], in the field in
  /// [style]; without it, the grid [MarkedText] draws, only read.
  Widget Function(int index, MarkdownTable table, TextStyle? style)? grid;

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
    final before = super.value.selection;
    var selection = snapSelection(
      newValue.text,
      newValue.selection,
      before: before,
    );
    if (_markdown) {
      selection = snapOutOfTables(
        _gridsIn(newValue.text),
        selection,
        before: before,
      );
    }
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
    if (_codeText) {
      final mono = (style ?? const TextStyle()).copyWith(
        fontFamily: 'monospace',
      );
      final grammar = codeGrammars(context).byScope(_language);
      final spans = grammar == null
          ? const <CodeSpan>[]
          : codeSpans(grammar, text);
      if (spans.isEmpty && !_finds) {
        return super.buildTextSpan(
          context: context,
          style: mono,
          withComposing: withComposing,
        );
      }
      return _colouredCode(context, mono, spans, composing);
    }
    final runs = tagRuns(text);
    if (_markdown || runs.isNotEmpty || _finds) {
      return _formatted(context, style, composing, runs);
    }
    final tags = Markdown.plainTags(text);
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

  /// A Code text in [style], each part of [spans] in its colour, what is
  /// being composed underlined.
  TextSpan _colouredCode(
    BuildContext context,
    TextStyle style,
    List<CodeSpan> spans,
    TextRange composing,
  ) {
    final text = this.text;
    final brightness = Theme.of(context).brightness;
    final marks = found;
    final cuts = <int>{
      0,
      text.length,
      for (final span in spans) ...[span.start, span.end],
      if (composing.isValid) ...[composing.start, composing.end],
      for (final (a, b) in marks?.ranges ?? const <(int, int)>[]) ...[a, b],
    }.where((at) => at >= 0 && at <= text.length).toList()..sort();
    var next = 0;
    final children = <InlineSpan>[];
    for (var i = 0; i + 1 < cuts.length; i++) {
      final (from, to) = (cuts[i], cuts[i + 1]);
      if (from >= to) continue;
      while (next < spans.length && spans[next].end <= from) {
        next++;
      }
      final colour = next < spans.length && spans[next].start <= from
          ? codeColour(spans[next].colour, brightness)
          : null;
      final composed =
          composing.isValid && composing.start <= from && to <= composing.end;
      final plain = colour == null && !composed
          ? null
          : TextStyle(
              color: colour,
              decoration: composed ? TextDecoration.underline : null,
            );
      children.add(
        TextSpan(
          text: text.substring(from, to),
          style: switch (marks?.at(from, to)) {
            final current? => foundStyle(context, plain, current: current),
            null => plain,
          },
        ),
      );
    }
    return TextSpan(style: style, children: children);
  }

  /// The colour of each code block's line of [text] whose fence names a
  /// language of [grammars] (obf-1036), by character; null where none.
  static List<CodeColour?> _blockColours(
    String text,
    List<MarkdownBlock> blocks,
    Grammars grammars,
  ) {
    final colours = List<CodeColour?>.filled(text.length, null);
    MarkdownCodeBlock? done;
    for (final block in blocks) {
      final code = block.code;
      if (code == null || identical(code, done)) continue;
      done = code;
      final grammar = grammars.forFence(code.language);
      if (grammar == null) continue;
      final lines = [
        for (var l = code.firstLine; l <= code.lastLine; l++)
          if (!code.isFence(l)) blocks[l],
      ];
      if (lines.isEmpty) continue;
      // Its lines as one piece, so what runs on over lines reads so.
      final from = lines.first.start;
      final to = lines.last.end;
      for (final span in codeSpans(grammar, text.substring(from, to))) {
        for (var i = from + span.start; i < from + span.end && i < to; i++) {
          colours[i] = span.colour;
        }
      }
    }
    return colours;
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
  static const _grid = 8192;
  static const _codeLine = 16384;
  static const _found = 32768;
  static const _foundNow = 65536;

  /// Above the other bits: a code block's character's [CodeColour], one
  /// more than its index, or 0.
  static const _colourShift = 17;

  /// The text formatted, character for character: each one gets the bits
  /// of what it is, and neighbours with the same bits share a span. A
  /// plain text is drawn so too when it has boxes ([runs]): each glyph in
  /// its state, its id as nothing. A table is its grid in place of its
  /// first character, the others nothing that never breaks the line.
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

    final raw = switch (_raw) {
      final index? when _markdown => _tablesIn(text).elementAtOrNull(index),
      _ => null,
    };
    final blocks = _markdown ? Markdown.parse(text) : <MarkdownBlock>[];
    for (final block in blocks) {
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
        case MarkdownBlockKind.table when block.table?.start == raw?.start:
          // A table shown as its lines shows them as written.
          break;
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
        case MarkdownBlockKind.code:
          // A code block's lines are code, its fences marks with the line
          // break after them, so a fence drawn as nothing takes no row.
          if (block.code?.isFence(block.line) ?? false) {
            var end = block.end;
            if (end < text.length && text[end] == '\r') end++;
            if (end < text.length && text[end] == '\n') end++;
            set(block.start, end, _mark);
          } else {
            set(block.start, block.end, _code | _codeLine);
          }
        case MarkdownBlockKind.paragraph:
          break;
      }
      block.spans.forEach(walk);
    }
    // A code block's words in the colours of its fence's language.
    final colours = _blockColours(text, blocks, codeGrammars(context));
    for (var i = 0; i < text.length; i++) {
      final colour = colours[i];
      if (colour != null && bits[i] & (_codeLine | _mark) == _codeLine) {
        bits[i] |= (colour.index + 1) << _colourShift;
      }
    }
    final tags = _markdown ? Markdown.tags(text) : Markdown.plainTags(text);
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
    // The places the find bar found.
    if (found case final marks?) {
      for (final (i, (a, b)) in marks.ranges.indexed) {
        set(a, b, i == marks.current ? _foundNow : _found);
      }
    }
    // A table's grid stands for its first character; the others are word
    // joiners, which take no room and keep the grid on its line.
    final grids = <int, (int, MarkdownTable)>{};
    for (final table in tables) {
      grids[table.start] = (_tablesIn(text).indexOf(table), table);
      labels.removeWhere((at, _) => at >= table.start && at < table.end);
      for (var i = table.start; i < table.end; i++) {
        bits[i] = _grid;
        shown[i] = 0x2060;
      }
    }
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
          !labels.containsKey(from) &&
          !grids.containsKey(from) &&
          !grids.containsKey(i)) {
        continue;
      }
      final b = bits[from];
      if (grids[from] case (final index, final table)) {
        final grid = this.grid;
        children.add(
          WidgetSpan(
            alignment: PlaceholderAlignment.middle,
            child: grid != null
                ? grid(index, table, style)
                // Only read: the field takes the taps.
                : ExcludeSemantics(
                    child: IgnorePointer(
                      child: MarkedText(
                        text.substring(table.start, table.end),
                        key: ValueKey('field-table-${table.start}'),
                        format: TextFormats.markdown,
                        style: style,
                        ticking: _boxes,
                        found: found?.shifted(-table.start),
                        tableMenu: false,
                      ),
                    ),
                  ),
          ),
        );
      } else if (labels[from] case final label?) {
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
    if (has(_grid)) {
      return const TextStyle(
        fontSize: 0.01,
        color: Colors.transparent,
        backgroundColor: Colors.transparent,
      );
    }
    final lines = [
      if (has(_strike)) TextDecoration.lineThrough,
      if (has(_link) || has(_composing)) TextDecoration.underline,
    ];
    final heading = has(_heading1)
        ? size * heading1Scale
        : has(_heading2)
        ? size * heading2Scale
        : null;
    final coloured = bits >> _colourShift;
    final style = TextStyle(
      fontSize: has(_code) ? (heading ?? size) * codeScale : heading,
      // A code block's lines lie together on their tint.
      height: has(_codeLine)
          ? formattedLineHeight / codeScale
          : has(_code)
          ? codeLineHeight
          : null,
      fontWeight: has(_heading1) || has(_heading2)
          ? headingWeight
          : has(_bold)
          ? FontWeight.bold
          : null,
      fontStyle: has(_italic) ? FontStyle.italic : null,
      fontFamily: has(_code) ? 'monospace' : null,
      backgroundColor: has(_code) ? codeTint(scheme) : null,
      color: has(_link) || has(_tag) || has(_sign)
          ? scheme.primary
          : coloured > 0
          ? codeColour(
              CodeColour.values[coloured - 1],
              Theme.of(context).brightness,
            )
          : has(_quote)
          ? scheme.onSurfaceVariant
          : null,
      decoration: lines.isEmpty ? null : TextDecoration.combine(lines),
    );
    if (has(_found) || has(_foundNow)) {
      // Found: on its tint; a mark and an id are drawn as they are.
      final tinted = has(_foundNow)
          ? TextStyle(
              backgroundColor: scheme.tertiary,
              color: scheme.onTertiary,
            )
          : TextStyle(
              backgroundColor: scheme.tertiaryContainer,
              color: scheme.onTertiaryContainer,
            );
      if (!has(_id) && !has(_mark)) return style.merge(tinted);
    }
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

/// [selection] with its ends out of each of [tables], drawn as one block
/// each: an end inside one goes to its end, or to its start when it came
/// back from further on ([before]); without [before], to the nearer one.
TextSelection snapOutOfTables(
  List<MarkdownTable> tables,
  TextSelection selection, {
  TextSelection? before,
}) {
  if (!selection.isValid || tables.isEmpty) return selection;
  int snap(int offset, int? was) {
    for (final table in tables) {
      if (offset <= table.start || offset >= table.end) continue;
      final back = was == null
          ? offset - table.start < table.end - offset
          : was > offset;
      return back ? table.start : table.end;
    }
    return offset;
  }

  final base = snap(selection.baseOffset, before?.baseOffset);
  final extent = snap(selection.extentOffset, before?.extentOffset);
  if (base == selection.baseOffset && extent == selection.extentOffset) {
    return selection;
  }
  return selection.copyWith(baseOffset: base, extentOffset: extent);
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
/// the search for it (ADR 0032). Each bare web address, `http://` or
/// `https://` ([Markdown.plainAutolinks], #222), is a link that opens
/// with a tap ([LinkOpenScope]), as one of a formatted text does; a tap
/// beside it and a long press go to what lies under the text.
///
/// Without [boxes] the text may be one stored with the ids of its boxes,
/// such as a Sent Item's: it is drawn without them, each box in the
/// glyph it was written with.
///
/// A Code text ([TextFormats.code], #213) is drawn as written in
/// monospace, unwrapped, coloured by its [language] (a grammar's scope,
/// obf-1036) when it is one known here.
///
/// A text in [TextFormats.markdown] is drawn formatted (ADR 0033), each
/// of its lines that [start] to [end] touches: headings larger, items
/// after a bullet or their number, quotes beside a bar, bold, italic,
/// struck through, code in a fixed width on a light tint and links; a
/// code block whose fence names a language known here is coloured by
/// it (obf-1036). A
/// link to a web or e-mail address opens with a tap ([LinkOpenScope]);
/// any other stays text. Boxes, tags and web addresses work in it as in
/// a plain text, except a tag in a link's label, which is part of the
/// link, and a web address in code, a code block or a link, which is
/// none ([Markdown.autolinks]).
///
/// A table is drawn as a grid at the text's size, its header row in bold,
/// its boxes as in a line, each column as wide as its words up to a
/// bound, a longer cell wrapping (obf-1048). A table wider than its room
/// moves sideways with a drag on it or by its bar, its first column
/// staying while it holds headings. With [rows] a table shows its header
/// and that many rows under it, and "Show all" unfolds the rest in
/// place; a table holding [unfoldAt] shows all of them. A tap on it opens
/// it full screen ([openTable]) in the text as stored ([source]), where
/// [tableEdit] changes it; it is only read there without one. Its ⋮ at
/// its top right corner copies it (obf-1050). A compact line
/// ([maxLines]) of a table shows the words of its row.
///
/// The places of [found], in [text] as it shows, are drawn on a tint
/// (obf-1047), in a table's cells too; its current one on a stronger one.
class MarkedText extends StatefulWidget {
  /// How many rows under its header a table on a card shows before
  /// "Show all" (obf-1048).
  static const cardRows = 10;

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
  final String? language;

  /// How many rows under its header each table shows folded; all when
  /// null.
  final int? rows;

  /// An offset in [text], such as a search hit's (obf-1047): the table
  /// holding it shows all its rows.
  final int? unfoldAt;

  /// The places a search or the find bar found in [text] (obf-1047).
  final FoundMarks? found;

  /// Whether each table has its ⋮ with Copy (obf-1050); not where the
  /// text is drawn only for a field that takes the taps.
  final bool tableMenu;

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
    this.language,
    this.rows,
    this.unfoldAt,
    this.found,
    this.tableMenu = true,
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
  List<(int, int)> get _addresses => _foundIn.markdownLinks;

  @override
  BoxTicking? get _ticking => widget.ticking;

  @override
  FoundMarks? get _marks => widget.found;

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

  // The text grows with the Device's text size (#326, obf-1019); what is
  // built reads the size it shows at below the [ContentText].
  @override
  Widget build(BuildContext context) =>
      ContentText(child: Builder(builder: _build));

  Widget _build(BuildContext context) {
    if (widget.format == TextFormats.markdown) return _formatted(context);
    if (widget.format == TextFormats.code) return _code(context);
    final found = _foundIn;
    final text = found.shown;
    final from = (widget.start ?? 0).clamp(0, text.length);
    final to = (widget.end ?? text.length).clamp(from, text.length);
    final links = [
      for (final link in found.links)
        if (link.$1 >= from && link.$2 <= to) link,
    ];
    final boxes = [
      for (final box in found.boxes)
        if (box.start >= from &&
            box.end <= to &&
            !_inAddress(found.links, box.start, box.end))
          box,
    ];
    final tags = [
      for (final tag in found.tags)
        if (tag.start >= from && tag.end <= to) tag,
    ];
    final marks = widget.found;
    if (boxes.isEmpty &&
        tags.isEmpty &&
        links.isEmpty &&
        (marks == null || marks.isEmpty)) {
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
    // Boxes, tags and web addresses never overlap: a glyph is no tag
    // character, and neither is a box or a tag inside an address. A tag
    // or an address a place found touches is drawn found as a whole.
    final parts = <({int start, int end, List<InlineSpan> spans})>[
      for (final (start, end) in links)
        (
          start: start,
          end: end,
          spans: [
            _addressSpan(
              context,
              start,
              end,
              _foundOver(
                context,
                start,
                end,
                widget.style ?? DefaultTextStyle.of(context).style,
              ),
            ),
          ],
        ),
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
          spans: [
            _tagSpan(
              context,
              t,
              tag,
              _foundOver(context, tag.start, tag.end, highlighted),
              onSearch,
            ),
          ],
        ),
    ]..sort((a, b) => a.start.compareTo(b.start));
    var at = from;
    final spans = <InlineSpan>[];
    for (final part in parts) {
      if (part.start > at) {
        spans.addAll(foundSpans(context, text, at, part.start, marks));
      }
      spans.addAll(part.spans);
      at = part.end;
    }
    if (to > at) spans.addAll(foundSpans(context, text, at, to, marks));
    return Text.rich(
      TextSpan(children: spans),
      style: widget.style,
      maxLines: widget.maxLines,
      overflow: widget.overflow,
    );
  }

  /// A Code text (#213) from [start] to [end] as written, in monospace,
  /// unwrapped and cut off at the edge: swipes on a card belong to the
  /// browser, and the Note editor shows it whole. Its lines lie on the
  /// tint of a code block, a compact line ([maxLines]) without
  /// it.
  Widget _code(BuildContext context) {
    final text = widget.text;
    final from = (widget.start ?? 0).clamp(0, text.length);
    final to = (widget.end ?? text.length).clamp(from, text.length);
    final style = DefaultTextStyle.of(context).style
        .merge(widget.style)
        .copyWith(fontFamily: 'monospace');
    // The whole text is read, so what began before [from] reads so.
    final grammar = codeGrammars(context).byScope(widget.language);
    final marks = widget.found;
    final painted = grammar == null
        ? null
        : colouredCode(context, text, codeSpans(grammar, text), from, to);
    final coloured = painted != null
        ? markSpans(context, painted, from, marks)
        : marks == null || marks.isEmpty
        ? null
        : foundSpans(context, text, from, to, marks);
    final lines = coloured == null
        ? Text(
            text.substring(from, to),
            style: style,
            softWrap: false,
            maxLines: widget.maxLines,
            overflow: widget.overflow ?? TextOverflow.clip,
          )
        : Text.rich(
            TextSpan(children: coloured),
            style: style,
            softWrap: false,
            maxLines: widget.maxLines,
            overflow: widget.overflow ?? TextOverflow.clip,
          );
    if (widget.maxLines != null) return lines;
    return Container(
      key: const ValueKey('code-text'),
      width: double.infinity,
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
      decoration: BoxDecoration(
        color: codeTint(Theme.of(context).colorScheme),
        borderRadius: BorderRadius.circular(4),
      ),
      child: lines,
    );
  }

  /// Each line of a formatted text in [start] to [end], one under the
  /// other; a table once, as a grid, and a code block once, as its lines
  /// in monospace on a tint. A compact card shows a code block's
  /// lines one by one, without its fences.
  Widget _formatted(BuildContext context) {
    final found = _foundIn;
    final text = found.shown;
    final from = widget.start ?? 0;
    final to = widget.end ?? text.length;
    final boxes = found.boxes;
    final tags = found.markdownTags;
    final base = DefaultTextStyle.of(context).style.merge(widget.style);
    final compact = widget.maxLines != null;
    final size = base.fontSize ?? 14;
    // The room is in parts of the size the text shows at, the system's
    // font size, "Fat fingers" and the text size in it (#326, obf-1019).
    final em = MediaQuery.textScalerOf(context).scale(size);
    final lines = <Widget>[];
    // The kind of the line drawn last and the blank lines since: the room
    // before a line goes by both (#327); a compact card's lines keep only
    // their height.
    MarkdownBlockKind? last;
    var blank = 0;
    void room(MarkdownBlockKind kind) {
      if (!compact && (last != null || blank > 0)) {
        final gap = blank > 0 ? blank * blankGap : lineGap(last ?? kind, kind);
        lines.add(SizedBox(height: gap * em));
      }
      last = kind;
      blank = 0;
    }

    MarkdownCodeBlock? drawn;
    for (final block in found.blocks) {
      if (block.end < from || block.start > to) continue;
      final table = block.table;
      if (block.code case final code?) {
        if (compact) {
          if (!code.isFence(block.line)) {
            room(MarkdownBlockKind.code);
            lines.add(
              _colouredCodeLine(context, code, block, base) ??
                  _line(context, block, base, boxes, tags),
            );
          }
        } else if (!identical(code, drawn)) {
          drawn = code;
          room(MarkdownBlockKind.code);
          lines.add(_codeBlock(context, code, base, from, to));
        }
      } else if (table == null) {
        // A blank line reads as room, not as a line.
        if (!compact &&
            block.kind == MarkdownBlockKind.paragraph &&
            text.substring(block.start, block.end).trim().isEmpty) {
          blank++;
          continue;
        }
        room(block.kind);
        lines.add(_line(context, block, base, boxes, tags));
      } else if (compact) {
        // Its comment line reads as nothing: the header row stands in.
        final row = block.row ?? (block.line == table.firstLine ? 0 : null);
        if (row != null) {
          room(MarkdownBlockKind.table);
          lines.add(_rowLine(context, table.rows[row], base, boxes, tags));
        }
      } else if (block.line == table.firstLine) {
        room(MarkdownBlockKind.table);
        lines.add(_table(context, table, base, boxes, tags));
      }
    }
    // Blank lines at the end keep their room.
    if (blank > 0) lines.add(SizedBox(height: blank * blankGap * em));
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
        fontWeight: headingWeight,
        fontSize: grows ? size * heading1Scale : null,
      ),
      MarkdownBlockKind.heading2 => base.copyWith(
        fontWeight: headingWeight,
        fontSize: grows ? size * heading2Scale : null,
      ),
      MarkdownBlockKind.quote => base.copyWith(color: scheme.onSurfaceVariant),
      MarkdownBlockKind.code => codeStyle(scheme, base),
      _ => base,
    };
    final spans = <InlineSpan>[];
    for (final span in block.spans) {
      _inline(context, span, style, null, boxes, tags, spans);
    }
    final words = Text.rich(
      TextSpan(children: spans),
      style: style,
      strutStyle: formattedStrut(base),
      maxLines: widget.maxLines,
      overflow: widget.overflow,
    );
    return switch (block.kind) {
      MarkdownBlockKind.paragraph ||
      MarkdownBlockKind.table ||
      MarkdownBlockKind.code => words,
      MarkdownBlockKind.heading1 ||
      MarkdownBlockKind.heading2 => Semantics(header: true, child: words),
      MarkdownBlockKind.bullet => _item(
        _grown(context, size),
        block.level == 0 ? '•' : '◦',
        block.level,
        style,
        words,
      ),
      MarkdownBlockKind.numbered => _item(
        _grown(context, size),
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

  /// The lines of [code] in [from] to [to] as written, in monospace on a
  /// tint: nothing in them is a format, a tag or a box.
  Widget _codeBlock(
    BuildContext context,
    MarkdownCodeBlock code,
    TextStyle base,
    int from,
    int to,
  ) {
    final found = _foundIn;
    final (all, starts) = _codeContent(code);
    // The lines in [from] to [to], one after the other.
    int? first;
    var last = 0;
    for (final MapEntry(key: l, value: at) in starts.entries) {
      final b = found.blocks[l];
      if (b.end < from || b.start > to) continue;
      first ??= at;
      last = at + b.end - b.start;
    }
    final start = first ?? 0;
    final end = first == null ? 0 : last;
    final grammar = codeGrammars(context).forFence(code.language);
    // The places found, from the text to where they lie in its lines.
    final marks = widget.found;
    FoundMarks? inLines;
    if (marks != null && !marks.isEmpty) {
      final ranges = <(int, int)>[];
      int? current;
      for (final (i, (a, b)) in marks.ranges.indexed) {
        for (final MapEntry(key: l, value: at) in starts.entries) {
          final line = found.blocks[l];
          if (a < line.start || a > line.end) continue;
          if (i == marks.current) current = ranges.length;
          ranges.add((at + a - line.start, at + b - line.start));
        }
      }
      inLines = FoundMarks(ranges, current);
    }
    final painted = grammar == null
        ? null
        : colouredCode(context, all, codeSpans(grammar, all), start, end);
    final coloured = painted != null
        ? markSpans(context, painted, start, inLines)
        : inLines == null
        ? null
        : foundSpans(context, all, start, end, inLines);
    final scheme = Theme.of(context).colorScheme;
    final grow = _grown(context, base.fontSize ?? 14);
    final style = codeStyle(
      scheme,
      base,
    ).copyWith(backgroundColor: Colors.transparent);
    return Container(
      key: ValueKey('code-${code.firstLine}'),
      width: double.infinity,
      padding: EdgeInsets.symmetric(horizontal: 8 * grow, vertical: 6 * grow),
      decoration: BoxDecoration(
        color: codeTint(scheme),
        borderRadius: BorderRadius.circular(4),
      ),
      // The block's tint is its ground: its words have none of their own.
      child: coloured == null
          ? Text(all.substring(start, end), style: style)
          : Text.rich(TextSpan(children: coloured), style: style),
    );
  }

  /// The lines of [code] but its fences as one piece, as its language
  /// reads them, and where each of them starts in it, by its line.
  (String, Map<int, int>) _codeContent(MarkdownCodeBlock code) {
    final found = _foundIn;
    final text = found.shown;
    final starts = <int, int>{};
    final all = StringBuffer();
    for (var l = code.firstLine; l <= code.lastLine; l++) {
      if (code.isFence(l)) continue;
      if (starts.isNotEmpty) all.write('\n');
      starts[l] = all.length;
      final b = found.blocks[l];
      all.write(text.substring(b.start, b.end));
    }
    return (all.toString(), starts);
  }

  /// A line of [code], [block], on a compact card in the colours of its
  /// fence's language; null when it names none known here, or nothing
  /// of the line is coloured.
  Widget? _colouredCodeLine(
    BuildContext context,
    MarkdownCodeBlock code,
    MarkdownBlock block,
    TextStyle base,
  ) {
    final grammar = codeGrammars(context).forFence(code.language);
    if (grammar == null) return null;
    final (all, starts) = _codeContent(code);
    final at = starts[block.line];
    if (at == null) return null;
    final coloured = colouredCode(
      context,
      all,
      codeSpans(grammar, all),
      at,
      at + block.end - block.start,
    );
    if (coloured == null) return null;
    return Text.rich(
      TextSpan(children: coloured),
      style: codeStyle(Theme.of(context).colorScheme, base),
      strutStyle: formattedStrut(base),
      maxLines: widget.maxLines,
      overflow: widget.overflow,
    );
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
      strutStyle: formattedStrut(base),
      maxLines: widget.maxLines,
      overflow: widget.overflow,
    );
  }

  /// [table] as a grid at the text's size; a tap opens it full screen.
  Widget _table(
    BuildContext context,
    MarkdownTable table,
    TextStyle base,
    List<DrawnBox> boxes,
    List<FoundTag> tags,
  ) {
    final found = _foundIn;
    final edit = widget.tableEdit;
    final ticking = widget.ticking;
    final at = widget.unfoldAt;
    return _ReadTable(
      key: ValueKey('table-${table.firstLine}'),
      table: table,
      base: base,
      rows: widget.rows,
      unfold: at != null && at >= table.start && at <= table.end,
      cell: (context, r, c, style) => Text.rich(
        TextSpan(
          children: [
            for (final span in table.rows[r].cells[c].spans)
              ..._inlineOf(context, span, style, boxes, tags),
          ],
        ),
        style: style,
        textAlign: cellAlign(table.aligns[c]),
      ),
      onOpen: () => unawaited(
        openTable(
          context,
          text: found.stored(widget.source),
          at: found.storedAt(table.start),
          edit: edit,
          ticking: ticking,
        ),
      ),
      onCopy: widget.tableMenu
          ? () => copyTable(
              context,
              text: found.stored(widget.source),
              at: found.storedAt(table.start),
              ticking: ticking,
            )
          : null,
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

  /// How many times its [size] the text around [context] shows at (#326,
  /// obf-1019): the room of an item and of a code block grows with it.
  double _grown(BuildContext context, double size) =>
      MediaQuery.textScalerOf(context).scale(size) / size;

  /// An item of a list: its bullet or number, the words beside it, its
  /// room [grow] times the usual.
  Widget _item(
    double grow,
    String sign,
    int level,
    TextStyle style,
    Widget words,
  ) => Padding(
    padding: EdgeInsetsDirectional.only(start: level * 20.0 * grow),
    child: Row(
      crossAxisAlignment: CrossAxisAlignment.baseline,
      textBaseline: TextBaseline.alphabetic,
      children: [
        ConstrainedBox(
          constraints: BoxConstraints(minWidth: 20 * grow),
          child: Text(sign, style: style),
        ),
        SizedBox(width: 4 * grow),
        Flexible(child: words),
      ],
    ),
  );
}

/// A table of a formatted text being read (obf-1048): each column as
/// wide as its widest shown cell's words, no wider than
/// [TableScreen.widest] or the room, a longer cell wrapping; wider than
/// its room it moves sideways with a drag on its cells or by its bar,
/// the first column staying while it holds headings. With [rows] it shows
/// its header and that many rows under it, and "Show all" under it
/// unfolds the rest in place; [unfold] shows all of them. Only the rows
/// shown are measured and built. A tap opens it ([onOpen]); its ⋮ at its
/// top right corner offers Copy ([onCopy], obf-1050), none without it.
class _ReadTable extends StatefulWidget {
  final MarkdownTable table;
  final TextStyle base;
  final int? rows;
  final bool unfold;

  /// The words of the cell in row [r] and column [c] in [style].
  final Widget Function(BuildContext context, int r, int c, TextStyle style)
  cell;
  final VoidCallback onOpen;
  final Future<void> Function()? onCopy;

  const _ReadTable({
    super.key,
    required this.table,
    required this.base,
    required this.rows,
    required this.unfold,
    required this.cell,
    required this.onOpen,
    this.onCopy,
  });

  @override
  State<_ReadTable> createState() => _ReadTableState();
}

class _ReadTableState extends State<_ReadTable> {
  /// The room around a cell's words, sideways and up and down.
  static const _across = 8.0;
  static const _down = 4.0;

  final _offset = ValueNotifier<double>(0);
  late bool _unfolded = widget.unfold;
  List<double> _widths = const [];

  /// How many rows show, the header row included.
  int get _shown {
    final rows = widget.table.rows.length;
    final folded = widget.rows;
    if (_unfolded || folded == null) return rows;
    return math.min(rows, folded + 1);
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _measure();
  }

  @override
  void didUpdateWidget(_ReadTable old) {
    super.didUpdateWidget(old);
    if (widget.unfold && !old.unfold) _unfolded = true;
    if (!identical(old.table, widget.table) ||
        old.base != widget.base ||
        old.rows != widget.rows ||
        widget.unfold != old.unfold) {
      _measure();
    }
  }

  @override
  void dispose() {
    _offset.dispose();
    super.dispose();
  }

  /// Each column as wide as its widest shown cell's words in bold, within
  /// bounds.
  void _measure() {
    final table = widget.table;
    final bold = widget.base.copyWith(fontWeight: FontWeight.bold);
    final scaler = MediaQuery.textScalerOf(context);
    final direction = Directionality.of(context);
    double width(String words) {
      final painter = TextPainter(
        text: TextSpan(text: words, style: bold),
        textDirection: direction,
        textScaler: scaler,
        maxLines: 1,
      )..layout();
      final width = painter.width;
      painter.dispose();
      return width;
    }

    final shown = _shown;
    _widths = [
      for (var c = 0; c < table.columns; c++)
        [for (var r = 0; r < shown; r++) width(table.rows[r].cells[c].visible)]
                .fold(0.0, math.max)
                .clamp(
                  TableScreen.narrowest - 2 * _across,
                  TableScreen.widest - 2 * _across,
                ) +
            2 * _across,
    ];
  }

  void _unfoldAll() => setState(() {
    _unfolded = true;
    _measure();
  });

  /// The ⋮ at [at] (global): Copy.
  Future<void> _menu(Offset at) async {
    final picked = await showTableMenu(context, at);
    if (!mounted || picked != TableAction.copy) return;
    await widget.onCopy?.call();
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final table = widget.table;
    final hidden = table.rows.length - _shown;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        Padding(
          padding: const EdgeInsets.symmetric(vertical: 4),
          child: widget.onCopy == null
              ? LayoutBuilder(builder: _grid)
              : Row(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Flexible(child: LayoutBuilder(builder: _grid)),
                    TableMenuButton(
                      key: ValueKey('table-menu-${table.firstLine}'),
                      onMenu: (at) => unawaited(_menu(at)),
                    ),
                  ],
                ),
        ),
        if (hidden > 0)
          TextButton(
            key: ValueKey('table-show-all-${table.firstLine}'),
            onPressed: _unfoldAll,
            child: Text(t.tableShowAll(table.rows.length - 1)),
          ),
      ],
    );
  }

  Widget _grid(BuildContext context, BoxConstraints constraints) {
    final t = context.t;
    final table = widget.table;
    final room = constraints.maxWidth;
    final first = table.headingColumn ? 1 : 0;
    // A column is never wider than the room, the one that stays no wider
    // than half of it: the others need room to move past it.
    final widths = [
      for (final (c, w) in _widths.indexed)
        math.min(w, c < first ? room / 2 : room),
    ];
    final fixed = first == 1 ? widths[0] : 0.0;
    var moving = 0.0;
    for (var c = first; c < table.columns; c++) {
      moving += widths[c];
    }
    final rest = math.max(0.0, room - fixed);
    final most = math.max(0.0, moving - rest);
    final rows = [
      for (var r = 0; r < _shown; r++) _row(context, r, widths, first, most),
    ];
    return Semantics(
      container: true,
      onTapHint: t.tableOpen,
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTap: widget.onOpen,
        // A drag on the cells moves a table wider than its room under the
        // finger; one that fits leaves the swipe to the page.
        onHorizontalDragUpdate: most > 0
            ? (d) => _offset.value = (_offset.value - d.delta.dx).clamp(0, most)
            : null,
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          mainAxisSize: MainAxisSize.min,
          children: [
            if (most > 0)
              TableSidewaysBar(
                key: ValueKey('table-sideways-${table.firstLine}'),
                offset: _offset,
                most: most,
                room: rest,
                whole: moving,
                label: t.tableSideways,
              ),
            ...rows,
          ],
        ),
      ),
    );
  }

  /// The row [r]: its first column stays when it holds headings ([first]
  /// is 1), the others move by the offset up to [most].
  Widget _row(
    BuildContext context,
    int r,
    List<double> widths,
    int first,
    double most,
  ) {
    final table = widget.table;
    final cells = [
      for (var c = first; c < table.columns; c++)
        SizedBox(width: widths[c], child: _cellAt(context, r, c)),
    ];
    return IntrinsicHeight(
      key: ValueKey('table-${table.firstLine}-row-$r'),
      child: Row(
        mainAxisSize: most > 0 ? MainAxisSize.max : MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (first == 1)
            SizedBox(width: widths[0], child: _cellAt(context, r, 0)),
          if (most > 0)
            Expanded(
              child: TableShifted(
                offset: _offset,
                most: most,
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: cells,
                ),
              ),
            )
          else
            ...cells,
        ],
      ),
    );
  }

  Widget _cellAt(BuildContext context, int r, int c) {
    final scheme = Theme.of(context).colorScheme;
    final table = widget.table;
    final heading = r == 0 || (c == 0 && table.headingColumn);
    final base = widget.base;
    final style = heading ? base.copyWith(fontWeight: FontWeight.bold) : base;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: heading ? scheme.surfaceContainerHighest : null,
        border: Border.all(color: scheme.outline, width: 0.5),
      ),
      child: Padding(
        padding: const EdgeInsets.symmetric(
          horizontal: _across,
          vertical: _down,
        ),
        child: widget.cell(context, r, c, style),
      ),
    );
  }
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

  late final ShownText _shownText = switch (given) {
    final boxes? => (text: text, boxes: boxes),
    null => TextBoxes.shown(text),
  };

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

  late final tags = Markdown.plainTags(shown);
  late final blocks = Markdown.parse(shown);
  late final markdownTags = Markdown.tags(shown);

  /// The web addresses of [shown] as a plain text and as a formatted one
  /// (#222).
  late final links = Markdown.plainAutolinks(shown);
  late final markdownLinks = Markdown.autolinks(shown);
}

/// Whether the part [start] to [end] of a text overlaps one of its web
/// addresses [links]: a box there is none.
bool _inAddress(List<(int, int)> links, int start, int end) =>
    links.any((l) => start < l.$2 && l.$1 < end);

/// The words of [cell] of a table of the formatted [text] as [MarkedText]
/// draws them: bold, italic, struck through, code and links, its web
/// addresses ([links], #222) links too, its boxes ([boxes], in [text])
/// buttons with [ticking], its tags links to the search. [boxes], [tags]
/// and [links] are those of the whole text.
class MarkedCell extends StatefulWidget {
  final String text;
  final MarkdownTableCell cell;
  final List<DrawnBox> boxes;
  final List<FoundTag> tags;
  final List<(int, int)> links;
  final TextStyle? style;
  final TextAlign textAlign;
  final BoxTicking? ticking;

  /// The places found in [text] (obf-1047).
  final FoundMarks? found;

  const MarkedCell(
    this.text,
    this.cell, {
    super.key,
    required this.boxes,
    required this.tags,
    required this.links,
    this.style,
    this.textAlign = TextAlign.start,
    this.ticking,
    this.found,
  });

  @override
  State<MarkedCell> createState() => _MarkedCellState();
}

class _MarkedCellState extends State<MarkedCell> with _InlineSpans {
  @override
  String get _source => widget.text;

  @override
  List<(int, int)> get _addresses => widget.links;

  @override
  BoxTicking? get _ticking => widget.ticking;

  @override
  FoundMarks? get _marks => widget.found;

  // A table's cell grows with the Device's text size too (#326, obf-1019).
  @override
  Widget build(BuildContext context) =>
      ContentText(child: Builder(builder: _build));

  Widget _build(BuildContext context) {
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

  /// The whole text the parts are of, its web addresses (#222), and how
  /// its boxes are ticked.
  String get _source;
  List<(int, int)> get _addresses;
  BoxTicking? get _ticking;

  /// The places found in [_source] (obf-1047).
  FoundMarks? get _marks;

  /// [style] as found where [start] to [end] of [_source] touches a
  /// place found, else as it is.
  TextStyle _foundOver(
    BuildContext context,
    int start,
    int end,
    TextStyle style,
  ) => switch (_marks?.at(start, end)) {
    final current? => foundStyle(context, style, current: current),
    null => style,
  };

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
        out.add(
          TextSpan(
            text: span.text,
            style: _foundOver(context, span.start, span.end, style),
            recognizer: link,
          ),
        );
      case MarkdownSpanKind.text:
        _literal(context, span.start, span.end, style, link, boxes, tags, out);
      case MarkdownSpanKind.code:
        out.add(
          TextSpan(
            text: span.text,
            style: _foundOver(
              context,
              span.start,
              span.end,
              codeStyle(scheme, style),
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
        // Another person may have written it: only the web, e-mail and
        // the app's own links (ADR 0046).
        if (!Markdown.mayOpen(span.url)) {
          inner(style);
          return;
        }
        final appLink = AppLink.parse(span.url);
        final address = appLink == null ? Uri.tryParse(span.url) : null;
        if (appLink == null && address == null) {
          inner(style);
          return;
        }
        final open = LinkOpenScope.of(context);
        final tap = TapGestureRecognizer()
          ..onTap = () => unawaited(switch ((appLink, address)) {
            (final AppLink link, _) => followAppLink(context, link),
            (_, final Uri uri) => _open(open, uri),
            _ => Future<void>.value(),
          });
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
  /// boxes, tags and web addresses among them.
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
    // A web address is never in a link; a box in one is none.
    final addresses = [
      for (final a in _addresses)
        if (a.$1 >= start && a.$2 <= end) a,
    ];
    final parts = <({int start, int end, List<InlineSpan> spans})>[
      for (final (from, to) in addresses)
        (
          start: from,
          end: to,
          spans: [
            _addressSpan(
              context,
              from,
              to,
              _foundOver(context, from, to, style),
            ),
          ],
        ),
      for (final box in boxes)
        if (box.start >= start &&
            box.end <= end &&
            !_inAddress(_addresses, box.start, box.end))
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
                _tagSpan(
                  context,
                  t,
                  tag,
                  _foundOver(
                    context,
                    tag.start,
                    tag.end,
                    tagStyle(context, style),
                  ),
                  onSearch,
                ),
              ],
            ),
    ]..sort((a, b) => a.start.compareTo(b.start));
    var at = start;
    final marks = _marks;
    for (final part in parts) {
      if (part.start > at) {
        out.addAll(
          foundSpans(
            context,
            text,
            at,
            part.start,
            marks,
            style: style,
            recognizer: link,
          ),
        );
      }
      out.addAll(part.spans);
      at = part.end;
    }
    if (end > at) {
      out.addAll(
        foundSpans(
          context,
          text,
          at,
          end,
          marks,
          style: style,
          recognizer: link,
        ),
      );
    }
  }

  /// The bare web address [start] to [end] of [_source] (#222), drawn in
  /// [style] as a link of a formatted text is and opened as one with a
  /// tap ([LinkOpenScope]); text alone where [Markdown.mayOpen] or the
  /// parsing of it says no.
  InlineSpan _addressSpan(
    BuildContext context,
    int start,
    int end,
    TextStyle? style,
  ) {
    final written = _source.substring(start, end);
    final address = Markdown.mayOpen(written) ? Uri.tryParse(written) : null;
    if (address == null) return TextSpan(text: written, style: style);
    final open = LinkOpenScope.of(context);
    final tap = TapGestureRecognizer()
      ..onTap = () => unawaited(_open(open, address));
    _links.add(tap);
    return TextSpan(
      text: written,
      style: (style ?? const TextStyle()).copyWith(
        color: Theme.of(context).colorScheme.primary,
        decoration: TextDecoration.combine([
          ?style?.decoration,
          TextDecoration.underline,
        ]),
      ),
      recognizer: tap,
    );
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
    // As tapped while its save waits (#338).
    final state = ticking?.stateOf(box.id, box.state) ?? box.state;
    final glyph = state.glyph;
    final label = boxStateLabel(t, state);
    final ticked = ticking?.labelOf(context, box.id);
    if (ticking == null || (!ticking.mayTick && ticked == null)) {
      return [
        TextSpan(text: glyph, semanticsLabel: label, style: _glyphStyle(style)),
      ];
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
              state: state,
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
        TextSpan(text: glyph, style: _glyphStyle()),
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
      child: Text(glyph, style: _glyphStyle(style)),
    ),
  );
}

/// [style] for a box's ✅ or ❌, with the emoji font by its name as well,
/// where the text's own font has neither and nothing falls back on its
/// own (the screenshots), as for the Frog.
TextStyle _glyphStyle([TextStyle? style]) => (style ?? const TextStyle())
    .copyWith(fontFamilyFallback: const ['Noto Color Emoji']);
