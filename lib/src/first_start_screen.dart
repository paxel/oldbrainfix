import 'package:flutter/material.dart';

import 'l10n.dart';
import 'system_bars.dart';

/// The first start's first screen: new to Old Brain Fix, a Device of
/// one's own being added, or a Backup being restored, where [onRestore]
/// is given. Each row is an action, not a choice of value.
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
      appBar: AppBar(title: Text(t.firstStartTitle)),
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
          ListTile(
            key: const ValueKey('first-new'),
            leading: const Icon(Icons.auto_awesome_outlined),
            title: Text(t.firstStartNew),
            subtitle: Text(t.firstStartNewDetail),
            onTap: onNew,
          ),
          ListTile(
            key: const ValueKey('first-device'),
            leading: const Icon(Icons.devices_outlined),
            title: Text(t.firstStartDevice),
            subtitle: Text(t.firstStartDeviceDetail),
            onTap: onAddDevice,
          ),
          if (onRestore case final restore?)
            ListTile(
              key: const ValueKey('restore-offer'),
              leading: const Icon(Icons.settings_backup_restore),
              title: Text(t.restoreOffer),
              subtitle: Text(t.restoreOfferDetail),
              onTap: restore,
            ),
        ],
      ),
    );
  }
}
