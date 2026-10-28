import 'dart:async';

import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'announcements.dart';
import 'auto_sync.dart';
import 'browser.dart';
import 'folder_platform.dart';
import 'l10n.dart';
import 'note_editor_screen.dart';
import 'person_screen.dart';
import 'place_names.dart';
import 'profile.dart';
import 'system_bars.dart';

/// "Ann shares Household with you as Member" for the offer [offer]: its
/// Area's name as the user reads it, or "an Area" when the roles left
/// the name out.
String areaOfferLine(AppLocalizations t, HeldAreaOffer offer) {
  final area = offeredAreaName(t, offer.areaStarter, offer.areaName);
  return t.areaOfferText(
    profileDisplayName(t, offer.sender.name),
    area.isEmpty ? t.areaOfferNoName : area,
    offer.role == AreaRole.manager ? t.roleManager : t.roleMember,
  );
}

/// What a refused Area offer is announced as; null for anything else.
String? areaOfferRefusal(AppLocalizations t, Object error) => switch (error) {
  AreaOfferNotForUs() => t.areaOfferNotForUs,
  AreaOfferFromBlocked() => t.areaOfferFromBlocked,
  AreaOfferRefused() => t.areaOfferRefused,
  NotAnAreaOffer() => t.notAnAreaOffer,
  _ => null,
};

/// The preview of an Area shared with the user from afar (#108, ADR
/// 0023), held until Accept or Decline: who shares it, with their picture
/// and name, verified or not with its short code, the Area and the role
/// it comes with, and how changes travel. "Compare codes" marks the
/// sharer verified on Accept, as for a Sent Item (#51). Accept takes the
/// Area in; one whose changes travel by a folder asks for the folder
/// when none is chosen yet. Decline keeps nothing; "Decline and block"
/// keeps nothing and blocks the sharer (#90). Leaving decides nothing:
/// the offer waits on the Agenda.
class AreaOfferScreen extends StatefulWidget {
  final Store store;
  final HeldAreaOffer offer;

  const AreaOfferScreen({super.key, required this.store, required this.offer});

  @override
  State<AreaOfferScreen> createState() => _AreaOfferScreenState();
}

class _AreaOfferScreenState extends State<AreaOfferScreen> {
  /// Whether the codes were compared, and matched.
  bool? _matched;

  Store get store => widget.store;
  HeldAreaOffer get offer => widget.offer;

  /// Announces [text] and closes.
  void _leave(String text) {
    AnnouncementsScope.of(context)?.announce(text);
    Navigator.of(context).pop();
  }

  /// Opens the sharer's Profile; once the user blocked them there, the
  /// offer is gone with the block, and the preview closes.
  Future<void> _openSharer() async {
    await openPerson(context, offer.sender.id);
    if (!mounted || !store.isBlocked(offer.sender.id)) return;
    _leave(context.t.areaOfferFromBlocked);
  }

  Future<void> _compare(String name) async {
    final t = context.t;
    final match = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(t.sentItemCompare),
        content: Text(t.sentItemCompareHint(name, offer.code)),
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
    setState(() => _matched = match);
  }

  Future<void> _accept() async {
    final t = context.t;
    try {
      acceptAreaOffer(store, offer.id, verified: _matched ?? false);
    } on StateError {
      // Decided meanwhile; a block drops it too.
      return _leave(
        store.isBlocked(offer.sender.id)
            ? t.areaOfferFromBlocked
            : t.areaOfferGone,
      );
    } on Exception catch (e) {
      return _leave(areaOfferRefusal(t, e) ?? t.notAnAreaOffer);
    }
    final folders = FolderScope.of(context);
    final auto = AutoSyncScope.of(context);
    AnnouncementsScope.of(context)?.announce(t.areaOfferAccepted);
    if (offer.transport == Transport.folder &&
        store.syncFolder == null &&
        folders.available) {
      final pick = await showDialog<bool>(
        context: context,
        builder: (context) => AlertDialog(
          title: Text(t.areaOfferFolderQuestion),
          content: Text(t.areaOfferFolderDetail(_area(t))),
          actions: [
            TextButton(
              key: const ValueKey('offer-folder-later'),
              onPressed: () => Navigator.of(context).pop(false),
              child: Text(t.areaOfferFolderLater),
            ),
            FilledButton(
              key: const ValueKey('offer-folder-pick'),
              onPressed: () => Navigator.of(context).pop(true),
              child: Text(t.folderPick),
            ),
          ],
        ),
      );
      if (pick == true) {
        final picked = await folders.pick();
        if (picked != null) store.chooseSyncFolder(picked);
      }
    }
    auto?.devicesChanged();
    if (!mounted) return;
    Navigator.of(context).pop();
  }

  void _decline() {
    declineAreaOffer(store, offer.id);
    _leave(context.t.sentItemDeclined);
  }

  /// Declines and blocks the sharer after asking once, as a Sent Item's
  /// preview does; an id copied from another person's Profile blocks
  /// nobody, and the offer is only declined.
  Future<void> _declineAndBlock(String name) async {
    final t = context.t;
    final announcements = AnnouncementsScope.of(context);
    final navigator = Navigator.of(context);
    var blocked = false;
    final sure = await confirmBlock(
      context,
      store,
      offer.sender.id,
      name,
      block: ({required leaveAreas}) => blocked = blockAreaOfferSender(
        store,
        offer.id,
        leaveAreas: leaveAreas,
      ),
    );
    if (!sure || !mounted) return;
    announcements?.announce(blocked ? t.sentItemBlocked : t.sentItemDeclined);
    navigator.pop();
  }

  String _area(AppLocalizations t) {
    final area = offeredAreaName(t, offer.areaStarter, offer.areaName);
    return area.isEmpty ? t.areaOfferNoName : area;
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final text = Theme.of(context).textTheme;
    final code = offer.code;
    final name = profileDisplayName(t, offer.sender.name);
    final trust = _matched == true ? SenderTrust.verified : offer.trust;
    final (trustIcon, trustText) = switch (trust) {
      SenderTrust.verified => (Icons.verified_user, t.sentItemVerified(code)),
      SenderTrust.known => (Icons.gpp_maybe_outlined, t.sentItemKnown(code)),
      SenderTrust.unknown => (
        Icons.gpp_maybe_outlined,
        t.sentItemUnknown(code),
      ),
    };
    return Scaffold(
      appBar: AppBar(title: Text(t.areaOfferTitle)),
      body: ListView(
        padding: clearOfSystemBars(context, const EdgeInsets.all(16)),
        children: [
          EditorPanel(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Row(
                  children: [
                    // A sharer kept here opens their Profile (#52); one
                    // not kept yet has nothing to open until accepted.
                    GestureDetector(
                      onTap:
                          offer.trust == SenderTrust.unknown ||
                              !canOpenPerson(context, offer.sender.id)
                          ? null
                          : _openSharer,
                      child: ProfileAvatar(
                        key: const ValueKey('offer-from-picture'),
                        profile: offer.sender,
                        size: 48,
                      ),
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: Text(
                        areaOfferLine(t, offer),
                        key: const ValueKey('offer-line'),
                        style: text.titleMedium,
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 8),
                _Line(icon: trustIcon, text: trustText),
                if (_matched == true && offer.trust != SenderTrust.verified)
                  _Line(
                    icon: Icons.info_outline,
                    text: t.sentItemVerifiedOnAccept,
                  ),
                if (_matched == false)
                  _Line(
                    icon: Icons.warning_amber,
                    text: t.sentItemCodesDiffered,
                  ),
                if (trust != SenderTrust.verified)
                  Align(
                    alignment: AlignmentDirectional.centerStart,
                    child: TextButton.icon(
                      key: const ValueKey('compare-codes'),
                      onPressed: () => _compare(name),
                      icon: const Icon(Icons.compare_arrows),
                      label: Text(t.sentItemCompare),
                    ),
                  ),
                _Line(
                  icon: offer.transport == Transport.folder
                      ? Icons.folder_shared_outlined
                      : Icons.send_outlined,
                  text: offer.transport == Transport.folder
                      ? t.areaOfferByFolder
                      : t.areaOfferByBundle,
                ),
                _Line(
                  icon: Icons.visibility_off_outlined,
                  text: t.areaOfferKeepsNothing,
                ),
                const SizedBox(height: 8),
                Row(
                  children: [
                    Expanded(
                      child: FilledButton.icon(
                        key: const ValueKey('accept-offer'),
                        onPressed: _accept,
                        icon: const Icon(Icons.check),
                        label: Text(t.sentItemAccept),
                      ),
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: OutlinedButton.icon(
                        key: const ValueKey('decline-offer'),
                        onPressed: _decline,
                        icon: const Icon(Icons.close),
                        label: Text(t.sentItemDecline),
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 8),
                OutlinedButton.icon(
                  key: const ValueKey('block-offer'),
                  onPressed: () => _declineAndBlock(name),
                  icon: const Icon(Icons.block),
                  label: Text(t.sentItemBlock),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

/// The Areas shared from afar that wait here (#108), the oldest first:
/// who shares which Area, as what, and when it came. A tap opens its
/// preview. Those left undecided, or by a restart, wait here until
/// Accept or Decline.
class HeldAreaOffersScreen extends StatefulWidget {
  final Store store;

  const HeldAreaOffersScreen({super.key, required this.store});

  @override
  State<HeldAreaOffersScreen> createState() => _HeldAreaOffersScreenState();
}

class _HeldAreaOffersScreenState extends State<HeldAreaOffersScreen> {
  /// Read once per change: each read checks every held offer again.
  List<HeldAreaOffer> _held = const [];
  var _reloadQueued = false;

  @override
  void initState() {
    super.initState();
    _held = heldAreaOffers(widget.store);
    // An offer accepted, declined or dropped by a block goes.
    widget.store.addListener(_changed);
  }

  @override
  void dispose() {
    widget.store.removeListener(_changed);
    super.dispose();
  }

  /// Reloads once after the store changed, never in the middle of
  /// whatever changed it.
  void _changed() {
    if (_reloadQueued) return;
    _reloadQueued = true;
    scheduleMicrotask(() {
      _reloadQueued = false;
      if (!mounted) return;
      setState(() => _held = heldAreaOffers(widget.store));
    });
  }

  Future<void> _open(HeldAreaOffer offer) async {
    await Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) => AreaOfferScreen(store: widget.store, offer: offer),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final locale = Localizations.localeOf(context).toString();
    final when = DateFormat.yMd(locale).add_Hm();
    return Scaffold(
      appBar: AppBar(title: Text(t.areaOffersWaitingTitle)),
      body: _held.isEmpty
          ? BrowserEmpty(text: t.areaOffersNoneWaiting)
          : ListView(
              padding: clearOfSystemBars(context, EdgeInsets.zero),
              children: [
                for (final offer in _held)
                  ListTile(
                    key: ValueKey('offer-${offer.id}'),
                    leading: const Icon(Icons.group_add_outlined),
                    title: Text(areaOfferLine(t, offer)),
                    subtitle: Text(when.format(offer.held.toLocal())),
                    onTap: () => _open(offer),
                  ),
              ],
            ),
    );
  }
}

/// An icon and a line of text beside it.
class _Line extends StatelessWidget {
  final IconData icon;
  final String text;

  const _Line({required this.icon, required this.text});

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.symmetric(vertical: 4),
    child: Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Icon(icon, size: 20),
        const SizedBox(width: 8),
        Expanded(child: Text(text)),
      ],
    ),
  );
}
