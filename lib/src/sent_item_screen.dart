import 'dart:io';

import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'announcements.dart';
import 'bundle_platform.dart';
import 'checklist_view.dart';
import 'failure_dialog.dart';
import 'l10n.dart';
import 'note_editor_screen.dart';
import 'person_screen.dart';
import 'profile.dart';
import 'schedule_text.dart';
import 'system_bars.dart';
import 'text_marks_view.dart';

/// Sends the Note [noteId] as a Sent Item (ADR 0009, 0015): asks who
/// gets it, writes the file, encrypted to that person or signed only,
/// and offers it to the app the user picks. A Note sent before goes as
/// an update, or as its cancellation. Announces a failure when no file
/// could be written or no app takes it, why behind the tap.
Future<void> sendNoteFile(
  BuildContext context,
  Store store,
  String noteId,
) async {
  final t = context.t;
  final announcements = AnnouncementsScope.of(context);
  final navigator = Navigator.of(context, rootNavigator: true);
  final files = BundleScope.of(context);
  void notSent(String cause, String fix, String details) => announceFailureWith(
    announcements,
    navigator,
    t.sentItemNotSent,
    title: t.sentItemNotSentTitle,
    cause: cause,
    fix: fix,
    details: details,
  );
  final picked = await showDialog<_Pick>(
    context: context,
    builder: (_) => SendNoteDialog(
      recipients: sentItemRecipients(store),
      update: wasSent(store, noteId),
    ),
  );
  if (picked == null) return;
  final path = await files.exportPath(sentItemFileName(store.clock()));
  if (path == null) {
    notSent(
      t.sentItemNotWrittenCause,
      t.nothingLostFix,
      'sent item: no path to write the file to',
    );
    return;
  }
  File(path).writeAsBytesSync(
    sendNote(store, noteId, to: picked.to, cancel: picked.cancel),
  );
  if (await files.share(path, type: sentItemMimeType) == ShareOutcome.noApp) {
    notSent(t.sendNoApp, t.sendNoAppFix, 'sent item: no app took the file');
  }
}

/// Who a Sent Item goes to: a known person, or anyone (null); and
/// whether it is a cancellation.
typedef _Pick = ({Recipient? to, bool cancel});

/// Asks who gets a Note: anyone, signed only, with a hint that whoever
/// holds the file can read it, or one of the people whose keys are
/// known here, encrypted to them. For a Note sent before ([update]) it
/// sends an update, or a cancellation instead.
class SendNoteDialog extends StatefulWidget {
  final List<Recipient> recipients;
  final bool update;

  const SendNoteDialog({
    super.key,
    required this.recipients,
    this.update = false,
  });

  @override
  State<SendNoteDialog> createState() => _SendNoteDialogState();
}

class _SendNoteDialogState extends State<SendNoteDialog> {
  /// The index of the person picked; -1 for anyone.
  late var _picked = widget.recipients.isEmpty ? -1 : 0;
  var _cancel = false;

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final recipients = widget.recipients;
    return AlertDialog(
      title: Text(widget.update ? t.sendUpdate : t.sendTitle),
      content: SingleChildScrollView(
        child: RadioGroup<int>(
          groupValue: _picked,
          onChanged: (i) => setState(() => _picked = i ?? -1),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(t.sendWho),
              for (final (i, r) in recipients.indexed)
                RadioListTile<int>(
                  key: ValueKey('send-to-${r.id}'),
                  contentPadding: EdgeInsets.zero,
                  value: i,
                  title: Text(t.sendEncrypted(profileDisplayName(t, r.name))),
                  secondary: const Icon(Icons.lock_outline),
                ),
              RadioListTile<int>(
                key: const ValueKey('send-to-anyone'),
                contentPadding: EdgeInsets.zero,
                value: -1,
                title: Text(t.sendAnyone),
                secondary: const Icon(Icons.lock_open),
              ),
              if (_picked < 0) Text(t.sendPlainHint),
              if (widget.update)
                CheckboxListTile(
                  key: const ValueKey('send-cancel'),
                  contentPadding: EdgeInsets.zero,
                  value: _cancel,
                  onChanged: (on) => setState(() => _cancel = on ?? false),
                  title: Text(t.sendCancel),
                  subtitle: Text(t.sendCancelHint),
                ),
              const SizedBox(height: 8),
              Text(t.sendWhat, style: Theme.of(context).textTheme.bodySmall),
            ],
          ),
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(t.cancel),
        ),
        TextButton(
          key: const ValueKey('send-note'),
          onPressed: () => Navigator.of(context).pop<_Pick>((
            to: _picked < 0 ? null : recipients[_picked],
            cancel: _cancel,
          )),
          child: Text(t.sendAction),
        ),
      ],
    );
  }
}

/// Opens a file another app handed over, or the user picked: a Sent
/// Item shows its preview, anything else is imported as a bundle. The
/// copy is removed either way. Returns how many Conflicts a bundle
/// brought, for the caller to announce.
Future<int> openReceivedFile(
  BuildContext context,
  Store store,
  String path,
) async {
  final t = context.t;
  final announcements = AnnouncementsScope.of(context);
  final navigator = Navigator.of(context);
  final bytes = _small(path);
  if (!looksLikeSentItem(bytes)) {
    return importBundleFile(context, store, path);
  }
  try {
    File(path).deleteSync();
  } on FileSystemException {
    // Gone already.
  }
  try {
    // A stranger's is held, and shows nothing of it until "Show" (#91).
    switch (receiveSentItem(store, bytes)) {
      case final OpenedSentItem opened:
        await navigator.push(
          MaterialPageRoute<void>(
            builder: (_) => SentItemScreen(
              store: store,
              opened: opened,
              reopen: () => switch (receiveSentItem(store, bytes)) {
                final OpenedSentItem again => again,
                // Held now: what showed stays.
                HeldSentItem() => throw const NotASentItem(),
              },
            ),
          ),
        );
      case final HeldSentItem held:
        await navigator.push(
          MaterialPageRoute<void>(
            builder: (_) => HeldSentItemScreen(store: store, held: held),
          ),
        );
    }
  } on Exception catch (e) {
    // Refused, why behind the tap; anything else is no refusal.
    if (_refused(t, e) == null) rethrow;
    _announceRefused(t, announcements, navigator, e);
  }
  return 0;
}

/// The bytes of the file at [path]; none when it cannot be read or is
/// larger than a Sent Item can be, as only bundles are.
List<int> _small(String path) {
  try {
    final file = File(path);
    return file.lengthSync() > maxSentItemBytes
        ? const []
        : file.readAsBytesSync();
  } on FileSystemException {
    return const [];
  }
}

/// Whether the file at [path] is a Sent Item rather than a bundle.
bool isSentItemFile(String path) => looksLikeSentItem(_small(path));

/// The preview of a Sent Item: who sent it, with their Profile's picture
/// and name, verified or not with its short code, whether it came
/// encrypted, its text, Sections and Routines and Reminders with their
/// times. "Compare codes" asks the user to compare the code with the
/// sender in person; a match marks the Profile verified when the item is
/// accepted (ADR 0017, #51). Accept takes it into Unsorted with its
/// alarms on and keeps the sender's Profile; for an update it shows what
/// changes, "Tue 10:00 → Wed 14:00", and applies it to the Note accepted
/// before; a cancellation moves that Note to the trash. Decline keeps
/// nothing; one already up to date, or with nothing to cancel, only
/// closes. "Decline and block" keeps nothing and blocks the sender
/// (#90); once the sender is blocked the preview closes. A stranger's
/// item shown from where it is held ([held]) is decided there: Accept,
/// Decline, Close and "Decline and block" each end the held set.
class SentItemScreen extends StatefulWidget {
  final Store store;
  final OpenedSentItem opened;

  /// The pending set a stranger's item is held in (#91), when it was
  /// shown from there.
  final String? held;

  /// Reads the item again, as it shows after the sender's Profile was
  /// opened and an Override set there (#52).
  final OpenedSentItem Function()? reopen;

  const SentItemScreen({
    super.key,
    required this.store,
    required this.opened,
    this.held,
    this.reopen,
  });

  @override
  State<SentItemScreen> createState() => _SentItemScreenState();
}

/// What comparing the short code came to.
enum _Compared { not, match, differ }

class _SentItemScreenState extends State<SentItemScreen> {
  var _compared = _Compared.not;

  Store get store => widget.store;
  late OpenedSentItem opened = widget.opened;

  /// Whether the item names the user's own Profile, which is not blocked.
  late final _fromSelf = store.isOwnPerson(widget.opened.sender.id);

  /// Shows the sender as the user sees them now; closes the preview,
  /// showing nothing more of it, once the user blocked them.
  void _reopen() {
    if (store.isBlocked(opened.sender.id)) return _closeBlocked(context);
    final reopen = widget.reopen;
    if (reopen == null) return;
    try {
      final again = reopen();
      setState(() => opened = again);
    } on SentItemFromBlocked {
      _closeBlocked(context);
    } on Exception {
      // Read once already; what showed stays.
    }
  }

  /// Closes the preview of an item whose sender is blocked now.
  void _closeBlocked(BuildContext context) {
    _announceRefused(
      context.t,
      AnnouncementsScope.of(context),
      Navigator.of(context, rootNavigator: true),
      const SentItemFromBlocked(),
    );
    Navigator.of(context).pop();
  }

  Future<void> _compare(String name) async {
    final t = context.t;
    final match = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(t.sentItemCompare),
        content: Text(t.sentItemCompareHint(name, opened.code)),
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
    setState(() => _compared = match ? _Compared.match : _Compared.differ);
  }

  void _accept(BuildContext context) {
    final announcements = AnnouncementsScope.of(context);
    final t = context.t;
    final verified = _compared == _Compared.match;
    try {
      if (widget.held case final held?) {
        acceptHeldSentItem(store, held, verified: verified);
      } else {
        acceptSentItem(store, opened.item, verified: verified);
      }
      announcements?.announce(switch (opened.state) {
        SentItemState.update => t.sentItemApplied,
        SentItemState.cancel => t.sentItemTrashed,
        _ => t.sentItemAccepted,
      });
    } on Exception catch (e) {
      // Refused, why behind the tap; anything else is no refusal.
      if (_refused(t, e) == null) rethrow;
      _announceRefused(
        t,
        announcements,
        Navigator.of(context, rootNavigator: true),
        e,
      );
    } on StateError {
      // The held item went meanwhile: a block drops it too.
      if (store.isBlocked(opened.sender.id)) {
        _announceRefused(
          t,
          announcements,
          Navigator.of(context, rootNavigator: true),
          const SentItemFromBlocked(),
        );
      } else {
        announcements?.announce(t.sentItemGone);
      }
    }
    Navigator.of(context).pop();
  }

  void _decline(BuildContext context) {
    if (widget.held case final held?) store.declinePending(held);
    AnnouncementsScope.of(context)?.announce(context.t.sentItemDeclined);
    Navigator.of(context).pop();
  }

  /// Closes an item with nothing to accept; a held one goes with it.
  void _close(BuildContext context) {
    if (widget.held case final held?) store.declinePending(held);
    Navigator.of(context).pop();
  }

  /// Declines and blocks the sender after asking once, as their Profile
  /// page does; an id copied from another person's Profile blocks
  /// nobody, and the item is only declined.
  Future<void> _declineAndBlock(String name) async {
    final t = context.t;
    final announcements = AnnouncementsScope.of(context);
    final navigator = Navigator.of(context);
    var blocked = false;
    final sure = await confirmBlock(
      context,
      store,
      opened.sender.id,
      name,
      block: ({required leaveAreas}) => blocked = switch (widget.held) {
        final held? => blockHeldSentItemSender(
          store,
          held,
          leaveAreas: leaveAreas,
        ),
        null => blockSentItemSender(store, opened, leaveAreas: leaveAreas),
      },
    );
    if (!sure || !mounted) return;
    announcements?.announce(blocked ? t.sentItemBlocked : t.sentItemDeclined);
    navigator.pop();
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final text = Theme.of(context).textTheme;
    final locale = Localizations.localeOf(context).toString();
    final when = DateFormat.yMd(locale).add_Hm();
    final item = opened.item;
    final code = opened.code;
    final name = profileDisplayName(t, opened.sender.name);
    final trust = _compared == _Compared.match
        ? SenderTrust.verified
        : opened.trust;
    final (trustIcon, trustText) = switch (trust) {
      SenderTrust.verified => (Icons.verified_user, t.sentItemVerified(code)),
      SenderTrust.known => (Icons.gpp_maybe_outlined, t.sentItemKnown(code)),
      SenderTrust.unknown => (
        Icons.gpp_maybe_outlined,
        t.sentItemUnknown(code),
      ),
    };
    final state = opened.state;
    final accept = switch (state) {
      SentItemState.fresh => t.sentItemAccept,
      SentItemState.update => t.sentItemApply,
      SentItemState.cancel => t.sentItemAcceptCancel,
      SentItemState.upToDate || SentItemState.nothingToCancel => null,
    };
    final full = state == SentItemState.fresh || state == SentItemState.update;
    String schedule(Schedule s) => scheduleText(t, locale, s);
    return Scaffold(
      appBar: AppBar(title: Text(t.sentItemTitle)),
      // Its tags are shown, not followed: the preview is no place to
      // leave before Accept or Decline.
      body: TagSearchScope(
        onSearch: null,
        child: ListView(
          padding: clearOfSystemBars(context, const EdgeInsets.all(16)),
          children: [
            EditorPanel(
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      // A sender kept here opens their Profile (#52); one
                      // not kept yet has nothing to open until accepted.
                      GestureDetector(
                        onTap:
                            opened.trust == SenderTrust.unknown ||
                                !canOpenPerson(context, opened.sender.id)
                            ? null
                            : () async {
                                await openPerson(context, opened.sender.id);
                                if (mounted) _reopen();
                              },
                        child: ProfileAvatar(
                          key: const ValueKey('sent-from-picture'),
                          profile: opened.sender,
                          size: 48,
                        ),
                      ),
                      const SizedBox(width: 12),
                      Expanded(
                        child: Text(
                          t.sentItemFrom(name),
                          style: text.titleMedium,
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 8),
                  _Line(icon: trustIcon, text: trustText),
                  if (_compared == _Compared.match &&
                      opened.trust != SenderTrust.verified)
                    _Line(
                      icon: Icons.info_outline,
                      text: t.sentItemVerifiedOnAccept,
                    ),
                  if (_compared == _Compared.differ)
                    _Line(
                      icon: Icons.warning_amber,
                      text: t.sentItemCodesDiffered,
                    ),
                  if (accept != null && trust != SenderTrust.verified)
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
                    icon: opened.encrypted
                        ? Icons.lock_outline
                        : Icons.lock_open,
                    text: opened.encrypted
                        ? t.sentItemEncrypted
                        : t.sentItemPlain,
                  ),
                  ...switch (state) {
                    SentItemState.fresh => const <Widget>[],
                    SentItemState.update => [
                      _Line(icon: Icons.update, text: t.sentItemUpdate),
                    ],
                    SentItemState.cancel => [
                      _Line(
                        icon: Icons.event_busy,
                        text: t.sentItemCancelled(
                          TextFormats.visible(item.textFormat, item.text),
                        ),
                      ),
                      _Line(
                        icon: Icons.delete_outline,
                        text: t.sentItemCancelDetail,
                      ),
                    ],
                    SentItemState.upToDate => [
                      _Line(icon: Icons.check, text: t.sentItemAlready),
                    ],
                    SentItemState.nothingToCancel => [
                      _Line(
                        icon: Icons.info_outline,
                        text: t.sentItemNothingToCancel,
                      ),
                    ],
                  },
                  const SizedBox(height: 8),
                  Wrap(
                    spacing: 8,
                    runSpacing: 8,
                    children: [
                      if (accept != null)
                        FilledButton.icon(
                          key: const ValueKey('accept-sent'),
                          onPressed: () => _accept(context),
                          icon: const Icon(Icons.check),
                          label: Text(accept),
                        ),
                      OutlinedButton.icon(
                        key: const ValueKey('decline-sent'),
                        onPressed: () => accept == null
                            ? _close(context)
                            : _decline(context),
                        icon: const Icon(Icons.close),
                        label: Text(
                          accept == null ? t.sentItemClose : t.sentItemDecline,
                        ),
                      ),
                    ],
                  ),
                  if (!_fromSelf) ...[
                    const SizedBox(height: 8),
                    OutlinedButton.icon(
                      key: const ValueKey('block-sent'),
                      onPressed: () => _declineAndBlock(name),
                      icon: const Icon(Icons.block),
                      label: Text(t.sentItemBlock),
                    ),
                  ],
                ],
              ),
            ),
            if (opened.changes case final changes?)
              EditorPanel(
                heading: t.sentItemChanges,
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    if (changes.restores)
                      _Line(icon: Icons.restore, text: t.sentItemRestores),
                    if (changes.textBefore case final before?) ...[
                      _Line(icon: Icons.notes, text: t.sentItemTextWas(before)),
                      _Line(
                        icon: Icons.edit_note,
                        text: t.sentItemTextNow(
                          TextFormats.visible(item.textFormat, item.text),
                        ),
                      ),
                      if (changes.textEdited)
                        _Line(
                          icon: Icons.warning_amber,
                          text: t.sentItemTextYours,
                        ),
                    ],
                    if (changes.makesChecklist)
                      _Line(
                        icon: Icons.checklist,
                        text: t.sentItemMakesChecklist,
                      ),
                    if (changes.makesPlain)
                      _Line(icon: Icons.notes, text: t.sentItemMakesPlain),
                    for (final l in changes.linesAdded)
                      _Line(
                        icon: Icons.playlist_add,
                        text: t.sentItemLineAdded(l.text),
                      ),
                    for (final r in changes.linesRenamed)
                      _Line(
                        icon: Icons.edit_outlined,
                        text: t.sentItemLineRenamed(r.before, r.after),
                      ),
                    for (final l in changes.linesRemoved)
                      _Line(
                        icon: Icons.playlist_remove,
                        text: t.sentItemLineRemoved(l.text),
                      ),
                    for (final s in changes.sectionsAdded)
                      _Line(
                        icon: Icons.add,
                        text: t.sentItemSectionAdded(
                          TextFormats.visible(s.format, s.text),
                        ),
                      ),
                    for (final r in changes.rescheduled)
                      _Line(
                        icon: Icons.schedule,
                        text: t.sentItemRescheduled(
                          schedule(r.before),
                          schedule(r.after),
                        ),
                      ),
                    for (final r in changes.routinesAdded)
                      _Line(
                        icon: Icons.alarm_add,
                        text: t.sentItemRoutineAdded(schedule(r.schedule)),
                      ),
                    for (final r in changes.routinesRemoved)
                      _Line(
                        icon: Icons.alarm_off,
                        text: t.sentItemRoutineRemoved(schedule(r.schedule)),
                      ),
                    if (changes.isEmpty)
                      _Line(icon: Icons.check, text: t.sentItemNoChanges),
                    const SizedBox(height: 4),
                    Text(t.sentItemKeeps, style: text.bodySmall),
                  ],
                ),
              ),
            EditorPanel(
              heading: t.noteTitle,
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  MarkedText(
                    item.text,
                    format: item.textFormat,
                    style: text.bodyLarge,
                  ),
                  // The Lines of a Checklist, to look at, not to check.
                  if (item.lines case final lines? when full)
                    ChecklistLines(
                      key: const ValueKey('sent-lines'),
                      lines: [
                        for (final l in lines)
                          (
                            id: l.id,
                            text: l.text,
                            inGroup: l.indent,
                            below: null,
                            checked: l.checked,
                          ),
                      ],
                    ),
                ],
              ),
            ),
            if (full && item.routines.isNotEmpty)
              EditorPanel(
                heading: t.routinesHeading,
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    for (final r in item.routines)
                      _Line(
                        icon: r.schedule.once
                            ? Icons.notifications_outlined
                            : Icons.repeat,
                        text: schedule(r.schedule),
                      ),
                  ],
                ),
              ),
            if (full)
              for (final section in item.sections)
                EditorPanel(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      MarkedText(
                        section.text,
                        format: section.format,
                        style: text.bodyLarge,
                      ),
                      const SizedBox(height: 4),
                      Text(
                        when.format(section.captured.toLocal()),
                        style: text.labelMedium,
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

/// Why a Sent Item was refused as [error] says, and what to do; null
/// for anything else.
(String, String)? _refused(AppLocalizations t, Object error) => switch (error) {
  NotASentItem() => (t.sentItemUnreadable, t.sentItemAskAgainFix),
  SentItemNotForUs() => (t.sentItemNotForUs, t.sentItemAskAgainFix),
  SentItemNotFromSender() => (t.sentItemNotFromSender, t.sentItemAskAgainFix),
  SentItemFromBlocked() => (t.sentItemFromBlocked, t.blockedFix),
  _ => null,
};

/// Announces that a Sent Item was not taken; the tap opens why and what
/// to do ([_refused]), [error] as the full text.
void _announceRefused(
  AppLocalizations t,
  Announcements? announcements,
  NavigatorState navigator,
  Object error,
) {
  final (cause, fix) =
      _refused(t, error) ?? (t.sentItemUnreadable, t.sentItemAskAgainFix);
  announceFailureWith(
    announcements,
    navigator,
    t.sentItemNotTaken,
    title: t.sentItemNotTakenTitle,
    cause: cause,
    fix: fix,
    details: '$error',
  );
}

/// The preview of a stranger's Sent Item, held until "Show" (#91, ADR
/// 0015, 0022): only who it is from and that they are unknown here, not
/// their picture or short code, nothing of what it holds. "Show" opens
/// the preview as for anybody else, Compare codes included; it is not
/// remembered, so the item is hidden again the next time it opens.
/// Accept, Decline and "Decline and block" work before "Show" too;
/// leaving decides nothing, and the item waits on the Agenda.
class HeldSentItemScreen extends StatefulWidget {
  final Store store;
  final HeldSentItem held;

  const HeldSentItemScreen({
    super.key,
    required this.store,
    required this.held,
  });

  @override
  State<HeldSentItemScreen> createState() => _HeldSentItemScreenState();
}

class _HeldSentItemScreenState extends State<HeldSentItemScreen> {
  /// The preview after "Show".
  OpenedSentItem? _shown;

  Store get store => widget.store;
  String get _id => widget.held.id;

  /// Announces [text] and closes.
  void _leave(String text) {
    AnnouncementsScope.of(context)?.announce(text);
    Navigator.of(context).pop();
  }

  /// Announces that the item cannot go on, why as [error] says behind
  /// the tap, and closes.
  void _refuse(Object error) {
    // Decided meanwhile; a block drops it too.
    if (error is StateError && !store.isBlocked(widget.held.sender.id)) {
      return _leave(context.t.sentItemGone);
    }
    _announceRefused(
      context.t,
      AnnouncementsScope.of(context),
      Navigator.of(context, rootNavigator: true),
      error is StateError ? const SentItemFromBlocked() : error,
    );
    Navigator.of(context).pop();
  }

  void _show() {
    try {
      final shown = showHeldSentItem(store, _id);
      if (shown == null) return _leave(context.t.sentItemGone);
      setState(() => _shown = shown);
    } on Exception catch (e) {
      _refuse(e);
    }
  }

  void _accept() {
    final t = context.t;
    try {
      final note = acceptHeldSentItem(store, _id);
      _leave(note == null ? t.sentItemNothingToCancel : t.sentItemAccepted);
    } on StateError catch (e) {
      _refuse(e);
    } on Exception catch (e) {
      _refuse(e);
    }
  }

  void _decline() {
    store.declinePending(_id);
    _leave(context.t.sentItemDeclined);
  }

  /// Declines and blocks the sender after asking once, as the preview
  /// after "Show" does.
  Future<void> _declineAndBlock(String name) async {
    final t = context.t;
    final announcements = AnnouncementsScope.of(context);
    final navigator = Navigator.of(context);
    var blocked = false;
    final sure = await confirmBlock(
      context,
      store,
      widget.held.sender.id,
      name,
      block: ({required leaveAreas}) =>
          blocked = blockHeldSentItemSender(store, _id, leaveAreas: leaveAreas),
    );
    if (!sure || !mounted) return;
    announcements?.announce(blocked ? t.sentItemBlocked : t.sentItemDeclined);
    navigator.pop();
  }

  @override
  Widget build(BuildContext context) {
    if (_shown case final shown?) {
      return SentItemScreen(
        store: store,
        opened: shown,
        held: _id,
        reopen: () =>
            showHeldSentItem(store, _id) ?? (throw const NotASentItem()),
      );
    }
    final t = context.t;
    final name = profileDisplayName(t, widget.held.sender.name);
    return Scaffold(
      appBar: AppBar(title: Text(t.sentItemTitle)),
      body: ListView(
        padding: clearOfSystemBars(context, const EdgeInsets.all(16)),
        children: [
          EditorPanel(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Text(
                  t.sentItemFrom(name),
                  style: Theme.of(context).textTheme.titleMedium,
                ),
                const SizedBox(height: 8),
                _Line(icon: Icons.gpp_maybe_outlined, text: t.sentItemStranger),
                _Line(
                  icon: Icons.visibility_off_outlined,
                  text: t.sentItemHidden,
                ),
                const SizedBox(height: 8),
                FilledButton.icon(
                  key: const ValueKey('show-sent'),
                  onPressed: _show,
                  icon: const Icon(Icons.visibility_outlined),
                  label: Text(t.sentItemShow),
                ),
                const SizedBox(height: 8),
                Row(
                  children: [
                    Expanded(
                      child: OutlinedButton.icon(
                        key: const ValueKey('accept-sent'),
                        onPressed: _accept,
                        icon: const Icon(Icons.check),
                        label: Text(t.sentItemAccept),
                      ),
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: OutlinedButton.icon(
                        key: const ValueKey('decline-sent'),
                        onPressed: _decline,
                        icon: const Icon(Icons.close),
                        label: Text(t.sentItemDecline),
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 8),
                OutlinedButton.icon(
                  key: const ValueKey('block-sent'),
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
