//! A page's content stream run for its text (ISO 32000-1, 8 and 9):
//! the text it shows in the order it shows it, a line break where the
//! text moves down, a space where it moves on further than a glyph; and
//! the largest picture the page draws, which may be its scan.

use std::collections::HashMap;
use std::rc::Rc;

use crate::file::{File, MAX_DECODED};
use crate::font::Font;
use crate::object::{Dict, Lexer, Obj, find, is_white};

type Matrix = [f64; 6];

const IDENTITY: Matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

/// `a` then `b`.
fn times(a: &Matrix, b: &Matrix) -> Matrix {
    [
        a[0] * b[0] + a[1] * b[2],
        a[0] * b[1] + a[1] * b[3],
        a[2] * b[0] + a[3] * b[2],
        a[2] * b[1] + a[3] * b[3],
        a[4] * b[0] + a[5] * b[2] + b[4],
        a[4] * b[1] + a[5] * b[3] + b[5],
    ]
}

/// The deepest forms drawn inside forms.
const MAX_FORMS: usize = 8;

/// The most operators run on one page, forms included.
const MAX_OPERATORS: usize = 5_000_000;

/// What a page shows.
#[derive(Default)]
pub struct Shown {
    pub text: String,
    /// The largest picture drawn: the stream's object number, its
    /// pixels.
    pub picture: Option<(u32, u64)>,
}

#[derive(Clone)]
struct State {
    ctm: Matrix,
    font: Option<Rc<Font>>,
    size: f64,
    char_spacing: f64,
    word_spacing: f64,
    scale: f64,
    leading: f64,
    rise: f64,
}

pub struct Runner<'f, 'b> {
    file: &'f File<'b>,
    fonts: HashMap<u32, Rc<Font>>,
    state: State,
    stack: Vec<State>,
    tm: Matrix,
    tlm: Matrix,
    shown: Shown,
    /// Where the text shown last ended and the size it had, in device
    /// space.
    last: Option<(f64, f64, f64)>,
    operators: usize,
    /// How many bytes of forms the page may still decode.
    forms_left: u64,
}

impl<'f, 'b> Runner<'f, 'b> {
    pub fn new(file: &'f File<'b>) -> Runner<'f, 'b> {
        Runner {
            file,
            fonts: HashMap::new(),
            state: State {
                ctm: IDENTITY,
                font: None,
                size: 0.0,
                char_spacing: 0.0,
                word_spacing: 0.0,
                scale: 1.0,
                leading: 0.0,
                rise: 0.0,
            },
            stack: Vec::new(),
            tm: IDENTITY,
            tlm: IDENTITY,
            shown: Shown::default(),
            last: None,
            operators: 0,
            forms_left: MAX_DECODED,
        }
    }

    /// Runs the content `data` of a page with its `resources`.
    pub fn run_page(mut self, data: &[u8], resources: Option<&Dict>) -> Shown {
        self.run(data, resources, 0);
        self.shown
    }

    fn run(&mut self, data: &[u8], resources: Option<&Dict>, depth: usize) {
        let mut lexer = Lexer::new(data, 0);
        let mut operands: Vec<Obj> = Vec::new();
        while let Some(object) = lexer.object() {
            let Obj::Keyword(op) = object else {
                if operands.len() < 1024 {
                    operands.push(object);
                }
                continue;
            };
            self.operators += 1;
            if self.operators > MAX_OPERATORS {
                return;
            }
            if op == b"BI" {
                skip_inline_image(&mut lexer);
            } else {
                self.operate(&op, &operands, resources, depth);
            }
            operands.clear();
            lexer.held = 0;
        }
    }

    fn operate(&mut self, op: &[u8], args: &[Obj], resources: Option<&Dict>, depth: usize) {
        let n = |i: usize| args.get(i).and_then(Obj::as_number).unwrap_or(0.0);
        let matrix = || -> Matrix { [n(0), n(1), n(2), n(3), n(4), n(5)] };
        match op {
            b"q" => {
                if self.stack.len() < 256 {
                    self.stack.push(self.state.clone());
                }
            }
            b"Q" => {
                if let Some(state) = self.stack.pop() {
                    self.state = state;
                }
            }
            b"cm" if args.len() == 6 => self.state.ctm = times(&matrix(), &self.state.ctm),
            b"BT" => {
                self.tm = IDENTITY;
                self.tlm = IDENTITY;
            }
            b"Tf" => {
                self.state.font = args
                    .first()
                    .and_then(Obj::as_name)
                    .and_then(|name| self.font(resources, name));
                self.state.size = n(1);
            }
            b"Tc" => self.state.char_spacing = n(0),
            b"Tw" => self.state.word_spacing = n(0),
            b"Tz" => self.state.scale = n(0) / 100.0,
            b"TL" => self.state.leading = n(0),
            b"Ts" => self.state.rise = n(0),
            b"Td" => self.move_line(n(0), n(1)),
            b"TD" => {
                self.state.leading = -n(1);
                self.move_line(n(0), n(1));
            }
            b"Tm" if args.len() == 6 => {
                self.tm = matrix();
                self.tlm = self.tm;
            }
            b"T*" => self.next_line(),
            b"Tj" => {
                if let Some(Obj::Str(s)) = args.first() {
                    self.show(s);
                }
            }
            b"'" => {
                self.next_line();
                if let Some(Obj::Str(s)) = args.first() {
                    self.show(s);
                }
            }
            b"\"" => {
                self.state.word_spacing = n(0);
                self.state.char_spacing = n(1);
                self.next_line();
                if let Some(Obj::Str(s)) = args.get(2) {
                    self.show(s);
                }
            }
            b"TJ" => {
                for item in args.first().and_then(Obj::as_array).unwrap_or_default() {
                    match item {
                        Obj::Str(s) => self.show(s),
                        other => {
                            let adjust = other.as_number().unwrap_or(0.0);
                            let tx = -adjust / 1000.0 * self.state.size * self.state.scale;
                            self.tm = times(&[1.0, 0.0, 0.0, 1.0, tx, 0.0], &self.tm);
                        }
                    }
                }
            }
            b"Do" => {
                if let Some(name) = args.first().and_then(Obj::as_name) {
                    self.draw(resources, name, depth);
                }
            }
            _ => {}
        }
    }

    fn move_line(&mut self, tx: f64, ty: f64) {
        self.tlm = times(&[1.0, 0.0, 0.0, 1.0, tx, ty], &self.tlm);
        self.tm = self.tlm;
    }

    fn next_line(&mut self) {
        self.move_line(0.0, -self.state.leading);
    }

    /// Where the text drawn now starts, in device space, and its size.
    fn origin(&self) -> (f64, f64, f64) {
        let s = &self.state;
        let text = [s.size * s.scale, 0.0, 0.0, s.size, 0.0, s.rise];
        let m = times(&times(&text, &self.tm), &s.ctm);
        (m[4], m[5], (m[2] * m[2] + m[3] * m[3]).sqrt())
    }

    /// Shows the string `bytes` in the font set now.
    fn show(&mut self, bytes: &[u8]) {
        let font = self
            .state
            .font
            .clone()
            .unwrap_or_else(|| Rc::new(Font::missing()));
        let glyphs = font.glyphs(bytes);
        let (x, y, size) = self.origin();
        let text: String = glyphs.iter().map(|g| g.text.as_str()).collect();
        if !text.is_empty() {
            if let Some((last_x, last_y, last_size)) = self.last {
                let tall = size.max(last_size);
                let out = &mut self.shown.text;
                if (y - last_y).abs() > 0.5 * tall {
                    out.push('\n');
                } else if x - last_x > 0.15 * size.max(1e-9)
                    && !out.ends_with([' ', '\n'])
                    && !text.starts_with(' ')
                {
                    out.push(' ');
                }
            }
            self.shown.text.push_str(&text);
        }
        let s = &self.state;
        for glyph in &glyphs {
            let spacing = if glyph.bytes == 1 && glyph.code == 32 {
                s.word_spacing
            } else {
                0.0
            };
            let tx = (glyph.width / 1000.0 * s.size + s.char_spacing + spacing) * s.scale;
            self.tm = times(&[1.0, 0.0, 0.0, 1.0, tx, 0.0], &self.tm);
        }
        if !text.is_empty() {
            let (end_x, _, _) = self.origin();
            self.last = Some((end_x, y, size));
        }
    }

    /// The font named `name` in `resources`, read once per object.
    fn font(&mut self, resources: Option<&Dict>, name: &[u8]) -> Option<Rc<Font>> {
        let fonts = self.file.dict_in(resources?, b"Font")?;
        let entry = fonts.get(name)?;
        if let Obj::Ref(number, _) = entry
            && let Some(font) = self.fonts.get(number)
        {
            return Some(font.clone());
        }
        let dict = self.file.resolve(entry)?.as_dict()?.clone();
        let font = Rc::new(Font::read(self.file, &dict));
        if let Obj::Ref(number, _) = entry {
            self.fonts.insert(*number, font.clone());
        }
        Some(font)
    }

    /// Draws the XObject `name`: a form runs, a picture is noted.
    fn draw(&mut self, resources: Option<&Dict>, name: &[u8], depth: usize) {
        let Some(objects) = resources.and_then(|r| self.file.dict_in(r, b"XObject")) else {
            return;
        };
        let Some(entry) = objects.get(name) else {
            return;
        };
        let Some(object) = self.file.resolve(entry) else {
            return;
        };
        let Some(dict) = object.as_dict() else {
            return;
        };
        match self.file.name_in(dict, b"Subtype").as_deref() {
            Some(b"Image") => {
                let number = match entry {
                    Obj::Ref(number, _) => *number,
                    _ => return,
                };
                let width = self.file.number_in(dict, b"Width").unwrap_or(0).max(0);
                let height = self.file.number_in(dict, b"Height").unwrap_or(0).max(0);
                let pixels = (width as u64).saturating_mul(height as u64);
                if self.shown.picture.is_none_or(|(_, held)| pixels > held) {
                    self.shown.picture = Some((number, pixels));
                }
            }
            Some(b"Form") if depth < MAX_FORMS => {
                let Some(data) = self.file.decoded(&object) else {
                    return;
                };
                // A form drawn again and again decodes again each time.
                let Some(left) = self.forms_left.checked_sub(data.len() as u64) else {
                    return;
                };
                self.forms_left = left;
                let inner = self.file.dict_in(dict, b"Resources");
                let saved = (self.state.clone(), self.stack.len());
                if let Some(m) = dict.get(b"Matrix").and_then(Obj::as_array)
                    && m.len() == 6
                {
                    let n = |i: usize| m.get(i).and_then(Obj::as_number).unwrap_or(0.0);
                    let matrix = [n(0), n(1), n(2), n(3), n(4), n(5)];
                    self.state.ctm = times(&matrix, &self.state.ctm);
                }
                self.run(&data, inner.as_ref().or(resources), depth + 1);
                self.state = saved.0;
                self.stack.truncate(saved.1);
            }
            _ => {}
        }
    }
}

/// Steps over an inline image after `BI`: its dictionary, `ID`, its
/// data up to `EI`.
fn skip_inline_image(lexer: &mut Lexer) {
    while let Some(object) = lexer.object() {
        if object == Obj::Keyword(b"ID".to_vec()) {
            break;
        }
    }
    let mut from = lexer.pos + 1;
    while let Some(at) = find(lexer.bytes, b"EI", from) {
        let before = at.checked_sub(1).and_then(|i| lexer.bytes.get(i)).copied();
        let after = lexer.bytes.get(at + 2).copied();
        if before.is_some_and(is_white) && after.is_none_or(is_white) {
            lexer.pos = at + 2;
            return;
        }
        from = at + 2;
    }
    lexer.pos = lexer.bytes.len();
}
