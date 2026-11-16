import 'dart:async';
import 'dart:io';
import 'dart:isolate';

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'file_platform.dart';
import 'ocr_platform.dart';

/// Runs the reading of a file's text off the UI isolate; tests run it in
/// place.
@visibleForTesting
Future<R> Function<R>(FutureOr<R> Function() work) fileTextWork = Isolate.run;

/// Reads the Attachment Texts this phone reads (#267, ADR 0047, Q645) in
/// the background, one file at a time: each new one after its Note is
/// saved, and those from before. A Word, Excel, OpenDocument, text or
/// Markdown file is read by the core off the UI isolate; a PDF by the
/// PDF reader of the app's native library, page by page, its scans by
/// the OCR. A file the attaching Device left to a computer is not read
/// here ([Attachment.computer]). A file whose bytes are not on this
/// Device yet waits for them; one that cannot be read is left alone until
/// the next start, and the file itself is always kept. Asked to, it reads
/// one file again, ahead of the others ([readAgain]).
class AttachmentReader extends ChangeNotifier {
  final Notes notes;
  final FilePlatform files;
  final OcrPlatform ocr;

  /// Where a PDF is put for the native reader, a file of its own each
  /// time; the app's cache.
  final Directory Function() spare;

  /// Files that could not be read since the start.
  final _skipped = <String>{};

  /// Files whose bytes were not here; looked at again after the next
  /// change of the log.
  final _missing = <String>{};

  /// Files the user asked to read again, oldest ask first.
  final _asked = <({String noteId, String hash, Completer<bool> done})>[];
  String? _current;
  var _running = false;
  var _queued = false;
  var _again = false;
  var _disposed = false;

  AttachmentReader({
    required this.notes,
    required this.files,
    required this.ocr,
    Directory Function()? spare,
  }) : spare = spare ?? (() => Directory.systemTemp) {
    notes.store.addListener(_changed);
    _kick();
  }

  /// The file being read, by its hash; null while none is.
  String? get current => _current;

  /// Reads the file [hash] of the Note [noteId] again, after the one
  /// running if any, and keeps its text in place of the one before or of
  /// a removed one; whether it did. A file left to a computer is not read
  /// here ([Attachment.computer]).
  Future<bool> readAgain(String noteId, String hash) {
    final file = notes
        .byId(noteId)
        ?.attachments
        .where((a) => a.hash == hash)
        .firstOrNull;
    if (_disposed || file == null || file.computer) return Future.value(false);
    final done = Completer<bool>();
    _asked.add((noteId: noteId, hash: hash, done: done));
    _skipped.remove(hash);
    _missing.remove(hash);
    _kick();
    return done.future;
  }

  void _changed() {
    // Bytes may have come with the change.
    _missing.clear();
    _kick();
  }

  void _kick() {
    if (_running) _again = true;
    if (_queued || _running) return;
    _queued = true;
    scheduleMicrotask(() {
      _queued = false;
      if (!_disposed) unawaited(_run());
    });
  }

  Future<void> _run() async {
    if (_running) return;
    _running = true;
    _again = false;
    Completer<bool>? waiting;
    var ocrUsed = false;
    try {
      while (!_disposed) {
        final asked = _asked.isEmpty ? null : _asked.removeAt(0);
        waiting = asked?.done;
        final next = asked == null
            ? notes
                  .unreadAttachments(computer: false)
                  .where(
                    (f) =>
                        !_skipped.contains(f.hash) &&
                        !_missing.contains(f.hash),
                  )
                  .firstOrNull
            : (noteId: asked.noteId, hash: asked.hash);
        if (next == null) break;
        final file = notes
            .byId(next.noteId)
            ?.attachments
            .where((a) => a.hash == next.hash)
            .firstOrNull;
        final bytes = file == null ? null : notes.fileBytes(next.hash);
        String? text;
        if (file != null && bytes == null) {
          _missing.add(next.hash);
        } else if (file != null && bytes != null) {
          _current = next.hash;
          notifyListeners();
          final read = await _read(file, bytes);
          if (_disposed) return;
          text = read.text;
          ocrUsed = ocrUsed || read.ocr;
          _current = null;
          if (text == null) _skipped.add(next.hash);
        } else {
          _skipped.add(next.hash);
        }
        // Still there: the Note may have been trashed or the file removed.
        final kept =
            text != null &&
            (notes
                    .byId(next.noteId)
                    ?.attachments
                    .any((a) => a.hash == next.hash) ??
                false);
        if (kept) notes.setAttachmentText(next.noteId, next.hash, text);
        waiting?.complete(kept);
        waiting = null;
        notifyListeners();
      }
      if (ocrUsed) await ocr.idle();
    } on Object {
      // A failure reading leaves the file as it is, to be read again
      // after the next start; nothing is lost.
    } finally {
      _running = false;
      waiting?.complete(false);
      if (_disposed) _dropAsked();
    }
    if (_again) _kick();
  }

  /// The text of [file], [bytes] long, or null when it cannot be read
  /// here; whether the OCR read a scan for it.
  Future<({String? text, bool ocr})> _read(
    Attachment file,
    Uint8List bytes,
  ) async {
    final kind = file.kind;
    if (kind != FileKind.pdf) {
      try {
        final reading = await fileTextWork(() => FileTexts.read(kind, bytes));
        return switch (reading) {
          TextRead(:final text) => (text: text, ocr: false),
          _ => (text: null, ocr: false),
        };
      } on Object {
        return (text: null, ocr: false);
      }
    }
    // The native reader reads the PDF from a file of its own.
    final path =
        '${spare().path}/attachment-${file.hash.substring(0, 16)}-'
        '${DateTime.now().microsecondsSinceEpoch}.pdf';
    try {
      File(path).writeAsBytesSync(bytes, flush: true);
    } on FileSystemException {
      return (text: null, ocr: false);
    }
    var usedOcr = false;
    try {
      final count = await files.pdfPages(path);
      if (count == null || _disposed) return (text: null, ocr: false);
      bool? readsScans;
      final pages = <String>[];
      for (var i = 0; i < count && !_disposed; i++) {
        final page = await files.pdfPage(path, i);
        if (page == null) continue;
        var text = page.text;
        if (page.scan case final scan?) {
          // A scan not read here leaves the file to a computer, which
          // reads every file still waiting: nothing half is written.
          if (!(readsScans ??= await ocr.supported())) {
            return (text: null, ocr: usedOcr);
          }
          usedOcr = true;
          final read = await ocr.read(scan);
          if (read == null) return (text: null, ocr: usedOcr);
          text = read;
        }
        pages.add(text);
      }
      return (text: FileTexts.joinPages(pages), ocr: usedOcr);
    } finally {
      try {
        File(path).deleteSync();
      } on FileSystemException {
        // Gone already.
      }
    }
  }

  void _dropAsked() {
    for (final asked in _asked) {
      asked.done.complete(false);
    }
    _asked.clear();
  }

  @override
  void dispose() {
    _disposed = true;
    _dropAsked();
    notes.store.removeListener(_changed);
    super.dispose();
  }
}

/// Hands the app's [AttachmentReader] down to every screen.
class AttachmentReaderScope extends InheritedWidget {
  final AttachmentReader reader;

  const AttachmentReaderScope({
    super.key,
    required this.reader,
    required super.child,
  });

  /// The app's [AttachmentReader], or null where there is none; read
  /// once, so it does not listen.
  static AttachmentReader? of(BuildContext context) =>
      context.getInheritedWidgetOfExactType<AttachmentReaderScope>()?.reader;

  @override
  bool updateShouldNotify(AttachmentReaderScope oldWidget) =>
      reader != oldWidget.reader;
}
