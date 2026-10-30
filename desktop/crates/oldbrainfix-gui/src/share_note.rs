//! A Note handed to another app from the desktop (#129), as the phone's
//! `share_note.dart` hands it to the share sheet: "Copy as text" puts the
//! Note's share text on the clipboard, with HTML beside formatted text,
//! and "Save as Markdown…" writes it as a Markdown file with the photos
//! and recordings this Device holds in a folder of its own beside it,
//! named as the file is, linked from the file by their place in it. What
//! goes along is the core's
//! ([`note_share`](oldbrainfix_core::share::note_share)).
//!
//! The Markdown file holds the Markdown source of formatted text as it
//! is, and plain text and the Sections' dates escaped, so they read as
//! written in any Markdown viewer.

use std::path::Path;

use oldbrainfix_core::Notes;
use oldbrainfix_core::share::NoteShare;

use crate::media;

/// The longest name a Markdown file is proposed under, without `.md`.
const NAME_LENGTH: usize = 40;

/// What a Note is proposed to be saved as: its first line, `title`, with
/// only letters, digits, spaces, `-` and `_`, at most [`NAME_LENGTH`]
/// characters, and `.md`; `note.md` when nothing of it is left.
pub fn file_name(title: &str) -> String {
    let first = title.lines().next().unwrap_or_default();
    let kept: String = first
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                ' '
            }
        })
        .collect();
    let words: Vec<&str> = kept.split_whitespace().collect();
    let mut name = String::new();
    for word in words {
        let length = name.chars().count() + usize::from(!name.is_empty()) + word.chars().count();
        if length > NAME_LENGTH {
            if name.is_empty() {
                name = word.chars().take(NAME_LENGTH).collect();
            }
            break;
        }
        if !name.is_empty() {
            name.push(' ');
        }
        name.push_str(word);
    }
    if name.is_empty() {
        name.push_str("note");
    }
    format!("{name}.md")
}

/// A photo or recording written beside the Markdown file: its name and
/// whether it is a photo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Beside {
    pub name: String,
    pub photo: bool,
}

/// The Markdown file of `text` with `files` in the folder `folder`
/// beside it: the text, then each file on a paragraph of its own, a photo
/// as an image and a recording as a link, both named by its name and
/// pointing at it in the folder.
pub fn markdown(text: &str, folder: &str, files: &[Beside]) -> String {
    let mut parts: Vec<String> = Vec::new();
    if !text.is_empty() {
        parts.push(text.to_string());
    }
    for file in files {
        let link = link_to(&format!("{folder}/{}", file.name));
        parts.push(if file.photo {
            format!("![{}]({link})", file.name)
        } else {
            format!("[{}]({link})", file.name)
        });
    }
    let mut out = parts.join("\n\n");
    if !out.is_empty() {
        out.push('\n');
    }
    out
}

/// `path` as a link in Markdown points at it: each ASCII character but
/// letters, digits and `-._~/` as `%` and its hex, so spaces and
/// brackets in a folder's name keep the link whole.
fn link_to(path: &str) -> String {
    let mut out = String::new();
    for c in path.chars() {
        if !c.is_ascii() || c.is_ascii_alphanumeric() || "-._~/".contains(c) {
            out.push(c);
        } else {
            out.push_str(&format!("%{:02X}", u32::from(c)));
        }
    }
    out
}

/// Whether saving `share` writes anything: it has words, or any of its
/// photos and recordings is on this Device, which is asked without
/// reading them.
pub fn has_anything(share: &NoteShare, notes: &Notes) -> bool {
    !share.text.is_empty()
        || share.photos.iter().any(|h| notes.has_photo(h))
        || share.recordings.iter().any(|h| notes.has_recording(h))
}

/// Writes `share` as the Markdown file `path`, its photos and then its
/// recordings that `notes` holds in a folder beside it named as the file
/// without `.md`, so the files of two Notes never mix. Each is named by
/// its hash as the phone names them (`photo-<12 hex>.jpg`,
/// `recording-<12 hex>.wav`): the same bytes under the same name, so
/// writing one again changes nothing. The files go first, so a link
/// never points at nothing; without any, there is no folder. `Ok(false)`
/// when there is nothing to write: no words and none of its photos and
/// recordings here.
pub fn save_markdown(path: &Path, share: &NoteShare, notes: &Notes) -> std::io::Result<bool> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let stem = path
        .file_stem()
        .map_or_else(|| "note".to_string(), |s| s.to_string_lossy().into_owned());
    let folder = parent.join(&stem);
    let mut files = Vec::new();
    for hash in &share.photos {
        // One not on this Device yet stays behind; the rest goes.
        let Some(bytes) = notes.photo_bytes(hash) else {
            continue;
        };
        files.push((
            Beside {
                name: format!("photo-{}.{}", short(hash), media::photo_extension(&bytes)),
                photo: true,
            },
            bytes,
        ));
    }
    for hash in &share.recordings {
        let Some(bytes) = notes.recording_bytes(hash) else {
            continue;
        };
        files.push((
            Beside {
                name: format!(
                    "recording-{}.{}",
                    short(hash),
                    media::recording_extension(&bytes)
                ),
                photo: false,
            },
            bytes,
        ));
    }
    if share.text.is_empty() && files.is_empty() {
        return Ok(false);
    }
    if !files.is_empty() {
        std::fs::create_dir_all(&folder)?;
    }
    for (file, bytes) in &files {
        std::fs::write(folder.join(&file.name), bytes)?;
    }
    let beside: Vec<Beside> = files.into_iter().map(|(file, _)| file).collect();
    std::fs::write(path, markdown(&share.markdown, &stem, &beside))?;
    Ok(true)
}

/// The first 12 characters of `hash`, as the phone shortens it.
fn short(hash: &str) -> &str {
    hash.char_indices()
        .nth(12)
        .map_or(hash, |(at, _)| &hash[..at])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_note_is_proposed_under_its_first_line() {
        assert_eq!(file_name("Milk, eggs & bread\nmore"), "Milk eggs bread.md");
        assert_eq!(file_name("../../etc/passwd"), "etc passwd.md");
        assert_eq!(file_name("**?**"), "note.md");
        assert_eq!(
            file_name("one two three four five six seven eight nine ten"),
            "one two three four five six seven eight.md"
        );
        assert_eq!(file_name(&"x".repeat(50)), format!("{}.md", "x".repeat(40)));
    }

    #[test]
    fn the_markdown_links_its_files_by_name() {
        let files = [
            Beside {
                name: "photo-abc.jpg".into(),
                photo: true,
            },
            Beside {
                name: "recording-def.wav".into(),
                photo: false,
            },
        ];
        assert_eq!(
            markdown("**Milk**", "Milk", &files),
            "**Milk**\n\n![photo-abc.jpg](Milk/photo-abc.jpg)\n\n[recording-def.wav](Milk/recording-def.wav)\n"
        );
        assert_eq!(
            markdown("", "Milk (2) ä", &files[1..]),
            "[recording-def.wav](Milk%20%282%29%20ä/recording-def.wav)\n"
        );
        assert_eq!(short("0123456789abcdef"), "0123456789ab");
        assert_eq!(short("abc"), "abc");
    }
}
