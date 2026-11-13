import 'dart:convert';
import 'dart:io';
import 'dart:isolate';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/announcements.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/attachment_reader.dart';
import 'package:oldbrainfix/src/attachment_views.dart';
import 'package:oldbrainfix/src/capture_screen.dart';
import 'package:oldbrainfix/src/file_platform.dart';
import 'package:oldbrainfix/src/l10n.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix/src/note_editor_screen.dart';
import 'package:oldbrainfix/src/photo_platform.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'bundle_test.dart' show FakeBundles;
import 'fake_files.dart';
import 'fake_ocr.dart';
import 'fake_photos.dart';
import 'open_capture.dart';

/// Attachments on the phone (#267, ADR 0047): a file of any kind picked
/// in Capture, the editor or shared in goes onto the Note; it shows with
/// its name and size, alone as the Note's card, opens with the system's
/// app, its text is read here or left to the computer, and the file and
/// its text are removed as a photo's are.
void main() {
  // The app's own way of keeping a picked file, off the UI isolate.
  final keepAside = keepDeviceFile;
  late Store store;
  late Notes notes;
  late Directory dir;
  late FakeFiles files;
  late FakePhotos photos;
  late FakeOcr ocr;
  late Announcements announcements;

  setUp(() {
    // The reading runs in place, where the test's clock reaches it.
    fileTextWork = <R>(work) async => work();
    keepDeviceFile = (files, path) async => files.putFrom(path);
    writeOpenedFile = (path, bytes) async => File(path).writeAsBytesSync(bytes);
    store = Store.inMemory(clock: () => DateTime.utc(2026, 11, 12, 9))
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    notes = Notes(store);
    dir = Directory.systemTemp.createTempSync('attachments_');
    files = FakeFiles(dir);
    photos = FakePhotos();
    ocr = FakeOcr();
    announcements = Announcements();
  });
  tearDown(() {
    fileTextWork = Isolate.run;
    keepDeviceFile = keepAside;
    store.close();
    announcements.dispose();
    dir.deleteSync(recursive: true);
  });

  Finder key(String value) => find.byKey(ValueKey(value));

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
    await tester.tap(finder);
    await tester.pumpAndSettle();
  }

  /// Lets the reader's and the file system's work arrive, then draws.
  Future<void> settle(WidgetTester tester) async {
    for (var i = 0; i < 10; i++) {
      await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 5)),
      );
      await tester.pumpAndSettle();
    }
  }

  Future<void> start(WidgetTester tester) async {
    tester.view.physicalSize = const Size(800, 2400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      OldBrainFixApp(
        store: store,
        files: files,
        photos: photos,
        ocr: ocr,
        bundles: FakeBundles(dir),
        announcements: announcements,
      ),
    );
    await settle(tester);
  }

  Future<void> allNotes(WidgetTester tester) async {
    await start(tester);
    await tap(tester, find.byTooltip('Search and browse'));
    await tap(tester, find.byTooltip('All Notes'));
  }

  Uint8List bytesOf(String text) => Uint8List.fromList(utf8.encode(text));

  testWidgets('File in Capture attaches the picked files; a text file\'s '
      'text is read here and shows dimmed under its label', (tester) async {
    files.nextPicked = [
      (name: 'Shopping.txt', bytes: bytesOf('Milk\nBread\n'), type: ''),
      (name: 'Plan.zip', bytes: Uint8List(1200), type: 'application/zip'),
    ];
    await start(tester);
    await tester.tap(find.byIcon(Icons.add));
    await tester.pumpAndSettle();
    await tap(tester, fanEntry('file'));
    await settle(tester);
    expect(find.byType(CaptureScreen), findsOneWidget);
    expect(files.picks, 1);
    final note = notes.all().single;
    expect(note.attachments.map((a) => a.name), ['Shopping.txt', 'Plan.zip']);
    expect(note.attachments.first.mediaType, 'text/plain');
    expect(find.text('Shopping.txt'), findsOneWidget);
    expect(find.text('1 KB'), findsOneWidget);
    // The copies on this Device go once attached.
    expect(dir.listSync().whereType<File>(), isEmpty);
    await tap(tester, find.text('Save'));
    await settle(tester);

    final read = notes.byId(note.id)!;
    expect(read.attachmentTexts[read.attachments.first.hash], 'Milk\nBread');
    // A .zip has no text and is not read.
    expect(read.attachmentTexts.length, 1);
    await tap(tester, find.byTooltip('Search and browse'));
    await tap(tester, find.byTooltip('All Notes'));
    expect(find.text('Attachment Text'), findsOneWidget);
    final shown = tester.widget<Text>(
      key('file-text:${read.attachments.first.hash}'),
    );
    expect(shown.data, 'Milk\nBread');
    expect(
      shown.style?.color,
      Theme.of(tester.element(find.byType(NoteCard)))
          .colorScheme
          .onSurfaceVariant,
    );
  });

  testWidgets('a Note of one file alone shows the file as its card, titled '
      'by its name, also where only its words fit', (tester) async {
    final id = notes
        .capture(
          '',
          files: [NewFile(name: 'Lease 2026.pdf', bytes: bytesOf('%PDF'))],
        )
        .id;
    await allNotes(tester);
    final row = tester.widget<AttachmentRow>(
      find.descendant(
        of: key('file:${notes.byId(id)!.attachments.single.hash}'),
        matching: find.byType(AttachmentRow),
      ),
    );
    expect(row.large, isTrue);
    expect(find.text('Lease 2026.pdf'), findsOneWidget);
    // Its pages were not counted: a computer reads it.
    expect(find.text('The text follows from the computer.'), findsOneWidget);

    final t = lookupAppLocalizations(const Locale('en'));
    final note = notes.byId(id)!;
    expect(noteText(t, note), 'Lease 2026.pdf');
    // Nameless, it is an Attachment; with a photo, it stays a photo Note.
    final nameless = notes.byId(
      notes
          .capture(
            '',
            files: [NewFile(name: '', bytes: bytesOf('x'))],
          )
          .id,
    )!;
    expect(noteText(t, nameless), 'Attachment');
    final withPhoto = notes.byId(
      notes
          .capture(
            '',
            photos: [onePixelPng],
            files: [NewFile(name: 'a.txt', bytes: bytesOf('a'))],
          )
          .id,
    )!;
    expect(noteText(t, withPhoto), 'Photo');
  });

  testWidgets('a file shared in opens Capture with it attached, as one Note', (
    tester,
  ) async {
    await start(tester);
    final copy = File('${dir.path}/shared-1')
      ..writeAsBytesSync(bytesOf('Dear tenant'));
    photos.share(
      SharedItem(
        files: [
          DeviceFile(path: copy.path, name: 'Letter.txt', type: 'text/plain'),
        ],
      ),
    );
    await settle(tester);
    expect(find.byType(CaptureScreen), findsOneWidget);
    expect(find.text('Letter.txt'), findsOneWidget);
    final note = notes.all().single;
    expect(note.attachments.single.name, 'Letter.txt');
    expect(copy.existsSync(), isFalse);
    expect(
      note.isUnsorted || note.place == null,
      isTrue,
      reason: 'kept even when the user just goes back',
    );
  });

  test('a picked file is copied and hashed off the UI isolate into the '
      'store\'s files, in chunks, and attached by its hash', () async {
    keepDeviceFile = keepAside;
    final disk = Store.open('${dir.path}/db.sqlite');
    addTearDown(disk.close);
    // Larger than one chunk, so the hash runs over several.
    final bytes = Uint8List.fromList([
      for (var i = 0; i < (1 << 20) + 300; i++) i % 251,
    ]);
    final copy = File('${dir.path}/picked')..writeAsBytesSync(bytes);
    // As the bytes hash whole.
    final whole = MemoryBlobs().putFrom(copy.path);
    final read = await readPickedFiles(files, [
      DeviceFile(path: copy.path, name: 'Big.bin'),
    ], disk);
    final file = read.files.single;
    expect(file.hash, whole);
    expect(file.size, bytes.length);
    expect(copy.existsSync(), isFalse);
    expect(disk.hasFile(file.hash!), isTrue);
    expect(Directory('${dir.path}/files').listSync().map((f) => f.path), [
      '${dir.path}/files/${file.hash}',
    ]);
    final note = Notes(disk).capture('', files: read.files);
    expect(note.attachments.single.hash, file.hash);
    expect(note.attachments.single.size, bytes.length);
    expect(Notes(disk).fileBytes(file.hash!), bytes);
  });

  test('a file said to be kept whose bytes are not here is refused', () {
    final missing = NewFile.kept(name: 'Gone.pdf', hash: 'a' * 64, size: 3);
    final note = notes.capture('Papers').id;
    expect(() => notes.attachFile(note, missing), throwsArgumentError);
    expect(() => notes.capture('', files: [missing]), throwsArgumentError);
    expect(notes.byId(note)?.attachments, isEmpty);
    expect(notes.all().length, 1);
  });

  testWidgets('a file larger than 256 MB is not attached and said at the '
      'top; the others are', (tester) async {
    final large = File('${dir.path}/large');
    final raf = large.openSync(mode: FileMode.write);
    raf
      ..setPositionSync(maxFileBytes)
      ..writeByteSync(1)
      ..closeSync();
    final small = File('${dir.path}/small')..writeAsBytesSync(bytesOf('ok'));
    final picked = [
      DeviceFile(path: large.path, name: 'Film.mkv'),
      DeviceFile(path: small.path, name: 'ok.txt'),
    ];
    final read = await readPickedFiles(files, picked, store);
    expect(read.files.map((f) => f.name), ['ok.txt']);
    // Kept before it is named, and nothing kept of the one left out.
    expect(read.files.single.bytes, isEmpty);
    expect(store.file(read.files.single.hash!), bytesOf('ok'));
    expect(read.tooLarge, isTrue);
    expect(read.refused.single, startsWith('Film.mkv'));
    expect(large.existsSync(), isFalse);
    expect(small.existsSync(), isFalse);

    await start(tester);
    final context = tester.element(find.byType(Scaffold).first);
    announceNotAttached(context.t, announcements, Navigator.of(context), read);
    expect(announcements.visible, 'A file was not attached.');
  });

  testWidgets('a PDF of up to ten pages is read here, its scans by the OCR; '
      'a longer one is left to the computer', (tester) async {
    final short = Uint8List(300);
    final long = Uint8List(400);
    final scan = Uint8List(77);
    files.pdfs[short.length] = [
      const PdfPage(text: 'Invoice 42'),
      PdfPage(scan: scan),
    ];
    files.pdfs[long.length] = List.filled(12, const PdfPage(text: 'p'));
    ocr.texts[scan.length] = 'Total 19.99';
    final picked = [
      for (final (name, bytes) in [('Invoice.pdf', short), ('Book.pdf', long)])
        DeviceFile(
          path: (File('${dir.path}/$name')..writeAsBytesSync(bytes)).path,
          name: name,
          type: 'application/pdf',
        ),
    ];
    final read = await readPickedFiles(files, picked, store);
    expect(read.files.map((f) => f.pages), [2, 12]);
    final id = notes.capture('Bills', files: read.files).id;
    final attached = notes.byId(id)!.attachments;
    expect(attached.map((a) => a.computer), [false, true]);

    await allNotes(tester);
    await settle(tester);
    final note = notes.byId(id)!;
    expect(
      note.attachmentTexts[attached.first.hash],
      'Invoice 42\n\nTotal 19.99',
    );
    expect(note.attachmentTexts.containsKey(attached.last.hash), isFalse);
    expect(ocr.readPhotos.single.length, scan.length);
    expect(ocr.idles, greaterThan(0));
    expect(find.text('The text follows from the computer.'), findsOneWidget);
    expect(find.text('300 B · 2 pages'), findsOneWidget);
  });

  test('a scan the OCR cannot read, or a phone without one, writes no '
      'text: the file waits for a computer', () async {
    final pdf = Uint8List(321);
    final scan = Uint8List(55);
    files.pdfs[pdf.length] = [
      const PdfPage(text: 'Page one'),
      PdfPage(scan: scan),
    ];
    final id = notes
        .capture(
          '',
          files: [NewFile(name: 'Scan.pdf', bytes: pdf, pages: 2)],
        )
        .id;
    for (final runs in [true, false]) {
      final reader = AttachmentReader(
        notes: notes,
        files: files,
        ocr: FakeOcr(runs: runs),
        spare: () => dir,
      );
      await Future<void>.delayed(const Duration(milliseconds: 20));
      reader.dispose();
      expect(notes.byId(id)!.attachmentTexts, isEmpty, reason: 'OCR $runs');
    }
    expect(dir.listSync(), isEmpty);
  });

  testWidgets('a tap on a file opens it with the system\'s app; one whose '
      'bytes are not here is said at the top', (tester) async {
    notes.capture(
      'Lease',
      files: [
        NewFile(
          name: 'Lease.pdf',
          bytes: bytesOf('%PDF-1.4'),
          mediaType: 'application/pdf',
        ),
      ],
    );
    await allNotes(tester);
    await tap(tester, find.text('Lease.pdf'));
    await settle(tester);
    expect(files.opened.single.type, 'application/pdf');
    expect(files.opened.single.bytes, utf8.encode('%PDF-1.4'));

    files.opens = false;
    await tap(tester, find.text('Lease.pdf'));
    await settle(tester);
    expect(announcements.visible, 'An Attachment was not opened.');
  });

  testWidgets('an app (.apk) asks before it opens, once, and Cancel '
      'opens nothing', (tester) async {
    notes.capture(
      'Game',
      files: [NewFile(name: 'game.apk', bytes: bytesOf('PK'))],
    );
    // A name that says PDF and the media type of an app asks too.
    notes.capture(
      'Lease',
      files: [
        NewFile(
          name: 'Lease.pdf',
          bytes: bytesOf('PK'),
          mediaType: apkMediaType,
        ),
      ],
    );
    await allNotes(tester);
    await tap(tester, find.text('game.apk'));
    expect(find.text('Open this file?'), findsOneWidget);
    expect(
      find.text('"game.apk" could install an app. Open it anyway?'),
      findsOneWidget,
    );
    await tap(tester, find.text('Cancel'));
    await settle(tester);
    expect(files.opened, isEmpty);
    await tap(tester, find.text('game.apk'));
    await tap(tester, key('fileRiskOpen'));
    await settle(tester);
    expect(files.opened.single.bytes, utf8.encode('PK'));

    await tap(tester, find.text('Lease.pdf'));
    expect(find.text('Open this file?'), findsOneWidget);
    await tap(tester, find.text('Cancel'));
    await settle(tester);
    expect(files.opened, hasLength(1));
  });

  test('only an app is asked about on the phone', () {
    Attachment file(String name, [String type = '']) => Attachment.read(
      'abc',
      jsonEncode({'name': name, 'type': type, 'size': 1}),
    );
    expect(mayInstall(file('game.APK')), isTrue);
    expect(mayInstall(file('bundle.xapk')), isTrue);
    expect(mayInstall(file('a.pdf', apkMediaType)), isTrue);
    expect(mayInstall(file('Lease.pdf', 'application/pdf')), isFalse);
    expect(mayInstall(file('setup.exe')), isFalse);
    expect(mayInstall(file('apk')), isFalse);
  });

  testWidgets('the editor removes the Attachment Text at once with an undo, '
      'and the file after asking once, its text staying', (tester) async {
    final id = notes
        .capture(
          'Rent',
          files: [NewFile(name: 'Rent.txt', bytes: bytesOf('500 EUR'))],
        )
        .id;
    final hash = notes.byId(id)!.attachments.single.hash;
    notes.setAttachmentText(id, hash, '500 EUR');
    await allNotes(tester);
    await tester.longPress(find.text('Rent'));
    await tester.pumpAndSettle();
    expect(find.byType(NoteEditorScreen), findsOneWidget);

    await tap(tester, key('remove-text:$hash'));
    expect(notes.byId(id)!.removedAttachmentTexts, [hash]);
    expect(announcements.visible, 'Attachment Text removed');
    announcements.visibleAction?.call();
    await tester.pumpAndSettle();
    expect(notes.byId(id)!.attachmentTexts[hash], '500 EUR');

    await settle(tester);
    await tap(tester, key('remove:$hash'));
    expect(
      find.text('Remove this Attachment and keep its Attachment Text?'),
      findsOneWidget,
    );
    await tap(tester, find.text('Remove'));
    final note = notes.byId(id)!;
    expect(note.attachments, isEmpty);
    expect(note.removedAttachments.single.name, 'Rent.txt');
    expect(note.attachmentTexts[hash], '500 EUR');
    expect(find.text('500 EUR'), findsOneWidget);
  });

  testWidgets('the editor\'s Attach a file adds files to the Note', (
    tester,
  ) async {
    final id = notes.capture('Trip').id;
    files.nextPicked = [
      (name: 'Tickets.pdf', bytes: Uint8List(10), type: 'application/pdf'),
    ];
    await allNotes(tester);
    await tester.longPress(find.text('Trip'));
    await tester.pumpAndSettle();
    await tap(tester, key('attach-file'));
    await settle(tester);
    expect(notes.byId(id)!.attachments.single.name, 'Tickets.pdf');
    expect(find.text('Tickets.pdf'), findsOneWidget);
  });

  test('a reader without a PDF reader leaves the PDF unread', () async {
    final id = notes
        .capture(
          '',
          files: [NewFile(name: 'a.pdf', bytes: Uint8List(3), pages: 1)],
        )
        .id;
    final reader = AttachmentReader(
      notes: notes,
      files: const NoFiles(),
      ocr: ocr,
      spare: () => dir,
    );
    addTearDown(reader.dispose);
    await Future<void>.delayed(const Duration(milliseconds: 20));
    expect(notes.byId(id)!.attachmentTexts, isEmpty);
    expect(dir.listSync(), isEmpty);
  });
}
