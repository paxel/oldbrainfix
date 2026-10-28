import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'announcements.dart';
import 'bundle_platform.dart';
import 'l10n.dart';
import 'notes_screen.dart';
import 'photo_platform.dart';
import 'place_names.dart';
import 'profile.dart';
import 'profile_card.dart';
import 'profile_platform.dart';
import 'profile_screen.dart';
import 'settings_screen.dart';
import 'system_bars.dart';

/// What opening another person's Profile needs, wherever their picture
/// shows (#52): the store, where an Override's picture comes from, and
/// whom to tell that changes were sent to them (#109).
class PersonScope extends InheritedWidget {
  final Store store;
  final ProfilePlatform platform;
  final PictureWork work;
  final VoidCallback? onChangesSent;

  const PersonScope({
    super.key,
    required this.store,
    required this.platform,
    this.work = PictureWork.isolated,
    this.onChangesSent,
    required super.child,
  });

  static PersonScope? of(BuildContext context) =>
      context.getInheritedWidgetOfExactType<PersonScope>();

  @override
  bool updateShouldNotify(PersonScope oldWidget) =>
      store != oldWidget.store ||
      platform != oldWidget.platform ||
      work != oldWidget.work ||
      onChangesSent != oldWidget.onChangesSent;
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
  final route = personRoute(context, id);
  if (route == null) return;
  await Navigator.of(context).push(route);
}

/// The route of the person [id]'s Profile, offering "Send yours back"
/// with [offerBack]; null for the user's own, or outside the app.
Route<void>? personRoute(
  BuildContext context,
  String id, {
  bool offerBack = false,
}) {
  final scope = PersonScope.of(context);
  if (scope == null || scope.store.isOwnPerson(id)) return null;
  return MaterialPageRoute<void>(
    builder: (_) => PersonScreen(
      store: scope.store,
      platform: scope.platform,
      work: scope.work,
      id: id,
      offerBack: offerBack,
      onChangesSent: scope.onChangesSent,
    ),
  );
}

/// Asks once whether to block the person [id], shown as [name] (#90,
/// ADR 0021), and blocks them on a yes. When they manage a Shared Area
/// this Device is in, the dialog names the Areas blocking leaves. The
/// block itself is [block], given whether to leave; it is asked again
/// should an Area to leave come in between. Returns whether the person
/// was blocked.
Future<bool> confirmBlock(
  BuildContext context,
  Store store,
  String id,
  String name, {
  required void Function({required bool leaveAreas}) block,
}) async {
  for (;;) {
    final areas = store.areasToLeaveForBlocking(id);
    final sure = await _askBlock(context, store, name, areas);
    if (!sure || !context.mounted) return false;
    try {
      block(leaveAreas: areas.isNotEmpty);
      return true;
    } on StateError {
      // An Area to leave came in by sync while the dialog was open.
      if (!context.mounted) return false;
    }
  }
}

Future<bool> _askBlock(
  BuildContext context,
  Store store,
  String name,
  List<String> areas,
) async {
  final t = context.t;
  final places = Places(store);
  final names = [
    for (final id in areas)
      if (places.areaById(id) case final area?) areaName(t, area),
  ].join(', ');
  final sure = await showDialog<bool>(
    context: context,
    builder: (context) => AlertDialog(
      title: Text(t.personBlockQuestion(name)),
      content: Text(
        areas.isEmpty
            ? t.personBlockDetail(name)
            : t.personBlockLeaveDetail(name, names),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(false),
          child: Text(t.cancel),
        ),
        TextButton(
          key: const ValueKey('block-confirm'),
          style: TextButton.styleFrom(
            foregroundColor: Theme.of(context).colorScheme.error,
          ),
          onPressed: () => Navigator.of(context).pop(true),
          child: Text(areas.isEmpty ? t.personBlock : t.personBlockLeave),
        ),
      ],
    ),
  );
  return sure ?? false;
}

/// Another person's Profile (ADR 0017, #52): their own picture and name,
/// their short code and whether the user compared it in person, with
/// "Compare codes"; "Their Notes" (#53); then "Name and picture for
/// me", the Override only the user sees, on their own Devices. The bar
/// blocks them, or unblocks a blocked person (#90). Just added from their
/// Profile card, it offers "Send yours back" (#107).
class PersonScreen extends StatefulWidget {
  final Store store;
  final ProfilePlatform platform;
  final PictureWork work;

  /// The person's Profile id.
  final String id;

  /// Whether to offer sending the own Profile card back: they were just
  /// added from a card that answers none of the user's.
  final bool offerBack;

  /// Told when changes went to them by bundle, so the Changes Nudge is
  /// planned anew.
  final VoidCallback? onChangesSent;

  const PersonScreen({
    super.key,
    required this.store,
    required this.platform,
    this.work = PictureWork.isolated,
    required this.id,
    this.offerBack = false,
    this.onChangesSent,
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
  var _blocked = false;

  /// Whether "Send changes" shows: changes travel with them by bundles
  /// (#108), and they are not blocked.
  var _sendsChanges = false;
  var _sending = false;

  /// Whether "Send yours back" shows: until the card went out once.
  late var _offerBack = widget.offerBack;

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
    _blocked = store.isBlocked(widget.id);
    _sendsChanges = store.sendsByBundle(widget.id);
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

  /// Shares the own Profile card back, marked as the answer, so the two
  /// do not ask each other again; once handed over, the offer goes.
  Future<void> _sendBack() async {
    final sent = await sendProfileCard(
      context,
      widget.store,
      widget.platform,
      back: true,
    );
    if (!mounted || !sent) return;
    setState(() => _offerBack = false);
  }

  /// Writes a bundle of what is new in the Areas shared with them, for
  /// all of their Devices at once (#108), and offers it to the app the
  /// user picks; nothing is written when nothing is new for them. Once
  /// an app took it, the Changes Nudge no longer waits for them (#109).
  Future<void> _sendChanges() async {
    setState(() => _sending = true);
    try {
      await sendChanges(
        store: widget.store,
        bundles: BundleScope.of(context),
        t: context.t,
        announcements: AnnouncementsScope.of(context),
        person: widget.id,
        onSent: widget.onChangesSent,
      );
    } finally {
      if (mounted) setState(() => _sending = false);
    }
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

  /// Blocks the person after asking once (#90); the page shows it at
  /// once, as the block tells the Profile listeners.
  Future<void> _block(String name) async {
    final store = widget.store;
    await confirmBlock(
      context,
      store,
      widget.id,
      name,
      block: ({required leaveAreas}) =>
          store.blockPerson(widget.id, leaveAreas: leaveAreas),
    );
  }

  /// Ends the block after asking once: what was refused comes back with
  /// the next syncs.
  Future<void> _unblock(String name) async {
    final t = context.t;
    final sure = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(t.personUnblockQuestion(name)),
        content: Text(t.personUnblockDetail),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: Text(t.cancel),
          ),
          TextButton(
            key: const ValueKey('unblock-confirm'),
            onPressed: () => Navigator.of(context).pop(true),
            child: Text(t.personUnblock),
          ),
        ],
      ),
    );
    if (sure != true || !mounted) return;
    widget.store.unblockPerson(widget.id);
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
    final blocked = _blocked;
    final theirName = profileDisplayName(t, theirs.name);
    final shownName = profileDisplayName(t, shown.name);
    final text = Theme.of(context).textTheme;
    return PopScope(
      // The name typed last is kept when the page is left.
      onPopInvokedWithResult: (_, _) => _saveName(),
      child: Scaffold(
        appBar: AppBar(
          title: Text(shownName),
          actions: [
            if (blocked)
              IconButton(
                key: const ValueKey('unblock-person'),
                tooltip: t.personUnblock,
                onPressed: () => _unblock(shownName),
                icon: const Icon(Icons.how_to_reg_outlined),
              )
            else
              IconButton(
                key: const ValueKey('block-person'),
                tooltip: t.personBlock,
                onPressed: () => _block(shownName),
                icon: const Icon(Icons.block),
              ),
          ],
        ),
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
              if (_offerBack && !blocked) ...[
                Padding(
                  padding: const EdgeInsets.fromLTRB(24, 16, 24, 8),
                  child: Text(t.personSendBackDetail(theirName)),
                ),
                Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 24),
                  child: FilledButton.icon(
                    key: const ValueKey('send-back'),
                    onPressed: _sendBack,
                    icon: const Icon(Icons.reply),
                    label: Text(t.personSendBack),
                  ),
                ),
              ],
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
              if (blocked)
                ListTile(
                  key: const ValueKey('personBlocked'),
                  contentPadding: const EdgeInsets.symmetric(horizontal: 24),
                  leading: const Icon(Icons.block),
                  title: Text(t.personBlocked),
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
              if (_sendsChanges)
                ListTile(
                  key: const ValueKey('sendChanges'),
                  contentPadding: const EdgeInsets.symmetric(horizontal: 24),
                  leading: const Icon(Icons.send_outlined),
                  title: Text(t.personSendChanges(shownName)),
                  subtitle: Text(t.personSendChangesDetail),
                  enabled: !_sending,
                  onTap: _sendChanges,
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

/// The People of the Profile page (#52): "Add a person" from their
/// Profile card (#107), then every Profile kept here, as the user sees
/// them, each opening its Profile. The blocked people
/// (#90) are left out and listed in a folded "Blocked" section below,
/// closed until opened; whether it is open is a setting of this Device.
class PeopleSection extends StatefulWidget {
  final Store store;

  const PeopleSection({super.key, required this.store});

  @override
  State<PeopleSection> createState() => _PeopleSectionState();
}

class _PeopleSectionState extends State<PeopleSection> {
  /// The local setting that keeps the "Blocked" section open.
  static const _blockedOpenKey = 'peopleBlockedOpen';

  /// The people as they show, read when their Profiles change rather than
  /// on every build, so a picture keeps its bytes and is not decoded
  /// again.
  late List<Profile> _people;
  late List<Profile> _blocked;
  late bool _blockedOpen;

  @override
  void initState() {
    super.initState();
    widget.store.addProfileListener(_changed);
    _read();
  }

  void _read() {
    final store = widget.store;
    _people = store.peopleShown;
    _blocked = store.blockedPeople;
    _blockedOpen = store.localSetting(_blockedOpenKey) == 'true';
  }

  void _setBlockedOpen(bool open) {
    _blockedOpen = open;
    widget.store.setLocalSetting(_blockedOpenKey, '$open');
  }

  @override
  void didUpdateWidget(PeopleSection old) {
    super.didUpdateWidget(old);
    if (old.store == widget.store) return;
    old.store.removeProfileListener(_changed);
    widget.store.addProfileListener(_changed);
    _read();
  }

  @override
  void dispose() {
    widget.store.removeProfileListener(_changed);
    super.dispose();
  }

  void _changed() {
    if (!mounted) return;
    setState(_read);
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final people = _people;
    final blocked = _blocked;
    return SettingsSection(
      children: [
        ListTile(
          key: const ValueKey('addPerson'),
          leading: const Icon(Icons.person_add_alt_1_outlined),
          title: Text(t.personAdd),
          subtitle: Text(t.personAddDetail),
          onTap: () => Navigator.of(context).push(
            MaterialPageRoute<void>(
              builder: (_) => AddPersonScreen(store: widget.store),
            ),
          ),
        ),
        if (people.isEmpty && blocked.isEmpty)
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
        if (blocked.isNotEmpty)
          ExpansionTile(
            key: const ValueKey('peopleBlocked'),
            shape: const Border(),
            collapsedShape: const Border(),
            leading: const Icon(Icons.block),
            title: Text(t.peopleBlocked),
            initiallyExpanded: _blockedOpen,
            onExpansionChanged: _setBlockedOpen,
            children: [
              for (final person in blocked)
                ListTile(
                  key: ValueKey('blocked-${person.id}'),
                  leading: ProfileAvatar(profile: person, size: 40),
                  title: Text(profileDisplayName(t, person.name)),
                  onTap: () => openPerson(context, person.id),
                ),
            ],
          ),
      ],
    );
  }
}
