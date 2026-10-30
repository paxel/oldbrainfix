//! The formats of user text (ADR 0018), as the Dart core's
//! `text_format.dart` reads and writes them: the text of a Note or a
//! Section is stored with the format it is in. This version knows
//! [`PLAIN`] and [`MARKDOWN`].

use serde_json::{Map, Value};

use crate::markdown;

/// Text as typed, without formatting.
pub const PLAIN: &str = "plain";

/// Formatted text, the subset of Markdown [`crate::markdown`] reads (ADR
/// 0033). A text stays plain until it is first formatted, and is
/// formatted from then on.
pub const MARKDOWN: &str = "markdown";

/// Every format this version reads and writes.
pub const KNOWN: [&str; 2] = [PLAIN, MARKDOWN];

/// The newest structure of the stored value this version knows: `v` 1
/// holds `format` and `text` and nothing else.
pub const VERSION: i64 = 1;

/// `text` in `format` as it reads without its marks, for search and
/// previews; a text in any other format as it is.
pub fn visible(format: &str, text: &str) -> String {
    if format == MARKDOWN {
        markdown::plain(text)
    } else {
        text.to_string()
    }
}

/// The plain text `text` as it is written into a text in `format` to
/// read as itself, such as a Transcript put into a Note's text.
pub fn literal(format: &str, text: &str) -> String {
    if format == MARKDOWN {
        markdown::escape(text)
    } else {
        text.to_string()
    }
}

/// A user text as stored: its format, its text, the source text for a
/// format this version does not know, and the version of the value's
/// structure.
///
/// It is stored as one value, `{"format":…,"text":…}`, keys sorted and no
/// spaces, the same bytes from both cores, so text and format travel,
/// conflict and win together in one entry. `v` is written only when it
/// is more than 1, its default; a value without it is version 1. A value that is no such object was written before the
/// format was recorded and reads as plain, whole.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredText {
    pub format: String,
    pub text: String,
    /// The version of the value's structure, `v`: which keys it has and
    /// what they mean.
    pub version: i64,
    /// Whether this version may change it: its structure and its format
    /// are known and the value holds nothing else.
    pub editable: bool,
}

impl StoredText {
    /// `text` as plain text.
    pub fn plain(text: &str) -> StoredText {
        StoredText {
            format: PLAIN.to_string(),
            text: text.to_string(),
            version: VERSION,
            editable: true,
        }
    }

    /// `text` in `format` with structure `version` as it arrived from
    /// elsewhere, such as a Sent Item, to be stored as it is.
    pub fn of(format: &str, text: &str, version: i64) -> StoredText {
        StoredText {
            format: format.to_string(),
            text: text.to_string(),
            version,
            editable: version <= VERSION && KNOWN.contains(&format),
        }
    }

    /// The text a stored `value` holds; `None` reads as empty plain text.
    ///
    /// Up to the version known here, a value needs a string `format` and a
    /// string `text`. A later version may change or add keys: its value
    /// shows `text` when it is a string, nothing otherwise, and is never
    /// editable here.
    pub fn read(value: Option<&str>) -> StoredText {
        let Some(value) = value else {
            return StoredText::plain("");
        };
        if value.starts_with('{')
            && let Ok(Value::Object(json)) = serde_json::from_str::<Value>(value)
            && let Some(marked) = Self::marked(&json)
        {
            return marked;
        }
        StoredText::plain(value)
    }

    /// The text `json` holds when it is a marked value, `None` when not.
    fn marked(json: &Map<String, Value>) -> Option<StoredText> {
        let version = match json.get("v") {
            None => VERSION,
            Some(v) => v.as_i64().filter(|v| *v >= 1)?,
        };
        let string = |key: &str| match json.get(key) {
            Some(Value::String(s)) => Some(s.clone()),
            _ => None,
        };
        if version > VERSION {
            return Some(StoredText {
                format: string("format").unwrap_or_default(),
                text: string("text").unwrap_or_default(),
                version,
                editable: false,
            });
        }
        let (format, text) = (string("format")?, string("text")?);
        let keys = if json.contains_key("v") { 3 } else { 2 };
        let editable = KNOWN.contains(&format.as_str()) && json.len() == keys;
        Some(StoredText {
            format,
            text,
            version,
            editable,
        })
    }

    /// The text as it reads without its marks ([`visible`]).
    pub fn visible(&self) -> String {
        visible(&self.format, &self.text)
    }

    /// The value it is stored as, keys inserted in order so the bytes are
    /// the same with or without serde_json's `preserve_order`; version 1,
    /// the default, says no `v`.
    pub fn encode(&self) -> String {
        let mut map = Map::new();
        map.insert("format".into(), Value::String(self.format.clone()));
        map.insert("text".into(), Value::String(self.text.clone()));
        if self.version != VERSION {
            map.insert("v".into(), Value::from(self.version));
        }
        Value::Object(map).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FUTURE: &str = r##"{"format":"x-future","text":"# Heading\n**bold**"}"##;
    const LATER: &str = r#"{"format":"plain","more":1,"text":"Later milk","v":2}"#;

    #[test]
    fn plain_text_is_written_with_its_format_keys_sorted() {
        assert_eq!(
            StoredText::plain("Quote \" and \\ and \n").encode(),
            r#"{"format":"plain","text":"Quote \" and \\ and \n"}"#
        );
        assert_eq!(
            StoredText::read(Some(r#"{"format":"plain","text":"milk","v":1}"#)),
            StoredText::plain("milk")
        );
    }

    #[test]
    fn version_1_is_the_default_and_never_written() {
        let unversioned = StoredText::read(Some(r#"{"format":"plain","text":"milk"}"#));
        assert_eq!(unversioned, StoredText::plain("milk"));
        // Written with its default, it is read the same and written without.
        assert_eq!(
            StoredText::read(Some(r#"{"format":"plain","text":"milk","v":1}"#)).encode(),
            r#"{"format":"plain","text":"milk"}"#
        );
        let future = StoredText::read(Some(FUTURE));
        assert_eq!(future.version, 1);
        assert!(!future.editable);
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
            r#"{"format":"plain","text":"x","v":"1"}"#,
            r#"{"format":"plain","text":"x","v":1.5}"#,
            r#"{"format":"plain","text":"x","v":0}"#,
            r#"{"format":"plain","text":"x","v":-1}"#,
            r#"{"format":"plain","text":"x","v":true}"#,
            r#"{"format":"plain","text":"x","v":null}"#,
            r#"{"format":"plain","v":1}"#,
            r#"{"text":"x","v":1}"#,
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
        for value in [
            r#"{"format":"plain","more":1,"text":"x"}"#,
            r#"{"format":"plain","more":1,"text":"x","v":1}"#,
        ] {
            let more = StoredText::read(Some(value));
            assert_eq!((more.format.as_str(), more.text.as_str()), (PLAIN, "x"));
            assert!(!more.editable);
        }
        let unknown = StoredText::read(Some(r#"{"format":"x-future","text":"x","v":1}"#));
        assert_eq!(unknown.format, "x-future");
        assert!(!unknown.editable);
    }

    #[test]
    fn a_later_structure_shows_its_text_never_editable() {
        let later = StoredText::read(Some(LATER));
        assert_eq!(
            (later.format.as_str(), later.text.as_str(), later.version),
            (PLAIN, "Later milk", 2)
        );
        assert!(!later.editable);
        assert_ne!(later, StoredText::plain("Later milk"));
        // Without text, or keys of another kind, it shows nothing.
        for value in [
            r#"{"format":"plain","v":2}"#,
            r#"{"format":"plain","text":["x"],"v":2}"#,
            r#"{"text":7,"v":3}"#,
        ] {
            let text = StoredText::read(Some(value));
            assert_eq!(text.text, "", "{value}");
            assert!(!text.editable, "{value}");
        }
        assert_eq!(StoredText::read(Some(r#"{"text":7,"v":3}"#)).format, "");
    }

    #[test]
    fn a_text_from_elsewhere_keeps_its_version() {
        let later = StoredText::of(PLAIN, "x", 2);
        assert!(!later.editable);
        assert_eq!(later.encode(), r#"{"format":"plain","text":"x","v":2}"#);
        assert_eq!(StoredText::read(Some(&later.encode())), later);
        assert!(StoredText::of(PLAIN, "x", VERSION).editable);
        assert!(!StoredText::of("x-future", "x", 1).editable);
    }
}
