import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'announcements.dart';
import 'app_lock.dart' show LockScope;
import 'bundle_platform.dart';
import 'failure_dialog.dart';
import 'frogs_screen.dart' show FrogIcon;
import 'l10n.dart';
import 'look.dart' show ControlBelow;
import 'note_editor_screen.dart' show kindIcon;
import 'page_bar.dart';
import 'profile.dart';
import 'profile_card.dart';
import 'routine_editor_screen.dart' show deviceWallClock;
import 'schedule_text.dart';
import 'system_bars.dart';

/// Packs (#164, ADR 0043): "Export as Pack…" in the menu of an Area, a
/// Topic and a Note opens the tree of what the Pack may hold, everything
/// ticked but Reminders and files, with its size as items are ticked, and
/// shares the file; a Pack opened with the app shows the same tree, all
/// ticked, who it is from and "Claim as mine", and adds the ticked Notes
/// in new Areas.

/// [bytes] in words: "800 B", "12 KB", "1.2 MB".
String packSizeText(String locale, int bytes) {
  if (bytes < 1000) return '$bytes B';
  final number = NumberFormat.decimalPatternDigits(
    locale: locale,
    decimalDigits: 1,
  );
  if (bytes < 1000 * 1000) return '${(bytes / 1000).round()} KB';
  return '${number.format(bytes / (1000 * 1000))} MB';
}

/// Opens the tree of the Pack exported from the Area, Topic or Note
/// [from]. A Tally of a locked Area that is closed now never goes along
/// with a Session.
Future<void> exportPack(BuildContext context, Store store, String from) {
  final lock = LockScope.of(context);
  final closed = {
    for (final area in Places(store).areas())
      if (lock?.isAreaClosed(area.id) ?? false) area.id,
  };
  return Navigator.of(context).push(
    MaterialPageRoute<void>(
      builder: (_) => PackExportScreen(
        store: store,
        pack: store.packOf(from, closed: closed),
      ),
    ),
  );
}

/// The export page (Q467–Q470): the tree with "Tick all" and "Untick
/// all" on the bar, the size live, "Add my Profile" off, and "Share…",
/// which writes the Pack and hands it to the share sheet.
class PackExportScreen extends StatefulWidget {
  final Store store;
  final Pack pack;

  const PackExportScreen({super.key, required this.store, required this.pack});

  @override
  State<PackExportScreen> createState() => _PackExportScreenState();
}

class _PackExportScreenState extends State<PackExportScreen> {
  late Set<String> _ticks = widget.pack.exportTicks();
  var _profile = false;

  Future<void> _share() async {
    final t = context.t;
    final announcements = AnnouncementsScope.of(context);
    final navigator = Navigator.of(context, rootNavigator: true);
    final files = BundleScope.of(context);
    void notSent(String cause, String fix, String details) =>
        announceFailureWith(
          announcements,
          navigator,
          t.packNotSent,
          title: t.packNotSentTitle,
          cause: cause,
          fix: fix,
          details: details,
        );
    final path = await files.exportPath(packFileName(widget.store.clock()));
    if (path == null) {
      notSent(t.packNotWrittenCause, t.nothingLostFix, 'pack: no path');
      return;
    }
    try {
      File(path).writeAsBytesSync(
        widget.store.writePack(widget.pack, _ticks, addProfile: _profile),
      );
    } on Exception catch (e) {
      notSent(t.packNotWrittenCause, t.nothingLostFix, '$e');
      return;
    }
    if (await files.share(path, type: packMimeType) == ShareOutcome.noApp) {
      notSent(t.sendNoApp, t.sendNoAppFix, 'pack: no app took the file');
    }
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final locale = Localizations.localeOf(context).toString();
    final picked = widget.pack.picked(_ticks).notes.isNotEmpty;
    return Scaffold(
      appBar: pageBar(
        context,
        title: t.packExportTitle,
        actions: packTickActions(
          t,
          onAll: () => setState(() => _ticks = widget.pack.allKeys()),
          onNone: () => setState(() => _ticks = {}),
        ),
      ),
      body: ListView(
        padding: clearOfSystemBars(context, const EdgeInsets.all(16)),
        children: [
          Text(
            t.packSize(packSizeText(locale, widget.pack.size(_ticks))),
            key: const ValueKey('pack-size'),
            style: Theme.of(context).textTheme.titleMedium,
          ),
          const SizedBox(height: 8),
          PackTreeView(
            pack: widget.pack,
            ticks: _ticks,
            onTick: (key, on) =>
                setState(() => _ticks = widget.pack.tick(_ticks, key, on)),
          ),
          const Divider(height: 32),
          _TickBelow(
            tickKey: const ValueKey('pack-add-profile'),
            value: _profile,
            onChanged: (on) => setState(() => _profile = on),
            title: t.packAddProfile,
            subtitle: t.packAddProfileHint,
          ),
          const SizedBox(height: 8),
          FilledButton.icon(
            key: const ValueKey('pack-share'),
            onPressed: picked ? () => unawaited(_share()) : null,
            icon: const Icon(Icons.share_outlined),
            label: Text(t.packShare),
          ),
          if (!picked) ...[
            const SizedBox(height: 8),
            Text(t.packNothingTicked),
          ],
        ],
      ),
    );
  }
}

/// A tick with its text above it, the box on its own line below (UI laws,
/// "Dialogs and pages").
class _TickBelow extends StatelessWidget {
  final Key tickKey;
  final bool value;
  final ValueChanged<bool> onChanged;
  final String title;
  final String subtitle;

  const _TickBelow({
    required this.tickKey,
    required this.value,
    required this.onChanged,
    required this.title,
    required this.subtitle,
  });

  @override
  Widget build(BuildContext context) => MergeSemantics(
    child: ControlBelow(
      padding: EdgeInsets.zero,
      title: Text(title),
      subtitle: Text(subtitle),
      control: Checkbox(
        key: tickKey,
        value: value,
        onChanged: (on) => onChanged(on ?? false),
      ),
    ),
  );
}

/// "Tick all" and "Untick all" on the bar of a Pack's tree, also the one
/// Send feedback opens (#214).
List<BarAction> packTickActions(
  AppLocalizations t, {
  required VoidCallback onAll,
  required VoidCallback onNone,
}) => [
  BarAction(
    key: const ValueKey('pack-tick-all'),
    label: t.packTickAll,
    onPressed: onAll,
    icon: Icons.select_all,
  ),
  BarAction(
    key: const ValueKey('pack-untick-all'),
    label: t.packUntickAll,
    onPressed: onNone,
    icon: Icons.deselect,
  ),
];

/// The bytes of the file at [path] when it is a Pack (#164), read whole:
/// a Pack has no size limit. Null for any other file, read only as far
/// as its first byte that is no blank when that opens no JSON object.
List<int>? packFileBytes(String path) {
  try {
    final file = File(path);
    final head = file.openSync();
    try {
      final first = head
          .readSync(64)
          .skipWhile((b) => b == 0x20 || b == 0x09 || b == 0x0a || b == 0x0d);
      if (first.isEmpty || first.first != 0x7b) return null;
    } finally {
      head.closeSync();
    }
    final bytes = file.readAsBytesSync();
    return looksLikePack(bytes) ? bytes : null;
  } on FileSystemException {
    return null;
  }
}

/// Opens the Pack [bytes] (a file opened with the app, #164): its page,
/// or why it did not open, behind the tap.
Future<void> openPackBytes(
  BuildContext context,
  Store store,
  List<int> bytes,
) async {
  final t = context.t;
  final navigator = Navigator.of(context);
  final OpenedPack opened;
  try {
    opened = store.openPack(bytes);
  } on Exception catch (e) {
    final (cause, fix) = switch (e) {
      PackNeedsUpdate() => (t.sentItemNeedsUpdate, t.updateOpenAgainFix),
      PackFromBlocked() => (t.packFromBlocked, t.blockedFix),
      _ => (t.packUnreadable, t.packAskAgainFix),
    };
    announceFailure(
      context,
      t.packNotOpened,
      title: t.packNotOpenedTitle,
      cause: cause,
      fix: fix,
      details: '$e',
    );
    return;
  }
  await navigator.push(
    MaterialPageRoute<void>(
      builder: (_) => PackImportScreen(store: store, opened: opened),
    ),
  );
}

/// The page of a Pack opened (Q459, Q462, Q463): who it is from, with
/// "Add … to People" while they are not kept here, the tree all
/// ticked with "Tick all" and "Untick all", "Claim as mine" ticked when
/// it says who it is from, and "Add", which adds the ticked Notes in new
/// Areas and closes the page.
class PackImportScreen extends StatefulWidget {
  final Store store;
  final OpenedPack opened;

  const PackImportScreen({
    super.key,
    required this.store,
    required this.opened,
  });

  @override
  State<PackImportScreen> createState() => _PackImportScreenState();
}

class _PackImportScreenState extends State<PackImportScreen> {
  late Set<String> _ticks = widget.opened.pack.allKeys();
  var _claim = true;

  Pack get _pack => widget.opened.pack;

  void _add() {
    final t = context.t;
    final added = widget.store.addPack(
      widget.opened,
      _ticks,
      claim: _claim,
      wallClock: deviceWallClock,
      suffix: t.packSuffix,
      noArea: t.packNoArea,
    );
    AnnouncementsScope.of(context)?.announce(t.packAdded(added.notes));
    Navigator.of(context).pop();
  }

  Future<void> _addPerson() async {
    final card = _pack.card;
    if (card == null) return;
    await showCardOpened(
      context,
      widget.store.openProfileCard(card),
      replace: false,
    );
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final from = widget.opened.from;
    final picked = _pack.picked(_ticks).notes.isNotEmpty;
    final name = from == null ? null : profileDisplayName(t, from.name);
    return Scaffold(
      appBar: pageBar(
        context,
        title: t.packTitle,
        actions: packTickActions(
          t,
          onAll: () => setState(() => _ticks = _pack.allKeys()),
          onNone: () => setState(() => _ticks = {}),
        ),
      ),
      body: ListView(
        padding: clearOfSystemBars(context, const EdgeInsets.all(16)),
        children: [
          if (from != null && name != null) ...[
            Row(
              key: const ValueKey('pack-from'),
              children: [
                ProfileAvatar(profile: from, size: 40),
                const SizedBox(width: 12),
                Expanded(
                  child: Text(
                    t.packFrom(name),
                    style: Theme.of(context).textTheme.titleMedium,
                  ),
                ),
              ],
            ),
            if (!widget.opened.fromKept) ...[
              const SizedBox(height: 8),
              OutlinedButton.icon(
                key: const ValueKey('pack-add-person'),
                onPressed: () => unawaited(_addPerson()),
                icon: const Icon(Icons.person_add_alt),
                label: Text(t.packAddPerson(name)),
              ),
            ],
            const Divider(height: 32),
          ],
          PackTreeView(
            pack: _pack,
            ticks: _ticks,
            onTick: (key, on) =>
                setState(() => _ticks = _pack.tick(_ticks, key, on)),
          ),
          const Divider(height: 32),
          if (from != null)
            _TickBelow(
              tickKey: const ValueKey('pack-claim'),
              value: _claim,
              onChanged: (on) => setState(() => _claim = on),
              title: t.packClaim,
              subtitle: t.packClaimHint,
            ),
          const SizedBox(height: 8),
          FilledButton(
            key: const ValueKey('pack-add'),
            onPressed: picked ? _add : null,
            child: Text(t.packAdd),
          ),
          if (!picked) ...[
            const SizedBox(height: 8),
            Text(t.packNothingTicked),
          ],
        ],
      ),
    );
  }
}

/// The tree of [pack] with a box for each item that can be ticked: an
/// Area and a Topic as headings, a Note by its kind's icon and its name,
/// under it its files with their sizes, their texts, its Routines,
/// Reminders and Frog, and under a Session the Tallies it brought from
/// outside. An item under a Note that is not ticked cannot be ticked.
class PackTreeView extends StatelessWidget {
  final Pack pack;
  final Set<String> ticks;
  final void Function(String key, bool on) onTick;

  const PackTreeView({
    super.key,
    required this.pack,
    required this.ticks,
    required this.onTick,
  });

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final locale = Localizations.localeOf(context).toString();
    final text = Theme.of(context).textTheme;
    final rows = <Widget>[];
    void add(PackItem item, int depth, bool enabled) {
      final pad = EdgeInsets.only(left: 16.0 * depth);
      switch (item.kind) {
        case PackItemKind.area || PackItemKind.topic:
          final (a, topic) = _placeOf(item.key);
          final area = pack.areas[a];
          rows.add(
            Padding(
              padding: pad.add(const EdgeInsets.only(top: 12, bottom: 4)),
              child: Text(
                topic == null ? area.name : area.topics[topic],
                style: item.kind == PackItemKind.area
                    ? text.titleMedium
                    : text.titleSmall,
              ),
            ),
          );
          for (final child in item.children) {
            add(child, depth + 1, true);
          }
          return;
        default:
          final ticked = ticks.contains(item.key);
          rows.add(
            Padding(
              padding: pad,
              child: CheckboxListTile(
                key: ValueKey('pack-item-${item.key}'),
                dense: item.kind != PackItemKind.note,
                contentPadding: EdgeInsets.zero,
                controlAffinity: ListTileControlAffinity.leading,
                value: ticked,
                onChanged: enabled
                    ? (on) => onTick(item.key, on ?? false)
                    : null,
                secondary: _icon(item),
                title: Text(
                  _label(t, locale, item),
                  maxLines: 2,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
            ),
          );
          for (final child in item.children) {
            add(
              child,
              depth + 1,
              item.kind == PackItemKind.note ? enabled && ticked : enabled,
            );
          }
      }
    }

    for (final item in pack.tree()) {
      add(item, 0, true);
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: rows,
    );
  }

  /// The Area and Topic `a<i>` and `a<i>.t<j>` name.
  static (int, int?) _placeOf(String key) {
    final parts = key.split('.');
    final a = int.parse(parts.first.substring(1));
    return (a, parts.length > 1 ? int.parse(parts[1].substring(1)) : null);
  }

  Widget? _icon(PackItem item) => switch (item.kind) {
    PackItemKind.note => switch (item.note) {
      final i? => Icon(kindIcon(pack.notes[i].kind)),
      null => null,
    },
    PackItemKind.photo => const Icon(Icons.photo_outlined),
    PackItemKind.photoText ||
    PackItemKind.transcript => const Icon(Icons.notes),
    PackItemKind.recording => const Icon(Icons.mic_none),
    PackItemKind.routine => const Icon(Icons.repeat),
    PackItemKind.reminder => const Icon(Icons.notifications_outlined),
    PackItemKind.frog => const FrogIcon(),
    PackItemKind.area || PackItemKind.topic => null,
  };

  String _label(AppLocalizations t, String locale, PackItem item) {
    // An Area or a Topic has no Note, and the Pack's photos, recordings
    // and Routines always come with their index: anything else is
    // labelled nothing.
    final at = item.note;
    if (at == null) return '';
    final note = pack.notes[at];
    String start(String? text) => (text ?? '').split('\n').first.trim();
    return switch ((item.kind, item.index)) {
      (PackItemKind.note, _) => _firstLine(note),
      (PackItemKind.photo, final i?) => t.packPhoto(
        packSizeText(locale, note.photos[i].size),
      ),
      (PackItemKind.photoText, final i?) => t.packPhotoText(
        start(note.photos[i].text),
      ),
      (PackItemKind.recording, final i?) => t.packRecording(
        packSizeText(locale, note.recordings[i].size),
      ),
      (PackItemKind.transcript, final i?) => t.packTranscript(
        start(note.recordings[i].text),
      ),
      (PackItemKind.routine, final i?) => t.packRoutine(
        scheduleText(t, locale, note.routines[i]),
      ),
      (PackItemKind.reminder, final i?) => t.packReminder(
        scheduleText(t, locale, note.routines[i]),
      ),
      (PackItemKind.frog, _) => t.packFrog,
      _ => '',
    };
  }

  /// The first line with words of [note]'s text as it reads.
  static String _firstLine(PackNote note) {
    final shown = TextFormats.visible(note.format, note.text);
    for (final line in shown.split('\n')) {
      if (line.trim().isNotEmpty) return line.trim();
    }
    return shown.trim();
  }
}
