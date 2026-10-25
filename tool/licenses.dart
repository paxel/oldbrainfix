/// Writes the licence lists of the Rust crates the apps ship, with
/// cargo-about (`cargo install cargo-about --locked --version 0.9.2
/// --features cli`):
///
///     dart run tool/licenses.dart
///
/// `licenses/desktop-crates.txt` lists the desktop app's crates
/// (desktop/), `licenses/android-crates.txt` those of the phone's
/// liboldbrainfix_ocr.so (android/ocr). CI runs it and fails when the
/// committed lists are not current. A crate whose licence text comes
/// without its copyright line makes it fail unless the crate is known to
/// have none anywhere: such a crate gets a clarification in
/// `licenses/about.toml` instead.
///
///     dart run tool/licenses.dart --check-maven <dependencies.txt>
///
/// checks that every library of the phone's release runtime classpath
/// (the output of `./gradlew :app:dependencies --configuration
/// releaseRuntimeClasspath`) has its entry in `licenses/android.txt`.
///
/// Every list holds entries of one or more names, one per line, a blank
/// line and the licence text, each closed by a line of 80 `=`; the apps
/// read the same format.
library;

import 'dart:io';

/// The line that closes every entry.
final separator = '=' * 80;

/// The placeholder cargo-about leaves in a standard MIT text when a crate
/// ships no licence file of its own.
const _placeholder = 'Copyright (c) <year> <copyright holders>';

/// Crates whose licence file with a copyright line is neither in their
/// package nor in a repository cargo-about can read it from (dasp_sample
/// names its repository with `.git`, which cargo-about cannot fetch
/// from): their standard MIT text names their authors in general.
final _withoutFile = RegExp(
  r'^(block2|cesu8|dasp_sample|dispatch|dispatch2|objc2|objc2-[a-z-]+|siphasher)$',
);

Future<void> main(List<String> args) async {
  final root = File.fromUri(Platform.script).parent.parent.path;
  if (args.length == 2 && args.first == '--check-maven') {
    exit(_checkMaven(root, File(args[1]).readAsStringSync()));
  }
  for (final (manifest, out, targets) in [
    ('desktop/Cargo.toml', 'desktop-crates.txt', <String>[]),
    (
      'android/ocr/Cargo.toml',
      'android-crates.txt',
      ['aarch64-linux-android', 'armv7-linux-androideabi'],
    ),
  ]) {
    final result = await Process.run(
      'cargo',
      [
        'about',
        'generate',
        '--locked',
        '--fail',
        '--workspace',
        '--manifest-path',
        '$root/$manifest',
        '-c',
        '$root/licenses/about.toml',
        for (final t in targets) ...['--target', t],
        '$root/licenses/about.hbs',
      ],
      environment: {'RAYON_NUM_THREADS': '2'},
    );
    if (result.exitCode != 0) {
      stderr.write(result.stderr);
      exit(1);
    }
    final problems = <String>[];
    final text = tidy(result.stdout as String, problems);
    if (problems.isNotEmpty) {
      stderr.writeln('$out:');
      problems.forEach(stderr.writeln);
      exit(1);
    }
    File('$root/licenses/$out').writeAsStringSync(text);
    stdout.writeln('licenses/$out');
  }
}

/// [generated] as written to the list: line ends and trailing blanks
/// evened out, and the placeholder replaced for the crates known to have
/// no copyright line. What is wrong with it goes into [problems].
String tidy(String generated, List<String> problems) {
  final out = StringBuffer();
  final entries = generated.replaceAll('\r\n', '\n').split('\n$separator\n');
  for (final entry in entries) {
    if (entry.trim().isEmpty) continue;
    final split = entry.indexOf('\n\n');
    final names = (split < 0 ? entry : entry.substring(0, split)).split('\n');
    final odd = names.where((n) => !RegExp(r'^[\w.+-]+ \S+$').hasMatch(n));
    if (split < 0 || odd.isNotEmpty) {
      problems.add('not an entry: ${names.first}');
      continue;
    }
    var body = entry
        .substring(split + 2)
        .split('\n')
        .map((l) => l.trimRight())
        .join('\n')
        .trim();
    if (body.contains(_placeholder)) {
      final unknown = [
        for (final n in names)
          if (!_withoutFile.hasMatch(n.split(' ').first)) n,
      ];
      if (unknown.isNotEmpty) {
        problems.add(
          'no copyright line, clarify in licenses/about.toml: '
          '${unknown.join(', ')}',
        );
      }
      body = body.replaceAll(
        _placeholder,
        'Copyright (c) the authors of the crates named above',
      );
    }
    out
      ..writeln(names.join('\n'))
      ..writeln()
      ..writeln(body)
      ..writeln(separator);
  }
  return out.toString();
}

/// 0 when every `group:artifact` of the Gradle [dependencies] output but
/// Flutter's own engine and BOMs is a name in `licenses/android.txt`.
int _checkMaven(String root, String dependencies) {
  final listed = File('$root/licenses/android.txt')
      .readAsStringSync()
      .split('\n$separator\n')
      .expand((e) => e.split('\n\n').first.split('\n'))
      .toSet();
  final used = {
    for (final m in RegExp(
      r'([\w.-]+):([\w.-]+):[\w.-]+',
    ).allMatches(dependencies))
      if (m[1] != 'io.flutter' && !m[2]!.endsWith('-bom')) '${m[1]}:${m[2]}',
  };
  final missing = used.difference(listed).toList()..sort();
  for (final m in missing) {
    stderr.writeln('not in licenses/android.txt: $m');
  }
  return missing.isEmpty ? 0 : 1;
}
