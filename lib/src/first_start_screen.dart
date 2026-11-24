import 'package:flutter/material.dart';

import 'l10n.dart';
import 'page_bar.dart';
import 'settings_search.dart';
import 'system_bars.dart';

/// The first start's first screen: new to Old Brain Fix, a Device of
/// one's own being added, or a Backup being restored, where [onRestore]
/// is given. Each row is an action, not a choice of value, on a card of
/// its own with a chevron, as tappable as the Import page's rows
/// (obf-1109).
class FirstStartScreen extends StatelessWidget {
  final VoidCallback onNew;
  final VoidCallback onAddDevice;

  /// Opens the restore from a Backup; none where it is not offered.
  final VoidCallback? onRestore;

  const FirstStartScreen({
    super.key,
    required this.onNew,
    required this.onAddDevice,
    this.onRestore,
  });

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return Scaffold(
      appBar: pageBar(context, title: t.firstStartTitle),
      body: ListView(
        padding: clearOfSystemBars(
          context,
          const EdgeInsets.symmetric(vertical: 8),
        ),
        children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 8, 16, 8),
            child: Text(t.firstStartIntro),
          ),
          _choice(
            key: const ValueKey('first-new'),
            icon: Icons.auto_awesome_outlined,
            title: t.firstStartNew,
            detail: t.firstStartNewDetail,
            onTap: onNew,
          ),
          _choice(
            key: const ValueKey('first-device'),
            icon: Icons.devices_outlined,
            title: t.firstStartDevice,
            detail: t.firstStartDeviceDetail,
            onTap: onAddDevice,
          ),
          if (onRestore case final restore?)
            _choice(
              key: const ValueKey('restore-offer'),
              icon: Icons.settings_backup_restore,
              title: t.restoreOffer,
              detail: t.restoreOfferDetail,
              onTap: restore,
            ),
        ],
      ),
    );
  }

  /// The card of one start choice: its [icon], [title] over [detail],
  /// and a chevron, the whole row taking the tap.
  static Widget _choice({
    required Key key,
    required IconData icon,
    required String title,
    required String detail,
    required VoidCallback onTap,
  }) => SettingsSection(
    children: [
      ListTile(
        key: key,
        leading: Icon(icon),
        title: Text(title),
        subtitle: Text(detail),
        trailing: const Icon(Icons.chevron_right),
        onTap: onTap,
      ),
    ],
  );
}
