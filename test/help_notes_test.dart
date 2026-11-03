import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/help_notes.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_alarms.dart';
import 'fake_profile.dart';

/// The help Notes' parts as a test hands them in: one release's notes
/// and a picture per group, so the app makes them without its assets.
class FakeHelp implements HelpSource {
  final Map<String, Map<String, String>> notes;

  const FakeHelp([
    this.notes = const {
      '0.4.0': {'en': 'Added\n• Help Notes.', 'de': 'Neu\n• Hilfe-Notizen.'},
    },
  ]);

  @override
  Future<Map<String, Map<String, String>>> whatsNew() async => notes;

  @override
  Future<Uint8List> picture(String path) async =>
      Uint8List.fromList(path.codeUnits);
}

/// A word holds a letter or a digit, as the guide's box rules count them
/// (#171): `+` and `▸` are none.
int words(String line) => line
    .split(RegExp(r'\s+'))
    .where((w) => w.contains(RegExp(r'[\p{L}\p{N}]', unicode: true)))
    .length;

void main() {
  final en = lookupAppLocalizations(const Locale('en'));
  final de = lookupAppLocalizations(const Locale('de'));

  group('the texts', () {
    test('Quick start: as many Lines in each language, each one action of '
        'at most 12 words, nothing of Advanced', () {
      expect(quickStartLines(de), hasLength(quickStartLines(en).length));
      for (final t in [en, de]) {
        for (final line in quickStartLines(t)) {
          expect(words(line), lessThanOrEqualTo(12), reason: line);
          expect(
            line,
            isNot(
              matches(
                RegExp(
                  'Tally|Session|Frog|Strichliste|Einheit|Frosch',
                  caseSensitive: false,
                ),
              ),
            ),
          );
        }
      }
    });

    test('every icon named in the text has a picture in both languages', () {
      for (final t in [en, de]) {
        final text = helpIconsText(t);
        expect(text.split('\n').first, t.helpIconsTitle);
        for (final group in helpIconGroups) {
          final at = text.indexOf('\n\n${group.heading(t)}\n');
          expect(at, isNonNegative, reason: group.heading(t));
          for (final icon in group.icons) {
            final line = '\n${icon.name(t)}: ${icon.means(t)}';
            // Under its own group's heading, before the next one.
            expect(text.indexOf(line, at), isNonNegative, reason: line);
          }
          expect(
            File(group.asset(t.localeName)).existsSync(),
            isTrue,
            reason: group.asset(t.localeName),
          );
        }
      }
    });

    test('the Agenda group shows the Agenda bar\'s icons in its order, New '
        'Session after New Routine (#246), then the Unsorted banner', () {
      final agenda = helpIconGroups.singleWhere((g) => g.slug == 'agenda');
      expect(
        [for (final icon in agenda.icons) icon.icon],
        [
          Icons.folder_outlined,
          Icons.format_list_bulleted,
          Icons.add_alarm,
          Icons.event_repeat,
          Icons.move_to_inbox,
        ],
      );
      expect(
        helpIconsText(en),
        contains(
          '\nNew Session: makes a Session with a new Note, in Advanced.',
        ),
      );
      expect(
        helpIconsText(de),
        contains(
          '\nNeue Einheit: legt in Erweitert eine Einheit mit einer neuen '
          'Notiz an.',
        ),
      );
    });

    test('the bundled What\'s new reads by version and language', () {
      expect(parseWhatsNew('{"0.4.0": {"de": "Neu", "en": "Added"}, "x": 1}'), {
        '0.4.0': {'de': 'Neu', 'en': 'Added'},
      });
      final bundled = File('assets/help/whats_new.json').readAsStringSync();
      for (final notes in parseWhatsNew(bundled).values) {
        expect(notes.keys, containsAll(['en', 'de']));
      }
    });
  });

  group('on start', () {
    late Store store;

    setUp(() => store = Store.inMemory());
    tearDown(() => store.close());

    Future<void> start(
      WidgetTester tester, {
      HelpSetup? help = const HelpSetup(FakeHelp(), version: '0.4.0'),
      String language = 'en',
    }) async {
      tester.platformDispatcher.localesTestValue = [Locale(language)];
      addTearDown(tester.platformDispatcher.clearLocalesTestValue);
      await tester.pumpWidget(
        OldBrainFixApp(
          store: store,
          alarms: FakeAlarms(),
          profile: FakeProfilePlatform(name: 'Pixel'),
          help: help,
        ),
      );
      await tester.pumpAndSettle();
    }

    /// The help Notes in the Area "Old Brain Fix", by text.
    List<String> helpNotes() {
      final area = Places(store)
          .areas()
          .where((a) => a.name == 'Old Brain Fix')
          .single;
      expect(area.icon, 'brain');
      return [for (final n in Notes(store).inside(area.id)) n.text]..sort();
    }

    testWidgets('a fresh install gets Quick start and What the icons mean, '
        'no What\'s new, nothing Unsorted', (tester) async {
      await start(tester);
      await tester.tap(find.byKey(const ValueKey('first-new')));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Start'));
      await tester.pumpAndSettle();
      expect(helpNotes(), ['Quick start', helpIconsText(en)]);
      expect(Notes(store).unsorted(), isEmpty);
      final person = store.ownProfile!.id;
      final list = Checklists(store).byNote(HelpNotes.quickStartId(person))!;
      expect([for (final l in list.lines) l.text], quickStartLines(en));
      final icons = Notes(store).byId(HelpNotes.iconsId(person))!;
      expect(icons.photos, hasLength(helpIconGroups.length));
    });

    testWidgets('a fresh install\'s Agenda shows the Quick start as one row '
        'with its count, not a row per Line', (tester) async {
      // Past the alarm setup, so the first start ends on the Agenda.
      store.alarmSetupDone = true;
      await start(tester);
      await tester.tap(find.byKey(const ValueKey('first-new')));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Start'));
      await tester.pumpAndSettle();
      final id = HelpNotes.quickStartId(store.ownProfile!.id);
      expect(find.byKey(ValueKey('checklist-row-$id')), findsOneWidget);
      expect(find.text('Quick start: 8 of 8 open'), findsOneWidget);
      for (final line in quickStartLines(en)) {
        expect(find.text(line), findsNothing);
      }
    });

    testWidgets('an update gets that release\'s What\'s new and the two '
        'help Notes, in the Device\'s language', (tester) async {
      store
        ..applyProfileDefaults(name: 'Pixel')
        ..starterSetOffered = true;
      await start(tester, language: 'de');
      expect(helpNotes(), [
        'Neu in 0.4.0\n\nNeu\n• Hilfe-Notizen.',
        'Schnellstart',
        helpIconsText(de),
      ]);
      // The next start makes nothing again.
      final written = store.allEntries().length;
      await tester.pumpWidget(const SizedBox());
      await start(tester, language: 'de');
      expect(store.allEntries().length, written);
    });

    testWidgets('without help Notes handed in, the app makes none', (
      tester,
    ) async {
      store
        ..applyProfileDefaults(name: 'Pixel')
        ..starterSetOffered = true;
      await start(tester, help: null);
      expect(Places(store).areas(), isEmpty);
      expect(store.localSetting(HelpNotes.versionSetting), isNull);
    });
  });
}
