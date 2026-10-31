//! Imports (ADR 0036), as the Dart core's `imports.dart`: Notes made
//! from the files of a folder, into a new Area named after it, each
//! dated by its file, none of them Nudged. The apps pick the folder and
//! read its files; the core does the rest.

use chrono::{DateTime, SubsecRound, Utc};

use crate::entry::dart_iso;
use crate::error::{Error, Result};
use crate::notes::keys;
use crate::places::Places;
use crate::search::lower_case;
use crate::store::Store;
use crate::text_boxes;
use crate::text_format::{self, KNOWN, MARKDOWN, PLAIN, StoredText, VERSION};
use crate::transcript_prompts::dart_trim;

/// The largest file read; the apps need not read a larger one.
pub const MAX_FILE_BYTES: usize = 1 << 20;

/// The largest text kept, as stored with its format: a Backup reads an
/// entry of up to 1 MiB, its value escaped once more inside it.
pub const MAX_TEXT_BYTES: usize = 256 << 10;

/// The most text one Import keeps, as stored; the newest files are kept
/// first.
pub const MAX_TOTAL_BYTES: usize = 32 << 20;

/// One file handed to an Import: its name, when it was last changed
/// (`None` when the app cannot tell) and its bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportFile {
    /// The file's name; a path's folders before the last `/` or `\` are
    /// left out.
    pub name: String,
    pub modified: Option<DateTime<Utc>>,
    pub bytes: Vec<u8>,
}

/// One Note an Import makes: its `text` in its `format`, when it was
/// `captured`, and the `name` of what it came from, which orders Notes
/// of the same date.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportedNote {
    pub name: String,
    pub text: String,
    pub format: String,
    pub captured: DateTime<Utc>,
}

/// What an Import made, for the apps to announce.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportSummary {
    /// The new Area the Notes landed in; `None` when there was nothing
    /// to import and no Area was made.
    pub area_id: Option<String>,
    /// How many Notes were made.
    pub notes: usize,
    /// Files skipped as no text: another kind of file, a hidden one, one
    /// holding a zero byte, or one with nothing in it or its name.
    pub not_text: usize,
    /// Files skipped as too large ([`MAX_FILE_BYTES`],
    /// [`MAX_TEXT_BYTES`], [`MAX_TOTAL_BYTES`]).
    pub too_large: usize,
}

impl ImportSummary {
    /// How many files were skipped.
    pub fn skipped(&self) -> usize {
        self.not_text + self.too_large
    }
}

/// Imports into one store.
pub struct Imports<'a> {
    store: &'a Store,
}

impl<'a> Imports<'a> {
    pub fn new(store: &'a Store) -> Imports<'a> {
        Imports { store }
    }

    /// Makes one Note of each Markdown (`.md`, `.markdown`) and text
    /// (`.txt`) file of `files` in a new Area named `area_name`, as
    /// [`Imports::notes`] writes them; any other file is skipped and
    /// counted. A Markdown file's Note is formatted, its Markdown as
    /// written; a text file's is plain. A Note is dated by its file's
    /// modification time, and by now when that is unknown or later.
    pub fn folder(&self, area_name: &str, files: &[ImportFile]) -> Result<ImportSummary> {
        let name = area(area_name)?;
        let now = self.store.now();
        let mut made = Vec::new();
        let (mut not_text, mut too_large) = (0, 0);
        for file in files {
            let base = base(&file.name);
            let Some((stem, format)) = format_of(base) else {
                not_text += 1;
                continue;
            };
            if file.bytes.len() > MAX_FILE_BYTES {
                too_large += 1;
                continue;
            }
            let Some(read) = decode(&file.bytes) else {
                not_text += 1;
                continue;
            };
            let text = titled(stem, &read, format);
            if text.is_empty() {
                not_text += 1;
                continue;
            }
            let at = match file.modified.map(|at| at.trunc_subsecs(6)) {
                Some(at) if at > DateTime::UNIX_EPOCH && at <= now => at,
                _ => now,
            };
            made.push(ImportedNote {
                name: base.to_string(),
                text,
                format: format.to_string(),
                captured: at,
            });
        }
        self.notes(name, &made, not_text, too_large)
    }

    /// Writes `notes` into a new Area named `area_name`, oldest first,
    /// filed there so none of them is Unsorted or Nudged, each captured
    /// and dated when it says. An Area of that name already there gets a
    /// new one beside it, named with a number after it. A text larger
    /// than [`MAX_TEXT_BYTES`], and the oldest beyond
    /// [`MAX_TOTAL_BYTES`], are skipped as too large, counted on top of
    /// `not_text` and `too_large`; with nothing left, no Area is made.
    pub fn notes(
        &self,
        area_name: &str,
        notes: &[ImportedNote],
        not_text: usize,
        too_large: usize,
    ) -> Result<ImportSummary> {
        let name = area(area_name)?;
        if let Some(note) = notes.iter().find(|n| !KNOWN.contains(&n.format.as_str())) {
            return Err(Error::Invalid(format!(
                "not a format known here: {}",
                note.format
            )));
        }
        let mut order: Vec<(usize, &ImportedNote, DateTime<Utc>)> = notes
            .iter()
            .enumerate()
            .map(|(i, n)| (i, n, n.captured.trunc_subsecs(6)))
            .collect();
        order.sort_by(|a, b| (a.2, &a.1.name, a.0).cmp(&(b.2, &b.1.name, b.0)));
        let mut kept = Vec::new();
        let mut large = too_large;
        let mut total = 0;
        for (_, note, at) in order.iter().rev() {
            let size = StoredText::of(&note.format, &note.text, VERSION)
                .encode()
                .len();
            if size > MAX_TEXT_BYTES || total + size > MAX_TOTAL_BYTES {
                large += 1;
                continue;
            }
            total += size;
            kept.push((*note, *at));
        }
        if kept.is_empty() {
            return Ok(ImportSummary {
                area_id: None,
                notes: 0,
                not_text,
                too_large: large,
            });
        }
        let s = self.store;
        let area_id = s.batch(|| {
            let area = Places::new(s).add_area(&self.unused(name)?)?;
            for (note, at) in kept.iter().rev() {
                let id = format!("note:{}", s.new_id()?);
                let date = Some(*at);
                s.append(&id, keys::TYPE, Some(keys::TYPE_NOTE), date)?;
                s.append(&id, keys::CAPTURED, Some(&dart_iso(*at)), date)?;
                let boxed = text_boxes::convert(&note.text, &note.format, || s.new_id())?;
                let text = StoredText::of(&note.format, &boxed, VERSION).encode();
                s.append(&id, keys::TEXT, Some(&text), date)?;
                s.append(&id, keys::PLACE, Some(&area.id), date)?;
            }
            Ok(area.id)
        })?;
        Ok(ImportSummary {
            area_id: Some(area_id),
            notes: kept.len(),
            not_text,
            too_large: large,
        })
    }

    /// `name`, or with " (2)", " (3)" and so on after it when an Area is
    /// called so already.
    fn unused(&self, name: &str) -> Result<String> {
        let taken: Vec<String> = Places::new(self.store)
            .areas()?
            .into_iter()
            .map(|a| a.name)
            .collect();
        if !taken.iter().any(|t| t == name) {
            return Ok(name.to_string());
        }
        let mut n = 2;
        loop {
            let candidate = format!("{name} ({n})");
            if !taken.contains(&candidate) {
                return Ok(candidate);
            }
            n += 1;
        }
    }
}

fn area(name: &str) -> Result<&str> {
    let trimmed = dart_trim(name);
    if trimmed.is_empty() {
        return Err(Error::Invalid("the Area's name must not be empty".into()));
    }
    Ok(trimmed)
}

/// Unicode's White_Space and the byte order mark, as Dart's `trim` knows
/// them.
fn is_space(c: char) -> bool {
    c.is_whitespace() || c == '\u{feff}'
}

/// The part of `name` after its last `/` or `\`.
fn base(name: &str) -> &str {
    name.rsplit(['/', '\\']).next().unwrap_or(name)
}

/// The name without its extension of the file `base`, and the format it
/// becomes a Note in, by its extension in any case of ASCII letters;
/// `None` for a hidden file and any other kind.
fn format_of(base: &str) -> Option<(&str, &'static str)> {
    if base.starts_with('.') {
        return None;
    }
    let (stem, extension) = base.rsplit_once('.')?;
    match extension.to_ascii_lowercase().as_str() {
        "md" | "markdown" => Some((stem, MARKDOWN)),
        "txt" => Some((stem, PLAIN)),
        _ => None,
    }
}

/// The text of `bytes`: UTF-8 without the byte order mark before it,
/// else Windows-1252, the encoding of older text files on Windows,
/// which reads every byte; line breaks as `\n`. `None` for bytes holding
/// a zero, which no text file does.
fn decode(bytes: &[u8]) -> Option<String> {
    let utf8 = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    let text = match std::str::from_utf8(utf8) {
        Ok(text) => text.to_string(),
        Err(_) => bytes
            .iter()
            .map(|&b| match b {
                0x80..=0x9f => CP1252[usize::from(b - 0x80)],
                _ => char::from(b),
            })
            .collect(),
    };
    if text.contains('\0') {
        return None;
    }
    Some(text.replace("\r\n", "\n").replace('\r', "\n"))
}

/// What Windows-1252 has at 0x80 to 0x9F; the five bytes it leaves open
/// read as Latin-1 does.
const CP1252: [char; 32] = [
    '\u{20ac}', '\u{0081}', '\u{201a}', '\u{0192}', '\u{201e}', '\u{2026}', '\u{2020}', '\u{2021}',
    '\u{02c6}', '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\u{008d}', '\u{017d}', '\u{008f}',
    '\u{0090}', '\u{2018}', '\u{2019}', '\u{201c}', '\u{201d}', '\u{2022}', '\u{2013}', '\u{2014}',
    '\u{02dc}', '\u{2122}', '\u{0161}', '\u{203a}', '\u{0153}', '\u{009d}', '\u{017e}', '\u{0178}',
];

/// `text` without its blank lines at the start and blanks at the end,
/// under the file's name `stem` as its first line, a heading in
/// Markdown, unless its first line with words already says it (the
/// Note's title, in any case).
fn titled(stem: &str, text: &str, format: &str) -> String {
    let trimmed = text.trim_end_matches(is_space);
    let lead = trimmed.len() - trimmed.trim_start_matches(is_space).len();
    let body = &trimmed[trimmed[..lead].rfind('\n').map_or(0, |i| i + 1)..];
    let name = stem.replace(['\r', '\n'], " ");
    let name = dart_trim(&name);
    if name.is_empty() {
        return body.to_string();
    }
    let visible = text_format::visible(format, body);
    let first = visible
        .split('\n')
        .map(dart_trim)
        .find(|words| !words.is_empty());
    if first.is_some_and(|words| lower_case(words) == lower_case(name)) {
        return body.to_string();
    }
    let title = if format == MARKDOWN {
        format!("# {}", text_format::literal(format, name))
    } else {
        name.to_string()
    };
    if body.is_empty() {
        title
    } else {
        format!("{title}\n{body}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notes::Notes;
    use crate::signing::SigningKey;
    use chrono::TimeZone;

    fn at(y: i32, m: u32, d: u32, h: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, h, 0, 0)
            .single()
            .unwrap_or_default()
    }

    fn now() -> DateTime<Utc> {
        at(2026, 10, 31, 12)
    }

    fn store() -> Store {
        match Store::in_memory(Box::new(now), || Ok(SigningKey::from_seed([1; 32]))) {
            Ok(s) => s,
            Err(e) => panic!("store: {e}"),
        }
    }

    fn file(name: &str, text: &str, modified: Option<DateTime<Utc>>) -> ImportFile {
        raw(name, text.as_bytes().to_vec(), modified)
    }

    fn raw(name: &str, bytes: Vec<u8>, modified: Option<DateTime<Utc>>) -> ImportFile {
        ImportFile {
            name: name.into(),
            modified,
            bytes,
        }
    }

    fn march() -> DateTime<Utc> {
        at(2025, 3, 1, 8)
    }

    fn may() -> DateTime<Utc> {
        at(2025, 5, 2, 9) + chrono::Duration::milliseconds(123)
    }

    /// The Notes inside the Area `summary` made, newest first.
    fn inside(store: &Store, summary: &ImportSummary) -> Result<Vec<crate::notes::Note>> {
        let id = summary
            .area_id
            .as_deref()
            .ok_or(Error::Invalid("no Area".into()))?;
        Notes::new(store).inside(id)
    }

    #[test]
    fn a_folder_becomes_notes_in_a_new_area_dated_by_their_files() -> Result<()> {
        let store = store();
        let summary = Imports::new(&store).folder(
            "From Notes",
            &[
                file("Later.txt", "Later\nsecond line", Some(may())),
                file("Earlier.md", "# Earlier\n**bold**", Some(march())),
                file("photo.png", "not text", Some(march())),
            ],
        )?;
        assert_eq!(
            (summary.notes, summary.skipped(), summary.not_text),
            (2, 1, 1)
        );
        let id = summary.area_id.clone().unwrap_or_default();
        let area = Places::new(&store).area_by_id(&id)?;
        assert_eq!(area.map(|a| a.name), Some("From Notes".into()));
        let notes = inside(&store, &summary)?;
        let texts: Vec<_> = notes.iter().map(|n| n.text.as_str()).collect();
        assert_eq!(texts, ["Later\nsecond line", "# Earlier\n**bold**"]);
        let dates: Vec<_> = notes.iter().map(|n| n.captured).collect();
        assert_eq!(dates, [may(), march()]);
        let formats: Vec<_> = notes.iter().map(|n| n.text_format.as_str()).collect();
        assert_eq!(formats, [PLAIN, MARKDOWN]);
        // Written oldest first, every entry of a Note dated by its file.
        let earlier = &notes[1].id;
        let first = store
            .all_entries()?
            .into_iter()
            .find(|e| e.entity.starts_with("note:"));
        assert_eq!(first.map(|e| e.entity), Some(earlier.clone()));
        for entry in store.history(earlier)? {
            assert_eq!(entry.date_at, march());
        }
        Ok(())
    }

    #[test]
    fn nothing_imported_is_unsorted_or_nudged() -> Result<()> {
        let store = store();
        let summary = Imports::new(&store).folder(
            "From Notes",
            &[
                file("a.txt", "a", Some(march())),
                file("b.md", "b", Some(may())),
            ],
        )?;
        assert!(Notes::new(&store).unsorted()?.is_empty());
        let notes = inside(&store, &summary)?;
        assert_eq!(notes.len(), 2);
        for note in notes {
            assert_eq!(note.place, summary.area_id);
            assert_eq!(store.current(&note.id, keys::NUDGE)?, None);
        }
        Ok(())
    }

    #[test]
    fn an_import_lands_beside_an_area_of_the_same_name() -> Result<()> {
        let store = store();
        let places = Places::new(&store);
        let mine = places.add_area("From Notes")?;
        let imports = Imports::new(&store);
        let first = imports.folder(" From Notes ", &[file("a.txt", "a", None)])?;
        let second = imports.folder("From Notes", &[file("b.txt", "b", None)])?;
        let name = |s: &ImportSummary| -> Result<Option<String>> {
            let id = s.area_id.clone().unwrap_or_default();
            Ok(places.area_by_id(&id)?.map(|a| a.name))
        };
        assert_eq!(name(&first)?, Some("From Notes (2)".into()));
        assert_eq!(name(&second)?, Some("From Notes (3)".into()));
        assert!(Notes::new(&store).inside(&mine.id)?.is_empty());
        assert!(imports.folder("  ", &[file("a.txt", "a", None)]).is_err());
        Ok(())
    }

    #[test]
    fn nothing_to_import_makes_no_area_and_writes_nothing() -> Result<()> {
        let store = store();
        let imports = Imports::new(&store);
        let none = [
            file("photo.jpg", "x", None),
            file(".hidden.md", "x", None),
            file("README", "x", None),
        ];
        for files in [&none[..0], &none[..]] {
            let summary = imports.folder("From Notes", files)?;
            assert_eq!(summary.area_id, None);
            assert_eq!((summary.notes, summary.not_text), (0, files.len()));
        }
        assert!(store.all_entries()?.is_empty());
        Ok(())
    }

    #[test]
    fn notes_of_one_date_go_by_name_as_code_points() -> Result<()> {
        let store = store();
        let files = [
            file("😀.txt", "emoji", Some(march())),
            file("～.txt", "tilde", Some(march())),
            file("b.txt", "b", Some(march())),
            file("a.txt", "a", Some(march())),
        ];
        Imports::new(&store).folder("From Notes", &files)?;
        let notes = Notes::new(&store);
        let mut lasts = Vec::new();
        for e in store.all_entries()? {
            if e.field == keys::TYPE && e.entity.starts_with("note:") {
                let text = notes.by_id(&e.entity)?.map(|n| n.text).unwrap_or_default();
                lasts.push(text.rsplit('\n').next().unwrap_or_default().to_string());
            }
        }
        assert_eq!(lasts, ["a", "b", "tilde", "emoji"]);
        Ok(())
    }

    #[test]
    fn a_file_of_unknown_zero_or_later_time_is_dated_now() -> Result<()> {
        let store = store();
        let summary = Imports::new(&store).folder(
            "From Notes",
            &[
                file("none.txt", "none", None),
                file("zero.txt", "zero", Some(DateTime::UNIX_EPOCH)),
                file("later.txt", "later", Some(at(2030, 1, 1, 0))),
            ],
        )?;
        for note in inside(&store, &summary)? {
            assert_eq!(note.captured, now());
        }
        Ok(())
    }

    /// The one Note `f` makes, as shown.
    fn only(f: ImportFile) -> Result<String> {
        let store = store();
        let summary = Imports::new(&store).folder("From Notes", std::slice::from_ref(&f))?;
        assert_eq!(summary.notes, 1, "{}", f.name);
        let notes = inside(&store, &summary)?;
        Ok(notes.first().map(|n| n.shown_text()).unwrap_or_default())
    }

    #[test]
    fn a_file_is_read_as_utf8_without_its_mark_or_as_windows_1252() -> Result<()> {
        let mut bom = vec![0xef, 0xbb, 0xbf];
        bom.extend(b"Shop\r\nmilk\reggs\r\n");
        assert_eq!(only(raw("Shop.txt", bom, None))?, "Shop\nmilk\neggs");
        let mut twice = vec![0xef, 0xbb, 0xbf, 0xef, 0xbb, 0xbf];
        twice.extend(b"Two");
        assert_eq!(only(raw("Two.txt", twice, None))?, "\u{feff}Two");
        let mut bytes = b"Herr M".to_vec();
        bytes.extend([0xfc]);
        bytes.extend(b"ller ");
        bytes.extend([0x80, 0x81]);
        assert_eq!(
            only(raw("Bill.txt", bytes, None))?,
            "Bill\nHerr Müller €\u{81}"
        );
        Ok(())
    }

    #[test]
    fn a_file_with_a_zero_byte_or_hidden_is_no_text() -> Result<()> {
        let store = store();
        let summary = Imports::new(&store).folder(
            "From Notes",
            &[
                raw("wide.txt", vec![0xff, 0xfe, 0x41, 0x00], None),
                file("._mac.md", "x", None),
            ],
        )?;
        assert_eq!((summary.not_text, summary.area_id), (2, None));
        Ok(())
    }

    #[test]
    fn a_file_is_taken_by_its_extension_in_any_case() -> Result<()> {
        let store = store();
        let summary = Imports::new(&store).folder(
            "From Notes",
            &[
                file("A.MD", "alpha", None),
                file("b.Markdown", "beta", None),
                file("sub/c.TXT", "gamma", None),
                file("win\\d.txt", "delta", None),
                file("e.text", "epsilon", None),
                file("f.mar\u{212a}down", "kelvin", None),
            ],
        )?;
        assert_eq!((summary.notes, summary.not_text), (4, 2));
        let mut texts: Vec<_> = inside(&store, &summary)?
            .iter()
            .map(|n| n.shown_text())
            .collect();
        texts.sort();
        assert_eq!(texts, ["# A\nalpha", "# b\nbeta", "c\ngamma", "d\ndelta"]);
        Ok(())
    }

    #[test]
    fn a_file_is_titled_by_its_name_unless_its_first_line_says_so() -> Result<()> {
        let cases = [
            ("Dentist.txt", "Tuesday 9:00", "Dentist\nTuesday 9:00"),
            (
                "dentist.txt",
                "\n  \nDENTIST\nTuesday\n\n",
                "DENTIST\nTuesday",
            ),
            ("Shop.md", "## *Shop*\n- milk", "## *Shop*\n- milk"),
            ("2*3 [x].md", "- a", "# 2\\*3 \\[x]\n- a"),
            ("Call the bank.md", "", "# Call the bank"),
            ("  .txt", "  kept  ", "  kept"),
        ];
        for (name, text, shown) in cases {
            assert_eq!(only(file(name, text, None))?, shown, "{name}");
        }
        let store = store();
        let summary = Imports::new(&store).folder("From Notes", &[file("  .txt", " \n ", None)])?;
        assert_eq!((summary.not_text, summary.area_id), (1, None));
        Ok(())
    }

    #[test]
    fn a_file_gets_its_boxes() -> Result<()> {
        let store = store();
        let summary = Imports::new(&store).folder(
            "From Notes",
            &[file("Trip.md", "# Trip\n- ☐ passport", None)],
        )?;
        let notes = inside(&store, &summary)?;
        assert_eq!(notes.first().map(|n| n.boxes.len()), Some(1));
        assert_eq!(
            notes.first().map(|n| n.shown_text()),
            Some("# Trip\n- ☐ passport".into())
        );
        Ok(())
    }

    #[test]
    fn a_file_or_text_too_large_is_skipped() -> Result<()> {
        let store = store();
        let summary = Imports::new(&store).folder(
            "From Notes",
            &[
                raw("big.txt", vec![b'a'; MAX_FILE_BYTES + 1], None),
                // Under the file's limit, but each quote stored as two bytes.
                file("quotes.txt", &"\"".repeat(MAX_TEXT_BYTES / 2 + 1), None),
                file("fits.txt", &"x".repeat(MAX_TEXT_BYTES - 100), None),
            ],
        )?;
        assert_eq!((summary.too_large, summary.notes), (2, 1));
        Ok(())
    }

    #[test]
    fn beyond_the_total_the_oldest_files_are_skipped() -> Result<()> {
        let store = store();
        let text = "a".repeat(256_000);
        let files: Vec<_> = (0..132)
            .map(|i| {
                file(
                    &format!("{i}.txt"),
                    &text,
                    Some(march() + chrono::Duration::minutes(i)),
                )
            })
            .collect();
        let summary = Imports::new(&store).folder("From Notes", &files)?;
        assert_eq!((summary.notes, summary.too_large), (131, 1));
        let dates: Vec<_> = inside(&store, &summary)?
            .iter()
            .map(|n| n.captured)
            .collect();
        assert!(!dates.contains(&march()));
        assert_eq!(
            dates.first(),
            Some(&(march() + chrono::Duration::minutes(131)))
        );
        Ok(())
    }

    #[test]
    fn only_formats_known_here_are_taken() {
        let store = store();
        let note = ImportedNote {
            name: "a".into(),
            text: "a".into(),
            format: "x-future".into(),
            captured: now(),
        };
        assert!(
            Imports::new(&store)
                .notes("From Keep", &[note], 0, 0)
                .is_err()
        );
    }
}
