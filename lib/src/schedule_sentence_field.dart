import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'l10n.dart';

/// The line at the top of the Routine editor's schedule (#227, ADR
/// 0044): the schedule as a sentence in the app's language. Typing in it
/// hands each schedule it reads to [onRead] as it is typed, with the
/// next words as suggestions under it; the words it does not understand
/// are marked and change nothing. Left, or when a chip changes the
/// schedule, it shows the schedule again. On
/// an Advanced schedule ([schedule] null) it shows [summary], read-only,
/// and says to switch to Simple to type.
class ScheduleSentenceField extends StatefulWidget {
  /// The schedule as edited; null on an Advanced schedule.
  final SimpleSchedule? schedule;

  /// What it shows on an Advanced schedule.
  final String summary;

  /// The day the sentence is written and read on.
  final Day today;

  final ValueChanged<SimpleSchedule> onRead;

  const ScheduleSentenceField({
    super.key,
    required this.schedule,
    required this.summary,
    required this.today,
    required this.onRead,
  });

  @override
  State<ScheduleSentenceField> createState() => _ScheduleSentenceFieldState();
}

class _ScheduleSentenceFieldState extends State<ScheduleSentenceField> {
  final _controller = _SentenceController();
  final _focus = FocusNode();

  /// The schedule the text is of.
  SimpleSchedule? _shown;

  String get _language => Localizations.localeOf(context).languageCode;

  @override
  void initState() {
    super.initState();
    _focus.addListener(_focused);
    _controller.addListener(_moved);
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _show();
  }

  @override
  void didUpdateWidget(ScheduleSentenceField old) {
    super.didUpdateWidget(old);
    _show();
  }

  @override
  void dispose() {
    _controller.dispose();
    _focus.dispose();
    super.dispose();
  }

  /// The schedule as a sentence, unless the text being typed is of it;
  /// a chip changed while typing writes it anew.
  void _show() {
    final schedule = widget.schedule;
    if (schedule != null && _focus.hasFocus && schedule == _shown) return;
    _shown = schedule;
    final text = schedule == null
        ? widget.summary
        : scheduleSentence(schedule, _language, widget.today);
    _controller.unknown = const [];
    if (_controller.text != text) {
      _controller.value = TextEditingValue(
        text: text,
        selection: TextSelection.collapsed(offset: text.length),
      );
    }
  }

  void _focused() => setState(() {
    if (!_focus.hasFocus) _show();
  });

  // The suggestions follow the cursor.
  void _moved() => setState(() {});

  void _typed(String text) {
    final read = readSchedule(text, _language, widget.today);
    _controller.unknown = read.unknown;
    final schedule = read.schedule;
    if (schedule != null) _shown = schedule;
    if (schedule != null && schedule != widget.schedule) {
      widget.onRead(schedule);
    } else {
      setState(() {});
    }
  }

  void _pick(SentenceSuggestion suggestion) {
    final text = _controller.text;
    final after = text.substring(suggestion.end);
    final space = after.startsWith(' ') ? '' : ' ';
    final typed = text.replaceRange(
      suggestion.start,
      suggestion.end,
      '${suggestion.text}$space',
    );
    _controller.value = TextEditingValue(
      text: typed,
      selection: TextSelection.collapsed(
        offset: suggestion.start + suggestion.text.length + 1,
      ),
    );
    _typed(typed);
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final simple = widget.schedule != null;
    final selection = _controller.selection;
    final suggestions = simple && _focus.hasFocus && selection.isCollapsed
        ? suggestSchedule(_controller.text, selection.baseOffset, _language)
        : const <SentenceSuggestion>[];
    return TextFieldTapRegion(
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          TextField(
            key: const ValueKey('scheduleSentence'),
            controller: _controller,
            focusNode: _focus,
            readOnly: !simple,
            autocorrect: false,
            enableSuggestions: false,
            textCapitalization: TextCapitalization.sentences,
            maxLines: null,
            textInputAction: TextInputAction.done,
            onChanged: _typed,
            decoration: InputDecoration(
              labelText: t.scheduleSentenceLabel,
              hintText: t.scheduleSentenceHint,
              helperText: !simple
                  ? t.scheduleSentenceAdvanced
                  : _controller.unknown.isEmpty
                  ? null
                  : t.scheduleSentenceUnknown,
              helperMaxLines: 2,
            ),
          ),
          if (suggestions.isNotEmpty)
            Padding(
              padding: const EdgeInsets.only(top: 8),
              child: Wrap(
                key: const ValueKey('scheduleSuggestions'),
                spacing: 8,
                runSpacing: 4,
                children: [
                  for (final suggestion in suggestions)
                    ActionChip(
                      label: Text(suggestion.text),
                      onPressed: () => _pick(suggestion),
                    ),
                ],
              ),
            ),
        ],
      ),
    );
  }
}

/// The sentence's text with the parts not understood marked: in the
/// error colour, with a wavy underline.
class _SentenceController extends TextEditingController {
  List<({int start, int end})> unknown = const [];

  @override
  TextSpan buildTextSpan({
    required BuildContext context,
    TextStyle? style,
    required bool withComposing,
  }) {
    final marked = [
      for (final r in unknown)
        if (r.end <= text.length) r,
    ];
    if (marked.isEmpty) {
      return super.buildTextSpan(
        context: context,
        style: style,
        withComposing: withComposing,
      );
    }
    final error = Theme.of(context).colorScheme.error;
    final wrong = TextStyle(
      color: error,
      decoration: TextDecoration.underline,
      decorationStyle: TextDecorationStyle.wavy,
      decorationColor: error,
    );
    var at = 0;
    return TextSpan(
      style: style,
      children: [
        for (final r in marked) ...[
          if (r.start > at) TextSpan(text: text.substring(at, r.start)),
          TextSpan(text: text.substring(r.start, at = r.end), style: wrong),
        ],
        if (at < text.length) TextSpan(text: text.substring(at)),
      ],
    );
  }
}
