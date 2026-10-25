import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

import '../l10n/app_localizations.dart';
import 'profile_platform.dart';

/// The user's own Profile as the screens show it (ADR 0017): made with
/// the phone's defaults on the first start, asking nothing, and changed
/// on the Profile page. Tells its listeners after every change, as local
/// settings do not reach the store's listeners.
class OwnProfileModel extends ChangeNotifier {
  final Store store;
  final ProfilePlatform platform;

  OwnProfileModel({required this.store, required this.platform});

  /// The own Profile; null until the defaults were applied once.
  Profile? get profile => store.ownProfile;

  /// The own Profile's short code; null until it exists.
  String? get code => store.ownProfileCode;

  /// Makes the Profile from the phone's name the first time, and takes a
  /// changed phone name later while the user has not typed one.
  Future<void> applyDefaults() async {
    final name = await platform.deviceName();
    final before = store.ownProfile?.encode();
    final after = store.applyProfileDefaults(name: name).encode();
    if (before != after) notifyListeners();
  }

  /// Gives the Profile the [name] the user typed.
  void rename(String name) {
    if (profile == null) return;
    final before = profile?.version;
    if (store.renameOwnProfile(name).version != before) notifyListeners();
  }

  /// Gives the Profile the [picture] the user chose, or the initials for
  /// null.
  void setPicture(Uint8List? picture) {
    if (profile == null) return;
    final before = profile?.version;
    if (store.setOwnProfilePicture(picture).version != before) {
      notifyListeners();
    }
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
