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
