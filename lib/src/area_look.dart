import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'l10n.dart';
import 'look.dart';
import 'page_bar.dart';
import 'place_names.dart';
import 'system_bars.dart';

/// The colours offered for an Area, as stored (`RRGGBB`): today's nine,
/// then lighter and darker shades (obf-1156). A colour once offered
/// stays, so every stored look still reads.
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
  'F06292',
  'FF8A65',
  'AED581',
  '4FC3F7',
  'BA68C8',
  'AD1457',
  '9E9D24',
  '1B5E20',
  '283593',
];

/// How far an Area's colour stands out from the ground, the question
/// marks and the cards it is drawn on, at least (WCAG, as for a graphic
/// or an icon).
const areaContrast = 3.0;

/// An Area's stored colour as this app draws it in [brightness], or null
/// when it has none or it cannot be read: darkened on the light look and
/// brightened on the dark one, in twentieths toward black or white, just
/// as far as it takes to stand out [areaContrast] from every surface of
/// that look (obf-1156). The desktop's `area_colour` draws it alike.
Color? areaColour(String? hex, Brightness brightness) {
  if (hex == null) return null;
  final value = int.tryParse(hex, radix: 16);
  if (value == null) return null;
  final dark = brightness == Brightness.dark;
  final tones = LookTones.of(brightness);
  final surfaces = [tones.background, tones.mark, tones.card];
  final toward = dark ? 255 : 0;
  final channels = [(value >> 16) & 0xFF, (value >> 8) & 0xFF, value & 0xFF];
  for (var step = 0; step <= 20; step++) {
    final t = step / 20;
    final [r, g, b] = [
      for (final c in channels) (c + (toward - c) * t).round(),
    ];
    final mixed = Color.fromARGB(255, r, g, b);
    if (surfaces.every((s) => _contrast(mixed, s) >= areaContrast)) {
      return mixed;
    }
  }
  return Color.fromARGB(255, toward, toward, toward);
}

/// The WCAG contrast of two opaque colours.
double _contrast(Color a, Color b) {
  final [high, low] = [_luminance(a), _luminance(b)]
    ..sort((x, y) => y.compareTo(x));
  return (high + 0.05) / (low + 0.05);
}

/// The WCAG relative luminance of an opaque colour.
double _luminance(Color colour) {
  double linear(double c) {
    final v = (c * 255).round() / 255;
    return v <= 0.04045
        ? v / 12.92
        : math.pow((v + 0.055) / 1.055, 2.4).toDouble();
  }

  return 0.2126 * linear(colour.r) +
      0.7152 * linear(colour.g) +
      0.0722 * linear(colour.b);
}

/// How this app draws each of the core's [areaIcons]: Material icons
/// Flutter has long had, the desktop drawing the same glyphs.
IconData? areaIconData(String? key) => switch (key) {
  'home' => Icons.home,
  'kitchen' => Icons.kitchen,
  'garden' => Icons.yard,
  'kids' => Icons.child_care,
  'pets' => Icons.pets,
  'cleaning' => Icons.cleaning_services,
  'laundry' => Icons.local_laundry_service,
  'bed' => Icons.bed,
  'repairs' => Icons.build,
  'plants' => Icons.local_florist,
  'family' => Icons.family_restroom,
  'work' => Icons.work,
  'school' => Icons.school,
  'computer' => Icons.computer,
  'meetings' => Icons.groups,
  'mail' => Icons.mail,
  'book' => Icons.menu_book,
  'science' => Icons.science,
  'idea' => Icons.lightbulb,
  'code' => Icons.code,
  'health' => Icons.favorite,
  'sport' => Icons.fitness_center,
  'doctor' => Icons.local_hospital,
  'medication' => Icons.medication,
  'food' => Icons.restaurant,
  'running' => Icons.directions_run,
  'cycling' => Icons.directions_bike,
  'relax' => Icons.spa,
  'music' => Icons.music_note,
  'photo' => Icons.photo_camera,
  'movie' => Icons.movie,
  'games' => Icons.sports_esports,
  'art' => Icons.palette,
  'crafts' => Icons.brush,
  'hiking' => Icons.hiking,
  'football' => Icons.sports_soccer,
  'car' => Icons.directions_car,
  'travel' => Icons.flight,
  'bus' => Icons.directions_bus,
  'train' => Icons.train,
  'boat' => Icons.directions_boat,
  'beach' => Icons.beach_access,
  'map' => Icons.map,
  'hotel' => Icons.hotel,
  'shopping' => Icons.shopping_cart,
  'money' => Icons.savings,
  'gift' => Icons.card_giftcard,
  'phone' => Icons.smartphone,
  'key' => Icons.vpn_key,
  'bank' => Icons.account_balance,
  'receipt' => Icons.receipt,
  'document' => Icons.description,
  'star' => Icons.star,
  'brain' => Icons.psychology,
  'flag' => Icons.flag,
  'event' => Icons.event,
  'party' => Icons.celebration,
  'coffee' => Icons.local_cafe,
  'bolt' => Icons.bolt,
  'sun' => Icons.wb_sunny,
  _ => null,
};

/// A Topic's icon in its Area's colour (obf-1156), or null while it has
/// none: a Topic without one draws as before.
Widget? topicMark(
  BuildContext context,
  Area area,
  Topic topic, {
  double? size,
}) {
  final icon = areaIconData(topic.icon);
  if (icon == null) return null;
  final colour =
      areaColour(area.colour, Theme.of(context).brightness) ??
      Theme.of(context).colorScheme.outline;
  return Icon(icon, size: size, color: colour);
}

/// Picks an Area's colour and icon, or a Topic's icon, shown in its
/// Area's colour (obf-1156); every tap applies at once.
class AreaLookScreen extends StatefulWidget {
  final Places places;

  /// The Area or the Topic.
  final String placeId;

  const AreaLookScreen({
    super.key,
    required this.places,
    required this.placeId,
  });

  @override
  State<AreaLookScreen> createState() => _AreaLookScreenState();
}

class _AreaLookScreenState extends State<AreaLookScreen> {
  late Topic? _topic = widget.places.topicById(widget.placeId);

  /// The Area, or the Topic's Area.
  late Area? _area = widget.places.areaById(_topic?.areaId ?? widget.placeId);

  void _reload() => setState(() {
    final topic = _topic = widget.places.topicById(widget.placeId);
    _area = widget.places.areaById(topic?.areaId ?? widget.placeId);
  });

  void _colour(String? hex) {
    widget.places.setColour(widget.placeId, hex);
    _reload();
  }

  void _icon(String? key) {
    if (_topic != null) {
      widget.places.setTopicIcon(widget.placeId, key);
    } else {
      widget.places.setIcon(widget.placeId, key);
    }
    _reload();
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final area = _area;
    if (area == null) return Scaffold(appBar: AppBar());
    final topic = _topic;
    final scheme = Theme.of(context).colorScheme;
    final brightness = Theme.of(context).brightness;
    final heading = Theme.of(context).textTheme.titleMedium;
    final shown = areaColour(area.colour, brightness);
    final picked = topic == null ? area.icon : topic.icon;
    Widget choices(List<Widget> children) =>
        Wrap(spacing: 8, runSpacing: 8, children: children);
    return Scaffold(
      appBar: pageBar(
        context,
        title: topic == null ? areaName(t, area) : topicName(t, topic),
      ),
      body: ListView(
        padding: clearOfSystemBars(context, const EdgeInsets.all(16)),
        children: [
          // A Topic shows in its Area's colour: it has none of its own.
          if (topic == null) ...[
            Text(t.colourHeading, style: heading),
            const SizedBox(height: 8),
            choices([
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
                  child: CircleAvatar(
                    backgroundColor: areaColour(hex, brightness),
                  ),
                ),
            ]),
            const SizedBox(height: 24),
          ],
          Text(t.iconHeading, style: heading),
          const SizedBox(height: 8),
          // Each group of the core's on its own lines (obf-1156).
          for (final (i, group) in areaIconGroups.indexed) ...[
            if (i > 0) const SizedBox(height: 16),
            choices([
              if (i == 0)
                _Choice(
                  selected: picked == null,
                  label: t.noIcon,
                  onTap: () => _icon(null),
                  child: const Icon(Icons.block),
                ),
              for (final key in group)
                _Choice(
                  selected: picked == key,
                  onTap: () => _icon(key),
                  child: Icon(
                    areaIconData(key),
                    color: shown ?? scheme.onSurface,
                  ),
                ),
            ]),
          ],
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
