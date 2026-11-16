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
