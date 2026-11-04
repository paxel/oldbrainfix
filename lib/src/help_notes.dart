import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'frogs_screen.dart' show FrogIcon;
import 'note_editor_screen.dart' show kindBadgeIcon;

/// One icon the help Note "What the icons mean" explains (#180): the icon
/// the app draws, its name and what it means, in words.
class HelpIcon {
  /// The icon, or null where [drawn] draws one that is no [IconData],
  /// such as the frog.
  final IconData? icon;
  final Widget? drawn;
  final String Function(AppLocalizations t) name;
  final String Function(AppLocalizations t) means;

  const HelpIcon(IconData this.icon, this.name, this.means) : drawn = null;

  const HelpIcon.drawn(Widget this.drawn, this.name, this.means) : icon = null;
}

/// A group of [HelpIcon]s: a heading of the Note and one picture.
class HelpIconGroup {
  /// The picture's file name part, `assets/help/icons-<slug>-<lang>.png`.
  final String slug;
  final String Function(AppLocalizations t) heading;
  final List<HelpIcon> icons;

  const HelpIconGroup(this.slug, this.heading, this.icons);

  /// The picture of this group in [language].
  String asset(String language) => 'assets/help/icons-$slug-$language.png';
}

String _kind(AppLocalizations t) => t.helpIconKind;
String _kindMeans(AppLocalizations t) => t.helpIconKindMeans;
String _importance(AppLocalizations t) => t.helpIconImportance;
String _importanceMeans(AppLocalizations t) => t.helpIconImportanceMeans;
String _place(AppLocalizations t) => t.helpIconPlace;
String _placeMeans(AppLocalizations t) => t.helpIconPlaceMeans;
String _captured(AppLocalizations t) => t.helpIconCaptured;
String _capturedMeans(AppLocalizations t) => t.helpIconCapturedMeans;
String _reminder(AppLocalizations t) => t.helpIconReminder;
String _reminderMeans(AppLocalizations t) => t.helpIconReminderMeans;
String _routine(AppLocalizations t) => t.helpIconRoutine;
String _routineMeans(AppLocalizations t) => t.helpIconRoutineMeans;
String _frog(AppLocalizations t) => t.helpIconFrog;
String _frogMeans(AppLocalizations t) => t.helpIconFrogMeans;
String _add(AppLocalizations t) => t.helpIconAdd;
String _addMeans(AppLocalizations t) => t.helpIconAddMeans;
String _browse(AppLocalizations t) => t.helpIconBrowse;
String _browseMeans(AppLocalizations t) => t.helpIconBrowseMeans;
String _allRoutines(AppLocalizations t) => t.helpIconAllRoutines;
String _allRoutinesMeans(AppLocalizations t) => t.helpIconAllRoutinesMeans;
String _newRoutine(AppLocalizations t) => t.helpIconNewRoutine;
String _newRoutineMeans(AppLocalizations t) => t.helpIconNewRoutineMeans;
String _newSession(AppLocalizations t) => t.helpIconNewSession;
String _newSessionMeans(AppLocalizations t) => t.helpIconNewSessionMeans;
String _unsorted(AppLocalizations t) => t.helpIconUnsorted;
String _unsortedMeans(AppLocalizations t) => t.helpIconUnsortedMeans;
String _fileInto(AppLocalizations t) => t.helpIconFileInto;
String _fileIntoMeans(AppLocalizations t) => t.helpIconFileIntoMeans;
String _alarm(AppLocalizations t) => t.helpIconAlarm;
String _alarmMeans(AppLocalizations t) => t.helpIconAlarmMeans;
String _merge(AppLocalizations t) => t.helpIconMerge;
String _mergeMeans(AppLocalizations t) => t.helpIconMergeMeans;
String _more(AppLocalizations t) => t.helpIconMore;
String _moreMeans(AppLocalizations t) => t.helpIconMoreMeans;
String _badges(AppLocalizations t) => t.helpIconsBadges;
String _agenda(AppLocalizations t) => t.helpIconsAgenda;
String _card(AppLocalizations t) => t.helpIconsCard;

/// The icons the help Note explains, by group, as the screens draw them:
/// the Note editor's badges, the Agenda's bar and banner, a Note card's
/// buttons. The desktop's `help_notes.rs` names the same keys.
const helpIconGroups = [
  HelpIconGroup('badges', _badges, [
    HelpIcon(kindBadgeIcon, _kind, _kindMeans),
    HelpIcon(Icons.label_important_outline, _importance, _importanceMeans),
    HelpIcon(Icons.folder_outlined, _place, _placeMeans),
    HelpIcon(Icons.schedule, _captured, _capturedMeans),
    HelpIcon(Icons.notifications_outlined, _reminder, _reminderMeans),
    HelpIcon(Icons.repeat, _routine, _routineMeans),
    HelpIcon.drawn(FrogIcon(size: 28), _frog, _frogMeans),
    HelpIcon(Icons.add, _add, _addMeans),
  ]),
  HelpIconGroup('agenda', _agenda, [
    HelpIcon(Icons.folder_outlined, _browse, _browseMeans),
    HelpIcon(Icons.format_list_bulleted, _allRoutines, _allRoutinesMeans),
    HelpIcon(Icons.add_alarm, _newRoutine, _newRoutineMeans),
    HelpIcon(Icons.event_repeat, _newSession, _newSessionMeans),
    HelpIcon(Icons.move_to_inbox, _unsorted, _unsortedMeans),
  ]),
  HelpIconGroup('card', _card, [
    HelpIcon(Icons.drive_file_move_outline, _fileInto, _fileIntoMeans),
    HelpIcon(Icons.alarm_add_outlined, _alarm, _alarmMeans),
    HelpIcon(Icons.call_merge, _merge, _mergeMeans),
    HelpIcon(Icons.more_vert, _more, _moreMeans),
  ]),
];

/// The Lines of the Quick start, in [t]'s language.
List<String> quickStartLines(AppLocalizations t) => [
  t.helpQuickStart1,
  t.helpQuickStart2,
  t.helpQuickStart3,
  t.helpQuickStart4,
  t.helpQuickStart5,
  t.helpQuickStart6,
  t.helpQuickStart7,
  t.helpQuickStart8,
];

/// The Quick start's Description in [t]'s language (#260), folded under
/// its name: "On the desktop", then the desktop's way for each Line, one
/// line each, in the Lines' order.
String quickStartDescription(AppLocalizations t) => [
  t.helpQuickStartDesktopTitle,
  t.helpQuickStartDesktop1,
  t.helpQuickStartDesktop2,
  t.helpQuickStartDesktop3,
  t.helpQuickStartDesktop4,
  t.helpQuickStartDesktop5,
  t.helpQuickStartDesktop6,
  t.helpQuickStartDesktop7,
  t.helpQuickStartDesktop8,
].join('\n');

/// The text of "What the icons mean": its name, then each group's
/// heading and its icons, "name: meaning" one per line, so search finds
/// what the pictures show.
String helpIconsText(AppLocalizations t) {
  final out = StringBuffer(t.helpIconsTitle);
  for (final group in helpIconGroups) {
    out.write('\n\n${group.heading(t)}');
    for (final icon in group.icons) {
      out.write('\n${icon.name(t)}: ${icon.means(t)}');
    }
  }
  return out.toString();
}

/// Where the help Notes' bundled parts come from: the What's new of
/// every release and the icon pictures (#180). The app reads its assets;
/// tests hand in their own or none, so no help Note appears in them.
abstract class HelpSource {
  /// Each release's Play notes by version, then by language.
  Future<Map<String, Map<String, String>>> whatsNew();

  /// The bytes of the asset [path].
  Future<Uint8List> picture(String path);
}

/// The help Notes' parts as the app bundles them:
/// `assets/help/whats_new.json` (written by `tool/whats_new.py --bundle`
/// from the release notes) and the icon pictures beside it.
class BundledHelp implements HelpSource {
  const BundledHelp();

  @override
  Future<Map<String, Map<String, String>>> whatsNew() async =>
      parseWhatsNew(await rootBundle.loadString('assets/help/whats_new.json'));

  @override
  Future<Uint8List> picture(String path) async =>
      (await rootBundle.load(path)).buffer.asUint8List();
}

/// What the app makes its help Notes with (#180): their bundled parts
/// and the version it runs.
class HelpSetup {
  final HelpSource source;
  final String version;

  const HelpSetup(this.source, {required this.version});
}

/// The bundled What's new as both apps read it.
Map<String, Map<String, String>> parseWhatsNew(String json) => {
  for (final MapEntry(:key, :value)
      in (jsonDecode(json) as Map<String, Object?>).entries)
    if (value is Map<String, Object?>)
      key: {
        for (final MapEntry(:key, :value) in value.entries)
          if (value is String) key: value,
      },
};

/// Makes the help Notes due on this Device (#180) in [t]'s language,
/// running [version]; [fresh] right after the first start. Called once
/// the own Profile exists.
Future<List<String>> makeHelpNotes(
  Store store,
  HelpSource source,
  AppLocalizations t, {
  required String version,
  required bool fresh,
}) async {
  // Read only when the version changed: every other start costs nothing.
  if (store.localSetting(HelpNotes.versionSetting) == version) return const [];
  final language = t.localeName;
  final whatsNew = await source.whatsNew();
  final pictures = [
    for (final group in helpIconGroups)
      await source.picture(group.asset(language)),
  ];
  return HelpNotes(store).makeDue(
    version: version,
    fresh: fresh,
    language: language,
    whatsNew: whatsNew,
    texts: HelpTexts(
      areaName: t.helpAreaName,
      quickStartTitle: t.helpQuickStartTitle,
      quickStartLines: quickStartLines(t),
      quickStartDescription: quickStartDescription(t),
      iconsText: helpIconsText(t),
      iconPictures: pictures,
      whatsNewTitle: (v) => t.helpWhatsNewTitle(v),
    ),
  );
}

/// The picture of the icon group [group] that "What the icons mean"
/// carries (#180): each icon as the screens draw it, with its name and
/// meaning, on a light background. The screenshots generator renders it
/// into `assets/help/` ([HelpIconGroup.asset]); the app never shows it.
class HelpIconsPicture extends StatelessWidget {
  final HelpIconGroup group;

  const HelpIconsPicture({super.key, required this.group});

  /// The picture's width and the height of its heading and of each row,
  /// in logical pixels.
  static const width = 360.0;
  static const headingHeight = 56.0;
  static const rowHeight = 64.0;

  /// The picture's size for [group].
  static Size sizeOf(HelpIconGroup group) =>
      Size(width, headingHeight + rowHeight * group.icons.length + 16);

  @override
  Widget build(BuildContext context) {
    final t = AppLocalizations.of(context);
    final theme = Theme.of(context);
    final scheme = theme.colorScheme;
    return ColoredBox(
      color: scheme.surface,
      child: Padding(
        padding: const EdgeInsets.fromLTRB(16, 0, 16, 16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            SizedBox(
              height: headingHeight,
              child: Align(
                alignment: Alignment.centerLeft,
                child: Text(
                  group.heading(t),
                  style: theme.textTheme.titleMedium,
                ),
              ),
            ),
            for (final icon in group.icons)
              SizedBox(
                height: rowHeight,
                child: Row(
                  children: [
                    switch (icon.drawn) {
                      // In the icons' square; the picture has no Material
                      // to give its text a style.
                      final drawn? => SizedBox.square(
                        dimension: 28,
                        child: FittedBox(
                          child: DefaultTextStyle(
                            style:
                                theme.textTheme.bodyMedium ?? const TextStyle(),
                            child: drawn,
                          ),
                        ),
                      ),
                      null => Icon(icon.icon, size: 28, color: scheme.primary),
                    },
                    const SizedBox(width: 16),
                    Expanded(
                      child: Column(
                        mainAxisAlignment: MainAxisAlignment.center,
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            icon.name(t),
                            style: theme.textTheme.titleSmall,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                          ),
                          Text(
                            icon.means(t),
                            style: theme.textTheme.bodySmall,
                            maxLines: 2,
                            overflow: TextOverflow.ellipsis,
                          ),
                        ],
                      ),
                    ),
                  ],
                ),
              ),
          ],
        ),
      ),
    );
  }
}
