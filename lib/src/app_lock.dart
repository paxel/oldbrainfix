import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'l10n.dart';
import 'lock_platform.dart';
import 'schedule_text.dart';

/// The names of the routes that show while the app is locked: Capture
/// never needs unlocking, and neither does the full-screen alarm.
abstract final class LockFreeRoutes {
  static const capture = '/capture';
  static const ring = '/ring';

  static bool contains(Route<dynamic> route) =>
      route.settings.name == capture || route.settings.name == ring;
}

/// The App Lock and the Area Locks while the app runs (CONTEXT.md,
/// "Privacy"). The app starts locked when the App Lock is on, and locks
/// again once it was in the background for the Device's re-lock time;
/// an Area opened with the PIN or biometrics stays open until then, or
/// until the user is back on the Agenda. Which Areas are locked, the
/// PIN and the rest are this Device's settings (ADR 0008).
class AppLock extends ChangeNotifier {
  final Store store;
  final LockPlatform platform;

  late bool _locked = store.appLockOn;
  final _open = <String>{};
  DateTime? _hiddenAt;
  var _asking = 0;

  AppLock({required this.store, this.platform = const NoBiometrics()});

  /// Whether the App Lock stands in front of browsing now.
  bool get locked => _locked && store.appLockOn;

  /// Whether the locked Area [areaId] is closed now.
  bool isAreaClosed(String areaId) =>
      store.isAreaLocked(areaId) && !_open.contains(areaId);

  /// Whether [note] stays out of every list now.
  bool hides(Note note) => AreaLocks(store).hides(note, _open);

  /// Whether [routine] stays out of every list now, as its Note does.
  bool hidesRoutine(Routine routine) =>
      AreaLocks(store).hidesRoutine(routine, _open);

  /// Whether biometrics or the screen lock may unlock.
  bool get biometricsOn => store.biometricsOn;

  /// The PIN or biometrics were given at the App Lock.
  void unlock() {
    if (!_locked) return;
    _locked = false;
    notifyListeners();
  }

  /// Opens the locked Area [areaId] until the app locks again.
  void openArea(String areaId) {
    if (_open.add(areaId)) notifyListeners();
  }

  /// Closes every Area opened, as when the user is back on the Agenda.
  void closeAreas() {
    if (_open.isEmpty) return;
    _open.clear();
    notifyListeners();
  }

  /// The app went to the background. While the system asks for
  /// biometrics or the screen lock, it does not count.
  void hidden() {
    if (_asking > 0) return;
    _hiddenAt ??= store.clock();
  }

  /// The app is back: it locks again, and its Areas close, once it was
  /// away for the re-lock time.
  void shown() {
    final at = _hiddenAt;
    _hiddenAt = null;
    if (at == null || store.clock().difference(at) < store.relockAfter) {
      return;
    }
    final lock = store.appLockOn && !_locked;
    if (!lock && _open.isEmpty) return;
    _locked = _locked || lock;
    _open.clear();
    notifyListeners();
  }

  /// Asks the system for biometrics or the screen lock, when they are
  /// switched on or [anyway]; whether the user gave them.
  Future<bool> biometrics(String title, {bool anyway = false}) async {
    if (!store.biometricsOn && !anyway) return false;
    _asking++;
    try {
      return await platform.authenticate(title: title);
    } finally {
      _asking--;
    }
  }
}

/// Hands the app's [AppLock] down to every screen.
class LockScope extends InheritedWidget {
  final AppLock lock;

  const LockScope({super.key, required this.lock, required super.child});

  /// The app's [AppLock], or null where there is none; read once, so it
  /// does not listen.
  static AppLock? of(BuildContext context) =>
      context.getInheritedWidgetOfExactType<LockScope>()?.lock;

  @override
  bool updateShouldNotify(LockScope oldWidget) => lock != oldWidget.lock;
}

/// The text a Masked Occurrence of [routine] shows instead of its Note's:
/// "Private Reminder", or "Private Routine, 08:00" with the time of
/// [occurrence] when there is one.
String maskedText(
  AppLocalizations t,
  String locale,
  Routine routine, [
  Occurrence? occurrence,
]) {
  if (routine.isReminder) return t.maskedReminder;
  if (occurrence == null) return t.maskedRoutine;
  return t.maskedRoutineAt(occurrenceTime(t, locale, occurrence));
}

/// Makes sure it is the owner: biometrics or the screen lock when they
/// are on, else, or when they fail, the PIN. [title] says what for.
/// Whether they are; true at once while no PIN is set.
Future<bool> confirmOwner(
  BuildContext context,
  AppLock lock, {
  required String title,
}) async {
  if (!lock.store.hasPin) return true;
  if (await lock.biometrics(title)) return true;
  if (!context.mounted) return false;
  final ok = await showDialog<bool>(
    context: context,
    builder: (_) => _PinDialog(store: lock.store, title: title),
  );
  return ok ?? false;
}

/// Makes sure it is the owner before something only the owner may
/// see, such as another Device's recovery key (ADR 0016): this Device's
/// own unlock, the App Lock's PIN (or biometrics) when there is one,
/// else the phone's screen lock where it has one. Whether they are; true
/// at once on a Device with neither.
Future<bool> confirmUnlock(
  BuildContext context,
  AppLock? lock, {
  required String title,
}) async {
  if (lock == null) return true;
  if (lock.store.hasPin) return confirmOwner(context, lock, title: title);
  if (!await lock.platform.available()) return true;
  return lock.biometrics(title, anyway: true);
}

/// Opens the locked Area [areaId] after [confirmOwner]; true at once when
/// it is not closed or there is no lock.
Future<bool> openLockedArea(
  BuildContext context,
  AppLock? lock,
  String areaId,
  String areaName,
) async {
  if (lock == null || !lock.isAreaClosed(areaId)) return true;
  final ok = await confirmOwner(
    context,
    lock,
    title: context.t.unlockArea(areaName),
  );
  if (ok) lock.openArea(areaId);
  return ok;
}

/// Asks for the PIN; pops true once it is right, false when given up.
class _PinDialog extends StatefulWidget {
  final Store store;
  final String title;

  const _PinDialog({required this.store, required this.title});

  @override
  State<_PinDialog> createState() => _PinDialogState();
}

class _PinDialogState extends State<_PinDialog> {
  final _pin = TextEditingController();
  var _wrong = false;

  @override
  void dispose() {
    _pin.dispose();
    super.dispose();
  }

  void _check() {
    if (widget.store.checkPin(_pin.text)) {
      Navigator.of(context).pop(true);
      return;
    }
    _pin.clear();
    setState(() => _wrong = true);
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return AlertDialog(
      icon: const Icon(Icons.lock_outline),
      title: Text(widget.title),
      content: PinField(
        key: const ValueKey('pinField'),
        controller: _pin,
        label: t.pinLabel,
        error: _wrong ? t.pinWrong : null,
        onSubmitted: _check,
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(false),
          child: Text(t.cancel),
        ),
        TextButton(onPressed: _check, child: Text(t.unlock)),
      ],
    );
  }
}

/// A field for a PIN: digits only, hidden, with the number keyboard.
class PinField extends StatelessWidget {
  final TextEditingController controller;
  final String label;
  final String? error;
  final VoidCallback? onSubmitted;
  final bool autofocus;

  const PinField({
    super.key,
    required this.controller,
    required this.label,
    this.error,
    this.onSubmitted,
    this.autofocus = true,
  });

  @override
  Widget build(BuildContext context) => TextField(
    controller: controller,
    autofocus: autofocus,
    obscureText: true,
    keyboardType: TextInputType.number,
    maxLength: PinHash.maxLength,
    textInputAction: TextInputAction.done,
    onSubmitted: (_) => onSubmitted?.call(),
    decoration: InputDecoration(
      labelText: label,
      errorText: error,
      counterText: '',
    ),
  );
}

/// Asks for a new PIN twice; the PIN, or null when given up.
Future<String?> askNewPin(BuildContext context) =>
    showDialog<String>(context: context, builder: (_) => const _NewPinDialog());

class _NewPinDialog extends StatefulWidget {
  const _NewPinDialog();

  @override
  State<_NewPinDialog> createState() => _NewPinDialogState();
}

class _NewPinDialogState extends State<_NewPinDialog> {
  final _pin = TextEditingController();
  final _again = TextEditingController();
  String? _error;

  @override
  void dispose() {
    _pin.dispose();
    _again.dispose();
    super.dispose();
  }

  void _done() {
    final t = context.t;
    if (!PinHash.valid(_pin.text)) {
      setState(() => _error = t.pinTooShort(PinHash.minLength));
      return;
    }
    if (_pin.text != _again.text) {
      setState(() => _error = t.pinMismatch);
      return;
    }
    Navigator.of(context).pop(_pin.text);
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return AlertDialog(
      icon: const Icon(Icons.pin_outlined),
      title: Text(t.pinNewTitle),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(t.pinNewExplain),
          PinField(
            key: const ValueKey('newPin'),
            controller: _pin,
            label: t.pinLabel,
          ),
          PinField(
            key: const ValueKey('newPinAgain'),
            controller: _again,
            label: t.pinAgainLabel,
            error: _error,
            autofocus: false,
            onSubmitted: _done,
          ),
        ],
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
