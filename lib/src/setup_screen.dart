import 'package:flutter/material.dart';

import '../l10n/app_localizations.dart';
import 'alarm_platform.dart';
import 'alarm_sync.dart';
import 'l10n.dart';

/// The name of [permission] and why the app wants it.
(String, String) permissionText(
  AppLocalizations t,
  AlarmPermission permission,
) => switch (permission) {
  AlarmPermission.notifications => (
    t.permNotifications,
    t.permNotificationsWhy,
  ),
  AlarmPermission.exactAlarms => (t.permExactAlarms, t.permExactAlarmsWhy),
  AlarmPermission.fullScreen => (t.permFullScreen, t.permFullScreenWhy),
};

/// Walks through the access alarms need, one step at a time (ADR 0004,
/// adapted from Chaos Alert): once on this Device's first start, and
/// from the Agenda's red banner whenever one goes missing. Every step
/// can be passed by; alarms then fall back to what is allowed.
class SetupScreen extends StatefulWidget {
  final AlarmSync alarms;

  /// Called after the last step.
  final VoidCallback onDone;

  const SetupScreen({super.key, required this.alarms, required this.onDone});

  @override
  State<SetupScreen> createState() => _SetupScreenState();
}

class _SetupScreenState extends State<SetupScreen> {
  static const _steps = AlarmPermission.values;
  var _step = 0;

  void _next() {
    if (_step < _steps.length - 1) {
      setState(() => _step++);
    } else {
      widget.onDone();
    }
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final permission = _steps[_step];
    final (title, why) = permissionText(t, permission);
    final last = _step == _steps.length - 1;
    return Scaffold(
      appBar: AppBar(title: Text(t.setupTitle)),
      body: SafeArea(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Text(t.setupStep(_step + 1, _steps.length)),
              const SizedBox(height: 16),
              Text(title, style: Theme.of(context).textTheme.headlineSmall),
              const SizedBox(height: 8),
              Text(why),
              const SizedBox(height: 24),
              ListenableBuilder(
                listenable: widget.alarms,
                builder: (context, _) => widget.alarms.granted(permission)
                    ? Row(
                        children: [
                          Icon(
                            Icons.check_circle,
                            color: Theme.of(context).colorScheme.primary,
                          ),
                          const SizedBox(width: 8),
                          Text(t.setupGranted),
                        ],
                      )
                    : FilledButton(
                        onPressed: () => widget.alarms.request(permission),
                        child: Text(t.setupAllow),
                      ),
              ),
              const Spacer(),
              OutlinedButton(
                onPressed: _next,
                child: Text(last ? t.setupDone : t.setupNext),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
