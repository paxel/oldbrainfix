import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:intl/date_symbol_data_local.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/note_card.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// A long press on a Note card, which opens its editor, gives back a
/// touch on both phones (#243): Material's ink vibrates on Android and
/// gives the Taptic Engine's heavy impact with a click on iOS, by the
/// platform the app runs on; the app adds nothing of its own.
void main() {
  setUpAll(initializeDateFormatting);

  /// The haptic calls a long press on a card makes on [platform], and
  /// how often its editor opened.
  Future<(List<Object?>, int)> longPress(
    WidgetTester tester,
    TargetPlatform platform,
  ) async {
    final haptics = <Object?>[];
    tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
      SystemChannels.platform,
      (call) async {
        if (call.method == 'HapticFeedback.vibrate') {
          haptics.add(call.arguments);
        }
        return null;
      },
    );
    addTearDown(
      () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SystemChannels.platform,
        null,
      ),
    );
    final store = Store.inMemory();
    addTearDown(store.close);
    final note = Notes(store).capture('Call the plumber');
    var edited = 0;
    debugDefaultTargetPlatformOverride = platform;
    try {
      await tester.pumpWidget(
        MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: Scaffold(
            body: ListView(
              children: [
                NoteCard(
                  note: note,
                  when: DateFormat.yMd('en').add_Hm(),
                  onEdit: () => edited++,
                ),
              ],
            ),
          ),
        ),
      );
      await tester.longPress(find.text('Call the plumber'));
      await tester.pumpAndSettle();
    } finally {
      debugDefaultTargetPlatformOverride = null;
    }
    return (haptics, edited);
  }

  testWidgets('the iPhone gives the heavy impact', (tester) async {
    final (haptics, edited) = await longPress(tester, TargetPlatform.iOS);
    expect(edited, 1);
    expect(haptics, ['HapticFeedbackType.heavyImpact']);
  });

  testWidgets('Android vibrates', (tester) async {
    final (haptics, edited) = await longPress(tester, TargetPlatform.android);
    expect(edited, 1);
    expect(haptics, [null]);
  });
}
