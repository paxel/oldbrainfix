import 'package:flutter/material.dart';

import '../l10n/app_localizations.dart';
import 'l10n.dart';
import 'look.dart';

/// What runs in place of the app when this Device's database was saved
/// by a newer version (ADR 0034): one screen that says so, and nothing
/// else, no Capture among it. The database stays as it was.
class StoreNeedsUpdateApp extends StatelessWidget {
  const StoreNeedsUpdateApp({super.key});

  @override
  Widget build(BuildContext context) => MaterialApp(
    onGenerateTitle: (context) => context.t.appTitle,
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    theme: lookTheme(Brightness.light),
    darkTheme: lookTheme(Brightness.dark),
    home: const StoreNeedsUpdateScreen(),
  );
}

class StoreNeedsUpdateScreen extends StatelessWidget {
  const StoreNeedsUpdateScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return Scaffold(
      body: SafeArea(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              const Icon(Icons.update, size: 64),
              const SizedBox(height: 16),
              Text(
                t.storeNeedsUpdate,
                style: Theme.of(context).textTheme.headlineSmall,
                textAlign: TextAlign.center,
              ),
              const SizedBox(height: 8),
              Text(t.storeNeedsUpdateUnchanged, textAlign: TextAlign.center),
            ],
          ),
        ),
      ),
    );
  }
}
