//! Tags in user text (ADR 0032), as the Dart core's `text_tags.dart`
//! finds them: a `#word` in any text is drawn highlighted, and a click on
//! it searches for it. Nothing is stored for a tag; it is part of the
//! text, and the tag tree is read from the texts ([`crate::tag_tree`]).
//!
//! A tag is a `#` followed by letters, digits, `_` and `-`, at least one
//! letter among them, the first no `-` and any `-` at its end left out.
//! A `/` inside parts a nested tag, such as "#life/health": each part
//! starts as the tag does, so a `/` at the end, `//` and `/-` end it.
//! Before the `#` comes the start of the text or a character that is no
//! letter, digit, `_` or `-` and no `#`, `/`, `\` or `&`, so neither "C#"
//! nor a web address's "/#top" is one. A letter is a character of
//! Unicode's Alphabetic property, a digit one of its numbers. Offsets are
//! in UTF-8 bytes here, where the Dart core counts UTF-16 code units.

use crate::search::lower_case;

/// A tag found in a text: where it lies, `start..end` in bytes with its
/// `#`, and the tag it is, `#` and its name in lower case, which is also
/// what a search for it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoundTag {
    pub start: usize,
    pub end: usize,
    pub tag: String,
}

/// Every tag in `text`, first one first.
pub fn find(text: &str) -> Vec<FoundTag> {
    let mut found = Vec::new();
    let mut at = 0;
    while let Some(hash) = text.get(at..).and_then(|rest| rest.find('#')) {
        let hash = at + hash;
        let from = hash + 1;
        at = from;
        if !text
            .get(..hash)
            .and_then(|before| before.chars().next_back())
            .is_none_or(may_precede)
        {
            continue;
        }
        let mut stop = from;
        let mut letters = false;
        let rest = text.get(from..).unwrap_or_default();
        for (i, c) in rest.char_indices() {
            if c == '/' {
                let next = rest.get(i + 1..).and_then(|r| r.chars().next());
                if i == 0 || !next.is_some_and(|n| n != '-' && in_tag(n)) {
                    break;
                }
                at = from + i + 1;
                continue;
            }
            if !in_tag(c) || (i == 0 && c == '-') {
                break;
            }
            letters |= c.is_alphabetic();
            at = from + i + c.len_utf8();
            if c != '-' {
                stop = at;
            }
        }
        if letters && let Some(name) = text.get(from..stop) {
            found.push(FoundTag {
                start: hash,
                end: stop,
                tag: format!("#{}", lower_case(name)),
            });
        }
    }
    found
}

/// The label `name` of another app as a tag, such as "Home & Garden" as
/// "#Home-Garden": each run of characters that are no letter, digit or
/// `_` becomes one `-`, and a `-` at either end goes; the case stays as
/// written. `None` when nothing is left. A name without a letter, such as
/// "2024", comes to "#2024", which reads as a number, not a tag.
pub fn from_label(name: &str) -> Option<String> {
    let mut out = String::new();
    let mut gap = false;
    for c in name.chars() {
        if in_tag(c) && c != '-' {
            if gap && !out.is_empty() {
                out.push('-');
            }
            out.push(c);
            gap = false;
        } else {
            gap = true;
        }
    }
    (!out.is_empty()).then(|| format!("#{out}"))
}

fn in_tag(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '-'
}

fn may_precede(c: char) -> bool {
    !(in_tag(c) || matches!(c, '#' | '/' | '\\' | '&'))
}

#[cfg(test)]
mod tests;
