//! Writing a Note as it is captured (#113): [`capture_choices::capture`],
//! which the quick note, the popup and the window's dialog save through,
//! over a store with its clock held still. A Note filed where it goes
//! gets no Nudge, an Unsorted one its Nudge, an empty one writes nothing,
//! and a Kind it is made applies to the typed lines only (#310). The quick
//! note itself sets nothing up any more (#352): what it showed for that
//! is the Note dialog's, tested there.

use std::sync::Arc;

use chrono::Duration;
use oldbrainfix_core::{
    Checklists, Note, Notes, OwnProfile, Places, ShoppingLists, SigningKey, Store, parse_time,
};
use oldbrainfix_gui::capture_choices::{self, CaptureChoices, CaptureKind, Captured};

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// A store with the Area "Household"; the store and the Area's id.
fn store() -> Result<(Arc<Store>, String), Box<dyn std::error::Error>> {
    let now = parse_time("2026-10-26T09:00:00.000Z")?;
    let store = Store::in_memory(Box::new(move || now), || Ok(SigningKey::from_seed([3; 32])))?;
    OwnProfile::new(&store).apply_defaults("Desk", None)?;
    let household = Places::new(&store).add_area("Household")?.id;
    Ok((Arc::new(store), household))
}

fn nudge(store: &Store, id: &str) -> Result<Option<String>, Box<dyn std::error::Error>> {
    Ok(store.current(id, "nudge")?)
}

fn wall(at: chrono::DateTime<chrono::Utc>) -> chrono::NaiveDateTime {
    at.naive_utc()
}

/// Captures `text` into `place` with `choices`, nothing held.
fn captured(
    store: &Store,
    text: &str,
    place: Option<&str>,
    choices: &CaptureChoices,
) -> Result<Captured, Box<dyn std::error::Error>> {
    Ok(capture_choices::capture(
        store,
        text,
        place,
        Duration::minutes(10),
        (&[], &[], &[]),
        (choices, &wall),
    )?)
}

fn kept(captured: Captured) -> Result<Note, Box<dyn std::error::Error>> {
    match captured {
        Captured::Note(note) => Ok(note),
        _ => Err("a Note".into()),
    }
}

#[test]
fn an_unsorted_capture_keeps_its_nudge_and_a_filed_one_none() -> TestResult {
    let (store, household) = store()?;
    let choices = CaptureChoices::default();
    let stamps = kept(captured(&store, "Buy stamps", None, &choices)?)?;
    assert_eq!(stamps.place, None);
    assert!(nudge(&store, &stamps.id)?.is_some());
    let tap = kept(captured(&store, "Fix the tap", Some(&household), &choices)?)?;
    assert_eq!(tap.place.as_deref(), Some(household.as_str()));
    assert_eq!(nudge(&store, &tap.id)?, None);
    Ok(())
}

#[test]
fn an_empty_capture_writes_nothing() -> TestResult {
    let (store, _) = store()?;
    let entries = store.all_entries()?.len();
    let choices = CaptureChoices::default();
    assert_eq!(
        captured(&store, "  \n ", None, &choices)?,
        Captured::Nothing
    );
    assert_eq!(store.all_entries()?.len(), entries);
    Ok(())
}

#[test]
fn a_list_kind_makes_each_typed_line_a_line() -> TestResult {
    let (store, _) = store()?;
    let checklist = CaptureChoices {
        kind: CaptureKind::Checklist,
        ..CaptureChoices::default()
    };
    let made = kept(captured(
        &store,
        "Passport\nTickets\nCharger",
        None,
        &checklist,
    )?)?;
    let list = Checklists::new(&store)
        .by_note(&made.id)?
        .ok_or("a Checklist")?;
    let lines: Vec<String> = list.lines.into_iter().map(|l| l.text).collect();
    assert_eq!(lines, ["Passport", "Tickets", "Charger"]);
    // A list leaves Unsorted.
    assert_eq!(nudge(&store, &made.id)?, None);
    let shopping = CaptureChoices {
        kind: CaptureKind::ShoppingList,
        ..CaptureChoices::default()
    };
    let weekend = kept(captured(&store, "# Weekend\nMilk\nEggs", None, &shopping)?)?;
    assert!(ShoppingLists::new(&store).is_shopping_list(&weekend.id)?);
    Ok(())
}

#[test]
fn with_nothing_typed_the_kind_is_no_list() -> TestResult {
    let (store, _) = store()?;
    let choices = CaptureChoices {
        kind: CaptureKind::Checklist,
        ..CaptureChoices::default()
    };
    let photo = vec![1, 2, 3];
    let captured = capture_choices::capture(
        &store,
        "  ",
        None,
        Notes::default_nudge(),
        (std::slice::from_ref(&photo), &[], &[]),
        (&choices, &wall),
    )?;
    let kept = kept(captured)?;
    assert!(!Checklists::new(&store).is_checklist(&kept.id)?);
    // Unsorted, it keeps its Nudge.
    assert!(nudge(&store, &kept.id)?.is_some());
    Ok(())
}
