import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix/src/routine_editor_screen.dart'
    show deviceWallClock;
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// How many Notes the store holds (obf-1069, C12).
const _notes = 5000;

/// How often each step is timed; its median counts.
const _repeats = 5;

/// A page change, and a search with its results (ui-laws.md, obf-1032;
/// obf-1068, C11).
const _page = Duration(milliseconds: 500);
const _searchBudget = Duration(milliseconds: 500);

/// The words the texts are made of; "milk", "mild" and "mint" for the
/// search to narrow.
const _words = [
  'milk', 'mild', 'mint', 'bread', 'garden', 'call', 'mother', 'about', //
  'the', 'trip', 'plan', 'remember', 'book', 'tomorrow', 'and', 'a',
  'window', 'repair', 'meeting', 'list', 'idea', 'letter', 'monday', 'price',
];

/// [count] words picked from [_words] after [seed], the same for the
/// same seed.
String _wordsOf(int seed, int count) {
  var x = seed * 1103515245 + 12345;
  return [
    for (var i = 0; i < count; i++)
      _words[((x = (x * 1103515245 + 12345) & 0x7fffffff) >> 16) %
          _words.length],
  ].join(' ');
}

/// The store of [_notes] Notes, as the desktop's budget test writes them
/// (desktop/crates/oldbrainfix-gui/tests/budget.rs), in one batch, on a
/// clock that stands still after them.
Store _store() {
  var now = DateTime.utc(2026, 3, 1, 9);
  final store = Store.inMemory(clock: () => now)
    ..starterSetOffered = true
    ..alarmSetupDone = true;
  store.batch(() {
    final places = Places(store);
    final homes = <String>[];
    for (var a = 0; a < 6; a++) {
      final area = places.addArea('Area $a').id;
      homes.add(area);
      for (var t = 0; t < 3; t++) {
        homes.add(places.addTopic(area, 'Topic $a.$t').id);
      }
    }
    NamedPlaces(store).add('Home', Coordinates(52.52, 13.405), radius: 300);
    final notes = Notes(store);
    final lists = Checklists(store);
    for (var n = 0; n < _notes; n++) {
      now = now.add(const Duration(minutes: 1));
      // Two of three filed, the others Unsorted.
      final place = n % 3 == 2 ? null : homes[n % homes.length];
      final (text, format) = switch (n % 10) {
        0 => (
          'Note $n\n${[for (var l = 0; l < 6; l++) _wordsOf(n * 7 + l, 3)].join('\n')}',
          TextFormats.plain,
        ),
        1 || 2 => (
          '# Note $n\n\n${_wordsOf(n, 25)} **${_wordsOf(n + 1, 2)}** '
              '#list${n % 7}\n\n- ${_wordsOf(n + 2, 5)}\n'
              '- ${_wordsOf(n + 3, 5)}\n\n${_wordsOf(n + 4, 20)}',
          TextFormats.markdown,
        ),
        _ => ('Note $n ${_wordsOf(n, 20 + n % 60)}', TextFormats.plain),
      };
      final note = notes.capture(
        text,
        format: format,
        place: place,
        photos: [
          if (n % 25 == 0) Uint8List.fromList([n % 251, n ~/ 251, 1, 2, 3]),
        ],
      );
      if (n % 10 == 0) lists.make(note.id);
      if (n % 4 == 0) {
        notes.locate(note.id, Coordinates(52.52 + (n % 9) / 1e5, 13.405));
      }
    }
  });
  return store;
}

/// What the phone's All Notes reads to show its list and its first
/// frame of cards (notes_screen.dart, browser.dart's `diaryCards`): the
/// Notes placed, what this Device may do to them, and what the cards
/// show, read once for the list.
List<PlacedNote> _allNotes(Store store) {
  final shown = placeNotes(
    Places(store),
    store.notesBy(Notes(store).all(), PersonFilter.everyone),
  );
  _cardReads(store, shown);
  return shown;
}

/// What the phone's search of an Area reads (areas_screen.dart), and its
/// cards.
List<PlacedNote> _search(Store store, String query) {
  final shown = placeNotes(Places(store), Search(store).notes(query));
  _cardReads(store, shown);
  return shown;
}

void _cardReads(Store store, List<PlacedNote> shown) {
  final ids = [for (final placed in shown) placed.note.id];
  Notes(store).sectionsOf(ids);
  Checklists(store).ofNotesNow(ids, deviceWallClock);
  ShoppingLists(store).ofNotes(ids, deviceWallClock);
  Routines(store).todayByNote(deviceWallClock);
  store.hiddenFromAiAmong(ids);
  store.packFromsOf(ids);
}

/// How long [body] took, its median of [_repeats], each after [before].
Duration _median(void Function() body, {void Function()? before}) {
  final times = [
    for (var i = 0; i < _repeats; i++)
      () {
        before?.call();
        final watch = Stopwatch()..start();
        body();
        return watch.elapsed;
      }(),
  ]..sort();
  return times[times.length ~/ 2];
}

/// The phone's share of the time budget on a large store (obf-1069,
/// C12): what its lists read of the core, on the host's JIT, a guard
/// against reads that grow with the Notes again, not the time on a
/// phone; the frames' own time is not in it. A tap that switches a card
/// reads nothing (many_cards_test.dart).
void main() {
  test(
    'All Notes and a search read 5000 Notes within the budget',
    () {
      final built = Stopwatch()..start();
      final store = _store();
      addTearDown(store.close);
      // ignore: avoid_print
      print('$_notes Notes written in ${built.elapsed}');
      expect(_allNotes(store), hasLength(_notes));

      final steps = <(String, Duration, Duration)>[
        (
          'All Notes opened, read anew',
          _median(() => _allNotes(store), before: store.changedElsewhere),
          _page,
        ),
        ('All Notes opened again', _median(() => _allNotes(store)), _page),
        for (final query in ['m', 'mi', 'mil'])
          (
            'Searched for "$query"',
            _median(() => _search(store, query)),
            _searchBudget,
          ),
      ];
      final found = _search(store, 'mil').length;
      expect(found, inExclusiveRange(0, _notes));
      for (final (name, median, budget) in steps) {
        // ignore: avoid_print
        print('$name: median $median, budget $budget');
      }
      expect([
        for (final (name, median, budget) in steps)
          if (median > budget) '$name: $median over $budget',
      ], isEmpty);
    },
    timeout: const Timeout(Duration(minutes: 5)),
    skip: 'a timing test; run with --run-skipped',
  );
}
