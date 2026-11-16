import 'package:oldbrainfix/src/print_platform.dart';

/// A [PrintPlatform] the test drives: what Android says of the print
/// service (#293), and how often its print settings were opened.
class FakePrinting implements PrintPlatform {
  /// What Android says now; the test changes it as the user would in
  /// Android's print settings.
  PrintServiceState state;

  /// How often Android's print settings were opened.
  int opened = 0;

  /// How often the state was asked for.
  int asked = 0;

  FakePrinting([this.state = PrintServiceState.on]);

  @override
  bool get available => true;

  @override
  Future<PrintServiceState> serviceState() async {
    asked++;
    return state;
  }

  @override
  Future<bool> openSettings() async {
    opened++;
    return true;
  }
}
