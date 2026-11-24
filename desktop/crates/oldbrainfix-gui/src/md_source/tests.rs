// A test may unwrap, expect, panic and index (#229).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use super::*;

fn write(path: &Path, text: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn png() -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(4, 3, image::Rgba([20, 120, 200, 255]));
    let mut out = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut out, image::ImageFormat::Png)
        .unwrap();
    out.into_inner()
}

#[test]
fn links_are_found_in_the_nearest_folder_holding_obsidian_else_the_files_own() {
    let dir = tempfile::tempdir().unwrap();
    let vault = dir.path().join("vault");
    std::fs::create_dir_all(vault.join(".obsidian")).unwrap();
    let note = vault.join("deep/er/Note.md");
    write(&note, b"x");
    assert_eq!(link_root(&note), vault.canonicalize().unwrap());
    let loose = dir.path().join("loose/Alone.md");
    write(&loose, b"x");
    assert_eq!(
        link_root(&loose),
        dir.path().join("loose").canonicalize().unwrap()
    );
}

#[test]
fn the_index_leaves_hidden_files_and_folders_out_shallowest_first() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(&root.join("b/Deep.md"), b"x");
    write(&root.join("Top.md"), b"x");
    write(&root.join(".hidden.md"), b"x");
    write(&root.join(".obsidian/app.json"), b"{}");
    write(&root.join(".trash/Old.md"), b"x");
    assert_eq!(index(root), ["Top.md", "b/Deep.md"]);
}

#[cfg(unix)]
#[test]
fn nothing_outside_the_folder_is_read_through_a_link() {
    let dir = tempfile::tempdir().unwrap();
    let outside = dir.path().join("secret");
    write(&outside.join("Secret.md"), b"secret");
    let root = dir.path().join("vault");
    write(&root.join("Note.md"), b"[[Secret]] [[../secret/Secret]]");
    std::os::unix::fs::symlink(outside.join("Secret.md"), root.join("Secret.md")).unwrap();
    std::os::unix::fs::symlink(&outside, root.join("linked")).unwrap();
    let root = root.canonicalize().unwrap();
    assert_eq!(index(&root), ["Note.md"]);
    assert_eq!(inside(&root, "Secret.md"), None);
    assert_eq!(inside(&root, "linked/Secret.md"), None);
    assert_eq!(inside(&root, "../secret/Secret.md"), None);
    let read = read_file(&root.join("Note.md"), None);
    assert!(read.notes.values().all(Option::is_none));
    assert!(read.linked.is_empty());
}

#[test]
fn a_file_is_read_with_the_files_it_links_to_and_its_pictures() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join(".obsidian")).unwrap();
    write(
        &root.join("notes/Garden.md"),
        b"# Garden\n[[Roses]] [[Lost]] [[Garden#Top]] [tulips](../Tulips.md)\n\n![[rose.png]] ![](missing.png)",
    );
    write(&root.join("plants/Roses.md"), b"Roses");
    write(&root.join("Tulips.md"), b"Tulips");
    write(&root.join("pics/rose.png"), &png());
    let read = read_file(&root.join("notes/Garden.md"), None);
    let root = root.canonicalize().unwrap();
    assert_eq!(read.root, root);
    assert_eq!(read.name, "Garden.md");
    assert!(read.doc.is_ok());
    assert_eq!(
        read.notes.get("Roses"),
        Some(&Some(root.join("plants/Roses.md")))
    );
    assert_eq!(read.notes.get("Lost"), Some(&None));
    assert_eq!(
        read.notes.get("../Tulips.md"),
        Some(&Some(root.join("Tulips.md")))
    );
    // Itself is no file it links to.
    assert_eq!(
        read.linked,
        [root.join("plants/Roses.md"), root.join("Tulips.md")]
    );
    let shown: Vec<(&str, bool)> = read
        .pictures
        .iter()
        .map(|(p, s)| (p.target.as_str(), s.is_some()))
        .collect();
    assert!(shown.contains(&("rose.png", true)));
    assert!(shown.contains(&("missing.png", false)));
}

#[test]
fn a_file_too_large_or_gone_says_why() {
    let dir = tempfile::tempdir().unwrap();
    let read = read_file(&dir.path().join("Gone.md"), None);
    assert!(read.doc.is_err());
}

#[test]
fn text_of_drops_the_byte_order_mark_and_reads_any_byte() {
    assert_eq!(text_of(b"\xef\xbb\xbfa\r\nb"), "a\nb");
    assert_eq!(text_of(b"caf\xe9"), "café");
}

#[test]
fn the_watch_reads_the_file_again_once_it_changes() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("Note.md");
    write(&file, b"# One");
    let watch = Watch::start(file.clone(), Arc::new(|| {}));
    let first = wait(&watch);
    let heading = |read: &Read| match &read.doc.as_ref().unwrap().blocks[0] {
        Block::Heading { text, .. } => text.text.clone(),
        other => panic!("no heading: {other:?}"),
    };
    assert_eq!(heading(&first), "One");
    // Another length, so the change shows on any file system's clock.
    write(&file, b"# Two, longer");
    assert_eq!(heading(&wait(&watch)), "Two, longer");
    let other = dir.path().join("Other.md");
    write(&other, b"# Other");
    watch.show(other);
    assert_eq!(heading(&wait(&watch)), "Other");
}

/// The next reading of `watch`, within five seconds.
fn wait(watch: &Watch) -> Read {
    for _ in 0..100 {
        if let Some(read) = watch.arrived() {
            return read;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("nothing read");
}

#[test]
fn a_link_back_from_a_folder_further_in_finds_its_file_without_obsidian() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    write(&root.join("Garden.md"), b"[[Roses]]");
    write(&root.join("plants/Roses.md"), b"Back to the [[Garden]].");
    let watch = Watch::start(root.join("Garden.md"), Arc::new(|| {}));
    let garden = wait(&watch);
    assert_eq!(garden.root, root);
    let roses = garden.notes.get("Roses").cloned().flatten().unwrap();
    watch.show(roses);
    let roses = wait(&watch);
    assert_eq!(roses.root, root);
    assert_eq!(
        roses.notes.get("Garden"),
        Some(&Some(root.join("Garden.md")))
    );
    // Read alone, its own folder holds no Garden.
    let alone = read_file(&root.join("plants/Roses.md"), None);
    assert_eq!(alone.notes.get("Garden"), Some(&None));
}
