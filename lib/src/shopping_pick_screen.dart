import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'app_lock.dart';
import 'area_look.dart';
import 'l10n.dart';
import 'look.dart';
import 'system_bars.dart';

/// The Shopping Lists a capture can go into (#124): each one this Device
/// may add Lines to, oldest capture first, but those a closed locked
/// Area keeps to itself.
List<Note> shoppingListsToAddTo(Store store, AppLock? lock) => [
  for (final note in ShoppingLists(store).addable())
    if (!(lock?.hides(note) ?? false)) note,
];

/// What a Shopping List is called where it is picked: the first line of
/// its text, "Shopping List" while that is blank.
String shoppingTitle(AppLocalizations t, Note note) =>
    switch (note.text.trim().split('\n').first.trim()) {
      '' => t.shoppingHeading,
      final title => title,
    };

/// Picks the Shopping List the lines of a capture go into; pops with
/// its Note's id.
class ShoppingPickScreen extends StatefulWidget {
  final Store store;

  /// The list picked before, marked in the list.
  final String? current;

  const ShoppingPickScreen({super.key, required this.store, this.current});

  @override
  State<ShoppingPickScreen> createState() => _ShoppingPickScreenState();
}

class _ShoppingPickScreenState extends State<ShoppingPickScreen> {
  List<Note> _lists = const [];
  var _loaded = false;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (_loaded) return;
    _lists = shoppingListsToAddTo(widget.store, LockScope.of(context));
    _loaded = true;
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final scheme = Theme.of(context).colorScheme;
    final places = Places(widget.store);
    Area? areaOf(Note note) => switch (note.place) {
      null => null,
      final place => places.areaById(places.topicById(place)?.areaId ?? place),
    };
    return Scaffold(
      appBar: AppBar(title: Text(t.addToShoppingList)),
      body: ListView(
        padding: clearOfSystemBars(
          context,
          const EdgeInsets.symmetric(vertical: 8),
        ),
        children: [
          for (final note in _lists)
            Padding(
              key: ValueKey('shopping-pick-${note.id}'),
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
              child: Embossed(
                // The row's ink shows on the card, not under it.
                child: Material(
                  type: MaterialType.transparency,
                  child: ListTile(
                    leading: Icon(
                      Icons.shopping_cart_outlined,
                      color: areaColour(areaOf(note)?.colour) ?? scheme.outline,
                    ),
                    title: Text(
                      shoppingTitle(t, note),
                      style: Theme.of(context).textTheme.titleMedium,
                    ),
                    trailing: note.id == widget.current
                        ? Icon(Icons.check, color: scheme.primary)
                        : null,
                    onTap: () => Navigator.of(context).pop(note.id),
                  ),
                ),
              ),
            ),
        ],
      ),
    );
  }
}
