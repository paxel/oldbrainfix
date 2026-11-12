//! What the codes of a font's strings say (ISO 32000-1, 9.6 to 9.10):
//! its ToUnicode map first, else for a simple font its encoding with
//! the glyph names of its differences; and how wide each glyph is.

use std::collections::HashMap;

use crate::file::File;
use crate::object::{Dict, Lexer, Obj};

/// A font as far as its text goes.
pub struct Font {
    /// A composite font: two-byte codes unless its map says otherwise.
    composite: bool,
    map: Option<CMap>,
    /// For a simple font: the text of each code.
    simple: Vec<Option<String>>,
    /// The widths of glyphs by code, in thousandths of the size.
    widths: HashMap<u32, f64>,
    default_width: f64,
}

/// A code of a string, how many bytes it took and the text it stands for.
pub struct Glyph {
    pub code: u32,
    pub bytes: usize,
    pub text: String,
    pub width: f64,
}

impl Font {
    /// The font dictionary `dict`.
    pub fn read(file: &File, dict: &Dict) -> Font {
        let composite = file.name_in(dict, b"Subtype").as_deref() == Some(b"Type0");
        let map = dict
            .get(b"ToUnicode")
            .and_then(|m| file.resolve(m))
            .and_then(|m| file.decoded(&m))
            .map(|data| CMap::read(&data));
        let mut font = Font {
            composite,
            map,
            simple: Vec::new(),
            widths: HashMap::new(),
            default_width: if composite { 1000.0 } else { 500.0 },
        };
        if composite {
            font.composite_widths(file, dict);
        } else {
            font.simple = simple_encoding(file, dict);
            font.simple_widths(file, dict);
        }
        font
    }

    /// A font that is not there: one byte a glyph, as Latin-1 reads.
    pub fn missing() -> Font {
        Font {
            composite: false,
            map: None,
            simple: (0u32..256)
                .map(|c| char::from_u32(c).map(String::from))
                .collect(),
            widths: HashMap::new(),
            default_width: 500.0,
        }
    }

    fn simple_widths(&mut self, file: &File, dict: &Dict) {
        let first = file.number_in(dict, b"FirstChar").unwrap_or(0);
        if let Some(widths) = dict.get(b"Widths").and_then(|w| file.resolve(w))
            && let Some(widths) = widths.as_array()
        {
            for (i, w) in widths.iter().enumerate() {
                let width = file.resolve(w).and_then(|w| w.as_number());
                if let (Some(width), Ok(code)) = (width, u32::try_from(first + i as i64)) {
                    self.widths.insert(code, width);
                }
            }
        }
        if let Some(descriptor) = file.dict_in(dict, b"FontDescriptor")
            && let Some(missing) = descriptor.get(b"MissingWidth").and_then(Obj::as_number)
            && missing > 0.0
        {
            self.default_width = missing;
        }
    }

    fn composite_widths(&mut self, file: &File, dict: &Dict) {
        let Some(descendants) = dict.get(b"DescendantFonts").and_then(|d| file.resolve(d)) else {
            return;
        };
        let Some(first) = descendants.as_array().and_then(|d| d.first()) else {
            return;
        };
        let Some(descendant) = file.resolve(first).and_then(|d| d.as_dict().cloned()) else {
            return;
        };
        if let Some(dw) = descendant.get(b"DW").and_then(Obj::as_number) {
            self.default_width = dw;
        }
        let Some(w) = descendant.get(b"W").and_then(|w| file.resolve(w)) else {
            return;
        };
        let Some(items) = w.as_array() else { return };
        let mut i = 0;
        while let Some(start) = items.get(i).and_then(Obj::as_int) {
            match items.get(i + 1) {
                Some(Obj::Array(widths)) => {
                    for (j, width) in widths.iter().enumerate() {
                        if let (Some(width), Ok(code)) =
                            (width.as_number(), u32::try_from(start + j as i64))
                        {
                            self.widths.insert(code, width);
                        }
                    }
                    i += 2;
                }
                Some(end) => {
                    let (Some(end), Some(width)) =
                        (end.as_int(), items.get(i + 2).and_then(Obj::as_number))
                    else {
                        break;
                    };
                    for code in start..=end.min(start + 65_535) {
                        if let Ok(code) = u32::try_from(code) {
                            self.widths.insert(code, width);
                        }
                    }
                    i += 3;
                }
                None => break,
            }
        }
    }

    /// The glyphs of the string `bytes`.
    pub fn glyphs(&self, bytes: &[u8]) -> Vec<Glyph> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < bytes.len() {
            let length = self.code_length(bytes.get(i..).unwrap_or_default());
            let mut code = 0u32;
            for b in bytes.get(i..i + length).unwrap_or_default() {
                code = code << 8 | u32::from(*b);
            }
            let text = self
                .map
                .as_ref()
                .and_then(|m| m.text(code, length))
                .or_else(|| {
                    if self.composite {
                        None
                    } else {
                        self.simple.get(code as usize).cloned().flatten()
                    }
                })
                .unwrap_or_default();
            let width = self
                .widths
                .get(&code)
                .copied()
                .unwrap_or(self.default_width);
            out.push(Glyph {
                code,
                bytes: length,
                text,
                width,
            });
            i += length;
        }
        out
    }

    /// How many bytes the code at the start of `bytes` takes.
    fn code_length(&self, bytes: &[u8]) -> usize {
        if let Some(map) = &self.map
            && !map.spaces.is_empty()
        {
            for &(length, low, high) in &map.spaces {
                let mut code = 0u32;
                let Some(part) = bytes.get(..length) else {
                    continue;
                };
                for b in part {
                    code = code << 8 | u32::from(*b);
                }
                if (low..=high).contains(&code) {
                    return length;
                }
            }
        }
        if self.composite { 2 } else { 1 }.min(bytes.len()).max(1)
    }
}

/// A ToUnicode map.
#[derive(Default)]
pub struct CMap {
    /// Code space ranges: byte length, lowest and highest code.
    spaces: Vec<(usize, u32, u32)>,
    single: HashMap<(usize, u32), String>,
    /// Ranges: byte length, lowest and highest code, and the text of the
    /// lowest or of each.
    ranges: Vec<(usize, u32, u32, RangeText)>,
}

enum RangeText {
    /// The UTF-16 of the lowest code; the others count on from it.
    From(Vec<u16>),
    Each(Vec<String>),
}

impl CMap {
    pub fn read(data: &[u8]) -> CMap {
        let mut map = CMap::default();
        let mut lexer = Lexer::new(data, 0);
        let mut operands: Vec<Obj> = Vec::new();
        while let Some(object) = lexer.object() {
            let Obj::Keyword(word) = object else {
                operands.push(object);
                continue;
            };
            match word.as_slice() {
                b"endcodespacerange" => {
                    for pair in operands.chunks(2) {
                        if let [Obj::Str(low), Obj::Str(high)] = pair
                            && (1..=4).contains(&low.len())
                        {
                            map.spaces.push((low.len(), number(low), number(high)));
                        }
                    }
                }
                b"endbfchar" => {
                    for pair in operands.chunks(2) {
                        if let [Obj::Str(code), target] = pair
                            && (1..=4).contains(&code.len())
                            && let Some(text) = target_text(target)
                        {
                            map.single.insert((code.len(), number(code)), text);
                        }
                    }
                }
                b"endbfrange" => {
                    for triple in operands.chunks(3) {
                        let [Obj::Str(low), Obj::Str(high), target] = triple else {
                            continue;
                        };
                        if !(1..=4).contains(&low.len()) {
                            continue;
                        }
                        let text = match target {
                            Obj::Str(utf16) => RangeText::From(utf16_units(utf16)),
                            Obj::Array(items) => RangeText::Each(
                                items
                                    .iter()
                                    .map(|i| target_text(i).unwrap_or_default())
                                    .collect(),
                            ),
                            _ => continue,
                        };
                        map.ranges
                            .push((low.len(), number(low), number(high), text));
                    }
                }
                _ => {}
            }
            operands.clear();
        }
        map
    }

    /// The text of `code`, `length` bytes long.
    fn text(&self, code: u32, length: usize) -> Option<String> {
        if let Some(text) = self.single.get(&(length, code)) {
            return Some(text.clone());
        }
        for (l, low, high, text) in &self.ranges {
            if *l != length || code < *low || code > *high {
                continue;
            }
            let offset = code - low;
            return match text {
                RangeText::From(units) => {
                    let mut units = units.clone();
                    if let Some(last) = units.last_mut() {
                        *last = last.wrapping_add(u16::try_from(offset).ok()?);
                    }
                    Some(String::from_utf16_lossy(&units))
                }
                RangeText::Each(texts) => texts.get(offset as usize).cloned(),
            };
        }
        None
    }
}

/// The big-endian number of `bytes`.
fn number(bytes: &[u8]) -> u32 {
    bytes.iter().fold(0u32, |n, b| n << 8 | u32::from(*b))
}

fn utf16_units(bytes: &[u8]) -> Vec<u16> {
    bytes
        .chunks(2)
        .map(|p| match p {
            [h, l] => u16::from(*h) << 8 | u16::from(*l),
            [h] => u16::from(*h) << 8,
            _ => 0,
        })
        .collect()
}

/// The text a map's target stands for: UTF-16 in a string, or a glyph
/// name.
fn target_text(target: &Obj) -> Option<String> {
    match target {
        Obj::Str(utf16) => Some(String::from_utf16_lossy(&utf16_units(utf16))),
        Obj::Name(name) => glyph_text(name),
        _ => None,
    }
}

/// The text of each code of a simple font: its base encoding with the
/// differences on top.
fn simple_encoding(file: &File, dict: &Dict) -> Vec<Option<String>> {
    let encoding = dict.get(b"Encoding").and_then(|e| file.resolve(e));
    let (base, differences) = match encoding.as_deref() {
        Some(Obj::Name(name)) => (Some(name.clone()), None),
        Some(other) => match other.as_dict() {
            Some(d) => (
                file.name_in(d, b"BaseEncoding"),
                d.get(b"Differences")
                    .and_then(|x| file.resolve(x))
                    .and_then(|x| x.as_array().map(<[Obj]>::to_vec)),
            ),
            None => (None, None),
        },
        None => (None, None),
    };
    let mut out: Vec<Option<String>> = (0u32..256)
        .map(|code| base_char(base.as_deref(), code as u8).map(String::from))
        .collect();
    let mut code: Option<usize> = None;
    for item in differences.unwrap_or_default() {
        match item {
            Obj::Int(n) => code = usize::try_from(n).ok(),
            Obj::Name(name) => {
                if let Some(c) = code {
                    if let Some(slot) = out.get_mut(c) {
                        *slot = glyph_text(&name);
                    }
                    code = Some(c + 1);
                }
            }
            _ => {}
        }
    }
    out
}

/// The character `code` stands for in the base encoding `base`: Mac
/// Roman, Standard (the default of a simple font) or WinAnsi.
fn base_char(base: Option<&[u8]>, code: u8) -> Option<char> {
    match base {
        Some(b"MacRomanEncoding") => {
            if code < 0x80 {
                return ascii(code);
            }
            MAC_ROMAN.chars().nth(usize::from(code - 0x80))
        }
        Some(b"WinAnsiEncoding") => match code {
            0x80..=0x9f => WIN_ANSI_HIGH
                .chars()
                .nth(usize::from(code - 0x80))
                .filter(|c| *c != '\0'),
            0xa0..=0xff => char::from_u32(u32::from(code)),
            _ => ascii(code),
        },
        _ => match code {
            0x27 => Some('\u{2019}'),
            0x60 => Some('\u{2018}'),
            0x80.. => STANDARD_HIGH
                .iter()
                .find(|(c, _)| *c == code)
                .map(|(_, ch)| *ch),
            _ => ascii(code),
        },
    }
}

fn ascii(code: u8) -> Option<char> {
    (0x20..0x7f).contains(&code).then(|| char::from(code))
}

/// What WinAnsi has at 0x80 to 0x9F; `\0` where it has nothing.
const WIN_ANSI_HIGH: &str = "€\0‚ƒ„…†‡ˆ‰Š‹Œ\0Ž\0\0‘’“”•–—˜™š›œ\0žŸ";

/// What Mac Roman has at 0x80 to 0xFF.
const MAC_ROMAN: &str = "ÄÅÇÉÑÖÜáàâäãåçéèêëíìîïñóòôöõúùûü†°¢£§•¶ß®©™´¨≠ÆØ∞±≤≥¥µ∂∑∏π∫ªºΩæø¿¡¬√ƒ≈∆«»…\u{a0}ÀÃÕŒœ–—“”‘’÷◊ÿŸ⁄€‹›ﬁﬂ‡·‚„‰ÂÊÁËÈÍÎÏÌÓÔ\u{f8ff}ÒÚÛÙıˆ˜¯˘˙˚¸˝˛ˇ";

/// What the Standard encoding has above 0x7F.
const STANDARD_HIGH: [(u8, char); 54] = [
    (0xa1, '¡'),
    (0xa2, '¢'),
    (0xa3, '£'),
    (0xa4, '⁄'),
    (0xa5, '¥'),
    (0xa6, 'ƒ'),
    (0xa7, '§'),
    (0xa8, '¤'),
    (0xa9, '\''),
    (0xaa, '“'),
    (0xab, '«'),
    (0xac, '‹'),
    (0xad, '›'),
    (0xae, 'ﬁ'),
    (0xaf, 'ﬂ'),
    (0xb1, '–'),
    (0xb2, '†'),
    (0xb3, '‡'),
    (0xb4, '·'),
    (0xb6, '¶'),
    (0xb7, '•'),
    (0xb8, '‚'),
    (0xb9, '„'),
    (0xba, '”'),
    (0xbb, '»'),
    (0xbc, '…'),
    (0xbd, '‰'),
    (0xbf, '¿'),
    (0xc1, '`'),
    (0xc2, '´'),
    (0xc3, 'ˆ'),
    (0xc4, '˜'),
    (0xc5, '¯'),
    (0xc6, '˘'),
    (0xc7, '˙'),
    (0xc8, '¨'),
    (0xca, '˚'),
    (0xcb, '¸'),
    (0xcd, '˝'),
    (0xce, '˛'),
    (0xcf, 'ˇ'),
    (0xd0, '—'),
    (0xe1, 'Æ'),
    (0xe3, 'ª'),
    (0xe8, 'Ł'),
    (0xe9, 'Ø'),
    (0xea, 'Œ'),
    (0xeb, 'º'),
    (0xf1, 'æ'),
    (0xf5, 'ı'),
    (0xf8, 'ł'),
    (0xf9, 'ø'),
    (0xfa, 'œ'),
    (0xfb, 'ß'),
];

/// The text of the glyph `name` (the Adobe Glyph List for the names
/// fonts use most): `uniXXXX` and `uXXXX` by their numbers, ligatures
/// by their parts, a suffix after a dot left out.
pub fn glyph_text(name: &[u8]) -> Option<String> {
    let name = std::str::from_utf8(name).ok()?;
    let name = name.split('.').next().unwrap_or(name);
    if name.contains('_') {
        let parts: Option<String> = name.split('_').map(|p| glyph_text(p.as_bytes())).collect();
        return parts;
    }
    if let Some(hex) = name.strip_prefix("uni")
        && !hex.is_empty()
        && hex.len() % 4 == 0
        && hex.chars().all(|c| c.is_ascii_hexdigit())
    {
        let units: Option<Vec<u16>> = (0..hex.len() / 4)
            .map(|i| u16::from_str_radix(hex.get(i * 4..i * 4 + 4)?, 16).ok())
            .collect();
        return Some(String::from_utf16_lossy(&units?));
    }
    if let Some(hex) = name.strip_prefix('u')
        && (4..=6).contains(&hex.len())
        && hex.chars().all(|c| c.is_ascii_hexdigit())
    {
        return char::from_u32(u32::from_str_radix(hex, 16).ok()?).map(String::from);
    }
    if name.len() == 1 && name.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Some(name.to_string());
    }
    GLYPH_NAMES
        .iter()
        .find(|(n, _)| *n == name)
        .map(|(_, c)| c.to_string())
}

/// The glyph names fonts use most, with their characters.
const GLYPH_NAMES: &[(&str, char)] = &[
    ("space", ' '),
    ("exclam", '!'),
    ("quotedbl", '"'),
    ("numbersign", '#'),
    ("dollar", '$'),
    ("percent", '%'),
    ("ampersand", '&'),
    ("quotesingle", '\''),
    ("quoteright", '’'),
    ("parenleft", '('),
    ("parenright", ')'),
    ("asterisk", '*'),
    ("plus", '+'),
    ("comma", ','),
    ("hyphen", '-'),
    ("period", '.'),
    ("slash", '/'),
    ("zero", '0'),
    ("one", '1'),
    ("two", '2'),
    ("three", '3'),
    ("four", '4'),
    ("five", '5'),
    ("six", '6'),
    ("seven", '7'),
    ("eight", '8'),
    ("nine", '9'),
    ("colon", ':'),
    ("semicolon", ';'),
    ("less", '<'),
    ("equal", '='),
    ("greater", '>'),
    ("question", '?'),
    ("at", '@'),
    ("bracketleft", '['),
    ("backslash", '\\'),
    ("bracketright", ']'),
    ("asciicircum", '^'),
    ("underscore", '_'),
    ("grave", '`'),
    ("quoteleft", '‘'),
    ("braceleft", '{'),
    ("bar", '|'),
    ("braceright", '}'),
    ("asciitilde", '~'),
    ("exclamdown", '¡'),
    ("cent", '¢'),
    ("sterling", '£'),
    ("currency", '¤'),
    ("yen", '¥'),
    ("brokenbar", '¦'),
    ("section", '§'),
    ("dieresis", '¨'),
    ("copyright", '©'),
    ("ordfeminine", 'ª'),
    ("guillemotleft", '«'),
    ("logicalnot", '¬'),
    ("registered", '®'),
    ("macron", '¯'),
    ("degree", '°'),
    ("plusminus", '±'),
    ("twosuperior", '²'),
    ("threesuperior", '³'),
    ("acute", '´'),
    ("mu", 'µ'),
    ("paragraph", '¶'),
    ("periodcentered", '·'),
    ("cedilla", '¸'),
    ("onesuperior", '¹'),
    ("ordmasculine", 'º'),
    ("guillemotright", '»'),
    ("onequarter", '¼'),
    ("onehalf", '½'),
    ("threequarters", '¾'),
    ("questiondown", '¿'),
    ("Agrave", 'À'),
    ("Aacute", 'Á'),
    ("Acircumflex", 'Â'),
    ("Atilde", 'Ã'),
    ("Adieresis", 'Ä'),
    ("Aring", 'Å'),
    ("AE", 'Æ'),
    ("Ccedilla", 'Ç'),
    ("Egrave", 'È'),
    ("Eacute", 'É'),
    ("Ecircumflex", 'Ê'),
    ("Edieresis", 'Ë'),
    ("Igrave", 'Ì'),
    ("Iacute", 'Í'),
    ("Icircumflex", 'Î'),
    ("Idieresis", 'Ï'),
    ("Eth", 'Ð'),
    ("Ntilde", 'Ñ'),
    ("Ograve", 'Ò'),
    ("Oacute", 'Ó'),
    ("Ocircumflex", 'Ô'),
    ("Otilde", 'Õ'),
    ("Odieresis", 'Ö'),
    ("multiply", '×'),
    ("Oslash", 'Ø'),
    ("Ugrave", 'Ù'),
    ("Uacute", 'Ú'),
    ("Ucircumflex", 'Û'),
    ("Udieresis", 'Ü'),
    ("Yacute", 'Ý'),
    ("Thorn", 'Þ'),
    ("germandbls", 'ß'),
    ("agrave", 'à'),
    ("aacute", 'á'),
    ("acircumflex", 'â'),
    ("atilde", 'ã'),
    ("adieresis", 'ä'),
    ("aring", 'å'),
    ("ae", 'æ'),
    ("ccedilla", 'ç'),
    ("egrave", 'è'),
    ("eacute", 'é'),
    ("ecircumflex", 'ê'),
    ("edieresis", 'ë'),
    ("igrave", 'ì'),
    ("iacute", 'í'),
    ("icircumflex", 'î'),
    ("idieresis", 'ï'),
    ("eth", 'ð'),
    ("ntilde", 'ñ'),
    ("ograve", 'ò'),
    ("oacute", 'ó'),
    ("ocircumflex", 'ô'),
    ("otilde", 'õ'),
    ("odieresis", 'ö'),
    ("divide", '÷'),
    ("oslash", 'ø'),
    ("ugrave", 'ù'),
    ("uacute", 'ú'),
    ("ucircumflex", 'û'),
    ("udieresis", 'ü'),
    ("yacute", 'ý'),
    ("thorn", 'þ'),
    ("ydieresis", 'ÿ'),
    ("Euro", '€'),
    ("endash", '–'),
    ("emdash", '—'),
    ("bullet", '•'),
    ("ellipsis", '…'),
    ("quotedblleft", '“'),
    ("quotedblright", '”'),
    ("quotesinglbase", '‚'),
    ("quotedblbase", '„'),
    ("dagger", '†'),
    ("daggerdbl", '‡'),
    ("perthousand", '‰'),
    ("fi", 'ﬁ'),
    ("fl", 'ﬂ'),
    ("OE", 'Œ'),
    ("oe", 'œ'),
    ("Scaron", 'Š'),
    ("scaron", 'š'),
    ("Zcaron", 'Ž'),
    ("zcaron", 'ž'),
    ("Ydieresis", 'Ÿ'),
    ("florin", 'ƒ'),
    ("circumflex", 'ˆ'),
    ("tilde", '˜'),
    ("trademark", '™'),
    ("guilsinglleft", '‹'),
    ("guilsinglright", '›'),
    ("minus", '−'),
    ("fraction", '⁄'),
    ("dotlessi", 'ı'),
    ("Lslash", 'Ł'),
    ("lslash", 'ł'),
    ("nbspace", '\u{a0}'),
    ("nonbreakingspace", '\u{a0}'),
    ("sfthyphen", '\u{ad}'),
    ("ff", 'ﬀ'),
    ("ffi", 'ﬃ'),
    ("ffl", 'ﬄ'),
];

#[cfg(test)]
mod tests {
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
}
