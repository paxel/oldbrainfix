import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'app_lock.dart';
import 'l10n.dart';
import 'look.dart';

/// What stands in front of the Agenda while the App Lock is locked: the
/// PIN, a button for biometrics or the screen lock where they are on,
/// and "+" to capture, which never needs unlocking (UI laws, "Capture").
class LockScreen extends StatefulWidget {
  final AppLock lock;

  /// Opens Capture, as "+" on the Agenda does.
  final VoidCallback onCapture;

  const LockScreen({super.key, required this.lock, required this.onCapture});

  @override
  State<LockScreen> createState() => _LockScreenState();
}

class _LockScreenState extends State<LockScreen> {
  final _pin = TextEditingController();
  var _wrong = false;

  @override
  void dispose() {
    _pin.dispose();
    super.dispose();
  }

  void _check() {
    if (widget.lock.store.checkPin(_pin.text)) {
      _pin.clear();
      widget.lock.unlock();
      return;
    }
    _pin.clear();
    setState(() => _wrong = true);
  }

  Future<void> _biometrics() async {
    final t = context.t;
    if (await widget.lock.biometrics(t.unlockApp)) widget.lock.unlock();
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final text = Theme.of(context).textTheme;
    return Scaffold(
      appBar: AppBar(title: Text(t.appTitle)),
      body: Center(
        child: SingleChildScrollView(
          padding: const EdgeInsets.fromLTRB(24, 24, 24, 160),
          child: Embossed(
            child: Padding(
              padding: const EdgeInsets.all(24),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  const Icon(Icons.lock_outline, size: 48),
                  const SizedBox(height: 12),
                  Text(
                    t.lockedTitle,
                    textAlign: TextAlign.center,
                    style: text.titleLarge,
                  ),
                  const SizedBox(height: 8),
                  Text(t.lockedExplain, textAlign: TextAlign.center),
                  if (widget.lock.biometricsOn) ...[
                    const SizedBox(height: 24),
                    FilledButton.icon(
                      key: const ValueKey('unlockBiometrics'),
                      onPressed: _biometrics,
                      icon: const Icon(Icons.fingerprint),
                      label: Text(t.unlockBiometrics),
                    ),
                  ],
                  const SizedBox(height: 16),
                  PinField(
                    key: const ValueKey('lockPin'),
                    controller: _pin,
                    label: t.pinLabel,
                    error: _wrong ? t.pinWrong : null,
                    autofocus: !widget.lock.biometricsOn,
                    onSubmitted: _check,
                  ),
                  const SizedBox(height: 8),
                  OutlinedButton(
                    key: const ValueKey('unlockPin'),
                    onPressed: _check,
                    child: Text(t.unlock),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
      floatingActionButton: FloatingActionButton(
        heroTag: 'lockedCapture',
        tooltip: t.captureTooltip,
        onPressed: widget.onCapture,
        child: const Icon(Icons.add),
      ),
    );
  }
}
