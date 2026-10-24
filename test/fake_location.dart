import 'dart:async';

import 'package:oldbrainfix/src/location.dart';
import 'package:oldbrainfix_core/oldbrainfix_core.dart';

/// A [LocationSource] the test drives: whether the location is allowed,
/// what asking answers, and when the position arrives.
class FakeLocation implements LocationSource {
  /// Whether the location is allowed now.
  bool allowed;

  /// What the user answers when asked.
  bool grantOnAsk;

  /// How often the user was asked.
  int asked = 0;

  /// How often the position was looked up.
  int lookups = 0;

  /// The position the next lookups get; complete it to let it arrive.
  /// Made on first use, inside the widget test's fake time: a completer
  /// made in setUp would answer on the real event loop.
  late Completer<Coordinates?> fix = Completer();

  FakeLocation({this.allowed = true, this.grantOnAsk = true});

  @override
  bool get available => true;

  @override
  Future<bool> granted() async => allowed;

  @override
  Future<bool> request() async {
    asked++;
    allowed = grantOnAsk;
    return allowed;
  }

  @override
  Future<Coordinates?> current() {
    lookups++;
    return fix.future;
  }
}
