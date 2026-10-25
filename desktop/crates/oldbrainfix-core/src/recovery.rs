//! The Recovery Sheet, as the Dart core's `recovery_sheet.dart` makes it
//! (ADR 0010, 0016): the backup key as a recovery key for people, in
//! Crockford base32 (upper case, no I, L, O or U) in groups of four,
//! and as a QR of the same text, on a printable PDF.

use std::fmt::Write;

use sha2::{Digest, Sha256};

const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// The 256 bits of the key and 4 check bits: 52 characters.
pub const CODE_LENGTH: usize = 52;

const KEY_BYTES: usize = 32;

/// The recovery key for the backup `key`: its bits, then the first four
/// bits of its SHA-256, so a typo is told apart from a wrong key.
pub fn encode_code(key: &[u8; KEY_BYTES]) -> String {
    let check = Sha256::digest(key)[0] >> 4;
    let mut out = String::new();
    let (mut held, mut bits, mut count) = (0u32, 0u32, 0usize);
    let mut put = |value: u32, width: u32, out: &mut String| {
        held = (held << width) | value;
        bits += width;
        while bits >= 5 {
            bits -= 5;
            if count > 0 && count.is_multiple_of(4) {
                out.push('-');
            }
            out.push(char::from(ALPHABET[((held >> bits) & 0x1f) as usize]));
            count += 1;
        }
        held &= (1 << bits) - 1;
    };
    for b in key {
        put(u32::from(*b), 8, &mut out);
    }
    put(u32::from(check), 4, &mut out);
    out
}

/// The backup key in a recovery key as typed or scanned: case, blanks
/// and dashes do not matter, O reads as 0 and I or L as 1; none when it
/// is no recovery key or its check bits do not fit.
pub fn decode_code(text: &str) -> Option<[u8; KEY_BYTES]> {
    let clean: Vec<u8> = text
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '-')
        .map(|c| match c.to_ascii_uppercase() {
            'O' => '0',
            'I' | 'L' => '1',
            c => c,
        })
        .map(|c| u8::try_from(c).unwrap_or(b'?'))
        .collect();
    if clean.len() != CODE_LENGTH {
        return None;
    }
    let mut out = [0u8; KEY_BYTES];
    let (mut held, mut bits, mut at) = (0u32, 0u32, 0usize);
    for c in clean {
        let value = ALPHABET.iter().position(|a| *a == c)? as u32;
        held = (held << 5) | value;
        bits += 5;
        if bits >= 8 && at < KEY_BYTES {
            bits -= 8;
            out[at] = ((held >> bits) & 0xff) as u8;
            at += 1;
            held &= (1 << bits) - 1;
        }
    }
    (bits == 4 && held == u32::from(Sha256::digest(out)[0] >> 4)).then_some(out)
}

/// The modules of the QR of `code`, rows from the top, dark as true.
pub fn qr(code: &str) -> Vec<Vec<bool>> {
    let Ok(qr) = qrcode::QrCode::with_error_correction_level(code, qrcode::EcLevel::M) else {
        return Vec::new();
    };
    let width = qr.width();
    let colors = qr.to_colors();
    colors
        .chunks(width.max(1))
        .map(|row| row.iter().map(|c| *c == qrcode::Color::Dark).collect())
        .collect()
}

/// What a Recovery Sheet says, in the app's language.
#[derive(Debug, Clone, Default)]
pub struct Sheet {
    pub title: String,
    /// Short lines under the title: when, and on which Device.
    pub about: Vec<String>,
    /// Paragraphs before the recovery key.
    pub intro: Vec<String>,
    /// The words over the recovery key.
    pub code_label: String,
    /// Paragraphs after the QR: what to do with the sheet.
    pub steps: Vec<String>,
}

/// The Recovery Sheet for `code` as a one-page A4 PDF, written by hand
/// as the phone writes it: the standard Helvetica and Courier fonts in
/// WinAnsi, which covers German, and the QR as filled squares.
pub fn sheet_pdf(sheet: &Sheet, code: &str) -> Vec<u8> {
    let mut page = Page::new();
    page.text(&sheet.title, "F2", 22.0);
    page.gap(6.0);
    for line in &sheet.about {
        page.text(line, "F1", 11.0);
    }
    page.gap(12.0);
    for paragraph in &sheet.intro {
        page.paragraph(paragraph);
    }
    page.gap(6.0);
    page.text(&sheet.code_label, "F2", 12.0);
    page.gap(4.0);
    let groups: Vec<&str> = code.split('-').collect();
    let half = groups.len().div_ceil(2);
    page.text(&groups[..half].join("-"), "F3", 15.0);
    page.text(&groups[half..].join("-"), "F3", 15.0);
    // The QR's quiet zone: four modules clear above it.
    page.gap(20.0);
    page.qr(&qr(code));
    page.gap(12.0);
    for paragraph in &sheet.steps {
        page.paragraph(paragraph);
    }
    document(&page.content)
}

struct Page {
    content: String,
    y: f64,
}

const LEFT: f64 = 56.0;
const HEIGHT: f64 = 842.0;
const MODULE: f64 = 4.0;

impl Page {
    fn new() -> Page {
        Page {
            content: String::new(),
            y: HEIGHT - 64.0,
        }
    }

    fn gap(&mut self, points: f64) {
        self.y -= points;
    }

    fn text(&mut self, line: &str, font: &str, size: f64) {
        self.y -= size * 1.25;
        let _ = writeln!(
            self.content,
            "BT /{font} {} Tf {} {} Td ({}) Tj ET",
            n(size),
            n(LEFT),
            n(self.y),
            pdf_string(line)
        );
    }

    /// `text` in lines of at most 86 characters, broken at blanks, and a
    /// little space after.
    fn paragraph(&mut self, text: &str) {
        for line in wrap(text, 86) {
            self.text(&line, "F1", 11.0);
        }
        self.gap(6.0);
    }

    fn qr(&mut self, modules: &[Vec<bool>]) {
        let size = modules.len() as f64 * MODULE;
        self.y -= size;
        self.content.push_str("0 0 0 rg\n");
        for (r, row) in modules.iter().enumerate() {
            for (c, dark) in row.iter().enumerate() {
                if !dark {
                    continue;
                }
                let _ = writeln!(
                    self.content,
                    "{} {} {} {} re",
                    n(LEFT + c as f64 * MODULE),
                    n(self.y + size - (r as f64 + 1.0) * MODULE),
                    n(MODULE),
                    n(MODULE)
                );
            }
        }
        self.content.push_str("f\n");
    }
}

/// `text` broken into lines of at most `width` characters at blanks, as
/// the Dart core counts them (UTF-16 units); a longer word stands alone.
fn wrap(text: &str, width: usize) -> Vec<String> {
    let units = |s: &str| s.encode_utf16().count();
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if line.is_empty() {
            line = word.to_string();
        } else if units(&line) + 1 + units(word) <= width {
            line.push(' ');
            line.push_str(word);
        } else {
            lines.push(std::mem::replace(&mut line, word.to_string()));
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// A number as PDF writes it: no exponent, at most two decimals.
fn n(v: f64) -> String {
    let s = format!("{v:.2}");
    let s = s.trim_end_matches('0');
    s.trim_end_matches('.').to_string()
}

/// WinAnsi's code for `c`, or `?` when it has none.
fn win_ansi(c: char) -> u8 {
    let code = u32::from(c);
    match code {
        0..0x80 | 0xa0..=0xff => code as u8,
        0x20ac => 0x80,
        0x201a => 0x82,
        0x0192 => 0x83,
        0x201e => 0x84,
        0x2026 => 0x85,
        0x2020 => 0x86,
        0x2021 => 0x87,
        0x02c6 => 0x88,
        0x2030 => 0x89,
        0x0160 => 0x8a,
        0x2039 => 0x8b,
        0x0152 => 0x8c,
        0x017d => 0x8e,
        0x2018 => 0x91,
        0x2019 => 0x92,
        0x201c => 0x93,
        0x201d => 0x94,
        0x2022 => 0x95,
        0x2013 => 0x96,
        0x2014 => 0x97,
        0x02dc => 0x98,
        0x2122 => 0x99,
        0x0161 => 0x9a,
        0x203a => 0x9b,
        0x0153 => 0x9c,
        0x017e => 0x9e,
        0x0178 => 0x9f,
        _ => b'?',
    }
}

/// `text` as a PDF literal string in WinAnsi: brackets and backslashes
/// escaped, everything beyond ASCII as an octal escape.
fn pdf_string(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        let code = win_ansi(c);
        match code {
            b'(' | b')' | b'\\' => {
                out.push('\\');
                out.push(char::from(code));
            }
            0..0x20 | 0x7f.. => {
                let _ = write!(out, "\\{code:03o}");
            }
            _ => out.push(char::from(code)),
        }
    }
    out
}

/// One page with `content` in a PDF 1.4 file, its cross-reference table
/// counted out byte for byte.
fn document(content: &str) -> Vec<u8> {
    let mut objects = vec![
        "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] \
         /Resources << /Font << /F1 5 0 R /F2 6 0 R /F3 7 0 R >> >> \
         /Contents 4 0 R >>"
            .to_string(),
        format!(
            "<< /Length {} >>\nstream\n{content}endstream",
            content.len()
        ),
    ];
    for font in ["Helvetica", "Helvetica-Bold", "Courier-Bold"] {
        objects.push(format!(
            "<< /Type /Font /Subtype /Type1 /BaseFont /{font} /Encoding /WinAnsiEncoding >>"
        ));
    }
    let mut out = String::from("%PDF-1.4\n");
    let mut offsets = Vec::new();
    for (i, object) in objects.iter().enumerate() {
        offsets.push(out.len());
        let _ = write!(out, "{} 0 obj\n{object}\nendobj\n", i + 1);
    }
    let xref = out.len();
    let _ = write!(out, "xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1);
    for offset in offsets {
        let _ = writeln!(out, "{offset:010} 00000 n ");
    }
    let _ = write!(
        out,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        objects.len() + 1
    );
    out.into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_code_reads_back_and_forgives_how_it_was_typed() {
        let key: [u8; 32] = std::array::from_fn(|i| (i * 9) as u8);
        let code = encode_code(&key);
        assert_eq!(code.len(), CODE_LENGTH + 12);
        assert_eq!(decode_code(&code), Some(key));
        let typed = code
            .to_lowercase()
            .replace('-', " ")
            .replace('0', "o")
            .replace('1', "l");
        assert_eq!(decode_code(&typed), Some(key));
    }

    #[test]
    fn a_typo_or_a_wrong_length_is_no_code() {
        let key: [u8; 32] = std::array::from_fn(|i| i as u8);
        let code = encode_code(&key);
        let mut typo: Vec<char> = code.chars().collect();
        typo[5] = if typo[5] == 'X' { 'Y' } else { 'X' };
        assert_eq!(decode_code(&typo.into_iter().collect::<String>()), None);
        assert_eq!(decode_code(&code[1..]), None);
        assert_eq!(decode_code("hello"), None);
    }

    #[test]
    fn the_sheet_is_a_pdf_whose_cross_references_hold() {
        let sheet = Sheet {
            title: "Wiederherstellungsblatt".into(),
            about: vec!["Gerät: Annas Telefon (Küche)".into()],
            intro: vec!["„Aus einer Sicherung wiederherstellen“ wählen. 🧠".into()],
            code_label: "Wiederherstellungsschlüssel".into(),
            steps: vec!["Gut aufbewahren.".into()],
        };
        let code = encode_code(&[7u8; 32]);
        let pdf = sheet_pdf(&sheet, &code);
        assert!(pdf.iter().all(|b| *b < 0x80));
        let text = String::from_utf8_lossy(&pdf).into_owned();
        assert!(text.starts_with("%PDF-1.4\n") && text.ends_with("%%EOF\n"));
        assert!(text.contains(r"Ger\344t: Annas Telefon \(K\374che\)"));
        assert!(text.contains(r"\204Aus einer Sicherung"));
        assert!(text.contains("w\\344hlen. ?"));
        let start = text.find("startxref\n").map(|i| i + 10).unwrap_or_default();
        let xref: usize = text[start..]
            .lines()
            .next()
            .and_then(|l| l.parse().ok())
            .unwrap_or_default();
        assert!(text[xref..].starts_with("xref\n0 8\n"));
        for (i, line) in text[xref..].lines().skip(3).take(7).enumerate() {
            let offset: usize = line[..10].parse().unwrap_or_default();
            assert!(text[offset..].starts_with(&format!("{} 0 obj\n", i + 1)));
        }
        let dark = qr(&code).iter().flatten().filter(|d| **d).count();
        assert_eq!(text.matches(" re\n").count(), dark);
    }
}
