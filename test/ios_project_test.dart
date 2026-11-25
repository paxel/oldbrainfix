import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

/// The iPad as a device of its own (obf-1149): the app and both its
/// extensions build for the iPhone and the iPad, and the iPad turns every
/// way. Read from the project's files, so it holds without a Mac.
void main() {
  test('every build configuration targets the iPhone and the iPad', () {
    final project = File('ios/Runner.xcodeproj/project.pbxproj')
        .readAsStringSync();
    final families = RegExp(r'TARGETED_DEVICE_FAMILY = ([^;]*);')
        .allMatches(project)
        .map((m) => m.group(1))
        .toList();
    // Debug, Release and Profile of the app, the Share extension and the
    // widgets.
    expect(families, hasLength(9));
    expect(families, everyElement('"1,2"'));
  });

  test('the iPad takes all four orientations', () {
    final info = File('ios/Runner/Info.plist').readAsStringSync();
    final ipad = RegExp(
      r'<key>UISupportedInterfaceOrientations~ipad</key>\s*<array>(.*?)</array>',
      dotAll: true,
    ).firstMatch(info);
    expect(ipad, isNotNull);
    final orientations = RegExp(r'<string>([^<]*)</string>')
        .allMatches(ipad?.group(1) ?? '')
        .map((m) => m.group(1))
        .toSet();
    expect(orientations, {
      'UIInterfaceOrientationPortrait',
      'UIInterfaceOrientationPortraitUpsideDown',
      'UIInterfaceOrientationLandscapeLeft',
      'UIInterfaceOrientationLandscapeRight',
    });
  });
}
