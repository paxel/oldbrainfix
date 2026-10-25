import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'l10n.dart';
import 'profile_platform.dart';

/// The user's own Profile as the screens show it (ADR 0017): made with
/// the phone's defaults on the first start, asking nothing, changed on
/// the Profile page, and taken from the user's other Devices in a sync.
/// Tells its listeners after every change, wherever it came from.
class OwnProfileModel extends ChangeNotifier {
  final Store store;
  final ProfilePlatform platform;

  OwnProfileModel({required this.store, required this.platform}) {
    store.addProfileListener(_changed);
    _question = store.profileQuestion;
  }

  ProfileChoice? _question;

  /// The own Profile; null until the defaults were applied once.
  Profile? get profile => store.ownProfile;

  /// The own Profile's short code; null until it exists.
  String? get code => store.ownProfileCode;

  /// The other Profile "Which one is you?" asks about, if it does.
  ProfileChoice? get question => _question;

  void _changed() {
    _question = store.profileQuestion;
    notifyListeners();
  }

  @override
  void dispose() {
    store.removeProfileListener(_changed);
    super.dispose();
  }

  /// Makes the Profile from the phone's name the first time, and takes a
  /// changed phone name later while the user has not typed one.
  Future<void> applyDefaults() async {
    final name = await platform.deviceName();
    store.applyProfileDefaults(name: name);
  }

  /// Gives the Profile the [name] the user typed.
  void rename(String name) {
    if (profile == null) return;
    store.renameOwnProfile(name);
  }

  /// Gives the Profile the [picture] the user chose, or the initials for
  /// null.
  void setPicture(Uint8List? picture) {
    if (profile == null) return;
    store.setOwnProfilePicture(picture);
  }
}

/// Asks "Which one is you?" (ADR 0017) when the user's Devices met with a
/// Profile each, both changed by the user: both are shown, and the one
/// picked is kept. "Later" leaves the question on the Profile page.
Future<void> askWhichProfile(BuildContext context, Store store) async {
  final other = store.profileQuestion;
  final mine = store.ownProfile;
  if (other == null || mine == null) return;
  final keepMine = await showDialog<bool>(
    context: context,
    builder: (_) => ProfileQuestionDialog(
      mine: (profile: mine, code: store.ownProfileCode ?? ''),
      other: other,
    ),
  );
  if (keepMine == null) return;
  store.answerProfileQuestion(keepMine: keepMine);
}

/// "Which one is you?": this Device's Profile and the other's, each
/// with its picture, name and short code, and a button that keeps it.
/// Pops with true for this Device's, false for the other's.
class ProfileQuestionDialog extends StatelessWidget {
  final ProfileChoice mine;
  final ProfileChoice other;

  const ProfileQuestionDialog({
    super.key,
    required this.mine,
    required this.other,
  });

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    return AlertDialog(
      title: Text(t.profileWhichTitle),
      scrollable: true,
      content: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(t.profileWhichText),
          const SizedBox(height: 16),
          _Choice(
            key: const ValueKey('whichMine'),
            choice: mine,
            where: t.profileWhichThisDevice,
            onPick: () => Navigator.of(context).pop(true),
          ),
          const SizedBox(height: 12),
          _Choice(
            key: const ValueKey('whichOther'),
            choice: other,
            where: t.profileWhichOtherDevice,
            onPick: () => Navigator.of(context).pop(false),
          ),
        ],
      ),
      actions: [
        TextButton(
          key: const ValueKey('whichLater'),
          onPressed: () => Navigator.of(context).pop(),
          child: Text(t.profileWhichLater),
        ),
      ],
    );
  }
}

class _Choice extends StatelessWidget {
  final ProfileChoice choice;
  final String where;
  final VoidCallback onPick;

  const _Choice({
    super.key,
    required this.choice,
    required this.where,
    required this.onPick,
  });

  @override
  Widget build(BuildContext context) {
    final t = context.t;
    final theme = Theme.of(context);
    return Card(
      margin: EdgeInsets.zero,
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Row(
              children: [
                ProfileAvatar(profile: choice.profile, size: 56),
                const SizedBox(width: 12),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        profileDisplayName(t, choice.profile.name),
                        style: theme.textTheme.titleMedium,
                      ),
                      Text(
                        choice.code,
                        style: theme.textTheme.bodyMedium?.copyWith(
                          fontFamily: 'monospace',
                          letterSpacing: 1.5,
                        ),
                      ),
                      Text(where, style: theme.textTheme.bodySmall),
                    ],
                  ),
                ),
              ],
            ),
            const SizedBox(height: 8),
            FilledButton.tonal(
              onPressed: onPick,
              child: Text(t.profileWhichPick),
            ),
          ],
        ),
      ),
    );
  }
}

/// How [name] shows: itself, or "No name" when it is empty.
String profileDisplayName(AppLocalizations t, String name) =>
    name.isEmpty ? t.profileNoName : name;

/// A Profile's picture in a circle of [size]; without one, its initials
/// on the circle coloured by its id; without a Profile or initials, a
/// person on a neutral circle.
class ProfileAvatar extends StatelessWidget {
  final Profile? profile;
  final double size;

  const ProfileAvatar({super.key, required this.profile, this.size = 32});

  @override
  Widget build(BuildContext context) {
    final profile = this.profile;
    final picture = profile?.picture;
    if (picture != null) {
      return ClipOval(
        child: Image.memory(
          picture,
          width: size,
          height: size,
          fit: BoxFit.cover,
          gaplessPlayback: true,
          // The placeholder stands in for a picture that will not show.
          errorBuilder: (context, _, _) => _placeholder(context, profile),
        ),
      );
    }
    return _placeholder(context, profile);
  }

  Widget _placeholder(BuildContext context, Profile? profile) {
    final initials = profile == null ? '' : profileInitials(profile.name);
    final scheme = Theme.of(context).colorScheme;
    return Container(
      key: const ValueKey('profilePlaceholder'),
      width: size,
      height: size,
      alignment: Alignment.center,
      decoration: BoxDecoration(
        shape: BoxShape.circle,
        color: profile == null
            ? scheme.surfaceContainerHighest
            : Color(profileColor(profile.id)),
      ),
      child: initials.isEmpty
          ? Icon(
              Icons.person,
              size: size * 0.6,
              color: profile == null ? scheme.onSurfaceVariant : Colors.white,
            )
          : Text(
              initials,
              maxLines: 1,
              overflow: TextOverflow.clip,
              style: TextStyle(
                color: Colors.white,
                fontSize: size * 0.4,
                fontWeight: FontWeight.w600,
                height: 1,
              ),
            ),
    );
  }
}

/// [ProfileAvatar] of the own Profile, kept current as it changes.
class OwnProfileAvatar extends StatelessWidget {
  final OwnProfileModel model;
  final double size;

  const OwnProfileAvatar({super.key, required this.model, this.size = 32});

  @override
  Widget build(BuildContext context) => ListenableBuilder(
    listenable: model,
    builder: (context, _) => ProfileAvatar(profile: model.profile, size: size),
  );
}
