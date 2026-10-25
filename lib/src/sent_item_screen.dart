import 'dart:io';

import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'announcements.dart';
import 'bundle_platform.dart';
import 'l10n.dart';
import 'note_editor_screen.dart';
import 'schedule_text.dart';

/// Sends the Note [noteId] as a Sent Item (ADR 0009, 0015): asks who
/// gets it, writes the file, encrypted to that person or signed only,
/// and offers it to the app the user picks.
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
    builder: (_) => SendNoteDialog(recipients: sentItemRecipients(store)),
  );
  if (picked == null) return;
  final path = await files.exportPath(sentItemFileName(store.clock()));
  if (path == null) {
    announcements?.announce(t.sendNoApp);
    return;
  }
  File(path).writeAsBytesSync(sendNote(store, noteId, to: picked.to));
  if (!await files.share(path, type: sentItemMimeType)) {
    announcements?.announce(t.sendNoApp);
  }
}

/// Who a Sent Item goes to: a known person, or anyone (null).
typedef _Pick = ({Recipient? to});

/// Asks who gets a Note: anyone, signed only, with a hint that whoever
/// holds the file can read it, or one of the people whose keys are
/// known here, encrypted to them.
class SendNoteDialog extends StatefulWidget {
  final List<Recipient> recipients;

  const SendNoteDialog({super.key, required this.recipients});

  @override
  State<SendNoteDialog> createState() => _SendNoteDialogState();
}

class _SendNoteDialogState extends State<SendNoteDialog> {
  /// The index of the person picked; -1 for anyone.
  late var _picked = widget.recipients.isEmpty ? -1 : 0;

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final recipients = widget.recipients;
    return AlertDialog(
      title: Text(t.sendTitle),
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
          onPressed: () =>
              Navigator.of(context)
                  .pop<_Pick>((to: _picked < 0 ? null : recipients[_picked])),
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
        builder: (_) => SentItemScreen(store: store, opened: opened),
      ),
    );
  } on NotASentItem {
    announcements?.announce(t.sentItemUnreadable);
  } on SentItemNotForUs {
    announcements?.announce(t.sentItemNotForUs);
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

/// The preview of a Sent Item: who sent it, verified or not with their
/// key's code, whether it came encrypted, its text, Sections and
/// Routines and Reminders with their times. Accept takes it into
/// Unsorted with its alarms on; Decline keeps nothing.
class SentItemScreen extends StatelessWidget {
  final Store store;
  final OpenedSentItem opened;

  const SentItemScreen({super.key, required this.store, required this.opened});

  void _accept(BuildContext context) {
    final announcements = AnnouncementsScope.of(context);
    final t = context.t;
    acceptSentItem(store, opened.item);
    announcements?.announce(t.sentItemAccepted);
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
    final code = item.from.code;
    final name = item.name.trim().isEmpty ? t.sentItemSomeone : item.name;
    final (trustIcon, trustText) = switch (opened.trust) {
      SenderTrust.verified => (Icons.verified_user, t.sentItemVerified(code)),
      SenderTrust.known => (Icons.gpp_maybe_outlined, t.sentItemKnown(code)),
      SenderTrust.unknown => (
        Icons.gpp_maybe_outlined,
        t.sentItemUnknown(code),
      ),
    };
    final already = opened.accepted != null;
    return Scaffold(
      appBar: AppBar(title: Text(t.sentItemTitle)),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          EditorPanel(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(t.sentItemFrom(name), style: text.titleMedium),
                const SizedBox(height: 8),
                _Line(icon: trustIcon, text: trustText),
                _Line(
                  icon: opened.encrypted ? Icons.lock_outline : Icons.lock_open,
                  text: opened.encrypted
                      ? t.sentItemEncrypted
                      : t.sentItemPlain,
                ),
                if (already) _Line(icon: Icons.check, text: t.sentItemAlready),
                const SizedBox(height: 8),
                Wrap(
                  spacing: 8,
                  runSpacing: 8,
                  children: [
                    if (!already)
                      FilledButton.icon(
                        key: const ValueKey('accept-sent'),
                        onPressed: () => _accept(context),
                        icon: const Icon(Icons.check),
                        label: Text(t.sentItemAccept),
                      ),
                    OutlinedButton.icon(
                      key: const ValueKey('decline-sent'),
                      onPressed: () => already
                          ? Navigator.of(context).pop()
                          : _decline(context),
                      icon: const Icon(Icons.close),
                      label: Text(t.sentItemDecline),
                    ),
                  ],
                ),
              ],
            ),
          ),
          EditorPanel(
            heading: t.noteTitle,
            child: Text(item.text, style: text.bodyLarge),
          ),
          if (item.routines.isNotEmpty)
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
                      text: scheduleText(t, locale, r.schedule),
                    ),
                ],
              ),
            ),
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
