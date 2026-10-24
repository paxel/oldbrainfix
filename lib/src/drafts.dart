import 'dart:convert';
import 'dart:io';

import 'package:flutter/widgets.dart';

/// Text typed into a Note's editor that has not reached the log yet, and
/// when it was typed.
typedef Draft = ({String text, DateTime saved});

/// Where the editor keeps its [Draft]s, one per Note, so typing survives
/// the app being killed. Not the log: drafts are never synced.
abstract interface class Drafts {
  Future<Draft?> read(String noteId);
  Future<void> write(String noteId, Draft draft);
  Future<void> delete(String noteId);
}

/// [Drafts] in memory: gone with the app. For tests and as the default.
class MemoryDrafts implements Drafts {
  final _held = <String, Draft>{};

  @override
  Future<Draft?> read(String noteId) async => _held[noteId];

  @override
  Future<void> write(String noteId, Draft draft) async => _held[noteId] = draft;

  @override
  Future<void> delete(String noteId) async => _held.remove(noteId);
}

/// [Drafts] as small JSON files in [dir], one per Note. Operations run
/// one after another, so a late write never outlives a delete.
class FileDrafts implements Drafts {
  final Directory dir;
  Future<void> _last = Future.value();

  FileDrafts(this.dir);

  File _file(String noteId) => File(
    '${dir.path}/${noteId.replaceAll(RegExp('[^A-Za-z0-9]'), '_')}.json',
  );

  Future<T> _inTurn<T>(Future<T> Function() op) {
    final result = _last.then((_) => op());
    _last = result.then((_) {}, onError: (_) {});
    return result;
  }

  @override
  Future<Draft?> read(String noteId) => _inTurn(() async {
    final file = _file(noteId);
    if (!await file.exists()) return null;
    try {
      final json = jsonDecode(await file.readAsString());
      if (json case {'text': final String text, 'saved': final String saved}) {
        return (text: text, saved: DateTime.parse(saved));
      }
    } on FormatException {
      // A draft cut off mid-write is no draft.
    }
    return null;
  });

  @override
  Future<void> write(String noteId, Draft draft) => _inTurn(() async {
    await dir.create(recursive: true);
    await _file(noteId).writeAsString(
      jsonEncode({
        'text': draft.text,
        'saved': draft.saved.toUtc().toIso8601String(),
      }),
      flush: true,
    );
  });

  @override
  Future<void> delete(String noteId) => _inTurn(() async {
    final file = _file(noteId);
    if (await file.exists()) await file.delete();
  });
}

/// Hands the app's [Drafts] down to every Note editor.
class DraftsScope extends InheritedWidget {
  final Drafts drafts;

  const DraftsScope({super.key, required this.drafts, required super.child});

  /// The app's [Drafts]; read once, so it does not listen.
  static Drafts of(BuildContext context) =>
      context.getInheritedWidgetOfExactType<DraftsScope>()?.drafts ??
      MemoryDrafts();

  @override
  bool updateShouldNotify(DraftsScope oldWidget) => drafts != oldWidget.drafts;
}
