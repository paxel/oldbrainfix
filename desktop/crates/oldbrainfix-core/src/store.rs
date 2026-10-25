//! The log: an append-only entry log over SQLite with a latest-wins
//! projection (ADR 0002), the same tables as the Dart core's `Store`.
//!
//! The store never reads the wall clock on its own: the clock is handed
//! in, so tests get a reproducible log.
//!
//! One store can be shared between threads: each call, and each batch as
//! a whole, holds the connection on its own, so two threads never write
//! into each other's transaction or take the same dseq.
//!
//! Photo and voice recording bytes live outside the log, content-addressed
//! by SHA-256: `photos` and `audio` directories next to the database file,
//! as the Dart core keeps them, or memory for an in-memory store.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::Path;

use chrono::{DateTime, Utc};
use parking_lot::ReentrantMutex;
use rusqlite::{Connection, OptionalExtension, Row, params};
use sha2::{Digest, Sha256};

use crate::blobs::{Blobs, FileBlobs, MemoryBlobs};
use crate::entry::{Entry, WireEntry, dart_iso, parse_time};
use crate::photos::strip_jpeg_metadata;
use crate::signing::{SigningKey, device_id_from_key, entry_bytes};
use crate::{Error, Result};

/// Where the store reads "now".
pub type Clock = Box<dyn Fn() -> DateTime<Utc> + Send + Sync>;

/// Where the store gets the ids of new entities: random, unless a test
/// counts them as the Dart core's fixture generator does.
pub type IdMaker = Box<dyn Fn() -> Result<String> + Send + Sync>;

/// The tables, word for word as the Dart core creates them.
const SCHEMA: &str = "
      CREATE TABLE IF NOT EXISTS entries (
        seq      INTEGER PRIMARY KEY AUTOINCREMENT,
        device   TEXT NOT NULL,
        dseq     INTEGER NOT NULL,
        entity   TEXT NOT NULL,
        field    TEXT NOT NULL,
        value    TEXT,
        date     TEXT NOT NULL,
        author   TEXT NOT NULL,
        recorded TEXT NOT NULL,
        sig      TEXT
      );
      CREATE TABLE IF NOT EXISTS local_settings (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
      );
      CREATE TABLE IF NOT EXISTS moments (
        id    INTEGER PRIMARY KEY AUTOINCREMENT,
        seq   INTEGER NOT NULL,
        cause TEXT NOT NULL,
        label TEXT,
        at    TEXT NOT NULL
      );
      CREATE UNIQUE INDEX IF NOT EXISTS idx_entries_device_dseq
        ON entries (device, dseq);
      CREATE INDEX IF NOT EXISTS idx_entries_entity_field
        ON entries (entity, field);
    ";

/// The current fields of one entity, in the order they were first
/// written, as the Dart core's `currentFields` map keeps them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Fields(Vec<(String, Option<String>)>);

impl Fields {
    /// The current value of `field`; `None` when absent or cleared.
    pub fn get(&self, field: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(k, _)| k == field)
            .and_then(|(_, v)| v.as_deref())
    }

    /// Every field with its current value, in first-written order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, Option<&str>)> {
        self.0.iter().map(|(k, v)| (k.as_str(), v.as_deref()))
    }
}

/// A Moment as the store keeps it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MomentRow {
    pub id: i64,
    pub seq: i64,
    pub cause: String,
    pub label: Option<String>,
    /// When, as stored.
    pub at: String,
}

pub struct Store {
    /// Reentrant, so a batch holds it while the calls inside take it too.
    db: ReentrantMutex<Connection>,
    clock: Clock,
    ids: IdMaker,
    photos: Box<dyn Blobs>,
    audio: Box<dyn Blobs>,
}

impl Store {
    /// Opens an on-disk store, creating it if needed. `new_key` makes the
    /// Device's signing key on first open.
    pub fn open(
        path: &Path,
        clock: Clock,
        new_key: impl FnOnce() -> Result<SigningKey>,
    ) -> Result<Store> {
        let dir = path.parent().unwrap_or(Path::new("."));
        let photos = Box::new(FileBlobs::new(dir.join("photos"))?);
        let audio = Box::new(FileBlobs::new(dir.join("audio"))?);
        Self::init(Connection::open(path)?, clock, new_key, photos, audio)
    }

    /// Opens a throwaway in-memory store (tests).
    pub fn in_memory(clock: Clock, new_key: impl FnOnce() -> Result<SigningKey>) -> Result<Store> {
        let photos = Box::<MemoryBlobs>::default();
        let audio = Box::<MemoryBlobs>::default();
        Self::init(Connection::open_in_memory()?, clock, new_key, photos, audio)
    }

    fn init(
        db: Connection,
        clock: Clock,
        new_key: impl FnOnce() -> Result<SigningKey>,
        photos: Box<dyn Blobs>,
        audio: Box<dyn Blobs>,
    ) -> Result<Store> {
        // The pragma answers with the mode it took; memory stays memory.
        db.query_row("PRAGMA journal_mode = WAL", [], |_| Ok(()))?;
        db.execute_batch(SCHEMA)?;
        let store = Store {
            db: ReentrantMutex::new(db),
            clock,
            ids: Box::new(random_id),
            photos,
            audio,
        };
        if store.local_setting("device")?.is_none() {
            let key = new_key()?;
            store.set_local_setting("device", &device_id_from_key(&key.public_key()))?;
            store.set_local_setting("signseed", &key.seed_hex())?;
        }
        Ok(store)
    }

    /// Now, by the store's clock.
    pub fn now(&self) -> DateTime<Utc> {
        (self.clock)()
    }

    /// Makes the ids of new entities with `ids` from now on.
    pub fn set_id_maker(&mut self, ids: IdMaker) {
        self.ids = ids;
    }

    /// A fresh id for a new entity.
    pub fn new_id(&self) -> Result<String> {
        (self.ids)()
    }

    /// Runs `body` in one transaction: all of its entries or none. Not
    /// to be nested.
    pub fn batch<T>(&self, body: impl FnOnce() -> Result<T>) -> Result<T> {
        let db = self.db.lock();
        let tx = db.unchecked_transaction()?;
        let out = body()?;
        tx.commit()?;
        Ok(out)
    }

    /// This installation's stable device id.
    pub fn device_id(&self) -> Result<String> {
        self.local_setting("device")?
            .ok_or_else(|| Error::Invalid("the store has no device id".into()))
    }

    /// This Device's signing key.
    pub fn signing_key(&self) -> Result<SigningKey> {
        let seed = self
            .local_setting("signseed")?
            .ok_or_else(|| Error::Invalid("the store has no signing key".into()))?;
        SigningKey::from_seed_hex(&seed)
    }

    /// Author name stamped on own entries.
    pub fn author(&self) -> Result<String> {
        Ok(self.local_setting("author")?.unwrap_or_default())
    }

    // ---------------------------------------------------- local settings

    /// A setting of this Device. Never synced (ADR 0008).
    pub fn local_setting(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .db
            .lock()
            .query_row(
                "SELECT value FROM local_settings WHERE key = ?",
                [key],
                |r| r.get(0),
            )
            .optional()?)
    }

    pub fn set_local_setting(&self, key: &str, value: &str) -> Result<()> {
        self.db.lock().execute(
            "INSERT OR REPLACE INTO local_settings (key, value) VALUES (?, ?)",
            [key, value],
        )?;
        Ok(())
    }

    pub fn remove_local_setting(&self, key: &str) -> Result<()> {
        self.db
            .lock()
            .execute("DELETE FROM local_settings WHERE key = ?", [key])?;
        Ok(())
    }

    /// Every setting of this Device whose key starts with `prefix`, as
    /// (key, value), by key.
    pub fn local_settings_with_prefix(&self, prefix: &str) -> Result<Vec<(String, String)>> {
        let db = self.db.lock();
        let mut stmt = db.prepare(
            "SELECT key, value FROM local_settings WHERE substr(key, 1, ?) = ? ORDER BY key",
        )?;
        let rows = stmt.query_map(params![prefix.chars().count() as i64, prefix], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    // ------------------------------------------------------------ photos

    /// Keeps the photo `bytes` without their metadata
    /// ([`strip_jpeg_metadata`]) and returns the hash it is kept under.
    /// The same photo twice is kept once.
    pub fn put_photo(&self, bytes: &[u8]) -> Result<String> {
        let clean = strip_jpeg_metadata(bytes);
        let hash = hex::encode(Sha256::digest(&clean));
        self.photos.put(&hash, &clean)?;
        Ok(hash)
    }

    /// The bytes of the photo `hash`, or `None` when they are not here.
    pub fn photo(&self, hash: &str) -> Option<Vec<u8>> {
        self.photos.get(hash)
    }

    // ------------------------------------------------------------- audio

    /// Keeps the voice recording `bytes` as they are and returns the hash
    /// it is kept under. The same recording twice is kept once.
    pub fn put_audio(&self, bytes: &[u8]) -> Result<String> {
        let hash = hex::encode(Sha256::digest(bytes));
        self.audio.put(&hash, bytes)?;
        Ok(hash)
    }

    /// The bytes of the voice recording `hash`, or `None` when they are
    /// not here.
    pub fn audio(&self, hash: &str) -> Option<Vec<u8>> {
        self.audio.get(hash)
    }

    /// The bytes of the `kind` (`photo` or `audio`) blob `hash`.
    pub fn blob(&self, kind: &str, hash: &str) -> Option<Vec<u8>> {
        match kind {
            "photo" => self.photos.get(hash),
            "audio" => self.audio.get(hash),
            _ => None,
        }
    }

    /// Keeps `bytes` another Device holds under `hash`, as they are.
    /// Bytes whose SHA-256 is not `hash`, or of an unknown `kind`, are
    /// not kept; returns whether they were.
    pub fn put_blob(&self, kind: &str, hash: &str, bytes: &[u8]) -> Result<bool> {
        if hex::encode(Sha256::digest(bytes)) != hash {
            return Ok(false);
        }
        match kind {
            "photo" => self.photos.put(hash, bytes)?,
            "audio" => self.audio.put(hash, bytes)?,
            _ => return Ok(false),
        }
        Ok(true)
    }

    // --------------------------------------------------------------- log

    /// Appends one fact under this Device, signed as the Dart core signs.
    /// `date` defaults to now.
    pub fn append(
        &self,
        entity: &str,
        field: &str,
        value: Option<&str>,
        date: Option<DateTime<Utc>>,
    ) -> Result<Entry> {
        let db = self.db.lock();
        let now = self.now();
        let device = self.device_id()?;
        let dseq = self.version_vector()?.get(&device).copied().unwrap_or(0) + 1;
        let date_iso = dart_iso(date.unwrap_or(now));
        let recorded_iso = dart_iso(now);
        let author = self.author()?;
        let sig = self.signing_key()?.sign(&entry_bytes(
            &device,
            dseq,
            entity,
            field,
            value,
            &date_iso,
            &author,
            &recorded_iso,
        ));
        db.execute(
            "INSERT INTO entries (device, dseq, entity, field, value, date, author, recorded, sig) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                device,
                dseq,
                entity,
                field,
                value,
                date_iso,
                author,
                recorded_iso,
                sig
            ],
        )?;
        self.entries(
            "SELECT * FROM entries WHERE device = ? AND dseq = ?",
            params![device, dseq],
        )?
        .pop()
        .ok_or_else(|| Error::Invalid("the appended entry is gone".into()))
    }

    /// Takes in an entry another Device wrote, as it is; one already held
    /// is skipped. Returns whether it was new.
    pub fn insert(&self, entry: &WireEntry) -> Result<bool> {
        let changed = self.db.lock().execute(
            "INSERT OR IGNORE INTO entries (device, dseq, entity, field, value, date, author, recorded, sig) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                entry.device,
                entry.dseq,
                entry.entity,
                entry.field,
                entry.value,
                entry.date,
                entry.author,
                entry.recorded,
                entry.sig
            ],
        )?;
        Ok(changed > 0)
    }

    /// Whether the entry (device, dseq) is held.
    pub fn holds(&self, device: &str, dseq: i64) -> Result<bool> {
        Ok(self
            .db
            .lock()
            .query_row(
                "SELECT 1 FROM entries WHERE device = ? AND dseq = ?",
                params![device, dseq],
                |_| Ok(()),
            )
            .optional()?
            .is_some())
    }

    /// Every entry a Device holding `vector` lacks, by (device, dseq).
    pub fn entries_since(&self, vector: &HashMap<String, i64>) -> Result<Vec<Entry>> {
        Ok(self
            .entries("SELECT * FROM entries ORDER BY device, dseq", params![])?
            .into_iter()
            .filter(|e| e.dseq > vector.get(&e.device).copied().unwrap_or(0))
            .collect())
    }

    /// The (entity, field) pairs whose field starts with `prefix`, by
    /// entity and field.
    pub fn fields_with_prefix(&self, prefix: &str) -> Result<Vec<(String, String)>> {
        let db = self.db.lock();
        let mut stmt = db.prepare(
            "SELECT DISTINCT entity, field FROM entries WHERE substr(field, 1, ?) = ? \
             ORDER BY entity, field",
        )?;
        let rows = stmt.query_map(params![prefix.chars().count() as i64, prefix], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Whether any entry ever wrote `field`.
    pub fn mentions(&self, field: &str) -> Result<bool> {
        Ok(self
            .db
            .lock()
            .query_row(
                "SELECT 1 FROM entries WHERE field = ? LIMIT 1",
                [field],
                |_| Ok(()),
            )
            .optional()?
            .is_some())
    }

    /// The log's high-water mark: the local handle of the newest entry, 0
    /// for an empty log.
    pub fn current_seq(&self) -> Result<i64> {
        Ok(self
            .db
            .lock()
            .query_row("SELECT COALESCE(MAX(seq), 0) FROM entries", [], |r| {
                r.get(0)
            })?)
    }

    /// Every entry written after the mark `seq`, in log order.
    pub fn entries_after(&self, seq: i64) -> Result<Vec<Entry>> {
        self.entries(
            "SELECT * FROM entries WHERE seq > ? ORDER BY seq",
            params![seq],
        )
    }

    // ----------------------------------------------------------- moments

    /// Records a Moment at the mark `seq`, at the store's now. Returns
    /// its id.
    pub fn add_moment(&self, seq: i64, cause: &str, label: Option<&str>) -> Result<i64> {
        let db = self.db.lock();
        db.execute(
            "INSERT INTO moments (seq, cause, label, at) VALUES (?, ?, ?, ?)",
            params![seq, cause, label, dart_iso(self.now())],
        )?;
        Ok(db.last_insert_rowid())
    }

    /// Every Moment as stored, newest first.
    pub fn moment_rows(&self) -> Result<Vec<MomentRow>> {
        let db = self.db.lock();
        let mut stmt =
            db.prepare("SELECT id, seq, cause, label, at FROM moments ORDER BY seq DESC, id DESC")?;
        let rows = stmt.query_map([], |r| {
            Ok(MomentRow {
                id: r.get(0)?,
                seq: r.get(1)?,
                cause: r.get(2)?,
                label: r.get(3)?,
                at: r.get(4)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// The newest dseq held from each Device.
    pub fn version_vector(&self) -> Result<HashMap<String, i64>> {
        let db = self.db.lock();
        let mut stmt = db.prepare("SELECT device, MAX(dseq) AS m FROM entries GROUP BY device")?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Every entry of `entity`, in log order.
    pub fn history(&self, entity: &str) -> Result<Vec<Entry>> {
        self.entries(
            "SELECT * FROM entries WHERE entity = ? ORDER BY seq",
            params![entity],
        )
    }

    /// Every entry, in log order.
    pub fn all_entries(&self) -> Result<Vec<Entry>> {
        self.entries("SELECT * FROM entries ORDER BY seq", params![])
    }

    /// The current value of `field` on `entity`: latest wins.
    pub fn current(&self, entity: &str, field: &str) -> Result<Option<String>> {
        Ok(self.current_fields(entity)?.get(field).map(str::to_string))
    }

    /// The current value of every field on `entity`: latest wins.
    pub fn current_fields(&self, entity: &str) -> Result<Fields> {
        let mut latest: Vec<Entry> = Vec::new();
        for e in self.history(entity)? {
            match latest.iter_mut().find(|held| held.field == e.field) {
                Some(held) => {
                    if Self::latest_wins(&e, held) == Ordering::Greater {
                        *held = e;
                    }
                }
                None => latest.push(e),
            }
        }
        Ok(Fields(
            latest.into_iter().map(|e| (e.field, e.value)).collect(),
        ))
    }

    /// Entities whose current `$type` is `ty`.
    pub fn entities_of_type(&self, ty: &str) -> Result<Vec<String>> {
        let ids: Vec<String> = {
            let db = self.db.lock();
            let mut stmt = db.prepare(
                "SELECT DISTINCT entity FROM entries WHERE field = '$type' AND value = ?",
            )?;
            let rows = stmt.query_map([ty], |r| r.get(0))?;
            rows.collect::<rusqlite::Result<_>>()?
        };
        let mut out = Vec::new();
        for id in ids {
            if self.current(&id, "$type")?.as_deref() == Some(ty) {
                out.push(id);
            }
        }
        Ok(out)
    }

    /// Latest-wins order: date, then recording time, then device, then
    /// dseq. Sorting a field's entries by it puts the current value last.
    pub fn latest_wins(a: &Entry, b: &Entry) -> Ordering {
        a.date_at
            .cmp(&b.date_at)
            .then(a.recorded_at.cmp(&b.recorded_at))
            .then_with(|| utf16_cmp(&a.device, &b.device))
            .then(a.dseq.cmp(&b.dseq))
    }

    fn entries(&self, sql: &str, args: impl rusqlite::Params) -> Result<Vec<Entry>> {
        let db = self.db.lock();
        let mut stmt = db.prepare(sql)?;
        let rows = stmt.query_map(args, raw_row)?;
        let mut out = Vec::new();
        for row in rows {
            let (seq, wire) = row?;
            out.push(Entry {
                seq,
                date_at: parse_time(&wire.date)?,
                recorded_at: parse_time(&wire.recorded)?,
                device: wire.device,
                dseq: wire.dseq,
                entity: wire.entity,
                field: wire.field,
                value: wire.value,
                date: wire.date,
                author: wire.author,
                recorded: wire.recorded,
                sig: wire.sig,
            });
        }
        Ok(out)
    }
}

fn raw_row(r: &Row) -> rusqlite::Result<(i64, WireEntry)> {
    Ok((
        r.get("seq")?,
        WireEntry {
            device: r.get("device")?,
            dseq: r.get("dseq")?,
            entity: r.get("entity")?,
            field: r.get("field")?,
            value: r.get("value")?,
            date: r.get("date")?,
            author: r.get("author")?,
            recorded: r.get("recorded")?,
            sig: r.get("sig")?,
        },
    ))
}

/// 16 random bytes as hex, as the Dart core's ids.
fn random_id() -> Result<String> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes)
        .map_err(|e| Error::Invalid(format!("no randomness for an id: {e}")))?;
    Ok(hex::encode(bytes))
}

/// Text order as Dart's `String.compareTo` has it: by UTF-16 code units.
pub fn utf16_cmp(a: &str, b: &str) -> Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::signing::verify_signature;
    use chrono::TimeZone;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicI64, Ordering as AtomicOrdering};

    /// A clock that starts at 2026-01-01 10:00 UTC and moves one second
    /// per read.
    pub fn ticking() -> Clock {
        let ticks = Arc::new(AtomicI64::new(0));
        Box::new(move || {
            let n = ticks.fetch_add(1, AtomicOrdering::SeqCst) + 1;
            let start = Utc.with_ymd_and_hms(2026, 1, 1, 10, 0, 0).single();
            start.unwrap_or_default() + chrono::Duration::seconds(n)
        })
    }

    pub fn store() -> Store {
        match Store::in_memory(ticking(), || Ok(SigningKey::from_seed([1; 32]))) {
            Ok(s) => s,
            Err(e) => panic!("store: {e}"),
        }
    }

    #[test]
    fn own_entries_are_signed_and_verify_under_the_device_key() -> Result<()> {
        let store = store();
        store.set_local_setting("author", "Ada")?;
        let e = store.append("note:a", "text", Some("hello"), None)?;
        let bytes = entry_bytes(
            &e.device,
            e.dseq,
            &e.entity,
            &e.field,
            e.value.as_deref(),
            &e.date,
            &e.author,
            &e.recorded,
        );
        let key = store.signing_key()?;
        assert!(verify_signature(
            &key.public_key(),
            &bytes,
            e.sig.as_deref()
        ));
        assert_eq!(store.device_id()?, device_id_from_key(&key.public_key()));
        assert_eq!(e.author, "Ada");
        assert_eq!(e.date, "2026-01-01T10:00:01.000Z");
        Ok(())
    }

    #[test]
    fn the_latest_date_wins_not_the_latest_append() -> Result<()> {
        let store = store();
        let day = |d| Utc.with_ymd_and_hms(2026, 5, d, 0, 0, 0).single();
        store.append("note:a", "text", Some("new"), day(2))?;
        store.append("note:a", "text", Some("old"), day(1))?;
        assert_eq!(store.current("note:a", "text")?.as_deref(), Some("new"));
        store.append("note:a", "text", None, day(3))?;
        assert_eq!(store.current("note:a", "text")?, None);
        Ok(())
    }

    #[test]
    fn dseq_counts_up_and_a_held_entry_is_not_taken_twice() -> Result<()> {
        let store = store();
        store.append("note:a", "text", Some("1"), None)?;
        let e = store.append("note:a", "text", Some("2"), None)?;
        assert_eq!(e.dseq, 2);
        assert_eq!(store.version_vector()?.get(&e.device), Some(&2));
        assert!(!store.insert(&e.wire())?);
        let mut other = e.wire();
        other.device = "other".into();
        assert!(store.insert(&other)?);
        assert_eq!(store.all_entries()?.len(), 3);
        assert_eq!(e.id(), format!("{}:2", e.device));
        Ok(())
    }

    #[test]
    fn the_device_identity_survives_a_reopen() -> Result<()> {
        let dir = tempfile::tempdir().map_err(|e| Error::Invalid(e.to_string()))?;
        let path = dir.path().join("oldbrainfix.db");
        let first = Store::open(&path, ticking(), SigningKey::generate)?;
        let device = first.device_id()?;
        first.append("note:a", "text", Some("kept"), None)?;
        drop(first);
        let again = Store::open(&path, ticking(), || {
            Err(Error::Invalid("no new key on reopen".into()))
        })?;
        assert_eq!(again.device_id()?, device);
        assert_eq!(again.current("note:a", "text")?.as_deref(), Some("kept"));
        Ok(())
    }

    #[test]
    fn ids_are_random_unless_counted_and_a_failed_batch_leaves_nothing() -> Result<()> {
        let mut store = store();
        let a = store.new_id()?;
        assert_eq!(a.len(), 32);
        assert_ne!(a, store.new_id()?);
        store.set_id_maker(Box::new(|| Ok("1".into())));
        assert_eq!(store.new_id()?, "1");
        let failed: Result<()> = store.batch(|| {
            store.append("note:a", "text", Some("lost"), None)?;
            Err(Error::Invalid("on purpose".into()))
        });
        assert!(failed.is_err());
        assert!(store.all_entries()?.is_empty());
        store.batch(|| store.append("note:a", "text", Some("kept"), None))?;
        assert_eq!(store.all_entries()?.len(), 1);
        Ok(())
    }

    #[test]
    fn blobs_lie_next_to_the_database_by_their_hash() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let store = Store::open(&dir.path().join("oldbrainfix.db"), ticking(), || {
            Ok(SigningKey::from_seed([1; 32]))
        })?;
        let photo = store.put_photo(b"not a jpeg")?;
        let audio = store.put_audio(b"RIFF....WAVE")?;
        assert_eq!(
            photo,
            hex::encode(Sha256::digest(b"not a jpeg")),
            "the photo's hash"
        );
        assert_eq!(
            std::fs::read(dir.path().join("photos").join(&photo))?,
            b"not a jpeg"
        );
        assert_eq!(
            std::fs::read(dir.path().join("audio").join(&audio))?,
            b"RIFF....WAVE"
        );
        assert_eq!(store.photo(&photo).as_deref(), Some(&b"not a jpeg"[..]));
        assert_eq!(store.audio(&photo), None);
        assert!(store.all_entries()?.is_empty());
        Ok(())
    }

    #[test]
    fn text_orders_by_utf16_code_units_like_dart() {
        // U+FF5E sorts after U+1F600 in UTF-16, before it by code point.
        assert_eq!(utf16_cmp("\u{FF5E}", "\u{1F600}"), Ordering::Greater);
        assert_eq!(utf16_cmp("a", "b"), Ordering::Less);
    }
}
