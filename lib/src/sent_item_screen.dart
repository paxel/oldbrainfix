import 'dart:io';

import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'announcements.dart';
import 'bundle_platform.dart';
import 'l10n.dart';
import 'note_editor_screen.dart';
import 'person_screen.dart';
import 'profile.dart';
import 'schedule_text.dart';

/// Sends the Note [noteId] as a Sent Item (ADR 0009, 0015): asks who
/// gets it, writes the file, encrypted to that person or signed only,
/// and offers it to the app the user picks. A Note sent before goes as
/// an update, or as its cancellation.
Future<void> sendNoteFile(
  BuildContext context,
  Store store,
  String noteId,
) async {
  final t = context.t;
  final announcements = AnnouncementsScope.of(context);
  final files = BundleScope.of(context);
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
    announcements?.announce(t.sendNoApp);
    return;
  }
  File(path).writeAsBytesSync(
    sendNote(store, noteId, to: picked.to, cancel: picked.cancel),
  );
  if (!await files.share(path, type: sentItemMimeType)) {
    announcements?.announce(t.sendNoApp);
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
                  title: Text(t.sendEncrypted(r.name)),
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
    final opened = openSentItem(store, bytes);
    await navigator.push(
      MaterialPageRoute<void>(
        builder: (_) => SentItemScreen(
          store: store,
          opened: opened,
          reopen: () => openSentItem(store, bytes),
        ),
      ),
    );
  } on NotASentItem {
    announcements?.announce(t.sentItemUnreadable);
  } on SentItemNotForUs {
    announcements?.announce(t.sentItemNotForUs);
  } on SentItemNotFromSender {
    announcements?.announce(t.sentItemNotFromSender);
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
/// closes.
class SentItemScreen extends StatefulWidget {
  final Store store;
  final OpenedSentItem opened;

  /// Reads the item again, as it shows after the sender's Profile was
  /// opened and an Override set there (#52).
  final OpenedSentItem Function()? reopen;

  const SentItemScreen({
    super.key,
    required this.store,
    required this.opened,
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

  /// Shows the sender as the user sees them now.
  void _reopen() {
    final reopen = widget.reopen;
    if (reopen == null) return;
    try {
      opened = reopen();
    } on Exception {
      // Read once already; what showed stays.
    }
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
    try {
      acceptSentItem(
        store,
        opened.item,
        verified: _compared == _Compared.match,
      );
      announcements?.announce(switch (opened.state) {
        SentItemState.update => t.sentItemApplied,
        SentItemState.cancel => t.sentItemTrashed,
        _ => t.sentItemAccepted,
      });
    } on SentItemNotFromSender {
      announcements?.announce(t.sentItemNotFromSender);
    }
    Navigator.of(context).pop();
  }

  void _decline(BuildContext context) {
    AnnouncementsScope.of(context)?.announce(context.t.sentItemDeclined);
    Navigator.of(context).pop();
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
      body: ListView(
        padding: const EdgeInsets.all(16),
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
                              if (mounted) setState(_reopen);
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
                  icon: opened.encrypted ? Icons.lock_outline : Icons.lock_open,
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
                      text: t.sentItemCancelled(item.text),
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
                          ? Navigator.of(context).pop()
                          : _decline(context),
                      icon: const Icon(Icons.close),
                      label: Text(
                        accept == null ? t.sentItemClose : t.sentItemDecline,
                      ),
                    ),
                  ],
                ),
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
                      text: t.sentItemTextNow(item.text),
                    ),
                    if (changes.textEdited)
                      _Line(
                        icon: Icons.warning_amber,
                        text: t.sentItemTextYours,
                      ),
                  ],
                  for (final s in changes.sectionsAdded)
                    _Line(
                      icon: Icons.add,
                      text: t.sentItemSectionAdded(s.text),
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
            child: Text(item.text, style: text.bodyLarge),
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
                    Text(section.text, style: text.bodyLarge),
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
