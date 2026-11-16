//! Backups, as the Dart core's `backup.dart` writes and reads them
//! (ADR 0010, 0016): an encrypted file holding everything on this
//! Device but its settings, the playback speed aside, written by hand or
//! weekly into a folder the user picks, and restored on a fresh install
//! with the passphrase or the recovery key of the Recovery Sheet.
//!
//! Setting Backups up makes a random backup key, which the Recovery
//! Sheet shows, and seals it once with a key the passphrase derives
//! (Argon2id). The Device keeps both, never the passphrase. The key
//! belongs to the own Profile: the user's other Devices receive it with
//! the sealed copy and write their own Backups with it. Every file is
//! sealed in chunks with a content key of its own, sealed with the
//! backup key. The bytes are the Dart core's, byte for byte.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use chrono::{DateTime, Duration, Utc};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::entry::{WireEntry, dart_iso, parse_time};
use crate::keys::{AREA_KEY_BYTES, AreaKey, SEAL_NONCE_BYTES, SEAL_TAG_BYTES, seal, unseal};
use crate::packs::{OpenedPack, Pack, PackAdded, PackTicks};
use crate::places::Area;
use crate::profile::OwnProfile;
use crate::recovery;
use crate::schedule::WallClock;
use crate::settings::{DeviceSettings, offered_speed, speed_text};
use crate::shared::{ROLE_PREFIX, role_field};
use crate::signing::{
    ImportReport, KeyRecord, KeyTrust, PinnedKey, SigningKey, device_id_from_key, entry_bytes,
};
use crate::store::Store;
use crate::sync::{BLOB_KINDS, split_blob_ref};
use crate::{Error, Result};

/// The file extension of a Backup.
pub const EXTENSION: &str = "obfbackup";

/// What the header line says a Backup is.
pub const TYPE: &str = "oldbrainfix-backup";

/// The format a Backup says it is in, in its header and its first
/// record: `format`, 1 when missing, and 1 is never written (ADR 0019).
pub const FORMAT: i64 = 1;

/// How many payload bytes each sealed chunk holds.
pub const CHUNK_BYTES: usize = 65536;

/// How many Backups of this Device a folder keeps; older ones go.
pub const KEPT: usize = 4;

/// How often a scheduled Backup is written, in days.
pub const INTERVAL_DAYS: i64 = 7;

/// The shortest passphrase taken.
pub const MIN_PASSPHRASE: usize = 8;

pub const KEY_BYTES: usize = AREA_KEY_BYTES;
const SALT_BYTES: usize = 16;
const MAX_HEADER_BYTES: u64 = 4096;
const MAX_RECORD_BYTES: usize = 1 << 20;
const MAX_BLOB_BYTES: u64 = 256 << 20;

/// What a sealed part of a Backup is bound to, as the Dart core's
/// `SealedAs`.
pub mod sealed_as {
    /// The backup key, sealed with the key the passphrase derives.
    pub const PASS: &str = "oldbrainfix:backup-pass";
    /// A Backup's content key, sealed with the backup key.
    pub const KEY: &str = "oldbrainfix:backup-key";

    /// Chunk `index` of the Backup whose header line hashes to `header`.
    pub fn chunk(header: &str, index: u64, last: bool) -> String {
        if last {
            format!("oldbrainfix:backup:{header}:{index}:last")
        } else {
            format!("oldbrainfix:backup:{header}:{index}")
        }
    }
}

/// Fresh random bytes; tests and fixtures hand in counted ones.
pub type Random<'a> = &'a mut dyn FnMut(usize) -> Result<Vec<u8>>;

/// Random bytes from the system.
pub fn system_random(n: usize) -> Result<Vec<u8>> {
    let mut out = vec![0u8; n];
    getrandom::fill(&mut out).map_err(|e| Error::Invalid(format!("random: {e}")))?;
    Ok(out)
}

fn key_of(bytes: Vec<u8>) -> Result<[u8; KEY_BYTES]> {
    bytes
        .try_into()
        .map_err(|_| Error::Invalid("not a 32-byte key".into()))
}

fn nonce(random: Random) -> Result<[u8; SEAL_NONCE_BYTES]> {
    random(SEAL_NONCE_BYTES)?
        .try_into()
        .map_err(|_| Error::Invalid("not a nonce".into()))
}

/// Why a Backup could not be restored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Problem {
    /// The file is no Backup.
    NotABackup,
    /// A Backup of a later format (ADR 0034): this app needs an update to
    /// restore it.
    NeedsUpdate,
    /// Neither the passphrase nor the recovery key opens it.
    WrongSecret,
    /// It opened, but was changed or cut off.
    Damaged,
    /// This Device holds a log already.
    NotEmpty,
}

fn problem(p: Problem) -> Error {
    Error::Backup(p)
}

/// The Argon2id cost of the passphrase: memory in KiB, passes, lanes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Kdf {
    pub memory: u32,
    pub iterations: u32,
    pub parallelism: u32,
}

impl Kdf {
    /// 64 MiB, three passes, one lane.
    pub const STANDARD: Kdf = Kdf {
        memory: 65536,
        iterations: 3,
        parallelism: 1,
    };

    /// Whether a file's cost is one to work through.
    pub fn sane(&self) -> bool {
        (1..=4).contains(&self.parallelism)
            && self.memory >= 8 * self.parallelism
            && self.memory <= 262_144
            && (1..=16).contains(&self.iterations)
    }
}

/// The key `passphrase` derives with `kdf` and `salt`: Argon2id, 32
/// bytes, of the passphrase without the blanks around it.
pub fn pass_key(passphrase: &str, kdf: Kdf, salt: &[u8]) -> Result<[u8; KEY_BYTES]> {
    let params = argon2::Params::new(kdf.memory, kdf.iterations, kdf.parallelism, Some(KEY_BYTES))
        .map_err(|e| Error::Invalid(format!("argon2: {e}")))?;
    let argon = argon2::Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
    let mut out = [0u8; KEY_BYTES];
    argon
        .hash_password_into(passphrase.trim().as_bytes(), salt, &mut out)
        .map_err(|e| Error::Invalid(format!("argon2: {e}")))?;
    Ok(out)
}

/// The backup key sealed with the passphrase's key, with the cost and
/// the salt that derive it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pass {
    pub kdf: Kdf,
    pub salt: Vec<u8>,
    pub sealed: Vec<u8>,
}

impl Pass {
    /// The `kdf` of a header: keys sorted.
    pub fn kdf_json(&self) -> Value {
        json!({
            "m": self.kdf.memory,
            "p": self.kdf.parallelism,
            "salt": BASE64.encode(&self.salt),
            "t": self.kdf.iterations,
        })
    }

    /// As the Device keeps it: keys sorted.
    pub fn to_json(&self) -> Value {
        json!({
            "m": self.kdf.memory,
            "p": self.kdf.parallelism,
            "salt": BASE64.encode(&self.salt),
            "sealed": BASE64.encode(&self.sealed),
            "t": self.kdf.iterations,
        })
    }

    /// The stanza in `kdf` with `sealed`, or none when it is none or asks
    /// for a cost that is not [`Kdf::sane`].
    pub fn read(kdf: &Value, sealed: &Value) -> Option<Pass> {
        let number = |key: &str| kdf.get(key)?.as_u64().and_then(|n| u32::try_from(n).ok());
        let cost = Kdf {
            memory: number("m")?,
            iterations: number("t")?,
            parallelism: number("p")?,
        };
        if !cost.sane() {
            return None;
        }
        Some(Pass {
            kdf: cost,
            salt: BASE64.decode(kdf.get("salt")?.as_str()?).ok()?,
            sealed: BASE64.decode(sealed.as_str()?).ok()?,
        })
    }

    pub fn from_json(json: &Value) -> Option<Pass> {
        Pass::read(json, json.get("sealed")?)
    }
}

/// The plain first line of a Backup.
#[derive(Debug, Clone)]
pub struct Header {
    pub created: DateTime<Utc>,
    pub pass: Pass,
    pub sealed_key: Vec<u8>,
    pub chunk: usize,
    /// The line's bytes, without its line break.
    pub line: Vec<u8>,
}

impl Header {
    /// The SHA-256 of the line, hex.
    pub fn hash(&self) -> String {
        hex::encode(Sha256::digest(&self.line))
    }

    /// The header line of a Backup made at `created`, keys sorted.
    pub fn encode(created: DateTime<Utc>, pass: &Pass, sealed_key: &[u8], chunk: usize) -> Vec<u8> {
        json!({
            "chunk": chunk,
            "created": dart_iso(created),
            "kdf": pass.kdf_json(),
            "key": BASE64.encode(sealed_key),
            "pass": BASE64.encode(&pass.sealed),
            "type": TYPE,
        })
        .to_string()
        .into_bytes()
    }

    /// The header in `line`, or none when it is none this format reads.
    pub fn parse(line: &[u8]) -> Option<Header> {
        let json: Value = serde_json::from_slice(line).ok()?;
        if json.get("type")?.as_str()? != TYPE || !in_format(&json) {
            return None;
        }
        let chunk = usize::try_from(json.get("chunk")?.as_u64()?).ok()?;
        if !(1024..=(1 << 20)).contains(&chunk) {
            return None;
        }
        Some(Header {
            created: parse_time(json.get("created")?.as_str()?).ok()?,
            pass: Pass::read(json.get("kdf")?, json.get("pass")?)?,
            sealed_key: BASE64.decode(json.get("key")?.as_str()?).ok()?,
            chunk,
            line: line.to_vec(),
        })
    }
}

/// The header of the Backup at `path`, or none when it is none;
/// [`Problem::NeedsUpdate`] for a Backup of a later format, as the Dart
/// core's `readBackupHeader`.
pub fn read_header(path: &Path) -> Result<Option<Header>> {
    let mut head = Vec::new();
    File::open(path)?
        .take(MAX_HEADER_BYTES)
        .read_to_end(&mut head)?;
    let Some(line) = head
        .iter()
        .position(|b| *b == b'\n')
        .and_then(|end| head.get(..end))
    else {
        return Ok(None);
    };
    let header = Header::parse(line);
    if header.is_none() && later_header(line) {
        return Err(problem(Problem::NeedsUpdate));
    }
    Ok(header)
}

/// Whether `line` is the header of a Backup of a later format: it says
/// it is a Backup, and a format this version does not read.
fn later_header(line: &[u8]) -> bool {
    serde_json::from_slice::<Value>(line).is_ok_and(|json| {
        json.get("type").and_then(Value::as_str) == Some(TYPE) && later_format(&json)
    })
}

/// The backup key that opens `header`: through `passphrase`, or the
/// `recovery_code` as typed or scanned; none when neither does.
pub fn unlock(
    header: &Header,
    passphrase: Option<&str>,
    recovery_code: Option<&str>,
) -> Result<Option<[u8; KEY_BYTES]>> {
    let key = if let Some(code) = recovery_code {
        recovery::decode_code(code)
    } else if let Some(passphrase) = passphrase {
        let derived = pass_key(passphrase, header.pass.kdf, &header.pass.salt)?;
        unseal(&derived, &header.pass.sealed, sealed_as::PASS).and_then(|k| k.try_into().ok())
    } else {
        None
    };
    Ok(key.filter(|k| unseal(k, &header.sealed_key, sealed_as::KEY).is_some()))
}

// ---------------------------------------------------------------- settings

const KEY: &str = "backup:key";
const PASS: &str = "backup:pass";
const FOLDER: &str = "backup:folder";
const WEEKLY: &str = "backup:weekly";
const LAST: &str = "backup:last";
const FAILED: &str = "backup:failed";
const VERSION: &str = "backup:version";

impl Store {
    /// The backup key, once Backups are set up.
    pub fn backup_key(&self) -> Result<Option<[u8; KEY_BYTES]>> {
        Ok(self
            .local_setting(KEY)?
            .and_then(|raw| BASE64.decode(raw).ok())
            .and_then(|key| key.try_into().ok()))
    }

    /// The backup key sealed with the passphrase's key.
    pub fn backup_pass(&self) -> Result<Option<Pass>> {
        Ok(self
            .local_setting(PASS)?
            .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
            .and_then(|json| Pass::from_json(&json)))
    }

    pub fn backups_set_up(&self) -> Result<bool> {
        Ok(self.backup_key()?.is_some() && self.backup_pass()?.is_some())
    }

    /// The recovery key the Recovery Sheet shows, once set up.
    pub fn recovery_code(&self) -> Result<Option<String>> {
        Ok(self.backup_key()?.map(|k| recovery::encode_code(&k)))
    }

    /// Keeps the backup `key` and `pass`, what a scheduled Backup needs,
    /// with the key's `version`.
    pub fn keep_backup_keys(&self, key: &[u8; KEY_BYTES], pass: &Pass, version: i64) -> Result<()> {
        self.set_local_setting(KEY, &BASE64.encode(key))?;
        self.set_local_setting(PASS, &pass.to_json().to_string())?;
        self.set_local_setting(VERSION, &version.to_string())
    }

    /// The folder Backups go into.
    pub fn backup_folder(&self) -> Result<Option<String>> {
        self.local_setting(FOLDER)
    }

    pub fn set_backup_folder(&self, folder: Option<&str>) -> Result<()> {
        match folder {
            Some(folder) => self.set_local_setting(FOLDER, folder),
            None => self.remove_local_setting(FOLDER),
        }
    }

    /// Whether a Backup is written every week; on unless switched off.
    pub fn backups_weekly(&self) -> Result<bool> {
        Ok(self.local_setting(WEEKLY)?.as_deref() != Some("off"))
    }

    pub fn set_backups_weekly(&self, on: bool) -> Result<()> {
        if on {
            self.remove_local_setting(WEEKLY)
        } else {
            self.set_local_setting(WEEKLY, "off")
        }
    }

    /// Whether Backups are written on their own.
    pub fn backups_scheduled(&self) -> Result<bool> {
        Ok(self.backups_set_up()? && self.backup_folder()?.is_some() && self.backups_weekly()?)
    }

    /// When the last Backup was written here.
    pub fn last_backup(&self) -> Result<Option<DateTime<Utc>>> {
        Ok(self
            .local_setting(LAST)?
            .and_then(|raw| parse_time(&raw).ok()))
    }

    /// When writing a Backup last failed, unless one was written since.
    pub fn backup_failed(&self) -> Result<Option<DateTime<Utc>>> {
        Ok(self
            .local_setting(FAILED)?
            .and_then(|raw| parse_time(&raw).ok()))
    }

    /// The backup key's version, as the Dart core's `backupKeyVersion`:
    /// 1 when Backups were first set up, one more each time they are set
    /// up again, 0 once the Profile it was made for was given up.
    pub fn backup_key_version(&self) -> Result<i64> {
        Ok(self
            .local_setting(VERSION)?
            .and_then(|v| v.parse().ok())
            .unwrap_or(1))
    }

    /// The backup key as it travels to the user's other Devices (ADR
    /// 0016): the key, its sealed copy, the own Profile it belongs to and
    /// its version; none before Backups are set up or before the own
    /// Profile exists.
    pub fn backup_key_to_send(&self) -> Result<Option<Value>> {
        let (Some(key), Some(pass), Some(profile)) = (
            self.backup_key()?,
            self.backup_pass()?,
            OwnProfile::new(self).get()?,
        ) else {
            return Ok(None);
        };
        Ok(Some(json!({
            "key": BASE64.encode(key),
            "pass": pass.to_json(),
            "profile": profile.id,
            "version": self.backup_key_version()?,
        })))
    }

    /// Takes the backup key `sent` by another of the user's own Devices,
    /// after its Profile was taken in, as the Dart core's
    /// `takeBackupKey`: only one made for the own Profile, and only when
    /// it is newer than the one held (a higher version, or at the same
    /// version the larger key). Whether it was kept.
    pub fn take_backup_key(&self, sent: Option<&Value>) -> Result<bool> {
        let Some(sent) = sent else {
            return Ok(false);
        };
        let (Some(key), Some(pass), Some(profile), Some(version)) = (
            sent.get("key").and_then(Value::as_str),
            sent.get("pass").and_then(Pass::from_json),
            sent.get("profile").and_then(Value::as_str),
            sent.get("version").and_then(Value::as_i64),
        ) else {
            return Ok(false);
        };
        if version < 0
            || OwnProfile::new(self)
                .get()?
                .is_none_or(|own| own.id != profile)
        {
            return Ok(false);
        }
        let Some(bytes) = BASE64
            .decode(key)
            .ok()
            .and_then(|k| <[u8; KEY_BYTES]>::try_from(k).ok())
        else {
            return Ok(false);
        };
        if let (Some(held), Some(_)) = (self.backup_key()?, self.backup_pass()?) {
            let mine = self.backup_key_version()?;
            let ours = BASE64.encode(held);
            if version < mine || (version == mine && key <= ours.as_str()) {
                return Ok(false);
            }
        }
        self.keep_backup_keys(&bytes, &pass, version)?;
        Ok(true)
    }

    /// Gives the backup key held up to whatever key the own Profile
    /// brings: the Profile it was made for was given up for another.
    pub fn yield_backup_key(&self) -> Result<()> {
        if self.backup_key()?.is_some() {
            self.set_local_setting(VERSION, "0")?;
        }
        Ok(())
    }

    /// Whether a scheduled Backup is due at `now`.
    pub fn backup_due(&self, now: DateTime<Utc>) -> Result<bool> {
        if !self.backups_scheduled()? {
            return Ok(false);
        }
        Ok(self
            .last_backup()?
            .is_none_or(|last| now - last >= Duration::days(INTERVAL_DAYS)))
    }
}

/// Sets Backups up with `passphrase`: a new backup key for the own
/// Profile, sealed with the key it derives, one version above the one
/// held. Returns the recovery key for the Recovery Sheet.
pub fn set_up(store: &Store, passphrase: &str, kdf: Kdf, random: Random) -> Result<String> {
    if passphrase.trim().chars().count() < MIN_PASSPHRASE {
        return Err(Error::Invalid("the passphrase is too short".into()));
    }
    let version = if store.backups_set_up()? {
        store.backup_key_version()? + 1
    } else {
        1
    };
    let key = key_of(random(KEY_BYTES)?)?;
    let salt = random(SALT_BYTES)?;
    let derived = pass_key(passphrase, kdf, &salt)?;
    let sealed = seal(&derived, &key, sealed_as::PASS, &nonce(random)?)?;
    store.keep_backup_keys(&key, &Pass { kdf, salt, sealed }, version)?;
    Ok(recovery::encode_code(&key))
}

// ----------------------------------------------------------------- writing

/// What a Backup holds, by count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    pub entries: usize,
    pub moments: usize,
    pub keys: usize,
    pub pinned: usize,
    pub blobs: usize,
}

/// Seals the payload in chunks into a file as it comes.
struct ChunkWriter<'a, W: Write> {
    out: W,
    key: [u8; KEY_BYTES],
    header: String,
    buffer: Vec<u8>,
    chunk: usize,
    index: u64,
    random: Random<'a>,
}

impl<W: Write> ChunkWriter<'_, W> {
    fn add(&mut self, mut bytes: &[u8]) -> Result<()> {
        while !bytes.is_empty() {
            let take = bytes.len().min(self.chunk - self.buffer.len());
            let (now, later) = bytes
                .split_at_checked(take)
                .ok_or_else(|| Error::Invalid("a Backup chunk past its bytes".into()))?;
            self.buffer.extend_from_slice(now);
            bytes = later;
            if self.buffer.len() == self.chunk {
                self.seal(false)?;
            }
        }
        Ok(())
    }

    fn line(&mut self, json: &str) -> Result<()> {
        self.add(json.as_bytes())?;
        self.add(b"\n")
    }

    fn seal(&mut self, last: bool) -> Result<()> {
        let aad = sealed_as::chunk(&self.header, self.index, last);
        let sealed = seal(&self.key, &self.buffer, &aad, &nonce(self.random)?)?;
        self.out.write_all(&sealed)?;
        self.buffer.clear();
        self.index += 1;
        Ok(())
    }
}

/// Every blob the log ever named whose bytes are here, sorted.
pub fn held_blobs(store: &Store) -> Result<Vec<String>> {
    let mut refs = BTreeSet::new();
    for (kind, prefix) in BLOB_KINDS {
        for (_, field) in store.fields_with_prefix(prefix)? {
            let hash = &field[prefix.len()..];
            if store.blob(kind, hash).is_some() {
                refs.insert(format!("{kind}/{hash}"));
            }
        }
    }
    Ok(refs.into_iter().collect())
}

/// Writes a Backup of `store` to `path`: this Device's identity, the own
/// Profile with its binding and the backup key's version, the playback
/// speed chosen here, every entry, Moment, Area key, pinned key and blob;
/// none of its other settings. Backups must be set up.
pub fn write_backup(store: &Store, path: &Path, random: Random) -> Result<Counts> {
    let (Some(backup_key), Some(pass)) = (store.backup_key()?, store.backup_pass()?) else {
        return Err(Error::Invalid("Backups are not set up".into()));
    };
    let content_key = key_of(random(KEY_BYTES)?)?;
    let sealed_key = seal(&backup_key, &content_key, sealed_as::KEY, &nonce(random)?)?;
    let line = Header::encode(store.now(), &pass, &sealed_key, CHUNK_BYTES);
    let mut file = std::io::BufWriter::new(File::create(path)?);
    file.write_all(&line)?;
    file.write_all(b"\n")?;
    let mut out = ChunkWriter {
        out: file,
        key: content_key,
        header: hex::encode(Sha256::digest(&line)),
        buffer: Vec::with_capacity(CHUNK_BYTES),
        chunk: CHUNK_BYTES,
        index: 0,
        random,
    };
    out.line(
        &json!({"about": {
            "device": store.device_id()?,
            "seed": store.signing_key()?.seed_hex(),
        }})
        .to_string(),
    )?;
    if let Some(profile) = OwnProfile::new(store).to_send()? {
        out.line(
            &json!({
                "backupVersion": store.backup_key_version()?,
                "profile": profile,
            })
            .to_string(),
        )?;
    }
    // As the setting is written, never as a number: the cores need not
    // write fractions alike.
    if let Some(speed) = DeviceSettings::new(store).chosen_playback_speed()? {
        out.line(&json!({"playbackSpeed": speed_text(speed)}).to_string())?;
    }
    let entries = store.all_entries()?;
    for e in &entries {
        out.line(&format!("{{\"entry\":{}}}", e.wire().line()?))?;
    }
    let seqs: Vec<i64> = entries.iter().map(|e| e.seq).collect();
    let mut moments = store.moment_rows()?;
    moments.reverse();
    for m in &moments {
        let at = dart_iso(parse_time(&m.at)?);
        out.line(
            &json!({"moment": {
                "at": at,
                "cause": m.cause,
                "label": m.label,
                "seq": seqs.partition_point(|s| *s <= m.seq),
            }})
            .to_string(),
        )?;
    }
    let keys = store.area_keys()?;
    for k in &keys {
        out.line(&json!({"key": k.to_json()}).to_string())?;
    }
    let pinned = store.pinned_keys()?;
    for p in &pinned {
        out.line(&json!({"pinned": p.to_json()}).to_string())?;
    }
    let blobs = held_blobs(store)?;
    for reference in &blobs {
        let (kind, hash) = split_blob_ref(reference);
        let Some(bytes) = store.blob(kind, hash) else {
            continue;
        };
        out.line(&json!({"blob": reference, "size": bytes.len()}).to_string())?;
        out.add(&bytes)?;
    }
    out.seal(true)?;
    out.out.flush()?;
    Ok(Counts {
        entries: entries.len(),
        moments: moments.len(),
        keys: keys.len(),
        pinned: pinned.len(),
        blobs: blobs.len(),
    })
}

// --------------------------------------------------------------- restoring

/// What a restore took in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Restored {
    /// The id of the Device the Backup was made on.
    pub from: String,
    pub entries: usize,
    pub moments: usize,
    pub keys: usize,
    pub pinned: usize,
    pub blobs: usize,
    /// The Shared Areas whose role the old Device handed to this one.
    pub roles: usize,
}

/// Opens the chunks one after the other and hands out the payload.
struct ChunkReader<R: Read> {
    input: R,
    key: [u8; KEY_BYTES],
    header: String,
    chunk: usize,
    index: u64,
    ended: bool,
    plain: Vec<u8>,
    at: usize,
}

impl<R: Read> ChunkReader<R> {
    /// Whether the next chunk is opened; false after the last one.
    fn next(&mut self) -> Result<bool> {
        if self.ended {
            return Ok(false);
        }
        let whole = self.chunk + SEAL_NONCE_BYTES + SEAL_TAG_BYTES;
        let mut sealed = Vec::with_capacity(whole);
        (&mut self.input)
            .take(whole as u64)
            .read_to_end(&mut sealed)?;
        if sealed.len() < SEAL_NONCE_BYTES + SEAL_TAG_BYTES {
            return Err(problem(Problem::Damaged));
        }
        let last = sealed.len() < whole;
        let aad = sealed_as::chunk(&self.header, self.index, last);
        self.plain = unseal(&self.key, &sealed, &aad).ok_or(problem(Problem::Damaged))?;
        self.index += 1;
        self.ended = last;
        self.at = 0;
        Ok(true)
    }

    /// The next line, without its break; none at the end.
    fn line(&mut self) -> Result<Option<String>> {
        let mut bytes = Vec::new();
        loop {
            if self.at == self.plain.len() {
                if !self.next()? {
                    if bytes.is_empty() {
                        return Ok(None);
                    }
                    return Err(problem(Problem::Damaged));
                }
                continue;
            }
            let rest = self
                .plain
                .get(self.at..)
                .ok_or_else(|| problem(Problem::Damaged))?;
            match rest.iter().position(|b| *b == b'\n') {
                Some(end) => {
                    bytes.extend_from_slice(
                        rest.get(..end).ok_or_else(|| problem(Problem::Damaged))?,
                    );
                    self.at += end + 1;
                    return String::from_utf8(bytes)
                        .map(Some)
                        .map_err(|_| problem(Problem::Damaged));
                }
                None => {
                    bytes.extend_from_slice(rest);
                    self.at = self.plain.len();
                }
            }
            if bytes.len() > MAX_RECORD_BYTES {
                return Err(problem(Problem::Damaged));
            }
        }
    }

    /// The next `length` bytes.
    fn bytes(&mut self, length: usize) -> Result<Vec<u8>> {
        let mut out = Vec::with_capacity(length);
        while out.len() < length {
            if self.at == self.plain.len() && !self.next()? {
                return Err(problem(Problem::Damaged));
            }
            let take = (length - out.len()).min(self.plain.len() - self.at);
            let taken = self
                .plain
                .get(self.at..self.at + take)
                .ok_or_else(|| problem(Problem::Damaged))?;
            out.extend_from_slice(taken);
            self.at += take;
        }
        Ok(out)
    }

    /// Whether the whole payload was read, up to its last chunk.
    fn at_end(&mut self) -> Result<bool> {
        Ok(self.at == self.plain.len() && !self.next()? && self.ended)
    }
}

/// Whether `json`, the header or the `about` record, is in the format
/// this version reads: `format`, 1 when missing (ADR 0019).
fn in_format(json: &Value) -> bool {
    json.is_object() && !later_format(json)
}

/// Whether `json` says a format this version does not read: a Backup of
/// a later version (ADR 0034), as the Dart core's `_laterFormat`. Only an
/// absent `format` and the whole number 1 are known; `"1"`, `1.0` or
/// `null` are later too.
fn later_format(json: &Value) -> bool {
    json.get("format")
        .is_some_and(|format| format.as_i64() != Some(FORMAT))
}

/// The old Device's signing key in the `about` record, or none.
fn about(json: &Value) -> Option<SigningKey> {
    let about = json.get("about")?;
    if !in_format(about) {
        return None;
    }
    let seed = about.get("seed")?.as_str()?;
    if seed.len() != 64 {
        return None;
    }
    let key = SigningKey::from_seed_hex(seed).ok()?;
    (device_id_from_key(&key.public_key()) == about.get("device")?.as_str()?).then_some(key)
}

/// A size field of a record, within what is taken.
fn size_of(json: &Value) -> Result<usize> {
    json.get("size")
        .and_then(Value::as_u64)
        .filter(|n| *n <= MAX_BLOB_BYTES)
        .and_then(|n| usize::try_from(n).ok())
        .ok_or(problem(Problem::Damaged))
}

struct MomentIn {
    seq: usize,
    cause: String,
    label: Option<String>,
    at: String,
}

/// What the payload of a Backup holds but its blobs, read whole and
/// checked.
struct Payload {
    /// The signing key of the Device the Backup was made on.
    old: SigningKey,
    entries: Vec<WireEntry>,
    moments: Vec<MomentIn>,
    keys: Vec<AreaKey>,
    pinned: Vec<PinnedKey>,
    profile: Option<(Value, i64)>,
    speed: Option<f64>,
    blobs: usize,
}

/// The header of the Backup at `path` and its content key, opened with
/// `backup_key`: [`Problem::NotABackup`] for a file that is none,
/// [`Problem::NeedsUpdate`] for one of a later format, and
/// [`Problem::WrongSecret`] when `backup_key` does not open it.
fn opened(path: &Path, backup_key: &[u8; KEY_BYTES]) -> Result<(Header, [u8; KEY_BYTES])> {
    let header = read_header(path)?.ok_or(problem(Problem::NotABackup))?;
    let content_key = unseal(backup_key, &header.sealed_key, sealed_as::KEY)
        .and_then(|k| k.try_into().ok())
        .ok_or(problem(Problem::WrongSecret))?;
    Ok((header, content_key))
}

/// Whether `bytes` are the `kind` blob `hash`: a photo, a recording or
/// an attached file whose SHA-256 is `hash`.
fn is_blob(kind: &str, hash: &str, bytes: &[u8]) -> bool {
    matches!(kind, "photo" | "audio" | "file") && hex::encode(Sha256::digest(bytes)) == hash
}

/// What a blob read from a Backup is handed to: its kind, its hash and
/// its bytes, the only blob whole in memory then. An error refuses it.
type OnBlob<'a> = dyn FnMut(&str, &str, Vec<u8>) -> Result<()> + 'a;

/// Reads the payload of the Backup at `path`, its `header` opened with
/// `content_key`, every record checked to its last chunk, as the Dart
/// core's `_readPayload`, and hands each blob to `blob` as it comes.
fn read_payload(
    path: &Path,
    header: &Header,
    content_key: &[u8; KEY_BYTES],
    blob: &mut OnBlob,
) -> Result<Payload> {
    let damaged = || problem(Problem::Damaged);
    let mut file = BufReader::new(File::open(path)?);
    let mut skipped = Vec::new();
    file.read_until(b'\n', &mut skipped)?;
    let mut payload = ChunkReader {
        input: file,
        key: *content_key,
        header: header.hash(),
        chunk: header.chunk,
        index: 0,
        ended: false,
        plain: Vec::new(),
        at: 0,
    };
    let mut old: Option<SigningKey> = None;
    let mut entries: Vec<WireEntry> = Vec::new();
    let mut moments: Vec<MomentIn> = Vec::new();
    let mut keys: Vec<AreaKey> = Vec::new();
    let mut pinned: Vec<PinnedKey> = Vec::new();
    let mut profile: Option<(Value, i64)> = None;
    let mut speed: Option<f64> = None;
    let mut blobs = 0;
    while let Some(text) = payload.line()? {
        let json: Value = serde_json::from_str(&text).map_err(|_| damaged())?;
        if !json.is_object() {
            return Err(damaged());
        }
        if old.is_none() {
            if json.get("about").is_some_and(later_format) {
                return Err(problem(Problem::NeedsUpdate));
            }
            old = Some(about(&json).ok_or_else(damaged)?);
            continue;
        }
        if let Some(e) = json.get("entry") {
            // Kept byte for byte as it was written (ADR 0019).
            let entry = match text
                .strip_prefix("{\"entry\":")
                .and_then(|t| t.strip_suffix('}'))
            {
                Some(written) => WireEntry::parse(written),
                None => WireEntry::from_value(e),
            };
            entries.push(entry.ok_or_else(damaged)?);
        } else if let Some(m) = json.get("moment") {
            let at = m.get("at").and_then(Value::as_str).ok_or_else(damaged)?;
            parse_time(at).map_err(|_| damaged())?;
            let label = match m.get("label") {
                Some(Value::Null) => None,
                Some(Value::String(s)) => Some(s.clone()),
                _ => return Err(damaged()),
            };
            moments.push(MomentIn {
                seq: m
                    .get("seq")
                    .and_then(Value::as_u64)
                    .and_then(|n| usize::try_from(n).ok())
                    .ok_or_else(damaged)?,
                cause: m
                    .get("cause")
                    .and_then(Value::as_str)
                    .ok_or_else(damaged)?
                    .to_string(),
                label,
                at: at.to_string(),
            });
        } else if let Some(k) = json.get("key") {
            keys.push(AreaKey::from_json(k).ok_or_else(damaged)?);
        } else if let Some(p) = json.get("pinned") {
            pinned.push(PinnedKey::from_json(p).ok_or_else(damaged)?);
        } else if let (Some(v), Some(p)) = (json.get("backupVersion"), json.get("profile")) {
            let v = v.as_i64().filter(|v| *v >= 0).ok_or_else(damaged)?;
            profile = Some((p.clone(), v));
        } else if let Some(text) = json.get("playbackSpeed").and_then(Value::as_str) {
            // One not offered here is passed over, as normal speed.
            if let Some(kept) = offered_speed(text) {
                speed = Some(kept);
            }
        } else if let Some(reference) = json.get("blob").and_then(Value::as_str) {
            let bytes = payload.bytes(size_of(&json)?)?;
            let (kind, hash) = split_blob_ref(reference);
            blob(kind, hash, bytes)?;
            blobs += 1;
        } else if json.get("size").is_some() {
            // A record of a later kind: its bytes are passed over.
            payload.bytes(size_of(&json)?)?;
        }
    }
    let old = old.ok_or_else(damaged)?;
    if !payload.at_end()? {
        return Err(damaged());
    }
    Ok(Payload {
        old,
        entries,
        moments,
        keys,
        pinned,
        profile,
        speed,
        blobs,
    })
}

/// Restores the Backup at `path`, opened with `backup_key`, into the
/// empty log of `store`, as the Dart core's `restoreBackup`: this Device
/// keeps its identity, takes in the entries, Moments, blobs, Area keys
/// and pinned keys, pins the old Device's key as verified, takes the own
/// Profile back with its own key bound to it by the old Device's, keeps
/// the backup key, and signs as the old Device its roles in the Shared
/// Areas for itself. Of the old Device's settings only the playback
/// speed comes along, if it is one offered. The log is written in one
/// transaction, once the whole file proved sound.
pub fn restore(store: &Store, path: &Path, backup_key: &[u8; KEY_BYTES]) -> Result<Restored> {
    if store.current_seq()? != 0 {
        return Err(problem(Problem::NotEmpty));
    }
    let (header, content_key) = opened(path, backup_key)?;
    let payload = read_payload(path, &header, &content_key, &mut |kind, hash, bytes| {
        if store.put_blob(kind, hash, &bytes)? {
            Ok(())
        } else {
            Err(problem(Problem::Damaged))
        }
    })?;
    let Payload {
        old,
        entries,
        moments,
        keys,
        pinned,
        profile,
        speed,
        blobs,
    } = payload;
    let roles = store.batch(|| {
        if store.current_seq()? != 0 {
            return Err(problem(Problem::NotEmpty));
        }
        // Each entry's local handle as it lands.
        let mut marks = vec![store.current_seq()?];
        for e in &entries {
            if store.insert(e)? {
                marks.push(store.current_seq()?);
            }
        }
        for m in &moments {
            // A moment past the last entry that landed goes to that one.
            let seq = *marks
                .get(m.seq)
                .or(marks.last())
                .ok_or_else(|| problem(Problem::Damaged))?;
            let at = dart_iso(parse_time(&m.at)?);
            store.add_moment_at(seq, &m.cause, m.label.as_deref(), &at)?;
        }
        store.learn_area_keys(&keys)?;
        let mut report = ImportReport::default();
        let record = KeyRecord::make(&old);
        store.learn_keys(
            std::slice::from_ref(&record),
            Some(&record.device),
            &mut report,
        )?;
        for p in &pinned {
            let verified = (p.trust == KeyTrust::Verified).then_some(p.record.device.as_str());
            store.learn_keys(std::slice::from_ref(&p.record), verified, &mut report)?;
        }
        OwnProfile::new(store).restore(profile.as_ref().map(|(p, _)| p), &old)?;
        store.keep_backup_keys(
            backup_key,
            &header.pass,
            profile.as_ref().map_or(1, |(_, v)| *v),
        )?;
        if let Some(speed) = speed {
            DeviceSettings::new(store).set_playback_speed(speed)?;
        }
        hand_roles(store, &old)
    })?;
    Ok(Restored {
        from: device_id_from_key(&old.public_key()),
        entries: entries.len(),
        moments: moments.len(),
        keys: keys.len(),
        pinned: pinned.len(),
        blobs,
        roles,
    })
}

/// Signs, as the old Device `old`, its role in every Shared Area for
/// this Device; returns how many.
fn hand_roles(store: &Store, old: &SigningKey) -> Result<usize> {
    let own = store.device_id()?;
    let device = device_id_from_key(&old.public_key());
    let judged = store.judged()?;
    let mut dseq = store
        .version_vector()?
        .get(&device)
        .copied()
        .unwrap_or_default();
    let author = store.author()?;
    let mut handed = 0;
    for (area, grants) in &judged.areas {
        // A grant not known here is not this version's to hand on.
        let Some(value) = grants.grant_of(&device).and_then(|g| g.encode()) else {
            continue;
        };
        if grants.ever_granted(&own) {
            continue;
        }
        dseq += 1;
        // After every grant there, whatever this Device's clock says.
        let mut now = store.now();
        for e in store.history(area)? {
            if e.field.starts_with(ROLE_PREFIX) && e.date_at >= now {
                now = e.date_at + Duration::milliseconds(1);
            }
        }
        let iso = dart_iso(now);
        let field = role_field(&own);
        let sig = old.sign(&entry_bytes(
            &device,
            dseq,
            area,
            &field,
            Some(&value),
            &iso,
            &author,
            &iso,
            None,
        ));
        store.insert(&WireEntry {
            device: device.clone(),
            dseq,
            entity: area.clone(),
            field,
            value: Some(value),
            date: iso.clone(),
            author: author.clone(),
            recorded: iso,
            ctx: None,
            sig: Some(sig),
            extra: Default::default(),
            raw: None,
        })?;
        handed += 1;
    }
    Ok(handed)
}

// -------------------------------------------------------- taking Notes back

/// A Backup opened to take Notes back from it (#285), as the Dart core's
/// `OpenedBackup`, without restoring it: its Notes as a Pack's tree
/// shows them (ADR 0043), read from a store of its own that held the
/// Backup's log as the Device that made it held it, gone once the Pack
/// was made. Its files stay in the Backup until they are added
/// ([`add_notes`]), so the file must stay where it is until then.
#[derive(Clone)]
pub struct OpenedBackup {
    /// Every Note of the Backup but those merged into another and those
    /// of a locked Area closed on this Device, those in its trash among
    /// them, as [`Store::pack_of_everything`] lists them; its files
    /// without their bytes.
    pub pack: Pack,
    /// The indices in `pack` of the Notes that lay in the Backup's trash.
    pub trashed: BTreeSet<usize>,
    /// The locked Areas closed on this Device whose Notes in the Backup
    /// were left out of `pack`.
    pub left_out: BTreeSet<String>,
    /// When the Backup was made.
    pub created: DateTime<Utc>,
    path: PathBuf,
    line: Vec<u8>,
    content_key: [u8; KEY_BYTES],
}

impl std::fmt::Debug for OpenedBackup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never its content key.
        f.debug_struct("OpenedBackup")
            .field("pack", &self.pack)
            .field("trashed", &self.trashed)
            .field("left_out", &self.left_out)
            .field("created", &self.created)
            .field("path", &self.path)
            .finish_non_exhaustive()
    }
}

/// Opens the Backup at `path` with `backup_key` (through [`unlock`]) to
/// take Notes back from it, as the Dart core's `openBackupNotes`, and
/// leaves `store` as it is: nothing of the Backup is restored, kept or
/// taken in. Its entries go into a store in memory of their own, read as
/// the Device that made the Backup read them, its signing key and
/// Profile and the keys it pinned, so the person's own Devices and the
/// roles of the Shared Areas count as they did there; its blobs are
/// checked against their hashes and only their sizes kept. `shown` names
/// the Areas as the app shows them, for their order. The Notes of
/// `closed`, the locked Areas closed on this Device, stay out, as the app
/// shows them nowhere then (ADR 0043); an Area of the Backup that is no
/// Area here has no lock here, and the desktop, without Area Locks,
/// hands in none. The same refusals as [`restore`]:
/// [`Problem::NotABackup`], [`Problem::NeedsUpdate`],
/// [`Problem::WrongSecret`] or [`Problem::Damaged`].
pub fn open_notes(
    store: &Store,
    path: &Path,
    backup_key: &[u8; KEY_BYTES],
    closed: &HashSet<String>,
    shown: impl Fn(&Area) -> String,
) -> Result<OpenedBackup> {
    let (header, content_key) = opened(path, backup_key)?;
    let mut sizes: HashMap<String, usize> = HashMap::new();
    let payload = read_payload(path, &header, &content_key, &mut |kind, hash, bytes| {
        if !is_blob(kind, hash, &bytes) {
            return Err(problem(Problem::Damaged));
        }
        sizes.insert(format!("{kind}/{hash}"), bytes.len());
        Ok(())
    })?;
    let now = store.now();
    // The Device that made it, so what it read as its own reads so here.
    let seed = *payload.old.seed();
    let view = Store::in_memory(Box::new(move || now), move || {
        Ok(SigningKey::from_seed(seed))
    })?;
    view.batch(|| {
        for e in &payload.entries {
            view.insert(e)?;
        }
        let mut report = ImportReport::default();
        for p in &payload.pinned {
            let verified = (p.trust == KeyTrust::Verified).then_some(p.record.device.as_str());
            view.learn_keys(std::slice::from_ref(&p.record), verified, &mut report)?;
        }
        OwnProfile::new(&view).restore(payload.profile.as_ref().map(|(p, _)| p), &payload.old)?;
        Ok(())
    })?;
    let size_of = |kind: &str, hash: &str| sizes.get(&format!("{kind}/{hash}")).copied();
    let everything = view.pack_of_everything(closed, shown, &size_of)?;
    Ok(OpenedBackup {
        pack: everything.pack,
        trashed: everything.trashed,
        left_out: everything.left_out,
        created: header.created,
        path: path.to_path_buf(),
        line: header.line,
        content_key,
    })
}

/// "Add" on the Backup `opened` with `ticks` (#285), as the Dart core's
/// `addBackupNotes`: the ticked Notes as copies, as a Pack's are added
/// ([`Store::add_pack`]), claimed, into new Areas named after the
/// Backup's, never into one there already. The files ticked are read
/// from the Backup one after the other and kept, each checked against
/// its hash, before the log names them; nothing of the Backup's
/// identity, keys or roles is taken, and the log is not touched
/// otherwise. [`Problem::Damaged`] when the file changed since it was
/// opened, before anything is written to the log; a file found damaged
/// half-way leaves blobs nobody refers to, never a half log.
pub fn add_notes(
    store: &Store,
    opened: &OpenedBackup,
    ticks: &PackTicks,
    wall_clock: &WallClock,
    suffix: &str,
    no_area: &str,
) -> Result<PackAdded> {
    let damaged = || problem(Problem::Damaged);
    let picked = opened.pack.picked(ticks);
    let mut needed: HashSet<String> = HashSet::new();
    for n in &picked.notes {
        for (kind, files) in [
            ("photo", &n.photos),
            ("audio", &n.recordings),
            ("file", &n.attachments),
        ] {
            for f in files.iter().filter(|f| f.has_file) {
                needed.insert(format!("{kind}/{}", f.hash));
            }
        }
    }
    if !needed.is_empty() {
        let header = match read_header(&opened.path) {
            Ok(Some(header)) if header.line == opened.line => header,
            _ => return Err(damaged()),
        };
        let mut kept: HashSet<String> = HashSet::new();
        let read = read_payload(
            &opened.path,
            &header,
            &opened.content_key,
            &mut |kind, hash, bytes| {
                let reference = format!("{kind}/{hash}");
                if !needed.contains(&reference) {
                    return Ok(());
                }
                if !store.put_blob(kind, hash, &bytes)? {
                    return Err(damaged());
                }
                kept.insert(reference);
                Ok(())
            },
        );
        match read {
            Ok(_) => {}
            Err(Error::Io(_)) => return Err(damaged()),
            Err(e) => return Err(e),
        }
        if kept.len() != needed.len() {
            return Err(damaged());
        }
    }
    // The bytes are here under the hashes the Pack names.
    let pack = OpenedPack {
        pack: opened.pack.clone(),
        from: None,
        from_kept: false,
    };
    store.add_pack(&pack, ticks, true, wall_clock, suffix, no_area)
}

// ------------------------------------------------------------------ folder

/// The name of this Device's Backup made at `at`.
pub fn file_name(at: DateTime<Utc>, device: &str) -> String {
    let short: String = device.chars().take(8).collect();
    format!(
        "oldbrainfix-{}-{short}.{EXTENSION}",
        at.format("%Y%m%d-%H%M%S")
    )
}

/// Whether `name` is a Backup of `device` as [`file_name`] names it.
pub fn is_backup_of(name: &str, device: &str) -> bool {
    let short: String = device.chars().take(8).collect();
    let Some(rest) = name
        .strip_prefix("oldbrainfix-")
        .and_then(|r| r.strip_suffix(&format!("-{short}.{EXTENSION}")))
    else {
        return false;
    };
    let digits = |s: &str, n: usize| s.len() == n && s.bytes().all(|b| b.is_ascii_digit());
    matches!(rest.split_once('-'), Some((day, time)) if digits(day, 8) && digits(time, 6))
}

/// Writes a Backup into the directory `dir` now and lets only the
/// newest [`KEPT`] of this Device stay. Returns its name; a failure is
/// kept for the screen and returned.
pub fn back_up_now(store: &Store, dir: &Path, random: Random) -> Result<String> {
    let now = store.now();
    let result = back_up_into(store, dir, now, random);
    match &result {
        Ok(_) => {
            store.set_local_setting(LAST, &dart_iso(now))?;
            store.remove_local_setting(FAILED)?;
        }
        Err(_) => store.set_local_setting(FAILED, &dart_iso(now))?,
    }
    result
}

fn back_up_into(store: &Store, dir: &Path, now: DateTime<Utc>, random: Random) -> Result<String> {
    let device = store.device_id()?;
    let name = file_name(now, &device);
    // Beside, then renamed: a cloud client must never upload half a file
    // as the whole.
    let tmp = dir.join(format!("{name}.tmp"));
    if let Err(e) = write_backup(store, &tmp, random) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    std::fs::rename(&tmp, dir.join(&name))?;
    let mut mine: Vec<String> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| is_backup_of(n, &device))
        .collect();
    mine.sort();
    let over = mine.len().saturating_sub(KEPT);
    for old in mine.iter().take(over) {
        std::fs::remove_file(dir.join(old))?;
    }
    Ok(name)
}

/// Writes the scheduled Backup into the chosen folder when it is due;
/// its name, or none when none was due.
pub fn back_up_if_due(store: &Store, random: Random) -> Result<Option<String>> {
    let Some(folder) = store.backup_folder()? else {
        return Ok(None);
    };
    if !store.backup_due(store.now())? {
        return Ok(None);
    }
    back_up_now(store, Path::new(&folder), random).map(Some)
}

#[cfg(test)]
mod tests;
