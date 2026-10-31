import 'dart:async';
import 'dart:isolate';
import 'dart:math' as math;
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'alarm_sync.dart';
import 'backup_screen.dart';
import 'failure_dialog.dart';
import 'l10n.dart';
import 'person_screen.dart';
import 'photo_platform.dart';
import 'profile.dart';
import 'profile_card.dart';
import 'settings_screen.dart';
import 'sync_screen.dart';
import 'system_bars.dart';

/// Opens the Profile page; returns once the user left it.
Future<void> openProfile(
  BuildContext context,
  Store store,
  AlarmSync alarms,
  OwnProfileModel model, {
  VoidCallback? onThemeChanged,
}) => Navigator.of(context).push(
  MaterialPageRoute<void>(
    builder: (_) => ProfileScreen(
      store: store,
      alarms: alarms,
      model: model,
      onThemeChanged: onThemeChanged,
    ),
  ),
);

/// What a picture is read and made with; the app does it off the main
/// isolate, tests hand in their own.
class PictureWork {
  /// The width and height of the picture, or null when it is none.
  final Future<({int width, int height})?> Function(Uint8List bytes) size;

  /// The Profile picture of the square at left, top with the edge size.
  final Future<Uint8List?> Function(
    Uint8List bytes,
    int left,
    int top,
    int size,
  )
  make;

  const PictureWork({required this.size, required this.make});

  static Future<({int width, int height})?> _size(Uint8List bytes) =>
      Isolate.run(() => pictureSize(bytes));

  static Future<Uint8List?> _make(
    Uint8List bytes,
    int left,
    int top,
    int size,
  ) => Isolate.run(
    () => makeProfilePicture(bytes, left: left, top: top, size: size),
  );

  /// Off the main isolate, so the page keeps moving meanwhile.
  static const isolated = PictureWork(size: _size, make: _make);
}

/// The Profile page (ADR 0017): the own Profile's picture, name and
/// short code, "Send my Profile" and "Show my QR" (#107), then Backups
/// and recovery, the user's Devices, People
/// (#52), and the settings of this Device. Reached through the Profile picture in the
/// Agenda's top corner, where the settings gear was.
class ProfileScreen extends StatefulWidget {
  final Store store;
  final AlarmSync alarms;
  final OwnProfileModel model;
  final PictureWork work;

  /// Called after the theme was changed in the settings, to show it.
  final VoidCallback? onThemeChanged;

  const ProfileScreen({
    super.key,
    required this.store,
    required this.alarms,
    required this.model,
    this.work = PictureWork.isolated,
    this.onThemeChanged,
  });

  @override
  State<ProfileScreen> createState() => _ProfileScreenState();
}

class _ProfileScreenState extends State<ProfileScreen> {
  late final _name = TextEditingController(
    text: widget.model.profile?.name ?? '',
  );
  final _nameFocus = FocusNode();

  @override
  void initState() {
    super.initState();
    _nameFocus.addListener(_focusChanged);
    widget.model.addListener(_profileChanged);
  }

  @override
  void dispose() {
    widget.model.removeListener(_profileChanged);
    _nameFocus
      ..removeListener(_focusChanged)
      ..dispose();
    _name.dispose();
    super.dispose();
  }

  /// A name that came from another Device shows at once, unless the user
  /// is typing one.
  void _profileChanged() {
    if (_nameFocus.hasFocus) return;
    final kept = widget.model.profile?.name ?? '';
    if (_name.text != kept) _name.text = kept;
  }

  void _focusChanged() {
    if (!_nameFocus.hasFocus) _saveName();
  }

  /// Keeps the typed name, sanitised, and shows it as kept.
  void _saveName() {
    final model = widget.model;
    if (model.profile == null) return;
    model.rename(_name.text);
    final kept = model.profile?.name ?? '';
    if (_name.text != kept) _name.text = kept;
  }

  Future<void> _push(Widget screen) =>
      Navigator.of(context)
          .push(MaterialPageRoute<void>(builder: (_) => screen));

  /// Offers where the picture comes from, where the finger is.
  Future<void> _pictureMenu(Offset at) async {
    final t = context.t;
    final platform = widget.model.platform;
    final camera = PhotoScope.of(context);
    final overlay =
        Overlay.of(context).context.findRenderObject() as RenderBox?;
    if (overlay == null) return;
    final hasPicture = widget.model.profile?.picture != null;
    final picked = await showMenu<String>(
      context: context,
      position: RelativeRect.fromRect(
        overlay.globalToLocal(at) & Size.zero,
        Offset.zero & overlay.size,
      ),
      items: [
        if (platform.canPick)
          PopupMenuItem(value: 'pick', child: Text(t.profilePictureChoose)),
        if (camera.available)
          PopupMenuItem(value: 'take', child: Text(t.profilePictureTake)),
        if (hasPicture)
          PopupMenuItem(
            value: 'initials',
            child: Text(t.profilePictureInitials),
          ),
      ],
    );
    if (picked == null || !mounted) return;
    if (picked == 'initials') {
      widget.model.setPicture(null);
      return;
    }
    final source = picked == 'pick'
        ? await platform.pickPicture()
        : await camera.takePhoto();
    if (source == null || !mounted) return;
    final made = await Navigator.of(context).push(
      MaterialPageRoute<Uint8List>(
        builder: (_) => ProfileCropScreen(source: source, work: widget.work),
      ),
    );
    if (made == null || !mounted) return;
    widget.model.setPicture(made);
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final model = widget.model;
    return PopScope(
      // The name typed last is kept when the page is left.
      onPopInvokedWithResult: (_, _) => _saveName(),
      child: Scaffold(
        appBar: AppBar(title: Text(t.profileTitle)),
        body: ListenableBuilder(
          listenable: model,
          builder: (context, _) => SingleChildScrollView(
            padding: clearOfSystemBars(
              context,
              const EdgeInsets.only(bottom: 24),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                const SizedBox(height: 16),
                Center(
                  child: Tooltip(
                    message: t.profilePictureChange,
                    child: GestureDetector(
                      key: const ValueKey('profilePicture'),
                      onTapUp: model.newer
                          ? null
                          : (details) =>
                                unawaited(_pictureMenu(details.globalPosition)),
                      child: ProfileAvatar(profile: model.profile, size: 112),
                    ),
                  ),
                ),
                Padding(
                  padding: const EdgeInsets.fromLTRB(24, 16, 24, 0),
                  child: TextField(
                    key: const ValueKey('profileName'),
                    controller: _name,
                    focusNode: _nameFocus,
                    // Saved by a newer version of the app: shown, not
                    // changed here (ADR 0034).
                    readOnly: model.newer,
                    textCapitalization: TextCapitalization.words,
                    textInputAction: TextInputAction.done,
                    decoration: InputDecoration(
                      labelText: t.profileName,
                      hintText: t.profileNoName,
                    ),
                    onSubmitted: (_) => _saveName(),
                  ),
                ),
                if (model.newer)
                  Padding(
                    key: const ValueKey('profileNewer'),
                    padding: const EdgeInsets.fromLTRB(24, 8, 24, 0),
                    child: Text(t.newerVersionNeeded),
                  ),
                if (model.question != null)
                  ListTile(
                    key: const ValueKey('profileQuestion'),
                    contentPadding: const EdgeInsets.symmetric(horizontal: 24),
                    leading: const Icon(Icons.people_outline),
                    title: Text(t.profileWhichTitle),
                    subtitle: Text(t.profileWhichWaiting),
                    onTap: () =>
                        unawaited(askWhichProfile(context, widget.store)),
                  ),
                if (model.code case final code?)
                  ListTile(
                    contentPadding: const EdgeInsets.symmetric(horizontal: 24),
                    title: Text(t.profileCode),
                    trailing: SelectableText(
                      code,
                      key: const ValueKey('profileCode'),
                      style: Theme.of(context).textTheme.titleMedium?.copyWith(
                        fontFamily: 'monospace',
                        letterSpacing: 1.5,
                      ),
                    ),
                  ),
                SettingsSection(
                  children: [
                    ListTile(
                      key: const ValueKey('sendProfile'),
                      leading: const Icon(Icons.share_outlined),
                      title: Text(t.profileCardSend),
                      subtitle: Text(t.profileCardSendDetail),
                      onTap: () =>
                          unawaited(sendProfileCard(context, widget.store)),
                    ),
                    ListTile(
                      key: const ValueKey('showProfileQr'),
                      leading: const Icon(Icons.qr_code_2),
                      title: Text(t.profileCardShowQr),
                      subtitle: Text(t.profileCardShowQrDetail),
                      onTap: () =>
                          unawaited(showProfileQr(context, widget.store)),
                    ),
                  ],
                ),
                SettingsHeading(t.profileBackups),
                SettingsSection(
                  children: [
                    ListTile(
                      key: const ValueKey('backups'),
                      leading: const Icon(Icons.backup_outlined),
                      title: Text(t.backupsSettingsEntry),
                      subtitle: Text(t.backupsSettingsDetail),
                      onTap: () => _push(BackupScreen(store: widget.store)),
                    ),
                  ],
                ),
                SettingsHeading(t.profileDevices),
                SettingsSection(
                  children: [
                    ListTile(
                      key: const ValueKey('sync'),
                      leading: const Icon(Icons.devices_outlined),
                      title: Text(t.syncSettingsEntry),
                      subtitle: Text(t.syncSettingsDetail),
                      onTap: () => _push(SyncScreen(store: widget.store)),
                    ),
                  ],
                ),
                SettingsHeading(t.peopleTitle),
                PeopleSection(store: widget.store),
                SettingsHeading(t.profileDeviceSettings),
                SettingsSection(
                  children: [
                    ListTile(
                      key: const ValueKey('deviceSettings'),
                      leading: const Icon(Icons.settings_outlined),
                      title: Text(t.settingsTitle),
                      subtitle: Text(t.profileDeviceSettingsDetail),
                      onTap: () => _push(
                        SettingsScreen(
                          store: widget.store,
                          alarms: widget.alarms,
                          onThemeChanged: widget.onThemeChanged,
                        ),
                      ),
                    ),
                  ],
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

/// Picks the square of [source] that becomes the Profile picture: the
/// picture whole, a square on it that follows the finger and a slider
/// for its size. Pops with the made Profile picture, or nothing.
class ProfileCropScreen extends StatefulWidget {
  final Uint8List source;
  final PictureWork work;

  const ProfileCropScreen({
    super.key,
    required this.source,
    this.work = PictureWork.isolated,
  });

  @override
  State<ProfileCropScreen> createState() => _ProfileCropScreenState();
}

class _ProfileCropScreenState extends State<ProfileCropScreen> {
  ({int width, int height})? _size;
  var _left = 0.0, _top = 0.0, _edge = 0.0;
  var _busy = false;

  @override
  void initState() {
    super.initState();
    unawaited(_read());
  }

  Future<void> _read() async {
    final size = await widget.work.size(widget.source);
    if (!mounted) return;
    if (size == null) {
      _unreadable();
      return;
    }
    final shortest = math.min(size.width, size.height).toDouble();
    setState(() {
      _size = size;
      _edge = shortest;
      _left = (size.width - shortest) / 2;
      _top = (size.height - shortest) / 2;
    });
  }

  void _unreadable() {
    final t = context.t;
    announceFailure(
      context,
      t.profilePictureNotSet,
      title: t.profilePictureNotSetTitle,
      cause: t.profilePictureUnreadable,
      fix: t.profilePictureFix,
      details: 'profile picture: its size cannot be read',
    );
    Navigator.of(context).pop();
  }

  /// The square kept inside the picture.
  void _place(double left, double top, double edge) {
    final size = _size;
    if (size == null) return;
    setState(() {
      _edge = edge;
      _left = left.clamp(0, size.width - edge);
      _top = top.clamp(0, size.height - edge);
    });
  }

  Future<void> _use() async {
    if (_busy) return;
    setState(() => _busy = true);
    final made = await widget.work.make(
      widget.source,
      _left.round(),
      _top.round(),
      _edge.round(),
    );
    if (!mounted) return;
    if (made == null) {
      _unreadable();
      return;
    }
    Navigator.of(context).pop(made);
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final size = _size;
    return Scaffold(
      appBar: AppBar(title: Text(t.profileCropTitle)),
      body: size == null
          ? const Center(child: CircularProgressIndicator())
          : Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Expanded(
                  child: Padding(
                    padding: const EdgeInsets.all(16),
                    child: LayoutBuilder(
                      builder: (context, box) {
                        final scale = math.min(
                          box.maxWidth / size.width,
                          box.maxHeight / size.height,
                        );
                        return Center(
                          child: SizedBox(
                            width: size.width * scale,
                            height: size.height * scale,
                            child: GestureDetector(
                              key: const ValueKey('cropArea'),
                              onPanUpdate: (d) => _place(
                                _left + d.delta.dx / scale,
                                _top + d.delta.dy / scale,
                                _edge,
                              ),
                              child: Stack(
                                fit: StackFit.expand,
                                children: [
                                  Image.memory(
                                    widget.source,
                                    fit: BoxFit.fill,
                                    gaplessPlayback: true,
                                  ),
                                  CustomPaint(
                                    painter: _SquarePainter(
                                      Rect.fromLTWH(
                                        _left * scale,
                                        _top * scale,
                                        _edge * scale,
                                        _edge * scale,
                                      ),
                                      Theme.of(context).colorScheme.primary,
                                    ),
                                  ),
                                ],
                              ),
                            ),
                          ),
                        );
                      },
                    ),
                  ),
                ),
                Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 24),
                  child: Text(t.profileCropHint),
                ),
                Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 12),
                  child: Slider(
                    key: const ValueKey('cropSize'),
                    semanticFormatterCallback: (_) => t.profileCropSize,
                    min: math.min(64, _shortest(size)).toDouble(),
                    max: _shortest(size).toDouble(),
                    value: _edge.clamp(
                      math.min(64, _shortest(size)).toDouble(),
                      _shortest(size).toDouble(),
                    ),
                    onChanged: (edge) => _place(
                      _left + (_edge - edge) / 2,
                      _top + (_edge - edge) / 2,
                      edge,
                    ),
                  ),
                ),
                Padding(
                  padding: const EdgeInsets.fromLTRB(24, 0, 24, 24),
                  child: FilledButton(
                    key: const ValueKey('cropUse'),
                    onPressed: _busy ? null : _use,
                    child: Text(t.profileCropUse),
                  ),
                ),
              ],
            ),
    );
  }

  static int _shortest(({int width, int height}) size) =>
      math.min(size.width, size.height);
}

/// Dims the picture outside the [square] and outlines it.
class _SquarePainter extends CustomPainter {
  final Rect square;
  final Color color;

  const _SquarePainter(this.square, this.color);

  @override
  void paint(Canvas canvas, Size size) {
    final outside = Path()
      ..fillType = PathFillType.evenOdd
      ..addRect(Offset.zero & size)
      ..addRect(square);
    canvas
      ..drawPath(outside, Paint()..color = const Color(0x99000000))
      ..drawRect(
        square,
        Paint()
          ..color = color
          ..style = PaintingStyle.stroke
          ..strokeWidth = 2,
      );
  }

  @override
  bool shouldRepaint(_SquarePainter old) =>
      old.square != square || old.color != color;
}
