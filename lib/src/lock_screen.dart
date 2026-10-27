import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'app_lock.dart';
import 'capture_fan.dart';
import 'l10n.dart';
import 'look.dart';
import 'system_bars.dart';

/// What stands in front of the Agenda while the App Lock is locked: the
/// PIN, a button for biometrics or the screen lock where they are on,
/// and "+" to capture, which fans out its ways as on the Agenda and
/// never needs unlocking (UI laws, "Capture").
class LockScreen extends StatefulWidget {
  final AppLock lock;

  /// Opens Capture the way picked in the fan of "+", as on the Agenda.
  final ValueChanged<CaptureWay> onCapture;

  const LockScreen({super.key, required this.lock, required this.onCapture});

  @override
  State<LockScreen> createState() => _LockScreenState();
}

class _LockScreenState extends State<LockScreen> {
  final _pin = TextEditingController();
  var _wrong = false;

  /// Where "+" sits, so the fan opens from it.
  final _plus = GlobalKey();

  @override
  void dispose() {
    _pin.dispose();
    super.dispose();
  }

  /// Fans out the ways to capture from "+" and opens the one picked.
  Future<void> _capture() async {
    final fan = captureFanFor(context, _plus);
    if (fan == null) return;
    final way = await Navigator.of(context).push(fan);
    if (way != null && mounted) widget.onCapture(way);
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
          padding: clearOfSystemBars(
            context,
            const EdgeInsets.fromLTRB(24, 24, 24, 160),
          ),
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
        key: _plus,
        heroTag: 'lockedCapture',
        tooltip: t.captureTooltip,
        onPressed: _capture,
        child: const Icon(Icons.add),
      ),
    );
  }
}
