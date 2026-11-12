//! A PDF file: its objects found by scanning for them, not by the
//! cross-reference table, so a damaged or updated file still reads; the
//! objects in object streams; its streams decoded; its pages in order.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::rc::Rc;

use crate::object::{Dict, Lexer, Obj, find, hex_digit, is_delimiter, is_white};

/// The most bytes one stream is decoded to.
pub const MAX_DECODED: u64 = 64 << 20;

/// The most filters one stream is decoded through.
const MAX_FILTERS: usize = 8;

/// The most pages read.
const MAX_PAGES: usize = 100_000;

/// Where an object lies.
#[derive(Debug, Clone, Copy)]
enum Place {
    /// After its `obj` keyword in the file.
    Direct(usize),
    /// The `index`th object of the object stream `stream`.
    InStream { stream: u32, index: usize },
}

/// The objects of an object stream, each with its number.
type Objects = Vec<(u32, Obj)>;

/// A page: its dictionary and the resources it inherits.
pub struct PageRef {
    pub dict: Dict,
    pub resources: Option<Dict>,
}

pub struct File<'b> {
    pub bytes: &'b [u8],
    places: HashMap<u32, Place>,
    /// Trailer dictionaries and cross-reference streams, in file order.
    trailers: Vec<Dict>,
    resolved: RefCell<HashMap<u32, Rc<Obj>>>,
    /// The objects being read now: one that refers to itself, as a
    /// stream's length may, reads as nothing instead of without end.
    resolving: RefCell<HashSet<u32>>,
    streams: RefCell<HashMap<u32, Rc<Objects>>>,
    /// How many bytes the object streams may still decode, together.
    streams_left: std::cell::Cell<u64>,
}

impl<'b> File<'b> {
    /// The file `bytes`; `None` when it is no PDF.
    pub fn open(bytes: &'b [u8]) -> Option<File<'b>> {
        let head = bytes.get(..bytes.len().min(1024))?;
        find(head, b"%PDF-", 0)?;
        let mut file = File {
            bytes,
            places: HashMap::new(),
            trailers: Vec::new(),
            resolved: RefCell::new(HashMap::new()),
            resolving: RefCell::new(HashSet::new()),
            streams: RefCell::new(HashMap::new()),
            streams_left: std::cell::Cell::new(MAX_DECODED),
        };
        file.scan();
        file.object_streams();
        Some(file)
    }

    /// Finds every `n g obj` and every `trailer`, stepping over stream
    /// data, so bytes inside a stream are never taken for an object.
    fn scan(&mut self) {
        let bytes = self.bytes;
        let mut i = 0;
        while i < bytes.len() {
            let Some(&b) = bytes.get(i) else { break };
            if b == b'o' && bytes.get(i..i + 3) == Some(b"obj") && ends_word(bytes, i + 3) {
                if let Some(number) = object_number(bytes, i) {
                    self.places.insert(number, Place::Direct(i + 3));
                }
                i += 3;
            } else if b == b's'
                && bytes.get(i..i + 6) == Some(b"stream")
                && i.checked_sub(1)
                    .and_then(|j| bytes.get(j))
                    .is_some_and(|&p| is_white(p) || p == b'>')
                && matches!(bytes.get(i + 6), Some(b'\r' | b'\n'))
            {
                i = find(bytes, b"endstream", i + 6).map_or(bytes.len(), |e| e + 9);
            } else if b == b't'
                && bytes.get(i..i + 7) == Some(b"trailer")
                && ends_word(bytes, i + 7)
            {
                let mut lexer = Lexer::new(bytes, i + 7);
                if let Some(Obj::Dict(dict)) = lexer.object() {
                    self.trailers.push(dict);
                }
                i += 7;
            } else {
                i += 1;
            }
        }
    }

    /// Notes the objects of every object stream whose number is not
    /// found as an object of its own, and takes the cross-reference
    /// streams as trailers.
    fn object_streams(&mut self) {
        let mut found: Vec<(u32, usize, u32)> = Vec::new();
        let mut numbers: Vec<u32> = self.places.keys().copied().collect();
        numbers.sort_unstable();
        for number in numbers {
            let Some(Obj::Stream(dict, _)) = self.get(number).as_deref().cloned() else {
                continue;
            };
            match self.name_in(&dict, b"Type").as_deref() {
                Some(b"XRef") => self.trailers.push(dict),
                Some(b"ObjStm") => {
                    if let Some(objects) = self.stream_objects(number) {
                        for (index, (n, _)) in objects.iter().enumerate() {
                            found.push((*n, index, number));
                        }
                    }
                }
                _ => {}
            }
        }
        for (n, index, stream) in found {
            self.places
                .entry(n)
                .or_insert(Place::InStream { stream, index });
        }
    }

    /// The objects of the object stream `number`, each with its number.
    fn stream_objects(&self, number: u32) -> Option<Rc<Objects>> {
        if let Some(held) = self.streams.borrow().get(&number) {
            return Some(held.clone());
        }
        let stream = self.get(number)?;
        let Obj::Stream(dict, _) = stream.as_ref() else {
            return None;
        };
        let data = self.decoded(&stream)?;
        // Their objects are kept: together they are held to one budget.
        let left = self.streams_left.get().checked_sub(data.len() as u64)?;
        self.streams_left.set(left);
        let count = usize::try_from(self.number_in(dict, b"N")?).ok()?;
        let first = usize::try_from(self.number_in(dict, b"First")?).ok()?;
        let mut header = Lexer::new(&data, 0);
        let mut objects = Vec::new();
        for _ in 0..count.min(1 << 16) {
            let (Some(Obj::Int(n)), Some(Obj::Int(offset))) = (header.object(), header.object())
            else {
                break;
            };
            let (Ok(n), Ok(offset)) = (u32::try_from(n), usize::try_from(offset)) else {
                break;
            };
            let mut lexer = Lexer::new(&data, first + offset);
            objects.push((n, lexer.object().unwrap_or(Obj::Null)));
        }
        let objects = Rc::new(objects);
        self.streams.borrow_mut().insert(number, objects.clone());
        Some(objects)
    }

    /// The object `number`, read once.
    pub fn get(&self, number: u32) -> Option<Rc<Obj>> {
        if let Some(held) = self.resolved.borrow().get(&number) {
            return Some(held.clone());
        }
        let place = *self.places.get(&number)?;
        if !self.resolving.borrow_mut().insert(number) {
            return None;
        }
        let object = match place {
            Place::Direct(at) => self.direct(at),
            Place::InStream { stream, index } => self
                .stream_objects(stream)
                .and_then(|objects| Some(objects.get(index)?.1.clone())),
        };
        self.resolving.borrow_mut().remove(&number);
        let object = Rc::new(object?);
        self.resolved.borrow_mut().insert(number, object.clone());
        Some(object)
    }

    /// The object after the `obj` keyword at `at`: a stream when its
    /// dictionary is followed by `stream`.
    fn direct(&self, at: usize) -> Option<Obj> {
        let mut lexer = Lexer::new(self.bytes, at);
        let object = lexer.object()?;
        let Obj::Dict(dict) = object else {
            return Some(object);
        };
        if !lexer.at_keyword(b"stream") {
            return Some(Obj::Dict(dict));
        }
        let mut start = lexer.pos + 6;
        if self.bytes.get(start) == Some(&b'\r') {
            start += 1;
        }
        if self.bytes.get(start) == Some(&b'\n') {
            start += 1;
        }
        let stated = dict
            .get(b"Length")
            .and_then(|l| match l {
                Obj::Int(n) => Some(*n),
                Obj::Ref(n, _) => self.get(*n).and_then(|o| o.as_int()),
                _ => None,
            })
            .and_then(|n| usize::try_from(n).ok())
            .filter(|n| {
                let mut after = Lexer::new(self.bytes, start + n);
                start + n <= self.bytes.len() && after.at_keyword(b"endstream")
            });
        let end = match stated {
            Some(n) => start + n,
            None => {
                let end = find(self.bytes, b"endstream", start).unwrap_or(self.bytes.len());
                let mut end = end;
                if end > start && self.bytes.get(end - 1) == Some(&b'\n') {
                    end -= 1;
                }
                if end > start && self.bytes.get(end - 1) == Some(&b'\r') {
                    end -= 1;
                }
                end
            }
        };
        Some(Obj::Stream(dict, start..end))
    }

    /// `object`, or the object it refers to.
    pub fn resolve(&self, object: &Obj) -> Option<Rc<Obj>> {
        match object {
            Obj::Ref(n, _) => self.get(*n),
            other => Some(Rc::new(other.clone())),
        }
    }

    /// The whole number under `key` in `dict`, followed if referred to.
    pub fn number_in(&self, dict: &Dict, key: &[u8]) -> Option<i64> {
        self.resolve(dict.get(key)?)?.as_int()
    }

    /// The name under `key` in `dict`, followed if referred to.
    pub fn name_in(&self, dict: &Dict, key: &[u8]) -> Option<Vec<u8>> {
        self.resolve(dict.get(key)?)?.as_name().map(<[u8]>::to_vec)
    }

    /// The dictionary under `key` in `dict`, followed if referred to.
    pub fn dict_in(&self, dict: &Dict, key: &[u8]) -> Option<Dict> {
        self.resolve(dict.get(key)?)?.as_dict().cloned()
    }

    /// Whether the file is encrypted: its text cannot be read.
    pub fn encrypted(&self) -> bool {
        self.trailers.iter().any(|t| t.get(b"Encrypt").is_some())
    }

    /// The document catalog: the last trailer's root, else the object
    /// of type Catalog with the lowest number.
    fn catalog(&self) -> Option<Dict> {
        for trailer in self.trailers.iter().rev() {
            if let Some(root) = self.dict_in(trailer, b"Root") {
                return Some(root);
            }
        }
        let mut numbers: Vec<u32> = self.places.keys().copied().collect();
        numbers.sort_unstable();
        numbers.into_iter().find_map(|n| {
            let object = self.get(n)?;
            let dict = object.as_dict()?;
            (self.name_in(dict, b"Type").as_deref() == Some(b"Catalog")).then(|| dict.clone())
        })
    }

    /// The pages in their order, each with the resources it inherits.
    pub fn pages(&self) -> Vec<PageRef> {
        let mut out = Vec::new();
        let Some(catalog) = self.catalog() else {
            return out;
        };
        let Some(root) = catalog.get(b"Pages") else {
            return out;
        };
        let mut seen: HashSet<u32> = HashSet::new();
        let mut stack: Vec<(Obj, Option<Dict>, usize)> = vec![(root.clone(), None, 0)];
        while let Some((node, inherited, depth)) = stack.pop() {
            if out.len() >= MAX_PAGES || depth > 64 {
                break;
            }
            if let Obj::Ref(n, _) = node
                && !seen.insert(n)
            {
                continue;
            }
            let Some(object) = self.resolve(&node) else {
                continue;
            };
            let Some(dict) = object.as_dict() else {
                continue;
            };
            let resources = self.dict_in(dict, b"Resources").or(inherited);
            let kids = dict
                .get(b"Kids")
                .and_then(|k| self.resolve(k))
                .and_then(|k| k.as_array().map(<[Obj]>::to_vec));
            let is_pages = self.name_in(dict, b"Type").as_deref() == Some(b"Pages");
            match kids {
                Some(kids) if is_pages || dict.get(b"Type").is_none() => {
                    for kid in kids.into_iter().rev() {
                        stack.push((kid, resources.clone(), depth + 1));
                    }
                }
                _ => out.push(PageRef {
                    dict: dict.clone(),
                    resources,
                }),
            }
        }
        out
    }

    /// The data of `stream` with its filters undone; `None` for a filter
    /// not read here.
    pub fn decoded(&self, stream: &Obj) -> Option<Vec<u8>> {
        self.decoded_until(stream, usize::MAX)
    }

    /// The data of `stream` with its first `count` filters undone.
    pub fn decoded_until(&self, stream: &Obj, count: usize) -> Option<Vec<u8>> {
        let Obj::Stream(dict, range) = stream else {
            return None;
        };
        let raw = self.bytes.get(range.clone())?;
        let (filters, parms) = self.filters(dict);
        let mut data = raw.to_vec();
        if filters.len() > MAX_FILTERS {
            return None;
        }
        for (i, filter) in filters.iter().enumerate().take(count) {
            data = decode(filter, &data, parms.get(i).and_then(Option::as_ref))?;
        }
        Some(data)
    }

    /// The filters of `dict` with their parameters, in order.
    pub fn filters(&self, dict: &Dict) -> (Vec<Vec<u8>>, Vec<Option<Dict>>) {
        let names = match dict.get(b"Filter").and_then(|f| self.resolve(f)).as_deref() {
            Some(Obj::Name(n)) => vec![n.clone()],
            Some(Obj::Array(items)) => items
                .iter()
                .filter_map(|i| i.as_name().map(<[u8]>::to_vec))
                .collect(),
            _ => Vec::new(),
        };
        let parms = match dict
            .get(b"DecodeParms")
            .and_then(|p| self.resolve(p))
            .as_deref()
        {
            Some(Obj::Dict(d)) => vec![Some(d.clone())],
            Some(Obj::Array(items)) => items
                .iter()
                .map(|i| self.resolve(i).and_then(|o| o.as_dict().cloned()))
                .collect(),
            _ => Vec::new(),
        };
        (names, parms)
    }
}

/// Whether nothing of a word follows at `at`.
fn ends_word(bytes: &[u8], at: usize) -> bool {
    bytes
        .get(at)
        .is_none_or(|&b| is_white(b) || is_delimiter(b))
}

/// The object number of `n g obj` whose `obj` starts at `at`.
fn object_number(bytes: &[u8], at: usize) -> Option<u32> {
    let mut i = at;
    let back_white = |i: &mut usize| {
        let start = *i;
        while *i > 0 && bytes.get(*i - 1).is_some_and(|&b| is_white(b)) {
            *i -= 1;
        }
        *i < start
    };
    let back_digits = |i: &mut usize| {
        let end = *i;
        while *i > 0 && bytes.get(*i - 1).is_some_and(u8::is_ascii_digit) {
            *i -= 1;
        }
        (*i < end).then_some((*i, end))
    };
    if !back_white(&mut i) {
        return None;
    }
    back_digits(&mut i)?;
    if !back_white(&mut i) {
        return None;
    }
    let (start, end) = back_digits(&mut i)?;
    if start > 0
        && !bytes
            .get(start - 1)
            .is_some_and(|&b| is_white(b) || is_delimiter(b))
    {
        return None;
    }
    std::str::from_utf8(bytes.get(start..end)?)
        .ok()?
        .parse()
        .ok()
}

/// `data` with the filter `filter` undone.
fn decode(filter: &[u8], data: &[u8], parms: Option<&Dict>) -> Option<Vec<u8>> {
    match filter {
        b"FlateDecode" | b"Fl" => {
            let inflated = inflate(data)?;
            predicted(inflated, parms)
        }
        b"ASCIIHexDecode" | b"AHx" => Some(ascii_hex(data)),
        b"ASCII85Decode" | b"A85" => ascii85(data),
        _ => None,
    }
}

/// `data` inflated: a zlib stream, else raw deflate; what inflates
/// before damage counts.
fn inflate(data: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let zlib = flate2::read::ZlibDecoder::new(data)
        .take(MAX_DECODED)
        .read_to_end(&mut out);
    if zlib.is_ok() || !out.is_empty() {
        return Some(out);
    }
    out.clear();
    let raw = flate2::read::DeflateDecoder::new(data)
        .take(MAX_DECODED)
        .read_to_end(&mut out);
    (raw.is_ok() || !out.is_empty()).then_some(out)
}

/// `data` with the PNG predictors of `parms` undone (ISO 32000-1,
/// 7.4.4.4); a TIFF predictor is not read here.
fn predicted(data: Vec<u8>, parms: Option<&Dict>) -> Option<Vec<u8>> {
    let int = |key: &[u8], default: i64| {
        parms
            .and_then(|p| p.get(key))
            .and_then(Obj::as_int)
            .unwrap_or(default)
    };
    let predictor = int(b"Predictor", 1);
    if predictor < 10 {
        return (predictor == 1).then_some(data);
    }
    let colors = usize::try_from(int(b"Colors", 1)).ok()?.clamp(1, 32);
    let bits = usize::try_from(int(b"BitsPerComponent", 8))
        .ok()?
        .clamp(1, 16);
    let columns = usize::try_from(int(b"Columns", 1)).ok()?.clamp(1, 1 << 16);
    let pixel = (colors * bits).div_ceil(8);
    let row = (colors * bits * columns).div_ceil(8);
    let mut out = Vec::with_capacity(data.len());
    let mut previous = vec![0u8; row];
    for line in data.chunks(row + 1) {
        let (Some(&kind), Some(raw)) = (line.first(), line.get(1..)) else {
            break;
        };
        let mut current = raw.to_vec();
        current.resize(row, 0);
        for i in 0..row {
            let left = if i >= pixel {
                current.get(i - pixel).copied().unwrap_or(0)
            } else {
                0
            };
            let up = previous.get(i).copied().unwrap_or(0);
            let corner = if i >= pixel {
                previous.get(i - pixel).copied().unwrap_or(0)
            } else {
                0
            };
            let add = match kind {
                1 => left,
                2 => up,
                3 => ((u16::from(left) + u16::from(up)) / 2) as u8,
                4 => paeth(left, up, corner),
                _ => 0,
            };
            if let Some(byte) = current.get_mut(i) {
                *byte = byte.wrapping_add(add);
            }
        }
        out.extend_from_slice(&current);
        previous = current;
    }
    Some(out)
}

fn paeth(a: u8, b: u8, c: u8) -> u8 {
    let p = i16::from(a) + i16::from(b) - i16::from(c);
    let (pa, pb, pc) = (
        (p - i16::from(a)).abs(),
        (p - i16::from(b)).abs(),
        (p - i16::from(c)).abs(),
    );
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

fn ascii_hex(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut high: Option<u8> = None;
    for &b in data {
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

fn ascii85(data: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut group: Vec<u32> = Vec::with_capacity(5);
    let body = data.strip_prefix(b"<~").unwrap_or(data);
    for &b in body {
        match b {
            b'~' => break,
            b'z' if group.is_empty() => out.extend_from_slice(&[0; 4]),
            b'!'..=b'u' => {
                group.push(u32::from(b - b'!'));
                if group.len() == 5 {
                    let value = group.iter().fold(0u64, |v, d| v * 85 + u64::from(*d));
                    out.extend_from_slice(&(u32::try_from(value).ok()?).to_be_bytes());
                    group.clear();
                }
            }
            _ => {}
        }
    }
    if group.len() > 1 {
        let kept = group.len() - 1;
        group.resize(5, 84);
        let value = group.iter().fold(0u64, |v, d| v * 85 + u64::from(*d));
        let bytes = (u32::try_from(value).ok()?).to_be_bytes();
        out.extend_from_slice(bytes.get(..kept)?);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii85_reads_as_the_spec_writes_it() {
        assert_eq!(ascii85(b"<~87cURD]i,\"Ebo80~>").unwrap(), b"Hello World!");
        assert_eq!(ascii85(b"z~>").unwrap(), vec![0; 4]);
        assert_eq!(ascii_hex(b"48 65 6c6C6f7>"), b"Hellop");
    }

    #[test]
    fn png_predictors_are_undone() {
        // Two rows of three bytes: Sub, then Up.
        let data = vec![1, 10, 5, 5, 2, 1, 1, 1];
        let parms = Dict(vec![
            (b"Predictor".to_vec(), Obj::Int(12)),
            (b"Columns".to_vec(), Obj::Int(3)),
        ]);
        assert_eq!(
            predicted(data, Some(&parms)).unwrap(),
            vec![10, 15, 20, 11, 16, 21]
        );
    }
}
