//! Notes and their Sections as the Dart core's `Notes` reads and writes
//! them: the text, the capture time, the Importance, where a Note is
//! filed, whether it is in the trash, what was merged into it, and its
//! photos and voice recordings with their Photo Texts and Transcripts,
//! and the Capture Context of Notes and Sections. Every write appends the same entries, in the same order and read from the
//! clock as often, as the Dart core's, so both logs come out alike.

use std::collections::{BTreeMap, HashMap, HashSet};

use chrono::{DateTime, Duration, Utc};

use crate::entry::{dart_iso, parse_time};
use crate::named_places::{Coordinates, NamedPlace, NamedPlaces};
use crate::places::Places;
use crate::store::{Fields, Store};
use crate::text_format::StoredText;
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
    /// The Note this one was merged into; absent while it stands alone.
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
    /// The Photo Text of a photo, `ocr:<hash>` = the text ocrs recognised
    /// in it, on the Note holding `photo:<hash>`. An empty text means no
    /// text was found; absent means not read (yet); null means removed
    /// while the photo stays (ADR 0025).
    pub const OCR_PREFIX: &str = "ocr:";
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
/// in a row or an alarm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Wordless {
    #[default]
    Photo,
    VoiceRecording,
}

impl Wordless {
    /// Its name, as the Dart core's enum has it.
    pub fn name(self) -> &'static str {
        match self {
            Wordless::Photo => "photo",
            Wordless::VoiceRecording => "voiceRecording",
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
}

impl Note {
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

    /// What the Note is titled as while its text is blank: a voice
    /// recording when it has one and no photo, removed ones whose texts
    /// stay included; a photo otherwise.
    pub fn wordless(&self) -> Wordless {
        if self.photos.is_empty()
            && self.removed_photos.is_empty()
            && !(self.recordings.is_empty() && self.removed_recordings.is_empty())
        {
            Wordless::VoiceRecording
        } else {
            Wordless::Photo
        }
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
        self.capture_with(text, place, nudge_in, &[], &[])
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
        self.store.batch(|| {
            let s = self.store;
            s.append(&id, keys::TYPE, Some(keys::TYPE_NOTE), at)?;
            s.append(&id, keys::CAPTURED, Some(&dart_iso(now)), at)?;
            let text = StoredText::plain(text).encode();
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
    /// wherever it was before.
    pub fn file(&self, note_id: &str, place_id: &str) -> Result<()> {
        self.require_note(note_id)?;
        if !Places::new(self.store).is_place(place_id)? {
            return Err(Error::Invalid(format!("not an Area or Topic: {place_id}")));
        }
        self.store
            .append(note_id, keys::PLACE, Some(place_id), None)
            .map(drop)
    }

    /// Fixes the text of the Note `note_id` in place. The old text stays
    /// in the log only. A text in a format this version does not know is
    /// never overwritten (ADR 0018).
    pub fn edit(&self, note_id: &str, text: &str) -> Result<()> {
        let Some(note) = self.by_id(note_id)? else {
            return Err(Error::Invalid(format!("not a Note: {note_id}")));
        };
        if !note.text_editable {
            return Err(Error::Invalid(format!(
                "the text of {note_id} is in a format not known here"
            )));
        }
        let text = StoredText::plain(Self::text(text)?).encode();
        self.store
            .append(note_id, keys::TEXT, Some(&text), None)
            .map(drop)
    }

    /// Amends the Note `note_id` with a new Section holding `text`,
    /// captured at the store's now.
    pub fn amend(&self, note_id: &str, text: &str) -> Result<Section> {
        self.require_note(note_id)?;
        if !self.store.may_write_in(note_id)? {
            return Err(Error::Invalid(format!(
                "no role in the Shared Area of {note_id}"
            )));
        }
        let text = StoredText::plain(Self::text(text)?).encode();
        let id = format!("section:{}", self.store.new_id()?);
        let now = self.store.now();
        let at = Some(now);
        self.store.batch(|| {
            let s = self.store;
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

    /// Merges the Note `note_id` into the Note `into_id`: it becomes a
    /// dated Section of it and leaves every list and Unsorted. Nothing is
    /// copied; the merged Note only points at its target.
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
        self.store
            .append(note_id, keys::MERGED, Some(into_id), None)
            .map(drop)
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
        }))
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
                sections.push(Self::merged_section(note, note_id));
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
                list.push(Self::merged_section(note, &end));
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
    fn merged_section(note: Note, note_id: &str) -> Section {
        Section {
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
                let kept = chain[start..]
                    .iter()
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
mod tests {
    use super::*;

    #[test]
    fn importance_reads_its_stored_name_and_defaults_to_normal() {
        for i in [
            Importance::Unimportant,
            Importance::Normal,
            Importance::Important,
        ] {
            assert_eq!(Importance::from_name(Some(i.name())), i);
        }
        assert_eq!(Importance::from_name(None), Importance::Normal);
        assert_eq!(Importance::from_name(Some("huge")), Importance::Normal);
    }

    #[test]
    fn a_note_without_words_is_titled_by_what_it_holds() -> Result<()> {
        let store = crate::store::tests::store();
        let notes = Notes::new(&store);
        let wordless =
            |id: &str| -> Result<Option<Wordless>> { Ok(notes.by_id(id)?.map(|n| n.wordless())) };
        let untranscribed = notes.capture_with("", None, None, &[], &[voice(1)])?;
        let silent = notes.capture_with("", None, None, &[], &[voice(2)])?;
        notes.set_transcript(&silent.id, &silent.recordings[0], "")?;
        let receipt = notes.capture_with("", None, None, &[photo(1)], &[])?;
        let both = notes.capture_with("", None, None, &[photo(2)], &[voice(3)])?;
        assert_eq!(wordless(&untranscribed.id)?, Some(Wordless::VoiceRecording));
        assert_eq!(wordless(&silent.id)?, Some(Wordless::VoiceRecording));
        assert_eq!(wordless(&receipt.id)?, Some(Wordless::Photo));
        assert_eq!(wordless(&both.id)?, Some(Wordless::Photo));
        // A file gone while its text stays still counts.
        let memo = notes.capture_with("", None, None, &[], &[voice(4)])?;
        notes.set_transcript(&memo.id, &memo.recordings[0], "call the bank")?;
        notes.remove_recording(&memo.id, &memo.recordings[0])?;
        notes.set_photo_text(&both.id, &both.photos[0], "RECEIPT")?;
        notes.remove_photo(&both.id, &both.photos[0])?;
        assert_eq!(notes.by_id(&memo.id)?.map(|n| n.recordings.len()), Some(0));
        assert_eq!(wordless(&memo.id)?, Some(Wordless::VoiceRecording));
        assert_eq!(notes.by_id(&both.id)?.map(|n| n.photos.len()), Some(0));
        assert_eq!(wordless(&both.id)?, Some(Wordless::Photo));
        Ok(())
    }

    #[test]
    fn writes_check_what_they_write_to() -> Result<()> {
        let store = crate::store::tests::store();
        let notes = Notes::new(&store);
        assert!(notes.capture("x", Some("area:none"), None).is_err());
        let a = notes.capture("  kept as typed ", None, None)?;
        assert_eq!(a.text, "  kept as typed ");
        assert!(notes.edit(&a.id, "   ").is_err());
        assert!(notes.edit("note:none", "x").is_err());
        assert!(notes.amend(&a.id, "").is_err());
        assert!(notes.file(&a.id, "topic:none").is_err());
        assert!(notes.merge(&a.id, &a.id).is_err());
        assert!(notes.merge(&a.id, "note:none").is_err());
        notes.edit(&a.id, " fixed ")?;
        assert_eq!(notes.by_id(&a.id)?.map(|n| n.text), Some("fixed".into()));
        let b = notes.capture("other", None, Some(Duration::minutes(10)))?;
        notes.trash(&b.id)?;
        assert!(notes.merge(&a.id, &b.id).is_err());
        notes.restore(&b.id)?;
        notes.merge(&a.id, &b.id)?;
        assert_eq!(notes.sections(&b.id)?.len(), 1);
        assert_eq!(
            store.current(&b.id, keys::NUDGE)?.as_deref(),
            Some("2026-01-01T10:10:06.000Z")
        );
        Ok(())
    }

    #[test]
    fn the_sections_of_many_notes_read_at_once_are_those_of_each() -> Result<()> {
        let store = crate::store::tests::store();
        let notes = Notes::new(&store);
        let plain = notes.capture("plain", None, None)?;
        let amended = notes.capture("amended", None, None)?;
        notes.amend(&amended.id, "amend")?;
        // A chain a → b → c, with Amends on a and b, read for all three.
        let a = notes.capture("a", None, None)?;
        notes.amend(&a.id, "a amend")?;
        let b = notes.capture("b", None, None)?;
        notes.amend(&b.id, "b amend")?;
        let c = notes.capture("c", None, None)?;
        notes.merge(&a.id, &b.id)?;
        notes.merge(&b.id, &c.id)?;
        // Two Notes merged into each other, and a Merge into a Note gone.
        let x = notes.capture("x", None, None)?;
        let y = notes.capture("y", None, None)?;
        notes.amend(&y.id, "y amend")?;
        store.append(&x.id, keys::MERGED, Some(&y.id), None)?;
        store.append(&y.id, keys::MERGED, Some(&x.id), None)?;
        let lost = notes.capture("lost", None, None)?;
        store.append(&lost.id, keys::MERGED, Some("note:never-synced"), None)?;
        notes.amend(&lost.id, "lost amend")?;
        // A Section whose Note never arrived.
        let orphan = "section:orphan";
        store.append(orphan, keys::TYPE, Some(keys::TYPE_SECTION), None)?;
        store.append(orphan, keys::NOTE, Some("note:never-synced"), None)?;
        store.append(orphan, keys::TEXT, Some("orphan"), None)?;
        let at = dart_iso(store.now());
        store.append(orphan, keys::CAPTURED, Some(&at), None)?;

        let mut ids: Vec<String> = [&plain, &amended, &a, &b, &c, &x, &y, &lost]
            .iter()
            .map(|n| n.id.clone())
            .collect();
        ids.push("note:never-synced".into());
        ids.push("note:unknown".into());
        let read = notes.sections_of(&ids)?;
        assert_eq!(read.len(), ids.len());
        for id in &ids {
            assert_eq!(read.get(id), Some(&notes.sections(id)?), "{id}");
            let alone = notes.sections_of(std::slice::from_ref(id))?;
            assert_eq!(alone.get(id), read.get(id), "{id}");
        }
        let texts = |id: &str| -> Vec<String> {
            read.get(id)
                .map(|list| list.iter().map(|s| s.text.clone()).collect())
                .unwrap_or_default()
        };
        assert_eq!(texts(&b.id), ["b amend"]);
        assert_eq!(texts(&c.id), ["a", "a amend", "b", "b amend"]);
        assert!(
            read.get(&c.id)
                .into_iter()
                .flatten()
                .all(|s| s.note_id == c.id)
        );
        assert_eq!(texts("note:never-synced"), ["orphan"]);
        assert_eq!(read.get("note:unknown"), Some(&Vec::new()));
        assert!(notes.sections_of(&[])?.is_empty());
        Ok(())
    }

    #[test]
    fn a_text_in_an_unknown_format_is_never_overwritten_here() -> Result<()> {
        const FUTURE: &str = r##"{"format":"x-future","text":"# Heading"}"##;
        let store = crate::store::tests::store();
        let notes = Notes::new(&store);
        let a = notes.capture("x", None, None)?;
        assert_eq!(
            store.current(&a.id, keys::TEXT)?.as_deref(),
            Some(r#"{"format":"plain","text":"x"}"#)
        );
        store.append(&a.id, keys::TEXT, Some("Old milk"), None)?;
        let old = notes.by_id(&a.id)?.ok_or(Error::Invalid("note".into()))?;
        assert_eq!((old.text.as_str(), old.text_editable), ("Old milk", true));
        store.append(&a.id, keys::TEXT, Some(FUTURE), None)?;
        let other = notes.capture("Other", None, None)?;
        let note = notes.by_id(&a.id)?.ok_or(Error::Invalid("note".into()))?;
        assert_eq!(note.text, "# Heading");
        assert_eq!(note.text_format, "x-future");
        assert!(!note.text_editable);
        assert!(notes.edit(&a.id, "Plain now").is_err());
        // Amend adds a Section; Merge points at the other Note.
        let section = notes.amend(&a.id, "Amended")?;
        assert!(section.text_editable);
        notes.merge(&a.id, &other.id)?;
        assert_eq!(store.current(&a.id, keys::TEXT)?.as_deref(), Some(FUTURE));
        store.append(&section.id, keys::TEXT, Some(FUTURE), None)?;
        let shown = notes.sections(&other.id)?;
        assert_eq!(shown.len(), 2);
        assert!(
            shown
                .iter()
                .all(|s| !s.text_editable && s.text == "# Heading")
        );
        Ok(())
    }

    #[test]
    fn a_text_in_a_later_structure_is_never_overwritten_here() -> Result<()> {
        const LATER: &str = r#"{"format":"plain","more":1,"text":"Later milk","v":2}"#;
        let store = crate::store::tests::store();
        let notes = Notes::new(&store);
        let a = notes.capture("x", None, None)?;
        store.append(&a.id, keys::TEXT, Some(LATER), None)?;
        let note = notes.by_id(&a.id)?.ok_or(Error::Invalid("note".into()))?;
        assert_eq!(note.text, "Later milk");
        assert_eq!(note.text_version, 2);
        assert!(!note.text_editable);
        assert!(notes.edit(&a.id, "Plain now").is_err());
        let section = notes.amend(&a.id, "Amended")?;
        store.append(&section.id, keys::TEXT, Some(LATER), None)?;
        let shown = notes.sections(&a.id)?;
        assert_eq!(shown.len(), 1);
        assert_eq!(shown[0].text_version, 2);
        assert!(!shown[0].text_editable);
        assert_eq!(store.current(&a.id, keys::TEXT)?.as_deref(), Some(LATER));
        Ok(())
    }

    // ------------------------------------------ removing media (ADR 0020)

    use crate::lan::{Joining, answer_sync_inviting, apply_answer_as, sync_request_as};
    use crate::shared::{AreaRole, Invite};
    use crate::signing::SigningKey;
    use crate::store::Clock;
    use crate::sync::split_blob_ref;
    use crate::{PairedDevice, Search};
    use chrono::TimeZone;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicI64, Ordering};

    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

    /// Devices sharing one clock, a second per read, so they write in a
    /// known order; each with a Profile of its own.
    struct Devices(Arc<AtomicI64>);

    impl Devices {
        fn new() -> Devices {
            Devices(Arc::new(AtomicI64::new(0)))
        }

        fn device(&self, seed: u8, name: &str) -> Result<Store> {
            let ticks = Arc::clone(&self.0);
            let clock: Clock = Box::new(move || {
                let n = ticks.fetch_add(1, Ordering::SeqCst) + 1;
                let start = Utc.with_ymd_and_hms(2026, 3, 1, 9, 0, 0).single();
                start.unwrap_or_default() + Duration::seconds(n)
            });
            let store = Store::in_memory(clock, move || Ok(SigningKey::from_seed([seed; 32])))?;
            crate::profile::OwnProfile::new(&store).apply_defaults(name, None)?;
            Ok(store)
        }
    }

    /// Stand-ins: a JPEG start, so the store keeps the photo as it is.
    fn photo(n: u8) -> Vec<u8> {
        vec![0xFF, 0xD8, 0xFF, 0xD9, n]
    }

    fn voice(n: u8) -> Vec<u8> {
        vec![0x52, 0x49, 0x46, 0x46, n]
    }

    /// A Note with a photo and a recording, both with their text.
    fn with_texts(store: &Store, place: Option<&str>, n: u8) -> Result<(String, String, String)> {
        let notes = Notes::new(store);
        let note = notes.capture_with("Dentist", place, None, &[photo(n)], &[voice(n)])?;
        let (p, r) = (note.photos[0].clone(), note.recordings[0].clone());
        notes.set_photo_text(&note.id, &p, "Dr. Weber, Mon 9-12")?;
        notes.set_transcript(&note.id, &r, "Bring the old X-ray")?;
        Ok((note.id, p, r))
    }

    /// The blobs `to` lacks, carried over from `from`.
    fn carry(from: &Store, to: &Store) -> Result<()> {
        for reference in to.missing_blobs()? {
            let (kind, hash) = split_blob_ref(&reference);
            if let Some(bytes) = from.blob(kind, hash) {
                to.put_blob(kind, hash, &bytes)?;
            }
        }
        Ok(())
    }

    /// One LAN session without the network, the blobs after it both
    /// ways: `joiner` joins `host`, which invites it when `inviting`.
    /// Both remember each other, as the apps do.
    fn exchange(joiner: &Store, host: &Store, inviting: Option<&Invite>) -> TestResult {
        let host_id = host.device_id()?;
        let joiner_id = joiner.device_id()?;
        let shared = inviting.is_some() || joiner.is_shared_peer(&host_id)?;
        let joining = match inviting {
            Some(_) => Joining::Invited,
            None => Joining::Plain,
        };
        let vector = host.version_vector()?;
        let request = sync_request_as(joiner, &vector, "Phone", &host_id, "", joining)?;
        let (answer, _) =
            answer_sync_inviting(host, &request, "Desk", inviting)?.ok_or("an answer")?;
        apply_answer_as(joiner, &answer, shared, None)?;
        for (store, id) in [(host, &joiner_id), (joiner, &host_id)] {
            store.remember_device(&PairedDevice {
                id: id.clone(),
                name: "Other".into(),
                fingerprint: String::new(),
                host: None,
                port: None,
                last_sync: store.now(),
                shared,
            })?;
        }
        carry(host, joiner)?;
        carry(joiner, host)?;
        Ok(())
    }

    fn found(store: &Store, query: &str) -> Result<Vec<String>> {
        Ok(Search::new(store)
            .notes(query, None)?
            .into_iter()
            .map(|n| n.id)
            .collect())
    }

    fn note(store: &Store, id: &str) -> Result<Note> {
        Notes::new(store)
            .by_id(id)?
            .ok_or_else(|| Error::Invalid(format!("no Note {id}")))
    }

    /// Ann's Household shared with Ben as a Member, and her private
    /// Health beside it: (area, private).
    fn household(
        ann: &Store,
        ben: &Store,
    ) -> std::result::Result<(String, String), Box<dyn std::error::Error>> {
        let places = Places::new(ann);
        let area = ann.share(&places.add_area("Household")?.id)?;
        let private = places.add_area("Health")?.id;
        let invite = Invite {
            area: area.clone(),
            role: AreaRole::Member,
        };
        exchange(ben, ann, Some(&invite))?;
        Ok((area, private))
    }

    #[test]
    fn a_removed_file_goes_and_its_text_stays_and_is_found() -> TestResult {
        let ann = Devices::new().device(1, "Ann")?;
        let notes = Notes::new(&ann);
        let (id, p, r) = with_texts(&ann, None, 1)?;
        assert!(notes.may_remove_photo(&id, &p)?);
        assert!(notes.may_remove_recording(&id, &r)?);
        notes.remove_photo(&id, &p)?;
        notes.remove_recording(&id, &r)?;
        let read = note(&ann, &id)?;
        assert!(read.photos.is_empty() && read.recordings.is_empty());
        assert_eq!(read.removed_photos, vec![p.clone()]);
        assert_eq!(read.removed_recordings, vec![r.clone()]);
        assert_eq!(
            read.photo_texts.get(&p).map(String::as_str),
            Some("Dr. Weber, Mon 9-12")
        );
        assert_eq!(
            read.transcripts.get(&r).map(String::as_str),
            Some("Bring the old X-ray")
        );
        assert_eq!(ann.photo(&p), None);
        assert_eq!(ann.audio(&r), None);
        assert!(ann.missing_blobs()?.is_empty(), "not wanted back");
        assert!(!ann.knows_blob(&format!("photo/{p}"))?);
        assert_eq!(found(&ann, "weber")?, vec![id.clone()]);
        assert_eq!(found(&ann, "x-ray")?, vec![id.clone()]);
        assert!(!notes.may_remove_photo(&id, &p)?);
        assert_eq!(ann.current(&id, &keys::photo(&p))?, None);
        Ok(())
    }

    #[test]
    fn a_file_without_a_text_or_with_an_empty_one_is_not_removed() -> TestResult {
        let ann = Devices::new().device(1, "Ann")?;
        let notes = Notes::new(&ann);
        let n = notes.capture_with("", None, None, &[photo(1), photo(2)], &[voice(1)])?;
        let (p1, p2, r) = (&n.photos[0], &n.photos[1], &n.recordings[0]);
        notes.set_photo_text(&n.id, p2, "   ")?;
        for hash in [p1, p2] {
            assert!(!notes.may_remove_photo(&n.id, hash)?);
            assert!(notes.remove_photo(&n.id, hash).is_err());
        }
        assert!(!notes.may_remove_recording(&n.id, r)?);
        assert!(notes.remove_recording(&n.id, r).is_err());
        assert!(
            notes.remove_photo(&n.id, r).is_err(),
            "a recording is no photo"
        );
        assert!(!notes.may_remove_photo("note:none", p1)?);
        assert_eq!(note(&ann, &n.id)?.photos, n.photos);
        assert!(ann.photo(p1).is_some());
        Ok(())
    }

    #[test]
    fn the_bytes_stay_while_another_note_shows_the_same_photo() -> TestResult {
        let ann = Devices::new().device(1, "Ann")?;
        let notes = Notes::new(&ann);
        let (id, p, _) = with_texts(&ann, None, 1)?;
        let other = notes.capture_with("Same card", None, None, &[photo(1)], &[])?;
        assert_eq!(other.photos, vec![p.clone()]);
        notes.remove_photo(&id, &p)?;
        assert_eq!(ann.photo(&p), Some(photo(1)));
        assert!(ann.removed_blobs()?.is_empty());
        let target = notes.capture("Health", None, None)?;
        notes.merge(&other.id, &target.id)?;
        notes.set_photo_text(&other.id, &p, "Same card")?;
        notes.remove_photo(&other.id, &p)?;
        let section = notes.sections(&target.id)?;
        assert_eq!(section[0].removed_photos, vec![p.clone()]);
        assert_eq!(ann.photo(&p), None, "gone with the last Note showing it");
        Ok(())
    }

    #[test]
    fn the_removal_reaches_another_own_device() -> TestResult {
        let devices = Devices::new();
        let ann = devices.device(1, "Ann")?;
        let desk = devices.device(3, "Ann")?;
        let (id, p, r) = with_texts(&ann, None, 1)?;
        exchange(&desk, &ann, None)?;
        assert!(desk.photo(&p).is_some() && desk.audio(&r).is_some());
        Notes::new(&ann).remove_photo(&id, &p)?;
        Notes::new(&desk).remove_recording(&id, &r)?;
        exchange(&desk, &ann, None)?;
        for store in [&ann, &desk] {
            let read = note(store, &id)?;
            assert!(read.photos.is_empty() && read.recordings.is_empty());
            assert_eq!(read.photo_texts.len() + read.transcripts.len(), 2);
            assert_eq!(store.photo(&p), None);
            assert_eq!(store.audio(&r), None);
            assert!(store.missing_blobs()?.is_empty());
            assert_eq!(found(store, "weber")?, vec![id.clone()]);
        }
        Ok(())
    }

    #[test]
    fn a_removal_in_a_shared_area_reaches_the_member_and_the_manager() -> TestResult {
        let devices = Devices::new();
        let ann = devices.device(1, "Ann")?;
        let ben = devices.device(2, "Ben")?;
        let (area, _) = household(&ann, &ben)?;
        let (id, p, _) = with_texts(&ann, Some(&area), 1)?;
        let (own, own_p, own_r) = with_texts(&ben, Some(&area), 2)?;
        exchange(&ben, &ann, None)?;
        assert!(ben.photo(&p).is_some() && ann.photo(&own_p).is_some());
        assert!(!Notes::new(&ben).may_remove_photo(&id, &p)?);
        Notes::new(&ann).remove_photo(&id, &p)?;
        // A Member removes from their own Note, a Manager from anyone's.
        Notes::new(&ben).remove_photo(&own, &own_p)?;
        Notes::new(&ann).remove_recording(&own, &own_r)?;
        exchange(&ben, &ann, None)?;
        for store in [&ann, &ben] {
            assert_eq!(note(store, &id)?.removed_photos, vec![p.clone()]);
            assert_eq!(note(store, &own)?.removed_photos, vec![own_p.clone()]);
            assert_eq!(note(store, &own)?.removed_recordings, vec![own_r.clone()]);
            assert_eq!(store.photo(&p), None);
            assert_eq!(store.photo(&own_p), None);
            assert_eq!(store.audio(&own_r), None);
            assert_eq!(found(store, "weber")?.len(), 2);
        }
        Ok(())
    }

    #[test]
    fn a_members_removal_on_anothers_note_is_refused_and_ignored() -> TestResult {
        let devices = Devices::new();
        let ann = devices.device(1, "Ann")?;
        let ben = devices.device(2, "Ben")?;
        let (area, _) = household(&ann, &ben)?;
        let (id, p, _) = with_texts(&ann, Some(&area), 1)?;
        exchange(&ben, &ann, None)?;
        assert!(Notes::new(&ben).remove_photo(&id, &p).is_err());
        // As a modified app would write it.
        ben.append(&id, &keys::photo(&p), None, None)?;
        ben.drop_removed_blobs()?;
        exchange(&ben, &ann, None)?;
        for store in [&ann, &ben] {
            assert_eq!(note(store, &id)?.photos, vec![p.clone()]);
            assert!(store.photo(&p).is_some());
        }
        Ok(())
    }

    #[test]
    fn another_persons_removal_outside_a_shared_area_keeps_the_bytes() -> TestResult {
        let devices = Devices::new();
        let ann = devices.device(1, "Ann")?;
        let ben = devices.device(2, "Ben")?;
        let (area, private) = household(&ann, &ben)?;
        let (id, p, _) = with_texts(&ann, Some(&area), 1)?;
        exchange(&ben, &ann, None)?;
        // Ann takes the Note out of the Shared Area; Ben has not seen it
        // and writes a removal as a modified app would.
        Notes::new(&ann).file(&id, &private)?;
        ben.append(&id, &keys::photo(&p), None, None)?;
        exchange(&ben, &ann, None)?;
        assert_eq!(ann.current(&id, &keys::photo(&p))?, None);
        assert!(ann.removed_blobs()?.is_empty());
        assert_eq!(ann.photo(&p), Some(photo(1)));
        Ok(())
    }

    #[test]
    fn a_removed_text_goes_its_file_stays_and_it_is_not_read_again() -> TestResult {
        let ann = Devices::new().device(1, "Ann")?;
        let notes = Notes::new(&ann);
        let (id, p, r) = with_texts(&ann, None, 1)?;
        assert!(notes.may_remove_photo_text(&id, &p)?);
        assert!(notes.may_remove_transcript(&id, &r)?);
        notes.remove_photo_text(&id, &p)?;
        notes.remove_transcript(&id, &r)?;
        let read = note(&ann, &id)?;
        assert_eq!(read.photos, vec![p.clone()]);
        assert_eq!(read.recordings, vec![r.clone()]);
        assert!(read.photo_texts.is_empty() && read.transcripts.is_empty());
        assert_eq!(read.removed_photo_texts, vec![p.clone()]);
        assert_eq!(read.removed_transcripts, vec![r.clone()]);
        assert!(read.removed_photos.is_empty() && read.removed_recordings.is_empty());
        assert!(ann.photo(&p).is_some() && ann.audio(&r).is_some());
        assert!(found(&ann, "weber")?.is_empty());
        assert!(found(&ann, "x-ray")?.is_empty());
        assert!(notes.unread_photos()?.is_empty());
        assert!(notes.untranscribed()?.is_empty());
        // Without a text the file may not go either.
        assert!(!notes.may_remove_photo(&id, &p)?);
        assert!(!notes.may_remove_photo_text(&id, &p)?);
        assert_eq!(ann.current(&id, &keys::transcript(&r))?, None);
        // Transcribing or reading again brings a text back.
        notes.set_photo_text(&id, &p, "Dr. Weber, Tue 9-12")?;
        notes.set_transcript(&id, &r, "Bring the new X-ray")?;
        let read = note(&ann, &id)?;
        assert!(read.removed_photo_texts.is_empty() && read.removed_transcripts.is_empty());
        assert_eq!(found(&ann, "tue")?, vec![id.clone()]);
        assert_eq!(found(&ann, "new x-ray")?, vec![id.clone()]);
        Ok(())
    }

    #[test]
    fn a_missing_or_empty_text_or_one_without_its_file_is_not_removed() -> TestResult {
        let ann = Devices::new().device(1, "Ann")?;
        let notes = Notes::new(&ann);
        let n = notes.capture_with("", None, None, &[photo(1), photo(2)], &[voice(1)])?;
        let (p1, p2, r) = (&n.photos[0], &n.photos[1], &n.recordings[0]);
        notes.set_photo_text(&n.id, p2, "")?;
        for hash in [p1, p2] {
            assert!(!notes.may_remove_photo_text(&n.id, hash)?);
            assert!(notes.remove_photo_text(&n.id, hash).is_err());
        }
        assert!(!notes.may_remove_transcript(&n.id, r)?);
        assert!(notes.remove_transcript(&n.id, r).is_err());
        assert!(
            notes.remove_photo_text(&n.id, r).is_err(),
            "a recording is no photo"
        );
        assert!(!notes.may_remove_transcript("note:none", r)?);
        let (id, p, _) = with_texts(&ann, None, 3)?;
        notes.remove_photo(&id, &p)?;
        assert!(!notes.may_remove_photo_text(&id, &p)?);
        assert!(notes.remove_photo_text(&id, &p).is_err());
        Ok(())
    }

    #[test]
    fn a_removed_text_shows_in_the_section_and_reaches_another_own_device() -> TestResult {
        let devices = Devices::new();
        let ann = devices.device(1, "Ann")?;
        let desk = devices.device(3, "Ann")?;
        let notes = Notes::new(&ann);
        let (id, p, r) = with_texts(&ann, None, 1)?;
        let target = notes.capture("Health", None, None)?;
        notes.merge(&id, &target.id)?;
        exchange(&desk, &ann, None)?;
        notes.remove_transcript(&id, &r)?;
        Notes::new(&desk).remove_photo_text(&id, &p)?;
        exchange(&desk, &ann, None)?;
        for store in [&ann, &desk] {
            let section = Notes::new(store).sections(&target.id)?;
            assert_eq!(section[0].removed_transcripts, vec![r.clone()]);
            assert_eq!(section[0].removed_photo_texts, vec![p.clone()]);
            assert!(store.photo(&p).is_some() && store.audio(&r).is_some());
            assert!(Notes::new(store).untranscribed()?.is_empty());
            assert!(Notes::new(store).unread_photos()?.is_empty());
            assert!(found(store, "weber")?.is_empty());
        }
        Ok(())
    }

    #[test]
    fn a_members_text_removal_on_anothers_note_is_refused_and_ignored() -> TestResult {
        let devices = Devices::new();
        let ann = devices.device(1, "Ann")?;
        let ben = devices.device(2, "Ben")?;
        let (area, _) = household(&ann, &ben)?;
        let (id, p, r) = with_texts(&ann, Some(&area), 1)?;
        let (own, own_p, own_r) = with_texts(&ben, Some(&area), 2)?;
        exchange(&ben, &ann, None)?;
        assert!(!Notes::new(&ben).may_remove_transcript(&id, &r)?);
        assert!(Notes::new(&ben).remove_transcript(&id, &r).is_err());
        // As a modified app would write them.
        ben.append(&id, &keys::transcript(&r), None, None)?;
        ben.append(&id, &keys::ocr(&p), None, None)?;
        // A Member removes from their own Note, a Manager from anyone's.
        Notes::new(&ben).remove_photo_text(&own, &own_p)?;
        Notes::new(&ann).remove_transcript(&own, &own_r)?;
        exchange(&ben, &ann, None)?;
        for store in [&ann, &ben] {
            let read = note(store, &id)?;
            assert_eq!(
                read.transcripts.get(&r).map(String::as_str),
                Some("Bring the old X-ray")
            );
            assert_eq!(
                read.photo_texts.get(&p).map(String::as_str),
                Some("Dr. Weber, Mon 9-12")
            );
            assert!(read.removed_transcripts.is_empty());
            let own = note(store, &own)?;
            assert_eq!(own.removed_photo_texts, vec![own_p.clone()]);
            assert_eq!(own.removed_transcripts, vec![own_r.clone()]);
        }
        Ok(())
    }

    #[test]
    fn a_members_machine_still_gives_anothers_note_its_texts() -> TestResult {
        let devices = Devices::new();
        let ann = devices.device(1, "Ann")?;
        let ben = devices.device(2, "Ben")?;
        let (area, _) = household(&ann, &ben)?;
        let shed =
            Notes::new(&ann).capture_with("Shed", Some(&area), None, &[photo(4)], &[voice(4)])?;
        exchange(&ben, &ann, None)?;
        Notes::new(&ben).set_photo_text(&shed.id, &shed.photos[0], "Spare key")?;
        Notes::new(&ben).set_transcript(&shed.id, &shed.recordings[0], "Oil the door")?;
        exchange(&ben, &ann, None)?;
        let read = note(&ann, &shed.id)?;
        assert_eq!(
            read.photo_texts.get(&shed.photos[0]).map(String::as_str),
            Some("Spare key")
        );
        assert_eq!(
            read.transcripts
                .get(&shed.recordings[0])
                .map(String::as_str),
            Some("Oil the door")
        );
        Ok(())
    }

    #[test]
    fn a_removed_photo_leaves_the_shared_folder() -> TestResult {
        use crate::folder::{MemoryFolder, folder_sync, random_nonce};
        let devices = Devices::new();
        let ann = devices.device(1, "Ann")?;
        let desk = devices.device(3, "Ann")?;
        exchange(&desk, &ann, None)?;
        let (id, p, r) = with_texts(&ann, None, 1)?;
        let mut folder = MemoryFolder::default();
        let blobs = |folder: &MemoryFolder| folder.dirs.get("blobs").map_or(0, |b| b.len());
        folder_sync(&ann, &mut folder, &mut random_nonce)?;
        assert_eq!(blobs(&folder), 2);
        folder_sync(&desk, &mut folder, &mut random_nonce)?;
        assert!(desk.photo(&p).is_some());
        Notes::new(&ann).remove_photo(&id, &p)?;
        folder_sync(&ann, &mut folder, &mut random_nonce)?;
        assert_eq!(blobs(&folder), 1);
        folder_sync(&desk, &mut folder, &mut random_nonce)?;
        assert_eq!(desk.photo(&p), None);
        assert!(desk.audio(&r).is_some());
        assert_eq!(note(&desk, &id)?.removed_photos, vec![p]);
        Ok(())
    }

    #[test]
    fn a_note_without_a_capture_time_is_no_note() -> Result<()> {
        let store = crate::store::tests::store();
        store.append("note:a", keys::TYPE, Some(keys::TYPE_NOTE), None)?;
        assert_eq!(Notes::new(&store).by_id("note:a")?, None);
        store.append(
            "note:a",
            keys::CAPTURED,
            Some("2026-01-01T10:00:00.000Z"),
            None,
        )?;
        let note = Notes::new(&store).by_id("note:a")?;
        assert!(note.is_some_and(|n| n.is_unsorted() && n.text.is_empty()));
        Ok(())
    }

    #[test]
    fn names_read_only_the_strings_of_a_json_list() {
        let read = |json: Option<&str>| names(json);
        assert_eq!(read(None), Vec::<String>::new());
        assert_eq!(read(Some(r#"["Standup","Lunch"]"#)), ["Standup", "Lunch"]);
        // As the Dart core's `whereType<String>`: in their order, empty
        // and repeated ones kept.
        assert_eq!(
            read(Some(r#" ["Dentist",3,null,"",["x"],{"a":"b"},"Dentist"] "#)),
            ["Dentist", "", "Dentist"]
        );
        for garbled in [
            "",
            "not json",
            r#""Car kit""#,
            r#"{"name":"Car kit"}"#,
            "[]",
            r#"["a",]"#,
            "[\"a\"] x",
        ] {
            assert_eq!(read(Some(garbled)), Vec::<String>::new(), "{garbled:?}");
        }
    }

    #[test]
    fn a_source_app_is_named_by_its_label_else_its_package() {
        let app = |label: Option<&str>| SourceApp {
            package: "com.whatsapp".into(),
            label: label.map(str::to_string),
        };
        assert_eq!(app(Some("WhatsApp")).name(), "WhatsApp");
        assert_eq!(app(Some(" WhatsApp\u{a0}")).name(), "WhatsApp");
        for blank in [None, Some(""), Some("  "), Some("\u{feff}\n")] {
            assert_eq!(app(blank).name(), "com.whatsapp", "{blank:?}");
        }
    }

    /// Writes `values` on `id`, as another version or Device might have.
    fn put(store: &Store, id: &str, values: &[(&str, Option<&str>)]) -> Result<()> {
        for (field, value) in values {
            store.append(id, field, *value, None)?;
        }
        Ok(())
    }

    #[test]
    fn notes_and_sections_read_their_capture_context() -> Result<()> {
        use crate::named_places::keys as place_keys;
        let store = crate::store::tests::store();
        let notes = Notes::new(&store);
        for (id, name, centre, radius) in [
            ("namedplace:home", "Home", "52.52,13.405", "300.0"),
            ("namedplace:kitchen", "Kitchen", "52.521,13.405", "30.0"),
        ] {
            put(
                &store,
                id,
                &[
                    (place_keys::TYPE, Some(place_keys::TYPE_NAMED_PLACE)),
                    (place_keys::NAME, Some(name)),
                    (place_keys::CENTRE, Some(centre)),
                    (place_keys::RADIUS, Some(radius)),
                ],
            )?;
        }
        let shared = notes.capture("Shared in", None, None)?;
        assert_eq!(shared.context, CaptureContext::default());
        assert!(!shared.context.has_more());
        put(
            &store,
            &shared.id,
            &[
                (keys::LOCATION, Some("52.521,13.405")),
                (keys::SOURCE_PACKAGE, Some("com.whatsapp")),
                (keys::CALENDAR, Some(r#"["Standup"]"#)),
                (keys::BLUETOOTH, Some(r#"["Car kit"]"#)),
            ],
        )?;
        let context = note(&store, &shared.id)?.context;
        assert_eq!(context.location, Coordinates::new(52.521, 13.405));
        // The smaller of the two holding it.
        assert_eq!(
            context.named_place.map(|p| p.id).as_deref(),
            Some("namedplace:kitchen")
        );
        let app = context.source_app.ok_or(Error::Invalid("no app".into()))?;
        assert_eq!((app.package.as_str(), app.label), ("com.whatsapp", None));
        assert_eq!(context.calendar_events, ["Standup"]);
        assert_eq!(context.bluetooth_devices, ["Car kit"]);
        // An Amend keeps its own, under the same keys; garbled values
        // read as nothing, and an empty package as no app.
        let host = notes.capture("Host", None, None)?;
        let amend = notes.amend(&host.id, "Later")?;
        put(
            &store,
            &amend.id,
            &[
                (keys::LOCATION, Some(" 52.52 , 13.405 ")),
                (keys::SOURCE_PACKAGE, Some("")),
                (keys::SOURCE_LABEL, Some("Ghost")),
                (keys::CALENDAR, Some("not json")),
                (keys::BLUETOOTH, Some(r#"["Ear buds"]"#)),
            ],
        )?;
        put(&store, &host.id, &[(keys::LOCATION, Some("91,0"))])?;
        notes.merge(&shared.id, &host.id)?;
        let host_context = note(&store, &host.id)?.context;
        assert_eq!(host_context.location, None);
        assert_eq!(host_context.named_place, None);
        let sections = notes.sections(&host.id)?;
        let at_once = notes.sections_of(std::slice::from_ref(&host.id))?;
        assert_eq!(at_once.get(&host.id), Some(&sections));
        let by_id = |id: &str| sections.iter().find(|s| s.id == id).map(|s| &s.context);
        // The merged Note brings its own.
        assert_eq!(by_id(&shared.id), Some(&note(&store, &shared.id)?.context));
        let amended = by_id(&amend.id).cloned().unwrap_or_default();
        assert_eq!(
            amended.named_place.as_ref().map(|p| p.name.as_str()),
            Some("Home")
        );
        assert_eq!(amended.source_app, None);
        assert!(amended.calendar_events.is_empty());
        assert_eq!(amended.bluetooth_devices, ["Ear buds"]);
        assert!(amended.has_more());
        // A Named Place renamed or deleted shows on what was captured
        // before.
        put(&store, "namedplace:kitchen", &[(place_keys::TYPE, None)])?;
        let moved = note(&store, &shared.id)?.context.named_place;
        assert_eq!(moved.map(|p| p.name).as_deref(), Some("Home"));
        put(
            &store,
            "namedplace:home",
            &[(place_keys::NAME, Some("Flat"))],
        )?;
        let renamed = note(&store, &shared.id)?.context.named_place;
        assert_eq!(renamed.map(|p| p.name).as_deref(), Some("Flat"));
        // A cleared field reads as unknown again.
        put(
            &store,
            &shared.id,
            &[(keys::CALENDAR, None), (keys::LOCATION, None)],
        )?;
        let cleared = note(&store, &shared.id)?.context;
        assert!(cleared.calendar_events.is_empty() && cleared.location.is_none());
        Ok(())
    }
}
