import 'dart:async';

import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'alarm_sync.dart';
import 'app_lock.dart';
import 'calendar_mirror.dart';
import 'l10n.dart';
import 'look.dart';
import 'place_names.dart';
import 'system_bars.dart';

/// Makes sure a PIN is set, asking for a new one when none is; whether
/// one is set now.
Future<bool> _ensurePin(BuildContext context, Store store) async {
  if (store.hasPin) return true;
  final pin = await askNewPin(context);
  if (pin == null) return false;
  store.setPin(pin);
  return true;
}

/// The App Lock's part of Settings: the lock itself, biometrics or the
/// screen lock where the Device has them, the PIN, how soon the app
/// locks again, and the way to the Area Locks. Turning a lock off,
/// changing the PIN and showing a locked Area's alarms in full ask for
/// the PIN or biometrics first; turning one on asks for a PIN when none
/// is set.
class LockSettingsSection extends StatefulWidget {
  /// The re-lock times offered, in minutes; 0 locks at once.
  static const relockChoices = [0, 1, 5, 15];

  final AppLock lock;
  final AlarmSync alarms;
  final Places places;

  const LockSettingsSection({
    super.key,
    required this.lock,
    required this.alarms,
    required this.places,
  });

  @override
  State<LockSettingsSection> createState() => _LockSettingsSectionState();
}

class _LockSettingsSectionState extends State<LockSettingsSection> {
  late final _store = widget.lock.store;

  /// Whether the Device has biometrics or a screen lock; null until it
  /// told.
  bool? _canBiometrics;

  /// The settings as shown, read again after every change.
  late bool _appLock;
  late bool _biometrics;
  late bool _hasPin;
  late int _relock;

  void _read() {
    _appLock = _store.appLockOn;
    _biometrics = _store.biometricsOn;
    _hasPin = _store.hasPin;
    _relock = _store.relockAfter.inMinutes;
  }

  void _reread() {
    if (mounted) setState(_read);
  }

  @override
  void initState() {
    super.initState();
    _read();
    unawaited(_check());
  }

  Future<void> _check() async {
    final available = await widget.lock.platform.available();
    if (!mounted) return;
    setState(() => _canBiometrics = available);
  }

  Future<void> _setAppLock(bool on) async {
    final t = context.t;
    if (on) {
      if (!await _ensurePin(context, _store)) return;
      _store.appLockOn = true;
    } else {
      if (!await confirmOwner(context, widget.lock, title: t.appLockOff)) {
        return;
      }
      _store.appLockOn = false;
    }
    _reread();
  }

  Future<void> _setBiometrics(bool on) async {
    final t = context.t;
    if (on) {
      // A new way in costs the PIN first, so the Device's screen lock
      // alone never opens the locks; then they are tried once.
      if (!await confirmOwner(
            context,
            widget.lock,
            title: t.unlockBiometrics,
          ) ||
          !await widget.lock.biometrics(t.unlockBiometrics, anyway: true)) {
        return;
      }
    }
    _store.biometricsOn = on;
    _reread();
  }

  Future<void> _changePin() async {
    final t = context.t;
    if (!await confirmOwner(context, widget.lock, title: t.pinChange)) return;
    if (!mounted) return;
    final pin = await askNewPin(context);
    if (pin == null) return;
    _store.setPin(pin);
    _reread();
  }

  void _setRelock(int minutes) {
    _store.relockAfter = Duration(minutes: minutes);
    _reread();
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final relock = _relock;
    final relockChoices = {
      ...LockSettingsSection.relockChoices,
      relock,
    }.toList()..sort();
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 4, 16, 4),
      child: Embossed(
        child: ClipRRect(
          borderRadius: BorderRadius.circular(16),
          // The tiles' ink shows on the card, not under it.
          child: Material(
            type: MaterialType.transparency,
            child: Column(
              children: [
                SwitchListTile(
                  key: const ValueKey('appLock'),
                  secondary: const Icon(Icons.lock_outline),
                  value: _appLock,
                  onChanged: _setAppLock,
                  title: Text(t.appLock),
                  subtitle: Text(t.appLockDetail),
                ),
                if (_hasPin && _canBiometrics == true)
                  SwitchListTile(
                    key: const ValueKey('biometrics'),
                    secondary: const Icon(Icons.fingerprint),
                    value: _biometrics,
                    onChanged: _setBiometrics,
                    title: Text(t.unlockBiometrics),
                    subtitle: Text(t.biometricsDetail),
                  ),
                if (_hasPin)
                  ListTile(
                    key: const ValueKey('changePin'),
                    leading: const Icon(Icons.pin_outlined),
                    title: Text(t.pinChange),
                    onTap: _changePin,
                  ),
                ListTile(
                  leading: const Icon(Icons.timer_outlined),
                  title: Text(t.relockAfter),
                  subtitle: Text(t.relockDetail),
                  trailing: DropdownButton<int>(
                    key: const ValueKey('relock'),
                    value: relock,
                    onChanged: (minutes) {
                      if (minutes == null) return;
                      _setRelock(minutes);
                    },
                    items: [
                      for (final m in relockChoices)
                        DropdownMenuItem(
                          value: m,
                          child: Text(
                            m == 0 ? t.relockAtOnce : t.renagMinutes(m),
                          ),
                        ),
                    ],
                  ),
                ),
                ListTile(
                  key: const ValueKey('areaLocks'),
                  leading: const Icon(Icons.folder_off_outlined),
                  title: Text(t.areaLocksTitle),
                  subtitle: Text(t.areaLocksDetail),
                  onTap: () async {
                    await Navigator.of(context).push(
                      MaterialPageRoute<void>(
                        builder: (_) => AreaLocksScreen(
                          lock: widget.lock,
                          alarms: widget.alarms,
                          places: widget.places,
                        ),
                      ),
                    );
                    // A PIN may have been set there.
                    _reread();
                  },
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

/// Every Area with its lock on this Device, and, while it is locked,
/// whether its alarms and Agenda rows show in full or Masked. The
/// alarms, the Agenda widget and the calendar mirror take a change in
/// at once.
class AreaLocksScreen extends StatefulWidget {
  final AppLock lock;
  final AlarmSync alarms;
  final Places places;

  const AreaLocksScreen({
    super.key,
    required this.lock,
    required this.alarms,
    required this.places,
  });

  @override
  State<AreaLocksScreen> createState() => _AreaLocksScreenState();
}

class _AreaLocksScreenState extends State<AreaLocksScreen> {
  late final _store = widget.lock.store;
  List<Area> _areas = const [];
  Set<String> _locked = const {};
  Set<String> _masked = const {};
  var _loaded = false;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (_loaded) return;
    _loaded = true;
    final t = context.t;
    _areas = widget.places.areas()
      ..sort(
        (a, b) => areaName(
          t,
          a,
        ).toLowerCase().compareTo(areaName(t, b).toLowerCase()),
      );
    _read();
  }

  /// Which Areas are locked and which of those Masked, as shown.
  void _read() {
    _locked = _store.lockedAreas;
    _masked = {
      for (final id in _locked)
        if (_store.isAreaMasked(id)) id,
    };
  }

  /// What shows outside the app changed: armed anew, mirrored anew.
  void _changed() {
    setState(_read);
    unawaited(widget.alarms.refresh());
    final mirror = CalendarMirrorScope.of(context);
    if (mirror != null) unawaited(mirror.sync());
  }

  Future<void> _setLocked(Area area, bool locked) async {
    final t = context.t;
    if (locked) {
      if (!await _ensurePin(context, _store)) return;
    } else if (!await confirmOwner(
      context,
      widget.lock,
      title: t.areaLockOff(areaName(t, area)),
    )) {
      return;
    }
    _store.setAreaLocked(area.id, locked);
    if (mounted) _changed();
  }

  Future<void> _setMasked(Area area, bool masked) async {
    final t = context.t;
    if (!masked &&
        !await confirmOwner(
          context,
          widget.lock,
          title: t.areaShowFull(areaName(t, area)),
        )) {
      return;
    }
    _store.setAreaMasked(area.id, masked);
    if (mounted) _changed();
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final areas = _areas;
    return Scaffold(
      appBar: AppBar(title: Text(t.areaLocksTitle)),
      body: ListView(
        padding: clearOfSystemBars(
          context,
          const EdgeInsets.symmetric(vertical: 8),
        ),
        children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(24, 0, 24, 8),
            child: Text(t.areaLocksExplain),
          ),
          if (areas.isEmpty)
            Padding(
              padding: const EdgeInsets.all(24),
              child: Text(t.noAreasYet, textAlign: TextAlign.center),
            ),
          for (final area in areas)
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 6),
              child: Embossed(
                child: ClipRRect(
                  borderRadius: BorderRadius.circular(16),
                  child: Material(
                    type: MaterialType.transparency,
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        SwitchListTile(
                          key: ValueKey('areaLock-${area.id}'),
                          secondary: Icon(
                            _locked.contains(area.id)
                                ? Icons.lock_outline
                                : Icons.lock_open,
                          ),
                          title: Text(areaName(t, area)),
                          value: _locked.contains(area.id),
                          onChanged: (on) => _setLocked(area, on),
                        ),
                        if (_locked.contains(area.id))
                          Padding(
                            padding: const EdgeInsets.fromLTRB(16, 0, 16, 12),
                            child: Column(
                              crossAxisAlignment: CrossAxisAlignment.start,
                              children: [
                                Text(t.areaAlarmsShow),
                                const SizedBox(height: 8),
                                SegmentedButton<bool>(
                                  key: ValueKey('areaMasked-${area.id}'),
                                  segments: [
                                    ButtonSegment(
                                      value: true,
                                      label: Text(t.areaAlarmsMasked),
                                      icon: const Icon(
                                        Icons.visibility_off_outlined,
                                      ),
                                    ),
                                    ButtonSegment(
                                      value: false,
                                      label: Text(t.areaAlarmsFull),
                                      icon: const Icon(
                                        Icons.visibility_outlined,
                                      ),
                                    ),
                                  ],
                                  selected: {_masked.contains(area.id)},
                                  showSelectedIcon: false,
                                  onSelectionChanged: (picked) =>
                                      _setMasked(area, picked.single),
                                ),
                              ],
                            ),
                          ),
                      ],
                    ),
                  ),
                ),
              ),
            ),
        ],
      ),
    );
  }
}
