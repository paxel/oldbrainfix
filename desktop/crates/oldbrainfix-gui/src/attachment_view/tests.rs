use super::*;

#[test]
fn a_file_says_its_size_and_its_pages() {
    let t = L10n::new("en");
    let mut file = oldbrainfix_core::Attachment::read(
        "abc",
        r#"{"name":"Lease.pdf","pages":3,"size":2500000,"type":"application/pdf"}"#,
    );
    assert_eq!(facts(&t, &file), "2.5 MB · 3 pages");
    assert_eq!(title(&t, &file), "Lease.pdf");
    assert_eq!(icon(file.kind()), icons::FILE_PDF);
    file.pages = None;
    file.name = String::new();
    assert_eq!(facts(&t, &file), "2.5 MB");
    assert_eq!(title(&t, &file), "Attachment");
}

#[test]
fn a_file_too_large_is_left_out_unread() -> std::io::Result<()> {
    let dir = tempfile::tempdir()?;
    let small = dir.path().join("a.txt");
    std::fs::write(&small, b"milk")?;
    let large = dir.path().join("b.bin");
    std::fs::File::create(&large)?.set_len(MAX_FILE_BYTES + 1)?;
    let gone = dir.path().join("c.txt");
    let read = read_paths([
        ("a.txt".to_string(), small.as_path()),
        ("b.bin".to_string(), large.as_path()),
        ("c.txt".to_string(), gone.as_path()),
    ]);
    assert_eq!(read.files, [("a.txt".to_string(), b"milk".to_vec())]);
    assert!(read.too_large);
    assert_eq!(read.refused.len(), 2);
    assert!(read.refused[0].starts_with("b.bin: "));
    Ok(())
}

#[test]
fn only_files_that_only_show_open_without_a_question() {
    for safe in [
        "Lease.pdf",
        "SCAN.PDF",
        "a.b.docx",
        "notes.md",
        "song.mp3",
        "photos.zip",
    ] {
        assert!(opens_safely(safe), "{safe}");
    }
    for risky in [
        "setup.exe",
        "SETUP.EXE",
        "Lease.pdf.exe",
        "run.bat",
        "run.cmd",
        "a.ps1",
        "a.vbs",
        "a.js",
        "a.msi",
        "a.lnk",
        "a.scr",
        "a.jar",
        "a.sh",
        "a.desktop",
        "a.app",
        "a.command",
        "a.AppImage",
        "a.apk",
        "page.html",
        "drawing.svg",
        "macro.docm",
        "a.exe.",
        "a.exe ",
        "a.pdf ",
        "a.txt:b.exe",
        "README",
        "",
        ".pdf.",
    ] {
        assert!(!opens_safely(risky), "{risky:?}");
    }
}

#[test]
fn a_copy_is_named_by_its_hash_and_a_clean_ending_alone() {
    let hash = "0123456789abcdef0123456789abcdef";
    assert_eq!(copy_name(hash, "Lease.PDF"), "0123456789abcdef.pdf");
    assert_eq!(
        copy_name(hash, "x\" & calc & \".exe"),
        "0123456789abcdef.exe"
    );
    assert_eq!(copy_name(hash, "a.exe "), "0123456789abcdef");
    assert_eq!(copy_name(hash, "a.p d f"), "0123456789abcdef");
    assert_eq!(copy_name(hash, "a.toolongending"), "0123456789abcdef");
    // A hash that is no hash leaves nothing of itself but hex digits.
    assert_eq!(copy_name("../../x", "a.txt"), "file.txt");
    assert_eq!(copy_name("ab/../cd", "a"), "abcd");
}

#[test]
fn a_pdf_held_counts_its_pages_and_other_files_do_not() {
    let held = HeldFile::new("notes.txt".into(), b"milk".to_vec());
    assert_eq!(held.pages, None);
    assert_eq!(held.new_file().name, "notes.txt");
    let broken = HeldFile::new("Scan.PDF".into(), b"no pdf".to_vec());
    assert_eq!(broken.pages, None);
}
