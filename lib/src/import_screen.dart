import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'import_run.dart';
import 'l10n.dart';
import 'page_bar.dart';

/// The page of a running Import (obf-1061, C4): what it reads, then how
/// many of its Notes are in ("120 of 340 Notes"), the line under its bar
/// moving meanwhile; the user may leave it, and the Import goes on. At
/// the end it says what went where, and [onOpen] opens the new Area.
class ImportScreen extends StatelessWidget {
  final ImportJob job;
  final void Function(Area area) onOpen;

  const ImportScreen({super.key, required this.job, required this.onOpen});

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final theme = Theme.of(context);
    return Scaffold(
      appBar: pageBar(
        context,
        title: switch (job.kind) {
          ImportKind.folder => t.importFolder,
          ImportKind.keep => t.importKeep,
        },
      ),
      body: ListenableBuilder(
        listenable: job,
        builder: (context, _) {
          final end = job.end;
          final of = job.of;
          return ListView(
            padding: const EdgeInsets.all(24),
            children: [
              if (end != null) ...[
                Text(
                  end.text,
                  key: const ValueKey('importEnd'),
                  style: theme.textTheme.bodyLarge,
                ),
                if (end.area case final area?) ...[
                  const SizedBox(height: 24),
                  FilledButton(
                    key: const ValueKey('importPageOpenArea'),
                    onPressed: () {
                      Navigator.of(context).pop();
                      onOpen(area);
                    },
                    child: Text(t.importOpenArea),
                  ),
                ],
              ] else ...[
                Text(
                  of == null
                      ? switch (job.kind) {
                          ImportKind.folder => t.importReading,
                          ImportKind.keep => t.importKeepReading,
                        }
                      : t.importWritten(job.written, of),
                  key: const ValueKey('importCount'),
                  style: theme.textTheme.headlineSmall,
                ),
                const SizedBox(height: 16),
                Text(t.importGoesOn, style: theme.textTheme.bodyLarge),
              ],
            ],
          );
        },
      ),
    );
  }
}
