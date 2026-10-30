import 'dart:io';

import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'announcements.dart';
import 'bundle_platform.dart';
import 'failure_dialog.dart';
import 'l10n.dart';

/// Hands the Note [noteId] to the app the user picks on the share sheet
/// (#129): its text and its Sections, each under its date as the editor
/// shows dates, with HTML beside a formatted text ([NoteShare]), and the
/// photos and recordings this Device holds as files. Only a Note the
/// screens show gets here, so a locked Area's are never handed along.
/// Announces a failure when no file could be written, nothing of the
/// Note is here yet or no app takes it, why behind the tap; a share
/// sheet left without an app says nothing.
Future<void> shareNoteToApp(
  BuildContext context,
  Notes notes,
  String noteId,
) async {
  final t = context.t;
  final announcements = AnnouncementsScope.of(context);
  final navigator = Navigator.of(context, rootNavigator: true);
  final files = BundleScope.of(context);
  final when = DateFormat.yMd(Localizations.localeOf(context).toString())
      .add_Hm();
  void notShared(String cause, String fix, String details) =>
      announceFailureWith(
        announcements,
        navigator,
        t.noteNotShared,
        title: t.noteNotSharedTitle,
        cause: cause,
        fix: fix,
        details: details,
      );
  final note = notes.byId(noteId);
  if (note == null) return;
  final share = NoteShare.of(
    note,
    notes.sections(noteId),
    date: (captured) => when.format(captured.toLocal()),
  );
  final store = notes.store;
  final shared = <SharedFile>[];
  try {
    for (final (hashes, name, read, kind) in [
      (share.photos, 'photo', store.photo, _photoKind),
      (share.recordings, 'recording', store.audio, _recordingKind),
    ]) {
      for (final hash in hashes) {
        // One not on this Device yet stays behind; the rest goes.
        final bytes = read(hash);
        if (bytes == null) continue;
        final (extension, type) = kind(bytes);
        // Named by its hash: the same bytes under the same name, so a
        // file another app may still read is never changed.
        final short = hash.length > 12 ? hash.substring(0, 12) : hash;
        final path = await files.exportPath('$name-$short.$extension');
        if (path == null) {
          notShared(
            t.shareNotWrittenCause,
            t.nothingLostFix,
            'share: no path to write the file to',
          );
          return;
        }
        File(path).writeAsBytesSync(bytes, flush: true);
        shared.add((path: path, type: type));
      }
    }
  } on Object catch (e) {
    notShared(t.shareNotWrittenCause, t.nothingLostFix, '$e');
    return;
  }
  if (share.text.isEmpty && shared.isEmpty) {
    notShared(
      t.shareNothingCause,
      t.shareNothingFix,
      'share: no words and no photo or recording here',
    );
    return;
  }
  final outcome = await files.shareText(
    share.text,
    html: share.html,
    files: shared,
  );
  if (outcome == ShareOutcome.noApp) {
    notShared(t.shareNoApp, t.shareNoAppFix, 'share: no app took the Note');
  }
}

/// The file extension and MIME type of the photo [bytes]: PNG, GIF or
/// WebP by their signatures, else JPEG, as the phone writes them.
(String, String) _photoKind(List<int> bytes) {
  String tag(int at) => bytes.length < at + 4
      ? ''
      : String.fromCharCodes(bytes.sublist(at, at + 4));
  return switch (tag(0)) {
    '\x89PNG' => ('png', 'image/png'),
    'GIF8' => ('gif', 'image/gif'),
    'RIFF' when tag(8) == 'WEBP' => ('webp', 'image/webp'),
    _ => ('jpg', 'image/jpeg'),
  };
}

/// The file extension and MIME type of the recording [bytes], as
/// `Voice.extension` tells them apart (Voice.kt): WAV as the desktop
/// records, Ogg as a messenger's voice message is, MP3, else the phone's
/// own AAC in MPEG-4.
(String, String) _recordingKind(List<int> bytes) {
  String tag(int at) => bytes.length < at + 4
      ? ''
      : String.fromCharCodes(bytes.sublist(at, at + 4));
  int byte(int at) => bytes.length <= at ? 0 : bytes[at];
  return switch (tag(0)) {
    'RIFF' when tag(8) == 'WAVE' => ('wav', 'audio/wav'),
    'OggS' => ('ogg', 'audio/ogg'),
    final t when t.startsWith('ID3') => ('mp3', 'audio/mpeg'),
    // An MPEG audio frame's sync bits, with a layer set: ADTS AAC has
    // the same sync but no layer.
    _ when byte(0) == 0xFF && byte(1) & 0xE0 == 0xE0 && byte(1) & 0x06 != 0 => (
      'mp3',
      'audio/mpeg',
    ),
    _ => ('m4a', 'audio/mp4'),
  };
}
