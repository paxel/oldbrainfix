import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:isolate';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/announcements.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/attachment_reader.dart';
import 'package:oldbrainfix/src/attachment_views.dart';
import 'package:oldbrainfix/src/file_platform.dart';
import 'package:oldbrainfix/src/pdf_viewer.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'bundle_test.dart' show FakeBundles;
import 'fake_files.dart';
import 'fake_ocr.dart';
import 'fake_photos.dart';

/// The PDF viewer on the phone (#286): a tap on an attached PDF shows it
/// full screen, its pages swiped and zoomed, drawn only around the page
/// shown, its Attachment Text one tap away; a PDF the system cannot draw
/// and any other file open with the system's app.
void main() {
  final keepAside = keepDeviceFile;
  late Store store;
  late Notes notes;
  late Directory dir;
  late FakeFiles files;
  late Announcements announcements;

  /// An A4 page in points.
  const a4 = Size(595, 842);
  final pdf = Uint8List.fromList(utf8.encode('%PDF-1.7 lease'));

  setUp(() {
    fileTextWork = <R>(work) async => work();
    keepDeviceFile = (files, path) async => files.putFrom(path);
    writeOpenedFile = (path, bytes) async => File(path).writeAsBytesSync(bytes);
    store = Store.inMemory(clock: () => DateTime.utc(2026, 11, 13, 9))
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    notes = Notes(store);
    dir = Directory.systemTemp.createTempSync('pdf_viewer_');
    files = FakeFiles(dir);
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

  /// Lets the file system's and the renderer's work arrive, then draws.
  Future<void> settle(WidgetTester tester) async {
    for (var i = 0; i < 10; i++) {
      await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 5)),
      );
      await tester.pumpAndSettle();
    }
  }

  Future<void> tap(WidgetTester tester, Finder finder) async {
    await tester.ensureVisible(finder);
    await tester.pumpAndSettle();
    await tester.tap(finder);
    await settle(tester);
  }

  /// All Notes with a Note holding Lease.pdf of [pages] pages the system
  /// draws, its Attachment Text [text] when given.
  Future<void> open(WidgetTester tester, {int pages = 50, String? text}) async {
    files.layouts[pdf.length] = List.filled(pages, a4);
    final note = notes.capture(
      'Lease',
      files: [
        NewFile(
          name: 'Lease.pdf',
          bytes: pdf,
          mediaType: 'application/pdf',
          pages: pages,
        ),
      ],
    );
    if (text != null) {
      notes.setAttachmentText(
        note.id,
        notes.byId(note.id)!.attachments.single.hash,
        text,
      );
    }
    tester.view.physicalSize = const Size(800, 2400);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      OldBrainFixApp(
        store: store,
        files: files,
        photos: FakePhotos(),
        ocr: FakeOcr(),
        bundles: FakeBundles(dir),
        announcements: announcements,
      ),
    );
    await settle(tester);
    await tap(tester, find.byTooltip('Search and browse'));
    await tap(tester, find.byTooltip('All Notes'));
    await tap(tester, find.text('Lease.pdf'));
  }

  /// Two fingers on the page shown: spread [apart] they zoom it in,
  /// pinched together they take it back to its size.
  Future<void> pinch(WidgetTester tester, {bool apart = true}) async {
    final centre = tester.getCenter(find.byType(PageView));
    final one = await tester.startGesture(centre - const Offset(50, 0));
    final two = await tester.startGesture(
      centre + const Offset(50, 0),
      pointer: 2,
    );
    final step = apart ? 15.0 : -4.5;
    for (var i = 1; i <= 10; i++) {
      await one.moveTo(centre - Offset(50.0 + i * step, 0));
      await two.moveTo(centre + Offset(50.0 + i * step, 0));
      await tester.pump();
    }
    await one.up();
    await two.up();
    await settle(tester);
  }

  testWidgets('a tap on a PDF shows it full screen; only the page shown '
      'and the next are drawn, as wide as the screen', (tester) async {
    await open(tester);
    expect(find.byType(PdfViewerScreen), findsOneWidget);
    expect(files.opened, isEmpty);
    expect(find.text('Page 1 of 50'), findsOneWidget);
    expect(files.drawn, [(index: 0, width: 800), (index: 1, width: 800)]);
    expect(key('pdf-page-0'), findsOneWidget);
  });

  testWidgets('a swipe turns the page, which draws the one after it', (
    tester,
  ) async {
    await open(tester);
    await tester.drag(find.byType(PageView), const Offset(-600, 0));
    await settle(tester);
    expect(find.text('Page 2 of 50'), findsOneWidget);
    expect(files.drawn.map((page) => page.index), [0, 1, 2]);
    await tester.drag(find.byType(PageView), const Offset(-600, 0));
    await settle(tester);
    expect(find.text('Page 3 of 50'), findsOneWidget);
    expect(files.drawn.map((page) => page.index), [0, 1, 2, 3]);
    // Back to the first: page 0 went out of reach and is drawn again.
    await tester.drag(find.byType(PageView), const Offset(600, 0));
    await settle(tester);
    await tester.drag(find.byType(PageView), const Offset(600, 0));
    await settle(tester);
    expect(find.text('Page 1 of 50'), findsOneWidget);
    expect(files.drawn.last.index, 0);
  });

  testWidgets('a pinch zooms the page, which is drawn anew larger, and '
      'stops the swipe until it is back at its size', (tester) async {
    await open(tester);
    await pinch(tester);
    final sharp = files.drawn.last;
    expect(sharp.index, 0);
    expect(sharp.width, greaterThan(1600));
    // A drag pans the zoomed page instead of turning it.
    await tester.drag(find.byType(PageView), const Offset(-600, 0));
    await settle(tester);
    expect(find.text('Page 1 of 50'), findsOneWidget);
  });

  testWidgets('a sharper drawing that fails keeps the page as it was, '
      'not asked for again', (tester) async {
    files.widest = 1000;
    await open(tester);
    await pinch(tester);
    expect(files.drawn.last.width, greaterThan(1000));
    expect(key('pdf-page-0'), findsOneWidget);
    expect(find.byIcon(Icons.broken_image_outlined), findsNothing);
    final asked = files.drawn.length;
    await pinch(tester);
    expect(files.drawn, hasLength(asked));
    // Let go two pages away, it comes back drawn as it shows.
    await pinch(tester, apart: false);
    for (final dx in [-600.0, -600.0, 600.0, 600.0]) {
      await tester.drag(find.byType(PageView), Offset(dx, 0));
      await settle(tester);
    }
    expect(find.text('Page 1 of 50'), findsOneWidget);
    expect(files.drawn.last, (index: 0, width: 800));
    expect(key('pdf-page-0'), findsOneWidget);
  });

  testWidgets('a page drawn after the pages turned away from it is not '
      'kept', (tester) async {
    final waiting = files.held[2] = Completer<Uint8List?>();
    await open(tester);
    await tester.drag(find.byType(PageView), const Offset(-600, 0));
    await settle(tester);
    expect(files.drawn.last.index, 2);
    await tester.drag(find.byType(PageView), const Offset(600, 0));
    await settle(tester);
    expect(find.text('Page 1 of 50'), findsOneWidget);
    files.held.clear();
    waiting.complete(onePixelPng);
    await settle(tester);
    // Page 2 was let go, so turning towards it again draws it anew.
    await tester.drag(find.byType(PageView), const Offset(-600, 0));
    await settle(tester);
    expect(files.drawn.where((page) => page.index == 2), hasLength(2));
  });

  testWidgets('the Attachment Text is one tap away, and back', (tester) async {
    await open(tester, text: 'Lease of the flat, from 1 May 2026.');
    expect(find.text('Lease of the flat, from 1 May 2026.'), findsNothing);
    await tap(tester, key('pdf-text'));
    expect(find.text('Lease of the flat, from 1 May 2026.'), findsOneWidget);
    expect(find.byType(PageView), findsNothing);
    await tap(tester, key('pdf-text'));
    expect(find.byType(PageView), findsOneWidget);
  });

  testWidgets('a PDF read here without text has no text button, and one '
      'page no page count', (tester) async {
    await open(tester, pages: 1);
    expect(key('pdf-text'), findsNothing);
    expect(key('pdf-page-of'), findsNothing);
    expect(files.drawn, [(index: 0, width: 800)]);
  });

  testWidgets('closing the viewer takes its copy away; one handed to '
      'another app stays', (tester) async {
    await open(tester);
    expect(File('${dir.path}/Lease.pdf').existsSync(), isTrue);
    await tap(tester, find.byType(BackButton));
    expect(find.byType(PdfViewerScreen), findsNothing);
    expect(File('${dir.path}/Lease.pdf').existsSync(), isFalse);

    await tap(tester, find.text('Lease.pdf'));
    await tap(tester, key('pdf-menu'));
    await tap(tester, find.text('Open in another app'));
    expect(files.opened.single.type, 'application/pdf');
    expect(files.opened.single.bytes, pdf);
    await tap(tester, find.byType(BackButton));
    expect(File('${dir.path}/Lease.pdf').existsSync(), isTrue);
  });

  testWidgets('a page the system cannot draw shows as broken, the others '
      'still do', (tester) async {
    files.undrawable.add(0);
    await open(tester);
    expect(find.byIcon(Icons.broken_image_outlined), findsOneWidget);
    expect(files.drawn.map((page) => page.index), [0, 1]);
  });

  testWidgets('a PDF the system cannot draw, and a Word file, open with '
      'the system\'s app', (tester) async {
    await open(tester);
    await tap(tester, find.byType(BackButton));
    files.layouts.clear();
    await tap(tester, find.text('Lease.pdf'));
    expect(find.byType(PdfViewerScreen), findsNothing);
    expect(files.opened.single.type, 'application/pdf');

    notes.capture(
      'Letter',
      files: [
        NewFile(
          name: 'Letter.docx',
          bytes: Uint8List.fromList(utf8.encode('PK letter')),
        ),
      ],
    );
    await tester.pumpAndSettle();
    await tap(tester, find.text('Letter.docx'));
    expect(find.byType(PdfViewerScreen), findsNothing);
    expect(files.opened, hasLength(2));
  });

  test('the sizes of the channel\'s answer are its pairs of points', () {
    expect(pdfSizes([595, 842, 842.5, 595.0]), [a4, const Size(842.5, 595)]);
    expect(pdfSizes(const []), isEmpty);
    expect(pdfSizes(null), isNull);
    expect(pdfSizes([595]), isNull);
    expect(pdfSizes([595, 0]), isNull);
    expect(pdfSizes(['595', 842]), isNull);
  });

  test('a page is drawn as wide as it shows, zoomed, within the caps', () {
    const phone = Size(400, 800);
    expect(pdfRenderWidth(a4, phone, 3, 1), 1200);
    // A landscape page on the same phone fills its width too.
    expect(pdfRenderWidth(const Size(842, 595), phone, 3, 1), 1200);
    // A portrait page on a phone turned sideways fills its height.
    expect(pdfRenderWidth(a4, const Size(800, 400), 2, 1), 565);
    final zoomed = pdfRenderWidth(a4, phone, 3, 6);
    expect(zoomed * zoomed * a4.height / a4.width, lessThan(maxRenderPixels));
    expect(pdfRenderWidth(const Size(100, 10000), phone, 3, 6), 40);
  });
}
