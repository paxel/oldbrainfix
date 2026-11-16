import 'package:flutter/foundation.dart';

import '../l10n/app_localizations.dart';

/// The words that differ on the iPhone (#244), where Android's would be
/// wrong: no full-screen alarm, as an escalated Occurrence rings as an
/// AlarmKit alarm, Face ID or Touch ID in place of the fingerprint, and
/// the app icon's menu in place of a shortcut on the home screen (#284).
extension IphoneWords on AppLocalizations {
  static bool get _iphone => defaultTargetPlatform == TargetPlatform.iOS;

  /// The Escalation switch in the Routine editor.
  String get escalationSwitchHere =>
      _iphone ? escalationSwitchIphone : escalationSwitch;

  /// What Escalation is set to, after [count] ignored Re-nags.
  String escalationSummaryHere(int count) =>
      _iphone ? escalationSummaryIphone(count) : escalationSummary(count);

  /// What Escalation does.
  String get escalationExplainHere =>
      _iphone ? escalationExplainIphone : escalationExplain;

  /// Unlocking the App Lock and the Area Locks without the PIN.
  String get unlockBiometricsHere =>
      _iphone ? unlockBiometricsIphone : unlockBiometrics;

  /// A Tally card's entry that adds its shortcut (#284).
  String get addToHomeScreenHere =>
      _iphone ? addToHomeScreenIphone : addToHomeScreen;
}
