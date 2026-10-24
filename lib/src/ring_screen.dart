import 'package:flutter/material.dart';

import 'alarm_platform.dart';
import 'alarm_sync.dart';
import 'l10n.dart';
import 'look.dart';

/// The full-screen alarm of an escalated Occurrence (adapted from Chaos
/// Alert's alarm screen): its Note's first line and its time, a big Done
/// and the snooze choices (10 min, 30 min, 1 h). It shows over the lock
/// screen while the alarm rings, offers no way into the rest of the app,
/// and closes when it is answered or stops ringing.
class RingScreen extends StatefulWidget {
  final AlarmSync alarms;
  final Ringing ring;

  const RingScreen({super.key, required this.alarms, required this.ring});

  @override
  State<RingScreen> createState() => _RingScreenState();
}

class _RingScreenState extends State<RingScreen> {
  /// Answered once; a second tap changes nothing.
  var _answered = false;

  Future<void> _answer({Duration? snooze}) async {
    if (_answered) return;
    setState(() => _answered = true);
    await widget.alarms.answer(widget.ring, snooze: snooze);
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final text = Theme.of(context).textTheme;
    final ring = widget.ring;
    return PopScope(
      // Answered with Done or Snooze, not swiped away while it rings.
      canPop: false,
      child: Scaffold(
        body: SafeArea(
          child: Center(
            child: SingleChildScrollView(
              padding: const EdgeInsets.all(24),
              child: Embossed(
                strong: true,
                child: Padding(
                  padding: const EdgeInsets.all(24),
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      const Icon(Icons.alarm, size: 64),
                      const SizedBox(height: 16),
                      Text(
                        ring.title,
                        textAlign: TextAlign.center,
                        style: text.headlineMedium,
                      ),
                      const SizedBox(height: 8),
                      Text(
                        ring.body,
                        textAlign: TextAlign.center,
                        style: text.titleLarge,
                      ),
                      const SizedBox(height: 32),
                      SizedBox(
                        width: double.infinity,
                        child: FilledButton.icon(
                          key: const ValueKey('ringDone'),
                          onPressed: _answered ? null : _answer,
                          icon: const Icon(Icons.check),
                          label: Padding(
                            padding: const EdgeInsets.all(12),
                            child: Text(t.markDone, style: text.titleLarge),
                          ),
                        ),
                      ),
                      const SizedBox(height: 24),
                      Text(t.snoozeQuestion),
                      const SizedBox(height: 8),
                      Wrap(
                        spacing: 8,
                        runSpacing: 8,
                        alignment: WrapAlignment.center,
                        children: [
                          for (final (minutes, label) in [
                            (10, t.snooze10),
                            (30, t.snooze30),
                            (60, t.snooze60),
                          ])
                            OutlinedButton(
                              onPressed: _answered
                                  ? null
                                  : () => _answer(
                                      snooze: Duration(minutes: minutes),
                                    ),
                              child: Text(label),
                            ),
                        ],
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
