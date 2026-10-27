import 'dart:async';

import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import 'browser.dart';
import 'l10n.dart';
import 'profile.dart';
import 'sent_item_screen.dart';
import 'system_bars.dart';

/// The strangers' Sent Items waiting here (#91), the oldest first: who
/// each is from and when it came, nothing of what it holds. A tap opens
/// its preview, hidden until "Show". Those left undecided, or by a
/// restart, wait here until Accept or Decline.
class HeldSentItemsScreen extends StatefulWidget {
  final Store store;

  const HeldSentItemsScreen({super.key, required this.store});

  @override
  State<HeldSentItemsScreen> createState() => _HeldSentItemsScreenState();
}

class _HeldSentItemsScreenState extends State<HeldSentItemsScreen> {
  /// Read once per change: each read checks every held file again.
  List<HeldSentItem> _held = const [];
  var _reloadQueued = false;

  @override
  void initState() {
    super.initState();
    _held = heldSentItems(widget.store);
    // An item accepted, declined or dropped by a block goes.
    widget.store.addListener(_changed);
  }

  @override
  void dispose() {
    widget.store.removeListener(_changed);
    super.dispose();
  }

  /// Reloads once after the store changed, never in the middle of
  /// whatever changed it.
  void _changed() {
    if (_reloadQueued) return;
    _reloadQueued = true;
    scheduleMicrotask(() {
      _reloadQueued = false;
      if (!mounted) return;
      setState(() => _held = heldSentItems(widget.store));
    });
  }

  Future<void> _open(HeldSentItem held) async {
    await Navigator.of(context).push(
      MaterialPageRoute<void>(
        builder: (_) => HeldSentItemScreen(store: widget.store, held: held),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final locale = Localizations.localeOf(context).toString();
    final when = DateFormat.yMd(locale).add_Hm();
    return Scaffold(
      appBar: AppBar(title: Text(t.sentItemsWaitingTitle)),
      body: _held.isEmpty
          ? BrowserEmpty(text: t.sentItemsNoneWaiting)
          : ListView(
              padding: clearOfSystemBars(context, EdgeInsets.zero),
              children: [
                for (final held in _held)
                  ListTile(
                    key: ValueKey('held-${held.id}'),
                    leading: const Icon(Icons.mark_email_unread_outlined),
                    title: Text(
                      t.sentItemFrom(profileDisplayName(t, held.sender.name)),
                    ),
                    subtitle: Text(when.format(held.held.toLocal())),
                    onTap: () => _open(held),
                  ),
              ],
            ),
    );
  }
}
