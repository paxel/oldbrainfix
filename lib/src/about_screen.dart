import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:url_launcher/url_launcher.dart';

import 'l10n.dart';
import 'look.dart';
import 'question_mark_background.dart';

const _repoUrl = 'https://github.com/paxel/oldbrainfix';
const _issuesUrl = '$_repoUrl/issues';
const _feedbackMail = 'taum@tuta.io';
const _donateUrl = 'https://ko-fi.com/paxel7';

const _siteUrl = 'https://paxel.github.io/oldbrainfix';

String _siteLanguage(String languageCode) => languageCode == 'de' ? 'de' : 'en';

/// The documentation site, in the app's language where the site has one
/// (English and German), English otherwise.
String documentationUrl(String languageCode) =>
    '$_siteUrl/${_siteLanguage(languageCode)}/';

/// The privacy policy on the documentation site, in the app's language
/// where the site has one, English otherwise.
String privacyPolicyUrl(String languageCode) =>
    '$_siteUrl/${_siteLanguage(languageCode)}/reference/privacy-policy.html';

/// Opens [url] outside the app; what opens it is the system's choice.
Future<void> openExternally(Uri url) async {
  await launchUrl(url, mode: LaunchMode.externalApplication);
}

/// Who made the app, where its documentation and source live, how to
/// reach the developer, and the licences (adapted from Chaos Alert).
class AboutScreen extends StatefulWidget {
  /// The version to show; read from the platform when null.
  final Future<PackageInfo>? version;

  /// Opens a link.
  final Future<void> Function(Uri url) open;

  const AboutScreen({super.key, this.version, this.open = openExternally});

  @override
  State<AboutScreen> createState() => _AboutScreenState();
}

class _AboutScreenState extends State<AboutScreen> {
  late final Future<PackageInfo> _version =
      widget.version ?? PackageInfo.fromPlatform();

  Future<void> _open(String url) async {
    try {
      await widget.open(Uri.parse(url));
    } on Exception {
      // No app for the link; the entry simply does nothing.
    }
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final language = Localizations.localeOf(context).languageCode;
    final docs = documentationUrl(language);
    final privacy = privacyPolicyUrl(language);
    final accent = LookTones.of(Theme.of(context).brightness).accent;
    return Scaffold(
      appBar: AppBar(title: Text(t.aboutTitle)),
      body: ListView(
        children: [
          const SizedBox(height: 24),
          Center(child: QuestionMarkIcon(size: 64, colour: accent)),
          const SizedBox(height: 8),
          Center(
            child: Text(
              t.appTitle,
              style: Theme.of(context).textTheme.headlineMedium,
            ),
          ),
          Center(
            child: FutureBuilder<PackageInfo>(
              future: _version,
              builder: (context, snapshot) {
                final info = snapshot.data;
                return Text(
                  info == null
                      ? ''
                      : t.versionLabel(info.version, info.buildNumber),
                );
              },
            ),
          ),
          const SizedBox(height: 8),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 32),
            child: Text(t.aboutTagline, textAlign: TextAlign.center),
          ),
          const SizedBox(height: 16),
          for (final (icon, title, subtitle, url) in [
            (Icons.menu_book_outlined, t.documentation, docs, docs),
            (Icons.privacy_tip_outlined, t.privacyPolicy, privacy, privacy),
            (
              Icons.code,
              t.sourceCode,
              '$_repoUrl — Apache-2.0 / MIT',
              _repoUrl,
            ),
            (
              Icons.bug_report_outlined,
              t.reportProblem,
              t.githubIssues,
              _issuesUrl,
            ),
            (
              Icons.mail_outline,
              t.writeTheDeveloper,
              _feedbackMail,
              'mailto:$_feedbackMail?subject=Old%20Brain%20Fix%20feedback',
            ),
            (Icons.coffee_outlined, t.buyCoffee, t.coffeeSubtitle, _donateUrl),
          ])
            ListTile(
              leading: Icon(icon),
              title: Text(title),
              subtitle: Text(subtitle),
              onTap: () => _open(url),
            ),
          ListTile(
            leading: const Icon(Icons.description_outlined),
            title: Text(t.openSourceLicenses),
            onTap: () => showLicensePage(
              context: context,
              applicationName: t.appTitle,
              applicationIcon: QuestionMarkIcon(size: 48, colour: accent),
            ),
          ),
          const SizedBox(height: 32),
        ],
      ),
    );
  }
}

var _licensesRegistered = false;

/// The lists of what ships inside the app but not as a Dart package, with
/// its licence: the ocrs models, whisper.cpp, the Whisper model, Material
/// Icons and Rust's standard library; the Android libraries; the Rust
/// crates in the OCR library (written by tool/licenses.dart).
const licenseLists = [
  'licenses/common.txt',
  'licenses/android.txt',
  'licenses/android-crates.txt',
];

/// Adds [licenseLists] from [bundle] to the open-source licences, which
/// list the Dart packages on their own. Once only.
void registerLicenses([AssetBundle? bundle]) {
  if (_licensesRegistered) return;
  _licensesRegistered = true;
  LicenseRegistry.addLicense(() async* {
    for (final list in licenseLists) {
      yield* Stream.fromIterable(
        licenseEntries(await (bundle ?? rootBundle).loadString(list)),
      );
    }
  });
}

/// The entries of a licence [list]: each is one or more names, one per
/// line, a blank line and the licence text, closed by a line of 80 `=`.
List<LicenseEntry> licenseEntries(String list) => [
  for (final entry in list.split('\n${'=' * 80}\n'))
    if (entry.trim().isNotEmpty)
      LicenseEntryWithLineBreaks(
        entry.substring(0, entry.indexOf('\n\n')).split('\n'),
        entry.substring(entry.indexOf('\n\n') + 2),
      ),
];
