import 'package:flutter/material.dart';

import 'look.dart';

/// What the search field on top of Settings asks for (#282): the words
/// typed, lowercased, empty while nothing is typed. Every [SettingsRow]
/// and [SettingsGroup] under it shows only what matches; without one above them they show everything, as on the
/// Profile page and the first start.
class SettingsSearch extends InheritedWidget {
  final String query;

  const SettingsSearch({super.key, required this.query, required super.child});

  /// The query of the search above [context]; empty without one.
  static String of(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<SettingsSearch>()?.query ?? '';

  /// The query as the field gives it to [SettingsSearch]: trimmed and
  /// lowercased.
  static String queryOf(String typed) => typed.trim().toLowerCase();

  /// Whether [words] match [query]: each of its words is in one of them,
  /// in any case; everything matches an empty query.
  static bool matches(Iterable<String> words, String query) {
    if (query.isEmpty) return true;
    final text = words.join('\n').toLowerCase();
    return query
        .split(RegExp(r'\s+'))
        .every((word) => word.isEmpty || text.contains(word));
  }

  /// Whether a search above [context] shows any row (#282). Read after a
  /// frame, as the sections decide while they build which of their rows
  /// show; never while building.
  static bool showsAnyRow(BuildContext context) {
    var any = false;
    void visit(Element element) {
      if (any) return;
      if (element.widget is _ShownRow) {
        any = true;
        return;
      }
      element.visitChildren(visit);
    }

    context.visitChildElements(visit);
    return any;
  }

  @override
  bool updateShouldNotify(SettingsSearch oldWidget) => query != oldWidget.query;
}

/// One setting of Settings: [child] with the [words] the search finds
/// it by, its label and its detail line, and the words of its options
/// where it has no label of its own.
class SettingsRow extends StatelessWidget {
  final List<String> words;
  final Widget child;

  const SettingsRow({super.key, required this.words, required this.child});

  /// Whether the row shows for [query].
  bool matches(String query) => SettingsSearch.matches(words, query);

  @override
  Widget build(BuildContext context) => matches(SettingsSearch.of(context))
      ? _ShownRow(child: child)
      : const SizedBox.shrink();
}

/// A [SettingsRow] that shows, as [SettingsSearch.showsAnyRow] finds it.
class _ShownRow extends StatelessWidget {
  final Widget child;

  const _ShownRow({required this.child});

  @override
  Widget build(BuildContext context) => child;
}

/// A section of Settings: its [heading] and its [children], the rows
/// themselves or on a [SettingsSection]. While a search runs it shows
/// only its matching rows under the heading, all of them when the
/// heading matches, and nothing when none does; anything else among
/// [children], a gap or a line between, shows only while nothing is
/// searched. [headingKey] lets an app link scroll to it (#261).
class SettingsGroup extends StatelessWidget {
  final String? heading;
  final Key? headingKey;
  final List<Widget> children;

  const SettingsGroup({
    super.key,
    this.heading,
    this.headingKey,
    required this.children,
  });

  static bool _shows(Widget child, String query) => switch (child) {
    SettingsRow() => child.matches(query),
    SettingsSection() => child.children.any((row) => _shows(row, query)),
    _ => false,
  };

  @override
  Widget build(BuildContext context) {
    final query = SettingsSearch.of(context);
    final heading = this.heading;
    final whole =
        query.isEmpty ||
        (heading != null && SettingsSearch.matches([heading], query));
    final shown = whole
        ? children
        : [
            for (final child in children)
              if (_shows(child, query)) child,
          ];
    if (shown.isEmpty) return const SizedBox.shrink();
    final group = Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        if (heading != null) SettingsHeading(heading, key: headingKey),
        ...shown,
      ],
    );
    // A matching heading shows its section whole.
    return whole && query.isNotEmpty
        ? SettingsSearch(query: '', child: group)
        : group;
  }
}

/// A section title of Settings and the Profile page.
class SettingsHeading extends StatelessWidget {
  final String text;

  const SettingsHeading(this.text, {super.key});

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.fromLTRB(24, 16, 24, 4),
    child: Text(text, style: Theme.of(context).textTheme.titleMedium),
  );
}

/// The tiles of one section on an embossed card; a [SettingsGroup]
/// leaves the card out while a search finds none of its rows.
class SettingsSection extends StatelessWidget {
  final List<Widget> children;

  const SettingsSection({super.key, required this.children});

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.fromLTRB(16, 4, 16, 4),
    child: Embossed(
      child: ClipRRect(
        borderRadius: BorderRadius.circular(16),
        // The tiles' ink shows on the card, not under it.
        child: Material(
          type: MaterialType.transparency,
          child: Column(children: children),
        ),
      ),
    ),
  );
}
