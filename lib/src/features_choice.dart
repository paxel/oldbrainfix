import 'package:flutter/material.dart';

import 'l10n.dart';
import 'look.dart';

/// Simple or Advanced (#197), as radio buttons under what it means: on
/// the first start's Starter Set page and in Settings. [advanced] is
/// the choice shown; [onChanged] gets the one picked.
class FeaturesChoice extends StatelessWidget {
  final bool advanced;
  final ValueChanged<bool> onChanged;

  const FeaturesChoice({
    super.key,
    required this.advanced,
    required this.onChanged,
  });

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(24, 0, 24, 0),
          child: Text(t.featuresDetail),
        ),
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16),
          child: RadioColumn<bool>(
            key: const ValueKey('features'),
            value: advanced,
            options: [
              RadioOption(
                key: const ValueKey('features-simple'),
                value: false,
                title: Text(t.featuresSimple),
                subtitle: Text(t.featuresSimpleDetail),
              ),
              RadioOption(
                key: const ValueKey('features-advanced'),
                value: true,
                title: Text(t.featuresAdvanced),
                subtitle: Text(t.featuresAdvancedDetail),
              ),
            ],
            onChanged: onChanged,
          ),
        ),
      ],
    );
  }
}
