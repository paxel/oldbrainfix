import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

void main() {
  group('TranscriptPrompts.of', () {
    test('both prompts list the Area and Topic names', () {
      final prompts = TranscriptPrompts.of(
        names: ['Work', 'Garten', 'Fahrradladen'],
        terms: ['Unsortiert', 'Routine', 'Agenda'],
      );
      expect(prompts.english, 'Work, Garten, Fahrradladen.');
      for (final name in ['Work', 'Garten', 'Fahrradladen']) {
        expect(prompts.german, contains(name));
      }
    });

    test('German leads in with English words used in German, then the '
        'app\'s words', () {
      final prompts = TranscriptPrompts.of(
        names: ['Garten'],
        terms: ['Unsortiert', 'Routine', 'Agenda'],
      );
      expect(prompts.german, startsWith(TranscriptPrompts.germanLeadIn));
      for (final word in [
        'Meeting',
        'Deadline',
        'Update',
        'Shopping',
        'Download',
        'Team',
        'Feedback',
        'Call',
        'Termin',
      ]) {
        expect(prompts.german, contains(word));
      }
      expect(
        prompts.german,
        '${TranscriptPrompts.germanLeadIn} Unsortiert, Routine, Agenda. '
        'Garten.',
      );
      // English gets only the names.
      expect(prompts.english, 'Garten.');
    });

    test('without names English leads in nothing, German still does', () {
      final prompts = TranscriptPrompts.of(names: const []);
      expect(prompts.english, isEmpty);
      expect(prompts.german, TranscriptPrompts.germanLeadIn);
    });

    test('empty names and repeats in any case are left out', () {
      final prompts = TranscriptPrompts.of(
        names: [' Work ', '', 'work', 'WORK', '  ', 'Home'],
      );
      expect(prompts.english, 'Work, Home.');
    });

    test('many long names are cut so both prompts fit Whisper', () {
      final names = [
        for (var i = 0; i < 200; i++) 'Grundstücksverwaltungsangelegenheit $i',
      ];
      final prompts = TranscriptPrompts.of(
        names: names,
        terms: ['Unsortiert', 'Routine', 'Agenda'],
      );
      expect(
        prompts.german.runes.length,
        lessThanOrEqualTo(TranscriptPrompts.maxLength),
      );
      expect(
        prompts.english.runes.length,
        lessThanOrEqualTo(TranscriptPrompts.maxLength),
      );
      // The lead-in stays whole and the first names come.
      expect(prompts.german, startsWith(TranscriptPrompts.germanLeadIn));
      expect(prompts.german, contains('Grundstücksverwaltungsangelegenheit 0'));
      expect(prompts.english, isNot(contains('angelegenheit 199')));
    });

    test('lists at most maxNames names', () {
      final prompts = TranscriptPrompts.of(
        names: [for (var i = 0; i < 100; i++) 'N$i'],
      );
      expect(prompts.english.split(', ').length, TranscriptPrompts.maxNames);
    });

    test('a name too long to fit is skipped, the next one still comes', () {
      final prompts = TranscriptPrompts.of(names: ['x' * 500, 'Work']);
      expect(prompts.english, 'Work.');
    });
  });

  group('TranscriptPrompts.forPlaces', () {
    late Store store;
    late Places places;

    setUp(() {
      store = Store.inMemory();
      places = Places(store);
    });
    tearDown(() => store.close());

    test('lists the Areas first, then their Topics', () {
      final work = places.addArea('Work');
      places.addArea('Arzt');
      places.addTopic(work.id, 'Meetings');
      final prompts = TranscriptPrompts.forPlaces(places);
      expect(prompts.english, 'Arzt, Work, Meetings.');
      expect(prompts.german, endsWith('Arzt, Work, Meetings.'));
    });

    test('takes the names as the front end shows them', () {
      places.plantStarterSet({'household', 'kitchen'});
      final prompts = TranscriptPrompts.forPlaces(
        places,
        areaName: (area) => area.starter == 'household' ? 'Haushalt' : '',
        topicName: (topic) => topic.starter == 'kitchen' ? 'Küche' : '',
        terms: ['Unsortiert'],
      );
      expect(prompts.english, 'Haushalt, Küche.');
      expect(prompts.german, contains('Unsortiert. Haushalt, Küche.'));
    });
  });
}
