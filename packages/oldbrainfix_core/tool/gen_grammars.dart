/// Writes `lib/src/built_in_grammars.dart` from the grammars the apps
/// ship with, `assets/grammars/*.sublime-syntax` (obf-1036), so that the
/// pure-Dart core holds them as the Rust core does with `include_str!`:
///
///     dart run tool/gen_grammars.dart
///
/// The order is the one the language picker shows and a guess tries;
/// the Rust core's `highlight.rs` lists them in the same order. A test
/// fails when the written file is not current.
library;

import 'dart:io';

/// The grammars, by file name without `.sublime-syntax`, in order.
const grammarFiles = [
  'shell',
  'python',
  'javascript',
  'rust',
  'dart',
  'kotlin',
  'java',
  'c',
  'go',
  'sql',
  'json',
  'yaml',
  'xml',
  'css',
  'markdown',
];

void main() {
  final core = File.fromUri(Platform.script).parent.parent.path;
  File('$core/lib/src/built_in_grammars.dart')
      .writeAsStringSync(builtInGrammarsSource('$core/../../assets/grammars'));
}

/// The Dart source of `built_in_grammars.dart` for the grammars in
/// [folder].
String builtInGrammarsSource(String folder) {
  final out = StringBuffer()
    ..writeln('// Written by tool/gen_grammars.dart from')
    ..writeln('// assets/grammars/*.sublime-syntax; do not edit.')
    ..writeln()
    ..writeln('/// The grammars the app ships with (obf-1036), in the order')
    ..writeln('/// the language picker shows them and a guess tries them.')
    ..writeln('const builtInGrammars = <String>[');
  for (final name in grammarFiles) {
    final text = File('$folder/$name.sublime-syntax').readAsStringSync();
    out.writeln('  // $name.sublime-syntax');
    final lines = text.split('\n');
    for (var k = 0; k < lines.length; k++) {
      final last = k == lines.length - 1;
      if (last && lines[k].isEmpty) break;
      out.writeln("      '${_escape(lines[k])}${last ? '' : r'\n'}'");
    }
    out.writeln('      ,');
  }
  out.writeln('];');
  return out.toString();
}

String _escape(String line) => line
    .replaceAll(r'\', r'\\')
    .replaceAll("'", r"\'")
    .replaceAll(r'$', r'\$')
    .replaceAll('\r', r'\r')
    .replaceAll('\t', r'\t');
