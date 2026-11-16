use super::*;

#[test]
fn only_markdown_and_text_files_not_hidden_are_read() {
    assert!(may_become_note("Dentist.txt"));
    assert!(may_become_note("Shopping.MD"));
    assert!(may_become_note("a.b.Markdown"));
    assert!(!may_become_note(".hidden.md"));
    assert!(!may_become_note("photo.png"));
    assert!(!may_become_note("README"));
    assert!(!may_become_note("notes.md.bak"));
}

#[test]
fn a_folder_is_named_by_its_last_part() {
    assert_eq!(folder_name(Path::new("/home/ann/Notes")), "Notes");
    assert_eq!(folder_name(Path::new("/")), "/");
}
