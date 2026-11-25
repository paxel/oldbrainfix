import 'dart:io';

import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'announcements.dart';
import 'bundle_platform.dart';
import 'failure_dialog.dart';
import 'folder_platform.dart';
import 'import_run.dart';
import 'l10n.dart';

/// AXT files (obf-1145, ADR 0056): "Export as AXT file…" beside every
/// "Export as Pack…", on a Note, a selection of Notes, a Topic and an
/// Area, writes their Notes as the records of an AXT file and hands it to
/// the share sheet at once, nothing asked: the file holds what the
/// example types can say, so there is nothing to tick.

/// The extension of an AXT file.
const axtExtension = 'axt';

/// The type an AXT file is shared as: ADR 0056 names none.
const axtMimeType = 'application/octet-stream';

/// The characters a file name cannot hold on any system the file may
/// reach, and those that steer text.
final _notInNames = RegExp(r'[\\/:*?"<>|\x00-\x1F\x7F]');

/// The names Windows keeps for its devices, with any ending.
final _reserved = RegExp(
  r'^(CON|PRN|AUX|NUL|COM[1-9]|LPT[1-9])$',
  caseSensitive: false,
);

/// The longest stem of an AXT file's name, in characters.
const _stemMax = 60;

/// The name an AXT file of [title] goes by, `Buy milk.axt`: each
/// character a file name cannot hold made `_`, without blanks and full
/// stops at either end, at most 60 characters, `_` before a name Windows
/// keeps for a device; `oldbrainfix-notes-2026-11-25.axt` of [day] when
/// nothing is left.
String axtFileName(String? title, DateTime day) {
  var stem = (title ?? '').split('\n').first.replaceAll(_notInNames, '_');
  final runes = stem.runes.toList();
  if (runes.length > _stemMax) {
    stem = String.fromCharCodes(runes.take(_stemMax));
  }
  stem = stem.replaceAll(RegExp(r'^[\s.]+|[\s.]+$'), '');
  if (stem.isEmpty) {
    String two(int n) => n.toString().padLeft(2, '0');
    stem = 'oldbrainfix-notes-${day.year}-${two(day.month)}-${two(day.day)}';
  }
  if (_reserved.hasMatch(stem.split('.').first)) stem = '_$stem';
  return '$stem.$axtExtension';
}

/// The title the Note [noteId] goes by in an AXT file: its record's.
String? axtNoteTitle(Store store, String noteId) =>
    AxtFiles(store).axtRecordOf(noteId)?.$2['title'] as String?;

/// Writes the Notes [noteIds] as an AXT file named after [title]
/// ([axtFileName]) and hands it to the app the user picks. The file is
/// made and written as a long job is ([JobRunner]), in an isolate of its
/// own on the phone, so the page keeps moving for an Area of thousands
/// of Notes. A Note in the trash or merged is left out; with none left
/// nothing is written, and the top says so. A failure is announced at
/// the top, with why.
Future<void> exportAxt(
  BuildContext context,
  Store store,
  List<String> noteIds, {
  required String? title,
}) async {
  final t = context.t;
  final announcements = AnnouncementsScope.of(context);
  final navigator = Navigator.of(context, rootNavigator: true);
  final files = BundleScope.of(context);
  final runner = ImportJobsScope.of(context)?.runner;
  void notSent(String cause, String fix, String details) => announceFailureWith(
    announcements,
    navigator,
    t.axtNotSent,
    title: t.axtNotSentTitle,
    cause: cause,
    fix: fix,
    details: details,
  );
  final notes = Notes(store);
  final going = [
    for (final id in noteIds)
      if (notes.byId(id) case final note?
          when !note.isTrashed && !note.isMerged)
        id,
  ];
  if (going.isEmpty) {
    announcements?.announce(t.axtNoNotes);
    return;
  }
  final path = await files.exportPath(axtFileName(title, store.clock()));
  if (path == null) {
    notSent(t.axtNotWrittenCause, t.nothingLostFix, 'axt: no path');
    return;
  }
  final job = axtFileJob(going, path);
  final stop = JobStop();
  try {
    if (runner == null) {
      await job(store, const NoFolders(), (_, _) {}, () => false);
    } else {
      await runner.run(store, job, (_, _) {}, stop);
    }
  } on Exception catch (e) {
    notSent(t.axtNotWrittenCause, t.nothingLostFix, '$e');
    return;
  } finally {
    stop.clear();
  }
  if (await files.share(path, type: axtMimeType) == ShareOutcome.noApp) {
    notSent(t.sendNoApp, t.sendNoAppFix, 'axt: no app took the file');
  }
}

/// The job that writes the AXT file of the Notes [ids] to [path] on the
/// store it is given. At the top level, so the closure that crosses into
/// the job's isolate holds only these two.
@visibleForTesting
LongWork<void> axtFileJob(List<String> ids, String path) =>
    (store, _, _, _) async =>
        File(path).writeAsBytesSync(await AxtFiles(store).write(ids));
