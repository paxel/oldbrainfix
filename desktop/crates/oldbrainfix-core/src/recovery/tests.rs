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
