import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

import 'people_test.dart' show device;

void main() {
  late Store ann;

  setUp(() {
    var now = DateTime.utc(2026, 3, 5, 10);
    ann = device(() => now = now.add(const Duration(seconds: 1)), 'Ann');
  });

  test('a Note is of one kind: plain, a list, or a Tally, which wins', () {
    final notes = Notes(ann);
    final plain = notes.capture('Call Ben').id;
    final check = notes.capture('Trip\nSocks').id;
    Checklists(ann).make(check);
    final shop = notes.capture('Shop\nMilk').id;
    ShoppingLists(ann).make(shop);
    final gym = notes.capture('Gym').id;
    Tallies(ann).addField(gym, name: 'Reps', unit: 'reps');
    final kinds = NoteKinds(ann);
    expect(kinds.of(plain), NoteKind.plain);
    expect(kinds.of(check), NoteKind.checklist);
    expect(kinds.of(shop), NoteKind.shoppingList);
    expect(kinds.of(gym), NoteKind.tally);
    // Two Devices apart: a list and a Tally at once shows as the Tally.
    ann.append(gym, ChecklistKeys.kind, ChecklistKeys.kindOf('x'));
    expect(kinds.of(gym), NoteKind.tally);
    Checklists(ann).toPlain(check);
    expect(kinds.of(check), NoteKind.plain);
  });

  group('name and description', () {
    test('the first line is the name, the rest the description, as '
        'stored', () {
      expect(NoteKinds.name('Groceries'), 'Groceries');
      expect(NoteKinds.description('Groceries'), '');
      expect(
        NoteKinds.name('Groceries\nfor Sunday\n\nand Monday'),
        'Groceries',
      );
      expect(
        NoteKinds.description('Groceries\nfor Sunday\n\nand Monday'),
        'for Sunday\n\nand Monday',
      );
      expect(NoteKinds.name('A\r\nB'), 'A');
      expect(NoteKinds.description('A\r\nB'), 'B');
      expect(NoteKinds.name(''), '');
      expect(NoteKinds.description('A\n'), '');
      expect(NoteKinds.name('# Shop **now**\n- milk'), '# Shop **now**');
    });

    test('joined back, they are the same text', () {
      for (final text in [
        'Groceries',
        'Groceries\nfor Sunday',
        'Groceries\n\nfor Sunday\n',
        ' Trip \n  socks',
      ]) {
        expect(
          NoteKinds.join(NoteKinds.name(text), NoteKinds.description(text)),
          text,
        );
      }
      expect(NoteKinds.join('Groceries', ''), 'Groceries');
    });
  });
}
