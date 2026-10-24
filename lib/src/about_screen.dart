import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:url_launcher/url_launcher.dart';

import 'l10n.dart';
import 'look.dart';
import 'question_mark_background.dart';

const _repoUrl = 'https://github.com/paxel/oldbrainfix';
const _issuesUrl = '$_repoUrl/issues';
const _feedbackMail = 'taum@tuta.io';
const _donateUrl = 'https://ko-fi.com/paxel7';

/// The documentation site, in the app's language where the site has one
/// (English and German), English otherwise.
String documentationUrl(String languageCode) =>
    'https://paxel.github.io/oldbrainfix/${languageCode == 'de' ? 'de' : 'en'}/';

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
    final docs = documentationUrl(Localizations.localeOf(context).languageCode);
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

/// Adds what ships inside the app but not as a Dart package to the
/// open-source licences: the ocrs models and whisper.cpp. Once only.
void registerLicenses() {
  if (_licensesRegistered) return;
  _licensesRegistered = true;
  LicenseRegistry.addLicense(
    () => Stream.fromIterable([
      const LicenseEntryWithLineBreaks(['ocrs models'], _ocrsModels),
      const LicenseEntryWithLineBreaks(['whisper.cpp'], _whisperCpp),
    ]),
  );
}

/// From android/app/src/main/assets/ocrs/NOTICE.
const _ocrsModels = '''text-detection.rten and text-recognition.rten are the pre-trained text detection and recognition models of the ocrs OCR engine by Robert Knight (https://github.com/robertknight/ocrs, models trained with https://github.com/robertknight/ocrs-models on the HierText dataset and synthetic data). They are bundled unchanged.

The models are licensed under the Creative Commons Attribution-ShareAlike 4.0 International License (CC BY-SA 4.0), https://creativecommons.org/licenses/by-sa/4.0/ (see https://huggingface.co/robertknight/ocrs). The HierText dataset they were trained on is licensed under CC BY-SA 4.0 as well.

The ocrs engine and its rten runtime are licensed under MIT OR Apache-2.0.''';

/// From android/app/src/main/cpp/whisper/LICENSE.
const _whisperCpp = '''MIT License

Copyright (c) 2023-2026 The ggml authors

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.''';
