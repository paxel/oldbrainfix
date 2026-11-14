//! The objects of a PDF and the reader of their syntax (ISO 32000-1,
//! 7.2 and 7.3), for the file's objects and for content streams alike.

use std::ops::Range;

/// A PDF object.
#[derive(Debug, Clone, PartialEq)]
pub enum Obj {
    Null,
    Bool(bool),
    Int(i64),
    Real(f64),
    Name(Vec<u8>),
    Str(Vec<u8>),
    Array(Vec<Obj>),
    Dict(Dict),
    /// An indirect reference: object number and generation.
    Ref(u32, u16),
    /// A stream: its dictionary and where its raw data lies in the file.
    Stream(Dict, Range<usize>),
    /// A keyword: an operator in a content stream.
    Keyword(Vec<u8>),
}

/// A dictionary, its entries in their order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Dict(pub Vec<(Vec<u8>, Obj)>);

impl Dict {
    /// The value under the name `key`.
    pub fn get(&self, key: &[u8]) -> Option<&Obj> {
        self.0.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }
}

impl Obj {
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Obj::Int(i) => Some(*i),
            _ => None,
        }
    }

    /// A number, whole or not.
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Obj::Int(i) => Some(*i as f64),
            Obj::Real(r) => Some(*r),
            _ => None,
        }
    }

    pub fn as_name(&self) -> Option<&[u8]> {
        match self {
            Obj::Name(n) => Some(n),
            _ => None,
        }
    }

    pub fn as_dict(&self) -> Option<&Dict> {
        match self {
            Obj::Dict(d) | Obj::Stream(d, _) => Some(d),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Obj]> {
        match self {
            Obj::Array(a) => Some(a),
            _ => None,
        }
    }
}

/// Whether `b` is white space in PDF.
pub fn is_white(b: u8) -> bool {
    matches!(b, 0 | 9 | 10 | 12 | 13 | 32)
}

/// Whether `b` ends a name, a number or a keyword.
pub fn is_delimiter(b: u8) -> bool {
    matches!(
        b,
        b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
    )
}

/// The most items arrays and dictionaries hold together until
/// [`Lexer::held`] is reset, and the deepest they nest: a crafted file
/// must not exhaust the memory or the stack. Items past it are read and
/// dropped.
const MAX_ITEMS: usize = 1 << 20;
const MAX_DEPTH: usize = 64;

/// Reads objects out of `bytes` from a position on.
pub struct Lexer<'b> {
    pub bytes: &'b [u8],
    pub pos: usize,
    /// How many items its arrays and dictionaries hold so far.
    pub held: usize,
}

impl<'b> Lexer<'b> {
    pub fn new(bytes: &'b [u8], pos: usize) -> Lexer<'b> {
        Lexer {
            bytes,
            pos,
            held: 0,
        }
    }

    /// Whether one more item may be kept.
    fn keeps(&mut self) -> bool {
        self.held += 1;
        self.held <= MAX_ITEMS
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn peek_at(&self, ahead: usize) -> Option<u8> {
        self.bytes.get(self.pos + ahead).copied()
    }

    /// Skips white space and comments.
    pub fn skip_white(&mut self) {
        while let Some(b) = self.peek() {
            if is_white(b) {
                self.pos += 1;
            } else if b == b'%' {
                while let Some(b) = self.peek() {
                    if b == b'\n' || b == b'\r' {
                        break;
                    }
                    self.pos += 1;
                }
            } else {
                break;
            }
        }
    }

    /// Whether `word` follows, as a whole keyword, after white space.
    pub fn at_keyword(&mut self, word: &[u8]) -> bool {
        self.skip_white();
        let end = self.pos + word.len();
        self.bytes.get(self.pos..end) == Some(word)
            && self
                .bytes
                .get(end)
                .is_none_or(|&b| is_white(b) || is_delimiter(b))
    }

    /// The next object, a keyword included; `None` at the end or at what
    /// does not read. Two numbers and `R` read as a reference.
    pub fn object(&mut self) -> Option<Obj> {
        self.object_at(0)
    }

    fn object_at(&mut self, depth: usize) -> Option<Obj> {
        if depth > MAX_DEPTH {
            return None;
        }
        self.skip_white();
        let b = self.peek()?;
        match b {
            b'/' => {
                self.pos += 1;
                Some(Obj::Name(self.name()))
            }
            b'(' => {
                self.pos += 1;
                Some(Obj::Str(self.literal()))
            }
            b'<' if self.peek_at(1) == Some(b'<') => {
                self.pos += 2;
                let mut dict = Vec::new();
                loop {
                    self.skip_white();
                    match self.peek()? {
                        b'>' => {
                            self.pos += 1;
                            if self.peek() == Some(b'>') {
                                self.pos += 1;
                            }
                            break;
                        }
                        b'/' => {
                            self.pos += 1;
                            let key = self.name();
                            let value = self.object_at(depth + 1)?;
                            if self.keeps() {
                                dict.push((key, value));
                            }
                        }
                        _ => {
                            // A value without a key: read past it.
                            self.object_at(depth + 1)?;
                        }
                    }
                }
                Some(Obj::Dict(Dict(dict)))
            }
            b'<' => {
                self.pos += 1;
                Some(Obj::Str(self.hex()))
            }
            b'[' => {
                self.pos += 1;
                let mut items = Vec::new();
                loop {
                    self.skip_white();
                    if self.peek()? == b']' {
                        self.pos += 1;
                        break;
                    }
                    let item = self.object_at(depth + 1)?;
                    if self.keeps() {
                        items.push(item);
                    }
                }
                Some(Obj::Array(items))
            }
            b']' | b'>' | b')' | b'{' | b'}' => {
                // Stray: taken as a keyword of its own.
                self.pos += 1;
                Some(Obj::Keyword(vec![b]))
            }
            b'+' | b'-' | b'.' | b'0'..=b'9' => {
                let number = self.number();
                if let Obj::Int(n) = number
                    && n >= 0
                {
                    let back = self.pos;
                    if let Some(reference) = self.reference(n) {
                        return Some(reference);
                    }
                    self.pos = back;
                }
                Some(number)
            }
            _ => {
                let start = self.pos;
                while let Some(b) = self.peek() {
                    if is_white(b) || is_delimiter(b) {
                        break;
                    }
                    self.pos += 1;
                }
                let word = self.bytes.get(start..self.pos)?.to_vec();
                Some(match word.as_slice() {
                    b"true" => Obj::Bool(true),
                    b"false" => Obj::Bool(false),
                    b"null" => Obj::Null,
                    _ => Obj::Keyword(word),
                })
            }
        }
    }

    /// `<generation> R` after the object number `n`.
    fn reference(&mut self, n: i64) -> Option<Obj> {
        self.skip_white();
        let start = self.pos;
        while self.peek().is_some_and(|b| b.is_ascii_digit()) {
            self.pos += 1;
        }
        if self.pos == start {
            return None;
        }
        let generation: u16 = std::str::from_utf8(self.bytes.get(start..self.pos)?)
            .ok()?
            .parse()
            .ok()?;
        self.skip_white();
        if self.peek()? != b'R' {
            return None;
        }
        if self
            .peek_at(1)
            .is_some_and(|b| !is_white(b) && !is_delimiter(b))
        {
            return None;
        }
        self.pos += 1;
        Some(Obj::Ref(u32::try_from(n).ok()?, generation))
    }

    fn number(&mut self) -> Obj {
        let start = self.pos;
        if matches!(self.peek(), Some(b'+' | b'-')) {
            self.pos += 1;
        }
        let mut real = false;
        while let Some(b) = self.peek() {
            match b {
                b'0'..=b'9' => {}
                b'.' if !real => real = true,
                _ => break,
            }
            self.pos += 1;
        }
        let text = self
            .bytes
            .get(start..self.pos)
            .and_then(|t| std::str::from_utf8(t).ok())
            .unwrap_or("0");
        if real {
            let fixed = match text {
                "." | "-." | "+." | "-" | "+" => "0",
                t => t,
            };
            Obj::Real(fixed.parse().unwrap_or(0.0))
        } else {
            match text.parse::<i64>() {
                Ok(n) => Obj::Int(n),
                Err(_) => Obj::Real(text.parse().unwrap_or(0.0)),
            }
        }
    }

    /// A name after its `/`, its `#xx` escapes resolved.
    fn name(&mut self) -> Vec<u8> {
        let mut out = Vec::new();
        while let Some(b) = self.peek() {
            if is_white(b) || is_delimiter(b) {
                break;
            }
            self.pos += 1;
            if b == b'#'
                && let (Some(h), Some(l)) = (
                    self.peek().and_then(hex_digit),
                    self.peek_at(1).and_then(hex_digit),
                )
            {
                self.pos += 2;
                out.push(h << 4 | l);
                continue;
            }
            out.push(b);
        }
        out
    }

    /// A literal string after its `(`.
    fn literal(&mut self) -> Vec<u8> {
        let mut out = Vec::new();
        let mut depth = 1usize;
        while let Some(b) = self.peek() {
            self.pos += 1;
            match b {
                b'(' => {
                    depth += 1;
                    out.push(b);
                }
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                    out.push(b);
                }
                b'\\' => {
                    let Some(e) = self.peek() else { break };
                    self.pos += 1;
                    match e {
                        b'n' => out.push(b'\n'),
                        b'r' => out.push(b'\r'),
                        b't' => out.push(b'\t'),
                        b'b' => out.push(8),
                        b'f' => out.push(12),
                        b'0'..=b'7' => {
                            let mut code = u32::from(e - b'0');
                            for _ in 0..2 {
                                match self.peek() {
                                    Some(d @ b'0'..=b'7') => {
                                        code = code * 8 + u32::from(d - b'0');
                                        self.pos += 1;
                                    }
                                    _ => break,
                                }
                            }
                            out.push((code & 0xff) as u8);
                        }
                        b'\r' => {
                            if self.peek() == Some(b'\n') {
                                self.pos += 1;
                            }
                        }
                        b'\n' => {}
                        other => out.push(other),
                    }
                }
                _ => out.push(b),
            }
        }
        out
    }

    /// A hexadecimal string after its `<`.
    fn hex(&mut self) -> Vec<u8> {
        let mut out = Vec::new();
        let mut high: Option<u8> = None;
        while let Some(b) = self.peek() {
            self.pos += 1;
            if b == b'>' {
                break;
            }
            let Some(d) = hex_digit(b) else { continue };
            match high.take() {
                Some(h) => out.push(h << 4 | d),
                None => high = Some(d),
            }
        }
        if let Some(h) = high {
            out.push(h << 4);
        }
        out
    }
}

pub fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Where `needle` first lies in `hay` from `from` on.
pub fn find(hay: &[u8], needle: &[u8], from: usize) -> Option<usize> {
    let first = *needle.first()?;
    let mut i = from;
    while i + needle.len() <= hay.len() {
        let rest = hay.get(i..)?;
        let at = rest.iter().position(|&b| b == first)?;
        i += at;
        if hay.get(i..i + needle.len()) == Some(needle) {
            return Some(i);
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests;
