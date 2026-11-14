use super::*;
use crate::alarms::Alarms;
use crate::lan::{Joining, answer_sync_inviting, apply_answer_as, read_request, sync_request_as};
use crate::notes::Notes;
use crate::places::Places;
use crate::routines::Routines;
use crate::schedule::{ClockTime, Day, Schedule};
use crate::shared::{AreaAccess, AreaRole, Invite};
use crate::signing::SigningKey;
use crate::store::tests::{store, ticking};
use chrono::NaiveDateTime;
use std::net::{IpAddr, Ipv4Addr};

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;
type Made<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn utc_wall(t: DateTime<Utc>) -> NaiveDateTime {
    t.naive_utc()
}

fn at_ten(day: i64) -> Made<Schedule> {
    let ten: Vec<ClockTime> = ClockTime::new(10, 0).into_iter().collect();
    Ok(Schedule::new(Day::new(2026, 1, day), &ten, None).ok_or("a schedule")?)
}

#[test]
fn writes_inside_drafting_wait_in_the_task_while_the_users_own_reach_the_log() -> TestResult {
    let store = store();
    let milk = Notes::new(&store).capture("Milk", None, None)?;
    let logged = store.all_entries()?.len();
    let task = store.open_ai_task()?;

    let made = store.drafting(&task, || {
        Notes::new(&store).edit(&milk.id, "Oat milk")?;
        // The user edits on another thread meanwhile.
        std::thread::scope(|s| s.spawn(|| Notes::new(&store).amend(&milk.id, "2 l")).join())
            .map_err(|_| Error::Invalid("the user's thread".into()))??;
        Ok(Notes::new(&store).capture("Bread", None, None)?.id)
    })?;

    assert_eq!(store.all_entries()?.len(), logged + 4, "only the Amend");
    let notes = Notes::new(&store);
    assert_eq!(
        notes.by_id(&milk.id)?.map(|n| n.text),
        Some("Oat milk".into())
    );
    assert_eq!(notes.by_id(&made)?.map(|n| n.text), Some("Bread".into()));
    assert!(store.logged_history(&made)?.is_empty());
    let set = store.pending_set(&task)?.ok_or("the task")?;
    assert_eq!(set.drafts().len(), 1 + 3, "the edit and the capture");
    // After the call, the same thread writes to the log again.
    notes.set_importance(&milk.id, crate::Importance::Important)?;
    assert_eq!(store.all_entries()?.len(), logged + 5);
    Ok(())
}

#[test]
fn a_call_that_fails_leaves_the_task_as_it_was() -> TestResult {
    let store = store();
    let milk = Notes::new(&store).capture("Milk", None, None)?;
    let task = store.open_ai_task()?;
    let failed: Result<()> = store.drafting(&task, || {
        Notes::new(&store).edit(&milk.id, "Oat milk")?;
        Notes::new(&store).file(&milk.id, "area:nowhere")
    });
    assert!(failed.is_err());
    assert!(
        store
            .pending_set(&task)?
            .ok_or("the task")?
            .drafts()
            .is_empty()
    );
    assert_eq!(
        Notes::new(&store).by_id(&milk.id)?.map(|n| n.text),
        Some("Milk".into())
    );
    Ok(())
}

#[test]
fn an_ai_task_shares_blocks_sends_and_sets_nothing() -> TestResult {
    let store = store();
    let house = Places::new(&store).add_area("Household")?;
    let milk = Notes::new(&store).capture("Milk", Some(&house.id), None)?;
    let task = store.open_ai_task()?;
    let tries: Vec<Result<()>> = vec![
        store.drafting(&task, || store.share(&house.id).map(drop)),
        store.drafting(&task, || {
            store.block_person("profile:carol", true).map(drop)
        }),
        store.drafting(&task, || {
            store.append(&milk.id, "sent:x", Some("{}"), None).map(drop)
        }),
        store.drafting(&task, || {
            store
                .append(
                    &milk.id,
                    &crate::notes::keys::photo("ab"),
                    Some("added"),
                    None,
                )
                .map(drop)
        }),
        store.drafting(&task, || {
            store
                .append(&milk.id, "$type", Some("routine"), None)
                .map(drop)
        }),
        // A captured detail it may remove, but never set.
        store.drafting(&task, || {
            store
                .append(&milk.id, "location", Some("52.52,13.405"), None)
                .map(drop)
        }),
        store.drafting(&task, || {
            store
                .append(&milk.id, "bluetooth", Some(r#"["Headset"]"#), None)
                .map(drop)
        }),
        store.drafting(&task, || store.set_local_setting("renagMinutes", "1")),
        store.drafting(&task, || {
            store.put_photo(&[0xFF, 0xD8, 0xFF, 0xD9]).map(drop)
        }),
    ];

    for (n, tried) in tries.iter().enumerate() {
        assert!(tried.is_err(), "try {n} went through");
    }
    assert!(
        store
            .pending_set(&task)?
            .ok_or("the task")?
            .drafts()
            .is_empty()
    );
    assert!(!store.is_shared(&house.id)?);
    assert!(store.local_setting("renagMinutes")?.is_none());
    Ok(())
}

/// Ann shares her Household with Ben as a Member, with her Note and
/// Reminder in it.
fn member() -> Made<(Store, Store, String, String, String)> {
    let shared = member_with(|_, _| Ok(Vec::new()))?;
    Ok((
        shared.ann,
        shared.ben,
        shared.area,
        shared.milk,
        shared.reminder,
    ))
}

/// Ann's Household shared with Ben, a Member; `more` what Ann set up
/// in it before, by id.
struct Shared {
    ann: Store,
    ben: Store,
    area: String,
    milk: String,
    reminder: String,
    more: Vec<String>,
}

/// [`member`], with `prepare` setting up more in the Household (its
/// id given) before it is shared.
fn member_with(prepare: impl FnOnce(&Store, &str) -> Made<Vec<String>>) -> Made<Shared> {
    let device = |seed: u8, name: &str| -> Made<Store> {
        let store = Store::in_memory(ticking(), move || Ok(SigningKey::from_seed([seed; 32])))?;
        crate::profile::OwnProfile::new(&store).apply_defaults(name, None)?;
        Ok(store)
    };
    let ann = device(1, "Ann")?;
    let ben = device(2, "Ben")?;
    let house = Places::new(&ann).add_area("Household")?;
    let milk = Notes::new(&ann).capture("Buy milk", Some(&house.id), None)?;
    let reminder = Routines::new(&ann).spawn(&milk.id, &at_ten(2)?)?;
    let more = prepare(&ann, &house.id)?;
    let area = ann.share(&house.id)?;
    let invite = Invite {
        area: area.clone(),
        role: AreaRole::Member,
    };
    let request = sync_request_as(
        &ben,
        &ann.version_vector()?,
        "P",
        &ann.device_id()?,
        "",
        Joining::Invited,
    )?;
    let (answer, _) =
        answer_sync_inviting(&ann, &request, "D", Some(&invite))?.ok_or("an answer")?;
    apply_answer_as(&ben, &answer, true, None)?;
    assert_eq!(ben.access_in(&area, None)?, AreaAccess::Member);
    Ok(Shared {
        ann,
        ben,
        area,
        milk: milk.id,
        reminder: reminder.id,
        more,
    })
}

#[test]
fn an_ai_task_does_only_what_the_role_in_a_shared_area_allows() -> TestResult {
    let (_ann, ben, area, milk, reminder) = member()?;
    let task = ben.open_ai_task()?;
    let notes = Notes::new(&ben);
    let later = at_ten(3)?;

    // A Member's own Note in the Area, and an Amend of Ann's: allowed.
    let own = ben.drafting(&task, || notes.capture("Bread", Some(&area), None))?;
    ben.drafting(&task, || notes.amend(&milk, "Two litres").map(drop))?;
    ben.drafting(&task, || notes.edit(&own.id, "Rye bread"))?;
    // Ann's Note and Reminder: not a Member's to change.
    assert!(
        ben.drafting(&task, || notes.edit(&milk, "Buy beer"))
            .is_err()
    );
    assert!(
        ben.drafting(&task, || Routines::new(&ben).reschedule(&reminder, &later))
            .is_err()
    );
    assert!(
        ben.drafting(&task, || Places::new(&ben).rename(&area, "Ours"))
            .is_err()
    );

    let drafted: Vec<String> = ben
        .pending_set(&task)?
        .ok_or("the task")?
        .drafts()
        .iter()
        .map(|d| d.entity.clone())
        .collect();
    assert!(!drafted.contains(&milk) && !drafted.contains(&reminder));
    assert!(!drafted.contains(&area));
    Ok(())
}

#[test]
fn an_ai_task_writes_none_of_the_users_own_data() -> TestResult {
    use crate::boxes::Boxes;
    use crate::checklists::Checklists;
    use crate::frogs::Frogs;
    use crate::schedule::Occurrence;
    use crate::sessions::{SessionRule, Sessions};
    use crate::shopping_lists::ShoppingLists;
    use crate::tallies::Tallies;
    let store = store();
    let notes = Notes::new(&store);
    let wall = |t: DateTime<Utc>| t.naive_utc();
    let run = notes.capture("Run", None, None)?.id;
    let field = Tallies::new(&store).add_field(&run, "Run", "km", None, None)?;
    let entry = Tallies::new(&store).add(&run, &BTreeMap::new(), "park", None)?;
    let trip = notes.capture("Trip\nSocks", None, None)?.id;
    Checklists::new(&store).make(&trip)?;
    let socks = Checklists::new(&store)
        .by_note(&trip)?
        .ok_or("a Checklist")?
        .lines[0]
        .id
        .clone();
    let shop = notes.capture("Shop\nMilk", None, None)?.id;
    ShoppingLists::new(&store).make(&shop)?;
    let milk = ShoppingLists::new(&store)
        .by_note(&shop, &wall)?
        .ok_or("a Shopping List")?
        .lines[0]
        .id
        .clone();
    let bank = notes.capture("Call the bank ☐", None, None)?;
    let frog = Frogs::new(&store).spawn(&bank.id)?;
    let box_id = bank.boxes.first().ok_or("a Box")?.id.clone();
    let daily = Schedule::new(
        Day::new(2026, 1, 1),
        &ClockTime::new(8, 0).into_iter().collect::<Vec<_>>(),
        Some(crate::schedule::Recurrence::parse("FREQ=DAILY").map_err(|e| e.to_string())?),
    )
    .ok_or("a schedule")?;
    // A Session an older version spawned from the Checklist, its setup
    // on the Routine (ADR 0039), so one Routine takes ticks and picks.
    let routine = Routines::new(&store).spawn_with(&trip, &daily, |id| {
        store.append(id, session_keys::POOL, Some(&run), None)?;
        store.append(id, session_keys::COUNT, Some("1"), None)?;
        store.append(
            id,
            session_keys::RULE,
            Some(SessionRule::Rolling.name()),
            None,
        )?;
        Ok(())
    })?;
    let session = Sessions::new(&store)
        .by_id(&routine.id)?
        .ok_or("a Session")?;
    let first = Occurrence::new(Day::new(2026, 1, 1), ClockTime::new(8, 0));
    let task = store.open_ai_task()?;
    let raw = |entity: &str, field: &str, value: &str| -> Result<()> {
        store.drafting(&task, || {
            store.append(entity, field, Some(value), None).map(drop)
        })
    };
    let at = "2026-01-01T10:00:00.000Z";
    let tries: Vec<(&str, Result<()>)> = vec![
        ("an entry", raw("tallyEntry:x", "$type", "tallyEntry")),
        (
            "an entry's value",
            raw(&entry.id, &crate::tallies::keys::value(&field), "5"),
        ),
        ("an entry's line", raw(&entry.id, "line", "track")),
        ("a Line's tick", raw(&socks, "checked", at)),
        ("a Shopping Line's check", raw(&milk, "checked", at)),
        (
            "a tick per Occurrence",
            raw(session.id(), &format!("check@{}@{socks}", first.key()), at),
        ),
        (
            "a check-off",
            raw(session.id(), &crate::routines::keys::done(&first), at),
        ),
        (
            "a skip",
            raw(session.id(), &crate::routines::keys::skip(&first), at),
        ),
        (
            "a move to tomorrow",
            raw(
                session.id(),
                &crate::routines::keys::skip(&first),
                &format!("{at} tomorrow"),
            ),
        ),
        (
            "a pick",
            raw(session.id(), &crate::sessions::keys::pick(&first), &run),
        ),
        (
            "an entry of a pick",
            raw(
                session.id(),
                &crate::sessions::keys::entry(&first, &entry.id),
                &run,
            ),
        ),
        ("an eaten Frog", raw(&frog.id, "done", at)),
        (
            "today's Frogs",
            raw("frogs:pick", "pick", &format!("2026-01-01,{}", frog.id)),
        ),
        (
            "a Box's tick",
            raw(&bank.id, &crate::boxes::tick_field(&box_id), "done"),
        ),
        ("an unknown field", raw(&run, "tally:x:values", "1")),
        (
            "the calls",
            store.drafting(&task, || {
                Tallies::new(&store)
                    .add(&run, &BTreeMap::new(), "track", None)
                    .map(drop)
            }),
        ),
        (
            "",
            store.drafting(&task, || Checklists::new(&store).check(&socks)),
        ),
        (
            "",
            store.drafting(&task, || ShoppingLists::new(&store).check(&milk, &wall)),
        ),
        (
            "",
            store.drafting(&task, || Frogs::new(&store).mark_done(&frog.id, None)),
        ),
        (
            "",
            store.drafting(&task, || Frogs::new(&store).pick_today(&wall).map(drop)),
        ),
        (
            "",
            store.drafting(&task, || Sessions::new(&store).pick_today(&wall).map(drop)),
        ),
        (
            "",
            store.drafting(&task, || {
                Sessions::new(&store)
                    .record(session.id(), &first, &run, &BTreeMap::new(), "track")
                    .map(drop)
            }),
        ),
        (
            "",
            store.drafting(&task, || {
                Routines::new(&store).mark_done(session.id(), &first, None)
            }),
        ),
        (
            "",
            store.drafting(&task, || {
                Boxes::new(&store).tick(&bank.id, &box_id, crate::text_boxes::BoxState::Done)
            }),
        ),
    ];
    for (n, (what, tried)) in tries.iter().enumerate() {
        // Refused by the judge, not by the call's own checks.
        let judged = tried
            .as_ref()
            .is_err_and(|e| e.to_string().starts_with("not for an AI Task"));
        assert!(judged, "try {n} ({what}): {tried:?}");
    }
    assert!(
        store
            .pending_set(&task)?
            .ok_or("the task")?
            .drafts()
            .is_empty()
    );
    Ok(())
}

#[test]
fn set_up_does_only_what_the_role_allows() -> TestResult {
    use crate::checklists::Checklists;
    use crate::tallies::Tallies;
    let shared = member_with(|ann, house| {
        let notes = Notes::new(ann);
        let trip = notes.capture("Trip\nSocks", Some(house), None)?.id;
        Checklists::new(ann).make(&trip)?;
        let run = notes.capture("Run", Some(house), None)?.id;
        Tallies::new(ann).add_field(&run, "Run", "km", None, None)?;
        let bank = notes.capture("Call the bank", Some(house), None)?.id;
        crate::frogs::Frogs::new(ann).spawn(&bank)?;
        let session = crate::sessions::Sessions::new(ann).spawn(
            &bank,
            &at_ten(2)?,
            std::slice::from_ref(&run),
            1,
            crate::sessions::SessionRule::Rolling,
        )?;
        Ok(vec![trip, run, bank, session.id().to_string()])
    })?;
    let (ben, milk) = (&shared.ben, &shared.milk);
    let (trip, run) = (&shared.more[0], &shared.more[1]);
    let bank = &shared.more[2];
    let socks = Checklists::new(ben)
        .by_note(trip)?
        .ok_or("a Checklist")?
        .lines[0]
        .id
        .clone();
    let field = Tallies::new(ben).by_note(run)?.ok_or("a Tally")?.fields[0]
        .id
        .clone();
    let task = ben.open_ai_task()?;
    let call = |name: &str, args: serde_json::Value| {
        ben.ai_tool(&task, name, &args, &|t: DateTime<Utc>| t.naive_utc())
    };
    let added = call("add_line", serde_json::json!({"id": trip, "text": "Shirt"}))?;
    assert!(!added.is_error, "a Member adds a Line: {added:?}");
    for (name, args) in [
        (
            "rename_line",
            serde_json::json!({"id": socks, "text": "Sox"}),
        ),
        ("remove_line", serde_json::json!({"id": socks})),
        (
            "make_tally",
            serde_json::json!({"fields": [{"name": "Milk"}], "id": milk}),
        ),
        (
            "rename_tally_field",
            serde_json::json!({"field": field, "id": run, "name": "Jog"}),
        ),
        ("make_frog", serde_json::json!({"id": milk})),
        (
            "make_list",
            serde_json::json!({"id": milk, "list": "checklist"}),
        ),
        ("list_to_plain", serde_json::json!({"id": trip})),
        ("switch_list", serde_json::json!({"id": trip})),
        ("move_line", serde_json::json!({"id": socks})),
        (
            "group_line",
            serde_json::json!({"id": socks, "under": false}),
        ),
        ("frog_to_plain", serde_json::json!({"id": bank})),
        ("set_session", serde_json::json!({"count": 2, "id": bank})),
    ] {
        let out = call(name, args)?;
        assert!(out.is_error, "{name} went through: {out:?}");
    }
    // Trashing a Frog has no check of its own: the judge refuses it.
    let frog = call("frog_to_plain", serde_json::json!({"id": bank}))?;
    assert!(format!("{frog:?}").contains("role"), "{frog:?}");
    // Even past the screens' checks, the judge refuses.
    let raw = ben.drafting(&task, || {
        ben.append(
            run,
            &crate::tallies::keys::field(&field, "name"),
            Some("Jog"),
            None,
        )
        .map(drop)
    });
    assert!(raw.is_err());
    let drafted: Vec<String> = ben
        .pending_set(&task)?
        .ok_or("the task")?
        .drafts()
        .iter()
        .map(|d| d.entity.clone())
        .collect();
    assert!(
        drafted
            .iter()
            .all(|e| e.starts_with("line:") && *e != socks)
    );
    assert!(!drafted.is_empty());
    Ok(())
}

#[test]
fn the_review_groups_the_set_ups_by_kind() -> TestResult {
    use crate::checklists::Checklists;
    use crate::frogs::Frogs;
    use crate::sessions::{SessionRule, Sessions};
    use crate::tallies::Tallies;
    let store = store();
    let notes = Notes::new(&store);
    let run = notes.capture("Run", None, None)?.id;
    let trip = notes.capture("Trip\nSocks", None, None)?.id;
    let bank = notes.capture("Call the bank", None, None)?.id;
    let later = at_ten(2)?;
    let task = store.open_ai_task()?;
    store.drafting(&task, || {
        Tallies::new(&store).add_field(&run, "Run", "km", Some(5000.0), None)?;
        Checklists::new(&store).make(&trip)?;
        Frogs::new(&store).spawn(&bank)?;
        Sessions::new(&store).spawn(
            &bank,
            &later,
            std::slice::from_ref(&run),
            1,
            SessionRule::Random,
        )?;
        Ok(())
    })?;
    let review = store.ai_review(&task)?;
    let kinds: Vec<AiChangeKind> = review.groups.iter().map(|(k, _)| *k).collect();
    for kind in [
        AiChangeKind::Tally,
        AiChangeKind::List,
        AiChangeKind::Frog,
        AiChangeKind::Session,
    ] {
        assert!(kinds.contains(&kind), "{kind:?} in {kinds:?}");
    }
    assert!(
        !kinds.contains(&AiChangeKind::Routine),
        "the Session as one"
    );
    let items = |kind: AiChangeKind| -> Vec<AiReviewItem> {
        review
            .groups
            .iter()
            .find(|(k, _)| *k == kind)
            .map(|(_, items)| items.clone())
            .unwrap_or_default()
    };
    let tally = &items(AiChangeKind::Tally)[0];
    assert_eq!(tally.note.as_deref(), Some(run.as_str()));
    let target = tally
        .changes
        .iter()
        .find(|c| c.field.ends_with(":target"))
        .ok_or("the target")?;
    assert_eq!(
        (target.before.as_deref(), target.after.as_deref()),
        (None, Some("5000"))
    );
    let lists = items(AiChangeKind::List);
    assert!(lists.iter().any(|i| i.entity == trip));
    assert!(
        lists
            .iter()
            .any(|i| i.entity.starts_with("line:") && i.note.as_deref() == Some(trip.as_str()))
    );
    assert_eq!(
        items(AiChangeKind::Frog)[0].note.as_deref(),
        Some(bank.as_str())
    );
    assert_eq!(
        items(AiChangeKind::Session)[0].note.as_deref(),
        Some(bank.as_str())
    );
    Ok(())
}

#[test]
fn the_review_groups_each_change_with_before_and_after() -> TestResult {
    let store = store();
    let notes = Notes::new(&store);
    let places = Places::new(&store);
    let house = places.add_area("Household")?;
    let milk = notes.capture("Milk", None, None)?;
    let old = notes.capture("Old list", None, None)?;
    let pills = notes.capture("Pills", None, None)?;
    let routine = Routines::new(&store).spawn(&pills.id, &at_ten(2)?)?;
    let task = store.open_ai_task()?;
    let bread = store.drafting(&task, || {
        notes.edit(&milk.id, "Oat milk")?;
        notes.file(&milk.id, &house.id)?;
        notes.set_importance(&pills.id, crate::Importance::Important)?;
        Routines::new(&store).set_renag(&routine.id, Some(chrono::Duration::minutes(5)))?;
        Routines::new(&store).trash_note(&old.id)?;
        places.add_topic(&house.id, "Kitchen")?;
        // Written back as it was: no change.
        notes.edit(&pills.id, "Pills")?;
        Ok(notes.capture("Bread", None, None)?.id)
    })?;
    // The user fixes the text meanwhile: hers stays.
    notes.edit(&milk.id, "Milk, 2 l")?;

    let review = store.ai_review(&task)?;

    let kinds: Vec<AiChangeKind> = review.groups.iter().map(|(k, _)| *k).collect();
    assert_eq!(
        kinds,
        [
            AiChangeKind::NoteAdded,
            AiChangeKind::NoteFiled,
            AiChangeKind::NoteTrashed,
            AiChangeKind::NoteText,
            AiChangeKind::NoteImportance,
            AiChangeKind::Routine,
            AiChangeKind::Place,
        ]
    );
    let item = |kind: AiChangeKind| {
        review
            .groups
            .iter()
            .find(|(k, _)| *k == kind)
            .and_then(|(_, items)| items.first())
            .cloned()
            .ok_or("an item")
    };
    let text = item(AiChangeKind::NoteText)?;
    assert_eq!(text.entity, milk.id);
    assert_eq!(text.note.as_deref(), Some(milk.id.as_str()));
    let change = &text.changes[0];
    assert_eq!(change.field, "text");
    assert!(change.mine_wins);
    let stored = |t: &str| crate::StoredText::plain(t).encode();
    assert_eq!(change.before, Some(stored("Milk, 2 l")));
    assert_eq!(change.after, Some(stored("Oat milk")));
    let filed = item(AiChangeKind::NoteFiled)?;
    assert_eq!(filed.changes[0].before, None);
    assert_eq!(filed.changes[0].after.as_deref(), Some(house.id.as_str()));
    assert!(!filed.changes[0].mine_wins);
    assert_eq!(item(AiChangeKind::NoteAdded)?.entity, bread);
    assert_eq!(item(AiChangeKind::NoteTrashed)?.entity, old.id);
    let renag = item(AiChangeKind::Routine)?;
    assert_eq!(renag.note.as_deref(), Some(pills.id.as_str()));
    assert_eq!(renag.changes[0].after.as_deref(), Some("5"));
    assert!(
        store
            .ai_review(&store.open_pending("bundle", true, None)?)?
            .is_empty()
    );
    Ok(())
}

#[test]
fn one_ai_task_at_a_time_and_none_drafts_into_another_set() -> TestResult {
    let store = store();
    let task = store.open_ai_task()?;
    assert_eq!(store.open_ai_task()?, task);
    assert_eq!(store.ai_task()?, Some(task.clone()));
    let other = store.open_pending("bundle", true, None)?;
    assert!(store.drafting(&other, || Ok(())).is_err());
    store.accept_pending(&task)?;
    assert_ne!(store.open_ai_task()?, task);
    Ok(())
}

#[test]
fn sync_waits_while_an_ai_task_does() -> TestResult {
    let store = store();
    let dir = tempfile::tempdir()?;
    let bundle = dir.path().join("out.zip");
    crate::profile::OwnProfile::new(&store).apply_defaults("Ann", None)?;
    let milk = Notes::new(&store).capture("Milk", None, None)?;
    assert!(!store.sync_paused()?);
    let task = store.open_ai_task()?;
    assert!(store.sync_paused()?);

    let vector = read_request(
        &mut std::io::Cursor::new(&b"GET /vector HTTP/1.1\r\n\r\n"[..]),
        IpAddr::V4(Ipv4Addr::LOCALHOST),
    )?
    .ok_or("a request")?;
    let (answered, _) = store.serve(&vector, "Desk")?;
    assert_eq!(answered.status, 503);
    let paused = |r: Result<()>| matches!(r, Err(Error::SyncPaused));
    assert!(paused(
        crate::bundle::write_bundle_now(&store, &bundle, None).map(drop)
    ));
    assert!(paused(
        crate::bundle::import_bundle(&store, &bundle).map(drop)
    ));
    let mut folder = crate::folder::LocalFolder::new(dir.path().join("folder"));
    assert!(paused(
        crate::folder::folder_sync(&store, &mut folder, &mut crate::folder::random_nonce).map(drop)
    ));
    assert!(paused(store.send_note(&milk.id, None, false).map(drop)));
    assert!(!dir.path().join("folder").exists());

    store.decline_pending(&task)?;
    assert!(!store.sync_paused()?);
    assert!(crate::bundle::write_bundle_now(&store, &bundle, None)?.is_some());
    assert_ne!(store.serve(&vector, "Desk")?.0.status, 503);
    Ok(())
}

#[test]
fn alarms_fire_as_the_log_has_them_until_the_task_is_accepted() -> TestResult {
    let store = store();
    let notes = Notes::new(&store);
    let pills = notes.capture("Pills", None, None)?;
    let walk = notes.capture("Walk", None, None)?;
    let routine = Routines::new(&store).spawn(&pills.id, &at_ten(2)?)?;
    let (later, same) = (at_ten(3)?, at_ten(2)?);
    let task = store.open_ai_task()?;
    store.drafting(&task, || {
        Routines::new(&store).reschedule(&routine.id, &later)?;
        Routines::new(&store).spawn(&walk.id, &same).map(drop)
    })?;
    let planned = |store: &Store| -> Made<Vec<(String, String)>> {
        Ok(Alarms::new(store)
            .plan(&utc_wall)?
            .into_iter()
            .map(|a| (a.routine_id, a.occurrence.key()))
            .collect())
    };

    assert_eq!(
        Routines::new(&store).all()?.len(),
        2,
        "the Agenda shows both"
    );
    let before = planned(&store)?;
    assert_eq!(before, [(routine.id.clone(), occurrence_key(2))]);

    store.accept_pending(&task)?;
    let after = planned(&store)?;
    assert_eq!(after.len(), 2);
    assert!(after.contains(&(routine.id.clone(), occurrence_key(3))));
    Ok(())
}

fn occurrence_key(day: i64) -> String {
    crate::schedule::Occurrence::new(Day::new(2026, 1, day), ClockTime::new(10, 0)).key()
}
