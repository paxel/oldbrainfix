//! The import from Google Keep (ADR 0036, Google Keep), as the phone's
//! `keep_import.dart`: the files of a Google Takeout export, its zip as
//! it is or the folder unpacked from it, read off the window's thread
//! and made into Notes by the core. Only the files the core reads
//! ([`keep_reads`]) are read, and only those of a folder named `Keep`
//! when any lie in one, so no photo of another Google product is opened;
//! none is read beyond its limit. A file that cannot be read fails the
//! Import, and then nothing is written.

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use oldbrainfix_core::imports::{ImportFile, keep_reads};
use zip::ZipArchive;

/// The Takeout export picked: its zip, or the folder unpacked from it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeepSource {
    Zip(PathBuf),
    Folder(PathBuf),
}

/// The files of `source` the core reads, by their paths inside the zip
/// or the folder, with how many notes were refused for a path leaving
/// the export (`..` or a path from the root), which a zip may name but
/// Takeout never does; a photo so named is not handed over, and counts
/// as missing should a note name it. The full text of why when it cannot
/// be read.
pub fn read(source: &KeepSource) -> Result<(Vec<ImportFile>, usize), String> {
    match source {
        KeepSource::Zip(zip) => read_zip(zip).map_err(|e| format!("{}: {e}", zip.display())),
        KeepSource::Folder(folder) => read_folder(folder)
            .map(|files| (files, 0))
            .map_err(|e| format!("{}: {e}", folder.display())),
    }
}

/// The entries of the zip at `path` the core reads.
fn read_zip(path: &Path) -> Result<(Vec<ImportFile>, usize), Box<dyn std::error::Error>> {
    let mut archive = ZipArchive::new(BufReader::new(File::open(path)?))?;
    // An entry whose name cannot be read is no file of Keep's.
    let names = (0..archive.len())
        .filter_map(|index| {
            let name = archive.name_for_index(index)?.ok()?;
            Some((name.into_owned(), index))
        })
        .collect();
    let mut files = Vec::new();
    let mut refused = 0;
    for (name, index, limit) in of_keep(names) {
        if !inside(&name) {
            if is_json(&name) {
                refused += 1;
            }
            continue;
        }
        let mut entry = archive.by_index(index)?;
        let size = entry.size();
        let bytes = read_up_to(&mut entry, size, limit)?;
        files.push(ImportFile {
            name,
            modified: None,
            bytes,
        });
    }
    Ok((files, refused))
}

/// The files of `folder` and of its folders, but not of hidden ones, the
/// core reads, by their paths inside the folder, folders separated by
/// `/`. A link to a folder is not followed.
fn read_folder(folder: &Path) -> std::io::Result<Vec<ImportFile>> {
    let mut paths = Vec::new();
    list_into(folder, "", &mut paths)?;
    let mut files = Vec::new();
    for (name, path, limit) in of_keep(paths) {
        let size = std::fs::metadata(&path)?.len();
        let bytes = read_up_to(&mut File::open(&path)?, size, limit)?;
        files.push(ImportFile {
            name,
            modified: None,
            bytes,
        });
    }
    Ok(files)
}

fn list_into(dir: &Path, prefix: &str, paths: &mut Vec<(String, PathBuf)>) -> std::io::Result<()> {
    let mut entries = std::fs::read_dir(dir)?.collect::<std::io::Result<Vec<_>>>()?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = format!("{prefix}{name}");
        if entry.file_type()?.is_dir() {
            if !name.starts_with('.') {
                list_into(&entry.path(), &format!("{path}/"), paths)?;
            }
        } else if keep_reads(&path).is_some()
            && !std::fs::metadata(entry.path()).is_ok_and(|m| m.is_dir())
        {
            paths.push((path, entry.path()));
        }
    }
    Ok(())
}

/// Of `entries` by their paths, those the core reads, each with how many
/// bytes it reads of it; when any lies in a folder named `Keep`, as in a
/// Takeout of several Google products, only those, as the core counts
/// them.
fn of_keep<T>(entries: Vec<(String, T)>) -> Vec<(String, T, usize)> {
    let taken: Vec<(String, T, usize)> = entries
        .into_iter()
        .filter_map(|(path, item)| keep_reads(&path).map(|limit| (path, item, limit)))
        .collect();
    if taken.iter().any(|(path, ..)| in_keep_folder(path)) {
        taken
            .into_iter()
            .filter(|(path, ..)| in_keep_folder(path))
            .collect()
    } else {
        taken
    }
}

/// Whether the file at `path` lies in a folder named `Keep`, in any case
/// of ASCII letters, as the core takes it.
fn in_keep_folder(path: &str) -> bool {
    let folders: Vec<&str> = path.split(['/', '\\']).collect();
    folders
        .iter()
        .rev()
        .nth(1)
        .is_some_and(|folder| folder.eq_ignore_ascii_case("keep"))
}

/// Whether the entry `name` stays inside the export: no `..` among its
/// folders, not from the root, not on a drive.
fn inside(name: &str) -> bool {
    let bytes = name.as_bytes();
    let drive = matches!(bytes, [letter, b':', ..] if letter.is_ascii_alphabetic());
    !name.starts_with(['/', '\\'])
        && !drive
        && !name.contains('\0')
        && !name.split(['/', '\\']).any(|part| part == "..")
}

/// Whether the entry `name` is a note's `.json`.
fn is_json(name: &str) -> bool {
    name.rsplit_once('.')
        .is_some_and(|(_, ending)| ending.eq_ignore_ascii_case("json"))
}

/// The bytes of `reader`, `size` long as it says, when no more than
/// `limit`; else `limit` and one more byte, unread, so the core counts it
/// as too large: only its length counts. One holding more than it says
/// is read no further than that.
fn read_up_to(reader: &mut impl Read, size: u64, limit: usize) -> std::io::Result<Vec<u8>> {
    let over = limit + 1;
    if size >= over as u64 {
        return Ok(vec![0; over]);
    }
    let mut bytes = Vec::with_capacity(size as usize);
    reader.take(over as u64).read_to_end(&mut bytes)?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_entries_in_keep_count_when_any_lie_there() {
        let entries = vec![
            ("Takeout/Keep/Note.json".to_string(), ()),
            ("Takeout/keep/Photo.PNG".to_string(), ()),
            ("Takeout/Keep/Note.html".to_string(), ()),
            ("Takeout/Keep/Labels.txt".to_string(), ()),
            ("Takeout/Photos/Trip.jpg".to_string(), ()),
            ("Takeout/archive_browser.json".to_string(), ()),
        ];
        let taken: Vec<String> = of_keep(entries).into_iter().map(|(p, ..)| p).collect();
        assert_eq!(taken, ["Takeout/Keep/Note.json", "Takeout/keep/Photo.PNG"]);
        let elsewhere = vec![("Note.json".to_string(), ()), ("a/b.gif".to_string(), ())];
        assert_eq!(of_keep(elsewhere).len(), 2);
    }

    #[test]
    fn names_leaving_the_export_are_refused() {
        assert!(inside("Takeout/Keep/Note.json"));
        assert!(inside("Takeout/Keep/a..b.json"));
        assert!(!inside("../Keep/Note.json"));
        assert!(!inside("Takeout/../../Keep/Note.json"));
        assert!(!inside("Takeout\\..\\Keep\\Note.json"));
        assert!(!inside("/etc/Keep/Note.json"));
        assert!(!inside("\\Keep\\Note.json"));
        assert!(!inside("C:Keep/Note.json"));
    }

    #[test]
    fn a_file_over_its_limit_is_handed_over_unread_one_byte_longer() -> std::io::Result<()> {
        assert_eq!(read_up_to(&mut &b"abc"[..], 3, 3)?, b"abc");
        assert_eq!(read_up_to(&mut &b"abcd"[..], 4, 3)?, [0; 4]);
        // One that says less than it holds is read no further.
        assert_eq!(read_up_to(&mut &b"abcdef"[..], 2, 3)?, b"abcd");
        Ok(())
    }
}
