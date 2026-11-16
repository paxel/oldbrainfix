import 'dart:io';
import 'dart:isolate';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/src/app.dart';
import 'package:oldbrainfix/src/attachment_reader.dart';
import 'package:oldbrainfix/src/attachment_views.dart';
import 'package:oldbrainfix/src/capture_screen.dart';
import 'package:oldbrainfix/src/file_platform.dart';
import 'package:oldbrainfix/src/photo_platform.dart';
import 'package:oldbrainfix/src/print_platform.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'fake_alarms.dart';
import 'fake_files.dart';
import 'fake_ocr.dart';
import 'fake_photos.dart';
import 'fake_printing.dart';
import 'open_settings.dart';

/// Print to Old Brain Fix (#293): Settings says whether Android has the
/// print service on, asks again when the app comes back and opens
/// Android's print settings; only Android has the row. A printed PDF
/// comes in as a share does: Capture opens with it attached, named after
/// the print job, its text read here up to ten pages, kept Unsorted with
/// the Nudge in 1 h when the user just goes back.
void main() {
  final keepAside = keepDeviceFile;
  final now = DateTime.utc(2026, 11, 13, 9);
  late Store store;
  late Notes notes;
  late Directory dir;
  late FakeFiles files;
  late FakePhotos photos;

  setUp(() {
    fileTextWork = <R>(work) async => work();
    keepDeviceFile = (files, path) async => files.putFrom(path);
    store = Store.inMemory(clock: () => now)
      ..starterSetOffered = true
      ..alarmSetupDone = true;
    notes = Notes(store);
    dir = Directory.systemTemp.createTempSync('print_service_');
    files = FakeFiles(dir);
    photos = FakePhotos();
  });
  tearDown(() {
    fileTextWork = Isolate.run;
    keepDeviceFile = keepAside;
    store.close();
    dir.deleteSync(recursive: true);
  });

  Finder key(String value) => find.byKey(ValueKey(value));

  /// Lets the reader's and the file system's work arrive, then draws.
  Future<void> settle(WidgetTester tester) async {
    for (var i = 0; i < 10; i++) {
      await tester.runAsync(
        () => Future<void>.delayed(const Duration(milliseconds: 5)),
      );
      await tester.pumpAndSettle();
    }
  }

  Future<void> start(
    WidgetTester tester, {
    PrintPlatform printing = const NoPrinting(),
  }) async {
    tester.view.physicalSize = const Size(2400, 3600);
    tester.view.devicePixelRatio = 3;
    addTearDown(tester.view.reset);
    await tester.pumpWidget(
      OldBrainFixApp(
        store: store,
        alarms: FakeAlarms(),
        files: files,
        photos: photos,
        ocr: FakeOcr(),
        printing: printing,
      ),
    );
    await settle(tester);
  }

  /// Settings, at the print service's row.
  Future<void> openRow(WidgetTester tester) async {
    await openDeviceSettings(tester);
    await tester.scrollUntilVisible(
      key('printService'),
      200,
      scrollable: find.byType(Scrollable).last,
    );
    await tester.pumpAndSettle();
  }

  String stateLine(WidgetTester tester) =>
      tester.widget<Text>(key('printService-state')).data!;

  group('the Settings row', () {
    testWidgets('says the service is on, or off, or what to do where '
        'Android does not say', (tester) async {
      final printing = FakePrinting(PrintServiceState.off);
      await start(tester, printing: printing);
      await openRow(tester);
      expect(find.text('Print to Old Brain Fix'), findsOneWidget);
      expect(stateLine(tester), startsWith('Off.'));

      printing.state = PrintServiceState.unknown;
      await tester.tap(find.byTooltip('Back'));
      await tester.pumpAndSettle();
      await tester.tap(key('deviceSettings'));
      await tester.pumpAndSettle();
      await tester.scrollUntilVisible(
        key('printService'),
        200,
        scrollable: find.byType(Scrollable).last,
      );
      expect(
        stateLine(tester),
        startsWith("Turn on Old Brain Fix in Android's print settings."),
      );
    });

    testWidgets('its button opens Android\'s print settings, and coming '
        'back reads the state again', (tester) async {
      final printing = FakePrinting(PrintServiceState.off);
      await start(tester, printing: printing);
      await openRow(tester);
      await tester.tap(key('printService-open'));
      await tester.pumpAndSettle();
      expect(printing.opened, 1);

      // Turned on there, the app comes back.
      printing.state = PrintServiceState.on;
      final asked = printing.asked;
      tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
      tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
      tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
      tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
      tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
      tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
      await tester.pumpAndSettle();
      expect(printing.asked, greaterThan(asked));
      expect(stateLine(tester), startsWith('On.'));
      expect(stateLine(tester), contains('pick Old Brain Fix as the printer'));
    });

    testWidgets('the Settings search finds it by its lines, such as '
        '"printer"', (tester) async {
      await start(tester, printing: FakePrinting());
      await openDeviceSettings(tester);
      await tester.enterText(
        find.descendant(
          of: key('settingsSearch'),
          matching: find.byType(TextField),
        ),
        'printer pdf',
      );
      await tester.pumpAndSettle();
      expect(key('printService'), findsOneWidget);
      expect(key('fatFingers'), findsNothing);
    });

    testWidgets('shows in Simple as in Advanced', (tester) async {
      store.advanced = false;
      await start(tester, printing: FakePrinting());
      await openRow(tester);
      expect(stateLine(tester), startsWith('On.'));
    });

    testWidgets('is not there without a print service, as on the iPhone', (
      tester,
    ) async {
      await start(tester);
      await openDeviceSettings(tester);
      expect(key('printService'), findsNothing);
      expect(
        find.text('Print to Old Brain Fix', skipOffstage: false),
        findsNothing,
      );
    });
  });

  test('the channel says on, off, or unknown where it cannot', () async {
    TestWidgetsFlutterBinding.ensureInitialized();
    const channel = MethodChannel('io.github.paxel.oldbrainfix/print');
    final messenger =
        TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
    addTearDown(() => messenger.setMockMethodCallHandler(channel, null));
    final calls = <String>[];
    Object? answer = 'on';
    messenger.setMockMethodCallHandler(channel, (call) async {
      calls.add(call.method);
      if (call.method == 'openSettings') return true;
      if (answer is Exception) throw answer;
      return answer;
    });
    const printing = AndroidPrinting();
    expect(await printing.serviceState(), PrintServiceState.on);
    answer = 'off';
    expect(await printing.serviceState(), PrintServiceState.off);
    answer = 'unknown';
    expect(await printing.serviceState(), PrintServiceState.unknown);
    answer = PlatformException(code: 'x');
    expect(await printing.serviceState(), PrintServiceState.unknown);
    expect(await printing.openSettings(), isTrue);
    expect(calls.last, 'openSettings');
  });

  test('a print comes over the share channel as a PDF named after its job, '
      'from no app', () async {
    TestWidgetsFlutterBinding.ensureInitialized();
    const channel = MethodChannel('io.github.paxel.oldbrainfix/photos');
    final messenger =
        TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
    addTearDown(() => messenger.setMockMethodCallHandler(channel, null));
    // As Shares.take hands over a print kept by PrintIn.
    messenger.setMockMethodCallHandler(channel, (call) async {
      if (call.method != 'shared') return null;
      return <String, Object?>{
        'subject': null,
        'text': null,
        'photos': <Object?>[],
        'recordings': <Object?>[],
        'files': [
          {
            'path': '/data/files/printed/1.pdf',
            'name': 'Kontoauszug März.pdf',
            'type': 'application/pdf',
          },
        ],
        'sourcePackage': null,
        'sourceLabel': null,
      };
    });
    final item = (await AndroidPhotos().takeShared())!;
    expect(item.files.single.name, 'Kontoauszug März.pdf');
    expect(item.files.single.type, 'application/pdf');
    expect(item.files.single.path, '/data/files/printed/1.pdf');
    expect(item.sourceApp, isNull);
    expect(item.holdsNoNote, isTrue);
  });

  testWidgets('a print waiting at the start opens Capture with its PDF named '
      'after the job, read here, kept Unsorted with the Nudge in 1 h', (
    tester,
  ) async {
    final pdf = Uint8List(321);
    files.pdfs[pdf.length] = [
      const PdfPage(text: 'Kontoauszug März'),
      const PdfPage(text: 'Kontostand 1.234,56 €'),
    ];
    final printed = File('${dir.path}/printed-1')..writeAsBytesSync(pdf);
    // Printed while the app was gone: it waits for the start.
    photos.waiting.add(
      SharedItem(
        files: [
          DeviceFile(
            path: printed.path,
            name: 'Kontoauszug März.pdf',
            type: 'application/pdf',
          ),
        ],
      ),
    );
    await start(tester);
    expect(find.byType(CaptureScreen), findsOneWidget);
    expect(find.text('Kontoauszug März.pdf'), findsOneWidget);
    final note = notes.all().single;
    final attachment = note.attachments.single;
    expect(attachment.name, 'Kontoauszug März.pdf');
    expect(attachment.pages, 2);
    expect(attachment.computer, isFalse, reason: 'read on the phone');
    expect(note.context.sourceApp, isNull);
    expect(printed.existsSync(), isFalse, reason: 'the copy let go');

    final dynamic navigator = tester.state(find.byType(Navigator).first);
    await (navigator as NavigatorState).maybePop();
    await settle(tester);
    final saved = notes.byId(note.id)!;
    expect(saved.isUnsorted, isTrue);
    expect(saved.nudge, now.add(Notes.defaultNudge));
    expect(
      saved.attachmentTexts[attachment.hash],
      'Kontoauszug März\n\nKontostand 1.234,56 €',
    );
  });
}
