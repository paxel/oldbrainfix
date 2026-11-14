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
//! Photo, voice recording and attached file bytes live outside the log,
//! content-addressed by SHA-256: `photos`, `audio` and `files`
//! directories next to the database file, as the Dart core keeps them,
//! or memory for an in-memory store.
//!
//! Pending Changes (ADR 0022) lie beside it, in a `pending` directory:
//! every read of what the log holds now ([`Store::history`],
//! [`Store::current_fields`] and the lists of entities and fields) sees
//! the shown sets on top of it; what is held, passed on, numbered or
//! trusted reads the log alone.
//!
//! What an AI Task does (ADR 0026) goes through the same writes as the
//! user's: on the thread that answers it, inside [`Store::drafting`],
//! every write becomes a draft of its pending set instead of an entry.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::Path;

use chrono::{DateTime, Utc};
use std::sync::Arc;

use parking_lot::{Mutex, ReentrantMutex};
use rusqlite::{Connection, OptionalExtension, Row, params};
use sha2::{Digest, Sha256};

use crate::blobs::{Blobs, FileBlobs, MemoryBlobs};
use crate::conflicts::{conflict_unit, conflict_unit_fields, encode_context};
use crate::entry::{Entry, WireEntry, dart_iso, parse_time};
use crate::pending::{
    FilePendingFiles, GiveWay, MemoryPendingFiles, PendingChange, PendingFiles, PendingSet,
    PendingState,
};
use crate::photos::strip_jpeg_metadata;
use crate::shared::{JudgedLog, judge_log};
use crate::signing::{SigningKey, device_id_from_key, entry_bytes};
use crate::{Error, Result};

/// Where the store reads "now".
pub type Clock = Box<dyn Fn() -> DateTime<Utc> + Send + Sync>;

/// Where the store gets the ids of new entities: random, unless a test
/// counts them as the Dart core's fixture generator does.
pub type IdMaker = Box<dyn Fn() -> Result<String> + Send + Sync>;

/// The entries this version applies (ADR 0019), as the Dart core's
/// `_applied`: those of a later structure are only held and passed on.
const APPLIED: &str = "(v IS NULL OR v <= 1)";

/// How many entities one query of [`Store::current_fields_of`] names,
/// well under SQLite's limit of bound values.
const IN_CHUNK: usize = 500;

/// The version of the tables this version keeps the log in, recorded as
/// the database's `user_version` (ADR 0034), as the Dart core's
/// `storeVersion`. 0, SQLite's own default, is a database from before the
/// version was recorded: the same tables.
pub const STORE_VERSION: i64 = 1;

/// The version `db` was saved with; [`Error::StoreNeedsUpdate`] for one
/// of a newer version. Reads only, so such a database stays as it was.
fn saved_version(db: &Connection) -> Result<i64> {
    let version: i64 = db.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version != 0 && version != STORE_VERSION {
        return Err(Error::StoreNeedsUpdate(version));
    }
    Ok(version)
}

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
        sig      TEXT,
        ctx      TEXT,
        raw      TEXT,
        v        INTEGER
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

/// What [`Store::worked_out`] keeps, by key, and the log's high-water
/// mark it was worked out at.
#[derive(Default)]
struct WorkedOut {
    seq: i64,
    held: HashMap<&'static str, Arc<dyn std::any::Any + Send + Sync>>,
}

/// The current fields of one entity, in the order they were first
/// written, as the Dart core's `currentFields` map keeps them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Fields(Vec<(String, Option<String>)>);

impl Fields {
    /// Fields holding `values`, in their order.
    pub fn new(values: Vec<(String, Option<String>)>) -> Fields {
        Fields(values)
    }

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

/// The local settings of the refusal marks, before the device.
const REFUSED_PREFIX: &str = "refused:";

/// The dseqs `from` to `to` of one Device, all refused, whose entities
/// lay in `scope`: an Area, or `personal`; the Dart core's `RefusedRun`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefusedRun {
    pub from: i64,
    pub to: i64,
    pub scope: String,
}

/// Which entries of a blocked person's Device were refused here (ADR
/// 0021), as the Dart core's `RefusalMark`: the runs of dseqs, by the
/// scope their entities lay in, and whether the person was unblocked
/// since, so those entries are wanted again.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RefusalMark {
    pub runs: Vec<RefusedRun>,
    pub released: bool,
}

impl RefusalMark {
    /// The newest dseq refused; 0 for none.
    pub fn high(&self) -> i64 {
        self.runs.iter().map(|r| r.to).max().unwrap_or(0)
    }

    /// This mark refusing again, with `dseq` of `scope` among its runs.
    pub fn adding(self, dseq: i64, scope: &str) -> RefusalMark {
        let mut scopes: std::collections::BTreeMap<i64, String> = std::collections::BTreeMap::new();
        for run in self.runs {
            for n in run.from..=run.to {
                scopes.insert(n, run.scope.clone());
            }
        }
        scopes.insert(dseq, scope.to_string());
        let mut runs: Vec<RefusedRun> = Vec::new();
        for (n, scope) in scopes {
            match runs.last_mut() {
                Some(last) if last.to == n - 1 && last.scope == scope => last.to = n,
                _ => runs.push(RefusedRun {
                    from: n,
                    to: n,
                    scope,
                }),
            }
        }
        RefusalMark {
            runs,
            released: false,
        }
    }

    /// Keys sorted, no spaces, as both cores keep it.
    pub fn encode(&self) -> String {
        let runs: Vec<serde_json::Value> = self
            .runs
            .iter()
            .map(|r| serde_json::json!({"from": r.from, "scope": r.scope, "to": r.to}))
            .collect();
        // Built in key order: a workspace build turns on serde_json's
        // preserve_order, which keeps keys as inserted.
        let json = if self.released {
            serde_json::json!({ "released": true, "runs": runs })
        } else {
            serde_json::json!({ "runs": runs })
        };
        json.to_string()
    }

    /// The mark `text` holds; none when it is none.
    pub fn parse(text: &str) -> Option<RefusalMark> {
        let json: serde_json::Value = serde_json::from_str(text).ok()?;
        let mut runs = Vec::new();
        for r in json.get("runs")?.as_array()? {
            let from = r.get("from")?.as_i64()?;
            let to = r.get("to")?.as_i64()?;
            let scope = r.get("scope")?.as_str()?;
            if from < 1 || to < from {
                return None;
            }
            runs.push(RefusedRun {
                from,
                to,
                scope: scope.to_string(),
            });
        }
        Some(RefusalMark {
            runs,
            released: json.get("released") == Some(&serde_json::Value::Bool(true)),
        })
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
    /// The files attached to Notes (ADR 0047), kept as they are.
    files: Box<dyn Blobs>,
    /// What the roles of the Shared Areas allow, worked out after every
    /// change ([`Store::judged`]).
    judged: Mutex<Option<Arc<JudgedLog>>>,
    /// Counts the changes, so a judgement worked out while another
    /// thread wrote is not kept.
    generation: std::sync::atomic::AtomicU64,
    /// What [`Store::worked_out`] keeps until the next change, by key.
    worked_out: Mutex<WorkedOut>,
    /// How often [`Store::history`] was read: what a test counts reads
    /// by.
    #[cfg(test)]
    pub(crate) history_reads: std::sync::atomic::AtomicUsize,
    /// Other people's Profiles as read from the local settings, until
    /// one is kept ([`crate::people`]).
    pub(crate) people: Mutex<Option<Arc<crate::people::PeopleIndex>>>,
    /// The own Profile's ids and keys, with what they were read from.
    pub(crate) own_keys: Mutex<Option<Arc<crate::people::OwnKeys>>>,
    /// Where the pending sets' files lie.
    pending_files: Box<dyn PendingFiles>,
    /// The pending sets held, and what is worked out from the shown ones
    /// after every change. Never held while the connection is taken.
    pending: Mutex<PendingState>,
    /// The pending set each thread drafts into ([`Store::drafting`]), the
    /// threads that read the log alone ([`Store::log_only`]) and those
    /// that read as the AI ([`Store::for_ai`]).
    pub(crate) scopes: Mutex<crate::ai_task::Scopes>,
}

impl Store {
    /// Opens an on-disk store, creating it if needed. `new_key` makes the
    /// Device's signing key on first open.
    pub fn open(
        path: &Path,
        clock: Clock,
        new_key: impl FnOnce() -> Result<SigningKey>,
    ) -> Result<Store> {
        let db = Connection::open(path)?;
        // Before the directories beside it are made: a database of a
        // newer version leaves everything as it was.
        saved_version(&db)?;
        let dir = path.parent().unwrap_or(Path::new("."));
        let photos = Box::new(FileBlobs::new(dir.join("photos"))?);
        let audio = Box::new(FileBlobs::new(dir.join("audio"))?);
        let files = Box::new(FileBlobs::new(dir.join("files"))?);
        let pending = Box::new(FilePendingFiles::new(dir.join("pending")));
        Self::init(db, clock, new_key, [photos, audio, files], pending)
    }

    /// Opens a throwaway in-memory store (tests).
    pub fn in_memory(clock: Clock, new_key: impl FnOnce() -> Result<SigningKey>) -> Result<Store> {
        let photos = Box::<MemoryBlobs>::default();
        let audio = Box::<MemoryBlobs>::default();
        let files = Box::<MemoryBlobs>::default();
        let pending = Box::<MemoryPendingFiles>::default();
        Self::init(
            Connection::open_in_memory()?,
            clock,
            new_key,
            [photos, audio, files],
            pending,
        )
    }

    fn init(
        db: Connection,
        clock: Clock,
        new_key: impl FnOnce() -> Result<SigningKey>,
        [photos, audio, files]: [Box<dyn Blobs>; 3],
        pending_files: Box<dyn PendingFiles>,
    ) -> Result<Store> {
        // Read before anything is written: a database of a newer version
        // is left exactly as it was.
        let version = saved_version(&db)?;
        // Another connection's write lock is waited for, as long as the
        // Dart core waits (#207), then the write fails instead of hanging.
        db.busy_timeout(std::time::Duration::from_secs(5))?;
        // The pragma answers with the mode it took; memory stays memory.
        db.query_row("PRAGMA journal_mode = WAL", [], |_| Ok(()))?;
        db.execute_batch(SCHEMA)?;
        // Today's tables: a database from before the version was recorded
        // gets it. A later version adds its steps here, one per version.
        if version == 0 {
            db.execute_batch(&format!("PRAGMA user_version = {STORE_VERSION}"))?;
        }
        let store = Store {
            db: ReentrantMutex::new(db),
            clock,
            ids: Box::new(random_id),
            photos,
            audio,
            files,
            judged: Mutex::new(None),
            generation: std::sync::atomic::AtomicU64::new(0),
            worked_out: Mutex::new(WorkedOut::default()),
            #[cfg(test)]
            history_reads: std::sync::atomic::AtomicUsize::new(0),
            people: Mutex::new(None),
            own_keys: Mutex::new(None),
            pending_files,
            pending: Mutex::new(PendingState::default()),
            scopes: Mutex::new(crate::ai_task::Scopes::default()),
        };
        if store.local_setting("device")?.is_none() {
            let key = new_key()?;
            store.set_local_setting("device", &device_id_from_key(&key.public_key()))?;
            store.set_local_setting("signseed", &key.seed_hex())?;
        }
        store.load_pending()?;
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

    /// Runs `body` in one transaction: all of its entries or none. A
    /// batch inside another one joins it: the outer one commits, or rolls
    /// back on an error from anywhere inside, as the Dart core's `batch`.
    /// It takes the database's write lock as it begins.
    pub fn batch<T>(&self, body: impl FnOnce() -> Result<T>) -> Result<T> {
        let db = self.db.lock();
        // The outer batch holds the connection until it ends, so an open
        // transaction here is this thread's own.
        if !db.is_autocommit() {
            return body();
        }
        // The write lock from the start: another connection's batch waits
        // until this one is done (#207), and what `body` reads is still
        // so when it writes.
        let tx =
            rusqlite::Transaction::new_unchecked(&db, rusqlite::TransactionBehavior::Immediate)?;
        let out = body();
        // Rolled back or not, what was worked out from inside is stale.
        self.changed();
        let out = out?;
        tx.commit()?;
        Ok(out)
    }

    /// Another connection to the same database wrote, such as the
    /// automatic sync's on a thread of its own (#225): what was worked
    /// out from the log and read from the local settings is forgotten, as
    /// the Dart core's `changedElsewhere`, so the next read sees it.
    pub fn changed_elsewhere(&self) {
        self.changed();
        *self.people.lock() = None;
        *self.own_keys.lock() = None;
    }

    /// Forgets what was worked out from the log.
    fn changed(&self) {
        let mut judged = self.judged.lock();
        self.generation
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        *judged = None;
        let mut pending = self.pending.lock();
        pending.overlay = None;
        pending.judged = None;
        self.worked_out.lock().held.clear();
    }

    /// What `build` works out from the reads, kept under `key` until what
    /// they show changes, as the Dart core's `workedOut`: the log, or the
    /// shown pending sets. An entry another store open on the same file
    /// wrote counts too, by the log's high-water mark. Inside
    /// [`Store::log_only`], which shows the log alone, and inside
    /// [`Store::for_ai`], which leaves out what is Hidden from AI, it is
    /// worked out anew and not kept.
    pub(crate) fn worked_out<T: Send + Sync + 'static>(
        &self,
        key: &'static str,
        build: impl FnOnce() -> Result<T>,
    ) -> Result<Arc<T>> {
        let fresh = self.reads_log_only() || self.reads_for_ai();
        // Read before the lock is taken: a write takes the connection
        // first, then this lock.
        let seq = self.current_seq()?;
        if !fresh {
            let mut worked = self.worked_out.lock();
            if worked.seq != seq {
                worked.seq = seq;
                worked.held.clear();
            }
            if let Some(held) = worked.held.get(key).cloned()
                && let Ok(held) = held.downcast::<T>()
            {
                return Ok(held);
            }
        }
        let generation = self.generation.load(std::sync::atomic::Ordering::SeqCst);
        let value = Arc::new(build()?);
        if !fresh {
            let after = self.current_seq()?;
            let mut worked = self.worked_out.lock();
            // Kept only when nothing changed while it was worked out.
            if self.generation.load(std::sync::atomic::Ordering::SeqCst) == generation
                && after == seq
                && worked.seq == seq
            {
                worked.held.insert(key, value.clone());
            }
        }
        Ok(value)
    }

    /// What the roles of the Shared Areas allow (ADR 0013), worked out
    /// from the whole log after every change, as the Dart core's
    /// `judged`; nothing to work out while no Area was ever shared.
    /// Pending Changes never count here: this is what decides who may do
    /// what, what is refused and what is deleted.
    pub fn judged(&self) -> Result<Arc<JudgedLog>> {
        if let Some(held) = self.judged.lock().as_ref() {
            return Ok(held.clone());
        }
        let generation = self.generation.load(std::sync::atomic::Ordering::SeqCst);
        let shared = self
            .db
            .lock()
            .query_row(
                &format!(
                    "SELECT 1 FROM entries WHERE {APPLIED} AND substr(field, 1, 5) = 'role:' LIMIT 1"
                ),
                [],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        let judged = Arc::new(if shared {
            judge_log(&self.entries(
                &format!("SELECT * FROM entries WHERE {APPLIED} ORDER BY seq"),
                params![],
            )?)
        } else {
            JudgedLog::default()
        });
        let mut held = self.judged.lock();
        if self.generation.load(std::sync::atomic::Ordering::SeqCst) == generation {
            *held = Some(judged.clone());
        }
        Ok(judged)
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

    /// Sets `key` to `value`, unless what `key` holds a later version of
    /// the app wrote ([`later_setting`]): that stays as it is, and an
    /// update notice says so ([`crate::sync::SETTINGS_SOURCE`]), as the
    /// Dart core's `setLocalSetting`.
    pub fn set_local_setting(&self, key: &str, value: &str) -> Result<()> {
        self.refuse_while_drafting()?;
        if self.keeps_later(key)? {
            return Ok(());
        }
        self.db.lock().execute(
            "INSERT OR REPLACE INTO local_settings (key, value) VALUES (?, ?)",
            [key, value],
        )?;
        Ok(())
    }

    /// Removes `key`, unless a later version wrote it, as
    /// [`Store::set_local_setting`].
    pub fn remove_local_setting(&self, key: &str) -> Result<()> {
        self.refuse_while_drafting()?;
        if self.keeps_later(key)? {
            return Ok(());
        }
        self.db
            .lock()
            .execute("DELETE FROM local_settings WHERE key = ?", [key])?;
        Ok(())
    }

    /// Whether `key` holds a setting of a later version, kept as it is;
    /// noted for the update notice. The own Profile's record has a rule
    /// of its own (ADR 0034): another of the user's Devices may replace
    /// it.
    fn keeps_later(&self, key: &str) -> Result<bool> {
        if key == crate::profile::RECORD {
            return Ok(false);
        }
        if !self
            .local_setting(key)?
            .is_some_and(|held| later_setting(&held))
        {
            return Ok(false);
        }
        self.note_update_needed(
            crate::sync::SETTINGS_SOURCE,
            crate::sync::UpdateNeeded::Here,
        )?;
        Ok(true)
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
        self.refuse_while_drafting()?;
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
        self.refuse_while_drafting()?;
        let hash = hex::encode(Sha256::digest(bytes));
        self.audio.put(&hash, bytes)?;
        Ok(hash)
    }

    /// The bytes of the voice recording `hash`, or `None` when they are
    /// not here.
    pub fn audio(&self, hash: &str) -> Option<Vec<u8>> {
        self.audio.get(hash)
    }

    // ------------------------------------------------------------- files

    /// Keeps the bytes of a file attached to a Note as they are and
    /// returns the hash they are kept under (ADR 0047). The same file
    /// twice is kept once.
    pub fn put_file(&self, bytes: &[u8]) -> Result<String> {
        self.refuse_while_drafting()?;
        let hash = hex::encode(Sha256::digest(bytes));
        self.files.put(&hash, bytes)?;
        Ok(hash)
    }

    /// The bytes of the attached file `hash`, or `None` when they are
    /// not here.
    pub fn file(&self, hash: &str) -> Option<Vec<u8>> {
        self.files.get(hash)
    }

    /// The blobs of `kind`: `photo`, `audio` or `file`.
    fn blobs_of(&self, kind: &str) -> Option<&dyn Blobs> {
        match kind {
            "photo" => Some(self.photos.as_ref()),
            "audio" => Some(self.audio.as_ref()),
            "file" => Some(self.files.as_ref()),
            _ => None,
        }
    }

    /// Whether the bytes of the `kind` (`photo`, `audio` or `file`) blob
    /// `hash` are here, without reading them.
    pub fn has_blob(&self, kind: &str, hash: &str) -> bool {
        self.blobs_of(kind).is_some_and(|b| b.has(hash))
    }

    /// The bytes of the `kind` (`photo`, `audio` or `file`) blob `hash`.
    pub fn blob(&self, kind: &str, hash: &str) -> Option<Vec<u8>> {
        self.blobs_of(kind)?.get(hash)
    }

    /// Lets go of the bytes of the `kind` blob `hash`; returns whether
    /// they were here. Only the log says when that is right (ADR 0020).
    pub fn drop_blob(&self, kind: &str, hash: &str) -> Result<bool> {
        self.refuse_while_drafting()?;
        match self.blobs_of(kind) {
            Some(blobs) => blobs.remove(hash),
            None => Ok(false),
        }
    }

    /// Keeps `bytes` another Device holds under `hash`, as they are.
    /// Bytes whose SHA-256 is not `hash`, or of an unknown `kind`, are
    /// not kept; returns whether they were.
    pub fn put_blob(&self, kind: &str, hash: &str, bytes: &[u8]) -> Result<bool> {
        self.refuse_while_drafting()?;
        if hex::encode(Sha256::digest(bytes)) != hash {
            return Ok(false);
        }
        match self.blobs_of(kind) {
            Some(blobs) => blobs.put(hash, bytes)?,
            None => return Ok(false),
        }
        Ok(true)
    }

    // --------------------------------------------------------------- log

    /// Appends one fact under this Device, signed as the Dart core signs.
    /// `date` defaults to now. An edit to a field concurrent edits are a
    /// Conflict on carries its causal context ([`Entry::ctx`]). Inside
    /// [`Store::drafting`] it is drafted into the AI Task instead, as the
    /// draft shows ([`Store::draft_entry`]).
    pub fn append(
        &self,
        entity: &str,
        field: &str,
        value: Option<&str>,
        date: Option<DateTime<Utc>>,
    ) -> Result<Entry> {
        if let Some(set) = self.drafting_into() {
            return self.draft_entry(&set, entity, field, value, date);
        }
        let db = self.db.lock();
        // Its number read and taken in one transaction, so another
        // connection's append waits for it instead of taking the same
        // number (#207).
        if db.is_autocommit() {
            drop(db);
            return self.batch(|| self.append(entity, field, value, date));
        }
        let now = self.now();
        let device = self.device_id()?;
        let vector = self.held_vector()?;
        let dseq = vector.get(&device).copied().unwrap_or(0) + 1;
        let ctx = match conflict_unit(field) {
            Some(unit) => self.context(entity, unit, &device, &vector)?,
            None => None,
        };
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
            ctx.as_deref(),
        ));
        db.execute(
            "INSERT INTO entries (device, dseq, entity, field, value, date, author, recorded, sig, ctx) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                device,
                dseq,
                entity,
                field,
                value,
                date_iso,
                author,
                recorded_iso,
                sig,
                ctx
            ],
        )?;
        self.changed();
        self.entries(
            "SELECT * FROM entries WHERE device = ? AND dseq = ?",
            params![device, dseq],
        )?
        .pop()
        .ok_or_else(|| Error::Invalid("the appended entry is gone".into()))
    }

    /// The causal context of a write to `unit` on `entity` by `device`
    /// holding `vector`: how far it has seen every other Device that
    /// wrote the unit here; `None` when nobody wrote it yet. What the
    /// shown pending sets hold of other Devices counts as seen (ADR
    /// 0022): the user saw it when they wrote.
    fn context(
        &self,
        entity: &str,
        unit: &str,
        device: &str,
        vector: &HashMap<String, i64>,
    ) -> Result<Option<String>> {
        let fields = conflict_unit_fields(unit);
        let shown = self.shown()?;
        let received: Vec<&Entry> = shown.iter().filter(|e| e.device != device).collect();
        let mut seen = vector.clone();
        for e in &received {
            let held = seen.entry(e.device.clone()).or_insert(0);
            if e.dseq > *held {
                *held = e.dseq;
            }
        }
        let mut writers: Vec<String> = {
            let db = self.db.lock();
            let marks = vec!["?"; fields.len()].join(", ");
            let mut stmt = db.prepare(&format!(
                "SELECT DISTINCT device FROM entries WHERE {APPLIED} AND entity = ? \
                 AND field IN ({marks})"
            ))?;
            let mut args: Vec<&str> = vec![entity];
            args.extend(fields.iter().copied());
            let rows = stmt.query_map(rusqlite::params_from_iter(args), |r| r.get(0))?;
            rows.collect::<rusqlite::Result<_>>()?
        };
        for e in &received {
            if e.entity == entity
                && fields.contains(&e.field.as_str())
                && !writers.contains(&e.device)
            {
                writers.push(e.device.clone());
            }
        }
        if writers.is_empty() {
            return Ok(None);
        }
        let seen: Vec<(String, i64)> = writers
            .into_iter()
            .filter(|d| d != device)
            .map(|d| {
                let n = seen.get(&d).copied().unwrap_or(0);
                (d, n)
            })
            .collect();
        Ok(Some(encode_context(&seen)))
    }

    /// Which Device wrote which of `fields` on which entity, as (entity,
    /// field, device), each once.
    pub fn writers_of(&self, fields: &[&str]) -> Result<Vec<(String, String, String)>> {
        let mut logged: Vec<(String, String, String)> = {
            let db = self.db.lock();
            let marks = vec!["?"; fields.len()].join(", ");
            let mut stmt = db.prepare(&format!(
                "SELECT DISTINCT entity, field, device FROM entries WHERE {APPLIED} AND field IN ({marks})"
            ))?;
            let rows = stmt.query_map(rusqlite::params_from_iter(fields.iter()), |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })?;
            rows.collect::<rusqlite::Result<_>>()?
        };
        let mut seen: std::collections::HashSet<(String, String, String)> =
            logged.iter().cloned().collect();
        for e in self.shown()?.iter() {
            if !fields.contains(&e.field.as_str()) {
                continue;
            }
            let key = (e.entity.clone(), e.field.clone(), e.device.clone());
            if seen.insert(key.clone()) {
                logged.push(key);
            }
        }
        Ok(logged)
    }

    /// Takes in an entry another Device wrote, as it is; one already held
    /// is skipped. Returns whether it was new. What it came as is kept
    /// when this version would write it otherwise, and its version when
    /// it is not 1.
    pub fn insert(&self, entry: &WireEntry) -> Result<bool> {
        self.refuse_while_drafting()?;
        let v = entry.v();
        // Keys of a later version live only in what the entry came as.
        let raw = match &entry.raw {
            None if !entry.extra.is_empty() => Some(entry.line()?),
            raw => raw.clone(),
        };
        let changed = self.db.lock().execute(
            "INSERT OR IGNORE INTO entries (device, dseq, entity, field, value, date, author, recorded, sig, ctx, raw, v) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                entry.device,
                entry.dseq,
                entry.entity,
                entry.field,
                entry.value,
                entry.date,
                entry.author,
                entry.recorded,
                entry.sig,
                entry.ctx,
                raw,
                (v != 1).then_some(v)
            ],
        )?;
        if changed > 0 {
            self.changed();
        }
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
        let mut logged: Vec<(String, String)> = {
            let db = self.db.lock();
            let mut stmt = db.prepare(&format!(
                "SELECT DISTINCT entity, field FROM entries WHERE {APPLIED} \
                     AND substr(field, 1, ?) = ? ORDER BY entity, field"
            ))?;
            let rows = stmt.query_map(params![prefix.chars().count() as i64, prefix], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })?;
            rows.collect::<rusqlite::Result<_>>()?
        };
        let mut seen: std::collections::HashSet<(String, String)> =
            logged.iter().cloned().collect();
        let mut more = false;
        for e in self.shown()?.iter() {
            if e.field.starts_with(prefix) && seen.insert((e.entity.clone(), e.field.clone())) {
                logged.push((e.entity.clone(), e.field.clone()));
                more = true;
            }
        }
        // As SQLite orders text: by its bytes.
        if more {
            logged.sort();
        }
        if self.reads_for_ai() {
            let mut left = Vec::with_capacity(logged.len());
            for (entity, field) in logged {
                if !self.ai_hides(&entity)? {
                    left.push((entity, field));
                }
            }
            logged = left;
        }
        Ok(logged)
    }

    /// Whether any entry ever wrote `field`.
    pub fn mentions(&self, field: &str) -> Result<bool> {
        let logged = self
            .db
            .lock()
            .query_row(
                &format!("SELECT 1 FROM entries WHERE {APPLIED} AND field = ? LIMIT 1"),
                [field],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        Ok(logged || self.shown()?.iter().any(|e| e.field == field))
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

    /// Records a restored Moment at the mark `seq`, at `at` as it was
    /// stored. Returns its id.
    pub fn add_moment_at(
        &self,
        seq: i64,
        cause: &str,
        label: Option<&str>,
        at: &str,
    ) -> Result<i64> {
        let db = self.db.lock();
        db.execute(
            "INSERT INTO moments (seq, cause, label, at) VALUES (?, ?, ?, ?)",
            params![seq, cause, label, at],
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

    /// How far this Device holds each Device's entries, as it tells
    /// other Devices, as the Dart core's `versionVector`: the newest dseq
    /// held, raised to the newest a blocked person's Device had refused,
    /// so nobody sends them again; after an unblock, below the first
    /// refused one that is not back yet and lay in a Shared Area this
    /// Device still holds a role in, so they are sent again (ADR 0021).
    pub fn version_vector(&self) -> Result<HashMap<String, i64>> {
        let mut vector = self.held_vector()?;
        for (device, mark) in self.refusal_marks()? {
            let held = vector.get(&device).copied().unwrap_or(0);
            if !mark.released {
                if mark.high() > held {
                    vector.insert(device, mark.high());
                }
                continue;
            }
            let Some(missing) = self.first_wanted(&device, &mark)? else {
                continue;
            };
            if missing > held {
                continue;
            }
            if missing - 1 > 0 {
                vector.insert(device, missing - 1);
            } else {
                vector.remove(&device);
            }
        }
        Ok(vector)
    }

    /// The first dseq of `mark`'s runs not held here whose scope this
    /// Device holds a role in; none when there is none.
    fn first_wanted(&self, device: &str, mark: &RefusalMark) -> Result<Option<i64>> {
        for run in &mark.runs {
            if !self.wants(run)? {
                continue;
            }
            if let Some(missing) = self.first_missing(device, run.from, run.to)? {
                return Ok(Some(missing));
            }
        }
        Ok(None)
    }

    /// Whether what was refused in `run` can come back: it lay in a
    /// Shared Area this Device holds a role in now.
    fn wants(&self, run: &RefusedRun) -> Result<bool> {
        let own = self.device_id()?;
        Ok(self
            .judged()?
            .areas
            .get(&run.scope)
            .is_some_and(|grants| grants.grant_of(&own).is_some()))
    }

    /// The newest dseq held from each Device.
    fn held_vector(&self) -> Result<HashMap<String, i64>> {
        let db = self.db.lock();
        let mut stmt = db.prepare("SELECT device, MAX(dseq) AS m FROM entries GROUP BY device")?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// The lowest dseq of `device` from `low` to `high` not held; none
    /// when all of them are.
    fn first_missing(&self, device: &str, low: i64, high: i64) -> Result<Option<i64>> {
        let held: Vec<i64> = {
            let db = self.db.lock();
            let mut stmt = db.prepare(
                "SELECT dseq FROM entries WHERE device = ? AND dseq BETWEEN ? AND ? ORDER BY dseq",
            )?;
            let rows = stmt.query_map(params![device, low, high], |r| r.get(0))?;
            rows.collect::<rusqlite::Result<_>>()?
        };
        let mut next = low;
        for dseq in held {
            if dseq != next {
                return Ok(Some(next));
            }
            next += 1;
        }
        Ok((next <= high).then_some(next))
    }

    // ---------------------------------------------------------- refusals

    /// Which entries of each of a blocked person's Devices were refused
    /// (ADR 0021), by device, as the Dart core's `refusalMarks`: a local
    /// setting of this Device, since each of the user's Devices refuses
    /// on its own. Released once the person was unblocked, until what was
    /// refused is back.
    pub fn refusal_marks(&self) -> Result<Vec<(String, RefusalMark)>> {
        Ok(self
            .local_settings_with_prefix(REFUSED_PREFIX)?
            .into_iter()
            .filter_map(|(key, value)| {
                let device = key.strip_prefix(REFUSED_PREFIX)?.to_string();
                RefusalMark::parse(&value).map(|mark| (device, mark))
            })
            .collect())
    }

    fn refusal_mark(&self, device: &str) -> Result<Option<RefusalMark>> {
        Ok(self
            .local_setting(&format!("{REFUSED_PREFIX}{device}"))?
            .and_then(|value| RefusalMark::parse(&value)))
    }

    fn keep_refusal_mark(&self, device: &str, mark: &RefusalMark) -> Result<()> {
        let key = format!("{REFUSED_PREFIX}{device}");
        if mark.runs.is_empty() {
            self.remove_local_setting(&key)
        } else {
            self.set_local_setting(&key, &mark.encode())
        }
    }

    /// Notes that the entry (`device`, `dseq`) was refused, its entity
    /// lying in `scope` (an Area, or the personal scope): the mark of
    /// `device` takes it in, and counts as refusing again.
    pub fn note_refused(&self, device: &str, dseq: i64, scope: &str) -> Result<()> {
        let held = self.refusal_mark(device)?;
        let mark = held.clone().unwrap_or_default().adding(dseq, scope);
        if held.as_ref() == Some(&mark) {
            return Ok(());
        }
        self.keep_refusal_mark(device, &mark)
    }

    /// Lets `device`'s mark go: what it refused is wanted again, until it
    /// is all back ([`Store::drop_filled_refusals`]). Returns whether it
    /// was refusing.
    pub fn release_refusals(&self, device: &str) -> Result<bool> {
        let Some(held) = self.refusal_mark(device)? else {
            return Ok(false);
        };
        if held.released {
            return Ok(false);
        }
        self.keep_refusal_mark(
            device,
            &RefusalMark {
                released: true,
                ..held
            },
        )?;
        Ok(true)
    }

    /// Makes `device`'s released mark refuse again.
    pub fn hold_refusals(&self, device: &str) -> Result<()> {
        match self.refusal_mark(device)? {
            Some(held) if held.released => self.keep_refusal_mark(
                device,
                &RefusalMark {
                    released: false,
                    ..held
                },
            ),
            _ => Ok(()),
        }
    }

    /// Forgets, of every released mark, the runs that are back and those
    /// that cannot come back: lying in no Shared Area this Device holds a
    /// role in now. A mark with nothing left goes.
    pub fn drop_filled_refusals(&self) -> Result<()> {
        for (device, mark) in self.refusal_marks()? {
            if !mark.released {
                continue;
            }
            let mut left = Vec::new();
            for run in &mark.runs {
                if self.wants(run)? && self.first_missing(&device, run.from, run.to)?.is_some() {
                    left.push(run.clone());
                }
            }
            if left.len() == mark.runs.len() {
                continue;
            }
            self.keep_refusal_mark(
                &device,
                &RefusalMark {
                    runs: left,
                    released: true,
                },
            )?;
        }
        Ok(())
    }

    /// Every entry `device` wrote that is held here, by dseq.
    pub fn entries_of(&self, device: &str) -> Result<Vec<Entry>> {
        self.entries(
            "SELECT * FROM entries WHERE device = ? ORDER BY dseq",
            params![device],
        )
    }

    /// Deletes the entries `dseqs` of `device` from the log: only a block
    /// does (ADR 0021), for what a blocked person wrote. Nothing else ever
    /// leaves the log.
    pub fn delete_entries(&self, device: &str, dseqs: &[i64]) -> Result<()> {
        self.refuse_while_drafting()?;
        let mut deleted = 0;
        {
            let db = self.db.lock();
            for dseq in dseqs {
                deleted += db.execute(
                    "DELETE FROM entries WHERE device = ? AND dseq = ?",
                    params![device, dseq],
                )?;
            }
        }
        if deleted > 0 {
            self.changed();
        }
        Ok(())
    }

    /// The entities whose id starts with `prefix`, sorted.
    pub fn entities_with_prefix(&self, prefix: &str) -> Result<Vec<String>> {
        let mut found = self.entities_with_prefix_all(prefix)?;
        if self.reads_for_ai() {
            found = self.left_for_ai(found)?;
        }
        Ok(found)
    }

    /// What the AI sees of `ids` ([`Store::for_ai`]): those not Hidden
    /// from AI.
    fn left_for_ai(&self, ids: Vec<String>) -> Result<Vec<String>> {
        let mut left = Vec::with_capacity(ids.len());
        for id in ids {
            if !self.ai_hides(&id)? {
                left.push(id);
            }
        }
        Ok(left)
    }

    fn entities_with_prefix_all(&self, prefix: &str) -> Result<Vec<String>> {
        let mut logged: Vec<String> = {
            let db = self.db.lock();
            let mut stmt = db.prepare(&format!(
                "SELECT DISTINCT entity FROM entries WHERE {APPLIED} \
                     AND substr(entity, 1, ?) = ? ORDER BY entity"
            ))?;
            let rows =
                stmt.query_map(params![prefix.chars().count() as i64, prefix], |r| r.get(0))?;
            rows.collect::<rusqlite::Result<_>>()?
        };
        let mut seen: std::collections::HashSet<String> = logged.iter().cloned().collect();
        let mut more = false;
        for e in self.shown()?.iter() {
            if e.entity.starts_with(prefix) && seen.insert(e.entity.clone()) {
                logged.push(e.entity.clone());
                more = true;
            }
        }
        if more {
            logged.sort();
        }
        Ok(logged)
    }

    /// Every entry of `entity` that counts, in log order, then those of
    /// the shown pending sets: what no role in a Shared Area allows is
    /// left out, judged with those sets too.
    pub fn history(&self, entity: &str) -> Result<Vec<Entry>> {
        #[cfg(test)]
        self.history_reads
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        if self.ai_hides(entity)? {
            return Ok(Vec::new());
        }
        let shown = self.shown()?;
        if shown.is_empty() {
            return self.logged_history(entity);
        }
        let mut all = self.entries(
            &format!("SELECT * FROM entries WHERE {APPLIED} AND entity = ? ORDER BY seq"),
            params![entity],
        )?;
        all.extend(shown.iter().filter(|e| e.entity == entity).cloned());
        let judged = self.display_judged(&shown)?;
        if judged.ignored.is_empty() {
            return Ok(all);
        }
        Ok(all
            .into_iter()
            .filter(|e| !judged.ignored.contains(&e.id()))
            .collect())
    }

    /// Every entry of `entity` in the log that counts, in log order, no
    /// pending change among them: what decides where an entity is
    /// sealed, and what the user's own Devices wrote.
    pub fn logged_history(&self, entity: &str) -> Result<Vec<Entry>> {
        if self.ai_hides(entity)? {
            return Ok(Vec::new());
        }
        let all = self.entries(
            &format!("SELECT * FROM entries WHERE {APPLIED} AND entity = ? ORDER BY seq"),
            params![entity],
        )?;
        let judged = self.judged()?;
        if judged.ignored.is_empty() {
            return Ok(all);
        }
        Ok(all
            .into_iter()
            .filter(|e| !judged.ignored.contains(&e.id()))
            .collect())
    }

    /// The value of `field` on `entity` in the log alone, latest wins.
    pub fn logged(&self, entity: &str, field: &str) -> Result<Option<String>> {
        let mut held: Option<Entry> = None;
        for e in self.logged_history(entity)? {
            if e.field == field
                && held
                    .as_ref()
                    .is_none_or(|h| Self::latest_wins(&e, h) == Ordering::Greater)
            {
                held = Some(e);
            }
        }
        Ok(held.and_then(|e| e.value))
    }

    /// Every entry of `entities` as it is, none left out, by (device,
    /// dseq): what is relayed to another Device, which judges for itself.
    pub fn raw_history_of(
        &self,
        entities: &std::collections::HashSet<String>,
    ) -> Result<Vec<Entry>> {
        Ok(self
            .entries("SELECT * FROM entries ORDER BY device, dseq", params![])?
            .into_iter()
            .filter(|e| entities.contains(&e.entity))
            .collect())
    }

    /// Every entry, in log order, those of a later structure too: what a
    /// Backup carries.
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

    /// What [`Store::current_fields`] gives for each of `entities`, read
    /// in one query rather than one per entity (#204), as the Dart
    /// core's `currentFieldsOf`; those without an entry that counts are
    /// left out. It counts as one [`Store::history`] read.
    pub fn current_fields_of<'a>(
        &self,
        entities: impl IntoIterator<Item = &'a str>,
    ) -> Result<HashMap<String, Fields>> {
        let mut ids: Vec<&str> = Vec::new();
        let mut wanted = std::collections::HashSet::new();
        for id in entities {
            if !self.ai_hides(id)? && wanted.insert(id) {
                ids.push(id);
            }
        }
        if ids.is_empty() {
            return Ok(HashMap::new());
        }
        #[cfg(test)]
        self.history_reads
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let shown = self.shown()?;
        let mut all = Vec::new();
        for chunk in ids.chunks(IN_CHUNK) {
            let marks = vec!["?"; chunk.len()].join(", ");
            all.extend(self.entries(
                &format!(
                    "SELECT * FROM entries WHERE {APPLIED} AND entity IN ({marks}) ORDER BY seq"
                ),
                rusqlite::params_from_iter(chunk.iter()),
            )?);
        }
        all.extend(
            shown
                .iter()
                .filter(|e| wanted.contains(e.entity.as_str()))
                .cloned(),
        );
        let judged = if shown.is_empty() {
            self.judged()?
        } else {
            self.display_judged(&shown)?
        };
        let mut latest: HashMap<String, Vec<Entry>> = HashMap::new();
        for e in all {
            if judged.ignored.contains(&e.id()) {
                continue;
            }
            let fields = latest.entry(e.entity.clone()).or_default();
            match fields.iter_mut().find(|held| held.field == e.field) {
                Some(held) => {
                    if Self::latest_wins(&e, held) == Ordering::Greater {
                        *held = e;
                    }
                }
                None => fields.push(e),
            }
        }
        Ok(latest
            .into_iter()
            .map(|(id, fields)| {
                let fields = fields.into_iter().map(|e| (e.field, e.value)).collect();
                (id, Fields(fields))
            })
            .collect())
    }

    /// Entities whose current `$type` is `ty`.
    pub fn entities_of_type(&self, ty: &str) -> Result<Vec<String>> {
        let mut out = Vec::new();
        for id in self.ever_of_type(ty)? {
            if self.current(&id, "$type")?.as_deref() == Some(ty) {
                out.push(id);
            }
        }
        Ok(out)
    }

    /// What [`Store::current_fields`] gives for each entity whose current
    /// `$type` is `ty`, read in one [`Store::current_fields_of`] query
    /// rather than one per entity (#216), as the Dart core's
    /// `currentFieldsOfType`.
    pub fn current_fields_of_type(&self, ty: &str) -> Result<HashMap<String, Fields>> {
        let ids = self.ever_of_type(ty)?;
        let mut out = self.current_fields_of(ids.iter().map(String::as_str))?;
        out.retain(|_, fields| fields.get("$type") == Some(ty));
        Ok(out)
    }

    /// The connection, for the search index's own tables beside the log
    /// (ADR 0048), held while `body` runs; nothing else reads or writes
    /// through it.
    pub(crate) fn with_connection<T>(
        &self,
        body: impl FnOnce(&Connection) -> Result<T>,
    ) -> Result<T> {
        let db = self.db.lock();
        body(&db)
    }

    /// Entities whose `$type` was `ty` at some time, the shown pending
    /// sets included, in the order [`Store::entities_of_type`] keeps:
    /// also for a list of some of them in that order (ADR 0048).
    pub(crate) fn ever_of_type(&self, ty: &str) -> Result<Vec<String>> {
        let mut ids: Vec<String> = {
            let db = self.db.lock();
            let mut stmt = db.prepare(&format!(
                "SELECT DISTINCT entity FROM entries WHERE {APPLIED} \
                     AND field = '$type' AND value = ?"
            ))?;
            let rows = stmt.query_map([ty], |r| r.get(0))?;
            rows.collect::<rusqlite::Result<_>>()?
        };
        for e in self.shown()?.iter() {
            if e.field == "$type" && e.value.as_deref() == Some(ty) && !ids.contains(&e.entity) {
                ids.push(e.entity.clone());
            }
        }
        if self.reads_for_ai() {
            ids = self.left_for_ai(ids)?;
        }
        Ok(ids)
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
            out.push(entry_of(seq, wire)?);
        }
        Ok(out)
    }

    // ----------------------------------------------------------- pending

    /// Keeps `set` in its file and shows it where it is shown.
    pub(crate) fn keep_pending_set(&self, set: PendingSet) -> Result<()> {
        self.pending_files.write(&set.id, &set.encode()?)?;
        self.pending.lock().sets.insert(set.id.clone(), set);
        self.changed();
        Ok(())
    }

    /// Deletes the set `id` and its file, and the mark of its Accept.
    pub(crate) fn forget_pending_set(&self, id: &str) -> Result<()> {
        self.pending_files.remove(id)?;
        self.remove_local_setting(&format!("{ACCEPTED_PENDING_PREFIX}{id}"))?;
        let removed = self.pending.lock().sets.remove(id).is_some();
        if removed {
            self.changed();
        }
        Ok(())
    }

    /// The pending sets, the oldest first.
    pub(crate) fn sorted_pending_sets(&self) -> Vec<PendingSet> {
        let mut sets: Vec<PendingSet> = self.pending.lock().sets.values().cloned().collect();
        sets.sort_by(|a, b| a.made.cmp(&b.made).then_with(|| a.id.cmp(&b.id)));
        sets
    }

    /// The set `id` as its file holds it now, and so as this store shows
    /// it from now on, as the Dart core's `freshPendingSet`: another store
    /// open on the same file may have changed or ended it. `None` when it
    /// is gone, or accepted and its file not deleted yet.
    pub(crate) fn fresh_pending_set(&self, id: &str) -> Result<Option<PendingSet>> {
        let text = match self.local_setting(&format!("{ACCEPTED_PENDING_PREFIX}{id}"))? {
            Some(_) => None,
            None => self.pending_files.read(id)?,
        };
        let fresh = text
            .as_deref()
            .and_then(PendingSet::parse)
            .filter(|set| set.id == id);
        self.cache_pending(id, fresh.clone())?;
        Ok(fresh)
    }

    /// Every pending set as the files hold them now, the oldest first.
    pub(crate) fn fresh_pending_sets(&self) -> Result<Vec<PendingSet>> {
        let files = self.pending_files.read_all()?;
        let held: Vec<String> = self.pending.lock().sets.keys().cloned().collect();
        for id in held {
            if !files.contains_key(&id) {
                self.cache_pending(&id, None)?;
            }
        }
        for (id, text) in &files {
            let set = match self.local_setting(&format!("{ACCEPTED_PENDING_PREFIX}{id}"))? {
                Some(_) => None,
                None => PendingSet::parse(text).filter(|set| &set.id == id),
            };
            self.cache_pending(id, set)?;
        }
        Ok(self.sorted_pending_sets())
    }

    /// Shows `set` as the set `id` from now on, none for `None`.
    fn cache_pending(&self, id: &str, set: Option<PendingSet>) -> Result<()> {
        let changed = {
            let mut held = self.pending.lock();
            match set {
                None => held.sets.remove(id).is_some(),
                Some(set) => match held.sets.get(id) {
                    Some(old) if old.encode()? == set.encode()? => false,
                    _ => {
                        held.sets.insert(id.to_string(), set);
                        true
                    }
                },
            }
        };
        if changed {
            self.changed();
        }
        Ok(())
    }

    /// Takes in the sets a restart left. One accepted before its file
    /// was gone is gone now; a file that is no set this version reads
    /// stays on disk, unread.
    fn load_pending(&self) -> Result<()> {
        let files = self.pending_files.read_all()?;
        for (id, text) in &files {
            if self
                .local_setting(&format!("{ACCEPTED_PENDING_PREFIX}{id}"))?
                .is_some()
            {
                self.forget_pending_set(id)?;
                continue;
            }
            if let Some(set) = PendingSet::parse(text)
                && &set.id == id
            {
                self.pending.lock().sets.insert(id.clone(), set);
            }
        }
        for (key, _) in self.local_settings_with_prefix(ACCEPTED_PENDING_PREFIX)? {
            let id = &key[ACCEPTED_PENDING_PREFIX.len()..];
            if !files.contains_key(id) {
                self.remove_local_setting(&key)?;
            }
        }
        Ok(())
    }

    /// The changes of the shown pending sets as entries, as the Dart
    /// core's `_shown`: oldest set first, each in the order its changes
    /// came; a draft as this Device's, numbered below zero, so it is
    /// numbered nowhere; a received entry as it came, unless the log
    /// holds it by now. Only what this version applies, and nothing an
    /// edit of the user's own made since gives way to at Accept
    /// ([`Store::gives_way`]): the reads show what Accept will keep.
    /// None inside [`Store::log_only`].
    pub(crate) fn shown(&self) -> Result<Arc<Vec<Entry>>> {
        if self.reads_log_only() {
            return Ok(Arc::new(Vec::new()));
        }
        let generation = self.generation.load(std::sync::atomic::Ordering::SeqCst);
        {
            let held = self.pending.lock();
            if let Some(overlay) = &held.overlay {
                return Ok(overlay.clone());
            }
            if held.sets.is_empty() {
                return Ok(Arc::new(Vec::new()));
            }
        }
        let device = self.device_id()?;
        let author = self.author()?;
        let mut drafted = 0;
        let mut out = Vec::new();
        let mut by = GiveWay::default();
        for set in self.sorted_pending_sets() {
            if !set.shown {
                continue;
            }
            by.ids = set
                .received()
                .iter()
                .map(|r| format!("{}:{}", r.entry.device, r.entry.dseq))
                .collect();
            for change in set.changes {
                match change {
                    PendingChange::Draft(d) => {
                        if self.gives_way(&d.entity, &d.field, d.mark, &mut by)? {
                            continue;
                        }
                        drafted += 1;
                        out.push(d.as_entry(&device, &author, -drafted));
                    }
                    PendingChange::Entry(r) => {
                        if r.entry.applies()
                            && !self.holds(&r.entry.device, r.entry.dseq)?
                            && !self.gives_way(&r.entry.entity, &r.entry.field, r.mark, &mut by)?
                        {
                            out.push(entry_of(0, r.entry)?);
                        }
                    }
                }
            }
        }
        let overlay = Arc::new(out);
        let mut held = self.pending.lock();
        if self.generation.load(std::sync::atomic::Ordering::SeqCst) == generation {
            held.overlay = Some(overlay.clone());
        }
        Ok(overlay)
    }

    /// What the roles allow with the shown pending sets on top of the
    /// log: what the reads leave out, nothing else.
    fn display_judged(&self, shown: &[Entry]) -> Result<Arc<JudgedLog>> {
        if shown.is_empty() {
            return self.judged();
        }
        let generation = self.generation.load(std::sync::atomic::Ordering::SeqCst);
        if let Some(held) = self.pending.lock().judged.as_ref() {
            return Ok(held.clone());
        }
        let logged = self.judged()?;
        let roles = shown
            .iter()
            .any(|e| e.field.starts_with(crate::shared::ROLE_PREFIX));
        let judged = if !roles && logged.areas.is_empty() && logged.ignored.is_empty() {
            logged
        } else {
            let mut all = self.entries(
                &format!("SELECT * FROM entries WHERE {APPLIED} ORDER BY seq"),
                params![],
            )?;
            all.extend(shown.iter().cloned());
            Arc::new(judge_log(&all))
        };
        let mut held = self.pending.lock();
        if self.generation.load(std::sync::atomic::Ordering::SeqCst) == generation {
            held.judged = Some(judged.clone());
        }
        Ok(judged)
    }
}

/// The local setting that says a pending set was accepted, written with
/// what Accept wrote: a restart before its file is gone deletes it.
pub const ACCEPTED_PENDING_PREFIX: &str = "pending:";

/// `wire` as the store reads it, at the local handle `seq`.
pub(crate) fn entry_of(seq: i64, wire: WireEntry) -> Result<Entry> {
    Ok(Entry {
        seq,
        extra: wire.extra,
        raw: wire.raw,
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
        ctx: wire.ctx,
    })
}

fn raw_row(r: &Row) -> rusqlite::Result<(i64, WireEntry)> {
    let raw: Option<String> = r.get("raw")?;
    // What the entry carries beyond version 1 lies in what it came as.
    let extra = raw
        .as_deref()
        .and_then(|raw| serde_json::from_str::<WireEntry>(raw).ok())
        .map(|e| e.extra)
        .unwrap_or_default();
    Ok((
        r.get("seq")?,
        WireEntry {
            extra,
            raw,
            device: r.get("device")?,
            dseq: r.get("dseq")?,
            entity: r.get("entity")?,
            field: r.get("field")?,
            value: r.get("value")?,
            date: r.get("date")?,
            author: r.get("author")?,
            recorded: r.get("recorded")?,
            ctx: r.get("ctx")?,
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

/// Whether a local setting's `value` is JSON a later version of the app
/// wrote (ADR 0034), as the Dart core's `laterSetting`: an object whose
/// `v` is there and not the whole number 1, or a list holding one; this
/// version writes neither. Every other value, JSON or not, is this
/// version's.
pub fn later_setting(value: &str) -> bool {
    fn later(json: &serde_json::Value) -> bool {
        json.is_object() && json.get("v").is_some_and(|v| v.as_i64() != Some(1))
    }
    serde_json::from_str::<serde_json::Value>(value).is_ok_and(|json| {
        later(&json) || json.as_array().is_some_and(|list| list.iter().any(later))
    })
}

#[cfg(test)]
pub(crate) mod tests;
