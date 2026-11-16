//! The text of a PDF for Old Brain Fix's Attachment Texts (ADR 0047):
//! the text each page shows, read from its content in the order it is
//! drawn, through the fonts' ToUnicode maps and encodings; and for a
//! page without text, the scan it shows, for the OCR to read (ADR 0007).
//! Pure Rust, so the desktop and the phones' native library read every
//! PDF alike.
//!
//! Not read: encrypted files, text drawn as outlines, and scans in JPEG
//! 2000, JBIG2 or CCITT fax; such a page gives no text.

mod content;
mod file;
mod font;
mod object;
mod scan;

use file::{File, PageRef};
use object::Obj;

/// A page as far as its text goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page {
    /// The text the page shows, a line break where it moves down; empty
    /// when it shows none.
    pub text: String,
    /// For a page with (almost) no text, the picture it shows when one
    /// is large enough to be its scan: a JPEG or a PNG for the OCR.
    pub scan: Option<Vec<u8>>,
}

/// The fewest characters other than blanks a page's text needs not to
/// count as a scan.
const MIN_TEXT_CHARS: usize = 4;

/// A PDF opened for its text.
pub struct Pdf<'b> {
    file: File<'b>,
    pages: Vec<PageRef>,
}

impl<'b> Pdf<'b> {
    /// The PDF `bytes`; `None` when they are no PDF, or an encrypted
    /// one.
    pub fn open(bytes: &'b [u8]) -> Option<Pdf<'b>> {
        let file = File::open(bytes)?;
        if file.encrypted() {
            return None;
        }
        let pages = file.pages();
        Some(Pdf { file, pages })
    }

    /// How many pages it has.
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// The page `index`, counted from 0.
    pub fn page(&self, index: usize) -> Option<Page> {
        let page = self.pages.get(index)?;
        let data = self.contents(page);
        let shown = content::Runner::new(&self.file).run_page(&data, page.resources.as_ref());
        let text = lines(&shown.text);
        let words = text.chars().filter(|c| !c.is_whitespace()).count();
        let scan = match shown.picture {
            Some((number, pixels)) if words < MIN_TEXT_CHARS && pixels >= scan::MIN_PIXELS => {
                scan::picture(&self.file, number)
            }
            _ => None,
        };
        Some(Page { text, scan })
    }

    /// The content of `page`, its streams one after the other.
    fn contents(&self, page: &PageRef) -> Vec<u8> {
        let mut out = Vec::new();
        let Some(contents) = page
            .dict
            .get(b"Contents")
            .and_then(|c| self.file.resolve(c))
        else {
            return out;
        };
        let streams: Vec<Obj> = match contents.as_ref() {
            Obj::Array(items) => items.clone(),
            other => vec![other.clone()],
        };
        for stream in streams {
            // One stream named again and again decodes again each time.
            if out.len() as u64 >= file::MAX_DECODED {
                break;
            }
            if let Some(stream) = self.file.resolve(&stream)
                && let Some(data) = self.file.decoded(&stream)
            {
                out.extend_from_slice(&data);
                out.push(b'\n');
            }
        }
        out.truncate(file::MAX_DECODED as usize);
        out
    }
}

/// How many pages the PDF `bytes` has, encrypted or not; `None` when
/// they are no PDF.
pub fn page_count(bytes: &[u8]) -> Option<u64> {
    let file = File::open(bytes)?;
    Some(file.pages().len() as u64)
}

/// Every page of the PDF `bytes`; `None` when they are no PDF, or an
/// encrypted one.
pub fn read(bytes: &[u8]) -> Option<Vec<Page>> {
    let pdf = Pdf::open(bytes)?;
    Some((0..pdf.page_count()).filter_map(|i| pdf.page(i)).collect())
}

/// `text` with blanks at the ends of its lines and blank lines at its
/// ends left out.
fn lines(text: &str) -> String {
    let kept: Vec<&str> = text.lines().map(str::trim_end).collect();
    kept.join("\n").trim_matches('\n').to_string()
}

#[cfg(test)]
mod tests;
