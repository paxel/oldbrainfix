use super::*;

fn parse_of(args: &[&str]) -> Result<Launch, Usage> {
    let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
    parse(&args)
}

#[test]
fn a_plain_start_opens_the_window_and_a_login_start_the_tray() {
    assert_eq!(parse_of(&[]), Ok(Launch::Window));
    assert_eq!(parse_of(&["--tray"]), Ok(Launch::Tray));
    assert_eq!(parse_of(&["--help"]), Ok(Launch::Help));
    assert_eq!(parse_of(&["--tray", "x"]), Err(Usage));
    assert_eq!(parse_of(&["captur"]), Err(Usage));
}

#[test]
fn version_is_asked_by_either_spelling_and_alone() {
    assert_eq!(parse_of(&["--version"]), Ok(Launch::Version));
    assert_eq!(parse_of(&["-V"]), Ok(Launch::Version));
    assert_eq!(parse_of(&["--version", "x"]), Err(Usage));
    assert_eq!(parse_of(&["-V", "--help"]), Err(Usage));
    assert_eq!(parse_of(&["--tray", "--version"]), Err(Usage));
    assert_eq!(parse_of(&["-v"]), Err(Usage));
    assert_eq!(parse_of(&["version"]), Err(Usage));
    assert_eq!(parse_of(&["capture", "--version"]), Err(Usage));
    assert_eq!(
        version_line(),
        format!("Old Brain Fix {}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn capture_opens_the_popup_or_takes_the_text_given() {
    assert_eq!(parse_of(&["capture"]), Ok(Launch::Popup));
    assert_eq!(
        parse_of(&["capture", "buy", "milk"]),
        Ok(Launch::Text("buy milk".into()))
    );
    assert_eq!(
        parse_of(&["capture", "call Anna, 3 pm"]),
        Ok(Launch::Text("call Anna, 3 pm".into()))
    );
    assert_eq!(parse_of(&["capture", "-"]), Ok(Launch::Stdin));
    assert_eq!(parse_of(&["capture", "--help"]), Ok(Launch::Help));
    // `--` lets a text start with a dash.
    assert_eq!(
        parse_of(&["capture", "--", "-", "5 kg"]),
        Ok(Launch::Text("- 5 kg".into()))
    );
    assert_eq!(parse_of(&["capture", "--"]), Err(Usage));
    assert_eq!(parse_of(&["capture", "-x", "milk"]), Err(Usage));
    assert_eq!(parse_of(&["capture", "-", "milk"]), Err(Usage));
}

#[test]
fn import_takes_one_file() {
    assert_eq!(
        parse_of(&["import", "a.obfsync"]),
        Ok(Launch::Import("a.obfsync".into()))
    );
    assert_eq!(parse_of(&["import", "--help"]), Ok(Launch::Help));
    assert_eq!(parse_of(&["import"]), Err(Usage));
    assert_eq!(parse_of(&["import", "a", "b"]), Err(Usage));
}

#[test]
fn open_takes_one_file() {
    assert_eq!(
        parse_of(&["open", "a.oldbrain"]),
        Ok(Launch::Open("a.oldbrain".into()))
    );
    assert_eq!(parse_of(&["open", "--help"]), Ok(Launch::Help));
    assert_eq!(parse_of(&["open"]), Err(Usage));
    assert_eq!(parse_of(&["open", "a", "b"]), Err(Usage));
}

#[test]
fn blank_text_is_nothing_to_capture() {
    assert_eq!(note_text("  buy milk\n"), Some("buy milk"));
    assert_eq!(
        note_text("line one\nline two\n"),
        Some("line one\nline two")
    );
    assert_eq!(note_text(" \n\t"), None);
}
