import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'announcements.dart';
import 'auto_sync.dart';
import 'l10n.dart';
import 'pairing_platform.dart';
import 'person_screen.dart';
import 'photo_platform.dart';
import 'place_names.dart';
import 'profile.dart';
import 'sync_screen.dart';

/// How an invite is run; the app passes the core's [lanSync], tests a
/// fake.
typedef InviteJoin = Future<SyncResult> Function(
  Store store,
  PairInfo info, {
  required String name,
  Invite? invite,
  Future<bool> Function(ProfileChoice other)? confirm,
});

/// Shares an Area with another person (ADR 0013): the role they get,
/// and the pair code their computer shows, scanned or typed. Nobody types
/// a name: once the phone reached their computer, it shows their Profile
/// (ADR 0017), and only when the user confirms it does the phone invite
/// it, so their computer gets this Area and nothing else, and the Area
/// shows as shared from then on. A Starter Set Area goes on under fresh
/// ids; [onShared] hears the id it has now.
class ShareAreaScreen extends StatefulWidget {
  final Store store;
  final String areaId;
  final InviteJoin join;
  final ValueChanged<String>? onShared;

  const ShareAreaScreen({
    super.key,
    required this.store,
    required this.areaId,
    this.join = lanSync,
    this.onShared,
  });

  @override
  State<ShareAreaScreen> createState() => _ShareAreaScreenState();
}

class _ShareAreaScreenState extends State<ShareAreaScreen> {
  final _code = TextEditingController();
  var _role = AreaRole.member;
  var _busy = false;
  late String _areaId = widget.areaId;
  late final Area? _area = Places(widget.store).areaById(widget.areaId);

  @override
  void dispose() {
    _code.dispose();
    super.dispose();
  }

  void _announce(String text) => AnnouncementsScope.of(context)?.announce(text);

  /// Takes a photo of their computer's screen and reads the code in it.
  Future<void> _scan() async {
    final photos = PhotoScope.of(context);
    final pairing = PairingScope.of(context);
    final photo = await photos.takePhoto();
    if (!mounted || photo == null) return;
    final code = await pairing.readQr(photo);
    if (!mounted) return;
    if (code == null || decodePairCode(code) == null) {
      _announce(context.t.syncNoCode);
      return;
    }
    _code.text = code;
    await _invite();
  }

  /// Shows the other person's Profile; true once the user confirms it.
  Future<bool> _confirm(ProfileChoice other) async {
    if (!mounted) return false;
    final area = _area;
    final sure = await showDialog<bool>(
      context: context,
      builder: (_) => InviteConfirmDialog(
        area: area == null ? '' : areaName(context.t, area),
        other: other,
      ),
    );
    return sure ?? false;
  }

  Future<void> _invite() async {
    final t = context.t;
    final info = decodePairCode(_code.text);
    if (info == null) {
      _announce(t.syncInvalidCode);
      return;
    }
    final pairing = PairingScope.of(context);
    final area = _area;
    setState(() => _busy = true);
    ProfileChoice? confirmed;
    try {
      final store = widget.store;
      _areaId = store.share(_areaId);
      widget.onShared?.call(_areaId);
      final name = await pairing.deviceName();
      await widget.join(
        store,
        info,
        name: name,
        invite: Invite(_areaId, role: _role),
        confirm: (other) async {
          final sure = await _confirm(other);
          if (sure) confirmed = other;
          return sure;
        },
      );
      if (!mounted) return;
      final who = profileDisplayName(t, confirmed?.profile.name ?? '');
      _announce(t.shareDone(area == null ? '' : areaName(t, area), who));
      Navigator.of(context).pop(true);
    } on SyncException catch (e) {
      if (!mounted) return;
      _announce(syncFailureText(t, e.failure));
    } on Object {
      if (!mounted) return;
      _announce(t.syncFailFailed);
    } finally {
      if (mounted) {
        setState(() => _busy = false);
        AutoSyncScope.of(context)?.devicesChanged();
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final area = _area;
    final canScan = PhotoScope.of(context).available;
    return Scaffold(
      appBar: AppBar(
        title: Text(t.shareTitle(area == null ? '' : areaName(t, area))),
      ),
      body: ListView(
        padding: const EdgeInsets.fromLTRB(16, 8, 16, 24),
        children: [
          Text(t.shareIntro),
          const SizedBox(height: 16),
          Text(t.roleHeading, style: Theme.of(context).textTheme.titleSmall),
          RadioGroup<AreaRole>(
            groupValue: _role,
            onChanged: (role) {
              if (role != null && !_busy) setState(() => _role = role);
            },
            child: Column(
              children: [
                RadioListTile<AreaRole>(
                  key: const ValueKey('role-member'),
                  contentPadding: EdgeInsets.zero,
                  value: AreaRole.member,
                  title: Text(t.roleMember),
                  subtitle: Text(t.roleMemberDetail),
                ),
                RadioListTile<AreaRole>(
                  key: const ValueKey('role-manager'),
                  contentPadding: EdgeInsets.zero,
                  value: AreaRole.manager,
                  title: Text(t.roleManager),
                  subtitle: Text(t.roleManagerDetail),
                ),
              ],
            ),
          ),
          const SizedBox(height: 16),
          if (canScan)
            FilledButton.icon(
              key: const ValueKey('share-scan'),
              onPressed: _busy ? null : _scan,
              icon: const Icon(Icons.qr_code_scanner),
              label: Text(t.syncScan),
            ),
          const SizedBox(height: 16),
          TextField(
            key: const ValueKey('share-code'),
            controller: _code,
            enabled: !_busy,
            autocorrect: false,
            enableSuggestions: false,
            textCapitalization: TextCapitalization.none,
            decoration: InputDecoration(
              labelText: t.syncCodeLabel,
              border: const OutlineInputBorder(),
            ),
            onSubmitted: (_) => _invite(),
          ),
          const SizedBox(height: 8),
          Align(
            alignment: Alignment.centerRight,
            child: _busy
                ? Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      const SizedBox.square(
                        dimension: 20,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      ),
                      const SizedBox(width: 12),
                      Text(t.syncSyncing),
                    ],
                  )
                : FilledButton.tonal(
                    key: const ValueKey('invite'),
                    onPressed: _invite,
                    child: Text(t.shareInvite),
                  ),
          ),
        ],
      ),
    );
  }
}

/// "Share this Area with this person?": the other person's picture, name
/// and short code, as their computer sent them (ADR 0017), to compare
/// with the code on their Profile page. Pops with true to share.
class InviteConfirmDialog extends StatelessWidget {
  final String area;
  final ProfileChoice other;

  const InviteConfirmDialog({
    super.key,
    required this.area,
    required this.other,
  });

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final text = Theme.of(context).textTheme;
    return AlertDialog(
      title: Text(t.inviteConfirmTitle(area)),
      scrollable: true,
      content: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          ProfileAvatar(
            key: const ValueKey('inviteAvatar'),
            profile: other.profile,
            size: 72,
          ),
          const SizedBox(height: 12),
          Text(
            profileDisplayName(t, other.profile.name),
            style: text.titleMedium,
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: 8),
          Text(t.profileTheirCode, style: text.labelMedium),
          SelectableText(
            other.code,
            style: text.titleLarge?.copyWith(fontFamily: 'monospace'),
          ),
          const SizedBox(height: 12),
          Text(t.inviteConfirmText),
        ],
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(false),
          child: Text(t.cancel),
        ),
        FilledButton(
          key: const ValueKey('inviteConfirm'),
          onPressed: () => Navigator.of(context).pop(true),
          child: Text(t.inviteConfirmShare),
        ),
      ],
    );
  }
}

/// The people of a Shared Area with their Profiles and roles, this
/// Device's person first. A Manager changes their roles and removes them here; anyone
/// else only sees who is in.
class PeopleScreen extends StatefulWidget {
  final Store store;
  final String areaId;

  const PeopleScreen({super.key, required this.store, required this.areaId});

  @override
  State<PeopleScreen> createState() => _PeopleScreenState();
}

class _PeopleScreenState extends State<PeopleScreen> {
  List<Person> _people = const [];

  /// Each person's Profile as it shows, by Profile id.
  Map<String, Profile> _shown = const {};
  String? _mine;
  bool _manages = false;
  Area? _area;

  @override
  void initState() {
    super.initState();
    _load();
  }

  void _load() {
    final store = widget.store;
    _people = store.people(widget.areaId);
    _shown = {for (final p in _people) p.id: store.personShown(p.id)};
    _mine = store.grantsOf(widget.areaId)?.personOf(store.deviceId);
    _manages = store.accessIn(widget.areaId) == AreaAccess.manager;
    _area = Places(store).areaById(widget.areaId);
  }

  Future<void> _act(Person person, _PersonAction action) async {
    final store = widget.store;
    switch (action) {
      case _PersonAction.manager:
        store.setRole(widget.areaId, person.id, AreaRole.manager);
      case _PersonAction.member:
        store.setRole(widget.areaId, person.id, AreaRole.member);
      case _PersonAction.remove:
        if (!await _confirmRemove(person) || !mounted) return;
        store.removePerson(widget.areaId, person.id);
    }
    setState(_load);
  }

  Future<bool> _confirmRemove(Person person) async {
    final t = context.t;
    final area = _area;
    final sure = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(
          t.removePersonQuestion(
            _name(t, person),
            area == null ? '' : areaName(t, area),
          ),
        ),
        content: Text(t.removePersonDetail),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(context).pop(false),
            child: Text(t.cancel),
          ),
          TextButton(
            style: TextButton.styleFrom(
              foregroundColor: Theme.of(context).colorScheme.error,
            ),
            onPressed: () => Navigator.of(context).pop(true),
            child: Text(t.removePerson),
          ),
        ],
      ),
    );
    return sure ?? false;
  }

  String _name(AppLocalizations t, Person person) =>
      profileDisplayName(t, _shown[person.id]?.name ?? '');

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return Scaffold(
      appBar: AppBar(title: Text(t.peopleTitle)),
      body: ListView(
        padding: const EdgeInsets.fromLTRB(16, 8, 16, 24),
        children: [
          Text(t.peopleIntro),
          const SizedBox(height: 8),
          for (final person in _people)
            ListTile(
              key: ValueKey('person-${person.id}'),
              contentPadding: EdgeInsets.zero,
              leading: ProfileAvatar(profile: _shown[person.id], size: 40),
              title: Text(
                person.id == _mine
                    ? t.peopleYou(_name(t, person))
                    : _name(t, person),
              ),
              subtitle: Text(
                person.role == AreaRole.manager ? t.roleManager : t.roleMember,
              ),
              // Another person's Profile opens from their line (#52).
              onTap: person.id == _mine || !canOpenPerson(context, person.id)
                  ? null
                  : () async {
                      await openPerson(context, person.id);
                      if (mounted) setState(_load);
                    },
              trailing: !_manages || person.id == _mine
                  ? null
                  : PopupMenuButton<_PersonAction>(
                      key: ValueKey('person-menu-${person.id}'),
                      onSelected: (action) => _act(person, action),
                      itemBuilder: (_) => [
                        if (person.role == AreaRole.member)
                          PopupMenuItem(
                            value: _PersonAction.manager,
                            child: Text(t.makeManager),
                          )
                        else
                          PopupMenuItem(
                            value: _PersonAction.member,
                            child: Text(t.makeMember),
                          ),
                        PopupMenuItem(
                          value: _PersonAction.remove,
                          child: Text(t.removePerson),
                        ),
                      ],
                    ),
            ),
        ],
      ),
    );
  }
}

enum _PersonAction { manager, member, remove }
