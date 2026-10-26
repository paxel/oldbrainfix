import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'announcements.dart';
import 'l10n.dart';
import 'notes_screen.dart';
import 'photo_platform.dart';
import 'profile.dart';
import 'profile_platform.dart';
import 'profile_screen.dart';
import 'settings_screen.dart';
import 'system_bars.dart';

/// What opening another person's Profile needs, wherever their picture
/// shows (#52): the store, and where an Override's picture comes from.
class PersonScope extends InheritedWidget {
  final Store store;
  final ProfilePlatform platform;
  final PictureWork work;

  const PersonScope({
    super.key,
    required this.store,
    required this.platform,
    this.work = PictureWork.isolated,
    required super.child,
  });

  static PersonScope? of(BuildContext context) =>
      context.getInheritedWidgetOfExactType<PersonScope>();

  @override
  bool updateShouldNotify(PersonScope oldWidget) =>
      store != oldWidget.store ||
      platform != oldWidget.platform ||
      work != oldWidget.work;
}

/// Whether the person [id]'s Profile can be opened from here: another
/// person's, inside the app.
bool canOpenPerson(BuildContext context, String id) {
  final scope = PersonScope.of(context);
  return scope != null && !scope.store.isOwnPerson(id);
}

/// Opens the Profile of the person [id]; nothing for the user's own, or
/// outside the app.
Future<void> openPerson(BuildContext context, String id) async {
  final scope = PersonScope.of(context);
  if (scope == null || scope.store.isOwnPerson(id)) return;
  await Navigator.of(context).push(
    MaterialPageRoute<void>(
      builder: (_) => PersonScreen(
        store: scope.store,
        platform: scope.platform,
        work: scope.work,
        id: id,
      ),
    ),
  );
}

/// Another person's Profile (ADR 0017, #52): their own picture and name,
/// their short code and whether the user compared it in person, with
/// "Compare codes"; "Their Notes" (#53); then "Name and picture for
/// me", the Override only the user sees, on their own Devices.
class PersonScreen extends StatefulWidget {
  final Store store;
  final ProfilePlatform platform;
  final PictureWork work;

  /// The person's Profile id.
  final String id;

  const PersonScreen({
    super.key,
    required this.store,
    required this.platform,
    this.work = PictureWork.isolated,
    required this.id,
  });

  @override
  State<PersonScreen> createState() => _PersonScreenState();
}

class _PersonScreenState extends State<PersonScreen> {
  late final _name = TextEditingController(text: _override?.name ?? '');
  final _nameFocus = FocusNode();

  PersonOverride? get _override => widget.store.personOverride(widget.id);

  /// What the page shows of the person, read when it opens and when
  /// their Profile or the Override changes rather than on every build, so
  /// a picture keeps its bytes and is not decoded again.
  late Profile _theirs;
  late Profile _shown;
  String? _code;
  var _verified = false;

  @override
  void initState() {
    super.initState();
    _nameFocus.addListener(_focusChanged);
    widget.store.addProfileListener(_changed);
    _read();
  }

  @override
  void didUpdateWidget(PersonScreen old) {
    super.didUpdateWidget(old);
    if (old.store != widget.store) {
      old.store.removeProfileListener(_changed);
      widget.store.addProfileListener(_changed);
    }
    if (old.store != widget.store || old.id != widget.id) _read();
  }

  void _read() {
    final store = widget.store;
    _theirs = store.personTheirs(widget.id);
    _shown = store.personShown(widget.id);
    _code = store.personCode(widget.id);
    _verified = store.personVerified(widget.id);
  }

  @override
  void dispose() {
    widget.store.removeProfileListener(_changed);
    _nameFocus
      ..removeListener(_focusChanged)
      ..dispose();
    _name.dispose();
    super.dispose();
  }

  /// A newer Profile, or an Override from another of the user's
  /// Devices, shows at once; the name field unless the user is typing.
  void _changed() {
    if (!mounted) return;
    setState(() {
      _read();
      if (_nameFocus.hasFocus) return;
      final kept = _override?.name ?? '';
      if (_name.text != kept) _name.text = kept;
    });
  }

  void _focusChanged() {
    if (!_nameFocus.hasFocus) _saveName();
  }

  /// Keeps the typed name for the user alone, sanitised, and shows it as
  /// kept.
  void _saveName() {
    widget.store.setOverrideName(widget.id, _name.text);
    final kept = _override?.name ?? '';
    if (_name.text != kept) _name.text = kept;
  }

  Future<void> _compare(String name, String code) async {
    final t = context.t;
    final match = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(t.sentItemCompare),
        content: Text(t.sentItemCompareHint(name, code)),
        actions: [
          TextButton(
            key: const ValueKey('codes-differ'),
            onPressed: () => Navigator.of(context).pop(false),
            child: Text(t.sentItemCodesDiffer),
          ),
          TextButton(
            key: const ValueKey('codes-match'),
            onPressed: () => Navigator.of(context).pop(true),
            child: Text(t.sentItemCodesMatch),
          ),
        ],
      ),
    );
    if (!mounted || match == null) return;
    if (!match) {
      AnnouncementsScope.of(context)?.announce(t.personCodesDiffered);
      return;
    }
    widget.store.verifyPerson(widget.id, code);
    setState(_read);
  }

  /// Opens All Notes with only this person's Notes (#53).
  Future<void> _theirNotes() async {
    final store = widget.store;
    await Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) => AllNotesScreen(
          notes: Notes(store),
          places: Places(store),
          people: PersonFilter(people: {widget.id}),
        ),
      ),
    );
  }

  /// Offers where the picture for the user comes from, where the finger
  /// is.
  Future<void> _pictureMenu(Offset at) async {
    final t = context.t;
    final camera = PhotoScope.of(context);
    final overlay =
        Overlay.of(context).context.findRenderObject() as RenderBox?;
    if (overlay == null) return;
    final picked = await showMenu<String>(
      context: context,
      position: RelativeRect.fromRect(
        overlay.globalToLocal(at) & Size.zero,
        Offset.zero & overlay.size,
      ),
      items: [
        if (widget.platform.canPick)
          PopupMenuItem(value: 'pick', child: Text(t.profilePictureChoose)),
        if (camera.available)
          PopupMenuItem(value: 'take', child: Text(t.profilePictureTake)),
        if (_override?.picture != null)
          PopupMenuItem(value: 'theirs', child: Text(t.personTheirPicture)),
      ],
    );
    if (picked == null || !mounted) return;
    if (picked == 'theirs') {
      widget.store.setOverridePicture(widget.id, null);
      setState(_read);
      return;
    }
    final source = picked == 'pick'
        ? await widget.platform.pickPicture()
        : await camera.takePhoto();
    if (source == null || !mounted) return;
    final made = await Navigator.of(context).push(
      MaterialPageRoute<Uint8List>(
        builder: (_) => ProfileCropScreen(source: source, work: widget.work),
      ),
    );
    if (made == null || !mounted) return;
    widget.store.setOverridePicture(widget.id, made);
    setState(_read);
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final theirs = _theirs;
    final shown = _shown;
    final code = _code;
    final verified = _verified;
    final theirName = profileDisplayName(t, theirs.name);
    final text = Theme.of(context).textTheme;
    return PopScope(
      // The name typed last is kept when the page is left.
      onPopInvokedWithResult: (_, _) => _saveName(),
      child: Scaffold(
        appBar: AppBar(title: Text(profileDisplayName(t, shown.name))),
        body: SingleChildScrollView(
          padding: clearOfSystemBars(
            context,
            const EdgeInsets.only(bottom: 24),
          ),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              const SizedBox(height: 16),
              Center(
                child: ProfileAvatar(
                  key: const ValueKey('personPicture'),
                  profile: theirs,
                  size: 112,
                ),
              ),
              const SizedBox(height: 12),
              Center(
                child: Text(
                  theirName,
                  key: const ValueKey('personName'),
                  style: text.headlineSmall,
                  textAlign: TextAlign.center,
                ),
              ),
              if (code != null)
                ListTile(
                  contentPadding: const EdgeInsets.symmetric(horizontal: 24),
                  title: Text(t.profileTheirCode),
                  trailing: SelectableText(
                    code,
                    key: const ValueKey('personCode'),
                    style: text.titleMedium?.copyWith(
                      fontFamily: 'monospace',
                      letterSpacing: 1.5,
                    ),
                  ),
                ),
              ListTile(
                key: const ValueKey('personVerified'),
                contentPadding: const EdgeInsets.symmetric(horizontal: 24),
                leading: Icon(
                  verified ? Icons.verified_user : Icons.gpp_maybe_outlined,
                ),
                title: Text(verified ? t.personVerified : t.personNotVerified),
              ),
              if (!verified && code != null)
                Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 16),
                  child: Align(
                    alignment: AlignmentDirectional.centerStart,
                    child: TextButton.icon(
                      key: const ValueKey('compare-codes'),
                      onPressed: () => _compare(theirName, code),
                      icon: const Icon(Icons.compare_arrows),
                      label: Text(t.sentItemCompare),
                    ),
                  ),
                ),
              ListTile(
                key: const ValueKey('theirNotes'),
                contentPadding: const EdgeInsets.symmetric(horizontal: 24),
                leading: const Icon(Icons.notes),
                title: Text(t.personTheirNotes),
                onTap: _theirNotes,
              ),
              SettingsHeading(t.personForMe),
              Padding(
                padding: const EdgeInsets.fromLTRB(24, 0, 24, 8),
                child: Text(t.personForMeDetail, style: text.bodySmall),
              ),
              Center(
                child: Tooltip(
                  message: t.personForMePicture,
                  child: GestureDetector(
                    key: const ValueKey('overridePicture'),
                    onTapUp: (details) =>
                        unawaited(_pictureMenu(details.globalPosition)),
                    child: ProfileAvatar(profile: shown, size: 72),
                  ),
                ),
              ),
              Padding(
                padding: const EdgeInsets.fromLTRB(24, 8, 24, 0),
                child: TextField(
                  key: const ValueKey('overrideName'),
                  controller: _name,
                  focusNode: _nameFocus,
                  textCapitalization: TextCapitalization.words,
                  textInputAction: TextInputAction.done,
                  decoration: InputDecoration(
                    labelText: t.personForMeName,
                    hintText: theirName,
                  ),
                  onSubmitted: (_) => setState(_saveName),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

/// The People of the Profile page (#52): every Profile kept here, as
/// the user sees them, each opening its Profile.
class PeopleSection extends StatefulWidget {
  final Store store;

  const PeopleSection({super.key, required this.store});

  @override
  State<PeopleSection> createState() => _PeopleSectionState();
}

class _PeopleSectionState extends State<PeopleSection> {
  /// The people as they show, read when their Profiles change rather than
  /// on every build, so a picture keeps its bytes and is not decoded
  /// again.
  late List<Profile> _people;

  @override
  void initState() {
    super.initState();
    widget.store.addProfileListener(_changed);
    _people = widget.store.peopleShown;
  }

  @override
  void didUpdateWidget(PeopleSection old) {
    super.didUpdateWidget(old);
    if (old.store == widget.store) return;
    old.store.removeProfileListener(_changed);
    widget.store.addProfileListener(_changed);
    _people = widget.store.peopleShown;
  }

  @override
  void dispose() {
    widget.store.removeProfileListener(_changed);
    super.dispose();
  }

  void _changed() {
    if (!mounted) return;
    setState(() => _people = widget.store.peopleShown);
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final people = _people;
    return SettingsSection(
      children: [
        if (people.isEmpty)
          ListTile(
            key: const ValueKey('peopleEmpty'),
            leading: const Icon(Icons.people_outline),
            title: Text(t.profilePeopleEmpty),
          ),
        for (final person in people)
          ListTile(
            key: ValueKey('people-${person.id}'),
            leading: ProfileAvatar(profile: person, size: 40),
            title: Text(profileDisplayName(t, person.name)),
            onTap: () => openPerson(context, person.id),
          ),
      ],
    );
  }
}
