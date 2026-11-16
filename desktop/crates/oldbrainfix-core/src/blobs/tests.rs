use super::*;

const HASH: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

#[test]
fn only_a_sha256_names_a_blob() {
    assert!(is_hash(HASH));
    assert!(!is_hash(&HASH.to_uppercase()));
    assert!(!is_hash("../../etc/passwd"));
    assert!(!is_hash(&HASH[1..]));
}

#[test]
fn a_file_blob_is_kept_once_and_never_outside_its_directory() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let blobs = FileBlobs::new(tmp.path().join("photos"))?;
    blobs.put(HASH, b"first")?;
    blobs.put(HASH, b"second")?;
    assert_eq!(blobs.get(HASH).as_deref(), Some(&b"first"[..]));
    assert_eq!(
        std::fs::read(tmp.path().join("photos").join(HASH))?,
        b"first"
    );
    blobs.put("../escaped", b"x")?;
    assert!(!tmp.path().join("escaped").exists());
    assert_eq!(blobs.get("../escaped"), None);
    // Nor does a removal: a hash from another Device's log decides it.
    std::fs::write(tmp.path().join("kept"), b"x")?;
    assert!(!blobs.remove("../kept")?);
    assert!(tmp.path().join("kept").exists());
    assert_eq!(blobs.get(&"0".repeat(64)), None);
    let names: Vec<_> = std::fs::read_dir(tmp.path().join("photos"))?
        .map(|e| e.map(|e| e.file_name()))
        .collect::<std::io::Result<_>>()?;
    assert_eq!(names, vec![std::ffi::OsString::from(HASH)]);
    assert!(blobs.has(HASH) && !blobs.has("../kept"));
    assert!(blobs.remove(HASH)?);
    assert!(!blobs.remove(HASH)?);
    assert_eq!(blobs.get(HASH), None);
    assert!(!blobs.has(HASH));
    Ok(())
}

#[test]
fn a_memory_blob_keeps_the_first_bytes() -> Result<()> {
    let blobs = MemoryBlobs::default();
    blobs.put(HASH, b"first")?;
    blobs.put(HASH, b"second")?;
    assert_eq!(blobs.get(HASH).as_deref(), Some(&b"first"[..]));
    assert_eq!(blobs.get("other"), None);
    assert!(blobs.has(HASH) && !blobs.has("other"));
    assert!(blobs.remove(HASH)?);
    assert_eq!(blobs.get(HASH), None);
    Ok(())
}
