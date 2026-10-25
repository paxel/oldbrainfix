//! What the command line asks for:
//!
//! - `oldbrainfix` opens the window,
//! - `oldbrainfix --tray` is the login start, into the tray,
//! - `oldbrainfix capture` opens the capture popup,
//! - `oldbrainfix capture buy milk` captures "buy milk" without a window,
//! - `oldbrainfix capture -` captures what comes in on stdin,
//! - `oldbrainfix import FILE` imports the sync bundle FILE.
//!
//! A capture without a window prints nothing when it worked; an import
//! prints what came in. The exit codes are [`OK`], [`FAILED`] and
//! [`USAGE`].

use crate::autostart::TRAY_ARG;

/// It worked.
pub const OK: i32 = 0;
/// The Note could not be kept or the bundle imported: no data
/// directory, the store or the running app failed, or the file is no
/// bundle.
pub const FAILED: i32 = 1;
/// The arguments make no sense, or there was nothing to capture.
pub const USAGE: i32 = 2;

/// The subcommand that captures.
pub const CAPTURE: &str = "capture";

/// The subcommand that imports a bundle.
pub const IMPORT: &str = "import";

/// What a start asks for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Launch {
    /// The window.
    Window,
    /// The login start: the tray, no window.
    Tray,
    /// The capture popup.
    Popup,
    /// Captures this text, as typed, without a window.
    Text(String),
    /// Captures what comes in on stdin, without a window.
    Stdin,
    /// Imports the bundle at this path, without a window.
    Import(String),
    /// Prints the usage.
    Help,
}

/// Arguments that make no sense.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Usage;

/// Reads the arguments after the program's name.
pub fn parse(args: &[String]) -> Result<Launch, Usage> {
    let Some((first, rest)) = args.split_first() else {
        return Ok(Launch::Window);
    };
    match first.as_str() {
        TRAY_ARG if rest.is_empty() => Ok(Launch::Tray),
        "-h" | "--help" | "help" if rest.is_empty() => Ok(Launch::Help),
        CAPTURE => match rest {
            [] => Ok(Launch::Popup),
            [one] if one == "-" => Ok(Launch::Stdin),
            [one] if one == "-h" || one == "--help" => Ok(Launch::Help),
            // After `--` everything is text, even "-".
            [dashes, text @ ..] if dashes == "--" => words(text),
            [first, ..] if first.starts_with('-') => Err(Usage),
            text => words(text),
        },
        IMPORT => match rest {
            [one] if one == "-h" || one == "--help" => Ok(Launch::Help),
            [path] if !path.is_empty() => Ok(Launch::Import(path.clone())),
            _ => Err(Usage),
        },
        _ => Err(Usage),
    }
}

fn words(text: &[String]) -> Result<Launch, Usage> {
    if text.is_empty() {
        return Err(Usage);
    }
    Ok(Launch::Text(text.join(" ")))
}

/// The text to capture as it is kept: without the blanks around it;
/// `None` when nothing is left.
pub fn note_text(text: &str) -> Option<&str> {
    Some(text.trim()).filter(|t| !t.is_empty())
}

#[cfg(test)]
mod tests {
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
    fn blank_text_is_nothing_to_capture() {
        assert_eq!(note_text("  buy milk\n"), Some("buy milk"));
        assert_eq!(
            note_text("line one\nline two\n"),
            Some("line one\nline two")
        );
        assert_eq!(note_text(" \n\t"), None);
    }
}
