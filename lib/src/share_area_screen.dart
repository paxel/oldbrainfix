import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'announcements.dart';
import 'auto_sync.dart';
import 'bundle_platform.dart';
import 'failure_dialog.dart';
import 'folder_platform.dart';
import 'host_screen.dart';
import 'l10n.dart';
import 'pairing_platform.dart';
import 'person_screen.dart';
import 'photo_platform.dart';
import 'place_names.dart';
import 'profile.dart';
import 'sync_screen.dart';
import 'system_bars.dart';

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
/// and the pair code their Device shows, scanned or typed, or the code
/// this Device shows for theirs ([HostScreen]). Nobody types a name: once
/// the two Devices met, this one shows their Profile (ADR 0017), and
/// only when the user confirms it does their Device get this Area and
/// nothing else, and the Area shows as shared from then on. A Starter
/// Set Area goes on under fresh ids; [onShared] hears the id it has now.
///
/// A person in People who is not nearby gets the Area from afar (#108,
/// ADR 0023): picked from those whose Devices' keys are known here, with
/// the role chosen above, and asked once how changes travel with them;
/// the offer goes out as a file through the app the user picks.
class ShareAreaScreen extends StatefulWidget {
  final Store store;
  final String areaId;
  final InviteJoin join;
  final StartHosting host;
  final ValueChanged<String>? onShared;

  const ShareAreaScreen({
    super.key,
    required this.store,
    required this.areaId,
    this.join = lanSync,
    this.host = startPairingHost,
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

  /// The People an offer can go to: not blocked, with a Device whose
  /// key is known here.
  late final List<AfarPerson> _afar = [
    for (final p in widget.store.keptPeople)
      if (!widget.store.isBlocked(p.id) &&
          offerDevices(widget.store, p.id).isNotEmpty)
        (
          id: p.id,
          shown: widget.store.personShown(p.id),
          verified: widget.store.personVerified(p.id),
        ),
  ];

  @override
  void dispose() {
    _code.dispose();
    super.dispose();
  }

  void _announce(String text) => AnnouncementsScope.of(context)?.announce(text);

  /// Announces that sharing from afar failed; the tap opens [cause],
  /// [fix] and [details].
  void _announceAfarFailed(String cause, String fix, String details) {
    final t = context.t;
    announceFailure(
      context,
      t.shareAfarFailed,
      title: t.shareAfarFailedTitle,
      cause: cause,
      fix: fix,
      details: details,
    );
  }

  /// Shows a code for their Device to join with, inviting it with the
  /// role chosen; once one joined, back to where sharing began.
  Future<void> _showCode() async {
    final t = context.t;
    final store = widget.store;
    final area = _area;
    _areaId = store.share(_areaId);
    widget.onShared?.call(_areaId);
    var joined = false;
    await Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) => HostScreen(
          store: store,
          title: t.shareTitle(area == null ? '' : areaName(t, area)),
          inviting: Invite(_areaId, role: _role),
          area: area == null ? '' : areaName(t, area),
          start: widget.host,
          onSession: (_) => joined = true,
        ),
      ),
    );
    if (!mounted) return;
    AutoSyncScope.of(context)?.devicesChanged();
    if (joined) Navigator.of(context).pop(true);
  }

  /// Takes a photo of their Device's screen and reads the code in it.
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

  /// Shares the Area with a person in People from afar: who, then how
  /// changes travel with them unless they were asked before, then the
  /// folder when it goes by one and none is chosen yet. Only then is
  /// anything written, so backing out of any step changes nothing; the
  /// offer goes to the app the user picks, and once handed over, back to
  /// where sharing began.
  Future<void> _shareAfar() async {
    final t = context.t;
    final store = widget.store;
    final folders = FolderScope.of(context);
    final bundles = BundleScope.of(context);
    final area = _area;
    final person = await Navigator.of(context).push<String>(
      MaterialPageRoute<String>(
        builder: (_) => SharePersonScreen(
          people: _afar,
          title: t.shareWithTitle(area == null ? '' : areaName(t, area)),
        ),
      ),
    );
    if (!mounted || person == null) return;
    final name = profileDisplayName(t, store.personShown(person).name);
    final canFolder = folders.available || store.syncFolder != null;
    var transport = store.transportWith(person);
    if (transport == null) {
      transport = canFolder
          ? await showDialog<Transport>(
              context: context,
              builder: (_) => TransportDialog(name: name),
            )
          : Transport.bundle;
      if (!mounted || transport == null) return;
    }
    if (transport == Transport.folder && store.syncFolder == null) {
      final picked = await folders.pick();
      if (!mounted || picked == null) return;
      store.chooseSyncFolder(picked);
    }
    setState(() => _busy = true);
    try {
      _areaId = shareAreaWith(
        store,
        _areaId,
        person,
        role: _role,
        transport: transport,
      );
      widget.onShared?.call(_areaId);
      final path = await bundles.exportPath(
        bundleFileName(store.clock().toLocal()),
      );
      if (!mounted) return;
      if (path == null) {
        _announceAfarFailed(
          t.shareAfarFailedCause,
          t.shareAfarFailedFix,
          'share from afar: no path to write the file to',
        );
        return;
      }
      await writeAreaOffer(store, path, _areaId, person);
      final shared = await bundles.share(path);
      if (!mounted) return;
      if (shared == ShareOutcome.noApp) {
        _announceAfarFailed(
          t.bundleNoApp,
          t.shareAfarNoAppFix,
          'share from afar: no app took the file',
        );
        return;
      }
      Navigator.of(context).pop(true);
    } on Object catch (e) {
      if (!mounted) return;
      _announceAfarFailed(t.shareAfarFailedCause, t.shareAfarFailedFix, '$e');
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
    final canSend = BundleScope.of(context).available;
    return Scaffold(
      appBar: AppBar(
        title: Text(t.shareTitle(area == null ? '' : areaName(t, area))),
      ),
      body: ListView(
        padding: clearOfSystemBars(
          context,
          const EdgeInsets.fromLTRB(16, 8, 16, 24),
        ),
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
          const SizedBox(height: 16),
          Text(t.shareShowIntro),
          const SizedBox(height: 8),
          Align(
            alignment: Alignment.centerLeft,
            child: FilledButton.tonalIcon(
              key: const ValueKey('share-show'),
              onPressed: _busy ? null : _showCode,
              icon: const Icon(Icons.qr_code_2),
              label: Text(t.syncShowCode),
            ),
          ),
          if (canSend) ...[
            const SizedBox(height: 24),
            Text(
              t.shareAfarHeading,
              style: Theme.of(context).textTheme.titleSmall,
            ),
            const SizedBox(height: 8),
            Text(_afar.isEmpty ? t.shareAfarNobody : t.shareAfarIntro),
            if (_afar.isNotEmpty) ...[
              const SizedBox(height: 8),
              Align(
                alignment: Alignment.centerLeft,
                child: FilledButton.tonalIcon(
                  key: const ValueKey('share-afar'),
                  onPressed: _busy ? null : _shareAfar,
                  icon: const Icon(Icons.person_search_outlined),
                  label: Text(t.shareAfarPick),
                ),
              ),
            ],
          ],
        ],
      ),
    );
  }
}

/// A person an Area can go to from afar, as the user sees them.
typedef AfarPerson = ({String id, Profile shown, bool verified});

/// "Share Household with…": the People an Area can go to from afar
/// ([offerDevices]), each with their picture and name as the user sees
/// them, and "Not verified" under those whose code was never compared in
/// person. A tap pops with the person's id.
class SharePersonScreen extends StatelessWidget {
  final List<AfarPerson> people;
  final String title;

  const SharePersonScreen({
    super.key,
    required this.people,
    required this.title,
  });

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return Scaffold(
      appBar: AppBar(title: Text(title)),
      body: ListView(
        padding: clearOfSystemBars(context, EdgeInsets.zero),
        children: [
          for (final person in people)
            _PersonRow(
              key: ValueKey('share-to-${person.id}'),
              id: person.id,
              profile: person.shown,
              verified: person.verified,
              onTap: () => Navigator.of(context).pop(person.id),
              notVerified: t.shareNotVerified,
            ),
        ],
      ),
    );
  }
}

class _PersonRow extends StatelessWidget {
  final String id;
  final Profile profile;
  final bool verified;
  final String notVerified;
  final VoidCallback onTap;

  const _PersonRow({
    super.key,
    required this.id,
    required this.profile,
    required this.verified,
    required this.notVerified,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) => ListTile(
    // The picture opens their Profile; the row picks them.
    leading: GestureDetector(
      onTap: canOpenPerson(context, id) ? () => openPerson(context, id) : null,
      child: ProfileAvatar(profile: profile, size: 40),
    ),
    title: Text(profileDisplayName(context.t, profile.name)),
    subtitle: verified
        ? null
        : Row(
            children: [
              const Icon(Icons.gpp_maybe_outlined, size: 16),
              const SizedBox(width: 4),
              Text(notVerified),
            ],
          ),
    onTap: onTap,
  );
}

/// "How do changes travel with Ann?", asked once per person when sharing
/// an Area with them from afar: a shared folder both reach, or files
/// through a messenger. Pops with the choice.
class TransportDialog extends StatefulWidget {
  final String name;

  const TransportDialog({super.key, required this.name});

  @override
  State<TransportDialog> createState() => _TransportDialogState();
}

class _TransportDialogState extends State<TransportDialog> {
  var _transport = Transport.bundle;

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return AlertDialog(
      title: Text(t.transportTitle(widget.name)),
      scrollable: true,
      content: RadioGroup<Transport>(
        groupValue: _transport,
        onChanged: (how) {
          if (how != null) setState(() => _transport = how);
        },
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            RadioListTile<Transport>(
              key: const ValueKey('transport-folder'),
              contentPadding: EdgeInsets.zero,
              value: Transport.folder,
              title: Text(t.transportFolder),
              subtitle: Text(t.transportFolderDetail),
            ),
            RadioListTile<Transport>(
              key: const ValueKey('transport-bundle'),
              contentPadding: EdgeInsets.zero,
              value: Transport.bundle,
              title: Text(t.transportBundle),
              subtitle: Text(t.transportBundleDetail),
            ),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(t.cancel),
        ),
        FilledButton(
          key: const ValueKey('transport-share'),
          onPressed: () => Navigator.of(context).pop(_transport),
          child: Text(t.inviteConfirmShare),
        ),
      ],
    );
  }
}

/// "Share this Area with this person?": the other person's picture, name
/// and short code, as their Device sent them (ADR 0017), to compare
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
        padding: clearOfSystemBars(
          context,
          const EdgeInsets.fromLTRB(16, 8, 16, 24),
        ),
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
