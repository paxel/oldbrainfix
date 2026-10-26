import 'package:flutter/material.dart';

import 'l10n.dart';
import 'speech_platform.dart';
import 'system_bars.dart';

/// First start, after the Starter Set: offers the speech model for
/// Transcripts. [onDownload] starts the download and setup goes on at
/// once; [onSkip] goes on without it. The speech model page in Settings
/// offers it again, and a file of the user's own.
class SpeechOfferScreen extends StatefulWidget {
  final SpeechModel model;
  final VoidCallback onDownload;
  final VoidCallback onSkip;

  const SpeechOfferScreen({
    super.key,
    required this.model,
    required this.onDownload,
    required this.onSkip,
  });

  @override
  State<SpeechOfferScreen> createState() => _SpeechOfferScreenState();
}

class _SpeechOfferScreenState extends State<SpeechOfferScreen> {
  /// Whether the Device is on mobile data, asked once.
  var _mobileData = false;

  @override
  void initState() {
    super.initState();
    _askMobileData();
  }

  Future<void> _askMobileData() async {
    final mobileData = await widget.model.platform.onMobileData();
    if (!mounted || !mobileData) return;
    setState(() => _mobileData = true);
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final text = Theme.of(context).textTheme;
    return Scaffold(
      appBar: AppBar(title: Text(t.speechModelTitle)),
      body: ListView(
        padding: clearOfSystemBars(context, const EdgeInsets.all(24)),
        children: [
          Text(t.speechOfferQuestion, style: text.headlineSmall),
          const SizedBox(height: 8),
          Text(t.speechOfferDetail(WhisperModel.megabytes)),
          const SizedBox(height: 24),
          if (_mobileData) ...[
            Row(
              key: const ValueKey('speech-offer-mobile-data'),
              children: [
                const Icon(Icons.signal_cellular_alt),
                const SizedBox(width: 16),
                Expanded(
                  child: Text(t.speechOfferMobileData(WhisperModel.megabytes)),
                ),
              ],
            ),
            const SizedBox(height: 8),
          ],
          // Still checking for a model in place: the download waits.
          ListenableBuilder(
            listenable: widget.model,
            builder: (context, _) => FilledButton.icon(
              onPressed: widget.model.status == ModelStatus.missing
                  ? widget.onDownload
                  : null,
              icon: const Icon(Icons.download),
              label: Text(t.speechModelDownload(WhisperModel.megabytes)),
            ),
          ),
          const SizedBox(height: 8),
          OutlinedButton(onPressed: widget.onSkip, child: Text(t.notNow)),
        ],
      ),
    );
  }
}
