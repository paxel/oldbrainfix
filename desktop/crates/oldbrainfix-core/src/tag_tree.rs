//! The tag tree (#267, ADR 0032), as the Dart core's `TagTree` reads it:
//! every tag in the texts of the Notes neither in the trash nor merged
//! and of their Sections, as the texts draw them, by its levels. Nothing
//! is stored for it; it is read from the texts. The Notes filed into an
//! Area of `closed`, the locked Areas not open now, are left out, as from
//! every list (Area Locks).

use std::collections::{HashMap, HashSet};

use crate::Result;
use crate::markdown;
use crate::notes::Notes;
use crate::places::Places;
use crate::search::lower_case;
use crate::store::{Store, utf16_cmp};
use crate::text_format;
use crate::text_tags::FoundTag;

/// One tag of the tag tree: a part of the nested tags written in the
/// Notes, such as "health" of "#life/health/back".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagNode {
    /// The part as it is written most often, such as "Health".
    pub name: String,
    /// The tag down to here, `#` and each part as it is written most
    /// often, such as "#Life/Health": what a click on it searches for.
    pub tag: String,
    /// How many Notes hold this tag or one under it, each counted once.
    pub notes: usize,
    /// The tags one level under it, by name.
    pub children: Vec<TagNode>,
}

pub struct TagTree<'a> {
    store: &'a Store,
}

impl<'a> TagTree<'a> {
    pub fn new(store: &'a Store) -> TagTree<'a> {
        TagTree { store }
    }

    /// The tags of the first level, each with those under it, by name: in
    /// lower case, by UTF-16 code units. Those only the Notes filed into
    /// an Area of `closed` hold do not show, nor do those Notes count.
    pub fn tree(&self, closed: &HashSet<String>) -> Result<Vec<TagNode>> {
        let read = self.store.worked_out("tag_tree", || self.read())?;
        let mut root = Level::default();
        for note in read.iter() {
            if note.area.as_ref().is_some_and(|a| closed.contains(a)) {
                continue;
            }
            for (parts, written) in &note.tags {
                let mut level = &mut root;
                for (part, spelling) in parts.iter().zip(written) {
                    let held = level.parts.entry(part.clone()).or_default();
                    *held.spellings.entry(spelling.clone()).or_default() += 1;
                    held.notes.insert(note.id.clone());
                    level = &mut held.under;
                }
            }
        }
        Ok(root.nodes("#"))
    }

    /// Every tag of the tree whose tag begins with `typed`, ignoring
    /// case, each before those under it: what is offered when a `#` is
    /// typed in the search. None when `typed` does not begin with a `#`.
    /// `closed` as for [`TagTree::tree`].
    pub fn suggestions(&self, typed: &str, closed: &HashSet<String>) -> Result<Vec<TagNode>> {
        let wanted = lower_case(typed.trim());
        if !wanted.starts_with('#') {
            return Ok(Vec::new());
        }
        fn walk(level: &[TagNode], wanted: &str, out: &mut Vec<TagNode>) {
            for node in level {
                if lower_case(&node.tag).starts_with(wanted) {
                    out.push(node.clone());
                }
                walk(&node.children, wanted, out);
            }
        }
        let mut out = Vec::new();
        walk(&self.tree(closed)?, &wanted, &mut out);
        Ok(out)
    }

    /// The tags of each Note in the lists, with the Area it is filed into,
    /// whatever is closed: kept until the log changes.
    fn read(&self) -> Result<Vec<NoteTags>> {
        let notes = Notes::new(self.store);
        let places = Places::new(self.store);
        let all = notes.all()?;
        let ids: Vec<String> = all.iter().map(|n| n.id.clone()).collect();
        let sections = notes.sections_of(&ids)?;
        let mut out = Vec::new();
        for note in &all {
            let mut texts = vec![(note.text_format.clone(), note.shown_text())];
            for s in sections.get(&note.id).into_iter().flatten() {
                texts.push((s.text_format.clone(), s.shown_text()));
            }
            let mut tags = Vec::new();
            for (format, text) in &texts {
                for found in tags_in(format, text) {
                    let written = text.get(found.start + 1..found.end).unwrap_or_default();
                    let parts = found.tag.get(1..).unwrap_or_default();
                    tags.push((
                        parts.split('/').map(str::to_string).collect(),
                        written.split('/').map(str::to_string).collect(),
                    ));
                }
            }
            let area = match &note.place {
                Some(place) => Some(match places.topic_by_id(place)? {
                    Some(topic) => topic.area_id,
                    None => place.clone(),
                }),
                None => None,
            };
            out.push(NoteTags {
                id: note.id.clone(),
                area,
                tags,
            });
        }
        Ok(out)
    }
}

/// The tags of one Note: each as its parts in lower case and as written,
/// and the Area the Note is filed into, directly or through a Topic.
struct NoteTags {
    id: String,
    area: Option<String>,
    tags: Vec<(Vec<String>, Vec<String>)>,
}

/// The tags of `text` in `format` as it is drawn: none in a Code text,
/// those outside code and links in a formatted one, those outside web
/// addresses in any other.
pub fn tags_in(format: &str, text: &str) -> Vec<FoundTag> {
    match format {
        text_format::CODE => Vec::new(),
        text_format::MARKDOWN => markdown::tags(text),
        _ => markdown::plain_tags(text),
    }
}

/// The parts of one level of the tree being read, by their lower case.
#[derive(Default)]
struct Level {
    parts: HashMap<String, Part>,
}

impl Level {
    fn nodes(&self, above: &str) -> Vec<TagNode> {
        let mut keys: Vec<&String> = self.parts.keys().collect();
        keys.sort_by(|a, b| utf16_cmp(a, b));
        keys.into_iter()
            .filter_map(|key| self.parts.get(key))
            .map(|part| part.node(above))
            .collect()
    }
}

#[derive(Default)]
struct Part {
    spellings: HashMap<String, usize>,
    notes: HashSet<String>,
    under: Level,
}

impl Part {
    fn node(&self, above: &str) -> TagNode {
        let mut name: Option<&String> = None;
        let mut most = 0;
        for (spelling, &count) in &self.spellings {
            let better = match name {
                None => true,
                Some(held) => {
                    count > most
                        || (count == most && utf16_cmp(spelling, held) == std::cmp::Ordering::Less)
                }
            };
            if better {
                name = Some(spelling);
                most = count;
            }
        }
        let name = name.cloned().unwrap_or_default();
        let tag = if above == "#" {
            format!("#{name}")
        } else {
            format!("{above}/{name}")
        };
        TagNode {
            children: self.under.nodes(&tag),
            notes: self.notes.len(),
            tag,
            name,
        }
    }
}

#[cfg(test)]
mod tests;
