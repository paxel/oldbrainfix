import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/store_needs_update_screen.dart';

/// A database a newer version of the app saved is not opened: one screen
/// says so, with no Capture and nothing else (ADR 0034).
void main() {
  final en = lookupAppLocalizations(const Locale('en'));
  final de = lookupAppLocalizations(const Locale('de'));

  testWidgets('the screen says the data is of a newer version and offers '
      'nothing else', (tester) async {
    await tester.pumpWidget(const StoreNeedsUpdateApp());
    await tester.pumpAndSettle();
    expect(
      find.text(
        "This Device's data was saved by a newer Old Brain Fix; install "
        'that version again.',
      ),
      findsOneWidget,
    );
    expect(find.text(en.storeNeedsUpdateUnchanged), findsOneWidget);
    expect(find.byType(FloatingActionButton), findsNothing);
    expect(find.byType(ButtonStyleButton), findsNothing);
    expect(find.byType(IconButton), findsNothing);
    expect(find.byType(TextField), findsNothing);
  });

  test('it is in German too', () {
    expect(de.storeNeedsUpdate, isNot(en.storeNeedsUpdate));
    expect(de.storeNeedsUpdateUnchanged, isNot(en.storeNeedsUpdateUnchanged));
  });
}
