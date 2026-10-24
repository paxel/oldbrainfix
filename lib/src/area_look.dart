import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'l10n.dart';
import 'place_names.dart';

/// The colours offered for an Area, as stored (`RRGGBB`).
const areaPalette = [
  'C62828',
  'EF6C00',
  'F9A825',
  '2E7D32',
  '00838F',
  '1565C0',
  '6A1B9A',
  '8D6E63',
  '546E7A',
];

/// An Area's stored colour as a Flutter colour, or null when it has none
/// or it cannot be read.
Color? areaColour(String? hex) {
  if (hex == null) return null;
  final value = int.tryParse(hex, radix: 16);
  return value == null ? null : Color(0xFF000000 | value);
}

/// How this app draws each of the core's [areaIcons].
IconData? areaIconData(String? key) => switch (key) {
  'home' => Icons.home,
  'work' => Icons.work,
  'car' => Icons.directions_car,
  'health' => Icons.favorite,
  'school' => Icons.school,
  'kids' => Icons.child_care,
  'kitchen' => Icons.kitchen,
  'garden' => Icons.yard,
  'shopping' => Icons.shopping_cart,
  'money' => Icons.savings,
  'pets' => Icons.pets,
  'sport' => Icons.fitness_center,
  'travel' => Icons.flight,
  'star' => Icons.star,
  _ => null,
};

/// Picks an Area's colour and icon; every tap applies at once.
class AreaLookScreen extends StatefulWidget {
  final Places places;
  final String areaId;

  const AreaLookScreen({super.key, required this.places, required this.areaId});

  @override
  State<AreaLookScreen> createState() => _AreaLookScreenState();
}

class _AreaLookScreenState extends State<AreaLookScreen> {
  late Area? _area = widget.places.areaById(widget.areaId);

  void _colour(String? hex) {
    widget.places.setColour(widget.areaId, hex);
    setState(() => _area = widget.places.areaById(widget.areaId));
  }

  void _icon(String? key) {
    widget.places.setIcon(widget.areaId, key);
    setState(() => _area = widget.places.areaById(widget.areaId));
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final area = _area;
    if (area == null) return Scaffold(appBar: AppBar());
    final scheme = Theme.of(context).colorScheme;
    final heading = Theme.of(context).textTheme.titleMedium;
    final shown = areaColour(area.colour);
    return Scaffold(
      appBar: AppBar(title: Text(areaName(t, area))),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          Text(t.colourHeading, style: heading),
          const SizedBox(height: 8),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              _Choice(
                selected: area.colour == null,
                label: t.noColour,
                onTap: () => _colour(null),
                child: const Icon(Icons.block),
              ),
              for (final hex in areaPalette)
                _Choice(
                  selected: area.colour == hex,
                  onTap: () => _colour(hex),
                  child: CircleAvatar(backgroundColor: areaColour(hex)),
                ),
            ],
          ),
          const SizedBox(height: 24),
          Text(t.iconHeading, style: heading),
          const SizedBox(height: 8),
          Wrap(
            spacing: 8,
            runSpacing: 8,
            children: [
              _Choice(
                selected: area.icon == null,
                label: t.noIcon,
                onTap: () => _icon(null),
                child: const Icon(Icons.block),
              ),
              for (final key in areaIcons)
                _Choice(
                  selected: area.icon == key,
                  onTap: () => _icon(key),
                  child: Icon(
                    areaIconData(key),
                    color: shown ?? scheme.onSurface,
                  ),
                ),
            ],
          ),
        ],
      ),
    );
  }
}

/// One tappable choice, ringed in the accent while selected.
class _Choice extends StatelessWidget {
  final bool selected;
  final String? label;
  final VoidCallback onTap;
  final Widget child;

  const _Choice({
    required this.selected,
    required this.onTap,
    required this.child,
    this.label,
  });

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Semantics(
      button: true,
      selected: selected,
      label: label,
      child: InkWell(
        customBorder: const CircleBorder(),
        onTap: onTap,
        child: Container(
          width: 52,
          height: 52,
          alignment: Alignment.center,
          decoration: BoxDecoration(
            color: scheme.surface,
            shape: BoxShape.circle,
            border: Border.all(
              color: selected ? scheme.primary : scheme.outlineVariant,
              width: selected ? 3 : 1,
            ),
          ),
          child: child,
        ),
      ),
    );
  }
}
