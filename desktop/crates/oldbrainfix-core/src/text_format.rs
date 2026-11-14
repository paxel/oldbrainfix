//! The formats of user text (ADR 0018), as the Dart core's
//! `text_format.dart` reads and writes them: the text of a Note or a
//! Section is stored with the format it is in. This version knows
//! [`PLAIN`], [`MARKDOWN`] and [`CODE`].

use serde_json::{Map, Value};

use crate::markdown;
use crate::text_boxes;

/// Text as typed, without formatting.
pub const PLAIN: &str = "plain";

/// Formatted text, the subset of Markdown [`crate::markdown`] reads (ADR
/// 0033). A text stays plain until it is first formatted, and is
/// formatted from then on.
pub const MARKDOWN: &str = "markdown";

/// A Note's whole text as code (#213): shown in the fixed-width font
/// exactly as written, unwrapped, nothing in it read as a format, a box
/// or a tag.
pub const CODE: &str = "code";

/// Every format this version reads and writes.
pub const KNOWN: [&str; 3] = [PLAIN, MARKDOWN, CODE];

/// The newest structure of the stored value this version knows: `v` 1
/// holds `format` and `text` and nothing else.
pub const VERSION: i64 = 1;

/// `text` in `format` as it reads without its marks and without the ids
/// of its boxes (ADR 0035), for search and previews; a text in any other
/// format as it is.
pub fn visible(format: &str, text: &str) -> String {
    match format {
        MARKDOWN => markdown::plain(&text_boxes::strip(text)),
        PLAIN | CODE => text_boxes::strip(text),
        _ => text.to_string(),
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

/// Whether a pasted `text` looks like a script or code, for the editor
/// to offer it as [`CODE`] (#213), the same in both cores: it starts with
/// `#!/`, or it has at least three lines that are not blank, at least
/// half of them ending in `;`, `{` or `}`, starting with `}`, or indented
/// by a tab or four spaces and not starting with `-`, `*` or `+` after
/// it. Only spaces and tabs count as blank and as indentation.
pub fn looks_like_code(text: &str) -> bool {
    if text.starts_with("#!/") {
        return true;
    }
    let blank = |c: &u8| *c == b' ' || *c == b'\t';
    let (mut lines, mut code) = (0, 0);
    for line in text.split("\r\n").flat_map(|l| l.split(['\r', '\n'])) {
        let bytes = line.as_bytes();
        let Some(&first) = bytes.iter().find(|c| !blank(c)) else {
            continue;
        };
        let last = bytes.iter().rfind(|c| !blank(c)).copied().unwrap_or(first);
        lines += 1;
        let indented = (line.starts_with('\t') || line.starts_with("    "))
            && !matches!(first, b'-' | b'*' | b'+');
        if matches!(last, b';' | b'{' | b'}') || first == b'}' || indented {
            code += 1;
        }
    }
    lines >= 3 && 2 * code >= lines
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
mod tests;
