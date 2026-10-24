import 'dart:convert';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

void main() {
  group('PBKDF2-HMAC-SHA256', () {
    test('matches the reference implementation', () {
      // python3: hashlib.pbkdf2_hmac('sha256', b'1234', b'salt', 1000)
      expect(
        hex(pbkdf2Sha256(utf8.encode('1234'), utf8.encode('salt'), 1000, 32)),
        'fef292d231f6e6f5e787b22db128b0e50d2e75d0c3d7f2989e77f7202646717b',
      );
      // python3: hashlib.pbkdf2_hmac('sha256', b'password', b'salt', 2, 32)
      expect(
        hex(pbkdf2Sha256(utf8.encode('password'), utf8.encode('salt'), 2, 32)),
        'ae4d0c95af6b46d32d0adff928f06dd02a303f8ef3c251dfd6e2d85a95474c43',
      );
    });

    test('derives more than one block when asked', () {
      final long = pbkdf2Sha256([1, 2], [3], 3, 40);
      expect(long, hasLength(40));
      expect(long.sublist(0, 32), pbkdf2Sha256([1, 2], [3], 3, 32));
    });
  });

  group('PinHash', () {
    test('the PIN hashed verifies, another PIN does not', () {
      final stored = PinHash.hash('2468', iterations: 10);
      expect(PinHash.verify('2468', stored), isTrue);
      expect(PinHash.verify('2469', stored), isFalse);
      expect(PinHash.verify('', stored), isFalse);
    });

    test('keeps no trace of the PIN and salts every hash anew', () {
      final a = PinHash.hash('2468', iterations: 10);
      final b = PinHash.hash('2468', iterations: 10);
      expect(a, isNot(contains('2468')));
      expect(a, isNot(b));
      expect(a, startsWith(r'pbkdf2-sha256$10$'));
    });

    test('a given salt gives the same hash', () {
      expect(
        PinHash.hash('2468', salt: [1, 2, 3], iterations: 10),
        PinHash.hash('2468', salt: [1, 2, 3], iterations: 10),
      );
    });

    test('anything malformed verifies nothing', () {
      for (final stored in [
        '',
        'plain',
        r'md5$10$00$00',
        r'pbkdf2-sha256$x$00$00',
        r'pbkdf2-sha256$0$00$00',
        r'pbkdf2-sha256$10$zz$00',
        r'pbkdf2-sha256$10$00$',
      ]) {
        expect(PinHash.verify('2468', stored), isFalse, reason: stored);
      }
    });

    test('a PIN is four to twelve digits', () {
      expect(PinHash.valid('1234'), isTrue);
      expect(PinHash.valid('123456789012'), isTrue);
      expect(PinHash.valid('123'), isFalse);
      expect(PinHash.valid('1234567890123'), isFalse);
      expect(PinHash.valid('12a4'), isFalse);
      expect(PinHash.valid(' 1234'), isFalse);
    });
  });

  group('LockSettings', () {
    late Store store;

    setUp(() => store = Store.inMemory());
    tearDown(() => store.close());

    test('everything is unlocked until the Device says otherwise', () {
      expect(store.appLockOn, isFalse);
      expect(store.hasPin, isFalse);
      expect(store.biometricsOn, isFalse);
      expect(store.relockAfter, LockSettings.defaultRelock);
      expect(store.lockedAreas, isEmpty);
    });

    test('the PIN is kept only as its hash, as a local setting', () {
      store.setPin('2468', iterations: 10);
      expect(store.hasPin, isTrue);
      expect(store.checkPin('2468'), isTrue);
      expect(store.checkPin('1357'), isFalse);
      expect(store.localSetting('lockPin'), isNot(contains('2468')));
      expect(store.allEntries(), isEmpty, reason: 'settings are never logged');
    });

    test('no PIN checks out while none is set', () {
      expect(store.checkPin('2468'), isFalse);
    });

    test('a PIN that is not four to twelve digits is refused', () {
      expect(() => store.setPin('12'), throwsArgumentError);
      expect(store.hasPin, isFalse);
    });

    test('a lock goes on only with a PIN', () {
      expect(() => store.appLockOn = true, throwsStateError);
      expect(() => store.setAreaLocked('area:a', true), throwsStateError);
      store.setPin('2468', iterations: 10);
      store.appLockOn = true;
      store.setAreaLocked('area:a', true);
      expect(store.appLockOn, isTrue);
      expect(store.isAreaLocked('area:a'), isTrue);
    });

    test('re-locking can be immediate, never negative', () {
      store.relockAfter = Duration.zero;
      expect(store.relockAfter, Duration.zero);
      store.relockAfter = const Duration(minutes: 5);
      expect(store.relockAfter, const Duration(minutes: 5));
      expect(
        () => store.relockAfter = const Duration(minutes: -1),
        throwsArgumentError,
      );
    });

    test('a locked Area is Masked until set to full, and again after '
        'unlocking', () {
      store.setPin('2468', iterations: 10);
      store.setAreaLocked('area:a', true);
      store.setAreaLocked('area:b', true);
      expect(store.isAreaMasked('area:a'), isTrue);
      store.setAreaMasked('area:a', false);
      expect(store.isAreaMasked('area:a'), isFalse);
      expect(store.isAreaMasked('area:b'), isTrue);
      store.setAreaLocked('area:a', false);
      expect(store.lockedAreas, {'area:b'});
      store.setAreaLocked('area:a', true);
      expect(store.isAreaMasked('area:a'), isTrue);
    });
  });

  group('AreaLocks', () {
    late Store store;
    late Notes notes;
    late Places places;
    late Routines routines;
    late AreaLocks locks;
    late Area doctor;
    late Area home;
    late Topic pills;

    final schedule = Schedule(
      start: Day(2026, 10, 24),
      times: [ClockTime(8, 0)],
    );

    setUp(() {
      store = Store.inMemory(clock: () => DateTime.utc(2026, 10, 24, 6));
      notes = Notes(store);
      places = Places(store);
      routines = Routines(store);
      locks = AreaLocks(store);
      doctor = places.addArea('Doctor');
      home = places.addArea('Home');
      pills = places.addTopic(doctor.id, 'Pills');
      store.setPin('2468', iterations: 10);
      store.setAreaLocked(doctor.id, true);
    });
    tearDown(() => store.close());

    test('a Note in a locked Area, or in one of its Topics, is hidden '
        'until its Area is open', () {
      final inArea = notes.capture('Blood test', place: doctor.id);
      final inTopic = notes.capture('Ibuprofen', place: pills.id);
      final elsewhere = notes.capture('Milk', place: home.id);
      final unsorted = notes.capture('nees mor sauce');
      expect(locks.hides(inArea, {}), isTrue);
      expect(locks.hides(inTopic, {}), isTrue);
      expect(locks.hides(elsewhere, {}), isFalse);
      expect(locks.hides(unsorted, {}), isFalse);
      expect(locks.hides(inArea, {doctor.id}), isFalse);
      expect(locks.hides(inTopic, {doctor.id}), isFalse);
      expect(locks.areaOf(inTopic), doctor.id);
      expect(locks.areaOf(unsorted), isNull);
    });

    test('a Routine from a locked Area is Masked, open or not', () {
      final routine = routines.spawn(
        notes.capture('Pills', place: pills.id).id,
        schedule,
      );
      expect(locks.masks(routine), isTrue);
      expect(locks.hidesRoutine(routine, {}), isTrue);
      expect(locks.hidesRoutine(routine, {doctor.id}), isFalse);
    });

    test('a locked Area set to full shows its Routines in full but still '
        'hides them from lists', () {
      store.setAreaMasked(doctor.id, false);
      final routine = routines.spawn(
        notes.capture('Pills', place: doctor.id).id,
        schedule,
      );
      expect(locks.masks(routine), isFalse);
      expect(locks.hidesRoutine(routine, {}), isTrue);
    });

    test('Routines of Unsorted Notes and unlocked Areas are never Masked', () {
      final loose = routines.create('Water plants', schedule);
      final athome = routines.spawn(
        notes.capture('Bins', place: home.id).id,
        schedule,
      );
      expect(locks.masks(loose), isFalse);
      expect(locks.masks(athome), isFalse);
    });

    test('a Routine follows its Note through a Merge into a locked Area', () {
      final note = notes.capture('Pills');
      final routine = routines.spawn(note.id, schedule);
      expect(locks.masks(routine), isFalse);
      final target = notes.capture('Medication', place: doctor.id);
      notes.merge(note.id, target.id);
      expect(locks.masks(routines.byId(routine.id)!), isTrue);
    });

    test('unlocking the Area unmasks its Routines', () {
      final routine = routines.spawn(
        notes.capture('Pills', place: doctor.id).id,
        schedule,
      );
      store.setAreaLocked(doctor.id, false);
      expect(locks.masks(routine), isFalse);
      expect(locks.hidesRoutine(routine, {}), isFalse);
    });
  });
}
