import 'dart:math';
import 'dart:typed_data';

import 'package:crypto/crypto.dart';

import 'notes.dart';
import 'places.dart';
import 'routines.dart';
import 'shared_areas.dart';
import 'signing.dart';
import 'store.dart';

/// PBKDF2 with HMAC-SHA256 (RFC 8018): [length] bytes derived from
/// [password] and [salt] in [iterations] rounds.
Uint8List pbkdf2Sha256(
  List<int> password,
  List<int> salt,
  int iterations,
  int length,
) {
  if (iterations < 1) {
    throw ArgumentError.value(iterations, 'iterations', 'at least one');
  }
  final hmac = Hmac(sha256, password);
  final out = Uint8List(length);
  var filled = 0;
  for (var block = 1; filled < length; block++) {
    var u = hmac.convert([
      ...salt,
      block >> 24 & 0xff,
      block >> 16 & 0xff,
      block >> 8 & 0xff,
      block & 0xff,
    ]).bytes;
    final t = Uint8List.fromList(u);
    for (var i = 1; i < iterations; i++) {
      u = hmac.convert(u).bytes;
      for (var j = 0; j < t.length; j++) {
        t[j] ^= u[j];
      }
    }
    final take = min(t.length, length - filled);
    out.setRange(filled, filled + take, t);
    filled += take;
  }
  return out;
}

/// The PIN behind the App Lock and the Area Locks, kept only as a salted
/// PBKDF2 hash: `pbkdf2-sha256$<iterations>$<salt hex>$<hash hex>`.
abstract final class PinHash {
  static const _scheme = 'pbkdf2-sha256';

  /// The rounds a new PIN is hashed with.
  static const iterations = 20000;

  /// The shortest and longest PIN.
  static const minLength = 4;
  static const maxLength = 12;

  static final _random = Random.secure();

  /// Whether [pin] can be a PIN: [minLength] to [maxLength] digits.
  static bool valid(String pin) =>
      pin.length >= minLength &&
      pin.length <= maxLength &&
      RegExp(r'^[0-9]+$').hasMatch(pin);

  /// [pin] hashed for keeping, under a fresh random salt unless [salt]
  /// is given.
  static String hash(
    String pin, {
    List<int>? salt,
    int iterations = PinHash.iterations,
  }) {
    final usedSalt = salt ?? List.generate(16, (_) => _random.nextInt(256));
    final derived = pbkdf2Sha256(pin.codeUnits, usedSalt, iterations, 32);
    return '$_scheme\$$iterations\$${hex(usedSalt)}\$${hex(derived)}';
  }

  /// Whether [pin] is the PIN [stored] was made from; false for anything
  /// [stored] that is not such a hash.
  static bool verify(String pin, String stored) {
    final parts = stored.split(r'$');
    if (parts.length != 4 || parts[0] != _scheme) return false;
    final iterations = int.tryParse(parts[1]);
    if (iterations == null || iterations < 1) return false;
    final List<int> salt;
    final List<int> expected;
    try {
      salt = fromHex(parts[2]);
      expected = fromHex(parts[3]);
    } on FormatException {
      return false;
    }
    if (expected.isEmpty) return false;
    final derived = pbkdf2Sha256(
      pin.codeUnits,
      salt,
      iterations,
      expected.length,
    );
    // Every byte is compared, so the time taken tells nothing.
    var difference = 0;
    for (var i = 0; i < expected.length; i++) {
      difference |= derived[i] ^ expected[i];
    }
    return difference == 0;
  }
}

/// The App Lock and the Area Locks of this Device, kept as local
/// settings and never synced (ADR 0008): whether the App Lock is on, the
/// PIN's hash, whether biometrics unlock too, how soon the app locks
/// again in the background, which Areas are locked and which of those
/// show their alarms in full.
extension LockSettings on Store {
  static const _appLock = 'appLock';
  static const _pin = 'lockPin';
  static const _biometrics = 'lockBiometrics';
  static const _relockMinutes = 'relockMinutes';
  static const _lockedAreas = 'lockedAreas';
  static const _fullAreas = 'lockedAreasFull';

  /// How long the app may stay in the background before it locks again,
  /// when nothing else is set.
  static const defaultRelock = Duration(minutes: 1);

  /// Whether the App Lock guards browsing; off until switched on, and
  /// only ever on with a PIN.
  bool get appLockOn => localSetting(_appLock) == 'true' && hasPin;

  set appLockOn(bool on) {
    if (on && !hasPin) throw StateError('no PIN set');
    setLocalSetting(_appLock, '$on');
  }

  /// Whether a PIN is set.
  bool get hasPin => (localSetting(_pin) ?? '').isNotEmpty;

  /// Sets [pin] as the PIN, kept as its hash only.
  void setPin(String pin, {int iterations = PinHash.iterations}) {
    if (!PinHash.valid(pin)) {
      throw ArgumentError.value('…', 'pin', 'not ${PinHash.minLength}+ digits');
    }
    setLocalSetting(_pin, PinHash.hash(pin, iterations: iterations));
  }

  /// Whether [pin] is the PIN set; false while none is.
  bool checkPin(String pin) {
    final stored = localSetting(_pin);
    return stored != null && PinHash.verify(pin, stored);
  }

  /// Whether biometrics or the screen lock unlock as well as the PIN.
  bool get biometricsOn => localSetting(_biometrics) == 'true';

  set biometricsOn(bool on) => setLocalSetting(_biometrics, '$on');

  /// How long the app may stay in the background before it locks again;
  /// zero locks at once.
  Duration get relockAfter {
    final minutes = int.tryParse(localSetting(_relockMinutes) ?? '');
    return minutes == null || minutes < 0
        ? defaultRelock
        : Duration(minutes: minutes);
  }

  set relockAfter(Duration after) {
    if (after.isNegative) {
      throw ArgumentError.value(after, 'after', 'must not be negative');
    }
    setLocalSetting(_relockMinutes, '${after.inMinutes}');
  }

  /// The Areas locked on this Device, by id: an Area moved to fresh ids
  /// when it was shared ([ShareKeys.movedTo]) stays locked as it goes on.
  Set<String> get lockedAreas => _followed(_ids(_lockedAreas));

  /// Whether the Area [areaId] is locked on this Device.
  bool isAreaLocked(String areaId) => lockedAreas.contains(areaId);

  /// Locks or unlocks the Area [areaId]. A newly locked Area shows its
  /// alarms Masked until told otherwise.
  void setAreaLocked(String areaId, bool locked) {
    if (locked && !hasPin) throw StateError('no PIN set');
    final ids = lockedAreas;
    locked ? ids.add(areaId) : ids.remove(areaId);
    _setIds(_lockedAreas, ids);
    if (!locked) setAreaMasked(areaId, true);
  }

  /// Whether the locked Area [areaId] shows its alarms and Agenda rows
  /// Masked rather than in full; Masked unless set otherwise.
  bool isAreaMasked(String areaId) =>
      !_followed(_ids(_fullAreas)).contains(areaId);

  void setAreaMasked(String areaId, bool masked) {
    final ids = _ids(_fullAreas);
    masked ? ids.remove(areaId) : ids.add(areaId);
    _setIds(_fullAreas, ids);
  }

  Set<String> _ids(String key) => {
    for (final id in (localSetting(key) ?? '').split(','))
      if (id.isNotEmpty) id,
  };

  /// [ids] with every Area each of them was moved on to.
  Set<String> _followed(Set<String> ids) {
    final all = {...ids};
    for (final id in ids) {
      var next = current(id, ShareKeys.movedTo);
      while (next != null && all.add(next)) {
        next = current(next, ShareKeys.movedTo);
      }
    }
    return all;
  }

  void _setIds(String key, Set<String> ids) =>
      setLocalSetting(key, (ids.toList()..sort()).join(','));
}

/// What the Area Locks of this Device mean for Notes and Occurrences.
class AreaLocks {
  final Store store;

  AreaLocks(this.store);

  /// The Area [note] is filed in, directly or through a Topic; null for
  /// an Unsorted Note or one whose place is gone.
  String? areaOf(Note note) {
    final place = note.place;
    if (place == null) return null;
    final places = Places(store);
    final topic = places.topicById(place);
    if (topic != null) return topic.areaId;
    return places.areaById(place)?.id;
  }

  /// The locked Area [note] is filed in; null when it is in none.
  String? lockedAreaOf(Note note) {
    final area = areaOf(note);
    return area != null && store.isAreaLocked(area) ? area : null;
  }

  /// Whether [note] stays out of every list, search and card: it is
  /// filed in a locked Area that is not among the [open] ones.
  bool hides(Note note, Set<String> open) {
    final area = lockedAreaOf(note);
    return area != null && !open.contains(area);
  }

  /// Whether [routine] stays out of every list, as its Note does.
  bool hidesRoutine(Routine routine, Set<String> open) {
    final note = Routines(store).note(routine);
    return note != null && hides(note, open);
  }

  /// Whether [routine] shows Masked wherever its text would leave the
  /// browser (alarms, the Agenda, the widget, the calendar): its Note is
  /// filed in a locked Area set to Masked. Whether that Area is open
  /// right now does not count.
  bool masks(Routine routine) {
    final note = Routines(store).note(routine);
    if (note == null) return false;
    final area = lockedAreaOf(note);
    return area != null && store.isAreaMasked(area);
  }
}
