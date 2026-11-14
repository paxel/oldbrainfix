//! Notes and their Sections as the Dart core's `Notes` reads and writes
//! them: the text, the capture time, the Importance, where a Note is
//! filed, whether it is in the trash, what was merged into it, and its
//! photos and voice recordings with their Photo Texts and Transcripts,
//! the files attached to it with their Attachment Texts (ADR 0047),
//! and the Capture Context of Notes and Sections. Every write appends the same entries, in the same order and read from the
//! clock as often, as the Dart core's, so both logs come out alike.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use chrono::{DateTime, Duration, Utc};

use crate::boxes::{self, Boxes};
use crate::entry::{Entry, dart_iso, parse_time};
use crate::file_text::{self, FileKind};
use crate::named_places::{Coordinates, NamedPlace, NamedPlaces};
use crate::places::Places;
use crate::profile::Profile;
use crate::store::{Fields, Store};
use crate::text_boxes::{self, ShownBox};
use crate::text_format::{self, StoredText};
use crate::transcript_prompts::dart_trim;
use crate::{Error, Result};

/// Field keys of a Note.
pub mod keys {
    pub const TYPE: &str = "$type";
    pub const TYPE_NOTE: &str = "note";
    pub const TEXT: &str = "text";
    pub const CAPTURED: &str = "captured";
    /// The Area or Topic the Note is filed in; absent while Unsorted.
    pub const PLACE: &str = "place";
    /// The Note's Importance by name; absent means normal.
    pub const IMPORTANCE: &str = "importance";
    /// When the Note was moved to the trash; absent while it is not there.
    pub const TRASHED: &str = "trashed";
    /// The Note this one was merged into; absent while it stands alone,
    /// null once its Merge was undone.
    pub const MERGED: &str = "merged";
    /// When the Note first spawned a Routine or Reminder.
    pub const SPAWNED: &str = "spawned";
    /// When the Note's Nudge is due, chosen at Capture.
    pub const NUDGE: &str = "nudge";

    /// One field per photo, `photo:<hash>` = [`ADDED`], so photos added on
    /// two Devices at once are both kept. The bytes are in the store's
    /// photos, under the hash. Null once the photo was removed and its
    /// Photo Text kept (ADR 0020).
    pub const PHOTO_PREFIX: &str = "photo:";
    /// One field per voice recording, `audio:<hash>` = [`ADDED`], as for
    /// photos. The bytes are in the store's audio, under the hash. Null
    /// once the recording was removed and its Transcript kept (ADR 0020).
    pub const AUDIO_PREFIX: &str = "audio:";
    /// The value of a photo's or a recording's field.
    pub const ADDED: &str = "added";
    /// The Transcript of a voice recording, `transcript:<hash>` = its
    /// text, on the Note holding `audio:<hash>`. An empty text means
    /// Whisper heard no words; absent means not transcribed (yet); null
    /// means removed while the recording stays (ADR 0025).
    pub const TRANSCRIPT_PREFIX: &str = "transcript:";
    /// The Photo Text of a photo, `ocr:<hash>` = the text RustO recognised
    /// in it, on the Note holding `photo:<hash>`. An empty text means no
    /// text was found; absent means not read (yet); null means removed
    /// while the photo stays (ADR 0025).
    pub const OCR_PREFIX: &str = "ocr:";
    /// One field per attached file (ADR 0047), `file:<hash>` = its name,
    /// media type and size as JSON ([`super::Attachment`]), so files
    /// attached on two Devices at once are both kept. The bytes are in
    /// the store's files, under the hash. Null once the file was removed
    /// and its Attachment Text kept, as for photos (ADR 0020).
    pub const FILE_PREFIX: &str = "file:";
    /// The Attachment Text of an attached file, `fileText:<hash>` = the
    /// text read out of it, on the Note holding `file:<hash>`. An empty
    /// text means no text was found; absent means not read (yet); null
    /// means removed while the file stays (ADR 0025).
    pub const FILE_TEXT_PREFIX: &str = "fileText:";
    /// Where a Note or Section was captured, `<latitude>,<longitude>`;
    /// absent while unknown.
    pub const LOCATION: &str = "location";
    /// The app something was shared in from, its package and its label;
    /// absent for anything not shared in, or when the platform did not
    /// say.
    pub const SOURCE_PACKAGE: &str = "sourcePackage";
    pub const SOURCE_LABEL: &str = "sourceLabel";
    /// The titles of the calendar events running at Capture, as a JSON
    /// list; absent while unknown or off.
    pub const CALENDAR: &str = "calendar";
    /// The names of the Bluetooth devices connected at Capture, as a JSON
    /// list; absent while unknown or off.
    pub const BLUETOOTH: &str = "bluetooth";

    pub fn photo(hash: &str) -> String {
        format!("{PHOTO_PREFIX}{hash}")
    }

    pub fn audio(hash: &str) -> String {
        format!("{AUDIO_PREFIX}{hash}")
    }

    pub fn transcript(hash: &str) -> String {
        format!("{TRANSCRIPT_PREFIX}{hash}")
    }

    pub fn ocr(hash: &str) -> String {
        format!("{OCR_PREFIX}{hash}")
    }

    pub fn file(hash: &str) -> String {
        format!("{FILE_PREFIX}{hash}")
    }

    pub fn file_text(hash: &str) -> String {
        format!("{FILE_TEXT_PREFIX}{hash}")
    }

    /// Field keys of a Section.
    pub const TYPE_SECTION: &str = "section";
    /// The Note a Section belongs to.
    pub const NOTE: &str = "note";
}

/// How much a Note matters (CONTEXT.md).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum Importance {
    Unimportant,
    #[default]
    Normal,
    Important,
}

impl Importance {
    /// The name the log stores.
    pub fn name(self) -> &'static str {
        match self {
            Importance::Unimportant => "unimportant",
            Importance::Normal => "normal",
            Importance::Important => "important",
        }
    }

    /// The Importance stored as `name`; normal for anything else.
    pub fn from_name(name: Option<&str>) -> Importance {
        match name {
            Some("unimportant") => Importance::Unimportant,
            Some("important") => Importance::Important,
            _ => Importance::Normal,
        }
    }
}

/// What a Note without words is titled as where only its words fit, as
/// in a row or an alarm: a photo, a voice recording, or a file, which
/// goes by its name ([`Note::wordless_name`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Wordless {
    #[default]
    Photo,
    VoiceRecording,
    File,
}

impl Wordless {
    /// Its name, as the Dart core's enum has it.
    pub fn name(self) -> &'static str {
        match self {
            Wordless::Photo => "photo",
            Wordless::VoiceRecording => "voiceRecording",
            Wordless::File => "file",
        }
    }
}

/// The app something was shared in from (CONTEXT.md, Capture Context).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceApp {
    /// Its Android package, such as `com.whatsapp`.
    pub package: String,
    /// Its name as the Device showed it at capture time, or `None` when
    /// the Device could not tell.
    pub label: Option<String>,
}

impl SourceApp {
    /// What to show: the label, else the package.
    pub fn name(&self) -> &str {
        match self.label.as_deref().map(dart_trim) {
            Some(label) if !label.is_empty() => label,
            _ => &self.package,
        }
    }
}

/// What was true when a Note or Section was captured (CONTEXT.md),
/// beyond the time, which is its `captured`: where it was, the app it
/// was shared in from, the calendar events running and the Bluetooth
/// devices connected. Notes and Sections keep it under the same keys.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CaptureContext {
    /// Where it was captured, or `None` when that is unknown: denied,
    /// unavailable, not arrived yet, or not readable.
    pub location: Option<Coordinates>,
    /// The Named Place `location` lies in, resolved when read, so a
    /// renamed, deleted or new Named Place shows on Notes captured before.
    pub named_place: Option<NamedPlace>,
    /// The app it was shared in from, or `None` when it was not shared in
    /// or the platform did not say.
    pub source_app: Option<SourceApp>,
    /// The titles of the calendar events running at capture; empty when
    /// none, unknown or off.
    pub calendar_events: Vec<String>,
    /// The names of the Bluetooth devices connected at capture; empty
    /// when none, unknown or off.
    pub bluetooth_devices: Vec<String>,
}

impl CaptureContext {
    /// Whether there is more than the time and place a card shows (UI
    /// laws, "Cards").
    pub fn has_more(&self) -> bool {
        self.source_app.is_some()
            || !self.calendar_events.is_empty()
            || !self.bluetooth_devices.is_empty()
    }

    /// Whether `detail` shows in it.
    pub fn holds(&self, detail: CapturedDetail) -> bool {
        match detail {
            CapturedDetail::Place => self.location.is_some(),
            CapturedDetail::Event => !self.calendar_events.is_empty(),
            CapturedDetail::Device => !self.bluetooth_devices.is_empty(),
            CapturedDetail::App => self.source_app.is_some(),
        }
    }
}

/// A detail of a Capture Context the user can remove on its own (ADR
/// 0037), as the Dart core's `CapturedDetail`: where it was captured (the
/// coordinates, and with them the Named Place), the calendar events
/// running, the Bluetooth devices connected and the app it was shared in
/// from. The capture time is none: it stays.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CapturedDetail {
    Place,
    Event,
    Device,
    App,
}

impl CapturedDetail {
    /// Every detail, in the Dart core's order.
    pub const ALL: [CapturedDetail; 4] = [
        CapturedDetail::Place,
        CapturedDetail::Event,
        CapturedDetail::Device,
        CapturedDetail::App,
    ];

    /// Its name, as the Dart core's enum names it.
    pub fn name(self) -> &'static str {
        match self {
            CapturedDetail::Place => "place",
            CapturedDetail::Event => "event",
            CapturedDetail::Device => "device",
            CapturedDetail::App => "app",
        }
    }

    /// The detail named `name`, if there is one.
    pub fn parse(name: &str) -> Option<CapturedDetail> {
        Self::ALL.into_iter().find(|d| d.name() == name)
    }

    /// The fields it is kept under, on a Note and a Section alike; the
    /// app's label goes with its package.
    fn fields(self) -> &'static [&'static str] {
        match self {
            CapturedDetail::Place => &[keys::LOCATION],
            CapturedDetail::Event => &[keys::CALENDAR],
            CapturedDetail::Device => &[keys::BLUETOOTH],
            CapturedDetail::App => &[keys::SOURCE_PACKAGE, keys::SOURCE_LABEL],
        }
    }
}

/// A Note as the screens want it (CONTEXT.md).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    /// Entity id, `note:<id>`.
    pub id: String,
    /// The text, the source text when its format is not known here.
    pub text: String,
    /// The format `text` is in ([`crate::text_format`]).
    pub text_format: String,
    /// The version of the structure `text` is stored in
    /// ([`StoredText`]).
    pub text_version: i64,
    /// Whether this version may change `text`: never one in a format it
    /// does not know, which it shows as its source text (ADR 0018).
    pub text_editable: bool,
    pub captured: DateTime<Utc>,
    /// Where it was captured and what else was true then.
    pub context: CaptureContext,
    /// The Area or Topic the Note is filed in, or `None` when it has
    /// none or that place is gone.
    pub place: Option<String>,
    pub importance: Importance,
    /// When the Note was moved to the trash.
    pub trashed: Option<DateTime<Utc>>,
    /// The Note this one was merged into and now is a Section of.
    pub merged_into: Option<String>,
    /// Whether the Note ever spawned a Routine or Reminder.
    pub spawned: bool,
    /// When its Nudge is due, as chosen at Capture; `None` when none was.
    pub nudge: Option<DateTime<Utc>>,
    /// The hashes of its photos, in the order they were added.
    pub photos: Vec<String>,
    /// The hashes of its voice recordings, in the order they were added.
    pub recordings: Vec<String>,
    /// The Transcripts of its voice recordings, by recording hash; a
    /// recording not transcribed yet has none.
    pub transcripts: BTreeMap<String, String>,
    /// The Photo Texts of its photos, by photo hash; a photo not read yet
    /// has none.
    pub photo_texts: BTreeMap<String, String>,
    /// The hashes of its photos that were removed while their Photo Text
    /// stays, in the order they were added; the text is in
    /// `photo_texts`.
    pub removed_photos: Vec<String>,
    /// The hashes of its voice recordings that were removed while their
    /// Transcript stays, in the order they were added; the text is in
    /// `transcripts`.
    pub removed_recordings: Vec<String>,
    /// The hashes of its voice recordings whose Transcript was removed
    /// while the recording stays, in the order they were added; they are
    /// not transcribed again unless asked to (ADR 0025).
    pub removed_transcripts: Vec<String>,
    /// The hashes of its photos whose Photo Text was removed while the
    /// photo stays, as for `removed_transcripts`.
    pub removed_photo_texts: Vec<String>,
    /// The files attached to it (ADR 0047), in the order they were
    /// attached.
    pub attachments: Vec<Attachment>,
    /// The Attachment Texts of its files, by file hash; a file not read
    /// yet, or of a kind without text, has none.
    pub attachment_texts: BTreeMap<String, String>,
    /// Its files that were removed while their Attachment Text stays, in
    /// the order they were attached, as they were before; the text is in
    /// `attachment_texts`.
    pub removed_attachments: Vec<Attachment>,
    /// The hashes of its files whose Attachment Text was removed while
    /// the file stays, as for `removed_photo_texts`.
    pub removed_attachment_texts: Vec<String>,
    /// The boxes of `text` as they show now, in the order they come (ADR
    /// 0035); none in a text this version may not change.
    pub boxes: Vec<ShownBox>,
}

impl Note {
    /// `text` as it shows: each box in the glyph of its current state,
    /// without its id; what `boxes` count their offsets in, and what is
    /// copied, shared and saved (ADR 0035). `text` as it is in a format
    /// not known here.
    pub fn shown_text(&self) -> String {
        shown_text(&self.text, self.text_editable, &self.boxes)
    }

    /// [`Note::shown_text`] as it reads without its marks, for search and
    /// previews.
    pub fn visible_text(&self) -> String {
        text_format::visible(&self.text_format, &self.shown_text())
    }

    pub fn is_trashed(&self) -> bool {
        self.trashed.is_some()
    }

    pub fn is_merged(&self) -> bool {
        self.merged_into.is_some()
    }

    /// Not filed, not merged, not deleted and without a Routine or
    /// Reminder spawned from it (CONTEXT.md).
    pub fn is_unsorted(&self) -> bool {
        self.place.is_none() && !self.is_trashed() && !self.is_merged() && !self.spawned
    }

    /// When the Nudge for this Note is due: as chosen at Capture, else
    /// [`Notes::default_nudge`] after it.
    pub fn nudge_at(&self) -> DateTime<Utc> {
        self.nudge.unwrap_or(self.captured + Notes::default_nudge())
    }

    /// The file a Note made from a file alone shows as its card (ADR
    /// 0047): its one attached file, while its text is blank and it has
    /// no photo, recording or other file, removed ones included.
    pub fn alone_file(&self) -> Option<&Attachment> {
        let [file] = self.attachments.as_slice() else {
            return None;
        };
        let alone = dart_trim(&self.shown_text()).is_empty()
            && self.photos.is_empty()
            && self.removed_photos.is_empty()
            && self.recordings.is_empty()
            && self.removed_recordings.is_empty()
            && self.removed_attachments.is_empty();
        alone.then_some(file)
    }

    /// What the Note is titled as while its text is blank, removed
    /// photos, recordings and files whose texts stay included: a photo
    /// when it has one; else a voice recording when it has one; else a
    /// file when it has one, by [`Note::wordless_name`]; a photo
    /// otherwise.
    pub fn wordless(&self) -> Wordless {
        if !(self.photos.is_empty() && self.removed_photos.is_empty()) {
            Wordless::Photo
        } else if !(self.recordings.is_empty() && self.removed_recordings.is_empty()) {
            Wordless::VoiceRecording
        } else if !(self.attachments.is_empty() && self.removed_attachments.is_empty()) {
            Wordless::File
        } else {
            Wordless::Photo
        }
    }

    /// The name a Note titled [`Wordless::File`] goes by (#267): its
    /// first attached file's, else its first removed one's; empty when
    /// that file has no name, or for any other Note.
    pub fn wordless_name(&self) -> String {
        if self.wordless() != Wordless::File {
            return String::new();
        }
        self.attachments
            .first()
            .or_else(|| self.removed_attachments.first())
            .map(|a| a.name.clone())
            .unwrap_or_default()
    }
}

/// A dated part of a Note, added by Amend or Merge (CONTEXT.md).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    /// `section:<id>` for an Amend, the merged Note's `note:<id>` for a
    /// Merge.
    pub id: String,
    pub note_id: String,
    /// The text, the source text when its format is not known here.
    pub text: String,
    /// The format `text` is in, the version of its structure and whether
    /// this version may change it, as for [`Note::text_format`],
    /// [`Note::text_version`] and [`Note::text_editable`].
    pub text_format: String,
    pub text_version: i64,
    pub text_editable: bool,
    pub captured: DateTime<Utc>,
    /// Where it was captured and what else was true then; a merged
    /// Note's own.
    pub context: CaptureContext,
    /// A merged Note's photos and recordings travel with it, with their
    /// Photo Texts and Transcripts; an Amend has none.
    pub photos: Vec<String>,
    pub recordings: Vec<String>,
    pub transcripts: BTreeMap<String, String>,
    pub photo_texts: BTreeMap<String, String>,
    /// The photos and voice recordings removed while their texts stay,
    /// as for [`Note::removed_photos`] and [`Note::removed_recordings`].
    pub removed_photos: Vec<String>,
    pub removed_recordings: Vec<String>,
    /// The recordings and photos whose texts were removed while they
    /// stay, as for [`Note::removed_transcripts`] and
    /// [`Note::removed_photo_texts`].
    pub removed_transcripts: Vec<String>,
    pub removed_photo_texts: Vec<String>,
    /// A merged Note's attached files with their Attachment Texts, those
    /// removed and those whose texts were removed, as for
    /// [`Note::attachments`]; an Amend has none.
    pub attachments: Vec<Attachment>,
    pub attachment_texts: BTreeMap<String, String>,
    pub removed_attachments: Vec<Attachment>,
    pub removed_attachment_texts: Vec<String>,
    /// The boxes of `text` as they show now, as for [`Note::boxes`].
    pub boxes: Vec<ShownBox>,
    /// When `text` was last edited, as the Dart core's `modified` (#183):
    /// the date of the text that shows, once more than the text it was
    /// captured with is kept; `None` while it was never edited. Whatever
    /// date it shows, [`Section::created`] stays.
    pub modified: Option<DateTime<Utc>>,
    /// Who edited `text` last, as their Profile shows
    /// ([`Store::editor_of`]): the user's own Profile for an edit on one
    /// of their Devices; `None` while it was never edited (`modified` is
    /// `None`) and for a writer not known.
    pub edited_by: Option<Profile>,
}

/// An error unless `file` is small enough to attach.
fn require_size(file: &NewFile) -> Result<()> {
    if file.bytes.len() as u64 > MAX_FILE_BYTES {
        return Err(Error::Invalid(format!(
            "{} is larger than {MAX_FILE_BYTES} bytes",
            file.name
        )));
    }
    Ok(())
}

/// An error unless `format` is a format this version writes.
fn require_format(format: &str) -> Result<()> {
    if text_format::KNOWN.contains(&format) {
        Ok(())
    } else {
        Err(Error::Invalid(format!("not a format known here: {format}")))
    }
}

impl Section {
    /// When the Section was captured, by Amend or as the merged Note; an
    /// edit of its text leaves it as it is.
    pub fn created(&self) -> DateTime<Utc> {
        self.captured
    }

    /// `text` as it shows, as for [`Note::shown_text`].
    pub fn shown_text(&self) -> String {
        shown_text(&self.text, self.text_editable, &self.boxes)
    }

    /// [`Section::shown_text`] as it reads without its marks, as for
    /// [`Note::visible_text`].
    pub fn visible_text(&self) -> String {
        text_format::visible(&self.text_format, &self.shown_text())
    }
}

/// `text` with each of `boxes` in the glyph of its state and no ids,
/// when `editable`; else as it is.
fn shown_text(text: &str, editable: bool, boxes: &[ShownBox]) -> String {
    if !editable {
        return text.to_string();
    }
    if boxes.is_empty() {
        return text_boxes::strip(text);
    }
    let ticks = boxes
        .iter()
        .filter_map(|b| Some((b.id.clone(), b.last.clone()?)))
        .collect();
    text_boxes::shown(text, &ticks).text
}

/// A Note's photos or recordings, their texts and those removed.
type Waiting<'n> = (
    &'n Vec<String>,
    &'n BTreeMap<String, String>,
    &'n Vec<String>,
);

/// A photo or a voice recording of a Note, by its hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Blob {
    pub note_id: String,
    pub hash: String,
}

/// The largest file that can be attached: the most a bundle, a Backup
/// and the local network carry of one (ADR 0003, 0047).
pub const MAX_FILE_BYTES: u64 = 256 << 20;

/// The largest whole number a size or a page count is read as: what a
/// double holds exactly, so both cores read the same.
const MAX_COUNT: u64 = 1 << 53;

/// The most characters of a file's name kept.
const MAX_NAME_CHARS: usize = 255;

/// The media type of a file no app named one for.
pub const UNKNOWN_TYPE: &str = "application/octet-stream";

/// A file attached to a Note (ADR 0047): its bytes are a blob, kept by
/// its hash; the rest is what the attaching Device wrote down. Another
/// person's Device may have written it, so it is read as data only: a
/// name without folders or control characters, a media type of the
/// usual shape, numbers that are whole.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attachment {
    pub hash: String,
    /// Its file name, without any folder; empty when none was given.
    pub name: String,
    /// Its media type in lower case, such as `application/pdf`;
    /// [`UNKNOWN_TYPE`] when none was given.
    pub media_type: String,
    /// Its size in bytes.
    pub size: u64,
    /// How many pages it has, when the attaching Device counted them.
    pub pages: Option<u64>,
    /// Whether only a computer reads its text (Q645,
    /// [`file_text::computer_reads`]); until one has, a phone shows that
    /// the text follows from the computer.
    pub computer: bool,
}

impl Attachment {
    /// What the file is, as far as its text goes.
    pub fn kind(&self) -> FileKind {
        file_text::kind_of(&self.name, &self.media_type)
    }

    /// The attachment `hash` from the value of its field: whatever does
    /// not read is left empty, never an error.
    pub fn read(hash: &str, value: &str) -> Attachment {
        let json = serde_json::from_str::<serde_json::Value>(value).ok();
        let object = json.as_ref().and_then(serde_json::Value::as_object);
        let field = |key: &str| object.and_then(|o| o.get(key));
        let count = |key: &str| {
            field(key)
                .and_then(serde_json::Value::as_u64)
                .filter(|n| *n <= MAX_COUNT)
        };
        Attachment {
            hash: hash.to_string(),
            name: clean_name(field("name").and_then(|v| v.as_str()).unwrap_or("")),
            media_type: clean_type(field("type").and_then(|v| v.as_str()).unwrap_or("")),
            size: count("size").unwrap_or(0),
            pages: count("pages"),
            computer: field("computer") == Some(&serde_json::Value::Bool(true)),
        }
    }

    /// The value of its field: JSON, keys sorted, `computer` only when
    /// true and `pages` only when counted.
    pub fn value(&self) -> String {
        let mut out = serde_json::Map::new();
        if self.computer {
            out.insert("computer".into(), true.into());
        }
        out.insert("name".into(), self.name.clone().into());
        if let Some(pages) = self.pages {
            out.insert("pages".into(), pages.into());
        }
        out.insert("size".into(), self.size.into());
        out.insert("type".into(), self.media_type.clone().into());
        serde_json::Value::Object(out).to_string()
    }
}

/// A file to attach: its name, the media type the app was told (empty,
/// or `application/octet-stream`, for the one its name's ending says,
/// [`file_text::media_type_of`]), its bytes and, for a PDF, how many
/// pages it has when the app counted them ([`crate::pdf::page_count`] on
/// the desktop).
#[derive(Debug, Clone, Copy)]
pub struct NewFile<'b> {
    pub name: &'b str,
    pub media_type: &'b str,
    pub bytes: &'b [u8],
    pub pages: Option<u64>,
}

/// `name` as a file's name is kept: the part after its last `/` or `\`,
/// without control characters and the marks that turn the direction of
/// text (which could make `exe.pdf` of `fdp.exe`), its blanks at the
/// ends trimmed, at most [`MAX_NAME_CHARS`] characters; `.` and `..` are
/// no name.
pub fn clean_name(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let kept: String = base
        .chars()
        .filter(|&c| {
            !(c < ' '
                || ('\u{7f}'..='\u{9f}').contains(&c)
                || ('\u{202a}'..='\u{202e}').contains(&c)
                || ('\u{2066}'..='\u{2069}').contains(&c)
                || c == '\u{200e}'
                || c == '\u{200f}')
        })
        .collect();
    let trimmed: String = dart_trim(&kept).chars().take(MAX_NAME_CHARS).collect();
    let trimmed = dart_trim(&trimmed);
    if trimmed == "." || trimmed == ".." {
        String::new()
    } else {
        trimmed.to_string()
    }
}

/// `media_type` as a file's media type is kept: in lower case, without
/// its parameters, when it has the shape `type/subtype` of letters,
/// digits and `!#$&^_.+-`, at most 127 characters; [`UNKNOWN_TYPE`]
/// otherwise.
pub fn clean_type(media_type: &str) -> String {
    let bare = media_type.split(';').next().unwrap_or("");
    let lower = dart_trim(bare).to_ascii_lowercase();
    let part = |p: &str| {
        !p.is_empty()
            && p.bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"!#$&^_.+-".contains(&b))
    };
    match lower.split_once('/') {
        Some((top, sub)) if lower.len() <= 127 && part(top) && part(sub) => lower,
        _ => UNKNOWN_TYPE.to_string(),
    }
}

/// Reading Notes.
pub struct Notes<'a> {
    store: &'a Store,
}

impl<'a> Notes<'a> {
    /// How long a deleted Note stays in the trash before it is purged.
    pub fn trash_keep() -> Duration {
        Duration::days(30)
    }

    /// How long after Capture the Nudge comes when none was picked (UI
    /// laws, "Capture").
    pub fn default_nudge() -> Duration {
        Duration::hours(1)
    }

    pub fn new(store: &'a Store) -> Notes<'a> {
        Notes { store }
    }

    /// Captures a new Note at the store's now with `text`, filed into the
    /// Area or Topic `place` when one is given, nudged `nudge_in` from now
    /// while it is Unsorted. The text is kept as given.
    pub fn capture(
        &self,
        text: &str,
        place: Option<&str>,
        nudge_in: Option<Duration>,
    ) -> Result<Note> {
        self.capture_full(text, text_format::PLAIN, place, nudge_in, &[], &[], &[])
    }

    /// Captures a new Note as [`Notes::capture`] does, its text in
    /// `format`.
    pub fn capture_as(
        &self,
        text: &str,
        format: &str,
        place: Option<&str>,
        nudge_in: Option<Duration>,
    ) -> Result<Note> {
        self.capture_full(text, format, place, nudge_in, &[], &[], &[])
    }

    /// Captures a new Note as [`Notes::capture`] does, with `photos` and
    /// voice `recordings`; the text may be empty for a photo or voice
    /// Note. The photos are kept without their metadata.
    pub fn capture_with(
        &self,
        text: &str,
        place: Option<&str>,
        nudge_in: Option<Duration>,
        photos: &[Vec<u8>],
        recordings: &[Vec<u8>],
    ) -> Result<Note> {
        self.capture_full(
            text,
            text_format::PLAIN,
            place,
            nudge_in,
            photos,
            recordings,
            &[],
        )
    }

    /// Captures a new Note as [`Notes::capture_with`] does, with `files`
    /// attached (ADR 0047): the text may be empty for a Note made from a
    /// file alone, which shows the file as its card
    /// ([`Note::alone_file`]). The files are kept as they are; one larger
    /// than [`MAX_FILE_BYTES`] is refused and nothing is written.
    pub fn capture_with_files(
        &self,
        text: &str,
        place: Option<&str>,
        nudge_in: Option<Duration>,
        photos: &[Vec<u8>],
        recordings: &[Vec<u8>],
        files: &[NewFile],
    ) -> Result<Note> {
        self.capture_full(
            text,
            text_format::PLAIN,
            place,
            nudge_in,
            photos,
            recordings,
            files,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn capture_full(
        &self,
        text: &str,
        format: &str,
        place: Option<&str>,
        nudge_in: Option<Duration>,
        photos: &[Vec<u8>],
        recordings: &[Vec<u8>],
        files: &[NewFile],
    ) -> Result<Note> {
        require_format(format)?;
        for file in files {
            require_size(file)?;
        }
        if let Some(place) = place
            && !Places::new(self.store).is_place(place)?
        {
            return Err(Error::Invalid(format!("not an Area or Topic: {place}")));
        }
        if let Some(place) = place
            && !self.store.may_write_in(place)?
        {
            return Err(Error::Invalid(format!(
                "no role in the Shared Area of {place}"
            )));
        }
        let id = format!("note:{}", self.store.new_id()?);
        let now = self.store.now();
        let at = Some(now);
        // The bytes are kept before the log names them.
        let photos = photos
            .iter()
            .map(|p| self.store.put_photo(p))
            .collect::<Result<Vec<_>>>()?;
        let audio = recordings
            .iter()
            .map(|r| self.store.put_audio(r))
            .collect::<Result<Vec<_>>>()?;
        let files = files
            .iter()
            .map(|f| self.keep_file(f))
            .collect::<Result<Vec<_>>>()?;
        self.store.batch(|| {
            let s = self.store;
            s.append(&id, keys::TYPE, Some(keys::TYPE_NOTE), at)?;
            s.append(&id, keys::CAPTURED, Some(&dart_iso(now)), at)?;
            let text = StoredText::of(format, &Self::boxed(s, text, format)?, text_format::VERSION)
                .encode();
            s.append(&id, keys::TEXT, Some(&text), at)?;
            if let Some(place) = place {
                s.append(&id, keys::PLACE, Some(place), at)?;
            }
            if let Some(nudge_in) = nudge_in {
                s.append(&id, keys::NUDGE, Some(&dart_iso(now + nudge_in)), at)?;
            }
            for hash in &photos {
                s.append(&id, &keys::photo(hash), Some(keys::ADDED), at)?;
            }
            for hash in &audio {
                s.append(&id, &keys::audio(hash), Some(keys::ADDED), at)?;
            }
            for file in &files {
                s.append(&id, &keys::file(&file.hash), Some(&file.value()), at)?;
            }
            Ok(())
        })?;
        self.by_id(&id)?
            .ok_or_else(|| Error::Invalid("the captured Note is gone".into()))
    }

    /// Adds the photo `bytes` to the Note `note_id`, without their
    /// metadata; returns the hash it is kept under.
    pub fn add_photo(&self, note_id: &str, bytes: &[u8]) -> Result<String> {
        self.require_note(note_id)?;
        let hash = self.store.put_photo(bytes)?;
        self.store
            .append(note_id, &keys::photo(&hash), Some(keys::ADDED), None)?;
        Ok(hash)
    }

    /// The bytes of the photo `hash`, or `None` when they are not on this
    /// Device (yet).
    pub fn photo_bytes(&self, hash: &str) -> Option<Vec<u8>> {
        self.store.photo(hash)
    }

    /// Whether the photo `hash` is on this Device, without reading it.
    pub fn has_photo(&self, hash: &str) -> bool {
        self.store.has_blob("photo", hash)
    }

    /// Adds the voice recording `bytes` to the Note `note_id`; returns
    /// the hash it is kept under.
    pub fn add_recording(&self, note_id: &str, bytes: &[u8]) -> Result<String> {
        self.require_note(note_id)?;
        let hash = self.store.put_audio(bytes)?;
        self.store
            .append(note_id, &keys::audio(&hash), Some(keys::ADDED), None)?;
        Ok(hash)
    }

    /// The bytes of the voice recording `hash`, or `None` when they are
    /// not on this Device (yet).
    pub fn recording_bytes(&self, hash: &str) -> Option<Vec<u8>> {
        self.store.audio(hash)
    }

    /// Whether the voice recording `hash` is on this Device, without
    /// reading it.
    pub fn has_recording(&self, hash: &str) -> bool {
        self.store.has_blob("audio", hash)
    }

    /// Attaches the file `file` to the Note `note_id` (ADR 0047), its
    /// bytes kept as they are; returns the hash they are kept under. A
    /// file larger than [`MAX_FILE_BYTES`] is refused. The same file
    /// attached again keeps one field, written anew.
    pub fn attach_file(&self, note_id: &str, file: &NewFile) -> Result<String> {
        self.require_note(note_id)?;
        require_size(file)?;
        let attachment = self.keep_file(file)?;
        self.store.append(
            note_id,
            &keys::file(&attachment.hash),
            Some(&attachment.value()),
            None,
        )?;
        Ok(attachment.hash)
    }

    /// Keeps the bytes of `file` and gives what its field holds.
    fn keep_file(&self, file: &NewFile) -> Result<Attachment> {
        let hash = self.store.put_file(file.bytes)?;
        let name = clean_name(file.name);
        // A type not named is the one its name's ending says.
        let media_type = match clean_type(file.media_type) {
            t if t == UNKNOWN_TYPE => file_text::media_type_of(&name).to_string(),
            t => t,
        };
        let size = file.bytes.len() as u64;
        let pages = file.pages.filter(|p| *p <= MAX_COUNT);
        let kind = file_text::kind_of(&name, &media_type);
        Ok(Attachment {
            hash,
            computer: file_text::computer_reads(kind, size, pages),
            name,
            media_type,
            size,
            pages,
        })
    }

    /// The bytes of the attached file `hash`, or `None` when they are not
    /// on this Device (yet).
    pub fn file_bytes(&self, hash: &str) -> Option<Vec<u8>> {
        self.store.file(hash)
    }

    /// Whether the attached file `hash` is on this Device, without
    /// reading it.
    pub fn has_file(&self, hash: &str) -> bool {
        self.store.has_blob("file", hash)
    }

    /// Keeps `text`, tidied ([`file_text::tidy`]), as the Attachment Text
    /// of the file `hash` of the Note `note_id`, a merged one included,
    /// next to the file. An empty text records that no text was found,
    /// so the file is not read again. Given again, as when reading the
    /// file again, it replaces the Attachment Text, a removed one
    /// included.
    pub fn set_attachment_text(&self, note_id: &str, hash: &str, text: &str) -> Result<()> {
        self.require_attached(note_id, hash)?;
        self.store
            .append(
                note_id,
                &keys::file_text(hash),
                Some(&file_text::tidy(text)),
                None,
            )
            .map(drop)
    }

    /// The attached files whose text is still to be read, newest capture
    /// first: those of a kind with text ([`FileKind::has_text`]) without
    /// an Attachment Text, nor one that was removed, on every Note not in
    /// the trash, merged ones included. On a phone (`computer` false)
    /// those only a computer reads are left out (Q645).
    pub fn unread_attachments(&self, computer: bool) -> Result<Vec<Blob>> {
        let waits = |note: &Note, file: &Attachment| {
            file.kind().has_text()
                && (computer || !file.computer)
                && !note.attachment_texts.contains_key(&file.hash)
                && !note.removed_attachment_texts.contains(&file.hash)
        };
        let mut notes = Vec::new();
        for id in self.store.entities_of_type(keys::TYPE_NOTE)? {
            if let Some(note) = self.by_id(&id)?
                && !note.is_trashed()
                && note.attachments.iter().any(|a| waits(&note, a))
            {
                notes.push(note);
            }
        }
        // By id within one capture time, so both cores list them alike.
        notes.sort_by(|a, b| b.captured.cmp(&a.captured).then_with(|| a.id.cmp(&b.id)));
        let mut out = Vec::new();
        for note in &notes {
            for file in &note.attachments {
                if waits(note, file) {
                    out.push(Blob {
                        note_id: note.id.clone(),
                        hash: file.hash.clone(),
                    });
                }
            }
        }
        Ok(out)
    }

    /// Whether this Device may remove the attached file `hash` of the
    /// Note `note_id`, a merged one included, and keep its Attachment
    /// Text, as for [`Notes::may_remove_photo`]: the file has one that
    /// is not empty, and the Note is this Device's to change.
    pub fn may_remove_attachment(&self, note_id: &str, hash: &str) -> Result<bool> {
        let Some(note) = self.by_id(note_id)? else {
            return Ok(false);
        };
        if !note.attachments.iter().any(|a| a.hash == hash)
            || !has_text(note.attachment_texts.get(hash))
        {
            return Ok(false);
        }
        self.store.may_change(note_id)
    }

    /// Removes the attached file `hash` from the Note `note_id` and keeps
    /// its Attachment Text, which search still finds, as
    /// [`Notes::remove_photo`] does (ADR 0020). Refused unless
    /// [`Notes::may_remove_attachment`].
    pub fn remove_attachment(&self, note_id: &str, hash: &str) -> Result<()> {
        self.require_attached(note_id, hash)?;
        if !self.may_remove_attachment(note_id, hash)? {
            return Err(Error::Invalid(format!(
                "the file {hash} of {note_id} may not be removed here"
            )));
        }
        self.store.append(note_id, &keys::file(hash), None, None)?;
        self.store.drop_removed_blobs().map(drop)
    }

    /// Whether this Device may remove the Attachment Text of the file
    /// `hash` of the Note `note_id` and keep the file, as for
    /// [`Notes::may_remove_photo_text`].
    pub fn may_remove_attachment_text(&self, note_id: &str, hash: &str) -> Result<bool> {
        self.may_remove_attachment(note_id, hash)
    }

    /// Removes the Attachment Text of the file `hash` from the Note
    /// `note_id`; the file stays (ADR 0025). Search no longer finds the
    /// text, and the file is not read again until
    /// [`Notes::set_attachment_text`] is given a new one. Refused unless
    /// [`Notes::may_remove_attachment_text`].
    pub fn remove_attachment_text(&self, note_id: &str, hash: &str) -> Result<()> {
        self.require_attached(note_id, hash)?;
        if !self.may_remove_attachment_text(note_id, hash)? {
            return Err(Error::Invalid(format!(
                "the Attachment Text of {hash} may not be removed here"
            )));
        }
        self.store
            .append(note_id, &keys::file_text(hash), None, None)
            .map(drop)
    }

    /// An error unless `hash` is a file attached to the Note `note_id`.
    fn require_attached(&self, note_id: &str, hash: &str) -> Result<()> {
        let held = self
            .by_id(note_id)?
            .is_some_and(|n| n.attachments.iter().any(|a| a.hash == hash));
        if held {
            Ok(())
        } else {
            Err(Error::Invalid(format!("not a file of {note_id}: {hash}")))
        }
    }

    /// Keeps `text`, trimmed, as the Transcript of the voice recording
    /// `hash` of the Note `note_id`, a merged one included, next to the
    /// recording. An empty text records that no words were heard, so the
    /// recording is not transcribed again. Given again, as when
    /// transcribing again, it replaces the Transcript, a removed one
    /// included.
    pub fn set_transcript(&self, note_id: &str, hash: &str, text: &str) -> Result<()> {
        let held = self
            .by_id(note_id)?
            .is_some_and(|n| n.recordings.iter().any(|h| h == hash));
        if !held {
            return Err(Error::Invalid(format!(
                "not a recording of {note_id}: {hash}"
            )));
        }
        self.store
            .append(note_id, &keys::transcript(hash), Some(text.trim()), None)
            .map(drop)
    }

    /// Keeps `text`, trimmed, as the Photo Text of the photo `hash` of
    /// the Note `note_id`, a merged one included, next to the photo. An
    /// empty text records that no text was found, so the photo is not
    /// read again. Given again, as when reading the text again, it
    /// replaces the Photo Text, a removed one included.
    pub fn set_photo_text(&self, note_id: &str, hash: &str, text: &str) -> Result<()> {
        let held = self
            .by_id(note_id)?
            .is_some_and(|n| n.photos.iter().any(|h| h == hash));
        if !held {
            return Err(Error::Invalid(format!("not a photo of {note_id}: {hash}")));
        }
        self.store
            .append(note_id, &keys::ocr(hash), Some(text.trim()), None)
            .map(drop)
    }

    /// The voice recordings without a Transcript, newest capture first:
    /// those of every Note not in the trash, merged ones included. One
    /// whose Transcript was removed is not among them.
    pub fn untranscribed(&self) -> Result<Vec<Blob>> {
        self.waiting(|n| (&n.recordings, &n.transcripts, &n.removed_transcripts))
    }

    /// The photos without a Photo Text, newest capture first: those of
    /// every Note not in the trash, merged ones included. One whose Photo
    /// Text was removed is not among them.
    pub fn unread_photos(&self) -> Result<Vec<Blob>> {
        self.waiting(|n| (&n.photos, &n.photo_texts, &n.removed_photo_texts))
    }

    /// Whether this Device may remove the photo `hash` of the Note
    /// `note_id`, a merged one included, and keep its Photo Text: the
    /// photo has one that is not empty, and the Note is this Device's to
    /// change ([`Store::may_change`]).
    pub fn may_remove_photo(&self, note_id: &str, hash: &str) -> Result<bool> {
        self.may_remove(note_id, hash, |n| (&n.photos, &n.photo_texts))
    }

    /// Whether this Device may remove the voice recording `hash` of the
    /// Note `note_id`, a merged one included, and keep its Transcript, as
    /// for [`Notes::may_remove_photo`].
    pub fn may_remove_recording(&self, note_id: &str, hash: &str) -> Result<bool> {
        self.may_remove(note_id, hash, |n| (&n.recordings, &n.transcripts))
    }

    fn may_remove(
        &self,
        note_id: &str,
        hash: &str,
        of: impl Fn(&Note) -> (&Vec<String>, &BTreeMap<String, String>),
    ) -> Result<bool> {
        let Some(note) = self.by_id(note_id)? else {
            return Ok(false);
        };
        let (blobs, texts) = of(&note);
        if !blobs.iter().any(|h| h == hash) || !has_text(texts.get(hash)) {
            return Ok(false);
        }
        self.store.may_change(note_id)
    }

    /// Removes the photo `hash` from the Note `note_id` and keeps its
    /// Photo Text, which search still finds (ADR 0020). The edit syncs
    /// like any other; every Device lets the bytes go once no Note refers
    /// to them. Refused unless [`Notes::may_remove_photo`].
    pub fn remove_photo(&self, note_id: &str, hash: &str) -> Result<()> {
        let held = self
            .by_id(note_id)?
            .is_some_and(|n| n.photos.iter().any(|h| h == hash));
        if !held {
            return Err(Error::Invalid(format!("not a photo of {note_id}: {hash}")));
        }
        if !self.may_remove_photo(note_id, hash)? {
            return Err(Error::Invalid(format!(
                "the photo {hash} of {note_id} may not be removed here"
            )));
        }
        self.store.append(note_id, &keys::photo(hash), None, None)?;
        self.store.drop_removed_blobs().map(drop)
    }

    /// Removes the voice recording `hash` from the Note `note_id` and
    /// keeps its Transcript, as [`Notes::remove_photo`] does. Refused
    /// unless [`Notes::may_remove_recording`].
    pub fn remove_recording(&self, note_id: &str, hash: &str) -> Result<()> {
        let held = self
            .by_id(note_id)?
            .is_some_and(|n| n.recordings.iter().any(|h| h == hash));
        if !held {
            return Err(Error::Invalid(format!(
                "not a recording of {note_id}: {hash}"
            )));
        }
        if !self.may_remove_recording(note_id, hash)? {
            return Err(Error::Invalid(format!(
                "the recording {hash} of {note_id} may not be removed here"
            )));
        }
        self.store.append(note_id, &keys::audio(hash), None, None)?;
        self.store.drop_removed_blobs().map(drop)
    }

    /// Whether this Device may remove the Photo Text of the photo `hash`
    /// of the Note `note_id`, a merged one included, and keep the photo:
    /// the photo is there with a text that is not empty, and the Note is
    /// this Device's to change ([`Store::may_change`]).
    pub fn may_remove_photo_text(&self, note_id: &str, hash: &str) -> Result<bool> {
        self.may_remove(note_id, hash, |n| (&n.photos, &n.photo_texts))
    }

    /// Whether this Device may remove the Transcript of the voice
    /// recording `hash` of the Note `note_id` and keep the recording, as
    /// for [`Notes::may_remove_photo_text`].
    pub fn may_remove_transcript(&self, note_id: &str, hash: &str) -> Result<bool> {
        self.may_remove(note_id, hash, |n| (&n.recordings, &n.transcripts))
    }

    /// Removes the Photo Text of the photo `hash` from the Note
    /// `note_id`; the photo stays (ADR 0025). Search no longer finds the
    /// text, and the photo is not read again until
    /// [`Notes::set_photo_text`] is given a new one. Refused unless
    /// [`Notes::may_remove_photo_text`].
    pub fn remove_photo_text(&self, note_id: &str, hash: &str) -> Result<()> {
        let held = self
            .by_id(note_id)?
            .is_some_and(|n| n.photos.iter().any(|h| h == hash));
        if !held {
            return Err(Error::Invalid(format!("not a photo of {note_id}: {hash}")));
        }
        if !self.may_remove_photo_text(note_id, hash)? {
            return Err(Error::Invalid(format!(
                "the Photo Text of {hash} may not be removed here"
            )));
        }
        self.store
            .append(note_id, &keys::ocr(hash), None, None)
            .map(drop)
    }

    /// Removes the Transcript of the voice recording `hash` from the
    /// Note `note_id` and keeps the recording, as
    /// [`Notes::remove_photo_text`] does. Refused unless
    /// [`Notes::may_remove_transcript`].
    pub fn remove_transcript(&self, note_id: &str, hash: &str) -> Result<()> {
        let held = self
            .by_id(note_id)?
            .is_some_and(|n| n.recordings.iter().any(|h| h == hash));
        if !held {
            return Err(Error::Invalid(format!(
                "not a recording of {note_id}: {hash}"
            )));
        }
        if !self.may_remove_transcript(note_id, hash)? {
            return Err(Error::Invalid(format!(
                "the Transcript of {hash} may not be removed here"
            )));
        }
        self.store
            .append(note_id, &keys::transcript(hash), None, None)
            .map(drop)
    }

    /// Whether this Device may remove `detail` from the Capture Context
    /// of the Note or Section `id`, a merged Note included: it shows
    /// there, and the Note or Section is this Device's to change
    /// ([`Store::may_change`]).
    pub fn may_remove_captured(&self, id: &str, detail: CapturedDetail) -> Result<bool> {
        match self.captured_of(id)? {
            Some(context) if context.holds(detail) => self.store.may_change(id),
            _ => Ok(false),
        }
    }

    /// Removes `detail` from the Capture Context of the Note or Section
    /// `id` (ADR 0037): its fields set to null, an edit like any other,
    /// signed, synced and judged by the roles of a Shared Area. The
    /// capture time and every other detail stay. Refused unless
    /// [`Notes::may_remove_captured`].
    pub fn remove_captured(&self, id: &str, detail: CapturedDetail) -> Result<()> {
        if self.captured_of(id)?.is_none() {
            return Err(Error::Invalid(format!("not a Note or Section: {id}")));
        }
        if !self.may_remove_captured(id, detail)? {
            return Err(Error::Invalid(format!(
                "the {} of {id} may not be removed here",
                detail.name()
            )));
        }
        self.store.batch(|| {
            for field in detail.fields() {
                self.store.append(id, field, None, None)?;
            }
            Ok(())
        })
    }

    /// The Capture Context of the Note or Section `id`, or `None` when it
    /// is neither.
    fn captured_of(&self, id: &str) -> Result<Option<CaptureContext>> {
        let fields = self.store.current_fields(id)?;
        match fields.get(keys::TYPE) {
            Some(keys::TYPE_NOTE) => Ok(self.by_id(id)?.map(|n| n.context)),
            Some(keys::TYPE_SECTION) => {
                let note = fields.get(keys::NOTE).unwrap_or_default().to_string();
                Ok(self.section(id, &note, &fields)?.map(|s| s.context))
            }
            _ => Ok(None),
        }
    }

    /// The blobs `of` every Note not in the trash that have no text yet,
    /// nor one that was removed.
    fn waiting(&self, of: impl Fn(&Note) -> Waiting<'_>) -> Result<Vec<Blob>> {
        let waits = |note: &Note, hash: &String| {
            let (_, texts, removed) = of(note);
            !texts.contains_key(hash) && !removed.contains(hash)
        };
        let mut notes = Vec::new();
        for id in self.store.entities_of_type(keys::TYPE_NOTE)? {
            if let Some(note) = self.by_id(&id)?
                && !note.is_trashed()
            {
                let (blobs, _, _) = of(&note);
                if blobs.iter().any(|h| waits(&note, h)) {
                    notes.push(note);
                }
            }
        }
        notes.sort_by_key(|n| std::cmp::Reverse(n.captured));
        let mut out = Vec::new();
        for note in &notes {
            let (blobs, _, _) = of(note);
            for hash in blobs {
                if waits(note, hash) {
                    out.push(Blob {
                        note_id: note.id.clone(),
                        hash: hash.clone(),
                    });
                }
            }
        }
        Ok(out)
    }

    /// Files the Note `note_id` into the Area or Topic `place_id`, out of
    /// wherever it was before. A Note Hidden from AI, by its own mark or
    /// its Area's, does not go into a Shared Area
    /// ([`Error::HiddenFromAi`], #267).
    pub fn file(&self, note_id: &str, place_id: &str) -> Result<()> {
        self.require_note(note_id)?;
        if !Places::new(self.store).is_place(place_id)? {
            return Err(Error::Invalid(format!("not an Area or Topic: {place_id}")));
        }
        if let Some(area) = self.store.area_of(place_id)?
            && self.store.is_shared(&area)?
        {
            Store::refuse_hidden_from_ai(self.store.hidden_from_ai_by(note_id)?)?;
        }
        self.store
            .append(note_id, keys::PLACE, Some(place_id), None)
            .map(drop)
    }

    /// Whether this Device may take the Note `note_id` out of its Area
    /// or Topic (#268): it shows filed in one that exists, and it is this
    /// Device's to change ([`Store::may_change`]), as filing it was.
    pub fn may_unfile(&self, note_id: &str) -> Result<bool> {
        Ok(self.by_id(note_id)?.is_some_and(|n| n.place.is_some())
            && self.store.may_change(note_id)?)
    }

    /// Takes the Note `note_id` out of its Area or Topic, filed nowhere
    /// again: back in Unsorted unless something else keeps it out, such
    /// as a Routine it spawned. One entry clears the place
    /// [`Notes::file`] wrote; latest wins, so the user's other Devices
    /// follow. Refused unless [`Notes::may_unfile`].
    pub fn unfile(&self, note_id: &str) -> Result<()> {
        if !self.by_id(note_id)?.is_some_and(|n| n.place.is_some()) {
            return Err(Error::Invalid(format!("not a filed Note: {note_id}")));
        }
        if !self.store.may_change(note_id)? {
            return Err(Error::Invalid(format!(
                "{note_id} may not be taken out of its place here"
            )));
        }
        self.store
            .append(note_id, keys::PLACE, None, None)
            .map(drop)
    }

    /// Fixes the text of the Note `note_id` in place. The old text stays
    /// in the log only. A text in a format this version does not know is
    /// never overwritten (ADR 0018). The text keeps the Note's format.
    pub fn edit(&self, note_id: &str, text: &str) -> Result<()> {
        self.edit_in(note_id, text, None)
    }

    /// Fixes the text of the Note `note_id` as [`Notes::edit`] does, in
    /// `format`; a formatted text stays formatted and a code text code,
    /// whatever `format` says (ADR 0033): [`Notes::set_format`] changes
    /// them.
    pub fn edit_as(&self, note_id: &str, text: &str, format: &str) -> Result<()> {
        self.edit_in(note_id, text, Some(format))
    }

    fn edit_in(&self, note_id: &str, text: &str, format: Option<&str>) -> Result<()> {
        if let Some(format) = format {
            require_format(format)?;
        }
        let Some(note) = self.by_id(note_id)? else {
            return Err(Error::Invalid(format!("not a Note: {note_id}")));
        };
        if !note.text_editable {
            return Err(Error::Invalid(format!(
                "the text of {note_id} is in a format not known here"
            )));
        }
        let written = match note.text_format.as_str() {
            text_format::MARKDOWN | text_format::CODE => &note.text_format,
            _ => format.unwrap_or(&note.text_format),
        };
        let boxed = Self::boxed(self.store, Self::text(text)?, written)?;
        let text = StoredText::of(written, &boxed, text_format::VERSION).encode();
        self.store
            .append(note_id, keys::TEXT, Some(&text), None)
            .map(drop)
    }

    /// Puts the text of the Note `note_id` in `format` as it is: a plain
    /// text becomes formatted, for its first format, and a plain or
    /// formatted text becomes code and back (#213). A formatted text
    /// never becomes plain, and nothing is written when the format is the
    /// same. Into code the text goes as it shows, each box in the glyph of
    /// its state and without its id; out of it, its glyphs become boxes.
    pub fn set_format(&self, note_id: &str, format: &str) -> Result<()> {
        require_format(format)?;
        let Some(note) = self.by_id(note_id)? else {
            return Err(Error::Invalid(format!("not a Note: {note_id}")));
        };
        if !note.text_editable {
            return Err(Error::Invalid(format!(
                "the text of {note_id} is in a format not known here"
            )));
        }
        if note.text_format == format
            || (note.text_format == text_format::MARKDOWN && format == text_format::PLAIN)
            || note.text.trim().is_empty()
        {
            return Ok(());
        }
        let text = if format == text_format::CODE {
            note.shown_text()
        } else {
            note.text.clone()
        };
        let boxed = Self::boxed(self.store, &text, format)?;
        let text = StoredText::of(format, &boxed, text_format::VERSION).encode();
        self.store
            .append(note_id, keys::TEXT, Some(&text), None)
            .map(drop)
    }

    /// Takes the Transcript or Photo Text `generated` over into the text
    /// of the Note `note_id` (ADR 0025): it becomes the text, or with
    /// `append` goes on a line of its own after it. Into a formatted text
    /// it goes as plain text, escaped so that it reads as itself. Gives
    /// the text written, its boxes with their ids (ADR 0035).
    pub fn take_over(&self, note_id: &str, generated: &str, append: bool) -> Result<String> {
        let Some(note) = self.by_id(note_id)? else {
            return Err(Error::Invalid(format!("not a Note: {note_id}")));
        };
        let taken = text_format::literal(&note.text_format, generated);
        let joined = if !append || note.text.trim().is_empty() {
            taken
        } else {
            format!("{}\n{}", note.text.trim_end(), taken)
        };
        self.edit(note_id, &joined)?;
        // As written: its glyphs boxes with their ids.
        match self.by_id(note_id)? {
            Some(note) => Ok(note.text),
            None => Ok(Self::text(&joined)?.to_string()),
        }
    }

    /// Amends the Note `note_id` with a new Section holding `text`,
    /// captured at the store's now.
    pub fn amend(&self, note_id: &str, text: &str) -> Result<Section> {
        self.amend_as(note_id, text, text_format::PLAIN)
    }

    /// Amends the Note `note_id` as [`Notes::amend`] does, the Section's
    /// text in `format`.
    pub fn amend_as(&self, note_id: &str, text: &str, format: &str) -> Result<Section> {
        require_format(format)?;
        self.require_note(note_id)?;
        if !self.store.may_write_in(note_id)? {
            return Err(Error::Invalid(format!(
                "no role in the Shared Area of {note_id}"
            )));
        }
        let trimmed = Self::text(text)?;
        let id = format!("section:{}", self.store.new_id()?);
        let now = self.store.now();
        let at = Some(now);
        self.store.batch(|| {
            let s = self.store;
            let text = StoredText::of(
                format,
                &Self::boxed(s, trimmed, format)?,
                text_format::VERSION,
            )
            .encode();
            s.append(&id, keys::TYPE, Some(keys::TYPE_SECTION), at)?;
            s.append(&id, keys::NOTE, Some(note_id), at)?;
            s.append(&id, keys::CAPTURED, Some(&dart_iso(now)), at)?;
            s.append(&id, keys::TEXT, Some(&text), at)?;
            Ok(())
        })?;
        let fields = self.store.current_fields(&id)?;
        self.section(&id, note_id, &fields)?
            .ok_or_else(|| Error::Invalid("the new Section is gone".into()))
    }

    /// Whether this Device may edit the text of the Section `id`, an
    /// Amend or a Note merged into another (#183), as the text of a Note:
    /// it is in a format known here (ADR 0018), and the Section is this
    /// Device's to change ([`Store::may_change`]), its author's or, in a
    /// Shared Area, a Manager's.
    pub fn may_edit_section(&self, id: &str) -> Result<bool> {
        Ok(
            self.section_by_id(id)?.is_some_and(|s| s.text_editable)
                && self.store.may_change(id)?,
        )
    }

    /// Fixes the text of the Section `id` in place, as [`Notes::edit`]
    /// fixes the text of a Note: a new entry on the Section, the old text
    /// stays in the log only, and the Section keeps its place and its
    /// Capture Context. The text keeps the Section's format; its bare
    /// glyphs become Boxes (ADR 0035). Nothing is written when the text
    /// stays as it is. Refused unless [`Notes::may_edit_section`].
    pub fn edit_section(&self, id: &str, text: &str) -> Result<()> {
        self.edit_section_in(id, text, None)
    }

    /// Fixes the text of the Section `id` as [`Notes::edit_section`]
    /// does, in `format`; a formatted text stays formatted, whatever
    /// `format` says (ADR 0033).
    pub fn edit_section_as(&self, id: &str, text: &str, format: &str) -> Result<()> {
        self.edit_section_in(id, text, Some(format))
    }

    fn edit_section_in(&self, id: &str, text: &str, format: Option<&str>) -> Result<()> {
        if let Some(format) = format {
            require_format(format)?;
        }
        let Some(section) = self.section_by_id(id)? else {
            return Err(Error::Invalid(format!("not a Section: {id}")));
        };
        if !self.may_edit_section(id)? {
            return Err(Error::Invalid(format!(
                "the text of {id} may not be edited here"
            )));
        }
        let written = if section.text_format == text_format::MARKDOWN {
            text_format::MARKDOWN
        } else {
            format.unwrap_or(&section.text_format)
        };
        // A Note merged into another keeps its text under the same key.
        let boxed = Self::boxed(self.store, Self::text(text)?, written)?;
        let value = StoredText::of(written, &boxed, text_format::VERSION).encode();
        if self.store.current(id, keys::TEXT)?.as_deref() == Some(value.as_str()) {
            return Ok(());
        }
        self.store
            .append(id, keys::TEXT, Some(&value), None)
            .map(drop)
    }

    /// Merges the Note `note_id` into the Note `into_id`: it becomes a
    /// dated Section of it and leaves every list and Unsorted. Nothing is
    /// copied; the merged Note only points at its target. Hidden and
    /// shared do not meet (#267): a Note Hidden from AI does not merge
    /// into a Note of a Shared Area, nor a Note of a Shared Area into a
    /// hidden one ([`Error::HiddenFromAi`]).
    pub fn merge(&self, note_id: &str, into_id: &str) -> Result<()> {
        for id in [note_id, into_id] {
            let listed = self
                .by_id(id)?
                .is_some_and(|n| !n.is_trashed() && !n.is_merged());
            if !listed {
                return Err(Error::Invalid(format!("not a Note in the lists: {id}")));
            }
        }
        if note_id == into_id {
            return Err(Error::Invalid("a Note cannot merge into itself".into()));
        }
        self.refuse_hidden_merge(&[note_id.to_string()], into_id)?;
        self.store
            .append(note_id, keys::MERGED, Some(into_id), None)
            .map(drop)
    }

    /// [`Error::HiddenFromAi`] when merging `note_ids` into `into_id`
    /// would put something Hidden from AI together with a Shared Area: a
    /// hidden Note into a Note of one, by the marks that hide it, or a
    /// Note of one into a hidden Note, by the marks that hide the target.
    /// The marks of all of them, sorted, each once.
    fn refuse_hidden_merge(&self, note_ids: &[String], into_id: &str) -> Result<()> {
        let marks = self.store.hidden_from_ai_marks()?;
        if marks.is_empty() {
            return Ok(());
        }
        let shared = |id: &str| -> Result<bool> {
            match self.store.area_of(id)? {
                Some(area) => self.store.is_shared(&area),
                None => Ok(false),
            }
        };
        let mut found: BTreeSet<String> = BTreeSet::new();
        let into_shared = shared(into_id)?;
        let into_hidden = self.store.hidden_by(into_id, &marks)?;
        for id in note_ids {
            if into_shared {
                found.extend(self.store.hidden_by(id, &marks)?);
            }
            if !into_hidden.is_empty() && shared(id)? {
                found.extend(into_hidden.iter().cloned());
            }
        }
        let mut found: Vec<String> = found.into_iter().collect();
        found.sort_by(|a, b| crate::store::utf16_cmp(a, b));
        Store::refuse_hidden_from_ai(found)
    }

    /// Whether this Device may undo the Merge of the Note `note_id`
    /// (#268): it shows merged into another, and it is this Device's to
    /// change ([`Store::may_change`]), in no Area or one it manages, or as
    /// a Member when they made it.
    pub fn may_undo_merge(&self, note_id: &str) -> Result<bool> {
        Ok(
            self.by_id(note_id)?.is_some_and(|n| n.is_merged())
                && self.store.may_change(note_id)?,
        )
    }

    /// Undoes the Merge of the Note `note_id`: it stands alone again, in
    /// the lists and where it was filed, with its Sections, Routines,
    /// Reminders and Frogs, all of which never left it. One entry clears
    /// the pointer [`Notes::merge`] wrote; latest wins, so the user's
    /// other Devices follow. Refused unless [`Notes::may_undo_merge`].
    pub fn undo_merge(&self, note_id: &str) -> Result<()> {
        if !self.by_id(note_id)?.is_some_and(|n| n.is_merged()) {
            return Err(Error::Invalid(format!("not a merged Note: {note_id}")));
        }
        if !self.store.may_change(note_id)? {
            return Err(Error::Invalid(format!(
                "the Merge of {note_id} may not be undone here"
            )));
        }
        self.store
            .append(note_id, keys::MERGED, None, None)
            .map(drop)
    }

    /// Merges every Note of `note_ids` into the Note `into_id` in one
    /// batch (#273), each as [`Notes::merge`] does: all of them or, when
    /// one is refused, none. A Note named twice merges once. Refused for
    /// the target among `note_ids`, a Note or target not in the lists, a
    /// Note this Device may not change ([`Store::may_change`]), and what
    /// is Hidden from AI meeting a Shared Area as for [`Notes::merge`].
    /// Gives the Notes merged, in the order named, for
    /// [`Notes::undo_merge_all`].
    pub fn merge_all<I, S>(&self, note_ids: I, into_id: &str) -> Result<Vec<String>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut ids: Vec<String> = Vec::new();
        for id in note_ids {
            let id = id.as_ref();
            if !ids.iter().any(|known| known == id) {
                ids.push(id.to_string());
            }
        }
        for id in ids.iter().map(String::as_str).chain([into_id]) {
            let listed = self
                .by_id(id)?
                .is_some_and(|n| !n.is_trashed() && !n.is_merged());
            if !listed {
                return Err(Error::Invalid(format!("not a Note in the lists: {id}")));
            }
        }
        if ids.iter().any(|id| id == into_id) {
            return Err(Error::Invalid(format!(
                "the target is among the Notes merged: {into_id}"
            )));
        }
        for id in &ids {
            if !self.store.may_change(id)? {
                return Err(Error::Invalid(format!("{id} may not be merged here")));
            }
        }
        self.refuse_hidden_merge(&ids, into_id)?;
        self.store.batch(|| {
            for id in &ids {
                self.store.append(id, keys::MERGED, Some(into_id), None)?;
            }
            Ok(())
        })?;
        Ok(ids)
    }

    /// Undoes the Merge of each Note of `note_ids` in one batch, as
    /// [`Notes::undo_merge`] does for each one that
    /// [`Notes::may_undo_merge`]; the others, which a sync or another step
    /// got to first, are left as they are. Gives the Notes that stand
    /// alone again.
    pub fn undo_merge_all<I, S>(&self, note_ids: I) -> Result<Vec<String>>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let note_ids: Vec<S> = note_ids.into_iter().collect();
        self.store.batch(|| {
            let mut undone: Vec<String> = Vec::new();
            for id in &note_ids {
                let id = id.as_ref();
                if undone.iter().any(|known| known == id) || !self.may_undo_merge(id)? {
                    continue;
                }
                self.store.append(id, keys::MERGED, None, None)?;
                undone.push(id.to_string());
            }
            Ok(undone)
        })
    }

    /// Sets the Importance of the Note `note_id`.
    pub fn set_importance(&self, note_id: &str, importance: Importance) -> Result<()> {
        self.require_note(note_id)?;
        self.store
            .append(note_id, keys::IMPORTANCE, Some(importance.name()), None)
            .map(drop)
    }

    /// Moves the Note `note_id` to the trash: it leaves every list and
    /// Unsorted, and can be restored until [`Notes::trash_keep`] has
    /// passed.
    pub fn trash(&self, note_id: &str) -> Result<()> {
        self.require_note(note_id)?;
        // The time is read on its own before the entry reads it again, as
        // in the Dart core.
        let at = dart_iso(self.store.now());
        self.store
            .append(note_id, keys::TRASHED, Some(&at), None)
            .map(drop)
    }

    /// Removes the Note `note_id` for good, without the trash, as purging
    /// does: for a Note just captured whose content went elsewhere, such as
    /// a capture sorted into a Shopping List (#124). Refused for a Note that
    /// spawned something, which would be left without its Note, and where
    /// this Device may not change it.
    pub fn discard(&self, note_id: &str) -> Result<()> {
        let Some(note) = self.by_id(note_id)? else {
            return Err(Error::Invalid(format!("not a Note: {note_id}")));
        };
        if note.spawned || !self.store.may_change(note_id)? {
            return Err(Error::Invalid(format!("{note_id} may not be discarded")));
        }
        self.store.append(note_id, keys::TYPE, None, None).map(drop)
    }

    /// Brings the Note `note_id` back from the trash, to where it was
    /// filed.
    pub fn restore(&self, note_id: &str) -> Result<()> {
        self.require_note(note_id)?;
        self.store
            .append(note_id, keys::TRASHED, None, None)
            .map(drop)
    }

    fn require_note(&self, id: &str) -> Result<()> {
        match self.by_id(id)? {
            Some(_) => Ok(()),
            None => Err(Error::Invalid(format!("not a Note: {id}"))),
        }
    }

    /// `text` trimmed; an error when nothing is left.
    /// `text` in `format` as it is saved: every bare ☐, ✅ and ❌ a box
    /// with an id of its own (ADR 0035).
    fn boxed(store: &Store, text: &str, format: &str) -> Result<String> {
        text_boxes::convert(text, format, || store.new_id())
    }

    fn text(text: &str) -> Result<&str> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(Error::Invalid("the text must not be empty".into()));
        }
        Ok(trimmed)
    }

    /// The Note `id`, trashed or not, or `None` when there is none.
    pub fn by_id(&self, id: &str) -> Result<Option<Note>> {
        let fields = self.store.current_fields(id)?;
        if fields.get(keys::TYPE) != Some(keys::TYPE_NOTE) {
            return Ok(None);
        }
        let Some(captured) = fields.get(keys::CAPTURED) else {
            return Ok(None);
        };
        // A place that is gone leaves the Note Unsorted.
        let place = match fields.get(keys::PLACE) {
            Some(p) if Places::new(self.store).is_place(p)? => Some(p.to_string()),
            _ => None,
        };
        let text = StoredText::read(fields.get(keys::TEXT));
        let transcripts = texts(&fields, keys::TRANSCRIPT_PREFIX);
        let photo_texts = texts(&fields, keys::OCR_PREFIX);
        let removed = |prefix: &str, texts: &BTreeMap<String, String>| -> Vec<String> {
            removed(&fields, prefix)
                .into_iter()
                .filter(|hash| has_text(texts.get(hash)))
                .collect()
        };
        let photos = added(&fields, keys::PHOTO_PREFIX);
        let recordings = added(&fields, keys::AUDIO_PREFIX);
        // Only while the file stays: a text is removed from it, never
        // from a file that is gone.
        let removed_texts = |blobs: &[String], field: fn(&str) -> String| -> Vec<String> {
            blobs
                .iter()
                .filter(|hash| removed_text(&fields, &field(hash)))
                .cloned()
                .collect()
        };
        let removed_transcripts = removed_texts(&recordings, keys::transcript);
        let removed_photo_texts = removed_texts(&photos, keys::ocr);
        let attachment_texts = texts(&fields, keys::FILE_TEXT_PREFIX);
        let attachments: Vec<Attachment> = fields
            .iter()
            .filter_map(|(key, value)| {
                let hash = key.strip_prefix(keys::FILE_PREFIX)?;
                Some(Attachment::read(hash, value?))
            })
            .collect();
        let hashes: Vec<String> = attachments.iter().map(|a| a.hash.clone()).collect();
        let removed_attachment_texts = removed_texts(&hashes, keys::file_text);
        let gone = removed(keys::FILE_PREFIX, &attachment_texts);
        let removed_attachments = if gone.is_empty() {
            Vec::new()
        } else {
            self.as_attached(id, &gone)?
        };
        let boxes = self.boxes(id, &text, &fields)?;
        Ok(Some(Note {
            id: id.to_string(),
            text: text.text,
            text_format: text.format,
            text_version: text.version,
            text_editable: text.editable,
            captured: parse_time(captured)?,
            context: self.context(&fields)?,
            place,
            importance: Importance::from_name(fields.get(keys::IMPORTANCE)),
            trashed: fields.get(keys::TRASHED).map(parse_time).transpose()?,
            merged_into: self.merged_into(id)?,
            spawned: fields.get(keys::SPAWNED).is_some(),
            nudge: fields.get(keys::NUDGE).and_then(|n| parse_time(n).ok()),
            // The fields come in the order they were first written.
            photos,
            recordings,
            removed_photos: removed(keys::PHOTO_PREFIX, &photo_texts),
            removed_recordings: removed(keys::AUDIO_PREFIX, &transcripts),
            transcripts,
            photo_texts,
            removed_transcripts,
            removed_photo_texts,
            attachments,
            attachment_texts,
            removed_attachments,
            removed_attachment_texts,
            boxes,
        }))
    }

    /// The removed files `hashes` of the Note `id` as they were attached:
    /// each by the value of its field that wins by latest wins among
    /// those that name the file.
    fn as_attached(&self, id: &str, hashes: &[String]) -> Result<Vec<Attachment>> {
        let history = self.store.history(id)?;
        Ok(hashes
            .iter()
            .map(|hash| {
                let field = keys::file(hash);
                let mut held: Option<&Entry> = None;
                for e in &history {
                    if e.field == field
                        && e.value.is_some()
                        && held
                            .is_none_or(|h| Store::latest_wins(e, h) == std::cmp::Ordering::Greater)
                    {
                        held = Some(e);
                    }
                }
                Attachment::read(hash, held.and_then(|e| e.value.as_deref()).unwrap_or(""))
            })
            .collect())
    }

    /// The boxes of the stored `text` of `id` with the ticks that count
    /// (ADR 0035); none in a text this version may not change, nor in a
    /// code text (#213).
    fn boxes(&self, id: &str, text: &StoredText, fields: &Fields) -> Result<Vec<ShownBox>> {
        if !text.editable || text.format == text_format::CODE {
            return Ok(Vec::new());
        }
        let ticked = fields.iter().any(|(k, _)| k.starts_with(boxes::PREFIX));
        let ticks = if ticked {
            Boxes::new(self.store).ticks(id)?
        } else {
            BTreeMap::new()
        };
        Ok(text_boxes::shown(&text.text, &ticks).boxes)
    }

    /// The Section `id` as it shows under its Note: an Amend, or a Note
    /// merged into another; `None` when there is none.
    pub fn section_by_id(&self, id: &str) -> Result<Option<Section>> {
        if !id.starts_with("note:") {
            let fields = self.store.current_fields(id)?;
            let Some(note_id) = fields.get(keys::NOTE).map(str::to_string) else {
                return Ok(None);
            };
            return self.section(id, &note_id, &fields);
        }
        let Some(note) = self.by_id(id)? else {
            return Ok(None);
        };
        match note.merged_into.clone() {
            Some(into) => Ok(Some(self.merged_section(note, &into)?)),
            None => Ok(None),
        }
    }

    /// Every Note neither in the trash nor merged, newest capture first.
    pub fn all(&self) -> Result<Vec<Note>> {
        let mut notes = Vec::new();
        for id in self.store.entities_of_type(keys::TYPE_NOTE)? {
            if let Some(note) = self.by_id(&id)?
                && !note.is_trashed()
                && !note.is_merged()
            {
                notes.push(note);
            }
        }
        notes.sort_by_key(|n| std::cmp::Reverse(n.captured));
        Ok(notes)
    }

    /// Every Unsorted Note, oldest capture first.
    pub fn unsorted(&self) -> Result<Vec<Note>> {
        Ok(self
            .all()?
            .into_iter()
            .rev()
            .filter(Note::is_unsorted)
            .collect())
    }

    /// The Notes filed in the Area or Topic `place_id`, newest capture
    /// first; an Area holds its Topics' Notes too.
    pub fn inside(&self, place_id: &str) -> Result<Vec<Note>> {
        let mut ids: HashSet<String> = HashSet::from([place_id.to_string()]);
        ids.extend(
            Places::new(self.store)
                .topics(place_id)?
                .into_iter()
                .map(|t| t.id),
        );
        Ok(self
            .all()?
            .into_iter()
            .filter(|n| n.place.as_ref().is_some_and(|p| ids.contains(p)))
            .collect())
    }

    /// The Notes in the trash that can still be restored, most recently
    /// deleted first.
    pub fn trashed(&self) -> Result<Vec<Note>> {
        let now = self.store.now();
        let mut notes = Vec::new();
        for id in self.store.entities_of_type(keys::TYPE_NOTE)? {
            if let Some(note) = self.by_id(&id)?
                && !note.is_merged()
                && let Some(at) = note.trashed
                && now < at + Self::trash_keep()
            {
                notes.push(note);
            }
        }
        notes.sort_by(|a, b| {
            let a = a.trashed.unwrap_or(a.captured);
            let b = b.trashed.unwrap_or(b.captured);
            b.cmp(&a)
        });
        Ok(notes)
    }

    /// The Sections of the Note `note_id`, oldest first: its Amends, the
    /// Notes merged into it, and their Amends.
    pub fn sections(&self, note_id: &str) -> Result<Vec<Section>> {
        let mut merged: Vec<String> = Vec::new();
        for id in self.store.entities_of_type(keys::TYPE_NOTE)? {
            if self.merged_into(&id)?.as_deref() == Some(note_id) && !merged.contains(&id) {
                merged.push(id);
            }
        }
        let mut sections = Vec::new();
        for id in &merged {
            if let Some(note) = self.by_id(id)? {
                sections.push(self.merged_section(note, note_id)?);
            }
        }
        for id in self.store.entities_of_type(keys::TYPE_SECTION)? {
            let owner = self.store.current(&id, keys::NOTE)?;
            let owned = owner
                .as_deref()
                .is_some_and(|o| o == note_id || merged.iter().any(|m| m == o));
            if owned {
                let fields = self.store.current_fields(&id)?;
                sections.extend(self.section(&id, note_id, &fields)?);
            }
        }
        sections.sort_by_key(|s| s.captured);
        Ok(sections)
    }

    /// The Sections of each Note of `note_ids`, exactly as
    /// [`Notes::sections`] gives them, read in one pass over the Notes and
    /// Sections: for a list that shows many Notes in full (#114). Every id
    /// asked for is a key, one that is not a Note with the Sections that
    /// name it, if any.
    pub fn sections_of(&self, note_ids: &[String]) -> Result<BTreeMap<String, Vec<Section>>> {
        let mut of: BTreeMap<String, Vec<Section>> =
            note_ids.iter().map(|id| (id.clone(), Vec::new())).collect();
        // Where each Note merged into one of them shows: its Amends show
        // there as well as under itself.
        let mut into: HashMap<String, String> = HashMap::new();
        for id in self.store.entities_of_type(keys::TYPE_NOTE)? {
            let Some(end) = self.merged_into(&id)? else {
                continue;
            };
            if !of.contains_key(&end) || into.contains_key(&id) {
                continue;
            }
            let note = self.by_id(&id)?;
            if let (Some(note), Some(list)) = (note, of.get_mut(&end)) {
                list.push(self.merged_section(note, &end)?);
            }
            into.insert(id, end);
        }
        for id in self.store.entities_of_type(keys::TYPE_SECTION)? {
            let Some(owner) = self.store.current(&id, keys::NOTE)? else {
                continue;
            };
            let under: Vec<&str> = [Some(owner.as_str()), into.get(&owner).map(String::as_str)]
                .into_iter()
                .flatten()
                .filter(|note_id| of.contains_key(*note_id))
                .collect();
            if under.is_empty() {
                continue;
            }
            let fields = self.store.current_fields(&id)?;
            for note_id in under {
                if let Some(section) = self.section(&id, note_id, &fields)?
                    && let Some(list) = of.get_mut(note_id)
                {
                    list.push(section);
                }
            }
        }
        for list in of.values_mut() {
            list.sort_by_key(|s| s.captured);
        }
        Ok(of)
    }

    /// The Note `note`, merged into `note_id`, as a Section of it.
    fn merged_section(&self, note: Note, note_id: &str) -> Result<Section> {
        let (modified, edited_by) = self.last_edit(&note.id)?;
        Ok(Section {
            id: note.id,
            note_id: note_id.to_string(),
            text: note.text,
            text_format: note.text_format,
            text_version: note.text_version,
            text_editable: note.text_editable,
            captured: note.captured,
            context: note.context,
            photos: note.photos,
            recordings: note.recordings,
            transcripts: note.transcripts,
            photo_texts: note.photo_texts,
            removed_photos: note.removed_photos,
            removed_recordings: note.removed_recordings,
            removed_transcripts: note.removed_transcripts,
            removed_photo_texts: note.removed_photo_texts,
            attachments: note.attachments,
            attachment_texts: note.attachment_texts,
            removed_attachments: note.removed_attachments,
            removed_attachment_texts: note.removed_attachment_texts,
            boxes: note.boxes,
            modified,
            edited_by,
        })
    }

    /// When the text of the Note or Section `id` was last edited, and who
    /// edited it, as the Dart core's `_lastEdit` (#183): the text that wins
    /// by latest wins, among those that count, once there is more than
    /// one; `None` while there is only the text it was captured with.
    /// Ordered by latest wins, not by the log, so every Device finds the
    /// same.
    fn last_edit(&self, id: &str) -> Result<(Option<DateTime<Utc>>, Option<Profile>)> {
        let mut latest: Option<Entry> = None;
        let mut texts = 0;
        for e in self.store.history(id)? {
            if e.field != keys::TEXT {
                continue;
            }
            texts += 1;
            let newer = latest
                .as_ref()
                .is_none_or(|held| Store::latest_wins(&e, held) == std::cmp::Ordering::Greater);
            if newer {
                latest = Some(e);
            }
        }
        match latest {
            Some(e) if texts > 1 => Ok((Some(e.date_at), self.store.editor_of(id, &e.device)?)),
            _ => Ok((None, None)),
        }
    }

    /// The Section `id` from its `fields`, shown under `note_id`.
    fn section(&self, id: &str, note_id: &str, fields: &Fields) -> Result<Option<Section>> {
        if fields.get(keys::TYPE) != Some(keys::TYPE_SECTION) {
            return Ok(None);
        }
        let (Some(_), Some(captured)) = (fields.get(keys::NOTE), fields.get(keys::CAPTURED)) else {
            return Ok(None);
        };
        let text = StoredText::read(fields.get(keys::TEXT));
        let boxes = self.boxes(id, &text, fields)?;
        let (modified, edited_by) = self.last_edit(id)?;
        Ok(Some(Section {
            id: id.to_string(),
            note_id: note_id.to_string(),
            text: text.text,
            text_format: text.format,
            text_version: text.version,
            text_editable: text.editable,
            captured: parse_time(captured)?,
            context: self.context(fields)?,
            photos: Vec::new(),
            recordings: Vec::new(),
            transcripts: BTreeMap::new(),
            photo_texts: BTreeMap::new(),
            removed_photos: Vec::new(),
            removed_recordings: Vec::new(),
            removed_transcripts: Vec::new(),
            removed_photo_texts: Vec::new(),
            attachments: Vec::new(),
            attachment_texts: BTreeMap::new(),
            removed_attachments: Vec::new(),
            removed_attachment_texts: Vec::new(),
            boxes,
            modified,
            edited_by,
        }))
    }

    /// The Capture Context in the `fields` of a Note or Section, with the
    /// Named Place its location lies in.
    fn context(&self, fields: &Fields) -> Result<CaptureContext> {
        let location = Coordinates::decode(fields.get(keys::LOCATION));
        let named_place = match location {
            Some(at) => NamedPlaces::new(self.store).at(at)?,
            None => None,
        };
        let source_app = match fields.get(keys::SOURCE_PACKAGE) {
            Some(package) if !package.is_empty() => Some(SourceApp {
                package: package.to_string(),
                label: fields.get(keys::SOURCE_LABEL).map(str::to_string),
            }),
            _ => None,
        };
        Ok(CaptureContext {
            location,
            named_place,
            source_app,
            calendar_events: names(fields.get(keys::CALENDAR)),
            bluetooth_devices: names(fields.get(keys::BLUETOOTH)),
        })
    }

    /// The Note `id` ends up in at the end of its Merge chain, or `None`
    /// when it stands alone. A chain stops before a Note this Device does
    /// not hold. A loop stops at its lowest id, so every Device keeps the
    /// same Note.
    fn merged_into(&self, id: &str) -> Result<Option<String>> {
        let mut chain = vec![id.to_string()];
        loop {
            let Some(last) = chain.last() else {
                return Ok(None);
            };
            let Some(next) = self.store.current(last, keys::MERGED)? else {
                break;
            };
            if self.store.current(&next, keys::TYPE)?.as_deref() != Some(keys::TYPE_NOTE) {
                break;
            }
            if let Some(start) = chain.iter().position(|c| *c == next) {
                let kept = chain
                    .iter()
                    .skip(start)
                    .min_by(|a, b| crate::store::utf16_cmp(a, b))
                    .cloned();
                return Ok(kept.filter(|k| k != id));
            }
            chain.push(next);
        }
        Ok(chain.last().filter(|last| *last != id).cloned())
    }
}

/// The hashes of the photos or recordings under `prefix`, in the order
/// they were added.
fn added(fields: &Fields, prefix: &str) -> Vec<String> {
    fields
        .iter()
        .filter(|(_, value)| *value == Some(keys::ADDED))
        .filter_map(|(key, _)| key.strip_prefix(prefix).map(str::to_string))
        .collect()
}

/// The hashes of the photos or recordings under `prefix` that were
/// removed, in the order they were added.
fn removed(fields: &Fields, prefix: &str) -> Vec<String> {
    fields
        .iter()
        .filter(|(_, value)| value.is_none())
        .filter_map(|(key, _)| key.strip_prefix(prefix).map(str::to_string))
        .collect()
}

/// Whether the text `field` was written and then removed: it holds null.
fn removed_text(fields: &Fields, field: &str) -> bool {
    fields
        .iter()
        .any(|(key, value)| key == field && value.is_none())
}

/// Whether a Transcript or Photo Text says anything: an empty one means
/// nothing was found.
fn has_text(text: Option<&String>) -> bool {
    text.is_some_and(|t| !t.trim().is_empty())
}

/// A stored JSON list of names, its strings in their order; empty when
/// absent or unreadable.
fn names(json: Option<&str>) -> Vec<String> {
    match json.map(serde_json::from_str::<serde_json::Value>) {
        Some(Ok(serde_json::Value::Array(items))) => items
            .into_iter()
            .filter_map(|item| match item {
                serde_json::Value::String(name) => Some(name),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// The texts under `prefix`, by the hash they belong to.
fn texts(fields: &Fields, prefix: &str) -> BTreeMap<String, String> {
    fields
        .iter()
        .filter_map(|(key, value)| {
            Some((key.strip_prefix(prefix)?.to_string(), value?.to_string()))
        })
        .collect()
}

#[cfg(test)]
mod tests;
