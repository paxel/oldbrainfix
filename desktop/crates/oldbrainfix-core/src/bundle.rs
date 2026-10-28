//! Sync by bundle file (ADR 0003), as the Dart core's `bundle.dart`: what
//! is new for a paired Device, or everything, in one zip sent through any
//! messenger. Inside lies the shared folder's layout (ADR 0012), sealed
//! the same way: the manifest, the segments of entries per key, the keys
//! file sealed with the personal key and the blobs, so the folder's
//! reader serves both and a Device lacking the keys reads nothing. What
//! comes in is applied as from the folder: signatures, Conflicts from
//! causal contexts, a Moment before it.
//!
//! Every file is stored, not deflated (sealed bytes do not shrink), and
//! dated 1980-01-01. The files inside are the same bytes the Dart core
//! writes; the zip around them is this crate's own.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use zip::result::ZipError;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::conflicts::Conflict;
use crate::entry::WireEntry;
use crate::folder::{
    FolderSyncResult, Manifest, Nonces, SyncFolder, random_nonce, read_sealed, write_sealed,
};
use crate::offers::{AREA_OFFER_FILE, HeldAreaOffer, OfferError, encode_area_offer, random_vec};
use crate::signing::ImportReport;
use crate::store::Store;
use crate::sync::{BLOB_KINDS, Moment, cause};
use crate::{Error, Result};

/// A bundle's file extension.
pub const EXTENSION: &str = "obfsync";

/// A file inside a bundle larger than this is not one of ours and is
/// left out, as the Dart core's `maxBundleFileBytes`.
pub const MAX_FILE_BYTES: u64 = 16 << 20;
/// The same for a photo or a voice recording.
pub const MAX_BLOB_BYTES: u64 = 256 << 20;

/// The name a bundle written on `day` goes by.
pub fn file_name(day: chrono::NaiveDate) -> String {
    format!("oldbrainfix-{}.{EXTENSION}", day.format("%Y-%m-%d"))
}

/// What a bundle import did.
#[derive(Debug, Clone)]
pub struct BundleResult {
    pub applied: Vec<WireEntry>,
    pub blobs_in: usize,
    /// Segments sealed with a key not held here.
    pub unreadable: usize,
    pub moment: Option<Moment>,
    pub report: ImportReport,
    pub conflicts: Vec<Conflict>,
    /// The Device that wrote it, and the name it goes by here.
    pub from: String,
    pub from_name: String,
    /// Whether the own Profile changed, or "Which one is you?" came up
    /// (ADR 0017).
    pub profile_changed: bool,
    /// The Area the bundle offers, held until the user accepts or
    /// declines it (#108); nothing else of the bundle was taken in then.
    pub offer: Option<HeldAreaOffer>,
}

impl BundleResult {
    /// Nothing could be read for want of keys: pair first.
    pub fn needs_pairing(&self) -> bool {
        self.applied.is_empty() && self.unreadable > 0
    }
}

fn zip_error(e: ZipError) -> Error {
    match e {
        ZipError::Io(e) => Error::Io(e),
        other => Error::Invalid(other.to_string()),
    }
}

/// The path of `name` in `dir` inside the zip.
fn path_of(dir: &str, name: &str) -> String {
    if dir.is_empty() {
        name.to_string()
    } else {
        format!("{dir}/{name}")
    }
}

/// Writes a bundle to `path`, as the Dart core's `writeBundle`, and
/// returns how many entries and blobs it carries; none, writing nothing,
/// when there is nothing to send. For the paired Device `for_device` it
/// holds what lies beyond what that Device last said it holds, else
/// everything; the blobs only of the entries in it. For another person's
/// Device, paired by invite or met from afar
/// ([`Store::is_others_device`]), only what lies in the Areas it shares,
/// with the whole history of Notes filed into them meanwhile (ADR 0013).
pub fn write_bundle(
    store: &Store,
    path: &Path,
    for_device: Option<&str>,
    nonce: Nonces<'_>,
) -> Result<Option<(usize, usize)>> {
    let since = match for_device {
        Some(device) => store.held_by(device)?,
        None => HashMap::new(),
    };
    let entries = match for_device {
        Some(device) if store.is_others_device(device)? => {
            store.entries_for(device, &since, false, &[])?
        }
        _ => store.entries_since(&since)?,
    };
    if entries.is_empty() {
        return Ok(None);
    }
    write_zip(store, path, &entries, nonce, None).map(Some)
}

/// Writes a bundle for the person `person` to `path` (#108), as the Dart
/// core's `writeBundle` with `forPerson`: for all of their Devices at
/// once ([`Store::entries_for_person`]). None, writing nothing, when
/// there is nothing to send.
pub fn write_person_bundle(
    store: &Store,
    path: &Path,
    person: &str,
    nonce: Nonces<'_>,
) -> Result<Option<(usize, usize)>> {
    let entries = store.entries_for_person(person)?;
    if entries.is_empty() {
        return Ok(None);
    }
    write_zip(store, path, &entries, nonce, None).map(Some)
}

/// [`write_person_bundle`] with fresh random nonces.
pub fn write_person_bundle_now(
    store: &Store,
    path: &Path,
    person: &str,
) -> Result<Option<(usize, usize)>> {
    write_person_bundle(store, path, person, &mut random_nonce)
}

/// Writes the offer of the Shared Area `area` to the person `person`
/// (#108, ADR 0023) to `path`, as the Dart core's `writeAreaOffer`: a
/// bundle of the Area's whole history with its photos, recordings and
/// Profiles, and beside it the offer, signed by this Device and sealed
/// to the X25519 keys of the Devices of `person` the Area's grants name.
/// Returns how many entries and blobs it carries. An error, writing
/// nothing, when the grants name no Device of theirs whose key is known
/// here. The bundle's nonces come from `nonce`, the offer's bytes from
/// `random`.
pub fn write_area_offer(
    store: &Store,
    path: &Path,
    area: &str,
    person: &str,
    nonce: Nonces<'_>,
    random: &mut dyn FnMut(usize) -> Result<Vec<u8>>,
) -> Result<(usize, usize)> {
    let to = store.offer_keys(area, person)?;
    if to.is_empty() {
        return Err(Error::Invalid(format!("no Device of {person} to offer to")));
    }
    let offer = encode_area_offer(
        &store.area_offer_for(area, person)?,
        &store.signing_key()?,
        &to,
        random,
    )?;
    write_zip(store, path, &store.area_history(area)?, nonce, Some(&offer))
}

/// [`write_area_offer`] with fresh random bytes.
pub fn write_area_offer_now(
    store: &Store,
    path: &Path,
    area: &str,
    person: &str,
) -> Result<(usize, usize)> {
    write_area_offer(
        store,
        path,
        area,
        person,
        &mut random_nonce,
        &mut random_vec,
    )
}

/// Writes `entries` as a bundle to `path`, with their blobs and, given
/// one, the `offer` after them.
fn write_zip(
    store: &Store,
    path: &Path,
    entries: &[crate::entry::Entry],
    nonce: Nonces<'_>,
    offer: Option<&[u8]>,
) -> Result<(usize, usize)> {
    let mut blobs = BTreeSet::new();
    for e in entries {
        for (kind, prefix) in BLOB_KINDS {
            if let Some(hash) = e.field.strip_prefix(prefix) {
                blobs.insert(format!("{kind}/{hash}"));
            }
        }
    }
    let mut folder = ZipWriteFolder {
        writer: Some(ZipWriter::new(File::create(path)?)),
        written: BTreeMap::new(),
    };
    let wrote = write_sealed(store, &mut folder, entries, &blobs, nonce).and_then(|counts| {
        if let Some(offer) = offer {
            folder.write("", AREA_OFFER_FILE, offer)?;
        }
        Ok(counts)
    });
    let finished = folder
        .writer
        .take()
        .map(|w| w.finish().map_err(zip_error))
        .transpose();
    match (wrote, finished) {
        (Ok(counts), Ok(_)) => Ok(counts),
        (Err(e), _) | (_, Err(e)) => {
            // Half a bundle is none: nobody should send it.
            let _ = std::fs::remove_file(path);
            Err(e)
        }
    }
}

/// [`write_bundle`] with fresh random nonces.
pub fn write_bundle_now(
    store: &Store,
    path: &Path,
    for_device: Option<&str>,
) -> Result<Option<(usize, usize)>> {
    write_bundle(store, path, for_device, &mut random_nonce)
}

/// Imports the bundle at `path`, as the Dart core's `importBundle`:
/// takes in what is new and the blobs the log lacks, and keeps what the
/// writer said it holds. [`Error::NotABundle`] for a file that is none,
/// [`Error::BundleNeedsUpdate`] for one holding only newer formats.
///
/// A bundle that offers an Area (#108) is held as the offer instead
/// ([`Store::receive_area_offer`]) and returned in
/// [`BundleResult::offer`], nothing of it taken in, unless this Device
/// holds a role in the Area already; [`Error::Offer`] as that says,
/// [`OfferError::NotForUs`] only when nothing of the bundle could be
/// read either.
pub fn import_bundle(store: &Store, path: &Path) -> Result<BundleResult> {
    let file = File::open(path).map_err(|_| Error::NotABundle)?;
    let archive = ZipArchive::new(file).map_err(|_| Error::NotABundle)?;
    let folder = ZipReadFolder::new(archive);
    let mut writers: BTreeMap<String, Manifest> = BTreeMap::new();
    let mut newer = false;
    for name in folder.list("")? {
        let Some(device) = name.strip_suffix(".manifest") else {
            continue;
        };
        if device.is_empty() {
            continue;
        }
        let bytes = folder.read("", &name)?;
        if let Some(manifest) = Manifest::parse(bytes.as_deref()) {
            writers.insert(device.to_string(), manifest);
        }
        newer |= Manifest::is_newer(bytes.as_deref());
    }
    if writers.is_empty() && newer {
        return Err(Error::BundleNeedsUpdate);
    }
    let own = store.device_id()?;
    let from = writers
        .keys()
        .find(|d| **d != own)
        .or_else(|| writers.keys().next())
        .cloned()
        .ok_or(Error::NotABundle)?;
    let mut not_for_us = false;
    if let Some(offer) = folder.read("", AREA_OFFER_FILE)? {
        match store.receive_area_offer(&folder, &offer)? {
            Ok(Some(held)) => {
                return Ok(BundleResult {
                    applied: Vec::new(),
                    blobs_in: 0,
                    unreadable: 0,
                    moment: None,
                    report: ImportReport::default(),
                    conflicts: Vec::new(),
                    from,
                    from_name: String::new(),
                    profile_changed: false,
                    offer: Some(held),
                });
            }
            Ok(None) => {}
            // Another Device of the user's may have taken the Area in and
            // passed its key on: then the bundle reads as any other.
            Err(OfferError::NotForUs) => not_for_us = true,
            Err(e) => return Err(Error::Offer(e)),
        }
    }
    let from_name = store
        .paired_device(&from)?
        .map(|d| d.name)
        .unwrap_or_default();
    let label = (!from_name.is_empty()).then_some(from_name.as_str());
    let read: FolderSyncResult = read_sealed(store, &folder, cause::BUNDLE, label)?;
    if not_for_us && read.applied.is_empty() && read.unreadable > 0 {
        return Err(Error::Offer(OfferError::NotForUs));
    }
    for (device, manifest) in &writers {
        let vector: HashMap<String, i64> = manifest.vector.clone().into_iter().collect();
        store.note_held_by(device, &vector)?;
    }
    Ok(BundleResult {
        applied: read.applied,
        blobs_in: read.blobs_in,
        unreadable: read.unreadable,
        moment: read.moment,
        report: read.report,
        conflicts: read.conflicts,
        from,
        from_name,
        profile_changed: read.profile_changed,
        offer: None,
    })
}

/// A bundle being written, as the folder's writer sees it: every file
/// goes into the zip as it is written; nothing to read.
struct ZipWriteFolder {
    writer: Option<ZipWriter<File>>,
    written: BTreeMap<String, BTreeSet<String>>,
}

impl SyncFolder for ZipWriteFolder {
    fn list(&self, dir: &str) -> Result<Vec<String>> {
        Ok(self
            .written
            .get(dir)
            .map(|names| names.iter().cloned().collect())
            .unwrap_or_default())
    }

    fn read(&self, _dir: &str, _name: &str) -> Result<Option<Vec<u8>>> {
        Ok(None)
    }

    fn write(&mut self, dir: &str, name: &str, bytes: &[u8]) -> Result<()> {
        if !self
            .written
            .entry(dir.to_string())
            .or_default()
            .insert(name.to_string())
        {
            return Err(Error::Invalid(format!(
                "written twice into a bundle: {name}"
            )));
        }
        let writer = self
            .writer
            .as_mut()
            .ok_or_else(|| Error::Invalid("the bundle is closed".into()))?;
        let options = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Stored)
            .last_modified_time(zip::DateTime::default())
            .unix_permissions(0o644);
        writer
            .start_file(path_of(dir, name), options)
            .map_err(zip_error)?;
        writer.write_all(bytes)?;
        Ok(())
    }

    fn delete(&mut self, _dir: &str, _name: &str) -> Result<()> {
        Ok(())
    }

    fn ensure(&mut self, _dir: &str) -> Result<()> {
        Ok(())
    }
}

/// A bundle being read, as the folder's reader sees it: the zip's files
/// directly in the root, `keys` and `blobs`; anything else in it is not
/// looked at. Each file is unpacked when it is read.
struct ZipReadFolder {
    archive: RefCell<ZipArchive<File>>,
    dirs: BTreeMap<String, BTreeMap<String, String>>,
}

impl ZipReadFolder {
    fn new(mut archive: ZipArchive<File>) -> ZipReadFolder {
        let mut dirs: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
        for i in 0..archive.len() {
            let Ok(file) = archive.by_index_raw(i) else {
                continue;
            };
            if !file.is_file() {
                continue;
            }
            // A name that does not decode is none of ours.
            let Ok(path) = file.name().map(|name| name.into_owned()) else {
                continue;
            };
            let parts: Vec<&str> = path.split('/').collect();
            let (dir, name) = match parts.as_slice() {
                [name] => ("", *name),
                [dir @ ("keys" | "blobs"), name] => (*dir, *name),
                _ => continue,
            };
            if name.is_empty() {
                continue;
            }
            dirs.entry(dir.to_string())
                .or_default()
                .insert(name.to_string(), path.clone());
        }
        ZipReadFolder {
            archive: RefCell::new(archive),
            dirs,
        }
    }
}

impl SyncFolder for ZipReadFolder {
    fn list(&self, dir: &str) -> Result<Vec<String>> {
        Ok(self
            .dirs
            .get(dir)
            .map(|names| names.keys().cloned().collect())
            .unwrap_or_default())
    }

    fn read(&self, dir: &str, name: &str) -> Result<Option<Vec<u8>>> {
        let Some(path) = self.dirs.get(dir).and_then(|names| names.get(name)) else {
            return Ok(None);
        };
        let cap = if dir == "blobs" {
            MAX_BLOB_BYTES
        } else {
            MAX_FILE_BYTES
        };
        let mut archive = self.archive.borrow_mut();
        // A damaged file inside, or one not stored, reads as absent.
        let Ok(file) = archive.by_name(path) else {
            return Ok(None);
        };
        if file.size() > cap {
            return Ok(None);
        }
        let mut bytes = Vec::new();
        match file.take(cap + 1).read_to_end(&mut bytes) {
            Ok(_) if bytes.len() as u64 <= cap => Ok(Some(bytes)),
            _ => Ok(None),
        }
    }

    fn write(&mut self, _dir: &str, name: &str, _bytes: &[u8]) -> Result<()> {
        Err(Error::Invalid(format!("a bundle is read only: {name}")))
    }

    fn delete(&mut self, _dir: &str, name: &str) -> Result<()> {
        Err(Error::Invalid(format!("a bundle is read only: {name}")))
    }

    fn ensure(&mut self, _dir: &str) -> Result<()> {
        Ok(())
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

    #[test]
    fn a_bundle_carries_everything_sealed_and_strangers_read_nothing() -> TestResult {
        let phone = device(1)?;
        let desk = device(2)?;
        meet(&phone, &desk)?;
        let doctor = Places::new(&desk).add_area("Doctor")?;
        Notes::new(&desk).capture_with(
            "Blood test",
            Some(&doctor.id),
            None,
            &[b"xray".to_vec()],
            &[],
        )?;
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("all.obfsync");
        let (entries, blobs) = write_bundle_now(&desk, &path, None)?.ok_or("a bundle")?;
        assert_eq!(entries, desk.all_entries()?.len());
        assert_eq!(blobs, 1);
        let raw = String::from_utf8_lossy(&std::fs::read(&path)?).into_owned();
        for secret in ["Blood", "Doctor", "note:", "area:"] {
            assert!(!raw.contains(secret), "{secret}");
        }
        let got = import_bundle(&phone, &path)?;
        assert!(!got.applied.is_empty());
        assert_eq!(got.blobs_in, 1);
        assert_eq!(got.from, desk.device_id()?);
        assert_eq!(got.moment.map(|m| m.cause).as_deref(), Some(cause::BUNDLE));
        assert_eq!(texts(&phone)?, texts(&desk)?);
        assert!(import_bundle(&phone, &path)?.applied.is_empty());

        let stranger = device(3)?;
        let seen = import_bundle(&stranger, &path)?;
        assert!(seen.needs_pairing());
        assert!(texts(&stranger)?.is_empty());
        Ok(())
    }

    #[test]
    fn a_bundle_for_a_paired_device_carries_what_it_lacks() -> TestResult {
        let phone = device(1)?;
        let desk = device(2)?;
        Notes::new(&desk).capture("Old", None, None)?;
        meet(&phone, &desk)?;
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("delta.obfsync");
        let phone_id = phone.device_id()?;
        assert!(write_bundle_now(&desk, &path, Some(&phone_id))?.is_none());
        assert!(!path.exists());
        let before = desk.all_entries()?.len();
        Notes::new(&desk).capture("New", None, None)?;
        let (entries, _) = write_bundle_now(&desk, &path, Some(&phone_id))?.ok_or("a bundle")?;
        assert_eq!(entries, desk.all_entries()?.len() - before);
        import_bundle(&phone, &path)?;
        assert_eq!(texts(&phone)?, vec!["New", "Old"]);
        assert_eq!(phone.held_by(&desk.device_id()?)?, desk.version_vector()?);
        phone.forget_device(&desk.device_id()?)?;
        assert!(phone.held_by(&desk.device_id()?)?.is_empty());
        Ok(())
    }

    #[test]
    fn a_bundle_leaves_the_shared_folder_to_read_in_full() -> TestResult {
        use crate::folder::{MemoryFolder, folder_sync};
        let phone = device(1)?;
        let desk = device(2)?;
        meet(&phone, &desk)?;
        Notes::new(&desk).capture("First", None, None)?;
        let mut folder = MemoryFolder::default();
        folder_sync(&desk, &mut folder, &mut random_nonce)?;
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("b.obfsync");
        write_bundle_now(&desk, &path, None)?;
        import_bundle(&phone, &path)?;
        assert!(phone.local_settings_with_prefix("folderRead:")?.is_empty());
        Notes::new(&desk).capture("Second", None, None)?;
        folder_sync(&desk, &mut folder, &mut random_nonce)?;
        folder_sync(&phone, &mut folder, &mut random_nonce)?;
        assert_eq!(texts(&phone)?, texts(&desk)?);
        Ok(())
    }

    #[test]
    fn a_file_that_is_no_bundle_says_so() -> TestResult {
        let phone = device(1)?;
        let dir = tempfile::tempdir()?;
        let junk = dir.path().join("junk.obfsync");
        std::fs::write(&junk, b"not a zip at all")?;
        assert!(matches!(
            import_bundle(&phone, &junk),
            Err(Error::NotABundle)
        ));
        assert!(matches!(
            import_bundle(&phone, &dir.path().join("missing")),
            Err(Error::NotABundle)
        ));
        let zip = dir.path().join("other.zip");
        let mut writer = ZipWriter::new(File::create(&zip)?);
        writer.start_file("photo.jpg", SimpleFileOptions::default())?;
        writer.write_all(b"x")?;
        writer.finish()?;
        assert!(matches!(
            import_bundle(&phone, &zip),
            Err(Error::NotABundle)
        ));
        Ok(())
    }

    #[test]
    fn a_bundle_of_only_newer_formats_needs_an_update() -> TestResult {
        let phone = device(1)?;
        let dir = tempfile::tempdir()?;
        let path = dir.path().join("later.obfsync");
        let mut writer = ZipWriter::new(File::create(&path)?);
        writer.start_file("abc.manifest", SimpleFileOptions::default())?;
        writer.write_all(br#"{"format":2,"later":true}"#)?;
        writer.finish()?;
        assert!(matches!(
            import_bundle(&phone, &path),
            Err(Error::BundleNeedsUpdate)
        ));
        Ok(())
    }

    #[test]
    fn the_file_name_tells_the_day() {
        let day = chrono::NaiveDate::from_ymd_opt(2026, 3, 1);
        assert_eq!(
            day.map(file_name).as_deref(),
            Some("oldbrainfix-2026-03-01.obfsync")
        );
    }
}
