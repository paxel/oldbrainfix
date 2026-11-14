use super::*;

#[test]
fn a_file_link_finds_only_files_inside_the_folder() -> std::result::Result<(), String> {
    let base = std::env::temp_dir().join(format!("obf-links-{}", std::process::id()));
    let folder = base.join("Sync");
    std::fs::create_dir_all(folder.join("Scans")).map_err(|e| e.to_string())?;
    std::fs::write(folder.join("Scans/tax 2026.pdf"), b"x").map_err(|e| e.to_string())?;
    std::fs::write(base.join("secret.txt"), b"x").map_err(|e| e.to_string())?;
    let found = file_in(&folder, "Scans/tax 2026.pdf");
    let outside = file_in(&folder, "../secret.txt");
    let dir = file_in(&folder, "Scans");
    let missing = file_in(&folder, "nope.pdf");
    #[cfg(unix)]
    let linked = {
        std::os::unix::fs::symlink(base.join("secret.txt"), folder.join("ln.txt"))
            .map_err(|e| e.to_string())?;
        file_in(&folder, "ln.txt")
    };
    drop(std::fs::remove_dir_all(&base));
    assert!(found.is_some_and(|f| f.ends_with("Scans/tax 2026.pdf")));
    assert_eq!(outside, None);
    assert_eq!(dir, None);
    assert_eq!(missing, None);
    #[cfg(unix)]
    assert_eq!(linked, None, "a symbolic link out of the folder");
    Ok(())
}

#[test]
fn a_desktop_file_names_its_app() {
    let text = "[Desktop Entry]\nType=Application\nName[de]=Dokumente\nName=Document Viewer\n[Desktop Action new]\nName=New";
    assert_eq!(desktop_entry_name(text).as_deref(), Some("Document Viewer"));
    assert_eq!(desktop_entry_name("[Other]\nName=x"), None);
}
