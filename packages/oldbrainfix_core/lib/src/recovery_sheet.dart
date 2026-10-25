import 'dart:convert';
import 'dart:typed_data';

import 'package:crypto/crypto.dart';
import 'package:qr/qr.dart';

/// The Recovery Sheet (ADR 0010, 0016): the backup key as a recovery
/// key for people, in Crockford base32 (upper case, no I, L, O or U),
/// in groups of four, and as a QR of the same text, on a printable PDF.

const _alphabet = '0123456789ABCDEFGHJKMNPQRSTVWXYZ';

/// The 256 bits of the key and 4 check bits: 52 characters.
const recoveryCodeLength = 52;

const _keyBytes = 32;

/// The recovery key for the backup [key]: its bits, then the first four
/// bits of its SHA-256, so a typo is told apart from a wrong key.
String encodeRecoveryCode(List<int> key) {
  if (key.length != _keyBytes) throw ArgumentError.value(key, 'key');
  final check = sha256.convert(key).bytes.first >> 4;
  final chars = StringBuffer();
  var bits = 0, held = 0, count = 0;
  void put(int value, int width) {
    held = (held << width) | value;
    bits += width;
    while (bits >= 5) {
      bits -= 5;
      if (count > 0 && count % 4 == 0) chars.write('-');
      chars.write(_alphabet[(held >> bits) & 0x1f]);
      count++;
    }
    held &= (1 << bits) - 1;
  }

  for (final b in key) {
    put(b, 8);
  }
  put(check, 4);
  return chars.toString();
}

/// The backup key in a recovery key as typed or scanned: case, blanks
/// and dashes do not matter, O reads as 0 and I or L as 1; null when it
/// is no recovery key or its check bits do not fit.
Uint8List? decodeRecoveryCode(String text) {
  final clean = text
      .toUpperCase()
      .replaceAll(RegExp(r'[\s-]'), '')
      .replaceAll('O', '0')
      .replaceAll(RegExp('[IL]'), '1');
  if (clean.length != recoveryCodeLength) return null;
  final out = Uint8List(_keyBytes);
  var bits = 0, held = 0, at = 0;
  int? check;
  for (final c in clean.split('')) {
    final value = _alphabet.indexOf(c);
    if (value < 0) return null;
    held = (held << 5) | value;
    bits += 5;
    if (bits >= 8 && at < _keyBytes) {
      bits -= 8;
      out[at++] = (held >> bits) & 0xff;
      held &= (1 << bits) - 1;
    }
  }
  // What is left after the key: the 4 check bits.
  if (bits == 4) check = held;
  if (check != sha256.convert(out).bytes.first >> 4) return null;
  return out;
}

/// The modules of the QR of [code], rows from the top, dark as true.
List<List<bool>> recoveryQr(String code) {
  final image = QrImage(
    QrCode(
      payload: QrPayload.fromString(code),
      errorCorrectLevel: QrErrorCorrectLevel.medium,
    ),
  );
  return [
    for (var r = 0; r < image.moduleCount; r++)
      [for (var c = 0; c < image.moduleCount; c++) image.isDark(r, c)],
  ];
}

/// What a Recovery Sheet says, in the app's language.
class RecoverySheet {
  final String title;

  /// Short lines under the title: when, and on which Device.
  final List<String> about;

  /// Paragraphs before the recovery key.
  final List<String> intro;

  /// The words over the recovery key.
  final String codeLabel;

  /// Paragraphs after the QR: what to do with the sheet.
  final List<String> steps;

  const RecoverySheet({
    required this.title,
    required this.about,
    required this.intro,
    required this.codeLabel,
    required this.steps,
  });
}

/// The Recovery Sheet for [code] as a one-page A4 PDF, written by hand:
/// the standard Helvetica and Courier fonts in WinAnsi, which covers
/// German, and the QR as filled squares. The same text gives the same
/// bytes.
Uint8List recoverySheetPdf(RecoverySheet sheet, String code) {
  final page = _Page();
  page.text(sheet.title, font: 'F2', size: 22);
  page.gap(6);
  for (final line in sheet.about) {
    page.text(line, font: 'F1', size: 11);
  }
  page.gap(12);
  for (final paragraph in sheet.intro) {
    page.paragraph(paragraph);
  }
  page.gap(6);
  page.text(sheet.codeLabel, font: 'F2', size: 12);
  page.gap(4);
  final groups = code.split('-');
  final half = (groups.length + 1) ~/ 2;
  page.text(groups.take(half).join('-'), font: 'F3', size: 15);
  page.text(groups.skip(half).join('-'), font: 'F3', size: 15);
  // The QR's quiet zone: four modules clear above it.
  page.gap(20);
  page.qr(recoveryQr(code));
  page.gap(12);
  for (final paragraph in sheet.steps) {
    page.paragraph(paragraph);
  }
  return _document(page.content.toString());
}

class _Page {
  static const left = 56.0;
  static const height = 842.0;
  static const module = 4.0;

  final content = StringBuffer();
  var y = height - 64;

  void gap(double points) => y -= points;

  void text(String line, {required String font, required double size}) {
    y -= size * 1.25;
    content.write(
      'BT /$font ${_n(size)} Tf ${_n(left)} ${_n(y)} Td (${_pdfString(line)}) Tj ET\n',
    );
  }

  /// [text] in lines of at most 86 characters, broken at blanks, and a
  /// little space after.
  void paragraph(String text) {
    for (final line in _wrap(text, 86)) {
      this.text(line, font: 'F1', size: 11);
    }
    gap(6);
  }

  void qr(List<List<bool>> modules) {
    final size = modules.length * module;
    y -= size;
    content.write('0 0 0 rg\n');
    for (var r = 0; r < modules.length; r++) {
      for (var c = 0; c < modules[r].length; c++) {
        if (!modules[r][c]) continue;
        content.write(
          '${_n(left + c * module)} ${_n(y + size - (r + 1) * module)} '
          '${_n(module)} ${_n(module)} re\n',
        );
      }
    }
    content.write('f\n');
  }
}

/// [text] broken into lines of at most [width] characters at blanks; a
/// word longer than that stands on its own line.
List<String> _wrap(String text, int width) {
  final lines = <String>[];
  var line = '';
  for (final word in text.split(RegExp(r'\s+'))) {
    if (word.isEmpty) continue;
    if (line.isEmpty) {
      line = word;
    } else if (line.length + 1 + word.length <= width) {
      line = '$line $word';
    } else {
      lines.add(line);
      line = word;
    }
  }
  if (line.isNotEmpty) lines.add(line);
  return lines;
}

/// A number as PDF writes it: no exponent, at most two decimals.
String _n(double v) {
  final s = v.toStringAsFixed(2);
  return s.replaceFirst(RegExp(r'\.?0+$'), '');
}

/// WinAnsi's characters beyond Latin-1, by their Unicode code point.
const _winAnsi = {
  0x20ac: 0x80, 0x201a: 0x82, 0x0192: 0x83, 0x201e: 0x84, 0x2026: 0x85, //
  0x2020: 0x86, 0x2021: 0x87, 0x02c6: 0x88, 0x2030: 0x89, 0x0160: 0x8a,
  0x2039: 0x8b, 0x0152: 0x8c, 0x017d: 0x8e, 0x2018: 0x91, 0x2019: 0x92,
  0x201c: 0x93, 0x201d: 0x94, 0x2022: 0x95, 0x2013: 0x96, 0x2014: 0x97,
  0x02dc: 0x98, 0x2122: 0x99, 0x0161: 0x9a, 0x203a: 0x9b, 0x0153: 0x9c,
  0x017e: 0x9e, 0x0178: 0x9f,
};

/// [text] as a PDF literal string in WinAnsi: brackets and backslashes
/// escaped, everything beyond ASCII as an octal escape, and what WinAnsi
/// lacks as `?`.
String _pdfString(String text) {
  final out = StringBuffer();
  for (final rune in text.runes) {
    final code = switch (rune) {
      < 0x80 => rune,
      >= 0xa0 && <= 0xff => rune,
      _ => _winAnsi[rune] ?? 0x3f,
    };
    if (code == 0x28 || code == 0x29 || code == 0x5c) {
      out.write('\\${String.fromCharCode(code)}');
    } else if (code < 0x20 || code >= 0x7f) {
      out.write('\\${code.toRadixString(8).padLeft(3, '0')}');
    } else {
      out.writeCharCode(code);
    }
  }
  return out.toString();
}

/// One page with [content] in a PDF 1.4 file, its cross-reference table
/// counted out byte for byte.
Uint8List _document(String content) {
  final objects = [
    '<< /Type /Catalog /Pages 2 0 R >>',
    '<< /Type /Pages /Kids [3 0 R] /Count 1 >>',
    '<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] '
        '/Resources << /Font << /F1 5 0 R /F2 6 0 R /F3 7 0 R >> >> '
        '/Contents 4 0 R >>',
    '<< /Length ${content.length} >>\nstream\n${content}endstream',
    for (final font in ['Helvetica', 'Helvetica-Bold', 'Courier-Bold'])
      '<< /Type /Font /Subtype /Type1 /BaseFont /$font '
          '/Encoding /WinAnsiEncoding >>',
  ];
  final out = StringBuffer('%PDF-1.4\n');
  final offsets = <int>[];
  for (var i = 0; i < objects.length; i++) {
    offsets.add(out.length);
    out.write('${i + 1} 0 obj\n${objects[i]}\nendobj\n');
  }
  final xref = out.length;
  out.write('xref\n0 ${objects.length + 1}\n0000000000 65535 f \n');
  for (final offset in offsets) {
    out.write('${offset.toString().padLeft(10, '0')} 00000 n \n');
  }
  out.write(
    'trailer\n<< /Size ${objects.length + 1} /Root 1 0 R >>\n'
    'startxref\n$xref\n%%EOF\n',
  );
  // Every byte is ASCII: the content escapes the rest.
  return ascii.encode(out.toString());
}
