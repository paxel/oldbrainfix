import 'dart:async';

import 'package:flutter/material.dart';

import 'crash_guard.dart';
import 'l10n.dart';

/// Mails a problem report with [body] to the developer; the user still
/// sends or discards the mail.
Future<void> mailProblemReport(String body) => mailCrashReport(
  '${crashReportHeader()}\n\n$body',
  subject: 'Old Brain Fix problem report',
);

/// What tapping a failure's announcement opens (UI laws,
/// "Announcements"): the [cause] and the [fix], one sentence each, the
/// full [details] text, Close and Report. Report hands the details to
/// [report].
Future<void> showFailure(
  BuildContext context, {
  required String title,
  required String cause,
  required String fix,
  required String details,
  Future<void> Function(String body) report = mailProblemReport,
}) => showDialog<void>(
  context: context,
  builder: (context) {
    final t = context.t;
    final theme = Theme.of(context);
    return AlertDialog(
      title: Text(title),
      content: SingleChildScrollView(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text('$cause $fix'),
            const SizedBox(height: 16),
            Container(
              width: double.infinity,
              padding: const EdgeInsets.all(8),
              decoration: BoxDecoration(
                color: theme.colorScheme.surfaceContainerHighest,
                borderRadius: BorderRadius.circular(8),
              ),
              child: SelectableText(
                details,
                key: const ValueKey('failureDetails'),
                style: const TextStyle(fontFamily: 'monospace', fontSize: 12),
              ),
            ),
          ],
        ),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(t.close),
        ),
        TextButton(
          onPressed: () {
            Navigator.of(context).pop();
            unawaited(report(details));
          },
          child: Text(t.report),
        ),
      ],
    );
  },
);
