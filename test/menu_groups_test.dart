import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:oldbrainfix/l10n/app_localizations.dart';
import 'package:oldbrainfix/src/menu_groups.dart';

import 'card_menu.dart';

/// A group of a menu opens as a second menu at the button, and shows at
/// most seven entries, the rest under "More…" (obf-1014).
void main() {
  Future<List<int>> open(WidgetTester tester, int count) async {
    final picked = <int>[];
    await tester.pumpWidget(
      MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(
          body: Center(
            child: Builder(
              builder: (button) => TextButton(
                key: const ValueKey('button'),
                onPressed: () => openMenuGroup(button, [
                  for (var i = 0; i < count; i++)
                    PopupMenuItem(
                      key: ValueKey('entry-$i'),
                      value: () => picked.add(i),
                      child: Text('Entry $i'),
                    ),
                ]),
                child: const Text('Open'),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.byKey(const ValueKey('button')));
    await tester.pumpAndSettle();
    return picked;
  }

  testWidgets('seven entries show as they are', (tester) async {
    final picked = await open(tester, 7);
    expect(menuEntries(tester), 7);
    expect(find.byKey(const ValueKey('menu-more')), findsNothing);
    await tester.tap(find.byKey(const ValueKey('entry-6')));
    await tester.pumpAndSettle();
    expect(picked, [6]);
  });

  testWidgets('past seven, six show and More… opens the rest', (tester) async {
    final picked = await open(tester, 10);
    expect(menuEntries(tester), 7);
    expect(find.byKey(const ValueKey('entry-5')), findsOneWidget);
    expect(find.byKey(const ValueKey('entry-6')), findsNothing);
    expect(find.text('More…'), findsOneWidget);

    await tester.tap(find.byKey(const ValueKey('menu-more')));
    await tester.pumpAndSettle();
    expect(menuEntries(tester), 4);
    expect(find.byKey(const ValueKey('entry-0')), findsNothing);
    await tester.tap(find.byKey(const ValueKey('entry-9')));
    await tester.pumpAndSettle();
    expect(picked, [9]);
    expect(find.byType(PopupMenuItem<VoidCallback>), findsNothing);
  });
}
