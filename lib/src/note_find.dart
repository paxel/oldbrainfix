import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/scheduler.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'found_marks.dart';
import 'l10n.dart';
import 'text_marks_view.dart';

/// What the find bar of an opened Note finds (obf-1047), the words of a
/// search or none, and the place the Note opens at, one the search found
/// in it; at its first place without one.
typedef NoteFind = ({String query, Finding? at});

/// A place the find bar found in an opened Note (obf-1047): in its
/// [part] of the media [key] ('' for a text) of the Section [section] or,
/// null, of the Note itself. It is drawn from [start] to [end] of
/// [field]'s text where a field shows it, else of the text as it shows;
/// [shown] is where it starts in that part's text as it shows, as a
/// [Finding] counts it.
class NoteHit {
  final FindingPart part;
  final String key;
  final String? section;
  final TagTextController? field;
  final int start;
  final int end;
  final int shown;

  const NoteHit({
    required this.part,
    required this.key,
    required this.section,
    required this.field,
    required this.start,
    required this.end,
    required this.shown,
  });

  /// The places [query] is found in the text [field] holds, its boxes
  /// with their ids, as a text in [format] of the Section [section] or
  /// the Note; [shift] is where the field's text starts in the part's.
  static List<NoteHit> inField(
    TagTextController field,
    String format,
    String query, {
    String? section,
    int shift = 0,
  }) {
    final shown = TextBoxes.shown(field.text);
    return [
      for (final f in Findings.inText(
        shown.text,
        format,
        query,
        section: section,
      ))
        NoteHit(
          part: FindingPart.text,
          key: '',
          section: section,
          field: field,
          start: storedOffset(shown.boxes, f.start),
          end: storedOffset(shown.boxes, f.end),
          shown: f.start + shift,
        ),
    ];
  }

  /// The places [query] is found in [text], drawn as it shows: the [part]
  /// of [key] of the Section [section] or the Note.
  static List<NoteHit> inText(
    String text,
    String format,
    String query, {
    required FindingPart part,
    String key = '',
    String? section,
  }) => [
    for (final f in Findings.inText(
      text,
      format,
      query,
      part: part,
      key: key,
      section: section,
    ))
      NoteHit(
        part: part,
        key: key,
        section: section,
        field: null,
        start: f.start,
        end: f.end,
        shown: f.start,
      ),
  ];

  /// Which of [hits] is [at], a place a search found: the one in the same
  /// text nearest to it, else the first.
  static int indexOf(List<NoteHit> hits, Finding at) {
    var best = 0;
    int? distance;
    for (final (i, hit) in hits.indexed) {
      if (hit.part != at.part ||
          hit.key != at.key ||
          hit.section != at.section) {
        continue;
      }
      final d = (hit.shown - at.start).abs();
      if (distance == null || d < distance) {
        best = i;
        distance = d;
      }
    }
    return best;
  }
}

/// The find bar of an opened Note (obf-1047), under its top bar: the
/// words it finds, editable, how many places it found and which of them
/// is shown ("3 of 12"), ↑ and ↓ to the one before and after, and ✕ to
/// close it. Where the page is narrow the buttons go on a line under the
/// field, so nothing shrinks.
class FindBar extends StatelessWidget {
  final TextEditingController controller;
  final FocusNode focusNode;

  /// The place shown, counted from 0, and how many there are.
  final int at;
  final int count;
  final ValueChanged<String> onChanged;
  final VoidCallback onPrevious;
  final VoidCallback onNext;
  final VoidCallback onClose;

  const FindBar({
    super.key,
    required this.controller,
    required this.focusNode,
    required this.at,
    required this.count,
    required this.onChanged,
    required this.onPrevious,
    required this.onNext,
    required this.onClose,
  });

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final theme = Theme.of(context);
    final words = controller.text.trim();
    final counted = Text(
      words.isEmpty
          ? ''
          : count == 0
          ? t.findNone
          : t.findCount(at + 1, count),
      key: const ValueKey('find-count'),
      style: theme.textTheme.labelLarge,
    );
    final field = TextField(
      key: const ValueKey('find-field'),
      controller: controller,
      focusNode: focusNode,
      onChanged: onChanged,
      onSubmitted: (_) => onNext(),
      textInputAction: TextInputAction.search,
      decoration: InputDecoration(
        hintText: t.findInNote,
        prefixIcon: const Icon(Icons.search),
        isDense: true,
        border: OutlineInputBorder(borderRadius: BorderRadius.circular(16)),
      ),
    );
    final buttons = [
      IconButton(
        key: const ValueKey('find-previous'),
        tooltip: t.findPrevious,
        onPressed: count == 0 ? null : onPrevious,
        icon: const Icon(Icons.keyboard_arrow_up),
      ),
      IconButton(
        key: const ValueKey('find-next'),
        tooltip: t.findNext,
        onPressed: count == 0 ? null : onNext,
        icon: const Icon(Icons.keyboard_arrow_down),
      ),
      IconButton(
        key: const ValueKey('find-close'),
        tooltip: t.findClose,
        onPressed: onClose,
        icon: const Icon(Icons.close),
      ),
    ];
    return Material(
      key: const ValueKey('find-bar'),
      color: theme.colorScheme.surfaceContainer,
      child: DecoratedBox(
        decoration: BoxDecoration(
          border: Border(bottom: BorderSide(color: theme.dividerColor)),
        ),
        child: Padding(
          padding: const EdgeInsets.fromLTRB(16, 8, 8, 8),
          child: LayoutBuilder(
            builder: (context, constraints) {
              // The field keeps room for some words at the text's size.
              final room =
                  4 * 48 + MediaQuery.textScalerOf(context).scale(16) * 12;
              if (constraints.maxWidth >= room) {
                return Row(
                  children: [
                    Expanded(child: field),
                    const SizedBox(width: 8),
                    counted,
                    ...buttons,
                  ],
                );
              }
              return Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                mainAxisSize: MainAxisSize.min,
                children: [
                  Padding(
                    padding: const EdgeInsets.only(right: 8),
                    child: field,
                  ),
                  Row(
                    children: [
                      Expanded(child: counted),
                      ...buttons,
                    ],
                  ),
                ],
              );
            },
          ),
        ),
      ),
    );
  }
}

/// Scrolls the list of [scroll] until the current place found, drawn
/// below [root] on [foundStyle]'s stronger tint, shows (obf-1047): at
/// once where it is built, else a screen further [down] or up at a time
/// until it is; at that end of the list the other way from where it
/// started, and back there when it shows nowhere. Neither the cursor nor
/// the focus moves.
Future<void> revealFound(
  BuildContext root,
  ScrollController scroll, {
  required bool down,
}) async {
  final current = foundStyle(root, null, current: true).backgroundColor;
  final from = scroll.hasClients ? scroll.position.pixels : null;
  var way = down ? 1 : -1;
  var turned = false;
  for (var tries = 0; tries < 200; tries++) {
    if (!root.mounted) return;
    final box = root.findRenderObject();
    if (box == null) return;
    final shown = _currentIn(box, current);
    if (shown != null) {
      final (render, rect) = shown;
      render.showOnScreen(
        rect: Rect.fromLTRB(
          rect.left,
          rect.top - 48,
          rect.right,
          rect.bottom + 48,
        ),
        duration: const Duration(milliseconds: 200),
        curve: Curves.easeOut,
      );
      return;
    }
    if (!scroll.hasClients || from == null) return;
    final position = scroll.position;
    final next = (position.pixels + position.viewportDimension * 0.8 * way)
        .clamp(position.minScrollExtent, position.maxScrollExtent);
    if (next == position.pixels) {
      if (turned) {
        scroll.jumpTo(
          from.clamp(position.minScrollExtent, position.maxScrollExtent),
        );
        return;
      }
      turned = true;
      way = -way;
      continue;
    }
    scroll.jumpTo(next);
    await SchedulerBinding.instance.endOfFrame;
  }
}

/// The text drawn under [box] that holds a span on the [current] tint,
/// with where that span starts in it; null where none does.
(RenderBox, Rect)? _currentIn(RenderObject box, Color? current) {
  (RenderBox, Rect)? found;
  void visit(RenderObject child) {
    if (found != null) return;
    if (child is RenderParagraph || child is RenderEditable) {
      final text = switch (child) {
        final RenderParagraph p => p.text,
        final RenderEditable e => e.text,
        _ => null,
      };
      final at = text == null ? null : _offsetOf(text, current);
      if (at != null) {
        final position = TextPosition(offset: at);
        found = switch (child) {
          final RenderEditable e => (e, e.getLocalRectForCaret(position)),
          final RenderParagraph p => (
            p,
            p.getOffsetForCaret(position, Rect.zero) &
                Size(1, p.getFullHeightForCaret(position)),
          ),
          _ => null,
        };
        if (found != null) return;
      }
    }
    child.visitChildren(visit);
  }

  visit(box);
  return found;
}

/// Where the first span of [text] on the [current] tint starts, counting
/// each placeholder as one character; null where none is.
int? _offsetOf(InlineSpan text, Color? current) {
  if (current == null) return null;
  var at = 0;
  int? found;
  text.visitChildren((span) {
    if (span is TextSpan) {
      if (span.style?.backgroundColor == current && span.text != null) {
        found = at;
        return false;
      }
      at += span.text?.length ?? 0;
    } else if (span is PlaceholderSpan) {
      at += 1;
    }
    return true;
  });
  return found;
}
