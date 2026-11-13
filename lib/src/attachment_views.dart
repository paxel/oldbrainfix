import 'dart:async';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'announcements.dart';
import 'attachment_reader.dart';
import 'bundle_platform.dart';
import 'failure_dialog.dart';
import 'file_platform.dart';
import 'l10n.dart';
import 'ocr_platform.dart';
import 'pack_screen.dart' show packSizeText;

/// Reads the bytes of a file on this Device; tests read in place, where
/// the file system's own futures do not run.
@visibleForTesting
Future<Uint8List> Function(String path) readDeviceFile = (path) =>
    File(path).readAsBytes();

/// Writes [bytes] to [path] for another app to open; tests write in place.
@visibleForTesting
Future<void> Function(String path, Uint8List bytes) writeOpenedFile = (
  path,
  bytes,
) => File(path).writeAsBytes(bytes, flush: true);

/// The icon of a file of [kind] (#267): a PDF, a text document, a
/// table, plain text or Markdown, anything else.
IconData attachmentIcon(FileKind kind) => switch (kind) {
  FileKind.pdf => Icons.picture_as_pdf_outlined,
  FileKind.docx || FileKind.odt => Icons.description_outlined,
  FileKind.xlsx || FileKind.ods => Icons.grid_on_outlined,
  FileKind.text => Icons.article_outlined,
  FileKind.other => Icons.insert_drive_file_outlined,
};

/// What a file's row says under its name: its size, and its pages when
/// they were counted.
String attachmentFacts(BuildContext context, Attachment file) {
  final locale = Localizations.localeOf(context).toString();
  return [
    packSizeText(locale, file.size),
    if (file.pages case final pages?) context.t.attachmentPages(pages),
  ].join(' · ');
}

/// The words a Note made from a file alone goes by where only its words
/// fit, as in a row or an alarm (#267): the file's name, else the word
/// for a file.
String aloneFileTitle(BuildContext context, Attachment file) =>
    file.name.isEmpty ? context.t.attachment : file.name;

/// An Attachment of a Note (#267, ADR 0047): the icon of its kind, its
/// name and size; a tap opens it with the system's app ([openAttachment]).
/// [large] draws the name as a card's title, for a Note made from the
/// file alone.
class AttachmentRow extends StatelessWidget {
  final Attachment file;
  final bool large;

  const AttachmentRow({super.key, required this.file, this.large = false});

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final theme = Theme.of(context);
    final name = aloneFileTitle(context, file);
    final notes = AttachmentScope.of(context);
    return Semantics(
      button: notes != null,
      label: t.openFile(name),
      excludeSemantics: true,
      child: InkWell(
        borderRadius: BorderRadius.circular(8),
        onTap: notes == null
            ? null
            : () => unawaited(openAttachment(context, notes, file)),
        child: Padding(
          padding: const EdgeInsets.symmetric(vertical: 4),
          child: ConstrainedBox(
            constraints: const BoxConstraints(minHeight: 48),
            child: Row(
              children: [
                Icon(
                  attachmentIcon(file.kind),
                  size: large ? 36 : 28,
                  color: theme.colorScheme.primary,
                ),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Text(
                        name,
                        maxLines: 2,
                        overflow: TextOverflow.ellipsis,
                        style: large
                            ? theme.textTheme.titleMedium
                            : theme.textTheme.bodyLarge,
                      ),
                      Text(
                        attachmentFacts(context, file),
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: theme.colorScheme.onSurfaceVariant,
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

/// The Attachment Text of [file], dimmed so it does not read as typed
/// text: [text] as read with the Note; under the words "Attachment Text"
/// when [labelled], its first three lines when [compact]. While a
/// computer is to read it and has not ([Attachment.computer]), it says
/// that the text follows from the computer; else nothing until it is
/// read.
class AttachmentText extends StatelessWidget {
  final Attachment file;
  final String? text;
  final bool compact;
  final bool labelled;

  const AttachmentText({
    super.key,
    required this.file,
    required this.text,
    this.compact = false,
    this.labelled = false,
  });

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final theme = Theme.of(context);
    final dimmed = theme.textTheme.bodyMedium?.copyWith(
      color: theme.colorScheme.onSurfaceVariant,
    );
    final text = this.text;
    if (text == null) {
      if (!file.computer || !file.kind.hasText) return const SizedBox.shrink();
      return Padding(
        key: ValueKey('file-text-later:${file.hash}'),
        padding: const EdgeInsets.only(top: 4),
        child: Row(
          children: [
            Icon(
              Icons.computer_outlined,
              size: 16,
              color: theme.colorScheme.onSurfaceVariant,
            ),
            const SizedBox(width: 6),
            Flexible(
              child: Text(
                t.attachmentTextFromComputer,
                style: dimmed?.copyWith(fontStyle: FontStyle.italic),
              ),
            ),
          ],
        ),
      );
    }
    if (text.isEmpty) return const SizedBox.shrink();
    final shown = Text(
      text,
      key: ValueKey('file-text:${file.hash}'),
      maxLines: compact ? 3 : null,
      overflow: compact ? TextOverflow.ellipsis : null,
      style: dimmed,
    );
    return Padding(
      padding: const EdgeInsets.only(top: 4),
      child: labelled
          ? GeneratedTextLabel(label: t.attachmentTextLabel, child: shown)
          : shown,
    );
  }
}

/// Hands the [Notes] whose files the rows below open down to them; a row
/// without one only shows its file.
class AttachmentScope extends InheritedWidget {
  final Notes notes;

  const AttachmentScope({super.key, required this.notes, required super.child});

  static Notes? of(BuildContext context) =>
      context.getInheritedWidgetOfExactType<AttachmentScope>()?.notes;

  @override
  bool updateShouldNotify(AttachmentScope oldWidget) =>
      notes != oldWidget.notes;
}

/// Opens [file] of [notes] with the system's app for its type: written
/// where the app hands files to other apps first. A file whose bytes are
/// not on this Device yet, or one no app opens, is announced at the top.
Future<void> openAttachment(
  BuildContext context,
  Notes notes,
  Attachment file,
) async {
  final t = context.t;
  final announcements = AnnouncementsScope.of(context);
  final navigator = Navigator.of(context, rootNavigator: true);
  final bundles = BundleScope.of(context);
  final files = FileScope.of(context);
  void failed(String cause, String fix, String details) => announceFailureWith(
    announcements,
    navigator,
    t.fileNotOpened,
    title: t.fileNotOpenedTitle,
    cause: cause,
    fix: fix,
    details: details,
  );
  final bytes = notes.fileBytes(file.hash);
  if (bytes == null) {
    return failed(
      t.fileNotHereCause,
      t.fileNotHereFix,
      'file ${file.hash}: its bytes are not on this Device yet',
    );
  }
  final path = await bundles.exportPath(file.name.isEmpty ? 'file' : file.name);
  if (path == null) {
    return failed(
      t.fileNoAppCause,
      t.fileNoAppFix,
      'file ${file.hash}: no path to write it to',
    );
  }
  try {
    await writeOpenedFile(path, bytes);
  } on FileSystemException catch (e) {
    return failed(t.fileNoAppCause, t.fileNoAppFix, 'file ${file.hash}: $e');
  }
  if (!await files.open(path, file.mediaType)) {
    failed(
      t.fileNoAppCause,
      t.fileNoAppFix,
      'file ${file.hash}: no app took ${file.mediaType}',
    );
  }
}

/// The files picked or shared on this Device, read in as the core
/// attaches them, and those left out: each with its reason in [refused],
/// [tooLarge] when one was larger than a file may be.
typedef ReadFiles = ({
  List<NewFile> files,
  List<String> refused,
  bool tooLarge,
});

/// The files [picked] on this Device as the core attaches them (#267):
/// each read in, a PDF's pages counted where this Device has a reader for
/// them, so the core knows who reads its text (Q645). A file larger than
/// the most a file can be ([maxFileBytes]) or one that cannot be read is
/// left out ([announceNotAttached]); nothing of it is written. The copies
/// on this Device go either way.
Future<ReadFiles> readPickedFiles(
  FilePlatform files,
  List<DeviceFile> picked,
) async {
  final kept = <NewFile>[];
  final refused = <String>[];
  var tooLarge = false;
  try {
    for (final file in picked) {
      final name = file.name.isEmpty ? file.path.split('/').last : file.name;
      try {
        final size = File(file.path).lengthSync();
        if (size > maxFileBytes) {
          tooLarge = true;
          refused.add('$name: $size bytes');
          continue;
        }
        final bytes = await readDeviceFile(file.path);
        final kind = FileTexts.kindOf(name, file.type);
        final pages = kind == FileKind.pdf
            ? await files.pdfPages(file.path)
            : null;
        kept.add(
          NewFile(name: name, mediaType: file.type, bytes: bytes, pages: pages),
        );
      } on FileSystemException catch (e) {
        refused.add('$name: $e');
      }
    }
  } finally {
    dropDeviceFiles(picked);
  }
  return (files: kept, refused: refused, tooLarge: tooLarge);
}

/// Says at the top that files of [read] were left out, when any were;
/// its tap gives why.
void announceNotAttached(
  AppLocalizations t,
  Announcements? announcements,
  NavigatorState navigator,
  ReadFiles read,
) {
  if (read.refused.isEmpty) return;
  announceFailureWith(
    announcements,
    navigator,
    t.fileNotAttached,
    title: t.fileNotAttachedTitle,
    cause: read.tooLarge ? t.fileTooLargeCause : t.fileUnreadableCause,
    fix: t.fileNotAttachedFix,
    details: read.refused.join('\n'),
  );
}

/// Attaches [files] to the Note [noteId] of [notes]; how many it did. A
/// file the core refuses is left out.
int attachFiles(Notes notes, String noteId, List<NewFile> files) {
  var attached = 0;
  for (final file in files) {
    try {
      notes.attachFile(noteId, file);
      attached++;
    } on ArgumentError {
      // Larger than a file may be: checked before already.
    }
  }
  return attached;
}

/// Reads the file [hash] of the Note [noteId] again, its Attachment Text
/// in place of the one before (ADR 0025); a failure is announced at the
/// top. Whether it did.
Future<bool> readAttachmentAgain(
  BuildContext context,
  String noteId,
  String hash,
) async {
  final reader = AttachmentReaderScope.of(context);
  if (reader == null) return false;
  final t = context.t;
  final announcements = AnnouncementsScope.of(context);
  final navigator = Navigator.of(context, rootNavigator: true);
  final read = await reader.readAgain(noteId, hash);
  if (!read) {
    announceFailureWith(
      announcements,
      navigator,
      t.fileReadAgainFailed,
      title: t.fileReadAgainFailedTitle,
      cause: t.fileReadAgainCause,
      fix: t.generatedTextAgainFix,
      details: 'read again: no Attachment Text for $hash',
    );
  }
  return read;
}
