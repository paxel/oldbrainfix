use std::collections::BTreeMap;

use chrono::{DateTime, NaiveDateTime, Utc};

use crate::boxes::Boxes;
use crate::checklists::Checklists;
use crate::notes::{NewFile, Note, Notes};
use crate::places::Places;
use crate::search::Search;
use crate::shopping_lists::{Reappear, ShoppingLists};
use crate::store::Store;
use crate::tallies::Tallies;
use crate::text_boxes::BoxState;
use crate::text_format::MARKDOWN;
use crate::{Result, text_format};

/// What every comparison searches for, as the Dart core's test does.
const QUERIES: &[&str] = &[
    "sauce",
    "MILK",
    "receipt",
    "x-ray",
    "müller",
    " e ",
    "ce",
    "sau",
    "**",
    "milk #code",
    "4711",
    "\"",
    "say \"hi\"",
    "*",
    "a OR b",
    "NEAR(x y)",
    "buy  milk",
    "İzmir",
    "#izmir",
    "οδοσ",
    "𝔸x",
    "𝔸",
    "\u{2705} call",
    "\u{2610}",
    "tent",
    "oat",
    "after work",
    "parcel",
    "lease",
    "rent due",
    "plan.dwg",
    "opening",
    "bags",
    "nothing at all",
    "buy sauce",
    "sauce after",
    "odd place",
    "nul\u{0}",
];

fn wall(t: DateTime<Utc>) -> NaiveDateTime {
    t.naive_utc()
}

fn ids(found: Vec<Note>) -> Vec<String> {
    found.into_iter().map(|n| n.id).collect()
}

/// Every query found with the index as without it, everywhere, and
/// among the trash.
fn same(store: &Store, reason: &str) -> Result<()> {
    let places = Places::new(store);
    let mut within = vec![None];
    for area in places.areas()? {
        within.push(Some(area.id.clone()));
        for topic in places.topics(&area.id)? {
            within.push(Some(topic.id));
        }
    }
    let trashed = Notes::new(store).trashed()?;
    for query in QUERIES {
        for place in &within {
            assert_eq!(
                ids(Search::new(store).notes(query, place.as_deref())?),
                ids(Search::scanning(store).notes(query, place.as_deref())?),
                "{reason}: {query:?} in {place:?}"
            );
        }
        assert_eq!(
            ids(Search::new(store).among(query, trashed.clone())?),
            ids(Search::scanning(store).among(query, trashed.clone())?),
            "{reason}: {query:?} in the trash"
        );
    }
    Ok(())
}

/// The ids `fill` made, by what they are.
type Made = BTreeMap<&'static str, String>;

/// Notes of every kind search reads.
fn fill(store: &Store) -> Result<Made> {
    let places = Places::new(store);
    let notes = Notes::new(store);
    let home = places.add_area("Household")?.id;
    let kitchen = places.add_topic(&home, "Kitchen")?.id;
    let work = places.add_area("Work")?.id;
    let sauce = notes.capture("Buy SAUCE and milk", Some(&kitchen), None)?;
    notes.capture_as("**bold** sauce `code`", MARKDOWN, None, None)?;
    notes.capture_as("milk #code tea", text_format::CODE, None, None)?;
    notes.capture("#İzmir trip and ΟΔΟΣ 𝔸x, say \"hi\"", Some(&home), None)?;
    let boxes = notes.capture(
        "\u{2610} call the x-ray clinic\n\u{2610} Müller",
        None,
        None,
    )?;
    let listed = notes.capture_as("- \u{2610} buy **sauce** pan", MARKDOWN, None, None)?;
    notes.capture("nul\u{0} then sauce after it", None, None)?;
    // Values no Note holds, or holds unreadable: candidates read away.
    store.append("section:odd", "text", Some("sauce in an odd place"), None)?;
    store.append("place:odd", "file:abc", Some("odd place sauce"), None)?;
    store.append(&sauce.id, "file:def", Some("{not json sauce"), None)?;
    let voice = notes.capture_with("", None, None, &[], &[b"voice".to_vec()])?;
    notes.set_transcript(&voice.id, &voice.recordings[0], "Pick up parcel")?;
    let photo = notes.capture_with("", None, None, &[b"photo".to_vec()], &[])?;
    notes.set_photo_text(&photo.id, &photo.photos[0], "RECEIPT 4711")?;
    notes.remove_photo_text(&photo.id, &photo.photos[0])?;
    notes.set_photo_text(&photo.id, &photo.photos[0], "receipt 12,50")?;
    let file = |name: &'static str, bytes: &'static [u8]| NewFile {
        name,
        media_type: "",
        bytes,
        pages: None,
    };
    let letter = notes.capture_with_files(
        "Papers",
        Some(&work),
        None,
        &[],
        &[],
        &[file("Lease.txt", &[1, 2]), file("plan.dwg", &[3])],
    )?;
    notes.set_attachment_text(
        &letter.id,
        &letter.attachments[0].hash,
        "Rent due on the first",
    )?;
    let host = notes.capture("Errands", Some(&home), None)?;
    let merged = notes.capture("Opening hours", None, None)?;
    notes.amend(&merged.id, "and the bags")?;
    notes.merge(&merged.id, &host.id)?;
    let tally = notes.capture("Push-ups", None, None)?;
    let tallies = Tallies::new(store);
    let reps = tallies.add_field(&tally.id, "times", "times", None, None)?;
    tallies.add(
        &tally.id,
        &BTreeMap::from([(reps.clone(), 20.0)]),
        "after work",
        None,
    )?;
    let gone = tallies.add(&tally.id, &BTreeMap::from([(reps, 5.0)]), "sauce run", None)?;
    tallies.trash_entry(&gone.id)?;
    let trip = notes.capture("Trip\nTent\nSleeping bag", None, None)?;
    Checklists::new(store).make(&trip.id)?;
    let shop = notes.capture("Shop\nOat milk\nSauce", None, None)?;
    ShoppingLists::new(store).make(&shop.id)?;
    let old = notes.capture("Old sauce recipe", None, None)?;
    notes.trash(&old.id)?;
    Ok(BTreeMap::from([
        ("sauce", sauce.id),
        ("boxes", boxes.id),
        ("listed", listed.id),
        ("host", host.id),
        ("merged", merged.id),
        ("tally", tally.id),
        ("trip", trip.id),
        ("shop", shop.id),
        ("old", old.id),
        ("letter", letter.id),
    ]))
}

fn id<'m>(made: &'m Made, what: &str) -> &'m str {
    made.get(what).map(String::as_str).unwrap_or_default()
}

#[test]
fn the_index_finds_what_reading_every_note_finds() -> Result<()> {
    let store = crate::store::tests::store();
    fill(&store)?;
    same(&store, "built")
}

#[test]
fn every_change_is_taken_in_before_the_next_search() -> Result<()> {
    let store = crate::store::tests::store();
    let made = fill(&store)?;
    let notes = Notes::new(&store);
    same(&store, "built")?;
    notes.edit(id(&made, "sauce"), "Buy tea")?;
    same(&store, "edited")?;
    notes.amend(id(&made, "sauce"), "Update: more MILK and sauce")?;
    same(&store, "amended")?;
    let boxed = notes
        .by_id(id(&made, "boxes"))?
        .ok_or(crate::Error::Invalid("boxes".into()))?;
    Boxes::new(&store).tick(id(&made, "boxes"), &boxed.boxes[0].id, BoxState::Done)?;
    same(&store, "ticked")?;
    let listed = notes
        .by_id(id(&made, "listed"))?
        .ok_or(crate::Error::Invalid("listed".into()))?;
    Boxes::new(&store).tick(id(&made, "listed"), &listed.boxes[0].id, BoxState::Done)?;
    same(&store, "ticked in a formatted text")?;
    notes.undo_merge(id(&made, "merged"))?;
    same(&store, "merge undone")?;
    notes.merge(id(&made, "host"), id(&made, "sauce"))?;
    same(&store, "merged in a chain")?;
    let tallies = Tallies::new(&store);
    let tally = tallies
        .by_note(id(&made, "tally"))?
        .ok_or(crate::Error::Invalid("tally".into()))?;
    let reps = tally.fields[0].id.clone();
    tallies.add(
        id(&made, "tally"),
        &BTreeMap::from([(reps, 3.0)]),
        "sauce at night",
        None,
    )?;
    same(&store, "entry added")?;
    let lists = Checklists::new(&store);
    let trip = lists
        .by_note(id(&made, "trip"))?
        .ok_or(crate::Error::Invalid("trip".into()))?;
    lists.delete(&trip.lines[0].id)?;
    lists.add(id(&made, "trip"), "Sauce pan", None, false)?;
    same(&store, "lines changed")?;
    lists.to_plain(id(&made, "trip"))?;
    same(&store, "back to a plain Note")?;
    lists.make(id(&made, "trip"))?;
    same(&store, "made again")?;
    let shopping = ShoppingLists::new(&store);
    let powder = shopping.add(id(&made, "shop"), "Milk powder", &wall)?;
    shopping.set_reappear(&powder, Reappear::Once)?;
    shopping.check(&powder, &wall)?;
    same(&store, "a Line gone")?;
    notes.restore(id(&made, "old"))?;
    same(&store, "restored")?;
    notes.trash(id(&made, "letter"))?;
    same(&store, "trashed")
}

#[test]
fn what_another_device_wrote_is_found_once_it_is_in_the_log() -> Result<()> {
    let store = crate::store::tests::store();
    fill(&store)?;
    same(&store, "built")?;
    let other = Store::in_memory(crate::store::tests::ticking(), || {
        Ok(crate::SigningKey::from_seed([3; 32]))
    })?;
    let theirs = Notes::new(&other).capture("Sauce from the other Device", None, None)?;
    Notes::new(&other).amend(&theirs.id, "with milk")?;
    for e in other.all_entries()? {
        store.insert(&e.wire())?;
    }
    same(&store, "received")?;
    assert_eq!(
        ids(Search::new(&store).notes("other device", None)?),
        std::slice::from_ref(&theirs.id)
    );
    let amends: Vec<i64> = other
        .all_entries()?
        .into_iter()
        .filter(|e| e.field == "text" && e.entity.starts_with("section:"))
        .map(|e| e.dseq)
        .collect();
    store.delete_entries(&other.device_id()?, &amends)?;
    same(&store, "deleted")?;
    assert!(Search::new(&store).notes("with milk", None)?.is_empty());
    Ok(())
}

#[test]
fn another_store_on_the_same_file_finds_what_this_one_wrote() -> Result<()> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("log.db");
    let open = || {
        Store::open(&path, crate::store::tests::ticking(), || {
            Ok(crate::SigningKey::from_seed([4; 32]))
        })
    };
    let one = open()?;
    let two = open()?;
    let first = Notes::new(&one).capture("first sauce", None, None)?;
    assert_eq!(
        ids(Search::new(&two).notes("sauce", None)?),
        std::slice::from_ref(&first.id)
    );
    let second = Notes::new(&one).capture("second sauce", None, None)?;
    two.changed_elsewhere();
    let mut found = ids(Search::new(&two).notes("SAUCE", None)?);
    found.sort();
    let mut both = vec![first.id, second.id.clone()];
    both.sort();
    assert_eq!(found, both);
    assert_eq!(ids(Search::new(&one).notes("second", None)?), [second.id]);
    Ok(())
}

#[test]
fn a_missing_or_older_index_is_built_anew() -> Result<()> {
    let store = crate::store::tests::store();
    let made = fill(&store)?;
    same(&store, "built")?;
    let run = |sql: &str| store.with_connection(|db| Ok(db.execute_batch(sql)?));
    run("DROP TABLE search_texts")?;
    assert!(
        ids(Search::new(&store).notes("sauce", None)?).contains(&id(&made, "sauce").to_string())
    );
    same(&store, "table dropped")?;
    run("UPDATE search_state SET value = 0 WHERE key = 'version'; DELETE FROM search_texts")?;
    same(&store, "another version")?;
    run("DELETE FROM search_state WHERE key = 'seq'; DELETE FROM search_texts")?;
    same(&store, "no mark")
}

#[test]
fn a_shown_pending_change_is_found_as_the_reads_show_it() -> Result<()> {
    let store = crate::store::tests::store();
    let made = fill(&store)?;
    same(&store, "built")?;
    let set = store.open_pending("ai-task", true, None)?;
    let value = crate::text_format::StoredText::plain("a drafted gooseberry").encode();
    store.draft_pending(&set, id(&made, "sauce"), "text", Some(&value), None)?;
    assert_eq!(
        ids(Search::new(&store).notes("gooseberry", None)?),
        [id(&made, "sauce").to_string()]
    );
    same(&store, "drafted")
}
