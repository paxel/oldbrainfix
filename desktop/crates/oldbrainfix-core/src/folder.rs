//! Sync through a shared folder (Syncthing, Nextcloud, a USB stick), as
//! the Dart core's `folderSync` does it (ADR 0003, 0012; Catlog's ADR
//! 0010): per Device a run of segments per key, `<device>.<key id>.<n>.seg`,
//! one sealed entry per line, with a manifest beside them; the keys a
//! Device holds under `keys/`, sealed with each personal key; the blobs
//! under `blobs/`, sealed and named by an HMAC. Only the manifests are
//! not sealed. The same layout, names, bytes and nonce order as the Dart
//! core, so either core reads what the other wrote.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde_json::{Value, json};

use crate::conflicts::Conflict;
use crate::entry::{Entry, WireEntry};
use crate::keys::{
    AreaKey, PERSONAL_SCOPE, SEAL_NONCE_BYTES, area_keys_of, blob_name, random_bytes, seal,
    sealed_as, unseal,
};
use crate::signing::{ImportReport, KeyRecord};
use crate::store::Store;
use crate::sync::{BLOB_KINDS, Moment, cause, split_blob_ref};
use crate::{Error, Result};

/// The folder everything lives in, inside the one picked.
pub const ROOT: &str = "oldbrainfix-sync";

/// A segment is closed once it passes this many bytes.
pub const SEGMENT_CAP: usize = 64 * 1024;

/// The manifest's format; a reader skips a higher one.
pub const MANIFEST_FORMAT: i64 = 1;

/// The shared folder as the sync sees it: the root (`""`), `keys` and
/// `blobs`.
pub trait SyncFolder {
    /// The file names directly in `dir`, sorted; an absent one is empty.
    fn list(&self, dir: &str) -> Result<Vec<String>>;
    /// The file's bytes, or none when it is not there.
    fn read(&self, dir: &str, name: &str) -> Result<Option<Vec<u8>>>;
    /// Writes `bytes` as `name` in `dir`, replacing what was there.
    fn write(&mut self, dir: &str, name: &str, bytes: &[u8]) -> Result<()>;
    fn delete(&mut self, dir: &str, name: &str) -> Result<()>;
    /// Makes sure `dir` exists (`""` makes the root).
    fn ensure(&mut self, dir: &str) -> Result<()>;
}

/// The folder as plain files under `<path>/oldbrainfix-sync`.
pub struct LocalFolder {
    root: PathBuf,
}

impl LocalFolder {
    pub fn new(path: impl AsRef<Path>) -> LocalFolder {
        LocalFolder {
            root: path.as_ref().join(ROOT),
        }
    }

    fn dir(&self, dir: &str) -> PathBuf {
        if dir.is_empty() {
            self.root.clone()
        } else {
            self.root.join(dir)
        }
    }

    /// A name from the folder that stays inside it.
    fn file(&self, dir: &str, name: &str) -> Result<PathBuf> {
        if name.is_empty() || name.contains(['/', '\\']) || name == "." || name == ".." {
            return Err(Error::Invalid(format!("not a folder file: {name:?}")));
        }
        Ok(self.dir(dir).join(name))
    }
}

impl SyncFolder for LocalFolder {
    fn list(&self, dir: &str) -> Result<Vec<String>> {
        let path = self.dir(dir);
        if !path.is_dir() {
            return Ok(Vec::new());
        }
        let mut names = Vec::new();
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            if entry.file_type()?.is_file() {
                names.push(entry.file_name().to_string_lossy().into_owned());
            }
        }
        names.sort();
        Ok(names)
    }

    fn read(&self, dir: &str, name: &str) -> Result<Option<Vec<u8>>> {
        let path = self.file(dir, name)?;
        match std::fs::read(path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn write(&mut self, dir: &str, name: &str, bytes: &[u8]) -> Result<()> {
        // Beside, then renamed: a cloud client must never upload half a
        // file as the whole.
        let path = self.file(dir, name)?;
        let tmp = self.file(dir, &format!("{name}.tmp"))?;
        std::fs::write(&tmp, bytes)?;
        std::fs::rename(tmp, path)?;
        Ok(())
    }

    fn delete(&mut self, dir: &str, name: &str) -> Result<()> {
        match std::fs::remove_file(self.file(dir, name)?) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.into()),
            _ => Ok(()),
        }
    }

    /// Makes `dir`, inside the picked folder only: one that was moved or
    /// deleted is gone, not made again where nobody syncs it.
    fn ensure(&mut self, dir: &str) -> Result<()> {
        if !self.root.parent().is_some_and(Path::is_dir) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("the folder is gone: {}", self.root.display()),
            )
            .into());
        }
        std::fs::create_dir_all(self.dir(dir))?;
        Ok(())
    }
}

/// A folder in memory, for tests.
#[derive(Debug, Default, Clone)]
pub struct MemoryFolder {
    pub dirs: BTreeMap<String, BTreeMap<String, Vec<u8>>>,
}

impl SyncFolder for MemoryFolder {
    fn list(&self, dir: &str) -> Result<Vec<String>> {
        Ok(self
            .dirs
            .get(dir)
            .map(|files| files.keys().cloned().collect())
            .unwrap_or_default())
    }

    fn read(&self, dir: &str, name: &str) -> Result<Option<Vec<u8>>> {
        Ok(self
            .dirs
            .get(dir)
            .and_then(|files| files.get(name))
            .cloned())
    }

    fn write(&mut self, dir: &str, name: &str, bytes: &[u8]) -> Result<()> {
        self.dirs
            .entry(dir.to_string())
            .or_default()
            .insert(name.to_string(), bytes.to_vec());
        Ok(())
    }

    fn delete(&mut self, dir: &str, name: &str) -> Result<()> {
        if let Some(files) = self.dirs.get_mut(dir) {
            files.remove(name);
        }
        Ok(())
    }

    fn ensure(&mut self, dir: &str) -> Result<()> {
        self.dirs.entry(dir.to_string()).or_default();
        Ok(())
    }
}

pub fn manifest_name(device: &str) -> String {
    format!("{device}.manifest")
}

pub fn segment_name(device: &str, key: &str, n: usize) -> String {
    format!("{device}.{key}.{n}.seg")
}

pub fn keys_file_name(device: &str, key: &str) -> String {
    format!("{device}.{key}.keys")
}

/// One segment as the manifest names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub name: String,
    pub key: String,
    pub lines: usize,
}

/// What a Device says about its segments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub format: i64,
    pub generation: i64,
    pub vector: BTreeMap<String, i64>,
    pub segments: Vec<Segment>,
}

impl Manifest {
    /// Its bytes, as the Dart core writes them: keys sorted, no spaces.
    pub fn encode(&self) -> Vec<u8> {
        let segments: Vec<Value> = self
            .segments
            .iter()
            .map(|s| json!({"key": s.key, "lines": s.lines, "name": s.name}))
            .collect();
        json!({
            "format": self.format,
            "generation": self.generation,
            "segments": segments,
            "vector": self.vector,
        })
        .to_string()
        .into_bytes()
    }

    pub fn parse(bytes: Option<&[u8]>) -> Option<Manifest> {
        let m: Value = serde_json::from_slice(bytes?).ok()?;
        let format = m.get("format")?.as_i64()?;
        if format > MANIFEST_FORMAT {
            return None;
        }
        let vector = m
            .get("vector")?
            .as_object()?
            .iter()
            .filter_map(|(k, v)| v.as_i64().map(|n| (k.clone(), n)))
            .collect();
        let segments = m
            .get("segments")?
            .as_array()?
            .iter()
            .filter_map(|s| {
                Some(Segment {
                    name: s.get("name")?.as_str()?.to_string(),
                    key: s.get("key")?.as_str()?.to_string(),
                    lines: usize::try_from(s.get("lines")?.as_u64()?).ok()?,
                })
            })
            .collect();
        Some(Manifest {
            format,
            generation: m.get("generation")?.as_i64()?,
            vector,
            segments,
        })
    }
}

/// What one folder sync did.
#[derive(Debug, Clone)]
pub struct FolderSyncResult {
    pub applied: Vec<WireEntry>,
    pub entries_out: usize,
    pub blobs_in: usize,
    pub blobs_out: usize,
    /// Segments of other Devices sealed with a key not held here.
    pub unreadable: usize,
    pub moment: Option<Moment>,
    pub report: ImportReport,
    pub conflicts: Vec<Conflict>,
}

/// Where the nonces come from: random, or counted in the fixture tests.
pub type Nonces<'a> = &'a mut dyn FnMut() -> Result<[u8; SEAL_NONCE_BYTES]>;

/// Fresh random nonces.
pub fn random_nonce() -> Result<[u8; SEAL_NONCE_BYTES]> {
    random_bytes()
}

/// Syncs `store` through `folder`, as the Dart core's `folderSync`:
/// learns the keys the others published, takes in what is new in their
/// segments, then publishes this Device's keys, entries and blobs and
/// fetches the blobs the log lacks. The nonces come from `nonce` in the
/// Dart core's order: keys files, entry lines, blobs.
pub fn folder_sync(
    store: &Store,
    folder: &mut dyn SyncFolder,
    nonce: Nonces<'_>,
) -> Result<FolderSyncResult> {
    folder.ensure("")?;
    folder.ensure("keys")?;
    folder.ensure("blobs")?;
    if folder.read("", ".nomedia")?.is_none() {
        // Keeps the photos out of a phone's gallery.
        folder.write("", ".nomedia", &[])?;
    }
    let mut report = ImportReport::default();
    learn_keys(store, folder, &mut report)?;
    let before = store.current_seq()?;
    let (applied, unreadable) = read_foreign(store, folder, &mut report, true)?;
    let moment = store.moment_for(before, !applied.is_empty(), cause::FOLDER, None)?;
    store.ensure_area_keys()?;
    publish_keys(store, folder, nonce)?;
    let entries_out = publish_entries(store, folder, nonce)?;
    let mut names: BTreeSet<String> = folder.list("blobs")?.into_iter().collect();
    let blobs_in = fetch_blobs(store, folder, &names)?;
    let blobs_out = publish_blobs(store, folder, &mut names, nonce, None)?;
    let conflicts = store.conflicts_among(&applied)?;
    Ok(FolderSyncResult {
        applied,
        entries_out,
        blobs_in,
        blobs_out,
        unreadable,
        moment,
        report,
        conflicts,
    })
}

/// Reads the sealed layout of `folder` as [`folder_sync`] reads the
/// shared folder, for a bundle, as the Dart core's `readSealed`: learns
/// the keys, takes in every other Device's segments of a key held here
/// from their first line, remembering nothing of how far it read, with a
/// Moment of `cause` before what was new, and the blobs the log lacks.
/// Writes nothing.
pub fn read_sealed(
    store: &Store,
    folder: &dyn SyncFolder,
    cause: &str,
    label: Option<&str>,
) -> Result<FolderSyncResult> {
    let mut report = ImportReport::default();
    learn_keys(store, folder, &mut report)?;
    let before = store.current_seq()?;
    let (applied, unreadable) = read_foreign(store, folder, &mut report, false)?;
    let moment = store.moment_for(before, !applied.is_empty(), cause, label)?;
    let names: BTreeSet<String> = folder.list("blobs")?.into_iter().collect();
    let blobs_in = fetch_blobs(store, folder, &names)?;
    let conflicts = store.conflicts_among(&applied)?;
    Ok(FolderSyncResult {
        applied,
        entries_out: 0,
        blobs_in,
        blobs_out: 0,
        unreadable,
        moment,
        report,
        conflicts,
    })
}

/// Writes `entries` into the empty `folder` as [`folder_sync`] writes
/// this Device's part of the shared folder, for a bundle, as the Dart
/// core's `writeSealed`: the keys files, the segments under generation 1
/// with the manifest, and the blobs of `blobs`, with nonces in the same
/// order. Returns how many entries and blobs went out.
pub fn write_sealed(
    store: &Store,
    folder: &mut dyn SyncFolder,
    entries: &[Entry],
    blobs: &BTreeSet<String>,
    nonce: Nonces<'_>,
) -> Result<(usize, usize)> {
    store.ensure_area_keys()?;
    publish_keys(store, folder, nonce)?;
    let out = write_segments(store, folder, nonce, entries, Vec::new(), 1)?;
    let blobs_out = publish_blobs(store, folder, &mut BTreeSet::new(), nonce, Some(blobs))?;
    Ok((out, blobs_out))
}

/// The (device, key id) a name `<device>.<key id>.<rest>` names.
fn device_and_key(name: &str) -> Option<(&str, &str)> {
    let mut parts = name.split('.');
    let device = parts.next().filter(|d| !d.is_empty())?;
    let key = parts.next().filter(|k| !k.is_empty())?;
    parts.next()?;
    Some((device, key))
}

fn learn_keys(store: &Store, folder: &dyn SyncFolder, report: &mut ImportReport) -> Result<()> {
    let own = store.device_id()?;
    let files: Vec<(String, String, String)> = folder
        .list("keys")?
        .into_iter()
        .filter(|name| name.ends_with(".keys"))
        .filter_map(|name| {
            let (device, key) = device_and_key(&name)?;
            (device != own).then(|| (device.to_string(), key.to_string(), name.clone()))
        })
        .collect();
    let mut opened = BTreeSet::new();
    let mut learned = true;
    while learned {
        learned = false;
        for (device, id, name) in &files {
            if opened.contains(name) {
                continue;
            }
            let Some(key) = store.area_key(id)?.filter(|k| k.scope == PERSONAL_SCOPE) else {
                continue;
            };
            let Ok(Some(bytes)) = folder.read("keys", name) else {
                continue;
            };
            let Some(plain) = unseal(&key.key, &bytes, &sealed_as::keys(device)) else {
                continue;
            };
            opened.insert(name.clone());
            let Ok(json) = serde_json::from_slice::<Value>(&plain) else {
                continue;
            };
            let signing: Vec<KeyRecord> = json
                .get("signing")
                .and_then(Value::as_array)
                .map(|list| list.iter().filter_map(KeyRecord::from_json).collect())
                .unwrap_or_default();
            store.learn_keys(&signing, None, report)?;
            if store.learn_area_keys(&area_keys_of(json.get("areaKeys")))? > 0 {
                learned = true;
            }
        }
    }
    Ok(())
}

fn read_state_key(device: &str) -> String {
    format!("folderRead:{device}")
}

/// The generation this Device read of `device`'s segments and the lines
/// it took of each.
fn read_state(store: &Store, device: &str) -> Result<(i64, BTreeMap<String, usize>)> {
    let state = store
        .local_setting(&read_state_key(device))?
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .and_then(|m| {
            let generation = m.get("generation")?.as_i64()?;
            let lines = m
                .get("lines")?
                .as_object()?
                .iter()
                .filter_map(|(k, v)| Some((k.clone(), usize::try_from(v.as_u64()?).ok()?)))
                .collect();
            Some((generation, lines))
        });
    Ok(state.unwrap_or((-1, BTreeMap::new())))
}

fn split_lines(bytes: &[u8]) -> Option<Vec<String>> {
    let text = std::str::from_utf8(bytes).ok()?;
    Some(text.lines().map(str::to_string).collect())
}

/// The entries of `segment` from line `have` on, or none when it is not
/// all there yet or does not open.
fn segment_entries(
    folder: &dyn SyncFolder,
    segment: &Segment,
    have: usize,
    key: &AreaKey,
) -> Option<Vec<WireEntry>> {
    let bytes = folder.read("", &segment.name).ok()??;
    let lines = split_lines(&bytes)?;
    if lines.len() < segment.lines {
        return None;
    }
    lines[have..segment.lines]
        .iter()
        .map(|line| {
            let sealed = BASE64.decode(line).ok()?;
            let plain = unseal(&key.key, &sealed, sealed_as::ENTRY)?;
            serde_json::from_slice(&plain).ok()
        })
        .collect()
}

/// Reads every other Device's segments sealed with a key held here, as
/// the Dart core does; never filtered by the version vector.
fn read_foreign(
    store: &Store,
    folder: &dyn SyncFolder,
    report: &mut ImportReport,
    remember: bool,
) -> Result<(Vec<WireEntry>, usize)> {
    let own = store.device_id()?;
    let mut fresh = Vec::new();
    let mut states = Vec::new();
    let mut unreadable = 0;
    for name in folder.list("")? {
        let Some(device) = name.strip_suffix(".manifest") else {
            continue;
        };
        if device == own || device.is_empty() {
            continue;
        }
        let Ok(bytes) = folder.read("", &name) else {
            continue;
        };
        let Some(manifest) = Manifest::parse(bytes.as_deref()) else {
            continue;
        };
        let (mut generation, mut lines) = if remember {
            read_state(store, device)?
        } else {
            (-1, BTreeMap::new())
        };
        if generation != manifest.generation {
            generation = manifest.generation;
            lines = BTreeMap::new();
        }
        let mut taken = lines.clone();
        let mut entries = Vec::new();
        let mut complete = true;
        for segment in &manifest.segments {
            let have = lines.get(&segment.name).copied().unwrap_or(0);
            if segment.lines <= have {
                continue;
            }
            let Some(key) = store.area_key(&segment.key)? else {
                unreadable += 1;
                continue;
            };
            match segment_entries(folder, segment, have, &key) {
                Some(got) => {
                    entries.extend(got);
                    taken.insert(segment.name.clone(), segment.lines);
                }
                None => {
                    complete = false;
                    break;
                }
            }
        }
        if !complete {
            continue;
        }
        fresh.extend(entries);
        states.push((device.to_string(), generation, taken));
    }
    let applied = store.apply_entries(&fresh, &[], None, report)?;
    if !remember {
        return Ok((applied, unreadable));
    }
    for (device, generation, lines) in states {
        store.set_local_setting(
            &read_state_key(&device),
            &json!({"generation": generation, "lines": lines}).to_string(),
        )?;
    }
    Ok((applied, unreadable))
}

fn publish_keys(store: &Store, folder: &mut dyn SyncFolder, nonce: Nonces<'_>) -> Result<()> {
    let own = store.device_id()?;
    let keys = store.area_keys()?;
    let plain = json!({
        "areaKeys": keys.iter().map(AreaKey::to_json).collect::<Vec<_>>(),
        "signing": store.key_records()?.iter().map(KeyRecord::to_json).collect::<Vec<_>>(),
    })
    .to_string()
    .into_bytes();
    let aad = sealed_as::keys(&own);
    for key in keys.iter().filter(|k| k.scope == PERSONAL_SCOPE) {
        let name = keys_file_name(&own, &key.id());
        let said = folder
            .read("keys", &name)?
            .and_then(|held| unseal(&key.key, &held, &aad));
        if said.as_deref() == Some(plain.as_slice()) {
            continue;
        }
        let sealed = seal(&key.key, &plain, &aad, &nonce()?)?;
        folder.write("keys", &name, &sealed)?;
    }
    Ok(())
}

/// The number a segment's name carries.
fn number(name: &str) -> usize {
    let parts: Vec<&str> = name.split('.').collect();
    if parts.len() < 4 {
        return 0;
    }
    parts[parts.len() - 2].parse().unwrap_or(0)
}

fn publish_entries(store: &Store, folder: &mut dyn SyncFolder, nonce: Nonces<'_>) -> Result<usize> {
    let own = store.device_id()?;
    let previous = Manifest::parse(folder.read("", &manifest_name(&own))?.as_deref());
    let present: BTreeSet<String> = folder.list("")?.into_iter().collect();
    let previous = previous.filter(|p| {
        p.format == MANIFEST_FORMAT && p.segments.iter().all(|s| present.contains(&s.name))
    });
    let since: HashMap<String, i64> = previous
        .as_ref()
        .map(|p| p.vector.clone().into_iter().collect())
        .unwrap_or_default();
    // A Note filed into a Shared Area goes out whole under its key: what
    // it held before was sealed with another (ADR 0013).
    let judged = store.judged()?;
    let entries = store.with_filed(store.entries_since(&since)?, &|area| {
        judged.areas.contains_key(area)
    })?;
    if previous.is_some() && entries.is_empty() {
        return Ok(0);
    }

    if previous.is_none() {
        for name in &present {
            if name.starts_with(&format!("{own}.")) && name.ends_with(".seg") {
                folder.delete("", name)?;
            }
        }
    }
    let generation = match &previous {
        Some(p) => p.generation,
        None => {
            Manifest::parse(folder.read("", &manifest_name(&own))?.as_deref())
                .map(|p| p.generation)
                .unwrap_or(0)
                + 1
        }
    };
    let segments = previous.map(|p| p.segments).unwrap_or_default();
    write_segments(store, folder, nonce, &entries, segments, generation)
}

/// Seals `entries` in log order, each with the key of its Area as it is
/// now, appends them to the last of `segments` of that key or a fresh
/// one past [`SEGMENT_CAP`], and writes the manifest of `generation`
/// after, as the Dart core's `_writeSegments`. Returns how many entries
/// went out.
fn write_segments(
    store: &Store,
    folder: &mut dyn SyncFolder,
    nonce: Nonces<'_>,
    entries: &[Entry],
    mut segments: Vec<Segment>,
    generation: i64,
) -> Result<usize> {
    let own = store.device_id()?;
    let mut key_of: HashMap<String, Option<AreaKey>> = HashMap::new();
    let mut lines: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut out = 0;
    for e in entries {
        let key = match key_of.get(&e.entity) {
            Some(key) => key.clone(),
            None => {
                let key = store.sealing_key(&e.entity)?;
                key_of.insert(e.entity.clone(), key.clone());
                key
            }
        };
        let Some(key) = key else {
            continue;
        };
        let plain = serde_json::to_vec(&e.wire()).map_err(|e| Error::Invalid(e.to_string()))?;
        let sealed = seal(&key.key, &plain, sealed_as::ENTRY, &nonce()?)?;
        lines
            .entry(key.id())
            .or_default()
            .push(BASE64.encode(sealed));
        out += 1;
    }

    for (id, fresh) in lines {
        let run: Vec<Segment> = segments.iter().filter(|s| s.key == id).cloned().collect();
        let mut n = run.len();
        let mut buffer: Vec<String> = Vec::new();
        if let Some(tail) = run.last()
            && let Some(bytes) = folder.read("", &tail.name)?
            && bytes.len() < SEGMENT_CAP
        {
            buffer = split_lines(&bytes).unwrap_or_default();
            buffer.truncate(tail.lines);
            segments.retain(|s| s.name != tail.name);
            n -= 1;
        }
        let mut size: usize = buffer.iter().map(|l| l.len() + 1).sum();
        for line in fresh {
            size += line.len() + 1;
            buffer.push(line);
            if size >= SEGMENT_CAP {
                n += 1;
                flush(folder, &own, &id, n, &mut buffer, &mut segments)?;
                size = 0;
            }
        }
        if !buffer.is_empty() {
            n += 1;
            flush(folder, &own, &id, n, &mut buffer, &mut segments)?;
        }
    }
    segments.sort_by(|a, b| {
        a.key
            .cmp(&b.key)
            .then(number(&a.name).cmp(&number(&b.name)))
    });
    let manifest = Manifest {
        format: MANIFEST_FORMAT,
        generation,
        vector: store.version_vector()?.into_iter().collect(),
        segments,
    };
    folder.write("", &manifest_name(&own), &manifest.encode())?;
    Ok(out)
}

fn flush(
    folder: &mut dyn SyncFolder,
    own: &str,
    id: &str,
    n: usize,
    buffer: &mut Vec<String>,
    segments: &mut Vec<Segment>,
) -> Result<()> {
    let name = segment_name(own, id, n);
    folder.write("", &name, buffer.join("\n").as_bytes())?;
    segments.push(Segment {
        name,
        key: id.to_string(),
        lines: buffer.len(),
    });
    buffer.clear();
    Ok(())
}

fn fetch_blobs(store: &Store, folder: &dyn SyncFolder, names: &BTreeSet<String>) -> Result<usize> {
    let missing = store.missing_blobs()?;
    if missing.is_empty() {
        return Ok(0);
    }
    let keys = store.area_keys()?;
    let mut fetched = 0;
    for reference in missing {
        let (kind, hash) = split_blob_ref(&reference);
        for key in &keys {
            let name = blob_name(&key.key, &reference);
            if !names.contains(&name) {
                continue;
            }
            let Ok(Some(bytes)) = folder.read("blobs", &name) else {
                continue;
            };
            if let Some(plain) = unseal(&key.key, &bytes, &sealed_as::blob(&reference))
                && store.put_blob(kind, hash, &plain)?
            {
                fetched += 1;
                break;
            }
        }
    }
    Ok(fetched)
}

fn publish_blobs(
    store: &Store,
    folder: &mut dyn SyncFolder,
    names: &mut BTreeSet<String>,
    nonce: Nonces<'_>,
    only: Option<&BTreeSet<String>>,
) -> Result<usize> {
    let mut wanted: BTreeMap<(String, String), AreaKey> = BTreeMap::new();
    for (kind, prefix) in BLOB_KINDS {
        for (entity, field) in store.fields_with_prefix(prefix)? {
            let reference = format!("{kind}/{}", &field[prefix.len()..]);
            if only.is_some_and(|only| !only.contains(&reference)) {
                continue;
            }
            if store.current(&entity, &field)?.is_none() {
                continue;
            }
            let Some(key) = store.sealing_key(&entity)? else {
                continue;
            };
            wanted.insert((reference, key.id()), key);
        }
    }
    let mut out = 0;
    for ((reference, _), key) in wanted {
        let name = blob_name(&key.key, &reference);
        if names.contains(&name) {
            continue;
        }
        let (kind, hash) = split_blob_ref(&reference);
        let Some(bytes) = store.blob(kind, hash) else {
            continue;
        };
        let sealed = seal(&key.key, &bytes, &sealed_as::blob(&reference), &nonce()?)?;
        folder.write("blobs", &name, &sealed)?;
        names.insert(name);
        out += 1;
    }
    Ok(out)
}

const FOLDER_KEY: &str = "folder";
const STATUS_KEY: &str = "folder:last";
const READ_PREFIX: &str = "folderRead:";

/// How the last folder sync on this Device went, as the Dart core's
/// `FolderStatus`, kept as a local setting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderStatus {
    pub at: chrono::DateTime<chrono::Utc>,
    pub entries_in: usize,
    pub unreadable: usize,
    pub failed: bool,
}

impl FolderStatus {
    pub fn to_json(&self) -> Value {
        json!({
            "at": crate::dart_iso(self.at),
            "entriesIn": self.entries_in,
            "unreadable": self.unreadable,
            "failed": self.failed,
        })
    }

    pub fn from_json(json: &Value) -> Option<FolderStatus> {
        Some(FolderStatus {
            at: crate::parse_time(json.get("at")?.as_str()?).ok()?,
            entries_in: usize::try_from(json.get("entriesIn")?.as_u64()?).ok()?,
            unreadable: usize::try_from(json.get("unreadable")?.as_u64()?).ok()?,
            failed: json.get("failed")?.as_bool()?,
        })
    }
}

impl Store {
    /// The shared folder, if one was chosen: a path here.
    pub fn sync_folder(&self) -> Result<Option<String>> {
        self.local_setting(FOLDER_KEY)
    }

    /// Chooses `folder`; what was read from another folder is forgotten.
    pub fn choose_sync_folder(&self, folder: &str) -> Result<()> {
        if self.sync_folder()?.as_deref() == Some(folder) {
            return Ok(());
        }
        self.forget_sync_folder()?;
        self.set_local_setting(FOLDER_KEY, folder)
    }

    /// Forgets the shared folder; nothing in it is touched.
    pub fn forget_sync_folder(&self) -> Result<()> {
        self.remove_local_setting(FOLDER_KEY)?;
        self.remove_local_setting(STATUS_KEY)?;
        for (key, _) in self.local_settings_with_prefix(READ_PREFIX)? {
            self.remove_local_setting(&key)?;
        }
        Ok(())
    }

    /// How the last folder sync went, or none before the first.
    pub fn folder_status(&self) -> Result<Option<FolderStatus>> {
        Ok(self
            .local_setting(STATUS_KEY)?
            .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
            .and_then(|json| FolderStatus::from_json(&json)))
    }

    fn set_folder_status(&self, status: &FolderStatus) -> Result<()> {
        self.set_local_setting(STATUS_KEY, &status.to_json().to_string())
    }
}

/// One folder sync as the app runs it, its outcome kept as the
/// [`FolderStatus`]: none when the folder could not be reached, which is
/// kept too. A store that fails is an error.
pub fn folder_round(
    store: &Store,
    folder: &mut dyn SyncFolder,
    nonce: Nonces<'_>,
) -> Result<Option<FolderSyncResult>> {
    match folder_sync(store, folder, nonce) {
        Ok(result) => {
            store.set_folder_status(&FolderStatus {
                at: store.now(),
                entries_in: result.applied.len(),
                unreadable: result.unreadable,
                failed: false,
            })?;
            Ok(Some(result))
        }
        Err(Error::Io(_)) => {
            store.set_folder_status(&FolderStatus {
                at: store.now(),
                entries_in: 0,
                unreadable: 0,
                failed: true,
            })?;
            Ok(None)
        }
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lan::{answer_sync, apply_answer, sync_request};
    use crate::signing::SigningKey;
    use crate::store::tests::ticking;
    use crate::{Notes, Places};

    type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

    fn device(seed: u8) -> Result<Store> {
        Store::in_memory(ticking(), move || Ok(SigningKey::from_seed([seed; 32])))
    }

    /// One LAN session without the network, as Pairing hands over keys.
    fn meet(joiner: &Store, host: &Store) -> TestResult {
        let request = sync_request(joiner, &host.version_vector()?, "Phone", "", "")?;
        let (answer, _) = answer_sync(host, &request, "Desk")?.ok_or("an answer")?;
        apply_answer(joiner, &answer)?;
        Ok(())
    }

    fn texts(store: &Store) -> Result<Vec<String>> {
        let mut texts: Vec<String> = Notes::new(store)
            .all()?
            .into_iter()
            .map(|n| n.text)
            .collect();
        texts.sort();
        Ok(texts)
    }

    fn sync(store: &Store, folder: &mut MemoryFolder) -> Result<FolderSyncResult> {
        folder_sync(store, folder, &mut random_nonce)
    }

    #[test]
    fn paired_devices_converge_and_strangers_read_nothing() -> TestResult {
        let phone = device(1)?;
        let desk = device(2)?;
        let doctor = Places::new(&desk).add_area("Doctor")?;
        meet(&phone, &desk)?;
        Notes::new(&phone).capture("Feeling dizzy", None, None)?;
        Notes::new(&desk).capture_with(
            "Blood test",
            Some(&doctor.id),
            None,
            &[b"xray".to_vec()],
            &[],
        )?;
        let kids = Places::new(&desk).add_area("Kids")?;
        let mut folder = MemoryFolder::default();
        sync(&phone, &mut folder)?;
        let second = sync(&desk, &mut folder)?;
        assert!(!second.applied.is_empty());
        let third = sync(&phone, &mut folder)?;
        assert_eq!(third.blobs_in, 1);
        assert_eq!(texts(&phone)?, texts(&desk)?);
        assert_eq!(
            phone.writing_key(&kids.id)?.map(|k| k.id()),
            desk.writing_key(&kids.id)?.map(|k| k.id())
        );
        let quiet = sync(&desk, &mut folder)?;
        assert!(quiet.applied.is_empty());
        assert_eq!(quiet.entries_out, 0);

        let stranger = device(3)?;
        let seen = sync(&stranger, &mut folder)?;
        assert!(seen.applied.is_empty());
        assert!(seen.unreadable > 0);
        for files in folder.dirs.values() {
            for bytes in files.values() {
                let text = String::from_utf8_lossy(bytes);
                for secret in ["dizzy", "Blood", "Doctor", "note:", "area:"] {
                    assert!(!text.contains(secret), "{secret}");
                }
            }
        }
        Ok(())
    }

    #[test]
    fn a_key_that_comes_late_fills_the_gap() -> TestResult {
        let phone = device(1)?;
        let desk = device(2)?;
        meet(&phone, &desk)?;
        let work = Places::new(&phone).add_area("Work")?;
        Notes::new(&phone).capture("Report", Some(&work.id), None)?;
        Notes::new(&phone).capture("Loose", None, None)?;
        let mut folder = MemoryFolder::default();
        sync(&phone, &mut folder)?;
        let keys = folder.dirs.get("keys").cloned().unwrap_or_default();
        folder.dirs.insert("keys".into(), BTreeMap::new());
        let early = sync(&desk, &mut folder)?;
        assert_eq!(early.unreadable, 1);
        assert_eq!(texts(&desk)?, vec!["Loose"]);
        folder.dirs.insert("keys".into(), keys);
        let late = sync(&desk, &mut folder)?;
        assert_eq!(late.unreadable, 0);
        assert_eq!(texts(&desk)?, texts(&phone)?);
        Ok(())
    }

    #[test]
    fn full_segments_stay_and_plain_files_carry_the_folder() -> TestResult {
        let phone = device(1)?;
        let desk = device(2)?;
        meet(&phone, &desk)?;
        let long = "x".repeat(2000);
        for i in 0..40 {
            Notes::new(&phone).capture(&format!("{i} {long}"), None, None)?;
        }
        let dir = tempfile::tempdir()?;
        folder_sync(&phone, &mut LocalFolder::new(dir.path()), &mut random_nonce)?;
        let own = phone.device_id()?;
        let read = |name: &str| std::fs::read(dir.path().join(ROOT).join(name));
        let manifest = Manifest::parse(Some(&read(&manifest_name(&own))?)).ok_or("manifest")?;
        assert!(manifest.segments.len() > 1);
        let first = read(&manifest.segments[0].name)?;
        assert!(first.len() >= SEGMENT_CAP);
        Notes::new(&phone).capture("After", None, None)?;
        folder_sync(&phone, &mut LocalFolder::new(dir.path()), &mut random_nonce)?;
        assert_eq!(read(&manifest.segments[0].name)?, first);
        folder_sync(&desk, &mut LocalFolder::new(dir.path()), &mut random_nonce)?;
        assert_eq!(texts(&desk)?, texts(&phone)?);
        Ok(())
    }

    #[test]
    fn a_round_keeps_how_it_went_and_a_new_folder_is_read_whole() -> TestResult {
        let phone = device(1)?;
        let desk = device(2)?;
        meet(&phone, &desk)?;
        Notes::new(&phone).capture("Hello", None, None)?;
        let mut folder = MemoryFolder::default();
        folder_round(&phone, &mut folder, &mut random_nonce)?;
        desk.choose_sync_folder("/mnt/a")?;
        let got = folder_round(&desk, &mut folder, &mut random_nonce)?.ok_or("a round")?;
        let status = desk.folder_status()?.ok_or("a status")?;
        assert_eq!(status.entries_in, got.applied.len());
        assert!(!status.failed);
        assert_eq!(FolderStatus::from_json(&status.to_json()), Some(status));
        let gone = tempfile::tempdir()?;
        let file = gone.path().join("file");
        std::fs::write(&file, b"")?;
        // A folder that is a file: nothing can be written into it.
        assert!(folder_round(&desk, &mut LocalFolder::new(&file), &mut random_nonce)?.is_none());
        assert_eq!(desk.folder_status()?.map(|s| s.failed), Some(true));
        // A folder moved away is gone, not made again.
        let moved = gone.path().join("moved");
        assert!(folder_round(&desk, &mut LocalFolder::new(&moved), &mut random_nonce)?.is_none());
        assert!(!moved.exists());
        assert!(!desk.local_settings_with_prefix("folderRead:")?.is_empty());
        desk.choose_sync_folder("/mnt/b")?;
        assert!(desk.local_settings_with_prefix("folderRead:")?.is_empty());
        assert_eq!(desk.sync_folder()?.as_deref(), Some("/mnt/b"));
        desk.forget_sync_folder()?;
        assert_eq!(desk.sync_folder()?, None);
        Ok(())
    }

    #[test]
    fn names_from_the_folder_stay_inside_it() {
        let folder = LocalFolder::new("/nowhere");
        assert!(folder.read("", "../etc").is_err());
        assert!(folder.read("blobs", "a/b").is_err());
    }
}
