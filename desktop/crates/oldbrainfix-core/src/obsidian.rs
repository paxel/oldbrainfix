//! Obsidian's links between the Markdown files of a folder (obf-1056),
//! for the desktop's viewer of a Markdown file and its Import as a Note;
//! the phone has neither, so the Dart core has no counterpart.
//!
//! A wiki link is `[[Name]]`, `[[Name|label]]`, `[[Name#Heading]]` or
//! both, on one line, with no `[` or `]` inside; a `!` before it embeds
//! what it names, a picture. Paths are inside the folder the links are
//! found in, the nearest one above the file holding `.obsidian`, else the
//! file's own: relative to it, their folders separated by `/`. Nothing
//! here reads a file; the apps hand over the paths they found, and a
//! path that would lead out of that folder finds nothing.

use crate::markdown;

/// A wiki link of a text: where it lies, in bytes, the `!` of an embed
/// included; whether it embeds; what it names, without its heading; the
/// heading after `#`, a block's `^id` counting as none; and the label
/// after `|`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiLink {
    pub start: usize,
    pub end: usize,
    pub embed: bool,
    pub target: String,
    pub heading: Option<String>,
    pub label: Option<String>,
}

impl WikiLink {
    /// The words it shows, as Obsidian shows them: its label, else what
    /// it names with " > " and its heading after it.
    pub fn shown(&self) -> String {
        match (&self.label, &self.heading) {
            (Some(label), _) => label.clone(),
            (None, Some(heading)) if self.target.is_empty() => heading.clone(),
            (None, Some(heading)) => format!("{} > {heading}", self.target),
            (None, None) => self.target.clone(),
        }
    }
}

/// The wiki link of `inner`, what stands between `[[` and `]]`; `None`
/// when it names nothing.
pub fn wiki_link(inner: &str) -> Option<(String, Option<String>, Option<String>)> {
    let (path, label) = match inner.split_once('|') {
        Some((path, label)) => (path, Some(label.trim().to_string())),
        None => (inner, None),
    };
    let (target, heading) = match path.split_once('#') {
        Some((target, heading)) => (target.trim(), Some(heading.trim())),
        None => (path.trim(), None),
    };
    let heading = heading
        .filter(|h| !h.is_empty() && !h.starts_with('^'))
        .map(str::to_string);
    if target.is_empty() && heading.is_none() {
        return None;
    }
    let label = label.filter(|l| !l.is_empty());
    Some((target.to_string(), heading, label))
}

/// The wiki links of `text`, first first; none in code, a code block or a
/// link's destination.
pub fn wiki_links(text: &str) -> Vec<WikiLink> {
    let literal = markdown::literal_ranges(text);
    let inside = |at: usize| literal.iter().any(|&(a, b)| a <= at && at < b);
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(open) = text.get(from..).and_then(|rest| rest.find("[[")) {
        let start = from + open;
        let body = start + 2;
        let Some(rest) = text.get(body..) else {
            break;
        };
        let close = rest
            .find(['[', ']', '\n'])
            .filter(|&at| rest.get(at..).is_some_and(|r| r.starts_with("]]")));
        let Some(close) = close else {
            from = body;
            continue;
        };
        let end = body + close + 2;
        let embed = start > 0 && text.get(start - 1..start) == Some("!");
        let start = if embed { start - 1 } else { start };
        if !inside(start)
            && let Some((target, heading, label)) = rest.get(..close).and_then(wiki_link)
        {
            found.push(WikiLink {
                start,
                end,
                embed,
                target,
                heading,
                label,
            });
        }
        from = end;
    }
    found
}

/// A picture of a text in Markdown's own way, `![alt](path)`: where it
/// lies, in bytes, and its path as written, without `<` and `>` around
/// it and without a title after it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picture {
    pub start: usize,
    pub end: usize,
    pub path: String,
}

/// The pictures of `text` in Markdown's own way, first first, on one
/// line each; none in code or a code block.
pub fn pictures(text: &str) -> Vec<Picture> {
    let literal = markdown::literal_ranges(text);
    let inside = |at: usize| literal.iter().any(|&(a, b)| a <= at && at < b);
    let mut found = Vec::new();
    let mut from = 0;
    while let Some(open) = text.get(from..).and_then(|rest| rest.find("![")) {
        let start = from + open;
        from = start + 2;
        if text.get(from..).is_some_and(|r| r.starts_with('[')) {
            continue;
        }
        let Some(rest) = text.get(from..) else {
            break;
        };
        let Some(close) = rest
            .find(['\n', ']'])
            .filter(|&at| rest.get(at..=at) == Some("]"))
        else {
            continue;
        };
        let Some(after) = rest.get(close + 1..).filter(|r| r.starts_with('(')) else {
            continue;
        };
        let Some(shut) = after
            .find([')', '\n'])
            .filter(|&at| after.get(at..=at) == Some(")"))
        else {
            continue;
        };
        let inner = after.get(1..shut).unwrap_or_default().trim();
        let path = match inner.strip_prefix('<') {
            Some(rest) => rest.split('>').next().unwrap_or_default(),
            None => inner.split([' ', '\t']).next().unwrap_or_default(),
        };
        let end = from + close + 1 + shut + 1;
        if !path.is_empty() && !inside(start) {
            found.push(Picture {
                start,
                end,
                path: path.to_string(),
            });
        }
        from = end;
    }
    found
}

/// Whether `name` is a Markdown file's: `.md` or `.markdown`, in any case
/// of ASCII letters.
pub fn is_markdown(name: &str) -> bool {
    name.rsplit_once('.').is_some_and(|(_, ending)| {
        ending.eq_ignore_ascii_case("md") || ending.eq_ignore_ascii_case("markdown")
    })
}

/// `name` without a Markdown file's ending.
fn without_ending(name: &str) -> &str {
    match name.rsplit_once('.') {
        Some((stem, _)) if is_markdown(name) => stem,
        _ => name,
    }
}

/// The last part of the path `path`.
fn base(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// The folder of the path `path`, with its `/`, empty at the top.
fn folder(path: &str) -> &str {
    match path.rfind('/') {
        Some(at) => path.get(..=at).unwrap_or_default(),
        None => "",
    }
}

/// The path `path` with its `.` and `..` worked out, `\` read as `/`;
/// `None` when it leads out of the folder at the top, or starts at a
/// root of its own.
pub fn normal(path: &str) -> Option<String> {
    let path = path.replace('\\', "/");
    if path.starts_with('/') || path.contains(':') {
        return None;
    }
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            part => parts.push(part),
        }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}

/// `text` with each `%` and two hexadecimal digits as the byte they name,
/// as a picture's path in Markdown's own way writes a space; as it is
/// when that is no UTF-8.
pub fn percent_decoded(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while let Some(&b) = bytes.get(at) {
        let hex = bytes
            .get(at + 1..at + 3)
            .and_then(|h| std::str::from_utf8(h).ok())
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        match hex {
            Some(byte) if b == b'%' => {
                out.push(byte);
                at += 3;
            }
            _ => {
                out.push(b);
                at += 1;
            }
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| text.to_string())
}

/// Which of `paths` the wiki link's `target` names, a Markdown file: the
/// one at that path, its ending left out or not, else the one of that
/// name, both in any case; of several, the first of `paths`, which the
/// apps hand over shortest first. `None` when none does.
pub fn note_target(target: &str, paths: &[String]) -> Option<usize> {
    let wanted = without_ending(target.trim()).to_lowercase();
    if wanted.is_empty() {
        return None;
    }
    let by_path = paths
        .iter()
        .position(|p| is_markdown(p) && without_ending(p).to_lowercase() == wanted);
    if by_path.is_some() {
        return by_path;
    }
    if wanted.contains('/') {
        let ending = format!("/{wanted}");
        return paths
            .iter()
            .position(|p| is_markdown(p) && without_ending(p).to_lowercase().ends_with(&ending));
    }
    paths
        .iter()
        .position(|p| is_markdown(p) && without_ending(base(p)).to_lowercase() == wanted)
}

/// Which of `paths` an embed's `target` names, as written in the file at
/// the path `from`: by its path from the top, from the file's folder, or
/// by its name, in any case; `None` when none does.
pub fn file_target(target: &str, from: &str, paths: &[String]) -> Option<usize> {
    let target = target.trim();
    let same = |a: &str, b: &str| a.to_lowercase() == b.to_lowercase();
    let near = normal(&format!("{}{target}", folder(from)));
    let top = normal(target);
    for wanted in [near, top].into_iter().flatten() {
        if let Some(found) = paths.iter().position(|p| same(p, &wanted)) {
            return Some(found);
        }
    }
    if target.contains('/') {
        return None;
    }
    paths.iter().position(|p| same(base(p), target))
}

/// Which of `paths` a picture's `path` in Markdown's own way names, as
/// written in the file at the path `from`: relative to that file's
/// folder, its `%20` as a space; none for a web address or a path out of
/// the folder.
pub fn picture_target(path: &str, from: &str, paths: &[String]) -> Option<usize> {
    let decoded = percent_decoded(path.trim());
    let wanted = normal(&format!("{}{decoded}", folder(from)))?;
    paths
        .iter()
        .position(|p| p.to_lowercase() == wanted.to_lowercase())
}

#[cfg(test)]
mod tests;
