import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

void main() {
  late Store store;
  late Notes notes;
  var now = DateTime.utc(2026, 10, 24, 8);

  setUp(() {
    now = DateTime.utc(2026, 10, 24, 8);
    store = Store.inMemory(clock: () => now);
    notes = Notes(store);
  });
  tearDown(() => store.close());

  group('source app', () {
    test('a shared-in Note records the app it came from', () {
      const whatsApp = SourceApp('com.whatsapp', label: 'WhatsApp');
      final note = notes.capture('nees mor sauce', sourceApp: whatsApp);
      expect(note.context.sourceApp, whatsApp);
      expect(note.context.sourceApp?.name, 'WhatsApp');
      expect(notes.byId(note.id)?.context.hasMore, isTrue);
    });

    test('without a label the package is shown', () {
      final note = notes.capture(
        'x',
        sourceApp: const SourceApp('com.example.chat'),
      );
      expect(note.context.sourceApp?.label, isNull);
      expect(note.context.sourceApp?.name, 'com.example.chat');
      expect(const SourceApp('a', label: ' ').name, 'a');
    });

    test('a typed Note has none and nothing more to show', () {
      final note = notes.capture('nees mor sauce');
      expect(note.context.sourceApp, isNull);
      expect(note.context.calendarEvents, isEmpty);
      expect(note.context.bluetoothDevices, isEmpty);
      expect(note.context.hasMore, isFalse);
    });
  });

  group('calendar and Bluetooth', () {
    test('arrive after Capture and are attached to the Note', () {
      final note = notes.capture('nees mor sauce');
      notes.attachContext(
        note.id,
        calendarEvents: ['Dentist'],
        bluetoothDevices: ['My Car'],
      );
      final context = notes.byId(note.id)!.context;
      expect(context.calendarEvents, ['Dentist']);
      expect(context.bluetoothDevices, ['My Car']);
      expect(context.hasMore, isTrue);
    });

    test('each alone counts as more to show', () {
      final a = notes.capture('a');
      notes.attachContext(a.id, calendarEvents: ['Dentist']);
      final b = notes.capture('b');
      notes.attachContext(b.id, bluetoothDevices: ['Headphones']);
      expect(notes.byId(a.id)!.context.hasMore, isTrue);
      expect(notes.byId(a.id)!.context.bluetoothDevices, isEmpty);
      expect(notes.byId(b.id)!.context.hasMore, isTrue);
      expect(notes.byId(b.id)!.context.calendarEvents, isEmpty);
    });

    test('empty lists write nothing', () {
      final note = notes.capture('a');
      final before = store.allEntries().length;
      notes.attachContext(note.id);
      expect(store.allEntries().length, before);
      expect(notes.byId(note.id)!.context.hasMore, isFalse);
    });

    test('an Amend Section gets its own', () {
      final note = notes.capture('nees mor sauce');
      now = now.add(const Duration(hours: 2));
      final section = notes.amend(note.id, 'sauce bought');
      notes.attachContext(
        section.id,
        calendarEvents: ['Shopping'],
        bluetoothDevices: ['My Car'],
      );
      final kept = notes.sections(note.id).single.context;
      expect(kept.calendarEvents, ['Shopping']);
      expect(kept.bluetoothDevices, ['My Car']);
      expect(notes.byId(note.id)!.context.hasMore, isFalse);
    });

    test('a merged Note keeps its context as a Section', () {
      final into = notes.capture('sauce');
      final merged = notes.capture(
        'nees mor sauce',
        sourceApp: const SourceApp('com.whatsapp', label: 'WhatsApp'),
      );
      notes.attachContext(merged.id, calendarEvents: ['Dentist']);
      notes.merge(merged.id, into.id);
      final kept = notes.sections(into.id).single.context;
      expect(kept.sourceApp?.name, 'WhatsApp');
      expect(kept.calendarEvents, ['Dentist']);
    });

    test('only Notes and Sections take it', () {
      expect(
        () => notes.attachContext('note:none', calendarEvents: ['x']),
        throwsArgumentError,
      );
    });

    test('a damaged stored list reads as none', () {
      final note = notes.capture('a');
      store.append(note.id, NoteKeys.calendar, 'not json');
      store.append(note.id, NoteKeys.bluetooth, '{"a": 1}');
      final context = notes.byId(note.id)!.context;
      expect(context.calendarEvents, isEmpty);
      expect(context.bluetoothDevices, isEmpty);
    });
  });

  group('running events', () {
    test('timed events win over all-day ones', () {
      expect(
        runningEventTitles([
          (title: 'Birthday Anna', allDay: true),
          (title: 'Dentist', allDay: false),
          (title: 'Call Mum', allDay: false),
        ]),
        ['Dentist', 'Call Mum'],
      );
    });

    test('all-day events count when nothing else runs', () {
      expect(runningEventTitles([(title: 'Holiday', allDay: true)]), [
        'Holiday',
      ]);
    });

    test('blank and repeated titles are dropped', () {
      expect(
        runningEventTitles([
          (title: ' ', allDay: false),
          (title: 'Dentist ', allDay: false),
          (title: 'Dentist', allDay: false),
        ]),
        ['Dentist'],
      );
      expect(runningEventTitles([(title: '', allDay: false)]), isEmpty);
      expect(runningEventTitles(const []), isEmpty);
    });
  });
}
