use super::*;

#[test]
fn a_copy_holds_the_bytes_and_is_written_once() {
    let dir = tempfile::tempdir().unwrap();
    let path = copy_to_open(dir.path(), "photo-abc.jpg", b"one").unwrap();
    assert_eq!(path, dir.path().join("photo-abc.jpg"));
    assert_eq!(std::fs::read(&path).unwrap(), b"one");
    // The same name again leaves the file as it is.
    std::fs::write(&path, b"two").unwrap();
    copy_to_open(dir.path(), "photo-abc.jpg", b"one").unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"two");
    // One cut off is written again.
    std::fs::write(&path, b"o").unwrap();
    copy_to_open(dir.path(), "photo-abc.jpg", b"one").unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), b"one");
}

#[test]
fn a_missing_folder_is_made() {
    let dir = tempfile::tempdir().unwrap();
    let inner = dir.path().join("a/b");
    let path = copy_to_open(&inner, "photo-x.png", b"png").unwrap();
    assert_eq!(std::fs::read(path).unwrap(), b"png");
}
