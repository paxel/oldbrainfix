import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:test/test.dart';

void main() {
  group('AppLink (#261, ADR 0046)', () {
    test('every form reads back from the link it writes', () {
      for (final link in const [
        AppLink(AppLinkKind.note, 'note:3f2a'),
        AppLink(AppLinkKind.place, 'area:help-p1'),
        AppLink(AppLinkKind.place, 'topic:x y/ü'),
        AppLink(AppLinkKind.agenda),
        AppLink(AppLinkKind.unsorted),
        AppLink(AppLinkKind.settings),
        AppLink(AppLinkKind.settings, 'alarms'),
        AppLink(AppLinkKind.routines),
        AppLink(AppLinkKind.backups),
        AppLink(AppLinkKind.sync),
        AppLink(AppLinkKind.profile),
        AppLink(AppLinkKind.frogs),
        AppLink(AppLinkKind.newSession),
        AppLink(AppLinkKind.guide),
        AppLink(AppLinkKind.guide, 'getting-started'),
        AppLink(AppLinkKind.guide, 'getting-started', 'set-up'),
        AppLink(AppLinkKind.file, 'Scans/Tax 2026 (a).pdf'),
      ]) {
        expect(AppLink.parse(link.url), link, reason: link.url);
      }
    });

    test('the links are written as the user picks them', () {
      expect(
        const AppLink(AppLinkKind.note, 'note:3f2a').url,
        'oldbrainfix://note/note:3f2a',
      );
      expect(
        const AppLink(AppLinkKind.place, 'topic:x y/ü').url,
        'oldbrainfix://area/topic:x%20y%2F%C3%BC',
      );
      expect(
        const AppLink(AppLinkKind.backups).url,
        'oldbrainfix://settings/backups',
      );
      expect(
        const AppLink(AppLinkKind.newSession).url,
        'oldbrainfix://session/new',
      );
      expect(
        const AppLink(AppLinkKind.file, 'Scans/Tax 2026 (a).pdf').url,
        'oldbrainfix://file/Scans/Tax%202026%20%28a%29.pdf',
      );
      expect(
        AppLink.parse('OldBrainFix://agenda'),
        const AppLink(AppLinkKind.agenda),
      );
    });

    test('a file link never leads out of the shared folder', () {
      for (final url in [
        'oldbrainfix://file/../etc/passwd',
        'oldbrainfix://file/a/%2E%2E/b',
        'oldbrainfix://file/./a',
        'oldbrainfix://file//etc/passwd',
        'oldbrainfix://file/a%2Fb',
        'oldbrainfix://file/a%5Cb',
        'oldbrainfix://file/C:',
        'oldbrainfix://file/C%3A',
        'oldbrainfix://file/a%00',
        'oldbrainfix://file/a/',
        'oldbrainfix://file',
      ]) {
        expect(AppLink.parse(url), isNull, reason: url);
      }
      expect(
        AppLink.parse('oldbrainfix://file/a/b%20c.pdf'),
        const AppLink(AppLinkKind.file, 'a/b c.pdf'),
      );
    });

    test('anything else with the scheme is no app link', () {
      for (final url in [
        'oldbrainfix://x',
        'oldbrainfix://',
        'oldbrainfix://nudge/10',
        'oldbrainfix://alarm/a/b/1',
        'oldbrainfix://capture/voice',
        'oldbrainfix://pick?session=s',
        'oldbrainfix://agenda/',
        'oldbrainfix://Agenda',
        'oldbrainfix://agenda?x',
        'oldbrainfix://agenda#x',
        'oldbrainfix://note/',
        'oldbrainfix://note/a/b',
        'oldbrainfix://note/%zz',
        'oldbrainfix://note/%C3',
        'oldbrainfix://settings/nothing',
        'oldbrainfix://settings/alarms/x',
        'oldbrainfix://session',
        'oldbrainfix://guide/Getting',
        'oldbrainfix://guide/a#B',
        'oldbrainfix://guide/a/b',
        'oldbrainfix:agenda',
        'https://agenda',
      ]) {
        expect(AppLink.parse(url), isNull, reason: url);
      }
    });

    test('a guide page opens on the website in the language', () {
      expect(
        const AppLink(AppLinkKind.guide).guideAddress('de'),
        'https://paxel.github.io/oldbrainfix/de/guide/',
      );
      expect(
        const AppLink(
          AppLinkKind.guide,
          'getting-started',
          'set-up',
        ).guideAddress('fr'),
        'https://paxel.github.io/oldbrainfix/en/guide/getting-started.html#set-up',
      );
    });

    test('app links open, and are left out of the HTML', () {
      expect(Markdown.mayOpen('oldbrainfix://settings/backups'), isTrue);
      expect(Markdown.mayOpen('oldbrainfix://file/a.pdf'), isTrue);
      expect(Markdown.mayOpen('oldbrainfix://capture/voice'), isFalse);
      expect(
        Markdown.html('[Backups](oldbrainfix://settings/backups)'),
        '<p>Backups</p>\n',
      );
    });

    test('a link put at a cursor shows the label it is given', () {
      expect(
        Markdown.link(
          'go ',
          3,
          3,
          'oldbrainfix://unsorted',
          label: 'Unsorted [all]',
        ),
        (
          text: r'go [Unsorted \[all\]](oldbrainfix://unsorted)',
          start: 4,
          end: 20,
        ),
      );
      // On a word, the word stays the label.
      expect(
        Markdown.link('see Frogs', 4, 4, 'oldbrainfix://frogs', label: 'x'),
        (text: 'see [Frogs](oldbrainfix://frogs)', start: 5, end: 5),
      );
    });
  });
}
