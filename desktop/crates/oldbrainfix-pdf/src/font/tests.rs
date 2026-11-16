use super::*;

#[test]
fn glyph_names_read_as_their_characters() {
    assert_eq!(glyph_text(b"adieresis").as_deref(), Some("ä"));
    assert_eq!(glyph_text(b"uni20AC").as_deref(), Some("€"));
    assert_eq!(glyph_text(b"uni00660069").as_deref(), Some("fi"));
    assert_eq!(glyph_text(b"u1F600").as_deref(), Some("😀"));
    assert_eq!(glyph_text(b"f_f_i").as_deref(), Some("ffi"));
    assert_eq!(glyph_text(b"a.sc").as_deref(), Some("a"));
    assert_eq!(glyph_text(b"g123"), None);
}

#[test]
fn the_encodings_have_their_characters() {
    assert_eq!(MAC_ROMAN.chars().count(), 128);
    assert_eq!(WIN_ANSI_HIGH.chars().count(), 32);
    assert_eq!(base_char(Some(b"MacRomanEncoding"), 0x8a), Some('ä'));
    assert_eq!(base_char(Some(b"WinAnsiEncoding"), 0x80), Some('€'));
    assert_eq!(base_char(Some(b"WinAnsiEncoding"), 0xfc), Some('ü'));
    assert_eq!(base_char(Some(b"WinAnsiEncoding"), 0x81), None);
    assert_eq!(base_char(None, 0xfb), Some('ß'));
    assert_eq!(base_char(None, 0x27), Some('’'));
}

#[test]
fn a_tounicode_map_reads_its_chars_and_ranges() {
    let map = CMap::read(
        b"begincmap 1 begincodespacerange <0000> <FFFF> endcodespacerange
            2 beginbfchar <0003> <0020> <0011> <00660069> endbfchar
            2 beginbfrange <0024> <0026> <0041> <0030> <0031> [<00E4> <00DF>] endbfrange endcmap",
    );
    assert_eq!(map.text(3, 2).as_deref(), Some(" "));
    assert_eq!(map.text(0x11, 2).as_deref(), Some("fi"));
    assert_eq!(map.text(0x25, 2).as_deref(), Some("B"));
    assert_eq!(map.text(0x31, 2).as_deref(), Some("ß"));
    assert_eq!(map.text(0x27, 2), None);
    assert_eq!(map.spaces, vec![(2, 0, 0xffff)]);
}
