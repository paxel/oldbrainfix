import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

void main() {
  late Store store;
  late Places places;
  late Notes notes;
  late Search search;
  var now = DateTime.utc(2026, 10, 24, 8);

  setUp(() {
    now = DateTime.utc(2026, 10, 24, 8);
    store = Store.inMemory(clock: () => now);
    places = Places(store);
    notes = Notes(store);
    search = Search(store);
  });
  tearDown(() => store.close());

  /// Captures [text] a minute after the last one, filed into [place].
  Note capture(String text, [String? place]) {
    now = now.add(const Duration(minutes: 1));
    return notes.capture(text, place: place);
  }

  test('finds Note text anywhere, ignoring case, newest first', () {
    final home = places.addArea('Household');
    final kitchen = places.addTopic(home.id, 'Kitchen');
    capture('Buy SAUCE', kitchen.id);
    capture('nees mor sauce');
    capture('call the garage', home.id);
    capture('apple sauce recipe', home.id);
    expect(search.notes('sauce').map((n) => n.text), [
      'apple sauce recipe',
      'nees mor sauce',
      'Buy SAUCE',
    ]);
  });

  test('a query matches as typed, spaces included', () {
    capture('mor sauce');
    capture('sauce mor');
    expect(search.notes('  mor sau ').map((n) => n.text), ['mor sauce']);
  });

  test('a blank query finds everything in scope', () {
    capture('a');
    capture('b');
    expect(search.notes('').map((n) => n.text), ['b', 'a']);
    expect(search.notes('   ').length, 2);
  });

  test('inside an Area it searches the Area and its Topics only', () {
    final home = places.addArea('Household');
    final kitchen = places.addTopic(home.id, 'Kitchen');
    final work = places.addArea('Work');
    capture('sauce in the kitchen', kitchen.id);
    capture('sauce at home', home.id);
    capture('sauce at work', work.id);
    capture('unsorted sauce');
    expect(search.notes('sauce', within: home.id).map((n) => n.text), [
      'sauce at home',
      'sauce in the kitchen',
    ]);
  });

  test('inside a Topic it searches that Topic only', () {
    final home = places.addArea('Household');
    final kitchen = places.addTopic(home.id, 'Kitchen');
    final garden = places.addTopic(home.id, 'Garden');
    capture('sauce in the kitchen', kitchen.id);
    capture('sauce in the garden', garden.id);
    capture('sauce at home', home.id);
    expect(search.notes('SAUCE', within: kitchen.id).map((n) => n.text), [
      'sauce in the kitchen',
    ]);
  });

  test('finds a Note by the text of its Amends', () {
    final note = capture('nees mor sauce');
    capture('call the garage');
    now = now.add(const Duration(minutes: 1));
    notes.amend(note.id, 'Update: bought tomatoes');
    expect(search.notes('TOMATO').map((n) => n.id), [note.id]);
  });

  test('finds merged text under the Note it was merged into, once', () {
    final target = capture('shopping');
    final merged = capture('nees mor sauce');
    now = now.add(const Duration(minutes: 1));
    notes.amend(merged.id, 'and basil');
    notes.merge(merged.id, target.id);
    expect(search.notes('sauce').map((n) => n.id), [target.id]);
    expect(search.notes('basil').map((n) => n.id), [target.id]);
  });

  test('follows a Merge chain to the Note at its end', () {
    final first = capture('first');
    final middle = capture('middle');
    final last = capture('last sauce');
    notes.merge(last.id, middle.id);
    notes.merge(middle.id, first.id);
    expect(search.notes('sauce').map((n) => n.id), [first.id]);
  });

  test('Section text of a trashed Note is not found', () {
    final note = capture('shopping');
    now = now.add(const Duration(minutes: 1));
    notes.amend(note.id, 'sauce');
    notes.trash(note.id);
    expect(search.notes('sauce'), isEmpty);
  });

  test('Section text counts only inside the searched Area or Topic', () {
    final home = places.addArea('Household');
    final work = places.addArea('Work');
    final atHome = capture('at home', home.id);
    final atWork = capture('at work', work.id);
    now = now.add(const Duration(minutes: 1));
    notes.amend(atHome.id, 'sauce');
    notes.amend(atWork.id, 'sauce');
    expect(search.notes('sauce', within: home.id).map((n) => n.id), [
      atHome.id,
    ]);
  });
}
