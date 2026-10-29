import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';
import 'package:qr/qr.dart';

import 'failure_dialog.dart';
import 'host_screen.dart';
import 'l10n.dart';
import 'pairing_platform.dart';
import 'person_screen.dart';
import 'photo_platform.dart';
import 'profile_platform.dart';
import 'system_bars.dart';

/// Profile cards on the phone (ADR 0017, #107): the own card sent as a
/// link through the app the user picks or shown as a QR, and another
/// person's card opened from a scan, a picture, a pasted link or a link
/// tapped in another app.

/// Shares the own Profile card through the app the user picks, the link
/// last so nothing after it joins the card; [back] when it answers a card
/// the user opened. Announces a failure when there is no card yet or no
/// app takes it, why behind the tap; returns whether it was handed over.
Future<bool> sendProfileCard(
  BuildContext context,
  Store store,
  ProfilePlatform platform, {
  bool back = false,
}) async {
  final t = context.t;
  final link = store.profileCard(back: back);
  if (link == null) {
    _announceNoCard(context);
    return false;
  }
  final shared = await platform.shareText(t.profileCardText(link));
  if (!shared && context.mounted) {
    announceFailure(
      context,
      t.profileCardNotSent,
      title: t.profileCardNotSentTitle,
      cause: t.profileCardNoApp,
      fix: t.profileCardNoAppFix,
      details: 'profile card: no app took the link',
    );
  }
  return shared;
}

/// Announces that no Profile card could be made yet, why behind the tap.
void _announceNoCard(BuildContext context) {
  final t = context.t;
  announceFailure(
    context,
    t.profileCardNotMade,
    title: t.profileCardNotMadeTitle,
    cause: t.profileCardNotYetCause,
    fix: t.profileCardNotYet,
    details: 'profile card: this Device is not bound to the Profile yet',
  );
}

/// Shows the own Profile card as a QR, or announces that there is none
/// yet.
Future<void> showProfileQr(BuildContext context, Store store) async {
  final link = store.profileCard();
  if (link == null) {
    _announceNoCard(context);
    return;
  }
  await Navigator.of(
    context,
  ).push(MaterialPageRoute<void>(builder: (_) => ProfileQrScreen(link: link)));
}

/// The modules of the QR of the card [link], dark as true; error
/// correction L, so a card of several Devices still fits and its modules
/// stay large enough to read from a photo. Null when it fits no QR.
List<List<bool>>? profileCardQr(String link) {
  try {
    final image = QrImage(
      QrCode(
        payload: QrPayload.fromString(link),
        errorCorrectLevel: QrErrorCorrectLevel.low,
      ),
    );
    return [
      for (var r = 0; r < image.moduleCount; r++)
        [for (var c = 0; c < image.moduleCount; c++) image.isDark(r, c)],
    ];
  } on InputTooLongException {
    return null;
  }
}

/// Opens the card in [text] and shows what it did: the person's Profile
/// when they are in People now, with "Send yours back" when they were
/// new here, else that nobody was added, why behind the tap. [inPerson]
/// only for a QR the camera just scanned.
Future<void> openCardText(
  BuildContext context,
  Store store,
  String text, {
  required bool inPerson,
}) => showCardOpened(
  context,
  store.openProfileCard(text, inPerson: inPerson),
  replace: false,
);

/// Shows what opening a card did ([openCardText]); with [replace] the
/// person's Profile takes the place of the page it was opened on.
Future<void> showCardOpened(
  BuildContext context,
  CardOpened opened, {
  required bool replace,
}) async {
  final t = context.t;
  final why = switch (opened.taken) {
    CardTaken.added || CardTaken.updated || CardTaken.unchanged => null,
    CardTaken.own => (t.cardOwn, t.cardOwnFix),
    CardTaken.blocked => (t.cardBlocked, t.blockedFix),
    CardTaken.refused => (t.cardRefused, t.cardRefusedFix),
    CardTaken.unreadable => (t.cardUnreadableCause, t.cardAskAgainFix),
  };
  final id = opened.id;
  if (why != null || id == null) {
    final (cause, fix) = why ?? (t.cardUnreadableCause, t.cardAskAgainFix);
    announceFailure(
      context,
      t.cardNotAdded,
      title: t.cardNotAddedTitle,
      cause: cause,
      fix: fix,
      details: 'profile card: ${opened.taken.name}',
    );
    return;
  }
  final route = personRoute(context, id, offerBack: opened.offerBack);
  if (route == null) return;
  final navigator = Navigator.of(context);
  await (replace ? navigator.pushReplacement(route) : navigator.push(route));
}

/// The own Profile card as a large QR, dark on white, for someone next
/// to the user to scan; scanned in person, it marks the user verified
/// for them at once.
class ProfileQrScreen extends StatefulWidget {
  /// The card's link.
  final String link;

  const ProfileQrScreen({super.key, required this.link});

  @override
  State<ProfileQrScreen> createState() => _ProfileQrScreenState();
}

class _ProfileQrScreenState extends State<ProfileQrScreen> {
  late List<List<bool>>? _modules;

  @override
  void initState() {
    super.initState();
    _modules = profileCardQr(widget.link);
  }

  @override
  void didUpdateWidget(ProfileQrScreen old) {
    super.didUpdateWidget(old);
    if (old.link != widget.link) _modules = profileCardQr(widget.link);
  }

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final modules = _modules;
    return Scaffold(
      appBar: AppBar(title: Text(t.profileQrTitle)),
      body: SingleChildScrollView(
        padding: clearOfSystemBars(context, const EdgeInsets.all(16)),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            if (modules == null)
              Text(t.profileQrTooLong, key: const ValueKey('profile-qr-long'))
            else ...[
              Center(
                child: ConstrainedBox(
                  constraints: const BoxConstraints(maxWidth: 560),
                  child: AspectRatio(
                    aspectRatio: 1,
                    child: CustomPaint(
                      key: const ValueKey('profile-qr'),
                      painter: QrPainter(modules),
                    ),
                  ),
                ),
              ),
              const SizedBox(height: 16),
              Text(t.profileQrHint),
            ],
          ],
        ),
      ),
    );
  }
}

/// Adds a person from their Profile card: their QR scanned with the
/// camera, which marks them verified at once, a QR in a picture, or
/// their link pasted. Once they are in People, their Profile takes this
/// page's place.
class AddPersonScreen extends StatefulWidget {
  final Store store;

  const AddPersonScreen({super.key, required this.store});

  @override
  State<AddPersonScreen> createState() => _AddPersonScreenState();
}

class _AddPersonScreenState extends State<AddPersonScreen> {
  final _link = TextEditingController();
  String? _error;
  var _busy = false;

  @override
  void dispose() {
    _link.dispose();
    super.dispose();
  }

  /// Takes a photo of their QR with the camera: in person.
  Future<void> _scan() async {
    final photo = await PhotoScope.of(context).takePhoto();
    if (!mounted || photo == null) return;
    await _read(photo, inPerson: true);
  }

  /// Reads their QR in a picture the user picks, such as a screenshot.
  Future<void> _picture() async {
    final pictures = await PhotoScope.of(context).pickPhotos();
    if (!mounted || pictures.isEmpty) return;
    await _read(pictures.first, inPerson: false);
  }

  Future<void> _read(Uint8List picture, {required bool inPerson}) async {
    setState(() => _busy = true);
    final text = await PairingScope.of(context).readQr(picture);
    if (!mounted) return;
    setState(() => _busy = false);
    if (text == null) {
      final t = context.t;
      announceFailure(
        context,
        t.cardNotRead,
        title: t.cardNotReadTitle,
        cause: t.cardNoQr,
        fix: t.cardNoQrFix,
        details: 'profile card: no QR code in the picture',
      );
      return;
    }
    await _opened(widget.store.openProfileCard(text, inPerson: inPerson));
  }

  /// Opens the pasted link; one that is no card is the field's error.
  Future<void> _open() async {
    final opened = widget.store.openProfileCard(_link.text);
    if (opened.taken == CardTaken.unreadable) {
      setState(() => _error = context.t.cardUnreadable);
      return;
    }
    setState(() => _error = null);
    await _opened(opened);
  }

  Future<void> _opened(CardOpened opened) =>
      showCardOpened(context, opened, replace: true);

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final photos = PhotoScope.of(context);
    return Scaffold(
      appBar: AppBar(title: Text(t.personAdd)),
      body: SingleChildScrollView(
        padding: clearOfSystemBars(context, const EdgeInsets.all(24)),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text(t.personAddIntro),
            const SizedBox(height: 16),
            if (photos.available) ...[
              FilledButton.icon(
                key: const ValueKey('add-person-scan'),
                onPressed: _busy ? null : _scan,
                icon: const Icon(Icons.qr_code_scanner),
                label: Text(t.personAddScan),
              ),
              const SizedBox(height: 8),
            ],
            if (photos.pickable) ...[
              OutlinedButton.icon(
                key: const ValueKey('add-person-picture'),
                onPressed: _busy ? null : _picture,
                icon: const Icon(Icons.image_outlined),
                label: Text(t.personAddPicture),
              ),
              const SizedBox(height: 16),
            ],
            TextField(
              key: const ValueKey('add-person-link'),
              controller: _link,
              keyboardType: TextInputType.url,
              autocorrect: false,
              enableSuggestions: false,
              maxLines: 3,
              minLines: 1,
              decoration: InputDecoration(
                labelText: t.personAddLink,
                errorText: _error,
                errorMaxLines: 3,
              ),
              onChanged: (_) {
                if (_error != null) setState(() => _error = null);
              },
            ),
            const SizedBox(height: 8),
            FilledButton(
              key: const ValueKey('add-person-open'),
              onPressed: _busy ? null : _open,
              child: Text(t.personAddOpen),
            ),
          ],
        ),
      ),
    );
  }
}
