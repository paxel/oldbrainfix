import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:package_info_plus/package_info_plus.dart';

import 'announcements.dart';
import 'app_lock.dart' show LockScope;
import 'bundle_platform.dart';
import 'crash_guard.dart';
import 'failure_dialog.dart';
import 'l10n.dart';
import 'look.dart' show ControlBelow;
import 'pack_screen.dart';
import 'page_bar.dart';
import 'system_bars.dart';

/// Send feedback (#214, Q536 (a)): Settings ▸ "Send feedback" is a page
/// with a text field, ticks for what goes along, a preview of exactly
/// what is sent, always shown above the buttons, and Mail, to the
/// developer's address, and Share…, to any app. Nothing leaves without
/// the preview.

/// The subject of every feedback mail.
const feedbackSubject = 'Old Brain Fix feedback';

/// What the feedback can say of this build and this phone.
class FeedbackFacts {
  /// The app's version and build number, "0.4.0+57".
  final String version;

  /// The system and the device, "Android 14 (SDK 34), Google Pixel 7".
  final String system;

  /// Where the app was installed from ([installMethodOf]).
  final String install;

  const FeedbackFacts({
    required this.version,
    required this.system,
    required this.install,
  });
}

/// Where the app came from, by the package name of the app that
/// installed it: Play and F-Droid by their stores, an APK by the
/// system's package installer or none recorded, "unknown" when the
/// system would not say ([known] false), and any other installer named,
/// as it is no store this app is published in.
String installMethodOf(String? installer, {bool known = true}) {
  if (!known) return 'unknown';
  return switch (installer) {
    'com.android.vending' => 'Play',
    'org.fdroid.fdroid' || 'org.fdroid.basic' => 'F-Droid',
    null ||
    'com.google.android.packageinstaller' ||
    'com.android.packageinstaller' => 'APK',
    final other => 'unknown ($other)',
  };
}

/// Reads [FeedbackFacts] of this phone: the version from the package,
/// the rest through the app's device channel; what cannot be read says
/// "unknown".
Future<FeedbackFacts> readFeedbackFacts() async {
  var version = 'unknown';
  try {
    final info = await PackageInfo.fromPlatform();
    version = '${info.version}+${info.buildNumber}';
  } catch (_) {}
  Map<Object?, Object?>? facts;
  try {
    facts = await const MethodChannel('io.github.paxel.oldbrainfix/device')
        .invokeMethod<Map<Object?, Object?>>('facts');
  } catch (_) {}
  if (facts == null) {
    return FeedbackFacts(
      version: version,
      system:
          '${Platform.operatingSystem} ${Platform.operatingSystemVersion}, '
          'device unknown',
      install: 'unknown',
    );
  }
  final maker = facts['manufacturer'] as String? ?? '';
  final model = facts['model'] as String? ?? '';
  final device = model.toLowerCase().startsWith(maker.toLowerCase())
      ? model
      : '$maker $model';
  return FeedbackFacts(
    version: version,
    system:
        'Android ${facts['release'] ?? '?'} (SDK ${facts['sdk'] ?? '?'}), '
        '${device.trim().isEmpty ? 'device unknown' : device.trim()}',
    install: installMethodOf(
      facts['installer'] as String?,
      known: facts['installerKnown'] as bool? ?? false,
    ),
  );
}

/// The text the feedback sends, exactly as the preview shows it: the
/// user's [text], then each of [version], [system] and [install] that
/// is ticked, then the [crash] report, thinned and cut as a crash mail
/// is so that the whole fits into a mail link where it can, at least
/// its first 400 characters.
String feedbackBody(
  String text, {
  String? version,
  String? system,
  String? install,
  String? crash,
}) {
  final facts = [
    if (version != null) 'Version: $version',
    if (system != null) 'System: $system',
    if (install != null) 'Installed from: $install',
  ];
  final before = [text.trim(), if (facts.isNotEmpty) facts.join('\n')];
  if (crash == null) return before.join('\n\n');
  const label = 'Last crash report:\n';
  final head = '${before.join('\n\n')}\n\n$label';
  final room = mailBodyLimit - head.length;
  return '$head${mailBody(crash, limit: room < 400 ? 400 : room)}';
}

/// The page (#214). [facts] and [crash] are read when it opens, [mail]
/// opens the mail app; tests and the guide's frames hand in their own.
class FeedbackScreen extends StatefulWidget {
  final Store store;
  final Future<FeedbackFacts> Function() facts;
  final ({String text, DateTime at})? Function() crash;
  final Future<void> Function(String body) mail;

  const FeedbackScreen({
    super.key,
    required this.store,
    this.facts = readFeedbackFacts,
    this.crash = keptCrashReport,
    this.mail = _mail,
  });

  static Future<void> _mail(String body) =>
      mailDeveloper(feedbackSubject, body);

  @override
  State<FeedbackScreen> createState() => _FeedbackScreenState();
}

class _FeedbackScreenState extends State<FeedbackScreen> {
  final _text = TextEditingController();
  FeedbackFacts? _facts;
  late final _crash = widget.crash();
  var _version = true;
  var _system = true;
  var _install = true;
  var _withCrash = false;

  /// The Pack of everything the tree offers, made when Notes are first
  /// ticked, and what is ticked in it.
  Pack? _pack;
  Set<String> _ticks = {};
  var _withNotes = false;

  @override
  void initState() {
    super.initState();
    _text.addListener(() => setState(() {}));
    unawaited(
      widget.facts().then((facts) {
        if (mounted) setState(() => _facts = facts);
      }),
    );
  }

  @override
  void dispose() {
    _text.dispose();
    super.dispose();
  }

  String get _body {
    final facts = _facts;
    return feedbackBody(
      _text.text,
      version: _version ? facts?.version : null,
      system: _system ? facts?.system : null,
      install: _install ? facts?.install : null,
      crash: _withCrash ? _crash?.text : null,
    );
  }

  /// The Notes going along, ticked and with at least one picked.
  Pack? get _picked {
    final pack = _pack;
    if (!_withNotes || pack == null) return null;
    return pack.picked(_ticks).notes.isEmpty ? null : pack;
  }

  /// Opens the Pack's tree, nothing ticked at first; the Notes go along
  /// while one is picked.
  Future<void> _pickNotes() async {
    final lock = LockScope.of(context);
    final pack = _pack ??= widget.store.packOfAll(
      closed: {
        for (final area in Places(widget.store).areas())
          if (lock?.isAreaClosed(area.id) ?? false) area.id,
      },
    );
    await Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) => _FeedbackNotesScreen(
          pack: pack,
          ticks: _ticks,
          onChanged: (ticks) => _ticks = ticks,
        ),
      ),
    );
    if (!mounted) return;
    setState(() => _withNotes = pack.picked(_ticks).notes.isNotEmpty);
  }

  Future<void> _share(String body) async {
    final t = context.t;
    final announcements = AnnouncementsScope.of(context);
    final navigator = Navigator.of(context, rootNavigator: true);
    final files = BundleScope.of(context);
    void notSent(String cause, String fix, String details) =>
        announceFailureWith(
          announcements,
          navigator,
          t.feedbackNotSent,
          title: t.feedbackNotSentTitle,
          cause: cause,
          fix: fix,
          details: details,
        );
    final shared = <SharedFile>[];
    if (_picked case final pack?) {
      final path = await files.exportPath(packFileName(widget.store.clock()));
      if (path == null) {
        notSent(t.packNotWrittenCause, t.nothingLostFix, 'feedback: no path');
        return;
      }
      try {
        File(path).writeAsBytesSync(widget.store.writePack(pack, _ticks));
      } on Exception catch (e) {
        notSent(t.packNotWrittenCause, t.nothingLostFix, 'feedback: $e');
        return;
      }
      shared.add((path: path, type: packMimeType));
    }
    final outcome = await files.shareText(
      body,
      files: shared,
      mailTo: crashMail,
      subject: feedbackSubject,
    );
    if (outcome == ShareOutcome.noApp) {
      notSent(t.sendNoApp, t.sendNoAppFix, 'feedback: no app took it');
    }
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final theme = Theme.of(context);
    final locale = Localizations.localeOf(context).toString();
    final facts = _facts;
    final body = _body;
    final written = _text.text.trim().isNotEmpty;
    final ready = written && facts != null;
    final fits = body.length <= mailBodyLimit;
    final picked = _picked;
    final crash = _crash;
    String? notes;
    if (picked != null) {
      notes = t.feedbackNotesPicked(
        picked.picked(_ticks).notes.length,
        packSizeText(locale, picked.size(_ticks)),
      );
    }
    Widget tick(
      String key,
      String title,
      String? detail,
      bool value,
      ValueChanged<bool>? onChanged,
    ) => MergeSemantics(
      child: ControlBelow(
        padding: const EdgeInsets.symmetric(vertical: 4),
        title: Text(title),
        subtitle: detail == null ? null : Text(detail),
        control: Checkbox(
          key: ValueKey('feedback-$key'),
          value: value,
          onChanged: onChanged == null ? null : (on) => onChanged(on ?? false),
        ),
      ),
    );
    return Scaffold(
      appBar: pageBar(context, title: t.feedbackTitle),
      body: ListView(
        padding: clearOfSystemBars(context, const EdgeInsets.all(16)),
        children: [
          TextField(
            key: const ValueKey('feedback-text'),
            controller: _text,
            minLines: 3,
            maxLines: 8,
            textCapitalization: TextCapitalization.sentences,
            decoration: InputDecoration(
              labelText: t.feedbackTextLabel,
              border: const OutlineInputBorder(),
            ),
          ),
          const SizedBox(height: 16),
          Text(t.feedbackGoesAlong, style: theme.textTheme.titleMedium),
          tick(
            'version',
            t.feedbackVersion,
            facts?.version,
            _version,
            (on) => setState(() => _version = on),
          ),
          tick(
            'system',
            t.feedbackSystem,
            facts?.system,
            _system,
            (on) => setState(() => _system = on),
          ),
          tick(
            'install',
            t.feedbackInstall,
            facts?.install,
            _install,
            (on) => setState(() => _install = on),
          ),
          tick(
            'crash',
            t.feedbackCrash,
            crash == null
                ? t.feedbackCrashNone
                : t.feedbackCrashFrom(
                    DateFormat.yMd(locale).add_Hm().format(crash.at),
                  ),
            _withCrash,
            crash == null ? null : (on) => setState(() => _withCrash = on),
          ),
          tick(
            'notes',
            t.feedbackNotes,
            notes ?? t.feedbackNotesHint,
            _withNotes,
            (on) => on
                ? unawaited(_pickNotes())
                : setState(() => _withNotes = false),
          ),
          if (_withNotes)
            Align(
              alignment: AlignmentDirectional.centerStart,
              child: TextButton(
                key: const ValueKey('feedback-pick-notes'),
                onPressed: () => unawaited(_pickNotes()),
                child: Text(t.feedbackPickNotes),
              ),
            ),
          const SizedBox(height: 16),
          Text(t.feedbackPreview, style: theme.textTheme.titleMedium),
          const SizedBox(height: 8),
          Container(
            width: double.infinity,
            padding: const EdgeInsets.all(8),
            decoration: BoxDecoration(
              color: theme.colorScheme.surfaceContainerHighest,
              borderRadius: BorderRadius.circular(8),
            ),
            child: SelectableText(
              body,
              key: const ValueKey('feedback-preview'),
              style: const TextStyle(fontFamily: 'monospace', fontSize: 12),
            ),
          ),
          if (notes != null) ...[
            const SizedBox(height: 8),
            Text(
              t.feedbackPackAlong(notes),
              key: const ValueKey('feedback-along'),
            ),
          ],
          const SizedBox(height: 16),
          Row(
            children: [
              Expanded(
                child: FilledButton.icon(
                  key: const ValueKey('feedback-mail'),
                  onPressed: ready && fits
                      ? () => unawaited(widget.mail(body))
                      : null,
                  icon: const Icon(Icons.mail_outline),
                  label: Text(t.feedbackMail),
                ),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: OutlinedButton.icon(
                  key: const ValueKey('feedback-share'),
                  onPressed: ready ? () => unawaited(_share(body)) : null,
                  icon: const Icon(Icons.share_outlined),
                  label: Text(t.feedbackShare),
                ),
              ),
            ],
          ),
          if (!written) ...[
            const SizedBox(height: 8),
            Text(t.feedbackTextFirst),
          ] else if (!fits) ...[
            const SizedBox(height: 8),
            Text(t.feedbackTooLongForMail),
          ],
          if (picked != null) ...[
            const SizedBox(height: 8),
            Text(t.feedbackPackNotInMail),
          ],
        ],
      ),
    );
  }
}

/// The Pack's tree for the feedback, nothing ticked at first: "Tick all"
/// and "Untick all" on the bar and the size over the tree, as the
/// export's; each change is handed to [onChanged], and back goes back.
class _FeedbackNotesScreen extends StatefulWidget {
  final Pack pack;
  final Set<String> ticks;
  final ValueChanged<Set<String>> onChanged;

  const _FeedbackNotesScreen({
    required this.pack,
    required this.ticks,
    required this.onChanged,
  });

  @override
  State<_FeedbackNotesScreen> createState() => _FeedbackNotesScreenState();
}

class _FeedbackNotesScreenState extends State<_FeedbackNotesScreen> {
  late Set<String> _ticks = widget.ticks;

  void _set(Set<String> ticks) {
    setState(() => _ticks = ticks);
    widget.onChanged(ticks);
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final locale = Localizations.localeOf(context).toString();
    final pack = widget.pack;
    return Scaffold(
      appBar: pageBar(
        context,
        title: t.feedbackNotesTitle,
        actions: packTickActions(
          t,
          onAll: () => _set(pack.allKeys()),
          onNone: () => _set({}),
        ),
      ),
      body: ListView(
        padding: clearOfSystemBars(context, const EdgeInsets.all(16)),
        children: [
          Text(
            t.packSize(packSizeText(locale, pack.size(_ticks))),
            key: const ValueKey('pack-size'),
            style: Theme.of(context).textTheme.titleMedium,
          ),
          const SizedBox(height: 8),
          PackTreeView(
            pack: pack,
            ticks: _ticks,
            onTick: (key, on) => _set(pack.tick(_ticks, key, on)),
          ),
          if (pack.picked(_ticks).notes.isEmpty) ...[
            const SizedBox(height: 8),
            Text(t.packNothingTicked),
          ],
        ],
      ),
    );
  }
}
