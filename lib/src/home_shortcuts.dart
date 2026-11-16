import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'announcements.dart';
import 'l10n.dart';

/// The platform seam for a Tally's shortcut (#284): "Add to home screen"
/// in its card's ⋮ puts one on Android's home screen, or among the
/// actions of the app icon's long press on the iPhone, and a tap on it
/// opens the Tally's entry page (android/…/TallyShortcuts.kt,
/// ios/Runner/CaptureChannel.swift). Widget tests hand in a fake.
abstract interface class HomeShortcuts {
  /// Whether this phone can have a Tally's shortcut at all.
  bool get available;

  /// Asks for a shortcut named [label] to the entry page of the Tally of
  /// [noteId]; false where the home screen takes none.
  Future<bool> pin(String noteId, String label);

  /// The Note of the Tally a shortcut asked for and not opened yet, or
  /// null; taking clears it.
  Future<String?> take();

  /// A shortcut was tapped while the app runs.
  Stream<void> get arrived;

  /// The entry page a shortcut opened closed: back to where the user
  /// was, on Android out of the app, to the home screen. The iPhone lets
  /// no app leave itself, so there it stays in the app.
  Future<void> leave();
}

/// No shortcuts: the default where none is handed in.
class NoHomeShortcuts implements HomeShortcuts {
  const NoHomeShortcuts();

  @override
  bool get available => false;

  @override
  Future<bool> pin(String noteId, String label) async => false;

  @override
  Future<String?> take() async => null;

  @override
  Stream<void> get arrived => const Stream.empty();

  @override
  Future<void> leave() async {}
}

/// The shortcuts behind the app's method channel, on Android and the
/// iPhone alike.
class ChannelHomeShortcuts implements HomeShortcuts {
  static const _channel = MethodChannel(
    'io.github.paxel.oldbrainfix/shortcuts',
  );
  final _arrived = StreamController<void>.broadcast();

  ChannelHomeShortcuts() {
    _channel.setMethodCallHandler((call) async {
      if (call.method == 'requested') _arrived.add(null);
    });
  }

  @override
  bool get available => true;

  @override
  Future<bool> pin(String noteId, String label) async =>
      await _ask<bool>('pinTally', {'note': noteId, 'label': label}) ?? false;

  @override
  Future<String?> take() async => switch (await _ask<String>('take')) {
    final String id when id.isNotEmpty => id,
    _ => null,
  };

  @override
  Stream<void> get arrived => _arrived.stream;

  @override
  Future<void> leave() => _ask<void>('leave');

  static Future<T?> _ask<T>(String method, [Object? args]) async {
    try {
      return await _channel.invokeMethod<T>(method, args);
    } on PlatformException {
      return null;
    } on MissingPluginException {
      return null;
    }
  }
}

/// Hands the app's [HomeShortcuts] down to every screen.
class HomeShortcutsScope extends InheritedWidget {
  final HomeShortcuts shortcuts;

  const HomeShortcutsScope({
    super.key,
    required this.shortcuts,
    required super.child,
  });

  /// The app's [HomeShortcuts]; read once, so it does not listen.
  static HomeShortcuts of(BuildContext context) =>
      context.getInheritedWidgetOfExactType<HomeShortcutsScope>()?.shortcuts ??
      const NoHomeShortcuts();

  @override
  bool updateShouldNotify(HomeShortcutsScope oldWidget) =>
      shortcuts != oldWidget.shortcuts;
}

/// The name of the shortcut to the Tally of [note]: its first line, or
/// "Private Tally" for one in a locked Area set to Masked, the home
/// screen being as public as the lock screen.
String tallyShortcutLabel(AppLocalizations t, Store store, Note note) {
  final area = AreaLocks(store).lockedAreaOf(note);
  if (area != null && store.isAreaMasked(area)) return t.maskedTally;
  final name = noteText(t, note).trim().split('\n').first.trim();
  return name.isEmpty ? t.tallyHeading : name;
}

/// "Add to home screen" of the Tally of [noteId] (#284). Android shows
/// its own prompt to place it; where the home screen takes no shortcut,
/// and on the iPhone where it went, it is said at the top.
Future<void> addTallyToHome(
  BuildContext context,
  Notes notes,
  String noteId,
) async {
  final note = notes.byId(noteId);
  if (note == null) return;
  final t = context.t;
  final announcements = AnnouncementsScope.of(context);
  final label = tallyShortcutLabel(t, notes.store, note);
  final pinned = await HomeShortcutsScope.of(context).pin(noteId, label);
  if (!pinned) {
    announcements?.announce(t.tallyShortcutNotTaken);
  } else if (defaultTargetPlatform == TargetPlatform.iOS) {
    announcements?.announce(t.tallyShortcutOnIcon(label));
  }
}
