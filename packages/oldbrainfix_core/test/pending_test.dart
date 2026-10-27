import 'dart:io';

import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'people_test.dart' show device;
import 'shared_areas_test.dart' show exchange, ticking;

void main() {
  late DateTime Function() clock;
  late Store ann;
  late Store bob;

  setUp(() {
    clock = ticking();
    ann = device(clock, 'Ann')..renameOwnProfile('Ann');
    bob = device(clock, 'Bob')..renameOwnProfile('Bob');
  });

  String text(String plain) => StoredText.plain(plain).encode();

  /// What the screens read of [store]: every Note's text, by id.
  Map<String, String> texts(Store store) => {
    for (final n in Notes(store).all()) n.id: n.text,
  };

  final pills = Schedule(
    start: Day(2026, 3, 1),
    times: [ClockTime(8, 0)],
    rule: Recurrence.parse('FREQ=DAILY'),
  );

  /// Bob holds Ann's log, so he can edit her Notes.
  void bobHoldsAnns() =>
      bob.applyEntries(ann.entriesSince({}), keys: ann.keyRecords());

  group('drafts', () {
    test('a shown set reads as stored, and the log sees none of it', () {
      final milk = Notes(ann).capture('Milk');
      final entries = ann.allEntries().length;
      final vector = ann.versionVector();
      final id = ann.openPending('ai-task', shown: true);
      ann.draftPending(id, milk.id, NoteKeys.text, text('Oat milk'));
      final made = Store.newId();
      ann.draftPending(id, 'note:$made', NoteKeys.type, NoteKeys.typeNote);
      ann.draftPending(
        id,
        'note:$made',
        NoteKeys.captured,
        clock().toIso8601String(),
      );
      ann.draftPending(id, 'note:$made', NoteKeys.text, text('Bread'));

      expect(Notes(ann).byId(milk.id)!.text, 'Oat milk');
      expect(Notes(ann).byId('note:$made')!.text, 'Bread');
      expect(ann.allEntries(), hasLength(entries));
      expect(ann.versionVector(), vector);
      expect(ann.entriesSince(vector), isEmpty);
      expect(ann.logged(milk.id, NoteKeys.text), text('Milk'));
    });

    test('a set not shown is read only through itself', () {
      final milk = Notes(ann).capture('Milk');
      final id = ann.openPending('sent-item', shown: false, payload: '{}');
      ann.draftPending(id, milk.id, NoteKeys.text, text('Oat milk'));

      expect(Notes(ann).byId(milk.id)!.text, 'Milk');
      final set = ann.pendingSet(id)!;
      expect(set.kind, 'sent-item');
      expect(set.payload, '{}');
      expect(set.drafts.single.value, text('Oat milk'));
    });

    test('Accept writes them numbered and signed then, dated when drafted, '
        'and reads as before', () {
      final milk = Notes(ann).capture('Milk');
      final routine = Routines(ann).spawn(milk.id, pills);
      final id = ann.openPending('ai-task', shown: true);
      ann.draftPending(id, milk.id, NoteKeys.text, text('Oat milk'));
      ann.draftPending(id, routine.id, RoutineKeys.renag, '15');
      final drafted = ann.pendingSet(id)!.drafts;
      final before = texts(ann);
      final renag = ann.currentFields(routine.id);
      final own = ann.versionVector()[ann.deviceId]!;

      final accepted = ann.acceptPending(id);

      expect(accepted.kept, isEmpty);
      expect(accepted.taken, isEmpty);
      expect(accepted.written.map((e) => e.dseq), [own + 1, own + 2]);
      for (final (i, e) in accepted.written.indexed) {
        expect(e.device, ann.deviceId);
        expect(e.date, drafted[i].made);
        expect(e.sig, isNotNull);
      }
      expect(texts(ann), before);
      expect(ann.currentFields(routine.id), renag);
      expect(ann.pendingSet(id), isNull);
      expect(ann.pendingSets, isEmpty);
      // What the log now holds verifies on another Device.
      bobHoldsAnns();
      expect(Notes(bob).byId(milk.id)!.text, 'Oat milk');
    });

    test('Decline leaves the log as it was', () {
      final milk = Notes(ann).capture('Milk');
      final entries = [for (final e in ann.allEntries()) e.wire];
      final id = ann.openPending('ai-task', shown: true);
      ann.draftPending(id, milk.id, NoteKeys.text, text('Oat milk'));

      expect(ann.declinePending(id), isTrue);

      expect([for (final e in ann.allEntries()) e.wire], entries);
      expect(Notes(ann).byId(milk.id)!.text, 'Milk');
      expect(ann.pendingSets, isEmpty);
      expect(ann.declinePending(id), isFalse);
    });

    test('own edits made meanwhile win, a whole schedule too, and the '
        'review lists them', () {
      final milk = Notes(ann).capture('Milk');
      final bread = Notes(ann).capture('Bread');
      final routine = Routines(ann).spawn(milk.id, pills);
      final id = ann.openPending('ai-task', shown: true);
      ann.draftPending(id, milk.id, NoteKeys.text, text('Oat milk'));
      ann.draftPending(id, routine.id, RoutineKeys.rule, 'FREQ=WEEKLY');
      // The user edits the Note and the Routine's start, then the AI
      // edits the other Note, after the user did.
      Notes(ann).edit(milk.id, 'Milk, two litres');
      Notes(ann).edit(bread.id, 'Brown bread');
      ann.append(routine.id, RoutineKeys.start, '2026-03-02');
      ann.draftPending(id, bread.id, NoteKeys.text, text('Rye bread'));

      final accepted = ann.acceptPending(id);

      expect(accepted.kept, [
        KeptEdit(
          milk.id,
          NoteKeys.text,
          pending: text('Oat milk'),
          kept: text('Milk, two litres'),
        ),
        KeptEdit(
          routine.id,
          RoutineKeys.rule,
          pending: 'FREQ=WEEKLY',
          kept: 'FREQ=DAILY',
        ),
      ]);
      expect(accepted.written.map((e) => e.entity), [bread.id]);
      expect(Notes(ann).byId(milk.id)!.text, 'Milk, two litres');
      expect(Notes(ann).byId(bread.id)!.text, 'Rye bread');
      expect(ann.current(routine.id, RoutineKeys.rule), 'FREQ=DAILY');
    });

    test('the reads show what Accept keeps: not a rule the user changed the '
        'schedule after', () {
      final milk = Notes(ann).capture('Milk');
      final routine = Routines(ann).spawn(milk.id, pills);
      final id = ann.openPending('ai-task', shown: true);
      ann.draftPending(id, routine.id, RoutineKeys.rule, 'FREQ=WEEKLY');
      expect(ann.current(routine.id, RoutineKeys.rule), 'FREQ=WEEKLY');
      ann.append(routine.id, RoutineKeys.start, '2026-03-02');
      final shown = ann.currentFields(routine.id);

      expect(shown[RoutineKeys.rule], 'FREQ=DAILY');
      ann.acceptPending(id);
      expect(ann.currentFields(routine.id), shown);
    });

    test('an edit another person wrote meanwhile does not stop a draft', () {
      final milk = Notes(ann).capture('Milk');
      bobHoldsAnns();
      final id = ann.openPending('ai-task', shown: true);
      ann.draftPending(id, milk.id, NoteKeys.text, text('Oat milk'));
      Notes(bob).edit(milk.id, 'Milk from Bob');
      ann.applyEntries(bob.entriesSince({}), keys: bob.keyRecords());

      final accepted = ann.acceptPending(id);

      expect(accepted.kept, isEmpty);
      expect(accepted.written.single.field, NoteKeys.text);
    });
  });

  group('a restart', () {
    late Directory dir;

    setUp(() => dir = Directory.systemTemp.createTempSync('pending'));
    tearDown(() => dir.deleteSync(recursive: true));

    Store open() {
      final store = Store.open('${dir.path}/log.db', clock: clock);
      store.setLocalSetting('author', 'Ann');
      return store;
    }

    test('finds the set waiting, as it was', () {
      final store = open();
      final milk = Notes(store).capture('Milk');
      final id = store.openPending('ai-task', shown: true);
      store.draftPending(id, milk.id, NoteKeys.text, text('Oat milk'));
      final file = File('${dir.path}/pending/$id.json');
      final written = file.readAsStringSync();
      store.close();

      final again = open();
      expect(again.pendingSet(id)!.encode(), written);
      expect(Notes(again).byId(milk.id)!.text, 'Oat milk');
      again.acceptPending(id);
      expect(file.existsSync(), isFalse);
      expect(again.logged(milk.id, NoteKeys.text), text('Oat milk'));
      again.close();
    });

    test('after an Accept cut short before its file went, finds none', () {
      final store = open();
      final id = store.openPending('ai-task', shown: true);
      store.draftPending(id, 'note:x', NoteKeys.text, text('Lost?'));
      // What an Accept writes with its entries, before the file goes.
      store.setLocalSetting('${Store.acceptedPendingPrefix}$id', 'accepted');
      store.close();

      final again = open();
      expect(again.pendingSets, isEmpty);
      expect(File('${dir.path}/pending/$id.json').existsSync(), isFalse);
      expect(again.localSetting('${Store.acceptedPendingPrefix}$id'), isNull);
      again.close();
    });

    test('a second store on the same file, as the background sync, sees '
        'what the other did to a set', () {
      final app = open();
      final milk = Notes(app).capture('Milk');
      final id = app.openPending('ai-task', shown: true);
      app.draftPending(id, milk.id, NoteKeys.text, text('Oat milk'));
      final background = open();
      // The app adds a draft the background store never read.
      app.draftPending(id, milk.id, NoteKeys.importance, 'important');
      background.draftPending(id, milk.id, NoteKeys.text, text('Rye milk'));
      expect(app.pendingSet(id)!.drafts, hasLength(3));

      app.acceptPending(id);

      expect(background.pendingSet(id), isNull);
      expect(Notes(background).byId(milk.id)!.text, 'Rye milk');
      expect(
        () => background.draftPending(id, milk.id, NoteKeys.text, text('No')),
        throwsStateError,
      );
      expect(File('${dir.path}/pending/$id.json').existsSync(), isFalse);
      background.close();
      app.close();
    });

    test('leaves a file it cannot read where it is, unread', () {
      final store = open();
      store.close();
      final file = File('${dir.path}/pending/abc.json')
        ..createSync(recursive: true)
        ..writeAsStringSync('{"changes":[],"v":2}');

      final again = open();
      expect(again.pendingSets, isEmpty);
      expect(file.existsSync(), isTrue);
      again.close();
    });
  });

  group('received entries', () {
    test('are checked as they come, pinned nothing, and taken in at '
        'Accept', () {
      final note = Notes(bob).capture('From Bob');
      final entries = bob.entriesSince({});
      final forged = Entry.fromJson({
        ...entries.last.toJson(),
        'value': text('Forged'),
      });
      final mine = Entry.fromJson({
        ...entries.first.toJson(),
        'device': ann.deviceId,
        'dseq': 99,
      });
      final report = ImportReport();
      final id = ann.openPending('bundle', shown: true);

      final taken = ann.receivePending(
        id,
        [...entries.take(entries.length - 1), forged, mine],
        keys: [bob.ownKeyRecord],
        report: report,
      );

      expect(taken, hasLength(entries.length - 1));
      expect(report.refusedCount, 2);
      expect(ann.pinnedKey(bob.deviceId), isNull);
      expect(ann.holds(bob.deviceId, 1), isFalse);
      expect(Notes(ann).byId(note.id), isNotNull);
      // The rest comes later; one already in the set is not taken twice.
      expect(
        ann.receivePending(id, entries, keys: [bob.ownKeyRecord]),
        hasLength(1),
      );

      final accepted = ann.acceptPending(id);

      expect(accepted.taken, hasLength(entries.length));
      expect(accepted.written, isEmpty);
      expect(ann.pinnedKey(bob.deviceId), isNotNull);
      expect(Notes(ann).byId(note.id)!.text, 'From Bob');
      expect(ann.versionVector()[bob.deviceId], entries.last.dseq);
    });

    test('give way to an own edit made over them, which saw them, so no '
        'Conflict shows before Accept or after', () {
      final milk = Notes(ann).capture('Milk');
      bobHoldsAnns();
      Notes(bob).edit(milk.id, 'Milk from Bob');
      final id = ann.openPending('bundle', shown: true);
      ann.receivePending(
        id,
        bob.entriesSince(ann.versionVector()),
        keys: bob.keyRecords(),
      );
      expect(Notes(ann).byId(milk.id)!.text, 'Milk from Bob');
      Notes(ann).edit(milk.id, 'Milk, two litres');
      expect(ann.conflicts(), isEmpty);

      final accepted = ann.acceptPending(id);

      expect(ann.conflicts(), isEmpty);
      expect(accepted.kept, [
        KeptEdit(
          milk.id,
          NoteKeys.text,
          pending: text('Milk from Bob'),
          kept: text('Milk, two litres'),
        ),
      ]);
      expect(accepted.written, isEmpty);
      expect(Notes(ann).byId(milk.id)!.text, 'Milk, two litres');
    });

    test('of a set not shown give way to an own edit made after them, '
        'written again so no Conflict stands', () {
      final milk = Notes(ann).capture('Milk');
      bobHoldsAnns();
      Notes(bob).edit(milk.id, 'Milk from Bob');
      final id = ann.openPending('area-offer', shown: false);
      ann.receivePending(
        id,
        bob.entriesSince(ann.versionVector()),
        keys: bob.keyRecords(),
      );
      Notes(ann).edit(milk.id, 'Milk, two litres');

      final accepted = ann.acceptPending(id);

      expect(ann.conflicts(), isEmpty);
      expect(accepted.kept, [
        KeptEdit(
          milk.id,
          NoteKeys.text,
          pending: text('Milk from Bob'),
          kept: text('Milk, two litres'),
        ),
      ]);
      expect(accepted.written.single.value, text('Milk, two litres'));
      expect(Notes(ann).byId(milk.id)!.text, 'Milk, two litres');
    });
  });

  group('trust', () {
    test('a pending change is trusted for nothing: no block, no Area', () {
      final carolId = Store.newId();
      final area = Places(ann).addArea('Household').id;
      final milk = Notes(ann).capture('Milk');
      final id = ann.openPending('ai-task', shown: true);
      ann.draftPending(
        id,
        '${BlockKeys.prefix}$carolId',
        BlockKeys.blocked,
        'Carol',
      );
      ann.draftPending(id, milk.id, NoteKeys.place, area);

      expect(ann.blockedIds, isEmpty);
      expect(ann.scopeOf(milk.id), personalScope);
      expect(Notes(ann).byId(milk.id)!.place, area);
    });

    test("a blocked person's entries never enter a set, and leave one at "
        'the block', () {
      final carol = device(clock, 'Carol')..renameOwnProfile('Carol');
      final area = ann.share(Places(ann).addArea('Household').id);
      exchange(carol, ann, inviting: Invite(area, role: AreaRole.member));
      final milk = Notes(ann).capture('Milk', place: area);
      final bins = Routines(ann).spawn(milk.id, pills);
      exchange(carol, ann);
      final early = ann.openPending('bundle', shown: true);
      final rude = Notes(carol).capture('Rude', place: area);
      ann.receivePending(early, carol.entriesSince(ann.versionVector()));
      expect(Notes(ann).byId(rude.id), isNotNull);

      ann.blockPerson(carol.ownProfile!.id);

      expect(Notes(ann).byId(rude.id), isNull);
      final left = [for (final r in ann.pendingSet(early)!.received) r.entry];
      expect(left, isNotEmpty);
      expect(left, everyElement(predicate<Entry>(keptFromBlocked)));
      Notes(carol).capture('Ruder', place: area);
      final eight = Occurrence(Day(2026, 3, 1), ClockTime(8, 0));
      Routines(carol).markDone(bins.id, eight);
      final late = ann.openPending('bundle', shown: true);
      final taken = ann.receivePending(
        late,
        carol.entriesSince(ann.versionVector()),
      );
      expect(taken, everyElement(predicate<Entry>(keptFromBlocked)));
      expect(taken.map((e) => e.field), contains(RoutineKeys.done(eight)));
      expect(taken.map((e) => e.field), isNot(contains(NoteKeys.text)));
    });

    test("her Device named only by a grant in the set is refused too", () {
      final carol = device(clock, 'Carol')..renameOwnProfile('Carol');
      // Ann blocks Carol, whom she knows from no Area.
      ann.blockPerson(carol.ownProfile!.id);
      final school = bob.share(Places(bob).addArea('School').id);
      exchange(carol, bob, inviting: Invite(school, role: AreaRole.member));
      final rude = Notes(carol).capture('Rude', place: school);
      exchange(carol, bob);
      final offer = ann.openPending('area-offer', shown: false);

      ann.receivePending(offer, bob.entriesSince({}), keys: bob.keyRecords());

      final held = [for (final r in ann.pendingSet(offer)!.received) r.entry];
      expect(held.where((e) => e.entity == school), isNotEmpty);
      expect(
        held.where((e) => e.device == carol.deviceId),
        everyElement(predicate<Entry>(keptFromBlocked)),
      );
      expect(held.where((e) => e.entity == rude.id), isNotEmpty);
      expect(
        held.where((e) => e.entity == rude.id && e.field == NoteKeys.text),
        isEmpty,
      );
      // The log's own judgement knows nothing of the offer.
      expect(ann.blockedDevices(), isEmpty);
    });
  });
}
