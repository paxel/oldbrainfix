import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'announcements.dart';
import 'browser.dart';
import 'l10n.dart';
import 'location.dart';
import 'look.dart';
import 'note_card.dart';
import 'system_bars.dart';

/// The radii a Named Place can have, in metres.
const namedPlaceRadii = [50.0, 100.0, 250.0, 500.0, 1000.0];

/// The user's Named Places, by name. "+" names the spot the Device is at;
/// each one can be renamed, given another radius or deleted. Opens from
/// Settings.
class NamedPlacesScreen extends StatefulWidget {
  final NamedPlaces namedPlaces;

  const NamedPlacesScreen({super.key, required this.namedPlaces});

  @override
  State<NamedPlacesScreen> createState() => _NamedPlacesScreenState();
}

class _NamedPlacesScreenState extends State<NamedPlacesScreen> {
  late List<NamedPlace> _places = widget.namedPlaces.all();

  /// While "+" waits for the position.
  bool _locating = false;

  void _reload() => setState(() => _places = widget.namedPlaces.all());

  /// Names the spot the Device is at; asks for the location first, with
  /// its explanation, when it is not allowed yet.
  Future<void> _add() async {
    final location = LocationScope.of(context);
    final announcements = AnnouncementsScope.of(context);
    final t = context.t;
    if (!await location.granted()) {
      if (!mounted) return;
      if (!await explainAndAskLocation(context, location)) {
        announcements?.announce(t.locationNotAllowed);
        return;
      }
    }
    setState(() => _locating = true);
    final here = await location.current();
    if (!mounted) return;
    setState(() => _locating = false);
    if (here == null) {
      announcements?.announce(t.locationUnknown);
      return;
    }
    final picked = await showDialog<_Picked>(
      context: context,
      builder: (_) => _NamedPlaceDialog(title: t.newNamedPlace),
    );
    if (!mounted || picked == null) return;
    widget.namedPlaces.add(picked.name, here, radius: picked.radius);
    _reload();
  }

  Future<void> _act(NamedPlace place, _Action action) async {
    final t = context.t;
    switch (action) {
      case _Action.edit:
        final picked = await showDialog<_Picked>(
          context: context,
          builder: (_) => _NamedPlaceDialog(
            title: t.editNamedPlace,
            name: place.name,
            radius: place.radius,
          ),
        );
        if (!mounted || picked == null) return;
        if (picked.name != place.name) {
          widget.namedPlaces.rename(place.id, picked.name);
        }
        if (picked.radius != place.radius) {
          widget.namedPlaces.setRadius(place.id, picked.radius);
        }
      case _Action.delete:
        final sure = await showDialog<bool>(
          context: context,
          builder: (context) => AlertDialog(
            title: Text(t.deleteNamedPlaceQuestion(place.name)),
            content: Text(t.deleteNamedPlaceDetail),
            actions: [
              TextButton(
                onPressed: () => Navigator.of(context).pop(false),
                child: Text(t.cancel),
              ),
              TextButton(
                onPressed: () => Navigator.of(context).pop(true),
                child: Text(t.delete),
              ),
            ],
          ),
        );
        if (!mounted || sure != true) return;
        widget.namedPlaces.delete(place.id);
    }
    _reload();
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return Scaffold(
      appBar: AppBar(title: Text(t.namedPlacesTitle)),
      body: ListView(
        padding: clearOfSystemBars(
          context,
          const EdgeInsets.symmetric(vertical: 8),
        ),
        children: _places.isEmpty
            ? [BrowserEmpty(text: t.noNamedPlacesYet)]
            : [
                for (final place in _places)
                  Padding(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 16,
                      vertical: 6,
                    ),
                    child: Embossed(
                      child: ListTile(
                        leading: const Icon(Icons.place_outlined),
                        title: Text(place.name),
                        subtitle: Text(
                          '${t.radiusMetres(place.radius.round())} · '
                          '${coordinatesText(place.centre)}',
                        ),
                        trailing: PopupMenuButton<_Action>(
                          onSelected: (action) => _act(place, action),
                          itemBuilder: (_) => [
                            PopupMenuItem(
                              value: _Action.edit,
                              child: Text(t.editNamedPlace),
                            ),
                            PopupMenuItem(
                              value: _Action.delete,
                              child: Text(t.delete),
                            ),
                          ],
                        ),
                      ),
                    ),
                  ),
              ],
      ),
      floatingActionButton: FloatingActionButton(
        tooltip: t.newNamedPlace,
        onPressed: _locating ? null : _add,
        child: _locating
            ? const SizedBox.square(
                dimension: 24,
                child: CircularProgressIndicator(strokeWidth: 3),
              )
            : const Icon(Icons.add_location_alt_outlined),
      ),
    );
  }
}

enum _Action { edit, delete }

/// A name and a radius, as picked in [_NamedPlaceDialog].
typedef _Picked = ({String name, double radius});

/// Asks for the name and the radius of a Named Place.
class _NamedPlaceDialog extends StatefulWidget {
  final String title;
  final String name;
  final double radius;

  const _NamedPlaceDialog({
    required this.title,
    this.name = '',
    this.radius = 100,
  });

  @override
  State<_NamedPlaceDialog> createState() => _NamedPlaceDialogState();
}

class _NamedPlaceDialogState extends State<_NamedPlaceDialog> {
  late final _name = TextEditingController(text: widget.name);
  late double _radius = widget.radius;

  @override
  void dispose() {
    _name.dispose();
    super.dispose();
  }

  void _done() {
    final name = _name.text.trim();
    if (name.isEmpty) return;
    Navigator.of(context).pop<_Picked>((name: name, radius: _radius));
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return AlertDialog(
      title: Text(widget.title),
      content: SingleChildScrollView(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            TextField(
              controller: _name,
              autofocus: true,
              textCapitalization: TextCapitalization.sentences,
              decoration: InputDecoration(hintText: t.namedPlaceHint),
              onSubmitted: (_) => _done(),
            ),
            const SizedBox(height: 16),
            Text(
              t.radiusHeading,
              style: Theme.of(context).textTheme.titleSmall,
            ),
            const SizedBox(height: 8),
            RadioColumn<double>(
              value: _radius,
              options: [
                for (final radius in {...namedPlaceRadii, widget.radius})
                  RadioOption(
                    value: radius,
                    title: Text(t.radiusMetres(radius.round())),
                  ),
              ],
              onChanged: (radius) => setState(() => _radius = radius),
            ),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(t.cancel),
        ),
        TextButton(onPressed: _done, child: Text(t.save)),
      ],
    );
  }
}
