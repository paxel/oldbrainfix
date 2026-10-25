//! The formats of user text (ADR 0018), as the Dart core's
//! `text_format.dart` reads and writes them: the text of a Note or a
//! Section is stored with the format it is in. This version knows only
//! [`PLAIN`]; formatted text comes later (#57).

use serde_json::{Map, Value};

/// Text as typed, without formatting.
pub const PLAIN: &str = "plain";

/// Every format this version reads and writes.
pub const KNOWN: [&str; 1] = [PLAIN];

/// A user text as stored: its format and its text, the source text for a
/// format this version does not know.
///
/// It is stored as one value, `{"format":…,"text":…}`, keys sorted and no
/// spaces, the same bytes from both cores, so text and format travel,
/// conflict and win together in one entry. A value that is no such
/// object was written before the format was recorded and reads as plain,
/// whole.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredText {
    pub format: String,
    pub text: String,
    /// Whether this version may change it: its format is known and the
    /// value holds nothing else.
    pub editable: bool,
}

impl StoredText {
    /// `text` as plain text, the only format this version writes.
    pub fn plain(text: &str) -> StoredText {
        StoredText {
            format: PLAIN.to_string(),
            text: text.to_string(),
            editable: true,
        }
    }

    /// `text` in `format` as it arrived from elsewhere, such as a Sent
    /// Item, to be stored as it is.
    pub fn of(format: &str, text: &str) -> StoredText {
        StoredText {
            format: format.to_string(),
            text: text.to_string(),
            editable: KNOWN.contains(&format),
        }
    }

    /// The text a stored `value` holds; `None` reads as empty plain text.
    pub fn read(value: Option<&str>) -> StoredText {
        let Some(value) = value else {
            return StoredText::plain("");
        };
        if value.starts_with('{')
            && let Ok(Value::Object(json)) = serde_json::from_str::<Value>(value)
            && let (Some(Value::String(format)), Some(Value::String(text))) =
                (json.get("format"), json.get("text"))
        {
            return StoredText {
                format: format.clone(),
                text: text.clone(),
                editable: KNOWN.contains(&format.as_str()) && json.len() == 2,
            };
        }
        StoredText::plain(value)
    }

    /// The value it is stored as.
    pub fn encode(&self) -> String {
        let mut map = Map::new();
        map.insert("format".into(), Value::String(self.format.clone()));
        map.insert("text".into(), Value::String(self.text.clone()));
        Value::Object(map).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FUTURE: &str = r##"{"format":"x-future","text":"# Heading\n**bold**"}"##;

    #[test]
    fn plain_text_is_written_with_its_format_keys_sorted() {
        assert_eq!(
            StoredText::plain("Quote \" and \\ and \n").encode(),
            r#"{"format":"plain","text":"Quote \" and \\ and \n"}"#
        );
        assert_eq!(
            StoredText::read(Some(r#"{"format":"plain","text":"milk"}"#)),
            StoredText::plain("milk")
        );
    }

    #[test]
    fn a_value_written_before_the_format_was_recorded_reads_as_plain() {
        for value in [
            "milk",
            "",
            "{not json",
            r#"{"format":"plain"}"#,
            r#"{"format":3,"text":"x"}"#,
            r#"["format","text"]"#,
            r#" {"format":"x-future","text":"x"}"#,
        ] {
            assert_eq!(StoredText::read(Some(value)), StoredText::plain(value));
        }
        assert_eq!(StoredText::read(None), StoredText::plain(""));
    }

    #[test]
    fn an_unknown_format_reads_as_its_source_text_not_editable() {
        let text = StoredText::read(Some(FUTURE));
        assert_eq!(text.format, "x-future");
        assert_eq!(text.text, "# Heading\n**bold**");
        assert!(!text.editable);
        let more = StoredText::read(Some(r#"{"format":"plain","more":1,"text":"x"}"#));
        assert_eq!((more.format.as_str(), more.text.as_str()), (PLAIN, "x"));
        assert!(!more.editable);
    }
}
