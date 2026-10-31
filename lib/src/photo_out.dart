import 'dart:io';

import 'package:flutter/material.dart';

import 'announcements.dart';
import 'bundle_platform.dart';
import 'failure_dialog.dart';
import 'l10n.dart';
import 'photo_platform.dart';
import 'share_note.dart';

/// What the ⋮ of the full-screen photo does with it (#149).
enum PhotoOut {
  /// Into the Device's Pictures.
  save,

  /// On the clipboard, as a picture.
  copy,

  /// To the app the user picks on the share sheet.
  share,
}

/// The name the photo [hash] goes out as, as the desktop and a shared
/// Note name it: `photo-` and the first 12 characters of its hash, with
/// the [extension] its bytes have.
String photoOutName(String hash, String extension) {
  final short = hash.length > 12 ? hash.substring(0, 12) : hash;
  return 'photo-$short.$extension';
}

/// Hands the photo [hash] out of its Note as [out] asks (#149): its file
/// is written where [BundlePlatform.exportPath] says, then saved into the
/// Device's Pictures, put on the clipboard or offered on the share sheet.
/// Saved and copied say nothing, nor does a share sheet left without an
/// app (UI laws); a failure is announced at the top, why behind the tap.
/// Only a photo this Device holds gets here: the viewer offers nothing
/// for one that is not.
Future<void> photoOut(BuildContext context, String hash, PhotoOut out) async {
  final t = context.t;
  final announcements = AnnouncementsScope.of(context);
  final navigator = Navigator.of(context, rootNavigator: true);
  final files = BundleScope.of(context);
  final photos = PhotoScope.of(context);
  final bytes = PhotoScope.bytesOf(context, hash);
  if (bytes == null) return;
  final (said, title) = switch (out) {
    PhotoOut.save => (t.photoSaveFailed, t.photoSaveFailedTitle),
    PhotoOut.copy => (t.photoCopyFailed, t.photoCopyFailedTitle),
    PhotoOut.share => (t.photoShareFailed, t.photoShareFailedTitle),
  };
  void failed(String cause, String fix, String details) => announceFailureWith(
    announcements,
    navigator,
    said,
    title: title,
    cause: cause,
    fix: fix,
    details: details,
  );
  final (extension, type) = photoKind(bytes);
  final name = photoOutName(hash, extension);
  final String path;
  try {
    final written = await files.exportPath(name);
    if (written == null) {
      failed(
        t.photoNotWrittenCause,
        t.nothingLostFix,
        'photo: no path to write the file to',
      );
      return;
    }
    // The same bytes under the same name: a file another app may still
    // read is never changed.
    await File(written).writeAsBytes(bytes, flush: true);
    path = written;
  } on Object catch (e) {
    failed(t.photoNotWrittenCause, t.nothingLostFix, '$e');
    return;
  }
  switch (out) {
    case PhotoOut.save:
      switch (await photos.savePhoto(path, name, type)) {
        case PhotoSaved.saved:
          break;
        case PhotoSaved.tooOld:
          failed(
            t.photoSaveTooOldCause,
            t.photoSaveTooOldFix,
            'save photo: Android older than 10',
          );
        case PhotoSaved.failed:
          failed(
            t.photoSaveFailedCause,
            t.photoSaveFailedFix,
            'save photo: not written into Pictures',
          );
      }
    case PhotoOut.copy:
      if (!await photos.copyPhoto(path, type)) {
        failed(
          t.photoCopyFailedCause,
          t.photoCopyFailedFix,
          'copy photo: the clipboard took no picture',
        );
      }
    case PhotoOut.share:
      if (await files.share(path, type: type) == ShareOutcome.noApp) {
        failed(
          t.photoShareNoAppCause,
          t.photoShareNoAppFix,
          'share photo: no app took it',
        );
      }
  }
}
