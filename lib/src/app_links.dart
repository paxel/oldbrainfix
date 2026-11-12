import 'dart:async';

import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'alarm_sync.dart';
import 'all_routines_screen.dart';
import 'announcements.dart';
import 'app_lock.dart';
import 'areas_screen.dart';
import 'backup_screen.dart';
import 'browser.dart';
import 'folder_platform.dart';
import 'frogs_screen.dart';
import 'l10n.dart';
import 'nudge_screen.dart';
import 'note_editor_screen.dart';
import 'page_bar.dart';
import 'place_names.dart';
import 'profile.dart';
import 'profile_screen.dart';
import 'routine_editor_screen.dart';
import 'settings_screen.dart';
import 'sync_screen.dart';
import 'text_marks_view.dart';

/// Opens the app link [link] tapped in a text below [context].
typedef AppLinkOpen = Future<void> Function(BuildContext context, AppLink link);

/// Asks which target a new app link opens (#261): the one picked, or
/// null.
typedef LinkPick = Future<LinkTarget?> Function(BuildContext context);

/// Where a tapped app link in a text below goes (#261, ADR 0046): the
/// app's own [AppLinks]; and what the format bar's "Link to…" picks
/// from. Without one, a tap says "Not available here" and the bar has no
/// "Link to…".
class AppLinkScope extends InheritedWidget {
  final AppLinkOpen onOpen;
  final LinkPick? onPick;

  const AppLinkScope({
    super.key,
    required this.onOpen,
    this.onPick,
    required super.child,
  });

  static AppLinkOpen? of(BuildContext context) =>
      context.getInheritedWidgetOfExactType<AppLinkScope>()?.onOpen;

  static LinkPick? pickOf(BuildContext context) =>
      context.getInheritedWidgetOfExactType<AppLinkScope>()?.onPick;

  @override
  bool updateShouldNotify(AppLinkScope oldWidget) =>
      onOpen != oldWidget.onOpen || onPick != oldWidget.onPick;
}

/// Follows [link], tapped in a text below [context].
Future<void> followAppLink(BuildContext context, AppLink link) async {
  final open = AppLinkScope.of(context);
  if (open == null) {
    announceNotAvailable(context);
    return;
  }
  await open(context, link);
}

/// Says at the top that a link's target cannot be seen here (#261).
void announceNotAvailable(BuildContext context) =>
    AnnouncementsScope.of(context)?.announce(context.t.linkNotAvailable);

/// Opens what an app link names (#261, ADR 0046): a Note, an Area or
/// Topic, the Agenda, Unsorted, the app's pages, a guide page on the
/// website, or a file of the shared folder after one question. A Note or
/// place this Device does not hold, or holds in the trash or merged,
/// says "Not available here"; one in a closed locked Area asks for the
/// PIN first, as opening the Area does.
class AppLinks {
  final Store store;
  final Notes notes;
  final Places places;
  final BrowserTrail trail;
  final AlarmSync alarms;
  final OwnProfileModel profile;
  final VoidCallback? onThemeChanged;

  /// Shows the Agenda, every page above it gone.
  final void Function(BuildContext context) showAgenda;

  const AppLinks({
    required this.store,
    required this.notes,
    required this.places,
    required this.trail,
    required this.alarms,
    required this.profile,
    required this.showAgenda,
    this.onThemeChanged,
  });

  Future<void> open(BuildContext context, AppLink link) async {
    Future<void> push(Widget page) =>
        Navigator.of(context)
            .push(MaterialPageRoute<void>(builder: (_) => page));
    switch (link.kind) {
      case AppLinkKind.note:
        final note = notes.byId(link.target ?? '');
        if (note == null || note.isTrashed || note.isMerged) {
          return announceNotAvailable(context);
        }
        if (!await _unlocked(context, note.place)) return;
        if (!context.mounted) return;
        await openNote(context, notes, places, note);
      case AppLinkKind.place:
        final id = link.target ?? '';
        final topic = places.topicById(id);
        final area = places.areaById(topic?.areaId ?? id);
        if (area == null) return announceNotAvailable(context);
        if (!await _unlocked(context, area.id)) return;
        if (!context.mounted) return;
        if (topic != null) {
          trail.choseTopic(topic);
          await Navigator.of(context).push(
            BrowserRoute<void>(
              builder: (_) =>
                  TopicScreen(notes: notes, places: places, topicId: topic.id),
            ),
          );
        } else {
          trail.choseArea(area);
          await Navigator.of(context).push(
            BrowserRoute<void>(
              builder: (_) => AreaScreen(
                notes: notes,
                places: places,
                trail: trail,
                areaId: area.id,
              ),
            ),
          );
        }
      case AppLinkKind.agenda:
        showAgenda(context);
      case AppLinkKind.unsorted:
        await openUnsorted(context, notes, places);
      case AppLinkKind.settings:
        await push(
          SettingsScreen(
            store: store,
            alarms: alarms,
            onThemeChanged: onThemeChanged,
            trail: trail,
            section: link.target,
          ),
        );
      case AppLinkKind.routines:
        await openAllRoutines(context, notes, places);
      case AppLinkKind.backups:
        await push(BackupScreen(store: store));
      case AppLinkKind.sync:
        await push(SyncScreen(store: store));
      case AppLinkKind.profile:
        await openProfile(
          context,
          store,
          alarms,
          profile,
          onThemeChanged: onThemeChanged,
          trail: trail,
        );
      case AppLinkKind.frogs:
        await openFrogs(context, notes, places);
      case AppLinkKind.newSession:
        // Sessions are made only in Advanced (#197).
        if (!store.advanced) return announceNotAvailable(context);
        await newSession(context, notes, places);
      case AppLinkKind.guide:
        final language = Localizations.localeOf(context).languageCode;
        final address = Uri.parse(link.guideAddress(language));
        try {
          await LinkOpenScope.of(context)(address);
        } on Object {
          // No browser takes it: the tap does nothing.
        }
      case AppLinkKind.file:
        await openFileLink(context, store, link.target ?? '');
    }
  }

  /// Whether the Area of [place], an Area's or Topic's id, is open now,
  /// asking for the PIN when it is closed; true without one.
  Future<bool> _unlocked(BuildContext context, String? place) async {
    if (place == null) return true;
    final topic = places.topicById(place);
    final area = places.areaById(topic?.areaId ?? place);
    if (area == null) return true;
    return openLockedArea(
      context,
      LockScope.of(context),
      area.id,
      areaName(context.t, area),
    );
  }
}

/// Opens the file at [path] inside the shared folder (#261, ADR 0046)
/// after one question naming it and the app that opens it; a file the
/// folder does not hold, or no shared folder, says "Not available here".
Future<void> openFileLink(
  BuildContext context,
  Store store,
  String path,
) async {
  final folder = store.syncFolder;
  final folders = FolderScope.of(context);
  final app = folder == null ? null : await folders.fileApp(folder, path);
  if (!context.mounted) return;
  if (folder == null || app == null) return announceNotAvailable(context);
  final t = context.t;
  final yes = await showDialog<bool>(
    context: context,
    builder: (context) => AlertDialog(
      title: Text(t.fileLinkTitle),
      content: Text(
        t.fileLinkQuestion(path, app.isEmpty ? t.fileLinkSomeApp : app),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(false),
          child: Text(t.cancel),
        ),
        TextButton(
          key: const ValueKey('fileLinkOpen'),
          onPressed: () => Navigator.of(context).pop(true),
          child: Text(t.fileLinkOpen),
        ),
      ],
    ),
  );
  if (yes != true) return;
  if (!await folders.openFile(folder, path) && context.mounted) {
    announceNotAvailable(context);
  }
}

/// One target of the Link to… picker: what the link opens and the name
/// it is put in with.
typedef LinkTarget = ({AppLink link, String label});

/// The Settings' sections the phone has, by their link's name, with
/// their headings.
List<(String, String)> settingsSections(BuildContext context) {
  final t = context.t;
  return [
    ('look', t.settingsLook),
    ('units', t.settingsUnits),
    ('agenda', t.settingsAgenda),
    ('alarms', t.settingsAlarms),
    ('nudges', t.settingsNudges),
    ('capture', t.settingsCapture),
    ('quick-note', t.settingsQuickNote),
    ('calendar', t.settingsCalendar),
    ('privacy', t.settingsPrivacy),
    ('typing', t.settingsTyping),
    ('history', t.settingsHistory),
    ('import', t.settingsImport),
    ('features', t.features),
  ];
}

/// Opens the Link to… picker (#261) of the app's pages, the Areas and
/// Topics and the Notes, with a search over them; the one picked, or
/// null. Notes of a closed locked Area are not offered.
Future<LinkTarget?> pickLinkTarget(
  BuildContext context,
  Notes notes,
  Places places,
) => Navigator.of(context).push(
  MaterialPageRoute<LinkTarget>(
    fullscreenDialog: true,
    builder: (_) => LinkToScreen(notes: notes, places: places),
  ),
);

/// The Link to… picker (#261).
class LinkToScreen extends StatefulWidget {
  final Notes notes;
  final Places places;

  const LinkToScreen({super.key, required this.notes, required this.places});

  @override
  State<LinkToScreen> createState() => _LinkToScreenState();
}

class _LinkToScreenState extends State<LinkToScreen> {
  final _search = TextEditingController();

  @override
  void dispose() {
    _search.dispose();
    super.dispose();
  }

  List<LinkTarget> _pages(BuildContext context) {
    final t = context.t;
    LinkTarget page(AppLinkKind kind, String label, [String? target]) =>
        (link: AppLink(kind, target), label: label);
    return [
      page(AppLinkKind.agenda, t.agenda),
      page(AppLinkKind.unsorted, t.unsorted),
      page(AppLinkKind.routines, t.allRoutinesTitle),
      page(AppLinkKind.frogs, t.frogsTitle),
      if (widget.notes.store.advanced)
        page(AppLinkKind.newSession, t.newSession),
      page(AppLinkKind.profile, t.profileTitle),
      page(AppLinkKind.backups, t.backupsTitle),
      page(AppLinkKind.sync, t.syncTitle),
      page(AppLinkKind.settings, t.settingsTitle),
      for (final (slug, heading) in settingsSections(context))
        page(AppLinkKind.settings, '${t.settingsTitle} ▸ $heading', slug),
      page(AppLinkKind.guide, t.linkToGuide),
    ];
  }

  List<LinkTarget> _places(BuildContext context) {
    final t = context.t;
    return [
      for (final area in widget.places.store.areasInOrder()) ...[
        (link: AppLink(AppLinkKind.place, area.id), label: areaName(t, area)),
        for (final topic in widget.places.topics(area.id))
          (
            link: AppLink(AppLinkKind.place, topic.id),
            label: '${areaName(t, area)} ▸ ${topicName(t, topic)}',
          ),
      ],
    ];
  }

  List<LinkTarget> _notes(BuildContext context) {
    final lock = LockScope.of(context);
    final places = widget.places;
    return [
      for (final note in widget.notes.all())
        if (_name(note) case final name when name.isNotEmpty)
          if (!_closed(lock, places, note.place))
            (link: AppLink(AppLinkKind.note, note.id), label: name),
    ];
  }

  static bool _closed(AppLock? lock, Places places, String? place) {
    if (lock == null || place == null) return false;
    final area = places.topicById(place)?.areaId ?? place;
    return lock.isAreaClosed(area);
  }

  /// The first line of [note] as it reads.
  static String _name(Note note) => note.visibleText.split('\n').first.trim();

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final query = _search.text.trim().toLowerCase();
    bool shown(LinkTarget target) =>
        query.isEmpty || target.label.toLowerCase().contains(query);
    final groups = [
      (t.linkToPages, _pages(context).where(shown).toList()),
      (t.linkToPlaces, _places(context).where(shown).toList()),
      (t.linkToNotes, _notes(context).where(shown).toList()),
    ];
    final rows = <Widget>[
      for (final (heading, targets) in groups)
        if (targets.isNotEmpty) ...[
          SettingsHeading(heading),
          for (final target in targets)
            ListTile(
              key: ValueKey('linkTo-${target.link.url}'),
              title: Text(target.label),
              onTap: () => Navigator.of(context).pop(target),
            ),
        ],
    ];
    return Scaffold(
      appBar: pageBar(context, title: t.formatLinkTo),
      body: Column(
        children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(16, 8, 16, 0),
            child: TextField(
              key: const ValueKey('linkToSearch'),
              controller: _search,
              autofocus: true,
              decoration: InputDecoration(
                hintText: t.linkToSearch,
                prefixIcon: const Icon(Icons.search),
              ),
              onChanged: (_) => setState(() {}),
            ),
          ),
          Expanded(
            child: rows.isEmpty
                ? Padding(
                    padding: const EdgeInsets.all(24),
                    child: Text(t.linkToNothing),
                  )
                : ListView(children: rows),
          ),
        ],
      ),
    );
  }
}
