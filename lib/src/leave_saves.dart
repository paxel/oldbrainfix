import 'dart:async';

import 'package:flutter/material.dart';

import 'l10n.dart';

/// Leaving an edit page by Back, the swipe back or its bar's arrow
/// (#275, UI laws, "Navigation"): a page left as it opened, not
/// [changed], just closes; a [valid] edit is saved by [save], which
/// closes the page as its Save does; an edit that can't be saved asks
/// once, "Discard changes?". Discard closes without saving; Keep editing
/// stays and runs [onKeep], which shows what to fix. The page holds its
/// leaving, so on the iPhone it takes no edge swipe (UI laws).
class LeaveSaves extends StatelessWidget {
  final bool Function() changed;
  final bool Function() valid;
  final VoidCallback save;
  final VoidCallback? onKeep;
  final Widget child;

  const LeaveSaves({
    super.key,
    required this.changed,
    required this.valid,
    required this.save,
    this.onKeep,
    required this.child,
  });

  Future<void> _leave(BuildContext context) async {
    if (!changed()) {
      Navigator.of(context).pop();
      return;
    }
    if (valid()) {
      save();
      return;
    }
    final discard = await askDiscard(context);
    if (!context.mounted) return;
    if (discard) {
      Navigator.of(context).pop();
    } else {
      onKeep?.call();
    }
  }

  @override
  Widget build(BuildContext context) => PopScope(
    canPop: false,
    onPopInvokedWithResult: (didPop, _) {
      if (!didPop) unawaited(_leave(context));
    },
    child: child,
  );
}

/// "Discard changes?": true for Discard, false for Keep editing or when
/// the dialog is left.
Future<bool> askDiscard(BuildContext context) async {
  final t = context.t;
  final discard = await showDialog<bool>(
    context: context,
    builder: (context) => AlertDialog(
      title: Text(t.leaveDiscardTitle),
      content: Text(t.leaveDiscardBody),
      actions: [
        TextButton(
          key: const ValueKey('leave-keep-editing'),
          onPressed: () => Navigator.of(context).pop(false),
          child: Text(t.leaveKeepEditing),
        ),
        TextButton(
          key: const ValueKey('leave-discard'),
          style: TextButton.styleFrom(
            foregroundColor: Theme.of(context).colorScheme.error,
          ),
          onPressed: () => Navigator.of(context).pop(true),
          child: Text(t.leaveDiscard),
        ),
      ],
    ),
  );
  return discard ?? false;
}
