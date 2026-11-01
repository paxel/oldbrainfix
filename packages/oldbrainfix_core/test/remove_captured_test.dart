import 'dart:io';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'backup_test.dart' show cheap, passphrase;
import 'remove_media_test.dart' show syncAll;
import 'shared_areas_test.dart' show exchange, ticking;

void main() {
  late DateTime Function() clock;
  late Store ann;
  late Notes notes;

  setUp(() {
    clock = ticking();
    ann = Store.inMemory(clock: clock)..applyProfileDefaults(name: 'Ann');
    notes = Notes(ann);
  });
  tearDown(() => ann.close());

  /// A Note captured at Work, shared in from a chat, during a meeting and
  /// with a headset connected, filed into [place] when given.
  String captured(Store store, {String? place, String text = 'Order toner'}) {
    final notes = Notes(store);
    final id = notes
        .capture(
          text,
          place: place,
          sourceApp: const SourceApp('org.example.chat', label: 'Chat'),
        )
        .id;
    notes.locate(id, Coordinates(52.52, 13.405));
    notes.attachContext(
      id,
      calendarEvents: ['Katzenklo'],
      bluetoothDevices: ['Headset'],
    );
    return id;
  }

  group('removing a captured detail', () {
    test('each goes on its own; the time and the rest stay', () {
      NamedPlaces(ann).add('Work', Coordinates(52.52, 13.405), radius: 100);
      final id = captured(ann);
      final at = notes.byId(id)!.captured;
      expect(notes.byId(id)!.context.namedPlace?.name, 'Work');
      for (final detail in CapturedDetail.values) {
        expect(notes.mayRemoveCaptured(id, detail), isTrue, reason: '$detail');
      }

      notes.removeCaptured(id, CapturedDetail.event);
      var context = notes.byId(id)!.context;
      expect(context.calendarEvents, isEmpty);
      expect(context.bluetoothDevices, ['Headset']);
      expect(context.namedPlace?.name, 'Work');
      expect(context.sourceApp?.name, 'Chat');

      notes.removeCaptured(id, CapturedDetail.place);
      context = notes.byId(id)!.context;
      expect(context.location, isNull);
      expect(context.namedPlace, isNull);
      expect(context.bluetoothDevices, ['Headset']);
      // The Named Place itself stays for every other Note.
      expect(NamedPlaces(ann).all().single.name, 'Work');

      notes.removeCaptured(id, CapturedDetail.device);
      notes.removeCaptured(id, CapturedDetail.app);
      final note = notes.byId(id)!;
      expect(note.context.bluetoothDevices, isEmpty);
      expect(note.context.sourceApp, isNull);
      expect(note.context.hasMore, isFalse);
      expect(note.captured, at);
      expect(note.text, 'Order toner');
      // An edit like any other: nulls in the log, the label with the app.
      expect(
        [for (final e in ann.history(id).reversed.take(5)) (e.field, e.value)],
        [
          (NoteKeys.sourceLabel, null),
          (NoteKeys.sourcePackage, null),
          (NoteKeys.bluetooth, null),
          (NoteKeys.location, null),
          (NoteKeys.calendar, null),
        ],
      );
    });

    test('what does not show is not removed', () {
      final id = notes.capture('Typed').id;
      final before = ann.allEntries().length;
      for (final detail in CapturedDetail.values) {
        expect(notes.mayRemoveCaptured(id, detail), isFalse);
        expect(() => notes.removeCaptured(id, detail), throwsStateError);
      }
      expect(ann.allEntries().length, before);
      // Removed once, it is gone; garbled, it never showed.
      final twice = captured(ann);
      notes.removeCaptured(twice, CapturedDetail.place);
      expect(notes.mayRemoveCaptured(twice, CapturedDetail.place), isFalse);
      ann.append(id, NoteKeys.location, 'somewhere');
      ann.append(id, NoteKeys.calendar, '[]');
      expect(notes.mayRemoveCaptured(id, CapturedDetail.place), isFalse);
      expect(notes.mayRemoveCaptured(id, CapturedDetail.event), isFalse);
      expect(
        () => notes.removeCaptured('note:none', CapturedDetail.place),
        throwsArgumentError,
      );
      final area = Places(ann).addArea('Work').id;
      expect(
        () => notes.removeCaptured(area, CapturedDetail.place),
        throwsArgumentError,
      );
    });

    test('an Amend and a merged Note lose theirs as Sections', () {
      final host = notes.capture('Toner');
      final amend = notes.amend(host.id, 'Ordered');
      notes.locate(amend.id, Coordinates(52.52, 13.405));
      notes.attachContext(amend.id, calendarEvents: ['Standup']);
      final merged = captured(ann);
      notes.merge(merged, host.id);
      expect(notes.mayRemoveCaptured(amend.id, CapturedDetail.event), isTrue);
      notes.removeCaptured(amend.id, CapturedDetail.event);
      notes.removeCaptured(merged, CapturedDetail.device);
      final sections = {for (final s in notes.sections(host.id)) s.id: s};
      expect(sections[amend.id]!.context.calendarEvents, isEmpty);
      expect(sections[amend.id]!.context.location, isNotNull);
      expect(sections[merged]!.context.bluetoothDevices, isEmpty);
      expect(sections[merged]!.context.calendarEvents, ['Katzenklo']);
    });

    test('the removal reaches the user\'s other Device', () {
      final desk = Store.inMemory(clock: clock)
        ..applyProfileDefaults(name: 'Ann');
      addTearDown(desk.close);
      final id = captured(ann);
      syncAll(desk, ann);
      notes.removeCaptured(id, CapturedDetail.place);
      Notes(desk).removeCaptured(id, CapturedDetail.event);
      syncAll(desk, ann);
      for (final store in [ann, desk]) {
        final context = Notes(store).byId(id)!.context;
        expect(context.location, isNull);
        expect(context.calendarEvents, isEmpty);
        expect(context.bluetoothDevices, ['Headset']);
      }
    });
  });

  group('in a Shared Area', () {
    late Store ben;
    late String area;

    setUp(() {
      ben = Store.inMemory(clock: clock)..applyProfileDefaults(name: 'Ben');
      area = ann.share(Places(ann).addArea('Office').id);
      exchange(ben, ann, inviting: Invite(area, role: AreaRole.member));
    });
    tearDown(() => ben.close());

    test('a Member removes from their own Note, a Manager from anyone\'s', () {
      final own = captured(ben, place: area, text: 'Ben\'s toner');
      syncAll(ben, ann);
      expect(Notes(ben).mayRemoveCaptured(own, CapturedDetail.place), isTrue);
      Notes(ben).removeCaptured(own, CapturedDetail.place);
      expect(notes.mayRemoveCaptured(own, CapturedDetail.event), isTrue);
      notes.removeCaptured(own, CapturedDetail.event);
      syncAll(ben, ann);
      for (final store in [ann, ben]) {
        final context = Notes(store).byId(own)!.context;
        expect(context.location, isNull);
        expect(context.calendarEvents, isEmpty);
        expect(context.bluetoothDevices, ['Headset']);
      }
    });

    test('a Member\'s removal on another\'s Note is refused and, written '
        'all the same, ignored everywhere', () {
      final id = captured(ann, place: area);
      syncAll(ben, ann);
      final mark = ben.currentSeq();
      for (final detail in CapturedDetail.values) {
        expect(Notes(ben).mayRemoveCaptured(id, detail), isFalse);
        expect(() => Notes(ben).removeCaptured(id, detail), throwsStateError);
      }
      expect(ben.entriesAfter(mark), isEmpty);
      // As a modified app would write them.
      ben.append(id, NoteKeys.location, null);
      ben.append(id, NoteKeys.calendar, null);
      ben.append(id, NoteKeys.bluetooth, null);
      ben.append(id, NoteKeys.sourcePackage, null);
      syncAll(ben, ann);
      for (final store in [ann, ben]) {
        final context = Notes(store).byId(id)!.context;
        expect(context.location, isNotNull);
        expect(context.calendarEvents, ['Katzenklo']);
        expect(context.bluetoothDevices, ['Headset']);
        expect(context.sourceApp?.name, 'Chat');
      }
    });
  });

  group('what carries it', () {
    late Directory dir;

    setUp(() => dir = Directory.systemTemp.createTempSync('captured'));
    tearDown(() => dir.deleteSync(recursive: true));

    test('a Bundle carries the removal to a paired Device', () async {
      final phone = Store.inMemory(clock: clock)
        ..applyProfileDefaults(name: 'Ann');
      addTearDown(phone.close);
      final id = captured(ann);
      exchange(phone, ann);
      notes.removeCaptured(id, CapturedDetail.device);
      final path = '${dir.path}/b.$bundleExtension';
      expect(await writeBundle(ann, path), isNotNull);
      await importBundle(phone, path);
      final context = Notes(phone).byId(id)!.context;
      expect(context.bluetoothDevices, isEmpty);
      expect(context.calendarEvents, ['Katzenklo']);
    });

    test('a Backup restores the removal', () async {
      final old = Store.open('${dir.path}/old.db', clock: clock);
      addTearDown(old.close);
      final id = captured(old);
      Notes(old).removeCaptured(id, CapturedDetail.place);
      final code = await setUpBackups(old, passphrase, kdf: cheap);
      final path = '${dir.path}/b.$backupExtension';
      await writeBackup(old, path);
      final key = await unlockBackup(
        readBackupHeader(path)!,
        recoveryCode: code,
      );
      final store = Store.open('${dir.path}/new.db', clock: clock);
      addTearDown(store.close);
      await restoreBackup(store, path, key!);
      final context = Notes(store).byId(id)!.context;
      expect(context.location, isNull);
      expect(context.bluetoothDevices, ['Headset']);
    });

    test('a Sent Item carries no Capture Context at all', () {
      final ben = Store.inMemory(clock: clock)
        ..applyProfileDefaults(name: 'Ben');
      addTearDown(ben.close);
      final id = captured(ann);
      notes.removeCaptured(id, CapturedDetail.event);
      final file = encodeSentItem(sentItemOf(ann, id), ann.signingKey);
      final got = acceptSentItem(ben, openSentItem(ben, file).item)!;
      final context = Notes(ben).byId(got)!.context;
      expect(context.location, isNull);
      expect(context.calendarEvents, isEmpty);
      expect(context.bluetoothDevices, isEmpty);
      expect(context.sourceApp, isNull);
    });
  });
}
