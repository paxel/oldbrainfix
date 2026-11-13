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
mod tests {
    use super::*;
    use crate::text_format::{CODE, MARKDOWN};

    /// `nodes` and those under them as "tag notes", each level indented.
    fn lines(nodes: &[TagNode], indent: &str) -> Vec<String> {
        let mut out = Vec::new();
        for node in nodes {
            out.push(format!("{indent}{} {}", node.tag, node.notes));
            out.extend(lines(&node.children, &format!("{indent}  ")));
        }
        out
    }

    fn tree(store: &Store) -> Result<Vec<String>> {
        Ok(lines(&TagTree::new(store).tree(&HashSet::new())?, ""))
    }

    #[test]
    fn nested_tags_come_by_their_levels_each_note_counted_once() -> Result<()> {
        let store = crate::store::tests::store();
        let notes = Notes::new(&store);
        notes.capture(
            "#life/health/back pain, #life/work and #life/work again",
            None,
            None,
        )?;
        notes.capture("#life/health", None, None)?;
        notes.capture("#garten", None, None)?;
        assert_eq!(
            tree(&store)?,
            [
                "#garten 1",
                "#life 2",
                "  #life/health 2",
                "    #life/health/back 1",
                "  #life/work 1",
            ]
        );
        let found = TagTree::new(&store).tree(&HashSet::new())?;
        let names: Vec<&str> = found.iter().map(|n| n.name.as_str()).collect();
        assert_eq!(names, ["garten", "life"]);
        Ok(())
    }

    #[test]
    fn each_part_is_written_as_written_most_often_ties_to_the_first() -> Result<()> {
        let store = crate::store::tests::store();
        let notes = Notes::new(&store);
        notes.capture("#Life/Health", None, None)?;
        notes.capture("#life/health #LIFE/x", None, None)?;
        notes.capture("#Life/back", None, None)?;
        notes.capture("#garten #Garten", None, None)?;
        assert_eq!(
            tree(&store)?,
            [
                "#Garten 1",
                "#Life 3",
                "  #Life/back 1",
                "  #Life/Health 2",
                "  #Life/x 1",
            ]
        );
        Ok(())
    }

    #[test]
    fn sections_and_merged_notes_count_trashed_ones_not() -> Result<()> {
        let store = crate::store::tests::store();
        let notes = Notes::new(&store);
        let host = notes.capture("Physio", None, None)?;
        notes.amend(&host.id, "then #health/back")?;
        let merged = notes.capture("#health/knee", None, None)?;
        notes.merge(&merged.id, &host.id)?;
        let gone = notes.capture("#trash/me", None, None)?;
        notes.trash(&gone.id)?;
        assert_eq!(
            tree(&store)?,
            ["#health 1", "  #health/back 1", "  #health/knee 1"]
        );
        Ok(())
    }

    #[test]
    fn tags_are_read_as_the_text_draws_them() -> Result<()> {
        let store = crate::store::tests::store();
        let notes = Notes::new(&store);
        notes.capture_as(
            "**#Bold/x** `#code/y` [#label](https://x.org/#dest)",
            MARKDOWN,
            None,
            None,
        )?;
        notes.capture_as("#script/a", CODE, None, None)?;
        notes.capture("see https://example.org/x#frag and C# #ok", None, None)?;
        assert_eq!(
            tree(&store)?,
            ["#Bold 1", "  #Bold/x 1", "#label 1", "#ok 1"]
        );
        Ok(())
    }

    #[test]
    fn a_changed_text_changes_the_tree() -> Result<()> {
        let store = crate::store::tests::store();
        let notes = Notes::new(&store);
        let note = notes.capture("#one", None, None)?;
        assert_eq!(tree(&store)?, ["#one 1"]);
        notes.edit(&note.id, "#two/three")?;
        assert_eq!(tree(&store)?, ["#two 1", "  #two/three 1"]);
        Ok(())
    }

    #[test]
    fn the_notes_of_a_closed_area_are_left_out_as_if_they_were_not_there() -> Result<()> {
        let store = crate::store::tests::store();
        let notes = Notes::new(&store);
        let places = Places::new(&store);
        let locked = places.add_area("Health")?.id;
        let topic = places.add_topic(&locked, "Back")?.id;
        let open = places.add_area("Home")?.id;
        let filed = notes.capture("#LIFE/Health #secret", Some(&locked), None)?;
        let in_topic = notes.capture("#LIFE/back", Some(&topic), None)?;
        notes.capture("#life/home", Some(&open), None)?;
        notes.capture("#Life", None, None)?;
        let every = tree(&store)?;
        assert_eq!(
            every,
            [
                "#LIFE 4",
                "  #LIFE/back 1",
                "  #LIFE/Health 1",
                "  #LIFE/home 1",
                "#secret 1",
            ]
        );
        let shut = HashSet::from([locked.clone()]);
        let closed = lines(&TagTree::new(&store).tree(&shut)?, "");
        // As the tree reads with those Notes gone.
        assert_eq!(closed, ["#Life 2", "  #Life/home 1"]);
        let tags = |typed: &str, closed: &HashSet<String>| -> Result<Vec<String>> {
            Ok(TagTree::new(&store)
                .suggestions(typed, closed)?
                .into_iter()
                .map(|n| n.tag)
                .collect())
        };
        assert_eq!(tags("#", &shut)?, ["#Life", "#Life/home"]);
        assert!(tags("#s", &shut)?.is_empty());
        assert_eq!(tags("#s", &HashSet::from([open]))?, ["#secret"]);
        // Asked again with nothing closed, nothing stays left out.
        assert_eq!(tree(&store)?, every);
        notes.trash(&filed.id)?;
        notes.trash(&in_topic.id)?;
        assert_eq!(tree(&store)?, closed);
        Ok(())
    }

    #[test]
    fn suggestions_are_the_tags_beginning_with_what_is_typed() -> Result<()> {
        let store = crate::store::tests::store();
        Notes::new(&store).capture(
            "#Garten #Garten/Beet #Gartenarbeit #life/health #gas",
            None,
            None,
        )?;
        let tags = |typed: &str| -> Result<Vec<String>> {
            Ok(TagTree::new(&store)
                .suggestions(typed, &HashSet::new())?
                .into_iter()
                .map(|n| n.tag)
                .collect())
        };
        assert_eq!(tags("#gar")?, ["#Garten", "#Garten/Beet", "#Gartenarbeit"]);
        assert_eq!(tags(" #GARTEN/ ")?, ["#Garten/Beet"]);
        assert_eq!(
            tags("#")?,
            [
                "#Garten",
                "#Garten/Beet",
                "#Gartenarbeit",
                "#gas",
                "#life",
                "#life/health"
            ]
        );
        assert!(tags("garten")?.is_empty());
        assert!(tags("#health")?.is_empty());
        Ok(())
    }
}
